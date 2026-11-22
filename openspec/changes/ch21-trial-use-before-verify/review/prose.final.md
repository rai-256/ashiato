# CH21 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
確認バッチを開く前に、その回で足した画面・PC・携帯の機能を AI が一度普通に使ってみて（PC は配る形、携帯はエミュレータで）、見つけた違和感は直った状態で届くか、判断の要るものだけが手順書の問いとして届く。処置（選別・直す・聞く・依頼にする）は harness2 の確認バッチに依頼し、それが入るまで CH21 は終わらない。

<!-- prose impact.overview -->
確認バッチを作るたびに、その回に画面・PC・携帯を変えた PR があれば、AI が一度使い（PC は配る形を Sandbox に入れて外にはつなぎ、私設網と手元の網は塞ぐ。携帯は網を切った専用のエミュレータ。画面は専用の環境）、件数と伏せた題を手順書に載せる。作り直した回は前の回で試していない新しく入ったものだけを試す。見つけた候補は harness2 の処置が別の席で選別し、直すか本人に聞くか違和感の issue にする（依頼。入るまで CH21 は終わらず、席も回らない）。代わりに準備は最大 9,360 秒延び、PC を変えた回は本人のデスクトップに Sandbox の窓が開き、この PC の網の IP と時刻が GitHub などに残り、審査を通った PR に直しが後から載る。携帯は網を切るので、外とつないで初めて出る携帯の不具合はこの席では見えない。

<!-- prose impact.problem -->
2026-11-19、本人が配る形を自分の PC に入れて普通に使ったら、更新の確認がエラーを出し続けることがすぐ見つかった（#329）。いまの試験は Scenario を 1 つずつ確かめるだけで、本人が最初の利用者になっている。見つけたものを手順書に件数で載せるだけでは、直すのも人間に回すのも本人の手になる。

<!-- prose uses[U1].title -->
確認バッチを開くと、AI が先に使って直した結果がある

<!-- prose uses[U1].trigger -->
画面・PC・携帯の収集アプリを変えた PR が確認バッチに入った。自分で入れて触る前に、普通に使うと何か引っかかるかを知りたい。

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
t1 に入ったものと、画面・PC・携帯の段落（ID と件数は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
画面の席だけが回り、件数と題が載る。PC は「回さなかった」

<!-- prose walkthroughs[W1].cases[1].result -->
3 つの相手とも範囲が空で、「回さなかった」が 3 つ載る

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

<!-- prose walkthroughs[W5].title -->
PC の Sandbox の網をどう確かめるか

<!-- prose walkthroughs[W5].scene -->
PC を変えた CH30 が確認バッチに入った。準備が Sandbox を起こし、配る形を入れる前に網を確かめる。

<!-- prose walkthroughs[W5].why -->
Windows のファイアウォールは塞ぐ規則が通す規則に勝つので、外向きの既定を塞ぎ、外の範囲だけを通す。届かないことを見るだけでは、もともと待ち受けが無い宛先でも「届かない」になるので、塞ぐ前に自分が立てた口へ届くことを先に見る。

<!-- prose walkthroughs[W5].figure.caption -->
確かめの結果と、配る形を入れるか（説明用）

<!-- prose walkthroughs[W5].cases[0].result -->
配る形が入り、更新の確認は本物の配布元に届く。#329 の型もこの席で見える

<!-- prose walkthroughs[W5].cases[1].result -->
何も入れずに断り、PC の段落は「相手を立てられなかった」。その回の PC の機能は AI に触られない

<!-- prose walkthroughs[W5].cases[2].result -->
確かめられないので入れずに断る（S8）

<!-- prose walkthroughs[W5].exceptions[0].text -->
塞いだ Sandbox で配る形が入るか・ファイアウォールが範囲の数を受け付けるかは確かめていない（Task 8）

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

<!-- prose walkthroughs[W4].title -->
作り直した回で、何を試すか

<!-- prose walkthroughs[W4].scene -->
確認バッチを t1 から t4 まで作り直す。画面を変えた ST12 と ST13 が入る。

<!-- prose walkthroughs[W4].why -->
試した記録は相手・ID・PR のいちばん新しいコミットの組で、席が終えたときだけ足す。main の進みは組に入らないので、main だけが進んだ回は試し直さない。

<!-- prose walkthroughs[W4].figure.caption -->
回ごとの画面の席の範囲（ID と tag は説明用）

<!-- prose walkthroughs[W4].cases[0].result -->
席の時間と token を ST13 に使う

<!-- prose walkthroughs[W4].cases[1].result -->
ST12 は範囲に無いので、組み合わせの引っかかりは狙って見ない

<!-- prose walkthroughs[W4].cases[2].result -->
記録を足さないので、t2 で ST12 も試し直す

<!-- prose walkthroughs[W4].exceptions[0].text -->
試した記録は repo の外の手元だけにあり、追記のみで消さない

<!-- prose model.caption -->
この change が一周させるもの

<!-- prose sections.makes.tldr -->
確認バッチのたびに AI がその回の画面・PC・携帯を使う。直すか聞くかの処置は harness2 に依頼する。

<!-- prose sections.change.tldr -->
手順書の冒頭に画面・PC・携帯の段落が載り、処置が入った後は直したものの 1 行と問いも載る。

<!-- prose sections.lost.tldr -->
準備が最大 9,360 秒延び、PC の席は外につなぐ。審査を通った PR に直した変更が載ることもある。

<!-- prose sections.uses.tldr -->
確認バッチを開くと AI が先に使った結果がある。処置が入れば、直したものと問いも届く。

<!-- prose sections.watch.tldr -->
PC を変えた回は窓が開き外につなぐ。作り直しでは新しく入ったものだけを試す。処置が入るまで席は回らない。

<!-- prose sections.findings.tldr -->
重大は 2 件。.env に届きうる（前回受け入れ）と、実機に届きうる（PATH から外して直した。残りあり）。

<!-- prose sections.rest.tldr -->
席の中身・手で呼んだときの処置・携帯を外につなぐことは作らない。置き場と試験の名前の書き方は AI に任せた。

<!-- prose features[hook].summary -->
席が本物を立てる口を tools/tsukau/ に置く。相手は画面・PC・携帯

<!-- prose features[hook].tldr -->
席が本物を立てる口を tools/tsukau/ に置く。相手は画面・PC・携帯のエミュレータの 3 つ。

<!-- prose points[hook.1].text -->
相手は画面（web）・PC（pc）・携帯のエミュレータ（android）の 3 つ。立てる上限は 15 分

<!-- prose points[hook.2].text -->
結果の置き場は repo の外の ~/.local/state/ashiato2/tsukau の 1 か所。どの作業ツリーから呼んでも同じ所に並ぶ

<!-- prose points[hook.3].text -->
手で呼んだ /tsukau は結果を残すだけ。処置へ渡す目録を作らず、本人が RESULT.md を読んで /iwakan か /hanasu に回す

<!-- prose points[hook.4].text -->
手で呼ぶ /tsukau の席からも adb と emulator を隠す。harness2 に席の環境から命令を隠す鍵（seat_hide）を依頼し、入った後に tsukau.json に足す。入るまでは手で呼んだ席に adb が見えるが、CH21 は harness2 の処置を待って main に入らない

<!-- prose features[scope].summary -->
入れた PR が変えたファイルで相手を決め、表題と価値を渡す

<!-- prose features[scope].tldr -->
その回に入れた PR が変えたファイルで画面・PC・携帯のどれかを決め、表題と価値の 1 行を範囲として渡す。

<!-- prose points[scope.1].text -->
画面のファイル（web/src/・web/index.html・web/public/）を変えたものは画面へ。サーバ・PC の収集アプリ・更新係・インストーラ・移行・配る形の組み立てを変えたものは PC へ。携帯の収集アプリ（collector-android/）を変えたものは携帯へ。2 つ以上なら、それぞれへ

<!-- prose points[scope.2].text -->
範囲は「<ID> <表題>: <何のためか>」を並べたもの。何のためかは価値の節の、引用と注記を飛ばした最初の段落。無ければ表題だけ

<!-- prose points[scope.3].text -->
文書や道具だけ・試験だけを変えたものは、どの相手にも入らない。CH21 だけの確認バッチでは席は回らず「回さなかった」が 3 つ載る

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
PC の範囲があれば、席を呼ぶ前に確認バッチの木から配る zip を手元で組む（上限 2,400 秒）。組めなければ PC の席は呼ばず、画面と携帯の席は呼ぶ。画面と携帯の相手には、準備が組んだサーバと画面の置き場を渡す

<!-- prose points[call.4].text -->
準備が延びる。相手 1 つの呼び出しは上限 2,220 秒、全体は 9,360 秒（組み立て 2,400 + 3 つの相手の 2,220 ずつ + 300）で打ち切る。その間、確認バッチの作業ツリーの錠を握り続け、記録と fix の撮り直しは錠を待ち、録画は遅れて始まる

<!-- prose points[call.5].text -->
席がどう終わっても準備は rc=0 のまま続き、手順書に「使ってみる席: 呼べなかった」の 1 段落が足される。既存の段落の後に足すだけ

<!-- prose points[call.6].text -->
どの相手の席も PATH から Android の道具の置き場を外して呼ぶ。席は adb と emulator を名前で呼べず、端末に届く口は phone.sh だけ。外しても名前で見つかれば、その回はどの相手の席も呼ばず「席を呼べなかった」と書く

<!-- prose features[report].summary -->
公開に出るのは止まり方・件数・番号つきの伏せた題だけ

<!-- prose features[report].tldr -->
手順書と verify PR の本文に載るのは、止まり方・件数・番号つきの題だけ。題は 1 つの台本で伏せる。

<!-- prose points[report.1].text -->
手順書の最後に、頭の 1 段落と「画面: 」「PC: 」「携帯: 」の段落を 1 つずつこの順に足す。終えた相手は「席が自分で終えた。違和感の候補 2 件」と、C1・C2 の番号つきの題

<!-- prose points[report.2].text -->
回らなかったときは止まり方ごとの決まった文言だけを書く。訳は手元の結果を開かないと分からない

<!-- prose points[report.3].text -->
題は tools/tsukau/public-title.sh 1 か所で伏せる。処置（harness2）が手順書の問い・issue・直しの commit・試験の名前に題を書くときも同じ台本を通す

<!-- prose points[report.4].text -->
伏せる判定を最後まで走らせられない回（gitleaks が無い など）は、全部の題を伏せる

<!-- prose points[report.5].text -->
伏せなかった題は 1 行にし、80 字で切り、@ と #数字 を全角に替える（公開の repo で通知や issue の繋がりにしない）

<!-- prose features[handoff].summary -->
別の席が選別し、直すか本人に聞くか依頼にする。処置は harness2

<!-- prose features[handoff].tldr -->
見つけた候補は別の席が選別し、直すか本人に聞くか違和感の issue にする。処置は harness2 に依頼する。

<!-- prose points[handoff.1].text -->
準備は手元にだけ目録 dist/verify-<tag>/tsukau.json を書く。相手ごとの状態（呼んだ・回さなかった・呼べなかった）・範囲・結果の置き場・前の回で試したので外したものだけで、候補の本文は持たない。自分では直さず issue も作らない

<!-- prose points[handoff.2].text -->
処置は harness2 の確認バッチへの依頼にする。見つけた席とは別の席が選別し、この回が原因で直し方が明らかなら直す、判断が要れば手順書の問い、前からなら違和感の issue、誤検知は件数だけ

<!-- prose points[handoff.3].text -->
処置は本人が見る前に直す。統合の CI を直すときと同じ道で、直した変更を審査を通った Story のブランチに載せ、バッチを作り直す。本人は直った後のものを見て、手順書の 1 行で直したと知る

<!-- prose points[handoff.4].text -->
選別に迷えば本人に聞く。直すのは 1 本の流れで 3 件まで・同じ候補は 1 回まで。それを越えたもの・Story の作業場所が汚れていて載せられないものも本人に聞く

<!-- prose points[handoff.5].text -->
直した後は、処置が TSUKAU_RECHECK で直した候補を渡し、その題だけを範囲に 5 分・10 万 token で確かめ直す。確かめ直しは試した記録で範囲を外さず、記録に足しもしない

<!-- prose points[handoff.6].text -->
公開の repo には題・種類・どの Story・回・番号だけを書く。直しの commit の本文・足す試験の名前と中の文字列・Story の issue へのコメントも同じ。根拠は本人に渡す手元の HTML から開く

<!-- prose points[handoff.7].text -->
CH21 の終わりは、harness2 の処置が入った後に、使い捨ての確認バッチの木で処置まで本物で通すこと。それまで CH21 は main に入らず、確認バッチで席は回らない

<!-- prose features[reuse].summary -->
前の回で試していない、新しく入ったものだけを範囲にする

<!-- prose features[reuse].tldr -->
確認バッチを作り直したら、前の回で試していない新しく入ったものだけを席に渡す。新しいものが無ければ呼ばない。

<!-- prose points[reuse.1].text -->
範囲に入ったものごとに、相手・ID・PR のいちばん新しいコミットの組を手元の試した記録と突き合わせ、席が終えた記録があるものを外す。段落に「前の回で試した: ST12（t1）」と書く

<!-- prose points[reuse.2].text -->
main が進んだだけでは試し直さない。PR のいちばん新しいコミットが変わったもの（Story にコミットが足された・処置が直しを載せた）は、新しく入ったものとして試し直す

<!-- prose points[reuse.3].text -->
新しく入ったものが無い相手は席を呼ばず、段落を「回さなかった（この回に新しく入った画面のものが無い）」で始める

<!-- prose points[reuse.4].text -->
前の回が「席が自分で終えた」「時間を使い切った」「token を使い切った」以外で終わっていたら記録を足さないので、次の回に試し直す

<!-- prose points[reuse.5].text -->
前に試した機能と新しく入った機能を一緒に使ったときの引っかかりは、範囲に入らないので狙って見ない（深掘り Q11 で選んだ案の代償）

<!-- prose features[pc].summary -->
私設網と手元の網を塞ぎ外にはつないだ Sandbox に配る形を入れ、窓を開いて触らせる

<!-- prose features[pc].tldr -->
まっさらな Sandbox を外にはつないで起こし、私設網と手元の網を塞いだと確かめてから配る形を入れ、窓を開く。

<!-- prose points[pc.1].text -->
網をつないだ Sandbox を 1 つ起こし、何かを入れる前に中のファイアウォールで私設網と手元の網を塞ぎ、この PC の 2 つの宛先に届かないことを確かめる。その後にその場の合言葉で配る形のサーバを入れ、窓を開き、PC の収集アプリを利用者のセッションで起こしてから席に渡す

<!-- prose points[pc.5].text -->
確かめは対照つき。その回だけの一時の口をこの PC の私設網と手元の網の 2 つの宛先に立て、塞ぐ前に届くこと・塞いだ後に届かないことを見る。ほかの機器（本人の携帯など）は当てず、数から組んだ通す範囲に頼る。範囲が私設網と私用の範囲を含まないことは自己試験で固定する

<!-- prose points[pc.2].text -->
外にはつなぐ（深掘り Q8 の本人の答え）。更新の確認が本物の配布元に届くので #329 の型も見える。代わりに PC を変えた回のたびに、この PC の網の IP と時刻が GitHub と席が開いたサイトに残る。正典 distribution の「本物の配布元へ繋ぐ自動の試験」に 3 つ目として足す

<!-- prose points[pc.6].text -->
一時の口を立てられない・塞ぐ前に届かないときは、確かめられないとして何も入れずに断る。塞いだ後に私設網か手元の網に届いたら外向きを全部塞いで確かめ直し、それでも届けば断る。外にだけ届かなかった回は続け、案内にそう書く

<!-- prose points[pc.7].text -->
塞ぎは Sandbox の中の規則で、中の管理者（インストーラ・席が走らせるコマンド）は外せる。守りは「確かめてから渡す」と、案内の「ファイアウォールの規則に触らない」まで

<!-- prose points[pc.3].text -->
PC を変えた回ごとに、Sandbox の窓が本人のデスクトップに開く（1 日 1〜5 回まで、10 分ほど）

<!-- prose points[pc.4].text -->
別の Sandbox が動いている・配る zip が無いときは立てずに断る。片付けは中のログを写してから自分の Sandbox だけを止め、止まらなければ非 0 で終わる

<!-- prose features[phone].summary -->
専用のエミュレータを網を切って起こし、その回のサーバ向けの APK を入れて触らせる

<!-- prose features[phone].tldr -->
携帯は専用のエミュレータを画面なし・網なしで起こし、その回のサーバ向けの APK を入れて席に触らせる。

<!-- prose points[phone.1].text -->
画面の相手と同じ形でその回の専用の環境を立て、その木で接続先を端末の中の 127.0.0.1:18787 にした debug の APK を組む。専用の AVD のエミュレータを画面なし・前のデータを消して起こし、APK を入れて起こしてから席に渡す

<!-- prose points[phone.2].text -->
席が端末に届く口は tools/tsukau/phone.sh の shell・shot・ui だけ。押す・打つは shell から input で行う。phone.sh は残した serial のほかを名指さず、引数で serial を受けない。道具は $ANDROID_HOME の下から自分で見つけ、無ければ 1 行で断る

<!-- prose points[phone.3].text -->
片付けは端末のログを写してから自分のエミュレータだけを止め、その回の project と木（組むときの合言葉を含む）を消す。止まらなければ非 0 で終わる

<!-- prose points[phone.4].text -->
端末の網（Wi-Fi と携帯の回線）は切る。Q8 は PC の Sandbox への答えで、携帯には聞いていないので、外へ何も出さない側に仮に置いた。携帯で外とつないで初めて出る不具合は見えない（debug の APK は更新の宛先も空）

<!-- prose points[phone.5].text -->
切る前にホストの別名の宛先に届くこと、切った後は届かず既定のネットワークも無いことを確かめ、サーバへは adb reverse の道だけで届ける。どれかが外れれば APK を入れずに断る

<!-- prose points[phone.6].text -->
APK の資格情報はその回の乱数を組み立ての引数で渡し、本人の .env と ~/.gradle/gradle.properties の値は使わない。その回の資格情報は、組み立ての間だけこの PC のプロセスの一覧に見える

<!-- prose points[phone.7].text -->
実機とほかのエミュレータ（本人のもの・計測テストのもの）には問いも入れも止めもしない。adb は端末の一覧のほかは全部、自分の serial を名指す

<!-- prose points[phone.8].text -->
KVM を開けない・エミュレータか SDK か JDK が無い・専用の AVD のエミュレータがもう動いているときは、何も起こさずに断る（待たない）

<!-- prose points[phone.9].text -->
席が名前で adb を呼ぶ道は閉じたが、$ANDROID_HOME からの絶対パスや adb の待ち受け（ホストの 5037 番）に直に話しかければ、本人の実機にまだ届く。Windows 側に adb.exe を入れれば、それも名前で見える。止めているのは席の規範と案内の「触らない」まで（IMP-13 の残り）

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

<!-- prose impact.scenarios[S7].result -->
画面の席の範囲は ST13 だけ。段落に「前の回で試した: ST12（t1）」

<!-- prose impact.scenarios[S7].lost -->
ST12 と ST13 を一緒に使ったときを、席が狙って見ること

<!-- prose impact.scenarios[S8].result -->
PC の段落は「相手を立てられなかった」

<!-- prose impact.scenarios[S8].lost -->
その回の PC の機能を AI が触ること

<!-- prose impact.scenarios[S9].result -->
携帯の段落は「相手を立てられなかった」

<!-- prose impact.scenarios[S9].lost -->
残ったエミュレータを本人が止めるまで、携帯の席

<!-- prose impact.scenarios[S10].result -->
画面・PC・携帯のどの席も呼ばれず、段落は「席を呼べなかった」。理由は手元の tsukau.log にだけ書かれる

<!-- prose impact.scenarios[S10].lost -->
その PC で /usr/bin/adb を消すまで、毎回の使ってみる席

<!-- prose impact.rules[R1].legit -->
本人が自分の Sandbox を使っている回・前の回の席が残した Sandbox がある回も、PC の席が回らない

<!-- prose impact.rules[R2].legit -->
組み立てが遅い・Windows 側の道具が無い回は、その回の PC の機能が AI に触られない

<!-- prose impact.rules[R3].legit -->
Sandbox の中のパス（C:\ProgramData\ashiato など）を書いた、私的でない題も伏せられる

<!-- prose impact.rules[R4].legit -->
時間内に終わらない正当な回は「時間の上限で打ち切った」になる

<!-- prose impact.rules[R5].legit -->
前に試した機能を、新しく入った機能と一緒に使ったときの引っかかりは狙って見ない。同じものをもう一度見たいときは手で /tsukau を呼べる

<!-- prose impact.rules[R6].legit -->
確認バッチの準備そのものなど、道具だけを変えた PR は席が回らない

<!-- prose impact.rules[R7].legit -->
前の回が閉じ損ねた自分の Sandbox やエミュレータは誰も閉じない。残りは R1・R16 で以後の PC と携帯の席を止める（IMP-04・IMP-12）

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

<!-- prose impact.rules[R14].legit -->
ほかの試験は本物の配布元へ繋がない。外とつないで初めて出る不具合は、名指しの 3 つで見る

<!-- prose impact.rules[R15].legit -->
tailscale が止まっている・Windows 側で一時の待ち受けを立てられない日は、PC の席が回らない

<!-- prose impact.rules[R16].legit -->
KVM が消えた日や、前の回か手で呼んだ席のエミュレータが残っている間は、携帯の席が回らない

<!-- prose impact.rules[R17].legit -->
本人が $ANDROID_HOME の外（apt など）に adb を入れた PC では、画面と PC の席も毎回回らない。いまの PC の adb は $ANDROID_HOME の下の 1 つだけ。止まるのは確認バッチの準備から呼ぶ席だけで、手で呼ぶ /tsukau は対象外（harness2 の鍵 seat_hide が入った後は、外せなければ席を起こさずに終える）

<!-- prose impact.findings[IMP-02].title -->
塞いだ Sandbox で配る形が入り、外に届くかは未確認

<!-- prose impact.findings[IMP-02].text -->
Sandbox の中のファイアウォールで外向きの既定を塞ぎ、外の範囲と 53 番だけを通したときに、配る形のインストーラ・サーバ・DB が入り、外の名前が引けて HTTPS が通り、私設網と手元の網に届かないかを確かめていない。Windows 側の一時の待ち受けが許可の窓を出すかも分からない。どれかが外れると、PC の席が毎回立たないか、外に届かないまま回る

<!-- prose impact.findings[IMP-03].title -->
準備が延び、その間は記録と fix が待つ

<!-- prose impact.findings[IMP-03].text -->
画面・PC・携帯を変えた回は、配る zip の組み立て（上限 2,400 秒）と 3 つの席（2,220 秒ずつ。携帯は APK の組み立てとエミュレータの起動を立てる時間に含む）で準備が最大 9,360 秒延びる。処置が直した回は、作り直しと確かめ直し（5 分）の分もさらに延びる。その間、確認バッチの作業ツリーの錠を握り続け、別の経路の記録・fix・ci_fix・conflict_fix の後の作り直しが錠を待つ。同じ流れの録画は準備の後に続くので、延びた分だけ遅れて始まる

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
処置で直した変更は統合の CI を直すときと同じく、審査を通った Story / change のブランチへ載る（依頼。harness2 の処置が入った後）。先例の統合の CI の直し（ci_fix）は CI を待つだけで審査をやり直さない（E14 の台本）。本人が直しの差分を見られるのは verify PR の diff で、手順書は 1 行で知らせるだけ

<!-- prose impact.findings[IMP-09].title -->
選別を誤ると、要らない直しが載るか本物が流れる

<!-- prose impact.findings[IMP-09].text -->
選別の席（AI）が誤検知を「この回が原因」とすれば要らない直しが Story に載り、本物を「誤検知」とすれば件数の 1 行に埋もれる。実地で確かめるときも、用意した候補が「本人に聞く」に倒れることがある

<!-- prose impact.findings[IMP-12].title -->
携帯の席はエミュレータの残りと取り合いで止まりうる

<!-- prose impact.findings[IMP-12].text -->
片付けが走らずに専用の AVD のエミュレータが残ると、以後の携帯の席は立たない。手で呼んだ席のエミュレータが動いている間に確認バッチの準備が走ると、準備は繋がっている emulator-* 全部に本人のサーバ向けの APK を install -r で入れる。席の APK も同じ debug の組み立てなので置き換わり、接続先が本人のサーバに変わる。網は切ってあるので送れず、席はそれを違和感として書きうる。手で呼んだ席は目録を作らないので、その偽の候補が処置（IMP-08・IMP-09 の道）に渡るのは、本人が手で回したときだけ

<!-- prose impact.findings[IMP-13].title -->
席はホストの adb で本人の実機に届きうる

<!-- prose impact.findings[IMP-13].text -->
直す前は、確認バッチの準備が Android の道具を PATH に足してから席を呼び（E21・E20）、席の台本は環境をそのまま渡すので、席（ホストの WSL で許しの形 auto で走る claude）は adb を名前で直に使えた。WSL の adb に本人の携帯が見える構成なら、案内に反して実機に入れる・消す・止めることを機械では止めていなかった。本人の答えで、準備は席の PATH から Android の道具の置き場を外して呼ぶようにした（D32）。いまは名前では届かず、$ANDROID_HOME からの絶対パスと adb の待ち受け（5037 番）に直に話しかける道が残る。WSL の adb に実機が見えない構成なら、この IMP は起きない

<!-- prose impact.scope.covers[0].text -->
確認バッチの準備から呼ぶ席と、手で呼ぶ /tsukau の、画面・PC・携帯の相手の立て方

<!-- prose impact.scope.covers[1].text -->
処置へ渡す手元の目録・公開の題の台本・確かめ直しの口（処置そのものは harness2）

<!-- prose impact.scope.preconditions[0].text -->
確認バッチの範囲はその回に「確認バッチ <tag> に入れる」merge で入った PR だけ。すでに main の Story は入らない

<!-- prose impact.scope.preconditions[1].text -->
PC の相手は、配る zip を組めたときだけ・ほかの Sandbox が動いていないときだけ立ち、この PC の私設網と手元の網の宛先に届かないと確かめた後にだけ配る形を入れる

<!-- prose impact.scope.preconditions[2].text -->
画面の相手は、準備が組んだサーバと画面の置き場があるときだけ立つ（手で呼んだときは別の木で組む）

<!-- prose impact.scope.preconditions[3].text -->
題は私的な形と commit の検査に掛かる形を伏せたものだけを写す。手順書に写すのは止まり方・件数・番号つきの題だけ

<!-- prose impact.scope.preconditions[4].text -->
作り直した回で範囲から外すのは、同じ相手・ID・PR のいちばん新しいコミットで、席が done・time・tokens で終えた記録があるものだけ

<!-- prose impact.scope.preconditions[5].text -->
確かめ直しは、処置が TSUKAU_RECHECK を渡したときだけ、直した候補だけを範囲にする

<!-- prose impact.scope.preconditions[6].text -->
席の呼び出しが落ちても準備は止まらず、既存の段落の後に足すだけ

<!-- prose impact.scope.preconditions[7].text -->
目録は手元にだけ書き、準備は自分では直さず issue も作らない

<!-- prose impact.scope.preconditions[8].text -->
処置が直すのは 3 件・1 回まで、迷えば本人へ（harness2 への依頼が約束し、ashiato2 の正典は約束しない）

<!-- prose impact.scope.preconditions[9].text -->
自動の試験が本物の配布元へ繋ぐのは名指しの 3 つだけで、使ってみる席の PC の相手は塞いで確かめた後にだけ外へつなぐ

<!-- prose impact.scope.preconditions[10].text -->
携帯の相手は、KVM と SDK と JDK があり、専用の AVD が動いておらず、網を切ったと確かめられたときだけ APK を入れる。adb は自分の serial だけを名指す

<!-- prose impact.scope.preconditions[11].text -->
差し込み口の鍵は env_hook・targets・out_root・ready_timeout_s の 4 つだけ。手で呼んだ席は目録を作らない

<!-- prose impact.scope.excludes[0].text -->
席の中身（触り方・候補の書き方）・手で呼んだ席の処置

<!-- prose impact.scope.excludes[1].text -->
携帯を外につなぐこと・携帯の配る APK で試すこと（携帯は網を切り、debug の APK で試す）

<!-- prose impact.scope.excludes[2].text -->
選別・直し・本人への問い・違和感の issue の台本（harness2 への依頼）

<!-- prose impact.scope.residual[0].text -->
伏せる網に掛からない私的な語（人や場所の名前）は題に出うる

<!-- prose impact.scope.residual[1].text -->
残った Sandbox が以後の PC の席と録画を止めうる（IMP-04）

<!-- prose impact.scope.residual[2].text -->
Sandbox の中の管理者は塞ぎの規則を外せる。守りは確かめてから渡すことと案内まで

<!-- prose impact.scope.residual[3].text -->
確かめで当てるのはこの PC の 2 つの宛先だけ。私設網のほかの機器への守りは、数から組んだ範囲と規則まで

<!-- prose impact.scope.residual[4].text -->
審査を通った PR に、本人が見る前に直した変更が載る（IMP-08）

<!-- prose impact.scope.residual[5].text -->
席はホストで動き、本人の .env と、繋がっていれば本人の実機に届きうる（IMP-05・IMP-13）

