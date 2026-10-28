# ST49 code review

## final review（d90ef3d..42ac63b）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 3 / Minor 13。
fix は 1 回（2b92476..d658b89）、scoped re-review 1 回で R1〜R8・R13 は全部 ADDRESSED、新しい Critical / Important なし。R9〜R12 は直さない（理由は各 R）。
Task ごとの ⚠️ は final reviewer が全体で確かめ、解消済み（Task 3 の `st49_mig_down_and_up_again`・clippy、Task 6 の D9、Task 8 の Scenario 名、Task 11 の本数ほか）か code-verify の席の担当（RED の証跡・`-- --list` の本数）に振り分けた。

## R1. `tools/check-openapi.sh` の `client_version` の除外が Task 9 の後も残り、注記（「Task 9 で外す」）と食い違う
- 成果物: tools/check-openapi.sh
- 根拠: check-openapi.sh:23-25（/ingest）・70-71（drops）/ Task 3 ⚠️・Task 6 F6 / `IngestRequest.kt` は `client_version` を既に持つ
- kind: technical
- 処置: fixed 9.2 —— /ingest・drops の両方で除外を外し注記を直した。`DropReportContractTest` の CONTRACT-FIELDS に `client_version` を足し素材を版つきの下書きにした（原文の鍵の比較からは除く）（2bfa158）。`./tools/check-openapi.sh` rc=0（破棄の報告の欄名 OK（12 欄））

## R2. `docs/handoff/ST29.md` に足した節が「衛星の資格情報はまだ無い・ST49 が先に入る」と書くが、ST29 は archive 済みで `Caller::Satellite => Internal` は既にある
- 成果物: docs/handoff/ST29.md
- 根拠: base d90ef3d の web_session.rs:45 / openspec/changes/archive/2026-10-27-st29-satellite-credentials / lib.rs:1337 / Task 11 F1（「どの版の欄にも書かない」とも読める）
- kind: technical
- 処置: fixed D9 —— 節を決定の記録に書き直した（衛星は `Writer::Internal` に振り分け済み。collector_version / web_version には書かず、server_version は S-01 が入れる）（26ae2b1）

## R3. `tools/check-build-stamp.sh` が CI からも `docs/testing.md` からも呼ばれていない（印の作り直しと `.dirty` の 7 本の Scenario を担保するのはこの台本だけ）
- 成果物: docs/testing.md
- 根拠: `.github/workflows/ci.yml`・`docs/testing.md` に名前が無い / evidence.jsonl は android 節だけ / tasks.md は CI への組み込みを求めていない（計画側の抜け）
- kind: technical
- 処置: fixed 1.2 —— `docs/testing.md` §1 の表に `tools/check-build-stamp.sh tool|server|windows|android` の行（担保する範囲・走らせ方・CI 未組み込みで重い・印の作り方を触ったら手元で走らせる）を足した（26ae2b1）。`tools/check-shared-appends.sh` rc=0。CI の job への組み込みは tasks.md の範囲外なので change に出す（PR 本文の未処置に挙げる）

## R4. C-01 の組み立てが `ASHIATO_BUILD_COMMIT` を読まず git だけを見る（D5 は C-01 も「環境変数 → git → unknown」）
- 成果物: collector-android/app/build.gradle.kts
- 根拠: build.gradle.kts:222 / design.md D5（commit の出所・「C-01 と V-01 は同じ規則」）
- kind: technical
- 処置: fixed D5 —— `providers.environmentVariable("ASHIATO_BUILD_COMMIT")`（trim・空でなければ）→ `git rev-parse HEAD` → `unknown` の順にした（e14a737）。`check-build-stamp.sh android` に環境変数ありと空の 2 場合を足し rc=0（d658b89）

## R5. 試験の小さな穴（状態符号・負の側・空振り）
- 成果物: crates/server/src/component_version_tests.rs, crates/collector-windows/src/runtime.rs
- 根拠: `st49_ingest_server_version_is_build_stamp`・`st49_dedup_*` が状態符号と accepted を見ない（Task 3 F1/F3）/ 画面のログインで /drops・/media/missing に送った版が NULL になる試験が無い（Task 6 F3）/ `st49_c02_heartbeat_has_no_version` が `send()` の後に `unwrap_or_default` で読み空振りしうる（Task 8 F2）
- kind: technical
- 処置: fixed 6.1 —— 3 本に `StatusCode::OK` と `accepted == true`、負の側 `st49_ledger_web_session_reports_drop_client_version`（`Writer::Web` を Collector 扱いに壊すと rc=101 で落ちることを確認）、heartbeat の試験は送れない transport で積んで `send()` の前に `unwrap()` で読む形に（b2f2311）。`cargo test -p ashiato-server st49_` 45 passed・`st49_c02_heartbeat_has_no_version` rc=0・clippy `-D warnings` rc=0

## R6. detekt の baseline に、この差分が生んだ import 順の違反（`LocationService.kt` の `KSerializer`）を足して隠している
- 成果物: collector-android/config/detekt-baseline.xml
- 根拠: Task 9 F1 / app/build.gradle.kts:173-174（新しい違反だけで落とす方針）
- kind: technical
- 処置: fixed 9.2 —— import を `android.util.Log` の直後へ移し baseline の 1 行を消した。組み直しで当たらなくなった `DropReportContractTest` の 2 行も消した（68f9703）。`:app:detekt` rc=0・単体 31 本 failed 0

## R7. export の新しい試験 5 本が `ashiato-export-file-*` を一時フォルダに残す
- 成果物: crates/server/src/export_component_version_tests.rs
- 根拠: evidence.jsonl の tmp_left / Task 7 F4（`HARNESS_TMP_LEAK=fail` に切り替えると落ちる）
- kind: technical
- 処置: fixed 7.1 —— 返した zip を包む `Exported`（Drop で `ashiato-export-file-` の置き場だけ消す）と `export_here` に 5 本を寄せた（8d2050d）。実行の後に新しい置き場は増えていない。共有の `export_once` を使うほかの試験の残りは ST49 より前からある（Task 11 F2）

## R8. `tools/check-build-stamp.sh` の注記と決め打ち（windows 節は rlib を読むのに「bin を strings」、版番号を `"0."*` に決め打ち、android 節の失敗の理由が gradle の失敗にならない）
- 成果物: tools/check-build-stamp.sh
- 根拠: check-build-stamp.sh:5・583・android 節の `read <<<"$(android_stamps …)"` / Task 8 F1・F3・Task 9 F2
- kind: technical
- 処置: fixed 1.2 —— 注記を rlib に、版番号は `Cargo.toml` の `[workspace.package]` version / `versionName` と比べ、式は版番号を逃がして組む。android 節は `got="$(android_stamps …)" || exit 1` で gradle の失敗の理由で止まる（d658b89）。4 節とも rc=0

## R9. `git status` が失敗すると印が「きれい」になる（Rust・web・gradle の 3 つとも）
- 成果物: crates/build-stamp/src/lib.rs, web/build-stamp.ts, collector-android/app/build.gradle.kts
- 根拠: Task 10 F1 / final reviewer は 3 つで揃っていることを確認
- kind: technical
- 処置: rejected: 3 つ（Rust・web・gradle）で揃った扱いで、`git status` だけが落ちて `rev-parse` が通る場面は実地で起きにくい。直すなら 3 つの印の意味を同時に変える（`.dirty` に倒すか `unknown` にするか）判断になり、この fix の範囲を越える。印は「その組み立ての申告」で、誤っていても記録の内容は失われない

## R10. 環境変数の commit の形を検査しない（`ci.yml:76` の `ci-<run id>` は D5 の正規表現に合わない印になる）
- 成果物: crates/build-stamp/src/lib.rs, web/build-stamp.ts
- 根拠: Task 10 F4 / `st49_stamp_env_commit_wins` / design.md D5（ST48 の D8（仮）と同じ順）
- kind: technical
- 処置: rejected: D5 は commit の出所を ST48 の D8（仮）と同じ順に定め、環境変数の値を形で検査しない（`st49_stamp_env_commit_wins` が固定）。`ci.yml` の `ci-<run id>` は CI の組み立てだけで配らない。形を検査するなら ST48 の台帳の `build_commit` と同時に変える必要がある

## R11. `crates/server/build.rs` の `commit_of` が組んだ印を分解して commit を取り出す（環境変数の値が `.dirty` / `.debug` で終わると ST48 の台帳と食い違う）
- 成果物: crates/server/build.rs
- 根拠: build.rs:426-430 / Task 1 F2
- kind: technical
- 処置: rejected: 環境変数に `.dirty` / `.debug` で終わる値や `+` を含む値を渡す運用は無い（CI は `ci-<run id>`、手元は git）。`ashiato-build-stamp` に commit を返す関数を公開すれば消える差で、実害が出たら直す

## R12. 滞在の「原文が変わったか」を Rust（`e.raw != raw`）と SQL（`raw IS DISTINCT FROM $2`）の 2 か所で判定している
- 成果物: crates/server/src/stay_store.rs
- 根拠: stay_store.rs:600・981 / Task 5 F1
- kind: technical
- 処置: rejected: いまは 2 か所の判定が一致し（`e.raw != raw` と `raw IS DISTINCT FROM $2` はどちらも原文の値の比較）、D2（仮）の試験（5.1）が両方の結果を見ている。bool を bind して 1 か所に寄せるのは整理で、振る舞いは変わらない

## R13. `DropLedger.openIndex` の版の違う下書きを閉じる loop は `load()` が先に閉じるので当たらない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: DropReport.kt:210 / Task 9 F4
- kind: technical
- 処置: fixed 9.3 —— loop を除き `indexOfFirst` の 1 行にした。版の違う下書きを閉じるのは `load()` だけ（a94db3e）。`*DropReport*`・`*DropLedger*`・`*St49*` 緑
