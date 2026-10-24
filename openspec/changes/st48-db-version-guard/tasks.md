# ST48 実装タスク — 古い版が新しい DB を書き換えない

読む順: `deep.md`（**本人の答え Q1〜Q5**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `design.md`（D1〜D17）→
`premises.md`（末尾の C1〜C7）→ `crates/server/src/lib.rs` の `migrate()`・`run_migrate()`・`run()` → `crates/server/src/migrate_backup.rs` → `crates/server/src/testdb.rs` →
`crates/server/tests/server_startup.rs` → `tools/stack.sh` → `tools/verify-prep.sh` → `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `./tools/db.sh up -d --wait db && ./tools/db-roles.sh` が前提（`scripts/verify-run` は `tools/verify-env.sh` が整える）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: 同じ版で当て直しても、何も変わらずに通る`）、bash は `echo "Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 41 本**（`distribution` の ADDED 7 つ: 8・7・6・6・7・3・4 本）。**41 本とも、このファイルのどこかの Task が名指しする**
- **件数つき検証**: このファイルで
  - **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server --lib <名前>'` rc=0
  - **`NL <名前と下限>`** は `bash -c 'set -- <名前と下限>; [ "$(cargo test -p ashiato-server --lib "$1" -- --list 2>/dev/null | grep -c ": test$")" -ge "$2" ]'` rc=0
  - **`IT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run it -- cargo test -p ashiato-server --test server_startup <名前>'` rc=0
  - **`NI <名前と下限>`** は `bash -c 'set -- <名前と下限>; [ "$(cargo test -p ashiato-server --test server_startup "$1" -- --list 2>/dev/null | grep -c ": test$")" -ge "$2" ]'` rc=0
  （`cargo test` は 0 本でも rc=0 になるので、本数の下限を別に見る。絞り込みは 1 つずつ書く）
- **新しく足す試験の名前の頭は `st48_`**
- **本人の決定（下流は変えない）**: Q1 まだ当たっていない版だけ当てる＋配った移行（印 `v*` の版にあるもの）の名前と中身を変えると CI が落ちる / Q2 記録の無い DB には全部を当てて全部に「記録より前」の印 /
  Q3 DB にまだ当たっていない版があればサーバは起動しない / Q4 止まるときは専用の終了コード 3 / Q5 確認バッチは回ごとに別の DB・止まったら手順を示す・止める仕組みを外す抜け道は作らない。
  （第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- **終了コード 3 と標準エラーの 1 行目 `error: kind=<種別>` の形は、配った版に焼き付く**（design D6）。種別は `db_ahead` / `db_behind` / `ledger_unreadable` の 3 つだけ。ほかの拒否の終了コード（1・2）を変えない
- **止める仕組みを外す環境変数・引数を作らない**（Q5）
- **版の記録に知らない行を書く試験は、worktree で共有する DB に書かない**（design D16）。使い捨ての database を作って消す。CH04 の `--lint` に掛かる行（`CREATE DATABASE`）には同じ行か 1 行上に `db-ok: <理由>` を書く
- **移行の版は足さない**（design D1。版の記録の表は ORDER の外の bootstrap）
- 一時ファイルは `"$TMPDIR"` の下に置く（`/tmp` の固定名に書かない）
- 走っている Story・change のブランチには触らない（CH04・ST23・ST29・ST41。後から入る側が rebase で追従する。proposal の Impact）

## Task 1: 版の記録の形と読み方

- [ ] 1.1 `crates/server/src/migration_ledger.sql`（design D1 の DDL）と `crates/server/src/migration_ledger.rs` を足し、`lib.rs` に `pub mod migration_ledger;` と試験のモジュール `migration_ledger_tests` を足す。
  `migration_ledger.rs` に、版の記録を読む `read_ledger`（`migration_ledger` が無い・`core` の有無・形の版・列を見て、D3 の 3 分岐と D5 の「読めない」を返す）と、
  純粋な判定 `compare(known, ledger)`（D2。`Ahead { unknown }` / `Behind { missing }` / `Even`）を書く。アプリの役割への `migration_ledger` の `USAGE` と `SELECT` は `grants.sql` でなく bootstrap（`migration_ledger.sql`）の末尾に置く（D1。`grants.sql` は記録の schema の無い DB にも psql で当たる —— `tools/check-immutable.sh`）。
  `testdb.rs` に使い捨ての database の口（`CREATE DATABASE st48_<乱数 12 桁>` を作り、Drop で `DROP DATABASE … WITH (FORCE)`。D16）を足す。
  検証: `cargo build -p ashiato-server --tests` rc=0
- [ ] 1.2 `crates/server/src/migration_ledger_tests.rs` に次を足す:
  `st48_ledger_shape_is_frozen`（`// Scenario: 版の記録の置き場と列の形は、決めた形のまま変わらない`。使い捨ての database に bootstrap を流し、`information_schema.columns` の 2 つの表の列の名前・型・NULL 可否が design D1 と一字一句同じ。`form` の値が 1）/
  `st48_ledger_app_role_reads_only`（`// Scenario: S-01 の実行時の役割は版の記録を読めるが、書き換えられない`。移行と付与を当てた使い捨ての database に、アプリの役割で繋ぎ、SELECT が通り、INSERT・UPDATE・DELETE が権限の誤り（SQLSTATE `42501`）で断られる）/
  `st48_compare_*`（純粋な判定。同じ集合 → `Even`、DB にだけある名前 → `Ahead` でその名前、自分にだけある名前 → `Behind`、両方あれば `Ahead` を先に返す。並びの順を入れ替えても結果が同じ。4 本以上）/
  `st48_read_ledger_unreadable_*`（`form` を 2 にした・`form` の行を消した・`applied` の列を 1 つ消した database で `read_ledger` が「読めない」を返す。3 本以上）。
  検証: `CT st48_ledger_` rc=0 / `NL st48_ledger_ 2` rc=0 / `CT st48_compare_` rc=0 / `NL st48_compare_ 4` rc=0 / `CT st48_read_ledger_unreadable_` rc=0 / `NL st48_read_ledger_unreadable_ 3` rc=0

## Task 2: 当てる処理 —— まだの版だけを、1 版 1 取引で、記録しながら

- [ ] 2.1 `migration_ledger.rs` に `apply_migrations(conn, versions)` を書く（design D3・D11・D7）: `pg_advisory_lock(4820251)` を取る → 記録を読む → ahead・読めないなら何も書かずに返す →
  bootstrap → 自分の版のうち記録に無いものを渡された並びの順に 1 版ずつ取引で当てて `INSERT`（`sha256` は中身の 16 進の小文字。`before_ledger` は D3 の 2 つ目の分岐だけ true）→
  記録にある版の指紋の違いは `tracing::warn!` に版の名前を出すだけ → 錠を外す（失敗しても外す）。
  `crates/server/build.rs` に組み立ての commit（design D8: `ASHIATO_BUILD_COMMIT` → `git rev-parse HEAD` → `unknown`。`cargo:rerun-if-env-changed=ASHIATO_BUILD_COMMIT`）を足し、版番号と一緒に `migration_ledger.rs` の定数にする。
  `lib.rs` の `migrate()` を `apply_migrations(&mut conn, MIGRATIONS)` → `GRANTS` に替える（ahead・読めないは呼び出し元に区別できる誤りで返す。Task 4 が終了コードにする）。
  検証: `cargo build -p ashiato-server --tests` rc=0 / `cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0
- [ ] 2.2 `migration_ledger_tests.rs` に次を足す（どれも使い捨ての database）:
  `st48_apply_records_every_version`（`// Scenario: 空の DB に移行を当てると、当てた版がすべて記録される`。名前の集合が `MIGRATIONS` と一致し、`before_ledger` が全部 false）/
  `st48_apply_row_has_fingerprint_and_build`（`// Scenario: 版の記録の 1 行は、名前・指紋・時刻・組み立ての版番号と commit を持つ`。指紋は試験の側で `MIGRATIONS` の中身から sha256 を計算し直して突き合わせる。版番号は `env!("CARGO_PKG_VERSION")`、commit は `env!("ASHIATO_BUILD_COMMIT")` と一致し、試験の走らせ方は `ASHIATO_BUILD_COMMIT` を渡さない場合も `git rev-parse HEAD` と一致する）/
  `st48_apply_pre_ledger_db_marks_all`（`// Scenario: 版の記録の無い古い DB には一度だけ全部を当て、印を付けて記録する`。先に `MIGRATIONS` を記録無しで直に当て、`core.event` に 1 件入れてから `apply_migrations` を呼ぶ。1 件が残り、全部の行の `before_ledger` が true）/
  `st48_apply_partial_failure_records_done_only`（`// Scenario: 移行が途中で落ちたら、当て終えた版だけが記録される`。試験用の 3 版の並び（2 つ目は `SELECT 1/0;`）。誤りが返り、記録は 1 つ目だけ）/
  `st48_apply_does_not_rerun_recorded`（`// Scenario: 記録にある版は、中身をもう一度走らせない`。試験用の並びの 1 版は `CREATE TABLE IF NOT EXISTS st48_trace(x int); INSERT INTO st48_trace VALUES (1);`。2 回当てても行は 1 行）/
  `st48_apply_fills_earlier_gap`（`// Scenario: 並びの前にある未適用の版も当たる`。試験用の 3 版の並びで 1 つ目と 3 つ目だけを先に当てた database に、3 版を渡す。2 つ目だけが当たり、3 つ目の `applied_at` は変わらない）/
  `st48_apply_concurrent_runs_once`（`// Scenario: 2 つの migrate が同時に走っても、版は 1 回ずつ当たる`。別々の接続から `tokio::join!` で 2 つ同時に呼び、どちらも Ok、行は版ごとに 1 つ）/
  `st48_apply_legacy_own_transaction_versions_are_recorded`（自分で `BEGIN` / `COMMIT` を持つ `202609082001_immutable_collected` と `202609112113_source_lifecycle` にも記録の行がある）。
  検証: `CT st48_apply_` rc=0 / `NL st48_apply_ 8` rc=0

## Task 3: 試験の足場を同じ当て方に寄せる

- [ ] 3.1 `crates/server/src/testdb.rs` の初回の当て込みを、`MIGRATIONS` を直に流す形から `apply_migrations` → `GRANTS` に替える。外側の取引と `pg_advisory_xact_lock(4820251)` は外し、取引の外の 1 本の接続で `apply_migrations` を呼ぶ（錠は `apply_migrations` が同じ鍵で取る。残すと自分を待ち続ける）（design D12・D11。CH04 が入っていれば、そのテンプレートの当て込みの行を替える）。
  `pool()`・`app_pool()`・`url()` の形は変えない。
  検証: `bash -c '! grep -nE "for .* in crate::MIGRATIONS" crates/server/src/testdb.rs'` rc=0 / `CT stay_tests` rc=0 / `CT ingest` rc=0
- [ ] 3.2 `migration_ledger_tests.rs` に `st48_testdb_has_ledger`（`// Scenario: 試験の足場が当てた DB にも、同じ版の記録がある`。`testdb::pool()` の DB の版の記録の名前の集合が `MIGRATIONS` と一致）を足し、
  書き出しの試験のファイル（`crates/server/src/export_file_tests.rs`。CH04 の後なら書き出しの試験の足場の口を使う）に `st48_export_starts_with_ledger`（`// Scenario: 版の記録があっても、全記録の書き出しは始まる`。版の記録のある DB で書き出しを始め、「分類の無い表」で断られず、書き出したファイルの一覧に `migration_ledger` を含む名前が無い）を足す。
  検証: `CT st48_testdb_has_ledger` rc=0 / `NL st48_testdb_has_ledger 1` rc=0 / `CT st48_export_starts_with_ledger` rc=0 / `NL st48_export_starts_with_ledger 1` rc=0

## Task 4: migrate の順と止め方 —— 検査を退避の前に、終了コード 3

- [ ] 4.1 `lib.rs` の `run_migrate()` を design D4 の順にする（接続 → 錠 → 読む → ahead・読めないなら退避を取らずに終了コード 3 → 退避 → 当てる → 付与）。
  止まるときの標準エラーを design D6 の形で出す（`error: kind=db_ahead` / `kind=ledger_unreadable` の 1 行目、`  db: <名前> (<版番号> <commit>)` を 20 件まで、`  self: <版番号> <commit>`、説明の 1 行）。
  `tracing::error!(kind = …)` にも同じ種別を出す。`MIGRATE_BACKUP` などどの環境変数でもこの検査を飛ばさない。移行の失敗はいまのまま終了コード 1。
  検証: `cargo build -p ashiato-server --bin ashiato-server` rc=0 / `bash -c '! grep -nE "MIGRATE_.*(GUARD|FORCE|SKIP)|ALLOW_DB_AHEAD" -r crates/server/src'` rc=0
- [ ] 4.2 `crates/server/tests/server_startup.rs` に、使い捨ての database（`CREATE DATABASE st48_<乱数>`。`db-ok:` の理由を付け、Drop で消す）に実行ファイル `migrate` を走らせる試験を足す。
  退避の置き場は `std::env::temp_dir()` の下の試験ごとのフォルダ（`TMPDIR` に従う）、`PG_DUMP` は試験が書く偽の台本（呼ばれたら置き場に `pre-migrate-*.dump` を書く・失敗させる版も持つ）:
  `st48_migrate_rerun_changes_nothing`（`// Scenario: 同じ版で当て直しても、何も変わらずに通る`。2 回目も rc=0、行の数・名前・`applied_at`・`pg_get_functiondef('core.reject_collected_rewrite'::regproc)` が 1 回目と同じ）/
  `st48_migrate_fingerprint_mismatch_warns`（`// Scenario: 指紋の違う版は当て直さず、名前を示して続ける`）/
  `st48_migrate_backs_up_even_with_nothing_to_apply`（`// Scenario: 当てる版が無い回にも、当てる前の退避を取る`）/
  `st48_migrate_refuses_db_ahead`（`// Scenario: 新しい版で当てた DB に古い版の migrate を走らせると、1 つも当てずに止まる`。記録に `209912312359_from_the_future`・`0.9.9`・`deadbeef` の行を足す。rc=3、標準エラーにその名前・`0.9.9`・`deadbeef`・自分の版番号がある。行と門の定義が前と同じ）/
  `st48_migrate_refusal_takes_no_backup`（`// Scenario: 止まるときは退避を取らない`）/
  `st48_migrate_refuses_unreadable_ledger`（`// Scenario: 版の記録が読めない DB では、何も書かずに止まる`）/
  `st48_migrate_backup_failure_applies_nothing`（`// Scenario: 退避に失敗すると、終了コード 1 で何も当てない`。失敗する偽の `PG_DUMP`。記録から消した 1 行が戻っていない）/
  `st48_migrate_content_failure_exits_1`（`// Scenario: 版の中身が当たらないと、終了コード 1 で終わる`。記録から 1 版の行を消し、その版が作る物と衝突する物（同じ名前で型の違う関数や列など）を所有者で置いて当たらなくする）/
  `st48_migrate_regrants_with_nothing_to_apply`（`// Scenario: 当てる版が無い回にも、アプリの役割への付与を当てる`。`REVOKE SELECT ON core.event FROM ashiato_app` の後に migrate、アプリの役割で `SELECT count(*) FROM core.event` が通る）/
  `st48_migrate_backup_settings_never_bypass`（`// Scenario: 退避の設定を変えても、知らない版の当たった DB には当たらない`。`MIGRATE_BACKUP=off`・`MIGRATE_BACKUP_DIR` を別の置き場・`PG_DUMP` を別の偽の台本、の 3 回とも rc=3）。
  `db_ahead` の試験は標準エラーを行ごとに見る（1 行目が `error: kind=db_ahead`、`  db: 209912312359_from_the_future (0.9.9 deadbeef)` の行、`  self: ` で始まる行）。
  検証: `IT st48_migrate_` rc=0 / `NI st48_migrate_ 10` rc=0

## Task 5: サーバの起動の検査

- [ ] 5.1 `lib.rs` の `run()` で、`check_db_role` の後・待ち受けを開く前に、アプリの役割で `read_ledger` → `compare` を通す（design D17）。
  ahead → `error: kind=db_ahead`（D6 の形）、behind（記録が無いを含む）→ `error: kind=db_behind` と「まだ当たっていない版が N 個。先に `ashiato-server migrate` を」、読めない → `error: kind=ledger_unreadable`。どれも終了コード 3。指紋の違いは警告だけ。
  検証: `cargo build -p ashiato-server --bin ashiato-server` rc=0
- [ ] 5.2 `server_startup.rs` に次を足す（使い捨ての database に実行ファイル `migrate` を当ててから、アプリの役割で起動する）:
  `st48_server_refuses_db_ahead`（`// Scenario: 新しい版で当てた DB に古い版のサーバを起動すると、起動しない`）/
  `st48_server_refuses_db_behind`（`// Scenario: まだ当たっていない版のある DB では、サーバは起動しない`。記録から 2 行を消し、標準エラーに数 2 がある）/
  `st48_server_refuses_pre_ledger_db`（`// Scenario: 版の記録の無い DB では、サーバは起動しない`。`DROP SCHEMA migration_ledger CASCADE` で記録の無い DB を作る）/
  `st48_server_starts_when_even`（`// Scenario: 版が揃っていれば、サーバは起動する`。`/healthz` が 200）/
  `st48_server_starts_with_fingerprint_warning`（`// Scenario: 指紋の違う版があっても、サーバは警告を出して起動する`）/
  `st48_server_refuses_unreadable_ledger`（`// Scenario: 版の記録が読めない DB では、サーバは起動しない`）。
  拒否の 4 本は、待ち受けを開いていないこと（終了までに `/healthz` に繋がらない）も見る。
  検証: `IT st48_server_` rc=0 / `NI st48_server_ 6` rc=0 / `IT server_startup_` rc=0

## Task 6: 配った移行の凍結の検査

- [ ] 6.1 `tools/check-released-migrations.sh` を足す（design D9）。`git tag --list 'v[0-9]*'` の印ごとに、印の版の `migrations/ORDER` の全部の名前が今の木の `ORDER` にあり、
  前進側の `.sql` の中身が同じであることを見る。違えば移行の名前と印の名前を出して rc=1。印が 0 個なら「配った印が 0 個」と出して rc=0。印の commit・`ORDER` を取り出せなければ rc=1。
  `--self-test` は `"$TMPDIR"` の下に使い捨ての git の木を作り、7 つの場合を走らせて、それぞれに `echo "Scenario: …"` を出す:
  `Scenario: 配った版にある移行の中身を変えると、検査が落ちる` / `Scenario: 配った版にある移行を消すと、検査が落ちる` / `Scenario: 配った版にある移行の名前を変えると、検査が落ちる` /
  `Scenario: 配った印が無ければ、検査は通る` / `Scenario: 配った印の後に足した移行は、変えても通る` / `Scenario: 配った版にある移行の戻し手順は、変えても通る` /
  `Scenario: 配った印の版を読めなければ、検査は通らない`（場合ごとに別の木で見る）（タグの指す commit を取り出せない木は、浅い clone か `git replace` で作る）。
  検証: `tools/check-released-migrations.sh --self-test` rc=0 / `tools/check-released-migrations.sh` rc=0
- [ ] 6.2 `.github/workflows/ci.yml` の rust の job の `./tools/check-migrations.sh` の後に、タグの取り寄せ（`git fetch --force --no-tags --depth=1 origin '+refs/tags/v*:refs/tags/v*'`）と、
  `./tools/check-released-migrations.sh --self-test`・`./tools/check-released-migrations.sh` を足す。`docs/testing.md` の手元で走らせる一覧（§ の `./tools/check-migrations.sh` の行）にも足す。
  `docs/handoff/ST52.md` に「配った印のタグの型は `v*`。別の型にするなら `tools/check-released-migrations.sh` の型を一緒に変える（ST48 design D9）」を足す。
  検証: `bash -c 'grep -q "check-released-migrations.sh --self-test" .github/workflows/ci.yml && grep -q "refs/tags/v" .github/workflows/ci.yml && grep -q check-released-migrations docs/testing.md && grep -q check-released-migrations docs/handoff/ST52.md'` rc=0

## Task 7: これから足す版の規則

- [ ] 7.1 `tools/check-migrations.sh` に design D10 の 2 つの規則を足す（名前の時刻が `202610250000` 以上の版だけ）: 前進側が文として `BEGIN` / `START TRANSACTION` / `COMMIT` / `ROLLBACK` / `END` を持たない（`$$` の本体の中は数えない）、
  `.down.sql` が `DELETE FROM migration_ledger.applied WHERE name = '<版の名前>'` を持つ。`--self-test` を足し、`"$TMPDIR"` の下の使い捨ての `migrations/` で次を走らせて `echo "Scenario: …"` を出す:
  `Scenario: 新しい版が自分で取引を持つと、検査が落ちる` / `Scenario: 新しい版の戻し手順が版の記録の行を消さないと、検査が落ちる`（それぞれ落ちて、版の名前が出る。`DO $$ BEGIN … END $$;` だけの版と、規則を守った版は通る）。
  今の木で走らせる本体の出力の前に `echo "Scenario: 前からある版には、この規則を求めない"` を出す。CI の rust の job に `./tools/check-migrations.sh --self-test` を足す。
  検証: `tools/check-migrations.sh --self-test` rc=0 / `tools/check-migrations.sh` rc=0 / `bash -c 'grep -q "check-migrations.sh --self-test" .github/workflows/ci.yml'` rc=0

## Task 8: 開発と確認の DB —— 回ごとの project・止まったときの手順

- [ ] 8.1 `tools/verify-prep.sh` が書く `run.sh` に `export COMPOSE_PROJECT_NAME="ashiato-verify-<tag を英小文字・数字・- に寄せたもの>"` を足す（design D13）。
  `tools/stack.sh` は DB を起こす前に、`ASHIATO_DB_PORT` を公開している容器が別の compose の project のものなら `docker stop` する（`down` しない）。止めたら 1 行出す。
  `tools/stack.sh` と `tools/dev.sh` は、`migrate` の終了コードが 3 なら作り直さずに design D14 の手順（`STACK_RESET=1` と `COMPOSE_PROJECT_NAME=<別の名前>`）を出して終了コード 3 で止まる。
  `tools/db-roles.sh` の冒頭の注記に D15 の 1 文、`tools/check-immutable.sh` の「run() は起動のたびに全版を当てる」の注記を D12 の文に直す。
  検証: `bash -n tools/stack.sh tools/dev.sh tools/verify-prep.sh` rc=0 / `bash -c 'grep -q COMPOSE_PROJECT_NAME tools/verify-prep.sh && grep -q "STACK_RESET=1" tools/stack.sh && grep -q COMPOSE_PROJECT_NAME tools/stack.sh && grep -q COMPOSE_PROJECT_NAME tools/dev.sh'` rc=0
- [ ] 8.2 `tools/check-db-version-stop.sh` を足す（新しいファイル。共有の台本に書き足さない）。始めに、この worktree の DB の port を持っている容器の名前を覚え、終わりに起こし直す。
  試験の project（`st48-a-<乱数>`・`st48-b-<乱数>`）は終わりに `down -v` で消す。退避の置き場は `"$TMPDIR"` の下。次を走らせ、それぞれに `echo "Scenario: …"` を出す:
  `Scenario: 確認バッチの起動は、回ごとの project に立つ`（`verify-prep.sh` の `run.sh` を書く部分を 2 つの違う tag で書かせるか、書かれた本文の `COMPOSE_PROJECT_NAME=` の行を取り出し、tag を含み、2 つで違う）/
  `Scenario: 別の project で立てると、前の DB は残る`（A で `stack.sh up --check-only` → `core.event` に 1 件 → B で `stack.sh up --check-only` が rc=0 → A を起こし直して 1 件ある）/
  `Scenario: 版の食い違いで止まると、作り直さずに手順を示す`（A の `migration_ledger.applied` に知らない版の行を足して `stack.sh up --check-only` が rc≠0、出力に `STACK_RESET=1` と `COMPOSE_PROJECT_NAME` があり、1 件と行が残る）/
  `Scenario: 作り直した DB には、今までどおり全部が当たる`（`STACK_RESET=1` で rc=0、記録の名前の集合が `ORDER` と一致し、退避の置き場に新しいファイルが無い）。
  CI の smoke の job の `./tools/check-immutable.sh` の後に足し、`docs/testing.md` の手元の一覧（`./tools/smoke.sh && …` の行）にも足す。
  検証: `scripts/quiet-run dbstop -- tools/check-db-version-stop.sh` rc=0 / `bash -c 'grep -q check-db-version-stop .github/workflows/ci.yml && grep -q check-db-version-stop docs/testing.md'` rc=0

## Task 9: 全体の確かめ

- [ ] 9.1 spec の 41 本の Scenario に印があり、既存の検査が通ることを確かめる。
  検証: `python3 scripts/check_scenarios.py . st48-db-version-guard` rc=0 / `cargo clippy --workspace --all-targets -- -D warnings` rc=0 / `scripts/quiet-run smoke -- tools/smoke.sh` rc=0 / `scripts/quiet-run immutable -- tools/check-immutable.sh` rc=0
