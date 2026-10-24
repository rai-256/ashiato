# CH04 実装タスク — 試験用 DB に溜まるデータで試験が遅くなるのを止める

読む順: `deep.md`（**本人の答え Q1〜Q4**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/test-database/spec.md` → `specs/data-durability/spec.md` → `design.md` →
`premises.md`（末尾の C1〜C7）→ `docs/changes/CH04.md` → `crates/server/src/testdb.rs` → CH02 の `design.md`（`openspec/changes/ch02-export-next-record-window/`。Task 8 の前提）→ `CLAUDE.md`。

**試験の期待値・`// Scenario:` の印・試験の名前と本数は変えない**（C1）。書き換えるのは、試験が繋ぐ database の作り方・持ち方・片付けと、書き出しの重なりの判定の式（除く記録は変えない。C2）だけ。
**消せない表のトリガを外さない・無効にしない・`session_replication_role` を変えない**（C4。依頼の逐語）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `./tools/db.sh up -d --wait db && ./tools/db-roles.sh` が前提（`scripts/verify-run` は `tools/verify-env.sh` が整える）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: 試験ごとの database は、試験が panic しても消える`）、bash は `echo "Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 25 本**（`test-database` の ADDED 6 つ: 3・3・4・3・3・6 本、`data-durability` の ADDED 1 つ: 3 本）。**25 本とも、このファイルのどこかの Task が名指しする**
- **件数つき検証**: このファイルで
  - **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0
  - **`NL <名前と下限>`** は `bash -c 'set -- <名前と下限>; [ "$(cargo test -p ashiato-server "$1" -- --list 2>/dev/null | grep -c ": test$")" -ge "$2" ]'` rc=0（`cargo test` は 0 本でも rc=0 になるので、本数の下限を別に見る）
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く）
- **新しく足す試験の名前の頭は `ch04_`**（design D8 の CI の絞り込み・D11）
- **`testdb::pool()`・`app_pool()`・`url()` の引数と戻り値の型を変えない**（design D3。st09・st13 が `testdb.rs` に足した関数が rebase の後も通る）
- **`CREATE DATABASE` は `testdb.rs` の中だけに書く**（design D5）。片付けの試験が持ち主のいない回の database を作る行は、同じ行か 1 行上に `db-ok: <理由>` を書く
- **本人の決定（下流は変えない）**: Q1 回ごとの database ＋ 書き出しの試験だけ試験ごとの database / Q2 CH02 の上で候補の行 × 消した時間の入れ子を直す。判定は N と M を一緒に 4 倍にして消した時間を読む量が 8 倍未満。時間は測って残すだけ。件数の数え方（ST33 D13）は替えない /
  Q3 残りを見る台本 ＋ 試験の接続に問い合わせの時間の上限 / Q4 今ある `ashiato` のデータと使われていない volume には何もしない。
  （第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- **`ashiato` database を消す・空にする台本を書かない**（Q4）。`tests/server_startup.rs` の繋ぎ先は変えない（C7）
- 走っている Story・change のブランチには触らない（rebase の後にそれぞれの gate → fix が追従する。proposal の Impact）

## Task 1: 回の database とテンプレート —— 試験の足場の繋ぎ先を替える

- [x] 1.1 `crates/server/src/testdb.rs` に design D1・D2・D3 を書く: 回の id（`ashiato_run_<12 桁>`）、`postgres` database への所有者の接続、`pg_advisory_lock(4820252)` の直列化、
  専用のスレッドと runtime で持つ回の助言ロック（`pg_advisory_lock(4820253, hashtext('<id>'))`）、テンプレート `<id>_tpl` の作成と移行・権限の当て、`CREATE DATABASE <id> TEMPLATE <id>_tpl`。
  `pool()` の初回（`OnceCell`。いまの `MIGRATED` を置き換える）がこれを行い、`pool()`・`app_pool()`・`url()` は回の database を指す。冒頭の注記に回の database の規則を書く（D15）。
  片付け（Task 3）はこの段では空の関数を呼ぶだけでよい。
  検証: `cargo build -p ashiato-server --tests` rc=0 / `bash -c '! grep -nE "session_replication_role|DISABLE TRIGGER" crates/server/src/testdb.rs'` rc=0
- [x] 1.2 `crates/server/src/testdb_tests.rs`（新しいファイル。`lib.rs` の試験のモジュールの並びに足す）に次の 3 本を足す:
  `ch04_testdb_points_to_the_run_database`（`// Scenario: 試験の足場の繋ぎ先は、その回の database である`。`pool()` と `app_pool()` の `current_database()` が同じで `ashiato_run_` で始まり、`url()` の末尾も同じ）/
  `ch04_testdb_records_stay_out_of_ashiato`（`// Scenario: 試験が置いた記録は、ashiato database に入らない`。`put_event` で置いた id が回の database にあり、`url()` の database 名を `ashiato` に替えて繋いだ先の `core.event` に無い。`ashiato` に表が無ければ「無い」と読む）/
  `ch04_testdb_run_database_keeps_the_immutable_triggers`（`// Scenario: 回の database でも、消せない表は行の削除と TRUNCATE を断る`。`DELETE` と `TRUNCATE` が失敗し、`pg_trigger` の `event_no_delete` と `event_no_truncate` の `tgenabled` が `O`）。
  検証: `CT ch04_testdb_` rc=0 / `NL ch04_testdb_ 3` rc=0
- [x] 1.3 回の database のまま、DB を使う主な試験の群が通ることを確かめる（書き出し以外。書き出しは Task 5）。
  検証: `CT coverage` rc=0 / `CT ingest` rc=0 / `CT deletion` rc=0 / `CT stay_tests` rc=0 / `NL stay_tests 65` rc=0

## Task 2: 試験ごとの database と使い捨ての database の手伝い

- [x] 2.1 `testdb::isolated()`（`TestDb`。`pool()`・`url()`・`name()`）と `testdb::scratch()`（空の `<id>_tmp_…`。同じ Drop を持つ値）を design D4・D5 のとおりに書く。
  試験ごとの写しは過程の中の `tokio::sync::Mutex` で 1 本ずつ（D2）。Drop は自分のスレッドと runtime で `DROP DATABASE IF EXISTS <名前> WITH (FORCE)` を打って待つ。
  **Drop は panic しない**: 繋げない・DROP が落ちた・スレッドが panic した、のどれでも名前と理由を `eprintln!` で出して戻る（`unwrap` を書かない。design D4）。
  検証: `cargo build -p ashiato-server --tests` rc=0
- [x] 2.2 `testdb_tests.rs` に 3 本を足す: `ch04_isolated_database_is_dropped_after_the_test`（`// Scenario: 試験ごとの database は、試験が通れば終わりに消える`。名前を控え、`TestDb` を drop した後に `pg_database` に無い）/
  `ch04_isolated_database_is_dropped_on_panic`（`// Scenario: 試験ごとの database は、試験が panic しても消える`。`tokio::spawn` の中で `isolated()` を受けて記録を置いてから panic し、`JoinError` を受けた後に無い）/
  `ch04_scratch_database_name_starts_with_the_run`（`// Scenario: 使い捨ての database の名前は、回の database の名前で始まる`）。
  検証: `CT ch04_isolated_` rc=0 / `NL ch04_isolated_ 2` rc=0 / `CT ch04_scratch_` rc=0 / `NL ch04_scratch_ 1` rc=0
- [x] 2.3 前提の確かめ（`premises.md` P16。同じテンプレートから写すのを 1 本ずつにすれば落ちない）: `ch04_isolated_many_in_parallel`（`isolated()` を 16 本同時に呼び、全部が作れて全部が消える）を足す。
  写す 1 本の時間を `eprintln!` で出し、design の「計測の記録」に書く（1 秒を超えれば D2 の反転条件）。
  検証: `CT ch04_isolated_many_in_parallel` rc=0 / `NL ch04_isolated_many_in_parallel 1` rc=0
- [x] 2.4 前提の確かめ（P15。アプリの役割が回の database に繋げる）: 役割の試験を回の database のまま通す。落ちたら回の database の作成の後に `GRANT CONNECT ON DATABASE … TO ashiato_app` を足し、design D3 に書き足す。
  検証: `CT app_role` rc=0 / `NL app_role 3` rc=0

## Task 3: 片付け —— 走っていない回のものと、印の無い `_tmp_` の残り

- [x] 3.1 design D6 の片付けを `testdb.rs` に書き、回の初め（1.1 の手順 3）で呼ぶ。消すのは `ashiato_run_<12 桁>` で始まり助言ロックを `pg_try_advisory_lock` で取れたもの（取れたらすぐ外す）と、`_tmp_` を含み `ashiato_run_` で始まらないもの。
  `DROP DATABASE … WITH (FORCE)`。`ashiato`・`postgres`・`template0`・`template1` とそれ以外の名前には触らない。片付けは関数として試験から呼べる形にする（`testdb::sweep()`）。
  検証: `cargo build -p ashiato-server --tests` rc=0
- [x] 3.2 `testdb_tests.rs` に 3 本を足す（片付けの試験は、ほかの回の database を作るので `db-ok: 片付けの試験の材料` の印を付ける）。
  材料の database は **TEMPLATE を使わない空の database** として作り、3 本は 1 つの `tokio::sync::Mutex`（`SWEEP_SERIAL`）で並ばせない（ほかの試験の片付けが作りかけの材料を消して作る段で落ちるのを避ける。design の Risks）:
  `ch04_sweep_drops_an_orphan_run`（`// Scenario: 持ち主のいない回の database は、次の回の始まりに消える`。誰もロックを持たない回の id で `_tpl`・本体・`_x_…`・`_tmp_…` の 4 つを作り、`sweep()` の後に 4 つとも無い）/
  `ch04_sweep_keeps_a_running_run_without_connections`（`// Scenario: 走っている回の database は、接続が 0 本でも消えない`。別の接続でその回の助言ロックを持ち、database には繋がずに `sweep()`、残っている。後でロックを外して自分で消す）/
  `ch04_sweep_drops_legacy_tmp_and_keeps_others`（`// Scenario: 回の印の無い使い捨ての database の残りは消え、ashiato と他の database は残る`。`ch04x_tmp_<乱数>` と `ch04keep_<乱数>` を作り、`sweep()` の後に前者が無く、後者と `ashiato` がある。後者は自分で消す）。
  検証: `CT ch04_sweep_` rc=0 / `NL ch04_sweep_ 3` rc=0

## Task 4: 問い合わせの時間の上限

- [x] 4.1 design D7 のとおり、試験の足場の全部の接続に `statement_timeout` を置く（既定 60 秒、`ASHIATO_TEST_STATEMENT_TIMEOUT` で替える。`0` で上限なし）。
  環境変数の値から上限を決める部分は純粋な関数 `statement_timeout_from(Option<&str>)` にし、上限を引数で受けて接続を作る内側の関数を足場に置く（環境変数は外側だけが読む）。
  検証: `cargo build -p ashiato-server --tests` rc=0
- [x] 4.2 `testdb_tests.rs` に 3 本を足す（環境変数を試験の中で書き換えない。並走する試験に効く）:
  `ch04_statement_timeout_cancels_a_long_query`（`// Scenario: 上限を超える問い合わせは、止まって失敗する`。上限を 1 秒にした足場の接続で `SELECT pg_sleep(5)` が 5 秒より前に失敗し、SQLSTATE が `57014`）/
  `ch04_statement_timeout_default_is_set`（`// Scenario: 上限の既定は 60 秒で、試験の足場の接続に入っている`。`testdb::pool()` の `SHOW statement_timeout` が `1min`。CI と verify-run は環境変数を渡さない）/
  `ch04_statement_timeout_from_env_value`（`// Scenario: 環境変数の値が上限になり、0 なら上限を置かない`。`statement_timeout_from(None)` が 60 秒・`Some("1s")` が 1 秒・`Some("0")` が上限なし）。
  検証: `CT ch04_statement_timeout_` rc=0 / `NL ch04_statement_timeout_ 3` rc=0

## Task 5: 書き出しの試験を試験ごとの database へ

- [x] 5.1 design D4 のとおり、`export_tests.rs`・`export_file_tests.rs`・`export_job_tests.rs`・`export_ledger_tests.rs`・`export_excluded_tests.rs` の `testdb::pool()` を `testdb::isolated()` の `TestDb` から取る形にする。
  1 つの試験の中の `pool()` は全部同じ `TestDb` から取り、`TestDb` を試験の最後まで持つ（`World` は欄に持つ）。期待値・`// Scenario:` の印・試験の名前は変えない。`EXPORT_SERIAL` は残す。
  CH02 の `export_window_tests.rs` が取り込まれていれば、同じ形にする。
  検証: `CT export_` rc=0 / `NL export_ 123` rc=0 / `bash -c '! grep -nE "testdb::pool\(\)" crates/server/src/export*_tests.rs'` rc=0
- [x] 5.2 `testdb_tests.rs`（足場の試験。回の database と試験ごとの database の両方を見る。書き出しの試験のファイルに置くと 5.1 の検証と 6.2 の `--lint` に落ちる）に `ch04_export_isolated_does_not_see_other_tests`（`// Scenario: 書き出しの試験は、同じ回の他の試験が消した滞在を判定しない`）を足す:
  回の database（`testdb::pool()`）に消した滞在と、その時間に入る緯度経度つきの記録を置いた後、`isolated()` の database で `core.event` と `core.stay_erased` の行の数が 0 で、`export::read_summary` が記録 0 件を返す。
  検証: `CT ch04_export_isolated_does_not_see_other_tests` rc=0 / `NL ch04_export_isolated_does_not_see_other_tests 1` rc=0
- [x] 5.3 `testdb.rs` の `unique_day()` の注記を、隔離の理由が「回の database の中で書き出し以外の試験が共有する」に変わったことに合わせて直す（関数は変えない。D3）。
  検証: `bash -c 'grep -B6 "pub fn unique_day" crates/server/src/testdb.rs | grep -q "回の database"'` rc=0

## Task 6: 使い捨ての database の 5 か所と、database を作る書き方の検査

- [x] 6.1 `clock_tests.rs`・`feelings_tests.rs`・`stay_tests.rs`・`attributes_tests.rs`・`places_tests.rs` の `fresh_db()` の中身を `testdb::scratch()` に替える（design D5。名前と戻り値の形は残す。
  future は `DROP DATABASE IF EXISTS` で二重に消しても落ちない）。`export_ledger_tests.rs`・`export_excluded_tests.rs` の `crate::stay_tests::fresh_db()` の呼び出しは変えない。
  検証: `CT clock_tests` rc=0 / `CT feelings_tests` rc=0 / `CT attributes_tests` rc=0 / `CT places_tests` rc=0 / `NL places_tests 158` rc=0 / `bash -c '[ "$(grep -lE "CREATE DATABASE" crates/server/src/*.rs crates/server/tests/*.rs | grep -v testdb)" = "" ]'` rc=0
- [x] 6.2 `tools/check-test-db-growth.sh --lint` を design D5 のとおりに書く（`crates/server/src` と `crates/server/tests` の `CREATE DATABASE` と、`crates/server/src/export*_tests.rs` の `testdb::pool()`。`testdb.rs` と、同じ行か 1 行上に理由つきの `db-ok:` がある行は通す。
  落ちたときの文言に `testdb::isolated()` と `testdb::scratch()` と `db-ok: <理由>` を出す）。`--self-test` に次を足し、それぞれの前に印を出す:
  `echo "Scenario: 足場の外で database を作る試験は、検査が落とし、足場の関数を示す"`（偽の木の `src/x_tests.rs` に `CREATE DATABASE` → rc=1・ファイルと行・`testdb::scratch`）/
  `echo "Scenario: 書き出しの試験が回の database を取ると、検査が落とし、試験ごとの database を示す"`（偽の木の `src/export_x_tests.rs` に `testdb::pool()` → rc=1・ファイルと行・`testdb::isolated`。理由つきの `db-ok:` が 1 行上にあれば rc=0）/
  `echo "Scenario: CH04 の後の木は、database を作る書き方の検査を通る"`（本物の木で `--lint` → rc=0）。冒頭のコメントに使い方・rc・なぜ要るか（CH04）を書く。
  検証: `tools/check-test-db-growth.sh --lint` rc=0 / `tools/check-test-db-growth.sh --self-test` rc=0

## Task 7: 再発を止める台本と CI

- [x] 7.1 `tools/check-test-db-growth.sh [cargo test の引数…]` の本体を design D8 のとおりに書く（一覧 S0 → 試験 → S1 → 試験 → S2。判定は試験の rc → 前の回の残り → 最後の回の余計な残り → `ashiato` の有無。
  `TEST_DB_CMD`・`TEST_DB_LIST_CMD` で差し替えられる）。`--self-test` に次を足し、それぞれの前に印を出す:
  `echo "Scenario: 前の回の database が 2 回目の後に残っていれば、名前を挙げて落ちる"` /
  `echo "Scenario: 2 回目の試験ごとの database が残っていれば、名前を挙げて落ちる"` /
  `echo "Scenario: ashiato database が消えていれば落ちる"` /
  `echo "Scenario: 試験が落ちれば、台本も試験の終了コードで落ちる"`（rc=101 → rc=101）/
  `echo "Scenario: 前の回が片付き、最後の回の分だけが残れば、台本は通る"`。
  検証: `tools/check-test-db-growth.sh --self-test` rc=0
- [x] 7.2 本物の木で、server の試験の全体を同じ worktree で 2 回続けて走らせ、前の回の database が残らないことを見る。1 回目と 2 回目の全体の時間を design の「計測の記録」に書く（2 回目が 1 回目と同じ桁なら積もっていない）。
  上限（D7）で落ちる正当な試験があれば、D7 の反転条件に従って既定を直してから走らせ直す。
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run growth -- tools/check-test-db-growth.sh -p ashiato-server'` rc=0
- [x] 7.3 `.github/workflows/ci.yml` の rust job の `cargo test` の行の後に 3 行を足す: `./tools/check-test-db-growth.sh --self-test`・`./tools/check-test-db-growth.sh --lint`・`./tools/check-test-db-growth.sh -p ashiato-server ch04_`（`cargo test` の行は変えない。CH03 が包む）。
  `--self-test` に `echo "Scenario: CI の rust job は台本を走らせる"` の印で、`ci.yml` の rust job にこの 3 行があることを見る項を足す。
  検証: `tools/check-test-db-growth.sh --self-test` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run growth -- tools/check-test-db-growth.sh -p ashiato-server ch04_'` rc=0

## Task 8: 書き出しの重なりの判定から、候補の行 × 消した時間の入れ子を取る

- [x] 8.1 前提: CH02 の実装が取り込まれている（`requires: [CH02]`。下流の入口が gate を通った SHA を取り込む）。
  検証: `python3 scripts/story_deps.py included CH04 CH02` rc=0
- [x] 8.2 `excluded.rs` の最後の重なりの条件（CH02 の `WHERE EXISTS (SELECT 1 FROM erased_spans s WHERE s.start_at <= w.width_end AND s.end_at >= w.event_time)`）を 1 つの定数に括り出す（中身は変えない）。
  新しい試験のファイル `crates/server/src/export_overlap_tests.rs` に、`excluded_sql(…).ctes()` の文字列からその条件を CH02 の形に置き換えた比べる相手を作る手伝いを書き、置き換えた数が 2（記録の側と前の版の側）であることを assert する（design D11。丸ごと写さない）。
  同じファイルに `ch04_overlap_matches_ch02`（`// Scenario: 直した判定は、足す前の判定と同じ記録と前の版を除く`）を書く: `testdb::isolated()` の database に D11 の族を置き、
  同じトランザクション・同じ写しの時点の式で本体と比べる相手を走らせ、`excluded_event`・`excluded_version` の id の集まりが一致する。この時点では本体 = CH02 なので通る。
  検証: `CT ch04_overlap_matches_ch02` rc=0 / `NL ch04_overlap_matches_ch02 1` rc=0
- [x] 8.3 同じファイルに `ch04_overlap_reads_erased_spans_linearly`（`// Scenario: 記録と消した滞在を一緒に 4 倍にしても、消した時間を読む量は 4 倍の桁に収まる`）を書く:
  `isolated()` の database に (N, M) = (1,000, 10) と (4,000, 40) を置き（消した滞在は 1 時間ずつ別の日。記録は緯度経度つきで、消した時間をまたぐものと、どの消した時間にも重ならないものを半々に混ぜる）、`ANALYZE` の後に
  `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)` で `SELECT (SELECT count(*) FROM excluded_event), (SELECT count(*) FROM excluded_version)` を走らせる。
  読んだ量は、消した時間を読む節（`CTE Name` が `erased_spans` と、D9 がそこから作る CTE の全部の `CTE Scan`）の `(Actual Rows + Rows Removed by Filter) × Actual Loops` の和（CH02 の `rows_read` と同じ式。
  EXISTS が SubPlan の形でも、捨てた行が `Rows Removed by Filter` に入るので数えられる。review/spec.md R2）。4 倍の量が 1 倍の量の 8 倍未満であることを見る。
  **同じ試験で 8.2 の比べる相手（足す前の形）にも当て、8 倍以上になることを assert する**（測り方が入れ子を捕まえていることの確かめ）。この時点では本体 = CH02 なので、本体の側の assert は落ちる（8.4 で通る）。
  検証: `NL ch04_overlap_reads_erased_spans_linearly 1` rc=0
- [x] 8.4 `crates/server/src/export/excluded.rs` の重なりの判定（8.2 の定数）を design D9 のとおりに替える（整った消した時間の `range_agg` の multirange と `&&`、幅の側の範囲は `CASE` の中でだけ作る、整っていない消した時間と整っていない幅は今の `EXISTS`）。
  公開の形と呼び出し側は変えない。`excluded.rs` の注記に D9 と CH04 を書く。8.2 の置き換えが D9 の後の文字列でも 2 か所で当たるように、置き換える相手は定数で持つ。
  検証: `CT ch04_overlap_` rc=0 / `NL ch04_overlap_ 2` rc=0 / `CT export_` rc=0 / `NL export_ 123` rc=0
- [x] 8.5 写しの `READ ONLY` の中で直した断片が走る（一時表を使っていない）ことを、既存の書き出しの試験が見ていることを確かめる。
  検証: `CT export_file` rc=0 / `CT export_summary` rc=0

## Task 9: 量の測り —— N と M を一緒に倍にして 2.5 倍以内

- [x] 9.1 `export_volume_one_run_reports_time_and_memory` に、design D14 のとおり `ASHIATO_EXPORT_VOLUME_ERASED` を件数として読ませる（M ≥ 2 は M 日に等分、M ≤ 1 の置き方と assert は変えない）。
  検証: `CT export_volume_one_run_reports_time_and_memory` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; ASHIATO_EXPORT_VOLUME_ROWS=20000 ASHIATO_EXPORT_VOLUME_ERASED=4 scripts/quiet-run vol -- cargo test -p ashiato-server export_volume_one_run_reports_time_and_memory'` rc=0
- [x] 9.2 `tools/export-scaling.sh` を design D14 のとおりに書く（3 点・`ASHIATO_TEST_STATEMENT_TIMEOUT=0`・値と隣り合う組の比を標準出力へ・`EXPORT_SCALING_CMD` で差し替え・`--self-test`。比で合否を決めない）。
  本体の走りの前に `echo "Scenario: 記録の行数と消した滞在の数を一緒に倍にした 3 通りで、書き出しと件数の数えの時間を測って残す"` の印を出す。
  D9 の前の値が要るので、9.1・9.2 は 8.4 より先に済ませてよい（Task の番号は順を縛らない）。8.4 の前の木と後の木で 1 回ずつ走らせ、両方の値を design の「計測の記録」に書く。後が前より遅ければ D14 の反転条件。
  検証: `tools/export-scaling.sh --self-test` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run scaling -- tools/export-scaling.sh'` rc=0

## Task 10: 文書と全体の確かめ

- [ ] 10.1 `docs/testing.md` に 1 節（design D15）、`docs/flow-gates.md` の検査の表に `tools/check-test-db-growth.sh` を 1 行足す。
  検証: `bash -c 'grep -q "check-test-db-growth" docs/testing.md && grep -q "check-test-db-growth" docs/flow-gates.md && grep -q "ASHIATO_TEST_STATEMENT_TIMEOUT" docs/testing.md'` rc=0
- [ ] 10.2 試験の期待値・`Scenario:` の印・試験の本数が `origin/main` から減っていない（C1）。消えた `// Scenario:` の行と `fn` の名前が、足した行にもあることを見る。
  検証: `bash -c 'd=$(git diff origin/main...HEAD -- crates/server); for k in "Scenario:" "fn "; do r=$(printf "%s\n" "$d" | grep -E "^-[^-].*$k" | sed "s/^-//" | sort); a=$(printf "%s\n" "$d" | grep -E "^\+[^+].*$k" | sed "s/^+//" | sort); [ -z "$(comm -23 <(printf "%s\n" "$r") <(printf "%s\n" "$a") | grep -v "^$")" ] || exit 1; done'` rc=0
- [ ] 10.3 `tests/server_startup.rs`・`tools/smoke.sh`・`tools/dev.sh` は `ashiato` を使うまま（C7。CH04 はこれらに触らない）。
  検証: `bash -c '[ -z "$(git diff origin/main...HEAD -- crates/server/tests/server_startup.rs tools/smoke.sh tools/dev.sh)" ]'` rc=0
- [ ] 10.4 鎖の検査。
  検証: `openspec validate ch04-test-db-growth --strict` rc=0 / `python3 scripts/check_scenarios.py . ch04-test-db-growth` rc=0 / `tools/check-migrations.sh` rc=0 / `cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0
