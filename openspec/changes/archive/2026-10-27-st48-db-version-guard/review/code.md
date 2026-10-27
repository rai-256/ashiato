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

## code_verify（HEAD `5e9836fe`。2026-10-25 実測。作業ツリーは差分なし）

席: code_verify（`final` の後・`finish` の前）。手 1〜7 を全部。壊したものはすべて `git checkout` で戻し、壊すたびに前後で `git status --porcelain` と `git diff` が同じ（空）であることを確かめた（15 回とも `restored`。版の検査を bind の後へ移す 1 通りは、1 回目が書き換えの側のコンパイル誤りで検査にならなかったので組み直して走らせ直した）。
検証コマンドは `scripts/verify-run` を使わず（evidence.jsonl を書き足さないため）、tasks.md の本文のとおりに直に走らせた。

### 申告と実測

申告: tasks.md の `[x]` は **16/16**（1.1〜9.1。`[ ]` は 0）。

| 項目 | 検証コマンド（tasks.md の本文） | 実測 | 一致 |
|---|---|---|---|
| 1.1・2.1 | `cargo build -p ashiato-server --tests` | rc=0 | 一致 |
| 1.2 | `CT`/`NL` `st48_ledger_`（≥2）・`st48_compare_`（≥4）・`st48_read_ledger_unreadable_`（≥3） | rc=0。2 passed / 5 passed / 3 passed | 一致 |
| 2.1・9.1 | `cargo clippy -p ashiato-server --all-targets -- -D warnings` / `cargo clippy --workspace --all-targets -- -D warnings` | rc=0 / rc=0 | 一致 |
| 2.2 | `CT`/`NL` `st48_apply_`（≥8） | rc=0。9 passed | 一致 |
| 3.1 | `! grep -nE "for .* in crate::MIGRATIONS" testdb.rs` / `CT stay_tests` / `CT ingest` | rc=0 / 65 passed / 118 passed | 一致 |
| 3.2 | `CT`/`NL` `st48_testdb_has_ledger`・`st48_export_starts_with_ledger`（各 ≥1） | rc=0。1 / 1 passed | 一致 |
| 4.1・5.1 | `cargo build … --bin ashiato-server` / `! grep -nE "MIGRATE_.*(GUARD\|FORCE\|SKIP)\|ALLOW_DB_AHEAD"` | rc=0 / rc=0 | 一致 |
| 4.2 | `IT`/`NI` `st48_migrate_`（≥10） | rc=0。10 passed | 一致 |
| 5.2 | `IT`/`NI` `st48_server_`（≥6）/ `IT server_startup_` | rc=0。6 passed / 9 passed | 一致 |
| 6.1 | `check-released-migrations.sh --self-test` / 本体 | rc=0（7 Scenario）/ rc=0（「配った印が 0 個」） | 一致 |
| 6.2・7.1・8.1・8.2 | `grep -q` の組 4 本 / `bash -n` | どれも rc=0 | 一致 |
| 7.1 | `check-migrations.sh --self-test` / 本体 | rc=0 / rc=0 | 一致 |
| 8.2 | `scripts/quiet-run dbstop -- tools/check-db-version-stop.sh` | rc=0（48 秒） | 一致 |
| 9.1 | `check_scenarios.py . st48-db-version-guard` / `tools/smoke.sh` / `tools/check-immutable.sh` | rc=0（scenarios: OK。41 本）/ rc=0（100 秒）/ rc=0（87 秒） | 一致 |

`cargo test --test <名前>` 型は `--test server_startup` だけで、`crates/server/tests/server_startup.rs` は実在する。

### 手 1: 固定値を独立に再計算する

- **指紋**: `testdb::pool()` の DB（CT の走行が当てたもの）の `migration_ledger.applied` の 31 行の `sha256` を、python の `hashlib.sha256(migrations/<名前>.sql のバイト列)` と突き合わせた → **31/31 一致**、ORDER との過不足なし。試験の `sha256_hex` は実装と同じ `sha2` crate なので、これが唯一の独立の検算
- **組み立ての commit**: `ASHIATO_BUILD_COMMIT=verify-24262 cargo build -p ashiato-server --bin ashiato-server` で作った実体で、使い捨ての database に `migrate` → 31 行とも `build_commit = verify-24262`（振る舞いは spec どおり）。環境変数なしで build し直し、実体から値が消えたことを `strings` で確かめた。ただしこの主張を観測する試験は無い → **R20**
- 同じ DB の行の `build_commit` は `651f049b`（「Task 1 を完了にする」）で、HEAD `5e9836fe` から 38 commit 古い（祖先）。D8 が許す「差分のビルドでは古いまま残る」が、この worktree で実際に起きている。R4 で rejected 済みなので重ねない（D8 の反転条件の材料として残す）
- `st48_ledger_shape_is_frozen` の期待の列の名前・型・NULL 可否・主キー・`CHECK (only_row)` は、spec の Requirement の表と目で一字ずつ突き合わせて一致

### 手 2: ガードをわざと壊す

| 壊したもの | 実行 | 結果 |
|---|---|---|
| `migration_ledger.rs:314` の `pg_advisory_lock` を `SELECT $1::bigint` に | `CT st48_apply_concurrent_runs_once` | 1 回目で落ちた（`migration_ledger_tests.rs:442`） |
| `migration_ledger.rs:337` の前で `before_apply()`（退避）を先に呼ぶ | `IT st48_migrate_refusal_takes_no_backup` | 落ちた |
| `compare`（`migration_ledger.rs:164`）で `Behind` を先に返す | `CT st48_compare_` | 落ちた（`st48_compare_ahead_wins_over_behind`） |
| `check-released-migrations.sh:37` の中身の差分の分岐を `elif false` に | `--self-test` | rc=1（「中身を変えると落ちる」が `FAIL: rc=0`） |
| `check-migrations.sh` の `check_new_rules` を常に `return 0` に | `--self-test` | rc=1（txn・noledger とも落ちない） |
| **版の検査（`lib.rs:3624`）を待ち受けの `bind`（`lib.rs:3695`）の後へ移す**（定期取得・書庫の起動よりも後になる） | `IT st48_server_` を 3 回 | **落ちない**（3 回とも 6 passed）→ **R19** |
| **CI の印の取り寄せで `origin` を読めない**（使い捨ての bare repo に `v0.1.0` を付けて中身を変え、浅い clone の `origin` の URL を壊す） | `ci.yml:79-84` の手順そのまま | **rc=0「配った印が 0 個」**（`origin` が読めるときは rc=1）→ **R18** |

確かめて指摘にしないもの: 新しい版の規則は `commit;`（小文字）・`begin work; … end;`・末尾の `;` の無い `COMMIT`・`COMMIT AND CHAIN` を落とす。`ABORT;` は通る（spec の列挙 5 語に無い）。コメントの中の `DELETE` と `$fn1$` は R13 で rejected 済み。

### 手 3: Scenario と test を突き合わせる

`check_scenarios.py . st48-db-version-guard` rc=0（41 本とも印あり。warn の `migrate_backup.rs` の古い印は ST48 の差分の外）。印の先を 1 件ずつ読んだ。

- 「版の記録の 1 行は…commit が組み立てに**渡した** `ASHIATO_BUILD_COMMIT` の値と一致する」→ 試験は `env!("ASHIATO_BUILD_COMMIT")`（行を書いた定数そのもの）と比べている → **R20**
- 「版の検査を、待ち受けを開く前に行う」（Requirement）と拒否の 4 本の「待ち受けを開かずに」→ 試験は「`/healthz` が 200 を返さなかった」で、開いたかどうかを見ていない → **R19**
- 「空の DB に移行を当てると…」「版の記録の無い古い DB には…終了コードは 0」は WHEN が `ashiato-server migrate` だが、試験は lib の `apply_migrations` を呼ぶ（退避・付与・終了コードを通らない）。実体で確かめた: 記録の schema を消して `core.event` に 1 件入れた使い捨ての database に `migrate` → **rc=0、31 行とも `before_ledger = t`、1 件が残る**。振る舞いは合っているので R20 の提案に添えるだけにする
- 「当てる版が無い回にも、当てる前の退避を取る」の THEN「**空でない**ファイル」は数だけを見る（偽の `pg_dump` は `fake-dump` を書くので空ではない）。軽微なので指摘にしない
- `st48_export_starts_with_ledger` は書き出しを 1 回走らせ、zip の名前に `migration_ledger` を含むものが無いことを assert している（主張と同じ階層）

### 手 4: 本人の決定が test で固定されているか

| 決定 | 書き換え | 結果 |
|---|---|---|
| Q1 まだ当たっていない版だけ当てる | （既存）`st48_apply_does_not_rerun_recorded`・`st48_migrate_rerun_changes_nothing` が直書きで見る | 固定されている |
| Q1 配った移行を変えると CI が落ちる | 上の手 2 の `elif false` | 落ちた |
| Q2 記録の無い DB は全部に「記録より前」 | `migration_ledger.rs:341` の `true` → `false` | 落ちた（`st48_apply_pre_ledger_db_marks_all`） |
| Q3 まだの版があれば起動しない | `startup_refusal` の `Behind` で `None` を返す | 落ちた（`st48_server_refuses_db_behind`） |
| Q4 終了コード 3（migrate） | `lib.rs` の `run_migrate` の `exit(3)` → `exit(4)` | 落ちた（4 本） |
| Q4 終了コード 3（起動） | `run()` の `exit(3)` → `exit(4)` | 落ちた（4 本） |
| Q5 確認バッチは回ごとに別の DB | `verify-prep.sh` の `project=` から tag を外す | 落ちた（`check-db-version-stop.sh` rc=1「project 名が tag を含まない」） |
| Q5 止める仕組みを外す抜け道を作らない | — | 試験は `MIGRATE_BACKUP`・`MIGRATE_BACKUP_DIR`・`PG_DUMP` の 3 つを見る。ほかの名前の抜け道は 4.1 の grep の型（`MIGRATE_.*(GUARD\|FORCE\|SKIP)\|ALLOW_DB_AHEAD`）に掛かる名前しか見ない（名前で探す以上の機械の検査は作りようが無いので指摘にしない） |
| spec の境 `202610250000`（D10） | `NEW_RULES_FROM=202610241300` に下げる | **落ちない**（`--self-test` と本体とも rc=0）。self-test の版は `202610250001` なので上げる側の 1 分しか固定せず、今の木の最新の版は `202610211000` —— その間のどの値でも全部通る。境ちょうどは R13 で rejected 済みなので R にはしない |
| （設計の値）`LOCK_KEY = 4_820_251`（D11） | `1` に変える | 落ちない（`st48_` の lib 20 本・`server_startup` 16 本とも緑）。testdb ももう同じ関数で錠を取るので、鍵の値は振る舞いに出ない。本人の決定ではないので R にしない |
| `DB_LINES_MAX = 20`・`FORM = 1`・頭の形 `error: kind=` / `  db: ` / `  more: ` / `  self: ` | — | 試験が直書きで比べている（`st48_ahead_report_caps_db_lines_and_counts_the_rest`・`st48_ledger_shape_is_frozen`・`st48_migrate_refuses_db_ahead`） |

### 手 5: tasks の `[x]` と実体

16 項目の検証コマンドは全部実在し、上の表のとおり rc=0。名指しされた試験の名前（`st48_*`）は全部実在し、本数の下限も満たす。該当なし。

### 手 6: 隙間

- **migrate が終了コード 3 で止まった回に退避が無いことと、動いている古いサーバ** → **R21**（ST58 への defer）
- 記録の無い古い DB の実体での migrate（上の手 3）: rc=0・記録 31 行・本人の記録 1 件が残る。捨てない
- 試験が `.down.sql` を当てる経路（`places_tests`・`clock_tests`・`media_migration_tests`・`feelings_tests`・`registry_tests`）を読んだ: どれも使い捨ての DB か、戻す取引の中。共有の DB の記録と実体が食い違う経路にならない。`check-immutable.sh` は終わりに `docker compose down -v` で DB ごと捨てる
- tracing の出力先は `tracing_subscriber::fmt()`（`lib.rs:262`・`3571`。既定の stdout、切り替えの設定なし）なので、標準エラーの 1 行目が `error: kind=` であることは運用の設定で崩れない

### 手 7: 機械が承認した検証コマンドの訂正

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st48-db-version-guard` → `[]`）。

## R18. CI の印の取り寄せが `git ls-remote` の失敗を「印が 0 個」と区別せず、配った移行の凍結の検査が緑になる
- 成果物: .github/workflows/ci.yml / tools/check-released-migrations.sh
- 根拠: `ci.yml:79-82` は `if [ -n "$(git ls-remote --tags origin 'refs/tags/v*')" ]` で、`ls-remote` の失敗（rc=128）も空の出力になり、取り寄せを飛ばす。使い捨ての bare repo（`v0.1.0` の後に版の中身を変えた）で再現: `origin` が読めるとき `check-released-migrations.sh` rc=1（「中身が印 v0.1.0 の版から変わっている」）、`origin` の URL を壊すと **rc=0「配った印が 0 個」**。spec「配った印の版を読めなければ、検査を通さない」の外側（印の一覧そのものを読めない）。いまは origin に印が 0 個（`git ls-remote --tags origin` rc=0・出力なし）で検査が働いていないが、ST52 が印を付けた時から効く
- kind: technical
- 提案: `tags="$(git ls-remote --tags origin 'refs/tags/v*')" || exit 1` のように取り寄せの失敗で落とす。`check-released-migrations.sh` に「印の一覧を読めない」場合の self-test を 1 つ足す
- 処置: fixed D9 —— CI の取り寄せを `tags="$(git ls-remote …)" || exit 1` にし、読めないときは「印の一覧を origin から読めない」で落とす。使い捨ての repo の `origin` を壊して rc=1 を確かめた。D9 に 1 行足した

## R19. 「版の検査を、待ち受けを開く前に行う」を観測する試験が無い
- 成果物: crates/server/tests/server_startup.rs / crates/server/src/lib.rs
- 根拠: `lib.rs:3624` の `startup_refusal` を `lib.rs:3695` の `TcpListener::bind` の後（`scheduled::start`・書庫の起動よりも後）へ移した木で `cargo test -p ashiato-server --test server_startup st48_server_` を 3 回 → **3 回とも 6 passed**（戻した後 `git status`・`git diff` は空）。`assert_refused_3`（`server_startup.rs:923-933`）は「`/healthz` が 200 を返さなかった」を見るだけで、bind の直後に exit(3) すれば HTTP は返らないので区別できない。いまのコードの順は正しい（3624 → 3695）
- kind: technical
- 提案: 拒否の試験の 1 本を、`BIND` に**先に塞いだ port** を渡して起動する形にする。検査が先なら rc=3 と `error: kind=`、bind が先なら bind の誤り（rc≠3）になり、順が観測できる（`lib.rs:3601` の `net_guard::check_bind` は名前を引いて loopback かを見るだけで、port を開かない —— `net_guard.rs:48-51`）
- 処置: fixed 5.2 —— 拒否の 4 本（db_ahead・db_behind・記録の無い DB・読めない）を、待ち受けの port を先に塞いだまま起動する `Rig::serve_on_held_port` にした。検査を `bind` の後へ移した木（`pool.clone()` で組めるようにした）で `IT st48_server_` → 4 本とも終了コード 3 でなく落ちた（戻した木で 16 passed）

## R20. 「commit が組み立てに渡した `ASHIATO_BUILD_COMMIT` の値と一致する」を観測する試験が無い（書いた定数と同じ定数を比べている）
- 成果物: crates/server/src/migration_ledger_tests.rs / crates/server/build.rs
- 根拠: `st48_apply_row_has_fingerprint_and_build`（`migration_ledger_tests.rs:313`）は行を `env!("ASHIATO_BUILD_COMMIT")` と比べ、行もその定数（`migration_ledger.rs:19`）で書かれる。`build.rs:77` を `env::var("ASHIATO_BUILD_COMMIT_IGNORED")` に替え（渡した値を無視する）、`ASHIATO_BUILD_COMMIT=injected-xyz cargo test -p ashiato-server --lib st48_apply_row_has_fingerprint_and_build` → **1 passed**（環境変数がある回は git の HEAD との比較も飛ばす）。振る舞いそのものは合っている（手 1: 注入した `verify-24262` が 31 行に入る）。あわせて「空の DB」「記録の無い古い DB」の 2 本は WHEN が `ashiato-server migrate` だが lib を呼ぶ（実体では rc=0・31 行 `before_ledger = t`・1 件残るを確かめた）
- kind: technical
- 提案: 試験の走る環境に `ASHIATO_BUILD_COMMIT` があるときは「行の値 == `std::env::var("ASHIATO_BUILD_COMMIT")`（走らせた時に渡した値）」を assert する（`rerun-if-env-changed` で定数も同じ値に組み直される）。CI で 1 回だけ乱数を渡してこの 1 本を走らせる
- 処置: fixed D8 —— 渡した値を `ASHIATO_EXPECT_BUILD_COMMIT` で受けて行の値と比べる（cargo は build script の `rustc-env` を試験の実行時の環境にも入れるので、`ASHIATO_BUILD_COMMIT` を実行時に読むと組み立て済みの値になる —— 実測で `given` が HEAD だった）。同じ理由で走らなかった HEAD の祖先の比較は、commit が 40 桁の 16 進のときに走る形に直した。`build.rs` が渡した値を無視する木で落ち、戻した木で通る。CI に乱数を両方に渡す 1 本を足した

## R21. 終了コード 3 の回は退避を取らないという契約が ST58（自動の戻し）に渡っておらず、動いている古いサーバは DB が新しくなっても止まらない
- 成果物: docs/handoff/（ST58 が無い）/ docs/stories/ST58.md
- 根拠: `st48_migrate_refusal_takes_no_backup` が通る（rc=3 の回は `pre-migrate-*` を作らない）。ST58 は FR-119「更新の中で移行が失敗する THEN 移行の前の退避から D-01 を戻す」を持ち、`requires: [ST57, ST48, ST51]` だが、`docs/stories/ST58.md` に終了コード 3・`kind=` の記述は無く（`grep -n "終了コード 3\|kind=db_ahead\|ST48"` → `requires` の行だけ）、`docs/handoff/ST58.md` も無い（ST52 には D8・D9 の handoff がある）。rc≠0 を一律に「移行の失敗」と扱うと、この回の退避は無いので前の回の退避（それ以後の記録を含まない）を戻しうる —— 捨てたものは復元できない型。また使い捨ての database で `migrate` → アプリの役割でサーバを起動（`/healthz` 200）→ 所有者で知らない版の行を足す → 2 秒後も `/healthz` 200・プロセス生存（spec どおり検査は起動のときだけ）。更新の手順でサーバを止めてから migrate することも ST58 が持つ
- kind: defer
- 提案: `docs/handoff/ST58.md` を置き、「終了コード 3 は移行の失敗ではない（何も書いていない・この回の退避は無い）。退避から戻さず、前の版のまま起動し直す」「新しい版の migrate の前に古い S-01 を止める」を渡す
- 処置: deferred ST58 —— `docs/handoff/ST58.md` に「終了コード 3 は移行の失敗でない・この回の退避は無い・退避から戻さない」「migrate の前に古い S-01 を止める」を書いた（ST58 は tasks.md が無い）

## security review（6bb4584..5e9836f）

見た経路:
- `ashiato-server migrate`（所有者の接続）→ `lib.rs:286` `migrate_after` → `migration_ledger.rs` `apply_migrations_with`（助言錠）→ `read_ledger_on` → ahead・読めないなら退避も書き込みも無しで `eprint!` + 終了コード 3。標準エラーに出るのは種別・版の名前・組み立ての版番号と commit・固定の理由文だけ（`migration_ledger.rs:56-84`）で、記録の中身（URL・位置・本文）に触れる問い合わせは無い（3）
- `ashiato-server`（`run()`・アプリの役割）→ `lib.rs:3624` `startup_refusal` → 読むだけ（`pg_namespace`・`information_schema.columns`・`migration_ledger.*` の SELECT）→ 食い違えば `lib.rs:3695` の `TcpListener::bind` より前に終了コード 3。新しい API・画面・ルートは足していない（3）
- 版の記録の置き場 `migration_ledger.sql` → 所有者が作り、アプリの役割には `USAGE` + `SELECT` だけ。`grants.sql` の `ALTER DEFAULT PRIVILEGES … IN SCHEMA core` はこの schema に届かない（4、下の台本）
- 指紋の警告 `warn_changed_fingerprints` → `tracing::warn!` に版の名前だけ（2）
- `build.rs` → `ASHIATO_BUILD_COMMIT` / `git rev-parse HEAD` を組み立てに焼き、DB の `build_commit` と標準エラーへ。秘密ではない（2）
- `tools/verify-prep.sh:18` → `run.sh` の `COMPOSE_PROJECT_NAME=ashiato-verify-<tag>` → `stack.sh up`（`STACK_RESET` 無し）→ `stack.sh:40` の `down` は `stop` だけ → 回ごとの DB の置き場は消えずに残る（3。R22）
- CI の `git ls-remote` / `git fetch` のタグ → `check-released-migrations.sh` の `git show "$tag:…"`（`v[0-9]*` に絞って引用付き。シェルへの差し込みは無い）（2）
- 依存: `tempfile = "3.27.0"`（`[dev-dependencies]`。Cargo.lock に `tempfile` `fastrand` `rustix 1.1.5` `linux-raw-sys 0.12.1` が増えた。どれも既知の正規の crate で、本番の組み立てには入らない）（2）

安全の決め手: **アプリの役割（`ashiato_app`）は版の記録を書けず、記録の置き場を先回りで作ることもできない** —— 確かさ 4。
これが崩れると、乗っ取られたサーバが未来の版の名前を `applied` に入れて、次の migrate に門の版（`core.reject_collected_rewrite` など）を飛ばさせるか、永久に終了コード 3 で止めさせられる。
既存の `st48_ledger_app_role_reads_only` は INSERT / UPDATE / DELETE だけを見ているので、残りを使い捨ての台本で走らせた（所有者で `CREATE DATABASE st48sec_<乱数>` → アプリの役割で `CREATE SCHEMA migration_ledger` → 本物の `target/debug/ashiato-server migrate`（HEAD 後の build）→ アプリの役割で下の 5 文 → `DROP DATABASE`。psql は `docker exec ashiato2-st48-db-1` 経由。台本は repo の外に置き、走らせた後に消した）:
```
-- real ashiato DB: app has CREATE on database? / on schema public?
f|f
-- fresh DB (before migrate): app tries to pre-create the ledger schema
ERROR:  permission denied for database st48sec_326441832
migrate rc=0   / applied 31 行
TRUNCATE migration_ledger.applied => ERROR:  permission denied for table applied
CREATE TABLE migration_ledger.x(i int) => ERROR:  permission denied for schema migration_ledger
DROP TABLE migration_ledger.applied => ERROR:  must be owner of table applied
INSERT INTO migration_ledger.applied(...) VALUES ('299901010000_x',...) => ERROR:  permission denied for table applied
ALTER TABLE migration_ledger.applied ADD COLUMN z int => ERROR:  must be owner of table applied
migration_ledger|ashiato_owner / form|ashiato_owner / applied|ashiato_owner
```

## R22. 確認バッチの DB が回ごとに残り続け、電話から届いた記録の写しが回の数だけ積もる。本体の DB で消した記録も、前の回の DB からは消えない
- 成果物: tools/verify-prep.sh, tools/stack.sh, openspec/changes/st48-db-version-guard/design.md
- 根拠: `verify-prep.sh:18` が回ごとに `ashiato-verify-<tag>` の project を決め、`run.sh` に `export COMPOSE_PROJECT_NAME` を書く → `stack.sh up` は `STACK_RESET` が無いので `down -v` しない（`stack.sh:77-79`）→ `stack.sh down` は `docker compose stop` に替わった（`stack.sh:40-41`）→ 別の project の DB も `docker stop` だけ（`stack.sh:80-86`）。`tools/` `scripts/` に `ashiato-verify` の置き場を消す道具は無い（`grep -rn ashiato-verify tools scripts` は `verify-prep.sh:18` だけ）。
  design の Risks（`design.md:211`）が「確認の間に電話が送った記録は、その回の DB にだけ入る」と書くとおり、確認バッチには本物の端末の記録（位置など）が届く。前は既定の project の 1 つの DB に入っていたので、削除（`deletion.rs`）が届く写しは 1 つだった。
  いまは回の数だけ無名の volume に残り、`COMPOSE_PROJECT_NAME=<前の名前> docker compose up -d db`（`design.md:178` が案内する手順）で、本体で消した記録が画面ごと戻る。D13 の反転条件（`design.md:180`）は「数十 GB」「片付けの手間」だけで、消した記録が残ることを条件に持たない（確かさ 3。この端末にはまだ `ashiato-verify` の volume は 0 個で、実物では再現していない）
- kind: premise
- 提案: 消した記録が前の回の DB に残ることを design の Risks に書き、D13 の反転条件か片付けの change の起点に「本体で削除したら、残っている回の DB をどうするか」を足す。片付けを後に回すなら、確認手順書（manifest）に「この回の DB は消えずに残る・消すなら `COMPOSE_PROJECT_NAME=<この回> docker compose down -v`」の 1 行を載せる

範囲外: なし
- 処置: escalated —— `deep.md` Q6（第 2 回）で答えが出た。本人が推奨（前の回の DB は残し、消し方を手順書に載せる）を選んだ。`verify-prep.sh` の manifest に「この回の DB は消えずに残る・消すなら `COMPOSE_PROJECT_NAME=<この回> docker compose down -v`」を載せ、design の Risks に消した記録が残ることを、D13 の反転条件に片付けの change の起点を足した。片付けの道具は後の change（escalated の経緯: Q5 の前提に触れ、残す・消すのどちらも失うものがあった。`deep.md` Q6）

## /code-review（6bb4584..5e9836f）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R23. New versions must now put `DELETE FROM migration_ledger.applied WHERE name = '<name>'` in their .down.sql, but that statement errors on any DB that has no ledger schema.
- 成果物: tools/check-migrations.sh:61
- 根拠: A new version from 202610250000 on writes the required DELETE in its .down.sql. tools/check-immutable.sh applies versions with psql, so its DB never gets the migration_ledger schema, and it already runs .down.sql files (lines 962, 1156, 1212...). Once a new version's down is added there, it fails with 'relation migration_ledger.applied does not exist'. The same happens when rolling back a DB that has no ledger. The rule should accept a guarded form, e.g. DO-block / to_regclass check.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D10 —— 再現: `check-immutable.sh` は psql で当て、`migration_ledger` を作らないので、新しい版の `.down.sql` の `DELETE` は落ちる。規則は緩めず、`check-immutable.sh` の節の前の準備（付与の後）で所有者が版の記録の置き場（`migration_ledger.sql`）だけを作る（merge gate の CI で本体の準備への書き足しが `check-shared-appends.sh` に落ちたので、置き場を Story の節 `tools/check-immutable.d/202610250000_st48-migration-ledger.sh` へ移した。順は同じ —— 準備の後・戻しの前）。記録の無い DB への戻しは、ST48 以後の migrate がどの DB にも置き場を作るので起きない

## R24. The shared worktree test DB now goes through ledger-aware `migrate()`, so a version that is already recorded is never applied again, even after its SQL is edited. The old code reapplied every version on each test process.
- 成果物: crates/server/src/testdb.rs:88
- 根拠: A developer adds migration X and runs cargo test, so X is recorded in the worktree DB. They then fix X (add a column) and run cargo test again. apply_locked skips X and only logs a fingerprint warning, so the tests run against the old schema and fail (or pass) for no clear reason until the DB is reset by hand. The same applies to dev.sh / stack.sh on a DB that is kept.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: D7（仮）の「指紋が違う版は当て直さず、警告だけ」と Risks の「印より前の版を直した worktree の DB は、直した中身が当たらないまま進む。`STACK_RESET=1` で直る」がこの場合そのもの。警告（`tracing::warn!` に版の名前）が出る。試験の DB だけ当て直すと、試験と本物の migrate の振る舞いが分かれる

## R25. ASHIATO_BUILD_COMMIT comes from `git rev-parse HEAD`, but the build script only reruns when ORDER, migrations/ or the env var change. Without the env var, BUILD_COMMIT stays at whichever commit first ran the script.
- 成果物: crates/server/build.rs:23
- 根拠: Build at commit A, commit B (touching only Rust code), then rebuild. build.rs does not rerun, so the binary still embeds A. Ledger rows record build_commit=A, and db_ahead reports print `self: … A` and `db: (… A)`, pointing ST58 and operators at the wrong build. Adding `cargo:rerun-if-changed` for .git/HEAD and the current ref fixes this.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: D8（仮）が「git の HEAD の変化では作り直さない（差分のビルドでは古いまま残りうる。診断の材料なので許す）」と決め、反転条件（読み違えた事例が出たら `.git/HEAD` と参照先を足す）を持つ。code-verify の手 1 も同じ現象を見て D8 の反転条件の材料として残した（R4）。配る組み立ては `ASHIATO_BUILD_COMMIT` を渡す

## R26. Down files of versions before NEW_RULES_FROM do not remove their ledger row, so rolling back one of them on a DB with a ledger leaves a stale row and the next migrate never reapplies the version.
- 成果物: tools/check-migrations.sh:17
- 根拠: On a ledger-tracked DB, run 202610020030_places.down.sql to roll back, then run `ashiato-server migrate`. The ledger still lists the version as applied, apply_locked skips it, startup_refusal reports Even, and the server starts without core.place, failing at runtime.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 境より前の版の `.down.sql` に記録の行を消させないのは D10 の決定（走っている Story の版に後から規則を課さない）。それらの戻しを当てるのは、記録の無い `check-immutable.sh` の DB と、使い捨ての DB か戻す取引の中の試験だけ（code-verify の手 6 で全部読んだ）。記録のある DB で古い戻しを手で当てる経路は手順に無い

## R27. verify-prep no longer counts or judges the c01-clock retry/hourly flip condition (ST05 D4). It now only prints a SQL line for a human to run and judge.
- 成果物: tools/verify-prep.sh:95
- 根拠: CLAUDE.md (/home/yosis/dev/ashiato2-st48/CLAUDE.md) says: '機械が判定できることは機械に決めさせる。規約に書く・人間に聞くで代替しない'. The removed block computed n_retry/n_hourly and printed the verdict. Now the flip from 5 to 15 minutes is only decided if a person runs the query and compares the numbers. It could instead be computed by run.sh or after the stack starts, against the per-batch project.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D13 —— 再現: 回ごとの DB は準備のとき空で、数えても意味が無いので数えるのを外していた。前の回の DB（`ashiato-verify-*`）と既定の project の DB を 1 つずつ起こして（止まっていたものは数えた後に止め直す）足し、反転条件を機械が判定する形に戻した。この worktree で走らせ「DB 1 個（ashiato2-st48）… hourly が 0 件なので比は出ない」と出て、動いていた DB は動いたまま

## R28. before_ledger is only true on the run that sees Ledger::BeforeLedger. If that run fails partway, the next run sees Recorded and marks every remaining version before_ledger=false, including versions that were already applied before the ledger existed.
- 成果物: crates/server/src/migration_ledger.rs:348
- 根拠: Old DB with core. migrate records versions 1-4 with before_ledger=true, then version 5 fails. After the fix, the next migrate inserts versions 5..N with before_ledger=false, although 5..k were applied by the pre-ledger tooling. The D3 provenance column is then wrong for those rows.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: R14 と同じ（`before_ledger` は診断の材料で、当てる・止めるの判定に使わない。D3）

## R29. before_apply (a full pg_dump) runs while the advisory lock is held, even when the comparison is Even and nothing will be applied.
- 成果物: crates/server/src/migration_ledger.rs:351
- 根拠: Every `stack.sh up` / `dev.sh` on a kept, up-to-date DB takes a full blocking pg_dump inside the lock and adds another pre-migrate-*.dump under backups/pre-migrate, though no version runs. Running the backup only when the missing set is not empty avoids the repeated I/O, disk growth and lock hold time.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: spec の Scenario「当てる版が無い回にも、当てる前の退避を取る」が求める振る舞い（`specs/distribution/spec.md:95`、試験は `server_startup.rs:689`）。退避は D4 の 5 で錠の中（検査の後・当てる前）に置くと決めている

## R30. At startup the app role reads the ledger through information_schema.columns, which hides columns the role has no privilege on. A missing grant is therefore reported as ledger_unreadable 'form の表が無い' instead of a permission problem.
- 成果物: crates/server/src/migration_ledger.rs:223
- 根拠: The migration_ledger schema exists but ashiato_app lost SELECT on it (grants revoked, or the schema was made by hand). startup_refusal returns Unreadable('form の表が無い') with exit 3. Following the printed advice (STACK_RESET=1) would wipe a DB whose ledger is actually intact.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 —— 再現: 試験でアプリの役割から `form` の `SELECT`（または schema の `USAGE`）を剥がすと「form の表が無い」が返った。形を見る前に `has_schema_privilege` / `has_table_privilege` で見て「版の記録を読む権限が無い（付与の抜け。DB を作り直さずに付与を確かめる）」を返す。試験 `st48_read_ledger_unreadable_app_lacks_privilege`（検査を外した木で落ちる）

## R31. TempDb::create / Drop re-implements the throwaway-database helper that already exists as `crate::stay_tests::fresh_db` (CREATE DATABASE + DROP … WITH (FORCE)).
- 成果物: crates/server/src/testdb.rs:115
- 根拠: There are now two helpers for temporary databases with different URL handling (fresh_db breaks on '?' params, TempDb handles them) and different cleanup models. Fixes to one will not reach the other. fresh_db could be moved into testdb and built on TempDb's with_database.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: `fresh_db` は `stay_tests`（ST08 の側）にあり、寄せると他の Story の試験に触れる。D16 は使い捨ての口を CH04 の `*_tmp_*` に載せ替えると決めており、2 つの口はそこで 1 つにする

## R32. The exit-code-3 handling block (run migrate, check rc==3, print a long recovery message, exit 3) is copied almost word for word into tools/dev.sh line 24.
- 成果物: tools/stack.sh:96
- 根拠: The two recovery texts already differ slightly (dev.sh adds '（tools/stack.sh）'). Any later change to the instructions, which ST58 and the tests read, has to be made in both scripts. One shared function or a small helper script would keep them in sync.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 2 か所の文言は D14 が両方の台本に置くと決めたもの。中身の契約（終了コード 3 のまま止まる）は `check-db-version-stop.sh` が両方を走らせて見ている。文言の違いは台本の名前の 1 語だけで、共通の部品に出すのは表面の整理（merge の前に直す理由にならない）

## finish の scoped re-review（e5147c3e。1 回）

R18・R19・R20・R23・R30 は ADDRESSED。R27 は PARTIAL —— 止まっている DB の `up` が落ちる（動いている別の回の DB が port 55499 を握る）と黙って飛ばし、数えた DB だけで判定を出す。
fix は 1 回だけなので直さず、Task 8 の ledger に `Ruling:` つきで park した（PR 本文に写した）。ほかに新しい Critical / Important は無い。
