# pr-review-toolkit — st01-location-ingest

`code-reviewer` / `pr-test-analyzer` / `silent-failure-hunter` の 3 本を
`git diff origin/main` に対してかけた結果（2026-09-09）。**重複は消さずに残す** ——
別々の目が同じところを指したこと自体が情報なので、`code.md` と重なる分も 1 件として書く。

処置は実装側が付けた。**根拠はすべて手元で再現してから**書いている
（`superpowers:receiving-code-review`）。

---

## R9. `.unreadable` へ退けられなかった原文が、60 秒後に黙って上書きされる

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutboxStore.kt
- 根拠: `code.md` R4 と同じ経路。`renameTo` が false のとき `kept`（残した）とログに書くが、
  次の `add` が `outbox.json` を上書きするので実際は残らない。**ログだけが事実と逆になる**
- kind: technical
- 処置: fixed D22

## R10. rename に失敗した `save` が、最新のデータを誰も読まない `.tmp` に置き去りにする

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutboxStore.kt
- 根拠: `load()` は `file.exists()` しか見ておらず `.tmp` を参照しない。
  「次の契機で書き直される」というコメントは**プロセスが次の契機まで生き延びた場合にしか成り立たない**が、
  この Story の脅威モデルはまさに `START_STICKY` の立て直し。前提が噛み合っていない
- kind: technical
- 処置: fixed D22

## R11. `save()` の失敗を上位が知る手段が型として存在しない

- 成果物: collector-android/.../OutboxStore.kt / Outbox.kt / FixCollector.kt
- 根拠: `save` が `Unit` を返すので、`Outbox` も `FixCollector` も `LocationService` も
  永続化できたかを構造的に知り得ない。ストレージ満杯なら**毎回**失敗し続け、位置はメモリにだけ
  積まれて、プロセスが落ちた瞬間に全部消える —— この PR が直したはずの欠陥がそのまま再現する
- kind: technical
- 処置: fixed D22

## R12. `runCatching { sender.flush() }` が Throwable を丸ごと捨てている

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `.getOrNull()` すら取らずログも出さない。`BASE_URL` が `http`/`https` 以外だと
  `openConnection() as HttpURLConnection` が `ClassCastException` を投げ、`IOException` ではないので
  `HttpTransport` の catch を素通りしてここで消える。**送信が 100% 失敗し続けるのに logcat に何も出ない**
- kind: technical
- 処置: fixed 10.5

## R13. 位置の契機ごとに、主スレッドで全件を同期書き込みしている

- 成果物: collector-android/.../Outbox.kt / OutboxStore.kt / LocationService.kt
- 根拠: `requestLocationUpdates(..., Looper.getMainLooper())` なので `onLocationResult` は
  主スレッド。その中の `add` が全件を書き直す。1 件 598 バイトで、1 日圏外 = 1,440 件 = 0.86 MB、
  **その日 1 日で書くバイト数は約 0.6 GB**。数 MB になると ANR / StrictMode に届く
- kind: technical
- 処置: fixed D22
- 注: **上限と破棄そのものは ST04**（FR-8 / FR-9 / NFR-7）。ここで直したのは
  「全件書き直し」を JSONL 追記に変えた部分だけで、件数の上限は置いていない

## R14. `save()` の catch が `IOException` だけで、encode の失敗は位置取得スレッドを貫通する

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutboxStore.kt
- 根拠: `load()` は `SerializationException` と `IOException` の両方を捕まえるのに `save()` は片方だけ。
  貫通すると `FixCollector.onLocationResult` から投げ、**残りの `locations` は積まれず
  ログも出ないまま部分的に消える**
- kind: technical
- 処置: fixed D22

## R15. `log` ラムダの既定値 `{}` が、失敗報告を丸ごと捨てられるようにしている

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutboxStore.kt
- 根拠: `OutboxStore` の既定を「黙って選ばれるから」と拒んでいるのに、同じ論法が `log` に効いていない。
  `FileOutboxStore(file)` と書けば `outbox_save_failed` も `outbox_unreadable` も存在しなくなる
- kind: technical
- 処置: fixed D22

## R16. この PR が追加した失敗分岐に、テストが 1 本もない

- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/OutboxStoreTest.kt
- 根拠: 新設の失敗経路 4 つ（save の IOException / rename 失敗 / 退避失敗 / `.tmp` 残存）が
  いずれも 0 カバレッジ。**R9 と R10 が実装のまま通ったのはこれが理由**
- kind: technical
- 処置: fixed 10.6

## R17. 電源断に対しては原子的でない（fsync が無い）

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/OutboxStore.kt
- 根拠: `writeText` はページキャッシュに書くだけで `fd.sync()` も親ディレクトリの fsync も無い。
  「途中で落ちても半端なファイルが残らない」はプロセス死には正しいが、電源断には当たらない
- kind: technical
- 処置: rejected: 指摘は正しいが**直さない**。JSONL 追記にしたので落とせるのは書き込み中の 1 件だけで、
  毎分 fsync を呼ぶ電池の代償に見合わない（60 秒に 1 回 = 1 日 1,440 回のフラッシュ）。
  **コメントが保証範囲を実態より広く書いていた点は直した** —— design D22 に「電源断には原子的でない」と
  明記し、deep.md の「確かめたが問わなかったこと」にも残した（黙って決めていない）

## R18. 権限拒否と設定欠如が、利用者にも稼働記録にも何も残さない

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: `SecurityException` → `stopSelf()` も `not_configured` も `Log.w` を 1 行出して黙る。
  **どちらも `core.coverage` に `stopped` を立てない**ので、「データが無いのは何も起きなかったからか、
  収集が止まっていたからか」という稼働記録の存在理由が、まさに必要な場面で失われる
- kind: defer
- 処置: deferred ST02
- 注: `stopped`（意図的な停止）は FR-34 で、`docs/stories/ST02.md` の触れる扉に
  `FR-33, FR-34, FR-9, FR-54` として挙がっている。ST01 が `satisfies` するのは FR-33 の
  「記録を格納したとき稼働を残す」側だけ。**なお、止まったこと自体はログに出るようになった**（10.5）

## R19. DB エラー 1 件がバッチ全体を 500 にし、収集側は永久にリトライする

- 成果物: crates/server/src/lib.rs / crates/server/src/ingest.rs
- 根拠: `results.push(ingest_one(&app, item).await?)` の `?` がバッチ全体を落とす。
  **この PR で `raw` が無検証の `String` になったので新しく到達可能**になった ——
  PostgreSQL の `text` は U+0000 を格納できず（22021）、1 件混ざると同じバッチの全記録が
  永久に送れなくなる。収集側は本文を読めず `unreadable_response` として 1 件も取り除かない
- kind: technical
- 処置: fixed D19
- 注: **到達可能になった経路を塞いだ**（空の原文・U+0000 を格納の前に断る）。
  「DB エラー一般がバッチ全体を落とす」構造そのものは残っている ——
  D14（400 は 1 件も受け付けなかった意味）を変えることになるので、**この Story では触らない**。
  収集側は 5xx を種別つきでログに出すようにした（10.5）

## R20. `Sender` がサーバの拒否理由を捨てている

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt
- 根拠: `IngestResult.error` を一度も読まない。恒久的に拒否される記録は未送信に残り続け、
  5 分ごとに再送され、ログには `send count=5 / accepted count=1` としか出ない。
  **永続化した後は「無言で消える」が「無言で永久に居座る」に変わっただけ**
- kind: technical
- 処置: fixed 10.5

## R21. `202609092315_raw_text.down.sql` の「戻す前の確認」は動かないクエリ

- 成果物: migrations/202609092315_raw_text.down.sql
- 根拠: `raw::jsonb` は JSON でない値に NULL を返さず**例外を投げる**（実測: `invalid input syntax
  for type json`）。件数が出ないうえ、**エラー本文に入力の断片が載って原文がオペレータの端末と
  シェル履歴に出る**（A-2 違反）。さらに 3 文にトランザクションが無く、`ALTER` が落ちると
  `core.event_live` が消えたまま復元されない（`/events` が壊れる）
- kind: technical
- 処置: fixed 10.8

## R22. `check-migrations.sh` が、この破壊的変更を通してしまう

- 成果物: tools/check-migrations.sh
- 根拠: 正規表現が `DROP TABLE|COLUMN|SCHEMA` と `ALTER TABLE ... DROP` だけで、
  0003 の `DROP VIEW` も `ALTER COLUMN ... TYPE` も見ない。down.sql 自身が「戻すと原文が失われる」と
  書いている版が、検査を rc=0 で素通りしていた
- kind: technical
- 処置: fixed 10.8

## R23. `internal()` は「種別だけ」と書いてあるが、DB のメッセージ全文を出している

- 成果物: crates/server/src/lib.rs
- 根拠: `%e` は `sqlx::Error` の Display ＝ `error returned from database: <PostgreSQL の本文>`。
  PostgreSQL は `invalid input syntax for type ...: "<値>"` のように入力値を本文に含める。
  `raw` が無検証の `text` になったぶん、ここから私的データが漏れる筋が太くなっている
- kind: technical
- 処置: fixed D20

## R24. 0003 以降、既に正規化された原文と本物の原文が区別できない

- 成果物: migrations/202609092315_raw_text.sql
- 根拠: 既存行は `jsonb` の正規化済み表現（キー辞書順・重複キー消滅・`1e2` → `100`）で text に落ちる。
  **移行後にその行を見分ける印が何も残らない。** 将来「署名の検証・外部との照合」をやると、
  0003 以前の行だけが理由不明で不一致になり、照合コードのバグと区別が付かない。
  さらに `content_hash` の入力が変わるので、同じ記録を再送すると別の鍵で二重に入る
- kind: irreversible
- 処置: escalated
- 注: **実装者には実データの有無が観測できない**（この作業ツリーの DB は smoke が毎回捨てる）。
  deep.md 第 3 回 R24 に選択肢 4 つを添えて本人へ返した。
  **本人の答え（2026-09-10）は「A. まだ実データは入っていない」** —— 移行の対象が
  1 件も無いので実装は変わらない。印を残す版は不要。8.4 は 0003 を当てたあとにやる

## R25. `fix` のログは「受け取った件数」で、「積めた件数」ではない

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/FixCollector.kt
- 根拠: `add` が全件について永続化に失敗していても `count=N` を出す。
  運用者は「取れているのに送れていない」と読み、原因を網側に誤診する
- kind: technical
- 処置: fixed 10.5

## R26. `raw` に対する最低限の検証が無くなった

- 成果物: crates/server/src/ingest.rs
- 根拠: `raw jsonb NOT NULL` が意図せず保証していた「原文が JSON として妥当」が消え、
  空文字も切り詰められた JSON も無検証で通る。**空の原文は冪等キーを潰し**、
  同じ `event_time` の別々の記録が 1 行に畳まれて `duplicate: true`（＝受理）で返る
- kind: technical
- 処置: fixed D19

## R27. 未送信の全件を 1 要求に載せるので、長い圏外のあと送信が永久に成功しなくなりうる

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt
- 根拠: 1 日圏外 → 1,440 件（0.86 MB）を 1 回の POST に載せ、サーバは 4,300 回のクエリを逐次処理。
  `readTimeout` 15 秒を超えると `Unreachable` → **1 件も取り除かれない** → 5 分後に同じ全件。
  その間に 5 件増えているので**復帰しない**。D9 の括弧「最大 5 件程度」は
  常時オンラインを前提にした記述で、永続化した今は効かない
- kind: technical
- 処置: fixed D23

## R28. Scenario「1 時間以内に届く」の印が、smoke 自身の実行時間を測っているだけ

- 成果物: tools/smoke.sh（手順 18）
- 根拠: `event_time` は手順 11 の `NOW=$(date -u …)`、`ingest_time` はその数秒後の `now()`。
  実測 `最大 0 秒`。遅延の本体（5 分バッチ + 再送）を 1 つも通っていない
- kind: technical
- 処置: fixed 10.11

## R29. Scenario「資格情報が無いと拒否される」と「同じ PC の別プロセスからでも拒否される」が同じ検査

- 成果物: tools/smoke.sh（手順 9）/ specs/record-envelope/spec.md
- 根拠: 2 つの印が同じ curl 3 本に付いており、2 つ目が独立に落ちる条件が存在しない
- kind: technical
- 処置: rejected: **この配備では 2 つの Scenario が同じ事象を指す。**
  S-01 は `BIND=127.0.0.1` の loopback で、外からの到達は Tailscale（PERM-7）が止める。
  したがって「資格情報を付けずに呼ぶ」経路は**同じ PC の別プロセス以外に存在しない** ——
  curl はまさにその別プロセスで、検査は両方の主張を同時に満たしている。
  分けて書きたいなら spec 側の Scenario 分割を見直す話になるが、**PERM-8（第三者製プラグインを
  素通しさせない）を独立に読める形で残す価値のほうが大きい**ので、印はどちらも残す。
  手順 9 には「curl は同じ PC の別プロセス」と根拠を書き足した

## R30. `FixCollectorTest` の端末識別子の印が、コンストラクタ引数の反射になっている

- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/FixCollectorTest.kt
- 根拠: 渡した定数が消えていないことしか見ていない。Scenario 後半「再起動しても変わらない」は
  `DeviceIdTest` が持っている
- kind: technical
- 処置: fixed 10.13

## R31. Scenario「1 件だけの裸の要求も受け取る」が、応答の形を見ていない

- 成果物: tools/smoke.sh（手順 4）
- 根拠: `grep -q '"duplicate":false'` だけなので、**裸のオブジェクトを返す実装**
  （design D12 が明示的に否定した互換応答）でも通る
- kind: technical
- 処置: fixed 10.13

## R32. `IntervalTest` の後半 2 本が恒真で、しかも定数が使われているかを見ていない

- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/IntervalTest.kt
- 根拠: 前 2 本が定数を厳密固定しているので後 2 本は独立に落ちない。
  さらに `LocationService` のリテラルに書き換えても 4 本とも緑のまま
- kind: technical
- 処置: fixed D25

## R33. `HttpTransportTest` の「本文を持ち出さない」がトートロジー

- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/HttpTransportTest.kt
- 根拠: `!kind.contains("35.68")` は、`e.message` に書き換えても
  `ConnectException` の message が "Connection refused" なので落ちない
- kind: technical
- 処置: fixed 10.13

## R34. `MainActivityTest` の `assertFalse(isDestroyed)` が何も見ていない

- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/MainActivityTest.kt
- 根拠: `destroy()` を呼んでいないので必ず false。クラッシュの検出は
  「例外が飛べば試験が落ちる」ことで既に成立している
- kind: technical
- 処置: fixed 10.13

## R35. `OutboxStoreTest` のログ漏洩検査に空振りガードが無い

- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/OutboxStoreTest.kt
- 根拠: `for (line in lines)` だけなので `lines` が空なら無条件に緑。
  同趣旨の `FixCollectorTest` と `TelemetryTest` には `isNotEmpty` がある
- kind: technical
- 処置: fixed 10.13

## R36. Kotlin の `assert(...)` は JVM の `-ea` 依存

- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/{LocationFixTest,IntervalTest}.kt
- 根拠: Gradle の既定では有効だが、`enableAssertions = false` を足された日や
  別のランナーで走らせた日に**無言で消える**（テストは緑のまま）
- kind: technical
- 処置: fixed 10.13

## R37. `/events` 越しの原文の忠実性を誰も見ていない

- 成果物: tools/smoke.sh（手順 22）/ crates/server/src/lib.rs
- 根拠: 手順 19 は DB を直に引いている。`EventRow.raw` が `Value` → `String` に変わったが、
  読み出し口が JSON へ載せ直すときに二重エスケープされないかは無検証
- kind: technical
- 処置: fixed 10.13

## R38. `0003` の「既に text ならスキップ」分岐が 0 カバレッジ

- 成果物: migrations/202609092315_raw_text.sql / tools/check-immutable.sh
- 根拠: smoke は毎回 `docker compose down -v` でまっさらな DB から始め、
  `check-immutable.sh` も各版を 1 回ずつしか当てない。`run()` は起動のたびに全版を当てるので、
  **2 回目の適用**を通る検査が 1 本も無かった
- kind: technical
- 処置: fixed 10.6

## R39. Kotlin と Rust の応答の形が、両側とも Fake 相手にしか確かめられていない

- 成果物: collector-android/.../IngestRequest.kt / crates/server/src/lib.rs
- 根拠: Kotlin の `IngestResult` は全フィールドに既定値があり `ignoreUnknownKeys = true` なので、
  **サーバが `accepted` を改名したら例外も出さずに全件 `false` と解釈し、未送信を永久に取り除かなくなる**。
  テストは全部緑
- kind: technical
- 処置: fixed 10.12

## R40. 400 の応答の形が契約から外れる経路が 2 つあり、どちらも無検証

- 成果物: crates/server/src/lib.rs
- 根拠: 空の配列と非配列の本文は平文を返していた。契約では 400 の本文も結果の配列
- kind: technical
- 処置: fixed 10.13

## R41. Scenario「重複は件数に加えない」の後半が担保されていない

- 成果物: tools/smoke.sh（手順 21）/ specs/collection-coverage/spec.md
- 根拠: 手順 13 の再送は手順 11 と同じ日なので、重複時に coverage の upsert をスキップしても
  手順 21 は緑のまま通る。「重複だけが届いた日」を作る検査が無い
- kind: technical
- 処置: fixed 10.13

## R42. `origin_enum_is_closed` が `authored` / `derived` を確かめていない

- 成果物: crates/server/src/ingest.rs
- 根拠: `ORIGINS` を `["collected"]` に縮めても緑。spec の 3 分類のうち 2 つが無検証
- kind: technical
- 処置: fixed 10.13

## R43. 「収集した記録は書き換えられない」が、0003 で新しく手に入った保証を見ていない

- 成果物: tools/check-immutable.sh
- 根拠: `raw` が `jsonb` だった頃は「JSON として同値なら書き換えを黙って許す」だった
  （`{"hello": "world"}` ← 空白違いが通る）。`text` になった今は拒むが、
  検査は明白に違う値しか投げていないので、**0003 の効き目が回帰から守られていない**
- kind: technical
- 処置: fixed 10.3

## R44. `HttpTransport` が `baseUrl` の末尾スラッシュに弱い

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/HttpTransport.kt
- 根拠: `BuildConfig.BASE_URL` は人間が gradle プロパティで渡す値なので `https://host/` が入りうる。
  `//ingest` が 404 になると、**収集は動き続けるが 1 件も届かない**状態が黙って続く
- kind: technical
- 処置: fixed 10.13

## R45. 未送信の保持に上限が無い

- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Outbox.kt
- 根拠: 圏外 1 日 = 1,440 件、1 週間 = 1 万件。永続化したので日をまたいで積み上がる
- kind: defer
- 処置: deferred ST04
- 注: `docs/stories/ST04.md` が FR-8 / FR-9 / NFR-7（**90 日かつ 2 GB**、破棄した期間と件数を
  稼働記録に残す）で持っている。ST01 の Non-Goals として妥当。
  **1 回の送信の上限は別問題**なので、そちらは D23 で置いた（R27）
