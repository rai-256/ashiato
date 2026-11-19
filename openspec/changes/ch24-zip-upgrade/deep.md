# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: 依存の zip を 2 から 9 に上げ、使う 4 か所を新しい API に合わせ、osv-scanner.toml の 3 件の例外を外す（上げきれなければ理由を事実に直す）だけ。本人の決定で振る舞いは変えない（書き出しの AES の zip が今と同じに読めることを試験で確かめる）。API・送信の契約・移行には触れない。新しい版が足す依存の許諾文は tools/licenses に置く
- 触れる予定のパス: crates/server/Cargo.toml / crates/collector-windows/Cargo.toml / Cargo.lock / crates/server/src/archive/open.rs / crates/server/src/export/dump.rs / crates/server/src/export/ledger.rs / crates/collector-windows/src/update_apply.rs / crates/server/src/archive_tests.rs / crates/server/src/archive_flow_tests.rs / crates/server/src/media_export_tests.rs / crates/server/src/export_job_tests.rs / crates/server/src/export_tests.rs / crates/server/src/export_file_tests.rs / crates/collector-windows/tests/st58_pc_update.rs / osv-scanner.toml / tools/licenses/** / docs/changes/CH24.md
