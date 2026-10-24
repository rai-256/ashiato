# CH04 コードレビュー

## final review（9874721..8cd9eb9）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`、review package `.superpowers/sdd/tasks/review-98747213..8cd9eb93.diff`）。
範囲には CH02（PR #138。main に merge 済み）の取り込み aa1cad15 の commit も入るが、CH02 の中身は見直さず、CH04 がそれらのファイルに足した分だけを見た。
判定: **Ready to merge: With fixes**（Critical 0・Important 2・Minor 8）。fix は 1 回（ff289db2・22f4f371、証跡 323159c7・34ea6933）、
scoped re-review 1 回（package `.superpowers/sdd/tasks/review-8cd9eb93..34ea6933.diff`）→ 直した 7 件はどれも ADDRESSED、新しい Critical / Important なし。
再レビューが fix の中に Minor 1 件を挙げた（R11。2 回目の fix はしない）。

前の Task で確かめられなかった項目（⚠️）の確かめ（reviewer）:
- Task 3（CI の Postgres に `ashiato` が在るか）: 在る（`docker-compose.yml` の `POSTGRES_DB: ashiato`）
- Task 4（試験が自前で繋ぐ接続の上限）: 5 か所の `fresh_db` は `scratch().pool()` 経由になり上限が入った。`testdb_tests` の自前の接続は管理用
- Task 4（`export-scaling.sh` が上限に 0 を渡すか）: 渡している（`tools/export-scaling.sh:23`）
- Task 5（CH02 の `export_window_tests` の置き換え）: `fresh_db`（= `scratch()`）で試験ごとに分かれ、D4 の目的は満たす（形は D4 の本文の `isolated()` と違う）
- Task 7（CI で psql が `pg_database` を読めるか）: 手元では確かめられない。PR の CI で見る
- Task 10（10.4 の PASS）: evidence の 8ce897e6 で PASS
- Task 1・3（Scenario の印と spec の名前）: `python3 scripts/check_scenarios.py . ch04-test-db-growth` は 10.4 の証跡で PASS
- Task 7・8・9 の開いていた Important: task-8（枝 2・3）と task-9（計測の記録）は branch の中で処置済み。task-7 の残りは R3

範囲外の気づき（re-review）: 7.2 の証跡（323159c7）の `tmp_left` が 188（`/tmp/ashiato-export-*`）。CH03 の `HARNESS_TMP_LEAK=fail` の運用に移ると引っかかりうる。CH04 の持ち込みかは未確認。

## R1. 完了の判定の「全体を 2 回」の証跡（7.2）が、CH02 の取り込み・D9 より前の木（c3fcce86）にしか無い
- 成果物: openspec/changes/ch04-test-db-growth/evidence.jsonl
- 根拠: 7.2 の最後の PASS が c3fcce86。その後に aa1cad15（CH02 の取り込み。`fresh_db` を使う `export_window_tests` の 16 本）・7700aa81（D9）・e27a521a が入ったが、全体の試験と growth の検査を走らせた証跡が無い。CI は `ch04_` に絞って走らせる
- kind: technical
- 処置: fixed 7.2 （`scripts/verify-run 7.2` を最後のコードの木 323159c7 で取り直して PASS・293 秒・2 回とも 1238 passed。証跡は 34ea6933）

## R2. 件数の数え（`read_summary`）が行数に対して線形を大きく超えて伸び（120 万行・M=100 で約 2000 秒）、Q2 の前提（120 万行で 3 秒超・25.5 秒）と桁が違う
- 成果物: openspec/changes/ch04-test-db-growth/design.md
- 根拠: design.md:252 の「注意」と計測の記録（summary_seconds 後: 15.6 → 537.2 → 2176.2 秒）。製品の `GET /export/summary` には上限が掛からない
- kind: technical
- 処置: rejected: この change では直さない。本人の Q2 の答え（deep.md:13「時間は 120 万行まで測って残し、合否にしない」）と Global Constraints（tasks.md:23「件数の数え方（ST33 D13）は替えない」）が範囲を決めている。数字は design.md:236-252 に残っている。CH04 は件数の数えを遅くしていない（前 2643.1 秒）。PR 本文と確認バッチの持ち越しに、数字と「Q2 の前提と桁が違う」を書いて本人に上げ、新しい change の候補にする

## R3. `testdb.rs` の Drop の注記が、DROP の拒否の原因を `ashiato_app` の接続と根拠なく断定し、待ちの合計「約 12 秒」も合わない
- 成果物: crates/server/src/testdb.rs
- 根拠: testdb.rs:375-378（8cd9eb93）。`isolated()` の database に app の役で繋ぐ試験は無い。観測した拒否の 1 件（`ch04_isolated_many_in_parallel`）にも app の接続は無い。sleep の合計は 9.5 秒。task-7 の開いていた件（F1）と F5 の残り
- kind: technical
- 処置: fixed D4 （ff289db2。原因は未特定で、`pg_stat_activity` の診断が次回に名指すと書き直し、合計を約 9.5 秒にした）

## R4. `tools/check-tests-kept.sh` が、base が無いと無言で rc=128 / `sort -u` で重複を潰す / `--self-test` が無い
- 成果物: tools/check-tests-kept.sh
- 根拠: check-tests-kept.sh:11（8cd9eb93）。task-10 の F1〜F3 の未処置
- kind: technical
- 処置: fixed 10.2 （22f4f371。ref の名前を出して rc=2、`sort` + `comm` で重複を残す、`--self-test` 5 通り。10.2 の証跡 PASS）

## R5. `tools/check-test-db-growth.sh` が `--workspace` などの cargo の引数を rc=2 で断る / 回の id の判定の自己検査が無い / `CREATE DATABASE` を大小文字を区別して探す
- 成果物: tools/check-test-db-growth.sh
- 根拠: check-test-db-growth.sh:201（8cd9eb93）。task-7 の F2〜F4、task-6 の F4
- kind: technical
- 処置: fixed 6.2 （22f4f371。`--*` は cargo test へ通し `-h|--help` だけを拾う、自己検査に回の id 2 つ・`_tpl`・不正な id、`grep -i`。6.2・7.1・7.3 の証跡 PASS）

## R6. 古い注記 —— `unique_day` の doc の後半と、`export_file_tests.rs` の量の試験の「座標を持つ行は共有の DB に残り」
- 成果物: crates/server/src/testdb.rs、crates/server/src/export_file_tests.rs
- 根拠: testdb.rs:597・export_file_tests.rs の量の試験（8cd9eb93）。書き出しの試験は試験ごとの database に移った。task-5 の F3
- kind: technical
- 処置: fixed 5.3 （ff289db2。5.3 の証跡 PASS）

## R7. 片付けの試験が database を作ってから助言ロックを取る（間に別の過程の片付けが消しうる）。鍵 `4820253` の直書き
- 成果物: crates/server/src/testdb_tests.rs
- 根拠: testdb_tests.rs:321-330（8cd9eb93）の `ch04_sweep_keeps_a_running_run_without_connections`。task-3 の F1・F2
- kind: technical
- 処置: fixed 3.2 （22f4f371。ロックを先に取り、`testdb::LOCK_RUN_CLASS` を `pub(crate)` にして bind。3.2 の証跡 PASS）

## R8. `fresh_db()` の同じ 15 行が 5 ファイルに並び、明示の `DROP DATABASE` が直後の Drop と重なる
- 成果物: crates/server/src/clock_tests.rs ほか 4 ファイル
- 根拠: clock_tests.rs:44。task-6 の F1
- kind: technical
- 処置: rejected: design.md:97（D5）が `fresh_db()` の名前と戻り値の形（プールと待つと消える future）を残し、future が Drop と二重に消しても落ちないことを決めている。重なりは害が無く、5 ファイルを 1 つにまとめるのは D5 の外の作り直し

## R9. main の `archive_scan_interval_tests.rs:20` が自前の `PgPoolOptions` で繋ぎ、`statement_timeout` が掛からない
- 成果物: crates/server/src/archive_scan_interval_tests.rs（rebase の後に入る）
- 根拠: reviewer が main（b5ec25d7）を merge した一時の木で見た
- kind: technical
- 処置: rejected: この branch の木には無い（`git ls-tree HEAD crates/server/src/archive_scan_interval_tests.rs` が空）。D7 の約束は足場の接続だけで、足場の外の接続はこれに反していない（reviewer も同じ判定）

## R10. `tools/export-scaling.sh` の「比が大きくても rc=0」の自己検査が比 ≈1.00 しか試していない
- 成果物: tools/export-scaling.sh
- 根拠: export-scaling.sh:56（8cd9eb93）。task-9 の F2
- kind: technical
- 処置: fixed 9.2 （22f4f371。偽の秒数を 2 乗にし比 4.00 で rc=0 を見る。9.2 の `--self-test` の証跡 PASS）

## R11. `tools/check-tests-kept.sh` が、ref はあるが merge base が無いとき `git diff` の失敗を無視して `[OK]`・rc=0 を返す
- 成果物: tools/check-tests-kept.sh
- 根拠: check-tests-kept.sh:40（34ea6933）。`check … || rc=$?`（117 行）の中なので `set -e` が効かない。re-review が orphan ブランチで実測し、`fatal: … no merge base` の後に rc=0。CI からは呼ばれない（10.2 の手動の検証だけ）
- kind: technical
