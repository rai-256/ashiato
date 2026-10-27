# ST51 実装タスク — 更新で止めた時間が、更新の履歴として残る

読む順: `deep.md`（**本人の答え Q1〜Q5**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `specs/collection-coverage/spec.md` →
`specs/device-collection/spec.md` → `specs/desktop-collection/spec.md` → `specs/data-durability/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提: ST49 の実装**（requires）。`python3 scripts/story_deps.py included ST51 ST49` rc=0 を Task 1 の前に確かめる（下流の入口が ST49 の gate を通った SHA を取り込んでいる。
S-01 の組み立ての印 `ASHIATO_BUILD_STAMP`・C-01 の `BuildConfig.COMPONENT_VERSION`・`tools/check-build-stamp.sh`・ST49 の移行がそこにある）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin はコメント（`// Scenario: 生存信号の版は、行に残る`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 59 本**（`distribution` 18・`collection-coverage` 15・`device-collection` 9・`desktop-collection` 8・`data-durability` 9）。
  うち **ST51 が新しく印を置くのは 42 本**（`distribution` 18・`collection-coverage` 15・`device-collection` 2・`desktop-collection` 2・`data-durability` 4 の ADDED と、MODIFIED で中身を変えた
  `台帳と導出の表は書き出しに入らない`）—— **全部このファイルのどこかの Task が名指しする**。残りの 17 本は MODIFIED で本文をそのまま写した Scenario で、
  印は ST49（`device-collection` 7・`desktop-collection` 6）と既存の書き出しの試験（`data-durability` 4）が持つ。ST49 の
  「携帯の生存信号には版が書き込まれない」「PC の生存信号には版が書き込まれない」の試験は**消して**、ST51 の反対向きの Scenario に置き換える（残すと落ちる）
- **件数つき検証**: このファイルで
  - **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
    `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` がその項目に書いた本数以上
  - **`CW <名前>`** は `scripts/quiet-run cw -- cargo test -p ashiato-collector-windows --lib <名前>` rc=0 かつ
    `cargo test -p ashiato-collector-windows --lib <名前> -- --list 2>/dev/null | grep -c ': test$'` がその項目に書いた本数以上
  - **`GT <クラス>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*<クラス>*'` rc=0 かつ
    `cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*<クラス>*.xml | grep -c '<testcase '` がその本数以上
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので本数の下限を必ず見る）
- **試験の名前の接頭辞は `st51_`**。新しい DB の試験は `crates/server/src/update_history_tests.rs`（Task 2〜3）・`crates/server/src/version_change_tests.rs`（Task 4〜5・8）・
  `crates/server/src/export_update_history_tests.rs`（Task 6）に置き、`lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;` を足す。本物の bin の試験は `crates/server/tests/server_startup.rs` の `st51_` で始まる関数。
  C-01 の試験は `collector-android/app/src/test/kotlin/dev/ashiato/collector/St51HeartbeatVersionTest.kt`
- **本人の決定（下流は変えない）**: Q1 1 行の中身は いつ・部品・きっかけ・前の版・今の版・移行の結果（当てた版の名前 / 当てる版無し / 落ちた版と SQLSTATE / 退避の失敗）、DB の誤りの本文は残さない /
  Q2 落ちた回の行は D-01 の中だけ（戻した後の書き直しは ST58）/ Q3 収集アプリの版の変化は生存信号から導くビュー / Q4 端末・部品ごとに発信の時刻の順、版なしは外し、最初は申告の始まり /
  Q5 画面・API・格子・途絶の判定は変えない。仮: D1（全部の生存信号に版）・D2（migrate と起動の両方で書く）・D3（`core` の追記のみの表・書き出しに入れる）・D4（形の固定）・D5（導くビュー）。
  （第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- **名前は統一する**: 表 `core.update_history`（列は `specs/distribution` の表のとおり）、生存信号の欄と列 `client_version`、ビュー `core.collector_version_change`、
  書き出しのファイル `update_history.jsonl`・`collector_version_changes.jsonl`
- **組み立ての印を実行時に差し替える口を作らない**（D15）。更新の履歴を書く関数は印を引数に取り、試験は前の行を置いて比べる
- **生存信号の冪等キー（`heartbeat::content_hash`）の作り方を変えない**（ソース・発信の時刻・原文）。版は原文に入るので、版の違う 2 件は別の鍵になる
- **移行は 1 つ・名前は作成時刻**（`migrations/<YYYYMMDDHHMM>_update_history.sql` と `.down.sql`、`migrations/ORDER` の末尾へ 1 行）。名前の時刻は `202610250000` 以後で、ST48 の D10（仮）の規則
  （前進側に取引の文を持たない・`.down.sql` が版の記録の自分の行を消す）に従う。2 回当てても落ちない形（`IF NOT EXISTS`・`CREATE OR REPLACE`）
- **DB の誤りの本文をログにも行にも出さない**（Q1）。版の値・版の名前・SQLSTATE はログに出してよい（私的データではない）
- **稼働状況・途切れ・通知の問い合わせに `client_version` と更新の履歴を入れない**（D14）
- 縦串・書き換え禁止の節は Story ごとの新しいファイル（`tools/check-immutable.d/`）に置き、共有の台本の本体に書き足さない

## Task 1: 移行（更新の履歴の表と門・生存信号の版の列・版の変化のビュー）と付与

- [ ] 1.1 `migrations/<YYYYMMDDHHMM>_update_history.sql` と `.down.sql` を作り、`migrations/ORDER` の末尾へ 1 行足す。前進側:
  `core.update_history`（列と型と NULL の可否は `specs/distribution` の表どおり。`seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY`。`component = 'S-01'`・`via IN ('migrate','startup')`・
  `outcome IN ('applied','nothing_to_apply','failed','backup_failed')` の CHECK）、門 `core.reject_update_history_rewrite()`（BEFORE UPDATE OR DELETE の行の trigger と TRUNCATE の文の trigger。所有者でも断る —— `core.reject_heartbeat_rewrite` と同じ形）、
  `ALTER TABLE core.heartbeat ADD COLUMN IF NOT EXISTS client_version text NULL`、ビュー `core.collector_version_change`（D5・D6。列は D11 の 8 つ。`pg_input_is_valid(raw, 'jsonb')` で原文を確かめてから読む）。
  `.down.sql` は逆順に消し、版の記録の自分の行を消す
  検証: `./tools/check-migrations.sh` rc=0 / `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run build -- cargo build -p ashiato-server'` rc=0
- [ ] 1.2 `crates/server/src/grants.sql` の全部の表への付与の後ろに `REVOKE UPDATE, DELETE ON core.update_history FROM ashiato_app;` を足す（D10）。`update_history_tests.rs` に:
  - `st51_form_fixed` — `// Scenario: 更新の履歴の置き場と列の形は、決めた形のまま変わらない`（information_schema の名前・型・NULL の可否を Requirement の表と一字一句で突き合わせる）
  - `st51_form_rejects_unknown_via` — `// Scenario: 決めていない値の行は入らない`
  - `st51_rows_cannot_be_rewritten` — `// Scenario: 更新の履歴の行は書き換えられない`（所有者とアプリの役割の両方で UPDATE・DELETE）
  - `st51_app_role_reads_and_inserts` — `// Scenario: S-01 の実行時の役割は、更新の履歴を読み、足せる`
  - `st51_heartbeat_column_form` — 生存信号の `client_version` が text・NULL 可（D4）
  検証: `CT st51_form_` rc=0（`-- --list` で 2 本以上）/ `CT st51_rows_` rc=0（`-- --list` で 1 本以上）/ `CT st51_app_role_` rc=0（`-- --list` で 1 本以上）/ `CT st51_heartbeat_column_` rc=0（`-- --list` で 1 本以上）/
  `CT app_role_tests` rc=0（`-- --list` で 1 本以上。既存のアプリの役割の試験が通る）
- [ ] 1.3 `tools/check-immutable.d/<YYYYMMDDHHMM>_st51-update-history.sh` を足す（README の形。`core` を作り直してから付与を当て直し、所有者と `app_psql` の両方から更新の履歴の UPDATE・DELETE・TRUNCATE を撃って断られることを見る）
  検証: `./tools/check-immutable.sh` rc=0 / `./tools/check-shared-appends.sh` rc=0

## Task 2: migrate の終わりに更新の履歴を書く

- [ ] 2.1 新しい `crates/server/src/update_history.rs`: `record_migrate(conn, stamp, outcome)`・`record_startup(conn, stamp)`・前の版を読む（D7。`outcome IN ('applied','nothing_to_apply')` の `seq` の最大。取引の頭で `pg_advisory_xact_lock`）・
  表が無ければ書かずに `tracing::warn!(kind = "update_history_unwritten")`（D9。`to_regclass`）。`migration_ledger::apply_migrations_with` を、当てた版の名前と、落ちたときの版の名前と SQLSTATE を返す形に広げる（D8。誤りの本文は返さない）。
  `run_migrate` が終わりに（0・1 のどちらでも。3 では呼ばない）書く。落ちた行は落ちた版の取引が戻った後の別の取引で書く
- [ ] 2.2 `update_history_tests.rs`（試験用の並び = `MIGRATIONS` + 試験用の版。印は引数で渡す）:
  - `st51_migrate_applied_row` — `// Scenario: 新しい組み立ての migrate が版を当てると、当てた版の名前を持つ 1 行が残る`
  - `st51_migrate_nothing_to_apply_row` — `// Scenario: 当てる版が無い入れ替えの migrate も 1 行残る`
  - `st51_migrate_same_stamp_no_row` — `// Scenario: 同じ組み立てで migrate を走らせ直しても、行は増えない`
  - `st51_migrate_failed_row_sqlstate` — `// Scenario: 移行が落ちた回は、落ちた版と SQLSTATE を持つ 1 行が残り、誤りの本文は残らない`（一意の制約に反する版。行の全列の文字列に入れた値が無い）
  - `st51_migrate_after_failure_prev_is_last_passed` — `// Scenario: 落ちた回の後の行の前の版は、最後に通った版になる`
  - `st51_migrate_failed_row_survives_rollback` — `// Scenario: 落ちた回の行は、落ちた版と一緒に戻らない`
  - `st51_migrate_first_row_prev_empty` — `// Scenario: 更新の履歴の初めの行は、前の版を空にする`
  - `st51_migrate_no_table_warns` — `// Scenario: 更新の履歴の表が無い D-01 で落ちても、migrate の終了コードは変わらない`
  - `st51_migrate_prev_by_seq_not_time` — `// Scenario: 時計が戻っても、前の版は書いた順で決まる`
  - `st51_migrate_concurrent_one_row` — 同じ印の書き込みを 2 つ同時に走らせても 1 行（D7）
  検証: `CT st51_migrate_` rc=0（`-- --list` で 10 本以上）/ `CT migration_ledger_` rc=0（`-- --list` で 20 本以上。ST48 の台帳の試験が通る）
- [ ] 2.3 本物の bin の試験（`crates/server/tests/server_startup.rs`。所有者の接続で別の印 `0.1.0+<commit A>` の通った行を先に置く。D15）:
  - `st51_bin_migrate_backup_failed_row` — `// Scenario: 退避に失敗した回は、版を当てていない行が残る`（`PG_DUMP` を失敗する道具にする。終了コード 1）
  - `st51_bin_migrate_db_ahead_writes_nothing` — `// Scenario: 知らない版の当たった DB で止まった migrate は、何も書かない`
  - `st51_bin_migrate_writes_compiled_stamp` — bin の migrate の行の今の版が、その bin の組み立ての印（`ASHIATO_BUILD_STAMP`）と一致する
  検証: `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run st -- cargo test -p ashiato-server --test server_startup st51_bin_migrate_'` rc=0 かつ
  `cargo test -p ashiato-server --test server_startup st51_bin_migrate_ -- --list 2>/dev/null | grep -c ': test$'` が 3 以上

## Task 3: 起動のときに更新の履歴を書く

- [ ] 3.1 `run` の版の検査（`startup_refusal`）の後・待ち受けの前に `record_startup(&pool, stamp)` を呼ぶ（D2・D8。実行時の役割の接続。書けなくても起動は続ける —— D9）
- [ ] 3.2 本物の bin の試験（`crates/server/tests/server_startup.rs`）:
  - `st51_bin_startup_row_on_swap` — `// Scenario: migrate を走らせずに入れ替えた S-01 は、起動のときに 1 行を書く`（先に印 `<commit A>` の通った行を置き、bin を起動。`/healthz` 200 の後に行を見る）
  - `st51_bin_startup_after_migrate_no_row` — `// Scenario: migrate の後に同じ組み立ての S-01 を起動しても、行は増えない`
  - `st51_bin_startup_refused_writes_nothing` — `// Scenario: 版の検査で断られた起動は、何も書かない`
  検証: `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run st -- cargo test -p ashiato-server --test server_startup st51_bin_startup_'` rc=0 かつ
  `cargo test -p ashiato-server --test server_startup st51_bin_startup_ -- --list 2>/dev/null | grep -c ': test$'` が 3 以上 /
  `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run st -- cargo test -p ashiato-server --test server_startup st48_'` rc=0（ST48 の起動の試験が通る）

## Task 4: 生存信号の受け口の `client_version`

- [ ] 4.1 `HeartbeatRequest` に `#[serde(default)] pub client_version: Option<String>` を足し、INSERT で列に入れる（空の文字列は NULL。U+0000 は取り除く。形を検査しない —— D6）。
  書庫の取り込み器が積む生存信号は `None`（D13）。`docs/openapi.json` を再生成する。`docs/collector-contract.md` の生存信号の節に欄を足す（「積んだ時の版。原文にも同じ値」）
- [ ] 4.2 `version_change_tests.rs`:
  - `st51_hb_version_stored` — `// Scenario: 生存信号の版は、行に残る`
  - `st51_hb_without_version_accepted` — `// Scenario: 版の欄を持たない生存信号も受け付ける`
  - `st51_hb_malformed_version_accepted` — `// Scenario: 形の崩れた版でも、生存信号は断られない`
  - `st51_hb_hash_is_pinned` — `// Scenario: 版の欄を足しても、生存信号の冪等キーは変わらない`（欄を足す前の固定値を試験に書く）
  - `st51_hb_archive_has_no_version` — `// Scenario: 書庫の取り込みが積む生存信号は版を持たない`
  - `st51_hb_hash_inputs_only` — 冪等キーがソース・発信の時刻・原文だけから作られる（premises 4 を確かさ 4 に上げる）
  検証: `CT st51_hb_` rc=0（`-- --list` で 6 本以上）/ `./tools/check-openapi.sh` rc=0 / `jq -e '.components.schemas.HeartbeatRequest.properties | has("client_version")' docs/openapi.json` rc=0

## Task 5: 収集アプリの版の変化のビュー

- [ ] 5.1 Task 1.1 のビューの振る舞いを `version_change_tests.rs` で確かめる（生存信号は受け口か直接の INSERT で置く。発信の時刻と受信の時刻を指定する）:
  - `st51_view_one_change_on_update` — `// Scenario: 収集アプリを更新すると、版の変化が 1 行出る`
  - `st51_view_late_queued_no_false_change` — `// Scenario: 更新の前に溜まった信号が後から届いても、偽の変化は出ない`
  - `st51_view_first_is_start_not_change` — `// Scenario: 最初に版を持った信号は、変化として出ない`
  - `st51_view_skips_versionless` — `// Scenario: 版を持たない信号が挟まっても、変化は版を持つ信号どうしで数える`
  - `st51_view_reads_raw_when_column_empty` — `// Scenario: 列に版が無い信号は、原文の版で数える`
  - `st51_view_non_json_raw_ok` — `// Scenario: 原文が JSON でない信号があっても、読み口は失敗しない`
  - `st51_view_separate_device_component` — `// Scenario: 端末と部品が違う信号は、別の並びで数える`
  - `st51_view_two_sources_one_change` — `// Scenario: 同じ端末の C-01 の 2 つのソースが同じ版を申告しても、変化は 1 行`
  - `st51_view_old_server_drops_field_keeps_raw` — 欄の無い要求（ST51 より前の S-01 が読み飛ばした形）でも原文の版で変化が出る（premises 3 を確かさ 4 に上げる）
  検証: `CT st51_view_` rc=0（`-- --list` で 9 本以上）

## Task 6: 書き出し

- [ ] 6.1 `export.rs`: `core.update_history` を `Class::Include`（`update_history.jsonl`・全列）にし、`HEARTBEAT_COLUMNS` に `client_version` を足し、読み口のファイル `collector_version_changes.jsonl` を写しの取引の中で書く（D11）。
  説明書に 3 つの説明（`specs/data-durability` の ADDED の文言）を足す。既存の `export_file_excludes_ledgers_and_derived_tables` の期待するファイルの一覧に 2 つを足す
- [ ] 6.2 `export_update_history_tests.rs`:
  - `st51_export_update_history_rows` — `// Scenario: 書き出しに更新の履歴の全行と全列が入る`
  - `st51_export_version_changes` — `// Scenario: 書き出しに収集アプリの版の変化が入る`
  - `st51_export_heartbeat_version_column` — `// Scenario: 書き出しの生存信号のファイルに版の列が入る`
  - `st51_export_readme_explains` — `// Scenario: 説明書が更新の履歴と版の変化と生存信号の版を説明する`
  - `st51_export_view_needs_no_class` — ビューの列が分類を求めない（`unclassified` が空。D11）
  既存の `export_file_excludes_ledgers_and_derived_tables` の印 `// Scenario: 台帳と導出の表は書き出しに入らない` はそのまま（期待だけを直す）
  検証: `CT st51_export_` rc=0（`-- --list` で 5 本以上）/ `CT export_` rc=0（`-- --list` で 130 本以上。既存の書き出しの試験が通る）

## Task 7: PC の収集アプリ（C-02）の生存信号の版

- [ ] 7.1 `crates/collector-windows/src/heartbeat.rs` の信号を組む所で、ST49 の印を `client_version` の欄と原文の `client_version` の両方に入れる（D12）。溜めた行は読み戻した形のまま送る（欄が無ければ足さない）。
  `heartbeat_raw_has_no_private_content` の鍵の一覧に `client_version` を足す。ST49 の試験「PC の生存信号には版が書き込まれない」を消す
- [ ] 7.2 `#[cfg(test)]` の試験:
  - `st51_c02_heartbeat_keeps_enqueued_version` — `// Scenario: PC の生存信号は、積んだときの版を欄と原文に持つ`（置き場を別の印で開き直して送る）
  - `st51_c02_old_heartbeat_sent_without_version` — `// Scenario: PC の版の欄を持たない溜めた生存信号は、版の無いまま送られる`
  - `st51_c02_heartbeat_first_after_start` — 起動の直後に生存信号を 1 件積む（`Schedule::new()` の最初の `due`。premises 1 を確かさ 4 に上げる）
  - `st51_c02_heartbeat_is_queued` — 生存信号は未送信に溜まり、送る前に落とした後も残る（premises 2）
  検証: `CW st51_c02_` rc=0（`-- --list` で 4 本以上）/ `scripts/quiet-run cw -- cargo test -p ashiato-collector-windows --lib` rc=0（既存の試験が通る）

## Task 8: 携帯の収集アプリ（C-01）の生存信号の版

- [ ] 8.1 `collector-android/app/src/main/kotlin/dev/ashiato/collector/Heartbeat.kt` の `emit()` で、`BuildConfig.COMPONENT_VERSION` を欄と原文の `client_version` に入れる（D12）。
  ST49 の試験「携帯の生存信号には版が書き込まれない」を消す
- [ ] 8.2 `St51HeartbeatVersionTest.kt`（Robolectric）:
  - `// Scenario: 携帯の生存信号は、積んだときの版を欄と原文に持つ`（置き場に積んだ後、別の印の版を返す差し替えで送る）
  - `// Scenario: 版の欄を持たない溜めた生存信号は、版の無いまま送られる`
  - 起動の直後（`startBeating()`）に刻みを待たず 1 件積む（premises 1）
  検証: `GT St51HeartbeatVersionTest` rc=0（`<testcase` が 3 以上）/ `GT Heartbeat` rc=0（`<testcase` が 20 以上。既存の生存信号の試験が通る）

## Task 9: 稼働状況と途切れの判定が変わらない

- [ ] 9.1 `version_change_tests.rs`:
  - `st51_coverage_same_with_or_without_version` — `// Scenario: 版の欄が違うだけの生存信号は、同じ稼働状況になる`（2 つの D-01 の稼働状況と途切れの応答を JSON で比べる）
  - `st51_coverage_outage_still_shown` — `// Scenario: 更新の間の空きは、更新の履歴があっても途絶として出る`
  検証: `CT st51_coverage_` rc=0（`-- --list` で 2 本以上）/ `CT outage_` rc=0（`-- --list` で 20 本以上。既存の途切れの試験が通る）

## Task 10: 申し送りと文書

- [ ] 10.1 `docs/handoff/ST58.md` の申し送り（上流が置いた節）を読み、ST58 が読む `update_history` の列と `outcome` の値・戻した後の 2 行の書き直しの前提が実装と合っているかを確かめる。食い違えば節に追記する
  検証: `python3 scripts/check_chain.py .` rc=0
- [ ] 10.2 全体の検査
  検証: `python3 scripts/check_scenarios.py . st51-update-history` rc=0 / `./tools/check-migrations.sh` rc=0 / `./tools/check-shared-appends.sh` rc=0
