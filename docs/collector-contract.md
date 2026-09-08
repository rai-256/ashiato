# 収集側の送信契約

**C-01（Kotlin）と C-02（Rust）の 2 実装が同じ形を送る。** 片方だけ直す事故を防ぐため、
形と冪等キーの作り方はこの文書と `crates/server/src/ingest.rs` が単一の情報源になる。

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
| `device_id` | text / null | FR-24 |
| `origin` | `collected` / `authored` / `derived` | FR-25 |
| `event_time` | RFC3339（UTC） | FR-19 |
| `tz_offset_min` | integer | FR-20 |
| `tz_id` | IANA のタイムゾーン識別子 | FR-20 |
| `schema_version` | integer | FR-26 |
| `unit_system` | text / 省略可（既定 `si`） | FR-28 |
| `crs` | text / 省略可（既定 `EPSG:4326`） | FR-28 |
| `raw` | JSON（取得元から受け取ったそのまま） | FR-18 |
| `payload` | JSON（解析済み） | FR-27 |

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
| `error` | 断った理由の種別。`malformed` / `unknown_origin` / `unknown_source` |

### 状態符号

| | |
|---|---|
| `200` | **1 件以上を受け付けた。** 一部が断られていても 200（本文の `accepted` を見る） |
| `400` | **1 件も受け付けなかった。** 本文は同じ形の配列 |
| `401` | 資格情報が無いか一致しない |

**一部の失敗で全部やり直さない。** 1 件の恒久的な失敗が後続を永久に止めるため、
成功した分だけを未送信から取り除く（design D9）。取り込み口は冪等なので、
成功したものを再送しても行は増えない（FR-22）。

## 冪等キー

サーバが `logical_source` + `event_time` + `raw` から **SHA-256** で作る。
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
