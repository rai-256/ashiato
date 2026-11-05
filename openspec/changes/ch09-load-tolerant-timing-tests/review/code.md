# CH09 code review

## final review（11c0c04..13c361f）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 7 / Minor 5（＋各 Task の残りの Minor）。判定は「With fixes」。
reviewer が手元で走らせたもの（どれも rc=0）: `check-product-unchanged.sh`、`check_scenarios.py`、`check_agreed_tests.py`、`check-inventory.sh`（`--self-test` を含む）、`check-test-names.sh`、壊す差分 2 つの `git apply --check`。
製品のコードは 0 行（Q0）、1 の試験・2 の試験の書き直し、直す前の再現の順序、台本の後始末、Scenario の印と本数は確かめて問題なし。
Task ごとの ⚠️ は final reviewer が全体で確かめた。解消: T1（scan が落ちた理由・before の順序）、T2（差分が HEAD に当たる）、T3、T4 の正典の文、T5 の行番号、T6、T8、T9、T13 の load=db と 13.2（reviewer の実行で PASS）。
残るもの: T5 / T11 / T12 F14-F16 → R7、T7 → R6、T10 の SETTLE_MS → R2、T12 F1 / F13 → R2、T1 F8 → R8、T2 F3 / F6 → R9、T13 F9 → R12、T7 F5・T8 F2 → R13。
fix は 1 回（95932fc..e34dab8、6 commit）。fix の担当が走らせたもの（rc=0）: check-inventory.sh（--self-test・本走行 166 件・PATH を絞っても）、cargo test の触った試験（scheduled / archive_flow / purge_compaction 各 1・testdb_tests 16・collector-windows 1。`--list` で 1 本以上）、gradle の 3 クラス、clippy -D warnings（linux と windows-gnu）・fmt、break-check / load-repeat の --self-test、verify-run 3.2、check-product-unchanged・check_scenarios・check-test-names・check_agreed_tests。走らせていない: Windows の実行時テスト・計測テスト・e2e・break-proof scan・load-proof before/after・smoke・cargo test --workspace。
scoped re-review 1 回で R1〜R13 は全部 ADDRESSED、新しい Critical / Important なし（Minor 2: Rust の検査が `mod tests` の字下げの形に依る・runtime_windows.rs の fmt の整形が混じる。どちらも実害なし）。
ここに R を立てない Task の Minor（字面・コメント・未使用の変数・報告の記述）は `task-*-findings.md` に残し、merge を止めない。

## R1. `check-inventory.sh` は `rg` が無いと 0 ファイルを一覧済みとして rc=0 で通る
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh
- 根拠: check-inventory.sh:10-26。候補の関数が `rg -l … || true` で command not found も握りつぶす。このマシンの `rg` は Claude Code の shell 関数で、`env -i PATH=/usr/bin:/bin:/usr/local/bin bash -c 'command -v rg'` は rc=1。reviewer の実行で `inventory: 0 ファイルを一覧済み` rc=0。`evidence.jsonl` に 12.1 / 13.2 の記録が無い
- kind: technical
- 提案: 先頭で `command -v rg` が無ければ rc=1（あるいは `grep -rl` に置き換えて依存を外す）。候補が 0 件なら rc=1。`--self-test` に「道具なし」「候補 0 件」の例
- 処置: fixed 12.1 —— rg をやめ grep -rlE に。道具が無い・grep rc≥2・候補 0 件は名指して rc=1、--self-test に例。env -i PATH=/usr/bin:/bin で 166 件 rc=0（a87db3a75）

## R2. `check-inventory.sh` が実在しないパス・ディレクトリの行を黙って飛ばす
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh
- 根拠: check-inventory.sh:144 `[ -f "$file" ] || continue`。`condition_files` が返す `observe_until`（Task 12 F13）、`collector-android/app/src/androidTest`（`SETTLE_MS` の唯一の行。Task 10 ⚠️）が検査されない。Task 12 F1「実在しないパスは rc=1」が直っていない
- kind: technical
- 提案: 解決できない行は名指しして rc=1。ディレクトリの行はファイルに解決する
- 処置: fixed 12.1 —— パスの無い行・解決できない行は行番号つきで rc=1、ディレクトリと * はファイルへ展開（a87db3a75）

## R3. `check-inventory.sh` の Rust / Kotlin の拾い方が狭く、取り残しが PASS になる
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh
- 根拠: check-inventory.sh:66-85。Rust は `for _ in 0..N` の後ろ 5 行しか眠りを探さず、`sleep(POLL)` のような定数の眠りの回数×時間を計算しない。Kotlin は `await(N, TimeUnit…)` だけで、spec の THEN が名指す `_MS` 定数・`device.wait(…, N)`・`toNanos(N)` を見ない。結果として R4〜R6 が検査をすり抜けた
- kind: technical
- 提案: 上の形を拾い、`--self-test` に違反の例（5 行を超える眠り・`sleep(POLL)`・`_MS` 定数）を足す。直した検査を HEAD で走らせ、R4〜R6 を名指すことを確かめる
- 処置: fixed 12.1 —— Rust は for の本体を波括弧の対応で読み回数×眠り（定数を解決）<30 秒で落とす、Kotlin は _MS 定数・device.wait・toNanos・tookMs < N も見る。95932fc に当てると R4/R5/R6/R11 の箇所を名指して rc=1（a87db3a75）

## R4. `scheduled_tests.rs` の実時間 1 秒の条件待ちに誤った `timing-ok` の理由が付いている
- 成果物: crates/server/src/scheduled_tests.rs
- 根拠: scheduled_tests.rs:587-593。`handle.is_finished()` を 40×25ms（`pause()` しない＝実時間 1 秒）待ってから `assert!(finished)`。上限 1 秒の条件待ちで、CH09 が無くす形そのもの。印の理由「短い下限で、条件待ちの上限ではない」は事実と違う
- kind: technical
- 提案: D4 を当てて実時間 30 秒の期限の条件待ちにする（`probe.calls()==0` の assert があるので検出力は落ちない）
- 処置: fixed 5.1 —— 誤った timing-ok を外し実時間 30 秒の期限の条件待ちに（20f57f13a）

## R5. D4 の対象なのに 30 秒未満の実時間の期限が 3 か所残っている
- 成果物: crates/server/src/archive_flow_tests.rs / crates/server/src/purge_compaction_tests.rs / crates/server/src/testdb.rs
- 根拠: archive_flow_tests.rs:6028（400×50ms＝20 秒でログの行を待つ。R3 の 5 行の窓ですり抜け）/ purge_compaction_tests.rs:240（500×POLL(20ms)＝10 秒の後に `assert!(waiting())`。`pause()` なし。一覧は「維持」と誤記）/ testdb.rs:298（`CREATE DATABASE` の 55006 を 100×50ms＝5 秒で打ち直す。`testdb.rs` は lib.rs:262 で `#[cfg(test)]` なのに一覧は「製品実装なので一覧化のみ」と誤分類）
- kind: technical
- 提案: 3 か所とも D4（実時間の期限で一律 30 秒、条件の待ちのまま）を当て、inventory.md の分類を直す
- 処置: fixed 5.1 —— 3 か所とも実時間 30 秒の期限の条件待ちに、期限切れは待った時間と最後の値つき。範囲外で TestDb::drop の DROP のやり直しも 30 秒に（20f57f13a）

## R6. 携帯の単体試験の打ち切り 2 か所に D6 が当たっていない（Task 7 ⚠️）
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/HealthNotificationTest.kt / FeelingReminderReceiverTest.kt
- 根拠: HealthNotificationTest.kt:156（`tookMs < 10_000`）、FeelingReminderReceiverTest.kt:371（相手の `timeoutMs = 1_000` に対して `tookMs < 3_000`）。`time_sync.rs` / `testdb_tests.rs` と同じ型なのに延ばしていない。一覧は「暗黙の上限 / 条件待ち」と誤分類
- kind: technical
- 提案: D6（計算の形は直さず、打ち切りは相手と上限を一緒に延ばす）を当てる。製品の値（`timeoutMs` の既定など）は変えない（Q0）—— 試験が渡す値で延ばせないなら、一覧に理由つきで残す
- 処置: fixed 7.1 —— FeelingReminderReceiverTest は相手を 60 秒に延ばし上限 30 秒（timeoutMs=1_000 は不変）、HealthNotificationTest は相手が返らないので上限 30 秒（ded2d53e1）

## R7. `inventory.md` が自分自身ともコードとも食い違う
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/inventory.md
- 根拠: 40-41 行は `runtime.rs:3612` / `:1936` を「待たない / 直さない」「経過比較を維持」と書くが書き換え済み。70・75・76 行は `time_sync.rs:650`・`collect.rs:868`・`runtime.rs:2499/:2512` を「製品実装なので一覧化のみ / Q0 により変更しない」と書くが、どれも直した試験のコード。`tools/check-panic-log.sh`（直した）と `tools/load-repeat.sh` を「速さが要件 / 一覧化」。106 行に `satellites.test.tsxundefined` の混入（Task 12 F14-F16）
- kind: technical
- 提案: 重複した表を 1 つにまとめ、各行の分類をコードと一致させる（D14 の一覧は PR 本文と人間に届く）
- 処置: fixed 12.1 —— 表を 1 つにまとめ、行番号と分類を HEAD のコードに合わせた。undefined の混入を消した（a87db3a75）

## R8. `load-repeat.sh` の DB 負荷の立ち上がり確認が自分自身の argv に当たり、負荷 0 本でも通る
- 成果物: tools/load-repeat.sh
- 根拠: load-repeat.sh:81 の `index($0, marker)` が自分の `sh -c` の argv に一致する。`stop_db_load` も自分の sh を kill の対象に拾う（Task 1 F8）
- kind: technical
- 提案: 自分の pid（と親）を除外するか、marker を含まない形で数える
- 処置: fixed 1.1 —— argv 末尾の <印>-<番号> だけで数え・止める。立ち上がりは nproc 本そろうまで待つ（b16a74dd7）

## R9. `break-check.sh` の `CARGO_TARGET_DIR` の既定が元の木の `target/` で、壊した木の組み立てが元の木を汚す
- 成果物: tools/break-check.sh
- 根拠: break-check.sh:87。Task 4 で元の木の `cargo test` が rc=101 になった実測あり（`break-proof.sh` は回避済みだが道具の既定が罠）。ほかに rc 137 を「文言が違う」と取り違える（Task 2 F3）、出力を流さない（F6）
- kind: technical
- 提案: 既定を作業木ごとの target にする（あるいは `$TMPDIR` 下の専用 target）。rc 137 / 124 は別の文言で落とす
- 処置: fixed 2.1 —— 既定の CARGO_TARGET_DIR を使い捨ての木の下に、124/137 を経過時間で区別、tee で流す、self-test に例（b16a74dd7）

## R10. `break-proof.sh history` の最後の PASS が、その後の `runtime.rs` / `collect.rs` の変更より前
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/break-proof.sh / evidence.jsonl
- 根拠: 最後の PASS は 58d760420、その後 9d4266521 / c3e5f3550 で補助関数が変わった。`git apply --check` は通る
- kind: technical
- 提案: fix の後の HEAD で `break-proof.sh history` を走らせ直し、証跡を残す
- 処置: fixed 3.2 —— b16a74dd7 で verify-run 3.2 PASS rc=0、evidence.jsonl へ（e34dab82c）

## R11. 時刻を兼ねるループに D14 の `// timing-ok:` が無い
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: runtime.rs:4232 の `t(sec)` のループ（Q5 で回数のまま残す形）に理由の印が無い
- kind: technical
- 提案: `// timing-ok: <理由>` を付ける
- 処置: fixed 12.1 —— runtime.rs のループに timing-ok を付け、検査は for sec＋印のものだけ許す（20f57f13a）

## R12. `load-proof.sh` が両方の負荷で再現した記録（`host,db`）を扱えない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/load-proof.sh
- 根拠: load-proof.sh:76-79。`--load host,db` が「未知の負荷」で落ちる（Task 13 F9。いまの `load-before.txt` では実害なし）
- kind: technical
- 提案: カンマ区切りを受けて各負荷で走らせる
- 処置: fixed 13.3 —— probe_each がカンマ区切りを負荷ごとに走らせ、未知の負荷は名指して rc=1（b16a74dd7。実際の host,db では未実行）

## R13. 待ちの補助が期限を過ぎても黙って返る / pool の中の AssertionError が飲み込まれる
- 成果物: crates/collector-windows/tests/runtime_windows.rs / collector-android/app/src/test/…（Task 7 の pool を使う試験）
- 根拠: Task 8 F2（`observe_until` / `wait_history` / clock_skew の while が期限後に黙って返る）、Task 7 F5（pool の中の AssertionError が飲み込まれる）
- kind: technical
- 提案: 期限切れは panic / 失敗にし、pool の失敗は呼び出し側へ伝える
- 処置: fixed 8.1 —— observe_until / wait_history / 時計のずれの待ちは期限切れで値つきで落ちる、UnavailableLogGateTest は Future から失敗を投げ直す（576b4288a。Windows の実行時テストは走らせていない）

## code_verify（7aa635c58）

検証の席（code_verify）。HEAD 7aa635c58・分岐点 11c0c043a。作業ツリーは触らず、壊す確かめは全部 `git worktree add --detach` の使い捨ての木（`mktemp -d` の下。終わりに `worktree remove --force` + `prune`）で行った。
前後の `git status --porcelain` は、この節を書いた `review/code.md` 以外に差が無い。

**申告: 22/22**（tasks.md の `[x]` 1.1〜13.4。final review の fix の後、evidence.jsonl の最後の PASS が対象のファイルの変更より前のもの 11 件（1.1・2.1・3.1・5.1・7.1・8.1・9.1・10.1・11.1・13.3・13.4）、記録の無いもの 2 件（12.1・13.2））

| 項目 | 検証コマンド（tasks.md の本文） | 実測 | 一致 |
|---|---|---|---|
| 1.1 | `tools/load-repeat.sh --self-test` | rc=0（4 s。印 3 つを出す） | 一致 |
| 1.2 | `load-proof.sh before` ＋ `load-before.txt` の grep 2 本 | grep 2 本 rc=0。`before` は committed の `load-before.txt` を `: >` で書き換えるので走らせていない（直した後のコードでは意味も無い） | 一致（grep のみ） |
| 2.1 | `tools/break-check.sh --self-test` | rc=0（6 s。印 4 つ） | 一致 |
| 3.1 | `CW history_large_fetch_keeps_observing_foreground` / `NL … 1` | `cargo test --workspace` の中で ok・`--list` 1 本 | 一致 |
| 3.2 | `break-proof.sh history` | rc=0（51 s）。壊した木で `runtime.rs:3740` が `見張りが読みを放した: 見回りが履歴の取得を待った` で panic | 一致 |
| 3.3 | `CW clock_worker_slow_read_does_not_hold_the_patrol` / `NL … 1` | workspace の中で ok・`--list` 1 本 | 一致 |
| 4.1 | `CT archive_scan_interval_change_takes_effect_without_restart` / `NL … 1` | workspace の中で ok・`--list` 1 本 | 一致 |
| 4.2 | `break-proof.sh scan` | rc=0（137 s）。壊した木で `走査が 2 回に届かない: 最後に見た数 1・時計の経過 29.029s・待った時間 30.013s`（落としたのは実時間の期限の側） | 一致 |
| 4.3 | `CT archive_scan_interval_` / `NL … 3` | workspace の中で ok・`--list` 5 本 | 一致 |
| 5.1 | `CW history` `CT scheduled` `CT outage` / `NL` 20・5・5 | workspace の中で ok・`--list` 128・14・92 本 | 一致 |
| 5.2 | `CT ch04_statement_timeout_cancels_a_long_query` / `NL … 1` / `CW time_sync_timeout_kills_the_child` | workspace の中で ok・`--list` 1・1 本 | 一致 |
| 6.1 | `npx vitest run src/__tests__/ch09-timeouts.test.ts` / `npx vitest run` | 全体 rc=0（52 ファイル・513 本） | 一致（ただし R16） |
| 7.1 | `KT *UnavailableLogGateTest*` | `--rerun` で UnavailableLogGate・HealthNotification・FeelingReminderReceiver の 26 本 rc=0（XML の timestamp は今回） | 一致 |
| 8.1 | clippy windows-gnu `-D warnings` / Windows 側の `--test runtime_windows`（13 本以上） | clippy rc=0。実行時テストは **15 本中 12 本 ok・3 本 FAILED** rc=101（3 本とも `focus_window_of` の `pid … の窓が 30 秒以内に見つからない`。Edge を開く試験）。**分岐点の版の runtime_windows.rs を Windows 側の clone に置いて同じ 1 本を走らせても `pid … の窓が見つからない` で落ちる**（14.58 s）ので、CH09 の変更ではなく、いまの Windows の机の上の Edge の状態による（msedge.exe が 60 本ほど残っていた）。576b4288a で変わった `clock_skew_runtime`・`outage_toast` の 2 本は ok | 不一致（環境。CH09 の差分に依らない） |
| 9.1 | typecheck・lint / `npm run test:e2e` | rc=0（244 passed。`ch09-timeouts.spec.ts` を含む） | 一致 |
| 10.1 | `assembleDebugAndroidTest` / `tools/android-emulator.sh` | rc=0・rc=0（1074 s） | 一致 |
| 11.1 | `bash -n` の for / `tools/check-shared-appends.sh` / `tools/smoke.sh` | rc=0・rc=0・rc=0（115 s） | 一致 |
| 12.1 | `check-inventory.sh` | rc=0（`--self-test` も rc=0。166 ファイル） | 一致（ただし R15） |
| 13.1 | `check-product-unchanged.sh --self-test && check-product-unchanged.sh` | rc=0 | 一致（ただし R14） |
| 13.2 | `check-test-names.sh` | rc=0 | 一致 |
| 13.3 | `load-proof.sh after` | 台本は committed の `load-after.txt` を書き換えるので、同じ中身を直に: `load-repeat.sh --times 10` を history に `--load host`・`--load db`、scan に `--load host`・`--load db` → 4 つとも `pass 10/10` rc=0 | 一致 |
| 13.4 | `cargo test --workspace` / `check_scenarios.py` / `check_agreed_tests.py` | rc=0（2,256 passed・0 failed・3 ignored。走らせている間、Windows 側の組み立てが並走し load average 22）/ `scenarios: OK` / `OK` | 一致 |

（8.1 の脚注: 残っていた msedge.exe は、相手役 `Edge` の `Drop` が `taskkill /PID <子> /T /F` で子の木だけを止める形（runtime_windows.rs:962-968。分岐点にもある）による取り残しの可能性がある。CH09 の範囲外で、R は立てない）

- **手 1**: 該当なし（確かめた範囲: 分岐点〜HEAD の差分の `+` 行で 16 桁以上の 16 進・hash・digest、3 桁以上の数を直書きした assert。どれも無い。上限の値（10,000・30,000・120,000）の担保は手 3・手 4 で見た）
- **手 2**: 製品不変の検査・一覧の検査・名前の検査を壊した複製で走らせた。名前の検査は試験 1 本を消すと rc=1（`fn ch04_statement_timeout_cancels_a_long_query` を名指す）。一覧の検査は回数の待ち・Kotlin の `toNanos`・e2e の `timeout:` では rc=1。すり抜けは R14・R15
- **手 3**: change の Scenario 24 本は全部に印があり、`check_scenarios.py` は OK。印の先を 1 本ずつ読み、主張の階層と違うのは R16（画面の単体試験の上限）。`test-timing` の「止めた読み」の 2 本は `runtime.rs:3669-3812` が見張り・`history(&rt) == before`・前景の `browser`・`max_queued_per_tick`・`N` 件の積みこぼし無しを実際に観測している。e2e の上限は `test.info().timeout`（効いている値）を見ている
- **手 4**: Q8 の「30 秒未満」は壊す差分（4.2）で、Q10 の見張りは 3.2 で、Q9 の待ち 10 秒は `getConfig().asyncUtilTimeout`、D15 の e2e の値は `ch09-timeouts.spec.ts` が落とす（playwright の設定を 5 s / 10 s にした複製で一覧の検査は rc=0 だが、この e2e が値を見ている）。値を変えても全部通るのは R15（Q5 の 30 秒の 3 形）・R16（試験の上限 30 秒）・R17（Q3 の 60/30 と 2,000 件）
- **手 5**: 表のとおり。`cargo test --test runtime_windows` の `tests/runtime_windows.rs` は実在する。テスト名の挙がった項目は `--list` で全部 1 本以上
- **手 6**: 製品は変えていない（波括弧の対応で独立に数え、3 ファイルとも試験モジュールより前は base と HEAD で同じ）。記録が失われる経路は差分に無い。台本の側で、止め方しだいで負荷が残る経路が 1 つ（R18）
- **手 7**: 該当なし（`python3 scripts/plan_fix.py pending openspec/changes/ch09-load-tolerant-timing-tests` → `[]`）。`correction-review.md` は作らない

## R14. `check-product-unchanged.sh` は `mod tests {` の開始行から後ろを全部「試験」とみなし、試験モジュールの後ろに足した製品のコードを通す
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-product-unchanged.sh
- 根拠: 使い捨ての作業木で `crates/collector-windows/src/history/collect.rs` の末尾（`mod tests { … }` の閉じの後）に `pub fn sneaky_product() -> u32 { 42 }` を足して commit → `check-product-unchanged.sh` は `product: 変更なし（分岐点 11c0c043 → HEAD）` rc=0。原因は check-product-unchanged.sh:24-40 の awk が `#[cfg(test)]` + `mod tests` の**開始行**しか取らず、閉じの位置を見ないこと（:65・:70 で `new_line < new_start` だけを製品扱い）。いまの差分そのものは問題ない —— 波括弧の対応で独立に数えると、base・HEAD とも 3 ファイル（collect.rs 814→1128・runtime.rs 931→4334・time_sync.rs 330→666）の `mod tests` は末尾の行で閉じ、その前の部分は base と HEAD で同じだった
- kind: technical
- 提案: 波括弧の対応で `mod tests` の閉じの行を求め、範囲の外の行を製品として数える。`--self-test` に「試験モジュールの後ろに製品の関数を足す」例を足す
- 処置: fixed 13.1 —— 試験モジュールの閉じを波括弧の対応で求め（文字列・文字・注記の中は数えない）、範囲の外の行を製品として数える。閉じが見つからなければ名指して rc=1。--self-test に「mod tests の後ろに製品の関数を足す」例。3 ファイルとも閉じは末尾の行（5c7b8b6c0）

## R15. `check-inventory.sh` は Rust の実時間の期限・Kotlin の `Future.get` の上限・台本の `SECONDS` の期限を見ず、CH09 が直した待ちを 30 秒未満に戻しても緑になる
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh / crates/server/src/scheduled_tests.rs / collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt / tools/smoke.sh
- 根拠: 使い捨ての作業木で 1 つずつ戻して commit し、`check-inventory.sh` を走らせた:
  - `scheduled_tests.rs` の `wait_for_calls` の `Duration::from_secs(30)` → `from_secs(2)`: rc=0（`inventory: 166 ファイルを一覧済み`）
  - `UnavailableLogGateTest.kt:29` の `task.get(30, TimeUnit.SECONDS)` → `get(2, …)`: rc=0
  - `tools/smoke.sh` の `shape_deadline=$((shape_started + 30))` → `+ 5`: rc=0
  - 対照（落ちるもの）: `wait_for_calls` に `for _ in 0..40 { … settle(25) }` を戻す → rc=1（`回数 × 眠り = 1.0 秒`）。Kotlin の `toNanos(30)` → `toNanos(2)` → rc=1。e2e に `toBeVisible({ timeout: 5_000 })` → rc=1
  spec `test-timing` の Scenario「一覧で直したとした条件の待ちに、回数の待ちと 30 秒より短い期限が残っていない」の THEN は「30 秒より短い待ちの上限」を名指しで残さないと言うが、CH09 自身が入れた期限の形（Rust の `Instant` + `Duration::from_secs(N)`、Kotlin の `Future.get(N, …)`、台本の `SECONDS + N`）が検査の外にある。本人の決定 Q5（実時間の期限で一律 30 秒）は、この 3 形では値を変えても全部通る
- kind: technical
- 提案: 3 形（`Duration::from_secs(<数>)` を期限に使う行・`.get(<数>, TimeUnit…)`・`SECONDS + <数>` / `$((… + <数>))` の期限）を拾って 30 未満を落とし、`--self-test` に違反の例を足す。直したら HEAD に上の 3 つを当て、名指して rc=1 になることを確かめる
- 処置: fixed 12.1 —— Rust の `deadline… = … + Duration::from_secs|millis(<数>)`（折れた右辺も）・Kotlin の引数の「<数>, TimeUnit」（`get`・`Tasks.await`・`latch.await`）・台本の `…deadline=$((… + <数>))` を拾い 30 秒未満を落とす。--self-test に違反の例。HEAD に席の 3 つ（scheduled_tests 2 秒・UnavailableLogGateTest get 2 秒・smoke.sh +5）を当てると 3 つとも名指して rc=1。強めた検査で `server_startup.rs` の 20 秒の期限 5 か所が見つかり 30 秒にした（一覧の行も直した）（5c7b8b6c0）

## R16. 画面の単体試験の「試験の上限 30 秒」の試験は設定ファイルの字面を正規表現で見ていて、効いている上限を観測していない
- 成果物: web/src/__tests__/ch09-timeouts.test.ts / web/vitest.config.ts
- 根拠: ch09-timeouts.test.ts:4・:10 は `../../vitest.config.ts?raw` を読み `toMatch(/testTimeout:\s*30_000/)` を見る。使い捨ての作業木（node_modules は symlink）で vitest.config.ts を `// testTimeout: 30_000,` ＋ `testTimeout: 1_000,` にすると、`npx vitest run src/__tests__/ch09-timeouts.test.ts` は `1 passed` rc=0。同じ木で 2 秒眠るだけの試験は `1 failed`（1007ms で打ち切り）—— 効いている上限は 1 秒なのに印の試験は緑。`web/package.json` の `"test": "vitest run"` に上書きの引数は無い。spec の Scenario は「上限を読む → 30,000ms である」で、主張は効いている値、試験は字面（主張の階層とテストの階層の違い）。e2e の側（`ch09-timeouts.spec.ts`）は `test.info().timeout` で効いている値を読んでいて、こちらは同じ問題が無い
- kind: technical
- 提案: 字面でなく値を読む —— 設定モジュール（`vitest.config.ts`）を import して `test.testTimeout` を見る。試験の文脈から効いている上限を取れればなおよい（取れるかはこの席では確かめていない）。直したら「30_000 をコメントにして 1_000 を置く」複製で落ちることを確かめる
- 処置: fixed 6.1 —— 試験の文脈の `task.timeout`（効いている上限）を見る。vitest.config.ts を「30_000 をコメント・1_000」にした複製で `expected 1000 to be 30000` で落ちる（5c7b8b6c0）

## R17. 本人の決定のうち、Q3 の「相手 60 秒・上限 30 秒」と spec の「1 回に積む 2,000 件」は、値を変えても落ちる試験が無い
- 成果物: crates/server/src/testdb_tests.rs / crates/collector-windows/src/runtime.rs / crates/collector-windows/src/history/collect.rs
- 根拠: 使い捨ての作業木で `testdb_tests.rs:373` の `pg_sleep(60)` → `pg_sleep(5)`、`:382` の `from_secs(30)` → `from_secs(5)` に戻して commit → `check-inventory.sh` rc=0・`check-test-names.sh` rc=0（打ち切りの型は一覧の検査の対象外。正典の Scenario の数値はこの試験そのものにしか無い）。
  「2,000 件以下」は runtime.rs:3781 が製品の定数 `QUEUE_PER_TICK`（collect.rs:169 `= 2_000`）と比べるだけで、`grep -rn 'QUEUE_PER_TICK' crates/collector-windows/src` に値そのものを固定する assert は無い。定数を大きくすると `max_queued_per_tick <= QUEUE_PER_TICK` も `queueing_ticks >= (N - 10) / QUEUE_PER_TICK` も一緒に緩む（30,000 以上なら下限が 0 になり、AND 節「2 回以上の見回りに分かれ」も見なくなる）
- kind: daily
- 提案: 2,000 は `assert_eq!(QUEUE_PER_TICK, 2_000)` を 1 行（試験のモジュールの中で足りる。製品は変えない）。testdb の 60/30 は R15 の検査に「打ち切りの型は相手 ≥ 60・上限 ≥ 30」を足すか、残るものとして受け入れる
- 処置: fixed D18 仮 —— 2,000 件は試験の中で `assert_eq!(QUEUE_PER_TICK, 2_000)`。打ち切りの 60/30 は検査に足さない（archive 後は期待の行を check_agreed_tests が守る）。反転条件は design D18、台帳 provisional.jsonl（5c7b8b6c0）

## R18. `load-repeat.sh` を KILL で止めると、全コアの busy loop が残り続ける
- 成果物: tools/load-repeat.sh
- 根拠: `tools/load-repeat.sh --times 1 --load host -- sleep 30` を背景で起こし、2 秒後に台本の pid へ SIGKILL を送った → 1 秒後も、印 `ch09-load-<台本の pid>-<番号>` の過程が 16 本（送る前も 16 本）。pid を挙げて止めるまで残った。load-repeat.sh:60-63 の負荷は `bash -c 'while :; do :; done'` で親の生死を見ず、片付けは load-repeat.sh:54-55 の `trap`（EXIT・INT・TERM）だけ。`--load db` も同じ形で、DB のコンテナの中に残る。spec は INT・TERM までを約束していて、SIGKILL（OOM の kill・人や見張りの強制終了）は外。残った負荷は、CH09 が無くしたい「別の Story の関門が混み具合で落ちる」をそのまま起こす
- kind: technical
- 提案: busy loop に親の生存の確認を混ぜる（例: 外側の loop の条件を `kill -0 <台本の pid>` にし、内側で数万回まわす）。DB の側は `timeout <上限>` を付ける。`--self-test` に SIGKILL の例を足す
- 処置: fixed 1.1 —— host の負荷は台本の pid の生存を自分で確かめ、台本が消えたら止まる。DB の負荷は台本が 5 秒ごとに触る印のファイルを見て、30 秒触られなければ止まる。--self-test に SIGKILL の例（host・db とも 45 秒以内に 0 本）（5c7b8b6c0）

## security review（11c0c04..7aa635c）

見た経路:
- 製品の Rust（`runtime.rs` `history/collect.rs` `time_sync.rs`）→ 変更した hunk はすべて `#[cfg(test)] mod tests` の中（runtime.rs:930-931 より後ろの 1889 行〜、collect.rs:813-814 より後ろ、time_sync.rs:329-330 より後ろ）。製品のバイナリに入らない（4）
- `crates/server/src/testdb.rs` → lib.rs:262-263 の `#[cfg(test)] pub mod testdb;` で試験の組み立てにだけ入る。変更は CREATE/DROP DATABASE の打ち直しを回数から実時間 30 秒へ変えただけで、`{name}` の出どころは試験の足場の生成名のまま（2）
- router（`crates/server/src/lib.rs`）・移行・`collector-android/app/src/main`・`web/src` の製品のコード → 差分なし。新しい API・画面・待ち受けは無い（4）
- 依存の宣言（Cargo.toml / Cargo.lock / package.json / package-lock.json / build.gradle.kts / gradle）→ 差分なし（4）
- `web/playwright.config.ts` `web/vitest.config.ts` → 試験の上限（`expect.timeout` `timeout` `testTimeout`）だけ。webServer・baseURL・待ち受けの host は変わらない（2）
- `tools/smoke.sh` `stack.sh` `check-panic-log.sh` `web-login.sh` `record-env.sh` `verify-env.sh` → 待ちを回数から `SECONDS` の期限へ変え、注記を足しただけ。期限切れの文言に出るのは偽データ（`SHAPE` の hash、`/archives/status` の smoke の書庫の状態、port ファイルの有無）。待ち受け（`--host 127.0.0.1`・`check-exposure.sh`）はそのまま（2）
- 新しい道具 `tools/break-check.sh`（使い捨ての `git worktree add --detach` に差分を当て、`mktemp -d` の下で走らせて後始末で消す。break-check.sh:66-80）・`tools/load-repeat.sh`（host の busy loop と `tools/db.sh exec` の中の負荷。外への送信・待ち受けは無い）→ 手元で開発者が呼ぶだけで、外から届く入口ではない（2）
- 記録として commit した `load-before.txt` `load-after.txt` `evidence.jsonl` `.superpowers/sdd/ch09-task-*/*.md` → `password|postgres://|token|secret|cookie|session=` の grep で 0 件。中身は試験名・秒数・rc（2）

安全の決め手: 「この差分は製品に入るコード（テストの外の Rust・Kotlin の main・web/src・router・移行・依存）を 1 行も変えていない」—— 確かさ 4。台本 `/tmp/ch09-sec-check.sh 11c0c043a` を走らせた（変更した各ファイルの最初の hunk の行が `#[cfg(test)] mod tests {` の行より後ろかを `git diff -U0` から数え、testdb.rs は lib.rs の `#[cfg(test)]` を、製品・依存のパスは `git diff --quiet` を見る。外れたら NG で rc=1）。出力:
```
OK  crates/collector-windows/src/history/collect.rs（最初の変更行 859 > #[cfg(test)] mod tests の行 814）
OK  crates/collector-windows/src/runtime.rs（最初の変更行 1889 > #[cfg(test)] mod tests の行 931）
OK  crates/collector-windows/src/time_sync.rs（最初の変更行 651 > #[cfg(test)] mod tests の行 330）
OK  crates/server/src/testdb.rs（lib.rs で cfg(test)）
OK  router・移行・製品の Android/画面・依存の宣言に差分なし
rc=0
```

指摘なし

## /code-review（11c0c04..7aa635c）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R19. check_rust_waits passes any count-bounded loop whose body mentions `deadline`, `Instant::now()` or `.elapsed()`, even when the count runs out long before the 30 s deadline; the self-test (line 470) even accepts `for _ in 0..40` with 25 ms sleeps as the fixed form.
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh:216
- 根拠: A wait written as `let deadline = now+30s; for _ in 0..40 { if done() || now>=deadline {break}; sleep(25ms) }` gives up after about 1 s no matter the deadline. Under load the condition is not met within 1 s and the test flakes, yet the inventory check reports no violation, which is exactly the shape CH09 is meant to remove.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 12.1 —— 期限を見ていても回数 × 眠りが計算でき 30 秒未満なら落とす（回数か眠りが計算できず期限を見ているものだけ許す）。--self-test の「直した形」の `for _ in 0..40` を `loop` に、同じ形を違反の例へ（5c7b8b6c0）

## R20. The interval-change test no longer pins the virtual elapsed time to 10 s: the retry loop re-advances the clock whenever a scan is missed, and the only bound left is `elapsed < 30s`. A changed interval that takes effect at 2 s or 3 s now passes.
- 成果物: crates/server/src/archive_scan_interval_tests.rs:312
- 根拠: A regression where `wait_for_next_scan` applies the new interval as 2 s (or rounds 1 s up to 2 s) needs two 1001 ms advances per scan, so 10 scans take about 20 virtual seconds. That is under 30, so the test is green. The old check `elapsed >= 10s && < 11s` caught this, and so did the comment's '変えた値が 1 秒以内に効く'.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.1 —— 進め直しは残し、「10 歩のどれかは 1 秒進めただけで走査が入る」を見る（間隔が 2 秒以上に効く退行では全部の歩が 2 回以上要る）。間隔を +1 秒に効かせる差分を当てると `変えた 1 秒が効いていない` で落ちる（10 歩とも 2 回）。走査を書いた直後は次の眠りの前なので、歩の間に 100ms 置く（5c7b8b6c0）

## R21. clock_worker_slow_read_does_not_hold_the_patrol dropped the total-time assertion; the watchdog only covers the 4 patrols for sec 0..=3, so the patrols in `run(4, 8)`, where the foreground switch happens, are no longer checked for blocking on the reference read.
- 成果物: crates/collector-windows/src/runtime.rs:1985
- 根拠: If a regression makes tick_at join the clock worker only after the foreground changes (ticks 4..8), those ticks block until SlowReference's 60 s recv_timeout. The watchdog has already exited and no elapsed check remains, so the test passes slowly instead of failing with '見回りが読み取りを待っている'.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.3 —— 見張りを 9 回（0〜8 秒）の見回り全部に当て、前景を変える 4 秒目からも 1 回ごとに見る（5c7b8b6c0。この退行の壊す差分は当てていない）

## R22. After the gate is released, the `while reader.reads() < 2 || is_reading()` loop has no real-time deadline (and nothing in the stopped phase asserts that a read was actually started and is blocked), so in this CH09-touched test it can hang forever or pass vacuously.
- 成果物: crates/collector-windows/src/runtime.rs:3764
- 根拠: If the second read is never scheduled after the 24 h jump, or the release token is consumed by the watchdog, `reads()` stays below 2 and the test spins with no end, hanging cargo test. The 5 'stopped read' patrols also pass trivially because no read is gated.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 —— 一部 rejected: 「永久に止まる」は起きない（`assert!(sec < 86_400 + 60_000)` が 60,000 回で落とす）。直したもの: 止めた読みの 5 回の後に「読み中・読んだ回数 1」を見る（始まっていなければ落ちる）、放した後のループに実時間 30 秒の上限と診断の文言（5c7b8b6c0）

## R23. The test now uses a gated FakeReader. `release` is declared before `rt`, so on a failed assertion `rt` (HistoryRuntime) is dropped first, while the reader thread is still blocked on the gate with a live sender.
- 成果物: crates/collector-windows/src/runtime.rs:3689
- 根拠: When any assert in the stopped phase fails, HistoryRuntime::drop polls threads_finished for the full 30 s while the reader blocks in `gate.recv()`. Only the watchdog's own 30 s timeout frees it, which races the drop's deadline. The result is a 30 s stall and the '一時フォルダが漏れうる' leak the drop is meant to prevent.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.1 —— 放す口 `release` を rt の後に束ね直し、rt より先に落とす。途中の assert で落ちても gate の読みが放されてから rt の drop が待つ（5c7b8b6c0）

## R24. check_kotlin_waits only flags `await(<number>, …SECONDS)`, so `Tasks.await(task, 5, TimeUnit.SECONDS)` and other awaits whose first argument is not a number are never checked.
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/check-inventory.sh:237
- 根拠: Reverting `Tasks.await(client.setMockMode(false), 30, TimeUnit.SECONDS)` to 5 s in LocationServiceInstrumentedTest or RetentionInstrumentedTest (both changed by hand in this PR) passes check-inventory, so the guard does not catch the regressions it exists for.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 12.1 —— R15 と同じ直し（引数のどこにあっても「<数>, TimeUnit」を見る）。`LocationServiceInstrumentedTest.kt` の `Tasks.await(…, 30, …)` を 5 にすると `:68: 30 秒未満の期限（5 秒）` で rc=1（5c7b8b6c0）

## R25. The broken-tree build uses `${CARGO_TARGET_DIR:-$CHECK_TEMP_ROOT/target}`, so an inherited CARGO_TARGET_DIR (record-env.sh exports one, for example) brings back the R9 problem the comment says is fixed: the patched build overwrites the shared target's artifacts.
- 成果物: tools/break-check.sh:96
- 根拠: Running break-proof.sh history from a shell with CARGO_TARGET_DIR exported compiles the deliberately broken collect.rs into the shared target dir. The next normal `cargo test` or release build in that environment may reuse the broken artifacts.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.1 —— 壊した木は常に使い捨ての木の下の target で組み立てる（受け継いだ CARGO_TARGET_DIR を使わない）。--self-test に CARGO_TARGET_DIR を export した例（5c7b8b6c0）

## R26. SHADE_TIMEOUT_MS went from 5 s to 30 s, but clickOngoingAction uses it first as a probe that is expected to miss when the ongoing notification is collapsed; that probe should stay short with a timing-ok, as the 2 s probe at line 102 does.
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/HealthConnectScreens.kt:193
- 根拠: On a device where the ongoing notification sits collapsed in the Silent section, every clickOngoingAction now waits the full 30 s before pressing the expand button. That adds about 25 s per call to the instrumented suite with no gain in robustness.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 10.1 —— トレイの常駐の通知の行が出るまでを 30 秒の条件の待ちにし、操作が見えるかは `SHADE_PROBE_MS` 5 秒（`// timing-ok:`）の窓で見る（5c7b8b6c0。計測テストは走らせていない。androidTest の組み立ては通る）

## R27. findAndClick tries the alternative selectors one after another with `device.wait(…, DIALOG_TIMEOUT_MS)`. Raising DIALOG_TIMEOUT_MS to 30 s means each selector that does not match the OS variant now burns 30 s (up to 3 times with STALE_RETRIES).
- 成果物: collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/PermissionDeniedInstrumentedTest.kt:210
- 根拠: On an emulator image whose deny button has no `permission_deny_button` id, so only the text selector matches, every dialog click waits 30 s on the first selector before trying the second. With up to 3 retries that is up to 90 s per dialog, and timeouts become more likely.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 10.1 —— 選び方を 1 つずつ 30 秒待たず、全部まとめて 250ms ごとに探して 30 秒の期限で切る `findAny` にした（5c7b8b6c0。計測テストは走らせていない。androidTest の組み立ては通る）

## R28. There is redundant or dead logic in the new helpers: awaitWithinD4 computes `remaining = deadline - nanoTime()` right after `started` (always about 30 s, so `remaining > 0` is always true); `.is_some_and(|last| last >= before + step)` in archive_scan_interval_tests.rs repeats wait_for_scans' own condition; load-repeat.sh self-test lines 162 and 168 are the same case twice; break-proof.sh sets CARGO_TARGET_DIR although break-check already defaults to a temporary one; check-test-names.sh `files=` is unused.
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt:20
- 根拠: No crash. Readers have to reason about conditions that can never be false and about duplicated scenarios. The simpler forms are `latch.await(30, SECONDS)`, `.is_some()`, a single self-test case, and dropping the unused variable and the redundant env override.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— `awaitWithinD4` は `latch.await(30, SECONDS)` に、`.is_some_and(…)` は `.is_some()` に、load-repeat の self-test の重複 1 つを消し、break-proof.sh の CARGO_TARGET_DIR の上書きと check-test-names.sh の未使用の `files=` を消した（5c7b8b6c0）

## finish の fix と scoped re-review（7aa635c..5c7b8b6）

fix は 1 回（5c7b8b6c0）。fix の側で走らせたもの（rc=0）: `cargo fmt --check`・`cargo clippy --workspace --all-targets -D warnings`・windows-gnu の clippy、触った試験（collector-windows 2 本・`archive_scan_interval_` 5 本・`--test server_startup` 39 本）、gradle の `UnavailableLogGateTest` と `compileDebugAndroidTestKotlin`、`npx vitest run`（513 本）、`check-inventory.sh`（`--self-test` を含む）・`check-product-unchanged.sh`（同）・`check-test-names.sh`・`break-check.sh --self-test`・`load-repeat.sh --self-test`（KILL の例を含む）、`break-proof.sh history`・`scan`、間隔を +1 秒に効かせる壊す差分（`変えた 1 秒が効いていない` で落ちる）。verify-run で 1.1・2.1・3.1・3.2・3.3・4.1・4.2・4.3・6.1・7.1・10.1・13.1・13.3 を記録した。
走らせていない: Windows の実行時テスト（runtime_windows.rs は変えていない）・e2e（設定も spec も変えていない）・smoke。

scoped re-review（`pr-review-toolkit:code-reviewer`、1 回）: R14〜R28 は全部 ADDRESSED（R17 は D18（仮）、R22 の「永久に止まる」への反論は正しい）。新しい Critical / Important なし。Minor 3 件:
- 負荷の下の確かめ（13.3）がこの commit で取り直されていない → 取り直した（verify-run 13.3）
- R20 の「1 回で届いた歩」は、DB を待つ間の時計の自動の進みとの競争に左右されうる（仕組みの上の保証は無い。壊す差分では 10 歩とも 2 回で落ちた）→ park。Ruling: 間隔 +1 秒の壊す差分で落ちることを確かめた。`break-proof.sh` へ足して繰り返し確かめるのは次の change で
- 期限の検査の取りこぼし（構造体の欄・型注記つきの束縛・定数・3 行以上に折れた右辺・`tokio::time::timeout`・`$(( $(date +%s) + N ))`）。いまの木に該当なし → park。Ruling: 検査が拾う形は design D14 の「拾う形」の範囲。広げるのは該当が出たとき
