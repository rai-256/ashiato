# ST53 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
最初の本物の版を配る前に、4 つの配布物（サーバの Linux 版・Windows 版、PC の収集アプリ、携帯の APK）に、本体の許諾（Google のライブラリと組み合わせて配ってよいという追加の許可つき）・依存の許諾の一覧・対応するソースの場所が入るようになる。許可の外の部品が依存に入れば CI が落ちる。正式な署名の鍵は本人の承認なしには公開の job に渡らない形を作ってから置けるようになる。

<!-- prose impact.overview -->
配布物 4 つに、追加の許可つきの LICENSE・依存の許諾の一覧・ソースの場所が入り、携帯を含む依存の許諾とソースの表示を CI が確かめるようになる（組み立ての台本を通した配布物と、表示を持つファイルについて）。正式な鍵は本人を必須の承認者に付け、エージェントの資格情報を絞った後に置く手順になる。できなくなること: 他人の GPL / AGPL や名指しの外の非オープンソースの部品を依存に足すこと、古い表示のままのファイルを足すこと、エージェントの資格情報での管理の操作（と、絞り方によっては CI の走らせ直し。IMP-03）、確認バッチの準備が実機に APK を入れること。絞った資格情報の外に本人の管理者の資格情報が残る場合の守りは、この仕様では決まっていない（IMP-02）。

<!-- prose impact.problem -->
本人が最初の本物の版を配ろうとしたとき、配布物には実行ファイルしか入っておらず、AGPL の本体が非オープンソースの Google のライブラリと組み合わさった APK を、許可の文も許諾の一覧も無いまま配ることになる。配った版は取り消せない。同時に、正式な鍵を Environment に置くと、印を push できる資格情報（いまは管理者のエージェントも含む）で main に無い commit から鍵を読める経路が開く（ST52 IMP-01）。

<!-- prose uses[U1].title -->
最初のベータ版を公開する前に、鍵を置く順を踏む

<!-- prose uses[U1].trigger -->
ST53 が main に入った。最初のベータ版 0.1.0-beta.1 を配りたいが、正式な鍵を置いた後で、エージェントが main に無い commit から鍵を読めないかが気になる。

<!-- prose uses[U1].outcome -->
配った最初の版に許可と許諾の一覧が入っていて、鍵は自分が承認した job にだけ渡ったと確かめられる

<!-- prose uses[U1].without -->
実行ファイルだけの版が許可の文なしで出回り（取り消せない）、印を打てる資格情報なら誰でも鍵を読める経路が開いたままになる

<!-- prose uses[U1].steps[0].do -->
docs/release.md §3 のとおり、エージェント用に権限を絞った token を作って差し替え、check-guards.sh agent-credential --repo rai-256/ashiato を走らせる

<!-- prose uses[U1].steps[0].see -->
rc=0（今の資格情報は管理者でない）

<!-- prose uses[U1].steps[1].do -->
§4 で Environment release に自分を必須の承認者として付け、管理者の飛ばしを外し、check-guards.sh approvals --repo rai-256/ashiato を走らせる

<!-- prose uses[U1].steps[1].see -->
rc=0

<!-- prose uses[U1].steps[2].do -->
§5 で鍵と Secrets を置き、main の commit に印 v0.1.0-beta.1 を打つ

<!-- prose uses[U1].steps[2].see -->
公開の job が「承認待ち」で止まり、承認の画面に印の commit が出る

<!-- prose uses[U1].steps[3].do -->
その commit が main にあり PR を通って入ったことを見て承認する

<!-- prose uses[U1].steps[3].see -->
4 つの配布物が並ぶ

<!-- prose uses[U1].steps[4].do -->
手順の最後の 2 行（gh release download と check-artifacts.sh --channel beta --expect-version 0.1.0-beta.1）を走らせる

<!-- prose uses[U1].steps[4].see -->
rc=0。tar を開くと LICENSE・THIRD-PARTY-NOTICES.txt・SOURCE.txt（Ref: v0.1.0-beta.1）がある

<!-- prose uses[U2].title -->
携帯に地図のライブラリを足そうとしたら CI が止める

<!-- prose uses[U2].trigger -->
携帯の収集アプリに地図を出したくなり、AI が play-services-maps を依存に足した PR を出した。

<!-- prose uses[U2].outcome -->
許可の外の非オープンソースの部品が、気づかないうちに配布物へ入ることはない（検査が見る携帯の依存について）。広げるかは本人が決める

<!-- prose uses[U2].without -->
地図のライブラリが許可の外のまま APK に入って配られ、その版の配布が AGPL と両立するかの解釈が割れたまま残る

<!-- prose uses[U2].steps[0].do -->
PR の CI を見る

<!-- prose uses[U2].steps[0].see -->
配る形を組む job の依存の許諾の検査が、com.google.android.gms:play-services-maps と「Android Software Development Kit License」を示して落ちている

<!-- prose uses[U2].steps[1].do -->
使うなら、許可の正本の Listed libraries: に 1 行足す（LICENSE の先頭も一緒に直る）

<!-- prose uses[U2].steps[1].see -->
検査は通るが、許可の文が変わる。次に配る版から、広げた許可で出回る

<!-- prose walkthroughs[W1].title -->
名指しの外の Google のライブラリを足したとき

<!-- prose walkthroughs[W1].scene -->
AI が携帯の依存に Google のライブラリを 1 つ足した PR を出す。CI の依存の許諾の検査がどう判定するか。

<!-- prose walkthroughs[W1].why -->
許可が受けるのは名指しした部品だけで、検査はその名指しを許可の正本から読む。許可の外の非オープンソースの部品を配ると、許可の無い組み合わせを配ることになるため、入る前に止める。

<!-- prose walkthroughs[W1].figure.caption -->
携帯の依存の部品ごとの判定（部品の名前は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
検査は通り、「通した PROPRIETARY」に 4 つの座標が出る

<!-- prose walkthroughs[W1].cases[1].result -->
CI が落ちる。使うには許可の正本を広げ、次の版から広げた許可で配る

<!-- prose walkthroughs[W1].exceptions[0].text -->
説明用の値。地図のライブラリを足す予定があるわけではない

<!-- prose walkthroughs[W2].title -->
配布物ごとに SOURCE.txt に何が書かれるか

<!-- prose walkthroughs[W2].scene -->
本人が手元にある配布物から対応するソースを取りに行こうとする。どの配布物でも同じ commit に辿り着けるか。

<!-- prose walkthroughs[W2].why -->
要件は「同じ印で示す」と言うが、開発版の印は動き、PR の組み立てには印が無い。動かない commit で示すことにした。

<!-- prose walkthroughs[W2].figure.caption -->
組み立ての種類ごとの SOURCE.txt（版番号は説明用）

<!-- prose walkthroughs[W2].cases[0].result -->
nightly の印は翌日動くが、SOURCE.txt の commit は動かないので同じソースを取れる

<!-- prose walkthroughs[W2].cases[1].result -->
書かれた merge commit を取れるかは確かめていない。取れなければ、配った成果物のソースの場所が辿れない

<!-- prose walkthroughs[W2].exceptions[0].text -->
説明用の値。実際の版番号の構成は確かめていない

<!-- prose walkthroughs[W3].title -->
main に無い commit に印を打たれたとき

<!-- prose walkthroughs[W3].scene -->
正式な鍵を置いた後、本人でない誰か（本人の資格情報で動くエージェントを含む）が、main に入っていない commit で workflow を書き換え、印 v0.5.0 を push する。

<!-- prose walkthroughs[W3].why -->
守りの本体は GitHub が job を始める前に効く承認で、承認できるのは承認の権限を持つ資格情報だけ。エージェントの資格情報を絞っても、エージェントが別の管理者の資格情報に手が届くなら分けられない。

<!-- prose walkthroughs[W3].figure.caption -->
印が push された後に何が起きるか

<!-- prose walkthroughs[W3].cases[0].result -->
job は承認待ちで止まり、本人が commit を見て拒めば鍵は渡らない

<!-- prose walkthroughs[W3].cases[1].result -->
エージェントがその token で承認者の規則を外すか承認すれば、鍵が読まれる

<!-- prose walkthroughs[W3].exceptions[0].text -->
説明用の値。誰かが実際に印を打ったわけではない

<!-- prose model.caption -->
許可の正本から、検査と配布物までの流れ

<!-- prose features[permission].summary -->
AGPL-3.0 に、名指しした Google のライブラリ 4 つと組み合わせて配ってよいという許可を英文で足す。

<!-- prose points[permission.1].text -->
許可の文の正本は英文の 1 ファイル（LICENSES/AdditionRef-ashiato-play-services.txt）。LICENSE の先頭にそのまま写し、その後に AGPL-3.0 の全文を変えずに置く。README には日本語の参考訳（正本は英語）を置く。

<!-- prose points[permission.2].text -->
許可が受けるのは、位置の取得に使う play-services-location と、それが引く -base・-basement・-tasks の 4 つだけ（どれも com.google.android.gms）。版は問わないが、改変したものは含まず、Google のライブラリのソースは求めない。

<!-- prose points[permission.3].text -->
許可の文は付けて配った版にそのまま付いて出回り、取り消せない。名指しを広げるのは次の版で足すことになるが、配った版の文は残る。

<!-- prose points[permission.4].text -->
LICENSE の先頭が正本と 1 字でも違う、または AGPL-3.0 の全文が今のものと違うと、許諾の文の検査が CI を落とす。CONTRIBUTING.md には貢献も同じ許諾で配ることを足すが、CLA の受け入れ条件は変えない。

<!-- prose features[headers].summary -->
ソースと部品の許諾の欄を、追加の許可を含む 1 つの式にそろえ、違うものは CI が止める。

<!-- prose points[headers.1].text -->
ソースの先頭の表示と、Cargo.toml・web/package.json（と lock の根）・API の契約の写しの許諾の欄を AGPL-3.0-only WITH AdditionRef-ashiato-play-services にする。

<!-- prose points[headers.2].text -->
書き換えるのは 506 ではなく 503 ファイル。移行の 3 つは AGPL-3.0-only のまま残す。移行は中身の指紋を DB に記録していて、表示の 1 行を変えるだけで、その版を当てたどの DB でも起動のたびに警告が出続けるため。許可は LICENSE が木の全体に掛けるので、この 3 つにも及ぶ。

<!-- prose points[headers.3].text -->
表示の検査は追跡しているファイルの先頭 25 行の SPDX-License-Identifier: だけを見る。symlink は辿らない。表示を持たないファイルは見ないので、表示を消して写した他人のコードは止められない。

<!-- prose features[depcheck].summary -->
サーバ・画面・携帯の 3 つの依存の許諾を機械で確かめ、許可の外が入れば CI が落ちる。

<!-- prose points[depcheck.1].text -->
携帯の節を新しく作る。配る APK に入る依存の部品ごとに POM の許諾を読み（無ければ親の POM）、対応表で SPDX の名前に直す。名前を 2 つ持つ部品は、どちらか 1 つが許可されていれば通る。

<!-- prose points[depcheck.2].text -->
オープンソースでない部品は、許可の正本に名指しされた 4 つだけを通す。名指しは正本のファイルから読み、検査は自分で一覧を持たない。正本から 1 行消せば、その部品で落ちる。

<!-- prose points[depcheck.3].text -->
名指しの外の Google のライブラリ（地図・広告など）を足すと CI が落ちる。使うには、許可の正本に名指しを足した次の版を出すことになる（許可の文が変わる）。

<!-- prose points[depcheck.4].text -->
許可の一覧から AGPL-3.0-only を外す。自分の部品（workspace の crate と画面の根）は名前と決めた式の組で通し、他人の GPL / AGPL の部品はどれも落ちる。ほかの 17 の値は変えない。今の依存で落ちるものは見つかっていない（Rust 357・Node 306 部品を読んだ）。

<!-- prose points[depcheck.5].text -->
節の対象が見つからない（node_modules が無い・gradle が依存を解決できない）ときは、通さずに落ちる。そのため CI では、画面の job が Rust と画面の節を、配る形を組む job が携帯の節を走らせる。

<!-- prose features[bundle].summary -->
4 つの配布物に LICENSE・依存の一覧・ソースの場所を入れ、欠けた配布物は公開しない。

<!-- prose points[bundle.1].text -->
tar / zip の根に LICENSE・THIRD-PARTY-NOTICES.txt・SOURCE.txt の 3 つを入れる。携帯の APK では中の assets/licenses/ に置く。組み立ての台本で入れるので、正式版・ベータ版だけでなく開発版・PR の組み立てにも入る。

<!-- prose points[bundle.2].text -->
依存の一覧は部品ごとに名前・版・許諾の式・全文と著作権の表示を書く。サーバと PC の収集アプリには Rust の標準ライブラリの項、サーバには画面に入る Node の部品（開発だけの部品は除く）、APK には Google のライブラリが中に持つ第三者の一覧も入る。

<!-- prose points[bundle.3].text -->
3 点のどれかが無い・LICENSE が木の LICENSE と違う配布物は、配布物の検査で落ちて公開されない。許諾の全文も著作権の表示も見つからない部品があれば、一覧を組まずに組み立てが落ちる。

<!-- prose points[bundle.4].text -->
携帯のアプリに許諾を読む画面は作らない。APK の中に置くだけで、本人がアプリから開く口は無い。

<!-- prose features[source].summary -->
正式版・ベータ版は印で、それ以外は組み立ての commit でソースの場所を示す。

<!-- prose points[source.1].text -->
SOURCE.txt はリポジトリの場所と、正式版・ベータ版なら印 v<版>、開発版・PR の組み立てなら 40 桁の commit を持つ。印は公開の workflow が渡す。

<!-- prose points[source.2].text -->
開発版の SOURCE.txt には nightly の文字を書かない。印 nightly は翌日に別の commit へ動くので、動かない commit を書く。要件 FR-127 の「同じ印で示す」とはこの点で違い、record の段が要件に補足を入れる。

<!-- prose points[source.3].text -->
公開の組み立てに渡された印が v<版番号> と違えば、配布物を作らずに落ちる。配布物の検査は、正式版・ベータ版の検査で印を、PR・下書き・開発版の検査で組み立ての印と同じ commit を求め、合わなければ公開しない。

<!-- prose points[source.4].text -->
サーバの画面（ブラウザで開くもの）には、ソースの場所も許諾へのリンクも出さない。一般向けに配る前に足すことを、要件の未決として残す。

<!-- prose features[approval].summary -->
鍵を置く前に、本人を必須の承認者に付け、エージェントの資格情報を管理の権限の無いものに替える。

<!-- prose points[approval.1].text -->
配布用の Environment release に本人を必須の承認者として付け、管理者が承認を飛ばせない設定にする。承認されるまで公開の job は始まらず、正式な鍵も渡らない。本人が印を打って本人が承認する運用は止めない。

<!-- prose points[approval.2].text -->
エージェントの資格情報を、権限を細かく絞った token（枝・印の push、PR・issue、workflow のファイル、Actions の読み）に替える。管理・Environment・公開の承認・Secrets の権限は持たせない。手順の順は「資格情報 → 承認者 → 鍵」。

<!-- prose points[approval.3].text -->
公開の job の中で確かめる手順（approvals）は守りの本体ではない。job は印の commit の workflow で走るので、書き換えられれば効かない。守りの本体は GitHub が job を始める前に効く承認で、本人が承認の画面で「印の commit が main にあり PR を通って入った」ことを見ずに押せば、守りは無いのと同じ。

<!-- prose features[firstrelease].summary -->
最初の本物の印・正式な鍵・本人のサーバの切り替えは、ST53 が main に入った後に本人が行う。

<!-- prose points[firstrelease.1].text -->
ST53 の確認バッチでは、PR の配布物から作った下書きのリリースで許諾の 3 点と守りを見る。本物の印・正式な鍵・本人のサーバの切り替えは、main に入った後に本人が docs/release.md の手順で行い、最後に公開した版を取り寄せて配布物の検査に通す。

<!-- prose points[firstrelease.2].text -->
ST52 が書いた「本物の公開は ST53 の確認で見る」とは変わる。確認バッチの中で印を打つと、その版は main の祖先の commit なので ST53 の許可も一覧も入らずに出回るため。ST52 の完了の判定の文は record の段が書き直す。

<!-- prose points[firstrelease.3].text -->
ST53 より前に PR の成果物として置かれた配布物（許諾なし。今 14 件、90 日で消える）は、main に入った後に本人が消す手順を docs/release.md に置く。

<!-- prose features[verifyprep].summary -->
開発用の鍵の APK を、本人の携帯（実機）に入れないようにする。

<!-- prose points[verifyprep.1].text -->
準備の台本は、adb が返す端末のうち名前が emulator- で始まるものにだけ開発用の APK を入れる。実機が繋がっていても入れない。

<!-- prose points[verifyprep.2].text -->
エミュレータが無ければ何も入れず、手順書に「入れていない」ことと手で入れるコマンドを書く。この修正が入るまでの確認では、準備の間に本人の携帯を繋がない。

<!-- prose impact.scenarios[S1].result -->
表示の検査が CI を落とす

<!-- prose impact.scenarios[S1].lost -->
その PR をそのまま merge すること

<!-- prose impact.scenarios[S2].result -->
依存の許諾の検査が CI を落とし、座標と名前を示す

<!-- prose impact.scenarios[S2].lost -->
表に 1 行足すまで merge できない

<!-- prose impact.scenarios[S3].result -->
依存の一覧を組まずに組み立てが落ちる

<!-- prose impact.scenarios[S3].lost -->
tools/licenses/texts/ に全文を足すまで配る形を組めない

<!-- prose impact.scenarios[S4].result -->
読めなければ approvals が 0 でない終了コードで終わり、署名も公開もしない

<!-- prose impact.scenarios[S4].lost -->
正当な公開そのもの

<!-- prose impact.scenarios[S5].result -->
走らせ直しが断られる（と読む）。verify_batch.sh は失敗を捨てるので、赤のまま ci-fix の AI か人間に回る

<!-- prose impact.scenarios[S5].lost -->
不安定なテストを機械が 1 回走らせ直して済ませること

<!-- prose impact.rules[R1].legit -->
許可の文を正当に直すときも、正本・LICENSE・検査の中の英文の定数を同時に直す必要がある（揃えれば通る）

<!-- prose impact.rules[R2].legit -->
第三者の MIT などのコードを表示ごと正当に写して取り込むときも落ちる。例外の一覧に足すか LICENSES/ の置き方を決めるまで止まる

<!-- prose impact.rules[R3].legit -->
オープンソースの部品でも、POM の許諾の名前が対応表に無い書き方（表記ゆれ）なら落ちる。表に 1 行足すまで止まる

<!-- prose impact.rules[R4].legit -->
新しい依存が LICENSE のファイルも作者の欄も持たないと、正当な依存でも組み立てが落ちる。tools/licenses/texts/ に全文を足すまで止まる

<!-- prose impact.rules[R5].legit -->
公開の job の GITHUB_TOKEN が承認者の設定を読めなければ、本人の正当な公開も署名の前で止まる（確かめていない）

<!-- prose impact.rules[R6].legit -->
ハーネスが CI の赤を走らせ直す操作（gh run rerun）も、絞り方によっては止まる

<!-- prose impact.rules[R7].legit -->
実機で確かめたいときは、示されたコマンドで手で入れられる

<!-- prose impact.findings[IMP-01].title -->
印を push できる人が main を通さずに正式な鍵を読めるかもしれない（ST52 から）

<!-- prose impact.findings[IMP-01].text -->
ST52 から申し送られた判断。この Story は Environment release に本人を必須の承認者として付け、エージェントの資格情報を管理と承認の権限の無いものに替えてから鍵を置く手順にして、この経路を閉じる（Q3 の推奨）。閉じるかは、本人が承認の画面で印の commit を見ることと、承認の仕組みが文書どおりに効くこと（未走行）に乗る。

<!-- prose impact.findings[IMP-02].title -->
本人の管理者の資格情報がエージェントと同じ OS の利用者に残ると、資格情報を替えても分けられない

<!-- prose impact.findings[IMP-02].text -->
仕様は「エージェントの資格情報」を絞り、今の資格情報が管理者でないことを確かめる（agent-credential）。しかしエージェントは本人と同じ OS の利用者で動き、Claude には sandbox が無い。いま、この利用者の gh の login（~/.config/gh/hosts.yml。scopes は repo・workflow）が本人の管理者の資格情報そのもので、git の push も gh auth git-credential を通ってそれを使う。切り替えの後もこの利用者に管理者の token が残れば、エージェントはそれで承認者の規則を外す・保留の公開を承認でき、鍵が読まれる。agent-credential は今使っている 1 つの資格情報しか見ない。

<!-- prose impact.findings[IMP-03].title -->
絞った token の権限では、ハーネスの CI の走らせ直しが断られるかもしれない

<!-- prose impact.findings[IMP-03].text -->
D3 がエージェントに持たせる権限は Actions の「読み」まで。ハーネスは CI の赤を 1 回だけ走らせ直す（story_ci.py・verify_batch.sh の gh run rerun --failed）。走らせ直しには Actions の書きが要ると読むので、切り替えの後は不安定なだけの赤も ci-fix の AI か人間に回る。verify_batch.sh は失敗を知らせずに捨てる（story_ci.py は走らせ直せた run を数える）。

<!-- prose impact.findings[IMP-04].title -->
公開の job から承認者の設定を読めないと、正当な公開も署名の前で止まる

<!-- prose impact.findings[IMP-04].text -->
公開の job は署名の前に approvals を走らせ、Actions の GITHUB_TOKEN で environments/release を読む。読めなければ approvals は落ち、本人が承認した正当な公開も署名されない。

<!-- prose impact.findings[IMP-05].title -->
GitHub の画面が許諾を AGPL-3.0 でなく Other と出すかもしれない

<!-- prose impact.findings[IMP-05].text -->
LICENSE の先頭に許可の文を置くので、GitHub の許諾の判定が AGPL-3.0 と読めず Other と出るかもしれない。README が最初の節で許諾を名指しする。

<!-- prose impact.findings[IMP-06].title -->
閉じた PR の組み立ての SOURCE.txt が指す commit を、後から取れるか分からない

<!-- prose impact.findings[IMP-06].text -->
PR の組み立ては GitHub が PR のために作る merge commit で組まれ、SOURCE.txt はその commit を書く。PR を閉じた後にその commit を取れなければ、90 日置かれる PR の成果物のソースの場所が辿れない。

<!-- prose impact.scope.covers[0].text -->
組み立ての台本（tools/release/build.sh）で作る 4 つの配布物。正式版・ベータ版・開発版・PR の組み立てのどれも

<!-- prose impact.scope.covers[1].text -->
配る携帯の APK に入る依存（releaseRuntimeClasspath）と、サーバ・PC の収集アプリ・画面の依存

<!-- prose impact.scope.preconditions[0].text -->
許可が受けるのは、名指しした Google のライブラリ 4 つだけ（改変したものは含まない）

<!-- prose impact.scope.preconditions[1].text -->
表示の検査が見るのは、追跡しているファイル（symlink を除く）の先頭 25 行に表示を持つものだけ

<!-- prose impact.scope.preconditions[2].text -->
SOURCE.txt に印を書くのは正式版・ベータ版の公開の組み立てだけ。それ以外は commit

<!-- prose impact.scope.preconditions[3].text -->
公開の承認の守りは、本人が承認の画面で印の commit を見ることと、エージェントが管理者の資格情報に届かないことに乗る

<!-- prose impact.scope.preconditions[4].text -->
準備が APK を入れるのは emulator- で始まり device の状態の端末だけ

<!-- prose impact.scope.excludes[0].text -->
移行の 3 つ（AGPL-3.0-only のまま）と gradle の wrapper の 2 つ（Apache-2.0）の表示

<!-- prose impact.scope.excludes[1].text -->
表示を持たずに写したコード（表示の検査は見ない）・試験だけの依存

<!-- prose impact.scope.excludes[2].text -->
携帯の許諾の画面・サーバの画面のソースの場所・最初の本物の公開

<!-- prose impact.scope.residual[0].text -->
管理者の資格情報がエージェントから届く限り、承認者の守りは分けられない（IMP-02）

<!-- prose impact.scope.residual[1].text -->
ST53 より前の PR の成果物は許諾なしで、消すまで（か 90 日）残る

