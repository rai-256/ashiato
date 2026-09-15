# ST19 深掘りの独立レビュー（deep-review）

- 対象: `openspec/changes/st19-personal-attributes/deep-questions.json`（Q1〜Q4）/ `deep.md`（C1〜C10・確かめたが問わなかったこと）/ `proto.html`
- 入力: `docs/stories/ST19.md`、`docs/requirements.md`（FR-18〜31 / FR-44 / FR-45 / FR-50 / FR-51 / PERM-2〜9 / §5 扉 #3 #15 #17）、`docs/ui-direction.md` S-6、既存コード
- 進め方: schema の deep の手順 1〜5 を一覧を見ずに先にやり、その後で突き合わせた

## 手順ごとの記録（一覧を見る前にやったこと）

- **手順 1（要件どうしの衝突）**: 3 組見つけた。(a) FR-44（追記する）と FR-50 / FR-51（記録を消す）→ 一覧の Q1 にある。
  (b) FR-44 / FR-45 と FR-22（同一の記録の判定は「外部識別子 → 内容の鍵」。FR-21 の識別子は判定に使わない）→ **一覧に無い**（C2 が黙って覆している。R1）。
  (c) FR-45 の 2 軸・C3 の「分からない」と FR-19 / FR-20（すべての記録に出来事の時刻とタイムゾーン）→ **一覧に無い**（R5）
- **手順 2（扉 #3 の幅）**: 「主張した日時」が「D-01 に入った時刻」か「本人がそう主張した時刻」か（C4 が前者に決めている。ST19 の時点では取り込みの経路が無いので、既存の行について失われるものは無い）。
  「いつから」の精度（C3）。訂正と変化の区別（C5）。どれも C に置いてあり、置き方に異論は無い
- **手順 3（新たに立つ一方通行）**: 捨てるもの＝内容の鍵で畳まれる再主張（R1）、削除済みと同じ内容の再主張（R2）。
  変換するもの＝受け取った入力を解析して値・精度に直すこと（原文を残すかどうか。R4）。鍵の入力＝`content_hash` に値を入れるか（R3）。
  外に出すもの＝感度の既定（Q3。外へ出る経路はまだ無い。`docs/production-prep.md:165` のとおり、いまは持ち主だけが呼べる状態）
- **手順 4（日常）**: 該当なし（確かめた範囲: 端末側のコードに変更は無い。通知・電池・容量に関わる判断は立たない。proto の 10 年ぶんのデータで、書き足した主張は 11 件）。
  Q4 の kind が `daily` になっているのは実態と合わない（R7）
- **手順 5（既存コード）**: 個人属性を扱うコードは無い（`grep -rniE "attribute|属性|claim|主張|valid_from|asserted|personal"` を crates / migrations / web / collector-android / tools に掛けた。該当はコメントと DOM 属性だけ）。
  Q1 の context (a) の引用（`gates.sql:40` / `:82` / `:241-248`、`check-immutable.sh:147-159`）はすべてコードと一致した。
  使い捨ての PostgreSQL 17（全 13 版を適用）で実行して確かめた: 「本人が書いた」行の `UPDATE raw` は `UPDATE 1`、`raw=''` の消去形は台帳なしで `UPDATE 1`、`DELETE` は `DELETE 1`。
  さらに `/ingest` の更新の経路が「本人が書いた」行も書き換える（R9）

## proto の実測の再現（deep.md の Q2）

`chrome-headless-shell`（playwright 同梱の 1234）で `proto.html#p=0`〜`#p=3` と末尾 `v` を読み、`#mHeight` / `#mClip` / `#mClaims` を DOM から取った。**8 つとも一致した。**

| hash | 主張 | 縦 px | 入りきらない区間 | deep.md の値 |
|---|---|---|---|---|
| p=0 / p=0v | 21 / 32 | 924 / 902 | — | 924 → 902 |
| p=1 / p=1v | 21 / 32 | 2,622 / 3,488 | — | 2,622 → 3,488 |
| p=2 / p=2v | 21 / 32 | 640 / 640 | 14 / 23 | 640・14 → 23 |
| p=3 / p=3v | 21 / 32 | 2,480 / 3,863 | — | 2,480 → 3,863 |

補足（指摘にはしない）: 見本は骨格以外の軸も変える（p=1 は 2 つの時刻を「同じ大きさで 2 行」、p=3 は「書いた順」+「2 行」）。
骨格だけを変え、ほかの軸を初期値のままにすると、カード・全部は 2,570 / 3,409 px、1 本の時系列は 2,686 / 4,041 px。
「3 倍」という結論は変わらない。入りきらない区間の数は canvas の `measureText`（`system-ui, "Noto Sans JP"`）で決まるので、端末の書体によって前後する。

---

## R1. C2（内容で畳まない・FR-21 の識別子で二重送信を 1 件にする）は FR-22 の文面を逆にしており、要件の衝突として一覧に無い
- 成果物: openspec/changes/st19-personal-attributes/deep.md
- 根拠: docs/requirements.md:198-203（「外部サービス上の識別子があればそれで、無ければ内容ハッシュで」「収集側が生成した識別子（FR-21）は判定に使わず」）/ crates/server/src/lib.rs:520（`ON CONFLICT (user_id, logical_source, content_hash) WHERE external_id IS NULL DO NOTHING`）/ 使い捨て DB で「本人が書いた」行を同じ鍵で 2 回挿入 → 2 回目は `INSERT 0 0`
- kind: conflict
- loss: discarded
- 提案: C2 の向き（捨てずに積む）は扉を開けたままにする側なので、C のままでもよい。ただし FR-22 の文面を覆すので、deep.md の「要件へ戻すもの」に FR-22（適用範囲から個人属性の主張を外す ★）を挙げ、衝突として明記する。人間に確認するなら、Q を 1 つ足す（選択肢は「FR-22 のとおり畳む＝再主張は捨てる」「畳まない＝C2」）
- 処置: escalated — 問いにはしない（畳まない側は何も失わず費用が小さい = 扉を開けたままにする既定）。本人には C の一覧で見せた（異論なし）。答えの後、deep.md の C2 を「原文に主張ごとの識別子と乱数、出来事の時刻に主張した日時を入れて鍵を主張ごとに違える」形に直し、**FR-22 の文面のまま成り立つので要件は改訂しない**とした

## R2. Q1 の選択肢 1・2 に「削除済みと同じ内容の主張は入らない」（FR-50）の帰結が書かれていない
- 成果物: openspec/changes/st19-personal-attributes/deep-questions.json
- 根拠: docs/requirements.md:386-388（「削除済みの記録には、同じ内容の再送も…取り込まない」）/ crates/server/src/lib.rs:467-469（「畳まない記録は `event_dedup_hash` が削除済みの行も含めて弾く」）/ migrations/202609120942_dedup_indexes.sql:46-47（一意索引に `deleted_at` の条件が無い）
- kind: irreversible
- loss: discarded
- 提案: 選択肢 1・2 の `irreversible` に「FR-50 がそのまま主張に及ぶと、消した主張と同じ値・同じ『いつから』をもう一度書いても入らない（C2 と衝突する）。避けるには FR-50 に主張の例外を入れる」を足す。例外を入れるなら、それも Q1 の答えとして要件へ戻す
- 処置: escalated — 問いにはしない（Q1 の選択肢の説明の抜けで、読み替えは扉を開けたままにする側）。Q1 の context (e) に FR-50 の帰結と、主張については識別子で判定する読み替えを足した。答えの後、C2 の形（原文に主張ごとの識別子と乱数）で消した主張と同じ値を書き直しても鍵が違うので入ることになり、FR-50 は改訂しない

## R3. Q1 選択肢 1 の「本文を消した主張の値は戻らない」は、いまの消去の形では成り立たないことがある
- 成果物: openspec/changes/st19-personal-attributes/deep-questions.json
- 根拠: migrations/202609120944_gates.sql:105-109（消去は `raw=''` / `payload='{}'` にするだけで、`event_time` と `content_hash` は動かさない形を要求する）/ crates/server/src/ingest.rs:178-192（`content_hash` = SHA-256(ソース名, 出来事の時刻, 原文)。塩が無い）
- kind: premise
- 提案: 主張を `core.event` に置き、原文が「種類 + 値」だけなら、消去後に残る鍵と「いつから」から住所を候補の総当たりで確かめられる（住所は候補が少ない）。C を 1 つ足す（例: 「消去で残る列に、値を推測できる形で入れない —— 鍵の入力に主張ごとの識別子を混ぜる、または消去で鍵も消す」）。あるいは、選択肢 1 の記述に条件を書く
- 処置: fixed deep.md — C12（原文に他の列へ写さない乱数を入れ、消去の後に残る鍵から値を当てられなくする。識別子は `id` の列に残るので鍵を守らない —— 答えの後に直した）を足し、Q1 選択肢 1 の不可逆の記述を C12 に結んだ

## R4. 主張の「受け取った原文」を残すかが一覧にも C にも無いのに、C4 と Q2 の入力の軸がそれを前提にしている
- 成果物: openspec/changes/st19-personal-attributes/deep.md
- 根拠: deep.md の C4（「画面が送った時刻は原文に残る」）/ 同「確かめたが問わなかったこと」（`core.event` に入れるか別の表にするかは design で決める）/ proto.html の `FROMS`（「1 つの欄に打つ …… 打った形から精度を読み取り」）/ docs/requirements.md:183-184（FR-18。適用は「記録を D-01 に格納する」とき）
- kind: irreversible
- loss: uncaptured
- 提案: 別の表を選んで原文の列を持たないと、画面が送った時刻や「いつから」の打ったままの文字列（読み取りを間違えた場合の元の値）は取っていない状態になる。C11 を足す: 「主張も受け取った要求の原文を `text` のまま残す（FR-18 を主張にも適用する。列を持つ既定）」。あわせて C4 の安全の根拠を C11 に結び付ける
- 処置: escalated — 問いにはしない（残す側は何も失わず費用が小さい = 列を持つ既定）。deep.md の C11（主張も受け取った原文を文字列のまま残す）を足した

## R5. 主張が FR-19〜FR-29 と PERM-2 の「記録」にあたるかを決めていない。FR-45 の 2 軸と FR-19 の 2 軸の対応も無い
- 成果物: openspec/changes/st19-personal-attributes/deep.md
- 根拠: docs/requirements.md:188-191（FR-19 / FR-20「すべての記録に出来事が起きた時刻…とタイムゾーン識別子」）/ migrations/202609081618_envelope.sql:21-24（`event_time` / `tz_*` は NOT NULL）/ deep.md の C3（「分からない」も値として受け付ける）/ reference/skeleton/entities.md:30（旧設計は「`asserted_at` / `valid_from` を `event_time` / `ingest_time` と混ぜない」）/ crates/server/src/ingest.rs:170-171（鍵の入力に `event_time` を使う）
- kind: conflict
- 提案: deep.md は「どの表に置くか（移行できる）」と書いているが、決まっていないのは表の名前ではない。どの要件が主張を縛るか（FR-19 / FR-20 / FR-26 / PERM-2 / FR-50 / FR-51）と、「いつから」を `event_time` に入れるか（入れると「分からない」を置く値が要り、その値が鍵の入力になる）が決まっていない。Q1 は「主張が記録なら」を、Q3 は PERM-2 が及ぶことを前提にしているので、C として明記する（例: 主張は「本人が書いた」記録。`event_time` には主張した日時を入れ、「いつから」は精度つきの別の欄に置く）
- 処置: escalated — 問いにはしない。本人には C の一覧で見せ、異論は番号で受ける。deep.md の C13（主張は本人が書いた記録として FR-19/20/24〜29/PERM-2 に乗る。「いつから」は出来事の時刻に入れず精度つきの別の欄）を足し、C4 の「主張した日時」を出来事の時刻に直した

## R6. C2 の実測の根拠（A → B → A の 3 件目が `INSERT 0 0`）が、いまの鍵では再現しない
- 成果物: openspec/changes/st19-personal-attributes/deep.md
- 根拠: crates/server/src/ingest.rs:178-192（鍵には出来事の時刻が入る）/ 使い捨て DB（全 13 版）で「本人が書いた」行を試した: A（2019-04）→ `INSERT 0 1`、B（2020-04）→ `INSERT 0 1`、A（2020-11）→ `INSERT 0 1`。畳まれたのは **A（2019-04）をもう一度書いたとき**だけ（`INSERT 0 0`）
- kind: premise
- 提案: C2 の結論は変えなくてよい。根拠の例を「同じ値・同じ『いつから』をもう一度書くと畳まれる（`INSERT 0 0`）。『いつから』が分からない主張を固定の時刻に置くと、戻った値も畳まれる」に直す
- 処置: fixed deep.md — C2 の根拠を「同じ値・同じ『いつから』を書き直したときに畳まれる」に直した

## R7. Q4 の kind が `daily` だが、日常に効く選択ではない
- 成果物: openspec/changes/st19-personal-attributes/deep-questions.json
- 根拠: deep.md の「確かめたが問わなかったこと」の手順 4（「主張は 1 人で一生に数十〜百件」）/ proto.html の `LATER`（10 年で書き足した主張は 11 件）
- kind: technical
- 提案: `kind` を `open` にする。`loss` が無く `recommended` もあるので、B（仮でよい）のままでよい
- 処置: fixed deep-questions.json — Q4 の kind を open にした（B のまま）

## R8. Q2 の選択肢の `irreversible` は作り直しの手間を書いており、失われるものではない
- 成果物: openspec/changes/st19-personal-attributes/deep-questions.json
- 根拠: Q2 の options[0].irreversible（「後から変えると 3 つの画面を作り直す」）/ proto.html の出力（どの軸を選んでも、積む主張の欄は変わらない。formHtml は どの入口でも「変わった / 間違っていた」と「どの主張を直すか」を取る）
- kind: technical
- 提案: `kind: visual` は規則どおり A のままでよい。`irreversible` は「画面は作り直せば戻る（データは失われない）。ただし『いつから』を 1 つの欄から読み取る場合は、打った文字列を残さないと読み違いが戻らない（R4）」に直す
- 処置: fixed deep-questions.json — Q2 の irreversible を「画面は作り直せば戻る。骨格を ST20 / ST21 が前提にするので手間がかかる」に直した

## R9. いまの `/ingest` は、外部識別子つきで届いた「本人が書いた」記録を書き換える。C1 と C2 はこの経路を塞ぐと言っていない
- 成果物: openspec/changes/st19-personal-attributes/deep.md
- 根拠: crates/server/src/lib.rs:566-569（`stored.content_hash != hash && external_id.is_some()` なら、由来を見ずに `apply_external_update`）/ crates/server/src/ingest.rs:13（`authored` を受け付ける）/ crates/server/src/lib.rs:1496（`/ingest`）
- kind: technical
- 提案: C1 か C2 に「主張は外部識別子を持たず、FR-22 の更新の経路に乗らない（乗った場合は 400 で断る）」を足す。C1 の錠だけを足すと、この経路は DB の例外（錠が即時か COMMIT 時かによらず）になり、`internal_at` を通って 500 で返る
- 処置: fixed deep.md — C2 に「ソースの外部識別子の粒度を『無し』にし、外部識別子が重複の判定にも更新の経路にも乗らない（`lib.rs:344-356` が `external_ref` へ回す）」を足した

