# 収集側の送信契約

**C-01（Kotlin）と C-02（Rust）の 2 実装が同じ形を送る。** 片方だけ直す事故を防ぐため、
形と冪等キーの作り方はこの文書と `crates/server/src/ingest.rs` が単一の情報源になる。

## 送る形

`POST /ingest` に次の JSON を 1 件ずつ送る。

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
| `raw` | JSON（取得元から受け取ったそのまま） | FR-18 |
| `payload` | JSON（解析済み） | — |

## 冪等キー

サーバが `logical_source` + `event_time` + `raw` から作る。**収集側が採番した `id` は混ぜない** ——
混ぜると再送のたびに別物になり、重複が入る（FR-22）。

同じ 1 件を 2 回送ると、2 回目は `{"duplicate": true}` が返り行は増えない。
**これを `tools/smoke.sh` の手順 5 が毎回確かめる。**

## 守ること

- **登録簿に無い `logical_source` は 400 で拒否される。** ソースを増やす操作は
  「登録簿へ 1 行 INSERT」だけで、API のコードは変えない（FR-61）
- **バッファから捨てたときは、捨てた期間と件数を稼働記録に残す**（FR-9 / FR-33）。
  残さないと「データが無い」の意味が後から区別できなくなる
- **私的データをログに出さない**（製造準備 A-2）。出すのは件数・ソース名・所要時間・エラーの種別だけ
