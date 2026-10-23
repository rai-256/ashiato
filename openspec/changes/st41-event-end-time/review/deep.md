# ST41 deep-questions.json の独立検査（deep-review）

問いの一覧を見る前に、手順 1〜8 を `docs/stories/ST41.md`・`docs/handoff/ST41.md`・`docs/requirements.md`（FR-18〜FR-23 / FR-30 / FR-50 / FR-51 / FR-64 / FR-89 / FR-91 / FR-99 / FR-103 / NFR-24 / 扉 #28）・
`crates/server/src/ingest.rs`・`crates/server/src/lib.rs`（`ingest_one` / `apply_external_update` / `advance_watermark` / `events`）・`migrations/202609120942_dedup_indexes.sql`・
`migrations/202609120943_version_and_ledger.sql`・`migrations/202609120944_gates.sql`・`migrations/202609271716_deletion_ledger.sql`・`crates/server/src/export/excluded.rs`・`crates/server/src/export.rs`・
`crates/server/src/stay_store.rs`・ST33 design D17・`openspec/changes/ch02-export-next-record-window/`・ST39 / ST43 / ST45 の Story から自分でやり直し、その後で note と Q1〜Q6 に突き合わせた。

## 手順の記録（指摘ではない）

- 手順 1（要件どうしの衝突）: FR-91（終わりを持つ）と FR-22 / FR-50（「内容」で同一を判定し、削除済みの「同じ内容」を入れない）が、**終わりは「内容」に入るか**で逆を指しうる → R1。
  FR-91 と FR-30 / FR-51（収集した記録の錠・消去の形）→ R4。FR-91 と FR-99（衛星の記録にも FR-20）→ R5
- 手順 2（扉の幅）: 扉 #28 の「持つ」自体に幅は無い。幅があるのは、(4) の「同じ外部の識別子で送り直す」が**記録ごとの外部識別子を持つソース**に限られる点 → R3。(3) の「出来事の時刻と同じ錠」が終わりの TZ の 2 列に及ぶか → R4
- 手順 3（新たに立つ一方通行）: 終わりを入れる鍵の式（`content_hash` の入力）→ R1。逆順の終わりを拒むか → Q1 / R2。前の版の履歴の列と門の照合 → R4。
  列の型（`timestamptz` と分の `integer`）は出来事の時刻と同じで、失われるもの（µs 未満の桁）は始まりと同じ。索引は NFR-24 で変更に数えず、作り直せる
- 手順 4（日常に影響する選択）: 進行中の記録が点と期間の 2 行で残る → R3。時計の飛んだ端末の何年もの幅が毎日の期間の問い合わせに出る → R7。起動時の索引の作成 → R8
- 手順 5（既存コードが要件を満たしていない箇所）: FR-91 は未実装（`grep -rln "end_time\|end_at" migrations/*.sql` は `deletion_ledger.sql` の `end_at`（ビューの別名）だけ）。
  FR-20 は「持たせる」だけで、一般の記録のずれ・TZ の値は検査していない（`ingest.rs:130-160` の `validate` に無く、範囲の検査は主張と場所の記録だけ `lib.rs:1042` / `lib.rs:1100`）→ R5。
  `core.event_live` は `SELECT *` で作られ（`envelope.sql:44`）、列を足したら作り直さないと出ない（`event_columns.sql:22` が同じ手当てをした）。書き出しは分類の無い列があると Catalog の段で止まる（`export.rs:890`）→ R6
- 手順 6（過去の取りこぼし）: 当てはまったのは 4 つ。「書き直せない記録の形は固定バイト列の試験で凍結する」（ST33 Q12）→ R1、「時計が大きく進む／戻る 1 件」（ST02 Q29・ST08 Q7）→ R7、
  「推奨に『失うものが無い』と書く前に、時刻・版・鍵を使う箇所をすべて辿る」（ST08 Q11）→ R1・R3（`advance_watermark` の経路）、「初回・最大量の見積もり」（ST08 Q9）→ R8。
  申し送り（`docs/handoff/ST41.md` の C7）は note の (7) が受けている。ただし前の版の側の読み口が抜けている → R6
- 手順 7（族の網羅）: Q1 の族（終わりの値がおかしい）に、時計の飛び（R7）と、ずれ・TZ の値の範囲外（R5）が無い。(4) の族（進行中→終わり）に、外部識別子の無いソース・終わりが消える更新・終わらない進行中が無い（R3）。
  (3) の族（錠の例外）に、FR-30 (b) の消去が無い（R4）。(7) の族（終わりの読み口）に、前の版の 2 か所と書き出しの列の分類が無い（R6）
- 手順 8（前提の確かさ）: premises.md の 3 行はどれも確かさ 4 で、使い捨ての PostgreSQL 17 で走らせた記録と合う。ただし Q1 の why が 2 行目から引いた推論「索引の式が逆順で落ちるので終わりを使わない形で入れることになる」は前提から出ない（R2。手元で再現した）。
  索引を作る時間（本物の約 120 万行・起動時の `migrate()`）は測っていない（R8）
- 衛星（ST39 / ST43、NFR-24）について（呼び出し元の (c)）: 後で困るのは R1（鍵の式の凍結）・R3（進行中の 2 行）・R4（前の版の履歴に終わりが無いと、FR-64 の「変更は FR-22 の経路だけ」で前の終わりが消える）。
  取り込み口の欄の名前は、FR-103 の衛星向けの契約が公開されると FR-89 / FR-95 で 6 か月並べることになるが、失うものは無い（作り直しの手間だけ）ので問いにしない
- 不要な問い: 無し。Q2〜Q6 はどれも要件や扉で決まっておらず、`recommended` がある。Q4 は `docs/stories/INDEX.md:108`（C34「日の境界のタイムゾーンは ST41 の上流で決める」）が置いた
- 確かめた context の記述: Q3「`GET /events` は全件を並べるだけ」（`lib.rs:3371`）・読み出しの記録はすべての口に掛かる（`lib.rs:3463` の `access_log::middleware`）・Q4 の `coverage.rs:14` / FR-41 / `docs/production-prep.md:51`・
  Q1 の `core.coverage_span` / `core.drop_report` の逆順の CHECK（`coverage_rebuild.sql:100` / `drop_reports.sql:29`。どちらも `<` で、同じ時刻も断っている）・Q6 の 1 日の並びの `range_end`（`day_records.rs:227`）は、どれもコードと合っていた

## R1. note (5)「終わりのある記録だけ内容のハッシュに終わりを混ぜる」は「どれを選んでも同じ」ではない —— 混ぜなければ終わりの訂正が捨てられ、混ぜれば削除済みの止めを終わりだけ違う到着がすり抜ける。式も凍結される
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json
- 根拠: 混ぜない側: `lib.rs:1371` —— 更新の経路（`apply_external_update`）に入るのは `stored.content_hash != hash && external_id.is_some()` のときだけで、原文と始まりが同じなら `advance_watermark`（`lib.rs:1540-1558`。更新時刻しか書かない）に落ち、終わりは書かれない。外部識別子の無い記録は `ON CONFLICT (user_id, logical_source, content_hash) … DO NOTHING`（`lib.rs:1321`）で畳まれ、新しい終わりは `duplicate: true` で捨てられる。終わりは原文と別の欄（note (2)）なので、原文が変わる保証は無い。
  混ぜる側: FR-50「削除済みの記録には、同じ内容の再送も外部サービスからの更新も取り込まない」（`docs/requirements.md:647`）を、外部識別子の無い記録では `event_dedup_hash`（削除済みも含む一意索引。`dedup_indexes.sql:46-47`）が、外部識別子の違う記録では `needs_deleted_check` の `content_hash` の照会（`lib.rs:1275-1286`）が担っている。終わりを鍵に混ぜると、**本人が消した記録が、終わりだけ違う到着で生きた行として戻り**、書き出し（ST33）へ出る。
  凍結: 収集した記録の `content_hash` は門（`gates.sql:86`）で履歴なしに動かせず、式を後で変えると保存済みの行が再送と一致しなくなる。`ingest.rs:269` の `hash_is_pinned` は 3 欄の式しか固定していない（「他言語から同じ鍵を再現できることの確認も兼ねる」`ingest.rs:268`）。
  `python3 scripts/upstream_misses.py report` の「書き直せない記録の形は固定バイト列の試験と読めない形の出し方を問いにする」（ST33 Q12）と「『失うものが無い』と書く前に鍵を使う箇所を辿る」（ST08 Q11）に当たる
- kind: irreversible
- loss: discarded
- 提案: (5) を note の C から外す。両側に失うものがある（混ぜない側 `discarded`、混ぜる側は削除済みの記録が書き出しへ出る `exported`）ので A の問いにする。失わない第 3 の選択肢（鍵には混ぜ、削除済みの照合だけは終わりを除いた鍵でも行う）があれば、その費用（列か式の索引）と並べて置く。
  どれを選んでも design に書くこと: 混ぜる欄は終わりの時刻だけで、終わりのずれ・TZ は混ぜない（始まりの TZ も混ぜていない）・終わりの無い記録と衝突しない形（末尾に長さ前置きの 1 欄）・`hash_is_pinned` に並ぶ終わりつきの固定値の試験・Q5 / Q6 で後から列に写した行の鍵を計算し直すか（直すと、その行の遅れた再送が `lib.rs:1257-1264` の `id_reused` で 400 になる）
- 処置: escalated — Q7（A・`loss: discarded`）。note の (5) を外し、「終わりだけ違う到着を同じ記録とみなすか」を問いにした。推奨は鍵の式を変えず、外部の識別子のある記録だけ終わりの違いを更新として入れる側（消した記録の止めが崩れない）。両側の失うものを `irreversible` に書いた。終わりつきの固定値の試験（`hash_is_pinned` に並ぶもの）は答えに依らず spec で置く

## R2. Q1 は失わない選択肢を欠いている —— 逆順の終わりを列にそのまま入れても、索引の式で始まりに寄せれば落ちない。選択肢 2 の「原文には残る」も保証が無い
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json
- 根拠: Q1 の why「入れるなら、期間で探す索引の式が逆順の値で落ちる…ので、終わりを使わない形で入れることになる」。使い捨ての `postgres:17-alpine` で、`tstzrange(event_time, coalesce(end_time, event_time), '[]')` の索引では逆順の 1 行が `ERROR: range lower bound must be less than or equal to range upper bound` で落ちるが、
  `tstzrange(event_time, greatest(event_time, coalesce(end_time, event_time)), '[]')` の索引では `INSERT 0 1` で入り、52 万行で `Bitmap Index Scan on e_clamp` が当たった（2026-10-21 に手元で実行）。書き出しの読み口は既に逆順の終わりを読む側で外している（`excluded.rs:246` の `CASE WHEN ka.t >= r.event_time`）。
  選択肢 2 の detail「原文には残る」: 終わりは取り込み口の任意の欄で（note (2)）、`IngestRequest.raw`（`ingest.rs:45`）とは別。送り手の原文が終わりを含む保証は無く、列に入れなければその値はどこにも残らない
- kind: premise
- loss: discarded
- 提案: Q1 に「終わりは送られたまま列に入れる（期間の索引は始まりに寄せて点として扱う）」の選択肢を足し、読む側が逆順を疑う費用を detail に書く。選択肢 2 の `irreversible` は「原文が終わりを含まない送り手では、終わりの値が残らない」に直す。
  足した選択肢が高くつかず範囲も越えないなら、Q1 は A ではなく仮（P）になりうる —— 分類はその後で付け直す
- 処置: escalated —— Q1。「送られたまま入れ、期間では始まりの点として扱う」を足して推奨にし（失わない側が安いので kind は `open`）、断る側の `irreversible` を「送り手が捨てたら残らない」に直した（原文に残る保証の記述は消した）。premises.md に始まりに寄せた式の行（確かさ 4）を足した

## R3. (4)「進行中は終わりなしで送り、後で同じ外部の識別子で終わりつきを送る」は、記録ごとの外部識別子を持たないソースでは成り立たない。族の兄弟が一覧に無い
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json
- 根拠: 更新の経路は外部識別子があるときだけ（`lib.rs:1371`）。`external_id_kind` が `none` / `subject` のソース（`ingest.rs:78-80` で外部識別子を判定に使わない）や、本人が書いた読書の感想のような記録では、終わりなしの 1 件と終わりつきの 1 件が、R1 で鍵に混ぜれば**点と期間の 2 行**として残り（`event_folded` も鍵が違うので畳まない。`version_and_ledger.sql:95-109`）、混ぜなければ終わりつきが畳まれて捨てられる。
  FR-64 の補足（`docs/stories/ST39.md`）は「睡眠の機器の再同期のような更新は、外部の識別子を付けて送り直す形」と言うだけで、外部識別子の無い衛星の進行中を決めていない。兄弟の「終わりがある→無い（更新で終わりが消える）」「進行中のまま終わりが届かない」も note に無い
- kind: daily
- 提案: (4) の族として B の問い（推奨つき）を足す —— 「記録ごとの外部識別子を持たないソースが進行中の記録を送ったとき、どう残すか」。既定が決まれば、衛星の種類の登録で外部識別子を求めるかを `docs/handoff/ST39.md` に申し送る。終わりが消える更新・終わらない進行中は、note (4) の兄弟として既定を 1 行ずつ書く
- 処置: escalated —— Q7 の族。外部の識別子の無い進行中の記録を兄弟に入れ、推奨の側で失うもの（後から来た終わり）を `irreversible` に書いた。「終わりが消える更新」は note (4)、「終わらない進行中」は Q7 の族に既定を書いた。ST39 への申し送り（外部の識別子の無い送り手は終わってから送る）は spec の段で `docs/handoff/ST39.md` に置く

## R4. note (3)「収集した記録の終わりは出来事の時刻と同じ錠」は、門の照合・前の版の錠・消去の形・終わりの TZ の扱いまで書かないと、前の終わりが履歴なしで消える穴が残る
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json
- 根拠: 門が「内容が変わった」と見る列は `raw` / `payload` / `event_time` / `content_hash` の 4 つ（`gates.sql:86-90`）で、履歴が「更新前の版」かの照合（R94）も同じ 4 つ（`gates.sql:135-141`）。終わりをここに足さないと、終わりだけ違う履歴を 1 行書けば前の終わりが消える（R94 が塞いだ穴と同じ形）。
  前の版の錠（`gates.sql:160-177` の `reject_version_rewrite`）は列を名指しで並べていて、足した列は書き換えられる。消去の形（`gates.sql:105-109`）は出来事の時刻と鍵を据え置かせているが、終わりは見ないので、消去の顔で終わりを書き換えられる。
  前の版を書く 2 か所（`lib.rs:1587-1595` の `apply_external_update`、`stay_store.rs:927-938` の `push_version`）は列を名指しで並べている。`core.event_live`（`envelope.sql:44`）と `core.event_version_live`（`version_and_ledger.sql:78-89`）は作り直さないと新しい列を出さない。
  始まりの TZ 2 列は「直せる余地を残す」ために錠の外（`gates.sql:66-71`）で、(3) の「出来事の時刻と同じ錠」が終わりの TZ にも及ぶのかは読み分かれる。ST33 D17 の件数の鍵（`openspec/changes/archive/2026-10-17-st33-data-export/design.md:442`）は「版を書かない内容の更新は無い」に乗っていて、終わりの変更も履歴を書く経路に限られていないと崩れる
- kind: technical
- loss: discarded
- 提案: note (3) に、門の 2 か所（内容の変化・更新前の照合）と消去の形に終わりを足す・前の版の錠に 3 列を足す・終わりの TZ 2 列は始まりの TZ と同じく錠の外・前の版を書く 2 か所と 2 つのビューを作り直す、を 1 行ずつ書く。消去で終わりを据え置くか空にするかは FR-51 の「本文」に終わりが入るかの読みなので、出来事の時刻と同じ（据え置く）と書く。
  `tools/check-immutable.d/` に ST41 のファイルで「終わりだけを履歴なしで変えると拒まれる」を置く
- 処置: escalated —— 問いの HTML の「聞かずに決めること」(3) に、門の 2 か所・前の版の錠・消去の形に終わりを足す、終わりのずれと TZ は始まりと同じく錠の外、前の版を書く 2 か所と 2 つのビューを作り直す、を書いて本人に見せる（失わない側。FR-30 から機械で決まる）。`check-immutable.d/` の ST41 のファイルは spec の段で tasks に置く

## R5. 終わりのずれ・TZ の値を確かめる規則が無い（一般の記録の始まりにも無い既存の穴）。note (1) は「揃って有るか」しか見ていない
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json
- 根拠: `ingest.rs:130-160` の `validate` は由来・原文・端末・外部識別子しか見ず、ずれの範囲も TZ の空文字も通す。範囲を断っているのは主張（`lib.rs:1042`）と場所の記録（`lib.rs:1100`）だけで、コメント（`lib.rs:1037-1041`）が「`core.event.tz_offset_min` に CHECK は無く、`validate` も見ていないので、範囲外の値は受理された顔をして格納され、読み出しから永久に消える（`FixedOffset` の上限は ±1,439 分）」と書いている。
  前の版の TZ は書き換えられない（`gates.sql:171-172`）ので、範囲外の終わりのずれが一度履歴に入ると直せない。FR-91「終わりの時刻にも FR-20 を課す」・FR-99（衛星の記録にも FR-20）
- kind: technical
- 提案: note (1) に、終わりのずれは −1,439〜1,439 分・終わりの TZ は空文字でない、を外れたら 400、を足す（`FixedOffset` の上限で機械的に決まる）。TZ の名前を IANA の一覧で確かめるのは、知らない名前の記録を断る（`discarded`）側に倒れるので、この回では入れない。
  始まりの側の同じ穴は ST01 の送信契約を変えるので ST41 では直さず、`scripts/hx change` の依頼にするか `docs/handoff/ST39.md`（FR-99 の持ち主）へ申し送る、と design に書く
- 処置: fixed deep-questions.json note (1) —— 終わりのずれ ±1,439 分・TZ は空でない、を外れたら 400 にする。IANA の照合はしない。始まりの側の同じ穴は ST01 の送信契約を変えるので直さず ST39 へ申し送る、と書いた

## R6. note (7) の終わりの読み口は前の版の側の 2 か所と書き出しの列の分類が抜けていて、走っている CH02 が同じ関数を作り替えていることにも触れていない
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json
- 根拠: 申し送り C7（`docs/handoff/ST41.md`）が名指ししたのは書き出しの (a) と `core.stay_erased` の 2 か所。ほかに、前の版の行を作る副問い合わせ（`excluded.rs:187-190`。`v.payload` / `v.raw` だけで終わりの列を持たない）と、消した滞在の前の版の時間（`excluded.rs:202` の `v.payload->>'end'`）がある。前の版の終わりが列にしか無い記録は (c)「次の記録まで」に落ち、期間が重なる記録では幅が縮んで、消した時間の緯度経度が外へ出うる（C7 と同じ `exported`）。
  書き出しは分類の無い列があると Catalog の段で止まる（`export.rs:887-896`。`EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS` は `export.rs:178-218`）。
  CH02（`feat/ch02-export-next-record-window` に Task 1 の commit があり、下流が走っている）は `overlap_sql` と `excluded_sql` の CTE を窓関数に作り替え、いまの SQL を `export_window_tests.rs` に逐語で写して新旧を突き合わせる（ch02 design.md:105）。`grep -n "ST41" openspec/changes/ch02-export-next-record-window/*.md` は 0 件
- kind: technical
- loss: exported
- 提案: note (7) に、前の版の 2 か所も列→内容の鍵の順で読む・書き出しの 2 つの列の一覧に終わりの 3 列を足す（出す側。説明書 `export/doc.rs` にも 1 行）、を足す。design の Impact に CH02 との重なりを書く: ST41 の下流は CH02 の merge 後の `overlap_sql` に当てる。
  ST41 が先に入る場合は、CH02 の逐語の写しと突き合わせの偽データに終わりの列の行が無いことを `docs/handoff/` で CH02 に申し送る。問いではない（順序は機械で決まる）
- 処置: escalated —— 問いの HTML の「聞かずに決めること」(6)（旧 (7)）に、前の版の側の読み口と書き出しの列の一覧を足して本人に見せる（失わない側）。CH02 との重なりは spec の段で proposal の Impact に書く（ST41 の下流は CH02 の merge 後の `overlap_sql` に当てる。先に入るなら `docs/handoff/` で CH02 に申し送る）

## R7. Q1 の族に「終わりが何年も先・幅が異常に長い（端末の時計の飛び）」が無い
- 成果物: openspec/changes/st41-event-end-time/deep-questions.json
- 根拠: Q1 の family は「終わりが未来 → 通す（出来事の時刻にも未来を断る規則は無い）」で止まっている。`python3 scripts/upstream_misses.py report` の時計の族（ST02 Q29: 27 年戻った信号の実測、ST08 Q7）。衛星は自宅 PC が止まっている間に端末で溜めて送る（FR-99 の補足）。
  幅が何年もある 1 件は、期間で引くたびに（Q3）どの期間にも出て、Q4 の推奨「重なった日すべてに出す」なら毎日の並びに出る。緯度経度を持たないと宣言されていない衛星のソースは持つとみなされ（`excluded.rs:304` の `NOT IN`）、消したどの滞在とも重なって書き出しから除かれる（除き過ぎの側）。収集した記録なら (3) の錠で、外部識別子の更新か削除でしか直せない
- kind: daily
- 提案: Q1 の family の兄弟に「幅が上限（例: 何日）を越える」を 1 行足し、既定（通す・断る・通して印を付ける）を書く。通す側は失うものが無く、断る側は `discarded` になる
- 処置: escalated —— Q1 の族の兄弟に「幅が何年もある」を足し、既定は通す（断ると失う）と書いて本人に見せる

## R8. 期間の索引を、本物の量（約 120 万行）で起動時の移行として作る時間を測っていない
- 成果物: openspec/changes/st41-event-end-time/premises.md
- 根拠: premises.md 1 行目は 20 万行の `EXPLAIN` だけ。`migrate()` は起動のたびに全版を当てる（`dedup_indexes.sql:21-25` の R105 の注意: 起動時の索引の作り直しの間 `core.event` が塞がる）。`CREATE INDEX` は作り終えるまで書き込みを止め、`CONCURRENTLY` はトランザクションの中で使えない。
  本物の写しは約 120 万行（`openspec/changes/ch02-export-next-record-window/proposal.md` の Why）。`upstream_misses.py report` の「初回・作り直し後の最大量を見積もる」（ST08 Q9）に当たる。利用者で絞る形（`user_id` を GiST に入れる）なら `btree_gist` が要るが、それも確かめていない
- kind: premise
- 提案: premises.md に、120 万行の偽データで式の索引を作る時間と、その間に取り込みが待つ時間を測った行を足す（確かさ 4 で）。長ければ、移行の外で作る形を design の候補に置く。問いではない

- 処置: fixed premises.md —— 120 万行で式の GiST 索引を作ると 29.3 秒（書き込みが待つのは最初の起動の 1 回）を測って足した（確かさ 4）。終わりのある行だけの部分索引を design の候補として書いた