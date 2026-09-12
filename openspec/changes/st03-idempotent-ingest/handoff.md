# ST03 から他の Story への申し送り

**この change では実装しない。** 走っている Story へは差し戻さない（2026-09-12 の規則。
`docs/handoff/README.md`）—— issue ができた時点でその Story の `tasks.md` は凍結。

**2026-09-12 に ST02 側の現物を確かめた。** 5 件のうち **4 件はすでに ST02 に入っている**
（ST02 は ST03 の上流が渡した指摘を取り込んで merge されていた）。残るのは R-e の 1 件だけで、
それを `docs/handoff/ST02.md` に置いてある（下流が読む入口はそちら）。
入っている 4 件は**記録として残す** —— 「渡した」と「入った」は別の事実で、
次に読む人が確かめ直せる根拠がここに要る。

## ST02 R-a — 稼働状況の状態を 7 → 8 にする

- 何を: `specs/collection-coverage/spec.md` が「7 つのいずれか」のままの箇所を 8 にする
- なぜ今入れないか: ST02 は issue #19 で凍結。走っている Story へ差し戻すと往復が生まれる
  （実測: ST03 の上流が ST02 の下流に 5 件を差し戻し、12 時間で 5 往復した）
- いつ・どこで: ST02 の merge 後に `fix/coverage-eight-states` で。担当は見つけた側（ST03）
- 根拠: `openspec/changes/st03-idempotent-ingest/review/deep-r5.md` R55
- **いまの状態: 入っている（2026-09-12 確認）。** `specs/collection-coverage/spec.md:362,364` が
  「8 つのいずれか」になっており、`crates/server/src/coverage.rs` の `DayState` も 8 状態を持つ
  （⑧「退役」のコメントつき）。**追加の作業は無い**

## ST02 R-b — `retired_on` は日付で持つ

- 何を: 真偽値ではなく `date`。**退役より前に起きた本物の途絶が遡って消える**のを防ぐ
- なぜ今入れないか: 同上（凍結）
- **いまの状態: 入っている（2026-09-12 確認）。** `202609112113_source_lifecycle.sql` が
  `retired_on date` で作り、コメントに R56 の理由を書いている。
  `specs/…/spec.md` も「退役した日を**日付**として持たせ、真偽値では持たせない」と書いている。
  **追加の作業は無い**
- 根拠: `review/deep-r5.md` R56

## ST02 R-c — 「① 記録あり」を `core.event` から引く

- 何を: `coverage.rs` の「記録あり」の判定を `coverage.event_count > 0` ではなく
  `core.event` から引く
- なぜ今入れないか: 同上（凍結）
- **順序が効く**: ST03 の更新経路（Q1）が main に入ると、**出来事の時刻が別の日へ動いたとき
  記録の無い日が「記録あり」になり、記録のある日が「途絶」になる**（実測）
- **いまの状態: 入っている（2026-09-12 確認）。** `coverage.rs:568-578` が
  `core.event` を `count(*)` で数えており、コメントに
  「`core.coverage.event_count` で決めていたときは、ST03 が外部サービスからの更新で
  出来事の時刻を動かすと**記録の無い日が「記録あり」・記録のある日が「途絶」になる**」と
  ST03 の実測を引いている。`202609112113_source_lifecycle.sql` が `event_by_source_time` 索引も
  足している。**追加の作業は無い** —— この Story の実装が main に入っても倒れない
- 根拠: `review/deep-r5.md` R57

## ST02 R-d — 退役したソースの格子を既定で畳む

- 何を: 退役したソースの格子を既定で畳む（または末尾に置く）。Must の 5 ソースが
  1 画面から押し出されるため
- なぜ今入れないか: 同上（凍結）。画面は `collection-coverage` の担当で ST03 は触らない
- いつ・どこで: ST02 の merge 後
- 根拠: `review/deep-r5.md` R63
- **いまの状態: 入っている（2026-09-12 確認）。** `web/src/__tests__/retired-source.test.tsx` に
  `Scenario: 退役したソースは後ろで畳まれている` の印があり、画面側の試験が立っている。
  **追加の作業は無い**

## ST02 R-e — 更新だけが届いた日を稼働記録でどう数えるか

- 何を: ST03 が更新の経路を開けたので、正典の「**新しく入った記録だけを数える**」に穴が開く
  —— 外部サービス側の更新だけが届いた日は `event_count` が 0 のまま
- **ST03 側の扱い**: `lib.rs` は**更新を 0 件として数える**（正典どおり）。
  ただし**行は立てる**ので、その日が⑥「途絶」には見えない
  （`core.coverage` に行があり、ST02 は「① 記録あり」を `core.event` から引く）
- なぜ今入れないか: 数え方の正典は `collection-coverage` が持つ。ST03 が変えると
  NFR-13 の分子が動く
- **いまの状態: 残っている（2026-09-12 確認）。** `coverage.rs:795` の
  `if facts.event_count > 0 { Recorded }` は `core.event` の実数から引いているので、
  **更新だけが届いた日も「① 記録あり」になる**（出来事の時刻がその日にある限り）——
  つまり画面は倒れない。倒れるのは **NFR-13 の達成日数を `core.coverage.event_count` から
  数える経路がもし残っていた場合**だけで、そこは ST02 の担当。
  `docs/handoff/ST02.md` に置いた **5 件のうち唯一の未処置**
- いつ・どこで: ST02 の merge 後、必要なら `fix/` で。**数え方を変えるなら本人に問う**
  （達成日数が動くため）
- 根拠: `tasks.md` 13.2 (e) / `deep.md`「あわせて、`lib.rs:224` は…」

## ST12 / ST13（外部サービスの取り込み）R-f — 収集側の識別子は毎回新しく振る

- 何を: 外部サービスから取り込むとき、**収集側の `id` を外部の識別子から導かない**。
  取得のたびに新しく振る
- なぜ: 導くと、更新された記録が「同じ `id`・違う内容」として届き、
  **FR-22 の「更新する」と「同じ識別子で中身が違えば断る」が同じ到着に逆を指す**（深掘り Q14）。
  ST03 は後者を `id_reused`（400）として実装しているので、導くと**正常な更新が断られる**
- いつ・どこで: ST12 / ST13 の上流（deep の前提として渡す）。どちらもまだ `tasks.md` が無いので
  `deferred` でよい
- 根拠: `deep.md` 第 2 回 Q14 / `docs/collector-contract.md` の `id_reused` の節

## ST02 R-g — `core.coverage` に削除・切り詰めの門が無い

- 何を: `core.coverage` と `core.coverage_span` に、`UPDATE` / `DELETE` / `TRUNCATE` の門を置く
  （`core.reject_truncate()` は既にあるので `TRUNCATE` は 2 行で足せる）
- なぜ: `lib.rs` のコメント自身が「**その日の稼働記録は二度と戻らない**（引き直す経路が無い）」と
  書いている帳簿に、門が 1 つも無い。実測で `TRUNCATE core.coverage` が rc=0 で通る
- **ST03 で入れない理由**: `core.coverage` は `collection-coverage`（ST02）の表で、
  門を置くと **ST02 のコードとテストが何をしてよいかが変わる**。
  他の capability の表に錠を掛けるのは、その capability の spec が決めること
- いつ・どこで: ST02 の merge 後に `fix/coverage-gates` で。担当は見つけた側（ST03）
- 根拠: 実装レビュー R100 / MEDIUM-17（`review/code.md`）

## ST23（本文を本当に消す）R-h — 消去は `deleted_at` も立てる

- 何を: 消去（`raw=''`）した記録に、**論理削除の印も同じまとまりで立てる**
- なぜ: いまの門は「内容が変わる書き換え」に履歴を要求するので、
  **消去済み（`raw=''`・`deleted_at IS NULL`）の行に外部サービスからの更新が届くと、
  履歴を書けば `raw` が埋め戻る**。消したはずの本文がその経路で戻る
- **ST03 で入れない理由**: 消す操作そのものは ST23 の担当で、ST03 が作るのは台帳と門だけ
  （design の Non-Goals）。いまは消す経路が実装されていないので到達しない
- いつ・どこで: ST23 の上流（deep の前提として渡す）。まだ `tasks.md` が無いので `deferred`
- 根拠: 実装レビュー（`review/code.md` の閾値未満の欄）

## ST12 / ST13 R-i — 外部から取り込むときは更新時刻を必ず送る

- 何を: `source_updated_at` を毎回載せる（外部サービスが返さないなら、取得の時刻でよい）
- なぜ: **更新時刻を持たない到着は「届いた順」で当たる**（Q20 の答え）。載せないと、
  応答を取り落として古い本文を再送しただけで**内容が過去へ戻る**
  （実測: `resend_after_update_is_not_id_reuse`。前の版は履歴に残るので**失われはしない**が、
  最新の 1 行が古い内容になる）
- いつ・どこで: ST12 / ST13 の上流。まだ `tasks.md` が無いので `deferred`
- 根拠: `deep.md` Q20 / `crates/server/src/dedup_tests.rs` の `resend_after_update_is_not_id_reuse`

## ST28 / ST29（守り）R-j — アプリ用の非所有者ロールを作る

- 何を: DB の役割を分ける（アプリは superuser でも表の所有者でもない）
- なぜ: **門は同じ役割から 1 文で外せる**（実測）——
  `SET session_replication_role='replica'` / `ALTER TABLE … DISABLE TRIGGER ALL`。
  spec は「これらの制限を、取り込み口の外から加えられた操作にも適用する」と書き、
  0002 / 0004 / gates.sql のコメントは脅威として「同じ PC の第三者製プラグイン（PERM-8）や
  psql を直に叩く運用」を名指ししている —— **その主体がこの 1 文を打てる**
- **ST03 で入れない理由**: 塞ぐには移行 1 本に加えて `tools/*.sh` と CI の接続文字列が全部動く。
  ST29（FR-77）がプラグインを別プロセスの HTTP に閉じているので実務上の露出は小さい
- **どの Story の担当でもない**（`docs/stories/INDEX.md` に DB の役割分離を持つ Story が無い。
  ST28 は網、ST29 はプラグインの宣言と承認）。**入口を作るところから要る**
- 根拠: 実装レビュー R103（`review/code.md`）
