# ST11 の前提 —— 外の形式・振る舞い（Health Connect）

FR-11 と扉 21 は「宣言していない期間の 30 日より前は永久に読めない」（§5 の EXT-C）を理由にしている。
この Story の決定はすべて Health Connect の振る舞いに依るので、問いを立てる前に確かめた（2026-10-24）。

**確かめ方（表の「実物」の行に共通）**: Android 15（API 35、`system-images;android-35;google_apis;x86_64`。Health Connect は OS の
module）のエミュレータに、使い捨ての検証用アプリを 3 つ入れた（repo の外に作って捨てた。製品のコードは触っていない）。
2 回に分けて走らせた（2 回目は deep-review の R3・R5・R12・R13 を受けて、新しいエミュレータで。書き手は距離の記録も書き、読み手は距離も読む）:

- 書き手（`dev.probe.writer`。`WRITE_STEPS` と説明の画面を持つ）: 歩数の記録を 120・60・35・20・1 日前の時刻で 5 件書く
- 読み手（`dev.probe.reader`。`READ_STEPS`・`READ_HEALTH_DATA_HISTORY`・`READ_HEALTH_DATA_IN_BACKGROUND` と説明の画面を持つ）:
  400 日前から今までの歩数を読み、何日前の記録が返ったかをログに出す。`requestPermissions` でダイアログも出す
- 説明の画面を持たない読み手（`dev.probe.bare`）: 同じ権限を求めるだけ

許可は `pm grant` / `pm revoke`（Health Connect の権限は実行時の権限なので shell から付け外しできる）、ダイアログは
`uiautomator dump` で画面の文字を読み、押して確かめた。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | 種類の読み取り権限だけでは、最初の許可の 30 日より前は読めない | FR-11 の理由（EXT-C の前半） | 読み手に `READ_STEPS` だけを与えて読む | 20・1 日前の 2 件だけが返った（35・60・120 日前は返らない） | 実物（エミュレータ Android 15） | 4 |
| P2 | **履歴の権限を後から与えると、最初の許可より前の全部が読める**（「永久に読めない」は成り立たない） | Q1（要件の理由の訂正・初回起動で求めるか） | P1 の後に `READ_HEALTH_DATA_HISTORY` を与えて読み直す | 5 件全部（120 日前を含む）が返った。外すとまた 2 件に戻った。公式の定義も "Allows an application to read the entire history of health data (of any type)" | 実物 + 公式文書 https://developer.android.com/reference/android/health/connect/HealthPermissions | 4 |
| P3 | 入れ直すと権限は全部外れるが、入れ直した後に履歴の権限を与えれば、やはり全部読める | Q1・完了の判定 1（入れ直すと初回起動でダイアログが出る） | 読み手を消して入れ直し、`READ_STEPS` だけ → 履歴も、の順に読む | 2 件 → 5 件。公式は「消すと履歴の権限も含めて全部外れ、入れ直した日の 30 日前からが既定」 | 実物 + 公式文書 https://developer.android.com/health-and-fitness/health-connect/read-data | 4 |
| P4 | **履歴の権限だけを求めても、ダイアログは出ずに即座に拒否で返る**。種類の権限を 1 つ以上と一緒に求めると出る | Q3（一緒に求める種類）。いまの manifest は履歴の権限だけを宣言している（ST01） | 読み手が `requestPermissions([READ_HEALTH_DATA_HISTORY])` と `([READ_STEPS, READ_HEALTH_DATA_HISTORY])` を呼ぶ | 前者は画面が変わらず `-1`（拒否）で返った。後者は「データの種類を選ぶ」画面 →「過去のデータへのアクセスを許可しますか（2026-09-24 より前に追加されたデータ）」の画面の 2 段 | 実物 | 4 |
| P5 | **説明の画面（プライバシーポリシーを出す Activity）を宣言していないアプリには、ダイアログが出ずに即座に拒否で返る** | Q6（説明の画面を持つ）。いまの manifest には無い | 説明の画面の無い `dev.probe.bare` が同じ 2 つを求める | 画面が変わらず、2 つとも `-1` で返った。公式は `androidx.health.ACTION_SHOW_PERMISSIONS_RATIONALE`（13 以前）と `VIEW_PERMISSION_USAGE` + `HEALTH_PERMISSIONS` の activity-alias（14 以降）を宣言させる | 実物 + 公式文書 https://developer.android.com/health-and-fitness/health-connect/get-started | 4 |
| P6 | 2 回断ると、3 回目以降はダイアログが出ずに即座に拒否で返る。種類の画面で断ると、過去のデータの画面は出ずに履歴も拒否になる | Q4（断られたとき） | (1) 種類を許し、過去のデータの画面で「許可しない」を 2 回押してから、もう一度求める (2) 新しい端末で、種類の画面で「許可しない」を 2 回押してから、もう一度求める | (1) 1・2 回目は過去のデータの画面が出た。3 回目以降は画面が変わらず `READ_STEPS=0 / HISTORY=-1`（種類は許したまま）。(2) 1・2 回目は種類の画面が出て、断ると過去のデータの画面を経ずに両方 `-1`。3 回目は画面が出なかった | 実物 | 4 |
| P7 | 端末で初めて Health Connect を使うときは、権限の画面の前に Health Connect の「はじめに」の画面が 1 度出る。**抜けるとその回のまま権限の画面に進む**（要求は潰れない。断った回数にも入らない） | Q4・Q7 | 新しいエミュレータで、読み手が初めて歩数と履歴を一緒に求める | 「Get started with Health Connect」→「Get started」を押すと、同じ回のまま種類の画面が出た。そこから 2 回断るまで画面は出続けた（P6 (2)）。**1 回目の実験で「はじめにを抜けると拒否で返る」と書いたのは誤り** —— その回は履歴の権限だけを求めていて、P4 で拒否になっていた（deep-review R3 を受けて 2 回目で確かめ直した） | 実物 | 4 |
| P8 | 履歴の権限と背景での読み取りの権限は API 35 で入り、Android 14 では U Extensions 13 以降、13 以前は Health Connect のアプリの版に依る。使えるかは機能の状態（`FEATURE_READ_HEALTH_DATA_HISTORY`）で見る | Q5・Q8（Health Connect が無い・履歴に対応していない端末） | 公式の reference の "Added in API level 35 / Also in U Extensions 13" を読む | Android 14 以下では端末の版で使えないことがある。Android 13 以前は Health Connect がアプリで、入っていない端末がある（minSdk は 30） | 公式文書（同上 2 つ） | 2 |
| P9 | Health Connect の自動削除は既定でオフ。選べるのは「3 か月後」「18 か月後」「しない」 | Q1・Q2（Health Connect から消える経路） | Health Connect の設定（`HEALTH_HOME_SETTINGS` →「Manage data」→「Auto-delete」）を `uiautomator dump` で辿る | 「Auto-delete: Off」。選択肢は After 3 months / After 18 months / Never。書いたアプリが自分の記録を消す・端末の初期化や機種の変更は、この設定と関係なく起きうる（確かめていない） | 実物 | 4 |
| P11 | アプリが「いつ許されたか」を Health Connect から後で読む公開の口は無い（今の状態を読む口だけ） | Q5（観た時に残さないと後から作れない） | 公式の reference（`HealthConnectManager`）の公開メソッドに、許可の時刻・履歴の起点を返すものがあるかを探す | 見つからない（`grant` / `HistoricalAccess` の語が公開の項目に無い）。今の状態は `checkSelfPermission` で読める | 公式文書 https://developer.android.com/reference/android/health/connect/HealthConnectManager | 2 |
| P12 | 2 回断って画面が出なくなった後でも、本人は Health Connect の設定の画面から種類も履歴も許せ、許せば全履歴が読める | Q4（断られた後の道） | P6 (2) の後に `MANAGE_HEALTH_PERMISSIONS`（このアプリ）を開き、「Steps」と「Additional access → Access past data」を押す | 種類と履歴が `granted=true` になり、歩数を 5 件全部（120 日前を含む）読めた。この画面はアプリから intent で直接開ける | 実物 | 4 |
| P13 | 履歴の権限が無いとき、読める範囲の起点は**アプリごと**（どれかの種類を最初に許した時）で、種類ごとではない | Q1（履歴が無い端末で、先に種類を許しておく意味）・Q3・Q5（記録する「最初に許した時」） | 歩数だけを許し（履歴は外す）、端末の時計を 40 日進めてから距離を許して、両方を読む | 距離も歩数と同じく「最初の許可（歩数）の 30 日前」から読めた（20・1 日前の 2 件）。種類ごとなら距離は 0 件のはず。EXT-C の逐語 "any permission was first granted" と一致 | 実物 | 4 |
| P14 | 種類の画面の次は「追加のアクセス」の 1 枚で、**履歴（Access past data）と背景での読み取り（Access data in the background）のトグルが並び、どちらも既定でオフ**。本人がオンにして「許可」を押す | Q9（背景も一緒に求めると画面が増えるか）・GO 資料（本人が何を押すか） | 歩数・距離・履歴・背景を一緒に求め、画面の文字を読み、トグルを押して許す | 種類の画面（Allow all / Distance / Steps）→ 追加のアクセスの画面（2 つのトグル）。トグルを両方押して「許可」で 4 つとも `0`（許可） | 実物 | 4 |
| P15 | Android 13 以前は Health Connect がアプリで、その権限は OS の実行時の権限ではない。求めるには Jetpack の `PermissionController.createRequestPermissionResultContract()`（Health Connect のアプリが画面を出す）を使い、許可は `getGrantedPermissions()` で読む。いまの `MainActivity` は `requestPermissions` / `checkSelfPermission` だけで、依存に Health Connect の client も無い | Q8（13 以前の端末）・spec（求め方の口） | 公式の始め方の文書と、いまの実装（`MainActivity.kt`・`app/build.gradle.kts`）を読む | 公式は client 1.1 以降に `compileSdk 36` と AGP 8.9.1 以上を要する（このアプリは 36 と 8.13.0）。13 以前で走らせてはいない（手元に 13 以前の image が無い） | 公式文書 https://developer.android.com/health-and-fitness/health-connect/get-started + repo | 2 |
| P10 | 本人の携帯の Android の版・Health Connect にデータを書いているアプリ・自動削除の設定 | Q1（失うものが本当にあるか）・Q3（求める種類）・Q8 | 本人の手元にしか無い | **Q2 で本人に聞く** | —— | —— |

## 失われるもの（`loss`）のある決定と、この表の関係

- **Q1**（初回起動で求めるか・要件の理由の訂正）: ST11 は何も読まないので、**履歴の権限が許される端末では、どちらを選んでも失うものは同じ**
  （ST34 が読み始めるまでに Health Connect から消えた分は、どちらでも読めない。deep-review R1）。差が出るのは**履歴の権限が無い端末**（断られた・版が
  対応していない）だけで、そこでは読める範囲の起点が「最初にどれかの種類を許した時の 30 日前」（P13）になる —— 初回起動で種類を許していれば、
  ST34 で初めて許すより起点が早い。P1・P2・P3・P13 は確かさ 4。本人の端末（P10）は Q2 で聞く。Q2 の答えが「Android 15 以上で履歴を許す」なら、
  Q1 には失うものが残らない
- **Q5**（許可を観た記録を残すか）: 失う側は「残さない」。Health Connect は許可した時刻を読む公開の口を持たない（P11。確かさ 2）ので、
  その時に残さなければ後から作れない —— この前提が外れても、残して失うものは無い
- エミュレータは Android 15 だけで確かめた。Android 14 以下の振る舞い（P8・P15）は公式文書で止めてある（確かさ 2）。どちらも失うもののある決定には載せていない（Q8 は B）。本人の版が 14 以下なら、第 2 回で確かめ直す
