# ST42 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
衛星（本体の外で作るアプリ）の作り手が、リポジトリのページを読めば何をどう呼べばよいか分かり、本体を更新しても同じ版の中で衛星が黙って壊れないようにする。

<!-- prose impact.overview -->
衛星の作り手がリポジトリのページから口・型・資格情報・断り・版を読めるようになり、衛星は /satellite/v1/me で自分と受ける版を確かめられる。CI が同じ版の中の壊す変更を止めるのは、決めた規則の表に当たる差だけ。代わりに、版 1 は main に入った日から同じ版の中では消す・変えるができなくなり、前の版を外すには版の一覧の notice に正典の Requirement の名前が要る。

<!-- prose impact.problem -->
衛星は本体と別に作られて別に更新される。本人が本体の口を直したとき、衛星が使う口まで黙って変わると、衛星が書くはずだった記録は壊れている間取れない。いまは衛星向けの契約も版も無く、どの変更が衛星を壊すか見分けられない。

<!-- prose uses[U1].title -->
読書記録の衛星を本体につなぐ

<!-- prose uses[U1].trigger -->
本人が読書記録の衛星（dev.example.booklog）を作り始める。どの道を、どの資格情報で呼べばよいか知りたい。

<!-- prose uses[U1].outcome -->
資格情報が通ることと、サーバが版 1.0 を受けることが分かり、版 1 の契約からクライアントを作り始められる

<!-- prose uses[U1].without -->
衛星の資格情報ではどの口も 403 で、設定できたかは管理の画面の「最後に使われた時刻」でしか分からない。口と型の契約も無い

<!-- prose uses[U1].steps[0].do -->
リポジトリの docs/satellite-api/README.md をブラウザで開く

<!-- prose uses[U1].steps[0].see -->
許諾 Apache-2.0、版の表（1.0・外す日なし）、版 1 のページへのリンク

<!-- prose uses[U1].steps[1].do -->
版 1 のページで資格情報の渡し方と確かめる口を読む

<!-- prose uses[U1].steps[1].see -->
Authorization: Bearer ashsat_… と GET /satellite/v1/me、断りの種別の表

<!-- prose uses[U1].steps[2].do -->
ST29 で発行した資格情報を衛星に設定し、GET /satellite/v1/me を呼ぶ

<!-- prose uses[U1].steps[2].see -->
200 で id dev.example.booklog・名前と、api_versions の 1.0

<!-- prose uses[U1].steps[3].do -->
資格情報を写し間違えて呼ぶ

<!-- prose uses[U1].steps[3].see -->
401 {"error": "unauthorized"}

<!-- prose uses[U2].title -->
本体を更新しても衛星が黙って壊れない

<!-- prose uses[U2].trigger -->
本人が本体の内部の口を直す PR を出す。衛星が使う口に響いていないか気になる。

<!-- prose uses[U2].outcome -->
衛星が頼る版 1 の形が、main に入る前に守られる

<!-- prose uses[U2].without -->
衛星向けの口が変わっても気づかず、衛星が書けない間の記録が取れない

<!-- prose uses[U2].steps[0].do -->
内部の口だけを変えた PR を出す

<!-- prose uses[U2].steps[0].see -->
衛星向けの契約の検査は通る

<!-- prose uses[U2].steps[1].do -->
うっかり衛星向けの確かめる口の欄 name を消した PR を出す

<!-- prose uses[U2].steps[1].see -->
CI が落ち、当たった規則（応答の欄を消す）と場所が出る

<!-- prose walkthroughs[W1].title -->
版 1 の欄を変えたくなったとき

<!-- prose walkthroughs[W1].scene -->
版 1 を main に入れた後、衛星がまだ無いうちに、本人が確かめる口の欄 name を display_name に変えたくなる。

<!-- prose walkthroughs[W1].why -->
比べる相手は main にある版 1 の写しで、衛星がいるかどうかは見ない。版 1 は main に入った日から凍る（Q3）。

<!-- prose walkthroughs[W1].figure.caption -->
PR の中身と CI の結果（欄の名前は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
同じ版で欄を消すことになり CI が落ちる。衛星が 1 つも無くても同じ

<!-- prose walkthroughs[W1].cases[1].result -->
通る。古い衛星は name を読み続けられる

<!-- prose walkthroughs[W1].exceptions[0].text -->
欄の名前は説明用の値。実際に変えたい欄があるかは確かめていない

<!-- prose walkthroughs[W2].title -->
版 2 を出して版 1 を外そうとする

<!-- prose walkthroughs[W2].scene -->
本人が壊す変更を版 2 として出し、衛星を全部版 2 に移した後、版 1 の口を消す PR を出す。

<!-- prose walkthroughs[W2].why -->
衛星の記録は途絶の通知に乗らないので、黙って版を外すと本人は格子の空白で初めて気づく。外す前に本人へ知らせる画面を条件にした（Q8）。

<!-- prose walkthroughs[W2].figure.caption -->
外す PR の結果（日付は説明用）

<!-- prose walkthroughs[W2].cases[0].result -->
外す変更は CI が止める。知らせの画面を作る Story は今どこにも無い

<!-- prose walkthroughs[W2].cases[1].result -->
通る。2 つの版が並んで動き続ける

<!-- prose walkthroughs[W2].exceptions[0].text -->
版 2 も印もまだ無い。日付は説明用の値

<!-- prose model.caption -->
この Story が作るもの

<!-- prose sections.makes.tldr -->
衛星が使う口だけの契約を版 1.0・Apache-2.0 で公開し、同じ版の壊す変更を CI が止める。

<!-- prose sections.change.tldr -->
衛星の資格情報で、衛星向けの道（いまは確かめる口だけ）が通るようになる。

<!-- prose sections.lost.tldr -->
版 1 の口と型を同じ版で変えられなくなり、README だけの PR でも重い検査を待つ。

<!-- prose sections.uses.tldr -->
衛星を作るとき、ページを読んで資格情報を設定し、確かめる口で通るかを見られる。

<!-- prose sections.watch.tldr -->
版 1 は main に入った日から凍る。無い版への 404 は記録に forbidden で残る。

<!-- prose sections.findings.tldr -->
中くらいの 5 件（版 1 が早く凍る・知らせの画面の持ち主が無い・URL の基点など）と軽い 1 件。重大は無い。

<!-- prose sections.rest.tldr -->
封筒・書く口・承認・読み出し・知らせの画面・SDK は作らない。版の一覧の置き場は AI に任せた。

<!-- prose features[contract].summary -->
衛星向けの契約を内部の口と別に作り、Apache-2.0 にする

<!-- prose features[contract].tldr -->
衛星が使う口だけの契約を別に作り、Apache-2.0 でリポジトリのページにする。

<!-- prose points[contract.1].text -->
衛星向けの契約は docs/satellite-api/v1/openapi.json に置き、許諾の欄は Apache-2.0。内部の口の契約の許諾は AGPL のまま

<!-- prose points[contract.2].text -->
資格情報の渡し方（Authorization: Bearer ashsat_…）と、401・403 の断り（種別の欄 error を持つ JSON）を、すべての口に載せる

<!-- prose points[contract.3].text -->
読むページは Markdown 2 枚（入口と版ごと）。GitHub がブラウザで整形して見せる。Apache-2.0 の全文 LICENSE を隣に 1 枚置く

<!-- prose points[contract.4].text -->
契約とページを手で直してもコードと違えば CI が落ちる。直すのはコードの側で、写しは作り直す

<!-- prose points[contract.5].text -->
Apache-2.0 で公開した後に許諾を締める向きには変えられない（緩める向きにだけ変えられる。扉 #31）

<!-- prose features[version].summary -->
道の先頭に大きい版を持ち、版の一覧を 1 つのファイルに持つ

<!-- prose features[version].tldr -->
衛星の口は /satellite/v1/… の形で、版は 1.0 から製品の版と別に数える。

<!-- prose points[version.1].text -->
衛星向けの口の道は /satellite/v1/ で始まる。読み出しの記録の口の名前にも版が残る

<!-- prose points[version.2].text -->
契約の版は 1.0 の形で、製品の版（出所の版）や送信契約の版とは別に数える

<!-- prose points[version.3].text -->
版の一覧・契約の写し・サーバの経路が食い違うと CI が落ちる

<!-- prose features[me].summary -->
衛星が自分の識別子と、サーバが受ける版を確かめる

<!-- prose features[me].tldr -->
GET /satellite/v1/me で、衛星の識別子・名前と受ける版の一覧だけが返る。

<!-- prose points[me.1].text -->
200 の本文は {satellite: {id, name}, api_versions: [{version: "1.0", sunset: null}]} だけ

<!-- prose points[me.2].text -->
資格情報が無い・取り消した・外した衛星は 401 {error: unauthorized}。どの衛星向けの道でも同じ

<!-- prose points[me.3].text -->
API の合言葉と画面のログインでは衛星向けの口は通らない（403 not_satellite）。ST29 の衛星の管理の口 /satellites… は衛星向けの道に数えない

<!-- prose points[me.4].text -->
衛星の資格情報でほかの口を呼ぶと、今どおり 403 で断る。通るのは衛星向けの契約の口だけで、記録を読む口も書く口もまだ無い

<!-- prose features[check].summary -->
同じ版の中の壊す変更と、小さい版の上げ忘れを CI が落とす

<!-- prose features[check].tldr -->
main にある版の写しと比べ、同じ版で口や欄を消す・変えると CI が落ちる。

<!-- prose points[check.1].text -->
落ちるのは、口・状態の番号・応答の欄を消す、型・形式を変える、送る側に必須を足す・選べる値を減らす、資格情報・許諾を変える、のどれか

<!-- prose points[check.2].text -->
口・応答の欄・任意の送る欄を足したら、小さい版を上げれば通る

<!-- prose points[check.3].text -->
比べる相手は main にある版ごとの写し。写しを書き換えても、比べる相手は行き先の写しなので通らない

<!-- prose points[check.4].text -->
内部の口だけの変更では落ちない。一方、契約の置き場や README・CONTRIBUTING だけの PR でも、サーバの検査の job が走る

<!-- prose points[check.5].text -->
版 1 は main に入った日から凍る。衛星がまだ 1 つも無くても、確かめる口の欄や断りの形は同じ版の中で変えられない

<!-- prose points[check.6].text -->
規則の表に無い壊し方（参照の付け替え・説明だけで意味を変える など）は見逃しうる。見つけたら規則を足す

<!-- prose features[retire].summary -->
壊す変更は次の版で出し、前の版は 6 か月以上と知らせの後でしか外さない

<!-- prose features[retire].tldr -->
版 2 を出しても版 1 は並べて動かし、外す日は Sunset の見出しで予告する。

<!-- prose points[retire.1].text -->
壊す変更は /satellite/v2/… と docs/satellite-api/v2/ として足し、版 1 を残せば検査は通る

<!-- prose points[retire.2].text -->
版 1 を外せるのは、版 2 を最初に配った日から 6 か月たち、版の一覧の notice が正典の Requirement の名前を持つときだけ

<!-- prose points[retire.3].text -->
配った日は印を作った日と印の commit の日の遅い方。同じ日が無い月は月の最終日まで外せない

<!-- prose points[retire.4].text -->
サーバの経路から前の版の口を消しても、外す変更として止まる。今の木ではどの版も外す日を持たない

<!-- prose points[retire.5].text -->
無い版の道は 404 unsupported_version と版の一覧、版の中の無い道は 404 not_found、違うメソッドは 405 method_not_allowed

<!-- prose points[retire.6].text -->
資格情報の判定が版の判定より先。資格情報の無い求めは無い版の道でも 401。外す日を持つ版の道では、401・403 にも Sunset が付く

<!-- prose features[log].summary -->
衛星向けの道への求めを読み出しの記録に 1 行ずつ残す

<!-- prose features[log].tldr -->
衛星向けの道の求めは返した状態と経路の形で読み出しの記録に 1 行残る。

<!-- prose points[log.1].text -->
確かめる口の 200 は ok、401 は unauthorized、403・404・405 は forbidden として、実際の状態の番号と一緒に残る

<!-- prose points[log.2].text -->
無い版や無い道への 404 も、結果の語は forbidden で残る（移行を足さないため。状態の列は 404）

<!-- prose points[log.3].text -->
記録に書けないときは、確かめる口は 500 {error: internal} で、衛星の情報も版の一覧も返さない

<!-- prose impact.scenarios[S1].result -->
CI が落ちる。直すには欄を足して 1.1 にするか、版 2 を出す

<!-- prose impact.scenarios[S1].lost -->
版 1 の形を直接直すこと

<!-- prose impact.scenarios[S2].result -->
CI が落ちる。知らせの画面の Requirement の名前を書くまで版 1 は並び続ける

<!-- prose impact.scenarios[S2].lost -->
前の版を外して、保つ口を減らすこと

<!-- prose impact.rules[R1].legit -->
衛星がまだ 1 つも無い間に、本人が版 1 の欄の名前や形の誤りを直すことも止まる

<!-- prose impact.rules[R2].legit -->
衛星を全部新しい版に移し終えた本人も、版の一覧の notice に正典の Requirement の名前を書くまで前の版を外せない

<!-- prose impact.rules[R3].legit -->
収集アプリも画面も衛星向けの口を使わないので、この分析では正当な利用を止めるものを見つけていない

<!-- prose impact.rules[R4].legit -->
なし（版の一覧と種別が応答に付くので、衛星は移り先を知れる）

<!-- prose impact.rules[R5].legit -->
衛星向けの口のほかに衛星が使う口はまだ無いので、この分析では見つけていない

<!-- prose impact.rules[R6].legit -->
DB が書けない間は確かめる口も使えない（内部の口と同じ扱い）

<!-- prose impact.findings[IMP-01].title -->
版 1 は衛星が無いうちから凍る

<!-- prose impact.findings[IMP-01].text -->
版 1 は main に入った日から凍る。衛星がまだ 1 つも無くても、確かめる口や断りの形の誤りは版 2 にしないと直せず、版 2 にすると版 1 は外せないまま並ぶ（IMP-02）

<!-- prose impact.findings[IMP-02].title -->
外す条件の知らせの画面を持つ Story がまだ無い

<!-- prose impact.findings[IMP-02].text -->
前の版を外すには、版の一覧の notice に正典の Requirement の名前が要る。本人に知らせる画面の Requirement を持つ Story はまだ無いので、版 2 を出しても版 1 は並び続ける

<!-- prose impact.findings[IMP-03].title -->
CONTRIBUTING の冒頭と古い検査が書き足しと食い違う

<!-- prose impact.findings[IMP-03].text -->
CONTRIBUTING の冒頭は「貢献も本体と同じ AGPL-3.0-only で配布」と言い、ライセンスの節に足す Apache-2.0 の記述と二通りに読める。ST53 の check-cla-section.sh もライセンスの節の変更で落ちる（CI には無い）

<!-- prose impact.findings[IMP-04].title -->
衛星向けの検査の比べる相手が changes の job とずれうる

<!-- prose impact.findings[IMP-04].text -->
rust の job の衛星向けの検査は、changes の job の比べる相手の式を写して使う。ST61 が main への push の比べる相手を変えても写しは追随しない。比べる相手を引けないとき、changes の job は全部走らせるが、衛星向けの検査は落とすので、main の push が赤になりうる

<!-- prose impact.findings[IMP-05].title -->
6 か月の起点にベータ版の印も入りうる

<!-- prose impact.findings[IMP-05].text -->
外せる条件の配った印は v* のすべてで、ST61 が作るベータ版の印（-beta.N）も含みうる。正式版だけを追う衛星の作り手には、移る期間が 6 か月より短くなる

<!-- prose impact.findings[IMP-06].title -->
衛星が呼ぶ URL の基点（/api の有無）が契約に無い

<!-- prose impact.findings[IMP-06].text -->
サーバは /api を剥がして経路に当てる（strip_api_prefix）。衛星向けの契約に servers の記述が無く、衛星が /satellite/v1/me と /api/satellite/v1/me のどちらで呼ぶかが契約から決まらない。Sunset を付ける層がこの剥がしの内か外かも書いていない

<!-- prose impact.scope.covers[0].text -->
衛星向けの契約（版 1）・確かめる口・版の受け口・衛星向けの道の読み出しの記録

<!-- prose impact.scope.covers[1].text -->
同じ版の中の壊す変更のうち、規則の表（design D9）に当たるもの

<!-- prose impact.scope.preconditions[0].text -->
読み出しの記録は追記のみ。行は書き換えも削除もしない（今のまま）

<!-- prose impact.scope.preconditions[1].text -->
衛星向けの道は /satellite そのものか /satellite/ で始まる道だけ。ST29 の管理の口 /satellites… は含まない

<!-- prose impact.scope.preconditions[2].text -->
外せる条件の 6 か月は印 v* を読める rust の job でだけ計算する。印がまだ無ければ外す条件は満たさない

<!-- prose impact.scope.preconditions[3].text -->
比べる相手にその版の写しが無いとき（その版を初めて足す PR）は比べずに通す

<!-- prose impact.scope.preconditions[4].text -->
Sunset は外す日を持つ版の道の応答だけに付く

<!-- prose impact.scope.excludes[0].text -->
衛星向けの契約には衛星だけが使う口を載せ、内部の口は載せない。版の一覧に載る版の道だけを受ける

<!-- prose impact.scope.excludes[1].text -->
記録の封筒の形・衛星が書く口（ST39）・承認（ST38）・衛星の読み出し（ST45）・MCP の道具（ST27）・SDK

<!-- prose impact.scope.excludes[2].text -->
本人に知らせる画面（どの Story も持っていない）

<!-- prose impact.scope.residual[0].text -->
規則の表に無い壊し方は見逃しうる

<!-- prose impact.scope.residual[1].text -->
版 1 は外せないまま並びうる（IMP-02）

