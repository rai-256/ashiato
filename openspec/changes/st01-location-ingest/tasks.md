## 1. 冪等キーを安定させる（他の変更より先。鍵が変わると保存済みが全部ずれる）

- [x] 1.1 `sha2` を workspace 依存に追加し、`cargo build` が rc=0 で通ることを確認する
- [x] 1.2 `ingest::content_hash` を SHA-256 に差し替える。既存の 2 つの単体テスト
      （同じ原文で同じ鍵 / 違う原文で違う鍵）が通ることを確認する
- [x] 1.3 **鍵が固定値であることのテストを足す** —— 既知の入力に対する期待値を直書きし、
      `cargo test` で一致することを確認する。これが無いと差し替えの意味が消える
      （版が上がって鍵が変わっても誰も気付かない）

## 2. エンベロープの不足分をサーバ側で埋める

- [x] 2.1 `unicode-normalization` を追加し、JSON の文字列を再帰的に NFC 化する関数を書く。
      NFD の濁点を含む入力が NFC になることを単体テストで確認する
- [x] 2.2 取り込み口で **`payload` にだけ** NFC 化を適用する。
      NFD で送った `payload` の文字列が NFC で保存されることを結合テストで確認する
- [x] 2.2b **`raw` が素通しであることをテストで固定する** —— NFD を含む原文を送り、
      保存された原文がバイト単位で一致することを確認する（design D2。
      後から「揃えたほうがきれい」と正規化を足されるのを止めるため）
- [x] 2.3 `IngestRequest` に `unit_system` / `crs` を `Option<String>` で追加し、
      省略時に既定（`si` / `EPSG:4326`）が入ることをテストで確認する
- [x] 2.4 指定した座標系がそのまま保存されることをテストで確認する
- [x] 2.5 `origin` の値を受け取り時に検査し、列挙外なら **400** を返す。
      応答本文に受け取った値が含まれないことをテストで確認する
- [x] 2.6 時刻・地域の欄（`event_time` / `tz_offset_min` / `tz_id`）を欠いた要求が
      400 で拒否されることをテストで確認する

## 3. 書き換え禁止を DB で強制する

- [x] 3.1 `migrations/0002_immutable_collected.sql` を書く。`core.event` の `BEFORE UPDATE` トリガで
      `origin = 'collected'` の行の `raw` / `payload` / `event_time` の変更を拒否する
- [x] 3.2 `deleted_at` / `deleted_by` の更新は通ることをテストで確認する（論理削除を壊さない）
- [x] 3.3 **わざと UPDATE を投げて落ちることを確認する** ——
      `tools/` に検査を 1 本足し、rc≠0 になることを確認する（製造準備 C の作法に合わせる）
- [x] 3.4 `./tools/check-migrations.sh` が rc=0 で通ることを確認する

## 4. 契約と生成物を更新する

- [x] 4.1 `docs/collector-contract.md` に `unit_system` / `crs` を追加し、
      冪等キーの定義を SHA-256 に更新する
- [x] 4.2 `cargo run -p ashiato-server --bin openapi > docs/openapi.json` で再生成し、
      `./tools/check-openapi.sh` が rc=0 で通ることを確認する

## 5. 取り込み口をまとめ送りに対応させる（design D9）

- [x] 5.1 `/ingest` が**複数件の配列**を受け取れるようにする。1 件だけの要求も
      引き続き通ることをテストで確認する（既存の `tools/smoke.sh` が壊れない）
- [x] 5.2 応答を**送った 1 件ごとの結果**（id と duplicate）の配列にする。
      3 件送って 3 件分の結果が返ることをテストで確認する
- [x] 5.3 **一部が不正でも、正しい分は格納される**ことをテストで確認する
      （1 件でも落ちると全部やり直しになり、恒久的な失敗が後続を永久に止める）
- [x] 5.4 `docs/collector-contract.md` と `docs/openapi.json` を新しい形に更新し、
      `./tools/check-openapi.sh` が rc=0 で通ることを確認する

## 6. C-01（Android）が位置を取る

- [x] 6.1 初回起動時に端末識別子（UUID）を 1 つ生成してアプリの保存領域に置く。
      アプリを再起動しても同じ値が返ることを単体テストで確認する
      （**入れ直すと別端末扱いになるのは本人が受け入れ済み** —— design D6）
- [x] 6.2 位置情報の権限（前景 + 背景）の要求フローを入れ、拒否されたときに
      何も送らずに落ちないことを確認する。
      **一度は検証が無いままチェックされ、独立検証で戻された（2026-09-08）。**
      Robolectric で本番経路（`onRequestPermissionsResult`）を通す 4 本を入れた
      （`MainActivityTest`。design D18）。拒否の 3 経路 + 「全部許可なら始まる」1 本 ——
      最後の 1 本が無いと、何もしない実装でも緑になる。
      検証: `./gradlew :app:testDebugUnitTest` rc=0（52 件）。
      **わざと拒否時に `start()` を呼ばせると 3 本落ちることを確かめた**
- [x] 6.3 前景サービス + `FusedLocationProviderClient` で 60 秒間隔の取得を実装する。
      緯度・経度・水平精度・端末時刻・端末識別子を含む記録が 1 件生成されることを
      単体テストで確認する（**常時通知は本人が受け入れ済み** —— design D7）
- [x] 6.4 **水平精度でふるい落とさない**ことをテストで確認する —— 精度が悪い記録も
      未送信に積まれる（design D11。捨てたものは復元できない）
- [x] 6.5 ログに緯度・経度・原文が出ないことをテストで確認する（出るのは件数・種別だけ）

## 7. C-01 が S-01 へまとめて送る

- [x] 7.1 未送信の記録をアプリ内に保持する置き場を作る（上限と破棄は ST04。ここでは保持だけ）
- [x] 7.2 **5 分間隔**で、その時点の未送信をまとめて 1 回の要求で送る（design D9）。
      5 件たまった状態で送信が 1 回であることを単体テストで確認する
- [x] 7.3 応答を 1 件ごとに見て、**成功した分だけを未送信から取り除く**。
      一部失敗のときに失敗分だけが残ることを単体テストで確認する
- [x] 7.4 送信の形を `docs/collector-contract.md` に合わせる。
      資格情報（`Authorization: Bearer`）を付ける。付けないと 401 になることを確認する
- [x] 7.5 同じ記録を 2 回送っても格納が 1 件のままであることを結合テストで確認する

## 8. 実データで縦串を通す

- [x] 8.1 `./tools/smoke.sh` を実データ経路（登録簿に `c01-location` を登録 → 取り込み → 読み出し）
      に拡張し、rc=0 で通ることを確認する
- [x] 8.2 CI（`.github/workflows/ci.yml`）で 8.1 が走ることを確認する
- [x] 8.3 `cargo clippy -- -D warnings` と `npm run lint --max-warnings 0` が rc=0 で通ることを確認する
- [ ] 8.4 **実機を持って外を歩き**、自宅 PC の `/events` に位置の記録が時刻順に並ぶことを確認する。
      1 行を開いて、原文・2 つの時刻・UTC ずれ・タイムゾーン識別子・収集側の識別子・
      ソースと端末・由来・スキーマ版・利用者識別子・単位系・座標系・感度が
      **すべて埋まっている**ことを見る
- [x] 8.5 生成から格納までが 1 時間以内であることを、`event_time` と `ingest_time` の差で確認する

---

## 人間の確認待ち

**8.4（実機を持って外を歩く）** —— 実機と実データが要る。機械で代われない。
CI は APK のビルドと単体（52 件）までしか見ない（design の Risks 欄どおり）。
接続先と資格情報の渡し方は `collector-android/README.md` §接続先と資格情報。

`docs/stories/ST01.md` の「完了の判定」のうち、ここに残るのは 1 行目と 2 行目
（実機で歩いて `/events` を見る / 1 行を開いて欄がすべて埋まっている）。
3 行目（登録簿に 1 行足すだけで別のソースを受け付ける）と
4 行目（資格情報の無い要求が 401）は `tools/smoke.sh` の手順 9・10 が毎回見ている。

## 実装で決めたこと（design.md に追記）

| | |
|---|---|
| D12 | まとめ送りでも 1 件だけの裸のオブジェクトを受け取り続ける（応答は互換にしない） |
| D13 | 稼働記録の件数は新しく入った行だけ数える（再送が常態になるため） |
| D14 | 400 は「1 件も受け付けなかった」の意味。一部成功は 200 |
| D15 | 位置の取得元は D7 どおり Play Services。**AGPL-3.0 との許諾の論点を Open Questions に残した** |


## 9. 深掘り 第 2 回と独立検証で出た分（2026-09-08 追加）

### 原文を `text` で保存する（deep.md 第 2 回。**先にこれをやる —— 鍵の値が変わる**）

- [x] 9.1 `migrations/0003_raw_text.sql` を書き、`core.event.raw` を `jsonb` から `text` にする。
      `./tools/check-migrations.sh` が rc=0 で通ることを確認する
- [x] 9.2 `IngestRequest.raw` を文字列で受け、**`content_hash` の入力を受け取った原文の文字列そのもの**
      にする。`hash_is_pinned` の期待値を**独立に再計算して**差し替える
      （実装の出力を写さない。Python 等で別に計算して一致を見る）
- [x] 9.3 **並び・重複・表記が保たれることを `tools/smoke.sh` で検査する** ——
      キーが辞書順でなく、重複キーを含み、指数表記の数値を含む原文を送り、
      取り出したものが送ったままであることを確認する（`jsonb` だと落ちる検査であること）
- [x] 9.4 `docs/collector-contract.md` / `docs/openapi.json` / Kotlin の送信を新しい型に合わせ、
      `./tools/check-openapi.sh` が rc=0 で通ることを確認する

### 未送信を停止と再開をまたいで残す（deep.md 第 2 回）

- [x] 9.5 Outbox を端末の保存領域に置く。**プロセスが立て直されても未送信が残る**ことを
      単体テストで確認する（いまはインスタンスフィールドで、最大 5 分ぶんが無言で消える）

### 本人の決定を回帰から守る

- [x] 9.6 `FIX_INTERVAL_MS` = 60 秒 / `SEND_INTERVAL_MS` = 5 分 を**テストで固定する**。
      いまはどのテストからも参照されておらず、値を書き換えても全部通る

### 担保の無い spec Scenario を埋める

- [x] 9.7 「出自の欄がすべて埋まる」「2 つの時刻が両方埋まる」「版と単位が埋まる」を
      `tools/smoke.sh` か結合テストで確認する（いま担保が無い）
- [x] 9.8 「契機ごとに 1 件生成される」を単体テストで確認する（本番経路の `LocationCallback` を通す）
- [x] 9.9 送信に `Authorization: Bearer` が付くことを `HttpTransport` のテストで確認する。
      `/ingest` に資格情報が無いと 401 になることを `tools/smoke.sh` で確認する
      （いまの 401 検査は `/events` だけ、Bearer の検査は Fake で素通り）

### グループ 9 の検証（すべて rc=0）

| コマンド | 見たもの |
|---|---|
| `cargo test --workspace` | 13 件（`raw` が文字列であること・鍵が構造でなく文字列に従うこと） |
| `./gradlew :app:testDebugUnitTest` | 52 件（23 → 52。Robolectric の 10 件を含む） |
| `./gradlew :app:assembleDebug` | APK が建つ |
| `./tools/smoke.sh` | 手順 22 まで。9.3 は手順 19、9.7 は手順 20、9.9 の 401 は手順 9 |
| `./tools/check-immutable.sh` | 0003 を当てたうえで書き換えが拒まれる |
| `./tools/check-migrations.sh` / `check-openapi.sh` / `check-licenses.sh` / `check-boundaries.sh` / `check-panic-log.sh` | 変更後も通る |
| `python3 scripts/check_scenarios.py . st01-location-ingest` | Scenario 30 件すべてに印（担保なし 0） |
| `python3 scripts/check_chain.py .` | 要件 → Story の鎖 |

**ガードをわざと壊して落ちることも確かめた**（製造準備 C の作法）:

| 壊したもの | 落ちた試験 |
|---|---|
| `Outbox.add` の `store.save` を消す | `OutboxStoreTest` 2 本 |
| `HttpTransport` の `authorization` ヘッダを消す | `HttpTransportTest` 2 本 |
| 権限拒否で `finish()` の代わりに `start()` を呼ぶ | `MainActivityTest` 3 本 |
| `raw` を `jsonb` のまま置く（手順 19 が同じ DB で確かめる） | `tools/smoke.sh` 手順 19 |


## 10. 独立レビュー 4 系統の指摘（2026-09-09 追加）

`pr-review-toolkit`（`code-reviewer` / `pr-test-analyzer` / `silent-failure-hunter`）と
`code-verify` で **45 件**。処置は `review/code.md` と `review/pr-toolkit.md` に 1 件ずつ。

**最も重かったのは「緑のまま spec が嘘になる」型**（review R1 / R2 / R3）——
本人が決めた値と、深掘り 第 2 回で戻した欠陥の修正が、**本番の配線では固定されていなかった**。

- [x] 10.1 smoke 手順 20 を**値の突合**に直す（review R3 / test #1）。
      13 欄の NULL を数えていたが 12 欄が DDL で `NOT NULL` なので原理的に落ちない。
      検証: 手順 20 が `7 / 8 件が送ったとおり` を出し、`./tools/smoke.sh` rc=0
- [x] 10.2 格納の前に断るものを `IngestRequest::validate()` に集約する（design D19）。
      空の原文・U+0000・「収集した」記録の端末識別子。
      検証: `cargo test --workspace` rc=0（18 件）/ smoke 手順 20b・20c
- [x] 10.3 由来を経由した迂回を塞ぐ（design D21 / `migrations/0004_immutable_origin.sql`）。
      検証: `./tools/check-immutable.sh` rc=0。**0004 を no-op にすると 4 行が NG になる**
- [x] 10.4 未送信の置き場を JSONL 追記にし、退避先に時刻を付け、成否を返す（design D22）。
      検証: `OutboxStoreTest` 13 件 rc=0
- [x] 10.5 送信と刻みの失敗を黙らせない（review CRITICAL-4 / HIGH-12）。
      `runCatching` の握り潰しをやめ、断られた件数と理由の種別をログに出す。
      検証: `./gradlew :app:testDebugUnitTest` rc=0
- [x] 10.6 失敗経路のテストを足す（review HIGH-8）。書き込み失敗・rename 失敗・
      `.tmp` からの復元・退避の衝突・壊れた 1 行。検証: `OutboxStoreTest` 13 件
- [x] 10.7 本番の配線を試験から観測する（design D25）。`FixSource` / `FlushScheduler` /
      `newOutbox()` を境目にし、`LocationServiceTest` で間隔・周期・置き場・`START_STICKY`・
      権限拒否を見る。検証: **60 秒 → 1 秒 / 5 分 → 10 秒 / 永続 → メモリ に戻すと 5 本落ちる**
- [x] 10.8 マイグレーションの検査を締め、`0003` の戻し手順を機能する形にする
      （review R21 / R22）。`ALTER COLUMN ... TYPE` と `DROP VIEW` を見るようにし、
      `raw::jsonb IS NULL`（例外を投げるうえ原文が漏れる）を `pg_input_is_valid` に替え、
      3 文をトランザクションで囲む。検証: `./tools/check-migrations.sh` rc=0
- [x] 10.9 ライセンス検査の適用範囲を出力に明記する（review R5）。
      Android を 1 件も見ていないのに緑が「確認済み」に読まれていた。
      検証: `./tools/check-licenses.sh` が `== Android の依存 / 対象外` を出す
- [x] 10.10 破壊試験の表を実測に合わせる（review R6）。`raw` を `jsonb` に戻すと
      **手順 4** で型不整合で落ち、手順 19 には到達しない。
      検証: 表の記述と `./tools/smoke.sh` の実測が一致する
- [x] 10.11 NFR-1（1 時間以内）の印を収集側へ移す（review R8 / test #2）。
      smoke 手順 18 は台本自身が `event_time` に「いま」を入れて即 POST しており、
      遅延の本体（5 分バッチ + 再送）を通っていない。
      検証: `IntervalTest` の「送信の間隔は NFR-1 の 1 時間に収まる」
- [x] 10.12 契約の欄名を両側で突き合わせる（review R14 / test #14）。
      サーバが `accepted` を改名しても Kotlin は例外を出さず全件 false と読む。
      検証: `./tools/check-openapi.sh` rc=0。**Kotlin 側で改名すると NG になる**
- [x] 10.13 テストの空振り・トートロジーを直す（review R7 / R8 / R9 / R10 / test #3〜#10）。
      検証: `./gradlew :app:testDebugUnitTest` rc=0（70 件）
- [x] 10.14 1 回の送信に載せる上限を置く（design D23）。
      永続化で未送信が日をまたぐようになり、長い圏外のあと**二度と復帰しない**経路ができていた。
      検証: `SenderTest` の上限 2 件

### グループ 10 の検証（すべて rc=0）

| コマンド | 前 | 後 |
|---|---|---|
| `cargo test --workspace` | 13 件 | **18 件** |
| `./gradlew :app:testDebugUnitTest` | 52 件 | **70 件** |
| `./tools/smoke.sh` | 手順 22 | 手順 22 + 20b / 20c / 20d / 21b |
| `./tools/check-immutable.sh` | 5 項目 | **11 項目**（迂回の 3 手を含む） |

**わざと壊して落ちることを確かめたもの**（前回の 4 つに加えて）:

| 壊したもの | 落ちた |
|---|---|
| `0004` を no-op にする | `check-immutable.sh` 4 行 NG（`collected \| {"tampered":1}` を実測） |
| 取得 60 秒 → 1 秒 / 送信 5 分 → 10 秒 / 永続 Outbox → メモリ | `LocationServiceTest` 5 本 |
| Kotlin の `accepted` を改名 | `check-openapi.sh` NG |
