# ST54 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
開発版を、正式版と同じ携帯・同じ PC・同じ機械に並べて入れて試せるようになる。並べても、正式版の端末内の記録・自動起動・通知・DB・送り先を開発版が取り合わない。開発版のリリースには携帯の APK も並び、「サポートしない」と先頭に書かれる。

<!-- prose impact.overview -->
開発版の携帯・PC の収集アプリとサーバを、正式版と同じ携帯・PC・機械に並べられるようになる。名前と既定を分け、同じ系統の相手とだけ送り・起動する。新しくできなくなることとして、この分析で見つけたのは次の 3 つ: 開発版の DB を正式版のサーバで使うこと、系統の違うサーバへ送ること（通知なしで手元に積む）、開発版の公開を始めた後に main の移行を直しの移行なしで直すこと。PC の開発版では、tasks のとおりに作るとブラウザの履歴の除外が効かない恐れがある（IMP-01）。

<!-- prose impact.problem -->
開発版を試すには、正式版と同じ携帯・PC に入れる必要がある。いまは携帯の配布物に開発版が無い。PC は自動起動の項目と通知の差出人が固定で、開発版を入れると正式版の分を上書きする。サーバは DB の繋ぎ先を取り違えると、開発版の移行で正式版の DB を先へ進め、正式版のサーバが起動しなくなる。

<!-- prose uses[U1].title -->
毎日の開発版で新しい振る舞いを、普段の記録を止めずに試す

<!-- prose uses[U1].trigger -->
main に入った新しい収集の振る舞いを、配る前に自分の携帯と PC で数日試したい。だが正式版の記録は 1 日も欠かしたくない。

<!-- prose uses[U1].outcome -->
開発版の記録は開発版の DB に、正式版の記録は正式版の DB に分かれて溜まる。開発版を消しても正式版の記録・自動起動・通知は残る。

<!-- prose uses[U1].without -->
携帯は開発版が正式版を上書きし、PC は自動起動の項目と通知の差出人を取り合う。開発版を試す間は、正式版を止めることになる。

<!-- prose uses[U1].steps[0].do -->
開発版のリリース（説明文の 1 行目が「この開発版はサポートしません。」）から 4 つの配布物を落とす

<!-- prose uses[U1].steps[0].see -->
ashiato-collector-dev-<版番号>.apk と、-dev の実行ファイルを含む PC の収集アプリの zip

<!-- prose uses[U1].steps[1].do -->
開発版の DB を別の器（ポート 55433）に立て、開発版のサーバを起こす

<!-- prose uses[U1].steps[1].see -->
サーバは 127.0.0.1:18788 で待ち受ける。正式版のサーバ（18787）はそのまま動いている

<!-- prose uses[U1].steps[2].do -->
PC に開発版の zip を展開し、正式版と同じ置き場と端末識別子を渡して起動する

<!-- prose uses[U1].steps[2].see -->
スタートアップに ashiato-collector.cmd と ashiato-collector-dev.cmd が並ぶ。開発版は置き場の下の dev に未送信を積み、端末識別子は pc-01-dev

<!-- prose uses[U1].steps[3].do -->
携帯に開発版の APK を入れる

<!-- prose uses[U1].steps[3].see -->
ランチャーに「あしあと。」と「あしあと。開発版」が並ぶ。正式版の未送信はそのまま

<!-- prose walkthroughs[W1].title -->
サーバと DB を取り違えたとき

<!-- prose walkthroughs[W1].scene -->
本人が同じ機械で正式版と開発版のサーバを動かしていて、開発版のサーバの DATABASE_URL に正式版の DB を書いてしまう。別の日には、開発版をやめて開発版の DB を正式版のサーバで使おうとする。

<!-- prose walkthroughs[W1].why -->
DB の系統は版の記録の最初の行で決まり、後から変えない。違う系統の部品は、退避も書かずに止まる。取り違えで正式版の DB が開発版の移行で先へ進むと、正式版のサーバが起動しなくなるため。

<!-- prose walkthroughs[W1].figure.caption -->
繋いだ組み合わせと結果（版番号は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
待ち受けを開かずに終了コード 4。正式版の DB は先へ進まず、正式版のサーバはそのまま起動できる

<!-- prose walkthroughs[W1].cases[1].result -->
今どおり当てて起動する

<!-- prose walkthroughs[W1].cases[2].result -->
正式版のサーバも migrate も終了コード 4 で止まる。開発版の DB の記録を正式版へ移す道は仕様に無い（IMP-03）

<!-- prose walkthroughs[W1].exceptions[0].text -->
説明用の版番号。実際の開発版の公開はまだ始まっていない

<!-- prose walkthroughs[W1].exceptions[1].text -->
版の記録より前の古い DB は正式版とみなす

<!-- prose walkthroughs[W2].title -->
PC に並べた開発版で、除外が効くか

<!-- prose walkthroughs[W2].scene -->
本人は正式版の PC の収集アプリで「パスワード管理」の窓と、ある銀行のサイトの閲覧履歴を除外している。同じ置き場 C:\ashiato を渡して開発版を並べる。

<!-- prose walkthroughs[W2].why -->
開発版は自分の置き場を渡された置き場の下の dev にする。除外の読み込み口が置き場から読み先を作っていると、dev の下を読む。前景の窓の口は D2 が直すが、履歴の取り込みの口は別の場所で置き場から読み先を作っている。

<!-- prose walkthroughs[W2].figure.caption -->
開発版が読む除外の登録（置き場は説明用）

<!-- prose walkthroughs[W2].cases[0].result -->
開発版も題名を記録しない

<!-- prose walkthroughs[W2].cases[1].result -->
tasks のとおりに作ると、開発版はその閲覧履歴を取り込み、開発版のサーバへ送る恐れがある

<!-- prose walkthroughs[W2].exceptions[0].text -->
説明用の置き場と除外。実際の除外の中身は確かめていない

<!-- prose model.caption -->
系統（開発版か正式版か）は組み立てのときの版番号で決まり、どの部品も同じ系統の相手とだけ組む

<!-- prose features[channel].summary -->
組み立てのときの版番号だけで決まり、3 つの部品が同じ規則で読む。

<!-- prose points[channel.1].text -->
版番号に -dev. を含む組み立てが開発版で、それ以外（ベータ版を含む）は正式版。サーバ・携帯の収集アプリ・PC の収集アプリが同じ規則で読む。

<!-- prose points[channel.2].text -->
版番号を渡さずに手で組んだ版は正式版として振る舞う。今の開発・縦串・確認バッチは、どれも正式版の名前と正式版の DB のまま動く。実行時の環境変数で系統を変える口は無い。

<!-- prose features[android].summary -->
開発版は識別子 dev.ashiato.collector.dev の別のアプリとして入り、名前「あしあと。開発版」で見分ける。

<!-- prose points[android.1].text -->
開発版を入れても正式版のアプリは残り、正式版の端末内の記録（未送信を含む）もそのまま残る。端末内の記録の置き場・権限・通知の設定は Android が識別子ごとに分ける。

<!-- prose points[android.2].text -->
アプリの名前は開発版が「あしあと。開発版」、正式版は「あしあと。」のまま。アイコンは同じ。常駐の通知の見出しもアプリの名前で分かれる。

<!-- prose points[android.3].text -->
開発版も正式版と同じ収集を全部動かす。並べて入れると、位置の取得と常駐の通知が 2 つずつになる。権限は開発版にも別に与える。

<!-- prose features[pc].summary -->
開発版は、自動起動の項目・通知の差出人・実行ファイル・置き場・端末識別子を自分で分ける。

<!-- prose points[pc.1].text -->
開発版は自動起動の項目・通知の差出人・実行ファイルの名前を正式版と分ける。開発版を仕込んでも正式版の項目と差出人の登録は書き換わらない（逆も同じ）。

<!-- prose points[pc.2].text -->
開発版は、渡された置き場の下の dev を自分の置き場にし、端末識別子の末尾に -dev を付ける（すでに -dev で終われば付けない）。同じ置き場を渡しても、2 つとも起動して別々に未送信を持つ。正式版の置き場と端末識別子は今のまま。

<!-- prose points[pc.3].text -->
除外の登録は、開発版も渡された置き場の exclusions.json を読む。正式版と同じ置き場を渡せば正式版の除外が効き、別の置き場を渡すと効かない（開発版は正式版の置き場を自分では知らない）。

<!-- prose points[pc.4].text -->
自動起動の項目には、dev を掘る前の置き場と -dev を付ける前の端末識別子を書く。次のログオンで同じ規則がもう一度当たるので、二重に掘らない。開発版は初回にブラウザの履歴を全件取り込む（正式版の初回と同じ量）。

<!-- prose features[send].summary -->
サーバが契約の問い合わせの応答の見出しで系統を答え、収集アプリは系統の違うサーバへ何も送らない。

<!-- prose points[send.1].text -->
サーバは GET /contract の応答に見出し Ashiato-Channel（dev か official）を載せる。答えの本文 {"contract_version": …} は変えない。系統が同じサーバには今どおり送る。

<!-- prose points[send.2].text -->
系統が違えば、その回は記録・生存信号・破棄の報告・原本の欠け・原本のどれも送らず、未送信を手元に残してログに理由を書く。正式版の収集アプリも開発版のサーバへは送らない（Q6 が聞いた向きの逆。AI が足した仮の決定）。

<!-- prose points[send.3].text -->
送らない間も通知は出ない。「サーバを更新してください」の条件にも数えない。送り先を取り違えて設定すると、気づくのはログか、サーバの稼働状況の途絶から。

<!-- prose features[db].summary -->
DB は最初に移行を当てた組み立ての系統のもの。違う系統は何も書かずに止まり、開発版の DB が main とずれたら main 側で直す。

<!-- prose points[db.1].text -->
DB の系統は、版の記録の最初の行の組み立ての版番号で決まる（新しい印の表は足さない）。違う系統のサーバと migrate は、退避も版の適用も更新の履歴も書かずに終了コード 4（error: kind=channel_mismatch）で止まる。版の食い違いより先に見る。

<!-- prose points[db.2].text -->
空の DB は、最初に migrate を当てた組み立ての系統になる。版の記録より前の古い DB（版の記録は無いが ashiato の表がある）は正式版とみなし、開発版の migrate は当てずに止まる。版の記録の行が 1 つも無い DB はどちらの系統でも当てる。

<!-- prose points[db.3].text -->
main で移行の名前を変えた・消したら、migrations/LINEAGE に旧名の対応（renamed / removed）を書く。サーバは旧名で当てた行を知っている版として読み、当て直さない。対応が無ければ今どおり知らない版で止まり、その名前と当てた組み立てを示す。版の記録の行は書き換えない。

<!-- prose points[db.4].text -->
開発版に載った移行（印 nightly の版の移行）は、旧名の対応か直しの移行（rewritten）を足さずに変えると CI が落ちる。開発版は毎日 main から作り直すので、開発版の公開を始めた後は、main に入った移行を翌日以降に直すときも直しの移行が要る。戻し手順（.down.sql）と nightly の後に足した移行は自由に直せる。

<!-- prose features[server].summary -->
開発版のサーバは、待ち受け・ログインのクッキー・退避の置き場の既定を正式版と分ける。

<!-- prose points[server.1].text -->
開発版のサーバの既定は、待ち受け 127.0.0.1:18788・クッキー ashiato_session_dev・退避 backups/pre-migrate-dev。正式版の既定（18787・ashiato_session・backups/pre-migrate）は変えない。BIND と MIGRATE_BACKUP_DIR を渡せば系統に依らずその値を使う。

<!-- prose points[server.2].text -->
クッキーはポートで分かれないので、同じ機械で 2 つの画面を開くとログインを上書きし合う。そのため名前を分けた。開発版のサーバは ashiato_session_dev だけを読み書きし、正式版のクッキーでは 401。

<!-- prose features[release].summary -->
開発版の作り直しが携帯の APK も組み、開発版の鍵で署名し、説明文の先頭に「サポートしない」を書いて公開する。

<!-- prose points[release.1].text -->
開発版のリリースも 4 つの配布物を並べる（携帯の APK は ashiato-collector-dev-<版番号>.apk）。APK は組み立ての job では署名せず、公開の job の中でだけ開発版の鍵で署名し、指紋を確かめてから入れ替える。開発版の置き場に正式版の名前の APK があれば公開しない。

<!-- prose points[release.2].text -->
開発版の鍵か指紋の期待値が置き場に無ければ署名せず、開発版のリリースは前の版のまま残り、落ちた理由に「開発版の鍵が無い」を出す。

<!-- prose points[release.3].text -->
公開のたびに説明文をリポジトリの決まった文で書き直し、1 行目が「この開発版はサポートしません。」で始まることを確かめてから印 nightly を動かす。始まらなければ公開の手順は落ちる。

<!-- prose points[release.4].text -->
この Story では本物の開発版の鍵を作らず、開発版の公開も始めない。完了は、手で起こした開発版の組み立てを使い捨ての開発版の鍵で署名した下書きのリリース（印を作らない）で確かめる。本物の公開は ST53 の後に本人が始める。

<!-- prose points[release.5].text -->
開発版の印 nightly は作り直すたびに動かす方式のまま。古い nightly を持つ手元の clone では git fetch --tags が落ちるので、手順書に git fetch --tags --force を書く。

<!-- prose impact.scenarios[S1].result -->
正式版のサーバも migrate も終了コード 4 で止まる。開発版の DB はそのまま残る

<!-- prose impact.scenarios[S1].lost -->
開発版の DB の記録を正式版のサーバで読むこと

<!-- prose impact.scenarios[S2].result -->
CI の検査が落ち、直しの移行を足して rewritten を書くよう示す

<!-- prose impact.scenarios[S2].lost -->
移行の .sql をその場で直すこと

<!-- prose impact.rules[R1].legit -->
開発版をやめて、開発版の DB に溜めた記録を正式版のサーバで使い続けたい本人も止まる

<!-- prose impact.rules[R2].legit -->
この分析では、止めて困る正当な送り先を見つけていない（開発版のサーバで正式版の記録を試したい場合は D7 の反転条件）

<!-- prose impact.rules[R3].legit -->
main に入れた移行の書き間違いを翌日に直したい開発者も、直しの移行を足すまで止まる

<!-- prose impact.rules[R4].legit -->
この分析では見つけていない。手元で開発版の名前を試すには、版番号に -dev. を渡して組めばよい

<!-- prose impact.rules[R5].legit -->
この分析では見つけていない（開発版の鍵を置くまで開発版は作り直されないが、公開を始めるのは鍵を置いた後）

<!-- prose impact.rules[R6].legit -->
この分析では見つけていない。正式版の値は変えない

<!-- prose impact.rules[R7].legit -->
この分析では見つけていない

<!-- prose impact.rules[R8].legit -->
この分析では見つけていない（例外は ST54 までの期限つきだった）

<!-- prose impact.findings[IMP-01].title -->
PC の開発版で、ブラウザの履歴の除外が効かない恐れ

<!-- prose impact.findings[IMP-01].text -->
Requirement は除外の登録を渡された置き場の exclusions.json から読むと書く。だが design D2 と tasks 6.1 が直すのは前景の窓の除外の読み込み口（main.rs）だけ。ブラウザの履歴の取り込みは、自分の置き場から除外の読み先を別に作っている（history/collect.rs:315）。開発版の置き場は <置き場>/dev で、PC の収集アプリには除外を書く口が無い（design D2）ので、<置き場>/dev/exclusions.json は誰も作らない。tasks のとおりに作ると、開発版の履歴の除外は常に空になる。開発版は初回に過去の履歴を全件取り込むので、除外したサイトの過去の閲覧履歴も全部が開発版のサーバへ送られる。Scenario は前景の窓の題名しか見ないので、試験は通ってしまう。

<!-- prose impact.findings[IMP-02].title -->
更新の失敗からの戻し（ST58）が、終了コード 4 を知らない

<!-- prose impact.findings[IMP-02].text -->
ST58 への申し送りは、migrate の終了コードを 1（移行の失敗。退避がある）と 3（退避を取らずに止まった）で分け、戻すのは 1 の回だけと書く。ST54 は、退避を取らずに止まる終了コード 4 を足す。申し送りには 4 が無い。ST58 が「1 か 3 以外の 0 でない値」を移行の失敗と扱って作ると、4 の回に前の回の退避（それ以後の記録を含まない）を戻しうる。 誤って戻すと、戻した退避より後の記録は復元できない（申し送り自身がそう書く）。この change の tasks は申し送りを書き足さない。

<!-- prose impact.findings[IMP-03].title -->
仕様は、開発版の DB の記録を正式版へ移す手順を定めていない

<!-- prose impact.findings[IMP-03].text -->
DB の系統は後から変わらず、正式版のサーバは開発版の DB で起動しない。開発版をやめて正式版に移るとき、開発版の DB にしか無い記録を正式版で読む道は仕様に無い。Q4 は開発版の DB の作り直しを、開発版にしか無い記録を失うものとして退けた。それと同じ記録が、正式版に移る日には読めなくなる。

<!-- prose impact.scope.covers[0].text -->
同じ携帯・同じ PC・同じ機械に、開発版と正式版を 1 つずつ並べること

<!-- prose impact.scope.covers[1].text -->
収集アプリの送り先とサーバの DB の取り違えを、系統で止めること

<!-- prose impact.scope.preconditions[0].text -->
系統は版番号だけで決まる。版番号を渡さずに手で組んだ版は正式版

<!-- prose impact.scope.preconditions[1].text -->
PC の開発版で正式版の除外が効くのは、正式版と同じ置き場を渡したときだけ

<!-- prose impact.scope.preconditions[2].text -->
見出しを答えない古いサーバは正式版とみなす

<!-- prose impact.scope.preconditions[3].text -->
明示の BIND と MIGRATE_BACKUP_DIR は、系統に依らずその値を使う

<!-- prose impact.scope.preconditions[4].text -->
開発版に載った移行の検査は印 nightly があるときだけ働き、戻し手順と nightly の後に足した移行は見ない

<!-- prose impact.scope.preconditions[5].text -->
旧名の対応は名前の変更と削除に効く。中身の直しは直しの移行を足して当てる

<!-- prose impact.scope.preconditions[6].text -->
完了は使い捨ての鍵と下書きのリリースで確かめる

<!-- prose impact.scope.preconditions[7].text -->
正式な鍵は印 v* の実行にだけ、開発版の鍵は開発版の予定の実行と main からの実行にだけ渡す。PR の組み立ては debug の鍵だけ

<!-- prose impact.scope.preconditions[8].text -->
開発版のリリースの APK は開発版の名前のものだけ。正式版の名前の APK があれば公開しない

<!-- prose impact.scope.preconditions[9].text -->
開発版は幹にその日の変更があった日だけ作り直し、携帯の APK は公開の job の中でだけ署名する

<!-- prose impact.scope.preconditions[10].text -->
携帯の開発版は、識別子から作られる名前を除いて正式版と同じ権限と常駐の口を持つ。名前だけを替える

<!-- prose impact.scope.excludes[0].text -->
本物の開発版の鍵と開発版の公開（ST53 の後）

<!-- prose impact.scope.excludes[1].text -->
収集アプリの接続先の設定（ST56）・サーバのインストーラ（ST55）

<!-- prose impact.scope.excludes[2].text -->
開発版の DB の記録を正式版へ移すこと（IMP-03）

<!-- prose impact.scope.residual[0].text -->
送り先を取り違えても通知は出ず、ログにだけ残る

<!-- prose impact.scope.residual[1].text -->
tasks のとおりに作ると、PC の開発版でブラウザの履歴の除外が効かない恐れ（IMP-01）

<!-- prose impact.scope.residual[2].text -->
直しの移行をどちらの中身で当てた DB にも同じ結果になるように書くことは、機械が見ない（手順書の規約）

