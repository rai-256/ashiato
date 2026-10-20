# CH02 spec レビュー（spec --draft・第 1 回の先読み）

対象: `openspec/changes/ch02-export-next-record-window/`（proposal・specs/data-durability・design・tasks・deep・premises・review/deep）、
`docs/changes/CH02.md`、正典 `openspec/specs/data-durability/spec.md`、`crates/server/src/export/excluded.rs`、`crates/server/src/export_*tests.rs`、
ST33 archive の design D5 / D13 / D17、`openspec/changes/st09-media-capture/`。deep.md の答えは推奨（答え待ち）を本人の答えとして読んだ。

## 機械の検査（最初に写す）

| コマンド | rc | 要点 |
|---|---|---|
| `openspec validate ch02-export-next-record-window --strict` | 0 | valid |
| `python3 scripts/check_chain.py .` | 0 | chain: OK（change なので観点 8 は無い） |
| `python3 scripts/check_scenarios.py . ch02-export-next-record-window` | 1 | 担保の無い Scenario 7 件 = CH02 で足した 7 本（MODIFIED 5・ADDED 2）。実装前なので想定内。tasks 2.1 / 4.2 / 4.3 の `// Scenario:` 7 つは spec の見出しと一字一句一致する（確かめた）。ほかは既存の warn のみ |
| `python3 scripts/review_triage.py . ch02-export-next-record-window` | 1 | `deep.md R1 / R2 / R7 / R10: escalated なのに deep.md に R<n> が無い` の 4 件 → R1 |

確かめた事実（指摘の根拠に使う）:
- 既存 17 本の Scenario と Requirement 本文は、正典から逐語で写っている（`diff` の差は (c) の書き足し・前の版の (c) の 1 文・導出元 2 行・新しい Scenario 5 本だけ）
- ST09 の `feat/st09-media-capture` の `excluded.rs` は、一覧（`SPAN_RULES` / `COORD_SOURCES`）に 3 行足すだけで `overlap_sql` / `excluded_sql` は同じ（proposal Impact の主張どおり）
- `cargo test` の名前の下限（Task 6.1）は、いまの試験の数と一致する（`#[tokio::test]` / `#[test]` を数えて export_excluded 24・export_file 43・export_job 38・export_ledger 3・export_tests 18）
- 使い捨ての `postgres:17-alpine`（17.11。`docker run --rm`、repo の DB には触っていない）に `migrations/ORDER` の全版を当て、いまの SQL（`excluded.rs` の断片を逐語で組み直したもの。原文の座標の正規表現だけ 2 本に縮めた）を
  `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)` し、design D7 の指標（`Relation Name` が `event` / `event_version` の節の `(Actual Rows + Rows Removed by Filter + Rows Removed by Index Recheck) × Actual Loops` の和）を取った。消した滞在 01:00〜02:00 を 1 件、`c02-window` を 00:00〜00:59 に散らした:

| 配置（N=1,000 → 4N=4,000） | いまの SQL の読んだ行の数 | 比 |
|---|---|---|
| 同じ端末（Task 4.2 の配置）・ANALYZE 前 | 1,002,002 → 16,002 | 0.016 |
| 同じ端末・ANALYZE 後 | 4,003 → 16,002 | 4.0 |
| 端末 50 種 + 別の利用者の後の行 N・ANALYZE 後 | 97,002 → 404,002 | 4.2 |
| 行ごとに別の端末 + 別の利用者の後の行 N・ANALYZE 後 | 501,002 → 8,004,002 | 16 |

  ANALYZE 後の計画は、次の記録の引きに `event_by_source_time (logical_source, event_time)` か `event_by_user_time_live (user_id, event_time)` の Index Scan + Incremental Sort + Limit を選び、1 回 3 行しか読まない。
  ANALYZE 前の N=1,000 は `event_by_user_source_ingest` を選び 1 回 1,000 行（2 乗）。消した滞在 0 件では `Seq Scan on event loops=1 Rows Removed by Filter=1001`（表は読む）・次の記録の Limit 以下は `loops=0`
- 同じ PG で `SELECT now() = transaction_timestamp()` は `t`（トランザクションの中で時間を置いても `t`）

---

## R1. review_triage が落ちる: escalated の R1 / R2 / R7 / R10 が deep.md に写っていない
- 成果物: openspec/changes/ch02-export-next-record-window/deep.md
- 根拠: `python3 scripts/review_triage.py . ch02-export-next-record-window` rc=1（`[FAIL] deep.md R1 / R2 / R7 / R10: escalated なのに deep.md に R<n> が無い`）。review/deep.md の処置は R1→Q1・R2→Q4・R7→Q3・R10→Q2 と書くが、deep.md の Q1〜Q4 に R の参照が無い
- kind: technical
- 提案: deep.md の各 Q に出どころの R（Q1 ← R1、Q2 ← R10、Q3 ← R7、Q4 ← R2）を書く。record が本人の答えに書き換えるときに消えない場所に置く
- 処置: fixed deep.md —— Q1〜Q4 に「出どころ: review/deep.md R<n>」を書いた（answer の行の外なので record の書き換えで消えない）

## R2. Task 4.2 の「いまの SQL にも当て、比が 8 以上」は、Task 4.2 の配置（同じ端末）では成り立たない（実測で比 4.0。ANALYZE 前は 0.016）
- 成果物: openspec/changes/ch02-export-next-record-window/tasks.md（4.2）・design.md（D7）・specs/data-durability/spec.md（ADDED の Scenario「記録を 4 倍にしても…」）
- 根拠: 上の表。tasks.md:78-80 は「同じ端末」に `SCALE_N` と 4 倍を置き、`legacy::excluded_sql("now()")` の比が `SCALE_LIMIT`（8）以上を assert する。実測で ANALYZE 後 4,003 → 16,002（4.0）。design.md:116 の反転条件「`ANALYZE` を当ててから測る」は、ANALYZE 前の 1,002,002 を 4,003 に下げ、assert をさらに外す側に働く。2 乗になったのは「組の数が N とともに増える」配置（行ごとに別の端末で比 16）か、計画が `event_by_user_source_ingest`（event_time を持たない索引）を選んだときだけ。spec の WHEN は端末の数・組の交じり方を決めていない
- kind: technical
- 提案: Scenario の WHEN と Task 4.2 に「組（端末）が N とともに増える配置」（例: 行ごとに別の端末 + 別の利用者の後の行）を書き、ANALYZE 後で比 16 になったことを premises に足す。いまの SQL の側の assert はその配置で置く。配置は上流で決める（下流に残さない）
- 処置: fixed 4.2 —— spec の WHEN と tasks 4.2・design D7 の配置を「端末が記録ごとに違い、別の利用者の後の行を同じ数」にし、ANALYZE を当ててから測る形にした。premises の 1 行目に実測を足した

## R3. 遅さの診断（「IS NOT DISTINCT FROM は索引で絞れない」「行ごとに 1.5〜4.5 万行を読み流す」）は計画しだいの観測で、引き方そのものの性質ではない
- 成果物: openspec/changes/ch02-export-next-record-window/premises.md（表の 1 行目）・proposal.md（Why）・design.md（Context）
- 根拠: 上の表の ANALYZE 後の計画は、`NOT (device_id IS DISTINCT FROM …)` を Filter に置いたまま `event_by_source_time` を順に歩き、1 回 3 行で止まる。premises.md の 1 行目は「合成表」（索引の有無を書いていない）での `Bitmap Heap Scan` の観測。2 乗になるのは、計画が組全体を読む索引を選ぶか、次の記録が遠い（組が交じる・組の最後の行）ときに限られる
- kind: premise
- 提案: premises.md の 1 行目に「計画が `event_by_user_source_ingest` を選んだとき」「`event_by_source_time` を選べば 1 回数行（ANALYZE 後の実測）」を足し、確かさを書き直す。窓関数に替える方針は計画に依らず線形なので結論は変わらないが、R2 の assert と design D7 の反転条件の前提がここに載る
- 処置: escalated —— 第 1 回の問いと同じ 1 枚で本人に見せる: deep.md の Q3 に「前提の訂正（review/spec.md R3）」を書き、Q3 の context にも足した（推奨は変わらない）。premises.md の 1 行目と design の Context も計画しだいと書き直した

## R4. ADDED「消した滞在が無ければ…」の Requirement 本文・THEN・Task 4.3・design D3 が、別々のことを測る
- 成果物: openspec/changes/ch02-export-next-record-window/specs/data-durability/spec.md:148・160-161、tasks.md:81-83、design.md:65
- 根拠: Requirement 本文と THEN の前半は「記録の表と前の版の表を読まない」（表の走査そのもの）、THEN の括弧は「次の記録を求める部分が 1 度も実行されない」、Task 4.3 は「`WindowAgg` とその**下の**節が `Actual Loops` 0」（窓の外で `core.event` を読む節があっても通る）。いまの SQL は消した滞在 0 件で `Seq Scan on event loops=1 Rows Removed by Filter=1001`（表を読む）・次の記録の節は `loops=0`（実測）—— 前半なら落ち、括弧なら通る。design D3 は「実行されない形にする」とだけ書き、仕組み（`EXISTS (SELECT 1 FROM erased_spans)` のような One-Time Filter の門）を決めていない。`r.event_time <= (SELECT max(end_at) …)` の形は InitPlan が NULL でも外側の走査を実行する（いまの SQL がその形）。また Q4 の context「いまは消した滞在が 0 件なら何も引かず即答し」は、表の全走査が残る点で実測と違う
- kind: technical
- 提案: どれを正とするか 1 つに決めて揃える。表を読まないことを正にするなら、THEN を「計画の中で、消した滞在を探す節のほかに `core.event` / `core.event_version` を読む節が 1 度も実行されない」とし、Task 4.3 を窓の下に限らず全部の節で見る形に、design D3 に門の形を書く。いまの SQL より強い保証になることを design に書く
- 処置: fixed D3 —— 「記録の表と前の版の表を読まない（消した滞在を探す部分を除く）」を正にし、Requirement・Scenario の名前と THEN・tasks 4.3 を揃えた。門は One-Time Filter（`WHERE EXISTS (SELECT 1 FROM erased_spans)`）と design D3 に書き、いまの SQL より強い保証であることを 4.3 で assert する。Q4 の context の「何も引かず即答」も直した

## R5. design D3 の後半（組ごとに `max(end_at)` より後の最初の記録を 1 行入れる）は、写しの時点が消した滞在の始まりより前のときにしか結果に効かず、どの試験もそれを置かない
- 成果物: openspec/changes/ch02-export-next-record-window/design.md（D3）・tasks.md（3.1・4.2）・specs/data-durability/spec.md:11
- 根拠: excluded.rs:261・265-267。時刻 t ≤ M（= `max(end_at)`）の行の本当の次の記録が M より後なら、M を持つ消した時間 [s*, M] は `s* ≤ M < 次の記録` かつ `M ≥ t` で必ず重なる。次の記録の代わりに `snapshot_at` を使っても `s* ≤ snapshot_at` なら同じく重なる。差が出るのは `snapshot_at < s*`（消した滞在の始まりが写しの時点より後）のときだけ。D6 の族（design.md:102-104）にも Task 4.2 の配置（全行が M より前）にもその配置が無いので、後半を誤って書いても落ちず、省いても落ちない。spec.md:11 の「消した時間より後の記録も次の記録になる」の句も同じ理由で、普通のデータ（写しの時点が消した滞在の始まりより後）では Scenario で見分けられない。また組ごとの最初の 1 行を自然な形（`DISTINCT ON` / `GROUP BY`）で取ると M より後の行を全部読む（PG 17 は skip scan を持たない。組ごとに LATERAL で `event_by_source_time` を歩く形なら短く済む）ので、消した滞在が古いときの量は Q4 の狙い（全行を並べない）の半分しか減らない。その量も Task 4 は測らない
- kind: technical
- 提案: D6 の族に「写しの時点を固定の値にし、消した滞在の始まりより前にする」配置を足す（R6 と同じ直し）か、後半を落としてその理由（上の重なりの理屈）を D3 に書くかを決める。落とすなら Q4 の推奨の文言に触れるので deep の答えと合わせる。古い消した滞在（行の大半が M より後）の読んだ量を Task 4 に 1 本置く
- 処置: fixed D3 —— 後半が効くのは写しの時点が消した滞在の始まりより前のときだけ、と理由を書き、残す。D6 / tasks 3.1 に固定の写しの時点（始まりより前）の族を足し、古い消した滞在の読んだ量を 4.2 に足した

## R6. Task 3.1 の「`t` = `now()` と `transaction_timestamp()` のどちらでも一致」は差を生まない（PG では同じ値）
- 成果物: openspec/changes/ch02-export-next-record-window/tasks.md:64-65・design.md:103（族「写しの時点の違い」）
- 根拠: 使い捨ての PG 17.11 で `SELECT now() = transaction_timestamp()` が `t`（トランザクションの中で時間を置いても `t`）。PostgreSQL の文書は `now()` を `transaction_timestamp()` と同じものとする。dump.rs:138・summary.rs:92・ledger.rs:431 の違いは関数ではなく、どのトランザクションで評価するか
- kind: technical
- 提案: 族「写しの時点の違い」を、固定の時刻の式（例 `'…T01:30:00Z'::timestamptz`）を `snapshot_at` に渡し、消した時間の前・中・後に置く形に替える。R5 の「写しの時点が消した滞在の始まりより前」もこれで置ける
- 処置: fixed 3.1 —— 写しの時点の族を、固定の時刻の式 3 つ（始まりより前・中・後）に替えた（design D6）

## R7. Scenario「前の版の幅は、親と同じ組の記録のうち…」は「前の版ではない」を見分けず、THEN が 3 つの主張を束ねる
- 成果物: openspec/changes/ch02-export-next-record-window/specs/data-durability/spec.md:132-135
- 根拠: 配置（前の版 00:10・00:40、記録 00:20・02:30・03:00、消した時間 01:00〜02:00）で、前の版を次の記録の候補に含める誤り（design D1「候補は is_event だけ」の反対）を当てても、00:10 の次は 00:20、00:40 の次は 02:30 のままで結果が同じ。Requirement 本文の「削除の印の無い記録（前の版ではない）」を見分ける Scenario が無い。THEN は「00:10 出る・00:40 出ない・03:00 出る」の 3 つ
- kind: technical
- 提案: 前の版を候補に含めると幅が縮む配置（例: 前の版 00:30 と 00:40、次の記録 02:30 → 00:30 の前の版は出ない。前の版が候補なら 00:40 で止まって出る）を足すか、この Scenario を差し替える。THEN は主張ごとに分ける
- 処置: fixed specs/data-durability/spec.md —— THEN を 1 つの主張にし、00:10 の側を別の Scenario にした。「前の版は、ほかの前の版を次の記録にしない」を足した（前の版 00:30・00:40、次の記録 02:30）

## R8. Requirement 本文だけが言い、Scenario が無いもの: 削除の印の付いた記録は次の記録にならない・前の版の組の利用者は親から取る
- 成果物: openspec/changes/ch02-export-next-record-window/specs/data-durability/spec.md:11・13
- 根拠: spec.md:11「削除の印の無い記録のうち」と spec.md:13「親と同じ利用者・同じソース・同じ端末」。どちらも新しい Scenario 5 本にも既存 17 本にも、それを見分ける配置が無い（既存「削除の印の付いた記録は出ない」は印の付いた行が出ないことだけを見る）。tasks 3.1 の族（「削除の印を間に挟む」「前の版の `user_id` が親と違う」）にはあるが、名前のある Scenario ではないので、正典に入った後の別の change が式を替えても spec では落ちない。premises.md「record の段への入力」と review/deep.md R8 も、この 2 つを「壊してはいけないもの」に挙げる
- kind: technical
- 提案: Scenario を 2 本足す（例: 00:10 の座標つきの記録・00:40 の印の付いた記録・02:30 の記録 → 00:10 は出ない。前の版の `user_id` を親と違う値にし、親の組の次の記録までの幅で読む）。tasks 3.1 の族と同じ配置でよい
- 処置: fixed specs/data-durability/spec.md —— Scenario「削除の印の付いた記録は、次の記録にならない」「前の版の組の利用者は親の利用者で読む」を足した（tasks 2.1）

## R9. `docs/changes/CH02.md` の「壊してはいけないもの」が空
- 成果物: docs/changes/CH02.md
- 根拠: 「壊してはいけないもの」の節は「（上流が深掘りの後に書く。…）」のまま。premises.md の「record の段への入力」に 5 件（READ ONLY・`core.event_live` から引く・前の版の鍵は親から・D17 の論拠・断片は 1 か所）があり、design D5 の表も同じものを挙げる。振る舞いを変えない change では、ここが Scenario で固定する対象の一覧になる
- kind: technical
- 提案: record の段で premises.md の 5 件を写し、それぞれ Scenario（R8 の 2 本を含む）か Task（3.4 の READ ONLY）のどれが固定するかを並べて書く
- 処置: rejected: 「壊してはいけないもの」は record の段が答えと一緒に書く（.claude/skills/story-upstream/SKILL.md:52「record: 答えで決まったことを CH<NN>.md の「価値」「壊してはいけないもの」に書く」）。中身と、それぞれを何が固定するかは design D5 の表に足した（R8 の 2 本・3.4 を含む）

## R10. Scenario「端末を持たない記録は…」はソースを名指ししていない
- 成果物: openspec/changes/ch02-export-next-record-window/specs/data-durability/spec.md:122-125
- 根拠: WHEN は「同じ利用者・同じソースで」とだけ書く。ソースが「時刻 1 点」（`SPAN_RULES` の `c01-location` など）なら 00:10 は点になり消した時間に入らず出るので、THEN「出ない」は (c) のソースでしか成り立たない。tasks 2.1 もソースを書かない
- kind: technical
- 提案: WHEN に (c) のソース（例 PC のウィンドウ）を書く。端末を持たない記録が実際に入りうるソースを選ぶなら、それも書く
- 処置: fixed specs/data-durability/spec.md —— WHEN を PC のウィンドウの記録と名指しした

## R11. 新しい Scenario の THEN は「記録のファイル / 前の版のファイル」を言うが、Task 2.1 は断片の除く id の集合で見る
- 成果物: openspec/changes/ch02-export-next-record-window/tasks.md:47-53
- 根拠: tasks.md:52「前の版のファイルの代わりに除く前の版の id で見る」。前の版がファイルに出るかは `version_kept`（excluded.rs:168-173。親が除かれるかも見る）で決まり、`excluded_version` の id だけでは決まらない。既存 17 本の印は `export_file_tests.rs` のファイルの試験にある
- kind: technical
- 提案: THEN を断片の言葉（「除く記録・前の版に入る / 入らない」）にするか、試験を `version_kept` / `event_kept` を通した残る集合で見る形にする（`export_excluded_tests.rs` の `kept` / `kept_versions` と同じ）
- 処置: fixed 1.1 —— 試験は `event_kept` / `version_kept` を通した残る集合（`kept_ids`）で見る形にした（2.1・3.1）

## R12. CH02.md の `capability: []` と、proposal の Modified Capabilities・delta の `data-durability` が一致しない（衝突待ちの仕組みを外す決定）
- 成果物: docs/changes/CH02.md（front matter）・openspec/changes/ch02-export-next-record-window/proposal.md:46
- 根拠: proposal.md:46「`capability: []` はそのままにする（…`data-durability` を書くと走っている ST09 と衝突待ちになるが、行も Requirement も重ならない）」。docs/stories/INDEX.md:382「ST09 が走っている間、ST10 / ST30 / ST31 / ST32（`data-durability`）は `衝突待ち`」。重ならないこと自体は確かめた（ST09 は ADDED 1 つ・一覧の行だけ）が、空のままだと、この change が走っている間に同じ Requirement「書き出しは本人が消した場面を出さない」を MODIFIED する Story が入っても機械は止めない
- kind: conflict
- 提案: `capability: [data-durability]` と書いて機械に判定させるか、外す例外を本人が決める（確認バッチでなく、いま）。外すなら admission が読む場所に例外として残す
- 処置: fixed D9 仮 —— `capability: []` のままにする理由と反転条件（同じ Requirement を MODIFIED する上流が入るなら書く）を design D9 に書いた。失うものは無い

## R13. 依頼「既存の export の試験（ST33・ST09）をすべて通す」のうち ST09 の分を、tasks は走らせず ST09 の rebase に送っている
- 成果物: openspec/changes/ch02-export-next-record-window/tasks.md:102・docs/changes/CH02.md（完了の判定 4 つ目）
- 根拠: CH02.md「依頼（逐語）」の最後の文。tasks.md:102「ST09 の export の試験は ST09 のブランチにあり、gate の rebase で同じ断片に当たる」。deep に、この範囲を欠く決定は無い。`feat/st09-media-capture` はこの repo にある
- kind: conflict
- 提案: Task 6 に「ST09 のブランチの export の試験を CH02 の断片の上で走らせる」項目（一時ブランチと `CT` の名前・本数）を足すか、依頼から外すことを deep の決定にして完了の判定に書く
- 処置: fixed D10 仮 —— 一時の worktree で ST09 のブランチに CH02 を merge し（push しない）、`CT st09_exp_` と `CT export_` を走らせる項目を足した。完了の判定も直した

## R14. Task 4.3 末尾・Task 5.2 の検証に、コマンドと終了条件の無い手順がある
- 成果物: openspec/changes/ch02-export-next-record-window/tasks.md:84-85・92-95
- 根拠: 4.3「窓の枠を一時的に順向きで尾を固定した枠に替えると 4.2 が落ちることを手元で 1 回確かめ、戻す（PR の本文に 1 行）」はコマンドも記録の置き場も無い。5.2 は「使い捨ての DB で走らせ」とあるが、試験は `testdb::url()`（`DATABASE_OWNER_URL`。crates/server/src/testdb.rs:44-46）で繋ぐので、どの値をどう差し替えるか・6 通りの実行コマンドが無い。終了条件の `grep -c '| |' design.md` は表が埋まったことしか見ない
- kind: technical
- 提案: 4.3 の確かめは、そもそも読んだ行の数では落ちない（R16）。R16 の直しに置き換える。5.2 は `DATABASE_OWNER_URL` の差し替えと 6 通りのコマンドを書く
- 処置: fixed 5.2 —— 4.3 末尾の手作業は 4.4 の断片の文字列の試験に置き換えた（R16）。5.2 に database の作り方・`DATABASE_OWNER_URL` の差し替え・実行と後始末のコマンドを書いた

## R15. D13 の反転条件の境を「3 秒以上」から「3 秒を超えるか」に替えて写している
- 成果物: openspec/changes/ch02-export-next-record-window/design.md:123・docs/changes/CH02.md（完了の判定 3 つ目）・tasks.md:93-94
- 根拠: openspec/changes/archive/2026-10-17-st33-data-export/design.md:378「件数の数えに 3 秒以上かかる量になったら」。CH02 の design.md:123・tasks.md:93 は「3 秒を超えるか」
- kind: technical
- 提案: ST33 D13 と同じ「3 秒以上」に揃える
- 処置: fixed D8 —— 「3 秒以上」に揃えた（design D3 / D8・表の見出し・tasks 5.2・CH02.md）

## R16. D7 の指標（読んだ行の数）は、順向きで尾を固定した枠の 2 乗を捉えない —— Task 4.3 の「替えると 4.2 が落ちる」は落ちない
- 成果物: openspec/changes/ch02-export-next-record-window/design.md:45-46・112・140、tasks.md:29・84-85
- 根拠: 使い捨ての PG 17.11（上と同じ手順で立てて消した）で、同じ端末の `c02-window` を N 行置き ANALYZE した後、鍵の列だけの窓を 2 通り `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)` した。
  D1 の枠（`ORDER BY event_time DESC RANGE BETWEEN UNBOUNDED PRECEDING AND '1 microsecond' PRECEDING`）と、使わないと書いた枠（`ORDER BY event_time ASC RANGE BETWEEN '1 microsecond' FOLLOWING AND UNBOUNDED FOLLOWING`）の読んだ行の数は、N=1,000 / 4,000 / 16,000 でどちらも 1,001 / 4,001 / 16,001（同じ）。
  実行時間は D1 の枠 2.5 / 10.1 / 42.8 ms、使わない枠 38.2 / 543.9 / 8,343.1 ms（4 倍ごとに約 15 倍）。数え直しは `WindowAgg` の中で起き、`core.event` を読み直さないので、`Relation Name` を持つ節の数は変わらない。
  design.md:140「D7 の試験が読んだ行の数で落とす」と tasks.md:29「Task 4 の試験が読んだ行の数で落とす」・tasks.md:84「替えると 4.2 が落ちることを手元で 1 回確かめ」は成り立たない
- kind: technical
- 提案: 枠の退行は読んだ行の数では固定できないと design D7 に書く。固定は、組んだ断片の文字列に D1 の枠があり `FOLLOWING AND UNBOUNDED FOLLOWING` が無いことを assert する試験（機械・揺れない）か、この 1 点だけ `WindowAgg` 節の `Actual Total Time` の N と 4N の比で見る形にする。Task 4.3 末尾の手作業の確かめは、その試験に置き換える
- 処置: fixed D7 —— 読んだ行の数では枠の退行を見分けられないと書き、枠は断片の文字列の試験（tasks 4.4）で固定する形にした。Global Constraints と Risks も直した

---

## 観点ごとのまとめ

- 観点 1（deep の決定が正典に写っているか）: Q1 は spec (c) の書き足し・導出元・新しい Scenario 1 本に写っている。Q2 は design D6・tasks Task 3、Q3 は spec ADDED・design D7 / D8・tasks Task 4 / 5、Q4 は spec ADDED の 2 本目・design D3・tasks 2.2 に写っている。古い方（`(event_time, id)` の順）は spec に残っていない。数値（N と 4N・8 倍）は Scenario にある。指摘は R1（triage）・R2 / R16（Q3 の試験が成り立たない）・R4 / R5（Q4 の写し方）
- 観点 2（Scenario が検証可能か）: R4・R7・R8・R10・R11
- 観点 3（置き場の誤り）: design の D 番号のうち観測できる振る舞いは spec にも置かれている（D3 の 0 件・D5 の写しの時点）。specs に実装の名前は新たに入っていない。proposal の Capabilities と specs のディレクトリは一致する。CH02.md の capability との不一致は R12
- 観点 4（tasks が検証を持つか）: `CT` の名前の下限は実在の試験の数と一致し、依存順（写し → 試験 → 書き換え → 突き合わせ → 量）も合っている。人間の確認待ちは無い。指摘は R2・R6・R14・R16
- 観点 5（change の規則）: 完了の判定は書かれていて deep の推奨と specs に合っている（R15 の境を除く）。「壊してはいけないもの」が空（R9）、その中身の一部が Scenario になっていない（R8）、依頼の範囲を欠く所（R13）
- 観点 6: 該当なし（`spec --draft`。`spec --answers` ではない）
