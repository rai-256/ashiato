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
- 処置: fixed 10.2 （ba9b44f7。`git diff` が落ちたら `[FAIL]` で rc=2。`--self-test` に orphan の branch を足した。10.2 の証跡 PASS）

## code_verify（54990f7f）

席: code_verify（独立検証。実装した者ではない）。作業ツリーは 54990f7f で clean。壊したものは毎回 `git checkout --` で戻し、`git status --porcelain` が壊す前と同じ（空）であることを確かめた。
commit が要る壊し方（10.2）は、使い捨ての clone（`/tmp/ch04v/clone`、確かめた後に消した）で行った。

### 申告と実測

申告: tasks 30/30 が `[x]`。検証コマンド一覧は tasks.md の各項目（`CT`・`NL`・`tools/check-test-db-growth.sh` の 3 形・`tools/export-scaling.sh`・`tools/check-tests-kept.sh`・`git diff` の空・`openspec validate`・`check_scenarios.py`・`check-migrations.sh`・clippy・`story_deps.py`・grep 4 本）。

| 検証 | 申告 | 実測（この席） |
|---|---|---|
| 7.2 全体を 2 回（`tools/check-test-db-growth.sh -p ashiato-server`） | PASS（293 秒） | 一致: rc=0・274 秒・2 回とも lib 1238 passed（127.41 秒 / 139.84 秒）＋ server_startup 7 passed。後に残ったのは最後の回と `_tpl` だけ。S0 にいた前の回 `ashiato_run_5a9aa32a12f9` は消えた |
| （追加）2 回の全体の後の `ashiato` の行数 | — | 不変（`ashiato` の全表の `count(*)` を前後で取り `diff` が空。core.event 2・core.source 25 …のまま） |
| 7.1・6.2・7.3 `--self-test` / `--lint` | PASS | 一致: どちらも rc=0（self-test の印 11 個、全項目 ok） |
| 7.3 `tools/check-test-db-growth.sh -p ashiato-server ch04_` | PASS | 一致: rc=0・12 秒 |
| NL の下限 14 本 | PASS | 一致: `ch04_testdb_` 3 / `stay_tests` 65 / `ch04_isolated_` 3 / `ch04_scratch_` 1 / `ch04_isolated_many_in_parallel` 1 / `app_role` 3 / `ch04_sweep_` 3 / `ch04_statement_timeout_` 3 / `export_` 152 / `ch04_export_isolated_…` 1 / `places_tests` 158 / `ch04_overlap_matches_ch02` 1 / `ch04_overlap_reads_erased_spans_linearly` 1 / `ch04_overlap_` 3。すべて下限以上 |
| CT（1.3・2.x・3.2・4.2・5.x・6.1・8.x の絞り込み） | PASS | 一致（間接）: どれも全体 2 回（上）の部分集合で、全体が 0 failed。絞り込みごとには走らせ直していない |
| 8.3 読んだ量 | PASS | 一致: `今 56 → 206 / 足す前 8035 → 120125`（比 3.7 倍 / 14.9 倍。8 倍の線の両側） |
| 9.1 `ROWS=20000 ERASED=4` | PASS | 一致: 1 passed |
| 9.2 `tools/export-scaling.sh` 本体 | PASS（3382 秒・head e27a521a） | 走らせ直していない（3382 秒）。証跡の head e27a521a は D9（7700aa81）を祖先に持ち、その後の `export_file_tests.rs`・`export-scaling.sh` の差分は注記と self-test だけ（`git diff e27a521a HEAD`）。`--self-test` は rc=0 |
| 10.1・10.3・5.1/5.3/6.1/1.1 の grep・8.1 | PASS | 一致: どれも rc=0 |
| 10.2 `tools/check-tests-kept.sh origin/main crates/server` | PASS | 一致: rc=0・`--self-test` rc=0。ただし R15 |
| 10.4 validate / check_scenarios / check-migrations / clippy | PASS | 一致: どれも rc=0（check_scenarios は 1510/1510 担保あり。warn は R16） |

### 手 1: 固定値を独立に再計算する
該当なし（確かめた範囲）: `ch04_statement_timeout_default_is_set` の `1min` は psql で `SET statement_timeout='60s'; SHOW statement_timeout` → `1min`。
`57014` は psql（`VERBOSITY=verbose`）で `ERROR: 57014: canceling statement due to statement timeout`。`ch04_overlap_reads_counts_only_the_erased_cte_scans` の `42` は python で `(3+7)*4+1*2+5*0 = 42`。
`CH02_CONDITION` は CH02 の最後の版（cac51b8b の `excluded.rs:298-300`）と空白を除いて同じ文字列。名前の最長は python で 41 字（design D1 の「最長 41 字」と一致）。

### 手 2: ガードをわざと壊す
- 重なりの条件を CH02 の形に戻す（`excluded.rs` の `OVERLAP_CONDITION` を CH02 の `EXISTS` 1 行に）→ `CT ch04_overlap_` が FAILED（`消した時間を読む量が 4 倍で 8 倍以上: 8035 → 120125`）。`ch04_overlap_matches_ch02` は通る（新旧が同一になるので当然）。**落ちた**
- `TestDb` の `Drop` を空に（`testdb.rs:357` の先頭で `return`）→ `CT ch04_` が 4 failed（`消えていない: …_x_…` / `…_tmp_…`）。**落ちた**
- 回の始まりの片付けを外す（`testdb.rs:287` の `sweep_previous_runs(&mut admin).await;` を消す）→ CI の形（`ch04_`）では **落ちなかった**。R12
- 試験ごと・アプリの役割の接続から上限を外す / 環境変数の名前を変える → **落ちなかった**。R13
- `check-tests-kept.sh`: `// Scenario:` の行を 1 本消して commit → rc=1・消えた印を挙げる。**落ちた**。期待値を変えて commit → **落ちなかった**。R15
- 検査の外側: `--lint` は `crates/server/src/export*_tests.rs` だけを見る。書き出しを呼ぶ他の試験のファイルを grep した（`/export` か `export::` を含み `testdb::pool()` を取るファイル）→ `testdb_tests.rs`（5.2 の意図した 1 本）だけ。外側に落ちている試験は無かった

### 手 3: Scenario と test
`python3 scripts/check_scenarios.py . ch04-test-db-growth` rc=0（この change の 25 本に印あり）。印の先を 1 件ずつ読んだ。階層が違うのは R12（「次の回の始まりに」消える、を `sweep()` の直呼びで見ている）と R13（Requirement は足場の全部の接続、Scenario とテストは `pool()` だけ。「環境変数の値が上限になり」は純粋な関数だけを見ている）。ほかは主張の階層で観測している。

### 手 4: 本人の決定が test で固定されているか
- Q3 の 60 秒: `DEFAULT_STATEMENT_TIMEOUT`（`testdb.rs:74`）を変えると `ch04_statement_timeout_from_env_value`（`"60s"`）と `ch04_statement_timeout_default_is_set`（`1min`）が落ちる。固定されている。ただし効く接続の範囲は R13
- Q2 の 4 倍・8 倍・(1,000, 10)/(4,000, 40): 試験の中の定数（`LIMIT`・`scale_erased_reads`）。手 2 で直す前の式に戻すと落ちることを確かめた
- Q2 の 3 点（120 万行まで）: `export-scaling.sh` の `POINTS` の 1 点目と 3 点目は `--self-test` が値の行で見る（2 点目は見ない。比の行が 2 本あることで点の数は見る）
- Q1 の回ごと＋書き出しだけ試験ごと: `ch04_testdb_points_to_the_run_database` と `--lint` が固定
- Q4 の「何もしない」: `ashiato` が残ることを `ch04_sweep_drops_legacy_tmp_and_keeps_others` と growth の最後の判定が見る。中身を空にしないことはテストが見ないが、全体 2 回の前後で `ashiato` の行数が不変だったことをこの席で実測した
- Q5: `check-tests-kept.sh --self-test` が名前で比べることを固定（5 通り）

### 手 5: tasks の `[x]` と実体
30 項目とも、挙がっているコマンド・テスト名・ファイルが実在し、上の表のとおり rc=0。`cargo test --test <名前>` 型の検証は無い。9.2 本体だけ走らせ直していない（理由は表）。

### 手 6: 隙間
- 確かめた範囲: 回の助言ロックの接続が切れる（DB の再起動）—— design D1 の反転条件に既に書いてある。`idle_session_timeout` は 0（この DB で `SHOW`）なので、待っている間に切られることはない
- 同じ worktree で 2 つの `cargo test` が並ぶ —— 片付けは助言ロックを取れた回だけを消し、作成と片付けは `4820252` で 1 本ずつ。片付けの試験の材料を別の過程の片付けが消しても、3 本とも「消えている」を期待する側か、ロックを先に取る側なので落ちない（コードを読んだ。並べて走らせてはいない）
- 消す側の誤り: 片付けが消すのは `ashiato_run_<12 桁の 16 進>` で始まりロックの取れたものと、`_tmp_` を含み `ashiato_run_` で始まらないものだけ。`tools/` に `_tmp_` を含む database を作る台本は無い（grep。smoke は `restored`）
- 所有者 `ashiato_owner` は superuser でない（`pg_roles`）ので、superuser の backend が繋がっている database は `WITH (FORCE)` で消せない。R14
- 範囲外（CH04 の持ち込みではない）: 書き出しの試験は 1 回ごとに `$TMPDIR/ashiato-export-*` を残す（この席の回だけで 1111 個）。origin/main の試験にも片付けの行は無い（`remove_dir` の grep が 0）。CH03 の `HARNESS_TMP_LEAK` の側

### 手 7: 機械が承認した検証コマンドの訂正
該当なし（`python3 scripts/plan_fix.py pending openspec/changes/ch04-test-db-growth` が `[]`。10.2 の訂正は人間の承認（Q5））。

## R12. 「持ち主のいない回の database は、次の回の始まりに消える」を、回の始まりの片付けを外しても誰も落とさない（CI の growth も緑）
- 成果物: crates/server/src/testdb.rs / crates/server/src/testdb_tests.rs / tools/check-test-db-growth.sh / .github/workflows/ci.yml
- 根拠: `testdb.rs:287` の `sweep_previous_runs(&mut admin).await;` を消した木で `tools/check-test-db-growth.sh -p ashiato-server ch04_`（CI と 7.3 の形）→ **rc=0**・`check-test-db-growth: OK`・2 回とも 17 passed。
  同じ壊した木で片付けの試験を含まない絞り込み `ch04_testdb_` → rc=1・`前の回の database が 2 回目の後にも残っている: ashiato_run_f66a381d0fee / …_tpl`。
  つまり growth の判定は効くが、2 回目の中で `ch04_sweep_*` の 3 本が `testdb::sweep()` を直に呼び、1 回目の回を消してしまうので、回の始まりの片付けが無くても通る。
  `ch04_sweep_drops_an_orphan_run`（`testdb_tests.rs`）も `sweep()` の直呼びで、spec の「新しい回の片付け」＝ `create_run` の中の呼び出しを見ていない。全体（7.2 の形）も `ch04_sweep_*` を含む
  （壊した木で全体を 2 度走らせたが、1 度目は 1 回目の走りが R14 の不安定で、2 度目は 2 回目の走りが `archive_flow_tests::archive_youtube_last_event_is_reported_per_logical_source`
  （`archive_flow_tests.rs:5485` `left: None right: Some("2026-09-12")`。CH04 が触っていない試験）で rc=101 になり、どちらも判定まで届かなかった。全体でも同じ仕組みで覆われるのは、コードを読んだ上での推論）。
  振る舞い自体は壊れていない（壊していない木の全体 2 回で、前の回 `ashiato_run_5a9aa32a12f9` は実際に消えた）。覆われているのは検査の方
- kind: technical
- 処置: fixed 7.3 （ba9b44f7。台本が 2 回の走りに `-- --skip ch04_sweep_` を足す。`sweep_previous_runs` の呼び出しを `create_run` から外した木で `tools/check-test-db-growth.sh -p ashiato-server ch04_` が rc=1・`前の回の database が 2 回目の後にも残っている`、戻した木で rc=0 を実測。7.3・7.2 の証跡 PASS）
- 提案: growth の 2 回の走りから片付けの試験を外す（CI・7.2 の引数に `-- --skip ch04_sweep_`、または台本が自分で足す）。あるいは片付けの試験を、子の過程で `pool()` を初めて呼ぶ形（回の始まり）で見る

## R13. 問い合わせの上限が、書き出しの試験の接続（`TestDb::pool`）・`app_pool()`・環境変数の名前のどれから外れても、試験は全部緑
- 成果物: crates/server/src/testdb.rs / crates/server/src/testdb_tests.rs / openspec/changes/ch04-test-db-growth/specs/test-database/spec.md
- 根拠: `testdb.rs:341`（`TestDb::pool`）と `:500`（`app_pool`）の `&statement_timeout()` を `"0"` にした木で `cargo test -p ashiato-server ch04_` → 17 passed・`app_role` → 3 passed。
  `testdb.rs:89` の `ASHIATO_TEST_STATEMENT_TIMEOUT` を別の名前にした木で `CT ch04_statement_timeout_` → 3 passed、さらに `ASHIATO_TEST_STATEMENT_TIMEOUT=0` を渡しても `ch04_statement_timeout_default_is_set` が通る（戻した木では同じ命令が `left: "0" right: "1min"` で落ちる）。
  Requirement は「`pool()`・`app_pool()`・試験ごとと使い捨ての database」の全部に上限を置くと言うが、Scenario とテストは `pool()` だけ。依頼の 8 分の問い合わせは書き出しの問い合わせで、書き出しの試験は CH04 の後 `TestDb::pool` で繋ぐ —— 上限が最も要る経路が観測されていない
- kind: technical
- 処置: fixed D7 （ba9b44f7。`ch04_statement_timeout_default_is_set` が `pool()`・`app_pool()`・`isolated().pool()`・`scratch().pool()` の `SHOW statement_timeout` をいまの環境の上限と比べ、`ch04_statement_timeout_env_is_read` が子の過程に `ASHIATO_TEST_STATEMENT_TIMEOUT=7s` を渡して読み口を見る。`TestDb::pool` の上限を `"0"` にした木で `isolated().pool() に上限が入っていない`、環境変数の名前を替えた木で `ch04_statement_timeout_env_is_read` が落ちることを実測）
- 提案: `ch04_statement_timeout_default_is_set` に `isolated().pool()`・`scratch().pool()`・`app_pool()` の `SHOW statement_timeout` を足す。環境変数の読み口は子の過程（`Command` で自分の試験の実行ファイルを `--exact` で起こし、環境変数を渡す）で 1 本見るか、見ないことを spec に書く

## R14. `fresh_db()` の明示の `DROP DATABASE … WITH (FORCE)` が、Drop が待って打ち直すのと同じ拒否（42501）で unwrap して試験を落とす
- 成果物: crates/server/src/stay_tests.rs:1331-1334（同じ 15 行が clock_tests・feelings_tests・attributes_tests・places_tests にもある）/ crates/server/src/testdb.rs:375-395
- 根拠: R12 の壊した木（回の始まりの片付けを外しただけ。この経路には触れていない）で全体を走らせたとき、1 回目が rc=101: `stay_tests::deletion_ledger_migration_applies_twice` が `stay_tests.rs:1334:14` で
  `code: "42501", message: "permission denied to terminate process" … routine: Some("TerminateOtherDBBackends")`。`TestDb::drop` はこの拒否を 7 回待って打ち直す（`testdb.rs:375-395` の注記）が、
  `fresh_db()` の future は 1 回で unwrap する。所有者 `ashiato_owner` は superuser でない（`pg_roles`）。同じ拒否は 2.3 の証跡（`.superpowers/sdd/logs/20261024-170426-evidence-2.3-…log:24`）にも 1 回ある。
  壊していない木では、`stay_tests`・`clock_tests`・`feelings_tests`・`places_tests`・`attributes_tests` を 12 回ずつ（計 60 回）と全体 2 回で再現しなかった（稀）。final review の R8 は「重なりは害が無い」で退けたが、片方だけが打ち直しを持つので害がある
- kind: technical
- 処置: fixed D5 （ba9b44f7。5 つの `fresh_db()` の future は接続を閉じて `drop(scratch)` だけ。消すのは Drop だけで、拒まれたときの待ちと打ち直しを持つ。D5 の本文も直した）
- 提案: `fresh_db()` の future から明示の `DROP` を外して `drop(scratch)` だけにする（Drop が打ち直しと診断を持つ）。名前と戻り値の形（D5）は変わらない

## R15. 10.2 の検査は「期待値が変わっていない」を見ていない —— assert の期待値を書き換えて commit しても rc=0
- 成果物: tools/check-tests-kept.sh / openspec/changes/ch04-test-db-growth/tasks.md（10.2 と archive の判定）/ docs/changes/CH04.md（壊してはいけないもの）
- 根拠: 使い捨ての clone で `stay_tests.rs` の `assert_eq!(applied.unwrap(), 1, …)` を `2` にして commit → `tools/check-tests-kept.sh origin/main crates/server` が `[OK]`・rc=0。
  同じ clone で `// Scenario:` の行を 1 本消して commit → rc=1（印は見ている）。tasks 10.2 の本文と CH04.md の archive の判定は「試験の期待値・`Scenario:` の印・試験の本数が減っていない」を 10.2 が見ると書くが、見ているのは印の行と `fn` の名前だけ。
  なお、この branch が実際に期待値を変えた箇所は見つからなかった（`git diff origin/main...HEAD -- crates/server/src` で消えた `assert` の行は 1 つで、CH02 の ca072726 の `if erased` 包み）
- kind: technical
- 処置: fixed 10.2 （ba9b44f7。消えた `assert` を含む行が、足した行にも前後の空白を除いて同じものとしてあることを見る（期待値を替えれば落ち、字下げし直しただけなら通る。self-test 2 通り）。行をまたぐ assert の期待値の行は見ないと台本の冒頭に書いた。`tools/check-tests-kept.sh origin/main crates/server` rc=0）
- 提案: 期待値は機械で見ないと 10.2 と CH04.md に書き直すか、消えた `assert` の行（`assert` を含む `-` 行）が足した行にも一字一句あることを同じ台本で見る

## R16. `tools/check-tests-kept.sh` の説明の行が、`check_scenarios.py` に「spec に無い Scenario を指す印」として拾われる
- 成果物: tools/check-tests-kept.sh:5
- 根拠: `python3 scripts/check_scenarios.py . ch04-test-db-growth` の出力に `[warn] spec に無い Scenario を指す印: \`の行は、足した行にも一字一句同じものがあること。  (tools/check-tests-kept.sh)`。rc は 0。
  台本の `--self-test` は印の字を分けて組んで避けている（`local m="Scen""ario:"`）が、冒頭のコメントの 5 行目は避けていない
- kind: daily
- 処置: fixed D16 仮 （ba9b44f7。冒頭の説明を「Scenario の印」に書き換え、`check_scenarios.py` の warn が消えた）
- 提案: 5 行目の `` `Scenario:` `` を「Scenario の印」と書き換える

## security review（9874721..54990f7）

見た経路:
- 入口の増減: `crates/server/src/lib.rs` の差分は `#[cfg(test)] mod export_overlap_tests` / `export_window_tests` / `testdb_tests` の 3 つだけ。router・middleware・listen・依存（`Cargo.toml` / `Cargo.lock`）に差分なし（2）
- 書き出し（`dump.rs:138`）・台帳（`ledger.rs:431`）・件数（`summary.rs:92`）→ `excluded_sql` → `overlap_sql` の最後の `WHERE {OVERLAP_CONDITION}`（`excluded.rs:270-277`）→ 除く記録の集合。消した滞在が書き出しに戻るかはこの式だけで決まる（3）
- `OVERLAP_CONDITION` の 3 つの枝: 整った幅 × 整った消した時間（`erased_range` の `&&`）/ 全部の幅 × 整っていない消した時間（`erased_odd`）/ 整っていない幅 × 全部の消した時間。`start_at`・`end_at` が NULL の消した時間は `erased_range`（`start_at <= end_at`）にも `erased_odd`（`start_at > end_at`）にも入らず、枝 3 でも比較が NULL で落ちるので、`tstzrange(NULL, …)` が無限の範囲になって除く範囲が変わることは無い（3）
- `tstzrange` を作るのは `CASE WHEN w.event_time <= w.width_end` の中だけで、端末が送る `payload` の終わり（`range_end` など）が始まりより前でも式がエラーにならない（エラーになっても書き出しが止まる側で、除き漏れにはならない）（3）
- 試験の足場 `testdb.rs` は `#[cfg(test)] pub mod testdb`（`lib.rs:133-134`）で本番の組み立てに入らない。`sweep_previous_runs`（`testdb.rs:183-226`）が消すのは、助言ロックの取れた `ashiato_run_<16 進 12 桁>…` と、回の印の無い `*_tmp_*` だけで、`ashiato` / `postgres` / `template0/1` は名前で外す（3）
- 秘密: 所有者・アプリの URL は `url_from_env`（`testdb.rs:466-471`）で環境からだけ読み、panic の文言に値を出さない。新しい `eprintln!` は database 名・DB のエラー・`pg_stat_activity` の `usename/backend_type` だけで記録の中身は出ない（2）
- 台本: `tools/check-test-db-growth.sh` の `bash -c "$TEST_DB_CMD"` / `"$TEST_DB_LIST_CMD"`、`tools/export-scaling.sh` の `EXPORT_SCALING_CMD` は走らせる人の環境変数だけが入口で、外から届く値は入らない。CI の rust job の 3 行も既存の job の中（2）

安全の決め手: 「新しい重なりの条件（`OVERLAP_CONDITION`）は、CH02 の `EXISTS` と同じ記録・前の版を除く（除き漏れが無い）」 —— 確かさ 4。
1. `scripts/quiet-run sec -- cargo test -p ashiato-server --lib ch04_overlap` → `ch04_overlap_matches_ch02 ... ok`・`ch04_overlap_reads_erased_spans_linearly ... ok`・`3 passed`（本物の PostgreSQL で、新旧の除く id の集合を 4 つの写しの時点で突き合わせる）
2. 条件の式だけを総当たりで新旧比べる SQL（`/tmp/ch04_prop.sql`。時刻の候補は NULL・`-infinity`・`infinity`・境目・1 µs の差の 8 つ。消した時間は 1〜2 個の全部の組、幅は event_time 非 NULL × width_end の全部）を worktree の DB の `postgres` database で読むだけで走らせた → `cases=116480 | leak(旧が除き新が除かない)=0 | over(新だけが除く)=0`

指摘なし

範囲外: 差分の `span_next`（次の記録を窓で引く式）は CH02 の実装の取り込み（aa1cad15）で、CH02 の review が持つ。この段では見ていない。

## /code-review（9874721..54990f7）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R17. create_run は移行の前に hold_run_lock を呼ぶので、移行か権限の付与が落ちると、助言ロックを持つスレッドと接続、作りかけの <id>_tpl が過程の終わりまで残る。RUN の OnceCell は初期化に失敗した後も未設定のままなので、試験ごとに作り直しが起き、そのたびに積み増す。
- 成果物: crates/server/src/testdb.rs:286
- 根拠: 移行の 1 本が壊れた木で cargo test を走らせると、数百本の試験がそれぞれ create_run を呼び直す。過程が終わるまで、ロック用の接続 1 本と ashiato_run_*_tpl が 1 つずつ増え続け、max_connections を使い切る。その結果、本当の原因（移行のエラー）が接続の上限エラーに埋もれる（以前は試験ごとに移行のエラーで落ちるだけだった）。（確かさ 2）
- kind: technical
- 処置: fixed D2 （ba9b44f7。`create_run` は 2 度目に入ると作らずに「最初に落ちた試験の出力を見る」と示して落ちる（`RUN_ATTEMPTED`）。ロックの接続と `_tpl` の残りは 1 組だけで、次の回の片付けが消す）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R18. 片付けは、名前に `_tmp_` を含み ashiato_run_ で始まらない database を、持ち主の有無を見ずに DROP … WITH (FORCE) で消す。回の印の無い古い書き方は、どの過程が今使っているのか区別できない。
- 成果物: crates/server/src/testdb.rs:212
- 根拠: CH04 を rebase していない Story のブランチ（fresh_db が st16_tmp_… を作る）が、同じ DB サーバで試験を走らせている最中に、CH04 の後の試験の過程が始まるか sweep() の試験が走る。すると使用中の st16_tmp_… が強制的に消え、相手の試験が接続断で落ちる。他人の試験が理由なく落ちたように見える。（確かさ 2）
- kind: technical
- 処置: rejected: 走っている他の branch と同じ DB サーバを共有しない。試験の DB の port は worktree ごとに `tools/ports.sh` が決める（`crates/server/src/testdb.rs:25` の `db_port()`）ので、CH04 を rebase していない別の Story の `st16_tmp_…` は別の容器にある。同じ worktree で古い build と並べる場合は design.md:116 の D6 の反転条件（作ってから 1 時間より古い `_tmp_` だけを消す）が既に決めている
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R19. ch04_statement_timeout_default_is_set は、環境変数 ASHIATO_TEST_STATEMENT_TIMEOUT が無い前提で SHOW statement_timeout == "1min" を固定している。pool() はこの環境変数を読むので、変数を置いた環境では試験が落ちる。
- 成果物: crates/server/src/testdb_tests.rs:391
- 根拠: 長い試験のために ASHIATO_TEST_STATEMENT_TIMEOUT=0（tools/export-scaling.sh が渡す値と同じ）や 5min を export したまま cargo test --workspace を走らせると、shown が "0" や "5min" になり assert が落ちる。既定値を確かめる試験は、pool_with_statement_timeout(&statement_timeout_from(None)) で見るべき。（確かさ 2）
- kind: technical
- 処置: fixed D7 （ba9b44f7。既定は `pool_with_statement_timeout(&statement_timeout_from(None))` で見て `1min`、足場の接続はいまの環境の値で繋いだ接続の `SHOW` と比べる。`ASHIATO_TEST_STATEMENT_TIMEOUT` を置いた環境でも落ちない）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R20. 積もりの検査は cargo test にフィルタ（CI では ch04_）を渡すが、1 本以上走ったかを確かめない。0 本でも rc=0 で、database も増えないので検査は素通りする。
- 成果物: tools/check-test-db-growth.sh:74
- 根拠: AGENTS.md の規則「`cargo test <名前>` 型は `-- --list` で 1 本以上あることも確かめる（0 本でも rc=0）」に反している。後で試験名の ch04_ 接頭辞を変えると、CI の `check-test-db-growth.sh -p ashiato-server ch04_` は試験を 0 本走らせて「OK（残りは最後の回の分だけ）」と出し、積もりの退行を見逃す。（確かさ 2）
- kind: technical
- 処置: fixed 7.1 （ba9b44f7。台本が先に `cargo test … -- --list` で数え、0 本なら `走る試験が 0 本` で rc=1。`tools/check-test-db-growth.sh -p ashiato-server no_such_test_` で rc=1 を実測。self-test に 0 本の場合を足した）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R21. 回が走っている印は、postgres database に繋いだ 1 本の接続が持つ助言ロックだけ。その接続がサーバ側で切れても誰も気付かず、作り直しもしない。
- 成果物: crates/server/src/testdb.rs:163
- 根拠: 試験の途中で回のロック用の接続が切れる（pg_terminate_backend、DB の再起動、idle_session_timeout など）。すると同じサーバで次に始まる過程の sweep_previous_runs が、pg_try_advisory_lock を取れてしまう。その結果、走っている回の <id>・<id>_tpl・<id>_x_… を FORCE で消し、残りの試験が全部接続断で落ちる。（確かさ 2）
- kind: technical
- 処置: rejected: design.md:55 の D1 の反転条件が、回のロックの接続が途中で切れる場合をいまは扱わないと決めている（DB の再起動の間の試験はどれにせよ落ちる）。`idle_session_timeout` はこの DB で 0（code_verify の手 6 で `SHOW` 済み）なので、待っている間には切られない。`pg_terminate_backend` は人の手でしか起きない
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R22. fresh_db が stay・attributes・clock・feelings・places の 5 ファイルに同じ形で複製されている。どれも、TestDb の Drop が既に打つ DROP DATABASE IF EXISTS を、所有者のプールを別に開いて二重に打つ。
- 成果物: crates/server/src/stay_tests.rs:1322
- 根拠: fresh_db を呼ぶたびに、回の database へのプール（admin）を余分に 1 本開き、試験の終わりまで持つ。片付けの規則を変えるときは 5 か所を揃えて直す必要がある。testdb に `scratch_pool() -> (PgPool, TestDb)` を 1 つ置き、closer は `closer.close().await; drop(scratch)` だけにすれば足りる。（確かさ 2）
- kind: technical
- 処置: fixed D5 （ba9b44f7。R14 の直しで二重の DROP と余分の所有者のプールは無くなった。5 ファイルに同じ形が並ぶことは直さない —— design.md:97 の D5 が `fresh_db()` の名前と形を各ファイルに残すと決めている。final の R8 と同じ判断）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R23. isolated()・scratch()・app_pool() は、回の database を作らせるためだけに drop(pool().await) で使い捨てのプールを開いて閉じる。
- 成果物: crates/server/src/testdb.rs:423
- 根拠: 試験ごとの database を取るたびに、所有者の TCP 接続と認証が 1 往復余分にかかる。export の試験や ch04_isolated_many_in_parallel（16 本）で、その分だけ遅くなる。`RUN.get_or_init(create_run).await` を直接呼べば同じことが接続なしでできる。（確かさ 2）
- kind: technical
- 処置: fixed D4 （ba9b44f7。`isolated()`・`scratch()`・`app_pool()` は `RUN.get_or_init(create_run).await` を直に呼び、使い捨てのプールを開かない）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R24. run_id_of は is_ascii_hexdigit で大文字の 16 進も回の id として受けるが、検査の台本の回の id の正規表現（^ashiato_run_[0-9a-f]{12}）と self-test は小文字だけを回の id とする。2 つの定義が食い違っている。
- 成果物: crates/server/src/testdb.rs:176
- 根拠: ashiato_run_ABCDEF012345 という database があると、片付けはこれを回とみなし、ロックを取れれば消す。一方で台本はこれを回の id と数えない（self-test は「回の id でない」として rc=0 を期待している）。回の id の定義が 2 か所で違うので、検査が見る集合と片付けが消す集合がずれる。is_ascii_hexdigit を小文字の 16 進だけに絞れば揃う。（確かさ 2）
- kind: technical
- 処置: fixed D1 （ba9b44f7。`run_id_of` は `0-9a-f` だけを受ける。`ch04_run_id_is_lowercase_hex` が大文字・16 進でない字・桁の足りない名前を回の id と読まないことを固定）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R25. 書き出しの試験の lint は、字のとおりの `testdb::pool()` しか見ない。testdb::app_pool()・testdb::pool_with_statement_timeout(…)・`use crate::testdb::pool;` の後の pool() は、どれも回の database を共有するのに素通りする。
- 成果物: tools/check-test-db-growth.sh:37
- 根拠: export_*_tests.rs に `let p = testdb::pool_with_statement_timeout("5s").await;` を書くと、回の database の消した滞在を共有したまま lint を通る。CH04 が塞いだはずの、試験どうしの混ざりが黙って戻る。（確かさ 2）
- kind: technical
- 処置: fixed 6.2 （ba9b44f7。lint は `testdb::(pool|app_pool|pool_with_statement_timeout|url)` と、それらを `use … testdb::{…}` で取り込む行を見る。self-test に 5 通りと、`testdb::isolated()` と `TestDb::pool()` を通す 1 通りを足した。6.2 の証跡 PASS）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R26. TestDb の Drop は、async の runtime のスレッドの上で std::thread の join を同期に待つ。DROP のやり直しは最長で約 9.5 秒に、接続の時間が加わる。current_thread の #[tokio::test] では、その間に spawn した仕事（書き出しの job など）が一切進まない。
- 成果物: crates/server/src/testdb.rs:446
- 根拠: export_job_tests で app が spawn した書き出しの job が、終わり際にプールの接続を持っている。その状態で db が drop されると、runtime が止まったまま FORCE で切られ、やり直しの待ちが重なる。試験 1 本ごとに最大で 10 秒近く runtime を塞ぐので、試験の時間が延び、時間の上限で落ちる試験が出うる。（確かさ 2）
- kind: technical
- 処置: rejected: design.md:82 の D4 が、Drop は自分のスレッドで DROP を打ってそのスレッドを待つと決めている（待たないと、次の試験と片付けが消しかけの database を見る）。待ちが伸びるのは `42501` で拒まれたときだけで、それは稀（code_verify は 5 ファイル 60 回と全体 2 回で再現しなかった）。拒まれなければ DROP 1 本で戻る。Drop は試験の終わりに走るので、塞ぐのは終わりかけの試験の runtime だけ
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## finish の scoped re-review（ba9b44f7）

席: `pr-review-toolkit:code-reviewer`（直した文脈から独立。`git show ba9b44f7` だけを見た。cargo test は走らせていない）。
fixed の 13 件（R11〜R17・R19・R20・R22〜R25）はどれも ADDRESSED。新しい Critical / Important なし。
Minor 4 件は 2 回目の fix をせずに park する（SDD の final review と同じく fix は 1 回）:
- `count_tests`（`tools/check-test-db-growth.sh:72`）: `--list` の stderr を捨てるので、ビルドが落ちると「絞り込みが合っていない」と出る（直前の `cargo test --no-run` が `set -e` で先に止めるので起きにくい）。`#[ignore]` の試験も数える（ignore だけに当たる絞り込みで 0 本を見逃す）
- `ch04_statement_timeout_env_child`（`crates/server/src/testdb_tests.rs:450`）: `-- --include-ignored` で走らせると `CH04_EXPECT_STATEMENT_TIMEOUT` の unwrap で落ちる（CI と台本は使わない）
- R25 の lint は行をまたぐ `use crate::testdb::{\n pool,\n}` を拾わない
- `check-tests-kept.sh` の assert の比べ方は `assert` を含む行を全部見る（`debug_assert`・説明の行も）。正当な書き換えを通す印は無い（CI では走らず 10.2 の 1 回の検証だけ）
