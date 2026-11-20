# CH21 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
確認バッチを開く前に、その回で足した画面と PC の機能を AI が一度普通に使ってみて（PC は配る形で）、その結果（違和感の候補の件数と題）を手順書の冒頭で読めるようになる。手で好きなときに呼ぶ /tsukau も同じ立て方を使う。

<!-- prose impact.overview -->
確認バッチを作るたびに、その回に画面か PC を変えた PR があれば、AI が一度使い（PC は配る形を Sandbox に入れ、画面は専用の開発の形で立てる）、件数と題を手順書に載せる。代わりに、その回の準備は最大 6,540 秒延び、PC を変えた回は本人のデスクトップに Sandbox の窓が開く。網を切るので、外とつないで初めて出る不具合（#329 の型）はこの席では見つけられない。

<!-- prose impact.problem -->
2026-11-19、本人が配る形を自分の PC に入れて普通に使ったら、更新の確認がエラーを出し続けることがすぐ見つかった（#329）。いまの試験は Scenario を 1 つずつ確かめるだけで、本人が最初の利用者になっている。

<!-- prose uses[U1].title -->
確認バッチを開く前に、AI が先に触った結果を読む

<!-- prose uses[U1].trigger -->
PC の収集アプリと更新係を変えた PR が確認バッチに入った。自分で入れて触る前に、普通に使うと何か引っかかるかを知りたい。

<!-- prose uses[U1].outcome -->
自分で触る前に、どこを重点的に見るか・PR を直すかを決められる

<!-- prose uses[U1].without -->
本人が最初の利用者になり、配る形を入れて普通に使って初めて気づく（#329）

<!-- prose uses[U1].steps[0].do -->
確認バッチの手順書（PR 本文）の冒頭を開く

<!-- prose uses[U1].steps[0].see -->
起動の手順の後に「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」の段落

<!-- prose uses[U1].steps[1].do -->
「PC: 」の段落を読む

<!-- prose uses[U1].steps[1].see -->
「席が自分で終えた。違和感の候補 1 件」と題「設定の画面で更新の状態が読めない」（題は説明用）

<!-- prose uses[U1].steps[2].do -->
確認バッチの作業ツリーの dist/verify-<tag>/tsukau.log で実行のフォルダを引き、手元の RESULT.md を開く

<!-- prose uses[U1].steps[2].see -->
候補の本文・根拠・画面の写し

<!-- prose walkthroughs[W1].title -->
確認バッチ t1 の手順書に何が載るか

<!-- prose walkthroughs[W1].scene -->
本人が確認バッチ t1 の手順書を開く。t1 に何が入ったかで、席の段落が変わる。

<!-- prose walkthroughs[W1].why -->
相手は、その回に入れた PR が変えたファイルだけで決まる。PC の席は配る zip が無いと立たないので、組み立てに失敗すると回らない。どの場合も準備は止まらず、決まった文言の段落が足される。

<!-- prose walkthroughs[W1].figure.caption -->
t1 に入ったものと、画面・PC の段落（ID と件数は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
画面の席だけが回り、件数と題が載る。PC は「回さなかった」

<!-- prose walkthroughs[W1].cases[1].result -->
どちらの相手も範囲が空で、「回さなかった」が 2 つ載る

<!-- prose walkthroughs[W1].cases[2].result -->
PC の席は回らず、その回の PC の機能は AI に触られないまま本人に渡る

<!-- prose walkthroughs[W1].exceptions[0].text -->
組み立てに何分かかるかは確かめていない（下流の Task 1 で測る）

<!-- prose walkthroughs[W2].title -->
片付けに失敗した Sandbox が残ったとき

<!-- prose walkthroughs[W2].scene -->
確認バッチ t1 の準備の途中で WSL が落ち、PC の席の Sandbox が開いたまま残った。翌日、確認バッチ t2 を作る。

<!-- prose walkthroughs[W2].why -->
席は本人や録画の Sandbox を止めないように、別の Sandbox が動いていれば立てずに断る。残った自分の Sandbox も「別の Sandbox」に見える。

<!-- prose walkthroughs[W2].figure.caption -->
残った Sandbox と、その後の回（tag は説明用）

<!-- prose walkthroughs[W2].cases[0].result -->
自分の Sandbox だけを止め、次の回の席と録画は回る

<!-- prose walkthroughs[W2].cases[1].result -->
本人が窓を閉じるまで、以後の PC の席は「相手を立てられなかった」、PC の録画も撮れない

<!-- prose walkthroughs[W2].exceptions[0].text -->
WSL が落ちたときに Sandbox が残るかは確かめていない

<!-- prose model.caption -->
この change が一周させるもの

<!-- prose sections.makes.tldr -->
確認バッチのたびに、その回の画面と PC の機能を AI が一度使い（PC は配る形で）、件数と題を手順書に載せる。

<!-- prose sections.change.tldr -->
手順書の最後に席の段落が足され、準備が延び、PC を変えた回は Sandbox の窓が開く。

<!-- prose sections.lost.tldr -->
準備がすぐ終わらなくなり、PC を変えた回は本人のデスクトップに窓が開く。

<!-- prose sections.uses.tldr -->
自分で触る前に、AI が先に引っかかったところを手順書で読める。

<!-- prose sections.watch.tldr -->
CH21 だけの回では席は回らない。残った Sandbox は以後の PC の席を止める。

<!-- prose sections.findings.tldr -->
重大な 1 件（席が本人の .env に届きうる）に答えるまで GO にならない。中くらいは 5 件。

<!-- prose sections.rest.tldr -->
携帯・席の中身・issue は作らない。置き場の決め方は AI に任せた。残った問いは無い。

<!-- prose features[hook].summary -->
画面と PC の 2 つの相手を立てる口

<!-- prose features[hook].tldr -->
席が本物を立てる口を tools/tsukau/ に置く。相手は画面と PC の 2 つで、携帯は持たない。

<!-- prose points[hook.1].text -->
差し込み口は画面（web）と PC（pc）の 2 つの相手を持ち、立てる上限は 15 分。携帯は相手に持たない（後で足せる形）

<!-- prose points[hook.2].text -->
結果の置き場は repo の外の ~/.local/state/ashiato2/tsukau の 1 か所。どの作業ツリーから呼んでも同じ所に並ぶ

<!-- prose features[scope].summary -->
変えたファイルで振り分け、表題を渡す

<!-- prose features[scope].tldr -->
その回に入れた PR が変えたファイルで画面か PC かを決め、表題と価値の 1 行を範囲として渡す。

<!-- prose points[scope.1].text -->
画面のファイルを変えたものは画面へ、サーバ・PC の収集アプリ・更新係・インストーラ・移行・配る形の組み立てを変えたものは PC へ。両方なら両方

<!-- prose points[scope.2].text -->
範囲は「<ID> <表題>: <価値の最初の段落の 1 行目>」を並べたもの。引用や注記で始まる段落は飛ばし、無ければ表題だけ

<!-- prose points[scope.3].text -->
携帯だけ・文書や道具だけ・試験だけを変えたものはどちらにも入らない。CH21 だけの確認バッチでは席は回らず「回さなかった」が 2 つ載る

<!-- prose points[scope.4].text -->
確認バッチに後から足す、すでに main に入った Story は木の差に出ないので範囲に入らない

<!-- prose features[call].summary -->
相手ごとに 1 回、10 分・20 万 token

<!-- prose features[call].tldr -->
準備の最後に、範囲のある相手ごとに 1 回、10 分・20 万 token で席を呼ぶ。落ちても準備は止めない。

<!-- prose points[call.1].text -->
範囲のある相手だけを、10 分・20 万 token・ラベル verify-<tag> で 1 回ずつ呼ぶ。範囲が空の相手は呼ばない

<!-- prose points[call.2].text -->
席は確認バッチの HEAD を取り出した使い捨ての作業ツリーで走る。harness2 を繋ぎ、本人の .env は写さない。席が何を書いても確認バッチの作業ツリーには残らない

<!-- prose points[call.3].text -->
準備が延びる。PC の zip の組み立ては上限 2,400 秒、相手 1 つの呼び出しは上限 1,920 秒、全体は 6,540 秒で打ち切る。その間、確認バッチの作業ツリーの錠を握り続ける

<!-- prose points[call.4].text -->
PC の範囲があれば、席を呼ぶ前に確認バッチの木から配る zip を手元で組む。組めなければ PC の席は呼ばず、画面の席は呼ぶ

<!-- prose points[call.5].text -->
席がどう終わっても準備は rc=0 のまま続き、手順書に「使ってみる席: 呼べなかった」の 1 段落が足される。既存の段落の後に足すだけ

<!-- prose points[call.6].text -->
この change の終わりは、画面と PC を変えた偽の PR を重ねた使い捨ての確認バッチの木で、同じ台本を本物で 1 回通すところまで

<!-- prose features[report].summary -->
件数・題・止まり方だけ。issue は作らない

<!-- prose features[report].tldr -->
公開の repo に出るのは件数・題・止まり方だけ。候補の本文・根拠・画像・ログは手元に残る。

<!-- prose points[report.1].text -->
手順書の最後に、頭の 1 段落と画面・PC の段落を 1 つずつ足す。終えた相手は「席が自分で終えた。違和感の候補 2 件」と題の一覧

<!-- prose points[report.2].text -->
題はこの PC の利用者名・ホスト名・私設網の名前・.env の値・絶対パスの形を含むと「（伏せた）」になる。commit の検査に掛かる形も伏せ、検査を走らせられなければ全部伏せる

<!-- prose points[report.3].text -->
伏せなかった題は 1 行にし、80 字で切り、@ と #数字 を全角に替える（PR 本文で通知や issue の繋がりにしない）

<!-- prose points[report.4].text -->
issue は作らない。どの実行のフォルダに結果があるかは手元の tsukau.log にだけ書く

<!-- prose features[reuse].summary -->
同じ中身なら前の結果を写す

<!-- prose features[reuse].tldr -->
main と PR のコミットが前の回と同じなら、席を呼ばずに前の結果を写す。

<!-- prose points[reuse.1].text -->
main のコミット・相手・範囲の ID と PR のいちばん新しいコミットが前と同じなら、前の結果（件数・題・止まり方）を写す

<!-- prose points[reuse.2].text -->
前の回が「席が自分で終えた」「時間を使い切った」「token を使い切った」以外で終わっていたら、写さずに呼び直す

<!-- prose features[pc].summary -->
網を切った Sandbox に配る形を入れる

<!-- prose features[pc].tldr -->
まっさらな Windows Sandbox を網を切って起こし、配る形を入れて窓を開き、席に触らせる。

<!-- prose points[pc.1].text -->
網を切った Sandbox を 1 つ起こし、配る形のサーバを入れ、窓を開き、PC の収集アプリを利用者のセッションで起こしてから席に渡す

<!-- prose points[pc.2].text -->
網を切る。外へも私設網へも何も出ない代わりに、更新の確認は毎回「つながらない」になり、外とつないで初めて出る不具合はこの席では見えない

<!-- prose points[pc.3].text -->
PC を変えた回ごとに、Sandbox の窓が本人のデスクトップに開く（1 日 1〜5 回まで）

<!-- prose points[pc.4].text -->
別の Sandbox が動いている・配る zip が無いときは立てずに断る。片付けは自分の Sandbox だけを止め、止まらなければ非 0 で終わる

<!-- prose features[web].summary -->
専用の環境・乱数の合言葉・偽データ

<!-- prose features[web].tldr -->
画面は録画と同じく専用の環境で立てる。本人の合言葉と確認バッチの DB は使わない。

<!-- prose points[web.1].text -->
--repo の HEAD を取り出した別の木に、乱数の合言葉と専用の project 名を持つ .env を置いて立てる。--repo には何も書かない

<!-- prose points[web.2].text -->
案内に画面の URL・その回の合言葉・写しを撮るコマンド・触らないものを書く。片付けはその回の project だけを消す

<!-- prose impact.scenarios[S1].result -->
PC の段落は「相手を立てられなかった」。同じバッチの PC の録画も撮れない

<!-- prose impact.scenarios[S1].lost -->
本人が窓を閉じるまで、PC の席と PC の録画

<!-- prose impact.scenarios[S2].result -->
PC の段落は「配る zip を組めず、回さなかった」

<!-- prose impact.scenarios[S2].lost -->
その回の PC の機能を AI が触ること

<!-- prose impact.scenarios[S4].result -->
新しい回の席は前の回の id を知らないので閉じず、R1 で立てずに断る

<!-- prose impact.scenarios[S4].lost -->
本人が手で閉じるまでの PC の席と録画

<!-- prose impact.scenarios[S3].result -->
題は「（伏せた）」で、件数にだけ数えられる

<!-- prose impact.scenarios[S3].lost -->
手順書だけで中身を読むこと（手元の RESULT.md には残る）

<!-- prose impact.rules[R1].legit -->
本人が自分の Sandbox を使っている回・前の回の席が残した Sandbox がある回も、PC の席が回らない

<!-- prose impact.rules[R2].legit -->
組み立てが遅い・Windows 側の道具が無い回は、その回の PC の機能が AI に触られない

<!-- prose impact.rules[R3].legit -->
Sandbox の中のパス（C:\ProgramData\ashiato など）を書いた、私的でない題も伏せられる

<!-- prose impact.rules[R4].legit -->
時間内に終わらない正当な回は「時間の上限で打ち切った」になる

<!-- prose impact.rules[R5].legit -->
同じ中身でもう一度見たいときは、手で /tsukau を呼べる

<!-- prose impact.rules[R6].legit -->
携帯だけの回は席が回らないが、依頼で外にした

<!-- prose impact.rules[R7].legit -->
前の回が閉じ損ねた自分の Sandbox は誰も閉じない。残りは R1 で以後の PC の席と録画を止める（IMP-04）

<!-- prose impact.rules[R8].legit -->
準備がサーバか画面を組めなかった回は画面の席も回らない

<!-- prose impact.rules[R9].legit -->
回らなかった訳は手元の結果を開かないと分からない

<!-- prose impact.findings[IMP-01].title -->
網を切るので、#329 の型はこの席で見えない

<!-- prose impact.findings[IMP-01].text -->
PC の相手は網を切るので、外との握手で初めて起きる不具合（#329 の更新係の panic）はこの席では起きない。更新の確認は毎回「つながらない」になる

<!-- prose impact.findings[IMP-02].title -->
網を切った Sandbox で配る形が入るかは未確認

<!-- prose impact.findings[IMP-02].text -->
網を切った Sandbox に配る形のインストーラ・サーバ・DB が入り、中で収集アプリとサーバが通じるかを確かめていない。入らなければ PC の席は毎回立たない

<!-- prose impact.findings[IMP-03].title -->
準備が延び、その間は記録と fix が待つ

<!-- prose impact.findings[IMP-03].text -->
PC を変えた回は、配る zip の組み立て（上限 2,400 秒）と 2 つの席（1,920 秒ずつ）で準備が最大 6,540 秒延びる。その間、確認バッチの作業ツリーの錠を握り続け、録画と fix の撮り直し、ci_fix・conflict_fix の後の作り直し、次の確認バッチの自動の起動が待たされる

<!-- prose impact.findings[IMP-04].title -->
残った Sandbox が以後の PC の席と録画を止める

<!-- prose impact.findings[IMP-04].text -->
WSL や PC ごと落ちて片付けが走らないと、席の Sandbox が残る。以後の回は「別の Sandbox が動いている」で PC の席が立たず、PC の録画も撮れない。本人が閉じるまで続く

<!-- prose impact.findings[IMP-05].title -->
席はホストで動き、本人の .env に届きうる

<!-- prose impact.findings[IMP-05].text -->
席（claude）はホストの WSL で許しの形 auto で走る。使い捨ての作業ツリーに .env は写さないが、同じ git の共通の置き場から main の checkout の .env（本人の合言葉）や確認バッチの DB に辿れる。止めているのは案内の「触らないもの」だけ

<!-- prose impact.findings[IMP-06].title -->
私的でない題も伏せられることがある

<!-- prose impact.findings[IMP-06].text -->
絶対パスの形で伏せるので、Sandbox の中のパス（C:\ProgramData\ashiato など）を書いた題も伏せられる。gitleaks を用意できない回は全部の題が伏せられる

<!-- prose impact.findings[IMP-07].title -->
Sandbox の窓が本人のライフログに入りうる

<!-- prose impact.findings[IMP-07].text -->
PC を変えた回は Sandbox の窓が本人のデスクトップに開く。本人の PC で収集アプリが動いていれば、その間の前景の窓の記録に Sandbox の窓が入りうる

<!-- prose impact.scope.covers[0].text -->
確認バッチの準備から呼ぶ席と、手で呼ぶ /tsukau の、画面と PC の相手の立て方

<!-- prose impact.scope.preconditions[0].text -->
確認バッチの範囲はその回に「確認バッチ <tag> に入れる」merge で入った PR だけ。すでに main の Story は入らない

<!-- prose impact.scope.preconditions[1].text -->
PC の相手は、配る zip を組めたときだけ・ほかの Sandbox が動いていないときだけ立つ

<!-- prose impact.scope.preconditions[2].text -->
画面の相手は、準備が組んだサーバと画面の置き場があるときだけ立つ

<!-- prose impact.scope.preconditions[3].text -->
題は私的な形と commit の検査に掛かる形を伏せたものだけを写す

<!-- prose impact.scope.preconditions[4].text -->
同じ中身の回は、前の回が done・time・tokens のときだけ写す

<!-- prose impact.scope.preconditions[5].text -->
席の呼び出しが落ちても準備は止まらず、既存の段落の後に足すだけ

<!-- prose impact.scope.excludes[0].text -->
携帯の相手・席の中身・候補を直すこと・issue

<!-- prose impact.scope.excludes[1].text -->
外とつないで初めて出る不具合（CH22 の CI に任せる）

<!-- prose impact.scope.residual[0].text -->
伏せる網に掛からない私的な語（人や場所の名前）は題に出うる

<!-- prose impact.scope.residual[1].text -->
残った Sandbox が以後の PC の席と録画を止めうる（IMP-04）

