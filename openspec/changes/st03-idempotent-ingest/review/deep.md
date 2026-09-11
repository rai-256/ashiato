# ST03 深掘りの独立レビュー

対象: `openspec/changes/st03-idempotent-ingest/deep-questions.json`（Q1〜Q5）。
`openspec/schemas/ashiato/schema.yaml` の `deep` 手順 1〜5 を、問いの一覧を開く前に独立にやり直し、
そのうえで突き合わせた差分だけを書く。**問いの JSON は 1 文字も触っていない。**

実施日: 2026-09-11。ブランチ `docs/st03-upstream`（clean, `e685f7c`）。

## 実測の方法

`migrations/0001`〜`0004` を使い捨ての `postgres:17-alpine`（別ポート 55439。ST02 の走っている
DB には触っていない）に当て、`ingest_one` が撃つのと同じ SQL を手で流した。以下「実験 N」はこれ。

| | 流したもの | 結果 |
|---|---|---|
| 実験 1 | 同じ `external_id`・違う `content_hash` を `ON CONFLICT (logical_source, content_hash) DO NOTHING` で | `ERROR: duplicate key ... "event_dedup_ext"` |
| 実験 2 | 同じ `id`・違う `content_hash` | `ERROR: duplicate key ... "event_pkey"` |
| 実験 3 | 別 `user_id`・同じ `content_hash` | `INSERT 0 0`（黙って消える） |
| 実験 4 | 同じ `content_hash`・違う `external_id` | `INSERT 0 0`。残った行の `external_id` は先着のまま |
| 実験 5 | `collected` の行の `raw` / `content_hash` を UPDATE | トリガが `RAISE`（0004） |
| 実験 6 | `deleted_at` を立てた行と同じ `content_hash` を再投入 | `INSERT 0 0`、`event_live` は 0 行 |
| 実験 7 | `external_id = ''` の 2 件（内容は別） | 2 件目が `ERROR: ... "event_dedup_ext"` |
| 実験 8 | `external_id IS NULL` を複数 | 何件でも入る（部分索引） |
| 実験 9 | 同じ派生を作り直した 2 版（`origin='derived'`、原文が違う） | **2 行になる** |

## 手順ごとの記録

- **手順 1（要件どうしの衝突）**: FR-22 / FR-23 に、扉 #12 の関与要件と、
  FR-18 / FR-19 / FR-21 / FR-24 / FR-25 / FR-29 / FR-30 / FR-31 / FR-50 / FR-51 / FR-61 を
  突き合わせた。FR-22 × FR-30（Q1）、FR-22 × FR-21（Q5）、FR-22 × FR-50（Q3）は一覧にある。
  **FR-22 × FR-31 が一覧に無い** → R2。FR-17（過去のエクスポートの重複検出）は
  同じ出来事が別の原文で届く経路だが、ST03 の `satisfies` に無いので問わないと判断した。
- **手順 2（扉の幅）**: 扉 #12 は「持つ」としか決めていないのに、実装は
  `event_dedup_ext` で「ソース内で一意」まで踏み込んでいる（`migrations/202609081618_envelope.sql:37-38`）。
  幅は 2 方向あり、片方（同じ識別子・違う内容）は Q1、**もう片方（同じ内容・違う識別子）が
  一覧に無い** → R1。扉 #7 / #9 の幅は Q1 / Q2 が拾っている。
- **手順 3（新たに立つ一方通行）**: 鍵の入力（Q2）、更新で失われる前の版（Q1）、
  断った記録（Q4 / Q5）、消した記録（Q3）は一覧にある。
  **「更新した事実と時刻」を残す列が無いこと**が選択肢に書かれていない → R3。
  鍵の作り方そのものの不可逆性は、原文が残っている以上 Q2 の `why` ほど固くない → R5。
- **手順 4（日常に影響する選択）**: 容量（Q1）は選択肢に書かれている。
  **断られた記録が端末の未送信に残り 5 分ごとに送られ続ける**こと（電池・通信・
  200 件で送信が止まる）が Q4 / Q5 のどちらにも書かれていない → R8。
  通知（FR-35）への影響は、重複だけが届いた日も稼働記録の行は立つので無いと確かめた
  （`crates/server/src/lib.rs:217-227`）。
- **手順 5（既存コードが要件を満たしていない箇所）**: FR-22 の「既存の記録を更新し」は
  `DO NOTHING` で未達（`lib.rs:190`）、FR-22 の「同一の識別子」は判定に使われていない、
  FR-23 は強制されていない —— この 3 件は Q1 / Q5 / Q4 が扱っている。
  一覧に無い実装欠陥が 3 件 → R11 / R12 / R13。
- **不要な問い**: **該当なし。** 5 問すべてについて、要件・扉・既存コードのいずれも
  答えを決めていないことを確かめた（扉 #12 は「持つ」までしか決めておらず、
  Q4 の「持たない記録を拒むか」は開いている）。
- **分類違い**: **該当なし。** Q1 / Q5 の `conflict`、Q2 / Q3 / Q4 の `irreversible` は
  いずれも実態と合っている（Q3・Q4 には日常の代償もあるが、選択の性質は不可逆で正しい。
  代償が書かれていない件は R8 で別に挙げる）。

---

## R1. 「同じ内容・違う外部識別子」で 2 件目が黙って消える —— 鍵が 2 本あるときの優先順位が問われていない

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: 実験 4（`INSERT 0 0`。残った行の `external_id` は先着の `ext-1` のまま）/ migrations/202609081618_envelope.sql:37-40（索引が 2 本ある）/ crates/server/src/lib.rs:190（仲裁は `content_hash` の索引だけ）/ docs/requirements.md:663-666（扉 #12「これが無いと同一物の更新を追えない」）
- kind: irreversible
- 処置: escalated — Q6。「同じ内容・違う外部識別子」を新しい問いとして立てた（どちらの鍵を正とするか。3 択）。Q1 の逆向きであることを両方の context に書いた
- 提案: Q1 は「外部識別子が同じ・内容が違う」向きだけを問うている。逆向き（内容が同じ・外部識別子が違う。外部サービスが識別子を振り直す / 同じ内容を 2 件出す）を 1 問足すか、Q1 の question を両方向にして選択肢に「どちらの鍵を正とするか」を入れる。いまの振る舞いは 2 件目が受理として捨てられ、生き残った行は古い外部識別子を持つので、扉 #12 の目的（同一物の更新を追う）がその行だけ失われる。

## R2. 派生を作り直すと行が増える —— FR-31 と FR-22 の衝突が一覧に無い

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: docs/requirements.md:143（FR-31「派生を作り直す」）と docs/requirements.md:131（FR-22「新しい行を作らない」）/ 実験 9（`origin='derived'` の 2 版が 2 行になった）/ crates/server/src/ingest.rs:100-116（鍵は `logical_source` + `event_time` + `raw` のみで、作り直すと必ず別の鍵になる）
- kind: conflict
- 処置: escalated — Q7。派生の作り直しを ST03 で決めるか ST16 に送るかを問う 1 問を立てた。**範囲を AI が勝手に閉じない** —— 「送る」を既定に置いたうえで本人に選ばせる
- 提案: FR-22 は「すべての記録」に掛かり、`ORIGINS` は `derived` を含む（ingest.rs:13）。作り直しのたびに古い版が残って積み上がる（FR-76 の滞在が該当）。1 問足す（作り直した派生の古い版をどうするか）か、context に「この Story は `collected` だけを対象にし、派生の作り直しは ST<NN> で決める」と範囲を明示して閉じる。いまは**どちらとも書かれていない**。

## R3. 「更新する」を選んでも、更新した事実と時刻がどこにも残らない

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: docs/requirements.md:125-126（FR-19 は「起きた時刻」と「入った時刻」の 2 本だけ）/ migrations/202609081618_envelope.sql:14-35（`updated_at` に相当する列は無い）/ migrations/202609100000_immutable_origin.sql（`ingest_time` も凍結対象）/ 実験 5
- kind: irreversible
- 処置: escalated — Q1。context に「更新された時刻を持つ欄が無く、入った時刻は 0004 が凍結している」を足し、選択肢 1 に「履歴が時刻を持つので自動的に満たす」、選択肢 2 の irreversible に「更新した事実と時刻も残らない」を書いた
- 提案: Q1 の選択肢 1・2 の `irreversible` に「更新した事実と時刻を残す列はいま無い」を足す。列は後から足せるが、**足す前に起きた更新の時刻は復元できない**（型・列は製造準備で決まっていて、論点に挙がらないまま固まる筋 —— `raw` を `jsonb` にしていたのと同じ形）。選択肢 1（履歴を別に残す）だけはこれを自動的に満たす、という違いが本人に見えていない。

## R4. Q2 の答えが ST02 の生存信号の冪等キーにも当たることが、context に書かれていない

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: openspec/changes/st02-collection-coverage/design.md:82-85（`core.heartbeat` に `content_hash` と `CREATE UNIQUE INDEX heartbeat_dedup ON core.heartbeat (logical_source, content_hash)` ——**利用者を含まない同じ形**）/ 同 specs/collection-coverage/spec.md:152-160（FR-78 が記録と同じ冪等キーを要求）/ ST02 は未 archive で、まだ直せる
- kind: premise
- 処置: fixed deep-questions.json — Q2 の context に「ST02 が生存信号にも同じ形の鍵（利用者を含まない）を置く設計で、まだ merge されていない」を足し、選択肢 1 を「索引 2 本と ST02 側にも同じく足す」に書き換えた
- 提案: Q2 の context に「同じ形の鍵を ST02 が `core.heartbeat` にも置く（未実装）」を足す。足さないと、記録側だけ利用者を含む鍵になり、生存信号側に同じ穴が残る。どちらが先に merge されても片方だけが直る形になっている。

## R5. Q2 の why「変えられるのは今だけ」は言い過ぎ —— 原文が残っているので鍵は当て直せる

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: migrations/202609081618_envelope.sql:31 + 202609092315_raw_text.sql（`raw` は `text` で全行に残る）→ 保存済みの全行について新しい鍵を再計算できる / ただし migrations/202609100000_immutable_origin.sql が `content_hash` を凍結しており、実験 5 のとおり UPDATE は `RAISE` で落ちる（＝再計算にはトリガを外す移行が要る）
- kind: premise
- 処置: fixed deep-questions.json — Q2 の why から「変えられるのは今だけ」を外し、「原文が残っているので計算し直せるが、凍結を外す移行が要る」に直した
- 提案: 「1 年動かしてから変えると過去分と新規分が別物になる」は正しいが、「変えられるのは実装が始まる前の今だけ」は誤り。正確には**移行でトリガを外して全行を再計算する手順が要る**（原文があるので値は作れる）。不可逆の度合いが違うと、本人が選択肢 2（いまのまま）を選ぶ動機が変わる。context を直すか、選択肢に「後から当て直す移行を書く」を足す。

## R6. Q2 の context が索引を 1 本しか挙げていない。別利用者の同一内容は今日すでに黙って消える

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: migrations/202609081618_envelope.sql:37-40（索引は `event_dedup_hash` と `event_dedup_ext` の 2 本。**どちらも `user_id` を含まない**）/ 実験 3（別 `user_id`・同じ内容 → `INSERT 0 0`）/ crates/server/src/lib.rs:230-233（その場合の応答は `accepted: true` / `duplicate: true`）
- kind: premise
- 処置: fixed deep-questions.json — Q2 の context を索引 2 本ぶんに直し、「別の利用者が同じ内容を送ると 1 行に畳まれ、受理として返る」を実測として足した
- 提案: context の「索引も (論理ソース, 冪等キー) で、利用者を含みません」を 2 本ぶんに直し、「いまは別利用者の同じ内容が黙って畳まれ、受理として返る」を足す。選択肢 1（利用者識別子を足す）を選んだとき `event_dedup_ext` も直すのかがこのままでは決まらず、穴が半分残る。

## R7. Q2 選択肢 3 の代償の説明が、いまの収集側と食い違う

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationFix.kt:63,78（原文そのものに `device_id` が入っている）→ 端末識別子が変われば**いまの鍵でも**別の鍵になる / 同 LocationFixTest.kt:33-45（原文の文字列が固定値で pin されており、`device_id` を含む）/ OutboxStore.kt:41-45（未送信は端末上のファイル。入れ直せば消える）
- kind: premise
- 処置: fixed deep-questions.json — Q2 選択肢 3 の detail を「いまの唯一の収集側では選択肢 1 と差が出ない（原文に端末識別子が入っているため）。差が出るのは原文に端末を含めない将来のソースだけ」に書き直し、選択肢 1 からも「2 台が畳まれる」の記述を外した
- 提案: 選択肢 3 の detail「アプリを入れ直すと端末識別子が変わるので、入れ直し前の未送信を送り直したときに全部が新しい行として入る」は、いまの唯一の収集側では選択肢の差にならない（原文が既に端末識別子を含むため、選択肢 1・2 でも同じことが起きる）。差が出るのは**原文に端末を含めない将来のソース**だけ、と書き直す。同じ理由で選択肢 1 の「2 台の端末が同時刻に同一の原文を送ると 1 行に畳まれる」も、位置ソースでは起こらない。

## R8. Q5 選択肢 1・Q4 選択肢 1 の「その 1 件だけを未送信から外せる」は誤り

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: collector-android/.../Sender.kt:92（`batch.filterIndexed { results[i].accepted }` —— **断られた分は返さない**）/ 同 Sender.kt:48（`outbox.remove(accepted)` で消えるのは受理された分だけ）/ 同 Sender.kt:88-91 のコメント「恒久的に断られる記録は未送信に居座り、5 分ごとに送られ続ける」/ LocationFix.kt:30（`MAX_BATCH = 200`。1 回に載るのは先頭 200 件）
- kind: premise
- 処置: fixed deep-questions.json — Q4 選択肢 1 と Q5 選択肢 1 に「断られた記録は未送信から取り除かれず 5 分ごとに送られ続け、200 件たまると送信が止まる」と「収集側を諦める作りにするのが ST03 の作業に入る」を足した。手順 4（日常）の項目はこれ 1 件
- 提案: 「断る」を選んだときの日常の代償（電池・通信・断られた記録が 200 件たまると送信が完全に止まる）を Q5 選択肢 1 と Q4 選択肢 1 に書く。**手順 4 で挙がるべき唯一の項目がここで、いまはどの問いにも出ていない。** 収集側が断られた記録を捨てるように直すなら、それは ST03 の tasks に入る作業なので、選択肢の detail ではなくその旨を書く。

## R9. Q3 選択肢 3・Q4 選択肢 2 の「稼働記録に残す」は、置き場が無くなる

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: openspec/changes/st02-collection-coverage/design.md D2（`core.coverage` を `(user_id, logical_source, day, event_count)` に作り直し、**`state` を落とす**。`note` 列も残らない）/ migrations/202609081618_envelope.sql:44-51（いまの `note` 列）/ ST02 tasks 1.1（`202609111111_coverage_rebuild.sql`）
- kind: premise
- 処置: fixed deep-questions.json — Q3 選択肢 3 と Q4 選択肢 2 を「跡を書く場所はいま決まっていない（ST02 が稼働記録を件数だけの表に作り直す）。別の表かログを新しく用意することになる」に書き直した
- 提案: 両方の選択肢の context に「ST02 が稼働記録を件数だけの表に作り直すので、跡を書く欄はいま無い」と書くか、選択肢を「別の跡（ログ、または別表）に残す」と言い換える。いまの文面だと、本人は「1 行足すだけ」に見える選択肢を選ぶことになる。

## R10. Q1 と Q5 は独立に答えられない組み合わせがある

- 成果物: openspec/changes/st03-idempotent-ingest/deep-questions.json
- 根拠: Q1 選択肢 4「最初に届いたものを正とし、更新は取り込まない」と Q5 選択肢 2「要件どおり既存を更新する」は、同じ行に対して逆の動作を指す / Q5 選択肢 2 の detail が「Q1 で履歴を残す側を選べば」と Q1 への依存を既に認めている
- kind: premise
- 処置: fixed deep-questions.json — Q5 の context に「Q1 で更新を取り込まない側を選んだ場合、選択肢 2 は選べない」を書き、Q1 選択肢 4 と Q5 選択肢 2 の detail にも同じ依存を明記した
- 提案: Q5 の `why` か `context` に「Q1 で更新を取り込まない側を選んだ場合、Q5 の選択肢 2 は選べない」と依存を明記する。`ask_wizard.py` は条件分岐を持たないので、文面で示すしかない。

## R11. 重複のとき、格納されている行ではなく送り主の識別子を返している（問いではなく欠陥）

- 成果物: crates/server/src/lib.rs
- 根拠: crates/server/src/lib.rs:230-233（`None => IngestResult::stored(req.id, true)`）/ docs/collector-contract.md:63-64（`id` = 「格納された記録の識別子」）/ 実験 3・実験 4（別の識別子で同じ内容が届くと、**DB に存在しない識別子**を受理として返す）
- kind: technical
- 処置: fixed deep.md — 「上流で見つかった既存実装の欠陥」として記録し、tasks で直す。返す識別子は Q1 の答え（更新するか否か）で形が決まるので、答えが揃ってから design に落とす
- 提案: 問いにしない。tasks で直す。`DO UPDATE ... RETURNING id`（Q1 の答え次第）か、重複時に既存行を引いて既存の識別子を返す。いま直さないと、ST03 が「重複だった」と返した先の識別子で後から記録を指せない。

## R12. `external_id` の空文字を受け口が弾かず、2 件目で 500 になる（問いではなく欠陥）

- 成果物: crates/server/src/ingest.rs
- 根拠: 実験 7（`external_id = ''` の 2 件目が `duplicate key ... event_dedup_ext`）/ 実験 8（`NULL` は部分索引の対象外で何件でも入る）/ crates/server/src/ingest.rs:71-87（`validate` が見るのは `origin` / `raw` / `device_id` だけ）
- kind: technical
- 処置: fixed deep.md — 同上。`external_id` が空文字なら格納の前に断る。ST01 が `device_id` の空文字で踏んだのと同型で、人間に問う幅が無い
- 提案: 問いにしない。tasks で直す。`external_id` が `Some` なら非空であることを `validate` で断り、`docs/collector-contract.md` の `error` 表に 1 行足す。ST01 が `device_id` の空文字で踏んだのと同型で（ingest.rs:83-85）、格納の前に断らないと 500 になり、まとめ送り全体が止まる。

## R13. 取り込みと稼働記録の書き込みが別トランザクションになっている（問いではなく欠陥）

- 成果物: crates/server/src/lib.rs
- 根拠: crates/server/src/lib.rs:184-228（`INSERT ... RETURNING` と `INSERT ... ON CONFLICT DO UPDATE` を `BEGIN` 無しで別々に撃っている）
- kind: technical
- 処置: fixed deep.md — 同上。取り込みと稼働記録を 1 トランザクションに束ねる。Q1 で「更新 + 履歴」が選ばれると 3 文になるので、更新経路を作る前に束ねる
- 提案: 問いにしない。design で 1 トランザクションに束ねる。Q1 で「更新 + 履歴」を選ぶと 3 文（本表の更新・履歴の挿入・稼働記録）になり、原子性が無いと「履歴だけ残って本表が古い」が起きる。ST03 が更新経路を作る前に束ねるのがいちばん安い。
