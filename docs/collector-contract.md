# 収集側の送信契約

**C-01（Kotlin）と C-02（Rust）の 2 実装が同じ形を送る。** 片方だけ直す事故を防ぐため、
形と冪等キーの作り方はこの文書と `crates/server/src/ingest.rs` が単一の情報源になる。

> ★ 2026-10-11 追加（2026-10-11 の整合の確認）。**この契約は収集アプリ 2 本（C-01 / C-02）のためのもの。**
> 衛星（要件 §1.3 の P-xx）が従う契約は別に作る（衛星の基盤の Story（/stories で立てる））。書き手（FR-24 / FR-92）・
> 終わりの時刻（FR-91）・衛星の溜めと再送（FR-99）はそこで決まり、この文書の欄はまだそれを持たない。

## 送る形

`POST /ingest` に**記録の配列**を送る（まとめ送り。design D9）。
1 件だけの裸のオブジェクトも受け取る —— 既存の収集側を壊さないため（design D12）。

```
POST /ingest
authorization: Bearer <API_TOKEN>
content-type: application/json

[ {…}, {…}, {…} ]
```

| 欄 | 型 | 由来する要件 |
|---|---|---|
| `id` | uuid（収集側で生成） | FR-21 |
| `user_id` | uuid | FR-29 / PERM-1 |
| `logical_source` | text（登録簿にある値のみ） | FR-61 |
| `external_id` | text / null | FR-23 |
| `device_id` | text（**`origin` が `collected` なら必須**） | FR-24 |
| `origin` | `collected` / `authored` / `derived` | FR-25 |
| `event_time` | RFC3339（UTC） | FR-19 |
| `tz_offset_min` | integer | FR-20 |
| `tz_id` | IANA のタイムゾーン識別子 | FR-20 |
| `schema_version` | integer | FR-26 |
| `unit_system` | text / 省略可（既定 `si`） | FR-28 |
| `crs` | text / 省略可（既定 `EPSG:4326`） | FR-28 |
| `source_updated_at` | RFC3339 / 省略可（**外部サービス側の更新時刻**） | FR-22（ST03 / Q20） |
| `external_ref` | text / 省略可（**「対象ごと」の識別子**。重複の判定には使わない） | FR-23（ST03 / Q24） |
| `raw` | **text**（取得元から受け取った文字列そのまま。**空にできない**） | FR-18 |
| `payload` | JSON（解析済み） | FR-27 |

> **`external_id` は「記録 1 件ごと」の識別子。`external_ref` は「対象ごと」の識別子**
> （ST03 / 深掘り Q13 / Q18 / Q24）。動画 ID のように「対象」を指す識別子を `external_id` に
> 入れると、**同じ対象についての 2 件目が一意違反で落ちる** —— 登録簿の
> `external_id_kind`（`record` / `subject` / `none`）がどちらであるかを宣言する。
> **宣言を欠いたソースは `record` に倒れ、識別子を持たない記録が断られる**（Q16）。
>
> **`source_updated_at` は外部サービス側の更新時刻で、こちらの受信時刻ではない。**
> 保存済みより古い到着では既存の行を書き換えない。**持たない到着は「届いた順」で適用され、
> 保存済みの値を消さない** —— 消すと、以後は古い版が来ても止められない。
> 端末からの収集ではどちらも省略する。

**`raw` は JSON の値ではなく「文字列」で送る**（design D16）。取得元から受け取ったものが
JSON なら、その JSON を**文字列にくるんで**送る:

```json
{ "raw": "{\"lat\":35.68,\"lon\":139.76}", "payload": {"lat":35.68,"lon":139.76} }
```

> **なぜ文字列か**: JSON の値として送ると、DB（`jsonb`）が保存の時点でキー順を並べ替え、
> 重複キーを畳み、`1e2` を `100` に展開する。実測:
> `{"b":1,"a":2,"a":3}` → `{"a": 3, "b": 1}`。
> **原文を残す目的（署名の検証・外部サービスとの照合）は、型が正規化する限り達成できない**
> （深掘り 第 2 回）。代償として**原文への SQL クエリはできなくなる** —— 引く側は `payload` を使う。

**`raw` はサーバも触らない。** `payload` の文字列だけが取り込み口で Unicode NFC に揃えられる
（design D2）。原文のバイト列は一度変換すると二度と戻らないので、正規化の対象にしない。

## 返る形

**送った順に並ぶ 1 件ごとの結果の配列。** 位置で対応づける。

```json
[ {"id":"…","duplicate":false,"accepted":true,"error":null},
  {"id":"…","duplicate":false,"accepted":false,"error":"unknown_origin"} ]
```

| 欄 | 意味 |
|---|---|
| `id` | 格納された記録の識別子。断られたときは null のことがある |
| `duplicate` | 既に同じ 1 件があった（再送。行は増えていない） |
| `accepted` | **未送信から取り除いてよい。収集側はこれだけを見る** |
| `error` | 断った理由の種別。下の表を見る |

| `error` | 意味 |
|---|---|
| `malformed` | 欄の形が解釈できない（必須の欄が無い場合を含む） |
| `unknown_origin` | 由来の分類が列挙のどれでもない |
| `unknown_source` | 登録簿に無い論理ソース |
| `invalid_raw` | **原文が空、または DB に格納できないバイト（U+0000）を含む** |
| `missing_device_id` | **「収集した」記録なのに端末識別子が無い** |
| `missing_external_id` | 登録簿が「記録ごと」と宣言したソースなのに外部識別子が無い（ST03 / Q4 / Q18） |
| `empty_external_id` | `external_id` または `external_ref` が**空文字**（ST03 / R12） |
| `id_reused` | 既に格納された記録と**同じ `id` で別の記録**が届いた（ST03 / Q5） |

> ★ 2026-10-11 追加（2026-10-11 の整合の確認）。上の 8 種は収集アプリが受け取りうるものだけ。サーバの列挙
> （`lib.rs` の `IngestError`）は **15 種**で、残りの 7 種（`malformed_claim` / `claim_not_authored` /
> `claim_has_external_id` / `unknown_attribute_kind` / `invalid_claim_value` / `invalid_valid_from` /
> `invalid_supersedes`）は**個人属性の主張だけに当たる**。その表と当たる条件の正本は
> `openspec/specs/personal-entities/spec.md` の「形の合わない主張は受け付けない」。

> **`id_reused` は正常系では一生出ない。** 収集側の識別子は**毎回新しく振る**と決めてある
> （深掘り Q14）ので、この応答は**採番が壊れていることの印**として働く。
> 外部の識別子から `id` を導いてはいけない —— 導くと、更新された記録が
> 「同じ `id`・違う内容」として届き、**「更新する」と「断る」が同じ到着に逆を指す**。
>
> **`empty_external_id` は空文字が NULL ではないために要る。** 空文字は部分索引
> `event_dedup_ext` に載るので、断らないと**2 件目で一意違反になりまとめ送り全体が 500 になる**
> —— ST01 が `device_id` の空文字で踏んだのと同型。

> `invalid_raw` と `missing_device_id` は独立検証で足した（review R3 / R11 / R18）。
> どちらも**格納の前に断らないと 500 になる型**で、500 はまとめ送り全体を落とす ——
> 収集側は本文を読めず 1 件も取り除けないので、**その 1 件が後続を永久に止める**。
> 空の原文はさらに悪く、冪等キーが `logical_source` + `event_time` + `raw` だけから
> 作られるため、**別々の記録が 1 行に畳まれて `duplicate: true`（＝受理）として返る**。

### 状態符号

| | |
|---|---|
| `200` | **1 件以上を受け付けた。** 一部が断られていても 200（本文の `accepted` を見る） |
| `400` | **1 件も受け付けなかった。** 本文は同じ形の配列（空の配列・非配列を送ったときも空の配列を返す） |
| `401` | 資格情報が無いか一致しない |

**一部の失敗で全部やり直さない。** 1 件の恒久的な失敗が後続を永久に止めるため、
成功した分だけを未送信から取り除く（design D9）。取り込み口は冪等なので、
成功したものを再送しても行は増えない（FR-22）。

## 冪等の判定（**2 段**。ST03 / 深掘り Q2 / Q6 / Q15）

**外部サービス上の識別子があればそれで、無ければ内容の鍵で**畳む。判定は
**利用者ごと・論理ソースごと**。

| 登録簿の `external_id_kind` | 何で畳むか | 外部の更新を追えるか |
|---|---|---|
| `record` | `external_id` | **追える**（行が増えず、前の版が履歴に残る） |
| `subject` | 内容の鍵 | 追えない（**更新が行を増やす**。深掘り Q25） |
| `none` | 内容の鍵 | —— |

- **同じ内容でも `external_id` が違えば別の行**（Q6）。同一物の更新を追う手段が
  そこにしか無いため（扉 #12）。読む側は「内容の鍵が同じ複数行を 1 件として読む形」を使う
- **利用者識別子は索引にだけ足し、鍵の中身には混ぜない**（Q15）—— 誤った値を後から直せる
- **本人が消した記録は、同じ内容が別の識別子で届いても入らない**（Q19）。
  取り込まなかった 1 件は**受理として返る**ので、収集側は未送信から取り除いてよい

## 冪等キー

サーバが `logical_source` + `event_time` + `raw` から **SHA-256** で作る。
**`raw` は受け取った文字列そのものを混ぜる**（design D16）—— 構造として解釈し直すと、
保存する値と鍵の入力がずれる。同じ内容でも表記が違えば別の鍵になるので、
**収集側は 1 件を同じ形で直列化し続けなければならない**（再送のたびに形を変えると重複が入る）。
**収集側が採番した `id` は混ぜない** —— 混ぜると再送のたびに別物になり、重複が入る（FR-22）。

各項目は長さを前置してから混ぜる（`("ab","c")` と `("a","bc")` が同じ鍵にならないように）。

> **`DefaultHasher` から差し替えた**（design D1）。std は版をまたぐ出力の安定性を保証しておらず、
> **コンパイラを上げた日に同じ原文が別の鍵になり、全件が重複として二重に入る**。
> 鍵の算出は**サーバ側だけ**が行う。収集側は鍵を作らない。

同じ 1 件を 2 回送ると、2 回目は `"duplicate": true` が返り行は増えない。
**これを `tools/smoke.sh` の手順 5 と 13 が毎回確かめる。**

## 送る間隔

**5 分間隔で、その時点の未送信をまとめて送る**（design D9）。
60 秒ごとに 1 件ずつ HTTP を叩くと電池と通信量を最も消費するのに対し、
NFR-1 の上限は 1 時間あり余裕がある。間隔は可逆な決定なので、実測してから調整する。

## 守ること

- **登録簿に無い `logical_source` は断られる**（`error: "unknown_source"`）。ソースを増やす操作は
  「登録簿へ 1 行 INSERT」だけで、API のコードは変えない（FR-61）
- **水平精度でふるい落とさない**（design D11）。捨てた記録は後から復元できない。
  閾値を設けるなら読む側（ST16 / ST25）の仕事
- **バッファから捨てたときは、捨てた期間と件数を稼働記録に残す**（FR-9 / FR-33）。
  残さないと「データが無い」の意味が後から区別できなくなる
- **私的データをログに出さない**（製造準備 A-2）。出すのは件数・ソース名・所要時間・エラーの種別だけ

## C-01（`c01-app-usage`）が送る `payload` の形（ST06 / design D2）

**利用状況のイベント 1 件につき記録を 1 件送る。** 下表の順に直列化する。
`package` / `class` と種別固有の欄は取得元が値を返したときだけ置き、`null` は置かない。
`event_type` と `event_time` は省略しない。`app_label` は取得時点に表示名を引けたときだけ
`payload` の末尾に置き、取得元の原文である `raw` には置かない。

| 欄 | 型 | 省略の規則 |
|---|---|---|
| `package` | text | 取得元が返さなければ欄ごと省く |
| `class` | text | 取得元が返さなければ欄ごと省く |
| `event_type` | integer | 省略しない |
| `event_time` | RFC3339（UTC） | 省略しない |
| `configuration` | text | その種別で取得元が返したときだけ置く |
| `shortcut_id` | text | その種別で取得元が返したときだけ置く |
| `interaction_action` | text | その種別で取得元が返したときだけ置く |
| `interaction_category` | text | その種別で取得元が返したときだけ置く |
| `standby_bucket` | integer | その種別で取得元が返したときだけ置く |
| `app_label` | text | 表示名を引けたときだけ `payload` の末尾に置く。`raw` には置かない |

## C-01（`c01-app-usage`）が送る取りこぼし（`kind: gap`）の形（ST06 / FR-85 / design D4）

**取りに行ったが取れなかった期間は、イベントと同じ論理ソース（`c01-app-usage`）に記録 1 件として積む。**
下表の順に直列化し、**どの欄も省略しない**。`event_time` は**期間の終わり**（`end` と同じ値）——
始まりに置くと収集を始めた日より前へ落ちうる。**解析済み（`payload`）は原文と同じ**（足す欄が無い）。

**アプリの名前もパッケージの名前も入れない。** この 1 件が言えるのは「この期間の記録が無い」だけで、
そこに何があったかは誰も知らない。推測で埋めると、無かったことが「あった」として正典に入る。

| 欄 | 型 | 省略の規則 |
|---|---|---|
| `kind` | `gap` | 省略しない |
| `begin` | RFC3339（UTC） | 省略しない |
| `end` | RFC3339（UTC） | 省略しない |
| `reason` | `retention` / `clock_skew_abandoned` | 省略しない |

**`reason` は 2 つあり、混ぜない。** 扉 #14 の「データが無い」を②「動いていたが記録が無い」と
見分ける材料なので、**取りに行って無かった**のと**取りに行っていない**のは別のことを意味する:

| `reason` | いつ積むか | その期間を取得元に問い合わせたか |
|---|---|---|
| `retention` | 窓の始まりが取得元の見込みの保持（10 日）より前で、その手前は取得元に無かった | **問い合わせた**（無かった） |
| `clock_skew_abandoned` | 端末の時計と単調な経過の食い違いが 10 日を超えて解消せず、**諦めて**見込みの下限から取り直した（深掘り Q9=c / design D8） | **していない**（取得元にはまだ残っていたかもしれない） |

> **イベントの記録と分けるのは `kind` の有無だけ。** イベントの原文には `kind` が無い
> （取得元が返す欄だけで組み立てる）ので、この欄そのものが 2 種類を分ける。
> 受け手（ST14）はこの表を読んで gap を畳む。

## C-01（`c01-app-usage-rollup`）が送る `payload` の形（ST06 / design D3）

**アプリ・期間・粒度ごとの集計 1 件につき記録を 1 件送る。** 下表の順に直列化する。
`package` は取得元が値を返したときだけ置き、`null` は置かない。それ以外の取得元の欄は
省略しない。`app_label` は取得時点に表示名を引けたときだけ `payload` の末尾に置き、
取得元の原文である `raw` には置かない。

| 欄 | 型 | 省略の規則 |
|---|---|---|
| `granularity` | `daily` / `weekly` / `monthly` / `yearly` | 省略しない |
| `package` | text | 取得元が返さなければ欄ごと省く |
| `begin` | RFC3339（UTC） | 省略しない |
| `end` | RFC3339（UTC） | 省略しない |
| `last_used` | RFC3339（UTC） | 省略しない |
| `last_visible` | RFC3339（UTC） | 省略しない |
| `last_foreground_service_used` | RFC3339（UTC） | 省略しない |
| `total_foreground_ms` | integer（ミリ秒） | 省略しない |
| `total_visible_ms` | integer（ミリ秒） | 省略しない |
| `total_foreground_service_ms` | integer（ミリ秒） | 省略しない |
| `app_label` | text | 表示名を引けたときだけ `payload` の末尾に置く。`raw` には置かない |
## 位置（C-01）の記録の時刻（ST05 / design D6）

### Location.getTime() の出どころ

**確かめた結果（2026-09-29）: エミュレータでは、`getTime()` は端末の壁時計（`System.currentTimeMillis()`）から来ていた。
端末の時計を +5 分ずらすと `getTime()` も追従した。** 衛星の時計ではない（少なくともエミュレータの GNSS では）。

- 手順: API 35 の `google_apis` エミュレータ（`tools/android-emulator.sh` と同じ AVD）で `auto_time` を切り、計測テスト（使い捨て・commit しない）が
  `FusedLocationProviderClient.requestLocationUpdates`（`HIGH_ACCURACY`・1 秒）と `LocationManager` の `gps` を同時に購読。
  ホストから `adb emu geo fix` を 1 秒ごとに送り、テストが `UiAutomation.executeShellCommand("cmd alarm set-time <ms>")` で
  壁時計を +5 分ずらして 15 秒後に戻す。各 fix で `getTime()`・`currentTimeMillis()`・`getElapsedRealtimeNanos()` と `elapsedRealtimeNanos()` を並べた
- 結果: ずらす前も後も `getTime() - currentTimeMillis()` は −1〜−18 ms（受け取りまでの遅れ）で、+5 分（300000 ms）のずれは現れなかった。
  `getElapsedRealtimeNanos()` は端末の起動からの経過時間として単調に進み、時計を変えても影響を受けなかった。
  fused の `getTime()` は `gps` の `getTime()` と同じ値だった（fused は GNSS の値を素通しにしていた）
- **限界**: エミュレータの GNSS HAL は端末の壁時計で時刻を刻むので、**実機の衛星の時刻を持つ GNSS（時計が狂っていても正しい時刻を返す）で
  同じかは、この手順では確かめられていない**。実機では `getTime()` が衛星の時刻になりうる。
  だから `event_time` は補正せず（C4）、`received_device_time` / `fix_elapsed_ns` / `received_elapsed_ms` を並べて残す（D6）。
  **結果がどちらでも spec と以降の Task は変わらない**

### 位置の記録に足した 4 欄（ST05 / design D6 / Q2）

`raw`（文字列）と `payload`（解析済み）の**両方**に載る。`schema_version` は 1 のまま。

| 欄 | 型 | 中身 |
|---|---|---|
| `received_device_time` | RFC3339（ミリ秒まで・UTC） | 測位の結果を**受け取ったときの端末の壁時計**（`DeviceClock.wallMs()`） |
| `fix_elapsed_ns` | integer | 測位の結果が持つ起動からの経過時間（`Location.getElapsedRealtimeNanos()`、ナノ秒） |
| `received_elapsed_ms` | integer | 受け取ったときの起動からの経過時間（`DeviceClock.monoMs()`、ミリ秒） |
| `boot_count` | integer または `null` | 起動の識別（`Settings.Global.BOOT_COUNT`）。**取れない端末では `null`** |

- **この 4 欄を足す前に積んだ記録には無い。** 読む側は欄の有無を許す（未送信に積まれた記録は積んだときの文字列のまま送られる）
- **出来事時刻の意味は変わらない。** `event_time` と `device_time` は今までどおり `Location.getTime()` で、補正しない（C4）
- `fix_elapsed_ns` と `received_elapsed_ms` を並べると測位から受け取りまでの遅れが引け、`received_device_time` から測位の時点の端末の壁時計を戻せる

## 端末の時計のずれ（`c01-clock`）が送る `payload` の形（ST05 / design D5）

**端末が 1 時間ごと・起動時・時計の変更のときに 1 件ずつ送る。** 論理ソースは位置の記録（`c01-location`）と別。
`origin = collected`、`schema_version = 1`。原文（`raw`）は `payload` と同じ JSON を直列化した**文字列**（位置と同じ）。

```json
{
  "kind": "clock-skew",
  "trigger": "hourly",
  "available": true,
  "device_time": "2026-09-29T01:00:00.123Z",
  "elapsed_ms": 123456789,
  "boot_count": 42,
  "references": [
    {"source": "network",  "time": "2026-09-29T00:55:00.100Z", "skew_ms": 300023, "mono_before_ms": 123456780, "mono_after_ms": 123456781},
    {"source": "s01-date", "time": "2026-09-29T00:51:00.000Z", "skew_ms": 300456, "mono_before_ms": 123200000, "mono_after_ms": 123200310,
     "raw": "Tue, 29 Sep 2026 00:51:00 GMT"}
  ],
  "unavailable": [{"source": "gnss", "reason": "not_available"}]
}
```

| 欄 | 型 | 中身 |
|---|---|---|
| `kind` | string | 常に `clock-skew` |
| `trigger` | string | 測った契機（`hourly` / `startup` / `clock_changed`） |
| `available` | boolean | `references` が 1 件以上か。**1 つも取れなかった記録も 1 件残る**（`references: []`） |
| `device_time` | RFC3339（ミリ秒まで・UTC・`Z`） | 測ったときの端末の壁時計。**出来事時刻（`event_time`）と同じ値**で、補正しない |
| `elapsed_ms` | integer | 測ったときの起動からの経過時間（`elapsedRealtime`） |
| `boot_count` | integer または `null` | 起動の識別。**取れない端末では `null`**（そのときは `elapsed_ms` が戻ったことで起動を知る） |
| `references[]` | array | 取れた基準。各要素は `source`・`time`（基準の時刻）・`skew_ms`・`mono_before_ms`・`mono_after_ms`（読む直前と直後の `elapsedRealtime`）。`s01-date` は `raw`（`Date` 見出しそのまま）と `host`（基準にした宛先の `host:port`）も持つ |
| `unavailable[]` | array | 取れなかった基準。各要素は `source` と `reason`。`s01-date` が `unreadable` のときだけ `raw`（`Date` 見出しの原文）と `host` も持つ |

- **出どころ（`source`）は 3 種**: `network`（`SystemClock.currentNetworkTimeClock()`、API 33 以上）/ `gnss`（`SystemClock.currentGnssTimeClock()`、API 29 以上）/
  `s01-date`（送信がすでに受け取った応答の `Date` 見出し）。**3 つのそれぞれが `references` と `unavailable` のどちらかに 1 回ずつ**出る
- **理由（`reason`）の値**: `unsupported`（OS の版がその基準の口を持たない）/ `not_available`（いま取れない）/
  `no_response_since_last`（前回の測定より後に S-01 から応答を受け取っていない）/
  `clock_changed_since`（時計の変更より後に応答を受け取っていない）/ `unreadable`（`Date` 見出しが無い・読めない）/
  `error:<例外の型名>`（読み取りの失敗。値は出さず種別だけ）
- **差（`skew_ms`）の符号**: `端末の壁時計 − 基準の時刻`。**端末が進んでいれば正、遅れていれば負**
- **差に使う壁時計は、その基準を読む直前と直後の間で読む。** `network` / `gnss` は読んだ直後、`s01-date` は応答を読み終えた直後。
  測定の先頭で読んだ壁時計を使い回さない（測定の途中で時計が動いても差に混ざらない）。`device_time` とは別の読み
- **`s01-date` の差は 0〜+999 ms 大きく出る。** HTTP の `Date` は秒で切り捨てなので、基準の時刻が実際より最大 999 ms 手前になる（PC の測定と同じ偏り）
- **測定のために通信を起こさない。** 外部の時刻サーバにも問い合わせない。`s01-date` は送信が受け取った応答の見出しを読むだけ
- **生存信号（`heartbeat`）は送らない。** 測定記録の送信は記録だけで、測定のための生存信号の経路を足さない
- **私的なものをログに出さない。** 時刻の値・差・原文は出さず、件数・種別・`available` だけ

## C-02（`c02-window`）が送る `payload` の形（ST07 / design D1）

**PC の前景から生まれた 1 件は、種類ごとに次の項目を持つ。**
`raw` は**収集側が組んだ JSON をそのまま文字列で**送り、`payload` は同じ中身の解析済み
（`crates/collector-windows/src/contract.rs` の `WindowPayload` が唯一の情報源）。

| 欄 | 型 | どの種類が持つか |
|---|---|---|
| `kind` | `foreground` / `idle` / `powered-off` / `excluded` / `clock-skew` | 全部 |
| `at` | RFC3339（**ミリ秒・UTC**）。`event_time` と同じ値 | 全部 |
| `app_name` | text（アプリの表示名） | `foreground` |
| `exe_path` | text（実行ファイルのパス） | `foreground` |
| `process_name` | text | `foreground` |
| `title` | text（ウィンドウ題名） | `foreground` |
| `url` | text（**アドレスバーに見えている文字列そのまま**） | `foreground`（前景がブラウザのときだけ） |
| `url_unavailable` | `true`（**ブラウザだがアドレスバーが読めなかった**。読めた記録では省く） | `foreground` |
| `range_end` | RFC3339（範囲の終わり） | `idle`（出た側）/ `powered-off` / `excluded` |
| `idle_ms` | integer（**最後の入力からの経過時間**。出た側は区間の長さ） | `idle` |
| `transition` | `enter` / `leave` | `idle` |
| `reason` | `idle` / `locked` / `suspended` | `idle` |
| `ended_by` | `superseded` / `restart` / `unreadable`（**入力の再開以外で閉じた区間**。普通は省く） | `idle`（出た側） |
| `mono_gap_ms` | integer（見回りが飛んだ間に**単調時計**が進んだ長さ） | `idle`（`suspended`） |
| `excluded_count` | integer（**除外した変化の件数**。対象を前景にしたこと自体を 1 回と数える） | `excluded` |
| `boot_at` | RFC3339（**OS が最後に起動した時刻**。区間の始まりより後なら PC は本当に止まっていた。`clock-skew` では**起動の識別**） | `powered-off` / `clock-skew` |
| `clean_stop` | `true`（前回の収集が自分で止まった。電源断・強制終了では省かれる） | `powered-off` |
| `skew_ms` | integer（基準時刻との差。正なら PC の時計が進んでいる。HTTP の日付は秒で切り捨てなので 0〜+999 ms に偏る） | `clock-skew`（**`s01-date` が取れたときだけ。取れなかった記録では無い**） |
| `skew_reference` | text（基準の出どころ = 取り込み口の `host:port`。ループバックなら自分の時計と比べている） | `clock-skew`（**`s01-date` が取れたときだけ。取れなかった記録では無い**） |
| `clock_trigger` | `hourly` / `start` / `jump` / `retry`（測った契機。`jump` は壁時計の飛び・戻り、`retry` は取れなかった契機の測り直しで取れた記録） | `clock-skew` |
| `clock_available` | boolean（`clock_references` が 1 件以上か。**1 つも取れなかった記録も 1 件残る**） | `clock-skew` |
| `uptime_ms` | integer（測ったときの起動からの経過時間。`boot_at` と組で起動をまたいだ比較に使う。本番の刻みは秒） | `clock-skew` |
| `clock_references` | array（取れた基準。要素は下記） | `clock-skew` |
| `clock_unavailable` | array（取れなかった基準。要素は `source`・`reason`・`raw?`） | `clock-skew` |

**`clock_references[]` の要素**: `source`・`mono_before_ms` / `mono_after_ms`（その基準を読む直前と直後の `uptime_ms`。**読み取り時間の上限**の幅）は必ず持つ。
`s01-date` は `time`・`skew_ms`・`host`（取り込み口の `host:port`）。`windows-time-sync` は `raw`（`w32tm /query /status /verbose` の出力。
cp932 のバイト列なので、**ASCII 以外のバイトと `\` は `\xNN`** に直して持つ）・`last_sync`（最後に正常に同期した時刻。表示のまま）・
`sync_source`・`os_offset_ms`（位相のずれ）・`sync_via`（どこから読んだか: `w32tm` / `eventlog`）。状態なので `windows-time-sync` は `skew_ms` を持たない。
**W32Time が止まっていた**（`w32tm` が `0x80070426`）ときは System のイベントログの Time-Service の同期の記録（Event 35 / 37）の最新 1 件から読み、
`sync_via: eventlog`・`last_sync` は**UTC の RFC 3339**（イベントの `TimeCreated`）・`sync_source` はイベントの `TimeSource`・`raw` はイベントの XML・`os_offset_ms` は無い。
記録が無い・読めないときは `clock_unavailable` に `service_stopped`。

- **出どころは 2 つ**（`s01-date` = 取り込み口の応答の日付 / `windows-time-sync` = Windows の時刻同期の状態）。
  **2 つのそれぞれが `clock_references` と `clock_unavailable` のどちらかに 1 回ずつ**出る。
  `reason` は `timeout` / `unreachable`（`s01-date`）・`spawn_failed` / `service_stopped`（Windows Time サービスが止まっていた。`w32tm` の終了コード `0x80070426`）/ `exit:<code>` / `timeout`（`windows-time-sync`）・
  `unparsed`（出力から項目を 1 つも読めなかった。`raw` を持つ）・`worker_failed`（読み取りのスレッドが結果を返さなかった）
- **取れなかった記録は 1 時間の契機ごとに 1 件まで**。測り直し（60 秒ごと）では増やさず、取れたら `retry` の記録を 1 件残す

> **`at` を原文にも入れる理由**: 冪等キーは `logical_source` + `event_time` + `raw` から
> 作られる。本文を持たない記録（`excluded`）の原文が全部同じ文字列だと、
> **同じ秒の 2 件が 1 件に畳まれて除外の件数が黙って減る**。
>
> **項目の並びと省略の規則まで契約**（`payload_shape_is_pinned` が 1 文字単位で固定）。
> 形を後から変えると、**同じ 1 件が別の鍵になって二重に入る**。
>
> **`excluded` は本文を 1 つも持たない**（FR-83 / 深掘り Q5）。除外の判定は
> **送る前**に行う —— 送ってから消すと、消す前にバックアップ（FR-66 / FR-68）へ入る。
>
> **感度の欄は無い**（深掘り Q3）。収集した記録の既定（`sensitivity = 1` =
> 外部 AI に出してよい）に委ねる。**収集側が厳しい側を付けない。**
>
> ★ 2026-10-11 注記（2026-10-11 の整合の確認）。4 段階の感度は要件から外れた（PERM-2★）。衛星に何を出すかは
> 衛星 × 種類の承認（PERM-11）と、本人が記録ごとに付ける「出さない」の印（PERM-2 / PERM-15）で決まる。
> **この既定の値（`sensitivity = 1`）は「出さない」の印ではない** —— 種類ごとの既定を写しただけで、
> 既にある記録はすべて印なしとして始める（PERM-2★）。収集側が欄を持たないことは変わらない。

PC 側の `blockers` は **`foreground`（前景が読めない）/ `uiautomation`（URL の経路が
応答しない）/ `idle`（最後の入力からの経過時間が読めない）** の 3 つ（design D4）。
**信号を出す瞬間ではなく、前回の信号からの区間に一度でも欠けたもの**を挙げる。
端末側の `permission` / `sensor` / `network` とは別の語彙。

> **C-02 は 400 の本文も読む**（`ureq` の既定は 400 で本文を捨てる）。
> 1 件ごとの結果の件数が送った件数と合わない応答では、**何も取り除かない**。

---

# 生存信号の送信契約（ST02 / FR-78）

**記録とは別の受け口 `POST /heartbeat`。** 統合しない（design D9）—— `/ingest` は記録の
エンベロープ（`event_time` / `tz_id` / `schema_version` / `crs` …）を必須にしており、
生存信号はそのどれも持たない。混ぜると片方のために必須の欄が緩む。

## なぜ送るか

FR-33 は**記録を生成したときにしか**稼働記録を書かないので、記録が 0 件の日は
行が無いだけになり、扉 #14 が求める「動きが無かったのか / 収集が壊れていたのか」を
区別できない。**区別は遡って作れない** —— day one から送らなければ、その期間は永久に
判別できない。

## 送る形

```json
[
  {
    "id": "uuid（収集側で採番。冪等キーには混ぜない）",
    "user_id": "uuid",
    "logical_source": "c01-location",
    "device_id": "端末識別子",
    "emitted_at": "2026-05-01T00:00:00Z",
    "capturable": true,
    "blockers": [],
    "attempts": 360,
    "successes": 230,
    "raw": "{\"alive\":true,...}"
  }
]
```

| 欄 | 何を | 断られる条件 |
|---|---|---|
| `emitted_at` | **信号を作った時刻**。受信時刻ではない | 形が解釈できない |
| `capturable` | そのソースを**取得できる状態か**（権限・センサ・接続） | — |
| `blockers` | 満たされていないもの。`permission` / `sensor` / `network` | `capturable=false` なのに空（`missing_blockers`） |
| `attempts` | 前回の信号からの**取得の試行回数** | 欄が無い（`malformed`）/ 負（`invalid_counts`） |
| `successes` | そのうち成功した回数 | `attempts` を超える（`invalid_counts`） |
| `raw` | 原文。**バイト単位で素通し**される | 空 / U+0000 を含む（`invalid_raw`） |
| `logical_source` | どのソースの信号か | 登録簿に無い（`unknown_source`）。★ 2026-10-11 追加（2026-10-11 の整合の確認。サーバの `HeartbeatError` は持っていたが、この表に無かった） |

**理由の無い「取れない」を断る**のは、それが状態③「動いていたが取れない状態だった」の
証拠にならないため —— 権限なのかセンサなのか接続なのかが分からなければ、
扉 #14 が求めた区別に届かない。

**成功が試行を超える信号を断る**のは、取得率が 1 を超えると「眠っていた / 生きていた」の
区別が数として壊れるため。端末の時計が戻ったときに起きうるので、
**収集側は `attempts` を `successes` より小さくしない**（そうしないと未送信に居座る）。

## 返る形

`/ingest` と**同じ形**（`id` / `duplicate` / `accepted` / `error` の配列、送った順）。
**`accepted` が真の分だけを未送信から取り除く。**
400 は「1 件も受け付けなかった」ことを意味し、本文は同じ形のまま返る。

## 冪等キー

サーバが `logical_source` + **`emitted_at`** + `raw` から SHA-256 で作る（記録と同じ作り）。
**収集側が採番した `id` は混ぜない** —— 混ぜると再送のたびに別物になる。
ST01 の Outbox は部分失敗の後で送り直すので、**重複の到着は常態**。

## 送る間隔

**登録簿の想定間隔（`core.source.expected_gap_sec`）に合わせる。**
初期値は 位置 6 時間 / アプリ利用 6 時間 / 写真 6 時間 / ウィンドウ 6 時間 /
ブラウザ履歴 24 時間（FR-35）。**ずらすと、受け手が「想定間隔を超えて何も来ない」と
判定する窓とずれ、正常な運用が「途絶」に見える**（FR-80）。

送信そのものは記録と**同じ 5 分の契機**に乗せる（未送信の仕組みも同じ）。
送出方式は **ST01 の R46（Doze の除外を要求しない）に従う** —— ST02 で決め直さない。

## 再送の扱い

**記録と同じ未送信の仕組み**（追記 JSONL・書きかけの回収・壊れた行の退避）に乗る。
**ファイルは分ける**（~~`heartbeat.jsonl`~~ ★ 2026-10-11: 区切りの置き場 `SegmentStore` の `heartbeats`。記録は `records`）
—— 同じ JSONL に混ぜると、読み戻しで片方が「壊れた行」に見えて退避に回る。

> ★ 2026-10-11 訂正（2026-10-11 の整合の確認）。`heartbeat.jsonl` は ST01 / ST02 の 1 本のファイルで、ST04 で
> 区切りの置き場（`SegmentStore`。`LocationService.kt` の `newHeartbeatOutbox`）へ移した。古いファイルは起動時に
> 取り込まれるだけ（`LegacyOutbox.kt`）。生存信号の置き場は**上限の対象にしない**（捨てない。ST04 の C1 / design D13）。

積んだ 1 件をそのまま送り直すので、**再送は最初と同じ冪等キーを持つ**。

## 守ること

- **記録の生成に相乗りさせない。** 記録が 1 件も生成されない期間に稼働を残すことが
  FR-78 の目的そのもので、記録の契機から呼ぶと記録が 0 件の日には 1 件も出ない
- **収集していないソースの信号を出さない**（design D20）。出すと「収集していないものを
  動いていた・取得できる状態だった」と報告することになり、NFR-13 の達成日数が偽装される
- **私的データを原文に入れない。** 生存信号の原文に載るのは稼働・取得可否・回数だけで、
  位置の値は入らない

---

# 破棄の報告の送信契約（ST04 / FR-9 / FR-33）

**記録・生存信号とは別の受け口 `POST /drops`。** 端末の未送信を保持の上限で捨てたとき、
置き場に書けずに失ったとき、置き場の行が読めなかったときに、**失ったことを範囲と件数で残す**。

## なぜ送るか

扉 #14 の「データが無い」は「バッファから破棄されたのか」を区別できなければならない。
**その事実は捨てた時点を過ぎると端末にも残らない**ので、どの端末が・なぜ・どの時間に何件捨てたかを
受け手が行として持つ（`core.drop_report` / `core.drop_report_hour`。書き換えも削除もできない）。

## 送る形

```json
[
  {
    "id": "uuid（端末が採番。冪等キーには混ぜない）",
    "user_id": "uuid",
    "logical_source": "c01-location",
    "device_id": "端末識別子",
    "reason": "age",
    "created_at": "2026-09-14T00:00:00Z",
    "range_start": "2026-06-01T01:00:00Z",
    "range_end": "2026-06-01T04:00:00Z",
    "count": 180,
    "hourly": [
      {"hour": "2026-06-01T01:00:00Z", "count": 60},
      {"hour": "2026-06-01T02:00:00Z", "count": 60},
      {"hour": "2026-06-01T03:00:00Z", "count": 60}
    ],
    "raw": "{…上の欄を組んだ JSON の文字列そのもの…}"
  }
]
```

| 欄 | 何を | 断られる条件 |
|---|---|---|
| `device_id` | どの端末が捨てたか | 無い・空（`missing_device`） |
| `reason` | `age`（積んでから 90 日）/ `bytes`（2 GB）/ `write_failed`（置き場に書けなかった）/ `unreadable`（置き場の行が読めなかった） | 無い・4 つのどれでもない（`invalid_reason`） |
| `created_at` | 端末が報告を作った時刻 | 形が解釈できない（`malformed`） |
| `range_start` / `range_end` | 捨てた記録の**出来事の時刻**の範囲。**始まりを含み終わりを含まない** | 終わりが始まりより後でない・片側だけ・`age` / `bytes` で欠く（`invalid_range`） |
| `count` | 捨てた件数 | 0 以下（`invalid_count`） |
| `hourly` | 出来事の時刻の **UTC の 1 時間**ごとの件数。`hour` は正時 | 合計が `count` と合わない・範囲と重ならない時間・正時でない・重複・範囲を持つのに空（`invalid_hourly`） |
| `raw` | 原文。**バイト単位で素通し**される | 空 / U+0000 を含む（`invalid_raw`） |

**範囲と `hourly` を持たない形**は `write_failed` と `unreadable` にだけ許す（出来事の時刻が分からない破棄）。
その報告はどの日の件数にも入らない。

**範囲の終わり**（端末の規則）: 同じソースで破棄せずに残った最も古い記録の出来事の時刻が、
最後に捨てた記録から 1 時間以内ならその時刻、なければ最後に捨てた記録の時刻 + 1 ms。
**1 件だけ捨てても終わりは始まりより後**になる。

**位置の値・原文の中身を載せない。** 報告に載るのはソース・端末・理由・時刻の範囲・件数だけ。

## 返る形

`/ingest` / `/heartbeat` と**同じ形**（`id` / `duplicate` / `accepted` / `error` の配列、送った順）。
400 は「1 件も受け付けなかった」ことを意味し、本文は同じ形のまま返る。
**同じ `id` で原文の違う報告**は `id_reused` で 1 件ごとに断る（500 にしない。一括ごと一時的な失敗に読まれると、後ろの報告が届かなくなる）。

**端末は `accepted` が真の分だけを未送信から取り除き、断られた報告は理由を問わず残して送り直す**
（証拠を捨てると扉 #14 の区別が遡って作れない）。形の不正で断られた報告は端末に残り続けるので、
**端末は断られない形でしか報告を作らない**。

## 冪等キー

サーバが `logical_source` + `raw` から SHA-256 で作る。**端末が採番した `id` は混ぜない。**
端末は**一度でも送ろうとした報告を書き換えない**（凍結）ので、再送は最初と同じ鍵になる。
凍結の後に続けて捨てた分は新しい報告になる。

## 稼働状況での見え方

`GET /coverage` の日のセルに `dropped_count`（その日に属する時間ごとの件数の合計）と
`dropped_ranges`（`[{from, to, count}]`。`Asia/Tokyo` の `HH:MM`、日の終わりまで続けば `24:00`）が載る。
同じソースの範囲は**端が接するもの・重なるものをつないでから**、その日を丸ごと覆えば「破棄された期間」になる。
丸ごと覆わない破棄は状態を決めない（画面は印と週の詳細の文字で出す）。
