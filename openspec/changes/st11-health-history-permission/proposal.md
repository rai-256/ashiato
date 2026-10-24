## Why

FR-11 は「C-01 の初回起動で健康データの履歴読み取り権限を要求する」と言う。いまの収集アプリは manifest に履歴の権限（`READ_HEALTH_DATA_HISTORY`）を**宣言しているだけ**で、
求めるコードが無い。しかも、そのままでは求めても何も出ない —— 履歴の権限だけを求めると Health Connect はダイアログを出さずに拒否で返し（`premises.md` P4）、
説明の画面（プライバシーポリシー）を持たないアプリにも拒否で返す（P5）。

要件が挙げた理由（「宣言していない期間の 30 日より前は永久に読めない」= EXT-C）は、Android 15 の実物では成り立たなかった。
**履歴の権限は後から与えても、最初の許可より前の全部が読める**（P2・P3）。それでも初回起動で求める意味は残る:
履歴の権限が無い端末（本人が断った・版が対応していない）では、読める範囲の起点が「どれかの種類を最初に許した時の 30 日前」で**アプリごと**に決まる（P13）。
初回起動で種類を許しておけば、ST34（収集の本体・Should）が読み始めるときより起点が早い。そして Health Connect は「いつ許したか」を後から読む口を持たない（P11）ので、
観た時に残さなければ、その起点は後から分からない。

この Story は、**初回起動で健康データの権限が実際に求められ、断られても収集が止まらず、許可の状態がその時の時刻とともに S-01 に残る**ところまでを作る。
健康データそのものは読まない（ST34）。

## What Changes

**深掘りの第 1 回（Q1〜Q10）は答え待ち。推奨の答えで先に書いた**（`deep.md` の `（答え待ち・推奨: …）`）。本人の答えが推奨と違えば `spec --answers` が該当の Q だけを直す。

- **初回起動で健康データの権限を求める**（Q1 推奨）。写真と動画の後・収集を始める前（Q7 推奨）に、Health Connect の権限の画面を 1 回で出す。
  一緒に求めるのは種類の読み取り 4 つ（睡眠・心拍・歩数・運動。Q3 推奨）と、履歴の権限・背景での読み取りの権限（Q9 推奨）
- **自動で求めるのは 1 度だけ**（Q4 推奨）。断られても収集（位置・アプリ利用・写真）は始め、以後は常駐の通知から、このアプリの Health Connect の権限の画面へたどれる
  （正典の「利用状況へのアクセスを初回起動で求める」の「ここで決めた形を ST11 が引き継ぐ」どおり）。ST11 入りの版へ**更新した**既存の端末は、更新の後の最初の起動で 1 度求める
- **Health Connect の説明の画面を持つ**（Q6 推奨）。短い固定の文（何を読み、どこへ送り、そこから先は承認の設定で決まること）を出す。これが無いと Health Connect は画面を出さない（P5）
- **Health Connect が無い・更新が要る端末では求めず、使えないことを記録して収集を始める**（Q8 推奨）。起動のたびに見直し、使えるようになったら初回と同じく 1 度だけ求める。
  Android 13 以前で Health Connect のアプリが入っていれば、そのアプリを通して求める。履歴の権限・背景での読み取りに対応していない端末は、対応しているものだけを求める（design D6（仮））
- **許可の状態を新しい論理ソース `c01-health-permission` の記録として S-01 へ送る**（Q5 推奨）。状態（Health Connect が使えるか・種類ごと・履歴・背景・自動で求めたか）が
  変わったときだけ 1 件を未送信に積み、端末の時計の時刻・起動からの経過時間・起動の識別を持たせる。状態が変わったかは中身だけで判定し、時刻を使わない。
  登録簿には外部の識別子なし・途切れの判定の対象外（`c01-clock` と同じ）で 1 行足す。稼働状況の格子には出さない。書き出しには入る
- **位置の権限を拒否したときの振る舞いを正典の Scenario にする**（Q10 推奨。ST06 からの申し送り `docs/handoff/ST11.md`）。
  いまは試験（`PermissionDeniedInstrumentedTest`）だけが守っている 5 点を、THEN / AND で 1 本に載せる。健康データの権限を断ったときも同じ形の Scenario を足す
- **要件の理由の訂正は record の段が入れる**（Q1 推奨: 理由だけを ★ で訂正する。FR-11・扉 21・EXT-C と、その写しの `docs/stories/ST11.md`（`stories.json` から再生成））。
  ST11 の spec は訂正後の理由で書き、「永久に読めない」を写さない。コードと文書の写し（`AndroidManifest.xml` の注記・`collector-android/README.md`）は tasks が直す

**BREAKING**: なし。`/ingest`・`/heartbeat` の形は変えない（新しい論理ソースは登録簿への 1 行で受け付けられる）。

## 深掘り（第 1 回。答え待ち）

| # | 問い | 推奨（先に書いた形） | 段 |
|---|---|---|---|
| Q1 | 要件の理由が崩れた —— 初回起動で求めるか | 初回起動で求める（要件の理由だけを訂正する） | A（premise・uncaptured） |
| Q2 | 本人の携帯の実物 | 補足に書いた | A（premise） |
| Q3 | 履歴の権限と一緒に求める種類 | ST34 で読む見込みの種類をまとめて求める（睡眠・心拍・歩数・運動） | B |
| Q4 | 断られたとき・後で外されたとき | 初回起動の 1 度だけ自動で求め、以後は常駐の通知から Health Connect の設定へたどれる | B |
| Q5 | 許可を観たことを、どこに記録するか | S-01 へ記録として送る（新しい論理ソース。端末にも残す） | A（irreversible・uncaptured） |
| Q6 | 説明の画面に何を書くか | 短い固定の文（何を読み、どこへ送り、その先は承認の設定で決まること） | B |
| Q7 | 初回起動の中での順 | 写真と動画の後・収集を始める前 | B |
| Q8 | Health Connect が無い・履歴に対応していない端末 | 無い・更新が要る端末は求めずに記録し、履歴に未対応の端末は種類と対応しているものだけを求める | B |
| Q9 | 背景での読み取りの権限も一緒に求めるか | 一緒に求める | B |
| Q10 | 位置の権限を拒否したときの Scenario | 叩き台の 1 本に、試験が見ている点を THEN / AND で全部載せる | B |

Q2 の答え（本人の端末の版・書いているアプリ）で、Q3 の種類と Q8 の扱い（design D5（仮）・D6（仮））を `spec --answers` が直しうる。

## Capabilities

### New Capabilities

無し。

### Modified Capabilities

- **`device-collection`** —— 健康データの権限を初回起動で求める・説明の画面・Health Connect が無い端末・許可の状態の記録・その論理ソースが識別子なしで受け付けられること の Requirement を足す（ADDED）。
  「ソースごとに独立して収集する」を `MODIFIED` し、位置の権限を拒否したときの Scenario（Q10）と、健康データの権限を断っても他のソースが集まる Scenario を足す

capability の割当は `docs/stories/INDEX.md` のまま（ST11 は `device-collection`）。前倒しは無い。
許可の状態の記録を S-01 が受け付けることは `c01-clock`（ST05）の前例に倣って `device-collection` に置いた（受け手の振る舞いだが、ソースの登録と対で決まる）。
同じ Requirement の「途切れの判定の対象外・稼働状況の格子に出さない・書き出しに含める」も `device-collection` に置いた（spec-review R2）。どれも**このソースの登録簿の 1 行の値**
（`outage_target`・`must_sources()` に入れない・`excluded.rs` の一覧）から決まり、途切れ・格子・書き出しの規則そのものは変えない。`collection-coverage` / `data-durability` に置くと、
`data-durability` を `MODIFIED` している ST29・ST41・CH04 と同じ capability を触ることになる（規則を変えないのに盤面が `衝突待ち` になる）。

## Impact

### 触るもの

- `collector-android/app/src/main/AndroidManifest.xml`（種類 4 つ・背景の権限の宣言、説明の画面の Activity と activity-alias、Health Connect のアプリを引く `<queries>`、健康データの印を自動バックアップから外す規則（`res/xml/` に 2 つ。design D14（仮））、誤った理由の注記の訂正）
- `collector-android/app/build.gradle.kts`（Health Connect の Jetpack の client の依存。design D3（仮））
- `collector-android/app/src/main/kotlin/dev/ashiato/collector/`（`MainActivity.kt` の権限の流れ・新しい `HealthPermissions.kt`（読み口と求める口）・`HealthPermissionRecord.kt`（記録）・
  新しい説明の画面 `HealthRationaleActivity.kt`・`LocationService.kt` の通知の操作と 6 時間ごとの見直し）と試験（`app/src/test`・`app/src/androidTest`）
- `collector-android/README.md`（誤った理由の訂正）
- `migrations/`（登録簿に `c01-health-permission` を 1 行足す移行 1 本）
- `crates/server/src/export/excluded.rs`（書き出しの分類の一覧に新しいソースを足す。足さないと既存の試験が名前つきで落ちる）と `crates/server/src/` の試験
- `docs/collector-contract.md`（`c01-health-permission` の `payload` の節）
- `tools/smoke.d/`（新しいファイル 1 本。縦串）
- `tools/android-emulator.sh` と `.github/workflows/ci.yml` の android-instrumented（健康データの計測テストの段を足す。design の Risks）
- `docs/handoff/ST34.md`・`docs/handoff/ST38.md`

### 走っている Story との重なり

- 走っている change（ST23・ST29・ST41・CH03・CH04）はどれも `device-collection` を触らない。`MainActivity.kt`・`LocationService.kt` を触る change も無い
- `crates/server/src/export/excluded.rs` は ST23・ST41・CH04 が触る（ST41 は `SPAN_RULES` / `NO_COORD_SOURCES` に触らない）。ST11 は `data-durability` の Requirement を変えず、一覧に名前を 1 つずつ足すだけ（衝突したら後から merge する側が rebase で解く。spec-review R21）
- **ST34（健康データを取り込む。未着手）が引き継ぐもの**: 求めた種類（Q3）・背景での読み取り（Q9）・`c01-health-permission` の記録。`docs/handoff/ST34.md` に書く（tasks）
- **ST38（種類と承認。未着手）が引き継ぐもの**: `c01-health-permission` を種類「稼働状況」に属させること（FR-102。登録簿に種類の列がまだ無い）。`docs/handoff/ST38.md` に書く（tasks）

### 触らないもの

- 健康データの読み取り（FR-73。ST34）。健康データの生存信号も足さない（収集していないソースの信号を出さない。ST13 design D20 と同じ）
- `/ingest`・`/heartbeat`・`/coverage` の形。稼働状況の格子（`c01-health-permission` は Must の 5 本にも格子にも出さない）
- 既存の権限の流れの順（位置 → 背景の位置 → 通知 → 写真と動画 → 収集を始める → 利用状況へのアクセス → 一時停止の案内）。健康データは写真と動画の後に 1 段入るだけ
- `docs/requirements.md`・`docs/stories/`（理由の訂正は record の段）
