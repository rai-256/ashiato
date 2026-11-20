# CH21 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
確認バッチを開く前に、その回で足した画面と PC の機能を AI が一度普通に使ってみて（PC は配る形で）、見つけた違和感は直った状態で届くか、判断の要るものだけが手順書の問いとして届く。処置（選別・直す・聞く・依頼にする）は harness2 の確認バッチに依頼し、それが入るまで CH21 は終わらない。

<!-- prose impact.overview -->
確認バッチを作るたびに、その回に画面か PC を変えた PR があれば、AI が一度使い（PC は配る形を網を切った Sandbox に入れ、画面は専用の環境で立てる）、件数と伏せた題を手順書に載せる。見つけた候補は harness2 の処置が別の席で選別し、直すか本人に聞くか違和感の issue にする（依頼。入るまで CH21 は終わらず、席も回らない）。代わりに準備は最大 7,140 秒延び、PC を変えた回は本人のデスクトップに Sandbox の窓が開き、審査を通った PR に直しが後から載る。網を切るので、外とつないで初めて出る不具合（#329 の型）はこの席では見つけられない。

<!-- prose impact.problem -->
2026-11-19、本人が配る形を自分の PC に入れて普通に使ったら、更新の確認がエラーを出し続けることがすぐ見つかった（#329）。いまの試験は Scenario を 1 つずつ確かめるだけで、本人が最初の利用者になっている。見つけたものを手順書に件数で載せるだけでは、直すのも人間に回すのも本人の手になる。

<!-- prose uses[U1].title -->
確認バッチを開くと、AI が先に使って直した結果がある

<!-- prose uses[U1].trigger -->
画面と PC の収集アプリを変えた PR が確認バッチに入った。自分で入れて触る前に、普通に使うと何か引っかかるかを知りたい。

<!-- prose uses[U1].outcome -->
直った後のものを確かめ、判断の要るものだけを決めればよい

<!-- prose uses[U1].without -->
本人が最初の利用者になり、配る形を入れて普通に使って初めて気づく（#329）。見つけても、直すか依頼にするかを自分で回す

<!-- prose uses[U1].steps[0].do -->
確認バッチの手順書（PR 本文）の冒頭を開く

<!-- prose uses[U1].steps[0].see -->
「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」の段落と、「画面: 席が自分で終えた。違和感の候補 2 件」（件数は説明用）

<!-- prose uses[U1].steps[1].do -->
その下を読む（harness2 の処置が入った後の回）

<!-- prose uses[U1].steps[1].see -->
「C1 を直した」の 1 行と、C2 についての問い（直す・依頼にする・このまま・確認できない）

<!-- prose uses[U1].steps[2].do -->
C2 の問いに、手元の HTML のリンクから根拠を開いて答える

<!-- prose uses[U1].steps[2].see -->
候補の本文・画面の写し（手元の RESULT.md。公開には出ない）

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

<!-- prose walkthroughs[W3].title -->
見つけた 3 つの候補の行き先

<!-- prose walkthroughs[W3].scene -->
画面を変えた ST60 が確認バッチ t2 に入った。画面の席が C1〜C3 の 3 つの候補を書いた。harness2 の処置が入った後の回。

<!-- prose walkthroughs[W3].why -->
見つけた席とは別の席が根拠とその回の差分を読んで分ける。直すのは確認バッチの流れ 1 本で 3 件・同じ候補 1 回までで、迷えば本人に回す。処置は harness2 の確認バッチへの依頼で、ashiato2 は目録と題の台本と確かめ直しの口を渡す。

<!-- prose walkthroughs[W3].figure.caption -->
候補ごとの選別と行き先（ID・題・tag は説明用）

<!-- prose walkthroughs[W3].cases[0].result -->
依頼どおりに処置が動けば、本人が見る前に直り、確かめ直しの結果も段落に載る。Story のブランチには直しの commit が 1 つ増える（IMP-08）

<!-- prose walkthroughs[W3].cases[1].result -->
本物の違和感が件数の 1 行に埋もれ、本人は手元の結果を開かないと気づかない（IMP-09）

<!-- prose walkthroughs[W3].cases[2].result -->
本人が手順書で答える。「直す」なら今の fix と同じ道

<!-- prose walkthroughs[W3].exceptions[0].text -->
処置の振る舞いは harness2 への依頼で、ashiato2 の正典は約束しない

<!-- prose model.caption -->
この change が一周させるもの

<!-- prose sections.makes.tldr -->
確認バッチのたびに AI がその回の画面と PC を使う。直すか聞くかの処置は harness2 に依頼する。

<!-- prose sections.change.tldr -->
手順書の冒頭に席の段落が載り、処置が入った後は直したものの 1 行と問いも載る。

<!-- prose sections.lost.tldr -->
準備が最大 7,140 秒延び、審査を通った PR に、本人が見る前に直した変更が載ることがある。

<!-- prose sections.uses.tldr -->
確認バッチを開くと AI が先に使った結果がある。処置が入れば、直したものと問いも届く。

<!-- prose sections.watch.tldr -->
PC を変えた回は窓が開く。網を切るので #329 の型は見えない。処置が入るまで席は回らない。

<!-- prose sections.findings.tldr -->
重大な 1 件（席が本人の .env に届きうる）に答えるまで GO にならない。中くらいが 7 件。

<!-- prose sections.rest.tldr -->
携帯と席の中身と手で呼んだときの処置は作らない。置き場と試験の名前の書き方は AI に任せた。

<!-- prose features[hook].summary -->
席が本物を立てる口を tools/tsukau/ に置く。相手は画面と PC

<!-- prose features[hook].tldr -->
席が本物を立てる口を tools/tsukau/ に置く。相手は画面と PC で、携帯は持たない。

<!-- prose points[hook.1].text -->
相手は画面（web）と PC（pc）の 2 つ。立てる上限は 15 分。携帯は相手に持たず、後で足せる形にしておく

<!-- prose points[hook.2].text -->
結果の置き場は repo の外の ~/.local/state/ashiato2/tsukau の 1 か所。どの作業ツリーから呼んでも同じ所に並ぶ

<!-- prose points[hook.3].text -->
手で呼んだ /tsukau は結果を残すだけ。処置へ渡す目録を作らず、本人が RESULT.md を読んで /iwakan か /hanasu に回す

<!-- prose features[scope].summary -->
入れた PR が変えたファイルで相手を決め、表題と価値を渡す

<!-- prose features[scope].tldr -->
その回に入れた PR が変えたファイルで画面か PC かを決め、表題と価値の 1 行を範囲として渡す。

<!-- prose points[scope.1].text -->
画面のファイル（web/src/・web/index.html・web/public/）を変えたものは画面へ。サーバ・PC の収集アプリ・更新係・インストーラ・移行・配る形の組み立てを変えたものは PC へ。両方なら両方

<!-- prose points[scope.2].text -->
範囲は「<ID> <表題>: <何のためか>」を並べたもの。何のためかは価値の節の、引用と注記を飛ばした最初の段落。無ければ表題だけ

<!-- prose points[scope.3].text -->
携帯だけ・文書や道具だけ・試験だけを変えたものは、どちらにも入らない。CH21 だけの確認バッチでは席は回らず「回さなかった」が 2 つ載る

<!-- prose points[scope.4].text -->
確認バッチに後から足す、すでに main に入った Story は木の差に出ないので範囲に入らない

<!-- prose features[call].summary -->
準備の最後に、相手ごとに 10 分・20 万 token で 1 回呼ぶ

<!-- prose features[call].tldr -->
準備の最後に、範囲のある相手ごとに 10 分・20 万 token で席を 1 回呼ぶ。落ちても準備は止めない。

<!-- prose points[call.1].text -->
範囲のある相手だけを、10 分・20 万 token・ラベル verify-<tag> で 1 回ずつ呼ぶ。範囲が空の相手は呼ばない

<!-- prose points[call.2].text -->
席は確認バッチの HEAD を取り出した使い捨ての作業ツリーで走る。harness2 を繋ぎ、本人の .env は写さない。席が何を書いても確認バッチの作業ツリーには残らない

<!-- prose points[call.3].text -->
PC の範囲があれば、席を呼ぶ前に確認バッチの木から配る zip を手元で組む（上限 2,400 秒）。組めなければ PC の席は呼ばず、画面の席は呼ぶ

<!-- prose points[call.4].text -->
準備が延びる。相手 1 つの呼び出しは上限 2,220 秒、全体は 7,140 秒で打ち切る。その間、確認バッチの作業ツリーの錠を握り続け、録画と fix の撮り直しが待つ

<!-- prose points[call.5].text -->
席がどう終わっても準備は rc=0 のまま続き、手順書に「使ってみる席: 呼べなかった」の 1 段落が足される。既存の段落の後に足すだけ

<!-- prose features[report].summary -->
公開に出るのは止まり方・件数・番号つきの伏せた題だけ

<!-- prose features[report].tldr -->
手順書と verify PR の本文に載るのは、止まり方・件数・番号つきの題だけ。題は 1 つの台本で伏せる。

<!-- prose points[report.1].text -->
手順書の最後に、頭の 1 段落と「画面: 」「PC: 」の段落を 1 つずつ足す。終えた相手は「席が自分で終えた。違和感の候補 2 件」と、C1・C2 の番号つきの題

<!-- prose points[report.2].text -->
回らなかったときは止まり方ごとの決まった文言だけを書く。訳は手元の結果を開かないと分からない

<!-- prose points[report.3].text -->
題は tools/tsukau/public-title.sh 1 か所で伏せる。処置（harness2）が手順書の問い・issue・直しの commit・試験の名前に題を書くときも同じ台本を通す

<!-- prose points[report.4].text -->
伏せる判定を最後まで走らせられない回（gitleaks が無い など）は、全部の題を伏せる。前の回から写した題も通し直す

<!-- prose points[report.5].text -->
伏せなかった題は 1 行にし、80 字で切り、@ と #数字 を全角に替える（公開の repo で通知や issue の繋がりにしない）

<!-- prose features[handoff].summary -->
別の席が選別し、直すか本人に聞くか依頼にする。処置は harness2

<!-- prose features[handoff].tldr -->
見つけた候補は別の席が選別し、直すか本人に聞くか違和感の issue にする。処置は harness2 に依頼する。

<!-- prose points[handoff.1].text -->
準備は手元にだけ目録 dist/verify-<tag>/tsukau.json を書く。相手ごとの状態（呼んだ・写した・回さなかった・呼べなかった）・範囲・結果の置き場だけで、候補の本文は持たない。自分では直さず issue も作らない

<!-- prose points[handoff.2].text -->
処置は harness2 の確認バッチへの依頼にする。見つけた席とは別の席が選別し、この回が原因で直し方が明らかなら直す、判断が要れば手順書の問い、前からなら違和感の issue、誤検知は件数だけ

<!-- prose points[handoff.3].text -->
処置は本人が見る前に直す。統合の CI を直すときと同じ道で、直した変更を審査を通った Story のブランチに載せ、バッチを作り直す。本人は直った後のものを見て、手順書の 1 行で直したと知る

<!-- prose points[handoff.4].text -->
選別に迷えば本人に聞く。直すのは 1 本の流れで 3 件まで・同じ候補は 1 回まで。それを越えたもの・Story の作業場所が汚れていて載せられないものも本人に聞く

<!-- prose points[handoff.5].text -->
直した後は、処置が TSUKAU_RECHECK で直した候補を渡し、その題だけを範囲に 5 分・10 万 token で確かめ直す。前の結果は写さない

<!-- prose points[handoff.6].text -->
公開の repo には題・種類・どの Story・回・番号だけを書く。直しの commit の本文・足す試験の名前と中の文字列・Story の issue へのコメントも同じ。根拠は本人に渡す手元の HTML から開く

<!-- prose points[handoff.7].text -->
CH21 の終わりは、harness2 の処置が入った後に、使い捨ての確認バッチの木で処置まで本物で通すこと。それまで CH21 は main に入らず、確認バッチで席は回らない

<!-- prose features[reuse].summary -->
中身が前の回と同じなら、席を呼ばずに前の結果を写す

<!-- prose features[reuse].tldr -->
確認バッチを作り直しても、中身が前と同じなら席を呼ばず前の結果を写す。直しが入れば呼び直す。

<!-- prose points[reuse.1].text -->
main のコミット・相手・範囲の ID と PR のいちばん新しいコミット・確認バッチの木が前と同じなら、前の結果（件数・題・止まり方）を写し、「前の回 <tag> と同じ中身なので、その結果を写した」と書く

<!-- prose points[reuse.2].text -->
前の回が「席が自分で終えた」「時間を使い切った」「token を使い切った」以外で終わっていたら、写さずに呼び直す

<!-- prose features[pc].summary -->
網を切ったまっさらな Sandbox に配る形を入れ、窓を開いて触らせる

<!-- prose features[pc].tldr -->
まっさらな Windows Sandbox を網を切って起こし、配る形を入れ、窓を開いて席に触らせる。

<!-- prose points[pc.1].text -->
網を切った Sandbox を 1 つ起こし、その場の合言葉で配る形のサーバを入れ、窓を開き、PC の収集アプリを利用者のセッションで起こしてから席に渡す

<!-- prose points[pc.2].text -->
網を切る。外へも私設網へも何も出ない代わりに、更新の確認は毎回「つながらない」になり、外とつないで初めて出る不具合はこの席では見えない（CH22 の CI に任せる）

<!-- prose points[pc.3].text -->
PC を変えた回ごとに、Sandbox の窓が本人のデスクトップに開く（1 日 1〜5 回まで、10 分ほど）

<!-- prose points[pc.4].text -->
別の Sandbox が動いている・配る zip が無いときは立てずに断る。片付けは中のログを写してから自分の Sandbox だけを止め、止まらなければ非 0 で終わる

<!-- prose features[web].summary -->
録画と同じく専用の環境で立て、本人の合言葉と確認バッチの DB を使わない

<!-- prose features[web].tldr -->
画面は録画と同じく専用の環境と偽データで立てる。本人の合言葉と確認バッチの DB は使わない。

<!-- prose points[web.1].text -->
--repo の HEAD を取り出した別の木に、乱数の合言葉と専用の project 名を持つ .env を置いて立てる。--repo には何も書かない

<!-- prose points[web.2].text -->
確認バッチでは準備が組んだサーバと画面を使い、無ければ立てずに断る。手で呼んだときは別の木で組む。案内に URL・その回の合言葉・写しを撮るコマンド・触らないものを書く

<!-- prose impact.scenarios[S1].result -->
PC の段落は「相手を立てられなかった」。同じバッチの PC の録画も撮れない

<!-- prose impact.scenarios[S1].lost -->
本人が窓を閉じるまで、PC の席と PC の録画

<!-- prose impact.scenarios[S2].result -->
PC の段落は「配る zip を組めず、回さなかった」

<!-- prose impact.scenarios[S2].lost -->
その回の PC の機能を AI が触ること

<!-- prose impact.scenarios[S3].result -->
題は「（題は手元の結果に）」で、件数にだけ数えられる

<!-- prose impact.scenarios[S3].lost -->
手順書・問い・issue の題だけで中身を読むこと（手元の RESULT.md には残る）

<!-- prose impact.scenarios[S4].result -->
新しい回の席は前の回の id を知らないので閉じず、R1 で立てずに断る

<!-- prose impact.scenarios[S4].lost -->
本人が手で閉じるまでの PC の席と録画

<!-- prose impact.scenarios[S5].result -->
審査を通った ST60 の PR に直しの commit が 1 つ増え、バッチが作り直され、席が確かめ直す

<!-- prose impact.scenarios[S5].lost -->
審査を通したときの PR の中身のまま merge すること

<!-- prose impact.scenarios[S6].result -->
確認バッチの準備は今までどおりで、席の段落は載らない

<!-- prose impact.scenarios[S6].lost -->
harness2 の処置が入るまで、確認バッチで席が回ること

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
処置（harness2）が入るまで、見つけた候補は手元の結果に残るだけ

<!-- prose impact.rules[R10].legit -->
審査を通した PR の中身が、本人が見る前に変わる（IMP-08）

<!-- prose impact.rules[R11].legit -->
4 件目からは直せても本人に聞く（依頼の文面。harness2 の処置がそう動くかは確かめていない）

<!-- prose impact.rules[R12].legit -->
その間、確認バッチで席が回らない（手で /tsukau を呼ぶことはできる）

<!-- prose impact.rules[R13].legit -->
回らなかった訳や候補の中身は、手元の結果を開かないと分からない

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
PC を変えた回は、配る zip の組み立て（上限 2,400 秒）と 2 つの席（2,220 秒ずつ）で準備が最大 7,140 秒延びる。処置が直した回は、作り直しと確かめ直し（5 分）の分もさらに延びる。その間、確認バッチの作業ツリーの錠を握り続け、別の経路の記録・fix・ci_fix・conflict_fix の後の作り直しが錠を待つ。同じ流れの録画は準備の後に続くので、延びた分だけ遅れて始まる

<!-- prose impact.findings[IMP-04].title -->
残った Sandbox が以後の PC の席と録画を止める

<!-- prose impact.findings[IMP-04].text -->
WSL や PC ごと落ちて片付けが走らないと、席の Sandbox が残る。以後の回は「別の Sandbox が動いている」で PC の席が立たず、PC の録画も撮れない。本人が閉じるまで続く

<!-- prose impact.findings[IMP-05].title -->
席はホストで動き、本人の .env に届きうる

<!-- prose impact.findings[IMP-05].text -->
席（claude）はホストの WSL で許しの形 auto で走る。使い捨ての作業ツリーに .env は写さないが、同じ git の共通の置き場から main の checkout の .env（本人の合言葉）や確認バッチの DB に辿れる。確認バッチの DB には確認の間に本人の携帯が送った記録が入りうる（偽データではない）。止めているのは案内の「触らないもの」だけ。CH21 で新しく増えるのは、確認バッチごとの自動の呼び出しと、Sandbox のログを読む選別の席（依頼）。同じ host で auto の claude を走らせる口が他にもあるかは確かめていない

<!-- prose impact.findings[IMP-06].title -->
私的でない題も伏せられることがある

<!-- prose impact.findings[IMP-06].text -->
絶対パスの形で伏せるので、Sandbox の中のパス（C:\ProgramData\ashiato など）を書いた題も伏せられる。gitleaks を用意できない回は全部の題が伏せられる。伏せた題は、本人に聞く問いや違和感の issue の題としても読めない

<!-- prose impact.findings[IMP-07].title -->
Sandbox の窓が本人のライフログに入りうる

<!-- prose impact.findings[IMP-07].text -->
PC を変えた回は Sandbox の窓が本人のデスクトップに開く。本人の PC で収集アプリが動いていれば、その間の前景の窓の記録に Sandbox の窓が入りうる

<!-- prose impact.findings[IMP-08].title -->
審査を通った PR に、本人が見る前に直した変更が載る

<!-- prose impact.findings[IMP-08].text -->
処置で直した変更は、統合の CI を直すときと同じく、審査を通った Story / change のブランチへ載る（依頼。harness2 の処置が入った後）。先例の統合の CI の直し（ci_fix）は CI を待つだけで審査をやり直さない（E14 の台本）。本人が直しの差分を見られるのは verify PR の diff で、手順書は 1 行で知らせるだけ

<!-- prose impact.findings[IMP-09].title -->
選別を誤ると、要らない直しが載るか本物が流れる

<!-- prose impact.findings[IMP-09].text -->
選別の席（AI）が誤検知を「この回が原因」とすれば要らない直しが Story に載り、本物を「誤検知」とすれば件数の 1 行に埋もれる。実地で確かめるときも、用意した候補が「本人に聞く」に倒れることがある

<!-- prose impact.scope.covers[0].text -->
確認バッチの準備から呼ぶ席と、手で呼ぶ /tsukau の、画面と PC の相手の立て方

<!-- prose impact.scope.covers[1].text -->
処置へ渡す手元の目録・公開の題の台本・確かめ直しの口（処置そのものは harness2）

<!-- prose impact.scope.preconditions[0].text -->
確認バッチの範囲はその回に「確認バッチ <tag> に入れる」merge で入った PR だけ。すでに main の Story は入らない

<!-- prose impact.scope.preconditions[1].text -->
PC の相手は、配る zip を組めたときだけ・ほかの Sandbox が動いていないときだけ立つ

<!-- prose impact.scope.preconditions[2].text -->
画面の相手は、準備が組んだサーバと画面の置き場があるときだけ立つ（手で呼んだときは別の木で組む）

<!-- prose impact.scope.preconditions[3].text -->
題は私的な形と commit の検査に掛かる形を伏せたものだけを写す。手順書に写すのは止まり方・件数・番号つきの題だけ

<!-- prose impact.scope.preconditions[4].text -->
同じ中身の回は、前の回が done・time・tokens のときだけ写す

<!-- prose impact.scope.preconditions[5].text -->
確かめ直しは、処置が TSUKAU_RECHECK を渡したときだけ、直した候補だけを範囲にする

<!-- prose impact.scope.preconditions[6].text -->
席の呼び出しが落ちても準備は止まらず、既存の段落の後に足すだけ

<!-- prose impact.scope.preconditions[7].text -->
目録は手元にだけ書き、準備は自分では直さず issue も作らない

<!-- prose impact.scope.preconditions[8].text -->
処置が直すのは 3 件・1 回まで、迷えば本人へ（harness2 への依頼が約束し、ashiato2 の正典は約束しない）

<!-- prose impact.scope.excludes[0].text -->
携帯の相手・席の中身（触り方・候補の書き方）・手で呼んだ席の処置

<!-- prose impact.scope.excludes[1].text -->
外とつないで初めて出る不具合（CH22 の CI に任せる）

<!-- prose impact.scope.excludes[2].text -->
選別・直し・本人への問い・違和感の issue の台本（harness2 への依頼）

<!-- prose impact.scope.residual[0].text -->
伏せる網に掛からない私的な語（人や場所の名前）は題に出うる

<!-- prose impact.scope.residual[1].text -->
残った Sandbox が以後の PC の席と録画を止めうる（IMP-04）

<!-- prose impact.scope.residual[2].text -->
審査を通った PR に、本人が見る前に直した変更が載る（IMP-08）

