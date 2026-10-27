# ST29 仮で進めた決定の確かめ（provisional_review）

対象: `provisional.jsonl` の D1〜D4（この節より前に確かめは無かった）。問いを作った AI の「失わない」は前提にせず、design・spec の差分・既存のコードを辿り直した。

## D1
- 判定: 確か
- 根拠: (1) 失わない側を選んでも何も失われない。design.md D7 は `core.satellite_credential` に `id uuid PRIMARY KEY`・`revoked_at` を持たせ、2 つの表の DELETE と TRUNCATE をトリガで拒む（tasks.md:39 `core.reject_satellite_delete()`）。衛星を外す操作も `revoked_at` を付けるだけ（design.md:37・tasks.md:84）で、行を消す道は無い。上書きされる `last_used_at` は R1 が「1 本の粒度の手がかりは求めごとの行にしか残らない」とした理由そのもので、D9 の `satellite_credential_id` が受けている（design.md:134・143）。書き出しでは資格情報の表が Exclude だが（D13）、DB には行が残るので exported も uncaptured も起きない。(2) 要る費用は列 1 つ・欄 2 つ・トリガだけで、資格情報の表は ST29 がもともと作るものなので範囲は越えない。requirements.md の本文も変えない（PERM-14 の「複数の資格情報を発行してよい」requirements.md:1042 の中に収まる）。(3) `follows` は無い。(4) Q2 の答え（登録し直しは「続き」）は D7 で同じ衛星の行を戻す形になっていて、取り消した 1 本は戻らない（design.md:108-109）ので D1 と矛盾しない。Q2 も Q5 も D1 の土台にはなっておらず、`rests_on: []` で抜けは無い
## D2
- 判定: 確か
- 根拠: (1) 本体が名前の前置きで判定している箇所を grep で洗い直した（`grep -rnE "LIKE '[a-z][0-9]|starts_with\(\"[a-z][0-9]|startsWith\(\"[a-z][0-9]" crates/server/src migrations web/src`）。当たったのは `c03-` と `c03-myactivity-` だけだった（lib.rs:2944・3131、archive/classify.rs:194、archive/worker.rs:1656・2374、202610141130_outage_alert.sql:31、web/src/App.tsx:54、archives.ts:103-104）。どれも `^[a-z][0-9]{2}-`（design.md D5:66・spec satellite-platform:50）で塞げる。衛星の名前は `<識別子>:<…>` なので、識別子を断れば名前も本体の前置きでは始まらない。まだ衛星は 1 つも登録していないので、断られて失う行も無い（discarded にならない）。(2) 費用は純粋関数の検査 1 つと 400 の種別 1 つで、範囲は越えない。requirements.md の FR-101（:873）の「本体の名前に衝突させない」の中に収まる。(3) `follows` は無い。(4) `rests_on: Q1` は答え「衛星が名乗る識別子を前置する」（deep-answers-1.txt）と同じ向き。名前の前置きを使わない答えであったなら、識別子だけを断る形では足りなかった。土台はほかに見当たらない
## D3
- 判定: 確か
- 根拠: (1) ST29 はモノの台帳を作らない。tasks.md・design.md・specs に FR-86 の表は無く、`grep -rlnE "CREATE TABLE[^;]*(thing|object|mono|entity)" migrations` も 0 件だった。書かれる行が無いので、形を決めずにおいても後から書き直す行は生じない。モノの型の名前の形は requirements.md:824-825 の NEEDS CLARIFICATION として ST40 の深掘りに残っていて、docs/handoff/ST40.md と docs/stories/ST40.md:36 にも同じことが書かれている。(2) 費用は申し送り 1 件だけ。spec satellite-platform:114 には「含めない」とだけ書き、要件の本文は決めていない。(3) `follows` は無い。(4) `name_owner` は ST38 と ST39 が論理ソースと種類で呼ぶだけで（design.md:68-69）、モノの型には使わない。このため Q1 の答えは D3 の土台にならない
## D4
- 判定: 確か
- 根拠: (1) design.md D4:59 は、欄に入れるものを識別子の文字列ではなく本体の中の番号（uuid）に絞っている。Q1 の形が変わっても、書き換えられない行は書き直さずに済む。番号を引く先の `core.satellite` は DELETE と TRUNCATE を拒むので（D7）、指す先は残る。外部キーを付けないことにも理由がある（design.md:147）。`access_log` は書き出しの Exclude のまま（export.rs:482-492）なので、exported にもならない。(2) 要件の本文を変えないかを確かめた。PERM-13 の ★ C6（requirements.md:1037-1039）と、その元の監査 docs/audit/2026-10-11-consistency.md の「C6」の節を読んだ。本人が A で決めたのは「記録の識別子は別の台帳に置き、`access_log` には入れない」で、直すところとして「新しい Story の `data-sensitivity` の MODIFIED」をあらかじめ挙げている。呼び出し元（衛星と 1 本）の番号を足すのはこの範囲に収まり、requirements.md の SHALL の文も変えない。spec data-sensitivity:7-9 も、記録と利用者の識別子の文字列は禁じたままにしている。(3) `follows` は無い。(4) `rests_on: D1` は正しい。1 本の番号の欄は D1 が土台になっている。Q6 の答え（403）は、行の結果の値を決めるだけ（spec data-sensitivity:10・22）で、欄を残すかどうかの土台ではない。付記: `.down.sql` は transaction で包まれていない。psql で上から当てると、欄を落とした後で元の CHECK の付け直しに失敗し、衛星の番号だけが消える順になりうる。tasks.md:40 の注記に加え、戻しの先頭で「衛星の行があれば止める」か BEGIN/COMMIT で包むことを、実装の段で確かめるとよい（戻しは人間がわざわざ当てる操作で、D4 の選択そのものの失い方ではない）
