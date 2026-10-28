# ST49 code review

## final review（d90ef3d..42ac63b）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 3 / Minor 13。
Task ごとの ⚠️ は final reviewer が全体で確かめ、解消済み（Task 3 の `st49_mig_down_and_up_again`・clippy、Task 6 の D9、Task 8 の Scenario 名、Task 11 の本数ほか）か code-verify の席の担当（RED の証跡・`-- --list` の本数）に振り分けた。

## R1. `tools/check-openapi.sh` の `client_version` の除外が Task 9 の後も残り、注記（「Task 9 で外す」）と食い違う
- 成果物: tools/check-openapi.sh
- 根拠: check-openapi.sh:23-25（/ingest）・70-71（drops）/ Task 3 ⚠️・Task 6 F6 / `IngestRequest.kt` は `client_version` を既に持つ
- kind: technical
- 処置: （fix 待ち）

## R2. `docs/handoff/ST29.md` に足した節が「衛星の資格情報はまだ無い・ST49 が先に入る」と書くが、ST29 は archive 済みで `Caller::Satellite => Internal` は既にある
- 成果物: docs/handoff/ST29.md
- 根拠: base d90ef3d の web_session.rs:45 / openspec/changes/archive/2026-10-27-st29-satellite-credentials / lib.rs:1337 / Task 11 F1（「どの版の欄にも書かない」とも読める）
- kind: technical
- 処置: （fix 待ち）

## R3. `tools/check-build-stamp.sh` が CI からも `docs/testing.md` からも呼ばれていない（印の作り直しと `.dirty` の 7 本の Scenario を担保するのはこの台本だけ）
- 成果物: docs/testing.md
- 根拠: `.github/workflows/ci.yml`・`docs/testing.md` に名前が無い / evidence.jsonl は android 節だけ / tasks.md は CI への組み込みを求めていない（計画側の抜け）
- kind: technical
- 処置: （fix 待ち）

## R4. C-01 の組み立てが `ASHIATO_BUILD_COMMIT` を読まず git だけを見る（D5 は C-01 も「環境変数 → git → unknown」）
- 成果物: collector-android/app/build.gradle.kts
- 根拠: build.gradle.kts:222 / design.md D5（commit の出所・「C-01 と V-01 は同じ規則」）
- kind: technical
- 処置: （fix 待ち）

## R5. 試験の小さな穴（状態符号・負の側・空振り）
- 成果物: crates/server/src/component_version_tests.rs, crates/collector-windows/src/runtime.rs
- 根拠: `st49_ingest_server_version_is_build_stamp`・`st49_dedup_*` が状態符号と accepted を見ない（Task 3 F1/F3）/ 画面のログインで /drops・/media/missing に送った版が NULL になる試験が無い（Task 6 F3）/ `st49_c02_heartbeat_has_no_version` が `send()` の後に `unwrap_or_default` で読み空振りしうる（Task 8 F2）
- kind: technical
- 処置: （fix 待ち）

## R6. detekt の baseline に、この差分が生んだ import 順の違反（`LocationService.kt` の `KSerializer`）を足して隠している
- 成果物: collector-android/config/detekt-baseline.xml
- 根拠: Task 9 F1 / app/build.gradle.kts:173-174（新しい違反だけで落とす方針）
- kind: technical
- 処置: （fix 待ち）

## R7. export の新しい試験 5 本が `ashiato-export-file-*` を一時フォルダに残す
- 成果物: crates/server/src/export_component_version_tests.rs
- 根拠: evidence.jsonl の tmp_left / Task 7 F4（`HARNESS_TMP_LEAK=fail` に切り替えると落ちる）
- kind: technical
- 処置: （fix 待ち）

## R8. `tools/check-build-stamp.sh` の注記と決め打ち（windows 節は rlib を読むのに「bin を strings」、版番号を `"0."*` に決め打ち、android 節の失敗の理由が gradle の失敗にならない）
- 成果物: tools/check-build-stamp.sh
- 根拠: check-build-stamp.sh:5・583・android 節の `read <<<"$(android_stamps …)"` / Task 8 F1・F3・Task 9 F2
- kind: technical
- 処置: （fix 待ち）

## R9. `git status` が失敗すると印が「きれい」になる（Rust・web・gradle の 3 つとも）
- 成果物: crates/build-stamp/src/lib.rs, web/build-stamp.ts, collector-android/app/build.gradle.kts
- 根拠: Task 10 F1 / final reviewer は 3 つで揃っていることを確認
- kind: technical
- 処置: （fix 待ち）

## R10. 環境変数の commit の形を検査しない（`ci.yml:76` の `ci-<run id>` は D5 の正規表現に合わない印になる）
- 成果物: crates/build-stamp/src/lib.rs, web/build-stamp.ts
- 根拠: Task 10 F4 / `st49_stamp_env_commit_wins` / design.md D5（ST48 の D8（仮）と同じ順）
- kind: technical
- 処置: （fix 待ち）

## R11. `crates/server/build.rs` の `commit_of` が組んだ印を分解して commit を取り出す（環境変数の値が `.dirty` / `.debug` で終わると ST48 の台帳と食い違う）
- 成果物: crates/server/build.rs
- 根拠: build.rs:426-430 / Task 1 F2
- kind: technical
- 処置: （fix 待ち）

## R12. 滞在の「原文が変わったか」を Rust（`e.raw != raw`）と SQL（`raw IS DISTINCT FROM $2`）の 2 か所で判定している
- 成果物: crates/server/src/stay_store.rs
- 根拠: stay_store.rs:600・981 / Task 5 F1
- kind: technical
- 処置: （fix 待ち）

## R13. `DropLedger.openIndex` の版の違う下書きを閉じる loop は `load()` が先に閉じるので当たらない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: DropReport.kt:210 / Task 9 F4
- kind: technical
- 処置: （fix 待ち）
