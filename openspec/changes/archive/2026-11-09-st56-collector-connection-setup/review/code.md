# ST56 code review

## final review（7ad55e1..00ca256）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 2 / Important 9 / Minor 9。判定は「With fixes」。
reviewer が手元で走らせたもの: `cargo fmt --all --check`（runtime.rs で差分 → R3）、`python3 scripts/check_scenarios.py . st56-collector-connection-setup`（OK）。ビルド・試験・clippy は走らせていない。
申し送り（`docs/handoff/ST56.md` の送り先の系統の取り違え）は開始前のもので、deep Q5・design（D12・C6）が受けている。開始後に増えた項目は無い。
Task ごとの Minor と ⚠️ は final reviewer が全体で triage した。直すもの: T4 ⚠️ → R6、T6 F6・⚠️ → R7、T6 F10 → R1、T7 F5 → R5、T7 F6 → R4、T11 F17 → R10、T11 ⚠️2 → R11、T12 F3 → R17、T1 F1 → R12。
解消済み: T1 ⚠️・F4、T2 ⚠️、T3 ⚠️・F3、T6 ⚠️（D11 の post）、T7 ⚠️、T8 ⚠️、T9 ⚠️・F9・F11、T10 ⚠️（reload の門）・F10、T11 F13、T12 ⚠️。
残してよい: T1 F2・F3・F5〜F7、T2 F3〜F6、T3 F2、T4 F5〜F8、T5 F5〜F7・F9・⚠️、T6 F7〜F9、T7 F4・F7〜F12、T8 F7〜F9、T9 ⚠️（ST54 の dev 規則）・F5〜F8・F10、T10 ⚠️（旧形の api_token）・F9・F11〜F15・F17、T11 ⚠️（drops.jsonl）・F11・F12・F14・F15・F18〜F20、T12 F4〜F6。T10 ⚠️（本物の Windows の DPAPI）は 10.2（CI）が確かめる。

fix は 1 回（d715a52..d32e3e1、4 commit）。指摘の外に: `tests/runtime_windows.rs` の `Reply { channel }` 抜け（windows target のコンパイルが落ちていた）・clippy pedantic の既存の落ち・`Config::reload_from_disk` が印の起動でも置き場で送り先を上書きしていた。
fix の担当が走らせたもの: `cargo test -p ashiato-collector-windows` rc=0（387）・`cargo fmt --all --check` rc=0・`cargo clippy --workspace --all-targets -- -D warnings` rc=0・windows-gnu の clippy rc=0・gradle `:app:testDebugUnitTest` 全体 BUILD SUCCESSFUL（813）・`check_scenarios.py` OK。`runtime_windows` の実行・エミュレータ・e2e は走らせていない（server と web は触っていない）。
scoped re-review 1 回で R1〜R20 は全部 ADDRESSED。fix の差分から新しく Important 1・Minor 1 → R21・R22（finish で処置する）。もう 1 つの Minor（保存の時点で保留に入るので y/N の最中に保留の知らせが出うる）は spec と矛盾しない振る舞いの変化として PR 本文に書く。

## R1. 携帯: 保存してから確かめが返るまで（画面を閉じればいつまでも）送り先の保留が掛からず、未送信を新しい送り先へ送る（Q7 の違反）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SetupActivity.kt
- 根拠: SetupActivity.kt:72-90 —— `SetupStore.save` が新しい送り先を commit し（`Config.baseUrl` は要求ごとに読む。D11）、門を回す `ACTION_SETUP_CHANGED` は `runContractProbe` の結果の後の `runOnUiThread { if (isDestroyed) return@runOnUiThread … }` の中でしか送らない。試験は確かめを同期で回し（St56SetupActivityTest.kt:35）、保留の試験は `onStartCommand(ACTION_SETUP_CHANGED)` を手で呼ぶ（St56DestinationGateTest.kt:53）ので捕まらない
- kind: technical
- 提案: 保存の commit と同時に、承認した送り先と違えば保留を立てる（サービスの reconcile は未送信 0 件なら解く）。少なくとも保存の直後に確かめと無関係に `ACTION_SETUP_CHANGED` を送る。試験: 確かめを返さない Executor で保存し、送る契機を回して新しい送り先への POST が 0 件
- 処置: fixed 7.1 —— `SetupStore.save` が送り先・資格情報・保留（`DestinationGate.holdOnSave`）を 1 回の commit で書き、画面は確かめの前に `ACTION_SETUP_CHANGED` を送る。試験「確かめが返らないまま保存しても送る契機は新しい送り先へ送らない」「未送信が無いまま保存すると突き合わせで保留が解ける」（2670c33d2）

## R2. PC: 収集アプリ自身の窓に貼る経路（D14）では確かめも y/N も保留の知らせも出ず、黙って保留のまま送らない
- 成果物: crates/collector-windows/src/main.rs
- 根拠: main.rs:141-155（窓の糸は `save_setup_string` を呼ぶだけ）/ config.rs:257-283（未送信があると門が `Ask` のまま setup-state.json を書く）/ `destination_notice` は `destination_notified_at` が要り、立てるのは `setup` 副命令の `hold_destination` だけ。`st56_runtime_collects_without_destination` は保存と再読込しか見ない
- kind: technical
- 提案: `run_setup` を読み手と書き手を受ける関数に切り出し、窓の糸からも同じ流れ（保存と確かめ → 結果の文 → y/N → 承認か保留）を回す。試験: 未送信がある置き場で窓の経路に貼り、y/N が出ることと N で知らせの時刻が立つこと
- 処置: fixed 11.1 —— `setup::run_setup_flow` を `setup` 副命令と窓の糸で共有し、保存の時点で `destination_notified_at` を立てる。試験 `st56_setupcmd_window_flow_*` 4 本（a9496528e）

## R3. `cargo fmt --all --check` が runtime.rs で差分を出す（CI が落ちる）。clippy `unnecessary_map_or` の見込み
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: `cargo fmt --all --check` → runtime.rs:1369・:1377 に差分 / runtime.rs:991・:1018 の `.map_or(true, …)`。evidence に clippy の実行が無い
- kind: technical
- 提案: `cargo fmt --all`、`cargo clippy --workspace --all-targets -- -D warnings`（collector-windows は windows target の clippy も CI と同じ形で）を通す
- 処置: fixed 12.1 —— fmt と `is_none_or`。clippy -D warnings は Linux と x86_64-pc-windows-gnu の両方 rc=0（a9496528e）

## R4. 携帯: 保留の間も送る契機ごとに `GET /contract` を新しい送り先へ送り、保持の上限も当てない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt
- 根拠: Retention.kt:249-254 —— `Drainer.tick` が `contract.ask()` を `destination.blocked` の判定より先に呼ぶ。Scenario「未送信があるまま送り先を変えると、押すまで何も送らない」の THEN は「B が受けた要求は保存の時の `GET /contract` の 1 件だけ」。印の付いた試験は `contract = null` の Drainer で捕まらない / Task 7 F6（保留の間は maintenance を飛ばす）
- kind: technical
- 提案: 保留の判定を `ask()` の前へ、`maintenance()` を保留の判定より前へ。試験は数える確かめを渡す
- 処置: fixed 7.1 —— `Drainer.tick` を見回り → 保留の判定 → `ask()` の順に。印付き試験に数える `ContractGuard`（probes==0・maintained==2）（2670c33d2）

## R5. 携帯: 保存した後の画面に保留（件数・期間・「この送り先へ送る」）が出ず、2 回目の保留は 24 時間通知も出ない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SetupActivity.kt / DestinationGate.kt
- 根拠: SetupActivity.kt:46（`showDestinationHold` は `onCreate` でしか組まない）/ DestinationGate.kt:84-92（`clearPending` が `NOTIFIED_AT` を消さない。Task 7 F5）/ spec「設定の画面に未送信の件数と期間を見せ」
- kind: technical
- 提案: 保存と保留の変化で画面を組み直す。保留を解くとき `NOTIFIED_AT` も消す
- 処置: fixed 7.1 —— 保存・prefs の listener・承認で保留の節を組み直し、`clearPending`・`holdOnSave` が `NOTIFIED_AT` を消す。試験「保留を解いた後の 2 回目の保留も知らせる」（2670c33d2）

## R6. 携帯: 保存できないときの文が spec と違い、「設定済み」と送り先の表示も無い
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SetupActivity.kt
- 根拠: SetupActivity.kt:75 は `UNREACHABLE` を含め全部「設定文字列を読み取れません」/ spec は読めない文字列 →「設定の文字列として読めません」、loopback・http →「この送り先は携帯から届きません。サーバの画面を Tailscale のアドレスで開いて、設定の文字列を出し直してください」、設定の画面は「設定済み」と送り先だけを示す / 試験は enum だけ（St56ConfigTest.kt:69,78）（Task 4 ⚠️）
- kind: technical
- 提案: 文を spec どおりに分け、定数にして試験から文言を見る。「設定済み」と送り先を出す
- 処置: fixed 6.1 —— `UNREADABLE_TEXT`・`UNREACHABLE_TEXT`・`CONFIGURED_TEXT` の定数と送り先の表示。試験 2 本が文言を見る（2670c33d2）

## R7. 携帯・PC: 未設定の知らせを 24 時間ごとに出し直さず、携帯は設定した後も消さない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / crates/collector-windows/src/main.rs
- 根拠: LocationService.kt:866（`onStartCommand` のたびに無条件で出す。24 時間の判定も `cancel(SETUP_NOTIFICATION_ID)` も無い。Task 6 F6 は NOT ADDRESSED、Task 6 ⚠️ D20）/ PC main.rs:203（起動時に 1 回だけ）/ spec の両端末の「未設定が続く間は 24 時間ごとに出し直す」
- kind: technical
- 提案: 最後に出した時刻を持ち 24 時間ごとに出す。設定したら消す。試験は試験の時計で
- 処置: fixed 6.1 —— （PC 側は 11.1）携帯 `LocationService.refreshSetupNotice`（24 時間・設定で消す）、PC runtime の `SetupNotice` と `dismiss_setup`。試験は両端末とも試験の時計（2670c33d2・a9496528e）

## R8. PC: 断られ・届かない・保留の知らせを押しても `setup` の窓が開かない
- 成果物: crates/collector-windows/src/toast.rs
- 根拠: `show_destination` / `show_destination_hold` は `launch="setup"` を付けるだけで、`Activated` の受け手を登録するのは `show_setup` だけ / spec「押すと `setup` の窓を開く」
- kind: technical
- 提案: 受け手の登録を共通の関数にし、全部の知らせで使う
- 処置: fixed 11.1 —— `toast.rs` の `show_opening_setup` に共通化し、4 つの知らせ全部が `Activated` の受け手を持つ。Windows 実機の確かめは 10.2（CI）（a9496528e）

## R9. PC: 時計合わせと途切れの問い合わせが、起動時の送り先と資格情報のまま変わらない
- 成果物: crates/collector-windows/src/main.rs
- 根拠: main.rs:175-181,211-215 —— `HttpDateClock::new(&cfg.base_url, …)`・`HttpOutageQuery::new(&cfg.base_url, &cfg.api_token, …)` は起動時の値で固まり、ループは `transport.reload` だけ。未設定で起動して設定すると空の送り先に向いたまま、A→B では A に旧資格情報で問い合わせ続ける（D11）
- kind: technical
- 提案: 送り先の再読込で時計と途切れの問い合わせも作り直す（または要求ごとに読む）
- 処置: fixed 9.1 —— `HttpDateClock::reload`・`HttpOutageQuery::reload` をループの再読込で呼ぶ。試験 `st56_http_date_clock_follows_reloaded_destination`・`st56_outage_query_follows_reloaded_destination`（a9496528e）

## R10. PC: 届かない・断られの知らせの文・起点・数えのやめ方が spec と違う
- 成果物: crates/collector-windows/src/runtime.rs / destination_gate.rs / config.rs
- 根拠: runtime.rs:1014 の文「サーバに届きません。送り先を確かめてください」（spec「送り先に届きません。設定を確かめてください」）/ destination_gate.rs:205-224 は 24 時間を最初の失敗から数え、一度答えが返った後の失敗でも数える（spec は「設定を保存した時から」「一度も」。Task 11 F17）/ 印で渡した送り先でも一度の実行の中で鳴る / config.rs:257 の `save_setup_string` が `auth_since`・`unreachable_since`・`*_notified_at` を消さない（spec「保存し直すと数えをやめる」）
- kind: technical
- 提案: 文を spec に合わせて定数にし試験で見る。起点を保存の時刻に、届いたことがあれば数えない。印の送り先では鳴らさない。保存で数えを消す
- 処置: fixed 11.1 —— 文を `destination_gate` の定数に、起点を `setup_saved_at`・答えが一度も無いときだけ・印の送り先で鳴らさない・保存で数えを消す。試験 `st56_gate_*`・`st56_setupcmd_save_resets_counts_and_starts_unreachable_clock` ほか（a9496528e・d32e3e12b）。直し方から来た誤報は R21

## R11. PC: `--config-from-env` の送り先が、前の `setup` の setup-state.json と未送信があると保留に落ちる
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: runtime.rs:264-274,216-227（Task 11 ⚠️2）/ D13「印で渡した送り先は承認したものとして扱う」・Global Constraints「PC の環境変数の起動は `--config-from-env` を足せば今どおり動く」
- kind: technical
- 提案: runtime に「印で起動した」を渡し門を開けたままにする。試験を足す
- 処置: fixed 9.1 —— `Config.config_from_env` で印の起動は門を開け、`reload_from_disk` も置き場を読まない。試験 `st56_runtime_config_from_env_is_not_held_by_previous_setup_state`（a9496528e）

## R12. PC: `SetupString` の `derive(Debug)` が資格情報を平文で出す
- 成果物: crates/collector-windows/src/setup_string.rs
- 根拠: setup_string.rs:8（Kotlin は伏せている。Task 1 F1）/ Global Constraints「資格情報をログに出さない」
- kind: technical
- 提案: 手書きの Debug で token を伏せる
- 処置: fixed 1.1 —— 手書きの Debug で `***`。試験 `st56_setup_string_debug_hides_the_token`（a9496528e）

## R13. PC: `save_setup_string` の書く順で、門が開く隙間がある
- 成果物: crates/collector-windows/src/config.rs
- 根拠: settings.json → credential.bin → setup-state.json の順。間に runtime が再読込すると `!store.exists()` で門が開く
- kind: technical
- 提案: setup-state.json を先に書く
- 処置: fixed 10.1 —— `save_setup_string_at` が setup-state.json を先に書く（a9496528e）

## R14. PC: y/N の未送信の期間が生存信号だけだと「不明」になり、時刻が UTC のまま
- 成果物: crates/collector-windows/src/destination_gate.rs
- 根拠: `pending_summary` が見るキーに `emitted_at` が無い / 時刻は UTC の RFC3339
- kind: technical
- 提案: `emitted_at` を見る。手元の時刻で出す
- 処置: fixed 11.1 —— `emitted_at` を見て、`chrono::Local` で出す。試験 `st56_gate_pending_summary_reads_heartbeat_time`（a9496528e）

## R15. 携帯: `reconcileDestination` が主糸で outbox と原本を全部読む
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `onCreate`・`onStartCommand` で 4 つの outbox と原本を数える。未送信が多いと ANR の恐れ
- kind: technical
- 提案: 件数の数えを背景の糸へ
- 処置: fixed 7.1 —— 数えを `runCollection`（背景の糸）へ移した（2670c33d2・d32e3e12b）

## R16. 試験の名前 `broken_settings_are_not_replaced_with_a_new_device_id` に `st56_` が無い
- 成果物: crates/collector-windows/src/config.rs
- 根拠: Global Constraints「試験の名前の接頭辞は `st56_`」
- kind: technical
- 提案: 名前を変える
- 処置: fixed 9.1 —— `st56_config_broken_settings_are_not_replaced_with_a_new_device_id` に改名（a9496528e）

## R17. 手順書 `docs/collector-setup.md` がボタンの名前・Windows のシェル・PC の保留の解き方で実際と違う
- 成果物: docs/collector-setup.md
- 根拠: §2「収集アプリの設定」（携帯のボタンは「送り先の設定」）/ §3 `ASHIATO_CONFIG_FROM_ENV=1 … --config-from-env` は bash の書き方（Task 12 F3）/ §5 に PC の保留を解く手順（`setup` で Enter → y）が無い
- kind: technical
- 提案: 実際の名前に揃え、cmd / PowerShell の書き方にし、PC の保留の解き方を書く
- 処置: fixed 12.1 —— §2 のボタン名、§3 の cmd と PowerShell の書き方、§5 の PC の保留の解き方（19f70ea32）

## R18. 作業場の `unsafe_code` を forbid から deny に緩めた注記が古い
- 成果物: crates/collector-windows/src/toast.rs
- 根拠: `secret.rs` のために deny へ。toast.rs 冒頭の「作業場は forbid」が古い
- kind: technical
- 提案: 注記を直す
- 処置: fixed 10.1 —— 注記を「作業場は deny、許すのは `secret.rs` だけ」に（a9496528e）

## R19. 携帯: `SETUP_SKIPPED` がバックアップされる prefs にある
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/SetupActivity.kt
- 根拠: `SETUP_SKIPPED` を `collector` の prefs に置く / D9「D3 と同じく外す」
- kind: technical
- 提案: `collector_setup.xml` に置く
- 処置: fixed 6.1 —— `SETUP_SKIPPED` を `collector_setup.xml` に。試験が `collector` の prefs に無いことも見る（2670c33d2）

## R20. PC: `secret.rs` に SAFETY の注記が無く `CRYPTPROTECT_UI_FORBIDDEN` を渡さない。自動起動の台本の書き換えが原子的でない
- 成果物: crates/collector-windows/src/secret.rs / autostart.rs
- 根拠: secret.rs の unsafe ブロック / `autostart::rewrite_existing_for_setup` の `fs::write` と `Box::leak`
- kind: technical
- 提案: SAFETY の注記と UI_FORBIDDEN を足す。一時ファイルに書いて置き換える
- 処置: fixed 10.1 —— SAFETY の注記・`CRYPTPROTECT_UI_FORBIDDEN`・`atomic_write`・`Box::leak` の除去（a9496528e）

## R21. PC: 保留のままにすると、確かめで繋がった送り先でも保存から 24 時間後に「送り先に届きません」が出る（R10 の直し方から来た誤報）
- 成果物: crates/collector-windows/src/destination_gate.rs
- 根拠: scoped re-review —— `notice_kind`（destination_gate.rs:241 付近）は `setup_saved_at` から 24 時間かつ `first_response_at` が無ければ Unreachable / config.rs:286 の保存で `first_response_at = None` / `save_and_check` の `GET /contract` の答えは `observe_response` に渡らない / 門が閉じている間 runtime は送らず答えを受けないが、runtime.rs:503 の閉じた側の分岐も `emit_destination_notice` を呼ぶ。保留の状態で 24 時間を回す試験は無い
- kind: technical
- 提案: 保存の時の確かめの答えを `observe_response` で state に入れ、門が閉じている間は Unreachable を数えない。試験: 未送信があり N と答えた置き場で 24 時間進めても「送り先に届きません」が出ない
- 処置: fixed 11.1 —— 門が閉じている間（保留・未設定）は「届かない」を数えない（runtime.rs `emit_destination_notice`）。`setup` の確かめの答えも `observe_response` で state に入れる（setup.rs `save_and_check`）。試験 `st56_runtime_held_destination_does_not_report_unreachable`（未送信あり・B を保留で 26 時間回して「届かない」が 0 回・保留の知らせは出る。閉じた門の分岐を外すと落ちる）

## R22. PC: `setup` で入力が閉じられた（EOF）ときも「設定の文字列として読めません」で失敗する
- 成果物: crates/collector-windows/src/main.rs
- 根拠: scoped re-review —— `run_setup` が `SetupOutcome::Closed` を `UNREADABLE_TEXT` で返す
- kind: technical
- 提案: 何も入力が無かった旨の文で終える
- 処置: fixed 10.1 —— 入力が閉じられたら `CLOSED_TEXT`（「設定の文字列が入力されませんでした。何も保存していません」）で終える（main.rs `run_setup`）。読めない文字列は今どおり `UNREADABLE_TEXT`

## code_verify（506270a）

独立検証（`code_verify` の段。手 1〜7）。HEAD `506270a7c`。壊した複製と変異はすべて元に戻した（開始時と終了時の `git status --porcelain` はどちらも空）。
R の番号は上の R21・R22（未処置）に続けて R23 から振る。

申告: `[x]` 14 / 17（`[ ]` は 10.2・13.1・13.2）。`[x]` の検証コマンドは全部走らせ直した。

| 項目 | 検証コマンド | 実測 | 一致 |
|---|---|---|---|
| 1.1 | `npx vitest run src/__tests__/collectorSetup.test.ts` / `cargo test … st56_setup_string`（`--list` 3 本、下限 2）/ gradle `*St56SetupStringTest*`（testcase 2、下限 2） | rc=0 / rc=0 / rc=0 | 一致 |
| 2.1 | `cargo test -p ashiato-server st56_`（6 passed、`--list` 6、下限 6）/ `./tools/check-openapi.sh` | rc=0 / rc=0 | 一致 |
| 3.1 | `npm run test:e2e -- st56-collector-setup.spec.ts`（4 passed）/ `npx tsc --noEmit && npx vitest run`（516 passed） | rc=0 / rc=0 | 一致 |
| 4.1 | gradle `*St56ConfigTest*`（11、下限 10）/ `:app:testDebugUnitTest` 全体 | rc=0 / rc=0 | 一致 |
| 5.1 | `tools/check-android-backup-rules.sh` / `--self-test` / `grep … ci.yml`（ci.yml:269） | rc=0 / rc=0 / rc=0 | 一致（検査の外側は R27） |
| **5.2** | `scripts/quiet-run emu -- tools/android-emulator.sh` | **rc=1（2 回。2 回目は `gradlew clean` の後）。build cache を使わないと計測テストがコンパイルできない** | **不一致（R23）** |
| 6.1 | gradle `*St56SetupActivityTest*`（13、下限 10） | rc=0 | 一致 |
| 7.1 | gradle `*St56DestinationGateTest*`（27、下限 11） | rc=0 | 一致 |
| 8.1 | gradle `*St56SetupNotifierTest*`（10、下限 8） | rc=0 | 一致 |
| 9.1 | `cargo test … st56_config`（`--list` 11、下限 9）/ `cargo test -p ashiato-collector-windows`（387 passed） | rc=0 / rc=0 | 一致 |
| 10.1 | `cargo test … st56_setupcmd_`（`--list` 20、下限 10）/ `tools/check-licenses.sh` | rc=0 / rc=0 | 一致 |
| 11.1 | `cargo test … st56_gate`（`--list` 16、下限 12） | rc=0 | 一致 |
| 12.1 | `grep … tools/record-env.sh` / `docs/collector-setup.md` の 5 語と `docs/release.md` | rc=0 / rc=0 | 一致 |
| 12.2 | `python3 scripts/check_scenarios.py . st56-collector-connection-setup`（scenarios: OK）/ `tools/check-contract-version.sh` | rc=0 / rc=0 | 一致 |
| 10.2（`[ ]`） | 本物の Windows（WSL から Windows 側の cargo。`C:\dev\ashiato2-rt-st56` に rsync）で `cargo test -p ashiato-collector-windows --test runtime_windows st56_ -- --test-threads=1` | rc=0（1 passed） | —（未了の部分は R30） |

実測: 14 項目のうち 13 項目が一致、**5.2 が不一致**。

### 手ごとの結果

- 手 1: 該当なし（確かめた範囲: `docs/contracts/collector-setup-vectors.json` の正しい 3 件と読めない 5 件を Python の `base64.urlsafe_b64decode` と `json` で読み直し、8 件とも材料の `output` と一致。`buildSetupString` と同じ組み方を Python で書いて `valid[0].input` と一致。3 言語の試験はどれも材料のファイルを読んでいて、期待値を直書きしていない）
- 手 2: R27（バックアップの除外の検査）。ほかに壊した複製: Windows の写しの `secret.rs` を素通し（`protect`・`unprotect` が平文を返す）にすると `st56_dpapi_setup_has_no_plaintext_and_can_send` が「平文が残った: …\ashiato/collector\credential.bin」で FAILED。戻すと PASS
- 手 3: R24・R25・R29（確かめた範囲: 81 本のうち、印の先まで読んだのはサーバ「求めは読み出しの記録に残り、値は残らない」、携帯の R29 の 3 本と保留の一群（St56DestinationGateTest.kt:75・:98・:174・:267・:282・:314 と :372・:385）、PC の保留の一群（destination_gate.rs:280-325）、「既定の置き場を使う」「印を付けると環境変数で動く」「置き場のファイルに資格情報の平文が無い」。残りは `check_scenarios.py` の印の突き合わせ（OK）と手 4 の変異で落ちた試験までで、印の先の主張は 1 本ずつは読んでいない）
- 手 4: 本人の決定の値は試験で固定されている。PC の 6 つ（`AUTH_AFTER` 1h→2h・`RENOTIFY_AFTER` 24h→12h・`UNREACHABLE_AFTER` 24h→12h・置き場 `ashiato/collector`（config.rs:145 だけ）・識別子 `pc-`+12 桁→10 桁・断られの文）と D17 の印（`marked = true ||`）、
  携帯の 6 つ（`AUTH_AFTER_MS` 1h→2h・`REPEAT_MS` 24h→12h・`UNREACHABLE_AFTER_MS` 24h→12h・`UNREACHABLE_TEXT`・D19 の host の小文字化・設定の画面に資格情報を足す）は、どれも 1 つ変えると `st56_*` / `St56*` の試験が落ちた（cargo rc=101 / gradle rc=1）。
  Q7 の門は値でなく**配線**が固定されていない（R24・R25）
- 手 5: R23（5.2）・R28（premises.md）
- 手 6: R26
- 手 7: 該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st56-collector-connection-setup` → `[]`）
- ほかに確かめて問題なし:
  - サーバのログ: 口に `tracing::info!(token = …)` を足すと `st56_collector_setup_logs_request_without_secrets` が落ちる。`debug!` にすると通るが、`lib.rs:372`・`:4410` が `with_env_filter("info")` を直書きしていて本番では出ない
  - `tools/check-contract-version.sh` は「配った印が 0 個（タグ v* が無い）: 印との比較はしない」で、いまは版の定数しか見ていない（比べる相手が空のまま緑）。ST56 のせいではない。封筒を変えていないことの代わりの根拠は `./tools/check-openapi.sh` の欄の数（収集側 要求 21 / 応答 4・生存信号 11・破棄の報告 13）が通ったこと
  - 依存の差分は `crates/collector-windows/Cargo.toml` の `Win32_Security_Cryptography` 1 つだけ（`git diff $(git merge-base HEAD origin/main)..HEAD`。`Cargo.lock`・`build.gradle.kts` は差分なし）
  - Windows で `runtime_windows` 全体も走らせた: 9 passed / 7 failed。落ちた 7 本は前景の窓の観測（:274 `set_focus`・:328・:385「前景が読めない」）で、st56 の試験は単独で PASS。前景の観測は WSL から起こした環境の見込みで、ST56 の指摘にはしない（CI の Windows の job が正）

## R23. 5.2 の検証コマンドが HEAD で rc=1。計測テストのソースがコンパイルできず、build cache が古いクラスを出していた
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OriginalLedger.kt / collector-android/app/src/androidTest/kotlin/dev/ashiato/collector/MediaCaptureInstrumentedTest.kt / tasks.md 5.2
- 根拠: `scripts/quiet-run emu -- tools/android-emulator.sh` → rc=1、`connectedDebugAndroidTest` で 18 本中 12 本が失敗（`gradlew clean` の後の 2 回目も同じ 12 本）。St56BackupTest の段に届かずに台本が止まる。
  失敗のうち 4 本は `NoSuchMethodError: No direct method <init>(ILjava/lang/String;…JJJLjava/lang/String;ILkotlin/jvm/internal/DefaultConstructorMarker;)V in class …Outcome$Responded`。`javap` で見ると、clean の後に作られた `debugAndroidTest` のクラスも `headers` の無い古い構築子を呼んでいる（`org.gradle.caching=true` の cache から戻ったもの）。
  `gradlew :app:compileDebugAndroidTestKotlin --no-build-cache --rerun-tasks` → `e: MediaCaptureInstrumentedTest.kt:236:13 No value passed for parameter 'log'.`（エラーはこの 1 件）。
  原因: `OriginalLedger.kt:426` の `uploader(source, transport, env, log, …)` の `log` の後ろに、ST56 が `canSend: () -> Boolean = { true }`（9f320f674）と `onOutcome: (Outcome) -> Unit = {}`（52ed4d08e）を足した。試験は `log` を末尾のラムダ `) {}` で渡していたので、そのラムダが `onOutcome` に付き、`log` に値が渡らない。
  evidence.jsonl で 5.2 が最後に PASS したのは 6d3d493c2（2026-11-06T14:21Z）で、この 2 つの commit より前。fix の担当は「エミュレータは走らせていない」と書いている。CI の `connectedDebugAndroidTest`（ci.yml:320）は cache の無い runner で同じコンパイルエラーになる見込み。
  ClockSkew・FeelingReminder・MainActivity・LocationService などの残り 8 本の失敗は古いテストのクラスで走った結果なので、ST56 の振る舞いの退行かどうかはまだ分からない
- kind: technical
- 提案: 試験の呼び出しを `log = {}` の名前付きにする（または `log` を末尾へ戻す）。`--no-build-cache` でコンパイルできることを確かめてから、エミュレータを走らせ直し、残り 8 本が通るかを見る。5.2 は通るまで `[ ]` に戻す
- 処置: fixed 5.2 —— `MediaCaptureInstrumentedTest` の `uploader(…)` を `log = {}` の名前付きにした（`--no-build-cache` でなくても、ソースが変わったので cache は効かない）。走らせ直すとコンパイルは通り、18 本中 7 本が落ちた。原因は 2 つで、どちらも ST56 の変化に計測テストが追いついていなかった: (a) ST56 は送り先と資格情報が揃わない組み立てで最初に設定の画面を出す（Q3）。計測テストは `-P` に送り先しか渡さないので、`MainActivity` を開く 6 本（ClockSkew 2・LocationService・MainActivity・Retention 1.9 GB など）が設定の画面で止まり、収集が始まらなかった → 新しい `LaunchMain.kt` の `launchMainSkippingSetup()`（「後で」を押した扱い）で開く。(b) `Outcome.Responded` に ST56 が足した `headers` を `HttpTransportInstrumentedTest` の比べ方が外していなかった → 比べる前に外す。直した後 `scripts/quiet-run emu -- tools/android-emulator.sh` rc=0（全段。St56BackupTest の prepareBackup / verifyRestored も OK）

## R24. PC: 保留の間に `GET /outages` を止める門を、どの試験も見ていない（門を外しても 387 本すべて通る）
- 成果物: crates/collector-windows/src/runtime.rs / crates/collector-windows/src/destination_gate.rs
- 根拠: `runtime.rs:702` の `if !self.destination_enabled { return; }`（poll_outages）を `if false && …` にして `cargo test -p ashiato-collector-windows` → rc=0、387 passed。
  送信の門（`runtime.rs:938`）も同じように外すと、落ちたのは `st56_runtime_config_from_env_is_not_held_by_previous_setup_state` の 1 本だけ（保留の Scenario の印の付いた試験ではない）。
  spec desktop-collection:186 の THEN は「B が受けた要求は … `GET /contract` の 1 件と `GET /healthz` だけで、記録・生存信号・破棄の報告・`GET /outages` は 0 件」。印（destination_gate.rs:280 `st56_gate_different_destination_holds_until_answered` など）は純粋関数 `allows()`・`answer()` の戻り値しか見ていない。主張は HTTP の要求の階層、試験は関数の階層
- kind: technical
- 提案: 未送信がある置き場で送り先 B を保存し、要求を記録する偽の transport と途切れの問い合わせで `Runtime` を回す試験を足す（記録・生存信号・`/outages` が 0 件、y の後は届く）。その試験に Scenario の印を移す
- 処置: fixed 11.1 —— runtime の配線ごと見る試験 `st56_runtime_held_destination_sends_nothing_and_asks_no_outages_until_yes` を足し、Scenario「未送信があるまま送り先を変えると、答えるまで送らない」の印を付けた（2 時間回して POST 0 件・途切れの問い合わせ 0 件、y の後は記録・生存信号・問い合わせが届く）。送信の門（tick と `send_at` の 2 重）と問い合わせの門（tick と `poll_outages` の 2 重）を両方外すと、それぞれこの試験が落ちる（1 つだけ外すと残りの門が守るので落ちない）

## R25. 携帯: 保留の間に途絶と気分の問い合わせを止める配線を、どの試験も見ていない（門を外しても単体試験がすべて通る）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / FeelingReminderReceiver.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/St56DestinationGateTest.kt
- 根拠: `LocationService.kt:123` の `canQuery = { destinationGate.maySend }` を `{ true }` にして `:app:testDebugUnitTest --tests '*'` → rc=0（失敗 0）。`FeelingReminderReceiver.kt:103` の `canQuery = { !DestinationGate(…).blocked }` を `{ true }` にしても rc=0（失敗 0）。
  St56DestinationGateTest.kt:372「OutagePoller は保留中に問い合わせない」と :385「FeelingReminder は保留中にサーバへ聞かない」は、`canQuery = { gate.maySend }` を**試験の側で**渡して組んでいて、アプリが実際に渡す値を通っていない。
  spec device-collection:179・:191（WHEN「送信と途絶の問い合わせと気分の通知の問い合わせの契機を 1 回ずつ回す」）の主張は、アプリの配線の階層にある
- kind: technical
- 提案: `LocationService` を Robolectric で組み（`TestableLocationService` がある）、保留の状態で生存信号の契機を回して途切れの問い合わせが 0 件であることを見る。気分は `FeelingReminderReceiver` の `onReceive` を通して確かめる
- 処置: fixed 7.1 —— アプリが組む門を通す試験を足した: `サービスの途切れの問い合わせは保留中に回らない`（TestableLocationService の生存信号の契機。`canQuery = { true }` にすると落ちる）、`気分の受け手が組む門は保留中に閉じる`（`FeelingReminderWiring.runner(app).canQuery`。`FeelingReminderRunner.canQuery` を internal にした。`{ true }` にすると落ちる）

## R26. PC: `setup-state.json` が壊れると収集アプリが起動せず、`setup` でも直せない（収集が止まる）
- 成果物: crates/collector-windows/src/runtime.rs / crates/collector-windows/src/destination_gate.rs / crates/collector-windows/src/config.rs
- 根拠: 一時的な試験（実行後に消した）で、置き場の `setup-state.json` を `{` にした:
  `Runtime::new` → `Err("setup-state.json を読めない: EOF while parsing an object at line 1 column 1")`。main.rs:171 が `?` で受けて終わるので、収集が止まる。
  同じ置き場で `cfg.save_setup_string(<正しい設定の文字列>)` → 同じ Err（config.rs の `save_setup_string_at` が `store.load()?` を先に呼ぶ）。本人は `setup` で立て直せず、ファイルを手で消すしかない。
  `destination_gate.rs:47-52` の `Store::load` は壊れた中身を Err で返す。数えのファイルは `CounterStore::load_or_quarantine` で退避して進み、`Runtime::new` の注記も「未送信の置き場が読めないときだけは起動しない」と書いている。spec desktop-collection:46 は戻せない資格情報でも「取って積み、送らず」。止まっている間の操作は後から取れない（未送信は残るので kind は technical。失うのは止まっている間の収集）
- kind: technical
- 提案: 壊れた `setup-state.json` を退避して、承認した送り先が無い状態として進める（ファイルは在るので、未送信があれば保留に倒れる）。試験: 壊れた `setup-state.json` で `Runtime::new` が Ok、送らない、`setup` で保存できる
- 処置: fixed 11.1 —— `Store::load` は壊れた `setup-state.json` を `setup-state.broken.json` へ退避し、承認の無い状態を書き直して進む（ファイルは残るので未送信があれば保留）。`Runtime::new` はログに `destination_state_quarantined` を出す。試験 `st56_runtime_starts_with_a_broken_setup_state_and_holds`（起動が Ok・送らない・`setup` で保存できる）と `st56_gate_corrupt_store_is_quarantined_and_stays_closed`（前の `st56_gate_corrupt_store_is_an_error` を置き換え）

## R27. 携帯のバックアップの除外の検査は、除外行を注釈にしても、設定のファイルの名前を変えても通る
- 成果物: tools/check-android-backup-rules.sh / collector-android/app/src/main/kotlin/dev/ashiato/collector/Config.kt
- 根拠: 3 か所の `<exclude domain="sharedpref" path="collector_setup.xml" />` を `<!-- … -->` で囲んだ写しを `ANDROID_BACKUP_RULES_ROOT` で渡す → `android backup rules: PASS` rc=0（`grep -F` が注釈の中も拾う）。
  作業ツリーの `Config.kt:14` の `SETUP_PREFS = "collector_setup"` を `"collector_setup_renamed"` にして（実行後に戻した）`tools/check-android-backup-rules.sh` → PASS rc=0。検査はファイルの名前を直書きしていて、アプリが実際に使う名前と結んでいない。名前がずれると、資格情報が Google のクラウドへ写るのに CI は緑のまま。
  `--self-test` が試すのは行を消す壊し方だけ。振る舞いを見る St56BackupTest はエミュレータでしか走らず、いまは走らない（R23）
- kind: technical
- 提案: 検査で `Config.kt` の `SETUP_PREFS` を読んで期待するファイルの名前を作り、XML の注釈を除いてから見る。`--self-test` に注釈の壊し方と名前の壊し方を足す
- 処置: fixed 5.1 —— 検査は `Config.kt` の `SETUP_PREFS` から期待する名前を作り、XML の注釈（行をまたぐものも）を除いてから見る。`--self-test` に注釈の壊し方 2 つと名前の壊し方を足した。`tools/check-android-backup-rules.sh` rc=0・`--self-test` rc=0

## R28. 5.2 は「premises.md の 1 行目を確かさ 5 で書き直す」が `[x]` だが、1 行目は確かさ 2 のまま
- 成果物: openspec/changes/st56-collector-connection-setup/premises.md / tasks.md 5.2
- 根拠: `premises.md` の 1 行目（Android の自動バックアップ）は、確かめ方が「公式文書を読む」、出所が文書と manifest、確かさが `2`。tasks.md 5.2 の本文は「結果を premises.md の 1 行目に確かさ 5 で書き直す」。HEAD で St56BackupTest は走らない（R23）
- kind: technical
- 提案: R23 を直してエミュレータで St56BackupTest を通し、その結果（実行の日付・commit）で 1 行目を確かさ 5 にする
- 処置: fixed 5.2 —— R23 の直しの後、エミュレータで `St56BackupTest` が通った（`bmgr backupnow` → 消す → `bmgr restore` で未送信は戻り、`collector_setup.xml` は戻らない）。premises.md の 1 行目を確かめ方・結果・出所（実行の日付と commit）とともに確かさ 5 に書き直した

## R29. Scenario の印の先が、Scenario の主張の一部しか見ていない（携帯の 3 本）
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St56DestinationGateTest.kt / St56ConfigTest.kt
- 根拠:
  (1)「未送信があるまま送り先を変えると、押すまで何も送らない」（spec :189-192: 記録 4・生存信号 2・破棄の報告 1、画面に未送信 7 件、B が受けたのは `GET /contract` 1 件だけ）→ 印の付いた St56DestinationGateTest.kt:174 は記録 1 件の `Drainer` で、生存信号と破棄の報告の送り手は数えない（`posts` は記録の transport だけを数える）。画面の件数は見ない。
  (2)「設定の前に積んだ記録は、設定の後に送られ、捨てられない」（spec :108-111: 利用者を焼き込まない組み立てで 2 件、どの行の利用者も nil）→ :282 は 1 件で、記録の利用者を試験が `Config.DEFAULT_USER_ID` で直接入れている。アプリが nil を付けることはこの試験からは見えない（St56ConfigTest の `保存した設定に利用者を持たない` が `Config.userId` を見ているので、穴にはなっていない）。
  (3)「資格情報は画面にもログにも出ない」（spec :33-36: WHEN 設定の画面を開き直し、送信を 1 回行う）→ St56ConfigTest.kt:103 は `toString()` と `SetupStore.save` の `ShadowLog` だけを見る。画面を組まず、送信もしない（画面に資格情報を足す変異は、印の無い St56SetupActivityTest「保存すると『設定済み』と送り先だけを示し…」が落とした）
- kind: technical
- 提案: 印を、主張全体を観測する試験に付け替えるか、試験を spec の件数と WHEN に合わせる（(1) は生存信号と破棄の報告も数える transport、(3) は `SetupActivity` を開き直して 1 回送る）
- 処置: fixed 7.1 —— (1) 印の付いた `サービスのすべての送信経路は保留中に門を通る` に生存信号を積み、生存信号の送信も 0 件であることを見る（破棄の報告は R31 の試験が 3 つの送り手の宛先を全部数える）。(3) 印を `St56SetupActivityTest` の「保存すると『設定済み』と送り先だけを示し…」へ移し、開き直しと送信 1 回の後に `ShadowLog` に資格情報が無いことを足した。(2) は指摘のとおり `St56ConfigTest` の `保存した設定に利用者を持たない` が `Config.userId` を見ているので変えない

## R30. 10.2: 本物の Windows の試験は DPAPI だけで、台本と `--config-from-env` の 2 つは無い。「既存の起動に `--config-from-env` を足す」は足す先が無い
- 成果物: crates/collector-windows/tests/runtime_windows.rs / tasks.md 10.2
- 根拠: Windows で `cargo test … --test runtime_windows st56_` → 1 passed（`st56_dpapi_setup_has_no_plaintext_and_can_send` だけ）。DPAPI を素通しにした写しでは落ちるので、この 1 本は振る舞いを固定している。
  一方 `grep -n "ASHIATO_\|config-from-env\|CONFIG_FROM_ENV\|install-autostart" crates/collector-windows/tests/runtime_windows.rs` → 0 件（`Command::new` の相手は powershell・cmd・ブラウザ（:990・:1430・:1468）・taskkill・sc・w32tm だけで、収集アプリの実行ファイルを起こす試験が無い）。
  10.2 が挙げる「`--install-autostart` が書いた `ashiato-collector.cmd` に `ASHIATO_API_TOKEN` が無い」「`--config-from-env` を付けた既存の起動が今どおり動く」の 2 つを持つ試験が無い。10.2 は `[ ]` なので申告とは食い違っていない
- kind: technical
- 提案: `--install-autostart` を一時的な Startup の置き場で走らせて `.cmd` を読む試験を `runtime_windows` に足す。`--config-from-env` は、収集アプリを起こす Windows の試験が無いので、10.2 の本文から外すか、起動を 1 本足す
- 処置: fixed 10.2 —— `runtime_windows` に `st56_install_autostart_script_has_no_credentials`（本物の `autostart::install` を一時的な `APPDATA` で走らせ、`.cmd` に資格情報・送り先が無い）と `st56_config_from_env_marker_works_on_windows`（印の起動の入口 `Config::from_sources` を Windows で通す）を足した。収集アプリの実行ファイルを起こす試験はこの台本に無いので、「既存の起動に `--config-from-env` を足す」は入口の試験で代える（PR 本文に書く）。Windows（WSL から `C:\dev\ashiato2-rt-st56`）で `cargo test -p ashiato-collector-windows --test runtime_windows st56_ -- --test-threads=1` → 3 passed。10.2 は CI の担当なので `[ ]` のまま

## security review（7ad55e1..506270a）

見た経路:
- 画面（cookie）→ `POST /api/collector-setup` → `credential_exempt`（collector_scope.rs:33）と `access_log::middleware` の素通し（access_log.rs:166）→ `collector_setup_post` が自分で `decided` → `Caller::WebSession` 以外は 401/403 → 失敗の数えを共有 → 合言葉の照合 → `{"token"}`（no-store）。`cargo test -p ashiato-server st56_` → 6 passed（全部 `crate::router()` を通す。印なし 401・`Bearer` の共有の資格情報で 403・合言葉違いで 403・数えの共有で 429・200 に `cache-control: no-store`）（4）
- 衛星の資格情報 → `authorize_quiet` が `Caller::Satellite` を返す（lib.rs:699-713）→ web_session.rs の `caller != Caller::WebSession` で 403（3）
- よそのサイトからの送信 → 印の cookie は `SameSite=Strict`（web_session.rs:256）で付かない。付いても合言葉の入れ直しが要る（3）
- 返す値は共有の API の資格情報 → `access_log.rs:179` で `COLLECTOR_ALLOWED`（送るだけの 9 口）の外は 403。読む口には届かない（2）
- 画面: 資格情報は React の state にだけ持ち、`localStorage` には書かない。送り先は `window.location.origin` から作る（CollectorSetupPanel.tsx）（2）
- 携帯: 貼った文字列 → `parseSetupString` → `SetupStore.save` が https 以外と loopback を断る → `collector_setup.xml` は cloud-backup・device-transfer・full-backup の 3 つから外す（res/xml）。`SetupActivity` は `exported=false`、通知の PendingIntent は明示の Intent で `FLAG_IMMUTABLE`（2）
- 携帯の送信: `Drainer.tick` が保留を見るのは契機の頭の 1 回だけ。続けて送る周回は、要求ごとに `Config.baseUrl`/`apiToken` を読み直す（HttpTransport の lambda）→ R31（4）
- 携帯の写真の原本: `OriginalUploader.run` が `canSend()` を見るのは頭の 1 回だけ（OriginalUploader.kt:132）→ `pass` の周回は 1 件ずつ送り先を読み直す（LocationService.kt:242）→ R31（3）
- PC: `setup` → `save_setup_string_at`（`ensure_no_cleartext_off_loopback`）→ setup-state.json → settings.json（資格情報は `skip_serializing`）→ DPAPI の `credential.bin`。自動起動の台本は `ASHIATO_STATE_DIR`/`ASHIATO_DEVICE_ID` の 2 つだけ（autostart.rs）（3）
- PC の送信: 見回りの糸 1 本の中で `reload_from_disk` → `transport.reload` → `rt.reload_config`（門の計算をやり直す）→ `tick` の順（main.rs:230-245）。送っている途中で送り先が変わることはない（3）

安全の決め手: サーバの新しい口は middleware の認可を外れているが、`collector_setup_post` が自分で「画面のログイン＋合言葉の入れ直し」だけを通す —— 確かさ 4。`cargo test -p ashiato-server st56_` は 6 本とも `router()` を通して通る（`-- --list` の代わりにログで 6 本を確かめた）。携帯の送り先の保留には、この 1 つにあたる決め手が**無い**（R31）

## R31. 携帯: 溜まった分を続けて送っている最中に別の送り先を貼ると、「この送り先へ送る」を押す前に、残りの記録と写真の原本を新しい送り先へ送る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt / OriginalUploader.kt / HttpTransport.kt
- 根拠: Retention.kt:252 は `destination?.blocked` を `tick()` の頭で 1 回だけ見て、`while (true)` の周回（:256-284。90 日分なら 648 回）では見直さない。`HttpTransport` は要求ごとに `Config.baseUrl`/`Config.apiToken` を読む（LocationService.kt:249）。`SetupStore.save` は URL・資格情報・`BLOCKED` を 1 つの commit で書く（R1 の直し）が、走っている周回はその `BLOCKED` を読まない。
  仮の Robolectric 試験で走らせた（走らせた後に消した）: A を承認 → 未送信 650 件 → 1 回目の POST の中で `SetupStore(app).save(B)` → `Drainer(..., destination = gate).tick()`。
  出力: `save B: SAVED blocked=true` / `batch=200` ×3 / `rounds=3` / `post to https://a.example/api|token-a : 1` と、同じ契機の残り 2 回（400 件）が `https://b.example/api|token-b` へ届いた。終えた後も `blocked=true approved=https://a.example/api`（押していない）。
  写真の原本も同じ形: OriginalUploader.kt:132 の `canSend()` は `run` の頭だけで、`pass`（:166-）は 1 件ずつ `{ Config.baseUrl }` で送る（確かさ 3）。原本の糸は記録の後に別の糸で走り、大きい写真を続けて送るので、隙間は記録より長い。
  端末を拾った誰かが自分のサーバの設定の文字列を貼った場合も同じで、Q7 の「押すまで 1 つも送らない」の守りが、続けて送っている間は効かない（確かさ 4）
- kind: technical
- loss: exported
- 提案: 送る契機の頭で送り先と資格情報を取って、その契機の間は固定する（`HttpTransport` に lambda でなく値を渡し直す）。あわせて、周回ごと（`Drainer` の while の頭・`OriginalUploader.pass` の 1 件ごと）に `blocked` を見直す。
  上の手順（1 回目の POST の中で B を保存すると、B への POST が 0 件）を St56DestinationGateTest に足す
- 処置: fixed D25 仮 —— 門を 3 か所で見る: `Drainer` の周回と 4 つの送り手の 1 本ごと（Retention.kt `held`）、`OriginalUploader.pass` の 1 件ごと、送る要求の資格情報を読んだ直後（LocationService.kt `sendingToken`。送り先 → 資格情報 → 門の順）。試験 `続けて送っている最中に別の送り先を保存すると残りを送らない`（650 件・1 回目の POST の中で B を保存 → 宛先は A の 1 回だけ、保留のまま。`held` を外すと落ちる）。Q7 の向き（`--follows Q7`）で `provisional.py add` 済み

## R32. 作業場の `unsafe_code = "deny"` は、どのファイルの `#[allow(unsafe_code)]` も止めない。「許すのは secret.rs だけ」を機械が見ていない
- 成果物: Cargo.toml（`[workspace.lints.rust]`）
- 根拠: Cargo.toml:27 を `forbid` から `deny` へ。server・build-stamp も `[lints] workspace = true` で同じ値を受ける。
  `rustc -D unsafe_code` に `#![allow(unsafe_code)]` 付きの unsafe を渡すと rc=0（通る）。`-F` なら `error[E0453]: allow(unsafe_code) incompatible with previous forbid`（確かさ 4）。
  `grep -rn "allow(unsafe_code)" crates/` → secret.rs:3 の 1 件だけ。`tools/` と `.github/` に `unsafe_code` を見る検査は無い。R18 の処置で直したのは注記だけ。外から入力を受けるサーバ（取り込み・書庫）にも、これから unsafe を足せば CI を通る
- kind: technical
- 提案: 作業場は `forbid` に戻し、collector-windows だけ `[lints]` を作業場から受けずに自分で並べて `deny` にする。または `#[allow(unsafe_code)]` が `crates/collector-windows/src/secret.rs` の外に出たら落ちる `tools/check-*.sh` を CI に足す
- 処置: fixed 10.1 —— 新しい `tools/check-unsafe-scope.sh`（`allow`/`expect`/`warn(unsafe_code)` が `crates/collector-windows/src/secret.rs` の外にあれば落ちる。`--self-test` つき）を CI の server の job に足した（ci.yml の `check-satellite-boundary.sh` の後）。作業場を `forbid` に戻すと collector-windows が `[lints]` の全表を写すことになるので、検査で代える

## /code-review（7ad55e1..506270a）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R33. While the PC destination is held, the tick still runs notice_kind on every pass. setup_saved_at is set, but nothing is sent while held, so first_response_at stays None and a false '送り先に届きません' fires after 24h. The /contract probe that save_and_check makes at save time is not passed to observe_response either.
- 成果物: crates/collector-windows/src/runtime.rs:503
- 根拠: The user pastes a new URL while records are still unsent and answers N (or closes the window). save_and_check reports '繋がりました'. The gate holds, so no request is made. 24h after the save, emit_destination_notice shows UNREACHABLE_TEXT for a server that just answered. Android does not do this because Drainer.tick returns before setupNotifier.update() while blocked.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 11.1 —— R21 と同じ直し（閉じた門では「届かない」を数えない・確かめの答えを `observe_response` へ）

## R34. save_setup_string_at writes settings.json (the new base_url) before credential.bin (the new token). The running collector calls reload_from_disk on every tick, so it can read the new URL together with the old token.
- 成果物: crates/collector-windows/src/config.rs:298
- 根拠: A PC is set up for server A with token TA. The user runs `setup` with a string for server B and nothing is pending, so the gate takes the Send path and approves B first. If a tick lands between the two writes, reload_from_disk returns changed=true and transport.reload(B, TA) runs. The next send then carries `Bearer TA` to server B, which leaks A's credential to a different server and starts the 401 count. Writing credential.bin first (or both in one file) closes the gap.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 10.1 —— `credential.bin` の中身を `{url, token}` にして送り先に結んで包む。読み直しは、結んだ送り先と `settings.json` の送り先が揃わなければ読まない（次の見回りで揃ったものを読む）。起動のときに揃っていなければ資格情報なし（未設定）。保存は資格情報を先に書く。ST56 の途中の形（資格情報だけの包み）は今どおり読む。試験 `st56_config_reload_does_not_pair_a_new_url_with_an_old_credential`

## R35. On a release launch, from_sources rewrites settings.json from SettingsFile. legacy_api_token is skip_serializing, so the rewrite drops the legacy `api_token` without moving it into credential.bin. The next reload_from_disk (on the first tick) then loads an empty token.
- 成果物: crates/collector-windows/src/config.rs:193
- 根拠: settings.json is {"base_url":"https://x","api_token":"T"} and credential.bin does not exist. from_sources returns api_token=T but has already rewritten the file without api_token. The first loop iteration calls reload_from_disk, which sets api_token to "" with changed=true. transport.reload sends an empty Bearer, needs_setup becomes true and the setup toast appears. The test st56_config_saved_credentials_are_selected_without_marker checks only the first load, so it does not catch this.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.1 —— `from_sources` は `settings.json` を書き直す前に、平文の `api_token` を `credential.bin`（送り先に結んで）へ移す。試験 `st56_config_legacy_token_survives_the_settings_rewrite`（書き直した `settings.json` に平文が無く、読み直しても資格情報が残る）

## R36. save_destination_state merges by hand: the collector does a read-modify-write and copies only approved_url from disk, then writes its own stale destination_notified_at back over the clear that the setup process just made. Two processes are writing the same JSON with no lock.
- 成果物: crates/collector-windows/src/runtime.rs:1047
- 根拠: A held collector loads the state in reload_config. The user answers y in the `setup` window, and approve_destination clears destination_notified_at. Before the next reload, the collector's emit_destination_notice calls save_destination_state: approved_url is copied, but destination_notified_at=Some is written back. Sending resumes, but destination_notice() stays true, so '送り先の確認を待っています' reappears every 24h. Fix: split the setup-owned fields and the collector-owned counters into separate files, or have the collector clear destination_notified_at itself once approved_url matches.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 11.1 —— 見回りが状態を書くとき、承認と保留（`approved_url`・`destination_notified_at`）は置き場の値を取る（`setup` の持ち物）。保留が解かれていれば保留の知らせの時刻も消す。試験 `st56_runtime_does_not_write_back_a_hold_answered_in_setup`（y の後に見回りが知らせを書いても保留を書き戻さず、その後「確認を待っています」を出さない）

## R37. On the Ask path, save_setup_string_at sets destination_notified_at but does not clear destination_notice_at, so a fresh hold does not notify right away. destination_gate::answer and Android holdOnSave (which removes NOTIFIED_AT) both reset it.
- 成果物: crates/collector-windows/src/config.rs:280
- 根拠: A hold for URL B was notified 2h ago and destination_notice_at=T-2h. The user saves URL C, which also asks. destination_notice() is false until T+22h, so the PC does not prompt for the new hold for almost a day, even though the spec says the hold prompts with a notice.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 11.1 —— `save_setup_string_at` の保留の枝で `destination_notice_at` を消す（`answer` と携帯の `holdOnSave` と揃う）。試験 `st56_config_new_hold_is_notified_at_once`

## R38. On every tick (every foreground-window change), the main loop does all of the following without caching or a cheap change check: re-reads settings.json, reads credential.bin and decrypts it with DPAPI (CryptUnprotectData), loads setup-state.json, and reads all of outbox.jsonl, heartbeat.jsonl and drops.jsonl through pending_in. The settings_mtime and credential_mtime fields are stored but never used to skip unchanged files.
- 成果物: crates/collector-windows/src/main.rs:231
- 根拠: With a long backlog (thousands of outbox lines after days offline), every window switch re-reads and re-counts the whole outbox and makes a DPAPI call. That is wasted I/O and CPU on the collector's hot path. Comparing the stored mtimes before reloading, and counting with in-memory Outbox lengths, would avoid it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.1 —— `reload_from_disk` は `settings.json` と `credential.bin` の更新時刻が前と同じなら読まない（DPAPI で戻さない）。未送信の数え直しは R39

## R39. pending_count re-reads the outbox files from disk through destination_gate::pending_in, although Runtime already holds the same queues in memory (self.events / self.beats). Runtime::new itself uses events.len()+beats.len(). pending_in also duplicates pending_summary().count.
- 成果物: crates/collector-windows/src/runtime.rs:1122
- 根拠: The pending count is computed in two different ways. The disk version also counts drops.jsonl, which the in-memory version does not, so the two can disagree. It also adds file I/O on every reload and send. Using self.events.len() + self.beats.len() + self.unsaved.len() (plus drops if needed) would avoid both.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 11.1 —— `pending_count` は記録と生存信号を手元の待ち行列（`events`・`beats`・`unsaved`）から数え、破棄の報告だけを置き場の `drops.jsonl` から数える（`destination_gate::pending_drops`）。`Runtime::new` も同じ数え方にした。`setup` の別のプロセスは手元の待ち行列を持たないので `pending_in` のまま

## R40. collector_setup_post leaves out two checks that session_post makes before using the password: the application/json Content-Type check (415) and the empty-password rejection. Wrong passwords on this endpoint also count against the shared login limiter.
- 成果物: crates/server/src/web_session.rs:400
- 根拠: A simple text/plain POST is accepted on an endpoint that returns the collector token, which /session deliberately refuses (only SameSite=Strict on the cookie stands in for that). Also, a few mistyped passwords in the collector-setup panel throttle the main web login for 60s.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.1 —— `collector_setup_post` に JSON でない本文の 415 と空の合言葉の拒否を足した（`is_json` をログインの口と共有。415 は読み出しの記録に `forbidden` で残す。`docs/openapi.json` を作り直した）。試験 `st56_collector_setup_rejects_non_json_and_empty_password`。失敗の数えをログインの口と共有するのは tasks 2.1・design D22（仮）の決定なので変えない

## R41. reconcile returns 'not blocked' when nothing is pending, but when normalize(url) is null it calls neither approve nor clearPending. The BLOCKED=true that holdOnSave wrote stays set, and approvePending cannot clear it because PENDING_URL was removed.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DestinationGate.kt:61
- 根拠: The saved URL does not survive normalize (for example, an unusual authority that java.net.URI parses with host=null). holdOnSave sets BLOCKED=true with no PENDING_URL, and reconcile with count=0 returns false. Drainer.tick returns 0 on every pass and the setup screen's 'この送り先へ送る' does nothing, so sending stops permanently.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— 門の比べる鍵を `DestinationGate.key`（`normalize` で読めなければ前後の空白を除いたそのままの文字）にし、`reconcile`・`holdOnSave`・`approve` が同じ鍵を使う。読めない送り先でも 0 件なら解け、あれば「この送り先へ送る」で解ける。試験 `読めない送り先で保存しても保留が解ける`

## R42. exclusions_path treats any state_dir whose last component is named 'dev' as the dev-build directory and reads exclusions.json from its parent, including directories set by --config-from-env or ASHIATO_STATE_DIR.
- 成果物: crates/collector-windows/src/config.rs:310
- 根拠: A release or env-launched collector with ASHIATO_STATE_DIR=C:\data\dev loads C:\data\exclusions.json instead of C:\data\dev\exclusions.json. The user's exclusions are silently ignored, so excluded windows are collected and sent. Deciding this from is_dev (the version) instead of the directory name would avoid it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.1 —— 除外の登録の置き場を置き場の名前で決めず、`Config.shared_exclusions_dir`（開発版のときだけ正式版の置き場）で決める。試験 `st56_config_release_state_dir_named_dev_keeps_its_own_exclusions`

## finish の scoped re-review（506270a + 作業ツリー）

finish の直し（R21〜R42）の差分を、直した文脈から独立の席（`pr-review-toolkit:code-reviewer`）が 1 回見た。R21〜R28・R31〜R42 は ADDRESSED。R29 は PARTIAL（(1) の印の先は生存信号まで。破棄の報告 1 件と画面の未送信 7 件は印の先では見ていない —— 破棄の報告の宛先は R31 の試験が数える。残りは park: 門は送り手ごとに同じ `held()` を通るので、送り手を 1 つ足しても主張の型は変わらない）。R30 は PARTIAL（tasks.md 10.2 の本文の「既存の起動に `--config-from-env` を足す」は残した。10.2 は CI の担当で `[ ]` のまま。足す先が無いことは PR 本文に書く）。
新しい指摘は Important 1・Minor 2（R43〜R45）。R43 は finish の R26 の直しが持ち込んだ止まり方の変化（大きな音 → 黙って止まる）なので、fix 1 回の規律から外れるが、直し・試験まで済ませた（再レビューはしていない。PR 本文に書く）。

## R43. PC: 壊れた `setup-state.json` を退避すると、知らせが何も出ないまま送信がいつまでも止まる
- 成果物: crates/collector-windows/src/destination_gate.rs
- 根拠: scoped re-review —— 退避は `State::default()` を書く。未送信が 1 件でもあれば `allows` は閉じるが、`destination_notified_at` が None なので保留の知らせは出ず、「届かない」は閉じた門で消え、`needs_setup()` も偽。本人は `setup` を自分から走らせるまで気づけない
- kind: technical
- 提案: 退避したら `destination_notified_at = Some(now)` で保留から始める
- 処置: fixed 11.1 —— 退避した状態は保留から始め、次の見回りで「送り先の確認を待っています」を出す（`setup` の y で承認し直す）。試験 `st56_runtime_starts_with_a_broken_setup_state_and_holds` に知らせを足し、`st56_gate_corrupt_store_is_quarantined_and_stays_closed` は `destination_notice` が真であることを見る

## R44. PC: `setup` の確かめが書いた「届いた」を、見回りの古い状態の書き戻しが消しうる
- 成果物: crates/collector-windows/src/runtime.rs / crates/collector-windows/src/setup.rs
- 根拠: scoped re-review —— `save_and_check` は保存の後に 2 度目の読み書きで `first_response_at` を書く。その間に見回りが読んだ状態で `save_destination_state` に入ると、`setup_saved_at` が等しいので else の枝に入り、`first_response_at = None` で書き戻す
- kind: technical
- 提案: 合わせ方で `first_response_at` も置き場の値を残す
- 処置: fixed 11.1 —— else の枝で `first_response_at` は手元の値が無ければ置き場の値を取る（`or`）

## R45. `tools/check-unsafe-scope.sh` は、行をまたぐ属性と、crate の `Cargo.toml` での上書きを素通りする
- 成果物: tools/check-unsafe-scope.sh
- 根拠: scoped re-review —— 1 行の grep だけ。`#[allow(\n unsafe_code)]` と、`[lints] workspace = true` を外す・`[lints.rust] unsafe_code = "allow"` は緑のまま
- kind: technical
- 提案: 各 crate の `Cargo.toml` が `workspace = true` を持ち `unsafe_code` を上書きしないことも見る
- 処置: fixed 10.1 —— 属性はファイル全体で（perl -0）見る。`crates/*/Cargo.toml` が `[lints] workspace = true` を持ち、`unsafe_code` を書かないことを見る。`--self-test` に行をまたぐ属性と `Cargo.toml` の上書きを足した（どちらも rc=0）

## R46. 携帯: 送り先の突き合わせが未送信の置き場を全部メモリに載せ、溜まった端末（1.9 GB）でアプリがメモリ切れで落ちる
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: finish の検証（エミュレータの 3 回目）—— `RetentionInstrumentedTest.nearlyTwoGigabytesStillStartsAndKeepsCollecting` でプロセスが `OutOfMemoryError`（logcat: `target footprint 201326592 … giving up on allocation`）で落ち、計測の実行が止まった。`countBacklog()`（Task 7）は 4 つの置き場を `snapshot()`（`readAll`）で全部読み込んで件数と期間を数える。突き合わせはサービスの起動と設定の保存のたびに回るので、90 日分が溜まった端末では起動のたびに落ちうる（2 回目の実行では通った —— 載せきれるかは時の運）
- kind: technical
- 提案: 1 件ずつ読んで件数・最古・最新だけを持つ
- 処置: fixed 7.1 —— `Outbox.span`（`SegmentStore.forEachItem` で 1 件ずつ読み、溜めない）で数える。試験 `未送信の件数と期間は置き場を 1 件ずつ読んで数える`。エミュレータの全段を走らせ直した（PR 本文の検証の表）

## R47. 携帯: `-P` の送り先を替えて組み直した APK は、未送信があると前の `-P` の承認が残って保留になり、何も送らない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: finish の検証（エミュレータの 4 回目）—— `RetentionInstrumentedTest.ninetyDaysOfBacklogStartsAndDrainsWhileCollectionContinues` が「送り切れていない（0 / 129600）」で落ちた。前の計測テストが `-P` の送り先（`http://127.0.0.1:18787`）を承認済みにし、この試験が上書きした送り先（偽のサーバの port）は「承認と違う・未送信 129600 件」で保留に倒れた。`reconcileDestination` は `-P` の送り先を、承認が 1 つも無いときしか承認しない。開発の携帯で `-P` の値を替えて組み直すと同じことが起き、Global Constraints「`-P` で焼き込んだ APK は今どおり送る」と D13（`-P` の送り先は承認済み）に反する（2 回目の実行で通ったのは試験の順と未送信の有無の巡り合わせ）
- kind: technical
- 提案: 保存した設定が無い間は、`-P` の送り先を毎回承認する
- 処置: fixed 7.1 —— 保存した設定が無い間は `Config.buildBaseUrl` を毎回承認する（保存した設定があれば今どおり通常の門で突き合わせる）。試験 `組み立ての送り先を替えて組み直しても、保存した設定が無ければ今どおり送る`（未送信を積んでから `-P` を替える。直しを外すと落ちる）。エミュレータの全段を走らせ直した
