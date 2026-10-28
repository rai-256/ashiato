# ST49 実装タスク — 記録ごとに、作った部品の版が残る

読む順: `deep.md`（**本人の答え Q1〜Q4**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/record-envelope/spec.md` → `specs/device-collection/spec.md` →
`specs/desktop-collection/spec.md` → `specs/derived-records/spec.md` → `specs/data-durability/spec.md` → `specs/record-deletion/spec.md` → `specs/subjective-log/spec.md` →
`specs/personal-entities/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提: ST48 の実装**（requires）。`python3 scripts/story_deps.py included ST49 ST48` rc=0 を Task 1 の前に確かめる（下流の入口が ST48 の gate を通った SHA を取り込んでいる。
`crates/server/build.rs` の `build_commit()` と `migration_ledger` と `tools/check-migrations.sh` の D10 の規則がそこにある）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 収集アプリの記録は、収集アプリの版と書いた S-01 の版を持つ`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 65 本**（`record-envelope` 27・`device-collection` 14・`desktop-collection` 7・`derived-records` 5・`data-durability` 5・`record-deletion` 3・`subjective-log` 2・`personal-entities` 2）。
  **全部 ST49 で新しく足した ADDED で、全部このファイルのどこかの Task が名指しする**
- **件数つき検証**: このファイルで
  - **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
    `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` がその項目に書いた本数以上
  - **`CW <名前>`** は `scripts/quiet-run cw -- cargo test -p ashiato-collector-windows --lib <名前>` rc=0 かつ
    `cargo test -p ashiato-collector-windows --lib <名前> -- --list 2>/dev/null | grep -c ': test$'` がその項目に書いた本数以上（ubuntu で走る `#[cfg(test)]`）
  - **`CS <名前>`** は `scripts/quiet-run cs -- cargo test -p ashiato-build-stamp <名前>` rc=0 かつ `cargo test -p ashiato-build-stamp <名前> -- --list 2>/dev/null | grep -c ': test$'` がその本数以上
  - **`GT <クラス>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*<クラス>*'` rc=0 かつ
    `cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*<クラス>*.xml | grep -c '<testcase '` がその本数以上
  - **`VT <ファイル>`** は `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/<ファイル> 2>&1 | tee "$TMPDIR/vt.log"' && grep -Eq 'Tests +[1-9][0-9]* passed' "$TMPDIR/vt.log"` rc=0
  - **`STAMP <節>`** は `scripts/quiet-run stamp -- ./tools/check-build-stamp.sh <節>` rc=0（節は `tool` / `server` / `windows` / `android`。Task 1.2 で作る）
  （`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので本数の下限を必ず見る）
- **試験の名前の接頭辞は `st49_`**。新しい DB の試験は `crates/server/src/component_version_tests.rs`（Task 2〜6）と `crates/server/src/export_component_version_tests.rs`（Task 7）に置き、
  `lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;` を足す。C-01 の試験は `collector-android/app/src/test/kotlin/dev/ashiato/collector/St49*Test.kt`、画面は `web/src/__tests__/buildStamp.test.ts` と `web/e2e/st49-client-version.spec.ts`
- **本人の決定（下流は変えない）**: Q1 版は `<版番号>+<commit 40 桁>[.dirty][.debug]`（`.dirty` はその部品のソースの未 commit の変更）/ Q2 S-01 の版をすべての記録に（衛星も。衛星自身の版は付けない）/
  Q3 破棄の報告・原本の欠けに収集アプリの版と S-01 の版、画面の操作の台帳（削除・気分の通知・滞在の基準・個人属性の種類の名前）に S-01 の版 / Q4 画面の記録は画面の版と S-01 の版。
  仮: D1（更新で版が動く）・D2（滞在の作り直しで版が動く）・D3（溜めの知らない欄を送り返す・S-01 を先に入れ替える）・D21（版の書き換えは前の版を積むときだけ）。
  （第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- **列と欄の名前は統一する**: DB の列と書き出しの列は `collector_version` / `web_version` / `server_version`、取り込み口・破棄の報告・原本の欠けの要求の欄は `client_version` の 1 つ（D8）。
  S-01・C-02 の組み立ての印の env は `ASHIATO_BUILD_STAMP`、C-01 は `BuildConfig.COMPONENT_VERSION`、画面は `__ASHIATO_WEB_VERSION__`
- **印の形の正規表現は 1 つ**: `^[0-9][^+]*\+([0-9a-f]{40}|unknown)(\.dirty)?(\.debug)?$`（D5）。4 つの組み立ての試験が同じ式で見る
- **内容の鍵（`content_hash_of`）・破棄の報告の鍵・原本の欠けの鍵・原文の形を変えない**（D11）。`hash_is_pinned` の固定値を変える変更は落とす
- **移行は 1 つ・名前は作成時刻**（`migrations/<YYYYMMDDHHMM>_record_component_version.sql` と `.down.sql`、`migrations/ORDER` の末尾へ 1 行）。名前の時刻は `202610250000` 以後で、ST48 の D10（仮）の規則に従う（D7）。
  2 回当てても落ちない形。門と錠の関数・読み口は**移行を当てる時点の本体を写して版の列を足すだけ**にし、ほかの条件を変えない（D19。ST41 が先に main に入っていれば終わりの列を含む本体を写す）
- **版の値は検査しない・断らない**（D8）。空の文字列だけを NULL、U+0000 は取り除く。前後の空白は除かない
- **衛星の資格情報・選び出す口・生存信号の版・送信契約の版は作らない**（D18）
- 記録の値（原文・時刻・TZ）をログに出さない。版の値はログに出してよい（私的データではない）

## Task 1: 組み立ての印（`crates/build-stamp`）と S-01 の印

- [x] 1.1 `crates/build-stamp`（`ashiato-build-stamp`。workspace の members に足す。依存は `std` だけ）を作る。関数: 印を組む `stamp(version, profile, repo, source_paths)`（D5 の形・commit の出所の順・`git status --porcelain -- <source_paths>`・`GIT_OPTIONAL_LOCKS=0`）と、
  作り直しの印を出す `emit_rerun(repo, source_paths)`（D6 の `rerun-if-changed` / `rerun-if-env-changed`。部品のソースの道を含む）。単体試験（使い捨ての git の木を `$TMPDIR` の下に作る）:
  - `st49_stamp_dirty_debug` — `// Scenario: 印は版番号・commit・未 commit の印・組み立ての種類を持つ`
  - `st49_stamp_release_clean` — `// Scenario: 綺麗な木の release の組み立ての印は、版番号と commit だけを持つ`
  - `st49_stamp_other_tree_is_not_dirty` — `// Scenario: ほかの部品の木だけが変わっていても、印は .dirty を持たない`
  - `st49_stamp_unknown_without_git` — `// Scenario: commit が取れない場所での組み立ての印は unknown を持つ`（`ASHIATO_BUILD_COMMIT` を消し、git の木の外で）
  - `st49_stamp_env_commit_wins`・`st49_stamp_rerun_paths_follow_worktree_git_dir`・`st49_stamp_status_does_not_touch_index`（`index` の mtime が変わらない）
  検証: `CS st49_stamp_` rc=0（`-- --list` で 7 本以上）
- [x] 1.2 `tools/check-build-stamp.sh` を作る。節ごとに、`git clone --local` で repo を `$TMPDIR` の下へ写し（`CARGO_TARGET_DIR` も `$TMPDIR` の下）、本物の部品を組み立てて埋め込まれた印を読み（Rust は組み立てた bin を `strings` して D5 の正規表現で探す、C-01 は生成された `BuildConfig.java`）、
  commit を 1 つ足して組み立て直す・部品のソースを 1 つ書き換えて add せずに組み立て直す、の 2 つで印が変わることを見る。節:
  - `tool`: 1.1 の関数を最小の crate から呼んで印を出す（節の動作の確かめ。印なし）
  - `server`（`cargo build -p ashiato-server --bin openapi`）: `echo "== Scenario: S-01 の組み立ての印は、commit を進めた後の差分の組み立てでも新しい commit になる"`・`echo "== Scenario: S-01 の組み立ての印は、add していないソースの変更の後の差分の組み立てで .dirty になる"`
  - `windows` と `android` は Task 8・9 で足す（節が無ければ rc≠0 で「節が無い」と出す）
  検証: `STAMP tool` rc=0
- [x] 1.3 `crates/server/build.rs` の ST48 の `build_commit()` を `ashiato_build_stamp` に置き換え（部品のソースの道は `crates/server`・`crates/build-stamp`・`migrations/`・`Cargo.toml`・`Cargo.lock`。D5）、`cargo:rustc-env=ASHIATO_BUILD_STAMP=<印>` を足す（`ASHIATO_BUILD_COMMIT` は同じ commit で出し続ける）。
  `crates/server/src/lib.rs` に `pub const BUILD_STAMP: &str = env!("ASHIATO_BUILD_STAMP");`。
  検証: `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run build -- cargo build -p ashiato-server'` rc=0 / `CT migration_ledger_` rc=0（`-- --list` で 20 本以上。ST48 の台帳の試験が通る）/ `STAMP server` rc=0

## Task 2: 移行（版の列・読み口・門と前の版の錠）と門の試験

- [x] 2.1 `migrations/<作成時刻>_record_component_version.sql` を design D7 のとおりに書く（8 表に列・`core.event_live` と `core.event_version_live` の作り直し・`core.require_version_or_ledger`（D21（仮））と `core.reject_version_rewrite` に 3 列）。
  `.down.sql` は逆順で今の形に戻し、`migration_ledger.applied` の行を消す。`migrations/ORDER` の末尾へ 1 行（前の行に `# ST49: 記録ごとの部品の版（8 表の列・読み口・門と前の版の錠）` の注記）。
  検証: `./tools/check-migrations.sh` rc=0 / `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run build -- cargo build -p ashiato-server'` rc=0
- [x] 2.2 `component_version_tests.rs` を作り、移行と門の試験を置く（`core.event` / `core.event_version` を直に書く。移行の前後は `media_migration_tests.rs` の使い捨ての DB の形で、ST49 の前の版まで当てて行を置いてから ST49 の移行を当てる）:
  - `st49_gate_versions_start_unknown` — `// Scenario: 版を残し始める前に入った記録の版は「不明」のまま`（移行を 2 回当てても落ちないことも見る）
  - `st49_gate_ledgers_start_unknown_deletion` — `// Scenario: 版を残し始める前の削除の台帳の行は、版が「不明」のまま`
  - `st49_gate_ledgers_start_unknown_reminder` — `// Scenario: 版を残し始める前の気分の通知の台帳の行は、版が「不明」のまま`
  - `st49_gate_ledgers_start_unknown_criteria` — `// Scenario: 版を残し始める前の滞在の基準の台帳の行は、版が「不明」のまま`
  - `st49_gate_ledgers_start_unknown_kind_name` — `// Scenario: 版を残し始める前の個人属性の種類の名前の台帳の行は、版が「不明」のまま`
  - `st49_gate_version_rewrite_without_history_rejected` — `// Scenario: 収集した記録の版を、前の版の履歴なしで書き換えると拒まれる`（3 列それぞれ）
  - `st49_gate_version_fix_with_history_keeps_old` — `// Scenario: 前の版を同じまとまりに積めば、収集した記録の版を直せて、前の値が残る`
  - `st49_gate_history_versions_are_frozen` — `// Scenario: 前の版の履歴の版は書き換えられない`（3 列それぞれ）
  - `st49_gate_raw_rewrite_still_rejected` — `// Scenario: 版の列を足しても、収集した記録の原文の書き換えは拒まれる`
  - `st49_cols_history_and_live_views_match` — `// Scenario: 前の版の読み口と前の版の表は、記録の表と同じ版の列を持つ`（`information_schema.columns` で `core.event`・`core.event_version`・`core.event_version_live`・`core.event_live` を見る）
  検証: `CT st49_gate_` rc=0（`-- --list` で 9 本以上）/ `CT st49_cols_` rc=0（`-- --list` で 1 本以上）/ `CT dedup_tests` rc=0（`-- --list` で 35 本以上。既存の外部更新が門を通る）
- [x] 2.3 `tools/check-immutable.d/<作成時刻>_st49-record-component-version.sh` に、前の版の版の書き換え（3 列）と、前の版なしの本表の版の書き換えが拒まれることを `tools/check-immutable.sh` の既存の節と同じ書き方で置く
  （印は `echo "== Scenario: 前の版の履歴の版は書き換えられない"` と `echo "== Scenario: 収集した記録の版を、前の版の履歴なしで書き換えると拒まれる"`。本体の末尾に書き足さない）。
  検証: `./tools/check-immutable.sh` rc=0 / `./tools/check-shared-appends.sh` rc=0

## Task 3: 取り込み口の `client_version` と、資格情報での振り分け

- [x] 3.1 先に試験を書く（`component_version_tests.rs`。HTTP の試験は `api_tests.rs` の組み立てで `router()` を通し、API の合言葉と画面のログインの印の両方で送る）:
  - `st49_ingest_collector_version_and_server_version` — `// Scenario: 収集アプリの記録は、収集アプリの版と書いた S-01 の版を持つ`
  - `st49_ingest_server_version_is_build_stamp` — `// Scenario: 記録の S-01 の版は、その S-01 の組み立ての印と同じで、印の形を持つ`（`crate::BUILD_STAMP` と一字一句同じ・D5 の正規表現に合う）
  - `st49_ingest_archive_gets_server_version_only` — `// Scenario: 書庫の取り込みの記録は、書いた S-01 の版だけを持つ`（`crate::store_one` を通す）
  - `st49_ingest_without_field_is_unchanged` — `// Scenario: 部品の版の欄を持たない要求の記録は、部品の版が「不明」で入る`（いまの収集アプリと同じ JSON を送り、結果の JSON の鍵の集合と状態符号が欄を足す前と同じ）
  - `st49_ingest_empty_or_odd_version_is_accepted` — `// Scenario: 部品の版が空か形の違う文字列でも、記録は断られない`
  - `st49_ingest_nul_is_removed` — `// Scenario: 格納できない文字を含む部品の版は、その文字を取り除いて残る`（同じまとめ送りの他の 1 件も入る）
  - `st49_ingest_web_session_goes_to_web_version` — `// Scenario: 画面のログインの要求の部品の版は、画面の版として入る`
  - `st49_ingest_internal_writer_drops_client_version` — `// Scenario: 合言葉でもログインでもない書き手の部品の版は、どこにも入らない`
  - `st49_dedup_version_only_difference_folds` — `// Scenario: 部品の版だけが違う同じ記録は、1 件のまま`
  いまの木で `st49_ingest_collector_version_and_server_version` が落ちることを `CT st49_ingest_collector` rc≠0 で確かめてから 3.2 に進む
- [x] 3.2 `ingest.rs` の `IngestRequest` に `client_version`（D8。空と U+0000 の扱いは `IngestRequest` の関数 1 つに閉じる）、`lib.rs` に `Writer`（D9。`match` に `_` を置かない）と `ingest_request` の引数、
  `ingest` は `decided()` の `Caller` から、`store_one` は `Writer::Internal` を渡す。`ingest_one` の 2 つの INSERT に 3 列（D10）。**前の版の行を書く 2 か所（`apply_external_update` の前の版の INSERT と `stay_store.rs` の `push_version`）にも、ここで 3 列を写す** ——
  門（Task 2 の D21（仮））は前の版の行の版を更新前の値と照らすので、写さないと版を持つ記録の外部更新が門で落ちる（版を動かす UPDATE は 4.2）。
  `ingest.rs` の単体試験に `st49_hash_ignores_client_version` — `// Scenario: 部品の版を足しても内容の鍵は変わらない`（`hash_is_pinned` と同じ固定値）。
  検証: `CT st49_ingest_` rc=0（`-- --list` で 8 本以上）/ `CT st49_dedup_` rc=0（`-- --list` で 1 本以上）/
  `bash -c 'scripts/quiet-run unit -- cargo test -p ashiato-server --lib st49_hash'` rc=0（`-- --list` で 1 本以上）/ `CT dedup_tests` rc=0（`-- --list` で 35 本以上。既存の重複の判定が変わらない）/
  `bash -c 'scripts/quiet-run unit -- cargo test -p ashiato-server --lib hash_is_pinned'` rc=0（`-- --list` で 1 本以上）
- [x] 3.3 `cargo run -q -p ashiato-server --bin openapi > docs/openapi.json` で契約を再生成し、`docs/collector-contract.md` に `client_version`（任意・検査しない・積む時点の版・資格情報で振り分ける・入れ替えは S-01 が先。design の Migration Plan）を足す。
  検証: `./tools/check-openapi.sh` rc=0 / `jq -e '.components.schemas.IngestRequest.properties | has("client_version")' docs/openapi.json` rc=0 /
  `bash -c 'test "$(grep -c client_version docs/collector-contract.md)" -ge 3'` rc=0

## Task 4: 外部サービス由来の更新で版が動く（D1（仮））

- [x] 4.1 先に試験を書く（`component_version_tests.rs`。外部の識別子の粒度つきのソースを `testdb::source_of_kind` で登録する。「印 A の S-01 が入れた記録」は、試験から `core.event` へ直に INSERT して `server_version`・`collector_version` を `0.1.0+<commit A>` にして作る（INSERT は門に掛からない）。2 回目の到着は API の合言葉で送る）:
  - `st49_update_versions_move_and_history_keeps_old` — `// Scenario: 外部サービス由来の更新で、版が新しい到着の版に替わり、前の版が履歴に残る`（本表の `server_version` が `crate::BUILD_STAMP` に、前の版の行が `0.1.0+<commit A>` に。前の版の読み口 `core.event_version_live` でも読める）
  - `st49_update_same_content_keeps_versions` — `// Scenario: 内容の変わらない再到着では、版は変わらない`
  - `st49_update_stale_keeps_versions` — `// Scenario: 古い更新時刻の到着では、版は変わらない`
  - `st49_update_deleted_keeps_versions` — `// Scenario: 削除済みの記録への更新では、版は変わらない`
  いまの木（Task 3 の後）で `st49_update_versions_move_and_history_keeps_old` が版の値のアサートで落ちる（門では落ちない）ことを `CT st49_update_versions` rc≠0 と失敗の文で確かめてから 4.2 に進む
- [x] 4.2 `lib.rs` の `Stored` と `load_stored` に 3 列、`apply_external_update` の本表の UPDATE で 3 列を動かす（D1（仮）。前の版の INSERT は 3.2 で済んでいる）。`advance_watermark` は変えない。
  検証: `CT st49_update_` rc=0（`-- --list` で 4 本以上）/ `CT dedup_tests` rc=0（`-- --list` で 35 本以上。既存の外部更新の試験が通る）

## Task 5: 滞在と滞在の基準の版（D2（仮））

- [x] 5.1 先に試験を書く（`component_version_tests.rs`。位置の記録を置いて `stay_store::rebuild_all` を走らせる。「別の S-01 の版」は試験から `server_version` を直に `0.1.0+<commit A>` に書き換えて作る（滞在は収集した記録でないので本表の門に掛からない。D21（仮））:
  - `st49_stay_new_has_server_version` — `// Scenario: 新しい滞在は、書いた S-01 の版を持つ`
  - `st49_stay_rebuild_with_new_raw_moves_version` — `// Scenario: 作り直しで原文が変わった滞在は、版が替わり、前の版が履歴に残る`
  - `st49_stay_rebuild_unchanged_keeps_version` — `// Scenario: 作り直しで原文が変わらなかった滞在の版は変わらない`（削除の印だけを書き換えた滞在の版も変わらないことを同じ試験で見る）
  - `st49_stay_criteria_row_has_server_version` — `// Scenario: 滞在の基準を変えた台帳の行は、書いた S-01 の版を持つ`（画面のログインで `POST /stays/rebuild` に基準を渡す）
- [x] 5.2 `stay_store.rs` の `insert_stay`・`update_stay`（原文が変わったときだけ）・`set_criteria` の INSERT に版の列（D2（仮）・D10。`push_version` は 3.2 で済んでいる）。
  検証: `CT st49_stay_` rc=0（`-- --list` で 4 本以上）/ `CT stay_tests` rc=0（`-- --list` で 65 本以上。既存の滞在の試験が通る）

## Task 6: 破棄の報告・原本の欠け・画面の操作の台帳（S-01 の側）

- [x] 6.1 先に試験を書く（`component_version_tests.rs`）:
  - `st49_ledger_drop_report_versions` — `// Scenario: 破棄の報告は、作った収集アプリの版と書いた S-01 の版を持つ`
  - `st49_ledger_media_missing_versions` — `// Scenario: 原本の欠けは、作った収集アプリの版と書いた S-01 の版を持つ`
  - `st49_ledger_drop_report_version_only_difference_folds` — `// Scenario: 部品の版だけが違う同じ破棄の報告は、1 件のまま`
  - `st49_ledger_reports_without_field_are_accepted` — `// Scenario: 部品の版の欄を持たない破棄の報告と原本の欠けも受け付けられる`
  - `st49_ledger_erase_row_has_server_version` — `// Scenario: 画面から消した操作の台帳の行は、書いた S-01 の版を持つ`（画面のログインで `POST /stays/erase`）
  - `st49_ledger_restore_row_has_server_version` — `// Scenario: 画面から戻した操作の台帳の行も、書いた S-01 の版を持つ`（`POST /stays/restore`）
  - `st49_ledger_feeling_reminder_has_server_version` — `// Scenario: 通知の時刻を変えた台帳の行は、書いた S-01 の版を持つ`
  - `st49_ledger_kind_name_has_server_version` — `// Scenario: 個人属性の種類を足した名前の台帳の行は、書いた S-01 の版を持つ`（`POST /attributes/kinds`）
  - `st49_ledger_inserts_name_server_version` — `crates/server/src` の試験でないファイル（`*_tests.rs`・`testdb.rs`・`#[cfg(test)]` の mod の中を除く）の、`server_version` を持つ 8 表への `INSERT INTO core.<表>` の文が、列の並びに `server_version` を名指すことを文字列で見る（印なし。D10 の機械の止め）
- [x] 6.2 `drops.rs`・`media_missing.rs` の要求に `client_version`（D8）と INSERT に 2 列（`Writer::Collector` のときだけ収集アプリの版。D9・D14）、`deletion.rs` と `stay_store.rs` の削除の台帳・`feelings.rs` の通知の台帳・`attributes_store.rs` の種類の名前の INSERT に `server_version`。
  重複の判定の問い合わせは変えない（D11）。`cargo run -q -p ashiato-server --bin openapi > docs/openapi.json` で契約を再生成し、`docs/collector-contract.md` の破棄の報告と原本の欠けの節にも欄を足す。
  検証: `CT st49_ledger_` rc=0（`-- --list` で 9 本以上）/ `CT drops_tests` rc=0（`-- --list` で 15 本以上）/ `CT media_missing_tests` rc=0（`-- --list` で 6 本以上）/ `CT deletion_tests` rc=0（`-- --list` で 26 本以上）/ `CT feelings_remind` rc=0（`-- --list` で 8 本以上）/
  `./tools/check-openapi.sh` rc=0 / `jq -e '(.components.schemas.DropReportRequest.properties | has("client_version")) and (.components.schemas.MediaMissingRequest.properties | has("client_version"))' docs/openapi.json` rc=0

## Task 7: 書き出しの列と説明（D17・D4）

- [x] 7.1 先に試験を書く（`export_component_version_tests.rs`。配置と書き出しの読み方は `export_file_tests.rs` に合わせ、zip を開いて行の鍵と値を見る）:
  - `st49_export_event_rows_have_versions` — `// Scenario: 書き出しの記録の行は、3 つの版を持つ`
  - `st49_export_version_rows_have_versions` — `// Scenario: 書き出しの前の版の行は、その版の版を持つ`
  - `st49_export_ledger_rows_have_versions` — `// Scenario: 書き出しの破棄の報告と画面の操作の台帳は、版の列を持つ`
  - `st49_export_doc_distinguishes_declared_and_server` — `// Scenario: 書き出しの説明書は、申告の版と S-01 が入れた版を書き分ける`
  - `st49_export_doc_says_current_content` — `// Scenario: 書き出しの説明書は、版を今の中身を作った部品の版と説明する`（`EVENT_COLUMNS` の 3 列の説明に「今の中身」、`EVENT_VERSION_COLUMNS` の 3 列に「その版の中身」があり、6 つのどれにも「記録を作った」「記録を組んだ」が無いことを見る）
  - `st49_export_existing_columns_are_kept` — `// Scenario: 版の列を足しても、書き出しの今の列は残る`（今の `EVENT_COLUMNS` などの名前の一覧を試験の中に逐語で写し、足した後の一覧に全部あり、差が版の列だけであることを見る）
- [x] 7.2 `export.rs` の `EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS`（`schema_version` の次）・`DROP_REPORT_COLUMNS`・`FEELING_REMINDER_COLUMNS`・`STAY_CRITERIA_COLUMNS`・`ATTRIBUTE_KIND_NAME_COLUMNS` に列と説明（D4 の文）、`export/doc.rs` の説明書に同じ文。
  検証: `CT st49_export_` rc=0（`-- --list` で 6 本以上）/ `CT export_` rc=0（`-- --list` で 130 本以上。既存の書き出しの試験が通る。表の列を一覧に足し忘れると落ちる）

## Task 8: PC の収集アプリ（C-02）の印・焼き込み・欄の保持

- [x] 8.1 `crates/collector-windows/build.rs` を作り（`ashiato-build-stamp` を build-dependency に。部品のソースの道は `crates/collector-windows`・`crates/build-stamp`・`Cargo.toml`・`Cargo.lock`。D5）、`ASHIATO_BUILD_STAMP` を出す。`contract.rs` の `IngestRequest` に
  `#[serde(default, skip_serializing_if = "Option::is_none")] client_version` と `#[serde(flatten)]` の知らない欄の置き場（D3（仮））、記録の溜め（`events`）に積む 1 か所で `client_version` を入れる（D13。生存信号の `beats` には入れない）。
  試験（`#[cfg(test)]`。`outbox.rs` / `contract.rs` / `runtime.rs` / `history` の隣）:
  - `st49_c02_pending_keeps_stamp_from_add_time` — `// Scenario: PC の収集アプリを更新した後に送った未送信は、積んだときの版で届く`（印 A で積んだ行を、印を差し替えた組み立て（試験では印を引数で渡す関数）で開き直して送る要求を見る）
  - `st49_c02_old_line_is_sent_without_version` — `// Scenario: PC の版の欄を持たない未送信の行は、版の欄の無いまま送られる`（材料: 版の欄を持たない今の形の 1 行を逐語で置く。`broken_lines()` が 0）
  - `st49_c02_unknown_field_survives_resend` — `// Scenario: PC の未送信の行が知らない欄を持っていても、送るときに落ちない`
  - `st49_c02_heartbeat_has_no_version` — `// Scenario: PC の生存信号には版が書き込まれない`
  - `st49_c02_history_reread_gets_current_stamp` — `// Scenario: ブラウザ履歴を読み直して送る記録は、読み直した時点の版を持つ`
  `tools/check-build-stamp.sh` に `windows` の節（`cargo build --release -p ashiato-collector-windows`。印: `echo "== Scenario: PC の収集アプリの組み立ての印は、commit を進めた後の差分の組み立てでも新しい commit になる"`・`echo "== Scenario: PC の収集アプリの印は、add していないソースの変更の後に .dirty になり、release では .debug を持たない"`）を足す。
  検証: `CW st49_c02_` rc=0（`-- --list` で 5 本以上）/ `scripts/quiet-run cw -- cargo test -p ashiato-collector-windows --lib` rc=0（既存の試験が通る）/
  `scripts/quiet-run clippy -- cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `STAMP windows` rc=0

## Task 9: 携帯の収集アプリ（C-01）の印・焼き込み・欄の保持・破棄の下書き

- [x] 9.1 `app/build.gradle.kts` に印（D6。`providers.exec` で `git rev-parse HEAD` と `git status --porcelain -- collector-android`、`androidComponents.onVariants` で `BuildConfig.COMPONENT_VERSION`。release だけ `.debug` を付けない）。
  `tools/check-build-stamp.sh` に `android` の節を足す: 写した木で `--configuration-cache --build-cache` を付けて `:app:generateDebugBuildConfig` と `:app:generateReleaseBuildConfig` を走らせ、生成された `BuildConfig.java` の `COMPONENT_VERSION` を読む。印:
  - `echo "== Scenario: 携帯の収集アプリの組み立ての印は、組み立てのたびに作り直される"`（commit を足して debug を組み直す）
  - `echo "== Scenario: 携帯の収集アプリの release の印は .debug を持たず、debug の印は持つ"`
  - `echo "== Scenario: 携帯の収集アプリの印は、add していないソースの変更の後に .dirty になる"`
  検証: `scripts/quiet-run stamp-android -- ./tools/check-build-stamp.sh android` rc=0
- [x] 9.2 `IngestRequest.kt`・`DropReport.kt`（`DropReport`）・`OriginalLedger.kt`（`MediaMissingReport`）に `@SerialName("client_version") val clientVersion: String? = null`。`Outbox` の組み立てに「印を書くか」を渡し、
  `Outbox.add` が欄の無い行に印を書く（記録・原本の欠けの溜めだけ。生存信号は書かない。D13）。`SegmentStore` の読み戻しは行の JSON を保ち、送り出しは保った JSON をそのまま送る（D3（仮））。`migrateLegacyOutbox` は印を書かない。
  試験（`St49OutboxTest.kt`。Robolectric）:
  - `pendingKeepsStampFromAddTime` — `// Scenario: 携帯の収集アプリを更新した後に送った未送信は、積んだときの版で届く`
  - `oldLineIsSentWithoutVersion` — `// Scenario: 版の欄を持たない未送信の行は、版の欄の無いまま送られる`（材料: 版の欄の無い今の形の 1 行を逐語で置く。`unreadable.jsonl` が空のまま）
  - `unknownFieldSurvivesResend` — `// Scenario: 携帯の未送信の行が知らない欄を持っていても、送るときに落ちない`
  - `legacyMigrationDoesNotStamp` — `// Scenario: 古い形の未送信を移すとき、版は書き込まれない`
  - `heartbeatHasNoVersion` — `// Scenario: 携帯の生存信号には版が書き込まれない`
  検証: `GT St49OutboxTest` rc=0（`<testcase` が 5 以上）
- [x] 9.3 `DropDraft` に `clientVersion`（D15）。下書きを始めるときに今の印を入れ、足そうとした下書きの版が今の印と違えば閉じて次の下書きにする。保存済みの下書きに版が無ければ閉じ、閉じた報告は版の欄の無いまま送る。
  試験（`St49DropDraftTest.kt`）:
  - `versionChangeClosesDraft` — `// Scenario: 収集アプリの版が変わったら、破棄の報告の下書きは閉じて次の報告になる`
  - `draftWithoutVersionIsClosedAndSentWithoutVersion` — `// Scenario: 版を持たない保存済みの下書きは閉じられ、その報告は版の欄の無いまま送られる`
  検証: `GT St49DropDraftTest` rc=0（`<testcase` が 2 以上）/ `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest` rc=0（既存の単体が全部通る）

## Task 10: 画面（V-01）の印と、送る 1 件の `client_version`

- [x] 10.1 `web/build-stamp.ts`（D6。git を呼ぶ関数を引数で差し替えられる形。部品のソースの道は `web/`）を作り、`vite.config.ts` と `vitest.config.ts` の両方で `define: { __ASHIATO_WEB_VERSION__ }` に入れる。`web/src/buildStamp.ts` が読み（型の宣言を `web/src/vite-env.d.ts` に）、
  `feelings.ts` の `ingestFeelingItem`・`places.ts` の場所の 1 件・`attributes.ts` の主張の 1 件・`MasterView.tsx` の取り込みの 1 件が `client_version` に入れる（D20）。
  vitest（`web/src/__tests__/buildStamp.test.ts`。印なし —— 画面の Scenario は 10.2 の e2e が担保する）: `build-stamp.ts` に偽の git を渡して `.dirty`・`unknown`・production で `.debug` が付かないこと、3 つの組み立ての関数の戻り値の `client_version` が `__ASHIATO_WEB_VERSION__` と同じこと。
  検証: `VT buildStamp.test.ts` rc=0 / `bash -c 'cd web && npx vitest run'` rc=0（既存の画面の試験が通る）/ `bash -c 'cd web && npm run build'` rc=0
- [x] 10.2 `web/e2e/st49-client-version.spec.ts`（`tools/stack.sh up` の本物の画面。`page.waitForRequest` で `/api/ingest` の本文を読む。送り先は本物の S-01 のまま）:
  - `// Scenario: 画面が送る気分・場所・個人属性の主張の 1 件は、画面の組み立ての印を持つ` —— 気分（`feel.spec.ts` の記入の流れ）・場所（`places.spec.ts` の登録の流れ）・個人属性の主張（`MasterView` の主張を書く流れ。いまの `web/e2e` に無いので、この spec の中に書く）を 1 件ずつ送り、
    3 つの本文の各要素の `client_version` が D5 の正規表現に合い、版番号が `web/package.json` の `version`、3 つが同じ値であることを見る
  検証: `bash -c 'cd web && npm run test:e2e -- st49-client-version'` rc=0

## Task 11: 申し送りと全体の確かめ

- [x] 11.1 `docs/handoff/ST23.md`（`st49-record-component-version` review/spec.md R5 —— ST23 が足す削除の台帳の INSERT（本文ごと消す行）は `server_version` を書く。`core.erasure_operation` に版を持たせるかは ST49 の Q3 の答えに従う。D10 の `st49_ledger_inserts_name_server_version` が落として知らせる）、
  `docs/handoff/ST50.md`（ST49 より前の S-01 に新しい収集アプリが送った間・古い版へ戻した間の版の脱落。期限の後は FR-125 / FR-130 で止める。design D3（仮）・Risks）、
  `docs/handoff/ST29.md`（衛星の資格情報の `Caller` は `Writer::Collector` / `Writer::Web` にしない。D9）、`docs/handoff/ST52.md`（版番号の出所を 1 つにしたら `crates/build-stamp`・`build.gradle.kts`・`web/build-stamp.ts` の版番号をそこから読む。commit は ST48 と同じ値）を
  `docs/handoff/README.md` の書式で書く（既にあるファイルには項を足す）。
  検証: `bash -c 'grep -q "st49-record-component-version" docs/handoff/ST23.md && grep -q R5 docs/handoff/ST23.md && grep -q st49 docs/handoff/ST50.md && grep -q st49 docs/handoff/ST29.md && grep -q st49 docs/handoff/ST52.md'` rc=0 / `./tools/check-shared-appends.sh` rc=0
- [x] 11.2 全体: `scripts/quiet-run fmt -- cargo fmt --all -- --check` rc=0 / `scripts/quiet-run clippy -- cargo clippy --workspace --all-targets -- -D warnings` rc=0 /
  `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run test -- cargo test -p ashiato-server'` rc=0 / `CS st49_` rc=0（`-- --list` で 7 本以上）/
  `./tools/check-migrations.sh && ./tools/check-boundaries.sh && ./tools/check-openapi.sh && ./tools/check-immutable.sh` rc=0 /
  `scripts/quiet-run smoke -- ./tools/smoke.sh` rc=0（今の収集アプリの形の要求がそのまま通る）/
  `python3 scripts/check_scenarios.py . st49-record-component-version` rc=0
