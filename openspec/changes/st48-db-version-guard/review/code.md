# ST48 code review

## final review（6bb4584..daef53c）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 5 / Minor 11。
fix は 1 回（75dbd55..1a85f56、証跡 4a36059）、scoped re-review 1 回で I1〜I3・M1〜M4・M6・M10 は全部 ADDRESSED、新しい Critical / Important なし。

## R1. 知らない版が 20 件を超えた分の行が `  db: ほか N 件` で、1 版 1 行の `  db: ` の頭（配った版に焼き付き ST58 が機械で読む）を流用している
- 成果物: crates/server/src/migration_ledger.rs
- 根拠: migration_ledger.rs:60-64（daef53c）/ design.md D6（`db:` は 1 版 1 行）/ 21 件以上を見る試験が無い（Task 4 の ⚠️）
- kind: technical
- 処置: fixed D6 —— 超過の行を `  more: <N>` に分け、D6 に形と 23 件の例を書いた。単体試験 `st48_ahead_report_caps_db_lines_and_counts_the_rest`（75dbd55・1a85f56）

## R2. 確認手順書・manifest の DB の行が既定の compose project を指し、`run.sh` の `COMPOSE_PROJECT_NAME=ashiato-verify-<tag>` の DB を見ない
- 成果物: tools/verify-prep.sh
- 根拠: verify-prep.sh:92-94・120（`docker exec ashiato2-db-1 psql …`・project なしの `docker compose exec -T db`）/ Task 8 F6（re-review の対象外のまま残っていた）
- kind: technical
- 処置: fixed 8.1 —— `dbq="COMPOSE_PROJECT_NAME=$project docker compose exec -T db psql …"` を 1 か所で作り手順書の行が使う。準備のときに既定の project の DB を数える処理は消し（回ごとの DB は準備のとき空）、その回の DB で数える 1 行と理由に替えた（e7aca5d）。`verify-run 8.1` rc=0

## R3. `migrate_after` が `GRANTS` を助言錠の外で当てている（design D4 の 6→7 からの逸脱。並ぶ migrate・足場で `tuple concurrently updated` になりうる）
- 成果物: crates/server/src/lib.rs
- 根拠: lib.rs:195-205（daef53c。`apply_migrations_after` が錠を外して戻った後に `raw_sql(GRANTS)`）/ design.md:83-84 / ST48 の前の足場は `pg_advisory_xact_lock` の中で `GRANTS` を当てていた
- kind: technical
- 処置: fixed D11 —— `apply_migrations_with(conn, versions, before_apply, grants)` にし、付与を `apply_locked` の末尾（錠の中）で流す。ahead・読めないときは流さない。試験 `st48_apply_grants_run_inside_lock_and_not_on_refusal`（85b32bf）。D11 を合わせた（1a85f56）

## R4. tasks.md 2.2 は「`ASHIATO_BUILD_COMMIT` を渡さない場合も `git rev-parse HEAD` と一致」と書くが、試験は「HEAD と一致するか HEAD の祖先」に緩めている
- 成果物: crates/server/src/migration_ledger_tests.rs
- 根拠: migration_ledger_tests.rs:271-289 / tasks.md:53 / design.md:141（D8: git の HEAD の変化では作り直さず、差分のビルドでは commit が古いまま残りうる）/ Task 2 F1
- kind: technical
- 処置: rejected: 緩めたのは D8（仮）が許す振る舞いそのものに合わせたもの。HEAD と一致を求めると、HEAD を進めた後の差分のビルドで試験が誤って落ちる。2.2 の検証コマンド（`CT`・`NL`）は HEAD で PASS（evidence.jsonl）し、2.2 が名指しする試験の名前も変わっていないので tasks.md の書き換え（plan_fix）は要らない

## R5. HEAD で `server_startup` の試験を走らせた証跡が無い（4.2・5.2 の PASS は `Rig::serve` の書き換え b511cd5 より前）
- 成果物: openspec/changes/st48-db-version-guard/evidence.jsonl
- 根拠: evidence.jsonl の 4.2・5.2 の head / 9.1 は `IT st48_` を含まない / Task 5 の ⚠️
- kind: technical
- 処置: fixed 4.2 —— fix の後の 1a85f56 で `verify-run 4.2`・`5.2`・`2.1`・`1.1`・`8.1`・`8.2`・`9.1` を走らせ全部 rc=0（4a36059 で証跡を commit）

## R6. 試験の注記が「`migrate()`/`run()` は起動のたびに全版を当てる」を理由にしている（いまは当てない）
- 成果物: crates/server/src/registry_tests.rs
- 根拠: registry_tests.rs:67・396・462 / drops_tests.rs:84 / Task 2 の ⚠️
- kind: technical
- 処置: fixed 2.1 —— 理由を「版の記録の無い古い DB には `migrate()` が全版を一度だけ当てる」に直した（11ff8fb）

## R7. 「接続が閉じれば錠は消える」はプールの接続では成り立たない（`migrate()` の接続はプールへ戻るだけ）
- 成果物: crates/server/src/migration_ledger.rs
- 根拠: migration_ledger.rs:314（daef53c）/ Task 2 F2
- kind: technical
- 処置: fixed D11 —— 錠を外せなかったら（Err か false）`conn.detach().close()` で pool に返さず閉じる。注記も直した（85b32bf）。そのため pub の `apply_migrations` は `PoolConnection<Postgres>` を値で受ける

## R8. `stack.sh`・`dev.sh` の止まったときの案内が「上の db: の行」と言うが、`ledger_unreadable` には `db:` の行が無い
- 成果物: tools/stack.sh
- 根拠: stack.sh・dev.sh の D14 の文言 / Task 8 F7
- kind: technical
- 処置: fixed D14 —— 「上の `error: kind=` の行が種別。db_ahead なら db: の行、ledger_unreadable なら読めない理由」に直した（ab09b5c。migrate は db_behind を出さない）

## R9. `check-db-version-stop.sh` は rc≠0 しか見ていない。終了コード 3 は焼き付く契約
- 成果物: tools/check-db-version-stop.sh
- 根拠: check-db-version-stop.sh:89（daef53c）/ Task 8 F3
- kind: technical
- 処置: fixed 8.2 —— `[ "$rc" = 3 ]` を見る（ab09b5c）。`verify-run 8.2` rc=0

## R10. `st48_ledger_shape_is_frozen` が主キー（`only_row`・`name`）と `CHECK (only_row)` を見ていない
- 成果物: crates/server/src/migration_ledger_tests.rs
- 根拠: spec の表は主キーを書く / Task 1 F5
- kind: technical
- 処置: fixed 1.1 —— 主キーと `pg_get_constraintdef` の `CHECK (only_row)` を assert した（75dbd55）。`verify-run 1.1` rc=0

## R11. 退避の失敗・版の中身の失敗の試験が、spec の THEN「`kind=` が無い」を assert していない
- 成果物: crates/server/tests/server_startup.rs
- 根拠: `st48_migrate_backup_failure_applies_nothing`・`st48_migrate_content_failure_exits_1` / Task 4 F1
- kind: technical
- 処置: fixed 4.2 —— 2 本に `!stderr.contains("kind=")` を足した（7250af4）

## R12. `stack.sh` の `docker stop … && echo` は stop が失敗しても黙って進み、後の `compose up` が port の取り合いで分かりにくく落ちる
- 成果物: tools/stack.sh
- 根拠: stack.sh:85 / Task 8 F4
- kind: technical
- 処置: rejected: 失敗しても次の `docker compose up` が port の衝突で rc≠0 で止まり、黙って違う DB に繋がることは無い（止まる側に倒れる）。表示の分かりやすさだけの差で merge の前に直す理由にならない

## R13. `check-migrations.sh` の規則 (b) はコメントの中の DELETE でも通り、tag に数字のある dollar-quote（`$fn1$`）を誤って落とす。self-test は境ちょうど `202610250000` を試していない
- 成果物: tools/check-migrations.sh
- 根拠: Task 7 F1〜F3
- kind: technical
- 処置: rejected: `$fn1$` は落とす側に倒れる（安全側）。コメントの中の DELETE で通るのは新しい版を書く人が意図して書いた場合だけで、配った後の凍結は check-released-migrations.sh が別に守る。境ちょうどの扱いは本体の比較（`>=`）で読める。いまの版に該当するものは無い（`check-migrations.sh` rc=0）

## R14. 記録の無い古い DB で版が途中で落ちると、次の回は記録ありと読まれ、残りの版が `before_ledger=false` になる
- 成果物: crates/server/src/migration_ledger.rs
- 根拠: Task 2 F3
- kind: technical
- 処置: rejected: `before_ledger` は診断の材料で、当てる・止めるの判定（名前の集合の比較）には使わない。残りの版は実際に記録の後で当たっているので、印は事実と食い違わない

## R15. `Rig::serve` が bind(0) で取った port をすぐ手放すので並列で競合しうる。指紋の警告の試験は名前が出力のどこかにあることしか見ない
- 成果物: crates/server/tests/server_startup.rs
- 根拠: Task 5 F1・F2
- kind: technical
- 処置: rejected: 既存の `server_startup` の試験と同じ取り方で、fix の後の HEAD で `verify-run 4.2`・`5.2` は rc=0。競合が起きれば待ち受けの失敗で落ちる（誤って通らない）。警告の試験は D7 の「警告だけ・止めない」を見るもので、行の形は D6 の契約の外

## R16. 使い捨ての `st48_*` database は、試験のプロセスが kill されると残る
- 成果物: crates/server/src/testdb.rs
- 根拠: Task 1 F4
- kind: technical
- 処置: rejected: 残るのは worktree の使い捨ての DB の中の空の database だけで、共有の DB の中身には触らない（D16）。CH04 の `*_tmp_*` の口に載せ替えるときに掃除をまとめるのが筋で、この change の範囲の外

## R17. `dev.sh` には `stack.sh` に足した「この port を持つ別 project の容器を止める」が無く、確認バッチの DB が port を握ったままだと生の port 衝突で落ちる
- 成果物: tools/dev.sh
- 根拠: final reviewer の Recommendations / tools/stack.sh:85 / design D13
- kind: technical
- 処置: rejected: 落ちるのは `docker compose up` の rc≠0 で、別の DB に黙って繋がることは無い。D13・tasks 8.x が求めたのは stack.sh（確認バッチと縦串）で、dev.sh へ広げるのは範囲の外。PR 本文の残りに挙げる
