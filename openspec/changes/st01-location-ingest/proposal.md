## Why

成功条件 1 は「1 年間データが途切れずに収集され続けている」であり、**位置の記録が携帯端末から
自宅 PC へ自動で届き続けること**がその最初の 1 本になる。ST01 は layer 0 —— 残り 35 本の Story が
すべてこの上に乗る土台であり、ここで決めた記録の骨格は後から作り直せない（扉 #6〜#13, #16）。

製造準備 B で縦串（ダミー 1 件を入れて → 保存され → 取り出せる）は通っているが、
**実データを運ぶ経路は存在しない**。C-01（Android）は Gradle が通っただけで位置を取らない。

## What Changes

- **`device-collection` を新設**し、C-01 が 60 秒間隔で位置を記録して S-01 へ送る振る舞いを定義する
- **`record-envelope` を新設**し、記録が失ってはならない項目（原文・2 つの時刻・地域・出自・版・単位・
  利用者）と、取り込み口の受け入れ条件を定義する
- 既存の `/ingest` に**不足している 3 点を足す**:
  - **NFC 正規化（FR-27）が実装されていない** —— 列も処理も無い。混在させて溜めると後から直せない
  - **単位と座標系（FR-28）が収集側から指定できない** —— 列は DEFAULT 任せで、要求本体に欄が無い
  - **`origin` の値検査が DB にしか無い** —— 不正値がアプリ層を素通りして 500 になる（400 であるべき）
- **冪等キーの算出を安定なハッシュへ差し替える（BREAKING）** —— 現在の `content_hash` は
  `std::collections::hash_map::DefaultHasher` を使っている。**Rust 標準ライブラリはこの出力が
  版をまたいで安定であることを保証していない**。DB に永続化して再送判定に使う鍵には使えない
- **「収集した」記録の書き換え禁止（FR-30）を機械で強制する** —— いまは誰も止めていない

## Capabilities

### New Capabilities

- `record-envelope`: 記録の骨格（原文・時刻・地域・出自・版・単位・利用者）と、取り込み口が
  受け入れ / 拒否する条件。API の資格情報もここに属する
- `device-collection`: 携帯端末（C-01）が位置を取得し、S-01 へ届けるまでの振る舞い

> **`docs/stories/INDEX.md` の capability 表は ST01 を `record-envelope` にだけ割り当てていた。**
> しかし FR-1（C-01 が 60 秒間隔で位置を取る）と FR-10（到達できるとき送る）は
> **端末側の振る舞い**であって記録の骨格ではない。表に従うと FR-1 の置き場が
> ST04（layer 2）まで存在しないことになる。**capability 名は表のまま**で、
> `device-collection` の作成を ST01 に前倒す。名前を変えていないので後続の割り当ては壊れない。

### Modified Capabilities

（なし。`openspec/specs/` は空で、既存 capability が無い）

## Impact

| 対象 | 影響 |
|---|---|
| `crates/server/src/ingest.rs` | `IngestRequest` に `unit_system` / `crs` を追加。`content_hash` を安定なハッシュへ差し替え |
| `crates/server/src/lib.rs` | NFC 正規化・`origin` の値検査・書き換え禁止の強制 |
| `migrations/` | 追加のマイグレーション 1 本（書き換え禁止の強制、`content_hash` の作り直し） |
| `collector-android/` | **本体の実装がここで初めて入る**（位置取得・送信・端末識別子） |
| `docs/collector-contract.md` | 欄の追加と冪等キーの定義変更を反映 |
| `docs/openapi.json` | コードから再生成（手書きしない） |
| 既存データ | **縦串のダミーのみ**なので、冪等キーの変更で失われるものは無い |
