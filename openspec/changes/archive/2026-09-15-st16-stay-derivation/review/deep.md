# ST16 深掘りの独立レビュー（deep-questions.json）

問いの一覧を見る前に schema の手順 1〜5 を自分でやり直し、そのうえで突き合わせた。
**問いの JSON は触っていない。**

確かめた範囲: `docs/requirements.md`（FR-1 / FR-18〜FR-33 / FR-50 / FR-51 / FR-76 / PERM-2〜PERM-9 /
§5 の扉 1〜26 と「扉ではないもの」）、`docs/stories/ST16,17,20,21,22,24,25.md` と `INDEX.md`、
`docs/ui-direction.md`、`docs/briefs/ST16-proto.html`（JS を切り出して node で実行）、
`migrations/*.sql` 12 本（**稼働中の PostgreSQL に全部当てて実測**）、
`crates/server/src/{lib,ingest,coverage,dedup_tests}.rs`、`collector-android/**/LocationFix.kt`、
`openspec/changes/st03-idempotent-ingest/{deep.md,specs/record-envelope/spec.md}`、`scripts/board.py`。

- 手順 1（要件どうしの衝突）: R1 / R2
- 手順 2（扉の幅）: R3（扉 #17 の型）/ R5（扉 #15）
- 手順 3（新たに立つ一方通行）: R2 / R3 / R7
- 手順 4（日常に影響する選択）: **該当なし（追加分は無い）**。常時通知・電池・容量・手作業の頻度を
  当たった結果、ST16 が新たに足すものは「作り直しをいつ走らせるか（Q5）」「1 行の粗さ（Q8）」
  「付け直しの手間（Q1 の選択肢 3）」の 3 つで、いずれも既に問いがある。端末側（`collector-android`）は
  1 行も変わらないので電池と容量に新しい判断は立たない。滞在の行数は proto の実測で 1 日 3〜14 件
  （1 年で数千件）なので、位置の 52 万件のような容量の判断も立たない
- 手順 5（既存コードが要件を満たしていない箇所）: R1 / R2 / R5 / R9

---

## R1. FR-22 と ST03 の spec は「派生は対象外」と書いているが、部分一意索引は派生にも掛かる。作り直しは実測で落ちる

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json
- 根拠: migrations/202609120942_dedup_indexes.sql:46-47 / crates/server/src/lib.rs:449-457 / 実測（psql で一意違反）
  - docs/requirements.md:189 「派生（FR-31）は対象外」
  - openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md:271, 278-281
    「THE SYSTEM SHALL 「派生させた」に分類された記録の作り直しを、この capability の対象としない」
    / Scenario「この capability は行を畳まず、作り直した版が別の記録として扱われる」
  - openspec/changes/st03-idempotent-ingest/deep.md:42-48（本人の答え「ST16 に送る」）
  - migrations/202609120942_dedup_indexes.sql:46-47 —
    `CREATE UNIQUE INDEX event_dedup_hash ON core.event (user_id, logical_source, content_hash) WHERE external_id IS NULL;`
    **`origin` の述語が無い**ので `origin='derived'` の行にも掛かる
  - crates/server/src/ingest.rs:170-180 — 冪等キーの入力は `logical_source` + `event_time` + `raw` の 3 つだけ
  - crates/server/src/lib.rs:449-457 — 外部識別子を持たない記録は `ON CONFLICT (user_id, logical_source, content_hash) … DO NOTHING`
  - 実測（migrations 12 本を当てた PostgreSQL に直接）:
    同じ内容の滞在を素直に 2 回入れると
    `ERROR: duplicate key value violates unique constraint "event_dedup_hash"`。
    `ON CONFLICT … DO NOTHING` を付けた取り込み経路の形だと `INSERT 0 0` で**黙って消える**
  - crates/server/src/dedup_tests.rs:1139-1147 — `derived_rebuild_is_not_folded` は
    `{"stay":"v1"}` と `{"stay":"v2"}` の**内容が違う** 2 件しか見ていない。
    「作り直したが内容が変わらなかった滞在」は 1 件も通っていない
- kind: conflict
- 提案: 「滞在の一意の鍵を何にするか（＝作り直しの 2 巡目が同じ行に当たるのか、別の行になるのか）」を
  問いとして足す。少なくとも Q1 の `context` の「DB の側は派生を守っていない」は事実の半分しかない
  （トリガは実測で素通しだが、**一意索引は派生にも掛かる**）ので、Q1 の選択肢 3
  「古い滞在も消さずに残す」が今のスキーマで成立するかをここで先に決める必要がある。
- 処置: escalated
  - **人間に返した。** 指摘の実体（作り直しの 2 巡目が同じ行に当たるのか）は、実装が決められる技術判断ではなく Q1 の選択肢そのもの —— 「安定な鍵を持たせるか」で一意索引に当たるかどうかが決まる。`deep-questions.json` の Q1 に選択肢「滞在に安定な鍵を与える」を足し、`context` を実測に差し替えた（門は派生を素通しだが**一意索引は `origin` の述語が無いので派生にも掛かる**、`ON CONFLICT … DO NOTHING` の形だと黙って消える）。鍵の**作り方**だけは design の D 番号に残す。

## R2. 本人が消した滞在を作り直しが戻すのか、誰も問うていない。実測では「戻らない／基準を変えると戻る」が偶然で決まる

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json
- 根拠: docs/requirements.md:351-357（FR-50）/ docs/requirements.md:220（FR-31）/ 実測（削除済みの滞在が作り直しで戻らない）
  - docs/requirements.md:351-357 FR-50「削除済みの記録には、同じ内容の再送も外部サービスからの更新も取り込まない」
  - docs/requirements.md:220 FR-31「派生の計算方法が変わる THE SYSTEM SHALL 原文から派生を作り直す」
  - 実測: 滞在を論理削除（`deleted_at`）してから同じ内容で作り直すと、
    `event_dedup_hash` は削除済みの行も含むので `INSERT 0 0` になり、
    `core.event_live` の滞在は **0 件のまま**。逆に半径を少し動かして `raw` が変われば鍵が変わるので
    **同じ滞在が復活する**。どちらも誰も選んでいない
  - `python3 scripts/board.py` — ST22「記録を消したことにできる」（record-deletion）は今 **[着手可]**。
    ST16 の後ではなく並んで走りうる
- kind: irreversible
- loss: discarded
- 提案: 「作り直しは、本人が消した滞在を戻すか」を A の問いとして足す。
  消したという行為は Q1 の主観・人物と同じで**原文から再計算できない**。
  戻さないと決めるなら、削除の印を何に掛けるか（区切りが変わっても効く鍵）が要る。
- 処置: escalated
  - **A の問いとして新設した**（新 Q2。`kind: irreversible` / `loss: discarded`）。第 1 回でまだ誰も答えていないので Q1 の直後に置き、以降を繰り下げた（旧 Q2〜Q8 → 新 Q3〜Q9）。実測（削除済みは `INSERT 0 0` で戻らない／基準を動かすと鍵が変わって復活する）を `context` に逐語で入れ、ST22 が [着手可] であることも書いた。

## R3. Q1 に「滞在の識別子を安定な計算で振る」案が無い（扉 #17 と同じ型の判断が問いの外にある）

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json（Q1 の選択肢）
- 根拠: docs/requirements.md:875-878（扉 #17）/ migrations/202609081618_envelope.sql:16 / deep-questions.json Q1 の 4 選択肢
  - docs/requirements.md:875-878 扉 #17「場所の識別子を不変にするか」——
    決めるもの: 値の性質。本人の言葉「id がずれると困る」。FR-49
  - docs/stories/ST17.md（FR-37）/ ST20.md（FR-47）はどちらも滞在を紐づけ先にする
  - migrations/202609081618_envelope.sql:16 `id uuid PRIMARY KEY`（書く側が値を決める。
    収集側の FR-21「毎回新しく振る」は**収集した記録**についての規定で、派生を縛っていない）
  - Q1 の 4 案はいずれも「区切りを動かすか／紐づけをどう移すか」で、
    **鍵の決め方**（同じ場所・同じ日の滞在は作り直しても同じ識別子になる、等）に触れていない
- kind: irreversible
- loss: discarded
- 提案: Q1 に「滞在の識別子を、区切りが変わっても同じになる鍵から作る」案を足すか、
  別の問いとして立てる。ST17 / ST20 が紐づけの行を作った後にこの鍵の決め方を変えると、
  既存の紐づけが指す先が失われる（移し直す入力がもう無い）。
- 処置: escalated
  - **Q1 の第 1 選択肢として人間に返した**（扉 #17 と同じ型であることを `irreversible` 欄に明記）。別の問いに分けなかったのは R1 と同じものを指しているため —— 鍵を持たせるかどうかが、そのまま一意索引に当たるかどうかを決める。

## R4. Q1 の中心の例（100 m → 150 m で 2 件が 1 件に統合される）は、人間が触る proto では再現しない

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json（Q1 の `why`）
- 根拠: `docs/briefs/ST16-proto.html` の JS を切り出し、`buildPoints` / `buildStays` を node で直接回した結果 ——
  5 日ぶんすべてで **半径 100 / 150 / 200 / 250 / 300 m の滞在の件数と区切りが完全に一致**する
  （平日 6 件、家にほぼ 1 日 3 件、出歩いた日 14 件、移動ばかり 5 件。
   `JSON.stringify(starts)` の比較も一致。400 m でようやく平日が 7 件になるが、これは**増える**側）。
  効くのは下げる側だけで、50 m にすると 19 / 21 / 22 / 10 件に砕ける（Q3 の「6 件 → 19 件」は正しい）
- kind: premise
- 提案: Q1 の `why` の例を、proto で実際に起きることに合わせる（下げて砕ける側の例にする）か、
  proto の生成モデル（場所どうしが 1 km 以上離れている）では統合が起きないことを `context` に書く。
  いまのままだと、人間がスライダを右へ動かして「何も変わらない」のを見たうえで
  「統合されたらどうするか」を答えることになる。
- 処置: fixed deep-questions.json
  - Q1 の `why` の例を**下げる側**（100 m → 50 m で 1 件が複数に割れる。平日 6 → 19 件）に差し替えた。併せて `context` の末尾に「proto でスライダを右へ動かしても件数は変わらない —— proto の場所どうしが 1 km 以上離れているため。実データで統合が起きるのは隣の店や同じ建物の別の階のような近い場所どうし」と書いた。

## R5. Q2 の `loss: exported` は、この Story の時点では成立しない。前提も 3 つ足りない

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json（Q2）
- 根拠: scripts/board.py の出力 / migrations/202609120944_gates.sql:82 / crates/server/src/ingest.rs（IngestRequest に sensitivity が無い）
  - 外部 AI への経路は ST27（`ai-access`、requires ST24, ST26）。`python3 scripts/board.py` で **[待ち]**。
    感度そのものの capability も ST24（`data-sensitivity`、requires ST09, ST17）で **[待ち]**。
    ST16 が入る時点で「外へ出る」手段が 1 つも無い
  - migrations/202609120944_gates.sql:82 `IF OLD.origin <> 'collected' THEN RETURN NULL;` ——
    派生の行は門の対象外。実測でも `UPDATE core.event SET raw=… / DELETE` が派生には通る。
    **滞在の感度は後から 1 文で締められ、作り直せば入れ直る**
  - 前提 (a): docs/requirements.md:477 PERM-3 が「**収集した記録**の既定の感度を外部 AI に出してよい」と
    既に決めている。Q2 の `context` はこれに触れず、device-collection/spec.md:96 の「位置は最も機微な記録」
    （**ログに値を出さない**という条項の根拠）を引いている
  - 前提 (b): 取り込みの契約に感度の欄が無い（`crates/server/src/ingest.rs` の `IngestRequest` に
    `sensitivity` が無く、`grep -rn sensitivity crates/` は書き込み経路を 1 つも返さない）。
    選択肢 1 の「実装は何も足さない」は正しいが、**選択肢 2 / 3 は列か経路の追加が要る**ことが書かれていない
  - 前提 (c): PERM-3〜PERM-6 は 収集 / 主観 / 人物 / 写真 の既定しか定めていない。
    **派生の既定を定める行が要件に無い**ので、Q2 の答えの落ち先（PERM の新設か PERM-3 の拡張か）が決まらない
- kind: premise
  <!-- レビューは `irreversible` と書いたが、`loss` を挙げていない。skill の表
       「`irreversible` で `loss` が無い → 不可逆ではない。kind を直す」に従い `premise` にした。
       指摘の実体は「問いに書いた前提（外へ出る経路がある）が事実と違う」で、premise の定義そのもの。 -->
- 提案: `loss` を外して B に落とし、`recommended` を付ける（扉 #15 の決着「既定は厳しい側」が既定になる）。
  A のまま残すなら、「ST24 より前に外部へ出る経路ができる」根拠を `context` に書く。
  併せて (a)(b)(c) を `context` に足す。
- 処置: fixed deep-questions.json
  - **B に落とした**（新 Q3）—— `loss` を外し、`kind` を `open` にして `recommended`（扉 #15 の決着「厳しい側」）を付けた。指摘のとおり ST24 / ST27 が [待ち] の時点では外へ出る経路が 1 つも無く、派生行は門の対象外なので 1 文の UPDATE で締め直せる。前提 (a)(b)(c) は `context` に 3 段落で入れた。答えの落ち先（PERM の新設か PERM-3 の拡張か）は deep.md の「答えの落ち先（要件側）」に積んだ —— **本人の答えが戻った回で要件へ戻す。**

## R6. Q9（作り直す範囲）は幅が無い。C（扉を開けたままにする既定）で足りる

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json（Q9）
- 根拠: docs/requirements.md:896-897（扉ではないもの）/ proto の実測（滞在は 1 日 3〜14 件）
  - docs/requirements.md:896-897 「**滞在の判定基準（半径 100 m / 10 分）。** 滞在は位置の記録からの
    派生なので、基準を変えれば作り直せる」（＝扉ではないもの）
  - Q9 自身が「どちらも計算し直せば戻るので仮でよい」と書いており、`loss` は無い
  - 費用が小さい: proto の実測で滞在は 1 日 3〜14 件（位置は 1,440 件）。
    全期間でも年あたり数千件で、Q5 の「52 万件」の話とは桁が違う
  - 「期間だけ作り直す」は後からいつでも足せる（作り直しは何度でも走る）。
    既定「全期間」が扉を開けたままにする側
- kind: defer
- 提案: Q9 を落とし、`design.md` の D 番号に「作り直しは全期間（反転条件: 1 回の作り直しが
  人間の待てる時間を超えたら期間指定を足す）」として残す。人間の時間を使う幅が無い。
- 処置: fixed deep-questions.json
  - **旧 Q9（作り直す範囲）を落とした。** deep.md の C 表に C10 として「作り直しは全期間」を書き、反転条件（1 回の作り直しが人間の待てる時間を超えたら期間指定を足す）を添えた。design.md ができたら D 番号に写す。

## R7. 「その滞在がどの基準で作られたか」を行に持つかが、どの問いにも無い

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json
- 根拠: docs/requirements.md:245-249（FR-32 / 扉 #22）/ deep-questions.json Q1 の選択肢 1 と Q9 の選択肢 2
  - docs/requirements.md:245-249 FR-32 と扉 #22 が同じ型の判断をしている ——
    「**この列は画面キャプチャ機能が Could であっても day one で持つ**。低品質のエンジンで走らせた
    期間の抽出結果は再計算できず、版が無いと後から選り分けられない」
  - Q1 の選択肢 1（紐づけのある滞在だけ固定する）と Q9 の選択肢 2（期間だけ作り直す）は、
    **どちらも「基準の違う滞在が同じ一覧に並ぶ」状態を作る**のに、その区別を持つ列を誰も問うていない。
    Q1 の選択肢 1 の `irreversible` 欄は「一覧に基準の違う行が混ざる」と書いているが、
    混ざったことを**見分ける手段**には触れていない
  - proto の「作り直したことの見え方」（軸 5）は画面側の表現で、行が基準を持つかとは別
- kind: technical
- 提案: 問いにしなくてよい（C の既定「列を持つ」に当てる）が、`design.md` の D 番号に明示する。
  Q1 の選択肢 1 と Q9 の選択肢 2 は**この列があって初めて成立する**ので、
  どちらかを人間が選んだ場合に黙って落ちないようにする。
- 処置: fixed deep.md
  - 問いにはしない（指摘の提案どおり）。deep.md の C 表の C2「滞在に『作った基準』を持たせる」が既にこれを当てている（レビューは `deep-questions.json` しか見ていないので、重複ではなく独立に同じ結論に来ている）。併せて Q1 の選択肢 2 の `irreversible` に「混ざったことを見分けるには行が『どの基準で作られたか』を持っている必要がある」と明記し、その選択肢が選ばれたときに C2 が黙って落ちないようにした。

## R8. Q7 の `kind: conflict` は根拠が無く、`why` の「実測」の出所は proto の疑似乱数

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json（Q7）
- 根拠: docs/requirements.md:79-80（FR-1）/ docs/briefs/ST16-proto.html:323
  - 水平精度について要件が言っているのは FR-1（docs/requirements.md:79-80「緯度・経度・水平精度・
    端末時刻・端末識別子を含む記録を 1 件生成する」）だけで、**判定に使うことを定めた要件は無い**。
    FR-76（:228-230）とも矛盾していない。衝突している 2 つの要件を挙げられない
  - `why` の「実測で、静止していても GPS は 8〜45 m 揺れる」の出所は
    docs/briefs/ST16-proto.html:323 `const acc = 8 + r()*37;`（proto が作る合成データの生成式）。
    実機の位置記録を測った値ではない（`collector-android` は精度でふるい落とさないのでデータ自体は
    残るが、リポジトリに実測の分布は無い）
- kind: premise
- 提案: `kind` を `open` に直す（`recommended` があるので B のまま扱いは変わらない）。
  `why` の数字には「proto の生成モデル」と出所を書く。実機の記録が溜まっているなら、
  そちらの分布を 1 行載せるほうが判断の土台になる。
- 処置: fixed deep-questions.json
  - 新 Q8 の `kind` を `conflict` から `open` に直した（衝突する 2 要件を挙げられない、は正しい。`recommended` があるので B のままで扱いは変わらない）。`why` の「8〜45 m」には**「proto が作る合成データの生成式であって実機の実測ではない」**と出所を書き、「実機の記録が溜まったら測り直して決め直せる」を足した。

## R9. Q4 の選択肢 1 の代償は「rename できない」ではなく、「ST25 が並列で始められなくなる」

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json（Q4 の選択肢 1 の `irreversible`）
- 根拠: `python3 scripts/board.py` の出力 / docs/stories/INDEX.md:73,78
  - `python3 scripts/board.py` の出力 —— ST25「1 日を時刻順に見る」は `browsing-views` で **[着手可]**、
    かつ「いま同時に始められる上流」に `scripts/upstream.sh ST25` が挙がっている
  - docs/stories/INDEX.md:73, 78 — `derived-records` は ST16、`browsing-views` は ST25 / ST26 / ST36
  - CLAUDE.md の並列の規則（同じ capability を 2 本が同時に触ると差し戻しが起きる。
    実測: ST02 と ST03 が登録簿を共有し 12 時間で 5 往復）
- kind: premise
- 提案: Q4 の選択肢 1 の `irreversible` に「ST25 が `衝突待ち` に落ち、ST16 が終わるまで始められない」を足す。
  rename の禁止は選択肢 1 を選んでも直ちには代償にならない（`browsing-views` という名前は
  どちらの Story が作っても同じ）が、**盤面の停止は選んだ瞬間に効く**。
- 処置: fixed deep-questions.json
  - 新 Q5 の選択肢 1 の `irreversible` を差し替えた —— 「**ST25 が `衝突待ち` に落ち、ST16 が終わるまで始められない**（盤面は今 ST25 を『同時に始められる上流』に挙げている）。capability の名前はどちらが作っても同じなので、rename の禁止は代償にならない」。

## R10. Q3 の proto の出力に判定基準（半径・最短分数）が混ざっており、FR-76 の逐語値を変える決定が画面の答えに紛れる

- 成果物: openspec/changes/st16-stay-derivation/deep-questions.json（Q3）/ docs/briefs/ST16-proto.html
- 根拠: docs/briefs/ST16-proto.html:648-649 / docs/requirements.md:228-230（FR-76 の逐語 100 m / 10 分）
  - docs/briefs/ST16-proto.html:648-649 —— 「指示をコピー」の 1 行目に
    `判定の基準 — 半径 ${state.radius} m / 最短 ${state.minmin} 分（FR-76 の既定 100 m / 10 分から動かした）`
    が出る。スライダは半径 20〜400 m / 最短 2〜60 分を動かせる
  - docs/requirements.md:228-230 FR-76 は 100 m / 10 分を逐語で持つ。
    判定の中身は Q6 / Q7 / Q8（いずれも B）で決めることになっている
- kind: premise
- 提案: Q3 の `note` に「半径と最短分数は**見るために**動かすもので、決めるのは Q6〜Q8」と書くか、
  出力からその 1 行を外す。いまのままだと、画面の構造の答えとして貼り戻した文字列が
  Q6 / Q7 / Q8 の答えと食い違う値を持ちうる。
- 処置: fixed proto.html
  - proto の「指示をコピー」の出力から、判定基準を**決定として**出す行を外した —— 「（このとき画面に出ていた基準: … 見るために既定から動かした値。基準そのものは Q7〜Q9 で決める）」に変えた。コントロール側の説明文と、問いの `note` / Q4 の `context` にも同じ断りを入れた。




---

## 処置のまとめ

10 件すべてに処置を付けた —— **escalated 3 件 / fixed 7 件**（`rejected` / `deferred` / `followup` は 0 件）。

escalated の 3 件（R1 / R2 / R3）は**どれも人間への問いになった** —— R2 は新しい A の問い（Q2）、R1 と R3 は Q1 の選択肢と `context`。
R5 は kind を `irreversible` から `premise` に直したうえで fixed にした（レビュー自身が「`loss` は成立しない」と論証しており、`loss` の無い `irreversible` は不可逆ではない）。

**問いの番号が変わった**（第 1 回でまだ誰も答えていないので、貼り戻しの互換は問題にならない）:

| 旧 | 新 | |
|---|---|---|
| — | **Q2** | R2 が見つけた抜け（消した滞在を作り直しが戻すか。A / discarded）を新設 |
| Q2 | Q3 | 感度。R5 で **A → B** に落とした |
| Q3 | Q4 | 画面（visual） |
| Q4 | Q5 | 一覧を ST16 が画面で満たすか（premise） |
| Q5〜Q8 | Q6〜Q9 | 繰り下げのみ |
| Q9 | — | R6 で落とし、C10 として design へ |

内訳は **A（止める）4 件 / B（仮でよい）5 件**（`ask_wizard.py` の出力で確認）。
A の増減は差し引き 0 —— R2 が 1 件足し、R5 が 1 件を B へ落とした。
