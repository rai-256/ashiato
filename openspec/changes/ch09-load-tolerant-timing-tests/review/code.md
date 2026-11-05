# CH09 code review

## final review（11c0c04..13c361f）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 7 / Minor 5（＋各 Task の残りの Minor）。判定は「With fixes」。
reviewer が手元で走らせたもの（どれも rc=0）: `check-product-unchanged.sh`、`check_scenarios.py`、`check_agreed_tests.py`、`check-inventory.sh`（`--self-test` を含む）、`check-test-names.sh`、壊す差分 2 つの `git apply --check`。
製品のコードは 0 行（Q0）、1 の試験・2 の試験の書き直し、直す前の再現の順序、台本の後始末、Scenario の印と本数は確かめて問題なし。
Task ごとの ⚠️ は final reviewer が全体で確かめた。解消: T1（scan が落ちた理由・before の順序）、T2（差分が HEAD に当たる）、T3、T4 の正典の文、T5 の行番号、T6、T8、T9、T13 の load=db と 13.2（reviewer の実行で PASS）。
残るもの: T5 / T11 / T12 F14-F16 → R7、T7 → R6、T10 の SETTLE_MS → R2、T12 F1 / F13 → R2、T1 F8 → R8、T2 F3 / F6 → R9、T13 F9 → R12、T7 F5・T8 F2 → R13。
ここに R を立てない Task の Minor（字面・コメント・未使用の変数・報告の記述）は `task-*-findings.md` に残し、merge を止めない。

## R1. `check-inventory.sh` は `rg` が無いと 0 ファイルを一覧済みとして rc=0 で通る
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh
- 根拠: check-inventory.sh:10-26。候補の関数が `rg -l … || true` で command not found も握りつぶす。このマシンの `rg` は Claude Code の shell 関数で、`env -i PATH=/usr/bin:/bin:/usr/local/bin bash -c 'command -v rg'` は rc=1。reviewer の実行で `inventory: 0 ファイルを一覧済み` rc=0。`evidence.jsonl` に 12.1 / 13.2 の記録が無い
- kind: technical
- 提案: 先頭で `command -v rg` が無ければ rc=1（あるいは `grep -rl` に置き換えて依存を外す）。候補が 0 件なら rc=1。`--self-test` に「道具なし」「候補 0 件」の例
- 処置:

## R2. `check-inventory.sh` が実在しないパス・ディレクトリの行を黙って飛ばす
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh
- 根拠: check-inventory.sh:144 `[ -f "$file" ] || continue`。`condition_files` が返す `observe_until`（Task 12 F13）、`collector-android/app/src/androidTest`（`SETTLE_MS` の唯一の行。Task 10 ⚠️）が検査されない。Task 12 F1「実在しないパスは rc=1」が直っていない
- kind: technical
- 提案: 解決できない行は名指しして rc=1。ディレクトリの行はファイルに解決する
- 処置:

## R3. `check-inventory.sh` の Rust / Kotlin の拾い方が狭く、取り残しが PASS になる
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh
- 根拠: check-inventory.sh:66-85。Rust は `for _ in 0..N` の後ろ 5 行しか眠りを探さず、`sleep(POLL)` のような定数の眠りの回数×時間を計算しない。Kotlin は `await(N, TimeUnit…)` だけで、spec の THEN が名指す `_MS` 定数・`device.wait(…, N)`・`toNanos(N)` を見ない。結果として R4〜R6 が検査をすり抜けた
- kind: technical
- 提案: 上の形を拾い、`--self-test` に違反の例（5 行を超える眠り・`sleep(POLL)`・`_MS` 定数）を足す。直した検査を HEAD で走らせ、R4〜R6 を名指すことを確かめる
- 処置:

## R4. `scheduled_tests.rs` の実時間 1 秒の条件待ちに誤った `timing-ok` の理由が付いている
- 成果物: crates/server/src/scheduled_tests.rs
- 根拠: scheduled_tests.rs:587-593。`handle.is_finished()` を 40×25ms（`pause()` しない＝実時間 1 秒）待ってから `assert!(finished)`。上限 1 秒の条件待ちで、CH09 が無くす形そのもの。印の理由「短い下限で、条件待ちの上限ではない」は事実と違う
- kind: technical
- 提案: D4 を当てて実時間 30 秒の期限の条件待ちにする（`probe.calls()==0` の assert があるので検出力は落ちない）
- 処置:

## R5. D4 の対象なのに 30 秒未満の実時間の期限が 3 か所残っている
- 成果物: crates/server/src/archive_flow_tests.rs / crates/server/src/purge_compaction_tests.rs / crates/server/src/testdb.rs
- 根拠: archive_flow_tests.rs:6028（400×50ms＝20 秒でログの行を待つ。R3 の 5 行の窓ですり抜け）/ purge_compaction_tests.rs:240（500×POLL(20ms)＝10 秒の後に `assert!(waiting())`。`pause()` なし。一覧は「維持」と誤記）/ testdb.rs:298（`CREATE DATABASE` の 55006 を 100×50ms＝5 秒で打ち直す。`testdb.rs` は lib.rs:262 で `#[cfg(test)]` なのに一覧は「製品実装なので一覧化のみ」と誤分類）
- kind: technical
- 提案: 3 か所とも D4（実時間の期限で一律 30 秒、条件の待ちのまま）を当て、inventory.md の分類を直す
- 処置:

## R6. 携帯の単体試験の打ち切り 2 か所に D6 が当たっていない（Task 7 ⚠️）
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthNotificationTest.kt / FeelingReminderReceiverTest.kt
- 根拠: HealthNotificationTest.kt:156（`tookMs < 10_000`）、FeelingReminderReceiverTest.kt:371（相手の `timeoutMs = 1_000` に対して `tookMs < 3_000`）。`time_sync.rs` / `testdb_tests.rs` と同じ型なのに延ばしていない。一覧は「暗黙の上限 / 条件待ち」と誤分類
- kind: technical
- 提案: D6（計算の形は直さず、打ち切りは相手と上限を一緒に延ばす）を当てる。製品の値（`timeoutMs` の既定など）は変えない（Q0）—— 試験が渡す値で延ばせないなら、一覧に理由つきで残す
- 処置:

## R7. `inventory.md` が自分自身ともコードとも食い違う
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/inventory.md
- 根拠: 40-41 行は `runtime.rs:3612` / `:1936` を「待たない / 直さない」「経過比較を維持」と書くが書き換え済み。70・75・76 行は `time_sync.rs:650`・`collect.rs:868`・`runtime.rs:2499/:2512` を「製品実装なので一覧化のみ / Q0 により変更しない」と書くが、どれも直した試験のコード。`tools/check-panic-log.sh`（直した）と `tools/load-repeat.sh` を「速さが要件 / 一覧化」。106 行に `satellites.test.tsxundefined` の混入（Task 12 F14-F16）
- kind: technical
- 提案: 重複した表を 1 つにまとめ、各行の分類をコードと一致させる（D14 の一覧は PR 本文と人間に届く）
- 処置:

## R8. `load-repeat.sh` の DB 負荷の立ち上がり確認が自分自身の argv に当たり、負荷 0 本でも通る
- 成果物: tools/load-repeat.sh
- 根拠: load-repeat.sh:81 の `index($0, marker)` が自分の `sh -c` の argv に一致する。`stop_db_load` も自分の sh を kill の対象に拾う（Task 1 F8）
- kind: technical
- 提案: 自分の pid（と親）を除外するか、marker を含まない形で数える
- 処置:

## R9. `break-check.sh` の `CARGO_TARGET_DIR` の既定が元の木の `target/` で、壊した木の組み立てが元の木を汚す
- 成果物: tools/break-check.sh
- 根拠: break-check.sh:87。Task 4 で元の木の `cargo test` が rc=101 になった実測あり（`break-proof.sh` は回避済みだが道具の既定が罠）。ほかに rc 137 を「文言が違う」と取り違える（Task 2 F3）、出力を流さない（F6）
- kind: technical
- 提案: 既定を作業木ごとの target にする（あるいは `$TMPDIR` 下の専用 target）。rc 137 / 124 は別の文言で落とす
- 処置:

## R10. `break-proof.sh history` の最後の PASS が、その後の `runtime.rs` / `collect.rs` の変更より前
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/break-proof.sh / evidence.jsonl
- 根拠: 最後の PASS は 58d760420、その後 9d4266521 / c3e5f3550 で補助関数が変わった。`git apply --check` は通る
- kind: technical
- 提案: fix の後の HEAD で `break-proof.sh history` を走らせ直し、証跡を残す
- 処置:

## R11. 時刻を兼ねるループに D14 の `// timing-ok:` が無い
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: runtime.rs:4232 の `t(sec)` のループ（Q5 で回数のまま残す形）に理由の印が無い
- kind: technical
- 提案: `// timing-ok: <理由>` を付ける
- 処置:

## R12. `load-proof.sh` が両方の負荷で再現した記録（`host,db`）を扱えない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/load-proof.sh
- 根拠: load-proof.sh:76-79。`--load host,db` が「未知の負荷」で落ちる（Task 13 F9。いまの `load-before.txt` では実害なし）
- kind: technical
- 提案: カンマ区切りを受けて各負荷で走らせる
- 処置:

## R13. 待ちの補助が期限を過ぎても黙って返る / pool の中の AssertionError が飲み込まれる
- 成果物: crates/collector-windows/tests/runtime_windows.rs / collector-android/app/src/test/…（Task 7 の pool を使う試験）
- 根拠: Task 8 F2（`observe_until` / `wait_history` / clock_skew の while が期限後に黙って返る）、Task 7 F5（pool の中の AssertionError が飲み込まれる）
- kind: technical
- 提案: 期限切れは panic / 失敗にし、pool の失敗は呼び出し側へ伝える
- 処置:
