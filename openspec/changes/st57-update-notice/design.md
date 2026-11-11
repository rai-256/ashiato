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
- 新しい検査 `tools/check-updater-deps.sh`: `cargo tree -p ashiato-updater -e normal` に DB の部品（`sqlx`・`tokio-postgres`・`postgres`・`diesel`・`sea-orm`）が無いこと、`cargo tree -p ashiato-server -e normal` に `ashiato-updater` が無いこと、`cargo tree -p ashiato-update-check -e normal` に `check-offsite.sh` の一覧の部品が無いこと。`--self-test` つき。CI の `lint` の job に足す

### D2. 問い合わせは決まった形だけ。見出しの集合を名指しする

> 聞かない既定（`defaults.md` C2）。review/deep.md R8

- 道は開発版でない部品が `/repos/rai-256/ashiato/releases?per_page=20`、開発版の部品が `/repos/rai-256/ashiato/releases/tags/nightly`（一覧の中で沈む P7 に依らない）
- `GET`・本文なし・`User-Agent: ashiato-update-check`・`Accept: application/vnd.github+json`・`Accept-Encoding: gzip`。転送は同じ `api.github.com` の中だけ追う（最大 3 回）。時間切れは接続 10 秒・全体 30 秒
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
- 最初の問い合わせ: 更新係はインストーラが最後の表示の後にサービスを開始する（それより前はサービスが止まっている）。携帯と PC は送り先の設定を保存したことが前提（`SetupStore` / `settings.json` に送り先がある。組み立ての時に `-P` で渡した送り先も設定済みに数える）

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
- `repair` の命令の一覧は変えない（試験で、`repair` の命令に `ashiato-updater` が出ないことを見る）
- 正典の 3 つの Requirement（インストール・走らせ直し・消す口）と配布物の中身を MODIFIED にした

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

- crate `ashiato-update-check`・`ashiato-updater`、実行ファイル `ashiato-updater.exe`、サービス `ashiato-updater`（`-dev`）、置き場 `C:\ProgramData\ashiato-updater`（`-dev`）、サーバの口 `GET /update-status`、携帯のチャンネル `update`・SharedPreferences `update_check.xml`、PC の札 `update`・`update-stale`、共有の材料 `docs/contracts/update-check-vectors.json`
- 反転条件: ST58 の上流が、入れる役を同じ実行ファイルに持たせて名前が合わなくなったとき（名前は配った版に焼き込まれるので、配る前に決め直す）

### D12（仮）. TLS は `rustls` と OS の根の確かめ（`rustls-platform-verifier`）

- `ureq` の機能 `rustls` と `platform-verifier`。Windows は OS の証明書の置き場、Linux（試験）は OS の根を読む。OpenSSL を要さないので Linux の CI でそのまま組める
- 更新係と PC の収集アプリの両方に同じ機能。`tools/check-licenses.sh`・`tools/check-vulns.sh` に通す
- 反転条件: 許諾の検査か脆弱性の検査が落ちて互換の範囲で直せないとき、または `NT SERVICE\ashiato-updater` で OS の根を読めない（Windows の CI で確かめられない）ときは `native-tls`（Windows は schannel）に替える

### D13（仮）. 見出しの集合は 5 つの名前の内側

- 許すのは `Host`・`User-Agent`・`Accept`・`Accept-Encoding`・`Connection`。`Connection` は Android の通信の部品が `Keep-Alive` を自分で足す（P10 と同じく知識。計測テストで 4 に上げる）ので許す。どの部品も値は定数で、自分の情報を含まない
- 反転条件: 計測テストで Android が 5 つの外の見出しを足すと分かったとき（その名前と値が自分の情報を含まなければ集合に足し、含むなら送り手を替える）

### D14（仮）. サーバの読みの口 `GET /update-status` の形

- 応答: `{"state": "off" | "not_running" | "pending" | "latest" | "available" | "stale" | "available_stale", "installed": "<サーバの版>", "latest": <版 | null>, "found_at": <時刻 | null>, "checked_at": <時刻 | null>, "last_ok_at": <時刻 | null>, "reason": <D4 の理由 | "not_running" | null>}`
- 画面のログインが要る（API の合言葉の許可の一覧に入れない。ST44 の既定で 403）。読み出しの記録には残さない（記録を読む口でない）。`Cache-Control: no-store`
- 結果のファイルは信用しない入力として読む: 64 KB を超えれば読まない、版は SemVer の形だけ、時刻は RFC 3339 だけ。外れたら `stale`（理由 `unreadable`）
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

- `install-windows` の job は、配る zip でも試験の zip でも、インストールの前に `netsh advfirewall firewall add rule … dir=out program=<プログラムの置き場>\ashiato-updater.exe action=block` を置く（本物の配布元に聞かない。共有の runner の IP で上限 60 回に当たらない）
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

## Risks / Trade-offs

- [携帯と PC の問い合わせの元の IP と時刻が毎日配布元に残る] → Q1 で本人が選ぶ（A）。中身は版の一覧の取得だけで、記録は出ない（FR-116 の試験）
- [配った版の中の宛先・読み方・証明書の確かめ方は、その版からは直せない] → 壊れたら 7 日で「確かめられていない」が出る（Q9）。リポジトリの名前を変えるときは、配った版が読めなくなることを `docs/release.md` に書く
- [サーバの更新係が止まっても OS は知らせない] → 48 時間で理由が「更新係が動いていない」、7 日で帯（D15（仮））
- [配布元が 0 件の間（今）は、どの部品も「更新なし」] → 空の一覧は「確かめた」に数えるので、確かめられていない知らせは出ない
- [`HttpURLConnection` が見出しを足す] → 計測テストで見る（D13（仮））
- [更新係の置き場を本人も読める] → 中身は版と時刻だけで、記録を持たない

## Migration Plan

- DB の移行は無い
- 本人の PC（ST55 で入れた形）: 新しい版の zip でインストーラを走らせ直すと、更新係が足される（D7）。最初の問い合わせは走らせ直しの最後の表示の後
- 携帯と PC の収集アプリ: 新しい版に入れ替えた後、既に送り先を設定していれば次の機会に確かめる（確認は既定で有効）。切るのは携帯の設定の画面・PC の `update-check off`
- 戻す: 更新係を知らない前の版へ戻したときに、更新係のサービスの登録と実行ファイルがどう残るかは確かめていない（前の版のインストーラは更新係を消さない）。戻す手順は ST58 が持つので、`docs/handoff/ST58.md` に申し送る。手で消すなら `ashiato-setup.exe update-check off` の後に `sc.exe delete ashiato-updater`（`docs/install.md` に書く）
