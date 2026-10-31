# ST51 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
サーバ・携帯・PC の収集アプリの版が変わったことが、後から書き換えられない記録として残り始める。稼働状況の途絶が「更新で止めた」ものか「壊れていた」ものかを、本人が全記録の書き出しを読んで区別できるようになる。

<!-- prose impact.overview -->
サーバ・携帯・PC の版が変わったことが、追記のみの記録に残り始める。サーバは更新の履歴の表へ 1 行ずつ書き、収集アプリの版の変化は生存信号に載せた版から導く。どちらも全記録の書き出しに入る。一方、画面・API・格子・途絶の判定は変わらず、区別には書き出しを読む必要がある。版の検査で止まった回と、退避から戻して消えた落ちた回の行は、この Story の範囲では残らない。

<!-- prose impact.problem -->
更新の間は収集もサーバも止まり、稼働状況の格子にはその時間が途絶として出る。後から見て「更新で止めた」のか「壊れていた」のかを区別する材料が無い。

<!-- prose walkthroughs[W1].title -->
サーバを何度か入れ替えたときに残る行

<!-- prose walkthroughs[W1].scene -->
本人がサーバ（S-01）を新しい組み立てに入れ替えていく。途中で 1 回、移行が落ちる。

<!-- prose walkthroughs[W1].why -->
書くのは移行の終わりと起動の検査の後で、版が前の通った行と違うときだけ。古いサーバが新しい DB に書かないよう、版の検査で止まった回は書かない。

<!-- prose walkthroughs[W1].figure.caption -->
入れ替えごとに更新の履歴へ残る行（A〜D は説明用の組み立ての印）

<!-- prose walkthroughs[W1].cases[0].result -->
前の版・今の版・当てた版の名前を持つ 1 行が残る

<!-- prose walkthroughs[W1].cases[1].result -->
行は増えない

<!-- prose walkthroughs[W1].cases[2].result -->
落ちた版と SQLSTATE を持つ 1 行が残る。次の行の前の版は最後に通った B

<!-- prose walkthroughs[W1].cases[3].result -->
更新の履歴には何も残らない。止まっていた間の途絶の理由は記録に残らない（IMP-01）

<!-- prose walkthroughs[W1].exceptions[0].text -->
表がまだ無い DB で落ちた回は書けず、警告だけ出る

<!-- prose walkthroughs[W2].title -->
携帯の収集アプリを更新した後に、溜まった信号が届く

<!-- prose walkthroughs[W2].scene -->
電波の届かない所で、版 A の携帯の収集アプリが生存信号を溜める。本人がアプリを版 B に更新して開き、その後で溜まった信号が届く。

<!-- prose walkthroughs[W2].why -->
版は信号を積む時点で焼き込まれ、変化は発信の時刻の順で数える。届いた順で数えると、溜まった A が B の後に入り、版が戻ったように見える。

<!-- prose walkthroughs[W2].figure.caption -->
積んだ時刻と届いた順（A・B は説明用の版）

<!-- prose walkthroughs[W2].cases[0].result -->
A → B の変化が 12:00 に 1 行だけ出る

<!-- prose walkthroughs[W2].cases[1].result -->
申告の始まりで、変化の行は出ない

<!-- prose walkthroughs[W2].cases[2].result -->
端末識別子が変わり、変化の行は出ない。新しい端末の申告の始まりに見える

<!-- prose walkthroughs[W2].exceptions[0].text -->
端末の時計が戻ると、版が行き来して見えることがある（並べ方の代償。ビューなので並べ方を直せば出し直せる）

<!-- prose walkthroughs[W3].title -->
移行が落ちて、退避から D-01 を戻す

<!-- prose walkthroughs[W3].scene -->
本人がサーバを入れ替えて移行が落ちた。退避（移行の前に取った DB の写し）から D-01 を手で戻し、前の組み立てで起動し直す。

<!-- prose walkthroughs[W3].why -->
落ちた回の行は D-01 の中だけに書き、退避は移行の前に取る。戻すと、退避の後に書いた行はすべて消える。

<!-- prose walkthroughs[W3].figure.caption -->
戻す前と後の更新の履歴（A・C は説明用の印）

<!-- prose walkthroughs[W3].cases[0].result -->
落ちた回の行は退避と一緒に消え、その回の試みは記録に残らない

<!-- prose walkthroughs[W3].cases[1].result -->
failed の行が残り、次の行の前の版は A

<!-- prose walkthroughs[W3].exceptions[0].text -->
戻した後に「落ちた試み」と「戻した」の 2 行を書き直すのは ST58

<!-- prose model.caption -->
版の変化が残る 2 つの道

<!-- prose features[s01].summary -->
サーバ（S-01）は移行（migrate）の終わりと起動のときに、自分の版が前と違えば 1 行を書く。

<!-- prose points[s01.1].text -->
移行を走らせ終えたとき、または起動して版の検査を通ったとき、自分の組み立ての印が前の行と違えば 1 行を書く。1 行はいつ・部品（S-01）・きっかけ（migrate か起動か）・前の版・今の版・移行の結果を持つ。

<!-- prose points[s01.2].text -->
同じ組み立てで移行を走らせ直しても、起動し直しても、行は増えない。2 つが同時に書いても 1 行になる。

<!-- prose points[s01.3].text -->
移行が落ちた回も 1 行残る。残すのは落ちた版の名前と SQLSTATE（5 文字の誤りの符号）だけで、DB の誤りの本文は残さない。本文は行の値を含みうるので、標準エラーにも出さない。

<!-- prose points[s01.4].text -->
「前の版」は最後に通った行の今の版にする。落ちた回は DB を動かしていないので数えない。並びは書いた順で決め、時計の時刻では決めない。

<!-- prose points[s01.5].text -->
知らない版の当たった DB で止まった移行（終了コード 3）と、版の検査で断られた起動は、何も書かない。古いサーバが新しい DB に書かないようにするためである。

<!-- prose points[s01.6].text -->
移行が落ちた回の行は DB（D-01）の中にしか無い。退避から D-01 を戻すと、その行も一緒に消える。戻した後に書き直すのは ST58 が受け持つ。

<!-- prose points[s01.7].text -->
更新の履歴の表がまだ無い・接続が切れたなどで書けないときは、警告を出すだけにする。移行の終了コードも起動の成否も変えない。

<!-- prose features[form].summary -->
更新の履歴は記録の schema の追記のみの表で、列の形を固定し、誰も書き換えられない。

<!-- prose points[form.1].text -->
更新の履歴は記録の schema（`core`）の表 `update_history` に置く。列の名前・型・NULL の可否は Requirement の表で固定し、形を変えるときは列か値を足すだけにする。

<!-- prose points[form.2].text -->
行は誰も書き換えられず、消せない（DB の所有者でも断る）。サーバの実行時の役割にできるのは、読むことと足すことだけである。

<!-- prose points[form.3].text -->
実行時の役割は起動の行を書くので、足す権限を持つ。サーバが乗っ取られると偽の行を足せる。ただし本物の行は消せず、書き換えもできない。

<!-- prose features[beat].summary -->
携帯（C-01）と PC（C-02）の収集アプリは、どの生存信号にもそれを積んだ時に動いていた版を載せる。

<!-- prose points[beat.1].text -->
収集アプリは生存信号を未送信に積む時点で、自分の組み立ての印を欄 `client_version` と原文の両方に書き込む。送る時点の版では書き換えない。

<!-- prose points[beat.2].text -->
サーバは版の値の形を検査せず、値を理由に信号を断らない。版を持たない信号も今までどおり受け付ける。

<!-- prose points[beat.3].text -->
版を足す前に溜まっていた生存信号は、版の無いまま送る（今の版を後から付けない）。

<!-- prose points[beat.4].text -->
サーバの中の書庫の取り込みが積む生存信号には版を入れない。生存信号の冪等キーの作り方も変えない。

<!-- prose points[beat.5].text -->
記録の版（ST49）の振る舞いはそのまま残る。記録は積んだ時の版で送り、溜めた記録の行の知らない欄も落とさない。

<!-- prose features[derive].summary -->
端末・部品ごとに生存信号を発信の時刻の順に並べ、版が変わった所を 1 行として読み口（ビュー）に出す。

<!-- prose points[derive.1].text -->
変化の行は表に書いて凍結せず、生存信号から毎回導く。並べ方を後から直しても、過去の分まで出し直せる。

<!-- prose points[derive.2].text -->
並べる順は、受信の時刻ではなく発信の時刻である。更新の前に溜まった信号が後から届いても、古い位置に入るので偽の変化にならない。

<!-- prose points[derive.3].text -->
版を持たない信号は並びから外す。端末・部品で初めて版を持った信号は「申告の始まり」で、変化として出さない。

<!-- prose points[derive.4].text -->
並べる単位は利用者・端末・部品（C-01 か C-02）である。論理ソースの名前が `c01-` か `c02-` で始まる信号だけを並べる。同じ端末の C-01 の 2 つのソースが同じ版を申告しても、変化は 1 行になる。

<!-- prose points[derive.5].text -->
携帯の収集アプリを消して入れ直すと端末識別子が変わり、変化ではなく「別の端末の申告の始まり」に見える。生存信号の行と版は残る。

<!-- prose features[export].summary -->
書き出しの zip に、更新の履歴と版の変化のファイルを足し、生存信号のファイルに版の列を足す。

<!-- prose points[export.1].text -->
zip に `update_history.jsonl`（更新の履歴の全行・全列）と `collector_version_changes.jsonl`（版の変化）を足し、`heartbeats.jsonl` に列 `collector_version` を足す。

<!-- prose points[export.2].text -->
説明書に 3 つの説明を書く。落ちた回の行は戻すと消えることがある・版の変化はサーバが確かめていない申告・生存信号の版が空なら申告なし。

<!-- prose points[export.3].text -->
書き出しの決まりは変えない。台帳・導出の表は入れず、分類の無い表や列があれば書き出しを始めない。

<!-- prose features[keep].summary -->
更新の間の空きは今までどおり途絶として出る。区別は本人が書き出しを読んで行う。

<!-- prose points[keep.1].text -->
版の列・更新の履歴・版の変化を、稼働状況・成功条件 1 の達成日・途切れの判定と通知の入力にしない。

<!-- prose points[keep.2].text -->
更新の間の空きは、更新の履歴があっても途絶として出る。画面にも API にも更新の印は出ず、区別するには書き出しを開いて読む。

<!-- prose impact.scenarios[S1].result -->
サーバは止まったままで、更新の履歴には行が無い。止まっていた理由は migrate の標準エラーの報告にしか出ない

<!-- prose impact.scenarios[S1].lost -->
止まっていた間の途絶を、後から「更新で止めた」と区別すること

<!-- prose impact.scenarios[S2].result -->
行は消えずに残る。後から読む人は、開発の組み立ての行も本物の更新と並べて読むことになる

<!-- prose impact.scenarios[S2].lost -->
更新の履歴から誤った行を取り除くこと

<!-- prose impact.scenarios[S3].result -->
戻した D-01 には落ちた回の行が無い。前の組み立てで起動し直しても行は増えない

<!-- prose impact.scenarios[S3].lost -->
落ちた試みがあったことを、更新の履歴から読むこと（ST58 の書き直しまで）

<!-- prose impact.scenarios[S4].result -->
版の変化の行は出ず、新しい端末の申告の始まりに見える。生存信号の行と版は残る

<!-- prose impact.scenarios[S4].lost -->
入れ直しの更新を、変化の 1 行として読むこと

<!-- prose impact.rules[R1].legit -->
止めない（断らない向きの規則）

<!-- prose impact.rules[R2].legit -->
外した信号の行は残り、数えないだけ。並べ方は後から直して出し直せる

<!-- prose impact.rules[R3].legit -->
更新の履歴は分類に載せ、ビューは走査に載らないので、ST51 の追加で書き出しが止まる経路はこの分析では見つけていない

<!-- prose impact.rules[R4].legit -->
後の版が生存信号に欄を足してから古い版へ戻すと、その欄は落ちうる。版の欄は原文にも入るので残る

<!-- prose impact.rules[R5].legit -->
本人が版を切り替えようとして止まった回も残らない。その間の途絶の理由は記録に無い

<!-- prose impact.rules[R6].legit -->
誤って本番の D-01 に書いた行（開発の組み立てを動かした回など）も消せない

<!-- prose impact.rules[R7].legit -->
落ちた理由の詳しい文は更新の履歴から読めない。SQLSTATE と版の名前から辿る

<!-- prose impact.findings[IMP-01].title -->
版の検査で止まった試みは、更新の履歴に残らない

<!-- prose impact.findings[IMP-01].text -->
古い組み立てやベータ版の DB を使う切り替えで終了コード 3 になった回は、更新の履歴に何も書かない。止まっていた間の途絶の理由は、migrate の標準エラーの報告にしか出ない

<!-- prose impact.findings[IMP-02].title -->
区別に使う見せ方を受け持つ Story を、この分析では見つけていない

<!-- prose impact.findings[IMP-02].text -->
更新の履歴と版の変化は書き出しにだけ出る。画面で途絶と並べて見せる Story は、この分析では見つけていない。それまでは、本人が zip を開いて途絶の時刻と突き合わせることになる

<!-- prose impact.findings[IMP-03].title -->
同じ commit の未 commit の変更つきの組み立てどうしの入れ替えは、行にならない

<!-- prose impact.findings[IMP-03].text -->
組み立ての印は「commit + .dirty」で、変更の中身を持たない。同じ commit の .dirty の組み立てを入れ替えても、印が同じなので行は増えない

<!-- prose impact.findings[IMP-04].title -->
ST58 より前に退避から戻すと、落ちた回の行が消える

<!-- prose impact.findings[IMP-04].text -->
落ちた回の行は D-01 の中だけにあり、退避より後に書かれる。ST58 の戻しができるまでは手で戻すので、戻すとその回の試みが更新の履歴から消える

<!-- prose impact.findings[IMP-05].title -->
既存のアプリの役割の試験が、更新の履歴の REVOKE で落ちる

<!-- prose impact.findings[IMP-05].text -->
既存の試験は、記録の schema のどの表でも実行時の役割が UPDATE・DELETE できることを確かめる。D10 が更新の履歴だけ外すので、この試験は落ちる。tasks に試験を直す手順が無い

<!-- prose impact.scope.covers[0].text -->
サーバの移行の終わりと、版の検査を通った起動で、版が前の通った行と違う回

<!-- prose impact.scope.covers[1].text -->
ST51 の後に組み立てた収集アプリが積む生存信号と、そこから導く版の変化

<!-- prose impact.scope.covers[2].text -->
全記録の書き出しの 2 ファイルと生存信号の版の列

<!-- prose impact.scope.preconditions[0].text -->
起動の行は、版の検査を通った後だけ書く。書けないときは警告だけで、終了コードと起動の成否は変えない

<!-- prose impact.scope.preconditions[1].text -->
更新の履歴の形は、列か値を足すだけで変える。実行時の役割は読むことと足すことだけ

<!-- prose impact.scope.preconditions[2].text -->
版の変化は `c01-`・`c02-` のソースだけを、利用者・端末・部品ごとに発信の時刻の順で数える

<!-- prose impact.scope.preconditions[3].text -->
収集アプリの版は ST49 の組み立ての印（release だけが .debug を持たない）を、積む時点で載せる

<!-- prose impact.scope.preconditions[4].text -->
書き出しは今の分類の決まり（台帳・導出は入れない）のまま、ファイルを足すだけ

<!-- prose impact.scope.excludes[0].text -->
終了コード 3 で止まった移行・版の検査で断られた起動（何も書かない）

<!-- prose impact.scope.excludes[1].text -->
収集アプリでないソース（書庫の取り込み・衛星）の信号

<!-- prose impact.scope.excludes[2].text -->
画面・API・格子の印・途絶の判定の変更（Q5）

<!-- prose impact.scope.excludes[3].text -->
戻した後に落ちた試みと戻したことを書き直すこと（ST58）・更新の後に C-01 を起き直させること（ST58）・C-02 を並べるときの識別子（ST54）

<!-- prose impact.scope.residual[0].text -->
版の検査で止まった試みは更新の履歴に残らない（IMP-01）

<!-- prose impact.scope.residual[1].text -->
ST58 より前に手で戻した回の痕跡が消える（IMP-04）

<!-- prose impact.scope.residual[2].text -->
ST58 より前に退避から戻すと、落ちた回の行が消える

<!-- prose impact.scope.residual[3].text -->
区別は書き出しを読むことでしかできない（IMP-02）

