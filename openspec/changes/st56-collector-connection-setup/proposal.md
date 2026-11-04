# ST56 — 収集アプリを入れた後に、送り先と資格情報を設定できる

## Why

いまの携帯の収集アプリは、送り先・資格情報・利用者を組み立ての時に APK へ焼き込み（`build.gradle.kts` の `buildConfigField`）、PC の収集アプリは環境変数を自動起動の `ashiato-collector.cmd` に平文で書く。
配る APK は送り先も資格情報も持たない（ST52）ので、配布元から落とした同じ APK と PC の収集アプリを入れても送信が始まらない（FR-128。`docs/audit/2026-10-25-consistency.md` の C3）。
資格情報は携帯と PC で共有する 1 本のまま（ST44 の深掘り Q2）なので、入れた後にその値と送り先を本人が入れる口が要る。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q5 は「仮でよい」問い、Q7・Q8 は失うものがある問い（本人は答えを省けない）。
> 本人の答えが推奨と違えば `spec --answers` が書き直す。聞かない既定（`review/deep.md` の C1〜C7）は design の D1〜D7 に写した。

- **画面に「収集アプリの設定」を置く**（Q1 の推奨）: 画面の設定に節を足し、画面の合言葉を入れ直すと、送り先と資格情報をまとめた 1 行の**設定の文字列**を出す（コピーのボタン付き）。
  送り先は画面を開いている口から作る（C7）。利用者は入れない（C1）。資格情報を出す口は、画面にログインしたうえで合言葉を入れ直した要求だけに答え、収集アプリの資格情報では読めない
- **携帯の収集アプリに設定の画面を足す**（Q3・Q4 の推奨）: 入れて初めて開くと、最初に設定の画面を出す。設定の文字列を貼ると、その場でサーバに問い合わせて結果を見せ、繋がらなくても保存する。
  「後で」で飛ばせ、権限の要求と収集はどちらでも始まる。未設定の間は通知で促す。設定した後は、サービスを起こし直さずに送り始める
- **PC の収集アプリに設定の口を足す**（Q2・Q8 の推奨）: 設定が無いまま起動しても取って積み、黒い窓で設定の文字列を貼らせる（`ashiato-collector-windows.exe setup` でいつでも設定し直せる）。
  資格情報は Windows の暗号（DPAPI。その利用者だけが戻せる）で包んで置き場に置き、自動起動の台本には実行ファイルの場所だけを書く（既にある台本も書き換える）。
  置き場は `%LOCALAPPDATA%\ashiato\collector`、端末の識別子は最初の起動で作って置き場に覚える。環境変数は、開発と試験の印（明示の引数）を付けて起動したときだけ読む
- **設定の前に積んだ記録を捨てない**（C1）: 携帯も PC も、記録の利用者を設定の前後とも既定の利用者（nil の UUID）に固定する。いまの配る APK は空の利用者を行に凍らせ、設定の後に送ると `malformed` で全部捨てる（`review/deep.md` R1）
- **断られ続けたら知らせる**（Q5 の推奨）: 資格情報で断られる（401）状態が 1 時間続いたら知らせ、続く間は 24 時間ごとに出し直す。一度も届いたことの無い送り先に 24 時間届かないときも知らせる。押すと設定の画面（PC は `setup`）が開く。断られた記録は今どおり捨てない
- **送り先を変えたら、押すまで送らない**（Q7 の推奨）: 未送信があるまま送り先を設定・変更したとき（前の送り先が分からないときも）、件数と期間を見せ、「この送り先へ送る」を押すまで記録・生存信号・破棄の報告・写真の原本・途絶と気分の問い合わせを 1 つも送らない
- **携帯の設定をバックアップと端末の移し替えから外す**（C3）。組み立ての時の `-P` の値は、保存した設定が無いときの初期値として残す（試験と計測テストを壊さない）
- **通知が開く画面のアドレスを送り先から作る**（C4）。携帯に貼った送り先が自分自身（loopback）なら貼った時点で断る（C5）。利用者に見せる語は「送り先」に揃える（C6）
- **繋ぎ方の手順書** `docs/collector-setup.md` を足す（携帯・PC の設定、サーバの `repair` の後の繋ぎ直し、資格情報を変えた後の設定し直し）

**BREAKING ではない**: 送信の封筒・送信契約の版・受け口は変えない。開発と試験の組み立て（`-P` の焼き込み・環境変数）はそのまま動く（PC は起動の引数を 1 つ足す）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `device-collection`: 携帯の設定の画面・設定の文字列の読み取りと保存・設定した時点の確かめ・未設定の知らせ・断られ続けた知らせ・送り先を変えたときの保留・バックアップからの除外・利用者の固定・通知の開く先（ADDED）
- `desktop-collection`: PC の設定の口（`setup`）・資格情報の置き方と自動起動の台本・設定の無い起動・既定の置き場と端末の識別子・断られ続けた知らせ・送り先を変えたときの保留（ADDED）
- `data-sensitivity`: 画面が設定の文字列を出す口（ログインと合言葉の入れ直しが要り、収集アプリの資格情報では読めない）（ADDED）

**capability の照合**: INDEX.md は ST56 を `device-collection`・`desktop-collection`・`data-sensitivity`・`distribution` に割り当てている。
収集アプリの振る舞いは前の 2 つ、資格情報を見せる口の守りは `data-sensitivity` で合っている。`distribution` には振る舞いを足さない —— 配る APK が資格情報を持たない検査（ST52 の Requirement）は変えず、繋ぎ方は手順書だけ。前倒しは無い。

**requires**: ST52（配る APK は送り先と資格情報を持たない）・ST44（共有の 1 本の資格情報は送ることだけができる）は archive 済みで正典にある。
**ST54 を requires に足した**（spec_overlap の観点 7）—— 設定した時点の確かめが見せる「送り先が開発版（正式版）のサーバです」は ST54 の `GET /contract` の系統の見出しを読み、
PC の既定の置き場と端末の識別子には ST54 の「開発版は置き場の下の `dev`・識別子の末尾に `-dev`」が当たる。ST54 は下流を走っている。archive は待たない。

## Impact

- 携帯（`collector-android`）: `Config.kt`（保存した設定を先に読み、`-P` は初期値）・新しい設定の画面（Activity）と設定の文字列の読み取り・`LocationService.kt`（設定した後に送る仕組みを立て直す・送り先の保留を全部の経路に当てる・利用者を固定する）・`MainActivity.kt`（最初の起動の順）・
  `FixCollector.kt` `Heartbeat.kt` `HealthPermissionRecord` などの利用者・`FeelingNotification.kt`（開く先）・`OriginalUploader.kt`・`OutagePoller.kt`・`FeelingReminderReceiver.kt`（保留）・`AndroidManifest.xml` と `res/xml` のバックアップの除外・新しい通知（未設定・断られ続け・届かない・送り先の保留）
- PC の収集アプリ（`crates/collector-windows`）: `config.rs`（置き場の既定・端末の識別子・設定のファイル）・`main.rs`（`setup` の口・設定の無い起動・`--config-from-env`）・`autostart.rs`（台本に実行ファイルの場所だけ）・新しい `setup.rs` と `secret.rs`（DPAPI）・`runtime.rs` `sender.rs`（保留・401 の数え）・`toast.rs`（新しい札）。`windows` crate に機能 `Win32_Security_Cryptography` を足す（`tools/check-licenses.sh` に通す。crate は既にある）
- サーバ（`crates/server`）: 設定の文字列のための資格情報を返す口（`POST /collector-setup`）。`web_session.rs` の合言葉の照合を使う。`docs/openapi.json` を作り直す
- 画面（`web/src`）: `SettingsView.tsx` に「収集アプリの設定」の節と新しい部品。画面を持つ部分ができる（ブリーフの「面」は空だが、Q1 と Q3 の推奨で画面の設定の節と携帯の設定の画面が増える）。e2e は `web/e2e/st56-collector-setup.spec.ts`
- 試験と道具: PC の収集アプリを環境変数で起こす台本（`tools/record-env.sh` などの起動）に `--config-from-env` を足す。新しい `tools/check-android-backup-rules.sh`（バックアップから設定のファイルが外れていること）
- 文書: 新しい `docs/collector-setup.md`、`docs/release.md` §7（本人の切り替えで収集アプリを繋ぐ手順への参照）
- 申し送り: `docs/handoff/ST55.md`（ST55 の「秘密は人が見ない」を「画面にログインして合言葉を入れ直した本人には、収集アプリの資格情報を見せる」に揃える・`docs/install.md` の繋ぎ直しの節から `docs/collector-setup.md` を指す）。`docs/handoff/ST59.md`（合言葉の入れ直しの守りと、資格情報を変えた後に収集アプリが出す知らせは ST56 が作る）
- **走っている change との重なり**:
  - `st54-dev-side-by-side`（下流）: 同じ `config.rs`（置き場と端末の識別子）・`autostart.rs`（台本の名前）・`ContractGate`（系統の見出し）を触る。ST54 は requires に入れ、ST54 の実装の上に積む。**ST54 へは差し戻さない** —— ST54 の「渡された置き場」は、ST56 の後は「渡された置き場か既定の置き場」になる（ST56 の Requirement が書く）
  - `st55-server-installer`（下流）: `web_session.rs`（合言葉の持ち方。ST55 D8）を触る。ST56 は合言葉の照合を同じ関数で呼ぶだけで、持ち方には触らない。requires には入れない（インストールした形でも開発の形でも、サーバは自分の資格情報を持っている）
  - `st53-license-bundle`（下流）: 携帯の `assets/licenses/` と許諾の検査。PC の `windows` crate の機能を足すだけで、新しい依存は無い
