# ST09 コードレビュー

## final review（77bbdba..4b18a1a）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package は
`.superpowers/sdd/tasks/review-77bbdbab..4b18a1ab.diff`（68 commits / 89 files）。判定は「With fixes」（Critical 0・Important 3・Minor 9）。
Task ごとの ⚠️ の確認の結果（問題なしとした分）は末尾の「Task の ⚠️ の確かめ」にまとめた。

## R1. 記録の受理と原本の送り待ちの行の登録が競合し、行が `awaiting_record` のまま固まる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MediaScanner.kt
- 根拠: MediaScanner.kt:245-248（`outbox.add` の後に `store.originals.queue`）/ OriginalLedger.kt:239-242（`OriginalRecordLink.accepted` は行が無ければ何もしない）。flusher が間に受理すると、後から積まれた行は ready にならず、Retention・拒否の経路にも乗らない
- kind: technical
- 処置: fixed D8 （381c71bb。行を先に積んでから記録を積む。GT MediaScannerTest.a_record_accepted_right_after_it_is_queued_still_makes_its_original_ready）

## R2. HEAD で計測テストが走っておらず、AT の証跡が evidence.jsonl に無い
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaCaptureInstrumentedTest.kt
- 根拠: 最後の全体の走行は `.superpowers/sdd/logs/20261020-101835-at.log`（6428315b）。その後 22bd6503（detekt の修正）で MediaCaptureInstrumentedTest.kt / PermissionDeniedInstrumentedTest.kt が書き換わった
- kind: technical
- 処置: fixed 10.2 （HEAD 71f203b0 で `scripts/quiet-run instr -- tools/android-emulator.sh` rc=0・934 秒（`.superpowers/sdd/logs/20261020-184204-instr.log`、`set -euo pipefail` で 3 段とも gradle が通った）。`<testcase classname=…>` で数えて: 1 段目 17 本・失敗 0（うち MediaCaptureInstrumentedTest 2 本）、3 段目 MediaPremisesInstrumentedTest 3 本・失敗 0。2 段目（PermissionDeniedInstrumentedTest）の XML は 3 段目が同じ名前で上書きするので本数は数えられず、rc=0 だけが根拠（R14 / Task 10 F6）。evidence.jsonl への記録はハーネスの証跡の段に任せる）

## R3. 権限を許可しても、最大 6 時間は読み手も知らせも戻らない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MediaCapture.kt
- 根拠: 知らせの見直しは `start()` と `onOutcome` だけ。実行時の許可ではプロセスが落ちないので、次の 6 時間の見直しか新しい写真の知らせまで戻らない。MediaCapabilityTest.kt:142-143 は `rig.review()` を手で呼ぶので遅れを見ていない
- kind: technical
- 処置: fixed D6 （381c71bb。読み手のスレッドで MEDIA_PERMISSION_POLL_MS = 60 秒ごとに権限を見回る。GT MediaCapabilityTest.a_permission_granted_at_runtime_brings_the_reader_and_the_notice_back_within_a_minute）

## R4. 本物の HTTP では 413 が `too_large` にならない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalAndroid.kt
- 根拠: OriginalAndroid.kt:57-63 / OriginalUploader.kt:365。S-01 は Content-Length を見て本文を読む前に 413 で閉じ、固定長の流し送りは書き込み中の IOException → `Unreachable` に畳まれる。1 GiB を超える画像を 1 時間ごとに読み直し続ける
- kind: technical
- 処置: fixed D9 （381c71bb。端末の ORIGINAL_BODY_LIMIT = 1 GiB を超えたら開かず・PUT せずに too_large。GT 2 本）

## R5. 走査の `Pass` の組み立てが try の外にあり、そこでの失敗が `onOutcome` に届かない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MediaScanner.kt
- 根拠: MediaScanner.kt:103。`store.loadCursor()` / `listing.volumes()` / `listing.version()` が投げると失敗の試行にも段にも数えられない（C19）
- kind: technical
- 処置: fixed D22 （381c71bb。openPass を try の中へ、失敗は 4 ソースとも stage_read。GT MediaCapabilityTest.a_listing_that_fails_before_the_pass_starts_counts_a_failure_with_the_read_stage）

## R6. 台帳を 1 行の変更ごと・200 件の頁ごとに全体で書き直す
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalLedger.kt
- 根拠: `OriginalLedger.write` と `MediaScanner.saveLedger`。初回の 1 万件で書き込みが件数の 2 乗で増え、R1 の窓も広げる（Task 3 F5 の延長）
- kind: technical
- 処置: fixed D8 （381c71bb。OriginalLedger.queue を 1 行の追記に。MediaScanner.saveLedger の頁ごとの書き直しは途中で落ちてもハッシュを計算し直さないために残す —— 書き込みは 1/200 に薄まる。GT OriginalLedgerTest.queueing_appends_one_line_and_a_torn_last_line_is_skipped_on_reload）

## R7. design D15 の 4 が「`SELECT` / `INSERT` だけ」のまま、実装の裁定（門のトリガが担う）と食い違う
- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠: `.superpowers/sdd/st09-task-1/progress.md` の Ruling と migrations/202610181200_media_capture.sql のコメント
- kind: technical
- 処置: fixed D15 （71f203b0。D15 の 4 を Task 1 の Ruling どおりに書き直した）

## R8. design Risks の「名前の整え方の取りこぼし → 書かずに `422`」と実装（`503 io`）が違う
- 成果物: crates/server/src/media.rs
- 根拠: media.rs:593,597,608 は `outside_root` / `bad_name` / `name_taken` を `503 io` にし `last_storage_error` に `io` を残す（Task 6 F5）
- kind: technical
- 処置: fixed D10 （71f203b0。名前を決められないときは 422 name_refused、直近の失敗に残さない。design Risks・docs/collector-contract.md を揃えた。CT st09_up_put_name_taken_everywhere_is_refused_without_a_storage_error）

## R9. 原本の受け口で 2 MB を超える本文が通ることを直接確かめる試験が無い
- 成果物: crates/server/src/media_tests.rs
- 根拠: 1024 の上限の上書きの試験だけ（Task 6 ⚠️ / F4）。経路の `DefaultBodyLimit` が既定の 2 MB を上書きしていることは構造の上でだけ確かめた
- kind: technical
- 処置: fixed D10 （71f203b0。CT st09_up_put_three_mb_body_passes_the_default_two_mb_limit）

## R10. 全版を当てた後に収集開始日がメディアのソースで戻ることを固定する試験が無い
- 成果物: crates/server/src/media_migration_tests.rs
- 根拠: `st09_mig_polluted_photo_started_on_is_repaired` は `media_sql()` だけを当てる。ORDER の並びが変わったときに守れない
- kind: technical
- 処置: fixed D13 （71f203b0。CT st09_mig_full_migrate_still_ends_with_the_heartbeat_started_on）

## R11. 欠けの報告が重複しうる（消せなかった回は新しい id で報告し直す）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalLedger.kt
- 根拠: `settle` / `gone` は報告してから行を消す（Task 5 F9）。`/media/summary` の `missing` が二重に数えられる
- kind: technical
- 処置: fixed D11 （381c71bb。欠けの報告の id を UUID v5（利用者・ソース・sha256・理由）。S-01 は同じ id・同じ中身を duplicate 200 で受ける。GT OriginalUploaderTest.a_missing_report_sent_again_after_a_failed_removal_keeps_the_same_id）

## R12. `tools/smoke.sh` の途中（23-25 行）に `ASHIATO_MEDIA_DIR` を足している
- 成果物: tools/smoke.sh
- 根拠: 共有の台本の途中への追記。`tools/check-shared-appends.sh` は通るが、並走する Story と衝突しうる
- kind: technical
- 処置: rejected: smoke.d はサーバを起動した後に `.` で読まれる（tools/smoke.sh:1127）が、`ASHIATO_MEDIA_DIR` は起動前の環境に要る（無ければ本物の HOME の既定の置き場に書く）。smoke.d の中で起動し直すと起動のコマンドを複製し後ろの節にも響く。末尾への追記ではなく `tools/check-shared-appends.sh` は rc=0

## R13. 原本を rename した後にディレクトリを fsync していない
- 成果物: crates/server/src/media.rs
- 根拠: Task 6 F3。電源断の直後に「行はあるがファイルが無い」状態ができ、HEAD が 200 を返すので端末が送り直さない
- kind: technical
- 処置: fixed D10 （71f203b0。rename の後、置いたフォルダから置き場の根まで fsync（unix のみ））

## R14. AT の数え方（結果の XML のファイル名でクラスを探す）が常に 0 本になる
- 成果物: openspec/changes/st09-media-capture/tasks.md
- 根拠: 結果のファイル名は `TEST-ashiato-api35(AVD) - 15-_app-.xml` でクラス名を含まない（Task 2 ⚠️ / Task 10 F6）。Global Constraints の `AT <クラス>` の `find -name 'TEST-*<クラス>*.xml'` は `<testcase classname=…>` で数える定義でないと成立しない
- kind: premise
- 処置: escalated （deep.md Q18。tasks.md の前置きの `AT` の定義は項目の検証コマンドでなく `plan_fix.py` の対象外。`AT` の略記は `find` で始まり evidence.py の検証コマンドにも拾われていない（実測: `declared()` が 2.2 で `[]`）。数え方の直しを本人に戻した）

## scoped re-review（6e023e9..71f203b）

fix は 1 回（`.superpowers/sdd/st09-final-fix-report.md`）。再レビューの判定: R1・R3〜R11・R13 は ADDRESSED、R12 は意図して直さず（上の rejected）、
R2 は fix の範囲では NOT ADDRESSED（HEAD の計測テストは controller が別に走らせる）。fix が持ち込んだ Critical / Important は無し
（Minor 2: 台帳の追記は fsync しないが、切れた最後の行を捨てて次の契機で足し直す形が吸収する / 同じ id の欠けの報告を二重に積みうるが S-01 が重複として畳む）。

## Task の ⚠️ の確かめ（final reviewer。指摘にしなかった分）

- Task 1（列の形）: `media_original` は主キー (user_id, sha256)・場所の列なし、`media_missing` は 7 理由の CHECK、`text_extraction` は D17 どおり
- Task 3（スレッド）: `start()` は `timer.after(0)` で位置とは別の `ScheduledExecutor`
- Task 4 / 7（`network`）: `#[serde(default)] Option<bool>`・`deny_unknown_fields` なし・`docs/openapi.json:3037` に出ている
- Task 5（欄の名前・状態コード）: サーバと一致。413 だけ R4。`onCreate` の順は `openStores()` → `buildSources()`、`startFlushing` は `onStartCommand`
- Task 6（1 GiB）: 外側に RequestBodyLimitLayer は無く、経路の layer が上書き（試験は R9）
- Task 8（起動時の当て直し）: `run()` は `migrate()` を呼ばない。実行時は `touch_started_on` だけで、印が `heartbeat` なら記録で動かさない（試験は R10）
- Task 9（SPAN_RULES）: `excluded.rs:53-55` で 3 表とも `Point`、`COORD_SOURCES` にも入っている
- Task 11（`openspec validate st09-media-capture --strict`）: final の段で走らせた。rc=0
- Ledger の Ruling（D15 の権限）: アプリの役割が所有者でも superuser でもないことを起動時に検査しており、トリガを外せない。受け入れる（正典の側は R7）

## code-verify（HEAD `7ec57aad`。作業ツリーは差分なし）

席: code_verify（`final` の後・`finish` の前）。手 1〜7 を全部。壊したもの・足した一時テストはすべて戻し、前後で `git status --porcelain` と `git diff` が空のまま一致することを毎回確かめた。

### 申告と実測

申告: tasks.md の `[x]` は **28/28**（1.1〜11.2）。実測は HEAD で全部走らせ直した。

| 項目 | 検証コマンド（tasks.md の本文） | 実測 | 一致 |
|---|---|---|---|
| 1.1 ほか | `cargo test -p ashiato-server`（全体） | 作り直した DB で **1279 passed / 0 failed・rc=0**（5150 秒。`.superpowers/sdd/logs/20261020-223042-ct.log`）。積もった DB では 2 本落ちた（下の注） | 一致 |
| 1.1〜9.2 | `CT` の接頭辞の下限（`-- --list`） | `st09_mig_` 13（≥6）/ `st09_up_ident_` 4（≥4）/ `st09_up_dir_` 5（≥5）/ `st09_up_put_` 17（≥15）/ `st09_up_log_` 1（≥1）/ `st09_miss_` 6（≥4）/ `st09_sum_` 6（≥6）/ `st09_cov_net_` 2（≥2）/ `st09_cov_start_` 5（≥5）/ `st09_cov_state_` 7（≥7）/ `st09_exp_` 7（≥7）/ `st09_del_` 3（≥3）/ `started_on` 14 | 一致 |
| 2〜5・10.2 | `GT <クラス>` の本数 | MediaRecordTest 22（≥16）/ MediaRecordSizeTest 3（≥1）/ MediaScannerTest 25（≥12）/ MediaVanishTest 12（≥10）/ MediaPermissionTest 5（≥2）/ MediaCapabilityTest 20（≥11）/ MediaNoticeTest 5（≥3）/ OriginalLedgerTest 12（≥5）/ OriginalUploaderTest 27（≥14）/ HeartbeatCountersTest 13 / RetentionTest 11。単体テスト全体 608 本・失敗 0 | 一致 |
| 2.2・4.1・4.3・10.2 | `AT <クラス>` | HEAD 71f203b0 の走行（final review R2）の XML を `classname=` で数え直した: 1 段目 `MediaCaptureInstrumentedTest` 2・3 段目 `MediaPremisesInstrumentedTest` 3・失敗 0。tasks の `find -name 'TEST-*<クラス>*.xml'` の数え方は 0 本になる（final review R14 のとおり） | 数え方を変えれば一致 |
| 1.1 | `tools/check-migrations.sh` | rc=0 | 一致 |
| 1.2 | `tools/check-immutable.sh` / `tools/check-shared-appends.sh` | rc=0（ST09 の節 9 件とも「拒んだ」）/ rc=0 | 一致 |
| 5.3・6.3・11.1 | `tools/check-log-private.sh` | rc=0（ただし R18） | 一致 |
| 6.2 | `cargo clippy -p ashiato-server --all-targets -- -D warnings` | rc=0 | 一致 |
| 7.2 | `tools/check-openapi.sh` | rc=0 | 一致 |
| 9.2 | `git diff --quiet origin/main -- crates/server/src/deletion.rs` | rc=0 | 一致 |
| 10.1 | `grep -c` 3 本 / `check_chain.py` | 3 / 2 / 1（どれも ≥1）/ rc=0 | 一致 |
| 10.2 | `scripts/quiet-run smoke -- tools/smoke.sh` | rc=0（ST09 の 2 節とも通過。`cmp` でバイト一致まで見ている） | 一致 |
| 11.1 | `check_scenarios.py .` / `check-private.sh` / gradle `detekt :app:assembleDebug :app:testDebugUnitTest` / `openspec validate --strict` | rc=0（Scenario 1621 件・担保あり 1621）/ rc=0 / rc=0 / rc=0 | 一致 |
| 11.2 | `grep -q` 4 ソース / `grep -cE` premises | rc=0 / 4 | 一致 |

注（ST09 の指摘にしない）: 5 時間ぶん積もったテスト用 DB で全体を走らせると `day_tests::st25_reasons_before_collection_started` と
`archive_flow_tests::archive_youtube_last_event_is_reported_per_logical_source` が落ちた。前者は ST33 の `testdb::unique_day()`（2000〜2004 年）が入れた
`c01-location` の記録で、その収集開始日が `2000-08-21` になり、ST25 の試験が「2001 年は開始前」と読めなくなったため
（`SELECT collection_started_on FROM core.source WHERE logical_source='c01-location'` → `2000-08-21`）。ST09 はどちらの試験もその経路も変えていない。
DB を作り直すと 2 本とも通る。11.1 の証跡（evidence.jsonl 155 行）は 22bd6503 の 1276 本で、その後の fix（381c71bb・71f203b0）を含む HEAD の全体の走行は無かった —— 上の 1279 本がそれに当たる。

### 手 1: 固定値を独立に再計算する

- `MediaRecordTest.content_hash_is_sha256_of_the_whole_file` の期待値 `c2d24b1e…5b762` を python の `hashlib.sha256` で `camera-gps.jpg` から計算 → **一致**
- 原本の欠けの報告の識別子（`mediaMissingId`。R11 の修正）はテストが自分の関数どうしを比べるだけで、外の値に固定されていない。一時テストで 2 件出し、
  python の `uuid.uuid5(6f0d5c1e-…, "<利用者>\n<ソース>\n<sha256>\n<理由>")` と比べた → `72ce836a-103f-53e1-a83b-88d47ff83011` / `89a797f0-50e3-5fa1-bc0c-789e33a17a4f` で**一致**（RFC 4122 の v5 どおり）
- 置き場の名前の固定値 `2026/10/20261017-143012_IMG.jpg`（`st09_up_put_same_name_other_content_gets_hash_prefix`）は、TAKEN `05:30:12Z` + 540 分 = `14:30:12` から手で求めた値と一致

### 手 2: ガードをわざと壊す

- 移行の `media_missing_append_only`（行トリガ）を消した複製で `tools/check-immutable.sh` → **rc=1**、`NG アプリの役割で変えられた: UPDATE core.media_missing …`。戻した後 rc=0
- `media.rs:533` のログに `path = ?part_path` を足して `tools/check-log-private.sh` → **rc=1**（許可していないフィールド `path`）。戻した
- Kotlin の側は落ちない → **R18**

### 手 3: Scenario と test

`check_scenarios.py . st09-media-capture` rc=0。ST09 の 183 本の印の置き場: CT だけ 90 / GT だけ 79 / CT+GT 7 / CT+smoke 3 / AT+GT 2 / AT+GT+smoke 2。
GT だけで担保し、THEN が D-02 / S-01 を主語にするものは 3 本。うち 2 本（生存信号が S-01 へ送られる）は送る側の振る舞いで、受けた側は `st09_cov_net_` が持つ。
残る 1 本は主張の階層とテストの階層が違う → **R16**

確かめて指摘にしないもの: 端から端の `MediaCaptureInstrumentedTest`（「写真を撮るとメタが D-01 に、原本が D-02 に入る」の印）は、テストのアプリが入れた画像を
`c01-image` として通している（MediaCaptureInstrumentedTest.kt:275 `assertEquals("c01-image", photoRecord.logicalSource)`）。WHEN はカメラの写真（D3 では `c01-photo`）だが、
`c01-photo` への振り分けは `MediaScannerTest` と smoke.d（`c01-photo` で `/ingest` → `PUT` → `cmp`）が見ており、記録と原本を送る経路は 4 ソースで共通

### 手 4: 本人の決定が test で固定されているか

tasks の Global Constraints の定数と、design の（仮）の値を書き換えて単体テスト全体・`media_tests::` を走らせた:

| 値 | 書き換え | 結果 |
|---|---|---|
| `OMIT_BINARY_OVER = 256`（D4） | 512 | 落ちた（`raw_leaves_out_embedded_thumbnail_and_binary_blobs_and_names_them`） |
| `RECORD_MISSING_GIVE_UP = 7 日`（D9） | 8 日 | 落ちた（`record_missing_is_retried_and_becomes_a_missing_original_after_seven_days`） |
| `MEDIA_INTERVAL_MS = 6 時間`（4 ソースの生存信号） | 5 時間 | 落ちた（`the_four_media_cadences_are_six_hours_and_satisfy_the_interval_invariant`） |
| Q9 過去ぶんは従量制で送らない | 絞り込みを外す | 落ちた（3 本） |
| Q15 位置の権限が無ければ開かない | `NO_LOCATION` でも開く | 落ちた（`nothing_is_opened_while_the_location_permission_is_missing`） |
| C12 選んだ写真だけでは消えたと数えない | `FULL` の条件を外す | 落ちた（`narrowing_access_to_selected_photos_does_not_vanish_the_rest`） |
| **`UPLOAD_RETRY` の最初の 1 分**（D9） | 2 分 | **落ちない**（608 本全部緑。`retries_wait_one_minute_then_double_up_to_one_hour` も緑）→ **R17** |
| **`NAME_MAX_CHARS = 120`**（D10a） | 100 | **落ちない**（`media_tests::` の 23 本全部緑。同じ走行に足した一時テスト 1 本を含め 24 passed）→ **R17** |

ほかの定数（`TAKEN_AT_FLOOR`・`TAKEN_AT_MAX_AHEAD`・`SCAN_BATCH`・`OBSERVER_DEBOUNCE`・`FULL_RECONCILE_EVERY`・`ORIGINAL_BODY_LIMIT`（端末と S-01 の両方）・`MEDIA_PERMISSION_POLL_MS`・`RECORD_SIZE_GATE`・`HASH_SUFFIX_CHARS`（`&sha_b[..8]`）・`BACKLOG_AGE`（23 時間 / 25 時間））はテストが値を直書きで比べている（grep で確かめた）。

### 手 5: tasks の `[x]` と実体

28 項目の検証コマンドは全部実在し、上の表のとおり rc=0。`cargo test --test <名前>` 型の検証は無い。AT の数え方は final review R14（premise）で既に挙がっているので重ねない。

### 手 6: 隙間

- **D-02 のファイルが後から失われたとき** → **R15**（捨てたものは復元できない型）
- 「選んだ写真だけ」に変えた後、選ばれていない写真の原本の送り待ち（`OriginalUploader.kt:130` は `PARTIAL` でも開く）: `FileNotFoundException` → `Vanished` → 行を消す、を疑い、
  エミュレータ（API 35。`READ_MEDIA_VISUAL_USER_SELECTED` と `ACCESS_MEDIA_LOCATION` だけを `pm grant`）で一時的な計測テストを走らせた →
  `SecurityException: dev.ashiato.collector has no access to …?requireOriginal=1` → `ContentResolverOriginalSource.open -> Denied`（行を残して試し直す）。**捨てない。指摘にしない**
- 欠けの報告を積めなかったとき（`OriginalMissingReporter.report` はメモリに持って行を消す）: 台帳と未送信が同じ置き場で、置き場が書けなければ `removeAll` も失敗して行が残る。ST04 の破棄の報告と同じ扱い。指摘にしない
- `put_inner` の名前を変えた後・受け取りの行を積む前に止まった跡は、次の `PUT` の `place()` が同じハッシュの既存ファイルを見て行を積む（自己修復）。指摘にしない

### 手 7: 機械が承認した検証コマンドの訂正

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st09-media-capture` → `[]`）。

## R15. D-02 の原本のファイルが失われると、HEAD も PUT の送り直しも 200 を返し、原本は二度と置かれない
- 成果物: crates/server/src/media.rs / collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalUploader.kt
- 根拠: 一時テスト（`media_tests.rs` の末尾に足して実行・削除）で、原本を `PUT` → 201 の後に D-02 のファイルを消し、`HEAD` と同じバイトの `PUT` をした →
  `ZZPROBE plain HEAD: placed=["2026/10/20261017-143012_lost.jpg"] after removing it: head=200 OK put_again=200 OK files=[]`
  （HEAD 7ec57aad の素のコードに一時テストだけを足した `cargo test -p ashiato-server st09_zz_` rc=0。`.superpowers/sdd/logs/20261021-000326-ct.log`。R16 の書き換えを入れた走行 `20261020-215512-ct.log` でも同じ）。
  `received()`（media.rs:574-585）は `core.media_original` の行だけを見て、ファイルを見ない。`PUT` は行があればファイルを置かずに 200（media.rs:510-511）。
  端末は `HEAD` 200 で読まずに台帳の行を消す（OriginalUploader.kt:231）。ST10 は「不一致を報告する」だけで、送り直しを求める口を持たない（docs/stories/ST10.md の完了の判定・docs/handoff/ST10.md）。
  Q12 で本人は D-02 のフォルダを開くことがある。手で動かした・消した原本は、端末にまだ写真があっても戻らず、Q2 の「まとめて消す」で端末からも消えると取れない
- kind: irreversible
- loss: discarded
- 処置: fixed D25 仮 （media.rs の `on_disk` / `received_and_on_disk`。行に加え、並びの 3 つの名前に受け取った大きさのファイルがあるかを見る。無ければ HEAD 404・PUT は置き直して 201。CT st09_up_put_lost_original_is_asked_again_and_placed_again（直す前の素のコードでは code-verify の一時テストどおり head=200）。provisional.jsonl に載せた）
- 提案: `received()` と `PUT` の重複の判定で、行に加えて置き場の名前（`layout()` はハッシュと記録から決まる）にハッシュの合うファイルがあるかを見て、無ければ 404 / 置き直す。
  ファイルを見ない形を残すなら、送り直しを ST10 の範囲として handoff に書く

## R16. 「原本は形式を変えずに送られる」の THEN（D-02 のファイルが HEIC のまま）を観測するテストが無い
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/OriginalUploaderTest.kt / crates/server/src/media_tests.rs
- 根拠: 印は `OriginalUploaderTest.a_heic_original_arrives_byte_for_byte`（OriginalUploaderTest.kt:50-64）だけで、見ているのは偽の `Transport` が受けたバイト。
  `media.rs` の `sanitize_name` に `.heic` / `.HEIC` を `.jpg` に変える 1 行を足して `cargo test -p ashiato-server media_tests::` → **既存の 23 本全部緑**（24 passed / 0 failed。1 本は R15 の一時テスト。同じ走行に R17 の `NAME_MAX_CHARS` の書き換えも入れた）。
  `grep -in heic crates/server/src/media*_tests.rs` は 0 件、smoke.d は `camera-gps.jpg` だけを送る
- kind: technical
- 処置: fixed D10 （CT st09_up_put_heic_original_keeps_its_format_and_bytes に Scenario「原本は形式を変えずに送られる」の印。`IMG_0001.HEIC` の名前と中身のバイトが D-02 で変わらないことを見る）
- 提案: `media_tests.rs` に、`IMG_0001.HEIC` の記録で `PUT` し、置かれたファイルの名前の拡張子と中身のバイトが送ったものと同じことを見るテストを足し、この Scenario の印を置く

## R17. 本人の決定ではない（仮）の値のうち 2 つは、書き換えても全部のテストが通る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalUploader.kt / crates/server/src/media.rs
- 根拠: `RETRY_FIRST_MINUTES = 1L`（OriginalUploader.kt:11）を `2L` にして `:app:testDebugUnitTest` → 608 本・失敗は他の書き換え由来の 3 本だけで、
  `retries_wait_one_minute_then_double_up_to_one_hour`（OriginalUploaderTest.kt:403）は緑（待ちを `UPLOAD_RETRY_FIRST` から読み、最後の上限 1 時間だけを直書きで比べる）。
  `NAME_MAX_CHARS = 120`（media.rs:28）を 100 にして `media_tests::` → 既存の 23 本全部緑（media_tests.rs:591 は `media::NAME_MAX_CHARS` と比べる）。
  tasks の Global Constraints は「本人の決定と仮決めの値は定数にして試験が名指しで固定する」で、この 2 つを名指ししている
- kind: technical
- 処置: fixed D10 （`assert_eq!(media::NAME_MAX_CHARS, 120)`（media_tests の separators の試験）と `assertEquals(Duration.ofMinutes(1), UPLOAD_RETRY_FIRST)`（OriginalUploaderTest.retries_wait_one_minute_then_double_up_to_one_hour）を足した）
- 提案: `assertEquals(Duration.ofMinutes(1), UPLOAD_RETRY_FIRST)` と `assert_eq!(media::NAME_MAX_CHARS, 120)` を足す

## R18. `tools/check-log-private.sh` は Kotlin の `Telemetry.line(...)` の引数の中身を見ない（内容ハッシュを出しても rc=0）
- 成果物: tools/check-log-private.sh / collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalUploader.kt
- 根拠: `OriginalUploader.kt` の `pass()` に `log(Telemetry.line("original_try", source = null, count = 1, error = row.sha256))` を足して `tools/check-log-private.sh` → **rc=0**（`check-log-private: OK`）。
  検査は第 1 引数が `Telemetry.line(` で始まれば通す（check-log-private.sh:196-197）。`Telemetry.line` は `error` を素通しで足す（Telemetry.kt:38）。
  S-01 の側は同じ種類の書き換えで rc=1 になる（手 2）。端末の側で守っているのは `OriginalUploaderTest.a_failed_upload_logs_only_the_kind_and_the_count` の 503 の 1 経路だけ
  （tasks の Global Constraints「ログに位置・ファイル名・保存先のフォルダ・原文・内容ハッシュ・`media_id` を出さない（端末と S-01）」）
- kind: technical
- 処置: fixed D21 （tools/check-log-private.sh に `check_telemetry_args`: `Telemetry.line` の kind / source / error（名前つき・位置引数・文字列テンプレートの中）に内容ハッシュ・位置・名前・保存先などの名前を渡したら落とす。code-verify と同じ書き換え（`error = row.sha256` を `pass()` に足す）で rc=1、戻して rc=0。self-test に 5 例）
- 提案: 検査で `Telemetry.line` の `error =` / `kind =` に変数を渡したら落とす（文字列の定数か、語の一覧に限る）か、ST09 の送り手と読み手のログの全経路を 1 本の試験で回して秘密の語を探す

## security review（77bbdba..7ec57aa）

見た経路:
- `PUT /media/originals/{sha256}` → `put_inner` の先頭の `authorize`（`crates/server/src/media.rs:431`）→ 本文を読む前に 401。`.incoming/` にも何も作らない（4）
- `HEAD /media/originals/{sha256}` → `head_original` の `authorize`（`media.rs:682`）→ 401。既存の試験には無く、本物のサーバで確かめた（4）
- `POST /media/missing` → `missing_post` の `authorize`（`media_missing.rs:196`）→ 401（4）。`GET /media/summary` → `summary_get` の `authorize`（`media_summary.rs:77`）→ 401（4）。要約が返すのは件数・バイト数・理由の語だけ（`media_summary.rs:80-160`）（3）
- 1 GiB の本文の上限（`lib.rs` の `DefaultBodyLimit::max`）は `/media/originals` の経路だけに付いていて、本文を読むのは認証を通った後（`media.rs:431` → `:470`）。認証の無い者は大きい本文を書かせられない（3）
- 端末が送る `payload.display_name` → `sanitize_name`（区切りの最後の後ろだけを残し、`..` は `media`）→ 名前の頭に時刻（`layout`）→ `file_name() == name` の検査と、正規化したフォルダが置き場の下にあるかの検査（`media.rs:620-631`）→ `rename`。フォルダは記録の時刻だけから作り、送られた文字は使わない。置き場の外には書けない（3）
- 記録を消した後の原本 → `find_record` は `deleted_at IS NULL` の記録しか見ない（`media.rs:551-556`）。消した記録で原本を置き直すことはできない（3）。D-02 に置いたファイルを消すのは設計で ST23 に送っている（design Non-Goals）
- 置いた原本 → `tokio::fs::create_dir_all` / `OpenOptions` / `create_dir`（`media.rs:448`, `:453`, `:611`）は権限を指定しない → umask 022 なら 0755 / 0644（4。R19）
- Android → `HttpOriginalTransport` は `/ingest` と同じ `baseUrl` と Bearer を使う（`OriginalAndroid.kt:49-75`）。新しい接続先・待ち受けは無い。新しいログ（`Telemetry.line`）に出るのは種別・件数・例外のクラス名だけ（`OriginalUploader.kt:155-158`、`MediaLedger.kt`、`MediaScanner.kt`）（2）
- 依存 → `http-body-util 0.1` は dev から本体へ移しただけで、版も同じ。`tokio` に `fs` / `io-util` の機能を足した。どちらも新しい crate ではない（2）

安全の決め手: 新しい 4 つの経路はどれも、ハンドラの最初の処理が `authorize` で、本文も DB も触る前に 401 で返る（router に認証の層は無く、ハンドラごとの呼び出しだけが守っている）—— 確かさ 4。
`/tmp/st09sec/run.sh`（HEAD の `target/debug/ashiato-server` をこの worktree の DB に繋ぎ、`ASHIATO_MEDIA_DIR` を一時のフォルダにして起動）の出力:
`PUT 401` / `HEAD 401` / `POST missing 401` / `GET summary 401` / `PUT bad-token 401` / `incoming: 0`（認証の無い PUT は書きかけを残さない）。
例外は query が壊れているとき（`user_id=zz`）で、axum の `Query` が先に断るので 400 になる。返るのは解釈の失敗の文言だけで、記録の値は出ない。

## R19. 撮影位置を含む写真の原本が、他の利用者も読める権限（フォルダ 0755・ファイル 0644）で置かれる
- 成果物: crates/server/src/media.rs
- 根拠: `/tmp/st09sec/run-default.sh`（`ASHIATO_MEDIA_DIR` を外し、`XDG_DATA_HOME` を一時のフォルダにして既定の置き場を使う。umask 022）で、
  写真の記録を `/ingest` に送ってから `camera-gps.jpg`（GPS 35.6812 N / 139.7671 E を EXIF に持つ。`app/src/test/resources/media/README.md`）を PUT すると 201 が返り、置き場はこうなった:
  `drwxr-xr-x …/ashiato/media`、`drwxr-xr-x …/media/2026/09`、`-rw-r--r-- …/2026/09/20260908-110000_camera-gps.jpg`、`drwxr-xr-x …/media/.incoming`。
  経路: `prepare` の `std::fs::create_dir_all`（`media.rs:155`）、`place` の `tokio::fs::create_dir_all`（`:611`）、`.incoming` の `create_dir`（`:448`）、`.part` の `OpenOptions::new().write(true).create_new(true)`（`:453`）のどれも権限を指定していない。
  `rename` は `.part` の 0644 をそのまま引き継ぐ。同じサーバの書き出し（`export.rs:1007-1030` の `ensure_dir` 0700 / `create_private_file` 0600）と `migrate_backup.rs:138,150` は持ち主だけに絞っていて、D-02 だけが絞っていない。
  ホームのフォルダが他の利用者に開いている環境（0755 の home・共有の `ASHIATO_MEDIA_DIR`）では、同じ機械の別の利用者や別のプロセスが、撮影位置・撮影時刻・カメラの写真を全部読める（確かさ 4）
- kind: technical
- loss: exported
- 処置: fixed D26 仮 （media.rs: 既定の置き場・年月・`.incoming/` を `DirBuilder::mode(0o700)`、`.part` を `OpenOptions::mode(0o600)`。既にあるものは変えない。CT st09_up_put_original_is_readable_only_by_its_owner。provisional.jsonl に載せた）
- 提案: 書き出しの `ensure_dir` / `create_private_file` と同じく、Unix では `.incoming`・年月のフォルダ・既定の置き場を `DirBuilder::mode(0o700)`、`.part` を `OpenOptions::mode(0o600)` で作る。
  既にあるフォルダの権限は変えない（書き出しと同じ方針）。umask 022 で置いたファイルとフォルダが持ち主だけに絞られていることを確かめる試験を `media_tests.rs` に足す

範囲外:
- `AndroidManifest.xml` の `<application>` に `android:allowBackup` / `dataExtractionRules` が無く、既定の自動バックアップが有効のまま（`filesDir` の未送信の置き場に、この差分から写真の記録（緯度経度・ファイル名）も積まれる）。仕組みは差分の前からあるもので、バックアップに実際に入るかは確かめていない（2）

## /code-review（77bbdba..7ec57aa）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R20. OriginalLedger.queue は、既存の行がある場合に recordId しか付け替えない（state が ready なら何も変えない）。そのため、項目が動いた・一覧が作り直されて番号が振り直された（MediaScanner.decide の consumed/twin の枝）後も、原本の送り待ちの行は古い volume / mediaId を指したまま残る。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalLedger.kt:121
- 根拠: 写真 X の行が mediaId=100 で ready のまま、MediaStore が作り直されて X が mediaId=5000 になる。MediaScanner は twin を consumed として同じ記録を更新し queue() を呼ぶが、行は ready なので変わらない（awaiting のときも recordId だけが変わる）。OriginalUploader は id 100 で setRequireOriginal の URI を開いて FileNotFoundException → Vanished → 欠け 'vanished' を報告して行を消す。原本は端末に残っているのに S-01 には二度と届かない。（確かさ 2）
- kind: technical
- 処置: fixed D8 （再現: 読みで確かめた —— `queue` は既存の行の recordId しか替えず、原文が同じ振り直し（twin・送り直し無し）では `queue` も呼ばれない。`OriginalLedger.queue` が付け替えで volume / media_id も移し、MediaScanner は送り直さない振り直しでも行を新しい番号へ移す。GT OriginalUploaderTest.an_original_waiting_through_a_rebuilt_listing_is_read_under_its_new_number / an_original_waiting_through_a_move_is_read_where_it_moved（直しを外すと 2 本とも落ちた））
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R21. 名前の上限 NAME_MAX_CHARS（120）を文字数で数えているが、ext4 / NTFS の名前の上限はバイト（または UTF-16 単位）で決まる。日本語の長い display_name に時刻（16 文字）とハッシュの頭（9 文字）が付くと 255 バイトを超え、rename が ENAMETOOLONG で落ちる。これが PlaceError::Io → storage_kind 'io' → 503 storage_unavailable に化ける。
- 成果物: crates/server/src/media.rs:230
- 根拠: display_name が日本語 100 文字（UTF-8 で 300 バイト）の画像を PUT する。rename(part, target) が名前の長すぎで失敗し、503 と last_storage_error=io が返る。置き場の障害ではないのに画面には書けない障害と出る。端末は 503 を Retry(stop=true) と読むので、1 時間ごとにこの行で送りの回が止まり、1 GiB まで読み直し続ける。（確かさ 2）
- kind: technical
- 処置: fixed D10 （再現: 120 文字の日本語は 360 バイトで、時刻とハッシュを足すと 255 を超える。`NAME_MAX_BYTES = 170`（時刻 16 + 全ハッシュ 65 を足しても 255 以内）でも切る。CT st09_up_put_long_japanese_name_fits_the_file_name_limit と separators の試験の切り方）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R22. 422 は種別を問わず（name_refused・hash_mismatch・video_not_accepted）欠けにせず、いつまでも試し直す。中でも name_refused（name_taken / outside_root / bad_name）は試し直しても変わらない拒否で、7 日で諦める道も無い。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalUploader.kt:290
- 根拠: 同じ秒・同じ名前で中身の違う画像が 3 枚あると、3 枚目は S-01 で name_taken になり毎回 422 になる。端末は 1 時間ごとに HEAD → 全体を 2 回読む（最大 1 GiB×2）→ PUT を永久に繰り返す。電池と通信を使い続け、/media/summary では送り待ちのまま残る。（確かさ 2）
- kind: technical
- 処置: fixed D10 （筋書きの「同じ秒・同じ名前の 3 枚目が name_taken」は再現しない —— 3 枚目は自分のハッシュの頭で別の名前になる（media.rs の layout）。永久の拒否になるのは頭 8 文字が重なる別の中身か、手で置かれたファイルがあるとき。3 つ目の候補として全ハッシュの名前を足し、name_taken を実質なくした。CT st09_up_put_both_names_taken_lands_under_the_full_hash_name。残る outside_root（年月のフォルダが置き場の外へのリンク）は本人が直せば通るので試し直しのまま）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R23. notifyListener は listener の例外を runCatching で飲み込み、空の held を返す。そのため例外が起きると listener が処理できなかった記録まで未送信から取り除かれ、『書けなかったら取り除かない』という保証が崩れる。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt:227
- 根拠: OriginalRecordLink.accepted の中で例外が出る（例: payload['kind'] がオブジェクトで jsonPrimitive が IllegalArgumentException を投げる、台帳のファイル操作が予期しない実行時例外で落ちる）。記録は outbox から消えるが、原本の行は awaiting_record のまま ready にならない。送られることも欠けとして報告されることもなく、行が永遠に残る。（確かさ 2）
- kind: technical
- 処置: fixed D8 （再現: 読みで確かめた —— 例外で held が空のまま返り、記録が未送信から消える。Sender.notifyListener は知らせごとに例外を受け、その分を取り除かずに残す。GT OriginalUploaderTest.a_listener_that_throws_keeps_the_record_until_it_is_handled（直しを外すと落ちた））
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R24. 置き場が設定されていない（MediaConfig::none）ときの Failure::Storage("dir_missing") は stored() を通らず、note_storage_error が呼ばれない。そのため /media/summary の last_storage_error が null のまま残る。
- 成果物: crates/server/src/media.rs:445
- 根拠: ASHIATO_MEDIA_DIR・LOCALAPPDATA・XDG_DATA_HOME・HOME のどれも無い環境で起動すると、PUT はすべて 503 storage_unavailable になる。それでも GET /media/summary は last_storage_error=null を返し、本人は送り待ちが増えるだけで、置き場が無いことに気付けない。（確かさ 2）
- kind: technical
- 処置: fixed D10 （再現: 読みで確かめた。置き場が決まらないときも `note_storage_error("dir_missing")`。HEAD も 404。CT st09_up_put_without_any_place_notes_the_storage_error）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R25. onDropped（OriginalRecordLink.dropped）の戻り値（removeAll が書けたか）を捨てている。台帳に書けないと、記録は捨て済みなのに原本の行は awaiting_record のまま残り、試し直す契機も無い。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt:102
- 根拠: 上限で記録が捨てられる。dropped が record_dropped を報告した後、ledger.removeAll がディスクの書き込みエラーで false になる。戻り値は無視され、行は awaiting のまま固まる。この記録は二度と受理されないので ready にも消去にもならず、台帳に古い行が溜まり続ける。（確かさ 2）
- kind: technical
- 処置: fixed D8 （再現: 読みで確かめた —— Retention は `onDropped` の戻りを捨て、記録は捨て済みなので知らせ直されない。OriginalRecordLink が消せなかった行（ハッシュ → 記録の id）を覚え、次の知らせ（受理・拒否・破棄）で消し直す。その間に別の記録へ付け替わった行は消さない。GT OriginalLedgerTest.a_row_that_could_not_be_removed_after_a_drop_is_removed_on_the_next_notice）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R26. 200 件の 1 ページごとに saveLedger がその置き場の台帳を全部書き直す。初回の全件の読みでは書き込み量が件数の 2 乗で増える。OriginalLedger は同じ理由（R6）で追記にしているのに、ここは揃っていない。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MediaScanner.kt:214
- 根拠: 写真 1 万枚の端末の初回の読みで、50 ページ × 平均 5 千行 ≈ 25 万行（数十 MB）を tmp に書いて rename するのを 50 回繰り返す。フラッシュへの書き込みと電池が無駄になる。ページごとに追記するか、置き場の終わりで 1 回だけ書けばよい。（確かさ 2）
- kind: technical
- 処置: fixed D8 （再現: 読みで確かめた —— 頁ごとに全体を書き直す。途中の保存を 1・2・4・8… 頁目と最後の頁だけにし、書く量を件数に比例させた（落ちても計算し直すのは読んだ分の半分まで。R6 の「途中で落ちても計算し直さない」を半分まで緩めた）。GT MediaScannerTest.the_first_full_read_rewrites_the_ledger_only_on_doubling_pages（1001 枚で 200 / 400 / 800 / 1001 / 1001 行の 5 回。読み直しで開き直さない））
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R27. newMediaTimer は呼ばれるたびに newSingleThreadScheduledExecutor を作るが、MediaScanDriver.stop / onDestroy は予約を取り消すだけで executor を shutdown しない。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:194
- 根拠: START_STICKY のサービスが同じプロセスの中で onDestroy → onCreate を繰り返すと、そのたびに読み手の糸が 1 本ずつ漏れる（非デーモンの糸が生き残る）。長く動かすと糸とメモリが増え続ける。（確かさ 2）
- kind: technical
- 処置: fixed D1 （再現: 読みで確かめた —— `MediaScanDriver.stop` は予約を取り消すだけ。`MediaScanTimer.close` を足し、`ExecutorMediaScanTimer` は `shutdownNow`、`LocationService.onDestroy` は `mediaCapture.close()`。GT MediaCapabilityTest.closing_the_capture_closes_the_reader_timer）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R28. 手順 5 の収集開始日の当て直しは、起動のたびに全版を当てる migrate で毎回走る。そのうえ touch_started_on が持つ『started_on_from IS NULL』の条件が無い。開始日を借りる先が設定されたメディアのソースでは、起動のたびに生存信号の最小日で上書きされる。加えて core.heartbeat 全体を、相関サブクエリで 2 回引く。
- 成果物: migrations/202610181200_media_capture.sql:130
- 根拠: 本人が c01-screenshot に started_on_from を設定しても、S-01 を起動し直すたびに collection_started_on が生存信号から計算し直される。受け口側（touch_started_on）とは違う値になる。生存信号が数百万行あると、起動のたびに全件を走査する。（確かさ 2）
- kind: technical
- 処置: rejected: 「起動のたびに」は再現しない —— `migrate()` はサーバの起動（`run()`）では呼ばれず、`ashiato-server migrate` のときだけ当たる（lib.rs:165-166 の取り決め・main.rs:8）。`started_on_from` のあるソースは、自分の `collection_started_on` を使わず借り先の鎖から引いた日で置き換える（coverage.rs:561-563）ので、当て直しで自分の値が変わっても見える開始日は変わらない。借りを外したときは、生存信号から引いた自分の値の方が正しい。生存信号の走査は印が `heartbeat` のソースに限り（いまはメディアの 4 ソース）、移行のときだけ
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## R29. 接続を確かめる処理が例外を投げると、getOrDefault(false) で『接続なし』として送る。『無い欄は分からない（null）』という D7 の取り決めに反する。
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Heartbeat.kt:347
- 根拠: ConnectivityManager が SecurityException などを投げると、生存信号に network=false が載り、raw の指紋にも入る。S-01 には接続が無かったと記録され、実際は接続できていた区間が圏外の区間に見える。getOrNull() にすれば欄ごと省ける。（確かさ 2）
- kind: technical
- 処置: fixed D7 仮 （再現: 読みで確かめた。`runCatching(it).getOrNull()` で、確かめられなければ欄ごと載せない（分からない = null）。GT MediaCapabilityTest.a_connection_check_that_throws_leaves_the_network_field_out）
- 提案: （/code-review は提案を持たない。finish が直し方を決める）

## scoped re-review（7ec57aa..0f58c59。finish の fix の後）

fix は 1 回（0f58c591）。再レビュー（pr-review-toolkit の code-reviewer。テストは走らせ直さず、fixer の証跡を読む）の判定:
R15〜R27・R29 は ADDRESSED（R25 は「消し直しはメモリだけ」の穴つき）、R28 は REJECTION OK（「4 ソースに限り」の言い方だけを直した）。

fix が持ち込んだ問題:
- **Important（直した）**: R15 の `on_disk` が今の記録の名前・時刻だけで置き先を組んでいたので、置いた後に記録が更新されて名前が変わる（端末で名前を変えた）と
  HEAD が 404 になり、同じ原本が D-02 に 2 つ目として置かれた（直す前は行だけで答えていたので起きなかった後退）。
  そのハッシュを指す画像の記録の今の版と前の版（`core.event_version`）の全部から候補を組むようにした（場所は持たない。C2 のまま）。
  CT st09_up_put_renamed_record_still_finds_the_placed_original。`cargo test -p ashiato-server st09_` 83 passed（`.superpowers/sdd/logs/20261021-010635-ct.log`。
  途中でこの worktree のテスト用 DB（55509）の入れ物が外から消されていたので `tools/db.sh up -d --wait db` と `tools/db-roles.sh` で作り直した）
- Minor 3（直した）: `ExecutorMediaScanTimer.close` を `shutdownNow` から `shutdown` に（走っている読みに割り込まない）
- Minor 2（大きさだけで見る）・Minor 4（同じ例外を投げ続ける listener）・R25 のメモリだけの消し直し: ledger に Ruling つきで park（PR 本文の Ruling に写した）
- check-log-private の語の一覧の誤検出の恐れ（`name` など）: いまのコードは rc=0。誤検出は `log-ok:` の印で逃がせる。直さない

直した後の検証: clippy -D warnings rc=0 / gradle `detekt :app:assembleDebug :app:testDebugUnitTest` rc=0（615 本・失敗 0）/ `tools/smoke.sh` rc=0 /
`tools/check-log-private.sh` rc=0 / `tools/check-openapi.sh` OK
