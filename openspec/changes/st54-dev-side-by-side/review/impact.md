# impact-review — st54-dev-side-by-side

seen: IMP-01 high, IMP-02 high, IMP-03 medium

## R1 — situations[carryover].can の「初回に履歴を全件取り込む」は、挙げた E6 が支えていない
- 対象: impact.situations[carryover].can / basis / evidence
- 種類: 根拠
- 指摘: carryover は `basis: observed`・evidence は E6 だけ。E6 は `crates/server/tests/server_startup.rs:57` の試験用の版番号 `-beta.2` で、「今ある DB は正式版の DB として読まれる」の半分すら直接は示さない（実際の版の記録の行は `migration_ledger.rs:556` が `BUILD_VERSION` を入れる）。後半の「開発版の PC の収集アプリは初回にブラウザの履歴を全件取り込む」は、E6 に無く design D2 の散文（design.md:43）にだけある。履歴の取得の印が無いときの全件取り込みを `history/collect.rs` で確かめた evidence が無いので、`observed` でなく `spec`/`guess`。D5（依存）の `holds: yes`・`observed` も同じ E6 で、今ある DB の版の記録の行が版番号なしの組み立てで当たったことは試験の文字列からは言えない。
- 直し方の案: carryover.can の後半は spec の引用に改め basis を直す。D5 の evidence は `migration_ledger.rs:17,556` と ST52 の `ASHIATO_VERSION` の規則に差し替える。
- 処置: fixed: carryover を derived にして evidence を外し、design D2・D3 を refs に。依存 D5 の evidence を migration_ledger.rs:17・556 と version.sh:138 に差し替えた

## R2 — IMP-01 の影響の書き方が「恐れ」に留まり、実際は dev 側に除外を書く口が無い
- 対象: impact.findings[IMP-01].text / if_accepted.residual、dependencies[D1].affects
- 種類: 言い切り（限定の付けすぎ）／重大度の理由
- 指摘: design.md:46 は「除外の登録は読むだけで書く口は無い」と言う。開発版の `state_dir` は `<置き場>/dev` で、`collect.rs:315` はそこの `exclusions.json` を読む。誰も `<置き場>/dev/exclusions.json` を作らないので、tasks のとおりなら「恐れ」でなく、履歴の除外は常に空で、しかも carryover の初回全件取り込みと重なる（履歴の全件が除外なしで開発版のサーバへ行く）。`if_accepted.residual` / `scope` は「除外を登録している本人」の「除外したサイトの閲覧履歴」とするが、初回に過去の全件が行く点が落ちている。`high` は妥当（本人が気づけない）だが、理由はこの組み合わせにある。
- 直し方の案: text / residual に「開発版の置き場の下に除外の登録は作られない・初回は過去の履歴の全件」を書く。
- 処置: fixed: IMP-01 の text・derived・if_accepted に「dev の下に除外は作られず常に空・初回は過去の全件」を書いた

## R3 — IMP-02 の閉じ方: 手当て（ST58 への申し送り）が tasks に無い
- 対象: impact.findings[IMP-02].options / verify / severity
- 種類: 閉じ方／重大度
- 指摘: option 1「ST58 へ『終了コード 4 も退避を取らない。戻さない』と申し送る」の実体は `docs/handoff/ST58.md` の書き換えだが、tasks.md に handoff / ST58 を書く task が grep で 1 つも無い（`docs/handoff/ST58.md:9-10` は 1 と 3 だけで、4 が無い）。申し送りの現状の文言「戻しは終了コード 1 の回だけ」に従えば起きない、は ST58 が申し送りを守る前提で、守らなかった帰結は ST58.md:9 自身が「捨てたものは復元できない」と書く（利用者が気づけない・配った後では直せない）。それで `medium` に置くなら、手当て（申し送りの追記）を tasks の 1 行にして実在させるべき。予定だけでは閉じない。
- 直し方の案: tasks に「`docs/handoff/ST58.md` に終了コード 4（退避なし・戻さない）を足す」を入れる。入れないなら重大度を再検討。
- 処置: fixed: reading の段は tasks を直せないので、IMP-02 を high にして if_deferred（ST58）と if_accepted を足した。後の Story で扱うと答えれば record が申し送りを構造から書く

## R4 — IMP-03 の derived は spec の否定（書く口が無い）の確認が足りない
- 対象: impact.findings[IMP-03].derived / unknowns
- 種類: 根拠
- 指摘: 「記録を移す口も、この change は持たない」は change の中の話で、リポジトリの他の口（書き出し・削除・取り込み系の `deletion.rs` など）で移せるかを「確かめていない」と自分で unknowns に書いている。それで `lost` / rule R1 の `legit` では「止まる」と断定し、`scope.excludes` で除外している。`blocks_legit: true` と整合するが、「他に道が無い」ことは確かめていないので、`medium` を supports するのは「手で DB を触る以外に道が無い」かどうかの確認。
- 直し方の案: 書き出し／取り込みの口の有無を 1 回確かめて unknowns を畳む。
- 処置: fixed: 書き出し（/export。lib.rs:4288）はあり、取り込みの口はルートに見つからないことを unknowns と evidence E14 に書いた。手で DB を触る以外の道は未確認のまま残した

## R5 — IMP-02 を high にした後の if_deferred: safe_now は成り立つが、interim の根拠は「申し送りの文を ST58 が守る」ことだけ
- 対象: impact.findings[IMP-02].if_deferred.interim / safe_now / severity
- 種類: 閉じ方
- 指摘: `safe_now`（自動で戻す処理はまだ無い）は実物と合う（`pg_restore` を使うのは `crates/server/tests/server_startup.rs` の試験だけ。`docs/handoff/ST58.md:9-10` も「ST58 が作る」）。R3 の直しで high にし if_deferred を足したことは、R1〜R4 の指摘を満たす。残るのは `interim.kind: existing` の中身で、ST58.md:10 の「戻しは終了コード 1 の回だけ」は 4 を名指ししておらず、4 を戻さない保証は ST58 の実装者がその一文を読むことに依る（機械では守られない）。同じ IMP の `unknowns` が「ST58 は上流に入っていない」と書くので、`safe_now` は「ST54 の間は安全」までで、ST58 の間は保証でない、と読める書き方のままにしておく必要がある（今の文言はそう読める）。`text` の「この change の tasks は申し送りを書き足さない」と `options` 2 の「tasks に追記を足す」が共存しており、人間が option 2 を選べば閉じる。
- 直し方の案: 重大度・閉じ方はこのままでよい。人間への提示で「ST58 が申し送りを読まなければ守られない」を interim の注に 1 句足す程度。
- 処置: fixed: IMP-02 の if_deferred.interim に「4 を名指ししていないので、ST58 の実装者がこの一文を読まなければ守られない」を足した。safe_now は ST54 の間に限った書き方のまま

（上記 R1〜R4 以外: T1〜T8 の searched の別の探し方で、stack.sh / dev.sh（rc 4 をそのまま返す）、`docs/openapi.json`（tasks 2.1 が作り直す）、ST50 の契約の試験は依存に見落としは見つからなかった。`BeforeLedger` の DB の扱いは design D3:53 が手当て済み。）


---

# 第 2 回（IMP-01・02 を reflected にした後）

IMP-01 の直しは実物と照らして成り立つ: 除外を読む口は `main.rs:81` と `history/collect.rs:315`（`HistoryCollector::new` の `state_dir.join("exclusions.json")`）の 2 つで、`runtime.rs:3928` は試験の書き込みだけ。`Runtime::new` は `&Config` を受ける（`runtime.rs:215`）ので、`exclusions_path` を持たせて `with_history`（`:310`）から渡す形は無理がない。tasks 6.1 の試験は `write_rules`（`cfg.state_dir` に書く。`runtime.rs:3924`）を使わず渡された置き場へ直接書き、`<置き場>/dev` に除外が無いことを assert するので、口を直さなければ `example.test/1` が送られて落ちる。IMP-02 の ST58 の requires への ST54 追加は `docs/stories/ST58.md`・`stories.json` に実在する。そのうえで次を指摘する。

## R6 — IMP-01・IMP-02 の text / derived / unknowns が直す前のまま、reflected と食い違う
- 対象: impact.findings[IMP-01].text / derived / unknowns、findings[IMP-02].text / unknowns
- 種類: 言い切り
- 指摘: IMP-01 の `derived` は「tasks 6.1 はその口を直さない。なので tasks のとおりなら履歴の除外は常に空」、`unknowns` は「下流の実装者が、tasks に無い履歴の取り込みの口も…直すか」のまま。今の tasks 6.1（「除外を読む口は 2 つとも」）と食い違い、unknowns の問いはもう無い。IMP-02 の `text` 末尾「この change の tasks は申し送りを書き足さない」は、tasks 3.2 が書き足すので今は偽。`unknowns` の「ST58 は上流に入っていない」も、requires に ST54 を足した後の読み方と合わせて書き直すべき。人間がこの欄を読むと、直したはずの帰結がまだ起きると読める。
- 直し方の案: text に「（直す前の状態）」と明記するか、現在形の文を直し後の状態にそろえる。unknowns は「直しが実装されるか」（下流の試験が見る）に替える。

- 処置: fixed: IMP-01 の text に「（直す前の状態）」を付け、derived を直す前と今の tasks 6.1 に分けて書き直し、unknowns を「直しが実装されるか（下流の試験が確かめる）」に替えた。IMP-02 の text の「この change の tasks は申し送りを書き足さない」を「直す前の tasks は書き足していなかった（今は tasks 3.2 が書き足す）」に、unknowns を requires に ST54 を足した後の読み方に直した
## R7 — IMP-01・02 の reflected_in / resolution が「直った」と言えるのは tasks の実装後で、仕様は未実装
- 対象: impact.findings[IMP-01].resolution / reflected_in、overview、findings[IMP-02].reflected_in
- 種類: 閉じ方／言い切り
- 指摘: overview は「PC の開発版の除外は…両方で、渡された置き場から読む（IMP-01 で直した）」と実現済みのように断定するが、`collect.rs:315` は今も `state_dir` から読む（コードは未変更。直すのは下流の tasks 6.1）。IMP-02 の `reflected_in` は決定 D3 だけで、手当ての実体である tasks 3.2（`docs/handoff/ST58.md` の追記）は挙げていない。`docs/handoff/ST58.md` にはまだ終了コード 4 の節が無い（grep で `channel_mismatch` 無し）。resolution の「ST58 の上流はこの申し送りが main に入ってから始まる」は、requires が「ST54 が main に入ってから」の意味であることと、3.2 が実際に実施されることの 2 つに依る。3.2 の検証は節見出しと語の grep のみで、「戻さない」という中身は見ない。
- 直し方の案: overview を「直すと仕様に書いた（実装は tasks 6.1）」に直す。IMP-02 の reflected_in に tasks 3.2 を足し、検証の grep に「戻さ」を含める。

- 処置: fixed: overview を「仕様に書いた（IMP-01 の直し。実装は下流の tasks 6.1）」に直し、IMP-02 の resolution に「申し送りの節はまだ書かれていない（下流の tasks 3.2 が書く）」を足した。tasks 3.2 の検証が中身を見ない点は IMP-02 の unknowns と GO 資料の open（未解決）に出した（reading の段は tasks を書き換えない）。kept: IMP-02 の reflected_in に tasks 3.2 は足さない —— reflected_in は Requirement / Scenario / 決定の形しか持てず、tasks 3.2 の中身は D3 が持つ
## R8 — 別の置き場を渡す場合の漏れが walkthrough にだけあり、帰結として持つ IMP も rule も無い
- 対象: impact.walkthroughs[1].steps「別の置き場 D:\try」(outcome: problem)、findings、scope.preconditions
- 種類: 新しい帰結／重大度
- 指摘: 直した後も、正式版の置き場と違う値を渡した開発版は、除外が空のまま初回に過去の履歴の全件を開発版のサーバへ送る（`exclusion.rs:112` は無いファイルを空として通す。エラーにならず、`exclusions_loaded` の件数 0 がログに出るだけ）。walkthrough は `problem` としているが、findings に無く、`situations` / `rules` にも無い。precondition に「同じ置き場を渡したときだけ」はあるが、本人が気づけない（利用者が自分で気づけない）という `high` の条件に当たる帰結が、決定 Q2 の読み替えの副作用として番号なしで残る。本人が Q2 を推奨で答えたことは別だが、その帰結（初回の全件が除外なしで行く）まで本人が見たかは go.json から分からない。
- 直し方の案: IMP として立てるか、`scope.residual` に「別の置き場を渡すと除外は空・初回は全件」を書いて Q2 の選択の帰結だと明示する。
- 処置: fixed: IMP-04（medium）として立て、根拠に E15（exclusion.rs:112）を足し、scope.residual と walkthroughs[1] の該当の例にも IMP-04 を書いた。high にしない理由: 送り先は本人の開発版のサーバ（外へ出ない）で、置き場は本人が選んで渡す。正式版と同じ置き場を渡せば起きず、要点 pc.3 が注意（直感注意・Privacy）として書いている
