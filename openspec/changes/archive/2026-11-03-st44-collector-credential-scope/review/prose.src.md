# ST44 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
携帯と PC の収集アプリが持つ合言葉（API の合言葉）では、記録を送ることと、通知に要る 4 つの問い合わせだけができるようになる。記録・滞在・気分を読めるのは画面の合言葉でログインしたときだけになる。落とした電話や古い収集アプリに残った値で、全記録を読まれることはこの合言葉の経路では無くなる。

<!-- prose impact.overview -->
収集アプリの合言葉で通る口を、送る 5 つと、記録の中身を返さない読み 4 つに限る。記録・滞在・気分の読み出しは、画面の合言葉でログインしたときだけになる（衛星の資格情報は ST29 のまま）。できなくなるのは、収集アプリの合言葉で読む・書き出す・消す・作り直すこと。道具と手で打つ curl は、画面の合言葉でログインして読む形に替わる。サーバの起動に新しい鍵 WEB_SESSION_KEY が要り、無ければ起動しない。

<!-- prose impact.problem -->
携帯の APK と PC の自動起動の台本は同じ合言葉を持ち、その値でサーバの全部の口が通る。落とした電話や古い APK、置き忘れた台本からこの値が漏れると、全記録を読まれ、読まれた記録は戻らない。

<!-- prose uses[U1].title -->
PC の台本に書いた合言葉が漏れたかもしれない

<!-- prose uses[U1].trigger -->
PC の収集アプリの自動起動の台本（合言葉が平文で入っている）を、うっかり共有フォルダに置いていた。誰かに全記録を読まれたのではないかが気になる。

<!-- prose uses[U1].outcome -->
漏れた値でできたのは、送ることと、記録の中身を返さない 4 つの読みだけだと分かる。合言葉を変えても、画面は使い続けられる。

<!-- prose uses[U1].without -->
漏れた値で全記録を読めた。合言葉を変えると、画面のログインも全部切れた。

<!-- prose uses[U1].steps[0].do -->
その値で記録の読み出しを試す（自分で curl を打つ）

<!-- prose uses[U1].steps[0].see -->
403。記録も滞在も気分も返らない

<!-- prose uses[U1].steps[1].do -->
読み出しの記録を見る

<!-- prose uses[U1].steps[1].see -->
資格情報の種類が「API の合言葉」で結果が 403 の行。読もうとした時刻と口が分かる

<!-- prose uses[U1].steps[2].do -->
API の合言葉を変えて、PC の収集アプリを入れ直す

<!-- prose uses[U1].steps[2].see -->
画面のログインは切れずに続く

<!-- prose walkthroughs[W1].title -->
同じ値で、送れるが読めない

<!-- prose walkthroughs[W1].scene -->
本人の PC の収集アプリが、いつもどおり記録を送る。同じ値を手に入れた誰かが、記録を読もうとする。

<!-- prose walkthroughs[W1].why -->
収集アプリの合言葉で通す口を、送る口と記録の中身を返さない読みの一覧に限るため。一覧は「あるもの」だけを通し、新しい口は足すまで断る。

<!-- prose walkthroughs[W1].figure.caption -->
収集アプリの合言葉を付けた求めの結果（締めた後）

<!-- prose walkthroughs[W1].cases[0].result -->
今どおり通る。収集アプリは入れ直さなくてよい

<!-- prose walkthroughs[W1].cases[1].result -->
403。中身は返らず、読み出しの記録に跡が残る

<!-- prose walkthroughs[W1].cases[2].result -->
通る。共有の 1 本の間は、どの端末から来たかも後から見分けられない（ST56 まで）

<!-- prose walkthroughs[W1].exceptions[0].text -->
説明用の例。口の一覧は spec の 9 つが正本

<!-- prose walkthroughs[W2].title -->
更新して最初に起動する

<!-- prose walkthroughs[W2].scene -->
本人が、自分の PC のサーバをこの Story の入った版に入れ替えて起動する。

<!-- prose walkthroughs[W2].why -->
印の鍵をサーバだけが持つ値に替え、鍵が無いまま緩い形で動かないよう、起動を止める側に倒したため。

<!-- prose walkthroughs[W2].figure.caption -->
入れ替えの手順と結果

<!-- prose walkthroughs[W2].cases[0].result -->
起動し、ログインを 1 回入れ直すだけ。記録は失わない

<!-- prose walkthroughs[W2].cases[1].result -->
起動しない。その間、収集アプリは溜めて待つ（携帯 90 日・2 GB）。足して起動すれば戻る（IMP-02）

<!-- prose walkthroughs[W2].exceptions[0].text -->
説明用の例。自動のインストールと更新（ST55・ST58）がこの鍵をどう作るかは、まだ決まっていない

<!-- prose model.caption -->
締めた後に、どの資格情報で何が通るか

<!-- prose features[collector].summary -->
送る 5 つの口と、記録の中身を返さない読み 4 つの口だけが通る。

<!-- prose points[collector.1].text -->
通る口は 9 つ。記録・生存信号・破棄の報告・原本の欠けを送る、写真の原本を置く、の 5 つと、送信契約の版・原本を受け取り済みか・気分の通知を出すか・途絶の判定、を読む 4 つ。

<!-- prose points[collector.2].text -->
携帯と PC の収集アプリは入れ直さなくてよい。両方が呼ぶ口は、ソースを辿るとすべてこの 9 つの中にある。

<!-- prose points[collector.3].text -->
気分の通知の判断は、収集アプリの合言葉のまま読める。返すのは通知の時刻と「その日に記録があるか」の真偽だけで、ひとことも尺度も返さない。通知の時刻を変える求めは 403 になる。

<!-- prose points[collector.4].text -->
4 つの読みは記録の中身を返さないが、残った値で「いつどのソースが途切れたか」「その日に気分を書いたか」「この写真の原本（hash）を受け取り済みか」は分かる。

<!-- prose points[collector.5].text -->
後から足した口は、一覧に足すまで収集アプリの合言葉では 403。一覧に足すかは、その口を収集アプリに使わせる Story が決める。

<!-- prose features[deny].summary -->
ハンドラも本文も見ずに 403 を返し、読み出しの記録に 1 行残す。

<!-- prose points[deny.1].text -->
収集アプリの合言葉で一覧の外の口を呼ぶと 403。記録・滞在・気分・書き出しは返らず、記録・滞在・場所・属性・衛星・書き出しの状態も変わらない。

<!-- prose points[deny.2].text -->
断った求めは、読み出しの記録に「API の合言葉・403」の 1 行で残る。落とした電話の値で読もうとした跡が見える。通知に要る 4 つの読みも、今どおり 1 回 1 行残る。

<!-- prose points[deny.3].text -->
ログインの状態を問う口（GET /session）も収集アプリの合言葉では 403。返すのは呼び出し元の種類だけだが、厳しい側に揃えた。

<!-- prose points[deny.4].text -->
衛星の登録・資格情報の発行と取り消しは、今どおり画面のログインでだけできる。収集アプリの合言葉では 403。

<!-- prose features[web].summary -->
画面のログインで通る範囲は今のまま。読む経路はここだけになる。

<!-- prose points[web.1].text -->
画面の合言葉でログインした印があれば、今どおり記録・滞在・気分を読み、記録を送れる（送信契約の版の口だけ今どおり 403）。

<!-- prose points[web.2].text -->
ログインの決まりは ST28 のまま。合言葉が違えば 401、失敗が続けば一時的に 429、ログアウトすると印は使えない。印はスクリプトから読めず、暗号化された接続だけで送られ、別のサイトから始まった求めには付かない。

<!-- prose points[web.3].text -->
収集アプリの合言葉とログインの印を両方付けた読み出しは、収集アプリの合言葉として扱われ 403 になる。読む道具は印だけを付ける。

<!-- prose points[web.4].text -->
衛星の形でなく、収集アプリの合言葉とも一致しない Bearer と有効な印を両方付けた読み出しは、今どおり印で通る。

<!-- prose points[web.5].text -->
「画面から記録を読むには合言葉のログインが要る」の Requirement は名前を替えて置き直した。前の本文の「API の合言葉の読み書きは変わらない」だけが成り立たなくなった。

<!-- prose features[key].summary -->
印の鍵を API の合言葉から WEB_SESSION_KEY に替える。

<!-- prose points[key.1].text -->
前は収集アプリの合言葉が印の鍵だった。収集アプリに残る値と DB の写しがそろうと、画面の合言葉を手元で総当たりできた。鍵をサーバだけが持つ値に替えるので、この 2 つがそろっても確かめられない。

<!-- prose points[key.2].text -->
サーバの起動に WEB_SESSION_KEY が要る。無い・16 文字未満・API の合言葉か画面の合言葉と同じ、のどれかなら起動せず、理由を出して終わる。更新の前に .env へ 1 行足す。

<!-- prose points[key.3].text -->
入れ替えた最初の起動で、全部のブラウザのログインが 1 回切れる。合言葉を入れ直せば戻り、記録は失わない。

<!-- prose points[key.4].text -->
収集アプリの合言葉を変えても、画面のログインは切れなくなる。ログインを締め出すのは、画面の合言葉か印の鍵を変えたとき。期限を設定しなければ、今どおり日が経っても切れない。

<!-- prose features[noauth].summary -->
生存確認・ログイン・ログアウト。ほかは引数を見る前に 401。

<!-- prose points[noauth.1].text -->
資格情報なしで通るのは、生存確認（200）とログアウト（204）。ログインは画面の合言葉そのものを資格情報として受け、無ければ 401。ほかは全部、引数や本文の形にかかわらず 401。

<!-- prose points[noauth.2].text -->
試験は API の説明（docs/openapi.json）に載る全部の口から一覧を作る。後の Story が足した口も、試験から黙って抜けない。

<!-- prose features[tools].summary -->
読む求めは画面の合言葉でログインした印で行う。送る求めは収集アプリの合言葉のまま。

<!-- prose points[tools.1].text -->
収集アプリの合言葉で記録を読んでいた台本と手順は 403 になる。縦串・e2e・録画・確認の手順書・偽データの台本を、印で読む形に替える。本人が手で打つ読みの curl も同じ。

<!-- prose points[tools.2].text -->
新しい tools/web-login.sh が、画面の合言葉でログインして印を 1 行出す。台本が一覧の外を収集アプリの合言葉で呼んでいないかは、新しい検査の台本が見る。

<!-- prose points[tools.3].text -->
道具がログインするたびに、期限の無い印が 1 本増える。tasks には道具がログアウトする手順が無い（IMP-01）。

<!-- prose features[dist].summary -->
携帯と PC は同じ合言葉のまま。端末ごとの発行は ST56 が作る。

<!-- prose points[dist.1].text -->
携帯と PC は同じ 1 本の合言葉のまま、その範囲だけを締める。端末ごとの発行と取り消し、どの資格情報で届いたかの記録は ST56 が作る。

<!-- prose points[dist.2].text -->
ST56 までに届いた記録は、どの資格情報（どの端末）で届いたかを持たない。後から足せない。

<!-- prose impact.scenarios[S1].result -->
403。記録は返らず、作り直しも走らない

<!-- prose impact.scenarios[S1].lost -->
API の合言葉だけで読む手軽さ

<!-- prose impact.scenarios[S2].result -->
起動せず、理由（鍵が無い）を出して終わる。収集アプリは溜めて待つ

<!-- prose impact.scenarios[S2].lost -->
足すまでの間、画面も送信も止まる

<!-- prose impact.scenarios[S3].result -->
一覧に足すまで、収集アプリの合言葉では 403。種類を足したときは、振り分けを決めるまでコンパイルが落ちる

<!-- prose impact.scenarios[S3].lost -->
足し忘れた間の、その口での収集アプリの動作

<!-- prose impact.scenarios[S4].result -->
電話のブラウザのログインは切れずに残り、全部を読める

<!-- prose impact.scenarios[S4].lost -->
API_TOKEN を変えるだけで画面のログインも締め出せたこと

<!-- prose impact.rules[R1].legit -->
API の合言葉で読んでいた本人の curl と台本も止まる。後の Story が収集アプリに新しい口を使わせるとき、一覧に足し忘れると収集アプリが 403 になる

<!-- prose impact.rules[R2].legit -->
鍵の行が無い .env のまま更新した本人のサーバも起動しない（自動のインストール・更新でも）

<!-- prose impact.rules[R3].legit -->
正当な呼び手は資格情報を付ける。この分析では止まる正当な呼び手を見つけていない

<!-- prose impact.rules[R4].legit -->
本人の全部のブラウザで 1 回ログインし直しになる（記録は失わない）

<!-- prose impact.findings[IMP-01].title -->
道具がログインするたびに、期限の無い印が残る

<!-- prose impact.findings[IMP-01].text -->
読む道具（tools/web-login.sh）は画面の合言葉でログインして印を作る。ログインの印は既定で期限が無く、tasks には道具がログアウトする手順が無い。縦串・確認の手順・手で打つ curl を走らせるたびに、全部を読める印が DB に 1 本ずつ増え、使い終わっても有効なまま残る。

<!-- prose impact.findings[IMP-02].title -->
自動のインストールと更新が、ログインの印の鍵を作るかは決まっていない

<!-- prose impact.findings[IMP-02].text -->
新しいサーバは WEB_SESSION_KEY が無いと起動しない。ST44 は手で .env に 1 行足す手順と、tools/env-init.sh が行を足すことを用意する。ST55（インストール）は DB の合言葉を自動で作ると書いているが、この鍵には触れていない。ST58（更新）が既にある設定に行を足すかも決まっていない。

<!-- prose impact.findings[IMP-03].title -->
締めていないサーバへ戻すと、締めが黙って外れ、切ったはずのログインが戻る

<!-- prose impact.findings[IMP-03].text -->
前の版のサーバに戻す（ST52 のベータ版から正式版へ切り替える場合を含む）と、API の合言葉でまた全部が読める。サーバは止まらず、何も知らせない。さらに、印の行は無効にされず鍵が合わなくなるだけなので、戻した前のサーバでは入れ替えの前に作ったログインがまた有効になる。WEB_SESSION_KEY を変えて落とした電話のブラウザを締め出していても、戻すとそのログインが戻る（画面の合言葉を変えて締め出したものは戻らない）。

<!-- prose impact.scope.covers[0].text -->
API の合言葉（携帯と PC の共有の 1 本）で通る口と、ログインの印の鍵

<!-- prose impact.scope.preconditions[0].text -->
収集アプリが呼ぶ口が 9 つの中にあること（携帯と PC のソースで確かめた）

<!-- prose impact.scope.preconditions[1].text -->
送信契約の版の口は、画面のログインでは今どおり断る

<!-- prose impact.scope.preconditions[2].text -->
資格情報なしで通るのは、生存確認・ログイン・ログアウトの 3 つだけ

<!-- prose impact.scope.preconditions[3].text -->
取り込みは、許可の一覧の中では今どおり読み出しの記録に残さない

<!-- prose impact.scope.preconditions[4].text -->
期限を設定したときだけ、過ぎた印は使えない

<!-- prose impact.scope.preconditions[5].text -->
衛星の登録と資格情報の操作は、画面のログインでだけできる

<!-- prose impact.scope.preconditions[6].text -->
通知の判断は、時刻と記録の有無の真偽だけを返す

<!-- prose impact.scope.preconditions[7].text -->
印の鍵はサーバだけが持ち、ログ・読み出しの記録・応答・DB に出さない

<!-- prose impact.scope.preconditions[8].text -->
画面の経路から読むときだけ、ログインの印を求める（画面を配る側は合言葉を付け足さない）

<!-- prose impact.scope.excludes[0].text -->
端末ごとの資格情報と、どの資格情報で届いたかの記録（ST56）

<!-- prose impact.scope.excludes[1].text -->
衛星の資格情報の範囲

<!-- prose impact.scope.residual[0].text -->
漏れた値で偽の記録を送ること・記録の中身を返さない 4 つの読み（途絶したソースと時刻・気分を書いたかの真偽・原本を受け取り済みか）は残る

<!-- prose impact.scope.residual[1].text -->
締める前に読まれた分は戻らない

<!-- prose impact.scope.residual[2].text -->
道具が作った期限の無い印（IMP-01）と、自動の更新が鍵を作るか（IMP-02）

