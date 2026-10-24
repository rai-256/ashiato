# CH04 spec レビュー（spec --draft。第 1 回の推奨の答えで先に書いた版）

対象: `proposal.md`・`specs/test-database/spec.md`・`specs/data-durability/spec.md`・`design.md`・`tasks.md`・`docs/changes/CH04.md`。
入力: `deep.md`・`deep-questions.json`・`premises.md`・`review/deep.md`、CH02 の下流（`origin/feat/ch02-export-next-record-window` = aaf264a7）。

## 機械の検査（最初に写す）

```
$ openspec validate ch04-test-db-growth --strict
Change 'ch04-test-db-growth' is valid

$ python3 scripts/check_chain.py .
chain: OK (0 件 / 未回収 0 件 / warn 0 件)

$ python3 scripts/check_scenarios.py . ch04-test-db-growth
[FAIL] 担保の無い Scenario: … 23 件（test-database 20・data-durability 3）
scenarios: FAIL (担保なし 23 件 / 名無しの確認待ち 0 件)
  ← 実装前なので印が無いのは想定どおり。23 本とも tasks.md の Task が名指ししている（下の観点 2）。
  ほかの [warn]（collector-windows・archive_flow_tests・st12_delta_diff.py の印）は CH04 と関係しない

$ python3 scripts/review_triage.py . ch04-test-db-growth
指摘 10 件 / 仮決め 0 件 / 要件へ戻すもの 0 件
triage: OK
```

手で走らせた検査（重い `cargo test` は走らせていない）:
- 使い捨ての PostgreSQL 17（`docker run --rm postgres:17-alpine`。手元の DB と同じ版。終わった後に消した）で、D9 と CH02 の式を同じ結果になるか比べ、tasks 8.3 の測り方を確かめた（R1・R2・R3）
- `cargo test -- --list` の本数は、`crates/server/src` を読んで試験の名前を数える小さな台本で数えた。ch03 の木の組み済みの試験の実行ファイルの `--list`（1,203 本）と、全部の絞り込みで同じ数になることを先に確かめた（観点 4）

- loss: 17 件とも無い（`loss:` の行は、失うものがあるときだけ書く形（`review_triage.py`）に合わせて省いた）。`処置:` は呼び出し元が書く

---

## R1. Q2 の推奨の判定（N と M を一緒に倍にして時間が 2.5 倍以内）は、CH02 の式のままでも通る。直したかを見分けられない
- 成果物: openspec/changes/ch04-test-db-growth/deep-questions.json（Q2 の推奨の detail）・specs/data-durability/spec.md:19-22・design.md D14・tasks.md 9.2・docs/changes/CH04.md:77-78
- 根拠:
  - 量の試験の行は `c02-window`（緯度経度を持たないソース）で、緯度経度を持つのは `g % 100 = 0` の 1% だけ（`git show origin/feat/ch02-export-next-record-window:crates/server/src/export_file_tests.rs` の 1464-1474 行）。`overlap_sql` は緯度経度で候補を絞るので（`{coord}`）、入れ子に入る候補の行は N の 1%。D14 は行の置き方を M 日に分けるだけで、この割合を変えない
  - 入れ子の費用だけを使い捨ての PG 17 で測った（候補の行と消した時間だけの小さな表で、CH02 と同じ `WHERE EXISTS (… s.start_at <= w.width_end AND s.end_at >= w.event_time)`。D14 と同じ置き方で、1 日に N/M 行、消した時間は各日の 01:00〜02:00。候補の行は**全部**の行）:
    (30 万, 25) で +0.65 秒・(60 万, 50) で +2.0 秒・(120 万, 100) で +7.3 秒
  - CH02 の実測の線形の部分（120 万行で書き出し 146.5 秒・件数の数え 25.5 秒。行数に比例させて縮める）に足すと、隣り合う組の比は、書き出しで 2.02・2.04、件数の数えで 2.11・2.22。
    **候補の行が全部の行でも 2.5 倍を下回る**。量の試験と同じに候補が 1% なら、入れ子の費用はさらに約 100 分の 1 になり、比はほぼ 2.0 になる
  - つまり Scenario「記録の行数と消した滞在の数を一緒に倍にしても、時間は 2.5 倍以内に収まる」と tasks 9.2 は、D9 を当てる前（Task 8.4 の前）にもう通る。CH04.md の完了の判定の 2 つ目の項目も同じ
  - Q2 の context は「N だけ・M だけを倍にしても 2 倍にしか見えないので、判定は N と M を一緒に伸ばして見る」と書くが、一緒に伸ばしても、この量では入れ子の 2 乗の項が線形の項に埋もれる
- kind: premise
- 提案: 本人に渡す前に Q2 の推奨の detail を直す。直したかの判定は、読んだ行の数（R2 の測り方。CH02 の Q3 と同じ理由で、時間は混み具合で揺れる）に置き、時間の 3 点は「測って渡す数字」に下げる。
  時間でも見分けたいなら、量の試験の緯度経度つきの割合を上げた形で走らせ、D9 の前と後の両方の値を design の「計測の記録」に残す（前の値で 2.5 倍を超えなければ、判定にならないことがその場で分かる）。
  あわせて Q2 の context に、入れ子の大きさ（120 万行・M = 100 で、件数の数え 25.5 秒に対して入れ子は数秒かそれより小さい）を書く。本人が「この change で直す」を選ぶ材料になる
- 処置: escalated —— Q2 はまだ本人に渡っていない第 1 回の問いなので、推奨の判定を「N と M を一緒に 4 倍にして消した時間を読む量が 8 倍未満（直す前の式では 8 倍以上）」に替え、時間は測って残すだけにし、入れ子の大きさ（+0.65・+2.0・+7.3 秒）を context に足して、本人の Q2 の答えに委ねる（deep.md の Q2 の注に R1）。specs/data-durability の 3 本目・design D14・tasks 9.2・CH04.md の完了の判定もこの推奨に合わせて直した

## R2. tasks 8.3 の「読んだ行の数」（`Actual Rows × Actual Loops`）では、D9 の形の中に入れ子が残っても見逃す
- 成果物: openspec/changes/ch04-test-db-growth/tasks.md 8.3（spec の data-durability:11-13 の Scenario の担保）
- 根拠: 使い捨ての PG 17 で、`erased_spans` の CTE Scan の節ごとに `Actual Rows × Actual Loops` の和と、`(Actual Rows + Rows Removed by Filter) × Actual Loops` の和を、(N, M) = (1,000, 10) と (4,000, 40) で取った:
  - CH02 の式（最後が `WHERE EXISTS`）: 6,011 → 80,041（13.3 倍）。EXISTS は Nested Loop Semi Join になり、捨てる行は親の節の `Rows Removed by Join Filter` に出て、CTE Scan の行の数には読んだ行が全部入る
  - 入れ子を OR の中に残した誤りの形（`… OR EXISTS (SELECT 1 FROM erased_spans s WHERE …)`。D9 は 1〜3 を OR で束ねるので、この形になる）: `Actual Rows × Loops` は 1,011 → 4,041（**4.0 倍。8 倍未満で通る**）、`(Actual Rows + Rows Removed by Filter) × Loops` は 5,011 → 84,041（16.8 倍）。EXISTS は SubPlan になり、条件は CTE Scan の Filter に入るので、1 回あたりの Actual Rows は 0 か 1 で、読んで捨てた行は `Rows Removed by Filter` にしか出ない
  - D9 の 2 の「整っていない消した時間」の CTE は `erased_spans` と別の名前になり、8.3 の「`CTE Name` が `erased_spans`」の和に入らない
  - CH02 は同じ問題を避けていた: `export_window_tests.rs:894-918`（CH02 のブランチ）の `rows_read` は `(Actual Rows + Rows Removed by Filter + Rows Removed by Index Recheck) × Actual Loops`
- kind: technical
- 提案: 8.3 の測り方を、CH02 の `rows_read` と同じ式にする。消した時間を読む節（`erased_spans` と、D9 がそこから作る CTE の全部）の CTE Scan で取る（Semi Join の形では CTE Scan の Actual Rows が読んだ行の全部で、SubPlan の形では捨てた行が Rows Removed by Filter に入るので、この式でどちらの形も数えられる。親の Join Filter の捨てた行は同じ行なので足さない）。
  8.3 の材料には、消した時間に重ならない緯度経度つきの記録も混ぜる（重なる行は最初の一致で止まるので、重ならない行が入れ子の最悪の形になる）。CH02 の式に当てて 8 倍以上になる assert は残す（測り方が入れ子を捕まえていることの確かめになる）
- 処置: fixed 8.3 —— 読んだ量を CH02 の `rows_read` と同じ `(Actual Rows + Rows Removed by Filter) × Actual Loops` にし、D9 がそこから作る CTE の全部の CTE Scan を数え、重ならない緯度経度つきの記録を半々に混ぜた。spec の 1 本目の THEN にも同じ数え方と「足す前の判定では 8 倍以上」を書いた

## R3. D9 の重なりの判定は CH02 の EXISTS と同じ結果を返す。ただし `tstzrange` を作るのは CASE の中でなければならない（AND は評価の順を約束しない）
- 成果物: openspec/changes/ch04-test-db-growth/design.md:133-149（D9）・tasks.md 8.4
- 根拠:
  - 同じになることの確かめ: PG 17 で、消した時間 12 件（端の 5% を NULL、終わりが始まりより前のものと端どうしが触れるものを含む。分刻みの格子に置いた）× 幅 400 行（終わりが始まりより前のものを含む）を乱数で作り、CH02 の `EXISTS (… s.start_at <= w.width_end AND s.end_at >= w.event_time)` と、D9 の 3 つの和（1 は `range_agg(tstzrange(…,'[]'))` の multirange に `&&`、2 は `start_at > end_at` の CTE に今の EXISTS、3 は `width_end < event_time` の行に今の EXISTS）を比べた。6 回とも差は 0 件（片方にだけある id がどちらの向きにも無い）
    - 閉じた区間: `tstzrange(t,t,'[]')` は空でなく 1 点で、端が触れるだけで `&&` は真になる。今の `<=` / `>=` と同じ
    - NULL の端: `range_agg(tstzrange(NULL, x, '[]'))` は `{(,x]}`（下の端が無い）になる。D9 が NULL を 1 と 2 の両方から外すのは正しい
    - 幅の終わりは NULL にならない（`coalesce(a.t, b.t, c.t)` の c は最後に `{snapshot_at}`）。NULL でも、1 の `<=` と 3 の `<` のどちらにも入らず、今の式と同じに除かない
  - 危ないところ: `SELECT tstzrange(now(), now() - interval '1s', '[]')` は `ERROR: range lower bound must be less than or equal to range upper bound` になる。D9 の 1 を `w.event_time <= w.width_end AND multirange && tstzrange(w.event_time, w.width_end, '[]')` と書くと、PostgreSQL は AND の評価の順を約束しない（文書の「Expression Evaluation Rules」。順を固定するのは CASE だけ）。手元では計画が安い方の比較を先に置いたので落ちなかったが、それは計画の都合で、約束ではない
  - D11 の族は整っていない幅を含むので、落ちれば試験がエラーで気づく（黙って除く記録が変わることは無い）。ただし本物の書き出しでは、写しの時点より後の時刻の記録があると、書き出しの全体がエラーで止まる
- kind: technical
- 提案: D9 の 1 に「範囲を作るのは `CASE WHEN w.event_time <= w.width_end THEN <multirange> && tstzrange(w.event_time, w.width_end, '[]') END` の中だけ（AND で守らない）」と書く。消した時間の側は CTE の WHERE で先に絞る（集約の前に WHERE が効くので、こちらは今のままでよい）
- 処置: fixed D9 —— 幅の側の範囲は `CASE WHEN w.event_time <= w.width_end THEN … END` の中でだけ作ると書き、乱数の族で差 0 件の確かめを根拠に足した。tasks 8.4 にも書いた

## R4. D11 の「CH02 の式を新しいファイルに逐語で写す」は、そのままではコンパイルできない。st41 の変更とも食い違っていく
- 成果物: openspec/changes/ch04-test-db-growth/design.md:155-161（D11）・tasks.md 8.2
- 根拠:
  - `crates/server/src/export.rs:24` は `mod excluded;`（私的）で、外に出るのは `pub use excluded::*`（`pub` の項目）だけ。CH02 の `overlap_sql`・`end_key_sql`・`duration_key_sql` は私的な `fn`、`ExcludedSql` の欄 `ctes` も私的（CH02 のブランチの excluded.rs:151-153・260・307・321）。`crate::export_overlap_tests` から逐語で写した `ExcludedSql { ctes }` と `end_key_sql(k)` は名前が見えず、コンパイルが落ちる
  - CH02 は同じことを `excluded.rs` の中の `#[cfg(test)] pub(crate) mod legacy`（`use super::*`）で解いている（CH02 のブランチの excluded.rs:374-486）。前の版の式の写しは、そこに既にある
  - 走っている st41 は、まさに写す範囲を変える: `erased_spans` の前の版の終わりに `v.end_time` を足し、`overlap_sql` の (a) の先頭に列の `end_time` を置く（`git diff $(git merge-base origin/main feat/st41-event-end-time) feat/st41-event-end-time -- crates/server/src/export/excluded.rs`）。写しは別のファイルなので rebase で衝突せず、黙って st41 の前の形のまま残る。
    族に `end_time` の列を持つ記録が無ければ突き合わせは通るが、「今の本体と、最後の条件だけが違う式」を比べることにならなくなる
- kind: technical
- 提案: 写しを丸ごと持たず、CH02 の `excluded_sql_strict` と同じ形にする。今の `excluded_sql(…).ctes()` の文字列から D9 の最後の条件を 1 か所だけ CH02 の `WHERE EXISTS (…)` に置き換えたものを比べる相手にし、置き換えた数が 1 であることを assert する。
  これなら私的な関数に触らず、st41・st09 が後から入っても、比べる 2 つは最後の条件だけが違う。丸ごと写すなら、置き場を `excluded.rs` の `#[cfg(test)]` の中にする
- 処置: fixed D11 —— 丸ごと写すのをやめ、最後の重なりの条件を定数に括り出して CH02 の形に 1 か所ずつ（計 2）置き換えた相手と比べる形にした（tasks 8.2）

## R5. 「終わりを持たない消した時間」の意味が spec と design で違う。終わりが NULL の消した時間は、今の view からは作れない
- 成果物: openspec/changes/ch04-test-db-growth/specs/data-durability/spec.md:8・16 / design.md:143・159
- 根拠: `core.stay_erased` は `end_at = coalesce(core.try_timestamptz(payload->>'end'), event_time + interval '1 microsecond')`（migrations/202609271716_deletion_ledger.sql:60）、`start_at = event_time`。`erased_spans` の前の版の側も `coalesce(…, v.event_time)`（CH02 のブランチの excluded.rs:204）。どちらの枝でも、消した時間の端が NULL になることは無い。
  design D11 は族を「終わりが NULL の消した時間（`core.stay_erased` で作れなければ、作れないことを試験の注記に書いて族から外す）」と読み、外す道を用意している。spec の Scenario の WHEN は族を条件なしで並べているので、下流が「作れない」として外すと、spec の言う材料を欠いたまま緑になる
- kind: technical
- 提案: spec の「終わりを持たない消した時間」を「内容に `end` を持たない消した滞在（終わりは始まりの 1 µs 後として読む）と、`end` を持たない前の版（終わりは前の版の時刻）」に書き換え、作れる材料にする。D11 の「族から外す」の道は消す。D9 の NULL の端の箇条は「今の view からは来ないが、来ても今の式と同じ」の注記に下げる
- 処置: fixed D11 —— spec の族を「内容に `end` を持たない消した滞在（1 µs 後）と前の版（前の版の時刻）」に書き換え、D11 の「族から外す」道を消した。D9 の NULL の端は「今の view からは来ない」の注記に下げた

## R6. statement_timeout の Scenario は環境変数で上限を替える形を言うが、tasks の試験は環境変数を通らない。「環境変数で替える」「0 で上限なし」は、どの Scenario も見ない
- 成果物: openspec/changes/ch04-test-db-growth/specs/test-database/spec.md:89・93 / tasks.md 4.2
- 根拠: Scenario「上限を超える問い合わせは、止まって失敗する」の WHEN は「`ASHIATO_TEST_STATEMENT_TIMEOUT` を `1s` にした接続」。tasks 4.2 は「環境変数を試験の中で書き換えない（並走する試験に効く）。上限を引数で受ける内側の関数を足場に置き」で、引数で 1 秒を渡す。
  環境変数を読んで値にする部分（`0` を上限なしにする部分も）を通る試験は無い。Requirement の本文（89 行）が言う「環境変数で替えられる（`0` で上限なし）」は Scenario に無い。tasks 9.2 の `export-scaling.sh` は `0` を渡すので、`0` を誤って読むと量の測りが 60 秒で落ちるまで分からない
- kind: technical
- 提案: Scenario の WHEN を「上限を 1 秒にした試験の足場の接続」に直す。環境変数の値を読む純粋な関数（未設定は 60 秒・`1s` はそのまま・`0` は上限なし）を足場に置き、その関数の Scenario を 1 本足す（環境変数を書き換えずに試せる）
- 処置: fixed 4.2 —— Scenario の WHEN を「上限を 1 秒にした足場の接続」にし、環境変数の値を読む純粋な関数 `statement_timeout_from` の Scenario を 1 本足した（tasks 4.1・4.2、design D7）

## R7. 上限の既定の値（60 秒）が spec に無く、「design D7（仮）の値」を指している
- 成果物: openspec/changes/ch04-test-db-growth/specs/test-database/spec.md:89・98
- 根拠: Requirement は「上限の既定は design D7（仮）の値とし」、Scenario「上限の既定は、試験の足場の接続に入っている」の THEN は「design D7（仮）の既定の値である」。値の `1min` は tasks 4.2 だけにある。
  `openspec archive` は正典に specs だけを写すので、正典の Scenario が archive した design を指すことになり、値を変えても正典の Scenario は落ちない（観点 1 の数値の規則）。CH04.md:75 は「既定 60 秒」と書いている
- kind: technical
- 提案: Requirement と Scenario に「60 秒」（`SHOW statement_timeout` が `1min`）を書く。仮であることは design D7 の反転条件が持つ（値が変われば spec も直す）
- 処置: fixed 4.2 —— Requirement と Scenario に 60 秒（`1min`）を書いた。仮であることは design D7 の反転条件が持つ

## R8. tasks 5.1 の検証（書き出しの試験のファイルに `testdb::pool()` が無い）は、5.2 で自分が足す試験で落ちる
- 成果物: openspec/changes/ch04-test-db-growth/tasks.md:85・86-88
- 根拠: 5.1 の検証は `! grep -nE "testdb::pool\(\)" … crates/server/src/export_excluded_tests.rs` rc=0。5.2 は同じ `export_excluded_tests.rs` に `ch04_export_isolated_does_not_see_other_tests` を足し、その試験は「回の database（`testdb::pool()`）に消した滞在と…を置いた後」に `isolated()` の database を数える。
  5.2 の後に 5.1 の検証を走らせ直すと（全体の検証・`scripts/verify-run` のやり直し）rc=1 になる
- kind: conflict
- 提案: 5.2 の試験を `testdb_tests.rs` に置く（回の database と試験ごとの database の両方を見る、足場の試験なので）。または 5.1 の grep を `db-ok:` の印と同じ規則で 1 行だけ通す形にする（R9 の検査に寄せる）
- 処置: fixed D4 仮 —— 5.2 の試験を足場の試験 `testdb_tests.rs` に移し、5.1 の grep を `export*_tests.rs` 全体に広げた

## R9. 「書き出しの試験は試験ごとの database を使う」を守らせる検査が無い。走っている st29 は、書き出しの試験のファイルに新しく `testdb::pool()` を 3 か所足している
- 成果物: openspec/changes/ch04-test-db-growth/specs/test-database/spec.md:31（Requirement の本文）/ tasks.md 5.1 / proposal.md:70
- 根拠:
  - Requirement の本文は「書き出しの試験（`export_*tests.rs` の試験）に…試験ごとの database を渡し」。3 本の Scenario は、試験ごとの database が空であることと消えることを見るが、書き出しの試験がそれを**使っている**ことはどれも見ない（観点 2: 本文だけが言っている）。
    守らせるのは tasks 5.1 の grep だけで、5 つのファイルの名前を固定して CH04 の下流で 1 回走るだけ（CI では走らない）
  - `git diff $(git merge-base origin/main feat/st29-satellite-credentials) feat/st29-satellite-credentials -- 'crates/server/src/export*_tests.rs' | grep -c '^+.*testdb::pool()'` → 3（`export_file_tests.rs`）。st29 が CH04 の後に main に入ると、その 3 本は回の database で書き出しを走らせ、同じ回の他の試験の消した滞在を判定する。どの検査も落ちない
  - proposal の Impact（70 行）は「st23・st29・st41 の新しい試験: `testdb::pool()` を使う限り、そのまま回の database に載る」と書くが、書き出しの試験ではそれが CH04 の Requirement に反する
  - st41 が足す `export_end_time_tests.rs` は `export_excluded_tests::World` を使うので、World が `TestDb` を持てば試験ごとの database に載る（こちらは問題ない）。ただし 5.1 の grep の一覧には入っていない
- kind: technical
- 提案: `tools/check-test-db-growth.sh --lint` に「`crates/server/src/export*_tests.rs` に `testdb::pool()` を書かない（`db-ok: <理由>` で通す）」を足し、Scenario を 1 本足す（WHEN 書き出しの試験のファイルに `testdb::pool()` がある木で `--lint` → THEN rc=1・ファイルと行・`testdb::isolated()` を示す）。CI の `--lint` の行がそのまま後から入る Story を止める。proposal の Impact の st29 の行を直す
- 処置: fixed 6.2 —— `--lint` に「書き出しの試験のファイルの `testdb::pool()`」を足し（`db-ok:` で通す）、Scenario を 1 本足した。proposal の Impact の st29 の行と design D5 を直した

## R10. 走っている変更との重なりの書き方に漏れがある。CH03 は 5.1 が書き換える書き出しの試験の 3 ファイルを大きく変えている
- 成果物: openspec/changes/ch04-test-db-growth/proposal.md:63-70
- 根拠: `git diff --stat $(git merge-base origin/main feat/ch03-test-tmp-cleanup) feat/ch03-test-tmp-cleanup` → `export_file_tests.rs | 103`・`export_job_tests.rs | 108`・`export_tests.rs | 71`。中身は `let dir = tmp_dir(…)` を `let dir_tmp = tmp_dir(…); let dir = dir_tmp.path()…` に替えるもので、5.1 が `testdb::pool()` を書き換える `let pool = testdb::pool().await;` の隣の行になる（例 export_tests.rs の `export_dir_writes_under_the_chosen_dir`）。
  Impact は CH03 について ci.yml だけを書く。ほかに、書き出しの試験のファイルを触る走っている枝: st09（export_file_tests 12・export_ledger_tests 6）、st13（export_ledger_tests 6）、st23（export_ledger_tests 6・export_tests 3）、st29（export_file_tests 122）、st41（export_excluded_tests 4・export_file_tests 4・export_ledger_tests 6・新しい export_end_time_tests.rs 288）。
  testdb.rs（st09・st13）・excluded.rs（CH02・st09・st41）・ci.yml（CH03 は tasks 8.1 で予定。まだ commit は無い）・server_startup.rs（st13）の書き方は合っている。st09・st13 の testdb.rs の足し分は `pool` を受け取る関数だけで、CH04 の形と衝突しないことも確かめた
- kind: conflict
- 提案: Impact に「書き出しの試験のファイル（`export_*tests.rs`）: CH03・st09・st13・st23・st29・st41 が触る。5.1 は各試験の `testdb::pool()` の行を書き換えるので、後から入る側が rebase で衝突を解く。CH03 と CH04 の後に入る側の新しい書き出しの試験は、R9 の検査が止める」を足す
- 処置: fixed D5 仮 —— proposal の Impact に書き出しの試験のファイルを触る CH03・st09・st13・st23・st29・st41 を並べ、後から入る側の新しい書き出しの試験は `--lint` が止めると書いた

## R11. review/deep.md R7 の処置の記録が、書き直した Q2 と食い違っている
- 成果物: openspec/changes/ch04-test-db-growth/review/deep.md:62-67
- 根拠: R7 の処置は「Q2 の判定を『N と M を一緒に倍にして 2.5 倍以内』と『120 万行・M=100 で件数の数えが 3 秒（ST33 の反転条件）以内』に替えた」。今の Q2 の推奨は「3 秒は合否にしない」（deep-questions.json Q2 options[0]、deep.md:13）。
  spec の段で CH02 に合わせて Q2 を書き直したとき（premises.md P14）、deep-review の処置の記録はそのまま残った。triage は形だけを見るので OK を返す。書き直した Q2 は deep-review の席を通っていない
- kind: conflict
- 提案: R7 の処置に「spec の段（2026-10-24）で CH02 を受けて Q2 を書き直し、3 秒の判定は外した（P14）」を書き足す。書き直した Q2（R1 の直しも入れるなら、その後の版）を本人に渡す前に、deep-review の席にもう一度通すかを呼び出し元が決める
- 処置: fixed D14 仮 —— review/deep.md の R7 の処置に、spec の段で CH02 を受けて 3 秒を外し、R1 を受けて 2.5 倍も外したことを追記した。書き直した Q2 は spec-review（この R1・R12）を通したので、deep-review にはもう一度通さない（PR 本文に書く）

## R12. 設計の Context が、CH02 の 25.5 秒を CH04 が取る入れ子の費用のように読める
- 成果物: openspec/changes/ch04-test-db-growth/design.md:15-16
- 根拠: 「CH02 の実測で 120 万行・消した滞在 1 件の件数の数えが 25.5 秒。CH04 が取るのは、この最後の入れ子」。消した滞在は 1 件（M = 1）なので、25.5 秒の中の入れ子は N × 1 で、ほとんどは窓の並べ替えと緯度経度の判定（Q2 の context 自身がそう書いている）。D9 を当てても 25.5 秒はほぼ変わらない。
  premises.md P14 の「行数 10 倍で時間 7〜11.5 倍」の 11.5 倍は 12 倍の行数のとき（CH02 の design の「計測の記録」）。数字そのものは CH02 の表（146.5・25.5・97.9・0.9 秒）と合っている
- kind: premise
- 提案: Context を「25.5 秒の大半は窓と緯度経度の判定で、CH04 の直しでは減らない。CH04 が取るのは、M が年数とともに増えたときに乗る N × M の項」に直す。P14 の倍率は「行数 10 倍で 7.3〜7.9 倍、12 倍で 11.3〜11.5 倍」と書く
- 処置: escalated —— design の Context を「25.5 秒の大半は窓と緯度経度の判定で CH04 では減らない。CH04 が取るのは M とともに乗る N × M の項」に直し、premises.md P14 の倍率を「10 倍で 7.3〜7.9 倍、12 倍で 11.3〜11.5 倍」に直した。Q2 の context にも同じことを足し、本人の Q2 の答えに委ねる（deep.md の Q2 の注に R12）

## R13. spec に実装の名前が多い（関数名・ファイル名・環境変数）
- 成果物: openspec/changes/ch04-test-db-growth/specs/test-database/spec.md（`testdb::pool()`・`app_pool()`・`url()`・`export_*tests.rs`・`current_database()`・`pg_sleep`）/ specs/data-durability/spec.md:5・20（`export/excluded.rs`・`tools/export-scaling.sh`）
- 根拠: 観点 3。`test-database` は試験を書く人のための道具の約束なので、足場の関数名・台本の名前・環境変数は外から見える口に当たり、CH03 の `test-hygiene` も同じ書き方をしている（先例あり）。
  `data-durability` は製品の正典で、ほかの Requirement はファイル名を書いていない（`openspec/specs/data-durability/spec.md` に `.rs` は 0 件）。spec:8 の「CH02 の後の判定と 1 件も変えない」も、archive の後の正典では、どの式かを正典の中で辿れない
- kind: technical
- 提案: `test-database` はこのままでよい。`data-durability` の本文からは `export/excluded.rs` と `tools/export-scaling.sh` を外し（Scenario の WHEN は「量を変えて書き出しと件数の数えを測る」にする）、「CH02 の後の判定」は「この Requirement を足す前の判定」と書く
- 処置: fixed specs/data-durability/spec.md —— data-durability の本文と Scenario から `export/excluded.rs`・`tools/export-scaling.sh` の名前を外し、「CH02 の後の判定」を「この Requirement を足す前の判定」に書き換えた。test-database はそのまま

## R14. Requirement の本文だけが言っていることがある（`ashiato` を使い続ける側）
- 成果物: openspec/changes/ch04-test-db-growth/specs/test-database/spec.md:11
- 根拠: 「`tests/server_startup.rs` が実行ファイルを起こして `ashiato` に移行を当てることと、縦串・開発用の起動が `ashiato` を使うことは変えない」は CH04.md の「壊してはいけないもの」（63 行）にもあるが、どの Scenario も見ない。片付け（D6）が `ashiato` を消さないことは Scenario「回の印の無い使い捨ての database の残りは消え、ashiato と他の database は残る」と台本の `ashiato` の有無が見るので、残るのは繋ぎ先だけ
- kind: technical
- 提案: CH04 はこれらのファイルに触らないので、Scenario を足すより、本文から外して proposal の「振る舞いを変えないもの」にだけ残す方が軽い。残すなら tasks 10.2 と同じ形で「`tests/server_startup.rs`・`tools/smoke.sh`・`tools/dev.sh` に差分が無い」を検証に足す
- 処置: fixed 10.3 —— Requirement の本文から外し（proposal の「振る舞いを変えないもの」に残る）、tasks 10.3 に `server_startup.rs`・`smoke.sh`・`dev.sh` に差分が無いことの検証を足し、CH04.md の archive の判定に入れた

## R15. D4 に「`TestDb` の Drop は panic しない」が無い。panic の巻き戻しの途中で Drop が panic すると、試験の実行ファイルごと止まる
- 成果物: openspec/changes/ch04-test-db-growth/design.md:78-87（D4）・tasks.md 2.1
- 根拠: D4 の Drop は「`std::thread` で立てた自分の runtime で `postgres` database に繋ぎ、`DROP DATABASE <名前> WITH (FORCE)` を打って、そのスレッドを待つ」。Scenario「試験ごとの database は、試験が panic しても消える」のとおり、この Drop は試験の panic の巻き戻しの途中でも走る。
  そこで Drop が panic すると（DB に繋げない・DROP が落ちたのを `unwrap` する・スレッドの join の結果を `unwrap` する）、Rust は巻き戻し中の 2 度目の panic で過程を abort する。同じ実行ファイルの全部の試験が、どの試験の結果も出さずに止まる。いまの `fresh_db()` は待った future の中で `unwrap` しているが、Drop の中ではない（clock_tests.rs:51-55）
- kind: technical
- 提案: D4 と tasks 2.1 に「Drop は panic しない。DROP が落ちたら名前と理由を `eprintln!` で出して戻る（残りは次の回の片付けが消す）」を書く。`std::thread::panicking()` のときだけでなく、いつでも panic しない形にする
- 処置: fixed D4 —— 「Drop は panic しない（いつでも）。名前と理由を `eprintln!` で出して戻る」を D4 と tasks 2.1 に書いた

## R16. 片付けの試験 3 本は同じ過程で並んで走り、互いの `sweep()` が作りかけの database を消しうる
- 成果物: openspec/changes/ch04-test-db-growth/tasks.md 3.2・design.md D6
- 根拠: `ch04_sweep_drops_an_orphan_run` は誰もロックを持たない回の id で `_tpl`・本体・`_x_…`・`_tmp_…` を作る。`ch04_sweep_drops_legacy_tmp_and_keeps_others` と `ch04_sweep_keeps_a_running_run_without_connections` も `sweep()` を呼ぶ。`cargo test` は既定で試験を並べて走らせるので、4 つを作っている途中にほかの試験の `sweep()` が持ち主のいない `_tpl` を消せる。
  本体や `_x_` を `TEMPLATE <id>_tpl` で写す書き方だと、その `CREATE DATABASE` が「template が無い」で落ち、`unwrap` で試験が落ちる（並びによって落ちたり通ったりする）。assert（4 つとも無い）は外れないが、作る段で落ちる
- kind: technical
- 提案: tasks 3.2 に「持ち主のいない回の 4 つは、TEMPLATE を使わない空の database として作る」と書く。または 3 本の片付けの試験を 1 つの `tokio::sync::Mutex` で並ばないようにする（`EXPORT_SERIAL` と同じ形）
- 処置: fixed 3.2 —— 材料を TEMPLATE を使わない空の database にし、3 本を `SWEEP_SERIAL` の Mutex で並ばせないと書いた。design の Risks にも 1 行足した

## R17. CH04.md の完了の判定の書き出しの項目は、直す前から通る。archive の判定に試験の本数の確かめが無い
- 成果物: docs/changes/CH04.md:77-81
- 根拠: 77-78 行の 2 つ目の判定（`tools/export-scaling.sh` の隣り合う組の比が 2.5 以下）は、R1 のとおり CH02 の式のままでも通る。直したことを見分けるのは 1 つ目の判定（tasks 8.3 の読んだ行の数）だけで、その測り方にも R2 の穴がある。
  81 行の archive の判定は `openspec validate` と `check_scenarios.py` だけで、「壊してはいけないもの」の 1 つ目（試験の本数・印を減らさない。tasks 10.2）が入っていない。`check_scenarios.py` は印があることを見るが、試験が通ったかも本数も見ない
- kind: technical
- 提案: 書き出しの判定は「直した式で、記録と消した滞在を一緒に 4 倍にして読んだ行の数が 8 倍未満、CH02 の式では 8 倍以上（tasks 8.3。R2 の測り方）」を合否にし、3 点の時間は「値が design の計測の記録にある」に下げる（R1 の直しと合わせる）。archive の判定に tasks 10.2 の検証を足す
- 処置: fixed ../../../docs/changes/CH04.md —— 完了の判定の書き出しの項目を、読んだ行の数（直した式で 8 倍未満・直す前で 8 倍以上）を合否にし、時間は直す前と後の値が残っていることに下げた。archive の判定に tasks 10.2・10.3 を足した

---

## 観点ごとのまとめ

- **観点 1（deep の決定が写っているか）**: 本人の答えはまだ無い（第 1 回の推奨で先に書いた版）。推奨の Q1〜Q4 は specs・design・tasks・CH04.md に写っている
  （Q1: 回ごと＋書き出しは試験ごと / Q2: N × M を直す・2.5 倍・3 秒は合否にしない・120 万行を測る / Q3: 台本＋statement_timeout / Q4: Non-Goals）。Q2 の数値（(30 万, 25)…・2.5 倍）は Scenario にある。
  決定の中の数値で Scenario に無いのは statement_timeout の既定（R7）。決定そのものの見分けの力は R1。当初案を覆したもの・要件へ戻すものは無い
- **観点 2（Scenario が検証できるか）**: 23 本とも tasks.md の Task が名指ししている（spec の `#### Scenario:` の名前で tasks.md を一字一句 grep。23 本とも 1 回以上）。
  見つけたもの: R2（測り方の見落とし）・R5（作れない材料）・R6（WHEN と試験の形の違い）・R9・R14（本文だけ）。
  「上限を超える…」の THEN は SQLSTATE `57014` を tasks が持っていて検証できる。data-durability の 3 本目は時間の比と出力に残すことを AND で束ねている（R1 で判定を読んだ行の数に移すなら、そのとき 1 つにする）
- **観点 3（置き場）**: proposal の Capabilities（`test-database` 新・`data-durability` 改）と `specs/` のディレクトリは合っている。change は INDEX の capability の表に載らない（`story_deps.py:78`）。
  design の D 番号で観察できる振る舞いを言うものは、D6（何を消し何を残すか）・D7（既定の値）・D8（台本の判定の順）で、D6 と D8 は spec の Scenario に入っている。D7 の値は R7。実装の名前は R13
- **design D1〜D8 の前提（確かめた範囲）**: D1: 助言ロックは database ごとに分かれるので、持つ側も片付けの側も `postgres` database で取る、で正しい。`pg_advisory_lock(int4, int4)` と `pg_advisory_lock(bigint)` は鍵の空間が別で、`4820253, hashtext(…)` と `4820252` は当たらない。
  D2・D5・D6: compose の DB は `postgres:17-alpine`（`docker-compose.yml:3`）で、`DROP DATABASE … WITH (FORCE)`・`range_agg`・multirange の `&&`・`CREATE DATABASE … TEMPLATE` は使える。テンプレートに残った接続は、`CREATE DATABASE … TEMPLATE` が数秒待って断る（閉じた直後なら待つ間に消える）。所有者は `CREATEDB` を持つ（P4）。
  D3: `tokio::sync::OnceCell` は試験ごとの runtime をまたいで待てる（いまの `MIGRATED` も同じ形。testdb.rs:62）。`testdb::url()` を呼ぶのは今の木では `fresh_db()` の 5 か所だけで、どれも `pool()` の後（D5 で置き換わる）。`app_pool()` は `DATABASE_URL` の database 名も替える必要がある（testdb.rs:110。D3 の「回の database に繋ぐ」に含まれる）。
  D7: `SHOW statement_timeout` は 60 秒を `1min` と出す（tasks 4.2 と合う）。
  D8: `tools/db.sh` は `COMPOSE_PROJECT_NAME` を変えない（tools/ports.sh は port だけを export）ので、CI の `docker compose up` で立てた容器にも `tools/db.sh exec -T db psql -U ashiato` で届く。
  成り立たない前提として見つけたのは D4 の Drop（R15）と D6 の片付けの試験の並び（R16）。D9・D11 は R3・R4
- **観点 4（tasks の検証）**: 全部のタスクにコマンドと rc がある。`cargo test` の本数の下限は今の木で成り立つ:
  `NL export_ 123`（今 129。ch03 の木の組み済みの実行ファイルでは 130）・`NL stay_tests 65`（ちょうど 65）・`NL places_tests 158`（ちょうど 158）・`NL app_role 3`（ちょうど 3）。ちょうどのものに余りは無いが、C1（本数を減らさない）の下では減らない。
  `CT export_file`（44 本）・`CT export_summary`（4 本）・`export_volume_one_run_reports_time_and_memory`（1 本）も 1 本以上ある。`story_deps.py included` の副命令はある（8.1）。
  依存の順の問題は R8（5.1 と 5.2）。8.3 は 8.4 の前に落ちる試験を足すが、検証を本数（NL）だけにしていて筋は通っている。人間の確認待ちのタスクは無い（CH04.md:79 のとおり）
- **観点 5（change の文書）**: CH04.md の「完了の判定」は空でなく、tasks の番号まで指している。確認バッチで本人が触るものが無いことも書いてある。
  足りないのは、書き出しの判定の 2 つ目の項目が R1 のとおり直す前から通ること（直したかの判定は 8.3 の読んだ行の数にしかない）と、archive の判定に tasks 10.2（試験の本数・印を減らしていない）が入っていないこと。
  「壊してはいけないもの」は、トリガ（Scenario）・除く記録（Scenario）・`ashiato` を消さない（Scenario と台本）・`pool()` の形（コンパイル）・本数と印（tasks 10.2）で守られる。繋ぎ先（`server_startup.rs` など）は R14。「依頼（逐語）」の 1〜3 を specs は越えておらず、欠いてもいない（依頼 3 の「行数が増え続けない」は、Q3 の推奨のとおり database の一覧で見る形に置き換えている）
- **観点 6**: 該当なし（`spec --draft`。`--answers` の段ではない）
