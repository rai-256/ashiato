# impact-review — st44-collector-credential-scope

seen: IMP-01 medium, IMP-02 medium, IMP-03 low

## R1 — 戻したときの「ログインがもう 1 回切れる」は半分しか言えない。入れ替え前のログインが生き返る
- 対象: impact.situations[rollback].cannot / impact.findings[IMP-03].text・derived（design.md Migration Plan の「戻し」も同じ）
- 種類: 言い切り（帰結の取りこぼし）
- 指摘: ログインの印が有効かは `core.web_session` の行の `secret_tag` が今のサーバの鍵と一致するかだけで決まる（`crates/server/src/web_session.rs:202-208`。`revoked_at IS NULL AND secret_tag = $2`）。ST44 は入れ替え時に既存の行を無効にしない（鍵が変わって一致しなくなるだけ）。よって前のサーバ（鍵が `API_TOKEN`）に戻すと、入れ替え前に作られた印は再び一致して有効になる。切れるのは ST44 のサーバが戻る間に作った印のほうである。
  具体例として、落とした電話のブラウザの印を `WEB_SESSION_KEY` を変えて切った後に前のサーバへ戻すと、`API_TOKEN` と画面の合言葉が同じなら、その印が生き返る。IMP-03 は「締めが外れる」だけを帰結としており、締め出したはずのログインが復活することが入っていない。
- 直し方の案: rollback の cannot と IMP-03 に「入れ替え前の印は戻すと再び有効になる（締め出しには画面の合言葉を変える）」を足し、Migration Plan の戻しの手順にも同じ注意を書く。
- 処置: fixed: IMP-03 に、戻すと入れ替えの前のログインが有効に戻ること（WEB_SESSION_KEY で締め出したものも）を足し、重大度を medium に上げた。rollback の場面・依存 D11・根拠 E15（web_session.rs:208）を足し、design の Migration Plan の食い違いは open に置いた（この段は仕様を直さない）

## R2 — D6（落とした電話の手順が成り立たなくなる）の leads_to が S2 で、対応する Scenario と finding が無い
- 対象: impact.dependencies[D6].leads_to / impact.scenarios
- 種類: 根拠（つなぎ）
- 指摘: D6 は `holds: no`（`docs/network.md:43` の quote は実在する）で、帰結は「手順どおりにしても電話のブラウザのログインが切れない」。しかし `leads_to` の S2 は「鍵が無いと起動しない」の場面で、この帰結を扱っていない。この帰結を持つ Scenario も IMP も無い。扱いは `lost[1]` と tasks の network.md 書き直し（tasks.md:127）だけである。
  セキュリティの手順が黙って効かなくなるのが D6 の中身なので、旧手順を覚えている本人が `API_TOKEN` だけを変えて「締め出した」と思い込む、という場面は S として立てて、結末を定めるべき。
- 直し方の案: 「旧手順で API_TOKEN だけ変えた」場面の S を足して D6 をそこへつなぎ、必要なら finding にする。network.md の旧手順の文が残らないことは tasks の検証に入れる。
- 処置: fixed: D6 の leads_to を新しい S4（古い手順どおり API_TOKEN だけを変えても、電話のブラウザのログインは残る）に替えた

## R3 — residual の「記録の中身を返さない 4 つの読み」に、存在の問い合わせが入る
- 対象: impact.scope.residual[0] / impact.overview / features[collector].summary
- 種類: 言い切り
- 指摘: 4 つの中の `HEAD /media/originals/{sha256}` は、漏れた値の持ち主に「この hash の原本がサーバにあるか」を答える（コード `OriginalAndroid.kt:79` の呼び方どおり、404/200 の違い）。`GET /outages` は表示名と最後に生きていた時刻（`lib.rs:3689-3695`）を返す。features の collector.4 は outages と通知判断の漏れ方は書いているが、原本の有無の問い合わせは書いていない。「中身を返さない」は言えるが、「漏れても分かることは無い」には読めない書き方になっている。重大度は low で十分。
- 直し方の案: collector.4 と residual に「原本の hash があるかが分かる」を 1 句足す。
- 処置: fixed: collector.4 と scope.residual に、原本（hash）を受け取り済みかが分かることを足した

## R4 — IMP-02 は medium の置き方で足りるが、`unknowns` と閉じ方の材料に実在の手当てが無い
- 対象: impact.findings[IMP-02].options / impact.dependencies[D8].holds
- 種類: 閉じ方
- 指摘: 帰結は「鍵が無ければ起動しない」で、利用者は理由の出力で気づけるうえ、手で 1 行足せば戻る（`tools/env-init.sh` が既存の `.env` へ足す：tasks.md:93）。そのため high にしないのは妥当。ただし、options の「ST55・ST58 に申し送る」は予定を書いただけで、ST55 の要件（`docs/stories/ST55.md:32`）には鍵の記述が無く、いま進めて安全と言える手当てが実在していない。W2 の「足して起動すれば戻る」に任せる形なら、その旨（手で足す手順が network.md にあること、env-init が足すこと）を `if_accepted` の側の根拠にするべき。
- 直し方の案: IMP-02 の閉じ方の根拠を「手で足す手順と env-init の追記が ST44 で実在する」に置き、ST55・ST58 への申し送りは別の項目として書く。
- 処置: fixed: IMP-02 の options の 1 番目を、ST44 に実在する手当て（手で 1 行足す手順・tools/env-init.sh の追記。tasks 4.2）に置き、申し送りはその後にした。unknowns に ST55 の本文が鍵に触れていないことを足した

## R5 — 言い切り: 「収集アプリは入れ直さない」「呼ぶ口はすべて一覧の中」は台本・テスト用の経路まで確かめていない
- 対象: impact.scope.preconditions[0] / features[collector].points[collector.2]
- 種類: 言い切り
- 指摘: Android は `LocationService.kt`・`OriginalAndroid.kt`・`FeelingReminderSource.kt` を、PC は `sender.rs`・`runtime.rs` を、私の探しでは確かめた範囲で 9 つに収まっている（見つけた呼び先は一覧の中だけだった）。ただし `collector-windows/examples/browser_history_smoke.rs` や `tests/runtime_windows.rs` のように `API_TOKEN` を使う実機側のスモークがある（grep で出る）。coverage の examined にも not_examined にも入っていない。
- 直し方の案: examined にそれらを足して見るか、not_examined に「PC の example・実機の試験」と書く。

（R1 が本命。ほかの IMP の severity と、T4/T5 の `no_dependents_why`、`reflected` / `dismissed` の IMP は無いので、確かめる対象は上記のみ。）
- 処置: fixed: coverage.not_examined に collector-windows の examples と tests/runtime_windows.rs を足した

