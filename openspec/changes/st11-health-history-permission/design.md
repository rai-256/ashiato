## Context

動機は `proposal.md` の Why。前提（Health Connect の振る舞い）は `premises.md` の P1〜P15 で、Android 15 のエミュレータで走らせた結果（確かさ 4）と公式文書（確かさ 2）。

いまの収集アプリ（C-01）:

- `MainActivity`（`android.app.Activity`）が権限を順に求める: 位置（前景）→ 背景の位置 → 通知 → 写真と動画と写真の位置（まとめて 1 度）→ 収集を始める（`start()`）→
  利用状況へのアクセスの設定画面へ 1 度だけ送る → 使っていないアプリの一時停止の案内（1 度だけ）。結果のコードは見ず、そのつどの実際の権限状態を見る（design D27 の教訓）
- 「1 度だけ」の印は `SharedPreferences`（`collector`）に持つ（`sent_to_usage_access`・ST13 の `UnusedAppRestrictionsAsked`）
- manifest は `READ_HEALTH_DATA_HISTORY` を宣言しているだけ。種類の権限・背景の権限・説明の画面・`<queries>` は無い。依存に Health Connect の client も `androidx.activity` も無い
- 常駐の通知（`LocationService` の前景サービス）は `setContentIntent` で利用状況へのアクセスの設定画面を開く
- 端末の時計のずれの測定記録（`c01-clock`。ST05）は、生存信号を持たない論理ソースとして、記録だけを未送信に積んで送る前例

## Goals / Non-Goals

**Goals**: 初回起動で Health Connect の権限の画面が実際に出る / 断られても収集は止まらない / 許可の状態が観た時刻とともに S-01 に残る / 位置の拒否の Scenario を正典に入れる。

**Non-Goals**: 健康データを読む（ST34）。健康データの生存信号・稼働状況の格子の行（収集していないソースの信号を出さない）。Health Connect の入手・更新の画面へ送ること（Q8 の選択肢 2 は採らない）。

## 決定の一覧

| D | 何を | 出所 | 仮 |
|---|---|---|---|
| D1 | 論理ソース `c01-health-permission` と登録簿の行 | Q5 推奨 | —（Q5 は A。答え待ち） |
| D2 | 記録の形と、状態が変わったかの判定 | Q5 推奨・R9 | — |
| D3 | 求める口・読む口は Health Connect の Jetpack の client | Q8 の問いの本文・P15 | 仮 |
| D4 | 初回起動の流れの中の位置と「1 度だけ」の印 | Q4・Q7 推奨 | 仮 |
| D5 | 求める種類と権限の名前 | Q3・Q9 推奨 | 仮 |
| D6 | Health Connect の使える状態ごとの扱い | Q8 推奨 | 仮 |
| D7 | 説明の画面の文と manifest の宣言 | Q6 推奨・P5 | 仮 |
| D8 | 常駐の通知の操作 | Q4 推奨 | 仮 |
| D9 | 権限の状態を観る契機 | Q5 推奨・Q4 の族 (d) | 仮 |
| D10 | 位置の拒否の Scenario の文面と試験の印 | Q10 推奨 | 仮 |
| D11 | 記録の送り方（生存信号を持たない・`CollectionSource` にしない） | `c01-clock` の前例 | — |
| D12 | 書き出しの分類（時刻 1 点・座標なし） | ST33 の `excluded.rs` | 仮 |
| D13 | 要件の誤った理由の写しを直す範囲 | Q1 推奨・deep-review R14・spec-review R12 | — |
| D14 | 健康データの印と最後の状態を自動バックアップから外す | spec-review R4 | 仮 |

## D1. 論理ソース `c01-health-permission` と登録簿の行（Q5 推奨）

移行 1 本で `core.source` に 1 行足す（`c01-clock` の移行 `202609291230_clock_source.sql` と同じ形。`ON CONFLICT DO NOTHING`）:

| 列 | 値 | 理由 |
|---|---|---|
| `logical_source` | `c01-health-permission` | `c01-` の前置きで ST13 の trigger が `acquisition = 'device'`・`fetch_trigger = 'on_change'` を入れる（`fetch_interval_sec` は NULL） |
| `display_name` | 携帯端末の健康データの権限 | |
| `expected_gap_sec` | 21600 | 列が `NOT NULL`。稼働状況にも途切れの判定にも使わない（`c01-clock` と同じ扱い） |
| `external_id_kind` | `none` | 外部サービスの記録ではない。既定の `record` だと全件 `missing_external_id` で断られ、端末の未送信に居座る（ST05 R3） |
| `outage_target` | `false` | 状態が変わったときだけ届くので、既定の対象のままだと想定間隔の 3 倍で途切れの知らせが鳴る（deep-review R8） |

- `coverage::must_sources()` には入れない（格子に出さない）。日の記録の画面（ST25）の正規化の一覧にも入れない（出さない）
- **種類（FR-102）は「稼働状況」**。登録簿にはまだ種類の列が無い（`grep` で確かめた。種類の列を作る Story が入れる）ので、この行の移行の注記と `docs/collector-contract.md` に書いておく
- `.down.sql` は `c01-clock` と同じく、記録・破棄の報告などが参照していれば行を残す
- 名前は記録に保存される（凍結）。変えるなら新しい名前を足す

## D2. 記録の形と、状態が変わったかの判定（Q5 推奨・deep-review R9）

`origin = collected`、`schema_version = 1`、`external_id` なし、`event_time` = 観た時の端末の壁時計（補正しない）。原文（`raw`）は `payload` と同じ JSON を直列化した文字列（`c01-clock` と同じ）。

```json
{
  "kind": "health-permission",
  "trigger": "request_result",
  "device_time": "2026-10-24T01:00:00.123Z",
  "elapsed_ms": 123456789,
  "boot_count": 42,
  "availability": "available",
  "features": {"history": true, "background": true},
  "granted": {"sleep": true, "heart_rate": true, "steps": true, "exercise": true, "history": true, "background": false},
  "auto_requested": true
}
```

| 欄 | 中身 |
|---|---|
| `trigger` | どこで観たか: `app_open`（アプリを開いた）/ `request_result`（権限の画面の結果が返った）/ `periodic`（収集の 6 時間ごとの見直し。D9） |
| `availability` | `available` / `not_installed` / `update_required` / `unreadable`（読む口が失敗した。`error` に例外の型名だけ） |
| `features` | 履歴・背景の読み取りに端末が対応しているか。`availability` が `available` でないときは両方 `false` |
| `granted` | D5 の 6 つの権限ごとの許可。`available` でないときは全部 `false` |
| `auto_requested` | 健康データの権限を自動で求めたことがあるか（D4 の印）。**断ったことはこれと `granted` の組で読む**（求めた・許されていない） |

- **状態が変わったかは `availability`・`features`・`granted`・`auto_requested` だけで判定する。** `trigger`・`device_time`・`elapsed_ms`・`boot_count` は使わない ——
  時計が戻っても進んでも、変わっていなければ 1 件にしない。最後に記録した状態（判定に使う 4 欄だけ）を `SharedPreferences` に持ち、未送信に積めた後に書き換える
  （積めなかったら書き換えず、次に観たときにもう一度作る）
- 初めて観たとき（入れ直した後を含む）は最後の状態が無いので、必ず 1 件
- **Q5 の推奨の「端末にも残す」は、この最後の状態（判定に使う 4 欄。時刻を持たない）と、送れるまで未送信に残る記録のこと**（spec の本文に書いた。spec-review R20）。
  観た時刻の履歴を端末に別に持たない —— 履歴は S-01 が持つ
- 冪等キーは `logical_source` + `event_time` + `raw`（`raw` が `elapsed_ms` を含むので、同じ状態の 2 回目の記録が畳まれることは無い。そもそも変わらなければ作らない）
- 欄は試験で固定する（`ClockSkewPayloadShapeTest` の型。キーの順も毎回同じ —— 冪等キーの入力なので形を変えない。形を変えるなら `schema_version` を上げる）

## D3（仮）. 求める口・読む口は Health Connect の Jetpack の client（Q8 の問いの本文・P15）

依存に `androidx.health.connect:connect-client`（1.1 系。`compileSdk 36`・AGP 8.9.1 以上を要する。このアプリは 36 と 8.13.0）を足し、全部の版で同じ口を使う:

- 使えるか: `HealthConnectClient.getSdkStatus(context)`（`SDK_AVAILABLE` / `SDK_UNAVAILABLE` / `SDK_UNAVAILABLE_PROVIDER_UPDATE_REQUIRED`）
- 対応しているか: `features.getFeatureStatus(FEATURE_READ_HEALTH_DATA_HISTORY / FEATURE_READ_HEALTH_DATA_IN_BACKGROUND)`
- 許されているか: `permissionController.getGrantedPermissions()`（suspend。`MainActivity` の主スレッドを塞がない）
- 求める: `PermissionController.createRequestPermissionResultContract()` の `createIntent` で作った intent を `startActivityForResult` で開き、
  `onActivityResult` で `parseResult` を通す。**`MainActivity` は `android.app.Activity` のまま**（`ComponentActivity` へ替えない。ライフサイクルの配線と既存の試験を動かさない）。
  結果の中身は使わず、そのつど `getGrantedPermissions()` で実際の状態を読む（位置と同じ。design D27 の教訓）

読む口と求める口は 1 つの差し替えられる型（`HealthPermissions`）にまとめ、`MainActivity` の `protected open fun healthPermissions()` で返す（`unusedAppRestrictionsReader()` と同じ形。Robolectric は Health Connect を持たない）。
manifest に Health Connect のアプリ（`com.google.android.apps.healthdata`）を引く `<queries>` を足す（Android 13 以前でアプリを見つけるのに要る）。

**反転条件**: 依存が手元か CI で取れない・Android 14 以降で client の求める口が framework の口（P4〜P7 で確かめた `requestPermissions`）と違う振る舞いをする（計測テストで分かる）なら、
Android 14 以降は framework の口（`requestPermissions` / `checkSelfPermission`）にし、13 以前は `availability = not_installed` と同じ扱い（求めない）に倒す。
13 以前の client の振る舞いは確かさ 2（P15。13 以前のエミュレータが手元に無い）。

## D4（仮）. 初回起動の流れの中の位置と「1 度だけ」の印（Q4・Q7 推奨）

`proceed()` の順を「位置 → 背景の位置 → 通知 → 写真と動画 → **健康データ** → 収集を始める」にする（`start()` の前の 1 段）。

- 健康データの段で**求める**のは、`availability = available` で、印 `asked_health_permissions` が立っておらず、D5 の求めるもの（D6 で対応していないものを除く）のどれかが許されていないとき。
  全部許されていれば印を立てずに次へ進む（求めていないので）。その後に本人が設定で外せば、次の起動で 1 度求める（spec の本文に書いた。spec-review R16）
- 観るのは求めるかどうかと別（D9）。使えるかに依らず、起動ごとに最初に段を通るときに観る（spec-review R15）
- **印は結果が返ったときに立てる**（`onActivityResult`。spec の「答えずに終わると次の起動でもう一度求める」の Scenario。spec-review R16）。開いたときに立てると、Health Connect の画面にいる間にプロセスが終わったとき、答えていないのに二度と出ない（ST13 の /code-review R33 と同じ型）。
  結果が返る前に作り直されても、`startActivityForResult` の結果は作り直された画面に届く
- **印は健康データ専用の新しい鍵**。前の版で初回起動を済ませた端末には鍵が無いので、更新の後の最初の起動で 1 度求める（Q4 の族 (f)）。入れ直すと `SharedPreferences` ごと消えるので初回起動と同じ（族 (e)・完了の判定 1）
- 結果が返ったら、印を立て、D9 の観測（`trigger = request_result`）をしてから `proceed()` に戻る（そのまま `start()` へ進む）
- その端末で初めて Health Connect を使うときの「はじめに」の画面は、同じ回のまま権限の画面に進む（P7）。数えない

**反転条件**: 本人の答え（Q4・Q7）が違う / 計測テストで「結果が返らない」経路が見つかる（そのときは開いたときに印を立てる側に倒す。失うのは 1 回分の案内だけ）。

## D5（仮）. 求める種類と権限の名前（Q3・Q9 推奨）

| 欄（D2） | 権限 |
|---|---|
| `sleep` | `android.permission.health.READ_SLEEP` |
| `heart_rate` | `android.permission.health.READ_HEART_RATE` |
| `steps` | `android.permission.health.READ_STEPS` |
| `exercise` | `android.permission.health.READ_EXERCISE` |
| `history` | `android.permission.health.READ_HEALTH_DATA_HISTORY` |
| `background` | `android.permission.health.READ_HEALTH_DATA_IN_BACKGROUND` |

manifest に 6 つとも宣言する（宣言していない権限は求めても画面に出ない）。一覧は 1 つの定数（`HEALTH_PERMISSIONS`）に置き、manifest との一致を試験で見る。
ST34 が足りない種類を見つけたら、そのときに足して求める（履歴の権限は全種類に効き、起点はアプリごとなので、後から足した種類も同じ範囲が読める。P2・P13）。

Q2 の答え（`premises.md` P10）: 本人の端末で Health Connect に書いているのは Google Fit だけで、書いている 19 種類に 4 種類が全部入っている（運動 = Fit の「エクササイズ」）。
残りの 15 種類（距離・体重・血圧など）はこの段で求めない —— 後から足しても、履歴の権限があれば全履歴が、無くても同じ起点（アプリごと。P13）から読めるので、
先に求めて得るものが無い。どれを読むかは ST34 が決める（tasks 9.1 で申し送る）。

**反転条件**: Q3 の答えが違う / ST34 が、初回起動で一緒に求めないと読めない種類を見つける（P13 が成り立たない）。

## D6（仮）. Health Connect の使える状態ごとの扱い（Q8 推奨）

| 状態 | 求めるもの | 記録（D2 の `availability`） |
|---|---|---|
| 使える・履歴と背景に対応 | 4 種類 + 履歴 + 背景 | `available` |
| 使える・履歴か背景に未対応（Android 14 で OS の更新が古い・13 以前で Health Connect のアプリが古い） | 4 種類 + 対応しているものだけ | `available`（`features` の該当が `false`） |
| 無い（13 以前で未導入） | 求めない | `not_installed` |
| 更新が要る | 求めない | `update_required` |
| 読めない（口が例外を投げた） | 求めない | `unreadable` |

- 求めない状態でも印は立てない。開くたびに読み直し、`available` になって印が無ければ 1 度だけ求める
- **履歴に未対応でも種類は求める。** Q8 の推奨の理由「待っても失うものは無い」が成り立つのは Health Connect が無い・更新が要るとき（データが溜まらない）だけで、
  Health Connect が使えて履歴にだけ未対応の端末では、種類を許した時が読める範囲の起点になる（P13）。求めない側は起点を遅らせる（Q1 の推奨の理由そのもの）。
  spec-review R1 を受けて、Q8 の推奨の選択肢をこの 2 つの場合に分けた（本人に渡す前。`deep-questions.json`）。求めない側の選択肢には失うもの（起点の遅れ）を書いた
- Play ストアへは送らない（Q8 の選択肢 2 は採らない）

**反転条件**: Q8 の答えが「入手・更新の画面へ 1 度だけ送る」/「履歴に未対応の端末でも求めない」 / Q2 の答えが Android 15 以降なら、この表の下 4 行はその端末では起きない（形は変えない）。
Q2 の答えは Android 16（`premises.md` P10・P16）で、本人の端末は表の 1 行目（使える・履歴と背景に対応）に当たる見込み。表の形は変えない（ほかの端末と、Health Connect の更新が要る間のため）。

## D7（仮）. 説明の画面の文と manifest の宣言（Q6 推奨・P5）

新しい `HealthRationaleActivity`（`exported = true`、ボタンなしの文だけ。文字の大きさとタップの的は既存の 1 行と同じ）。manifest に 2 つの開き方を宣言する（公式の始め方の文書）:

- Android 13 以前: その activity に `intent-filter` の `androidx.health.ACTION_SHOW_PERMISSIONS_RATIONALE`
- Android 14 以降: `activity-alias`（`ViewPermissionUsageActivity`、`targetActivity` は同じ activity、`permission = android.permission.START_VIEW_PERMISSION_USAGE`）に
  `intent-filter` の `android.intent.action.VIEW_PERMISSION_USAGE` + `android.intent.category.HEALTH_PERMISSIONS`

文（1 つの定数 `HEALTH_RATIONALE`）:

> 収集アプリは、睡眠・心拍・歩数・運動の健康データを読む権限を求めます。読んだものは、あなたの自宅の PC（あしあとのサーバ）にだけ送ります。そこから先に何を渡すかは、あしあとの承認の設定で決まります。いまは権限を取るだけで、健康データを読み始めるのは健康データの取り込みを入れてからです。

**反転条件**: Q6 の答えが違う（Web の画面へのリンクにする）/ 承認の設定（FR-88）の健康の扱いが変わる。

## D8（仮）. 常駐の通知の操作（Q4 推奨）

前景サービスの通知に `addAction` で 1 つ足す（ラベル「健康データの権限」）。押すと収集アプリの Health Connect の権限の画面を開く:
Android 14 以降は `HealthConnectManager.ACTION_MANAGE_HEALTH_PERMISSIONS` + `Intent.EXTRA_PACKAGE_NAME`（P12 で収集アプリの権限の画面が開くと確かめた）、13 以前は client の `HealthConnectClient.getHealthConnectManageDataIntent(context)`（Health Connect のアプリの画面。収集アプリの権限まで何回押すかは確かめていない。spec の Scenario は 14 以降に限った。spec-review R17）。

- 出すのは、通知を組み立てるときに `availability = available` のときだけ（無い端末で押しても開く画面が無い）。許されているかには依らず出す（外す道にもなる。利用状況へのアクセスの `setContentIntent` も常に出ている）
- `setContentIntent`（利用状況へのアクセス）は変えない
- 開けなかったら `health_permission_settings_missing` を種別だけログに出す（`usage_access_settings_missing` と同じ）

**反転条件**: 本人が通知の操作を邪魔と感じる（違和感）→ 許されていないものがある間だけ出す。

## D9（仮）. 権限の状態を観る契機（Q5 推奨・Q4 の族 (d)(g)）

| 契機 | `trigger` | どこで |
|---|---|---|
| 収集アプリを開いた（起動ごとに `proceed()` が初めて写真と動画の段を越えたとき。**Health Connect が使えるかに依らない**） | `app_open` | `MainActivity` |
| 権限の画面の結果が返った | `request_result` | `MainActivity.onActivityResult` |
| 収集の 6 時間ごとの見直し | `periodic` | `LocationService`（生存信号と同じ 6 時間の刻みの隣。生存信号は送らない） |

- 本人が収集アプリを開かずに Health Connect の設定で外した・休止で剥がれた（ST13 が防ぐ）ときは、6 時間ごとの見直しで拾う（変わった瞬間は 6 時間の幅になる。起点の推定は 30 日の単位なので足りる）
- **完了の判定の「許可した日時」は「許可を観た時刻」になる**（spec-review R22）。初回の画面で許したときは結果が返った時刻で一致する。断った後に設定で許したときは、
  次に開いたか 6 時間ごとの見直しの時刻（最大 6 時間後）。Health Connect は許した時刻を読む口を持たない（P11）ので、これより細かくは取れない。確認バッチで本人に見せる
- `onResume` は 1 回の起動で何度も通るが、変わらなければ記録しないので増えない（D2）。読む口は主スレッドを塞がない

**反転条件**: 6 時間の幅で足りないと分かる（ST34 が起点を時間の単位で要する）→ 見直しを短くする。

## D10（仮）. 位置の拒否の Scenario の文面と試験の印（Q10 推奨）

「ソースごとに独立して収集する」を `MODIFIED` し、`#### Scenario: 位置の権限を拒否しても収集は始まり、拒否したことが残る` を足す。THEN / AND は試験が見ている 5 点
（`docs/handoff/ST11.md`: 落ちない・収集が始まる（前景サービスの通知が出る）・取得できない状態の生存信号・`kind=permission_denied` のログ・続けて通知の権限・位置は拒否のまま）。
`PermissionDeniedInstrumentedTest` に `// Scenario:` の印を付ける（いまは印が無い。正典に無いので付けると warn が出ていた）。
生存信号の主張（取得できない状態として載る）は計測テストでは見ていないので、既存の Robolectric（`SourceIndependenceTest` の「位置の取得条件が欠けても位置の生存信号は届く」）と同じ型で 1 本足して、2 か所に印を置く。

**反転条件**: Q10 の答えが「叩き台の文面のまま 1 本だけ」。

## D11. 記録の送り方（`c01-clock` の前例）

- 記録は既存の未送信（`Outbox` / `SegmentStore`）に積み、既存の送信（5 分ごと）で送る。保持の上限（90 日・2 GB）と破棄の報告も既存のまま
- **生存信号を送らない・`CollectionSource` にしない。** 取得契機を持つ収集ではなく、状態の変化を残すだけなので、正典の「どのソースも生存信号の区間に取得契機が 1 回以上入る」の対象にならない
  （`c01-clock` と同じ。`CollectionSource` の一覧に入れると、その Scenario の試験が区間と間隔を比べる対象になる）
- ログは `health_permission_recorded`（`trigger` と `availability` だけ。`granted` の値は出さない。spec の本文も同じ。spec-review R18）

## D12（仮）. 書き出しの分類（ST33 の `excluded.rs`）

`c01-health-permission` を `SPAN_RULES` に `SpanRule::Point`（観た時刻の 1 点）、`NO_COORD_SOURCES` に足す（payload に座標の項目が無いことは D2 の形で決まる）。
足さないと、登録簿の全ソースが一覧のちょうど一方にあることを見る既存の試験が名前つきで落ちる。

**反転条件**: 状態の記録を「次の記録まで続く状態」として幅で読む使い方が出る（ST34）→ `SPAN_NEXT_SOURCES` へ移す（除き過ぎの側に倒れるだけ）。

## D13. 要件の誤った理由の写しを直す範囲（Q1 推奨・deep-review R14）

`grep -rn "永久に読めない"` の 6 か所のうち、

- `docs/requirements.md`（FR-11・扉 21）と `docs/stories/ST11.md`（`stories.json` から再生成）と `docs/stories/ST34.md`（扉の逐語）は **record の段**（答えを受けて ★ で訂正する）
- `docs/requirements.md:950`（FR-73 の「権限の宣言は後回しにできない」）とその逐語 `docs/stories/ST34.md` も **record の段**（語が違うので grep に当たらない。spec-review R12）
- `collector-android/app/src/main/AndroidManifest.xml:54` の注記と `collector-android/README.md:103`、同じ README の `:100`（「5 つの権限は、いずれも宣言しない期間が取り返せないか…」。数も合わなくなる）は **tasks**（Task 2）。
  訂正後の理由（履歴の権限が無い端末で読める範囲の起点を早める）で書き直す

Q1 の答えが「初回起動では求めない」なら、この change は Q10 と前提の訂正だけになり、tasks は作り直し（`spec --answers Q1`）。

## D14（仮）. 健康データの印と最後の状態を、端末の自動バックアップから外す（spec-review R4）

manifest に `allowBackup` も `dataExtractionRules` も無いので、Android の自動バックアップは既定で有効で、`SharedPreferences` を含み、入れ直したときに戻しうる（確かさ 2: 戻しが実際に起きるかは確かめていない）。
戻ると「1 度求めた」の印が残り、入れ直しても求められない（完了の判定 1 が本人の携帯で偽になる）。最後の状態も戻るので、入れ直した後の最初の記録も出ない。

- 健康データの印（`ASKED_HEALTH_PERMISSIONS`）と最後の状態（D2）は、新しい専用の置き場 `health_permission`（`SharedPreferences` の名前）に持ち、`collector` には置かない
- manifest に `android:dataExtractionRules`（Android 12 以降）と `android:fullBackupContent`（Android 11）を足し、`health_permission` だけをクラウドのバックアップと端末の移行から外す
- 既存の印（`sent_to_usage_access`・ST13 の除外の印）と `collector` の置き場の扱いは変えない（この change の範囲の外。同じ型の穴は確認バッチの持ち越しに書く）

**反転条件**: 自動バックアップを丸ごと止める（`allowBackup = false`）と決める change が入る → 専用の置き場は残してよい（害は無い）。

## Risks / Trade-offs

- [計測テストで Health Connect の「2 回断った」状態が残る] → `pm reset-permissions` が Health Connect の権限と断った回数を戻すかは確かめていない（位置は戻る）。
  Task 7 は許す本と断る本をクラスで分け、`tools/android-emulator.sh` と CI に段を足して、段の間で `pm reset-permissions` する（spec-review R6）。断る本を最後の段に置く
- [新しいエミュレータでは「はじめに」の画面が先に出る] → 計測テストは「Get started」を押してから権限の画面を探す（P7。押さないと進まない）
- [Jetpack の client の依存を手元で取れない] → D3 の反転条件（framework の口に倒す）
- [Android 13 以前・14 の振る舞いは確かさ 2] → CI のエミュレータは Android 15 だけ。13 以前・14 の分岐は Robolectric で読み口を差し替えて確かめる（D3）。本人の端末は Android 16（Q2。`premises.md` P10）なので、13 以前・14 の分岐は本人の端末では起きない
- [本人の端末は Android 16（API 36）、前提の確かめと CI のエミュレータは Android 15（API 35）] → 権限は 35 以降の全部の版にある（P16。確かさ 2）。
  計測テストは CI の 35 で画面の文字を読むので、36 で文字や並びが違っても試験は落ちない。本人の端末で違えば、確認バッチで本人が見る（違和感として返る）
- [`getGrantedPermissions()` が suspend] → 主スレッドを塞がない。結果が返るまで健康データの段は先へ進まない（`inFlight` と同じ扱い）
- [通知の操作が 1 つ増える] → 毎日目に入る（deep の手順 4）。Q4 の選択肢に書いてある。違和感なら D8 の反転条件

## Migration Plan

- 移行 1 本（登録簿に 1 行）。`migrations/ORDER` の末尾に 1 行。既存の行は動かさない
- 端末: 更新の後の最初の起動で 1 度求める（D4）。印の無い既存の端末で、許されていない本人の邪魔になるのはこの 1 回だけ
- 戻すとき: 移行の `.down.sql` は記録が参照していれば行を残す。端末の印は残っても害が無い

## Open Questions

- 13 以前で Health Connect のアプリが入っている端末の、client の求める口の振る舞い（確かさ 2。本人の端末は Android 16（Q2）で、本人の端末では起きない）
- 13 以前で通知の操作が開く Health Connect のアプリの画面から、収集アプリの権限まで何回押すか（D8。確かさ 2。本人の端末では起きない）
- 本人の端末（Android 16）で Health Connect の自動削除の設定が見つからない（Q2。`premises.md` P17・確かさ 1）。ST11 は健康データを読まないので、どの決定にも効かない。ST34 へ申し送る（tasks 9.1）
