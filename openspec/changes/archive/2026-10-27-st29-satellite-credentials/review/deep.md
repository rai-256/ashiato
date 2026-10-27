# ST29 深掘りの問いの独立レビュー（deep-review）

対象: `openspec/changes/st29-satellite-credentials/deep-questions.json`（Q1〜Q8）・`premises.md`・`proto.html`（`docs/briefs/ST29-proto.html` と `diff -q` で同一）。
手順 1〜8 は問いの一覧を読む前にやり、そのあとで一覧と突き合わせた。

## 手順ごとの確かめ

- **手順 1（要件どうしの衝突）**: FR-77 / FR-90 / FR-101 / PERM-8 / PERM-10 / PERM-14 と、関係する FR-24 / FR-62〜64 / FR-86 / FR-88 / FR-92 / FR-94 / FR-96 / FR-97 / PERM-11〜15・§1.3 を読んだ。要件の本文どうしが食い違う組は無かった。ただし AI の既定（C）の 1 つが正典 `data-sensitivity` と PERM-13 の★ C6 と食い違う（R4）
- **手順 2（扉の幅）**: 扉 #13 では、「書き手」が衛星を指すのか発行した 1 本の資格情報を指すのかで読みが分かれる（PERM-14 は 1 つの衛星に何本でも発行してよいとしている）→ R1。扉 #15 は ST29 では承認がまだ無いので幅は無い。扉 #30 は Q1 が受けているが、同じ族の「モノの型」が抜けている → R3
- **手順 3（新たに立つ一方通行）**: (a) 衛星の識別子を、書き換えられない読み出しの記録（行のトリガが UPDATE と DELETE を拒む）へ ST29 の中で書き始める → R4。(b) 資格情報の 1 本ずつの帰属を残さない → R1。(c) 名前の前置き（`c03-`）に本体が意味を持たせている → R2
- **手順 4（日常に影響する選択）**: 期限（Q5）のほかには、毎日目に入るものも手作業も無い。S-7 は登録・発行・取り消しのときにだけ開く画面。該当は Q5 だけで、Q5 は一覧にある
- **手順 5（既存コードとの突き合わせ）**: `authorize` は 33 か所から呼ばれていて、どの `Caller` でも通す。読み出しの記録の middleware は、判定がハンドラより前なので ok/200 で書き、取り込み口では書かない → R5。`/healthz` と `DELETE /session` は資格情報が無くても 2xx を返す → R6。書き出しは、分類の無い表があると失敗する → R7
- **手順 6（過去の取りこぼし）**:
  - 「書き直せない記録の形」は該当する（読み出しの記録に新しい値と欄が入る）→ R4
  - 「申し送りの全項目」は該当し、4 項目とも受けている（st28 Q4 と st17 R5 は ST44 へ移ったと Q6 に書いてある。C2 / C4 は Q6 の context にある）
  - 「時計」は Q5 の 2 つ目の選択肢（期限）と「最後に使われた時刻」が該当する。どちらもサーバの時計で、表示にしか使わない。推奨では期限を付けないので、時計が狂っても失うものは無い。指摘なし
  - 「登録簿の実体のライフサイクル」は Q2 と Q3 が受けている
  - 「他 Story の決定の波及」は該当する。走っている ST09 と ST13 が足す口は、router の全 route を試す形（Q6）が拾う
  - 初回・大量、欠けた入力、外部の DB を読むものは該当しない（ST29 は記録を読み書きしない）
- **手順 7（族の網羅）**: 「名前の形」の族にモノの型（FR-86）が無い → R3。「資格情報の形と寿命」の族に、取り消した 1 本を残すか消すかが無い → R1。「衛星を外す」の族は揃っている
- **手順 8（前提の確かさ）**: P1（確かさ 4）は同じ grep を走らせ直し、31 種・`[:.]` を含むもの 0 で再現した。ただし P1 は名前の完全一致での衝突しか見ていない → R2。P2（確かさ 3）は Q1 の「本体の種類の名前に `:` を使わない」を支えている。migrations の `CREATE TABLE … kind` は `attribute_kind` と `feeling_target_kind` だけで、FR-88 の登録簿は無かった。前提の誤りではないので指摘しない。P3（確かさ 2）は、失うものの無い Q4 にしか載っていない。外の形式のうち、AI が MCP へ資格情報をどう渡すかが premises に無い → R8

## R1. 読み出しの記録に衛星の識別子しか残さない既定は、どの 1 本が使われたかを残さない（取り消した 1 本を残すかも決まっていない）
- 成果物: openspec/changes/st29-satellite-credentials/deep-questions.json
- 根拠: docs/requirements.md:1042（「1 つの衛星に複数の資格情報を発行してよい（端末ごとなど）」）/ Q5 の context（最後に使われた時刻は 1 本に 1 つで、上書きされる）/ Q6 の context（断った求めに残すのは「衛星」と衛星の識別子だけ）/ `migrations/202609290900_access_control.sql:6-13`（画面のログインの印は「行は消さない」で revoked_at を付ける。いまある資格情報の扱いは、残す側に置いてある）/ docs/audit/2026-10-09-satellite-foundation.md:25（U2 の uncaptured）
- kind: irreversible
- loss: uncaptured
- 提案: 1 つの衛星が複数の本数を持つと、漏れた 1 本を後から特定する手がかりは、求めごとの行に資格情報の識別子があるときだけ残る（最後に使われた時刻は上書きされる）。失わない側は高くつかず、範囲も越えないので P に落とす。C を「読み出しの記録には衛星と発行した 1 本の識別子を残す」「取り消した 1 本の行は消さずに取り消しの時刻を付ける」に直し、族「資格情報の形と寿命」の兄弟に足す。書き手の列（ST37）と衛星の読み出しの台帳（ST45）を衛星の粒度にするか 1 本の粒度にするかは、申し送りに書く。
- 処置: fixed D1 仮 —— Q4 の context に「1 本ずつに識別子・取り消した 1 本は取り消しの時刻を付けて残す」を AI の既定として足し、Q6 の読み出しの記録に 1 本の識別子も残すと書いた。Q4・Q5 の族に兄弟を足した。書き手と衛星の読み出しの台帳の粒度は deferred ST37 / deferred ST45（docs/handoff/ST37.md・ST45.md に書いた）

## R2. 「本体の名前と形の上で衝突しない」（Q1・P1）は前置きの一致を見ていない。`c03-` で始まる衛星の名前は書庫のソースとして扱われる
- 成果物: openspec/changes/st29-satellite-credentials/deep-questions.json / premises.md
- 根拠: `grep -rn "LIKE 'c03-" crates/server/src migrations` → crates/server/src/lib.rs:2944・lib.rs:3131（書庫の状態と稼働状況の書庫の段）・migrations/202610141130_outage_alert.sql:31（`LIKE 'c03-%'` に途絶の通知 75 日を付ける。この版は起動のたびに当て直される）/ web/src/App.tsx:54・web/src/archives.ts:103-104（`startsWith("c03-")`）。Q1 の推奨では識別子の形が「英小文字・数字・`-` を `.` でつなぐ」なので、`c03-x.y` が通り、名前の `c03-x.y:reading` も `LIKE 'c03-%'` に当たる。3 つ目の選択肢（前置きしない）なら `c03-foo` がそのまま通る
- kind: premise
- loss: rewrite-all
- 提案: Q1 の context と P1 に「本体は名前の前置き（`c03-`）にも意味を持たせている」を足す。どの選択肢でも、本体の前置きの形（`^[a-z][0-9]{2}-`）で始まる識別子と名前を断ることを C に置く。あるいは前置きで判定している箇所を登録簿の欄に移すことを design に送る。断るのは書き始める前なら安く済むが、書き始めた後の名前の付け替えは全行の書き直しになる。P1 の確かめ方に `grep -n "LIKE '\|starts_with(\"\|startsWith(\""` を足す。
- 処置: fixed D2 仮 —— Q1 の context に本体の前置き（`c03-` ほか）の意味と「本体の前置きの形で始まる識別子と名前は断る」既定を足し、P1 の結果に前置きの grep を足した

## R3. 「名前の形」の族にモノの型（FR-86）が無い。衛星をまたいで共有するという要件の記述と、前置きの規則が食い違いうる
- 成果物: openspec/changes/st29-satellite-credentials/deep-questions.json
- 根拠: docs/requirements.md:816（モノの型・識別子・名前を 1 つの汎用の台帳に保持する）/ docs/requirements.md:821（「衛星をまたいだ重複（モノの型ごとの外部の鍵で判定する）」）/ docs/requirements.md:870（FR-101 が受けるのは「論理ソースか種類」だけ）/ Q1 の family.siblings（論理ソース・種類・識別子・再利用の 4 つ）
- kind: irreversible
- loss: rewrite-all
- 提案: Q1 の family に「モノの型の名前」を足す（decided は「ST40 で問う（期限は最初のモノが書かれる前）」）。モノの型も台帳の行ごとに保存されるので、形を後から変えると全行の書き直しになる。読書ログが 2 本ある場合のように「本」の型を衛星どうしで共有するなら、衛星の識別子を前置きする形とぶつかる。Q1 の選択肢が論理ソースと種類だけに効くことを、context に 1 行書く。ST40 への申し送りにも書く。
- 処置: fixed D3 仮 —— ST40 へ送る（deferred ST40）。Q1 の context に「この問いは論理ソースと種類だけに効く」を書き、族に「モノの型の名前（ST40 で問う）」を足した。docs/handoff/ST40.md を作った

## R4. 断った求めに衛星の識別子を残す既定は、正典 `data-sensitivity` の「だけを持たせ」「識別子の文字列を持たせない」と PERM-13 の★ C6（「そのまま使う」）とぶつかる。しかも書き換えられない表に、識別子の形を ST29 の中で固定する
- 成果物: openspec/changes/st29-satellite-credentials/deep-questions.json
- 根拠: openspec/specs/data-sensitivity/spec.md:295-296（資格情報の種類は「画面のログイン / API の合言葉 / 無し」の 3 つ、「識別子の文字列を持たせない」）/ docs/requirements.md:1033（C6。本人が A を選び、既存の読み出しの記録はそのまま使う）/ migrations/202609290900_access_control.sql:19・22-23（credential と outcome の CHECK）・:29-42（UPDATE・DELETE・TRUNCATE を拒む）/ Q1 の context（「ST29 では衛星はまだ名前を登録しない」）
- kind: conflict
- 提案: 残す側（U2 の uncaptured を塞ぐ側）は失わない側なので、問いにはしない。Q6 の context に次の 3 つを書く。(1) `data-sensitivity` の読み出しの記録の Requirement を MODIFIED にし、資格情報の種類に「衛星」を、欄に衛星（と R1 の 1 本）の識別子を足す。(2) C6 が禁じたのは記録の識別子であり、衛星の識別子は別のものだという読み方。(3) 衛星の識別子の形は、ST29 で書き換えられない表に入った時点で固まる（ST39 で書き始めるときではない）。あわせて過去の取りこぼし「書き直せない記録の形」に従い、行の形を固定した値で試す試験と、知らない値の読み方を design に置く。
- 処置: fixed D4 仮 —— Q6 の context に data-sensitivity の読み出しの記録の Requirement を MODIFIED にすること・C6 が禁じたのは記録と利用者の識別子だという読み方・識別子の形がここで固まることを書いた

## R5. いまの middleware と `authorize` に衛星を足すだけだと、断った求めが ok/200 で残るか、まったく残らない
- 成果物: openspec/changes/st29-satellite-credentials/deep-questions.json（Q6 の context）
- 根拠: crates/server/src/access_log.rs:126（`caller.is_some() && is_ingest(&route)` なら書かずに通す）・access_log.rs:130（`Some(c) => (c.as_str(), "ok", 200)`。ハンドラより前に書く）/ crates/server/src/lib.rs:515-523（`authorize` はどの `Caller` でも `Ok`）・`grep -c "authorize("` は 33 か所 / crates/server/src/web_session.rs:420-446（`GET /session` は middleware を通らず、`caller.as_str()` を ok で書いて 200 を返す）
- kind: technical
- 提案: 何もしなければ、衛星が `/ingest` `/heartbeat` `/drops` に送った断られた求めは記録に残らない。読み出しの口では、403 を返したのに ok/200 と残る。`GET /session` は 200 で衛星を通す。Q6 の context に次を書く。衛星の判定は最も外側の層（`decide`）で 1 回だけ下し、`authorize` は衛星を通さない（足し忘れても閉じている側）。読み出しの記録の行は、実際の結果（403 か 401）と状態の番号で書く。router の全 route を試す試験では、応答の番号と記録の行の両方を見る。
- 処置: fixed deep-questions.json —— Q6 の context に実装の向き（最も外側で 1 回判定・`authorize` は衛星を通さない・実際の結果で記録・取り込みの口の断りも残す・`GET /session` も断る・試験は応答と記録の両方）を書いた。spec の段が design の D に写す

## R6. 「衛星の資格情報で 2xx を返す口が 0」（Q6 の context・Q7 の 1 つ目の選択肢）は、書いたとおりの Scenario にすると成り立たない
- 成果物: openspec/changes/st29-satellite-credentials/deep-questions.json
- 根拠: crates/server/src/lib.rs:3422（`/healthz` は資格情報を見ずに 200）/ crates/server/src/web_session.rs:379-414（`DELETE /session` は資格情報が無くても 204）/ docs/handoff/ST29.md:35（C4 の言い方は「2xx を返す口が、衛星向けに作った口だけ」）
- kind: premise
- 提案: 「資格情報の要らない口（生存確認・ログイン・ログアウト）と衛星向けの口を除いて、2xx が 0」と書き直す。資格情報の要らない口は、衛星の資格情報を付けても付けなくても同じ応答を返す、と試験で確かめる。
- 処置: fixed deep-questions.json —— Q6 の context と Q7 の選択肢 1 を「資格情報の要らない口を除いて 2xx が 0」に直し、その口は衛星の資格情報の有無で同じ応答と確かめる、を足した

## R7. 足す表（衛星の登録簿・資格情報）は書き出しで分類しないと、書き出しが失敗する
- 成果物: openspec/changes/st29-satellite-credentials/deep-questions.json
- 根拠: crates/server/src/export.rs:583-589（分類の無い表を `unclassified` に挙げる）・export.rs:890-895（1 つでもあれば書き出しを断る）・export.rs:482-492（`access_log` と `web_session` は Exclude）
- kind: technical
- 提案: C に足す。資格情報の表（ハッシュ・最後に使われた時刻）は Exclude にする。衛星の登録簿（識別子・名前・外した時刻）は、書き手（ST37）を書き出しの中で名前に引けるよう Include を既定にする。DB に残っているかぎり後から変えられるので、問いにはしない。
- 処置: fixed deep-questions.json —— Q4 の context に、資格情報の表は書き出しから外し、衛星の登録簿は入れる（AI の既定）を足した

## R8. 最初からある 2 つの AI が資格情報を MCP へどう渡すか（Bearer の header を付けられるか）を premises.md で確かめていない
- 成果物: openspec/changes/st29-satellite-credentials/premises.md
- 根拠: premises.md P3（`Authorization: Bearer` を前提にしている。確かさ 2）/ proto.html:136-137（例として「Claude Desktop（ノート PC）」「Ollama（自宅 PC）」に発行している）/ docs/requirements.md §1.3（A-01 は S-01 の中で動き、AI はその向こうの別プロセス）
- kind: premise
- 提案: 決めたことは足すだけで直せる（OAuth の層を後から足せる）ので、問いは増やさない。premises.md に P6 として「AI の MCP クライアントが、手元の網の中の HTTP の MCP に固定の Bearer を付けられるか」を置く。確かめ方と確かさを書き、確かめられなければ ST27 への申し送りにする。proto の例は確かめるまで仮の値だと書く。
- 処置: deferred ST27 —— premises.md に P6（確かさ 1）を足し、Q8 の context に資格情報の名前の例は見本だと書いた。docs/handoff/ST27.md に書いた

## R9. Q2 の 2 つ目の選択肢の「前の記録は書き手のない記録として残る」は、FR-24 と FR-96 に合わない
- 成果物: openspec/changes/st29-satellite-credentials/deep-questions.json
- 根拠: docs/requirements.md:377（FR-24。すべての記録が書き手を持つ）/ docs/requirements.md:892（FR-96。外した衛星が書いた記録は残す）/ Q6 の context と C（外した衛星の行は removed で残す）
- kind: premise
- 提案: 「前の記録は外した衛星（removed）を書き手としたまま残り、どの衛星も直せない」に直す。Q2 を open（B）にしたことは確かめた。変更は FR-22 の経路を通るので前の版が残る。消すのも FR-50 の印で、戻す口がある（openspec/specs/record-deletion/spec.md:141）。このため、どちらを選んでも失うものは無い。
- 処置: fixed deep-questions.json —— Q2 の選択肢 2 を「外した衛星を書き手としたまま残り、どの衛星も直せない」に直した

## R10. Q8 の画面の「識別子」の欄は、Q1 の推奨（衛星が名乗る識別子）を前提に描いてある
- 成果物: openspec/changes/st29-satellite-credentials/proto.html
- 根拠: proto.html の登録の欄 `識別子（衛星の説明書にあるもの。後から変えられない）`。Q1 の 2 つ目の選択肢（本人が登録の時に付ける短い識別子）を選ぶと、欄の意味も見本（`dev.example.booklog`）も変わる
- kind: technical
- 提案: Q8 の context に「Q1 の答えが推奨と違えば、登録の欄を描き直す」と 1 行書く。描き直しは画面だけで済み、失うものは無い。
- 処置: fixed deep-questions.json —— Q8 の context に「Q1 の答えが推奨と違えば登録の欄を描き直す」を書いた

## 不要・分類違い・既定の無い問い
- **不要**: 該当なし。Q2・Q3・Q5・Q6・Q7 は、要件でも扉でも決まっていない幅を問うている
- **分類違い・不可逆の根拠が無い**: 該当なし。Q1 の rewrite-all は、名前が行ごとに保存されるので本物の失い方。Q2〜Q7 にはどれも失うものが無いことを確かめた（Q2 は R9 の根拠、Q4 は発行済みの形を受け続けられる、Q5〜Q7 は足すか変えるだけ）
- **文字で決めさせている画面の問い**: 該当なし（Q8 が visual + proto）
- **既定の無い仮の問い**: 該当なし（Q2〜Q7 に推奨があり、Q8 に draft がある）

## 仮の決定（spec の段へ）

R1〜R4 は失わない側を選んで `provisional.jsonl` に D1〜D4 で載せた。**spec の段は design.md に `### D1（仮）.`〜`### D4（仮）.` を同じ番号で書き、反転条件を付ける**（台帳の id と design の見出しが一致しないと `provisional.py check` が落ちる）。
