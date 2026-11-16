# ST58 settle report

## R1 / R13 / R14 / R15 / R16 / R17 / R18 / R19 / R20 / R9 / R10 / R12

- `record-rollback` の CLI 組み立てに版と種別を渡し、`replay` を positional path に合わせた。控えが無い replay は no-op にした。
- サーバに `--version` を実装し、既存インストールの版読取りを可能にした。
- 更新・戻しの停止を停止待機の既存コマンドに置換し、DB 再起動後は接続可能になるまで待つ。プログラム移動の前に `moved` 段を永続化し、待受後の失敗は rollback と記録しない。
- Android は置換開始印を cleanup から除外し、通知で発行した一回限りのトークンだけが更新を開始する。
- 配布物検査に `ASHIATO_SERVER_TEST_` を追加し、P20 は Windows CI 未実行のため確かさ 3 に戻した。

## R11

- A: 失敗 DB の写しを保持するか消すかは、本人の削除した記録と調査材料のどちらかを失う選択である。Q10 を `deep.md`、`deep-questions-r3.json`、`docs/briefs/ST58-deep-r3.html` に作成した。

## 検証

- `cargo test -p ashiato-setup st58_` — PASS（1 passed）
- `cargo test -p ashiato-server st58_` — PASS（0 tests selected）
- `cargo test -p ashiato-collector-windows st58_` — PASS（19 passed）
- Android unit test は Gradle が実行中に応答を返さず、完了結果は未取得。

## 追加の settle（R1 / R4 / R5）

- R1: command builder による `record-rollback` の全引数と、終了コード 5 の pending 経路を固定した。
- R4: `migrate` の機械可読な失敗行から種別・落ちた移行名・SQLSTATE を outcomes の理由へ写すようにした。
- R5: 再開した戻しでも理由付き `rolled_back` を書く既存修正を、status の読み取り試験と照合した。
- 検証: `scripts/quiet-run st58-settle-r1-r4 -- cargo test -p ashiato-setup st58_` — rc=0、2 passed。

## 処置記録の整備（R1〜R10 / R12〜R22）

- 直近の recovery 修正コミットに対応する R1・R4・R5・R9・R10・R12〜R20 の処置を、自己参照ではなく設計決定または change 直下の成果物を指す形式にした。
- R2・R3 は Windows 通しが採る実状態と self-test の判定範囲、R6〜R8 は PC/携帯/状態の既存検証経路、R21・R22 は更新実行経路へそれぞれ対応付けた。
- R8 の既存の詳細な壊し試験を triage が根拠として認識できる形式に補った。R11 は既に Q10 として本人の判断を待つため、この回では変更していない。

## 検証（この回）

- `python3 scripts/review_triage.py . st58-update-apply-rollback --no-verdict` — rc=0。
- `scripts/quiet-run st58-settle-installer -- cargo test -p ashiato-setup st58_` — rc=0、2 passed。
- `scripts/quiet-run st58-settle-pc -- cargo test -p ashiato-collector-windows st58_` — rc=0、19 passed。
- `tools/release/update-apply-install.sh --self-test` — rc=0。

## 機械検査の追補（R6 / R8 / R21 / R22）

- R6: 指紋不一致の PC 配布物を loopback の実更新経路へ渡し、失敗時に `versions/0.5.0` を作らないことを確認する試験へ替えた。
- R21: 生存印の無い収集アプリには handoff を発行・待機せず、直ちに staged 版の起動へ進む。生存印のある既存プロセスには従来どおり停止確認を求める。
- R22: state.json の段名から `UpdateStage` への変換を `update_plan` に集約し、実行経路が同じ復旧段の定義を使うようにした。
- R8: 追加した根拠行は、機械検査の指示に従い元へ戻した。

## 検証（追補）

- `scripts/quiet-run st58-settle-followup -- cargo test -p ashiato-collector-windows st58_` — rc=0、19 passed。
- `scripts/quiet-run st58-settle-plan -- cargo test -p ashiato-setup st58_` — rc=0、2 passed。

## 追加の settle（R2 / R3 / R7 / R23 / R24）

- R2/R3/R23: Windows 台本は自己申告の文字列を廃止し、設定指紋・退避と履歴の件数・失敗 DB 数・サービス状態・実行した DB 所有者を実状態から採取して判定する。自己試験は Scenario 判定関数の成功観測と壊れた観測を直接通す。
- R7: Android の unit test は `docs/contracts/update-apply-vectors.json` を Gradle test resource として取り込み、Android 向けの全ベクターとリダイレクト規則を検査する。開発版 APK の `ashiato-collector-dev-` 接頭辞も実装側で選べるようにした。
- R24: 移行失敗の kind/version/sqlstate を更新印に保存し、戻し履歴を `migration-failed` として再開時にも同じ値で記録する。

## 検証（追加の settle）

- `tools/release/update-apply-install.sh --self-test` — rc=0。
- `scripts/quiet-run installer-st58 -- cargo test -p ashiato-setup st58_` — rc=0、2 passed。
- `scripts/quiet-run installer-lib-st58 -- cargo test -p ashiato-setup --lib st58_` — rc=0、28 passed。
- `scripts/quiet-run android-compile -- ./collector-android/gradlew -p collector-android :app:compileDebugUnitTestKotlin` — rc=0。
- `scripts/quiet-run android-st58 -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St58UpdateInstallTest*'` — 前回は共有ベクター検査で開発版 APK の接頭辞不一致を検出。接頭辞を修正後、Gradle 実行が 30 秒でハーネスから切れ、完了結果は未取得。

## 追加の settle（R3 / R23 / R25 / R26 / R27）

- R3/R25: rollback 後に別接続で `current_user` を読む判定を廃止した。`record-rollback` に実際に渡す `DATABASE_OWNER_URL` の利用者名を trace し、移行失敗を起こして戻す Scenario が `ashiato_owner` を確認する。
- R23: 更新確認は trace の出ない語を grep せず、`check.json` の `enabled` を読み取る。窓の 2 Scenario は installer の日本語出力「戻しました」「入りませんでした」を確認する。自己試験はそれぞれの判定関数を通す。
- R26: 移行失敗の詳細と終了コードを state.json に反映する経路を、`.writing` へ完全な JSON を書いて rename する原子的更新へ統一し、失敗は呼出元に返すようにした。
- R27: 共有ベクターを通す Android Robolectric 試験を含め、St58 の unit test を完走した。

## 検証（R3 / R23 / R25 / R26 / R27）

- `scripts/quiet-run update-apply-self-test -- tools/release/update-apply-install.sh --self-test` — rc=0。
- `scripts/quiet-run installer-st58 -- cargo test -p ashiato-setup st58_` — rc=0、4 passed。
- `scripts/quiet-run android-st58 -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St58*'` — rc=0、BUILD SUCCESSFUL（55s）。
- `cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St58*.xml | grep -c '<testcase '` — 22 以上、rc=0。
- `rustfmt --check crates/installer/src/main.rs` / `git diff --check` / `python3 scripts/review_triage.py . st58-update-apply-rollback --no-verdict` — すべて rc=0。

## R27 の成果物コミット

- `St58UpdateInstallTest` に、開発版が正式版の接頭辞を選ばず `ashiato-collector-dev-` の APK を選ぶ試験を追加した。
- `scripts/quiet-run android-r27 -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St58UpdateInstallTest*'` — rc=0、BUILD SUCCESSFUL（82s）。

## 追加の settle（R23 / R24 / R28 / R29）

- R23/R28: 窓に「戻しました」「入りませんでした」を求める 2 Scenario に、移行失敗と移行中断を実際に起こす hook を追加した。自己試験は手書きの evidence 出力を使わず、各 hook を通した偽インストーラの失敗出力を Scenario 判定に渡す。
- R24/R29: `finish_restored` と通常の `rollback` を同じ state ベースの履歴コマンド組み立てに揃えた。再開した移行失敗でも `migration-failed`、落ちた版、SQLSTATE を維持する。

## 検証（R23 / R24 / R28 / R29）

- `scripts/quiet-run st58-settle-selftest -- tools/release/update-apply-install.sh --self-test` — rc=0。
- `scripts/quiet-run st58-settle-installer -- cargo test -p ashiato-setup st58_` — rc=0、5 passed。
- `scripts/quiet-run st58-settle-script -- bash -n tools/release/update-apply-install.sh` — rc=0。
- `git diff --check` / `python3 scripts/review_triage.py . st58-update-apply-rollback --no-verdict` — rc=0。

## 追加の settle（R23 / R28 / R30）

- `STOP_AT=migrating` の停止位置を migrate の後から前へ移し、実際に「移行の前に止まる」状態を作るようにした。
- 本物の `update_failure_reason` はこの停止を `not_applied` として分類する。これにより窓の文は「入りませんでした」となり、自己試験の偽インストーラの分岐と一致する。
- `st58_update_failure_reason_marks_stop_before_migration_not_applied` で本物の分類契約を固定した。

## 検証（R23 / R28 / R30）

- `scripts/quiet-run st58-settle-r23-r28-r30 -- cargo test -p ashiato-setup st58_` — rc=0、6 passed。
- `tools/release/update-apply-install.sh --self-test` — rc=0。
- `bash -n tools/release/update-apply-install.sh` / `git diff --check` — rc=0。
# settle 2026-11-16 — R31–R46

- R31–R34, R36, R42–R44: インストーラの更新・復旧状態を補正した。`previous` を先に作成し、`failed` を移動対象から除外した。入替後は `swapped` 段を残すため、再開で同じ入替を繰り返さない。初期段の復旧は DB サービスも起動し、更新係は既存設定を変更せず有効時だけ起動する。
- R33: 古いサーバの `--version` 非対応を互換基準版として扱うようにした。
- R35, R40: 試み行がない更新履歴では失敗行を追加せず、戻した行の記録を継続する。
- R37, R45: PC の失敗文を仕様の文言に揃え、通常起動で自動起動台本を上書きしないようにした。
- R39: Android の収集再開待ち通知は、更新取得を開始せず通常の画面を開くようにした。
- R41, R46: migrate の機械可読な失敗種別は全出力から読むようにし、待受失敗の通知を移行失敗から分離した。
- 検証: `scripts/quiet-run st58-settle-rust-all -- bash -c 'cargo test -p ashiato-setup st58_ && cargo test -p ashiato-server st58_history_ && cargo test -p ashiato-collector-windows st58_pc_'` — rc=0（setup 6件、collector 19件）。

## 追加の settle（R37 / R38 / R47 / R48）

- R37: Android は提供元不明の許可待ちと APK の取得・照合失敗を永続化し、メイン画面の更新行で「許可した後に、もう一度押してください」または「入れられませんでした（理由）」として表示する。
- R38: 製品を呼ばない恒真式を削除し、loopback・実際の staged program・handoff を通す PC 更新試験で、更新確認の記録と未送信が変わらないことを観測する。
- R47: 更新で program を入れ替えた段と、rollback で前の program を戻した段を `program_swapped` / `program_restored` に分離し、更新直後の中断でも rollback がファイルを戻すようにした。
- R48: 更新印への migrate 詳細の補足に失敗しても、元の migrate エラーを返して `rolled_back` の分類を維持する。

## 検証（R37 / R38 / R47 / R48）

- `scripts/quiet-run settle-installer -- cargo test -p ashiato-setup st58_` — rc=0、7 passed。
- `scripts/quiet-run settle-pc -- cargo test -p ashiato-collector-windows st58_pc_` — rc=0、14 passed。
- `scripts/quiet-run settle-android-2 -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St58*'` — rc=0、BUILD SUCCESSFUL（50s）。
- `git diff --check` — rc=0。

## 追加の settle（R49 / R50 / R51）

- R49: 更新印への migrate 終了コード追記を best-effort にし、印が壊れていても元の「移行に失敗した」を返して rollback の分類を保つ。
- R50: 提供元不明アプリの許可を与えて戻った時だけ、古い許可待ち表示を消す。取得・照合・インストールの失敗表示は成功まで残す。
- R51: `--version` が旧サーバ固有の「知らない引数」拒否なら互換版を使い、それ以外の失敗では更新を止める。

## 検証（R49 / R50 / R51）

- `scripts/quiet-run st58-settle-r49-r51 -- cargo test -p ashiato-setup st58_` — rc=0、8 passed。
- `scripts/quiet-run st58-settle-r50 -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St58UpdateInstallTest*'` — rc=0、BUILD SUCCESSFUL。

## 追加の settle（R52 / R53 / R55 / R57 / R59 / R61）

- R52/R57: 更新係と前版サービスの再起動を、start type を戻してから既に起動済みを許す経路へ統一した。
- R53: `program_restoring` を状態印に加え、途中停止したプログラム復元を再開しても、戻した前版や既存の `failed` を動かさないようにした。
- R55: 更新失敗の分類は更新印の `migrate_kind`・版・SQLSTATE を優先して読むようにした。
- R59: 旧版の `--version` 非対応を表す `0.0.0` を系統の実在版として扱わず、開発版の更新も進めるようにした。
- R61: 待ち受け失敗の戻しを画面と distribution spec で移行失敗から分離した。

## 検証（R52 / R53 / R55 / R57 / R59 / R61）

- `scripts/quiet-run st58-settle-installer -- cargo test -p ashiato-setup st58_` — rc=0、8 passed。
- `scripts/quiet-run st58-settle-web -- npm --prefix web test -- --run src/__tests__/update.test.ts` — rc=0、6 passed。
