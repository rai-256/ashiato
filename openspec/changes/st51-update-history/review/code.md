# ST51 code review

## final review（f0b45f3..3331639）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 2 / Minor 13。
reviewer が手元で走らせたもの: `cargo test -p ashiato-server --lib` 1760 passed / 0 failed、`:app:testDebugUnitTest`（St49OutboxTest 7 本・St51HeartbeatVersionTest 3 本）BUILD SUCCESSFUL、
`openspec validate st51-update-history --strict` valid、`HX_CHANGE=st51-update-history python3 scripts/check_scenarios.py .` rc=0。
fix は 1 回（9dac25a..05bb619。指摘の外に `cargo fmt --check` を通す整形 4941dd8 を 1 つ）、scoped re-review 1 回で R1〜R4・R6〜R13 は全部 ADDRESSED、新しい Critical / Important なし。R5・R14・R15 は直さない（理由は各 R）。
Task ごとの ⚠️ は final reviewer が全体で確かめ、全部 VERIFIED-OK（Task 9 の D14: `coverage*.rs` / `outage*.rs` に `collector_version` / `update_history` の語は 0 件、
Task 8 の ST49 の Scenario: ADDED（改名）+ REMOVED で外し ST49 の試験も消してある、Task 4 の `St51HeartbeatVersionTest` は実在し送った body の `client_version` を見る、
Task 2 の失敗の作り方と `record_startup` の配線（`lib.rs:4201`）、Task 1/6 のビューの試験、印の名前）。
並走の change で `device-collection` / `desktop-collection` を触るのは st50-contract-version だけで、改名した Requirement には触れない（grep で確認）。

## R1. `st51_coverage_outage_still_shown` が Scenario の GIVEN（途切れの間に更新の履歴の行と版の変化がある）を作れていない
- 成果物: crates/server/src/version_change_tests.rs
- 根拠: version_change_tests.rs:717 付近 —— `core.update_history` の行を `at` を指定せずに INSERT（`DEFAULT now()` = 実時刻で途切れの区間の外）、版の変化も区間の始まりの点 / Task 9 F1（未修正）
- kind: technical
- 処置: fixed 9.1 —— 更新の履歴の行の at を 2026-10-19T06:00Z に明示し、版の変化は同じ端末の別の C-01 ソースの 10-19 12:00 の v2 で区間の中に作る（位置のソースを区間に置くと途切れが終わるため）。区間の中にあることと last_alive_at を assert（ed415bd8）

## R2. `heartbeatKeepsVersionFromAddTime` の「入れ替わった後の送り手」が v2 を使っておらず、`assertTrue(v2 != v1)` が何も確かめていない
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St51HeartbeatVersionTest.kt
- 根拠: St51HeartbeatVersionTest.kt:244 付近 `Outbox.inDir(dir, HeartbeatRequest.serializer()) {}` / Task 8 F1（未修正）
- kind: technical
- 処置: fixed 8.2 —— 入れ替わった後の置き場を `Outbox.inDir(dir, …, v2)` にし、送った body が v1 のままを見る。空の assert は削除。読み出し時に v2 を付ける形に壊すと落ちることを確認（872a4d59）

## R3. ST58 への申し送りの「表がまだ無い回は試みも結果も書かず警告だけ」が実装と合わない（実装は表を作った後に結果の行を書く）
- 成果物: docs/handoff/ST58.md
- 根拠: docs/handoff/ST58.md:27 / `st51_bin_migrate_writes_compiled_stamp`（applied の 1 行）/ spec の Scenario「警告を出し、結果の 1 行だけを残す」
- kind: technical
- 処置: fixed 10.1 —— 初回は試みの行を書かず警告、表を作った後は結果の行（applied / failed、from は空）を書き、その回の退避には表が無いので戻すと消える、に書き直した（e1f56c53）

## R4. 警告の種別 `update_history_unwritten` が 2 か所に直書きで、試験は種別でなく `#[cfg(test)]` のスレッドローカルの理由の語を見ている
- 成果物: crates/server/src/update_history.rs
- 根拠: update_history.rs:470 / lib.rs:4201（`pool.acquire()` 失敗の分岐は `why` も持たない）/ Task 2 F1
- kind: technical
- 処置: fixed 2.1 —— `update_history::UNWRITTEN_KIND` に集約し、lib.rs の acquire 失敗も `warn_unwritten("acquire", …)` を通す。試験が種別そのものを取り込む形は並走の試験との取り合いで見送り、種別を取り違える経路は構造でなくした（6d0e12c8）

## R5. migrate の失敗の 1 行を `tracing::error!` と `eprintln!` の両方に出している
- 成果物: crates/server/src/lib.rs
- 根拠: lib.rs:126-129 / Task 2 F2
- kind: technical
- 処置: rejected: 重複ではない。`run_migrate` の `tracing_subscriber::fmt()` は writer 未指定で標準出力、`eprintln!` は標準エラーで、それぞれ 1 行。ST48 の `mismatch_report` と同じ形（final reviewer が確認、re-review も R8 の判定で同じ読み）

## R6. design D15 の関数の形（`record_migrate(conn, stamp, outcome)`・`record_startup(conn, stamp)`）と実装（`attempt`・`versions` の引数、`apply_migrations_recorded` の新設）がずれている
- 成果物: openspec/changes/st51-update-history/design.md
- 根拠: design.md D15 / update_history.rs:637 / Task 2 F3
- kind: technical
- 処置: fixed D15 —— 決定は変えず、`record_attempt` / `record_migrate(…, attempt, outcome)` / `record_startup(…, versions)` / `apply_migrations_recorded(…, stamp: Option<&str>)` の形と理由（錠を持つ同じ接続で書く）を書いた（9bb9e1ee）

## R7. Rust の語「`None`」が契約の説明文（docs/openapi.json）に漏れている
- 成果物: crates/server/src/heartbeat.rs
- 根拠: heartbeat.rs:33・247 → docs/openapi.json:3591 / Task 4 F2
- kind: technical
- 処置: fixed 4.1 —— 「欄の無い要求は『不明』」に直し docs/openapi.json を再生成。`./tools/check-openapi.sh` rc=0（81cf6c12）。ST09 由来の `network` の同じ書き方は範囲外

## R8. ST48 の判定の緩め方が粗い（`kind=` の行が `error: kind=migrate_failed ` で始まれば、後ろに `db_ahead` などが付いても通る）
- 成果物: crates/server/tests/server_startup.rs
- 根拠: server_startup.rs:817-823 / Task 3 F2
- kind: technical
- 処置: fixed 2.3 —— 食い違いの種別が標準エラーに無いこと・`kind=` の行が 1 本・語の並びが `failure_line` の形に限られることを見る（43edf7dd）

## R9. `st51_bin_startup_row_on_swap` が `h.len() == 3` と前置きの中身に決め打ちしている
- 成果物: crates/server/tests/server_startup.rs
- 根拠: server_startup.rs:1306 / Task 3 F1
- kind: technical
- 処置: fixed 3.2 —— 起動の前の行を取り、`before + 1` と前の行が変わらないことで比べる（43edf7dd）

## R10. `st51_coverage_same_with_or_without_version` の `/outages` が空どうしの比較で、途切れの判定の側を見ていない
- 成果物: crates/server/src/version_change_tests.rs
- 根拠: version_change_tests.rs:690・729 / Task 9 F2・F3
- kind: technical
- 処置: fixed 9.1 —— 入力を 10-19 03:00 / 09:00 にして 27 時間の途切れを出し、位置のソースの途切れ 1 件と last_alive_at 09:00 を見る。版の有無の比較はそのまま（ed415bd8）

## R11. `segmentOutbox` の KDoc「stamp を渡した置き場だけが版を書く」が古い（生存信号も emitter を通して版を持つ）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: LocationService.kt:324 / Task 8 F2
- kind: technical
- 処置: fixed 8.1 —— 生存信号の置き場は stamp を渡さず `HeartbeatEmitter` が積むときに版を入れる、を KDoc に足した（167d31f1）

## R12. `file_note` がファイル名の文字列で分岐し、`VIEW_FILES` / `CATALOG` と名前を二重に持つ
- 成果物: crates/server/src/export/doc.rs
- 根拠: export/doc.rs:54 / Task 6 F1
- kind: technical
- 処置: fixed 6.1 —— `export::UPDATE_HISTORY_FILE` / `COLLECTOR_VERSION_CHANGES_FILE` を置き、CATALOG・VIEW_FILES・`file_note`・dump.rs の並べ方が同じ定数を引く。説明の文を項目の欄へ移す形は CATALOG の 17 項目すべてに欄が要り並走の Story と衝突するので見送り（05bb6190）

## R13. 所有者の TRUNCATE が「何かの DB の誤り」しか見ず、アプリの役割の INSERT は件数の増加だけで足した行を読み戻していない
- 成果物: crates/server/src/update_history_tests.rs
- 根拠: update_history_tests.rs:257・292 / Task 1 F1・F2
- kind: technical
- 処置: fixed 1.2 —— 所有者の TRUNCATE は門の文言と P0001、アプリの役割は 42501 を見る。INSERT は `RETURNING seq` の行をアプリの役割で読み戻す（46aaf094）

## R14. 同じ印で版の集合だけが違う組み立て（`.dirty` の開発ビルドで移行を足した場合）は行を残さない
- 成果物: crates/server/src/update_history.rs
- 根拠: design.md D16（試みの行も結果の行も「版が違うとき」だけ）
- kind: technical
- 処置: rejected: design D16（試みも結果も「版が違うとき」だけ書く）どおりの帰結で、同じ印で版の集合だけ違う組み立ては `.dirty` の開発ビルドでしか起きない。記録にとどめる

## R15. agreed-test-changes.md に ST49 の試験の削除と改名（`heartbeatHasNoVersion` → `recordOutboxCarriesStamp`、`st49_c02_heartbeat_has_no_version` の削除）が無い
- 成果物: openspec/changes/st51-update-history/agreed-test-changes.md
- 根拠: tasks.md の Global Constraints（ST49 の試験を消して置き換える）/ git diff の削除
- kind: technical
- 処置: rejected: 一覧に書かなくても通る。ST49 の Scenario は change の spec の `## REMOVED Requirements` で名指しされており、`check_agreed_tests.py` はそれを合意した変更として通す（agreed-test-changes.md は合意の外の書き換えのための一覧）
