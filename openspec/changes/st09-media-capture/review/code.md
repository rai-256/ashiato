# ST09 コードレビュー

## final review（77bbdba..4b18a1a）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package は
`.superpowers/sdd/tasks/review-77bbdbab..4b18a1ab.diff`（68 commits / 89 files）。判定は「With fixes」（Critical 0・Important 3・Minor 9）。
Task ごとの ⚠️ の確認の結果（問題なしとした分）は末尾の「Task の ⚠️ の確かめ」にまとめた。

## R1. 記録の受理と原本の送り待ちの行の登録が競合し、行が `awaiting_record` のまま固まる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MediaScanner.kt
- 根拠: MediaScanner.kt:245-248（`outbox.add` の後に `store.originals.queue`）/ OriginalLedger.kt:239-242（`OriginalRecordLink.accepted` は行が無ければ何もしない）。flusher が間に受理すると、後から積まれた行は ready にならず、Retention・拒否の経路にも乗らない
- kind: technical

## R2. HEAD で計測テストが走っておらず、AT の証跡が evidence.jsonl に無い
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaCaptureInstrumentedTest.kt
- 根拠: 最後の全体の走行は `.superpowers/sdd/logs/20261020-101835-at.log`（6428315b）。その後 22bd6503（detekt の修正）で MediaCaptureInstrumentedTest.kt / PermissionDeniedInstrumentedTest.kt が書き換わった
- kind: technical

## R3. 権限を許可しても、最大 6 時間は読み手も知らせも戻らない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MediaCapture.kt
- 根拠: 知らせの見直しは `start()` と `onOutcome` だけ。実行時の許可ではプロセスが落ちないので、次の 6 時間の見直しか新しい写真の知らせまで戻らない。MediaCapabilityTest.kt:142-143 は `rig.review()` を手で呼ぶので遅れを見ていない
- kind: technical

## R4. 本物の HTTP では 413 が `too_large` にならない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalAndroid.kt
- 根拠: OriginalAndroid.kt:57-63 / OriginalUploader.kt:365。S-01 は Content-Length を見て本文を読む前に 413 で閉じ、固定長の流し送りは書き込み中の IOException → `Unreachable` に畳まれる。1 GiB を超える画像を 1 時間ごとに読み直し続ける
- kind: technical

## R5. 走査の `Pass` の組み立てが try の外にあり、そこでの失敗が `onOutcome` に届かない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MediaScanner.kt
- 根拠: MediaScanner.kt:103。`store.loadCursor()` / `listing.volumes()` / `listing.version()` が投げると失敗の試行にも段にも数えられない（C19）
- kind: technical

## R6. 台帳を 1 行の変更ごと・200 件の頁ごとに全体で書き直す
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalLedger.kt
- 根拠: `OriginalLedger.write` と `MediaScanner.saveLedger`。初回の 1 万件で書き込みが件数の 2 乗で増え、R1 の窓も広げる（Task 3 F5 の延長）
- kind: technical

## R7. design D15 の 4 が「`SELECT` / `INSERT` だけ」のまま、実装の裁定（門のトリガが担う）と食い違う
- 成果物: openspec/changes/st09-media-capture/design.md
- 根拠: `.superpowers/sdd/st09-task-1/progress.md` の Ruling と migrations/202610181200_media_capture.sql のコメント
- kind: technical

## R8. design Risks の「名前の整え方の取りこぼし → 書かずに `422`」と実装（`503 io`）が違う
- 成果物: crates/server/src/media.rs
- 根拠: media.rs:593,597,608 は `outside_root` / `bad_name` / `name_taken` を `503 io` にし `last_storage_error` に `io` を残す（Task 6 F5）
- kind: technical

## R9. 原本の受け口で 2 MB を超える本文が通ることを直接確かめる試験が無い
- 成果物: crates/server/src/media_tests.rs
- 根拠: 1024 の上限の上書きの試験だけ（Task 6 ⚠️ / F4）。経路の `DefaultBodyLimit` が既定の 2 MB を上書きしていることは構造の上でだけ確かめた
- kind: technical

## R10. 全版を当てた後に収集開始日がメディアのソースで戻ることを固定する試験が無い
- 成果物: crates/server/src/media_migration_tests.rs
- 根拠: `st09_mig_polluted_photo_started_on_is_repaired` は `media_sql()` だけを当てる。ORDER の並びが変わったときに守れない
- kind: technical

## R11. 欠けの報告が重複しうる（消せなかった回は新しい id で報告し直す）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalLedger.kt
- 根拠: `settle` / `gone` は報告してから行を消す（Task 5 F9）。`/media/summary` の `missing` が二重に数えられる
- kind: technical

## R12. `tools/smoke.sh` の途中（23-25 行）に `ASHIATO_MEDIA_DIR` を足している
- 成果物: tools/smoke.sh
- 根拠: 共有の台本の途中への追記。`tools/check-shared-appends.sh` は通るが、並走する Story と衝突しうる
- kind: technical

## R13. 原本を rename した後にディレクトリを fsync していない
- 成果物: crates/server/src/media.rs
- 根拠: Task 6 F3。電源断の直後に「行はあるがファイルが無い」状態ができ、HEAD が 200 を返すので端末が送り直さない
- kind: technical

## R14. AT の数え方（結果の XML のファイル名でクラスを探す）が常に 0 本になる
- 成果物: openspec/changes/st09-media-capture/tasks.md
- 根拠: 結果のファイル名は `TEST-ashiato-api35(AVD) - 15-_app-.xml` でクラス名を含まない（Task 2 ⚠️ / Task 10 F6）。Global Constraints の `AT <クラス>` の `find -name 'TEST-*<クラス>*.xml'` は `<testcase classname=…>` で数える定義でないと成立しない
- kind: premise

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
