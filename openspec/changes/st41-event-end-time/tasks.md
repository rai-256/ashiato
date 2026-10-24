# ST41 実装タスク — 幅のある出来事に終わりの時刻を持たせる

読む順: `deep.md`（**本人の答え Q1〜Q7**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/record-envelope/spec.md` → `specs/data-durability/spec.md` →
`specs/collection-coverage/spec.md` → `design.md` → `premises.md` → `docs/handoff/ST41.md` → `CLAUDE.md`。

**収集アプリ（`collector-android` / `crates/collector-windows`）は触らない**（design D3）。**滞在（`stay_store.rs`）の書き方と、収集済みの行の内容は変えない**（D13（仮）・D14（仮））。
**`excluded.rs` の `SPAN_RULES` / `NO_COORD_SOURCES` / `COORD_SOURCES` と、CH02 の窓の枠には触らない**（proposal の Impact。走っている ST09 / CH02 の持ち場）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: 終わりの時刻つきの記録は、終わりのずれとタイムゾーンごと残る`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 73 本**（`record-envelope` 63・`data-durability` 7・`collection-coverage` 3）。うち **ST41 で新しく足したのは 56 本**。
  **新しい 56 本は全部、このファイルのどこかの Task が名指しする。** 残る 17 本は MODIFIED で写した既存分で、印は既存の試験（`tools/smoke.sh`・`dedup_tests.rs`・`tools/check-immutable.sh`）のまま
- **件数つき検証**: このファイルで **`CT <名前>`** は
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
  `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` が、その項目に書いた本数（`-- --list で N 本以上`）以上
  を指す（`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので `-- --list` の下限を必ず見る）
- **試験の名前の接頭辞は `st41_`**: `st41_gate_`（Task 1）/ `st41_ingest_`（Task 2）/ `st41_update_`・`st41_dedup_`（Task 3）/ `st41_coverage_`（Task 4）/ `st41_range_`（Task 5）/ `st41_export_`（Task 6）。
  新しい DB の試験は `crates/server/src/end_time_tests.rs`（Task 1〜5）と `crates/server/src/export_end_time_tests.rs`（Task 6）に置き、`lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;` を足す。
  ソースは `testdb::source_of_kind`（外部の識別子の粒度つき）で試験ごとに登録する
- **本人の決定（下流は変えない）**: Q1 逆順の終わりは送られたまま入れ、期間では始まりの点 / Q2 期間も記録も「始まりを含み、終わりを含まない」・点も入れる /
  Q3 `GET /events` に `from` / `to` / `source` / Q4 日は Asia/Tokyo・件数は始まりの日 / Q5 滞在は列に書かない / Q6 収集済みは写さない /
  **Q7 鍵は変えない。外部の識別子のある記録だけ終わりの違いを更新にする**。仮: D17（data-durability は ADDED のまま）/ D18（内容が同じで終わりの無い到着は終わりを消さない）。（第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）
- **内容の鍵（`content_hash_of`）の入力を変えない**（D7）。`hash_is_pinned` の固定値 `39d0ebc5…15df1` を変える変更は落とす
- **移行は 1 つ・名前は作成時刻**（`migrations/<YYYYMMDDHHMM>_event_end_time.sql` と `.down.sql`、`migrations/ORDER` の末尾へ 1 行）。連番にしない。
  2 回当てても落ちない形（`IF NOT EXISTS` / `CREATE OR REPLACE`）。門と錠の関数はいまの本体を写して足すだけにし、ほかの条件を変えない（D5・D15）
- **新しい応答の欄・列の名前は `end_time` / `end_tz_offset_min` / `end_tz_id` の 3 つで統一する**（DB の列・取り込みの欄・`GET /events` の欄・書き出しの列。D1）
- **`GET /events` の終わりを持たない行の鍵は 6 つのまま**（`skip_serializing_if`。D9）。`st25_api_unchanged_events_and_stays_shapes` のアサートを変えない（呼び方だけ直す）
- 記録の値（原文・時刻・TZ）をログに出さない（種別とソース名だけ。既存の `tracing::warn!` の形）

## Task 1: 移行（3 列・CHECK・ビュー・門と錠・部分索引）と錠の試験

- [x] 1.1 `migrations/<作成時刻>_event_end_time.sql` を design D15 の順で書く: `core.event` / `core.event_version` に 3 列（D1）と CHECK 2 つ・
  `core.event_live` の作り直し・`core.event_version_live` の末尾に 3 列・`core.stay_erased` の `end_at` を列→内容の `end` の順（D11）・
  `core.require_version_or_ledger` と `core.reject_version_rewrite` に終わりを足す（D5）・部分索引 `event_end_range`（D10）。`.down.sql` は逆順でいまの形に戻す。
  `migrations/ORDER` の末尾へ 1 行（前の行に `# ST41: 幅のある出来事の終わりの時刻（3 列・門・期間の索引）` の注記）。
  検証: `./tools/check-migrations.sh` rc=0 / `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run build -- cargo build -p ashiato-server'` rc=0（`build.rs` が ORDER から `MIGRATIONS` を組む）
- [x] 1.2 `crates/server/src/end_time_tests.rs` を作り、記録を終わりつきで置く道具（`put_with_end(pool, user, source, at, end: Option<(&str, i32, &str)>, external_id: Option<&str>)`）と、
  `core.event` / `core.event_version` を直に書く試験を置く（門と錠は DB の側にあるので、どの接続でも同じに拒む）:
  - `st41_gate_end_only_rewrite_is_rejected` — `// Scenario: 収集した記録の終わりの時刻だけを履歴なしで変えると拒まれる`（終わりを別の時刻にする・NULL にするの 2 通り。NULL にするときは 3 列とも NULL にして CHECK でなく門で落ちることを SQLSTATE と本文で見る）
  - `st41_gate_version_with_only_end_different_is_rejected` — `// Scenario: 終わりだけが違う履歴を書いても、終わりの書き換えは通らない`
  - `st41_gate_erasure_cannot_move_end` — `// Scenario: 本文の消去で終わりの時刻を変えると拒まれる`
  - `st41_gate_end_tz_can_be_corrected` — `// Scenario: 収集した記録の終わりのずれとタイムゾーンは直せる`
  - `st41_gate_version_end_is_frozen` — `// Scenario: 履歴の前の版の終わりは書き換えられない`（3 列それぞれ）
  - `st41_gate_check_rejects_partial_end_in_db` — `// Scenario: 取り込み口を通さない書き込みでも、終わりの一部だけ・範囲外のずれ・空の識別子は入らない`（記録の表と履歴の表の両方。D1 の CHECK。移行を 2 回当てても落ちないことも同じ試験で見る）
  検証: `CT st41_gate_` rc=0（`-- --list` で 6 本以上）
- [x] 1.3 `tools/check-immutable.d/<作成時刻>_st41-event-end-time.sh` に、1.2 の拒まれる 3 つ（終わりだけを履歴なしで変える・終わりだけ違う履歴・前の版の終わり）を
  `tools/check-immutable.sh` の既存の節と同じ書き方で置き、それぞれ `echo "== Scenario: <名前>"` の印を付ける（本体の末尾に書き足さない）。
  検証: `./tools/check-immutable.sh` rc=0 / `./tools/check-shared-appends.sh` rc=0

## Task 2: 取り込み口の 3 欄と検査

- [x] 2.1 先に試験を書く（`end_time_tests.rs`。取り込み口 `crate::ingest` を通す）:
  - `st41_ingest_end_is_stored_with_offset_and_tz` — `// Scenario: 終わりの時刻つきの記録は、終わりのずれとタイムゾーンごと残る`
  - `st41_ingest_start_and_end_keep_their_own_tz` — `// Scenario: 始まりと終わりのタイムゾーンが違う記録も、それぞれのまま残る`
  - `st41_ingest_partial_end_is_rejected_per_item` — `// Scenario: 終わりの 3 つの一部だけを持つ記録は受け付けない`
  - `st41_ingest_end_offset_out_of_range_is_rejected` — `// Scenario: 終わりのずれが範囲の外の記録は受け付けない`
  - `st41_ingest_end_offset_at_edges_is_accepted` — `// Scenario: 終わりのずれが範囲の端の記録は受け付ける`
  - `st41_ingest_future_and_long_spans_are_accepted` — `// Scenario: 終わりが未来の記録も、幅が何年もある記録も受け付ける`
  - `st41_ingest_end_is_truncated_to_micros` — `// Scenario: マイクロ秒より細かい桁は切り捨てて格納される`
  - `st41_ingest_empty_or_nul_end_tz_is_rejected` — `// Scenario: 終わりのタイムゾーンが空か格納できない文字を含む記録は受け付けない`
  - `st41_ingest_unknown_tz_name_is_accepted` — `// Scenario: 一覧に無いタイムゾーンの名前でも終わりは受け付ける`
  - `st41_ingest_end_before_start_is_kept` — `// Scenario: 終わりが始まりより前の記録も、送られたまま残る`
  - `st41_ingest_end_equal_to_start_is_kept` — `// Scenario: 終わりと始まりが同じ時刻の記録も受け付ける`
  - `st41_ingest_without_end_is_unchanged` — `// Scenario: 終わりを送らない記録は、終わりを持たない記録として今のまま残る`（いまの収集アプリと同じ JSON を送り、結果の JSON と状態符号が欄を足す前と同じ形であることを、結果の鍵の集合で見る）
  いまの木で落ちることを確かめてから 2.2 に進む（いまの木は終わりの欄を読まずに捨てるので、`st41_ingest_end_is_stored_with_offset_and_tz`・`st41_ingest_partial_end_is_rejected_per_item`・
  `st41_ingest_end_is_truncated_to_micros` の 3 本が落ちる。`CT st41_ingest_end_is_stored` が rc≠0 であることを確かめる）
- [x] 2.2 `ingest.rs` の `IngestRequest` に 3 欄（`#[serde(default)]`）、`Invalid::EndTime` と `validate` の検査（D2）、`lib.rs` の `IngestError::InvalidEndTime` と対応、
  `ingest_one` の 2 つの INSERT に 3 列を足す。`ingest.rs` の単体試験に `validate` の境界（±1,439 / ±1,440・空・U+0000・一部だけ）を `st41_ingest_validate_` で置く。
  受けた直後に `end_time` を µs に切り捨てる（D7）。
  検証: `CT st41_ingest_` rc=0（`-- --list` で 13 本以上）/ `CT dedup_` rc=0（`-- --list` で 30 本以上。既存の取り込みが変わらない）
- [x] 2.3 `cargo run -q -p ashiato-server --bin openapi > docs/openapi.json` で契約を再生成し、`docs/collector-contract.md` に任意の 3 欄・`invalid_end_time`・「収集アプリは送らない」を足す。
  検証: `./tools/check-openapi.sh` rc=0 / `jq -e '.components.schemas.IngestRequest.properties | has("end_time") and has("end_tz_offset_min") and has("end_tz_id")' docs/openapi.json` rc=0 /
  `git diff --quiet -- collector-android crates/collector-windows` rc=0

## Task 3: 同じ記録の鍵と更新の経路（Q7）

- [x] 3.1 `ingest.rs` の単体試験に `st41_hash_with_end_is_pinned` を置く —— `// Scenario: 終わりつきの記録の内容ハッシュは、終わりを持たない同じ記録と同じ値になる`。
  `req(Uuid::nil(), "x")` と、それに終わり（2025-09-04T16:33:20Z・540・`Asia/Tokyo`）を足したものの `content_hash` がどちらも `39d0ebc5c3c1d17a5deec93e03e60c3be02e5ddd741a2d5575b789c16ee15df1`。
  検証: `bash -c 'scripts/quiet-run unit -- cargo test -p ashiato-server --lib st41_hash'` rc=0（`-- --list` で 1 本以上）
- [x] 3.2 先に試験を書く（`end_time_tests.rs`）:
  - `st41_update_end_arrives_later_by_external_id` — `// Scenario: 進行中を終わりなしで送り、同じ外部の識別子で終わりつきを送ると終わりが入る`
  - `st41_update_end_only_change_keeps_previous_end` — `// Scenario: 終わりだけを直した版は、前の終わりを履歴に残す`
  - `st41_update_version_without_end_clears_end` — `// Scenario: 終わりを持たない版が届くと、記録の終わりは空になり前の終わりは履歴に残る`
  - `st41_update_resend_without_end_keeps_end` — `// Scenario: 終わりを持たない同じ版の再送で、入った終わりは消えない`（D18（仮））
  - `st41_update_end_tz_only_change_is_not_an_update` — `// Scenario: 終わりのずれとタイムゾーンだけが違う版は更新にならない`
  - `st41_update_sub_micro_end_change_is_not_an_update` — `// Scenario: マイクロ秒より細かい桁だけが違う終わりの版は更新にならない`
  - `st41_update_stale_end_change_is_ignored` — `// Scenario: 古い更新時刻の版は、終わりだけが違っても書き換えない`
  - `st41_update_same_end_adds_no_version` — `// Scenario: 終わりも内容も同じ版は履歴を増やさない`
  - `st41_dedup_end_only_difference_folds_without_external_id` — `// Scenario: 外部の識別子の無い記録で終わりだけが違う到着は畳まれる`
  - `st41_dedup_deleted_stays_deleted_on_end_only_difference` — `// Scenario: 削除済みの記録は、終わりだけが違う到着で戻らない`
  - `st41_dedup_deleted_external_stays_deleted_on_end_only_difference` — `// Scenario: 外部の識別子のある削除済みの記録も、終わりだけが違う版で戻らない`
  「収集した」由来（`origin = collected`）で置き、更新が門（Task 1）を通ることも同じ試験で見る。いまの木で `st41_update_end_arrives_later_by_external_id`・
  `st41_update_end_only_change_keeps_previous_end`・`st41_update_version_without_end_clears_end` の 3 本が落ちることを `CT st41_update_end_` rc≠0 で確かめてから 3.3 に進む
- [x] 3.3 `lib.rs` の `Stored` に 3 列、`load_stored` の SELECT に 3 列、更新の判定を design D7 の式に、`apply_external_update` の前の版の INSERT と本表の UPDATE に 3 列を足す。
  `advance_watermark` の経路（内容も終わりも同じ）は変えない。
  検証: `CT st41_update_` rc=0（`-- --list` で 8 本以上）/ `CT st41_dedup_` rc=0（`-- --list` で 3 本以上）/ `CT dedup_` rc=0（`-- --list` で 30 本以上）

## Task 4: 稼働記録の日の数え方（Q4）

- [x] 4.1 `end_time_tests.rs` に置く（コードは変えない想定。落ちたら `ingest_one` の稼働記録の upsert が `event_time` の日で数えているかを直す）:
  - `st41_coverage_spanning_event_counts_on_start_day` — `// Scenario: 日をまたぐ幅のある出来事は始まりの日に 1 件だけ数えられる`
  - `st41_coverage_foreign_tz_counts_on_tokyo_day` — `// Scenario: 始まりが海外の地域の記録も、Asia/Tokyo の日で数えられる`
  - `st41_coverage_started_on_uses_start` — `// Scenario: 収集を始めた日は始まりの日で決まる`
  件数は `core.coverage` の `(user_id, logical_source, day)` の行を直に読む。
  検証: `CT st41_coverage_` rc=0（`-- --list` で 3 本以上）

## Task 5: 期間に重なる記録を引く（`GET /events`）

- [x] 5.1 先に試験を書く（`end_time_tests.rs`。ハンドラを `crate::events(State, headers, Query(EventsQuery{…}))` で呼ぶ。401 と読み出しの記録は router 越しに `tower::ServiceExt::oneshot` で見る —— `access_log_tests.rs` の呼び方に合わせる）:
  - `st41_range_overlapping_span_is_returned` — `// Scenario: 期間に重なる幅のある出来事が引ける`
  - `st41_range_results_are_in_event_time_order` — `// Scenario: 期間で引いた記録は出来事の時刻の順に並ぶ`
  - `st41_range_source_only_returns_all_of_source` — `// Scenario: 期間を指定せずソースだけを指定すると、そのソースのすべての記録が返る`
  - `st41_range_spans_crossing_either_edge_are_returned` — `// Scenario: 期間より前に始まり期間の中で終わる記録も、期間の中で始まり期間の後に終わる記録も返る`
  - `st41_range_touching_edges_do_not_overlap` — `// Scenario: 終わりちょうど・始まりちょうどの期間には重ならない`
  - `st41_range_point_is_in_one_of_adjacent_periods` — `// Scenario: 隣り合う 2 つの期間で同じ点の記録は片方にだけ返る`
  - `st41_range_overnight_span_is_on_both_days` — `// Scenario: 日をまたぐ出来事は、重なった日の期間のどちらにも返る`
  - `st41_range_reversed_end_is_a_point` — `// Scenario: 終わりが始まりより前の記録は、始まりの 1 点として扱われる`
  - `st41_range_zero_length_is_a_point` — `// Scenario: 終わりと始まりが同じ記録は点として扱われる`
  - `st41_range_source_filter` — `// Scenario: ソースを指定すると、そのソースの記録だけが返る`
  - `st41_range_deleted_is_not_returned` — `// Scenario: 期間で引いても削除済みの記録は返らない`
  - `st41_range_rows_without_end_keep_six_keys` — `// Scenario: 終わりを持たない記録の行は、いまの形のまま返る`
  - `st41_range_without_period_returns_all` — `// Scenario: 期間を指定しなければ、いまのまますべての記録が返る`
  - `st41_range_bad_period_is_400` — `// Scenario: 期間の片方だけ・読めない時刻・逆さの期間は断られる`（5 通り）
  - `st41_range_without_credentials_is_401` — `// Scenario: 資格情報の無い期間の求めは断られる`
  - `st41_range_is_access_logged` — `// Scenario: 期間で引いたことは読み出しの記録に残る`
  いまの木では `EventsQuery` が無くコンパイルが通らないので、先に `EventsQuery` と `events` の引数だけを足し（中身は今のまま）、`CT st41_range_overlapping` が rc≠0 であることを確かめてから 5.2 に進む。
  共有の DB では他の試験の記録が混ざるので、期間の試験はソースか利用者で絞ったうえで、置いた id の集合だけを比べる（`source` を指定しない試験は試験ごとの利用者の記録だけを数える）
- [x] 5.2 `lib.rs` に `EventsQuery { from, to, source }`（`Option<String>` で受けて自分で RFC 3339 を読む —— ずれの無い時刻を 400 にするため）、`EventRow` に 3 欄（`skip_serializing_if`）、
  `events` の SQL を design D10 の 2 つの引き方の `UNION ALL`（期間なしは今の SELECT に `source` の条件だけ）にする。`core.event_live` を引く（素の表を引かない）。
  `events` を呼ぶ既存の試験（`grep -rn "crate::events(" crates/server/src`）に `Query(EventsQuery::default())` を渡す。
  検証: `CT st41_range_` rc=0（`-- --list` で 16 本以上）/ `CT st25_api_unchanged` rc=0（`-- --list` で 1 本以上）/
  アサートを変えていないこと: `git diff -U0 origin/main -- crates/server/src/day_records_tests.rs | grep -E '^[-+][^-+]' | grep -vE 'crate::events\(|EventsQuery|^[-+]\s*$'` の出力が空（rc=1）
- [x] 5.3 `st41_range_uses_partial_index` — まっさらな DB（`crate::stay_tests::fresh_db()` に `crate::MIGRATIONS` を全部当てたもの）に 1 万行（うち 1 千行が終わりつき）を置き `ANALYZE` した後、
  5.2 の終わりのある側の SQL の `EXPLAIN (FORMAT JSON)` に `event_end_range` の名前があることを見る（印なし。D10 の確かめ）。
  検証: `CT st41_range_uses_partial_index` rc=0（`-- --list` で 1 本以上）/ `./tools/check-openapi.sh` rc=0（`/events` の問い合わせの欄と応答の欄が契約に出る。`cargo run -q -p ashiato-server --bin openapi > docs/openapi.json` で再生成してから）

## Task 6: 書き出しと消した滞在の時間を列から読む（申し送り C7）

- [x] 6.1 先に試験を書く（`crates/server/src/export_end_time_tests.rs`。配置と書き出しの読み方は `export_excluded_tests.rs` の `World` と `kept` / `kept_versions` に合わせる）:
  - `st41_export_end_column_span_is_excluded` — `// Scenario: 終わりを列に持つ緯度経度つきの記録は、消した時間と重なれば出ない`
  - `st41_export_end_column_wins_over_payload` — `// Scenario: 列の終わりは内容の終わりより先に読まれる`
  - `st41_export_reversed_end_column_falls_back_to_payload` — `// Scenario: 列の終わりが記録の時刻より前なら、内容の終わりから読む`
  - `st41_export_end_column_equal_to_start_falls_back_to_payload` — `// Scenario: 列の終わりが記録の時刻と同じなら、内容の終わりから読む`
  - `st41_export_version_end_column_is_read` — `// Scenario: 前の版の終わりの列も幅として読まれる`
  - `st41_export_erased_stay_end_column_is_read` — `// Scenario: 終わりを列に持つ消した滞在の時間も、消した時間に数える`（滞在の行を試験の中で直に作り、`end_time` 列にだけ終わりを置いて消す）
  - `st41_export_rows_have_end_columns` — `// Scenario: 書き出しの行は終わりの 3 つの列を持つ`（zip を開いて記録と前の版のファイルの行の鍵と値を見る。`export_file_tests.rs` の読み方に合わせる）
  いまの木（列が無い移行の前）ではなく Task 1 の後の木で、`st41_export_end_column_span_is_excluded`・`st41_export_end_column_wins_over_payload`・`st41_export_version_end_column_is_read`・
  `st41_export_rows_have_end_columns` が落ちることを `CT st41_export_end_column_span` rc≠0 で確かめてから 6.2 に進む
- [x] 6.2 `export/excluded.rs` の幅の (a) の先頭に列の終わり（D11 の `CASE`）を足し、前の版の行を作る副問い合わせに `v.end_time` を足す。消した滞在の前の版の時間を `coalesce(v.end_time, …)` にする
  （`core.stay_erased` は Task 1 の移行で済んでいる）。`export.rs` の `EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS` に 3 列を `tz_id` の次に足し、`export/doc.rs` に説明を足す。
  **main に CH02 の窓の形が入っていれば、その形に足す**（窓の枠と窓に載る列は変えない。終わりは窓の後に `id` で戻した行から読む）。
  検証: `CT st41_export_` rc=0（`-- --list` で 7 本以上）/ `CT export_` rc=0（`-- --list` で 130 本以上。既存の書き出しの試験が全部通る。「記録の行は記録の表のすべての列を持つ」が 3 列を足し忘れると落ちる）

## Task 7: 申し送りと全体の確かめ

- [ ] 7.1 `git log --oneline origin/main -- crates/server/src/export/excluded.rs | head -3` で CH02 の窓の形が main に入っているかを見る。**入っていなければ**、`docs/handoff/CH02.md` を作り
  「`st41-event-end-time` design D11 — `legacy` の写しと突き合わせの偽データに、終わりを列に持つ記録の行が無い。CH02 の新旧の突き合わせは ST41 の (a) の列を写していない」を
  `docs/handoff/README.md` の書式で書く（入っていれば書かない。どちらだったかを PR 本文に 1 行）。
  検証: `git log --oneline origin/main --grep 'ch02' -- crates/server/src/export/excluded.rs | grep -q . || test -f docs/handoff/CH02.md` rc=0 /
  `python3 scripts/review_triage.py . st41-event-end-time` rc=0
- [ ] 7.1b 滞在と収集済みの記録を変えていないこと（Q5 / Q6 の推奨。D13（仮）・D14（仮））:
  `git diff --quiet origin/main -- crates/server/src/stay_store.rs collector-android crates/collector-windows` rc=0 /
  `end_time_tests.rs` の `st41_stays_and_collected_rows_have_no_end_column` —— 位置の記録を置いて `stay_store::rebuild_all` を走らせた後、その利用者の滞在と位置の記録の `end_time` がすべて NULL であることを見る。
  検証: `CT st41_stays_and_collected` rc=0（`-- --list` で 1 本以上）
- [ ] 7.2 全体: `scripts/quiet-run fmt -- cargo fmt --all -- --check` rc=0 / `scripts/quiet-run clippy -- cargo clippy --workspace --all-targets -- -D warnings` rc=0 /
  `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run test -- cargo test -p ashiato-server'` rc=0 /
  `./tools/check-migrations.sh && ./tools/check-boundaries.sh && ./tools/check-openapi.sh && ./tools/check-immutable.sh` rc=0 /
  `scripts/quiet-run smoke -- ./tools/smoke.sh` rc=0（「2 つの時刻が両方埋まる」「時刻の欄が欠けた要求は拒否される」が今のまま通る）/
  `python3 scripts/check_scenarios.py . st41-event-end-time` rc=0
