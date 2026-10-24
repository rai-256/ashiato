# CH02 design — 書き出しの「次の記録」の引きを窓関数の一度読みにする

> 第 1 回の深掘りの推奨の答え（Q1〜Q4）で先に書いた。Q1 の答えが「いまのまま」なら D1・D4 が変わる（反転条件に書いた）。

## Context

動機は `proposal.md` の Why。いまの形（`crates/server/src/export/excluded.rs` の `overlap_sql`）:

- 記録 `r` を `core.event_live`（記録）と、`core.event_version_live` を親に JOIN した行（前の版。鍵は親の `user_id`・`logical_source`・`device_id`）から読み、
  `r.event_time <= max(erased_spans.end_at)` と緯度経度の判定で先に絞ってから、幅の (c) の行にだけ相関副問い合わせで次の記録を引く
  （記録は `(n.event_time, n.id) > (r.event_time, r.id)`、前の版は `n.event_time > r.event_time`）
- 副問い合わせ 1 回が読む量は計画しだいで決まる。計画が `event_by_user_source_ingest` を選ぶと、`n.device_id IS NOT DISTINCT FROM r.device_id` を索引で絞れず組（利用者・ソース）の行を毎回読み流し、読む量は「絞った行の数 × 組の行の数」（2 乗）になる（CH02.md の 84 秒・`premises.md`）。統計を取り直して `event_by_source_time` を選べば 1 回数行で止まることもあるが、次の記録が遠い配置（組が入り交じる・組の最後の行）では同じく組を読み流す（review/spec.md R2・R3 の実測: 端末が行ごとに違う配置で N → 4N の読んだ行の数が 16 倍）。どちらの計画になるかは統計しだいで、本物の写しで 2 乗になったのが観測（窓関数は計画に依らず一度読み）
- 同じ断片を書き出しの本体（`dump.rs:138`。`transaction_timestamp()`）・`GET /export/summary`（`summary.rs:92`。`now()`）・後に消した記録の件数（`ledger.rs:431`。`now()`）が通る

ST33 design D5 は「`lead()` は全行を並べ替えてから絞る」ことを理由に相関副問い合わせを選んだ。D5 が守りたかった絞り込み（消した時間の終わりより後の行に次の記録を求めない）は、
窓の**前に**時刻だけで置ける（D3）。窓の外の緯度経度の判定は、次の記録の候補を減らしてはいけない（spec: 緯度経度を持たない記録も次の記録になる）ので、窓の**後**に置く。

## Goals / Non-Goals

**Goals:**
- 除く範囲の判定が読む量を、記録と前の版の行数に対して線形に近くする（spec の ADDED Requirement）
- どの記録・前の版が除かれるかを、Q1 の同じ時刻の組のほかは変えない（いまの SQL を試験に残して突き合わせる。D6）

**Non-Goals:**
- 試験の DB の積もり（共有の DB に行が積もって遅くなること）の直し・server 全体の遅い問い合わせの洗い出し（範囲は依頼のとおり。別の change）
- ST33 D13 の「件数を前回の書き出しの目録から出す」への替え（計測の値を渡すだけ。D8）
- 幅の (a)(b)(d)・緯度経度の判定（D5a）・`SPAN_RULES` などの一覧の中身を変えること
- 移行・索引を足すこと（`premises.md`: 索引 + 相関副問い合わせは 120 万行で約 14 分）

## Decisions

### D1. 次の記録は、逆向きの並びで頭を固定した枠の `min` で求める（Q1 の推奨のとき）

次の記録の時刻は、組（`user_id`・`logical_source`・`device_id`）の中で次で求める:

```sql
min(event_time) FILTER (WHERE is_event) OVER (
  PARTITION BY user_id, logical_source, device_id
  ORDER BY event_time DESC
  RANGE BETWEEN UNBOUNDED PRECEDING AND '1 microsecond' PRECEDING)
```

- 逆向きの並びの「頭（最も後の時刻）から自分より 1 µs 後まで」の `min` = 自分より**厳密に後**の最初の時刻（`timestamptz` の精度は 1 µs。`premises.md`）。同じ時刻の記録は枠に入らない（Q1）
- `PARTITION BY` は `device_id` の NULL を 1 つの組にまとめる（`IS NOT DISTINCT FROM` と同じ。`premises.md` で食い違い 0 件）
- 枠の頭が固定なので、`min` は枠が伸びるだけで数え直さない（線形）。**順向きで尾を固定した枠（`RANGE BETWEEN '1 microsecond' FOLLOWING AND UNBOUNDED FOLLOWING` の `min`）は使わない** ——
  枠の頭が動くたびに数え直し、2 乗に戻る（`premises.md`: 120 万行で 9 分を超えて止めた）
- 候補は `is_event`（`core.event_live` の行）だけ。前の版の行は窓に並ぶが候補にならない（前の版の次の記録は前の版ではない。ST33 D5 の前の版の 3）
- 退けた案: `lead(event_time) OVER (… ORDER BY event_time, id)` —— Q1 の推奨では同じ時刻の隣を飛ばせない。前の版を混ぜると前の版が「次」になる。
  索引 `(user_id, logical_source, device_id, event_time, id)` + 端末の NULL と非 NULL で 2 本に分けた相関副問い合わせ —— 120 万行で約 14 分（`premises.md`）

反転条件: Q1 の答えが「いまのまま（`(event_time, id)` の順で隣の記録）」なら、記録の側は `lead(event_time) OVER (PARTITION BY … ORDER BY event_time, id)` を記録だけの並びで当て、
前の版の側だけをこの枠で求める（並びを 2 つにする）。

### D2. 窓に載せるのは鍵の列だけにし、`payload` / `raw` は窓の後に `id` で戻す

窓が並べ替えるのは `id`・`user_id`・`logical_source`・`device_id`・`event_time`・`is_event` だけ。幅の (a)(b)(d) と緯度経度の判定（`payload` / `raw` を読む）は窓の後に、
次の記録の時刻を持った行へ `id` で戻して当てる。

- 本物の行は `payload`・`raw` が太く、並べ替えに載せると一時ファイルの量が桁で変わる（review/deep.md R3。`premises.md` の 1.9 秒は細い表の値）
- 窓の行には緯度経度の判定で絞る前の**全部**（D3 の時刻の絞り込みだけを通した行）を入れる。緯度経度を持たない記録も次の記録になる（spec の Scenario）。
  緯度経度で絞った後に窓を当てると幅が延び、除く行が変わる（review/deep.md R4 (2)）

### D3（仮）. 窓の前に、時刻だけで絞る（Q4 の推奨）

- 消した滞在が 0 件なら、記録の表と前の版の表を読まない。門は `core.event_live` / `core.event_version_live` を読む各副問い合わせに
  `WHERE EXISTS (SELECT 1 FROM erased_spans)` を置き、PostgreSQL の One-Time Filter（計画の `One-Time Filter: (InitPlan …)`。偽なら下の節を 1 度も実行しない）にする。
  **いまの形（`r.event_time <= (SELECT max(end_at) FROM erased_spans)`）は、値が NULL でも外側の走査を実行する**（表を全部読み、次の記録だけを引かない。review/spec.md R4 の実測）ので、
  これはいまより強い保証になる（spec の ADDED の Scenario「消した滞在が無ければ、除く範囲の判定は記録の表を読まない」）
- 窓に入れるのは、時刻が `max(erased_spans.end_at)` 以下の行（記録と前の版）と、**組ごとに、それより後の記録の最初の 1 行**（時刻が最小のもの。候補としてだけ使う）
  - 時刻が `max(end_at)` 以下の行の「厳密に後の最初の記録」は、組の中で `max(end_at)` 以下にあるか、無ければ組の `max(end_at)` より後の最初の記録なので、これで結果は変わらない
  - `max(end_at)` より後の行は、幅が消した時間の終わりより後に始まるので、それ自身は除かれない（いまの `r.event_time <= max(end_at)` と同じ）
  - 組ごとの後の最初の 1 行が結果に効くのは、写しの時点（`snapshot_at`）が消した滞在の始まりより前のときだけ（そうでなければ、次の記録の代わりに写しの時点を使っても `max(end_at)` を持つ消した時間と必ず重なる。
    review/spec.md R5）。それでも入れるのは、写しの時点を固定の値で渡す呼び出しや時計の狂った端末の滞在で結果を変えないため。D6 の族「写しの時点が消した滞在の始まりより前」が固定する
  - 組ごとの最初の 1 行は `GROUP BY user_id, logical_source, device_id` の `min(event_time)`（`max(end_at)` より後の行を 1 度読む。線形）。PostgreSQL 17 は skip scan を持たないので、索引で組ごとに飛ぶ形にはしない
- 結果が変わらないことは D6 の突き合わせが固定する

反転条件: 絞り込みの SQL が長くなって D6 の突き合わせの組み合わせを増やしても読み違いが残るなら、絞り込みを外して全行に窓を当てる（合成 120 万行で 1 回 2 秒の桁）。
消した滞在が最近のときは絞り込みがほぼ全行を通し、窓は全行を並べる（線形に近いが、並べ替えの分 `n log n`）。D8 の計測で 1 回 3 秒以上なら、並べ替えの `work_mem` か形を見直す。

### D4. 記録と前の版を 1 つの並びに混ぜ、1 回の窓で両方の次の記録を求める

Q1 の推奨では記録も前の版も「厳密に後の最初の記録」で同じなので、記録（`is_event = true`）と前の版（親の鍵を持たせ `is_event = false`）を `UNION ALL` した 1 つの並びに D1 の枠を 1 回当てる。
いまは記録と前の版で別々に 2 回引いている（`overlap_event` / `overlap_version`）。`EXCLUDED_EVENT` と `EXCLUDED_VERSION` の CTE の名前と中身（id の集まり）は変えない。

- 前の版の組の鍵は親（`core.event_live`）から取る —— `core.event_version` は `device_id` を持たず、`user_id` は凍結され親と違いうる（review/deep.md R4 (1)）
- 反転条件: Q1 が「いまのまま」なら D1 の反転条件どおり並びを 2 つにする

### D5. 変えないもの

`docs/changes/CH02.md` の「壊してはいけないもの」は record の段が答えと一緒に書く（入力は `premises.md` の「record の段への入力」）。ここでは、それぞれを何が固定するかを並べる。

| もの | なぜ | 固定するもの |
|---|---|---|
| 写しは `READ ONLY` のトランザクションで、一時表を作らない | `CREATE TEMP TABLE` が落ちる（ST33 D5）。窓の結果は CTE（`MATERIALIZED`）に置く | tasks 3.4 |
| 次の記録は `core.event_live` から引く | 削除の印の行を間に挟まない（幅が長くなる側。ST33 D5・C21） | Scenario「削除の印の付いた記録は、次の記録にならない」 |
| 前の版の組の鍵は親から取る | `core.event_version` は `device_id` を持たず、`user_id` は凍結される | Scenario「前の版の組の利用者は親の利用者で読む」 |
| 窓は緯度経度で絞る前の `core.event_live` の並びで次を決める | ST33 D17 の件数の鍵の論拠「新しい記録の取り込みは幅を縮める側にだけ働く」を保つ（新しい記録は、どの行にとっても次の記録が手前に来るか変わらないだけ） | Scenario「緯度経度を持たない記録も、次の記録として幅の終わりになる」 |
| 除く範囲の式は `excluded.rs` の 1 か所 | 書き出しの本体・`GET /export/summary`・D17 が同じ断片を使う（ST33 D5）。呼び出し側は触らない | 既存の `export_summary_rows_equal_the_sum_of_counts`（件数と行の和の一致）と tasks 6.1 |
| 公開の形（`excluded_sql(snapshot_at)`・`ExcludedSql::ctes` / `event_kept` / `version_kept`・`EXCLUDED_EVENT` / `EXCLUDED_VERSION`） | 呼び出し側と D17 が名前で流す | コンパイル（呼び出し側を触らない） |
| 次の記録が無いときの幅の終わり（写しの時点 `snapshot_at`） | 呼び出し側が渡す式をそのまま使う | tasks 3.1 の族「写しの時点」 |
| (a)(b)(d) の読み方・緯度経度の判定・`SPAN_RULES` などの一覧 | Non-Goals。ST09 が一覧に足す行と重ならない | 既存の `export_excluded` の試験と tasks 3.1 |

### D6. 変わらないことは、いまの SQL を試験に残してまっさらな DB で突き合わせる（Q2 の推奨）

- いまの `overlap_sql` / `excluded_sql` を、試験のファイル（`export_window_tests.rs`）に**逐語で**写して残す（`legacy_excluded_sql`）。本体からは消す
- 突き合わせの相手は 2 つ:
  1. いまの SQL の記録の側の条件を `n.event_time > r.event_time` にした版（「厳密に後」の古い形）—— 新しい断片と**除く記録と前の版の id の集合が一致する**
  2. いまの SQL そのもの —— 新しい断片との差が、同じ時刻の組の名指しした id だけで、新しい断片の側が多い（除く側にだけ変わる。Q1）
- 走らせるのは `crate::stay_tests::fresh_db()` で作った**まっさらな DB**（全移行を当てる。`export_excluded_sql_every_registered_source_is_classified_by_name` と同じ手順）。
  共有の DB では他の試験の消した滞在が混ざり、いまの SQL 自身が 84 秒の型で遅い（review/deep.md R5）
- 偽データ（Q2 の推奨の列挙）: 同じ時刻の組・端末 NULL・前の版の時刻のずれ（0 と 5 秒）・前の版の `user_id` が親と違う・削除の印を間に挟む・
  緯度経度を持たない行や消した時間より後の行が次の記録になる配置・1 µs の差・消した滞在 0 件・
  写しの時点（`snapshot_at` に固定の時刻の式を渡し、消した滞在の始まりより前・消した時間の中・後の 3 通り。`now()` と `transaction_timestamp()` は同じトランザクションでは同じ値なので族にしない。review/spec.md R6）・
  ほかの前の版が次の記録の候補にならない配置・
  ソースの種類（点・終わり・長さ・次の記録まで）・(a) の読めた終わりが記録の時刻より前
- 同じ試験のファイルで、新しい断片を `BEGIN READ ONLY` の中で走らせる（一時表を使う実装がここで落ちる）

### D7（仮）. 線形に近いことは、読んだ行の数を N と 4N で比べ、窓の枠は断片の文字列で固定する（Q3 の推奨）

- まっさらな DB に、消した滞在 1 件（01:00〜02:00）と、その時間より前に始まる終わりを持たない緯度経度つきの `c02-window` を N = 1,000 と 4N = 4,000 置く。
  **端末は記録ごとに違う値にし、別の利用者の `c02-window` を消した時間より後に同じ数だけ置く**（組が N とともに増え、次の記録が遠い配置。いまの SQL が計画に依らず 2 乗になる。review/spec.md R2 の実測で比 16）。
  置いた後に `ANALYZE` を当て、`EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)` で除く範囲の判定（`SELECT (SELECT count(*) FROM excluded_event), (SELECT count(*) FROM excluded_version)`）を走らせる
- 量は、計画の木の `Relation Name` が `event` / `event_version` の節の `(Actual Rows + Rows Removed by Filter + Rows Removed by Index Recheck) × Actual Loops` の和（読んだ行の数）。
  4N の量が N の量の 8 倍未満であることを見る（spec）。**同じ試験で、D6 に残したいまの SQL にも当て 8 倍以上になることを assert する**（相関副問い合わせに戻すと落ちる）
- 消した滞在が古い配置（行の大半が `max(end_at)` より後）でも同じ比を見る（D3 の後半の `GROUP BY` が線形であることの確かめ）
- 消した滞在 0 件の DB で、`erased_spans` の CTE の下の節（消した滞在とその前の版を探す部分）を除き、`event` / `event_version` を読む節がどれも `Actual Loops` 0（か `Never Executed`）であることを見る（spec）
- **窓の枠の退行（順向きで尾を固定した枠）は読んだ行の数では見分けられない**（数え直しは `WindowAgg` の中で起き、表を読み直さない。review/spec.md R16 の実測: 読んだ行の数は同じで時間は 4 倍ごとに約 15 倍）。
  枠は、組んだ断片の文字列に D1 の枠（`ORDER BY event_time DESC RANGE BETWEEN UNBOUNDED PRECEDING AND '1 microsecond' PRECEDING`）があり、`FOLLOWING AND UNBOUNDED FOLLOWING` が無いことを assert する試験で固定する
- 時間は見ない（機械の混み具合で揺れる。Q3）

反転条件: `ANALYZE` 後でも読んだ行の数が計画の違いで揺れて 8 倍の線を越えたり割ったりするなら、指標を共有バッファの数（`Shared Hit Blocks + Shared Read Blocks`）に替える。

### D8（仮）. 時間は既存の量の試験で 1 回記録し、ST33 D13 の判定に渡す

- `export_file_tests.rs` の `export_volume_one_run_reports_time_and_memory` を、使い捨ての DB（`docker compose` の DB に別の名前の database を作り、終わったら落とす。ST33 tasks 5.6）で
  `ASHIATO_EXPORT_VOLUME_ROWS` = 10,000 / 100,000 / 1,200,000 の 3 通り、消した滞在あり・無しの 2 通りで走らせ、出る `seconds`（書き出し全体）と `summary_seconds`（件数の数え 1 回）を
  この design の「計測の記録」の表に書く。消した滞在の有無は環境変数 `ASHIATO_EXPORT_VOLUME_ERASED`（既定 `1`。`0` で消した滞在を置かない）で切り替える
- **件数の数えの時間が 3 秒以上か**（ST33 D13 の反転条件「3 秒以上かかる量になったら」と同じ境）を、判定として同じ表に書く。替えるかは本人が決める（この change では替えない）
- 使い捨ての DB は、`docker compose` の DB に `ch02_volume_<行数>_<0|1>` の database を作り、`DATABASE_OWNER_URL` の末尾の database 名だけをそれに差し替えて試験を走らせ、終わったら `DROP DATABASE … WITH (FORCE)` する
  （試験の DB の繋ぎ先は `testdb::url()` が `DATABASE_OWNER_URL` から読み、port だけを worktree のものに差し替える。移行は試験の初回に当たる）
- 反転条件: 120 万行の 1 本が手元で 30 分を超えて回らないなら、10 万行までの 2 点で傾き（行数 10 倍で時間が何倍か）を書き、120 万行は外す

### D9（仮）. `docs/changes/CH02.md` の `capability` は空のままにする

`data-durability` を書くと、走っている ST09 と衝突待ちになる。ST09 が触るのは別の Requirement（ADDED）と一覧の行だけで、CH02 の MODIFIED と重ならないことを確かめた（proposal の Impact・review/spec.md R12）。
失うものは無い（重なれば gate の rebase で追従する）。反転条件: CH02 が走っている間に、別の Story / change が「書き出しは本人が消した場面を出さない」を MODIFIED する上流に入るなら、
`capability: [data-durability]` を書いて admission に止めさせる。

### D10（仮）. ST09 の export の試験は、CH02 の下流の worktree で ST09 のブランチを一時に merge して走らせる

依頼は「既存の export の試験（ST33・ST09）をすべて通す」。ST09 の試験は走っている `feat/st09-media-capture` にあり、main にはまだ無い。CH02 の worktree の中で `git merge --no-commit` して走らせ、`git merge --abort` で戻す。push も commit もしない（tasks 6.2。素の worktree は `scripts/` の symlink と `.env` を持たず試験が走らない）。
ST09 のコードを CH02 のブランチに取り込まない（確認前の実装を運ばない）。反転条件: ST09 が CH02 の下流より先に main に入っていれば、rebase 後の通常の `CT st09_exp_` で見て、一時の merge は要らない。

## 計測の記録（下流が Task 5 で書く）

| 行数 | 消した滞在 | seconds | summary_seconds | D13（3 秒以上か） |
|---|---|---|---|---|
| 10,000 | あり | 1.6 | 0.2 | いいえ |
| 10,000 | 無し | 1.2 | 0.0 | いいえ |
| 100,000 | あり | 12.7 | 2.0 | いいえ |
| 100,000 | 無し | 8.7 | 0.1 | いいえ |
| 1,200,000 | あり | 146.5 | 25.5 | はい（3 秒以上） |
| 1,200,000 | 無し | 97.9 | 0.9 | いいえ |

6 通りとも 1 本ずつ使い捨ての DB（`ch02_volume_<行数>_<0|1>`、所有者は `ashiato_owner`）で走らせ、終わりに `DROP DATABASE … WITH (FORCE)` した。全部 rc=0。120 万行も 30 分に収まった（最長 146.5 秒）ので、D8 の反転条件は働かない。
行数 10 倍あたりの `seconds` は 10,000→100,000 で 約 7.9 倍（あり）/ 7.3 倍（無し）、100,000→1,200,000（12 倍）で 約 11.5 倍 / 11.3 倍で、ほぼ線形。`max_rss_kib` は 25〜34 MiB で行数にほぼ依らない。
D13 の判定: 件数の数えが 3 秒以上になったのは 120 万行・消した滞在ありの 1 通り（25.5 秒）だけ。ST33 D13 の件数の出し方を替えるかは本人が決める（この change では替えない）。

## Risks / Trade-offs

- [消した滞在が最近だと、窓が全行を並べ替える（`n log n` と一時ファイル）] → D2 で鍵の列だけを並べ、D8 で 120 万行の時間を測る。3 秒以上なら D3 の反転条件
- [窓の書き方を誤ると 2 乗に戻る（順向きで尾を固定した枠）] → D1 に名指しで書き、D7 の断片の文字列の試験で落とす（読んだ行の数では落ちない）
- [同じ時刻の組の扱いが変わり、前に出たファイルとの比較で除かれる行が増える] → 除く側にだけ変わる（D6 の突き合わせ 2 が差を名指しで固定）。前に出たファイルは戻らないが、それはどちらを選んでも同じ（Q1）
- [本物の写しに同じ時刻の組が何件あるかは数えていない（`premises.md` の確かさ 1）] → Q1 の判断は「漏れる形が 1 つでもある」ことに載せていて、件数には載せない
- [ST09 が同じ `excluded.rs` の一覧に行を足す] → 触る行が重ならない。gate の rebase で後から入る側が追従する

## Migration Plan

移行は無い。戻すときは `excluded.rs` を前の commit に戻すだけ（DB の形・台帳・ファイルの形に触らない）。
