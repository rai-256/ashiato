# ST56 実装タスク — 収集アプリを入れた後に、送り先と資格情報を設定できる

読む順: `deep.md`（本人の答え Q1〜Q5・Q7・Q8）→ `review/deep.md` の「聞かない既定」C1〜C7 → このファイル → `specs/*/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST52・ST44 は archive 済みで main にある。ST54 は requires で、下流の入口が ST54 の gate 済みの SHA を取り込む（`python3 scripts/story_deps.py included ST56 ST54` で確かめる）。
`GET /contract` の応答の見出し `Ashiato-Channel`・`Channel::Dev`・PC の開発版の置き場の規則（`dev`・`-dev`）は ST54 のもの。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
件数つきの検証は、試験を走らせた後に `-- --list`（gradle は結果の XML）の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 設定が無くても起動して前景の窓を積む`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 81 本**（`device-collection` 41・`desktop-collection` 32・`data-sensitivity` 8）。全部このファイルのどこかの Task が名指しする
- **保留を解く手順を含む Scenario は、門を作る Task（携帯は Task 7、PC は Task 11）が持つ**（門より前の Task は保留に当たらない形で試す。review/spec.md R14）
- **本人の決定（下流は変えない）**: Q1 画面の設定に「収集アプリの設定」を置き、合言葉を入れ直すと 1 行の設定の文字列を出す / Q2 PC は黒い窓で貼らせ、資格情報は DPAPI で包んで置き場に置き、台本に秘密を書かない /
  Q3 携帯は最初に設定の画面を出し「後で」で飛ばせる / Q4 保存の時点で確かめて結果を見せ、繋がらなくても保存する / Q5 401 が 1 時間続いたら知らせ、24 時間ごとに出し直す /
  Q7 未送信があるまま送り先を変えたら、押すまで何も送らない / Q8 PC の置き場は `%LOCALAPPDATA%\ashiato\collector`、識別子は最初の起動で作って覚える。
  聞かない既定: D1〜D7（`review/deep.md` C1〜C7）。仮: D11・D14・D17〜D23（反転条件は design）
- **名前は統一する**: 設定の文字列の接頭辞 `ashiato-collector:1:`（D18（仮））・サーバの口 `POST /collector-setup`・読み出しの記録の口の名前 `collector-setup`・
  携帯の `SetupActivity`・SharedPreferences `collector_setup.xml`・通知のチャンネル `setup`・門 `DestinationGate`（携帯）/ `destination_gate`（PC）・
  PC の `setup` 副命令・`--config-from-env`・置き場のファイル `settings.json`・`credential.bin`・`setup-state.json`・PC の札 `setup` / `auth` / `unreachable` / `destination`・
  共有の材料 `docs/contracts/collector-setup-vectors.json`・既定の利用者 `00000000-0000-0000-0000-000000000000`
- **資格情報と画面の合言葉を、ログ・標準出力・通知・画面・試験の期待値の出力に出さない**（試験の値も `token-for-test-0123456789` のような偽の値だけ）
- **送信の封筒・送信契約の版・受け口の形を変えない**（FR-130。`tools/check-contract-version.sh` が落ちない）
- **今の開発と試験の組み立てを壊さない**（Story の「壊してはいけないもの」）: `-P` で焼き込んだ APK は今どおり送る。計測テストは今どおり通る。PC の環境変数の起動は `--config-from-env` を足せば今どおり動く
- **新しい Rust の crate・Android の依存を足さない**。PC は既にある `windows` crate に機能 `Win32_Security_Cryptography` を足すだけ（cfg(windows) の依存。`tools/check-licenses.sh` に通す）
- **長いコマンドは `scripts/quiet-run`**（cargo test・gradle・エミュレータ・e2e）。試験の名前の接頭辞は `st56_`（Rust）・`St56`（Kotlin のクラス名）・`st56-`（e2e のファイル名）。サーバの DB の試験は `crates/server/src/st56_tests.rs`
- 縦串・書き換え禁止・偽データのシナリオを足すなら Story ごとの新しいファイル（`tools/smoke.d/` など）に置き、共有の台本の本体に書き足さない

## Task 1: 設定の文字列の形と、3 つの言語の読み書き

- [x] 1.1 `docs/contracts/collector-setup-vectors.json` を作る（D18（仮））: 正しい文字列（`https://pc.example.ts.net/api` と `token-for-test-0123456789`・前後に空白と改行が付いたもの・知らない欄を持つもの）と読めない文字列（`hello`・接頭辞の版が `2`・`url` が欠ける・`token` が欠ける・base64url でない）と、それぞれの読んだ結果。
  画面の `web/src/collectorSetup.ts`（組む `buildSetupString(url, token)` と、試験のための `parseSetupString`）・PC の `crates/collector-windows/src/setup_string.rs`（読む）・携帯の `SetupString.kt`（読む）を足し、どれも材料の全件で試す。
  検証: `bash -c 'cd web && npx vitest run src/__tests__/collectorSetup.test.ts'` rc=0 / `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st56_setup_string && test "$(cargo test -p ashiato-collector-windows st56_setup_string -- --list 2>/dev/null | grep -c ": test$")" -ge 2'` rc=0 /
  `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*St56SetupStringTest*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St56SetupStringTest*.xml | grep -c "<testcase ")" -ge 2'` rc=0

## Task 2: サーバの口 `POST /collector-setup`

- [x] 2.1 `crates/server` に `POST /collector-setup`（D8・D22（仮））を足す: 有効なログインの印が要る。本文 `{"password": "…"}` を `web_session.rs` のログインと同じ照合・失敗の遅れ・回数の制限（ログインの口と同じ数え）で見て、一致すれば `{"token": <収集アプリの資格情報>}` を返し、違えば 403 `{"error": "wrong_password"}`（401 にしない。design D8）。失敗の数えはログインの口と共有し、超えたら 429。
  API の合言葉の許可の一覧には入れない（ST44 の既定で 403）。成功も失敗も読み出しの記録に口の名前 `collector-setup` で 1 行。応答に `Cache-Control: no-store`（正典の「応答はブラウザに記録を残させない」）。`docs/openapi.json` を作り直す。
  試験（`crates/server/src/st56_tests.rs`）: `// Scenario: ログインの印が無ければ答えない`・`// Scenario: 収集アプリの資格情報では答えない`・`// Scenario: 求めは読み出しの記録に残り、値は残らない`（読み出しの記録の行とサーバのログの捕まえに、資格情報と合言葉の文字列が無い）・`// Scenario: 失敗が回数の制限を超えると、正しい合言葉でも答えない`（ログインの口の失敗も数える）と、正しい合言葉で資格情報が返る・違えば 403 の 2 本
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st56_ && test "$(cargo test -p ashiato-server st56_ -- --list 2>/dev/null | grep -c ": test$")" -ge 6'` rc=0 / `./tools/check-openapi.sh` rc=0

## Task 3: 画面の「収集アプリの設定」と e2e

- [x] 3.1 `web/src/CollectorSetupPanel.tsx` を足し、`SettingsView.tsx` に置く（D6・D7・D8）: 見出し「収集アプリの設定」、合言葉の入力欄（`type="password"`）と「表示」、成功したら `buildSetupString(location.origin + "/api", token)` を読み取り専用の欄とコピーのボタンで出す。
  403 の `wrong_password` なら「合言葉が違います」を出して留まる（ログイン切れとして扱わない。`session.tsx` の見張りは 401 だけを見るので変えない）。文字列は状態にだけ持つ（保存しない。別の画面へ移ると消える）。「携帯に入れるなら、携帯から開いているアドレス（Tailscale）でこの画面を開いてください」を添える。利用者に見せる語は「送り先」（D6）。
  e2e（`web/e2e/st56-collector-setup.spec.ts`、本物のブラウザ。印は `// Scenario: …`）: `合言葉を入れ直すと、設定の文字列とコピーのボタンが出る`（欄の文字列を `parseSetupString` で読み、送り先が `page.url()` の origin + `/api`・資格情報がスタックの `API_TOKEN`・利用者の欄が無い）・
  `合言葉が違えば出さない`（「合言葉が違います」が見え、URL は設定のまま・ログインの画面の要素が無く、欄が無く、応答が 403）・`別の画面へ移って戻ると設定の文字列は消える`・`開き直すと設定の文字列は消える`（`page.reload()` の後に欄が無く、合言葉の入力欄がある）。
  検証: `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st56-collector-setup.spec.ts'` rc=0 / `bash -c 'cd web && npx tsc --noEmit && npx vitest run'` rc=0

## Task 4: 携帯の設定の保存と読み出し・利用者・送り先の検査・画面のアドレス

- [x] 4.1 `Config.kt` を、保存した設定（`collector_setup.xml`）を先に読み、無ければ `BuildConfig` の値にする（D3）。利用者は `BuildConfig.USER_ID` が空なら既定の利用者（D1）。`webUrl` は保存した設定があれば送り先から `/api` を外した値（D4）。
  保存の口 `SetupStore.save(text)` は `SetupString` で読み、`https://` でない・loopback の送り先を断る（D5。`-P` の値には当てない）。送り先と資格情報を仕組みを立てた時の値で固めない —— `HttpTransport`・`HttpOriginalTransport`・途絶と気分の問い合わせは呼ぶたびに `Config` から読む（D11（仮））。
  資格情報を `toString`・ログ・`Telemetry.line` に出さない。
  試験（Robolectric、`St56ConfigTest`）: `// Scenario: 貼った設定の文字列の送り先と資格情報で送る`・`読めない文字列は保存せず、前の設定を残す`・`保存した設定は、組み立ての時の値より先に使う`・`保存した設定が無ければ、組み立ての時の値で送る`・`資格情報は画面にもログにも出ない`（`ShadowLog` に値が無い）・
  `送り先から /api を外して画面を開く`・`保存した設定が無ければ、組み立ての時の画面のアドレスを使う`・`自分自身を指す送り先は保存しない`・`暗号化されない送り先は保存しない`・`組み立ての時の loopback の送り先はそのまま使う`
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*St56ConfigTest*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St56ConfigTest*.xml | grep -c "<testcase ")" -ge 10'` rc=0 /
  `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest'` rc=0（既存の試験が通る）

## Task 5: 携帯の設定をバックアップから外す

- [x] 5.1 `health_full_backup_content.xml` と `health_data_extraction_rules.xml`（`cloud-backup`・`device-transfer`）に `<exclude domain="sharedpref" path="collector_setup.xml"/>` を書く（D3）。新しい `tools/check-android-backup-rules.sh` は manifest の `android:fullBackupContent`・`android:dataExtractionRules` から規則のファイルを辿り、3 か所で外していることを見る。
  `--self-test` は行を 1 つずつ消した写しを `$TMPDIR` に作って落ちることを見る（`echo "== Scenario: 両方のバックアップの規則が設定のファイルを外している"`・`echo "== Scenario: 設定のファイルを外す行を消すと、検査が落ちる"`）。CI の `lint` の job に足す。
  検証: `tools/check-android-backup-rules.sh` rc=0 / `tools/check-android-backup-rules.sh --self-test` rc=0 / `bash -c 'grep -q "check-android-backup-rules.sh" .github/workflows/ci.yml'` rc=0
- [x] 5.2 前提（premises.md 1 行目）をエミュレータで確かめる: 計測テスト `St56BackupTest`（`app/src/androidTest`）が、設定を保存した状態で `bmgr backupnow` → アプリのデータを消す → `bmgr restore` を通し、未送信は戻り、`collector_setup.xml` は戻らないことを見る。結果を premises.md の 1 行目に確かさ 5 で書き直す。
  検証: `scripts/quiet-run emu -- tools/android-emulator.sh` rc=0（計測テストを全部走らせ、`St56BackupTest` を含む）（`/dev/kvm` が要る。Codex の sandbox の中では `BLOCKED_INFRA` になり、外での実行を申請する —— `AGENTS.md`）

## Task 6: 携帯の設定の画面・最初の起動・確かめ・保存の後に送り始める

- [x] 6.1 `SetupActivity` を足す（D9・D10・D11（仮））: 貼る欄・「保存」・「後で」・結果の文。保存すると `GET /contract` を 1 回引き、`ContractGate` の分け方と ST54 の `Ashiato-Channel` で 4 つの文のどれかを出し、答えに依らず保存する（この問い合わせで未送信を送らない）。保存したら `LocationService` に `ACTION_SETUP_CHANGED` を送り、`startFlushing` を立て直す。
  `MainActivity` は送り先が無く「後で」を押していないときだけ、権限の要求の前に `SetupActivity` を開く。メインの画面に「送り先の設定」の行。未設定の間はチャンネル `setup` に「送り先が設定されていません」の通知（D20（仮））。
  試験（Robolectric、`St56SetupActivityTest`）: `// Scenario: 初めて開くと、権限の要求より前に設定の画面が出る`・`「後で」を押しても、権限の要求と収集が始まる`・`未設定の間は通知で促し、押すと設定の画面が開く`・`組み立ての時に送り先を渡した収集アプリは、設定の画面を先に出さない`・
  `繋がれば「繋がりました」`・`401 なら「資格情報か送り先が違います」と出て、設定は保存される`・`系統の違うサーバなら、その旨を出す`・`届かなくても保存し、繋がったら送ると示す`・`確かめの問い合わせでは記録を送らない`・`資格情報だけを変えると、次の送信から新しい値で送る`
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*St56SetupActivityTest*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St56SetupActivityTest*.xml | grep -c "<testcase ")" -ge 10'` rc=0

## Task 7: 携帯の送り先の保留

- [x] 7.1 `DestinationGate` を足す（D13・D19（仮））: 承認した送り先を `collector_setup.xml` に持ち、保存のときに比べる（scheme と host を小文字・既定の port を外す・末尾の `/` を外す）。未送信（記録・生存信号・破棄の報告・原本の送り待ち）があれば保留にし、`SetupActivity` に件数と積んだ期間と「この送り先へ送る」を出す。
  記録・生存信号・破棄の報告の `Sender`・`OriginalUploader`・`OutagePoller`・`FeelingReminderReceiver` の問い合わせは、送る前に門を見る（送り手ごとに条件を書かない）。`-P` の送り先は承認したものとして扱う。保留の間はチャンネル `setup` に「送り先の確認を待っています」（D20（仮））。
  試験（Robolectric、`St56DestinationGateTest`）: `// Scenario: 未送信があるまま送り先を変えると、押すまで何も送らない`・`「この送り先へ送る」を押すと送り始める`・`承認した送り先が分からないまま未送信があっても、押すまで送らない`・`未送信が無ければ、保存した送り先へそのまま送る`・`資格情報だけを変えたなら保留しない`・`保留の間も収集は続く`・`保留の画面は積んだ期間を示す`・`保留の間は写真の原本も送らない`・`保留の間は通知で促し、押すと設定の画面が開く`・
  `設定の前に積んだ記録は、設定の後に送られ、捨てられない`（偽のサーバが本物の取り込みと同じく空の利用者を `malformed` で断る形で、nil の利用者なら受理されることを見る）・`設定を保存すると、サービスを起こし直さずに送り始める`
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*St56DestinationGateTest*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St56DestinationGateTest*.xml | grep -c "<testcase ")" -ge 11'` rc=0

## Task 8: 携帯の断られ続けと届かないの知らせ

- [x] 8.1 `SetupNotifier` を足す（D12・D20（仮））: 401 の数え始め・最後に知らせた時刻・最後に届いた時刻・設定した時刻を `collector_setup.xml` に持つ。送信と問い合わせの 401 で数え、401 でない答えか設定の保存で数え直して通知を消す。1 時間で「サーバに断られています。資格情報か送り先が違います。設定し直してください」、一度も届かない送り先は 24 時間で「送り先に届きません。設定を確かめてください」、どちらも 24 時間ごとに出し直す。押すと `SetupActivity`。
  決め方は ST50 の `ContractNotifier` と同じ形（時計を差し替えられる純粋な関数）。401 で断られた記録は取り除かない（今どおり）。
  試験（Robolectric、`St56SetupNotifierTest`）: `// Scenario: 401 が 1 時間続くと知らせる`・`断られ続ける間は 24 時間ごとに出し直す`・`401 で断られた記録は取り除かない`・`一度も届かない送り先は 24 時間で知らせる`・`断られる状態は再起動をまたいで数える`・`届けば数えをやめて通知を消す`・`401 だけを返す送り先は「届かない」と知らせない`・`断られている間に 5xx が挟まっても数え直さない`
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*St56SetupNotifierTest*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St56SetupNotifierTest*.xml | grep -c "<testcase ")" -ge 8'` rc=0

## Task 9: PC の置き場・端末の識別子・印・設定の無い起動

- [x] 9.1 `config.rs` を 2 つの読み方にする（D2・D16・D17（仮）・D23（仮））: `--config-from-env` なら今の `from_lookup`。無ければ置き場（環境の `ASHIATO_STATE_DIR` → 自動起動の台本の行 → `%LOCALAPPDATA%\ashiato\collector`）と識別子（環境 → 台本 → `settings.json` → `pc-` + 16 進 12 文字を作って書く）を決め、送り先と資格情報は置き場の設定から読む（無ければ「送り先なし」）。利用者は nil の UUID。
  ST54 の開発版の規則（`dev`・`-dev`・除外は渡された置き場）は、決めた置き場と識別子に当てる。`main.rs` は設定が無くても終わらず、`Runtime` を送り先なしで始める（送る契機は何もしない）。窓があれば別の糸で設定の文字列を待つ（D14（仮））。
  試験（`#[cfg(test)]`、名前は `st56_`）: `// Scenario: 既定の置き場を使う`・`前の形の台本の置き場と端末の識別子を引き継ぐ`・`端末の識別子は最初の起動で作って覚える`・`開発版は既定の置き場の下の dev と、末尾に -dev の識別子を使う`・`印を付けると環境変数で動く`・`印を付けなければ、環境変数の資格情報を使わない`・`設定が無くても起動して前景の窓を積む`（偽の窓の取り手で、送信の要求 0 件）・`既定の置き場が決まらなければ起動を止める`・`設定の無い起動は窓で貼らせ、待つ間も収集する`（標準入力を差し替える）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st56_config && test "$(cargo test -p ashiato-collector-windows st56_config -- --list 2>/dev/null | grep -c ": test$")" -ge 9'` rc=0 / `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows'` rc=0（既存の試験が通る）

## Task 10: PC の `setup`・資格情報の置き方・自動起動の台本・確かめ

- [x] 10.1 `setup` 副命令と `secret.rs`（DPAPI。Windows 以外は試験用の可逆な包み）を足す（D10・D11（仮）・D14（仮）・D15）: 設定の文字列を読み、`settings.json`（送り先・承認した送り先・識別子）と `credential.bin`（DPAPI の包み）に書き、`GET /contract` を 1 回引いて 4 つの文のどれかを窓に出し、答えに依らず保存する。
  `autostart::startup_script` は送り先・資格情報・利用者の行を書かない。`setup` は前の形の台本の 3 つの行を消して書き直す（置き場と識別子の行は残す。D23（仮））。走っている収集アプリは送る契機ごとに `settings.json` の更新時刻を見て読み直す。未設定の通知（札 `setup`）を押すと `setup` が新しい窓で開く。
  試験（`#[cfg(test)]`、名前は `st56_setupcmd_`。Task 1 の `st56_setup_string` と前方一致させない）: `// Scenario: setup で貼った設定の文字列を保存する`・`読めない文字列は保存しない`・`自動起動の台本は実行ファイルの場所だけを持つ`・`前の形の台本は、設定を保存すると書き換わる`・`401 でも保存して結果を示す`・`届かなくても保存する`・
  `PC で繋がれば「繋がりました」`・`PC で系統の違うサーバなら、その旨を出す`・`PC の確かめの問い合わせでは記録を送らない`・`戻せない資格情報は未設定として扱う`（試験用の包みで戻せない中身を置く）。
  Linux の試験用の包みには `置き場のファイルに資格情報の平文が無い` の印を置かない（本物の DPAPI の 10.2 が持つ。review/spec.md R13）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st56_setupcmd_ && test "$(cargo test -p ashiato-collector-windows st56_setupcmd_ -- --list 2>/dev/null | grep -c ": test$")" -ge 10'` rc=0 / `tools/check-licenses.sh` rc=0
- [ ] 10.2 本物の Windows で DPAPI と台本を確かめる: `tests/runtime_windows.rs` に `st56_` の試験を足す —— `// Scenario: 置き場のファイルに資格情報の平文が無い`（本物の `CryptProtectData` で包んだ `credential.bin` と置き場の全ファイルに平文が無く、同じ利用者で戻せる）・`--install-autostart` が書いた `ashiato-collector.cmd` に `ASHIATO_API_TOKEN` が無い・`--config-from-env` を付けた既存の起動が今どおり動く。既存の `runtime_windows` の起動に `--config-from-env` を足す（D17（仮））。
  検証: Windows で `cargo test -p ashiato-collector-windows --test runtime_windows st56_ -- --test-threads=1` が rc=0（CI の Windows の job）

## Task 11: PC の保留と知らせ

- [x] 11.1 `destination_gate`（D13・D19（仮）・D21（仮））と 401・届かないの数え（D12・D20（仮））を足す: `setup` は保存の後、未送信（記録・生存信号・破棄の報告）があり承認した送り先と違えば「未送信 <n> 件（<最も古い時刻> 〜 <新しい時刻>）があります。この送り先へ送りますか [y/N]」と聞き、`y` で承認、それ以外は保留（札 `destination` の通知）。
  記録・生存信号・破棄の報告の送り手と `GET /outages` は送る前に門を見る（`GET /contract` の確かめと `GET /healthz` は見ない）。401 の数えは `setup-state.json` に持ち、1 時間・24 時間・出し直しは携帯と同じ（札は断られ続けが `auth`、届かないが `unreachable`）。`contract_notice::decide` と同じ形の純粋な関数にする。
  試験（`#[cfg(test)]`、名前は `st56_gate`）: `// Scenario: 未送信があるまま送り先を変えると、答えるまで送らない`・`送ると答えれば送り始める`・`PC は未送信が無ければ、保存した送り先へそのまま送る`・`PC は保留の間、知らせで促す`・`設定を保存すると、起動し直さずに送り始める`・
  `PC は 401 が 1 時間続くと知らせる`・`PC は断られ続ける間 24 時間ごとに出し直す`・`PC は断られる状態を再起動をまたいで数える`・`PC は届けば数えをやめる`・`PC は 401 で断られた記録を取り除かない`・`PC は一度も届かない送り先を 24 時間で知らせる`・`PC は 401 だけを返す送り先を「届かない」と知らせない`
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st56_gate && test "$(cargo test -p ashiato-collector-windows st56_gate -- --list 2>/dev/null | grep -c ": test$")" -ge 12'` rc=0

## Task 12: 道具・手順書・申し送り

- [x] 12.1 PC の収集アプリを環境変数で起こす道具に印を足す（D17（仮））: `tools/record-env.sh` は `WINDOWS_ENV_ASHIATO_CONFIG_FROM_ENV=1` を渡す（起動は harness2 の録画の台本が持つので、引数でなく環境の印）。`examples/`・`docs/` の起動の例には `--config-from-env`。`rg -l "ASHIATO_BASE_URL" tools docs crates/collector-windows` で漏れを探す。
  `docs/collector-setup.md` を書く: 携帯（Tailscale のアドレスで画面を開く → 合言葉を入れ直す → コピー → 携帯の設定の画面に貼る → 保留なら「この送り先へ送る」）・PC（`setup`）・知らせが出たとき・サーバの `repair` や資格情報を変えた後の設定し直し・前の形の台本からの移り方（D23（仮）・Migration Plan）。`docs/release.md` §7 から指す。
  申し送り（`docs/handoff/ST55.md`・`docs/handoff/ST59.md`）は上流で書いた。実装が design と違う形になったら、その節を直す。
  検証: `bash -c 'grep -q "WINDOWS_ENV_ASHIATO_CONFIG_FROM_ENV=1" tools/record-env.sh'` rc=0 / `bash -c 'for h in "ashiato-collector-windows.exe setup" "この送り先へ送る" "repair" "--config-from-env" "収集アプリの設定"; do grep -q -- "$h" docs/collector-setup.md || exit 1; done && grep -q "collector-setup.md" docs/release.md'` rc=0
- [x] 12.2 Scenario の印を突き合わせる。
  検証: `python3 scripts/check_scenarios.py . st56-collector-connection-setup` rc=0 / `tools/check-contract-version.sh` rc=0（封筒と送信契約の版を変えていない）

## 人間の確認待ち

- [ ] 13.1 配る形の確かめ（確認バッチ）: 開発版でない組み立ての APK（送り先と資格情報を焼き込まない）を携帯に入れ、最初に設定の画面が出る → 画面を Tailscale のアドレスで開いて設定の文字列を出す → 貼る → 「繋がりました」と保留の確認 → 「この送り先へ送る」→ 画面に記録が出る、を録画で見る。PC も配る zip の実行ファイルで `setup` を通し、`ashiato-collector.cmd` に資格情報が無いことを見る
  > 物理: device
- [ ] 13.2 本人の PC の切り替え: 前の形の台本で動いている本人の PC で、新しい実行ファイルに替えた後も前の置き場と除外の登録のまま動き（D23（仮））、`setup` の後に台本から秘密の行が消えることを見る
  > 物理: realdata
