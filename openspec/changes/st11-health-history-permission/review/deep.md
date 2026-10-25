# ST11 の深掘り（第 1 回）の独立検査

対象: `openspec/changes/st11-health-history-permission/deep-questions.json`（Q1〜Q10）・`premises.md`（P1〜P11）。
読んだもの: `docs/stories/ST11.md`、`docs/handoff/ST11.md`、`docs/stories/ST34.md`、`docs/requirements.md` の FR-11 / FR-73 / FR-88 / FR-102 / NFR-2 / EXT-C / §5 扉 21、
`openspec/specs/device-collection/spec.md`（「端末は想定間隔ごとに生存信号を送る」「ソースごとに独立して収集する」「利用状況へのアクセスを初回起動で求める」）、
`docs/collector-contract.md`（生存信号の契約・`unused_app_restrictions`）、`collector-android/app/src/main/AndroidManifest.xml`、`MainActivity.kt`、`DeviceId.kt`、`app/build.gradle.kts`、
`migrations/202609291230_clock_source.sql`・`202610141130_outage_alert.sql`・`202610211000_source_acquisition.sql`、`tools/android-emulator.sh`、`.github/workflows/ci.yml`、`python3 scripts/upstream_misses.py report`。

## 手順ごとの記録（問いの一覧を見る前に当てた結果）

- 手順 1（要件の衝突）: FR-11 の理由（「宣言していない期間の 30 日より前は永久に読めない」）と、P2 / P3 の実測（履歴の権限は後から与えても全履歴が読める）が食い違う → Q1 が受けている。
  ただし Q1 の「失うもの」の向きが違う → R1。FR-11 と正典「ソースごとに独立して収集する」（断られても収集を始める）は Q4 が受けている。
  正典「利用状況へのアクセスを初回起動で求める」の「ここで決めた形を ST11 が引き継ぐ」と Q4 の推奨が食い違う → R4。
  FR-73 の 24 時間と「どのソースも生存信号の区間に取得契機が 1 回以上入る」は、ブラウザ履歴と同じく生存信号の区間も 24 時間にすれば両立するので衝突ではない（ST34 の範囲）
- 手順 2（扉の幅）: 扉 21 の「初回起動」が、ST11 入りの版へ更新した既存の端末（初回起動が済んでいる）を含むか → R6。「宣言」（manifest）と「要求」（ダイアログ）の幅 → Q1 が受けている（P4: 宣言だけでは求められない）
- 手順 3（一方通行）: 許可の状態を観た時刻（取らなければ後から作れない。P11）→ Q5。S-01 へ送ると D-01 の書き換えできない行と FR-88 の承認に乗る → R7。
  観た時刻の入力（端末の時計）→ R9。初回起動で種類の権限を先に許すと、履歴の権限が無い端末で読める範囲の起点が早まる（EXT-C "any permission was first granted"）→ R1・R13
- 手順 4（日常）: 新しい論理ソースが途切れの判定の対象のまま入ると通知が鳴る → R8。常駐の通知の操作が増える → Q4 の選択肢 3 が受けている。電池・容量に効く選択は無い（ST11 は読まない）
- 手順 5（既存コード）: `grep -rni health --include=*.kt collector-android` は 0 件（求めるコードが無い。Q1 の context どおり）。
  manifest は `READ_HEALTH_DATA_HISTORY` だけ（`AndroidManifest.xml:55`）、説明の画面も種類の権限も背景の権限も無い（Q1・Q6・Q9 の context どおり）。
  `MainActivity` は `android.app.Activity` で、`requestPermissions` / `checkSelfPermission` だけを使う（`MainActivity.kt:57` `:95-96` `:165` `:173`）。
  `app/build.gradle.kts` の依存に Health Connect の client も `androidx.activity` も無い。manifest に `<queries>` も無い → R2。
  要件の誤った理由はコードと文書にも写っている → R14
- 手順 6（過去の取りこぼし）: 時計（R9）/ 「本人があとで X できる」の費用（R5）/ 書き直せない記録の形（R7）/ 登録簿に載る実体の追加（R7・R8）/ OS の振る舞いを実物で確かめる（R2・R3・R5）/
  申し送りの見出し単位の突き合わせ（`docs/handoff/ST11.md` の見出しは 1 つで Q10 が受けている。該当の抜けなし）/ 「保存場所が丸ごと消えた・入れ直した」（Q4 の族の (d) と Q5 の選択肢 2 が受けている）。
  検証コマンド系の 11 件は tasks がまだ無いので該当なし（確かめた範囲: change に tasks.md が無い）。収集経路の縦串は、Q5 で S-01 へ送る側を採ったときに tasks に 1 項目要る（問いではない）
- 手順 7（族）: Q4 の族に「ST11 入りの版へ更新した既存の端末」（R6）・「はじめにの画面が最初の要求を消費した」（R3）・「種類の一部だけ許した」（R13）が無い。Q5 に時計の兄弟が無い（R9）
- 手順 8（前提）: P9（確かさ 1）が Q1 の loss を、P11（確かさ 2）が Q5 の loss を支えている → R12。P1〜P7 の「実物・4」は確かめ方（エミュレータで検証用アプリを走らせた）と合う。
  P8（公式文書・2）・P11（公式文書・2）・P9（知識・1）も確かめ方と段が合う。ただし P1〜P7 はすべて API 35 の framework の口だけで、minSdk 30 の範囲を覆っていない → R2

## R1. Q1 の「失うもの」は「求めない」側だけのものではない —— ST11 は何も読まないので、Health Connect から消えた分は初回起動で求めても戻らない
- 分類: 不可逆の根拠が誤り（不可逆の記述が誤り）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q1）/ premises.md（「失われるもの」の節）
- 根拠: Q1 の選択肢 2 の irreversible は「ST34 が入るまでの間に Health Connect から消えた分（自動削除・書いたアプリの削除・機種の変更）は読めない」。
  ところが ST11 は読み取りを作らない（FR-73 は ST34。`docs/stories/ST34.md`）。選択肢 1 で初回起動に許されても、ST34 が読み始めるまでに Health Connect から消えた分は**同じく読めない** ——
  権限を許しても Health Connect の自動削除・書いたアプリの削除・端末の初期化は止まらない（P9 の 3 つの経路はどれも権限と無関係）。
  2 つの選択肢で本当に差が出るのは、**履歴の権限が無い端末**（本人が断った・P8 の版で使えない）だけ:
  EXT-C の逐語 "up to 30 days prior to when any permission was first granted"（`docs/requirements.md:1395`）と P1 から、種類の権限を ST11 で先に許せば、読める範囲の起点が「ST11 の許可の 30 日前」まで早まる。
  premises.md の「推奨（初回起動で求める）は P9 に依らない」は正しいが、「失う側は求めない側で、失うのは P9」は誤り
- kind: premise
- loss: uncaptured
- 提案: Q1 の why・選択肢 2 の irreversible を「履歴の権限が使えない・断られた端末では、ST34 で初めて許した日の 30 日より前が読めない（初回起動で種類だけでも許していれば、その 30 日前から読める）」に直す。
  「ST34 までに Health Connect から消えた分」はどちらの選択肢でも失うと両方に書く（それを防ぐのは読み始める時期 = ST34 の順で、この問いの外だと context に 1 行）。
  Q2 の答えが「Android 15 以上・履歴を許す」なら Q1 には失うものが残らない（B に落ちる）ことも Q1 の context に書く
- 処置: escalated —— Q1 の why・context・選択肢 2 の irreversible を直した。「ST34 までに Health Connect から消えた分」はどちらでも失うと context に書き、差は履歴の権限が無い端末の起点（P13）だけとした。Q2 の答えが「Android 15 以上で履歴を許す」なら B に落ちることも context に書いた（Q1 は premise / uncaptured のまま A）

## R2. P1〜P7 は Android 15 の framework の口だけで確かめた —— minSdk 30（11〜13 は Health Connect がアプリ）で、いまの求め方・読み方が効くかが前提に無い
- 分類: 抜け（既存コード・前提が誤りうる）
- 成果物: openspec/changes/st11-health-history-permission/premises.md / deep-questions.json（Q1・Q4・Q7・Q8・Q9）
- 根拠: premises.md の確かめ方は `system-images;android-35` で `requestPermissions([...])` を呼んだもの。`collector-android/app/build.gradle.kts:24` は `minSdk = 30`。
  `MainActivity.kt:57` は `android.app.Activity`（Jetpack の結果の契約を受ける `ComponentActivity` ではない）、権限の判定は `checkSelfPermission`（`:95-96`）、求めるのは `requestPermissions`（`:165` `:173`）。
  `app/build.gradle.kts` の dependencies に `androidx.health.connect` も `androidx.activity` も無く、manifest（`AndroidManifest.xml`）に Health Connect のアプリを引く `<queries>` も無い。
  Android 13 以前で Health Connect の権限が OS の実行時の権限として `checkSelfPermission` / `requestPermissions` で扱えるかは premises.md のどこにも無い（P8 は「履歴の権限が使えるか」だけで、求め方の口を見ていない）。
  Q8 は「Health Connect が無い」だけを扱い、「13 以前でアプリが入っている」端末の求め方は族にも無い。
  Q9 の「初回起動の Health Connect の画面に 1 項目増える」も P4 で確かめていない（読み手は背景の権限を宣言したが、求めたのは歩数と履歴の 2 つ）
- kind: premise
- 提案: premises.md に「Android 13 以前（Health Connect がアプリ）と 14 で、求める口・判定の口が何か」の行を足す（実物で走らせられなければ公式文書の段で確かさを付ける）。
  Q8 の族に「13 以前でアプリが入っている」を足し、求め方の違いは design の判断として context に書く（失うものは無いので B のまま）。Q9 の「1 項目増える」は P4 の追試で確かめるか、未確認と書く
- 処置: fixed premises.md —— P15: Android 13 以前の求め方（Jetpack の client と Health Connect のアプリ）と、いまの実装が OS の口だけで client の依存も無いことを P15（確かさ 2。13 以前の image が手元に無い）に書いた。Q8 の問いと context に「13 以前でアプリが入っている」端末の求め方を足した（口の選び方は design。B のまま）。Q9 の「1 項目増える」は 2 回目の実験で確かめ、P14 にした

## R3. 「はじめに」の画面が最初の要求を拒否で返す（P7）のに、Q4 の推奨「自動で求めるのは 1 度だけ」がその 1 度の消費を区別しない
- 分類: 抜け（族: Q4 の「健康データの権限が無い状態」）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q4・Q7）/ premises.md（P6・P7）
- 根拠: P7「『Get started』を押すと、その回の要求は拒否で返った（2 回目から権限の画面が出た）」。Q7 の context は「この 1 回を数えずにもう一度求める必要がある」と書くが、
  P7 の拒否も本人の拒否も結果は同じ `-1` で、Q4 の推奨（初回起動の 1 度だけ・以後は自動で求めない）のままだと、その端末で初めて Health Connect を使う本人には**権限の画面が 1 度も出ない**。
  また P6（2 回断ると以後は出ない）の 2 回に「はじめに」の 1 回が数えられるかは確かめていない（P6 は「はじめに」を抜けた後の端末で測っている）
- kind: technical
- 提案: Q4 の族に「その端末で初めて Health Connect を使い、はじめにの画面で要求が消費された」を足し、何をもって「1 度求めた」とするか（例: 権限の画面が出たことを何で知るか）を design の判断として context に書く。
  premises.md に「はじめに → 1 回断る → もう一度求めると出るか」を 1 行足して走らせる
- 処置: rejected: 前提（P7 の旧い結果）が誤りだった —— 2 回目の実験で、「はじめに」の画面の「Get started」を押すとその回のまま種類の画面が出て、そこから 2 回断るまで画面は出続けた（`openspec/changes/st11-health-history-permission/premises.md` の P7・P6 (2) の行）。1 回目は履歴の権限だけを求めていて P4 で拒否になっていた。P7 を書き直し、Q4・Q7 の context も直した

## R4. Q4 の推奨（以後は案内しない）は、正典の「ここで決めた形を ST11 が引き継ぐ」（常駐の通知から設定画面へたどれる）と食い違うのに、選択肢がそれを書いていない
- 分類: 要件・正典との食い違い
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q4）
- 根拠: `openspec/specs/device-collection/spec.md` の「利用状況へのアクセスを初回起動で求める」は「設定画面へ**自動で送るのを 1 度だけ**とし、以後は常駐の通知から同じ設定画面へたどれるようにする」と書き、
  続けて「**ここで決めた形を ST11 が引き継ぐ。** 同じ capability で、同じ『その場では取れない権限』の型にあたる」。
  Q4 の推奨は「許したくなったら Health Connect の設定で本人が許す（アプリは案内しない）」で、正典の形は選択肢 3 のほう。context はこの正典を引くが、推奨が正典の形から外れることと、外れるなら正典の注記を MODIFIED で直すことを書いていない。
  `docs/handoff/ST11.md` の点 3 は、常駐の通知からの道を「戻る道」と呼び、それが消える経路を潰した経緯を書いている
- kind: conflict
- 提案: Q4 の選択肢 1 の detail に「正典の『ST11 が引き継ぐ』の形（通知からたどれる）から外れる。採るなら正典の注記を直す」と書く。選択肢 3 の detail に「正典の形どおり」と書く（どちらを推すかは変えない）
- 処置: escalated —— Q4（B）の推奨を正典の形（初回起動の 1 度だけ自動で求め、以後は常駐の通知から Health Connect の権限の画面へたどれる）に替え、「以後は案内しない」の選択肢に「正典の注記も直す」と書いて本人に問う

## R5. Q4 の推奨の費用「本人が Health Connect の設定で許す」は、実物で確かめていない
- 分類: 前提が誤りうる（過去の取りこぼし「仮の費用が『本人があとで X できる』」）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q4）/ premises.md（P2・P3・P6）
- 根拠: Q4 の context「Health Connect の設定からは、本人がいつでも許せる・外せる」に対応する行が premises.md に無い。P2・P3 は `pm grant` / `pm revoke` で付け外ししており、
  Health Connect の設定の画面で（とくに P6 の 2 回の拒否の後に）履歴の権限を本人が許せるかは見ていない（P9 は「エミュレータの画面では辿れなかった」と書いている）。
  `upstream_misses.py report` の末尾 2 項（st23 Q7・Q8）は、費用が「本人があとで X できる」の決定で、その X が出る条件を確かめずに置いた型
- kind: premise
- 提案: premises.md に「P6 の状態（履歴を 2 回断った）から、Health Connect の設定の画面でこのアプリの履歴の権限を許せるか」を足し、`uiautomator dump` で辿って確かめる。
  辿れないなら Q4 の選択肢 1 の detail に「後から許す道が確かめられていない」と書く
- 処置: fixed premises.md —— P12: 2 回断って画面が出なくなった後に、Health Connect の設定の画面（このアプリ）で種類と履歴を許せ、全履歴が読めることを実物で確かめた（確かさ 4）。Q4 の context に書いた

## R6. 扉 21 の「初回起動」の幅 —— ST11 入りの版へ更新した既存の端末（初回起動が済んでいる）が、Q4 の族にも完了の判定にも無い
- 分類: 抜け（扉の幅・族）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q4 の `family.siblings`）
- 根拠: 扉 21「C-01 の初回起動時に要求する」（`docs/requirements.md:1531-1534`）。完了の判定 1 は「アプリを入れ直すと、初回起動で…ダイアログが出る」。
  更新の後の起動は初回起動ではない。入れ直さないと出ない読み方だと、本人の携帯で権限を取る道が「入れ直す」だけになり、入れ直すと端末識別子が変わり（`DeviceId.kt` の注記「アプリを消して入れ直すと変わるが、それは『別の端末』として扱ってよい」）、
  端末の未送信も消える。一方、既存の「1 度だけ」の印（`MainActivity.kt:283-291` の `SENT_TO_USAGE_ACCESS`）は印ごとに別で、健康データの印を新しく持てば更新の後の最初の起動で 1 度求められる
- kind: conflict
- 提案: Q4 の族に「ST11 入りの版へ更新した、初回起動が済んでいる端末」を足し、既定（例: 更新の後の最初の起動で 1 度求める）を置く。
  完了の判定 1 の「入れ直す」はエミュレータ（`tools/android-emulator.sh` の `pm reset-permissions`）で確かめるもので、本人の携帯で入れ直させるものではないと context に書く
- 処置: escalated —— Q4（B）の族に「ST11 入りの版へ更新した既存の端末」を足し、既定（健康データ用の印を新しく持ち、更新の後の最初の起動で 1 度求める）を置いて本人に問う。完了の判定 1 の「入れ直す」はエミュレータで確かめるもので、本人の携帯で入れ直させるものではない（端末識別子が変わり未送信も消える）と context に書いた

## R7. Q5 の推奨（新しい論理ソースとして S-01 へ送る）の「irreversible: なし」「移行は登録簿への 1 行だけ」が足りない —— 種類（FR-102）と記録の形の凍結が書かれていない
- 分類: 不可逆の記述が無い（過去の取りこぼし「書き直せない記録の形」「登録簿に載る実体の追加」）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q5）
- 根拠: FR-102「すべての論理ソースを、登録簿でどれか 1 つの種類（FR-88）に属させる」（`docs/requirements.md:907`）。候補の「健康」「稼働状況」はどちらも FR-88 の表で外部 AI に「承認」（`docs/requirements.md:872` ほか）——
  どちらに置いても、外部 AI の経路ができた後は許可の状態（種類ごとに何を許したか）が外へ出る。選択肢 1 の detail に種類が無い。
  前例の `c01-clock` は `external_id_kind = 'none'` を要した（`migrations/202609291230_clock_source.sql:3-9`）うえ、後の移行で取り方の列（`202610211000_source_acquisition.sql:88`）と途切れの判定の対象外の印（R8）が足されている —— 「1 行だけ」ではない。
  送った記録の原文の形と出来事の時刻は冪等キーの入力（`docs/collector-contract.md`「冪等キー」）で、D-01 に入った後は書き換えない
- kind: premise
- loss: exported
- 提案: Q5 の選択肢 1 の detail に、属させる種類（と、それが FR-88 で外部 AI に承認されていること）を書き、irreversible に「外部 AI の経路ができた後は、送った許可の状態が外へ出る」と「原文の形は D-01 で凍結される（形を変えるなら版を足す）」を書く。
  context の「移行は登録簿への 1 行だけ」を、前例が実際に要した列（`external_id_kind`・`outage_target`）を挙げる形に直す
- 処置: escalated —— Q5 の推奨の detail に種類（稼働状況）・外部の識別子なし・途切れの判定の対象外を書き、irreversible に「原文の形は D-01 で凍結」「稼働状況は外部 AI に承認済み（FR-88）で、経路ができた後は外へ出る」を書いた。context の前例を、`c01-clock` が実際に要した設定（external_id_kind・outage_target・取り方・種類）を挙げる形に直した（Q5 は irreversible / uncaptured のまま A）

## R8. 状態が変わったときだけ 1 件の論理ソースを足すと、途切れの判定の既定（対象）のままでは通知が鳴る
- 分類: 前提が誤り（日常に影響）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q5）
- 根拠: `migrations/202610141130_outage_alert.sql:4` `outage_target boolean NOT NULL DEFAULT true`（新しいソースは既定で判定の対象）。
  `c01-clock` は同じ移行の `:25-27` で明示的に外している（Q5 の context は「`c01-clock` と同じ」と書くが、その「同じ」に外す印が入っていない）。
  Q5 の記録は「状態が変わったときだけ 1 件」なので、何も変わらない端末では想定間隔の 3 倍（`alert_after_days` が NULL のとき）を越えて何も来ず、途切れとして本人に知らせが出る（ST14）
- kind: daily
- 提案: Q5 の選択肢 1 の detail に「途切れの判定の対象から外す（`c01-clock` と同じ印）」を書く。外さない理由があるなら、知らせが鳴る頻度を選択肢に書く
- 処置: escalated —— Q5 の推奨の detail に「途切れの判定の対象外（`c01-clock` と同じ設定）」を書いて本人に問う（Q5 は A）

## R9. Q5 に時計の兄弟が無い —— 「許可した日時」は端末の時計で、狂っていると読める範囲の起点も誤って残る
- 分類: 抜け（族・過去の取りこぼし「時刻を入力にして状態を一方向に動かす決定」）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q5）
- 根拠: `upstream_misses.py report` の第 1 項（ST02 Q29 / ST08 Q7・Q11 / ST06 Q9）。Q5 の context は「権限の状態を観た時刻」を残すとし、選択肢 3 の irreversible は「読める範囲の起点（最初の許可の 30 日前）」をそこから求める前提。
  正典の位置の記録は端末の時計の時刻に加えて**起動からの経過時間と起動の識別**を持ち（`openspec/specs/device-collection/spec.md`「位置の記録が測位の起動からの経過時間と起動の識別を持つ」）、ST05 の測定記録で後から照らせる。
  観た時刻だけを残すと、時計が大きく狂った端末では起点の推定が誤り、経過時間と起動の識別は後から足せない
- kind: technical
- loss: uncaptured
- 提案: Q5 の context の「何を残すか」に「端末の時計の時刻・起動からの経過時間・起動の識別（位置の記録と同じ 3 つ）」を足す（失わない側で費用も小さいので、問いを分けずに既定でよい）。時計が戻った・進んだ 1 件の扱い（状態の変化の判定に時刻を使わない）も 1 行書く
- 処置: escalated —— Q5 の context の「何を残すか」に端末の時計の時刻・起動からの経過時間・起動の識別を足し、状態の変化の判定に時刻を使わない（時計が戻っても進んでも、変わっていなければ 1 件にしない）と書いた。どの選択肢でも同じ既定として Q5（A）で本人に問う

## R10. Q5 の選択肢に、同じ capability の前例（`c01-location` の生存信号の原文に状態を載せる。ST13）が無い
- 分類: 抜け（選択肢）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q5）
- 根拠: `docs/collector-contract.md` の「原文の追加の欄 `unused_app_restrictions`（ST13 / design D12）」—— 権限に近い端末の状態を、新しい論理ソースを足さずに `c01-location` の生存信号の原文だけに載せ、`capturable` / `blockers` に影響させない形が既にある。
  同じ節は「`unused_app_restrictions` は `c01-location` の原文だけ。他のソースの原文には載せない」「収集していないソースの信号を出さない（design D20）」と書く（健康の生存信号は立てられない）。
  この形なら移行が要らず、R7・R8 は起きないが、残る時刻の細かさは生存信号の区間（6 時間）になる
- kind: technical
- 提案: Q5 に選択肢を 1 つ足すか、context に「ST13 の形（`c01-location` の原文に載せる）を採らない理由」を書く。その選択肢の irreversible は「変わった瞬間は 6 時間の幅でしか残らない」
- 処置: fixed deep-questions.json —— Q5: 選択肢「位置の生存信号の原文に載せる（ST13 と同じ形）」を足した（irreversible: 変わった瞬間は 6 時間の幅でしか残らない）。context に前例を 2 つ並べた

## R11. Q6 の推奨の文例「ほかの場所へは送りません」が、FR-88（健康は外部 AI に承認）と合わない
- 分類: 前提が誤り
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q6）
- 根拠: Q6 の選択肢 1 の例「健康データは、あなたの自宅の PC（あしあとのサーバ）にだけ送ります。ほかの場所へは送りません」。
  `docs/requirements.md:872` の FR-88 の表で「健康」は外部 AI・ローカル AI ともに「承認」。外部 AI の経路ができれば、健康データは本人の PC の外へ出る
- kind: premise
- 提案: 文例を「収集アプリが送るのは自宅の PC（あしあとのサーバ）だけ。そこから先に何を渡すかは、あしあとの承認の設定で決まる」の型に直すか、FR-88 の健康の外部 AI への承認に触れる。文面は仮のまま
- 処置: fixed deep-questions.json —— Q6: 文例を「自宅の PC にだけ送る。そこから先に何を渡すかは、あしあとの承認の設定で決まる」の型に直した

## R12. P9（確かさ 1）と P11（確かさ 2）に、loss のある Q1・Q5 が載っている
- 分類: 前提が誤りうる
- 成果物: openspec/changes/st11-health-history-permission/premises.md / deep-questions.json（Q1・Q5）
- 根拠: premises.md の P9「自動削除が既定で入っているかは確かめていない」（知識・1）が Q1 の loss（uncaptured）の唯一の根拠。P11「公開の口に許可の時刻を返すものが見つからない」（公式文書・2）が Q5 の loss の根拠。
  premises.md 自身は「推奨はこれらに依らない」と書くが、**問いが A（人間に返す）か B（仮）かはこれらに依る** —— P9 が外れれば Q1（R1 の訂正後も、Health Connect から消える経路が無いなら）に失うものは無く、P11 が外れれば Q5 の選択肢 2・3 の irreversible は消える。
  P9 は同じエミュレータで Health Connect の設定（データとアクセス → 自動削除）を `uiautomator dump` で辿れば 4 にできる
- kind: premise
- 提案: P9 を実物で確かめて段を上げる（辿れなければ Q1 の context に「自動削除の既定は未確認」と明記）。P11 は段 2 のまま、Q5 の why に「公式文書の公開の項目を探した範囲で」と範囲を書く
- 処置: fixed premises.md —— P9: Health Connect の設定を辿り、自動削除は既定でオフ（選べるのは 3 か月・18 か月・しない）と実物で確かめた（確かさ 4）。P11 は段 2 のまま、Q5 の why に「公式の reference の公開の項目を探した範囲で」と範囲を書いた

## R13. Q3 の「どれを選んでも失うものは無い」は履歴の権限が許された場合だけ —— 許されない端末では、読める範囲の起点が「種類ごと」か「アプリごと」かで変わり、確かめていない
- 分類: 前提が誤りうる（族: 種類の一部だけ許した）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q3・Q4 の `family.siblings`）/ premises.md（P1）
- 根拠: EXT-C の逐語は "30 days prior to when **any** permission was first granted"（アプリごとの起点と読める）。P1 は歩数 1 種類だけで測っており、後から別の種類（睡眠）を許したときの起点が最初の許可（歩数）か、その種類の許可かは見ていない。
  起点が種類ごとなら、Q3 で「睡眠だけ」を選び履歴を断られた端末は、心拍・歩数の範囲を ST34 の許可の 30 日前からしか読めない。
  Q4 の族の (a)(b) は「全部断った」「履歴だけ断った」で、Health Connect の種類の画面で一部だけ許した場合が無い
- kind: premise
- 提案: premises.md に「歩数を許した後、エミュレータの時計を進めてから睡眠を許し、睡眠の古い記録がどこまで返るか」を足す（確かめられなければ EXT-C の逐語で段 2 と書く）。Q4 の族に「種類の一部だけ許した」を足す
- 処置: fixed premises.md —— P13: 端末の時計を 40 日進めて別の種類を許し、起点はアプリごと（最初にどれかを許した時）だと実物で確かめた（確かさ 4）。Q3 の why をそれに合わせ、Q4 の族に「種類の一部だけ許した」を足した

## R14. 要件の誤った理由は、Q1 の訂正の範囲（FR-11・扉 21・EXT-C）の外にも写っている
- 分類: 抜け（既存コード・文書）
- 成果物: openspec/changes/st11-health-history-permission/deep-questions.json（Q1 の選択肢 1）
- 根拠: `grep -rn "永久に読めない"` の結果 —— `collector-android/app/src/main/AndroidManifest.xml:54`、`collector-android/README.md:103`、`docs/stories/ST11.md:14`（価値の文「なぜなら、宣言していない期間の 30 日より前は永久に読めないから」）、
  `docs/stories/ST34.md:28`（扉の逐語）、`docs/requirements.md:211`・`:1533`
- kind: technical
- 提案: Q1 の選択肢 1 の detail の訂正の範囲に、manifest の注記・collector-android の README・ST11 の価値の文を足す（Story と ST34 の扉の逐語は要件を直せば再生成で揃うなら、そう書く）
- 処置: fixed deep-questions.json —— Q1: 選択肢 1 の detail の訂正の範囲に manifest の注記・collector-android の README・ST11 の価値の文（stories.json から再生成）を足し、ST34 の扉の逐語は要件の再生成で揃うと書いた（直すのは record の段）
