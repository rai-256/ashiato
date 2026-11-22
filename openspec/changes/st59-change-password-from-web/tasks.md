# ST59 実装タスク — 合言葉を画面から変えられる

読む順: `deep.md`（本人の答え Q1〜Q3。第 1 回は推奨で先に書いてある）→ `review/deep.md` の「聞かない既定」C1〜C7 → このファイル → `specs/data-sensitivity/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST55（照らす値・`server.json`・インストーラの `password`）と ST56（「収集アプリの設定」・`POST /collector-setup`・`buildSetupString`・断られ続けた知らせ）は archive 済みで main にある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
件数つきの検証は、試験を走らせた後に `-- --list` の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

**Windows のサービスとインストーラの実物の確かめ（Task 7）は、下流の手元では走らない。** それは Windows の CI の job `install-windows` が `tools/release/install-check.sh` で毎回見る。
下流の手元の検証は、Linux の `cargo test`・`x86_64-pc-windows-gnu` への `cargo clippy`・`install-check.sh --self-test` で閉じる（ST55 と同じ形）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: …`）、bash は `echo "== Scenario: …"`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change で ST59 が印を置く Scenario は 33 本**（ADDED 28 本・MODIFIED で足した 5 本）。全部このファイルのどこかの Task が名指しする。
  MODIFIED で本文をそのまま写した Scenario（「画面のログインは期限で切れず…」の 6 本・「誰がいつ読んだかを…」の 10 本）の印は、ST28・ST29・ST44 が持っている。**消さない・名前を変えない**
- **本人の決定（下流は変えない）**: Q1 ほかの端末のログインは全部切り、変えた画面だけ新しい合言葉でログインしたままにする / Q2 設定の画面に 2 つの節を別々に置く（「画面の合言葉を変える」「収集アプリの値を変える」）/
  Q3 設定のファイルを指して起こしたサーバでだけ変えられ、開発の形では断る。
  聞かない既定: D1（C2）・D2（C3）・D3（C4）・D4（C5）・D6（C6）・D7（C7）・D9（C1）。仮: D8・D9・D10・D11（反転条件は design）
- **名前は統一する**: サーバの口 `POST /web-password`・`POST /collector-token`、読み出しの記録の口の名前 `/web-password`・`/collector-token`、断りの本文 `wrong_password`・`too_short`・`placeholder`・`same_as_api_token`・`same_as_session_key`・`mismatch`・`not_changeable`・`not_saved`、
  設定の鍵 `WEB_PASSWORD_VERIFIER`・`API_TOKEN`（名前を変えない）、一時のファイル `server.json.writing`、画面の部品 `WebPasswordChangePanel.tsx`・`CollectorTokenChangePanel.tsx`、台本 `tools/settings-server.sh`、e2e `web/e2e/st59-change-secrets.spec.ts`
- **秘密（今の合言葉・新しい合言葉・照らす値・新旧の収集アプリの値・ログインの印の鍵）を、ログ・読み出しの記録・応答の見出し・試験の期待値の出力・命令の引数に出さない**（試験の値も偽の値だけ）
- **壊してはいけないもの**（Story）: 画面のログインとセッション（ST28・ST44）・収集アプリの共有の値で送ることだけができる守り（ST44）・設定し直しを促す動き（ST56）。
  `.env` で起こす開発の形・`tools/stack.sh`・既存の e2e は今どおり動く。インストーラのコードは、Task 7 で届かないと分かったときだけ変える（D5 の反転条件）
- **新しい Rust の crate・npm の依存を足さない**（`argon2`・`rand`・`base64` はサーバに既にある）。移行を足さない（D9（仮））
- **長いコマンドは `scripts/quiet-run`**（cargo test・e2e）。試験の名前の接頭辞は `st59_`（Rust）・`st59-`（e2e のファイル名）。サーバの DB の試験は `crates/server/src/st59_tests.rs`（`lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;`）
- 縦串・書き換え禁止・偽データのシナリオを足すなら Story ごとの新しいファイル（`tools/smoke.d/` など）に置き、共有の台本の本体に書き足さない

## Task 1: 照らす値・印の材料・共有の値を、差し替えられる 1 つの組にする（D1）

- [x] 1.1 `web_session::WebLogin` と `App.token` を、照らす値（開発の形では元の合言葉）・印の材料 `tag`・共有の値の 3 つを持つ 1 つの組にまとめ、差し替えられる共有の置き場に置く。期限・失敗の遅れ・`limiter` は組の外に置く（差し替えで数え直さない）。
  1 つの求めは組を最初に 1 回だけ取る: `decide` の共有の値の照合・`session_valid` の `tag`・ログインの口の照合と印の行の `secret_tag`・`collector_setup_checked` が返す値。
  組を差し替える関数（Task 3・4 が使う）と、試験が照合の途中で差し替えを割り込ませる口（`#[cfg(test)]`。既存の `hold_login_check_for_test` と同じ形）を足す。振る舞いは変えない
  試験（`crates/server/src/st59_tests.rs`）: `変えている最中に前の合言葉で照合が通ったログインの印は、変えた後は使えない`（前の合言葉のログインの照合が通った後・印の行を書く前に組を差し替え、ログインが受け取った印で記録の読み出しが 401）・
  差し替えた後は新しい共有の値だけが `ApiToken` になる・差し替えても `limiter` の数えが残る
  寿命の出し直し（`refresh_cookie`）は、応答を返す時点の組の `tag` に合う印にだけ付ける（D1）。試験: `変える前に始まった求めの応答は、変えた画面の新しい印を上書きしない`（読み出しの途中で組を差し替え、その応答に `Set-Cookie` が無い）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st59_swap_ && test "$(cargo test -p ashiato-server st59_swap_ -- --list 2>/dev/null | grep -c ": test$")" -ge 4'` rc=0
- [x] 1.2 既存の試験が変わらず通る（ログイン・ST44 の印の鍵・ST55 の照らす形・ST56 の「収集アプリの設定」・CH15）。本数は 2026-11-11 の数（`web_session` 16・`st55_` 47・`st56_` 7・`ch15_` 8）を下限にする
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; for p in web_session:16 st55_:47 st56_:7 ch15_:8; do n=${p%%:*}; m=${p##*:}; scripts/quiet-run ct -- cargo test -p ashiato-server "$n" || exit 1; test "$(cargo test -p ashiato-server "$n" -- --list 2>/dev/null | grep -c ": test$")" -ge "$m" || exit 1; done'` rc=0
- [x] 1.3 完了の判定「古い値の収集アプリは送れなくなり、記録を捨てずに設定し直しを促される」の収集アプリの側（ST56 が作り、ST59 は変えない）を走らせ、premises 4 行目を確かさ 4 に上げる（premises.md の 4 行目の確かさと出所を書き直す）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st56_runtime_unauthorized_keeps_pending_records && test "$(cargo test -p ashiato-collector-windows st56_runtime_unauthorized_keeps_pending_records -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0 /
  `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*St56SetupNotifierTest*'` rc=0

## Task 2: 設定のファイルへの書き戻し（D2・D3）

- [ ] 2.1 `crates/server/src/service.rs` に、`ASHIATO_SETTINGS_PATH` の `server.json` を読み直し、1 つの鍵（`WEB_PASSWORD_VERIFIER` か `API_TOKEN`）だけを置き換えて、同じフォルダの `server.json.writing` に書いて `sync_all` し、`server.json` へ名前を付け替える関数を足す。
  読めない・JSON でない・書けない・付け替えられないときは型の付いた誤りを返し、`server.json` を変えない。値は誤りにもログにも載せない
  試験（`crates/server/src/st59_tests.rs`。DB を使わない）: 置き換えた 1 つの鍵のほかは鍵も値も同じ・書いたファイルを `load_settings` で読める・`server.json.writing` の名前でフォルダを先に置いて書けなくすると（読みだけの権限は root で効かないので使わない）誤りで `server.json` が前の中身のまま・
  JSON でない `server.json` には書かない・残った `server.json.writing` があっても上書きして終わる
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-server st59_settings_ && test "$(cargo test -p ashiato-server st59_settings_ -- --list 2>/dev/null | grep -c ": test$")" -ge 5'` rc=0 /
  `cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0

## Task 3: 画面の合言葉を変える口（D4・D9・D10・D12）

- [ ] 3.1 `POST /web-password` を足す（`web_session.rs`。middleware を素通しする口に足す。API の合言葉の許可の一覧に入れない）。順は D4: ログインの印（無ければ 401）→ 呼び出し元が画面のログインか（API の合言葉・衛星は 403）→ JSON か（415）→
  開発の形か（`ASHIATO_SERVICE_MODE` と `ASHIATO_SETTINGS_PATH` が無ければ 409 `not_changeable`）→ `LOGIN_CHECK` を取ったまま失敗の数え（429）と今の合言葉の照合（403 `wrong_password`。失敗の遅れ）→
  新しい合言葉の 5 つの検査（400）→ 照らす値を作り起動の 2 つの検査を通す → 新しい `tag` で印の行を書く → 読み出しの記録に `ok` の行 → Task 2 で書き戻す（落ちたら打ち消しの行と 500 `not_saved`）→ 組を差し替える → `Set-Cookie` → 204（D1・D9）。
  どの結果も読み出しの記録に 1 行（D9（仮）の結果の語と状態）。`run_to_completion` で通す（求めが取り消されても行を書き切る）。`utoipa` の注釈を付ける
  試験（`crates/server/src/st59_tests.rs`。設定のファイルは `$TMPDIR` の下に作る）:
  `新しい合言葉が検査に当たれば、何も変わらない`・`ログインの印が無い・収集アプリの値・衛星の資格情報では、合言葉を変えられない`・`画面の合言葉を変えると、設定のファイルは照らす値だけが変わる`・
  `画面から合言葉を変えると、起動し直さずにほかのログインが使えなくなる`（2 つ目のログインの印での読み出しが 401、変えた応答の `Set-Cookie` の印での読み出しが通る。変えた求めが持ってきた古い印も 401）・
  `今の合言葉が違えば、新しい合言葉が検査に当たっても今の合言葉の違いで断る`・`合言葉を変えても、失敗の数えは数え直されない`・`変えている最中に前の合言葉で照合が通ったログインの印は、変えた後は使えない`（口を通す形。ログインは 204 で印を受け取り、その印の読み出しは 401）・
  `変えた後に起動し直しても、新しい合言葉でログインでき、前の合言葉ではできない`（書き直した `server.json` から組み立て直した `App` で、前の合言葉のログインが 401・新しい合言葉が 204）・
  今の合言葉が違えば 403 `wrong_password` でログインが保たれ設定のファイルが変わらない・入れた合言葉と照らす値がサーバのログと読み出しの記録に無い
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st59_web_password_ && test "$(cargo test -p ashiato-server st59_web_password_ -- --list 2>/dev/null | grep -c ": test$")" -ge 10'` rc=0

## Task 4: 収集アプリの値を変える口（D6・D7・D9・D10）

- [ ] 4.1 `POST /collector-token` を足す（Task 3 と同じ守りと順。本文は今の合言葉だけ）。新しい値は D6 の作り方で、照らす値に当たる・ログインの印の鍵と同じなら作り直す。読み出しの記録に `ok` の行を書いてから Task 2 で `API_TOKEN` を書き戻し（落ちたら打ち消しの行と 500）、組を差し替え、200 `{"token": …}` を返す。画面のログインの印の行には触れない
  試験（`crates/server/src/st59_tests.rs`）: `変えた後は、前の値の記録は断られ、新しい値の記録は取り込まれる`・`今の合言葉が違えば、収集アプリの値は変わらない`・
  `ログインの印が無い・収集アプリの値・衛星の資格情報では、収集アプリの値を変えられない`・`収集アプリの値を変えても、画面のログインは切れない`・`新しい値はサーバが作り、変えるたびに違う`・
  `収集アプリの値を変えると、設定のファイルは収集アプリの値だけが変わる`・`変えた後に起動し直しても、新しい収集アプリの値で動く`（書き直した `server.json` から組み立て直した `App`）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st59_collector_token_ && test "$(cargo test -p ashiato-server st59_collector_token_ -- --list 2>/dev/null | grep -c ": test$")" -ge 7'` rc=0
- [ ] 4.2 2 つの口に共通の Scenario の試験（`crates/server/src/st59_tests.rs`）: `失敗が回数の制限を超えると、正しい合言葉でも変えられない`（ログインの口 1 回・`/collector-setup` 1 回・2 つの変える口 4 回ずつ失敗 → 2 つとも 429・設定のファイルが変わらない）・`本文が JSON でない求めでは、何も変わらない`・
  `合言葉と収集アプリの値を変える求めの断りは、返した状態のとおりに残る`（401・409・400・429 の行の状態と結果の語）・`書き戻しに落ちた求めは、変えた行と打ち消しの行の 2 行で残る`・`合言葉と収集アプリの値を変える求めの値は、記録にもログにも応答の見出しにも残らない`・
  読み出しの記録に `ok` の行を書けなければ（`with_access_sink` で書けなくする）、2 つとも 500 で設定のファイルも組も変わらない・
  `設定のファイルに書けなければ、何も変わらない`（`server.json.writing` の名前でフォルダを先に置いて書けなくする → 2 つとも 500 `not_saved`・前の合言葉と前の値が効く・書きかけが無い）・
  開発の形（`ASHIATO_SERVICE_MODE` なし）では 2 つとも 409 `not_changeable` で何も変わらない（e2e の `開発の形のサーバでは、画面から変えられないと出る` の口の側）・
  `合言葉と収集アプリの値を変える求めは、変えたものも断ったものも残る`（2 つの口の名前で `forbidden`・403 の行と `ok`・204 / 200 の行が 1 行ずつ）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st59_both_ && test "$(cargo test -p ashiato-server st59_both_ -- --list 2>/dev/null | grep -c ": test$")" -ge 8'` rc=0 / `./tools/check-openapi.sh` rc=0 / `tools/check-log-private.sh` rc=0

## Task 5: 設定のファイルを指してサーバを起こす台本（D8（仮））

- [ ] 5.1 `tools/settings-server.sh` を足す: `up <設定のファイル>` は鍵を環境へ入れ、`WEB_PASSWORD` を環境から外し（D8。照らす形は `raw_password_present` で断る）、`ASHIATO_SERVICE_MODE=1`・`ASHIATO_SETTINGS_PATH` を付けて `target/release/ashiato-server`（無ければ `tools/stack.sh` と同じく画面を組み込んで build）を `tools/ports.sh` の `ASHIATO_HTTP_PORT + 1` で起こし、`/healthz` を待つ。
  `down` は止める。`make <置き場>` は試験用の設定のファイル（`.env` の DB の URL・偽の `API_TOKEN` と `WEB_SESSION_KEY`・試験用の固定の合言葉の照らす値・`BIND`）を `<置き場>/settings/server.json` に作る。
  照らす値の元の合言葉と偽の値は台本に持ち、標準出力に出さない（e2e は台本が書いた別のファイルから読む）。`--self-test` は DB を使わずに、`make` の JSON の鍵の一覧と、`up` が組む環境（値でなく名前。`WEB_PASSWORD` が無いこと）を確かめる。`tools/check-secrets.sh` が拾うなら許可の表に足す
  検証: `tools/settings-server.sh --self-test` rc=0 / `tools/check-secrets.sh` rc=0

## Task 6: 画面の 2 つの節と e2e（D11（仮））

- [ ] 6.1 `web/src/WebPasswordChangePanel.tsx`・`web/src/CollectorTokenChangePanel.tsx` を足し、`SettingsView.tsx` の「収集アプリの設定」のすぐ下に、「携帯を落としたときは両方を変える」の 1 行と 2 つの節を置く。文言は `messages.ts`（D11 の文）。
  収集アプリの値の節は、成功したら `buildSetupString(window.location.origin + "/api", token)` の設定の文字列・コピーのボタン・貼り直しの案内を出し、別の画面へ移る・読み込み直すと消す（ST56 の部品と同じ持ち方）。合言葉の節は、成功したら入力欄を空にして「合言葉を変えました。ほかの端末のログインは切れました」を出す
  e2e（`web/e2e/st59-change-secrets.spec.ts`、本物のブラウザ。印は `// Scenario: …`。Task 5 の台本で試験ごとに設定のファイルを作ってサーバを起こし、その組み込みの画面を開く。最後に止める）:
  `今の合言葉を入れ直すと画面の合言葉が変わり、前の合言葉ではログインできない`・`今の合言葉が違えば、画面の合言葉は変わらない`・`合言葉を変えると、ほかのブラウザのログインは切れ、変えた画面はログインしたまま`（2 つの browser context）・
  `今の合言葉を入れ直すと、新しい値の設定の文字列とコピーのボタンが出る`（欄の文字列を `parseSetupString` で読み、送り先が `page.url()` の origin + `/api`・資格情報が設定のファイルの前の `API_TOKEN` と違い、変えた後の設定のファイルの `API_TOKEN` と一致・利用者の欄が無い）・
  `別の画面へ移って戻ると、新しい値の設定の文字列は消える`・`読み込み直すと、新しい値の設定の文字列は消える`・`設定に 2 つの節が並び、落としたときは両方を変えると書いてある`・
  `設定のファイルに書けないと、画面は合言葉が変わっていないと出す`（台本で作った設定のファイルの隣に `server.json.writing` の名前でフォルダを置いてから押す）。
  `開発の形のサーバでは、画面から変えられないと出る` は共有の stack のサーバ（`tools/stack.sh` の開発の形）で、2 つの節を押して文が出ることと、その後も `.env` の合言葉でログインでき `.env` の `API_TOKEN` で記録を送ると取り込まれることを見る
  検証: `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st59-change-secrets.spec.ts'` rc=0 / `bash -c 'cd web && npx tsc --noEmit && npx vitest run && npm run lint'` rc=0
- [ ] 6.2 既存の e2e が変わらず通る（共有の stack のサーバの合言葉と値を変えていないこと）
  検証: `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e'` rc=0

## Task 7: Windows の CI の通し（D5）

- [ ] 7.1 `tools/release/install-check.sh` の `scenario_names` に ST59 の 3 本を足し、インストールした形の通しの中（ST55 の `password` の確かめの後）に関数を足す:
  ログインして `POST /web-password` と `POST /collector-token` で 2 つの値を変える → `server.json` の 2 つの鍵が変わり、ほかの鍵と ACL（`icacls`）が変える前と同じ →
  `== Scenario: 変えた後に起動し直しても、新しい合言葉でログインでき、前の合言葉ではできない`・`== Scenario: 変えた後に起動し直しても、新しい収集アプリの値で動く`（サーバのサービスを止めて起こし、前と新しい合言葉でログイン・前と新しい値で記録を送る）→
  `== Scenario: 画面で変えた後も、インストーラで決め直せる`（`ashiato-setup.exe password` で決め直し、決め直した合言葉でログインでき、画面で変えた合言葉ではできず、`API_TOKEN` は画面で変えた値のまま）。
  `password` が `server.json` を書けない・読めないなら、D5 の反転条件のとおりインストーラの一時の書きを直す（`crates/installer`。試験は `st59_` の接頭辞）。直せなければ上流へ A として返す。
  `crates/installer/src/commands.rs` の一時の書きのコメント（settings はインストーラだけが書く）を、サーバも `server.json` を書く形に直す。
  [x] は手元の検証で付ける。実地は PR の CI の `install-windows`（`--list` とログを突き合わせ、走らなかった Scenario があれば落ちる）が merge gate で見る。CI が赤なら D5 の反転条件に入る
  検証: `tools/release/install-check.sh --self-test` rc=0 / `bash -c 'n=$(tools/release/install-check.sh --list | grep -cE "^(画面で変えた後も、インストーラで決め直せる|変えた後に起動し直しても、新しい合言葉でログインでき、前の合言葉ではできない|変えた後に起動し直しても、新しい収集アプリの値で動く)$"); test "$n" -eq 3'` rc=0（実地は PR の CI の `install-windows`）

## Task 8: 手順書と申し送り

- [ ] 8.1 `docs/network.md` §4（電話を落としたとき）を書き直す: 配る形では、画面の設定で「画面の合言葉を変える」と「収集アプリの値を変える」の両方を押し、新しい設定の文字列を全部の収集アプリに貼り直す。開発の形は今どおり `.env` を書き換えて起動し直す。衛星の資格情報は変わらないことを書く。
  `docs/install.md` の「合言葉を忘れたら」の近くに、覚えているうちは画面の設定から変えられること・忘れたら今どおり `password` を書く。`docs/collector-setup.md` の「資格情報を変えた後の設定し直し」に、画面で値を変えた後の手順を足す
  検証: `bash -c 'grep -q "画面の合言葉を変える" docs/network.md && grep -q "収集アプリの値を変える" docs/network.md && grep -q "画面の合言葉を変える" docs/install.md && grep -q "収集アプリの値を変える" docs/collector-setup.md'` rc=0
- [ ] 8.2 申し送り: `docs/handoff/ST60.md` に 1 節足す（Linux の配る形でも、設定のファイルから起こす形にすれば画面から変えられる。`ASHIATO_SERVICE_MODE` と `ASHIATO_SETTINGS_PATH` の判定・`tools/settings-server.sh` を口に替える反転条件（ST59 D8（仮））。
  あわせて、正典 `distribution` の「秘密はインストーラが自動で作り…人に見せない」が、共有の値を本人の画面に出す口（ST56 の `/collector-setup`・ST59 の `/collector-token`）と食い違っていること、`distribution` を次に MODIFIED する Story が「共有の値は、ログインして合言葉を入れ直した本人の画面にだけ出す」例外を足すことを書く（review/spec.md R13）
  検証: `bash -c 'grep -q "st59-change-password-from-web" docs/handoff/ST60.md && grep -q "settings-server.sh" docs/handoff/ST60.md && grep -q "collector-token" docs/handoff/ST60.md'` rc=0

## Task 9: 全体

- [ ] 9.1 `cargo fmt --all -- --check`・`cargo clippy --workspace --all-targets -- -D warnings`・Windows の 2 crate の `--target x86_64-pc-windows-gnu` の clippy・`cargo test --workspace`・`tools/check-openapi.sh`・`tools/check-log-private.sh`・`tools/check-secrets.sh`・Scenario の突き合わせ
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p ashiato-setup -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings && scripts/quiet-run all -- cargo test --workspace'` rc=0 /
  `bash -c 'tools/check-openapi.sh && tools/check-log-private.sh && tools/check-secrets.sh && python3 scripts/check_scenarios.py . st59-change-password-from-web'` rc=0

## 人間の確認待ち（確認バッチ）

無い。画面の Scenario は e2e（Task 6）、Windows のサービスとインストーラは CI の `install-windows`（Task 7）が見る。物理で残るもの（本物の端末・UAC）は、この Story の Scenario に無い。
