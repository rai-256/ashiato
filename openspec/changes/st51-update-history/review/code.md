# ST51 code review

## final review（f0b45f3..3331639）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 2 / Minor 13。
reviewer が手元で走らせたもの: `cargo test -p ashiato-server --lib` 1760 passed / 0 failed、`:app:testDebugUnitTest`（St49OutboxTest 7 本・St51HeartbeatVersionTest 3 本）BUILD SUCCESSFUL、
`openspec validate st51-update-history --strict` valid、`HX_CHANGE=st51-update-history python3 scripts/check_scenarios.py .` rc=0。
Task ごとの ⚠️ は final reviewer が全体で確かめ、全部 VERIFIED-OK（Task 9 の D14: `coverage*.rs` / `outage*.rs` に `collector_version` / `update_history` の語は 0 件、
Task 8 の ST49 の Scenario: ADDED（改名）+ REMOVED で外し ST49 の試験も消してある、Task 4 の `St51HeartbeatVersionTest` は実在し送った body の `client_version` を見る、
Task 2 の失敗の作り方と `record_startup` の配線（`lib.rs:4201`）、Task 1/6 のビューの試験、印の名前）。
並走の change で `device-collection` / `desktop-collection` を触るのは st50-contract-version だけで、改名した Requirement には触れない（grep で確認）。

## R1. `st51_coverage_outage_still_shown` が Scenario の GIVEN（途切れの間に更新の履歴の行と版の変化がある）を作れていない
- 成果物: crates/server/src/version_change_tests.rs
- 根拠: version_change_tests.rs:717 付近 —— `core.update_history` の行を `at` を指定せずに INSERT（`DEFAULT now()` = 実時刻で途切れの区間の外）、版の変化も区間の始まりの点 / Task 9 F1（未修正）
- kind: technical

## R2. `heartbeatKeepsVersionFromAddTime` の「入れ替わった後の送り手」が v2 を使っておらず、`assertTrue(v2 != v1)` が何も確かめていない
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St51HeartbeatVersionTest.kt
- 根拠: St51HeartbeatVersionTest.kt:244 付近 `Outbox.inDir(dir, HeartbeatRequest.serializer()) {}` / Task 8 F1（未修正）
- kind: technical

## R3. ST58 への申し送りの「表がまだ無い回は試みも結果も書かず警告だけ」が実装と合わない（実装は表を作った後に結果の行を書く）
- 成果物: docs/handoff/ST58.md
- 根拠: docs/handoff/ST58.md:27 / `st51_bin_migrate_writes_compiled_stamp`（applied の 1 行）/ spec の Scenario「警告を出し、結果の 1 行だけを残す」
- kind: technical

## R4. 警告の種別 `update_history_unwritten` が 2 か所に直書きで、試験は種別でなく `#[cfg(test)]` のスレッドローカルの理由の語を見ている
- 成果物: crates/server/src/update_history.rs
- 根拠: update_history.rs:470 / lib.rs:4201（`pool.acquire()` 失敗の分岐は `why` も持たない）/ Task 2 F1
- kind: technical

## R5. migrate の失敗の 1 行を `tracing::error!` と `eprintln!` の両方に出している
- 成果物: crates/server/src/lib.rs
- 根拠: lib.rs:126-129 / Task 2 F2
- kind: technical

## R6. design D15 の関数の形（`record_migrate(conn, stamp, outcome)`・`record_startup(conn, stamp)`）と実装（`attempt`・`versions` の引数、`apply_migrations_recorded` の新設）がずれている
- 成果物: openspec/changes/st51-update-history/design.md
- 根拠: design.md D15 / update_history.rs:637 / Task 2 F3
- kind: technical

## R7. Rust の語「`None`」が契約の説明文（docs/openapi.json）に漏れている
- 成果物: crates/server/src/heartbeat.rs
- 根拠: heartbeat.rs:33・247 → docs/openapi.json:3591 / Task 4 F2
- kind: technical

## R8. ST48 の判定の緩め方が粗い（`kind=` の行が `error: kind=migrate_failed ` で始まれば、後ろに `db_ahead` などが付いても通る）
- 成果物: crates/server/tests/server_startup.rs
- 根拠: server_startup.rs:817-823 / Task 3 F2
- kind: technical

## R9. `st51_bin_startup_row_on_swap` が `h.len() == 3` と前置きの中身に決め打ちしている
- 成果物: crates/server/tests/server_startup.rs
- 根拠: server_startup.rs:1306 / Task 3 F1
- kind: technical

## R10. `st51_coverage_same_with_or_without_version` の `/outages` が空どうしの比較で、途切れの判定の側を見ていない
- 成果物: crates/server/src/version_change_tests.rs
- 根拠: version_change_tests.rs:690・729 / Task 9 F2・F3
- kind: technical

## R11. `segmentOutbox` の KDoc「stamp を渡した置き場だけが版を書く」が古い（生存信号も emitter を通して版を持つ）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: LocationService.kt:324 / Task 8 F2
- kind: technical

## R12. `file_note` がファイル名の文字列で分岐し、`VIEW_FILES` / `CATALOG` と名前を二重に持つ
- 成果物: crates/server/src/export/doc.rs
- 根拠: export/doc.rs:54 / Task 6 F1
- kind: technical

## R13. 所有者の TRUNCATE が「何かの DB の誤り」しか見ず、アプリの役割の INSERT は件数の増加だけで足した行を読み戻していない
- 成果物: crates/server/src/update_history_tests.rs
- 根拠: update_history_tests.rs:257・292 / Task 1 F1・F2
- kind: technical

## R14. 同じ印で版の集合だけが違う組み立て（`.dirty` の開発ビルドで移行を足した場合）は行を残さない
- 成果物: crates/server/src/update_history.rs
- 根拠: design.md D16（試みの行も結果の行も「版が違うとき」だけ）
- kind: technical

## R15. agreed-test-changes.md に ST49 の試験の削除と改名（`heartbeatHasNoVersion` → `recordOutboxCarriesStamp`、`st49_c02_heartbeat_has_no_version` の削除）が無い
- 成果物: openspec/changes/st51-update-history/agreed-test-changes.md
- 根拠: tasks.md の Global Constraints（ST49 の試験を消して置き換える）/ git diff の削除
- kind: technical
