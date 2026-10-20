# CH02 deep-questions.json の独立検査（deep-review）

問いの一覧を見る前に、手順 1〜8 を `docs/changes/CH02.md`・`premises.md`・`crates/server/src/export/excluded.rs`・
`openspec/specs/data-durability/spec.md`（幅の (a)〜(d)）・ST33 design D5 / D13 / D17・既存の試験から自分でやり直し、その後で突き合わせた。

## 手順の記録（指摘ではない）

- 手順 1（要件どうしの衝突）: CH02 は `satisfies: []`。要件どうしの衝突は無し。ただし**依頼（lead() の窓関数）と ST33 design D5（lead() を退けた理由）が同じ対象について逆を言っている** → R2
- 手順 2（扉の幅）: 扉は無い。正典の (c)「同じ利用者・同じソース・同じ端末の次の記録の時刻まで」は、同じ時刻の記録が並ぶときの「次」に幅がある → R1
- 手順 3（新たに立つ一方通行）: 列の型・保存形式・鍵の計算は変わらない（移行なし。台帳の `written` の形（D17）にも触れない）。外へ出るのは書き出しのファイルだけで、「いまの規則を残す」こと自体が漏れる側を残す箇所が 1 つある → R1
- 手順 4（日常に影響する選択）: 設定画面を開くたびの `GET /export/summary`、書き出し中の `GET /export` の D17 の数え直し、窓の並べ替えの一時ファイル → R2・R3・R7
- 手順 5（既存コードが要件を満たしていない箇所）: `excluded.rs:192` の `(n.event_time, n.id) > (r.event_time, r.id)` は、同じ時刻の兄弟がいる記録の幅を 0 にする（DB で再現した。R1）。それ以外の (a)(b)(d)・緯度経度の判定・前の版の親の鍵は spec と合っていた
- 手順 6（過去の取りこぼし）: 当てはまったのは 4 つ。「人間の決定の根拠の実測は合成負荷でなく実データで」（ST06 Q10）→ R3、「DB を共有する検証は前段の行に依存しないか」（ST33 Q10）→ R5、「推奨に『失うものが無い』と書く前に経路を辿る」（ST08 Q11）→ R1、「検証は凍結前に壊すと落ちるかを確かめる」→ R6。
  時計の族は、この change が (c) の写しの時点（`now()` / `transaction_timestamp()`）の扱いを変えないので当てはまらない（確かめた範囲: `excluded.rs:198` の呼び出し、`dump.rs:138`、`summary.rs:92`、`ledger.rs:431`）
- 手順 7（族の網羅）: Q2 の偽データの列挙は「時刻・端末・削除の印」の族の一部で、兄弟が 5 つ抜けている → R4
- 手順 8（前提の確かさ）: 確かさ 2 の前提は 1 つ（`'1 microsecond' PRECEDING` = 厳密な後）。`loss` のある決定には載っていないが、手元の PG で `'…00.0000004Z'::timestamptz = '…00Z'` が t、1 µs 差の記録が前の版の次になることを確かめた（4 に上げられる）。確かさ 4 の「120 万行で 1.9 秒」は細い合成表の値 → R3。「失われるもののある決定は無い」は R1 で崩れる
- D17 の件数の鍵（`ledger.rs:398` の `erased_key`）: 鍵は削除の台帳・前の版・吸収の台帳の行の数で、design D17 の論拠は「鍵に無い書き込み（新しい記録の取り込み）は幅を縮める側にだけ働く」（design.md:446）。(c) の次の記録を `core.event_live` から引く限り、lead() に替えても成り立つ。崩れるのは、窓を絞った後の行に当てて「次」を変える実装だけ（R4・R8 で固定する）。単独の指摘にはしない

## R1. 同じ時刻の記録が並ぶと、(event_time, id) で先の記録は幅 0 になり消した時間に重なっても出る —— 「結果を変えない」はこの漏れる側を残す決定で、一覧に無い
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: crates/server/src/export/excluded.rs:192（記録は `(n.event_time, n.id) > (r.event_time, r.id)`）と excluded.rs:195（前の版は `n.event_time > r.event_time` の厳密な後）。手元の PG で 01:00 の記録 2 つ（id …0a・…0b）と 03:00 の記録、消した時間 01:30〜02:00 を当てると、…0a は次の記録が 01:00（幅 0）で `excluded=f`、…0b は 03:00 までで `excluded=t`（どちらが漏れるかは乱数の UUID で決まる）。openspec/specs/data-durability/spec.md:229「読めない幅を点として扱うと、消した時間の前に開いて中まで見ていた地図が出る」・spec.md:227「出したファイルは戻らない」（C21 の既定は厳しい側）。premises.md は「4 行ずつ同じ時刻」で新旧の食い違い 0 件 = lead() はこの振る舞いを忠実に残す。既存の試験に同じ時刻の記録の組は 0 本（`grep -n "同じ時刻" crates/server/src/export_excluded_tests.rs` 0 件）
- kind: irreversible
- loss: exported
- 提案: A の問いを 1 つ足す —— 「記録の (c) の『次の記録』を、前の版と同じ厳密に後の時刻にするか、いまの (event_time, id) の順を残すか」。失わない側（厳密な後）は除く行が増え、依頼の「結果を変えない」と design D5:160 を越える（だから A）。premises.md に「本物の写しで、(c) かつ緯度経度を持ちうるソースの同じ時刻の組が何件あるか」を数える前提を足し、件数を問いの context に置く。note の「後から変えても何も失われない」と premises.md 末尾の「loss のある決定は無い」は、この問いを足した後に直す
- 処置: escalated —— 問い Q1（`kind: irreversible`・`loss: exported`）として足した。note と premises.md の「loss のある決定は無い」も直した。本物の件数は未確認（確かさ 1）と premises.md に置いた

## R2. ST33 D5 が lead() を退けた理由（全表を並べ替えてから絞る）が、消した時間が無い・古いときにはいまも成り立つ。絞り込みを残すかが一覧に無い
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: openspec/changes/archive/2026-10-17-st33-data-export/design.md:161-163（lead() を退けた理由）。crates/server/src/export/excluded.rs:261 `WHERE r.event_time <= (SELECT max(end_at) FROM erased_spans)` —— 消した滞在が 0 件なら NULL との比較で 0 行になり、次の記録は 1 度も引かれない。窓関数の外の `event_time` や緯度経度の条件は PARTITION BY の列でないので窓の下へ押し下げられず、lead() は呼ぶたびに `core.event_live` の全行を並べ替える。呼ぶのは設定画面を開くたび（web/src/SettingsView.tsx:72 の `GET /export/summary`）と、鍵が変わるたびの D17 の数え直し（crates/server/src/export/job.rs:66-80）と書き出しの 3 本
- kind: daily
- 提案: B の問い（推奨つき）を足す —— 「窓を当てる前に、消した滞在が無ければ 0 行で返す・消した時間の終わりより後の行は各組の最初の 1 行だけを窓に入れる、などで D5 の絞り込みを残すか」。どちらも結果は変えない（R4 の答え合わせで固定）。design に「D5 の lead() を退けた判断を覆す」ことと、その理由（CH02.md の 84 秒の実測）を書く
- 処置: escalated —— 問い Q4（B・推奨は絞り込みを残す）として足した

## R3. 「120 万行で lead() が 1.9 秒」は payload・raw を持たない細い合成表の値で、本物の写し（約 120 万行）で測っていない
- 成果物: openspec/changes/ch02-export-next-record-window/premises.md
- 根拠: premises.md:5（合成データの説明に payload・raw の大きさが無い）と premises.md の表の 4 行目（確かさ 4）。docs/changes/CH02.md「本物の写しは約 120 万行」。窓の並べ替えに `payload`・`raw` を載せるか鍵の列だけにするかで、並べ替えの量と一時ファイル（既定の work_mem を越えて外へ書く）が桁で変わる（overlap_sql は excluded.rs:242 で `payload` `raw` を返す行を作っている）。`python3 scripts/upstream_misses.py report` の「合成負荷でなく本人の実データで、パラメータを変えて結論が反転しないか」（ST06 Q10）に当たる
- kind: premise
- 提案: premises.md に、本物の写し（使い捨ての DB に戻したもの）か、行の幅を本物に寄せた偽データで、窓の時間と一時ファイルの量を測った行を足す。それまで Q3 の context の「窓関数は全行で 1.9 秒」に「細い合成表」と書く。窓には鍵の列（id・user_id・logical_source・device_id・event_time）だけを載せて id で戻す形を design の候補に書く
- 処置: fixed premises.md —— 1.9 秒の行に「細い表の値」と書き確かさを 3 に下げた。Q3 の context にも書き、本物の量の時間は量の試験で測る（Q3 の推奨）

## R4. Q2 の偽データの列挙に、新旧が分かれうる兄弟の場合が 5 つ抜けている
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: (1) 前の版の `user_id` は凍結され（migrations/202609120944_gates.sql:164）、親の `user_id` は後から直せる（gates.sql:66）—— いまの引きは親から取る（excluded.rs:186）が、記録と前の版を混ぜた窓を `core.event_version` 自身の `user_id` で区切ると組が変わる。(2) 緯度経度を持たない行・消した時間の終わりより後の行も「次の記録」になる —— 窓を絞った後に当てると幅が延び、既存の export_excluded_tests.rs:350 は次の記録が座標なしでも答えが同じになる配置なので気づけない。(3) 1 µs 差（前の版の枠 premises.md:13 の境）。(4) 消した滞在が 0 件（R2 の短絡）。(5) 写しの時点が呼び出しで違う（dump.rs:138 は `transaction_timestamp()`、summary.rs:92・ledger.rs:431 は `now()`）
- kind: technical
- 提案: Q2 の推奨の detail に (1)〜(5) を足す。特に (2) は「座標つき 00:10・座標なし 00:40・消した時間 01:00〜02:00 → 00:10 は残る」のように、絞った後に窓を当てると落ちる配置を 1 本は決め打ちで置く
- 処置: fixed deep-questions.json —— Q2 の推奨の detail に (1)〜(5)（利用者のずれ・座標なしや後の行が次の記録になる配置・1 µs・0 件・写しの時点）を足した

## R5. Q2 の答え合わせ（旧 SQL を試験に残す）を共有の積もる DB で走らせると、旧 SQL 自身が 84 秒の型で遅く、他の試験の消した滞在も混ざる
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: crates/server/src/export_excluded_tests.rs:5「消した滞在の時間は全テストで共有される」。`kept()`（export_excluded_tests.rs:149-163）は `id = ANY($1)` で絞るが、MATERIALIZED の CTE は DB の全行に次の記録を引く。docs/changes/CH02.md の 84 秒・240 秒は、この共有の DB での値。まっさらな DB を作る口は既にある（crates/server/src/stay_tests.rs:1322 `fresh_db()`、export_excluded_tests.rs:197 が使う）。upstream_misses の ST33 Q10「DB を共有する検証は前段のテストが残す行に依存しないか」
- kind: technical
- 提案: Q2 の推奨の detail に「新旧の答え合わせは `fresh_db()` のまっさらな DB で走らせる」を足す（旧 SQL を共有の DB で回さない）
- 処置: fixed deep-questions.json —— Q2 の推奨を「まっさらな DB（fresh_db()）で答え合わせ」にし、context に共有の DB で 84 秒かかることを書いた

## R6. Q3 の推奨「EXPLAIN で行の数だけ回る副計画が無いことを見る」の判定の基準が名指しされておらず、「揺れない」は基準しだい
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: 重なりの判定 `EXISTS (SELECT 1 FROM erased_spans s …)`（excluded.rs:265-267）は、正しい形でも行ごとに消した時間（数十件）を回る。R2 の短絡で「各組の最後の 1 行だけ次を引く」形にすると、`core.event` を読む副計画が組の数だけ正当に残る。`loops` は `EXPLAIN ANALYZE` でないと出ない（Q3 の context は「loops = 行数」と書く）。既存の量の試験 crates/server/src/export_file_tests.rs:1447-1449 が `seconds` と `summary_seconds` を既に出しているのに、Q3 の選択肢に無い
- kind: technical
- 提案: Q3 の推奨の detail を「`EXPLAIN (ANALYZE, BUFFERS)` で `core.event` を読んだ行数（またはバッファ数）を N と 4N で比べ、4 倍の桁に収まることを見る（機械の混み具合に依らない）。旧 SQL に戻すとこの試験が落ちることを凍結前に確かめる」に直すか、選択肢として足す。計測は既存の `export_volume_one_run_reports_time_and_memory` を `ASHIATO_EXPORT_VOLUME_ROWS` で回す、と書く
- 処置: fixed deep-questions.json —— Q3 の推奨を「EXPLAIN (ANALYZE, BUFFERS) で読んだ行数を N と 4N で比べる・旧 SQL で落ちることを先に確かめる・既存の量の試験で時間を測る」に直した

## R7. D13 の反転条件（件数の数えが 3 秒以上）は 2026-10-14 に「当たったが替えていない」状態で、この change はその判定の入力を変える。測って判定し直すかが一覧に無い
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: openspec/changes/archive/2026-10-17-st33-data-export/design.md:380-382（35 万行で `GET /export/summary` 1 回 6.5 秒。反転条件に当たったがこの fix wave では替えていない）と design.md:449（D17 の数え直しも同じ桁）。数えは設定画面を開くたびに走る（web/src/SettingsView.tsx:72）
- kind: daily
- 提案: B の問い（推奨つき）を足すか Q3 に含める —— 「この change の計測で `summary_seconds` を本物の量（120 万行。消した滞在あり・無しの両方）で測り、D13 の 3 秒の判定を design に書き直す」。替える（目録から出す）かどうかは D13 の持ち主の判断で、ここでは測った値を渡すだけ
- 処置: escalated —— Q3 の推奨に「件数の数えの時間を消した滞在あり・無しで測り D13 の 3 秒の判定に渡す」を入れ、context に D13 の反転条件を書いた（人間が Q3 で見る）

## R8. CH02.md の「壊してはいけないもの」に入るべき制約が、問いからも premises からも読めない
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: 写しは `READ ONLY` のトランザクションで一時表を作れない（design.md:159。窓の結果を一時表に置く実装は落ちる）。次の記録は `core.event_live` から引き、削除済みの行を間に挟まない（design.md:160）。前の版の組の鍵は親の `user_id`・`logical_source`・`device_id`（excluded.rs:186）。D17 の鍵の論拠は「取り込みは幅を縮める側にだけ働く」（design.md:446）。Q1 の context は「ついでに変えたいものは今のところ見つかっていない」とだけ書く
- kind: technical
- 提案: 問いにはしない。上流が CH02.md の「壊してはいけないもの」を書くときの入力として、上の 4 つと R1 の答え（同じ時刻の扱い）を deep.md に書き出す
- 処置: fixed premises.md —— 「record の段への入力」の節に 5 つ（R1 の答えは Q1）を書いた。record が CH02.md の「壊してはいけないもの」に写す

## R9. Q1（範囲）は依頼の本文で決まっていて、人間に問う幅が無い
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: docs/changes/CH02.md「依頼:」の段落が、直すもの（次の記録の引き）・変えないもの（除かれる行）・示すもの（線形）・通すもの（ST33・ST09 の試験）を名指ししている。選択肢 2・3（試験の DB の作り直し・server 全体の洗い出し）は依頼に無い別の change（`scripts/hx change`）で、失うものも無い
- kind: defer
- 提案: Q1 を一覧から外し、design に「範囲は依頼のとおり。試験の DB の積もりは別の change」と書く。範囲について人間に問うべきものは R1（振る舞いを変えるか）のほうで、そちらを足す
- 処置: fixed deep-questions.json —— 旧 Q1（範囲）を外し、note に「範囲は依頼のとおり。試験の DB の積もりは別の change」と書いた。範囲の問い（振る舞いを変えるか）は新しい Q1

## R10. Q2 は `loss` の無い open ではない —— 弱い側を選ぶと、試験 0 本の経路の誤りが書き出しに出る（失わない側が安いので P）
- 成果物: openspec/changes/ch02-export-next-record-window/deep-questions.json
- 根拠: Q2 の why 自身が「1 行でも減れば消した時間の緯度経度が書き出しに出る（外に出たものは戻せない）」と書く。既存の試験は、premises.md:13-14 で窓の形が別になり 2 乗に戻る罠もある前の版の (c) の経路を 1 本も通していない（前の版を置く試験は export_excluded_tests.rs:408・548・565 と export_file_tests.rs の各所で、どれも `s01-stay`（(a)）か `c01-photo`（(d)）か (c) でも消した時間から遠い時刻）。端末 NULL も 0 本（`put_as` は端末を `&str` で必ず入れる。export_excluded_tests.rs:44-52）
- kind: irreversible
- loss: exported
- 提案: Q2 を `loss: exported` に直し、失わない側（推奨の答え合わせ）が高くつかず範囲を越えないので P として台帳（`provisional.py add`）に載せ、provisional_review の確かめを通す。context の「一部しか通っていない」は「前の版の (c)・端末 NULL・同じ時刻は 0 本」に直す
- 処置: escalated —— Q2 は人間に見せる一覧に残し（推奨は失わない側）、context を「前の版の (c)・端末 NULL・同じ時刻の組は 0 本」に直した。record で推奨のままなら P として台帳に載せる
