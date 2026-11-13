# ST57 design — 新しい版が出たら知らせる

## Context

動機は `proposal.md` の Why。いまの状態（`premises.md`・`review/deep.md` の手順 5）:

- 更新の確認のコードは 1 行も無い。配布元は公開のリポジトリ `rai-256/ashiato` で、認証なしで一覧を取れる（P1。いまはリリース 0 件で `[]`）。上限は IP ごとに 1 時間 60 回（P2）。`User-Agent` の無い問い合わせは断られる（P3）
- サーバは外へ送る部品を持たない（`tools/check-offsite.sh` が `cargo tree -p ashiato-server` を見る。P11）。サービスは `ashiato-db`・`ashiato-server`、置き場の並びは `crates/installer/src/layout.rs`（ST55 D14。申し送り `docs/handoff/ST57.md`）
- 状態の置き場（`C:\ProgramData\ashiato`）は 2 つのサービスのアカウントと本人だけが読める（P12）。消す口は 2 つのサービスだけを消す（`commands.rs` の `uninstall_commands`）
- PC の収集アプリは TLS を持たない（`ureq` の `default-features = false`。P8）。試験の組み立ての印（`ashiato_test_build`）はインストーラとサーバにだけある
- 携帯は https の宛先を制限しない（P9）。`HttpURLConnection` は既定の `User-Agent` に機種と OS の版を入れる（P10。確かさ 1）。携帯に WorkManager は無く、`LocationService`（前景のサービス）が定期の仕事を持つ
- 画面の見出しは各画面の `<header>`、画面の切り替えは `web/src/Root.tsx`（ハッシュ）。ひとスクロールの勘定（`web/src/__tests__/one-scroll.test.tsx`）は帯の無い形で数える
- 版の出所と系統は `crates/build-stamp`（`Channel::Dev` / `Official`）と携帯の `BuildConfig.CHANNEL`・`VERSION_NAME`

## Goals / Non-Goals

**Goals**: 3 つの部品が 1 日 1 回、版の一覧だけを取って新しい版に気づき、本人に見せること。サーバの守り（外へ送る部品を持たない）を開けないこと。確かめられない状態を黙らせないこと。試験と CI が本物の配布元に聞かないこと。

**Non-Goals**:
- 「更新あり」を押して落として入れる（FR-114。ST58）。帯は押しても何もしない（Q8 の推奨）
- 自動で入れる設定（FR-114 の「後で足す」）
- Linux のサーバの更新係の起こし方（FR-131 の Story。D10）
- ベータ版を追う設定（Q2 の選択肢 2。選ばなかった）
- 画面から更新の確認を切る口（Q3 の選択肢 2。選ばなかった）

## Decisions

### D1. 更新係は別の crate・別の実行ファイル。純粋な部分は共有の crate に分ける

> 聞かない既定（`defaults.md` C1）

- `crates/update-check`（package `ashiato-update-check`）: 一覧の読み方・版の選び方・時期の判定（24 時間・未来の時刻・7 日・48 時間）・結果と記録の形・問い合わせの道と見出しの定数。**HTTP・TLS・DB の部品を持たない**（`serde`・`serde_json`・`semver` だけ）
- `crates/updater`（package `ashiato-updater`・bin `ashiato-updater`）: `ashiato-update-check` + HTTP（`ureq` + TLS。D12（仮））+ `crates/winsvc`（`cfg(windows)`）+ `ashiato-build-stamp`。サービスの口 `ashiato-updater service` と、試験のための 1 回だけ確かめる口 `ashiato-updater once`
- サーバは `ashiato-update-check` だけに依る（結果の読み方と版の比べ方）。`ashiato-updater` には依らない。`check-offsite.sh` は今のまま通る
- 新しい検査 `tools/check-updater-deps.sh`: `cargo tree -p ashiato-updater -e normal` に DB の部品（一覧は検査が持つ: `sqlx`・`tokio-postgres`・`postgres`・`diesel`・`sea-orm`）が無いこと、`ashiato-updater` と `ashiato-collector-windows` に同梱の根の一覧の部品（`webpki-roots`・`webpki-root-certs`）が無いこと、`cargo tree -p ashiato-server -e normal` に `ashiato-updater` が無いこと、`cargo tree -p ashiato-update-check -e normal` に `check-offsite.sh` の一覧の部品が無いこと。`--self-test` つき。CI の `lint` の job に足す

補足: `ashiato-update-check` は RFC 3339 の時刻を型安全に扱うため `chrono`（serde 機能のみ）も使う。HTTP・TLS・DB の部品ではなく、D1 の「純粋な部分」の範囲に含める。

### D2. 問い合わせは決まった形だけ。見出しの集合を名指しする

> 聞かない既定（`defaults.md` C2）。review/deep.md R8

- 道は開発版でない部品が `/repos/rai-256/ashiato/releases?per_page=20`、開発版の部品が `/repos/rai-256/ashiato/releases/tags/nightly`（一覧の中で沈む P7 に依らない）
- `GET`・本文なし・`User-Agent: ashiato-update-check`・`Accept: application/vnd.github+json`・`Accept-Encoding: gzip`（C2 の 4 つ。Android が足す `Connection` を許すのは D13（仮））。転送は同じ `api.github.com` の中だけ追う（最大 3 回）。時間切れは接続 10 秒・全体 30 秒
- 証明書の根は OS のもの（D12（仮））
- FR-116 の機械の検査は、各部品の問い合わせを手元の受け手（loopback の HTTP）で受け、方法・道・見出しの名前の集合・固定の値・本文の長さ・自分の版と識別子が無いことを試験で固定する。携帯は本物の Android の上（計測テスト）で見る（Robolectric は JVM の通信の部品で送るので、本物の見出しを見ない）

### D3. 結果は「見つけた最新の版」で持ち、「更新あり」は読む側が決める。選び方は共有の材料で試す

> 聞かない既定（`defaults.md` C3）

- 選び方は spec の「新しい版かどうかは…」の規則のとおり。比べ方は SemVer（`0.4.0` は `0.4.0-beta.2` より新しい。開発版の `dev.<日付>` は数で比べる）
- 共有の材料 `docs/contracts/update-check-vectors.json`: 各件は「自分の版・部品（`server` / `collector-windows` / `android`）・一覧（GitHub の形の JSON の抜き）・見つけた最新の版・更新あり／なし」。Rust（`ashiato-update-check`）と Kotlin（`UpdateCheck.kt`）が全件で試す（ST56 の `collector-setup-vectors.json` と同じ形）
- 開発版の部品が別の土台の版の開発版を見たとき（`0.4.0-dev.20261110` と `0.5.0-dev.20261101`）も SemVer で比べる（D21（仮））

### D4. 確かめた記録は追記で残す。携帯ではバックアップから外す

> 聞かない既定（`defaults.md` C4）。review/deep.md R3

- 1 行 = `{"at": <RFC 3339>, "ok": bool, "reason": <なし | "unreachable" | "timeout" | "rate_limited" | "not_found" | "unreadable" | "no_destination">, "latest": <版 | null>}` の JSON Lines
- 置き場: 更新係は `C:\ProgramData\ashiato-updater\checks.jsonl`、PC は `%LOCALAPPDATA%\ashiato\collector\update-checks.jsonl`、携帯は `noBackupFilesDir/update_checks.jsonl`（OS がバックアップしない置き場）。1 MB を超えたら古い半分を `*.1.jsonl` に退ける（D19（仮））
- 結果（`status.json` / 携帯は SharedPreferences `update_check.xml`）: `latest`・`found_at`・`checked_at`・`last_ok_at`・`last_reason`・`notified_version`・`stale_notified`
- 携帯の `update_check.xml` を `health_full_backup_content.xml` と `health_data_extraction_rules.xml`（`cloud-backup`・`device-transfer`）で外し、`tools/check-android-backup-rules.sh` が見る名前に足す

### D5. 「1 日 1 回」は最後に確かめてから 24 時間。最初の問い合わせは切る口に触れた後

> 聞かない既定（`defaults.md` C5）。review/deep.md R2・R4

- 判定は純粋な関数 `due(now, last_checked_at) -> bool`（`last_checked_at` が無い・24 時間以上前・未来なら真）。失敗も `last_checked_at` を進める
- 機会: 更新係は起動の時と 1 時間ごと。PC は起動の時と、実行時の 1 時間ごとの仕事。携帯は `LocationService` の起動の時と、生存信号の定期の時（D16（仮））
- 最初の問い合わせ: 更新係はインストーラが最後の表示の後にサービスを開始する（それより前はサービスが止まっている）。携帯と PC は送り先の設定を保存したことが前提（`SetupStore` / `settings.json` に送り先がある。組み立ての時に `-P` で渡した送り先も設定済みに数える —— spec に書いた）。前の版から入れ替えた部品は D23（切り方を示すまで問い合わせない。Q11）

### D6. 更新係の置き場は状態の置き場の外。切った印はインストーラが書く

> 聞かない既定（`defaults.md` C6・C9）。review/deep.md R9

- `layout.rs` に列を足す: 更新係のサービス `ashiato-updater`（開発版 `ashiato-updater-dev`）・置き場 `C:\ProgramData\ashiato-updater`（開発版 `C:\ProgramData\ashiato-updater-dev`）
- 置き場の中: `settings\check.json`（`{"enabled": bool, "since": <RFC 3339>}`。管理者と SYSTEM だけが書く）・`status.json`・`checks.jsonl`（更新係が書く）・`logs\`（更新係のログ）
- ACL: 受け継ぎを切り、`NT SERVICE\ashiato-updater` に読み書き（`settings` は読みだけ）・`NT SERVICE\ashiato-server` と本人に読み・Administrators と SYSTEM にすべて。状態の置き場の ACL には更新係を足さない
- サーバの「切ってある／動いていない／確かめられていない」は `check.json` と `status.json` の時刻だけで決める（サービスの状態を問い合わせない）。`enabled: false` なら「切ってある」、置き場が無ければ「動いていない」、`status.json` の `checked_at` が 48 時間より古ければ理由を「更新係が動いていない」（D15（仮））

### D7. 走らせ直しで足し、切った確認は戻さない。消す口は更新係も消す。`repair` は触れない

> 聞かない既定（`defaults.md` C7）。review/deep.md R10

- 走らせ直し: 更新係のサービスが無ければ登録し、置き場と ACL を足す。`check.json` が無ければ `enabled: true`・`since` = 今で作る。あれば触らない。`enabled: false` ならサービスの開始の種類を「無効」にしたまま開始しない
- 消す: `uninstall_commands` に更新係のサービスの停止と `sc.exe delete` を足す。更新係の置き場は残し、出力に場所を示す。ACL の後片付けは状態の置き場と同じ
- `repair` の命令の一覧は変えない（spec の ADDED「`repair` の口は、更新係のサービスと置き場に触れない」。試験で、`repair` の命令に `ashiato-updater` が出ないことを見る）
- 正典の 3 つの Requirement（インストール・走らせ直し・消す口）と配布物の中身、依存の許諾の一覧（サーバの Windows 版に更新係の部品。`tools/release/notices.sh` の `server-windows` が `ashiato-updater` の依存の木も辿る。review/spec.md R11）を MODIFIED にした

### D8. 宛先は組み立ての時の定数。差し替えは試験の組み立てだけ

> 聞かない既定（`defaults.md` C8）。review/deep.md R5

- 更新係: `build.rs` が `ASHIATO_RELEASE` の無い組み立てに `cfg(ashiato_test_build)` を立て（インストーラと同じ）、その時だけ `ASHIATO_UPDATER_TEST_URL`（宛先の根）と `ASHIATO_UPDATER_TEST_NOW`（時計の差し込み）を読む。無ければ確かめない（「宛先なし」を記録）。
  サービスとして動く試験の組み立ては環境変数を受け取れないので、同じ 2 つを更新係の置き場の `settings\test-hooks.json` からも読む（試験の組み立てだけ。インストーラの試験の組み立てが `ASHIATO_SETUP_TEST_UPDATER_URL` を渡されたときだけ書く）
- PC の収集アプリ: 同じ形の `cfg(ashiato_test_build)` を足し、`ASHIATO_COLLECTOR_TEST_UPDATE_URL` を読む
- 携帯: `BuildConfig.UPDATE_CHECK_URL`。release の組み立ては配布元の定数で、`-PupdateCheckUrl` は debug の組み立てにだけ効く（release で渡すと組み立てが落ちる）。debug で渡さなければ空で、確かめない
- `check-artifacts.sh`: 配る `ashiato-updater.exe`・`ashiato-collector-windows.exe` に `ASHIATO_UPDATER_TEST_`・`ASHIATO_COLLECTOR_TEST_UPDATE_URL` が無く、`https://api.github.com/repos/rai-256/ashiato/releases` があること

### D9. サーバは切った印と時刻だけで状態を見分ける

> 聞かない既定（`defaults.md` C9）。D6 に書いた

### D10. Linux のサーバはこの Story では確かめない

> 聞かない既定（`defaults.md` C10）。review/deep.md R16

- Linux の配布物の中身は変えない。サーバは更新係の置き場が無ければ「動いていない」を返し、帯を出さない（設定の節にだけ出す）
- 開発の形（環境変数で起こすサーバ）は `ASHIATO_UPDATER_DIR` があればそこを、無ければ置き場なしとして読む（e2e が偽の結果を置くための口。D14（仮））
- Linux の更新係の起こし方は `docs/handoff/ST60.md` に申し送る

### D11（仮）. 名前

- crate `ashiato-update-check`・`ashiato-updater`、実行ファイル `ashiato-updater.exe`、サービス `ashiato-updater`（`-dev`）、置き場 `C:\ProgramData\ashiato-updater`（`-dev`）、サーバの口 `GET /update-status`、携帯のチャンネル `update`・SharedPreferences `update_check.xml`、PC の札 `update`・`update-stale`・`update-intro`、共有の材料 `docs/contracts/update-check-vectors.json`
- 反転条件: ST58 の上流が、入れる役を同じ実行ファイルに持たせて名前が合わなくなったとき（名前は配った版に焼き込まれるので、配る前に決め直す）

### D12（仮）. TLS は `rustls` と OS の根の確かめ（`rustls-platform-verifier`）

- `ureq` の機能 `rustls` と `platform-verifier`。Windows は OS の証明書の置き場、Linux（試験）は OS の根を読む。OpenSSL を要さないので Linux の CI でそのまま組める
- 更新係と PC の収集アプリの両方に同じ機能。`tools/check-licenses.sh`・`tools/check-vulns.sh` に通す
- 反転条件: 許諾の検査か脆弱性の検査が落ちて互換の範囲で直せないとき、または `NT SERVICE\ashiato-updater` で OS の根を読めない（Windows の CI で確かめられない）ときは `native-tls`（Windows は schannel）に替える

### D13（仮）. 見出しの集合は 5 つの名前の内側

- 許すのは `Host`・`User-Agent`・`Accept`・`Accept-Encoding`・`Connection`。`Connection` は Android の通信の部品が `Keep-Alive` を自分で足す（P10 と同じく知識。計測テストで 4 に上げる）ので許す。どの部品も値は定数で、自分の情報を含まない
- 反転条件: 計測テストで Android が 5 つの外の見出しを足すと分かったとき（その名前と値が自分の情報を含まなければ集合に足し、含むなら送り手を替える）

### D14（仮）. サーバの読みの口 `GET /update-status` の形

- 欄と `state` の 8 つは spec に書いた（review/spec.md R5・R10）。ここに残すのは仮の部分: `reason` の符号（D4 の 6 つと `not_running`）、`pending` から `stale` への 7 日の起点は `check.json` の `since`
- 画面のログインが要る（API の合言葉の許可の一覧に入れない。ST44 の既定で 403）。読み出しの記録は今の層（`access_log.rs` の `middleware`。`/healthz`・`/session`・`/collector-setup` のほかは全部の口を名前で残す）のとおり、口の名前 `/update-status` で成功も断りも 1 行ずつ残す（層の例外に足さない）
- 結果のファイルは信用しない入力として読む: 64 KB を超えれば読まない、版は SemVer の形だけ、時刻は RFC 3339 だけ。外れたら `unreadable`（帯を出さず、設定にだけ「結果を読めない」）。壊れたファイルで 7 日を待たずに帯が出ないようにした（review/spec.md R5）
- 置き場はサービスの形では系統の定数（D6）、開発の形は `ASHIATO_UPDATER_DIR`（D10）
- 反転条件: ST58 の上流が、押して入れる口のために別の形を要るとき（`GET` の欄を足すのはよい。消すのは ST58 と合わせる）

### D15（仮）. サーバは更新係の時刻が 48 時間より古ければ「更新係が動いていない」とする

- 更新係は 1 時間ごとに機会があり、24 時間ごとに必ず確かめる。48 時間は 1 回の取りこぼしを許す長さ
- 反転条件: 本人の PC が夜に止まる運用で 48 時間を毎週越えると分かったとき（長くするだけで、記録から計算し直せる）

### D16（仮）. 携帯の機会は `LocationService` の起動と生存信号の定期の時

- 新しい依存（WorkManager）を足さない。前景のサービスが動いていない携帯は確かめない（収集も止まっている）
- 反転条件: 確認バッチで、生存信号の定期の時が端末の省電力で 24 時間を大きく超えて来ないと分かったとき

### D17（仮）. PC は `setup` の最後に聞き、`update-check off|on` の口でいつでも切り替える

- PC の設定の窓は黒い窓（ST56）で、チェックの箱が無い。`setup` は保存の後に「新しい版を確かめますか [Y/n]」を聞く。`update-check` の口は設定の文字列を貼り直さずに切り替える
- 反転条件: PC の収集アプリに画面（GUI）の設定の窓ができたとき（そこにチェックの箱を置く）

### D18（仮）. Windows の CI は更新係から外への接続を塞いでからインストールする

- `install-windows` の job は、**job の最初のインストール（`install-check.sh` の配る zip）より前に** `netsh advfirewall firewall add rule … dir=out program=<プログラムの置き場>\ashiato-updater.exe action=block` を置き、job の終わりまで外さない（本物の配布元に聞かない。共有の runner の IP で上限 60 回に当たらない）
- 試験の zip では、FR-116 の Scenario のために `ASHIATO_UPDATER_TEST_URL` を loopback の受け手に向ける（loopback は塞がない形にする: 規則は `remoteip` を loopback 以外に限る）
- 反転条件: runner で規則を置けない・loopback まで塞がれると分かったとき（その Scenario は `ashiato-updater once` を runner の上で直に走らせる形に替える）

### D19（仮）. 確かめた記録は 1 MB で古い半分を退ける

- 1 日 1 行・1 行 100 バイトほどなので、1 MB は約 30 年分。退けた分も消さない
- 反転条件: 記録に載せる欄を増やして 1 行が 1 KB を超えたとき

### D20（仮）. 文

- 画面の帯: 「更新あり <版>（<M>/<D> に見つけた）」「更新を 7 日確かめられていない（最後に確かめた <M>/<D>・<理由>）」（確かめられたことが無ければ「最後に確かめた —」）
- 理由の語: 配布元に届かない・時間切れ・問い合わせの上限・配布元が見つからない・読めない答え・更新係が動いていない
- 携帯と PC の通知: 「新しい版 <版> が出ています」「更新を 7 日確かめられていません（最後に確かめた <M>/<D>・<理由>）」
- 反転条件: 確認バッチで本人が文に違和感を返したとき（表示だけで、記録は変わらない）

### D21（仮）. 開発版も SemVer で比べる

- 開発版の部品は印 `nightly` の配布物の版が自分より SemVer で新しいときだけ「更新あり」。土台の版が上がった開発版は、日付が古くても新しい（main から作るので、土台が上がった版の方が後にできている）
- 反転条件: 土台の版を下げる運用（緊急の修正の枝から開発版を作る）が始まったとき

### D22. 携帯の通知（Q5）と「確かめられていない」（Q9）は FR-113 ★ 2026-11-11 に記録した

- Q5・Q9 の内容は FR-113（画面と PC の通知に「更新あり」）へ追加済みである。`docs/requirements.md` と `docs/stories/ST57.md` は record の生成物で、ここからは書き換えない。

### D23. 確認の無い前の版から入れ替えた収集アプリは、切り方を示すまで問い合わせない（深掘り Q11）

- 送り先を設定済みの人が新しい版に入れ替えると、「送り先を保存した後」の条件は満たされたまま。本人が切る口を見る前に最初の問い合わせが出る（問い合わせの元の IP と時刻が出る。`exported`）。review/spec.md R16
- 入れ替えの後に初めて起きた時に、「更新の確認: 有効」と切り方を 1 回示し（PC は札 `update-intro` のトースト、携帯はチャンネル `update` の通知か画面）、示した印を立て、その時には問い合わせず、その後の機会に確かめる。印は置き場（PC は `update-check.json` の `intro_shown`、携帯は `update_check.xml`）に持つ。入れたての部品は送り先の設定の時に示したものとみなす（設定の画面と `setup` の問いで切る口を見ている）
- **示したとみなす時**（本人の答え。Q11「示す口を広げ、どれでも示せない間は問い合わせない」）: 携帯は、通知を出せた時か、本人がアプリの画面を開いて「更新の確認: 有効」と切り方が画面に出た時。PC は、トーストが成功（`Ok`）を返した時。出せない間は問い合わせず、PC は次の起動で出し直す（定期の時は、画面がロックされておらず最後の入力から 5 分以内の時だけ試みる。D25・Q13）。携帯の「画面に出た時」は、行を一番上に組んで画面に出した時（D24・Q12）。通知を切っていて画面も開かない携帯と、トーストを出せない PC は、確認が止まったまま
- 携帯の「通知を出せた」は、チャンネル `update` で判断する: Android 13 以降の通知の許可・`areNotificationsEnabled()`・**チャンネル `update` の**重要度が `IMPORTANCE_NONE` でない、の 3 つが揃った時に `notify()` を呼び、そこで印を立てる。既存の `SetupNotifier.canPost()` は送り先の設定のチャンネル（`setup`）を見るので、そのまま使うと `update` だけを切った人を「出せた」と数える（Android は切ったチャンネルの `notify()` を黙って捨てる）。判断はチャンネルを引数に取る 1 か所の関数にする（整合の確認 K1。`docs/audit/2026-11-11-st57-fr112-consistency.md`）。Q11 の問い自身が「チャンネルの通知を切っている」を出せない側に挙げていたので、答えの中身として書く
- 残るもの（本人が受け入れた）: PC で Windows の設定から通知を切っていてもトーストは `Ok` を返しうるので、その人は見ないまま問い合わせが出うる（Risks）
- 示し方は携帯のチャンネル `update`（Q5・D22（仮））に、携帯の「その後の機会」は D16（仮）に乗る。経緯: 仮で進めた D23（通知で示す）を別の席（`provisional-review.md` D23）が「否」とし（通知を出せない端末での向きが無い）、台帳から外して Q11 で本人に問うた

### D24. 携帯で「画面に出した」とみなすのは、最初の画面が入れ替えの行を一番上に組んで画面に出した時と、設定の画面を組んだ時（深掘り Q12）

- 本人の答え（Q12「行を組んで画面に出した時だけ印を立てる（行は一番上、閉じずに出す）」・補足なし。推奨と同じ側）
- 印を立てていない入れ替えた携帯は、最初の画面（`MainActivity`）に「更新の確認: 有効（切るには設定の画面の「新しい版を確かめる」）」の行を、ほかの行より上（一番上）に組む。その画面を `setContentView` した時に印を立てる。`onResume` だけでは立てない
- まだ求めていない権限を求めている間（`proceed()` が `ask` / `requestPermissionGroup` / `proceedHealth` へ分かれて `start()` まで進まない回。`MainActivity.kt:197-210`）は、行を組まないので印を立てない
- ランチャー以外から開かれた時（`ACTION_MAIN` でない Intent。通知を押した時など）に他の行が無くても、入れ替えの行を出す間は `finish()` しない（`MainActivity.kt:415-423` の分かれに入れ替えの行を足す）。今は印の無いまま `MainActivity` を開く通知は無い（ST57 の `update` の通知は示した後にしか出ない）ので守りとして置く（review/spec.md R23）
- 行を出すかは「印が無い、またはこの画面（Activity）が既に行を出した」で決める。同じ Activity が生きている間は、組み直しても（権限の結果と直後の `onResume` で `start()` が続けて走っても。`MainActivity.kt:128-131`・`:145-147`）行を残し、新しく開いた最初の画面からは出さない。そのとき他の行が無く、ランチャー以外から開いた画面なら、今までどおり閉じる（review/spec.md R24）
- 設定の画面（`SetupActivity`）は「新しい版を確かめる」のスイッチを送り先の入力欄より上に置き、画面を組んだ時（`setContentView`）に印を立てる（`SetupActivity.kt:33-67` は `ScrollView` を持たないので、下に置くと見えない端末がある）
- 残るもの（本人が受け入れた。選択肢の「不可逆」）: 行が画面に出たが本人が読まずに閉じた場合は、見ないまま問い合わせが出うる（Risks）
- 同じ回に別の画面や窓を前に出す分かれ（使用状況のアクセスの設定・未使用のアプリの制限の確認）は答えが言っていないので D29（仮）（はじめの D27 は別の席が否とした）
- 経緯: はじめ「画面が前に出た時（`onResume`）」を仮で置いたが、別の席（`provisional-review.md` D24）が「否」とし（権限を求める途中・通知から開いて閉じる時に行を見ないまま印が立つ）、台帳から外して Q12 で本人が決めた

### D25. PC の出し直しは、次の起動に加えて、画面がロックされておらず最後の入力から 5 分以内の定期の時にも試みる（深掘り Q13）

- 本人の答え（Q13「定期の時は、ロックしておらず直前（5 分以内）に入力があった時だけ出し直す」・補足なし。推奨と同じ側）
- 起動の時（ログオンの直後）は今のまま試みる。定期の時（1 時間ごとの仕事）は、その回の観測で画面がロックされておらず（`obs.locked` が偽。`runtime.rs:493`）、最後のキーボード・マウスの入力からの経過（`obs.idle`。`engine.rs:82` の `IdleRead::Elapsed`）が 300 秒以下の時だけトーストを試みる。それ以外の定期の時はトーストの出し手を呼ばず、印も立てず、問い合わせも出さない
- 境界: 経過が 300 秒ちょうどなら試み、301 秒なら試みない（「5 分以内」を含む側で読む）
- 経過を読めない（`IdleRead::Unavailable`）時は答えが言っていないので D28（仮）
- Q11 の答えは「出せない間は次の起動で出し直す」。PC の収集アプリはログオンで起きて常駐するので（`main.rs` の `--install-autostart`）、次の起動は次のログオンまで来ないことがある。定期の時にも試みると、本人が PC を触っている間に止まっている時が短くなる
- 残るもの（本人が受け入れた。選択肢の「不可逆」）: 入力の直後に席を外し、トーストを見なかった人は、見ないまま問い合わせが出うる（Risks）
- 経緯: はじめ「定期の時はいつでも試みる」を仮で置いたが、別の席（`provisional-review.md` D25）が「否」とし（トースト（`toast.rs:58-72`）はロック中・離席中でも `Ok` を返す）、台帳から外して Q13 で本人が決めた

### D26（仮）. 入れ替えた収集アプリの 7 日の起点は、切り方を示した時

- 示すまで止めている間も確認は「有効」なので、7 日の Requirement のままだと、送り先の設定から数えて示した瞬間に「更新を 7 日確かめられていません（最後に確かめた —）」が並ぶ（携帯は同じ最初の画面に出る）。止めているのは本人の決定（Q11）で、確かめられないのではない
- 示すまでは「確かめられていない」を出さず、7 日の起点を示した時にする（`update-check.json` / `update_check.xml` に示した時刻を持つ）。整合の確認 C2
- 反転条件: 確認バッチで、示せないまま止まっている人に何も見えないことが困ると分かったら、示せない間も何かを出す口を足す（そのときは上流へ返す。通知も画面も使えない人に出す口は無い）


### D27. （置き換え）携帯で、同じ回に別の画面や窓を前に出したときは、それが退いてから印を立てる

- 経緯: 仮で置いたが、別の席（`provisional-review.md` D27）が「否」とした。前の回の確認の窓が出たまま（`askToExemptOnce()` は `exemptDialog?.isShowing == true` で戻る。`MainActivity.kt:442`）設定から戻って画面を組み直すと、窓が行を覆ったまま印が立つ。台帳から外し、遅らせる側へ直した D29（仮）に置き換えた（本人が 2026-11-12 に「A でよい」と答えた。今その状態の端末は無い）

### D29（仮）. 携帯で、別の画面や確認の窓が前に出ている間は印を立てず、それが退いて行が見える状態で組み直した時か窓を閉じた時に立てる

- Q12 の答えは「行を組んで画面に出した時」。ところが `start()` は、`showStatus`（`setContentView`。`MainActivity.kt:524`）の前に `sendToUsageAccessOnce()`（`:534-542`。使用状況のアクセスの設定を `startActivity` で前に出す）を、後に `askToExemptOnce()`（`:441-453`。未使用のアプリの制限の確認の窓 `AlertDialog`）を呼ぶ。どちらかが前に出ていると、`setContentView` の時に行は本人の目の前に無い
- 使用状況のアクセスの設定を前に出した回は印を立てない（設定から戻った次の `onResume` が画面を組み直す時に立てる。ただし下の窓の条件を満たす時だけ）
- 未使用のアプリの制限の確認の窓が出ている間（`exemptDialog?.isShowing == true`）は、その窓をこの回に出したか前の回から残っているかに依らず、画面を組み直しても印を立てない
- 窓が「あとで」か外を触って閉じた時（設定を開かずに閉じた時）に、入れ替えの行が出ていれば印を立てる。窓が「設定を開く」で閉じた時（`openUnusedAppRestrictionsSettings()` が設定を前に出す。`MainActivity.kt:444-452`・`:463-466`）は立てず、設定から戻って窓の無い画面を組み直した時に立てる（review/spec.md R21）
- 使用状況のアクセスの設定を開けなかった回（`sent` が偽。`:540-541`）も「前に出した回」に数えてよい（印が次に組み直すまで遅れるだけ）
- 失わない側: 印を遅らせるだけで、問い合わせは早まらない。費用は分かれの条件（窓が出ているか）
- 反転条件: 本物の端末で、確認の窓の後ろの行が見えて読める（窓が行を覆わない）と分かれば、窓の回も `setContentView` の時に立ててよい（それでも外へ出るのは遅れるだけで、戻す必要は無い）

### D28（仮）. PC で、最後の入力からの経過を読めない定期の時は、切り方のトーストを試みない

- Q13 の答えは「直前（5 分以内）に入力があった時だけ」。経過を読めない（`IdleRead::Unavailable`。`engine.rs:85` は「入力があったとも無かったとも読まない」）時は答えが言っていない
- 読めない時は「入力があった」と読まず、その定期の時には試みない（印も問い合わせも出ない）。起動の時は今のまま試みる
- 失わない側: 試みないだけで、外へ何も出ない。止まっている時が延びるのは、読めない回が続く PC だけ
- 反転条件: 本物の Windows で経過を読めない状態が長く続き、確認がずっと止まると分かれば、ロックしていなければ試みる側へ変える（上流へ返す。見ないまま出うる側なので A）

### D30. 更新の知らせは既存の通知を置き換えず、NFR-18 の本文コントラストにそろえる

- 帯の文字と地のコントラストは 4.5:1 以上にし、携帯の更新通知はチャンネル `update` と専用の通知 ID を使う。Q14 の答えどおり、切り方を示す通知は ID 9、更新ありは ID 10、確かめられていないは ID 11 とし、切り方を示した通知を更新ありで上書きしない。既存のサーバ更新通知（チャンネル `contract`）は ID 5 のまま、系統不一致通知（`ChannelMismatchNotifier`）を ID 12 へ移し、導入案内（6）とも重ならないようにする。
- FR-113 ★ 2026-11-11 を導出元として Q5・Q9 の完了形へ直し、仕様・proposal・design の未来形を残さない。
- Q14 の答え: 切り方の通知を更新ありと並べて残す。通知が 2 件並ぶ間はあるが、後から切り方を読める。
- 失わない側: 既存の通知と切り方を示した通知を消さず、本文に必要な 4.5:1 の読みやすさを守る。費用は、通知が 2 件並ぶ間と、既に表示中の旧 ID 6 の系統不一致通知が更新後に一度だけ残り得ること。

## Risks / Trade-offs

- [携帯と PC の問い合わせの元の IP と時刻が毎日配布元に残る] → Q1 で本人が選ぶ（A）。中身は版の一覧の取得だけで、記録は出ない（FR-116 の試験）
- [配った版の中の宛先・読み方・証明書の確かめ方は、その版からは直せない] → 壊れたら 7 日で「確かめられていない」が出る（Q9）。リポジトリの名前を変えるときは、配った版が読めなくなることを `docs/release.md` に書く
- [サーバの更新係が止まっても OS は知らせない] → 48 時間で理由が「更新係が動いていない」、7 日で帯（D15（仮））
- [配布元が 0 件の間（今）は、どの部品も「更新なし」] → 空の一覧は「確かめた」に数えるので、確かめられていない知らせは出ない
- [配る zip を入れる録画（ST55 の `tools/recording/scenarios.d/` の台本）では、更新係が本物の配布元に 1 回問い合わせうる] → 問い合わせの中身は版の一覧の取得だけ（FR-116）。録画の台本が外への接続を塞ぐかは、録画を持つ Story が決める（この Story は台本を変えない）
- [`HttpURLConnection` が見出しを足す] → 計測テストで見る（D13（仮））
- [更新係の置き場を本人も読める] → 中身は版と時刻だけで、記録を持たない
- [入れ替えた収集アプリは、示せない間は確認が黙って止まる（携帯で通知を許さず画面も開かない人・PC でトーストを出せない人）] → 本人の決定（Q11）。止まっている間は「確かめられていない」も出さない（D26（仮））ので、本人には何も見えない。画面の帯（サーバ）は更新係が確かめるので出る
- [入れ替えた携帯で、入れ替えの行が画面に出たが本人が読まずに閉じた] → 見ないまま問い合わせが出うる（Q12 で本人が受け入れた残り。D24）
- [入れ替えた PC で、入力の直後に席を外してトーストを見なかった] → 見ないまま問い合わせが出うる（Q13 で本人が受け入れた残り。D25）
- [PC で Windows の設定から通知を切っていてもトーストは成功を返しうる] → その人は切り方を見ないまま問い合わせが出うる（Q11 で本人が受け入れた残り）。`ToastNotifier::Setting()` で切ってあるかを読めれば減らせる見込みだが、本物の Windows で値を確かめていないので仕様にしない（`docs/handoff/ST57.md` の K2。確かめてから採るなら上流へ返す）

## Migration Plan

- DB の移行は無い
- 本人の PC（ST55 で入れた形）: 新しい版の zip でインストーラを走らせ直すと、更新係が足される（D7）。最初の問い合わせは走らせ直しの最後の表示の後
- 携帯と PC の収集アプリ: 新しい版に入れ替えた後、既に送り先を設定していれば、切り方を示した（D23）後の機会に確かめる（確認は既定で有効）。示せない間（携帯は通知を出せず画面も開かない・PC はトーストを出せない）は確かめない。切るのは携帯の設定の画面・PC の `update-check off`
- 戻す: 更新係を知らない前の版へ戻したときに、更新係のサービスの登録と実行ファイルがどう残るかは確かめていない（前の版のインストーラは更新係を消さない）。戻す手順は ST58 が持つので、`docs/handoff/ST58.md` に申し送る。手で消すなら `ashiato-setup.exe update-check off` の後に `sc.exe delete ashiato-updater`（`docs/install.md` に書く）
