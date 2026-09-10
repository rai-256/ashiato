# code-verify — st01-location-ingest

> **処置は 2026-09-09 に実装側が追記した。** `kind` を変えた 4 件は理由を `再分類:` に書いた
> （`superpowers:receiving-code-review` の規範 —— 鵜呑みにも空返事にもしない）。
> 根拠はすべて手元で再現してから処置している。

独立検証。読んだ印象ではなく、**実行した結果**だけを書く。コードは触っていない
（`git status` は検証の前後とも clean。破壊試験はすべて `/tmp/.../scratchpad` の複製で行った）。

## 申告と実測

実装側の申告: tasks 43/44（残りは 8.4 の実機のみ）、検証コマンドすべて rc=0、ガード 4 つを
わざと壊して落ちることを確認済み。

| # | 申告 | 実行したコマンド | 実測 | 一致 |
|---|---|---|---|---|
| 1 | `cargo test --workspace` 13 件 | 同左 | rc=0 / 13 passed | 一致 |
| 2 | `cargo clippy -- -D warnings` | `cargo clippy --workspace --all-targets -- -D warnings` | rc=0 | 一致 |
| 3 | `cargo fmt --all --check` | 同左 | rc=0 | 一致 |
| 4 | `npm run lint` | `cd web && npm run lint` | rc=0 | 一致 |
| 5 | Android 単体 52 件 | `./gradlew :app:testDebugUnitTest` | rc=0 / testcase 52 | 一致 |
| 6 | `./tools/smoke.sh` 手順 22 まで | 同左 | rc=0 / `縦串 OK（実データ経路まで）` | 一致 |
| 7 | `check-immutable.sh` | 同左 | rc=0 | 一致 |
| 8 | `check-migrations/openapi/licenses/boundaries/panic-log.sh` | 同左（5 本） | 全部 rc=0 | 一致 |
| 9 | `check_scenarios.py . st01-location-ingest` | 同左 | rc=0 / Scenario 30・印 33・担保なし 0 | 一致 |
| 10 | `check_chain.py .` | 同左 | rc=0 | 一致 |
| 11 | `hash_is_pinned` は独立算出 | python3 hashlib で再計算 | `39d0ebc5…5df1` **一致** | 一致 |
| 12 | `hash_follows_text_not_structure` は独立算出 | 同上 | `738cb0ca…accf` **一致** | 一致 |
| 13 | 壊すと落ちる: `Outbox.add` の `store.save` | 複製で削除 → gradle | rc=1 / `OutboxStoreTest` 2 本 FAIL | 一致 |
| 14 | 壊すと落ちる: `authorization` ヘッダ | 複製で削除 → gradle | rc=1 / `HttpTransportTest` 2 本 FAIL | 一致 |
| 15 | 壊すと落ちる: 拒否時に `start()` | 複製で置換 → gradle | rc=1 / `MainActivityTest` 3 本 FAIL | 一致 |
| 16 | 壊すと落ちる: `raw` を `jsonb` のまま → **手順 19** | 複製で 0003 を無効化 → smoke | rc=22。**手順 4** で `column "raw" is of type jsonb but expression is of type text`。手順 19 は実行されない | **不一致**（R6） |

**捏造・空テストは無い。** 申告された 15 件は再現した。以下は申告の外側で出たもの。

---

## R1. 本人の決定 3 件が着地する唯一の場所（`LocationService`）にテストが 1 本も無い。60 秒・5 分・永続 Outbox を全部差し替えても 52 件が緑のまま通る

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: 複製で以下 3 か所を同時に書き換えた。
  - `LocationRequest.Builder(PRIORITY_HIGH_ACCURACY, FIX_INTERVAL_MS)` → `1_000L`（取得 60 秒 → 1 秒）
  - `scheduleWithFixedDelay(..., SEND_INTERVAL_MS, SEND_INTERVAL_MS, ...)` → `10_000L, 10_000L`（送信 5 分 → 10 秒）
  - `Outbox(FileOutboxStore(File(filesDir,"outbox.json")){…})` → メモリだけの匿名 `OutboxStore`
    （**深掘り 第 2 回で「最大 5 分ぶんが無言で消える」として本人に戻した欠陥そのもの**）

  結果: `./gradlew :app:testDebugUnitTest` → **rc=0 / tests=52 failures=0**。`assembleDebug` も通る。
  `.github/workflows/ci.yml` の android job は `assembleDebug` + `testDebugUnitTest` のみなので CI も緑。
- 追加の根拠: `grep -rn "LocationService" app/src/test/` → 0 件。`IntervalTest` は定数の値
  （`assertEquals(60_000L, FIX_INTERVAL_MS)`）を固定しているだけで、**その定数が本番経路で使われているか**は
  誰も見ていない。`TestOutbox.kt` は「既定を持たせない」設計意図を書いているが、匿名オブジェクトで簡単に迂回できる。
- kind: technical
- 処置: fixed D25
- 提案: `LocationService` を Robolectric で起こす試験を 1 本（`MainActivityTest` と同じ道具立て）。
  `onCreate` 後に `outbox` が `FileOutboxStore` を使っていること、`onStartCommand` が `FIX_INTERVAL_MS` を
  `LocationRequest` に渡していること、flusher の周期が `SEND_INTERVAL_MS` であることを観測する。
  取りにくければ最低限、間隔と store の生成を `LocationService` の外（純関数 / factory）へ出して観測可能にする。

## R2. 「収集した記録は書き換えられない」は 3 本の UPDATE で素通りする。原文を書き換えて `collected` に戻せる

- 成果物: migrations/0002_immutable_collected.sql / openspec/changes/st01-location-ingest/specs/record-envelope/spec.md
- 根拠: 複製の DB に 0001〜0003 を当てて psql から直接実行（`check-immutable.sh` と同じ経路）。

  ```
  UPDATE core.event SET raw='{"tampered":1}' …;      → 拒否（RAISE EXCEPTION）
  UPDATE core.event SET origin='authored'   …;      → 通る   ← トリガは origin を見ていない
  UPDATE core.event SET raw='{"tampered":1}' …;      → 通る   ← OLD.origin が 'authored' になっている
  UPDATE core.event SET origin='collected'  …;      → 通る
  SELECT origin||' | '||raw …;  →  collected | {"tampered":1}
  ```

  さらに `UPDATE … SET content_hash='rewritten', tz_id='UTC', ingest_time='2000-01-01'` も
  `origin='collected'` のまま通り、実測で `rewritten | UTC | 2000-01-01 00:00:00+00` になった。
  トリガ関数が見ているのは `raw` / `payload` / `event_time` の 3 つだけ（0002_immutable_collected.sql:9-13）。
- 想定脅威との関係: 0002 のコメントが自ら「同じ PC で動く第三者製プラグイン（PERM-8）や psql を直に叩く運用が
  素通りする」を根拠に DB 側へ置いたと書いている。その素通りする主体がまさに 3 本の UPDATE を打てる。
- kind: technical
- 再分類: irreversible → technical。**tamper の結果は不可逆だが、選択そのものが無い** —— spec の SHALL「収集した記録の原文と解析済みの内容を、格納後に書き換えない」が実装で 偽になっていただけで、真にするのに人間の判断は要らない。範囲を広げた分（`content_hash` / `ingest_time` の凍結）は**締める方向で、緩めるのはいつでもできる**。`tz_id` を凍結しない判断は design D21 に理由つきで記録し、deep.md の「問わなかったこと」にも挙げた
- 処置: fixed D21
- 提案: トリガで `NEW.origin IS DISTINCT FROM OLD.origin`（少なくとも `collected` からの離脱）と
  `content_hash` / `ingest_time` の変更も拒む。`check-immutable.sh` に「origin を経由した迂回」の 3 手を足す
  （いまの検査は 1 手ずつしか投げていないので、この経路は原理的に見えない）。

## R3. Scenario「出自の欄がすべて埋まる」の印は主張を観測していない。`device_id` を省いた要求が 200 で通り NULL で保存される

- 成果物: tools/smoke.sh:265（手順 20）/ crates/server/src/ingest.rs:24 / migrations/0001_envelope.sql
- 根拠: 複製でサーバを起動し、`device_id` と `external_id` を省いた要求を投げた。

  ```
  POST /ingest {"id":…,"logical_source":"probe","origin":"collected", … }   （device_id 無し）
  → [{"id":"f0000001-…","duplicate":false,"accepted":true,"error":null}]
  SELECT id, coalesce(device_id,'<NULL>') …  →  f0000001-… | <NULL>
  ```

  spec は「THE SYSTEM SHALL すべての記録に、どの論理ソース・**どの端末が生成したか**を持たせる」だが、
  `IngestRequest.device_id` は `Option<String>`、DDL も `device_id text`（NOT NULL でない）で、
  アプリ層にも検査が無い。
- 印が空振りしている理由: smoke 手順 20 の `missing` クエリは `WHERE logical_source='c01-location'` に限られ、
  その行は台本自身が `"device_id":"c01-smoke"` を入れて作っている。`device_id` 以外の 12 列は
  0001 で NOT NULL なので、この照会は**原理的に 0 以外を返せない**（DDL の言い換え）。
- kind: technical
- 処置: fixed D19
- 提案: `device_id` を必須にする（`IngestRequest` を `String` に、または `origin='collected'` のとき必須の検査 +
  DDL に NOT NULL）か、spec の SHALL を「収集した記録には端末識別子を持たせる」に狭める。
  どちらにせよ smoke に「`device_id` を省いた要求が 400 になる」を 1 手足さないと印は空振りのまま。

## R4. `FileOutboxStore` の「捨てずに脇へ退ける」は 1 回しか成り立たない。2 回目の退避が 1 回目を無言で上書きする

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutboxStore.kt（`salvage()`）
- 根拠: 複製に一時テストを置いて実行（実行後に削除）。

  ```
  outbox.json = "壊れ1" → Outbox(FileOutboxStore(…))  → 1回目退避後: 壊れ1
  outbox.json = "壊れ2" → Outbox(FileOutboxStore(…))  → 2回目退避後: 壊れ2
  dir=[outbox.json.unreadable]        ← ファイルは 1 つしか無い
  org.junit.ComparisonFailure: expected:<壊れ[1]> but was:<壊れ[2]>
  ```

  退避先の名前が固定（`"${file.name}.unreadable"`）で、`File.renameTo` は Unix で上書きするため。
  コメントは「上書きして消すと、何が失われたのか後から誰にも分からない（FR-18 と同じ理由）」と
  書いているが、2 回目以降はまさにそれが起きる。`OutboxStoreTest` の
  「読めない置き場は捨てずに脇へ退ける」は 1 回目しか通していない。
- kind: technical
- 再分類: irreversible → technical。**D17 が既に「捨てずに脇へ退ける」と答えを書いている。** 実装がそれを達成していなかっただけで、新しい選択は無い
- 処置: fixed D22
- 提案: 退避先に時刻か連番を付ける（`outbox.json.unreadable.<epochMillis>`）。
  既存の退避があるときは上書きしないことを `OutboxStoreTest` に 1 本足す。

## R5. `check-licenses.sh` は Android の依存を 1 件も見ていない。D15 で本人へ残した論点（Play Services と AGPL-3.0）がちょうど検査の外側にある

- 成果物: tools/check-licenses.sh / collector-android/app/build.gradle.kts
- 根拠: `./tools/check-licenses.sh` → rc=0、出力は
  `== Rust の依存 / 240 件を確認 / 不許可 0 件` と `== Node の依存 / 170 件を確認 / 不許可 0 件` の 2 節だけ。
  スクリプト本文が読むのは `cargo metadata` と `web/node_modules` のみで、
  gradle / maven を指す行は 1 つも無い（`grep -in "gradle\|android\|maven" tools/check-licenses.sh` → 0 件）。
  一方 `app/build.gradle.kts` は `com.google.android.gms:play-services-location:21.3.0`（Android SDK Terms、
  SPDX の許可一覧に無い）を `implementation` している。
  スクリプト冒頭は「**本体を公開する = 配布が発生する**ので、ここが赤くなったら入れてはいけない」と書いている。
- kind: technical
- 再分類: premise → technical。**許諾の判断そのものは D15 として既に本人の手元にある**（新しく積まない）。この指摘の実体は「検査の緑が『Android も確認済み』に読まれる」ことなので、検査の出力に対象外だと書いて塞いだ
- 処置: fixed 10.9
- 提案: この Story で許諾判断まで踏み込まないなら、少なくとも検査の適用範囲を 1 行明記する
  （`echo "== Android の依存: 対象外（D15 / 公開前に決める）"` を出して、緑が「確認済み」に読まれるのを止める）。
  実質的に塞ぐなら `./gradlew :app:dependencies` か licensee 相当の 1 節を足す。

## R6. tasks.md グループ 9 の「わざと壊して落ちる」表の 4 行目が実態と違う。`raw` を `jsonb` に戻すと smoke は手順 4 で落ち、手順 19 は実行されない

- 成果物: openspec/changes/st01-location-ingest/tasks.md（末尾の破壊試験の表）
- 根拠: 複製で `migrations/0003_raw_text.sql` を no-op（`SELECT 1;`）に置き換えて `./tools/smoke.sh` を実行。

  ```
  RC=22
  == 4. ダミーを 1 件送る
  ERROR ashiato_server: データベース操作に失敗 kind="db"
        detail=error returned from database: column "raw" is of type jsonb but expression is of type text
  ```

  手順 19（並び・重複・表記）には到達しない。表は「落ちた試験: `tools/smoke.sh` 手順 19」と書いており、
  **申告どおりの実験は行われていない**（rc≠0 になること自体は真）。
- なお手順 19 そのものは空振りしていない: 同じ DB で `('{"b":1,"a":2,…}')::jsonb::text` を評価し、
  `{"a": 3, "b": 1, "m": 100, "n": 1.100, "z": "  spaced  "}` に変わることを確認している（実測ログで確認）。
- kind: technical
- 再分類: premise → technical。**申告した実験の記述が実態と違っただけ**で、判断は挟まらない。表を実測どおりに直し、手順 19 まで到達する壊し方（`::jsonb` を挟む）を別に足した
- 処置: fixed 10.10
- 提案: 表の 4 行目を「手順 4（INSERT の型不整合）で落ちる」に直すか、
  型が jsonb でも INSERT が通る形（`::jsonb` キャスト）で壊して手順 19 まで到達させ、
  「手順 19 が守っているもの」を実際に確かめる。

## R7. spec の「資格情報の比較を、一致した長さから内容が推測されない方法で行う」を守るテストが 1 本も無い。定数時間比較を `==` に戻しても全部緑

- 成果物: crates/server/src/lib.rs:36-42 / openspec/changes/st01-location-ingest/specs/record-envelope/spec.md
- 根拠: 複製で `authorize` の畳み込み比較を `let ok = given == app.token;` に置き換えた。

  ```
  cargo test --workspace   → rc=0 / 13 passed
  ./tools/smoke.sh         → rc=0 / 縦串 OK（実データ経路まで）
  ```

  この SHALL には `#### Scenario:` が付いていないため、`check_scenarios.py` の対象外
  （同スクリプトは `^#### Scenario:` しか拾わない）。`git grep` でもこの性質を観測するテストは無い。
- 併せて: `crates/server/src/lib.rs` には単体テストが 0 本で、13 件はすべて `ingest.rs`。
  `authorize` / `ingest_one` / `ingest` / `events` の担保は `tools/smoke.sh` だけに乗っている。
- kind: technical
- 処置: fixed D24
- 提案: 定数時間比較を関数に切り出して「先頭 1 バイトだけ違う合言葉と、全部違う合言葉で
  同じ経路を通る」ことを単体テストで固定する。あるいは spec 側でこの SHALL に Scenario を与えて
  `check_scenarios.py` の網に入れる（いまは SHALL だけの行が機構の穴になっている）。

## R8. Scenario「到達できる間は 1 時間以内に届く」の印（smoke 手順 18）は台本自身が `event_time` に「いま」を入れて即 POST しており、時計がずれない限り落ちない

- 成果物: tools/smoke.sh:234（手順 18）/ 同 108（`NOW=$(date -u …)`）
- 根拠: `./tools/smoke.sh` の実測出力 `== 18. 生成から格納まで 1 時間以内（NFR-1 / tasks 8.5）` → `最大 0 秒`。
  測っているのは `ingest_time - event_time` で、`event_time` は手順 11 が `date -u` から入れた値。
  すなわちシェルの時計と DB の時計のずれ + HTTP 1 往復ぶんしか測れず、**実際の遅延源**
  （C-01 の 5 分バッチ + 到達不能時の再送）を 1 つも通っていない。
- kind: technical
- 処置: fixed 10.11
- 提案: この Scenario は C-01 側の遅延が本体なので、印を Android 側へ移すか
  （R1 で提案した `LocationService` の試験で「生成から送信までの上限が `SEND_INTERVAL_MS`」を観測する）、
  spec の Scenario を「取り込み口に届いた記録は即座に格納される」へ狭めて手順 18 の主張と揃える。

---

## 手ごとの結果

- **手 1（固定値の独立再計算）**: `hash_is_pinned` / `hash_follows_text_not_structure` の 2 件を
  python3 `hashlib` + `struct` で別実装から算出。**両方一致**。実装の出力を写した値ではない。指摘なし。
- **手 2（ガードをわざと壊す）**: 申告の 4 件のうち 3 件は申告どおり（表 #13〜#15）、1 件は落ちる場所が違う（R6）。
  検査の外側として R2（トリガの origin 経由）と R5（Android の依存）を追加。
  `check-immutable.sh` 自体は有効 —— トリガから `raw` の判定だけを外した複製で
  `書き換え禁止 NG / rc=1` になることを確認した。
- **手 3（Scenario と test の突合）**: 30 件すべての印の位置を出して中身を読んだ。
  空振りしていたのは 2 件（R3 / R8）。`tools/smoke.sh` に置いた印のうち手順 14（原文のバイト一致）・
  手順 19（並び・重複・表記）・手順 16（部分失敗）・手順 21（重複を数えない）は主張を実際に観測している。
  `check-immutable.sh:31` の印も有効（手 2 で確認）。
- **手 4（本人の決定が test で固定されているか）**: 5 件を確認。
  「原文を NFC 正規化しない」= smoke 手順 14（バイト列で比較、`->>` を使っていない）で固定 ✓。
  「水平精度でふるい落とさない」= `FixCollectorTest` / `OutboxTest` / smoke 手順 12 で固定 ✓。
  「原文は text」= `raw_must_be_a_string` + 固定鍵 2 本 + smoke 手順 19 で固定 ✓。
  **「60 秒 / 5 分」と「未送信は停止と再開をまたいで残る」は定数と部品の層でしか固定されておらず、
  本番の配線を差し替えても落ちない（R1）。**
- **手 5（tasks の `[x]` と実体）**: 43 件の `[x]` すべてについて、本文に挙がったコマンド・テスト名の実在と
  rc を確認した。**存在しないテスト名・存在しないコマンドは 1 件も無かった。**
  8.4 のみ `[ ]` で「人間の確認待ち」節に挙がっており、`merge_gate.sh` の条件を満たす。
  ずれは R1（9.6 / 9.5 の担保が本番経路に届いていない）・R3（9.7 の担保が空振り）・R6（表の記述）に集約される。
- **手 6（隙間）**: R2 / R4 が「捨てたものは復元できない」型。
  未送信の上限と破棄は `docs/stories/ST04.md` が FR-8 / FR-9 / NFR-7（90 日かつ 2 GB）で持っており、
  ST01 の Non-Goals として妥当（`kind: defer` にはしない —— 隙間に落ちていない）。
  読めなくなった置き場の扱い（R4）は ST04 に無く、ST01 が自分で足した約束なのでここに属する。
  稼働記録の日付境界（`AT TIME ZONE 'UTC'`）は ST02 の担当で `docs/stories/ST02.md` にある。

## 実行したコマンド一覧

```
python3 <独立ハッシュ算出>                          rc=0  両方一致
cargo test --workspace                              rc=0  13 passed
cargo clippy --workspace --all-targets -- -D warnings  rc=0
cargo fmt --all --check                             rc=0
(cd web && npm run lint)                            rc=0
(cd collector-android && ./gradlew :app:testDebugUnitTest)  rc=0  52 testcase
./tools/smoke.sh                                    rc=0  手順 22 まで
./tools/check-immutable.sh                          rc=0
./tools/check-migrations.sh                         rc=0
./tools/check-openapi.sh                            rc=0
./tools/check-licenses.sh                           rc=0  Rust 240 / Node 170（Android は対象外）
./tools/check-boundaries.sh                         rc=0
./tools/check-panic-log.sh                          rc=0
python3 scripts/check_scenarios.py . st01-location-ingest   rc=0  30/30
python3 scripts/check_chain.py .                    rc=0
--- 破壊試験（すべて scratchpad の複製。本体は無変更）---
Outbox.add の store.save を削除 → gradle           rc=1  OutboxStoreTest 2 本 FAIL
HttpTransport の authorization を削除 → gradle     rc=1  HttpTransportTest 2 本 FAIL
MainActivity の finish() を start() に → gradle    rc=1  MainActivityTest 3 本 FAIL
0003 を no-op に → smoke.sh                        rc=22 手順 4 で型不整合（手順 19 に到達せず）
0002 から raw の判定を削除 → check-immutable.sh    rc=1  書き換え禁止 NG
LocationService の間隔と store を差し替え → gradle rc=0  52/52 緑（R1）
authorize を given == token に → cargo test/smoke  rc=0 / rc=0（R7）
psql から origin 経由で raw を書き換え             成功（R2）
device_id を省いた POST /ingest                    200 / NULL で保存（R3）
salvage を 2 回起こす一時テスト                    2 回目が 1 回目を上書き（R4）
```
