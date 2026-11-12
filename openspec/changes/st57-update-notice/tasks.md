# ST57 実装タスク — 新しい版が出たら知らせる

読む順: `deep.md`（**本人の答え Q1〜Q10**。第 1 回は推奨で先に書いてある）→ `defaults.md`（聞かない既定 C1〜C10）→ このファイル → `specs/distribution/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST52（配布物の名前・3 つのチャンネル・`crates/build-stamp`）と ST55（インストーラ・サービス・`layout.rs`・`install-check.sh`・CI の `install-windows`）は archive 済みで main にある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
件数つきの検証は、試験を走らせた後に `-- --list`（gradle は結果の XML）の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

**Windows で管理者の権限が要る確かめ（サービスの登録・ACL・`update-check` の口の通し）は、下流の手元では走らない。** Windows の CI の job `install-windows` が新しい `tools/release/update-check-install.sh` で毎回見る（ST55 と同じ形）。
下流の手元の検証は、(a) 命令を組む純粋な関数と更新係の 1 回の確かめの Linux の `cargo test`、(b) `x86_64-pc-windows-gnu` への `cargo clippy`、(c) `update-check-install.sh --self-test`（偽の `sc.exe`・`icacls` を `PATH` の先に置き、確かめの論理だけを試す）の 3 つで閉じる。本物の Windows の通しは PR の CI で見る。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 24 時間たたないうちは確かめない`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario**: `distribution` の ADDED 105 本と、MODIFIED で足した 5 本（`インストールの最後に更新の確認が有効なことと切り方を示し、その後に更新係が起きる`・`配布元に届かなくてもインストールは終わる`・`更新係の無いインストールを走らせ直すと、更新係が足される`・`切った更新の確認は、走らせ直しでも切ったまま`・`サーバの Windows 版の依存の一覧は、更新係だけが持つ部品を含む`）、計 **110 本に ST57 が印を置く**。全部このファイルのどこかの Task が名指しする。
  MODIFIED で文を変えた ST55 の Scenario（`配る zip からのインストールで、2 つのサービスが自動で起きる形で入る`・`消すとサービスとプログラムと登録が消える`・`消しても記録の置き場は残り、その場所を示す`・`Windows の配布物にインストーラと PostgreSQL が入り、管理画面は入らない`・`インストーラとサーバの版番号はそろう`・`配る組み立ては試験の差し込みを持たない`・`Linux の配布物は PostgreSQL を持たない`）は、ST55 の印のある確かめ（`install-check.sh`・`check-artifacts.sh`）を新しい THEN に合わせて直す（Task 6・7）。**印を消さない**
- **本人の決定（下流は変えない）**: Q1 3 つの部品がそれぞれ直に配布元に聞く / Q2 入っている版のチャンネルで決める / Q3 サーバはインストーラの `update-check off|on` で切る / Q4 収集アプリは端末ごとに切る /
  Q5 携帯は画面に出し、新しい版ごとに 1 回通知する / Q6 PC は新しい版ごとに 1 回通知する / Q7 どの画面でも見出しの上の帯に 1 行・設定の画面に状態 / Q8 帯を押しても何も起きない / Q9 7 日確かめられなければ「更新を確かめられていない」/ Q10 携帯は網を問わない / Q11 入れ替えた収集アプリは切り方を示すまで問い合わせない（携帯は通知を出せた時か画面に出た時、PC はトーストの成功で示したとみなす。どれでも示せない間は止まったまま。D23）。
  聞かない既定: D1〜D10（`defaults.md` C1〜C10）。仮: D11〜D22・D26〜D28（反転条件は design）。D23 は本人の決定（Q11）、D24 は Q12、D25 は Q13。（第 1 回は推奨で先に書き、本人の答えが推奨と違えば上流が写してから下流へ渡す）
- **名前は統一する**（D11（仮））: crate `crates/update-check`（package `ashiato-update-check`）・`crates/updater`（package `ashiato-updater`・bin `ashiato-updater`・口 `service` と `once`）、サービス `ashiato-updater`（開発版 `ashiato-updater-dev`）・アカウント `NT SERVICE\ashiato-updater`、
  置き場 `C:\ProgramData\ashiato-updater`（開発版 `-dev`）の `settings\check.json`・`settings\test-hooks.json`（試験の組み立てだけ）・`status.json`・`checks.jsonl`・`logs\`、インストーラの口 `update-check off|on`、サーバの口 `GET /update-status`（開発の形の置き場 `ASHIATO_UPDATER_DIR`）、
  PC の口 `update-check off|on`・札 `update`・`update-stale`・`update-intro`・置き場のファイル `update-check.json`・`update-checks.jsonl`、携帯の `UpdateCheck.kt`・`UpdateChecker.kt`・チャンネル `update`・SharedPreferences `update_check.xml`・`noBackupFilesDir/update_checks.jsonl`・`BuildConfig.UPDATE_CHECK_URL`、
  試験の差し込み `ASHIATO_UPDATER_TEST_URL`・`ASHIATO_UPDATER_TEST_NOW`・`ASHIATO_COLLECTOR_TEST_UPDATE_URL`・`ASHIATO_COLLECTOR_TEST_NOW`・`ASHIATO_SETUP_TEST_UPDATER_URL`、共有の材料 `docs/contracts/update-check-vectors.json`、道具 `tools/check-updater-deps.sh`・`tools/release/update-check-install.sh`
- **自動の試験と CI は本物の配布元（`api.github.com`）に問い合わせない**（D8・D18（仮））。受け手は loopback の手元の HTTP。試験の値に本物の資格情報を使わない
- **サーバの依存に HTTP・TLS の部品を足さない**（`tools/check-offsite.sh` が今のまま通る）。サーバが依るのは `ashiato-update-check` だけ
- **新しい依存**: `ashiato-update-check` に `semver`（と既にある `serde`・`serde_json`）。`ashiato-updater` と PC の収集アプリに `ureq` の機能 `rustls`・`platform-verifier`（D12（仮））。携帯は新しい依存を足さない（`HttpURLConnection`）。
  `tools/check-licenses.sh`・`tools/check-vulns.sh` に通す（落ちたら `AGENTS.md` の「依存の脆弱性」の手順）
- **`unsafe` は書かない**。Windows のサービスの口は `crates/winsvc` を使う（サービス名を引数で受ける形にするなら `winsvc` を直す）
- **送信の封筒・送信契約の版・取り込みの口・DB を変えない**（`tools/check-contract-version.sh` が落ちない）。移行は無い
- **長いコマンドは `scripts/quiet-run`**（cargo test・gradle・エミュレータ・e2e）。試験の名前の接頭辞は `st57_`（Rust）・`St57`（Kotlin のクラス名）・`st57-`（e2e のファイル名）。サーバの試験は `crates/server/src/st57_tests.rs`
- 縦串・書き換え禁止・偽データ・録画のシナリオを足すなら Story ごとの新しいファイル（`tools/smoke.d/` など）に置き、共有の台本の本体に書き足さない。Windows の更新係の確かめは `install-check.sh` に書き足さず、新しい `tools/release/update-check-install.sh` に置く

## Task 1: 選び方・時期・結果の形の共有の crate と材料

- [x] 1.1 `crates/update-check`（D1・D3・D4・D5・D15（仮）・D21（仮））を作業場の member に足す: 一覧（GitHub のリリースの JSON。`tag_name`・`draft`・`assets[].name`）を読み、部品（`server` / `collector-windows` / `android`）と自分の版から「見つけた最新の版」と更新あり／なしを返す関数・
  `due(now, last_checked_at)`・`stale(now, last_ok_at_or_since)`（7 日。未来の時刻は過ぎたとみなす）・`updater_not_running(now, checked_at)`（48 時間）・問い合わせの道と見出しの定数・記録の 1 行と結果（`status.json`）の形と、信用しない入力としての読み方（64 KB・SemVer・RFC 3339）。HTTP・TLS・DB の部品を持たない。
  `docs/contracts/update-check-vectors.json` を作る。`cases`（選び方）は spec の「新しい版かどうかは…」の Scenario の全部の場合と、3 つの部品の配布物の名前・開発版の入れ替えの途中・`[]`・数えるリリースが無い一覧を含む（30 件以上）。`timing`（時期）は 23 時間・25 時間・失敗の 1 時間後・3 年先の最後の時刻（`due`）と 6 日・8 日・1 年先（`stale`）と 47 時間・49 時間（`updater_not_running`）を含む（10 件以上。review/spec.md R7）。Rust と Kotlin（Task 9.1）が両方の全件を通す。
  試験（`#[cfg(test)]`、名前は `st57_choose_`）: 材料の全件と `// Scenario: 正式版の部品は、より新しい正式版だけを更新ありにする`・`ベータ版の部品は、より新しいベータ版と正式版を更新ありにする`・`正式版を入れた後は、ベータ版を知らせない`・`ベータ版の部品は、土台の版の上がったベータ版も更新ありにする`・`開発版の部品は、印 nightly の配布物の版だけを見る`・`開発版は土台の版が上がった版を新しいとみなす`・
  `開発版の入れ替えの途中で 2 つの版が並べば、新しい方を取る`・`下書きのリリースは数えない`・`開発版の先行公開をベータ版に数えない`・`自分の部品の配布物が無いリリースは数えない`・`自分の版が一覧より新しければ更新なし`・`新しい版を入れた後は、古い結果でも更新ありにならない`
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-update-check st57_choose_ && test "$(cargo test -p ashiato-update-check st57_choose_ -- --list 2>/dev/null | grep -c ": test$")" -ge 14'` rc=0 / `bash -c 'test "$(python3 -c "import json;d=json.load(open(\"docs/contracts/update-check-vectors.json\"));print(min(len(d[\"cases\"])//3,len(d[\"timing\"])))")" -ge 10'` rc=0

## Task 2: 更新係（1 回の確かめ・サービスの口・試験の差し込み・部品の検査）

- [x] 2.1 `crates/updater`（D1・D2・D4・D5・D6・D8・D12（仮）・D13（仮））: `ashiato-updater once`（1 回だけ確かめて結果と記録を書く）と `ashiato-updater service`（Windows のサービス。起動の時と 1 時間ごとに `due` を見て確かめる。`cfg(windows)` だけ）。置き場は系統で選ぶ（D6）。`settings\check.json` が `enabled: false` なら問い合わせない。
  宛先は `https://api.github.com` の定数。`build.rs` で `cfg(ashiato_test_build)` を立て、その時だけ `ASHIATO_UPDATER_TEST_URL`・`ASHIATO_UPDATER_TEST_NOW`（環境変数か `settings\test-hooks.json`）を読む。差し込みの宛先が無い試験の組み立ては確かめずに「宛先なし」を記録する。転送は `api.github.com` の中だけ追う。確かめられない理由は D4 の 6 つ。
  試験（Linux。`crates/updater/tests/st57_once.rs`。loopback の受け手を試験の中で立て、`ashiato-updater once` を一時の置き場で走らせる）: `// Scenario: 更新係の問い合わせは版の一覧の取得だけである`・`開発版の部品は印 nightly の 1 件だけを取る`・`別の宛先への転送は追わない`・`空の一覧は確かめた・新しい版なしになる`・
  `24 時間たたないうちは確かめない`・`24 時間が過ぎた後の最初の機会に 1 回確かめる`・`失敗した回も 24 時間の数えに入る`・`最後に確かめた時刻が未来なら確かめる`・`確かめるたびに記録が 1 行増え、前の行は変わらない`・`確かめられない理由を状態の番号で分ける`・`答えない配布元は時間切れになる`（接続 10 秒・全体 30 秒・転送は 3 回まで）・`切った印があれば、更新係を開始しても問い合わせない`
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-updater st57_ && test "$(cargo test -p ashiato-updater st57_ -- --list 2>/dev/null | grep -c ": test$")" -ge 12'` rc=0 / `cargo clippy -p ashiato-updater -p ashiato-winsvc --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0
- [x] 2.2 `tools/check-updater-deps.sh`（D1）: `cargo tree -p ashiato-updater -e normal` に DB の部品（`sqlx`・`tokio-postgres`・`postgres`・`diesel`・`sea-orm`）が無い（`echo "== Scenario: 更新係は DB に繋ぐ部品を持たない"`）・`cargo tree -p ashiato-server -e normal` に `ashiato-updater` が無く `tools/check-offsite.sh` が通る（`echo "== Scenario: サーバは更新係の部品に依らない"`）・`ashiato-update-check` に `check-offsite.sh` の一覧の部品が無い・`ashiato-updater` と `ashiato-collector-windows` に同梱の根の一覧の部品（`webpki-roots`・`webpki-root-certs`）が無い（`echo "== Scenario: 更新係と PC の収集アプリは根の一覧を同梱しない"`。PC の TLS は Task 8.1 で足すので、8.1 の後にもう一度走らせる）。
  `--self-test` は一覧の名前を 1 つ含む偽の出力で落ちることを見る。CI の `lint` の job に `tools/check-updater-deps.sh` と `--self-test` を足す
  検証: `tools/check-updater-deps.sh` rc=0 / `tools/check-updater-deps.sh --self-test` rc=0 / `tools/check-offsite.sh` rc=0 / `bash -c 'grep -q "check-updater-deps.sh" .github/workflows/ci.yml'` rc=0

## Task 3: サーバの読みの口 `GET /update-status`

- [x] 3.1 `crates/server` に `GET /update-status`（D6・D9・D10・D14（仮）・D15（仮））を足す: サービスの形は系統の定数の置き場、開発の形は `ASHIATO_UPDATER_DIR`（無ければ置き場なし）から `settings\check.json` と `status.json` を読み、`ashiato-update-check` で状態を決めて返す。画面のログインが要り、API の合言葉の許可の一覧に入れない。`Cache-Control: no-store`。サーバの依存に HTTP の部品を足さない。`docs/openapi.json` を作り直す。
  試験（`crates/server/src/st57_tests.rs`。一時の置き場に結果のファイルを置く）: `// Scenario: 更新の状態の口はログインが要り、API の合言葉では答えない`・`// Scenario: 読めない結果のファイルは画面に出さない`（65 KB のファイルと、版が `<script>` のファイル）・`// Scenario: 64 KB より小さい正しい結果のファイルは読む`・`// Scenario: 更新係の時刻が古ければ、理由は更新係が動いていない`・`// Scenario: 更新係の時刻が 48 時間たたなければ、理由は最後の回のまま`（時計を差し込む）・`// Scenario: 更新の状態の口は応答を残させない`と、`state` の 8 つ（`off`・`not_running`・`pending`（`status.json` が無く確認から 2 日）・`latest`・`available`・`stale`（`status.json` が無く確認から 8 日も）・`available_stale`・`unreadable`）を 1 本ずつ
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st57_ && test "$(cargo test -p ashiato-server st57_ -- --list 2>/dev/null | grep -c ": test$")" -ge 14'` rc=0 / `./tools/check-openapi.sh` rc=0 / `tools/check-offsite.sh` rc=0

## Task 4: 画面の帯と設定の節・e2e

- [ ] 4.1 `web/src/update.ts`（`GET /api/update-status` の型と文。D20（仮））・`web/src/UpdateBand.tsx`（`Root.tsx` で、ログインした全部の画面の見出しより上に置く。`role="status"`・押しても何もしない・`available` / `stale` / `available_stale` だけ出す）・`SettingsView.tsx` に「更新の確認」の節（状態・入っている版・見つけた最新の版と見つけた日・最後に確かめた日時と理由）。
  帯の文字と地は `contrast.ts` で 3:1 以上、押せる所は無い（帯は押す口でない）。360 px で横に流さない。e2e の偽の結果は、playwright の起こすサーバに `ASHIATO_UPDATER_DIR` を渡し、試験ごとにそこへ `status.json`・`settings/check.json` を書いて置く（サーバは求めごとに読む）。サーバは全部の spec で共有（`workers: 1`）なので、各試験の後に置き場を空にし、置き場は空で始まる（ほかの spec の画面に帯が漏れない）。読み出しの記録は口の名前 `/update-status` で今の層のとおり残す（D14（仮））。
  e2e（`web/e2e/st57-update-notice.spec.ts`、本物のブラウザ）: `// Scenario: 更新ありはどの画面でも見出しの上に出る`（帯の下端の y ≤ 各画面の `<h1>` の上端の y）・`6 日確かめられていないだけなら帯は出ない`・`7 日確かめられなければ帯に出る`・`帯の文字と地のコントラストは 3:1 以上`（`getComputedStyle` の色を `contrast.ts` で計る）・`更新ありと確かめられていないが同時なら 2 行出る`（360 px で `document.documentElement.scrollWidth <= innerWidth`）・`新しい版が無ければ帯は出ない`（`<h1>` の y が帯の無い画面と同じ）・
  `確認を切ってあれば帯は出ず、設定に切ってあると出る`・`帯を押しても何も起きない`（押す前後で `page.url()` が同じで、要求の数が増えない）・`設定に更新の確認の状態が出る`・`更新係の無いサーバは設定に動いていないと出し、帯を出さない`・`結果を読めないときは帯を出さず、設定にだけ出す`・`まだ確かめていない更新係は、7 日たつまで帯を出さない`・`新しい版なしは設定にだけ出る`・`帯を出しても外部への要求は 0 件である`（`offsite.spec.ts` と同じ数え方）
  検証: `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st57-update-notice.spec.ts'` rc=0 / `bash -c 'cd web && npx tsc --noEmit && npx vitest run'` rc=0（`one-scroll.test.tsx` を含む既存の試験が通る）

## Task 5: インストーラ（置き場の列・登録・ACL・最後の表示・切り替えの口・走らせ直し・消す）

- [ ] 5.1 `crates/installer`（D6・D7・D8・D11（仮））: `layout.rs` に更新係の列（D6）。インストールで `ashiato-updater.exe` をプログラムの置き場に置き、更新係の置き場を作り、ACL（D6）を付け、`settings\check.json` が無ければ `enabled: true`・`since` で書き、サービス `ashiato-updater` を `NT SERVICE\ashiato-updater`・開始の種類「自動」・回復の起こし直しで登録する（共有のアカウントへ倒さない）。
  最後の段で「更新の確認: 有効（切るには `ashiato-setup.exe update-check off`）」を示し、その後に `enabled: true` のときだけ更新係のサービスを開始する（開始に失敗してもインストールは失敗にしない）。試験の組み立てで `ASHIATO_SETUP_TEST_UPDATER_URL` があれば `settings\test-hooks.json` を書く。
  `update-check off|on`（管理者の確認 1 回。off は印を書いてサービスを止め「無効」、on は印と `since` を書いて「自動」にして開始）。走らせ直しは更新係が無ければ足し、`check.json` があれば触らず、`enabled: false` なら開始しない。`uninstall_commands` に更新係の停止と削除を足し、出力に更新係の置き場を示す。`repair` の命令の一覧は変えない。
  試験（Linux の `cargo test -p ashiato-setup`、名前は `st57_`）: 組む命令の順（表示の後に開始）・登録の命令が `NT SERVICE\ashiato-updater` を名指しし共有のアカウントを名指ししない・ACL の命令に `Users`・`Authenticated Users`・`Everyone` が無く、状態の置き場の ACL の命令に `ashiato-updater` が無い・`// Scenario: 切り替えの口は管理者の確認を 1 回だけ出す`・
  `off` の命令が開始の種類を「無効」にして止める・`on` の命令が「自動」にして開始する・走らせ直しで `enabled: false` なら開始の命令が無い・消す命令に `ashiato-updater` の停止と削除がある・`// Scenario: repair は更新係に触れない`（`repair` の命令に `ashiato-updater` が出ない。Windows の実地は 7.1）・開発版の列は `ashiato-updater-dev`
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st57_ && test "$(cargo test -p ashiato-setup st57_ -- --list 2>/dev/null | grep -c ": test$")" -ge 10'` rc=0 / `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0

## Task 6: 配布物に更新係を入れる

- [ ] 6.1 `tools/release/build.sh server-windows` と `package.sh windows`（配る zip と `--test-build` の試験の zip）が `ashiato-updater.exe` を同じ版番号で組んで zip の根に入れる。Linux の配布物は変えない。`release.yml` の Windows の組み立ても同じ道を通る。
  `check-artifacts.sh` に、Windows の zip に `ashiato-updater.exe` がある・その組み立ての印の版番号がサーバと同じ・配る `ashiato-updater.exe` に `ASHIATO_UPDATER_TEST_`・`test-hooks.json` の文字列が無く `https://api.github.com/repos/rai-256/ashiato/releases` がある・Linux の配布物に `ashiato-updater` が無い、を足す（PC の収集アプリの同じ検査は、宛先を入れる Task 8.1 の 8.2 で足す。review/spec.md R18）。
  `tools/release/notices.sh` の `server-windows` が `ashiato-updater` の依存の木も辿り、`check-artifacts.sh` が Windows 版の `THIRD-PARTY-NOTICES.txt` に更新係だけが持つ TLS の部品（`rustls`）の項があることを見る（`echo "== Scenario: サーバの Windows 版の依存の一覧は、更新係だけが持つ部品を含む"`。review/spec.md R11）。
  ST55 の印のある `check-artifacts.sh` の確かめ（`Windows の配布物にインストーラと PostgreSQL が入り、管理画面は入らない`・`インストーラとサーバの版番号はそろう`・`配る組み立ては試験の差し込みを持たない`・`Linux の配布物は PostgreSQL を持たない`）を MODIFIED の THEN に合わせる。
  `--self-test` に、更新係の差し込みの名前を含む写しと、宛先の無い写しと、依存の一覧に `rustls` の無い写しで rc≠0 を足す
  検証: `tools/release/check-artifacts.sh --self-test` rc=0 / `tools/release/package.sh --self-test` rc=0 / `tools/release/notices.sh --self-test` rc=0 / `tools/check-licenses.sh` rc=0 / `bash -c 'tools/check-vulns.sh --base "$(git merge-base HEAD origin/main)"'` rc=0

## Task 7: Windows の CI で更新係を確かめる

- [ ] 7.1 新しい `tools/release/update-check-install.sh`（D18（仮））を CI の `install-windows` に足す（`install-check.sh` の後、同じ runner で）。**job の最初のインストール（`install-check.sh` の配る zip）より前に**、更新係の実行ファイルから loopback 以外への外への接続を塞ぐ規則を job の段として置き、job の終わりまで外さない。試験の zip は loopback の受け手を立てて `ASHIATO_SETUP_TEST_UPDATER_URL` を渡す。
  置く Scenario（`echo "== Scenario: <名前>（配る zip | 試験の zip）"`）: `更新係はサービス専用のアカウントで自動で起きる`・`更新係のアカウントは状態の置き場を読めない`・`更新係の置き場は、サーバが読めて書けない`・`切ると更新係のサービスが止まり、問い合わせが無くなる`・`入れ直すと更新係が起きて確かめる`・
  `インストールの最後に更新の確認が有効なことと切り方を示し、その後に更新係が起きる`・`配布元に届かなくてもインストールは終わる`・`更新係の無いインストールを走らせ直すと、更新係が足される`（試験の組み立ての差し込みで更新係を消した形）・`切った更新の確認は、走らせ直しでも切ったまま`・`repair は更新係に触れない`。
  `install-check.sh` の ST55 の確かめ `配る zip からのインストールで、2 つのサービスが自動で起きる形で入る`・`消すとサービスとプログラムと登録が消える`・`消しても記録の置き場は残り、その場所を示す` は、MODIFIED の THEN（`ashiato-updater` の登録・削除・更新係の置き場が残る）に合わせて確かめを直す（印の名前は変えない）。`インストールの間、ネットワークに出ない` の外向きの遮断の一覧（`install-check.sh` の `block_outbound_program`）に `ashiato-updater.exe` を足し、MODIFIED の「インストーラの段」の範囲で見る（review/spec.md R9）。
  `--self-test`（偽の `sc.exe`・`icacls`・`netsh` を `PATH` の先に置く。Linux で走る）は各確かめが合う出力で通り、1 つずつ外れた出力で rc≠0 になることを見る。job の終わりに `update-check-install.sh --list` の全部が出力に出たかを見る
  検証: `tools/release/update-check-install.sh --self-test` rc=0 / `tools/release/install-check.sh --self-test` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0

## Task 8: PC の収集アプリの更新の確認

- [ ] 8.1 `crates/collector-windows`（D2・D4・D5・D8・D12（仮）・D17（仮）・D20（仮）・D23・D25・D26（仮）・D28（仮））: `ureq` に機能 `rustls`・`platform-verifier` を足し、`build.rs` で `cfg(ashiato_test_build)` を立てる（その時だけ `ASHIATO_COLLECTOR_TEST_UPDATE_URL`・`ASHIATO_COLLECTOR_TEST_NOW` を読む）。宛先は配布元の定数。
  起動の時と実行時の 1 時間ごとの仕事で、送り先が設定済みで確認が有効なら `due` を見て確かめ、置き場の `update-check.json` と `update-checks.jsonl` に書く。新しい版ごとに 1 回札 `update`、7 日確かめられなければ続く間に 1 回札 `update-stale`（D20（仮）の文）。サーバには問い合わせない。
  `update-check off|on` の口と、`setup` の保存の後の「新しい版を確かめますか [Y/n]」（空の行は確かめる。値は `settings.json`）。`tools/check-no-time-server.sh` が今のまま通る（子プロセスを足さない）。
  試験（`#[cfg(test)]` と `tests/`、名前は `st57_pc_`。loopback の受け手と差し替えた時計）: `// Scenario: PC の収集アプリの問い合わせは版の一覧の取得だけである`・`試験の組み立ては差し替えの宛先が無ければ確かめない`・`PC の収集アプリは送り先を設定するまで確かめない`・`PC の収集アプリは新しい版をサーバに問い合わせない`（サーバの宛先の受け手が 0 件）・`確認の無い前の版から入れ替えた収集アプリは、切り方を示してから確かめる`（D23。札 `update-intro`）・`入れ替えた PC の収集アプリは、トーストを出せない間は問い合わせない`（偽の通知の出し手が `Err` を返す。起動し直すと試み直す）・`入れ替えた PC の収集アプリは、トーストを出せた後の機会に確かめる`・`入れ替えた PC の収集アプリは、起動し直さなくても定期の時に切り方のトーストを試み直す`（D25。その回の観測を差し替え、ロックしておらず経過 60 秒）・`入れ替えた PC の収集アプリは、画面のロック中の定期の時には切り方のトーストを試みない`（D25。観測は `locked` が真で経過 60 秒。偽の出し手が呼ばれた回数 0）・`入れ替えた PC の収集アプリは、最後の入力から 5 分を過ぎた定期の時には切り方のトーストを試みない`（D25。301 秒で 0 回・300 秒で 1 回）・`入れ替えた PC の収集アプリは、入力の経過を読めない定期の時には切り方のトーストを試みない`（D28（仮）。`IdleRead::Unavailable`）・`入れ替えた収集アプリは、示すまでの間を 7 日に数えない`（D26（仮））・
  `サーバの確認を切っても、収集アプリは確かめる`・`PC で update-check off にすると問い合わせが無くなる`・`PC の setup は確かめるかを聞き、既定は確かめる`（標準入力を差し替える）・`携帯で切っても PC は確かめる`（携帯の値に依らず PC の値だけを見る）・`PC は新しい版ごとに 1 回だけ通知する`（偽の通知の出し手）・`PC の更新の通知はほかの知らせと文が違う`・
  `6 日確かめられていないだけなら知らせない`・`7 日確かめられなければ PC が 1 回知らせる`・`確かめられたら、次に続いたときにまた知らせる`・`確認を切った部品は確かめられていないと知らせない`・`最後に確かめられた時刻が未来なら、7 日が過ぎたとみなす`
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st57_pc_ && test "$(cargo test -p ashiato-collector-windows st57_pc_ -- --list 2>/dev/null | grep -c ": test$")" -ge 23'` rc=0 / `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows'` rc=0（既存の試験が通る） /
  `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `tools/check-no-time-server.sh` rc=0 / `tools/check-licenses.sh` rc=0 / `tools/check-updater-deps.sh` rc=0
- [ ] 8.2 配る PC の収集アプリの宛先: `check-artifacts.sh` が、配る `ashiato-collector-windows.exe` に `ASHIATO_COLLECTOR_TEST_UPDATE_URL` が無く `https://api.github.com/repos/rai-256/ashiato/releases` があることを見る。6.1 の `--self-test` の `echo "== Scenario: 配る組み立ては宛先を変える口を持たない"` を、更新係と PC の両方を見る形にしてここで置く（宛先を入れた後の Task なので、本物の配布物でも落ちない。review/spec.md R18）
  検証: `tools/release/check-artifacts.sh --self-test` rc=0

## Task 9: 携帯の収集アプリの更新の確認

- [ ] 9.1 `collector-android`（D2・D3・D4・D5・D8・D13（仮）・D16（仮）・D20（仮）・D23・D24・D26（仮）・D29（仮））: `build.gradle.kts` に `BuildConfig.UPDATE_CHECK_URL`（release は配布元の定数。`-PupdateCheckUrl` は debug にだけ効き、release で渡すと組み立てが落ちる）。`UpdateCheck.kt`（選び方・時期・7 日。共有の材料で試す）と `UpdateChecker.kt`（`HttpURLConnection` で `GET`・見出しを D2 の値に固定し `http.agent` の既定を送らない・gzip を受ける・網の種類を見ない）。
  `LocationService` の起動と生存信号の定期の時に、送り先が設定済みでスイッチが入なら `due` を見て確かめる。結果は `update_check.xml`、記録は `noBackupFilesDir/update_checks.jsonl`。最初の画面（`MainActivity`）に「更新あり <版>（<M>/<D> に見つけた）」と「更新を確かめられていない…」の行。チャンネル `update` に新しい版ごとに 1 回・確かめられない間に 1 回の通知（押すと `MainActivity`）。`SetupActivity` に「新しい版を確かめる」のスイッチ（既定は入）。入れ替えた携帯の示した印（D23）: 通知を出せるかはチャンネルを引数に取る 1 か所の関数で、許可・`areNotificationsEnabled()`・チャンネル `update` の重要度を見る。印の無い間は `MainActivity` に「更新の確認: 有効」と切り方の行をほかの行より上に組み、`setContentView` した時に印を立てる（権限を求めている途中は立てない・ランチャー以外から開かれて他の行が無くても入れ替えの行を出す間は `finish()` しない・行を出すかは「印が無いか、この Activity が既に行を出した」で決める。D24）。同じ回に使用状況のアクセスの設定を前に出した回は立てず、未使用のアプリの制限の確認の窓が出ている間（前の回から残っている窓も含む）は組み直しても立てず、窓が設定を開かずに閉じた時に立てる（D29（仮））。`SetupActivity` のスイッチは送り先の入力欄より上に置き、画面を組んだ時に印を立てる（D24）。示すまでは問い合わせず、「確かめられていない」も出さない（D26（仮））。
  `health_full_backup_content.xml` と `health_data_extraction_rules.xml` で `update_check.xml` を外し、`tools/check-android-backup-rules.sh` が見る名前に足す（`--self-test` に行を消した写しを足す）。
  試験（Robolectric、`St57UpdateCheckTest`・`St57UpdateNoticeTest`）: `// Scenario: 3 つの部品は同じ材料で同じ版を選ぶ`（材料の `cases` と `timing` の全件。Rust は Task 1 と同じ材料）・`携帯の収集アプリは送り先を設定するまで確かめない`・`携帯の収集アプリは新しい版をサーバに問い合わせない`・`確認の無い前の版から入れ替えた携帯は、切り方を示してから確かめる`（D23）・`入れ替えた携帯は、通知を出せず画面も開かなければ問い合わせない`（通知の許可を外す形と、`areNotificationsEnabled()` が false の形の 2 通り）・`入れ替えた携帯は、通知を後から許すと次の機会に切り方を通知してから確かめる`・`入れ替えた携帯は、設定の画面を開くと示したとみなす`（D24。`ActivityScenario` で `SetupActivity` を開き、スイッチの子の位置が入力欄より前であることをアサートする）・`入れ替えた携帯は、チャンネル update だけを切ってあっても示したとみなさない`（チャンネル `update` だけを `IMPORTANCE_NONE`）・`入れ替えた携帯は、画面に切り方を出した後の機会に確かめる`（D24。`ActivityScenario` で `MainActivity` を開き、入れ替えの行が縦の並びの最初の子であることをアサートする）・`入れ替えた携帯は、権限を求めている途中は画面に出したとみなさない`（D24。権限を与えないまま開き、`ShadowActivity` で権限の要求が出たことと印が無いことを見る）・`入れ替えた携帯は、ランチャー以外から開かれて他の行が無くても入れ替えの行を閉じずに出す`（D24。`ACTION_MAIN` でない Intent で開き、`isFinishing` が偽であることと印が立つことを別々にアサートする）・`入れ替えた携帯は、同じ回に確認の窓を重ねたときは窓が閉じた時に画面に出したとみなす`（D27（仮）。`ShadowAlertDialog` で窓を取り、出ている間は印が無く、否定のボタンで閉じると立つ）・`入れ替えた携帯は、確認の窓を「設定を開く」で閉じた回は画面に出したとみなさない`・`入れ替えた携帯は、前の回の確認の窓が出たまま画面を組み直しても画面に出したとみなさない`（D29）（D27（仮）。肯定のボタンで閉じ、`getNextStartedActivity` が設定で印が無い。戻して `onResume` の後に立つ）・`入れ替えた携帯は、示した後に新しく開いた最初の画面には入れ替えの行を出さない`（D24。`ActivityScenario` を閉じて開き直す）・`入れ替えた携帯は、同じ画面を組み直しても入れ替えの行を残す`（D24。同じ Activity で `onRequestPermissionsResult` と `onResume` を続けて呼び、行が残る）・`入れ替えた携帯は、同じ回に別の画面を前に出したときは画面に出したとみなさない`（D27（仮）。`ShadowActivity.getNextStartedActivity` が使用状況のアクセスの設定で、印が無い。戻して `onResume` の後に印が立つ）・`入れ替えた携帯は、切り方を画面に出した時に確かめられていないを並べない`（D26（仮））・`携帯でスイッチを切ると問い合わせが無くなる`・
  `携帯は新しい版を最初の画面に出す`・`携帯は新しい版ごとに 1 回だけ通知する`・`携帯の更新の通知を押すと最初の画面が開く`・`携帯は 7 日確かめられなければ画面に出し、1 回通知する`・`携帯は携帯の回線でも確かめる`（`ShadowConnectivityManager` で回線だけ）
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*St57*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St57*.xml | grep -c "<testcase ")" -ge 23'` rc=0 /
  `tools/check-android-backup-rules.sh` rc=0 / `tools/check-android-backup-rules.sh --self-test` rc=0（`echo "== Scenario: 携帯の確かめた記録はバックアップから外れる"`。記録のファイルが `noBackupFilesDir` の下にあることは Robolectric の `St57UpdateCheckTest` で見る） / `tools/check-no-time-server.sh` rc=0
- [ ] 9.2 本物の Android の上の問い合わせ（D2・D13（仮））: 計測テスト `St57UpdateQueryTest`（`app/src/androidTest`）が、端末の中に loopback の受け手を立て、`-PupdateCheckUrl` で向けた debug の組み立てで 1 回確かめ、`// Scenario: 携帯の問い合わせは版の一覧の取得だけである`（見出しの名前が 5 つの内側・`User-Agent`・`Accept` の値・本文 0 バイト・`Build.MODEL`・`Build.VERSION.RELEASE`・`BuildConfig.VERSION_NAME` がどの値にも無い）と
  `// Scenario: 携帯は圧縮した応答を受ける` を見る。届いた見出しの名前と値（機種名は伏せる）を premises.md の P10 に確かさ 4 で書き直す。5 つの外の名前が来たら D13（仮）の反転条件として上流へ返す
  検証: `scripts/quiet-run emu -- tools/android-emulator.sh` rc=0（計測テストを全部走らせる） / `bash -c 'test "$(cat collector-android/app/build/outputs/androidTest-results/connected/*/TEST-*.xml collector-android/app/build/outputs/androidTest-results/connected/TEST-*.xml 2>/dev/null | grep -c "<testcase [^>]*St57UpdateQueryTest")" -ge 2'` rc=0（review/spec.md R18）
- [ ] 9.3 携帯の配る APK の宛先: `tools/release/build.sh apk` で組んだ release の APK の `BuildConfig.UPDATE_CHECK_URL` が配布元の定数で、release に `-PupdateCheckUrl` を渡すと組み立てが落ちることを、`check-artifacts.sh`（APK の dex の文字列を読む）と `tools/release/build.sh --self-test` に置く（`echo "== Scenario: 携帯の配る APK の宛先は配布元である"`）
  検証: `tools/release/check-artifacts.sh --self-test` rc=0 / `bash -c 'tools/release/build.sh --self-test'` rc=0

## Task 10: 手順書と申し送り

- [ ] 10.1 `docs/install.md` に「更新の確認」の節（有効なこと・`ashiato-setup.exe update-check off|on`・更新係の置き場と確かめた記録の場所・配布元へ送るもの（版の一覧の取得だけ・元の IP と時刻は見える）・手で消すとき `sc.exe delete ashiato-updater`）。`docs/collector-setup.md` に携帯のスイッチと PC の `update-check off|on`。
  `docs/release.md` に「リポジトリの名前を変えると、配った版の更新の確認は 7 日後に『確かめられていない』を出す」を足す。申し送り: `docs/handoff/ST58.md`（`GET /update-status` の形・帯を押す口はまだ無い・切った確認を更新で戻さない・前の版へ戻したときの更新係のサービスの扱いは未確かめ）・`docs/handoff/ST60.md`（Linux の更新係の起こし方。C10）
  検証: `bash -c 'for h in "update-check off" "ashiato-updater" "版の一覧"; do grep -q -- "$h" docs/install.md || exit 1; done && grep -q "update-check" docs/collector-setup.md && grep -q "update-status" docs/handoff/ST58.md && grep -q "更新係" docs/handoff/ST60.md'` rc=0
- [ ] 10.2 全体: Scenario の印の突き合わせと、作業場の検査
  検証: `python3 scripts/check_scenarios.py . st57-update-notice` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && scripts/quiet-run all -- cargo test --workspace'` rc=0 /
  `bash -c 'tools/check-offsite.sh && tools/check-updater-deps.sh && tools/check-contract-version.sh && tools/check-licenses.sh && tools/check-openapi.sh'` rc=0

## 人間の確認待ち（確認バッチ。PR の CI が緑になった後）

Scenario ではない —— 機械が全部の Scenario を見る。ここは本物の配布元・本物の端末の通知の見え方で、結果は確認バッチが `premises.md` に書く。

- [ ] 11.1 **本物の配布元で 1 回確かめる**: 配布元に最初のリリース（ベータ版か開発版）が出た後、本人の PC のサーバ（走らせ直しで更新係が足された形）・携帯・PC の収集アプリで「更新あり」か「新しい版なし」が出ることを見る。**暗号化した接続が OS の根で通ること（`NT SERVICE\ashiato-updater` を含む）を最初に見るのはここ**（試験と CI は loopback の平文の受け手で、本物の TLS を通らない。D12（仮）の反転条件。review/spec.md R12）。いまは配布元のリリースが 0 件（P1）なので、出るまで待つ
  > 物理: realdata —— 本物の配布元のリリースは手元の受け手では作れない（試験・CI は本物の配布元に聞かない。D8）
- [ ] 11.2 **本人の PC の走らせ直し**: ST55 で入れた本人の PC で新しい版の zip のインストーラを走らせ直し、更新係が足され、DB と設定はそのままで、最後に「更新の確認: 有効」と切り方が出ることを見る
  > 物理: realdata —— 本人の PC の前のインストールの状態は CI で再現しきれない
