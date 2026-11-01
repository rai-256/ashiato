# ST52 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
印（タグ）を 1 つ付けると、サーバ（画面を含む）・携帯の収集アプリ・PC の収集アプリが同じ版番号で配布元に並ぶ仕組みを作る。印の名前で正式版・ベータ版・開発版に分かれ、配れる形を壊した PR はその PR の CI で落ちる。ただし ST52 では本物の印を打たず、下書きのリリースと使い捨ての鍵で形を確かめるところまで。最初の本物の公開は ST53 の後に本人が始める。

<!-- prose impact.overview -->
印を付けると 3 部品が同じ版で並ぶ仕組みと、PR ごとに配る形を組む検査が入る。版番号は根の Cargo.toml の 1 か所になり、配るサーバは画面も配る。できなくなることとして見つけたのは、今の携帯のアプリを配布版へ上書きで移ること（消して入れ直す）、出した正式版の後に出所を上げるまで開発版を組むこと、配った印を消す・付け替えること。ST52 の範囲では本物の公開は起きず、印を push できる人が正式な鍵に届くか（IMP-01）は確かめていない。

<!-- prose impact.problem -->
いまは配る仕組みが無く、版番号は 3 か所に手書きで、携帯の APK は組んだ PC の debug 鍵で署名されている。配れる形を壊しても、配ろうとした日まで誰も気づかない。ST53〜ST58 は「印を付けると 3 部品が同じ版で並ぶ」を土台にする。

<!-- prose uses[U1].title -->
配れる形を壊した PR に、その PR で気づく

<!-- prose uses[U1].trigger -->
AI が書いた PR が携帯の組み立ての設定を変えた。テストは通っているが、配る形で組めるかは分からない。

<!-- prose uses[U1].outcome -->
印を打つ前、merge の前に、配る形を壊したことが分かって直せる

<!-- prose uses[U1].without -->
印を打った日に初めて組めないと分かる。配った印は消せないので、壊れた版番号を 1 つ飛ばして出し直す

<!-- prose uses[U1].steps[0].do -->
PR の CI を見る

<!-- prose uses[U1].steps[0].see -->
package の job が赤。出力に「ashiato-collector-0.1.0.apk: API_TOKEN が空でない」と項目の名前だけが出て、値は出ない

<!-- prose uses[U1].steps[1].do -->
PR を直して push し直す

<!-- prose uses[U1].steps[1].see -->
package の job が緑。CI の成果物に 4 つの配布物が 0.1.0 で並ぶ

<!-- prose uses[U2].title -->
最初のベータ版を出して、自分のサーバをその版で動かす（ST53 の後）

<!-- prose uses[U2].trigger -->
ST53（許諾の同梱）が済んだ。手元で組んだサーバをやめて、配った版で自分の環境を動かしたい。

<!-- prose uses[U2].outcome -->
自分の環境が配った版と同じ組み立てで動き、更新の失敗が自分 1 人のうちに見つかる

<!-- prose uses[U2].without -->
手元で組んだ版だけで動き、配った版で起きる失敗に気づけない

<!-- prose uses[U2].steps[0].do -->
main の commit に印 v0.1.0-beta.1 を付けて push する

<!-- prose uses[U2].steps[0].see -->
Actions で release の workflow が走り、配布元に先行公開のリリース 0.1.0-beta.1 と 4 つの配布物が並ぶ

<!-- prose uses[U2].steps[1].do -->
サーバの zip を落とし、今のサーバと入れ替えて起動する（docs/release.md の手順）

<!-- prose uses[U2].steps[1].see -->
同じ番号の / で画面が開き、更新の履歴に今の版 0.1.0-beta.1+<commit> の行が足される

<!-- prose walkthroughs[W1].title -->
ベータ版の版番号を渡して組む

<!-- prose walkthroughs[W1].scene -->
出所の版が 0.4.0 の main で、ベータ版 0.4.0-beta.2 の配布物を組む。

<!-- prose walkthroughs[W1].why -->
版番号の元は Cargo.toml の 1 か所だけで、渡せるのはその後ろに -beta.N か -dev.日付 を足した形だけ。ファイルを書き換えないので、配った組み立ての印が .dirty にならない。

<!-- prose walkthroughs[W1].figure.caption -->
渡す版番号と組み立ての結果（出所の版は 0.4.0。値は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
4 つの部品の印・版の記録・更新の履歴がどれも 0.4.0-beta.2 になり、木は書き換わらない

<!-- prose walkthroughs[W1].cases[1].result -->
組み立てが 0 でない終了コードで落ち、出所の版 0.4.0 と渡した 0.5.0-beta.1 が示される

<!-- prose walkthroughs[W1].exceptions[0].text -->
API の契約の版（openapi.json）は渡した版番号でなく出所の版のまま

<!-- prose walkthroughs[W2].title -->
今の携帯のアプリを配布版に入れ替える

<!-- prose walkthroughs[W2].scene -->
本人が、いま携帯に入っている収集アプリ（PC の debug 鍵で組んだもの）を、配布元の正式な鍵の APK に入れ替えようとする。

<!-- prose walkthroughs[W2].why -->
Android は入っているアプリと違う鍵で署名された APK を上書きで入れない。配る APK は資格情報を焼き込まない（公開リポジトリに資格情報を出さないため）。

<!-- prose walkthroughs[W2].figure.caption -->
入れ替えの手順と結果（エミュレータで再現した結果。premises P2・P3・P15）

<!-- prose walkthroughs[W2].cases[0].result -->
未送信の記録が消え、入れた配布版は ST56 まで何も送らない

<!-- prose walkthroughs[W2].cases[1].result -->
失う記録は無く、配布版で送り先を設定して送り始める

<!-- prose walkthroughs[W2].exceptions[0].text -->
ST52 は本人の携帯を切り替えない。いつ移すかは本人の答え（Q1）

<!-- prose walkthroughs[W3].title -->
ベータ版の DB のまま、同じ版の正式版へ移る

<!-- prose walkthroughs[W3].scene -->
本人がベータ版 0.4.0-beta.2 のサーバで使っていた DB のまま、正式版 0.4.0 のサーバに入れ替える。

<!-- prose walkthroughs[W3].why -->
サーバは DB に自分の知らない版があると起動しない（ST48）。ST52 の印の判定は、正式版の印が main の祖先にあることと出所の版が合うことを見るが、同じ版のベータ版の印を祖先に持つことは見ない。

<!-- prose walkthroughs[W3].figure.caption -->
正式版の印を打つ commit と、ベータ版の DB で起動したときの結果（移行 A・B・C は説明用の名前）

<!-- prose walkthroughs[W3].cases[0].result -->
正式版はベータ版の移行を全部知っているので、そのまま起動する

<!-- prose walkthroughs[W3].cases[1].result -->
正式版のサーバは知らない版 C で止まる。次の正式版が C を含むまで正式版へ移れない

<!-- prose walkthroughs[W3].exceptions[0].text -->
説明用の値。実際の版と移行の並びは、まだ印を 1 つも打っていないので確かめていない

<!-- prose walkthroughs[W3].exceptions[1].text -->
ベータ版から正式版へ移るとき DB は戻さず、正式版が追いつくのを待つのは要件（FR-105）どおり

<!-- prose walkthroughs[W4].title -->
印を push すると、その commit の workflow が正式な鍵を読む

<!-- prose walkthroughs[W4].scene -->
印 v* の push で走る release の workflow が、Environment release の正式な鍵で携帯の APK に署名する。印を打つ commit によって何が走るかを見る。

<!-- prose walkthroughs[W4].why -->
鍵の置き場（Environment release）は「印 v* の実行」にだけ開く。main の祖先かの確かめは workflow の中の判定（version.sh plan）にあり、印の作成を止める規則は無い（ルールセットは削除と付け替えだけ）。

<!-- prose walkthroughs[W4].figure.caption -->
印を打つ commit と、正式な鍵に届くか（GitHub の振る舞いは確かめていない）

<!-- prose walkthroughs[W4].cases[0].result -->
review を通った workflow が判定を通し、正式な鍵で署名して公開する

<!-- prose walkthroughs[W4].cases[1].result -->
Environment は印の名前だけを見るので、その commit の workflow に鍵が渡る恐れがある。確かめていない

<!-- prose walkthroughs[W4].exceptions[0].text -->
説明用の表。GitHub がどの commit の workflow を走らせるか、エージェントが印を push できるかは確かめていない

<!-- prose model.caption -->
版番号の出所から、3 つのチャンネルのリリースまで

<!-- prose features[version].summary -->
版番号は根の Cargo.toml だけが持ち、ベータ版・開発版の後ろの部分は組み立ての時に渡す

<!-- prose points[version.1].text -->
サーバ・画面・携帯・PC の 4 つの組み立ての印は、根の Cargo.toml の版（出所の版）から版番号を取る。携帯の組み立ての設定と画面の package.json に版番号を書くと、版の検査が CI を落とす。

<!-- prose points[version.2].text -->
ベータ版（-beta.N）と開発版（-dev.YYYYMMDD）は、ファイルを書き換えずに組み立ての時に版番号を渡す。渡した版番号は、組み立ての印・版の記録・更新の履歴・知らない版で止まったときの self の行に同じ値で入る。

<!-- prose points[version.3].text -->
渡せるのは出所の版と同じ MAJOR.MINOR.PATCH の 3 つの形だけ。出所が 0.4.0 なら 0.5.0-beta.1 や 0.4.0-rc.1 は組み立てごと失敗する。出所の版は 0.x の間 MAJOR が 0 でなければ検査が落ち、1.0 に上げる変更は検査の値も同じ commit で変える。

<!-- prose points[version.4].text -->
携帯と画面の組み立ての印は、根の Cargo.toml も自分のソースに数える。版を変えていなくても、Cargo.toml に commit していない変更があれば携帯と画面の印も .dirty になる。API の契約の版（openapi.json）だけは、渡した版番号でなく出所の版のまま。

<!-- prose features[versioncode].summary -->
Android が新旧を比べる整数を、版番号から決まった式で出す

<!-- prose points[versioncode.1].text -->
versionCode は手で付けず、版番号から計算する。ベータ版 < 同じ版の正式版 < 次の版、の順になる。

<!-- prose points[versioncode.2].text -->
式は配った後に変えない。Android は今より小さい versionCode の上書きを断るので、一度配った付け方より小さくなる付け方には戻せない。MINOR と PATCH は 99 まで（100 以上は組み立てが落ちる）。

<!-- prose points[versioncode.3].text -->
開発版は末尾が 0 なので、同じ出所の版の間は毎日同じ versionCode になる（同じ数の上書きは入る）。開発版は .dev の識別子の別のアプリなので、正式版・ベータ版の数とは比べられない。

<!-- prose features[channels].summary -->
v<版> は正式版、v<版>-beta.N はベータ版、nightly は上書きされる開発版。枝は 1 本（main）

<!-- prose points[channels.1].text -->
印 v0.4.0 を push すると通常のリリース（正式版）、v0.4.0-beta.2 は先行公開（ベータ版）になる。1 つのリリースに、サーバ（Windows 向けと Linux 向け）・携帯の APK・PC の収集アプリの 4 つの配布物が同じ版番号で、決まった名前で並ぶ。

<!-- prose points[channels.2].text -->
公開しないのは、印の形が違う（v0.4.0-rc.1 など）・印の版と出所の版が違う・印の commit が main の祖先でない・配布物が 1 つでも欠ける、のどれか。

<!-- prose points[channels.3].text -->
開発版は、日本時間 0:00 に起き、印 nightly の commit と main の HEAD が違う日だけ組み直して 1 つのリリースを上書きする。変更の無い日は何も組まない。1 回落ちても次の回に同じ変更を拾う。

<!-- prose points[channels.4].text -->
出所の版の正式版（たとえば v0.4.0）を出した後は、出所の版を 0.4.1 などに上げるまで開発版は組まれない（出したばかりの正式版より古い版番号を名乗らないため）。止まったことは予定の実行のログに出るだけ（IMP-04）。

<!-- prose points[channels.5].text -->
配った印 v* は消せず、付け替えられない（配布元のルールセットで拒み、入っていなければ公開しない）。壊れた版を出したら、印を消さずに次の版で直す。開発版の印 nightly は v* の外なので、配った印に数えない。

<!-- prose points[channels.6].text -->
チャンネルごとの枝は持たない。例外は正式版だけに緊急の修正を当てるときで、配った正式版の印から release/<MAJOR>.<MINOR> を切り、PATCH を上げた正式版だけを出す。その枝で移行や封筒の形を変えた印は公開しない。

<!-- prose features[guard].summary -->
配るものは綺麗な木の release で資格情報を持たない。コードを触った PR ごとに同じ手順で組んで確かめる

<!-- prose points[guard.1].text -->
どれかの配布物の印が .dirty か .debug を持つ、または携帯の APK に接続先・資格情報・本人の識別子が焼き込まれていれば公開しない。当たった項目の名前だけを示し、資格情報の値は出さない。

<!-- prose points[guard.2].text -->
配布版の携帯の APK は接続先と資格情報を持たないので、入れても記録を送らない（取得は続け、未送信に積む）。送り先を設定する口は ST56 が作る。

<!-- prose points[guard.3].text -->
コードを触った PR では、配るときと同じ手順で 4 つの配布物を出所の版で組み、検査に当たれば CI を落とす。公開はせず、CI の成果物として置くだけ。文書だけの PR では飛ばす。PR の APK は debug 鍵で署名する。

<!-- prose features[signing].summary -->
正式版とベータ版は正式な鍵、開発版は .dev の識別子と別の鍵。鍵は CI の Environment に置く

<!-- prose points[signing.1].text -->
正式版とベータ版の APK は識別子 dev.ashiato.collector と正式な鍵、開発版は dev.ashiato.collector.dev と開発版の鍵で署名する。識別子と鍵を取り違えると署名しない。

<!-- prose points[signing.2].text -->
署名するのは CI。正式な鍵は印 v* の push の実行にだけ開いた Environment（release）に、開発版の鍵は main の実行にだけ開いた Environment（nightly）に置き、PR の実行には渡さない。鍵が置き場に無ければ署名せず、公開もしない。

<!-- prose points[signing.3].text -->
正式な鍵の写しが GitHub の Environment に置かれる（扉 34 の「手元の USB と拠点外の 2 か所」に置き場が 1 つ増える）。Environment の値は取り出せないので控えにはならない。印を push できる人が main を通さずに鍵を読めるかは確かめていない（IMP-01）。

<!-- prose points[signing.4].text -->
今の携帯に入っているアプリ（PC の debug 鍵で署名）の上に、正式な鍵の APK は上書きで入らない。移るには一度消して入れ直し、そのとき未送信の溜めと設定が消える。ST52 は本人の携帯を切り替えない（推奨: ST56 の後に、送り切ってから入れ替える）。

<!-- prose points[signing.5].text -->
鍵は本人が台本で作る（RSA 4096 bit・有効期限 25 年以上。作った後に読み直して確かめる）。ST52 は台本と手順書（docs/release.md）を用意し、鍵そのものは作らない。

<!-- prose features[webserve].summary -->
配るサーバは画面を実行ファイルに組み込み、同じ番号で / に画面、/api/… に今の API を出す

<!-- prose points[webserve.1].text -->
配るサーバだけを起動すれば、Node を入れずにブラウザで画面が開き、ログインして稼働状況が出る。画面のファイルには今の vite preview と同じ守りの見出し（CSP・no-store）が付く。

<!-- prose points[webserve.2].text -->
/api/<道> は今の /<道> と同じ認可・応答・読み出しの記録になる。収集アプリの送り先（/ingest・/heartbeat など）は今のまま。

<!-- prose points[webserve.3].text -->
画面を組み込むのは配る組み立てだけ。開発と試験の組み立てでは / は 404 で、API は今のまま動く。開発の起動と画面の e2e は今どおり vite preview で画面を開く。配る組み立てで画面が無ければサーバの組み立てが落ちる。

<!-- prose features[tagchecks].summary -->
配った移行の凍結と送信契約の版の検査は、今の木の祖先の印だけを数え、最後の印を版番号で選ぶ

<!-- prose points[tagchecks.1].text -->
配った移行の検査と送信契約の版の検査は、印 v[0-9]* のうち今の木の HEAD の祖先にあるものだけを数える。修正の枝の印は、その枝を merge していない main の検査に入らない。

<!-- prose points[tagchecks.2].text -->
送信契約の版を比べる「最後の配った印」は、打った日時でなく版番号の最も大きい印。同じ版ならベータ版より正式版、ベータ版どうしは番号を数として比べる（beta.10 > beta.9）。

<!-- prose points[tagchecks.3].text -->
版の記録（DB の中の移行の台帳）の「当てた組み立ての版番号」は、組み立ての印と同じ版番号になる。置き場と列の形は変えない。

<!-- prose features[first].summary -->
ST52 は本物の印を打たず、下書きのリリースと使い捨ての鍵で形を確かめる

<!-- prose points[first.1].text -->
ST52 が終わっても、配布元に公開されたリリースは 1 つも無い。確認バッチで見るのは、PR の配布物で作った下書きのリリース（印を作らず、書き手にしか見えない）と、使い捨ての鍵で通した署名の手順。最初の本物の印 v0.1.0-beta.1 と開発版の公開は、許諾の同梱（ST53）の後に本人が始める。Story の完了の判定をこの形に読み替えるのは仮の決定で、答えが推奨のままなら record の段が完了の判定の文を書き直す。

<!-- prose points[first.2].text -->
開発版の予定の実行は、本人が公開の設定（ASHIATO_NIGHTLY_PUBLISH）を入れるまで何も組まず、何も公開せず、印 nightly も動かさない。手で起こした実行は組むが公開しない。

<!-- prose points[first.3].text -->
本人のいまのサーバは、最初の印を打った直後にその版へ移す手順を docs/release.md に置く（ST52 では移さない）。DB が配布版より先に進んでいれば配布版のサーバは知らない版で止まるので、次の印まで移らない。

<!-- prose points[first.4].text -->
PC の収集アプリを新しい版から古い版へ入れ直すことは、ST52 では止めない（入れる操作が無い）。止めるか知らせるかは ST58 が決める。

<!-- prose impact.scenarios[S1].result -->
取得は続けるが送らず、未送信に積む。送り先を設定する口は ST56

<!-- prose impact.scenarios[S1].lost -->
ST56 までの送信

<!-- prose impact.scenarios[S2].result -->
上書きを断られる。消して入れ直すと未送信の溜めと設定が消える

<!-- prose impact.scenarios[S2].lost -->
送り切っていない記録

<!-- prose impact.scenarios[S3].result -->
開発版の作り直しは組まずに終わり、「出所の版を上げる」を予定の実行のログに出す

<!-- prose impact.scenarios[S3].lost -->
出所の版を上げるまで、開発版に main の変更が届かない

<!-- prose impact.scenarios[S4].result -->
同じ名前の印の付け替えはルールセットに拒まれ、v0.2.0-beta.2 で出し直す。公開されなかった v0.2.0-beta.1 も配った印として数えられ、その版の移行が凍結される

<!-- prose impact.scenarios[S4].lost -->
版番号 1 つと、公開されなかった版の移行を直す自由

<!-- prose impact.scenarios[S5].result -->
v0.3.1 は公開されない。main で直して次の版で出す

<!-- prose impact.scenarios[S5].lost -->
正式版の利用者だけに移行を伴う修正を出すこと（main の未公開の変更と一緒に受ける）

<!-- prose impact.scenarios[S6].result -->
正式版のサーバは知らない版 C で止まる（終了コード 3。DB は書き換えない）

<!-- prose impact.scenarios[S6].lost -->
次の正式版が C を含むまで、正式版へ移ること

<!-- prose impact.rules[R1].legit -->
1.0 に上げるときは、出所の版と検査の値を同じ commit で変える必要がある。ほかの正当な組み立てはこの分析では止めていない

<!-- prose impact.rules[R2].legit -->
0.x の間に MINOR を 100 回以上上げる使い方はできない（1.0 に上げる）

<!-- prose impact.rules[R3].legit -->
判定は印を打つ順を見ないので、ベータ版の DB から移れない正式版や、携帯に入らない小さい印も公開できる（IMP-02）。正当な公開を止める経路はこの分析では見つけていない

<!-- prose impact.rules[R4].legit -->
配布版の携帯の APK は接続先を持たないので、入れても ST56 まで記録を送らない

<!-- prose impact.rules[R5].legit -->
今の携帯の debug 鍵のアプリから配布版へ上書きで移れない（消して入れ直す）

<!-- prose impact.rules[R6].legit -->
正式版を出した後、出所の版を上げ忘れると、その間 main に入った変更が開発版に届かない

<!-- prose impact.rules[R7].legit -->
配る形を壊していない PR は止めない。Windows の組み立ての時間が PR の CI に足される

<!-- prose impact.rules[R8].legit -->
間違えた commit に打った印、公開に失敗した印も消せない。次の版番号で出し直す

<!-- prose impact.rules[R9].legit -->
移行を要る緊急の修正は、修正の枝から出せない（main で直して次の版で出す）

<!-- prose impact.rules[R10].legit -->
数える印を絞る変更なので、今より止める範囲は広がらない。印が 0 個の今は何も凍結しない

<!-- prose impact.rules[R11].legit -->
ベータ版の DB のまま、最後のベータ版の移行を持たない正式版へ移れない

<!-- prose impact.rules[R12].legit -->
開発の組み立てで / を開いても画面は出ない（開発と e2e は vite preview の番号で開く）

<!-- prose impact.findings[IMP-01].title -->
印を push できる人が、main を通さずに正式な鍵を読めるかもしれない

<!-- prose impact.findings[IMP-01].text -->
正式な鍵の置き場（Environment release）は印の名前 v* だけを見て開く。main の祖先かの確かめは workflow の中の判定にあり、印の作成を止める規則は無い（ルールセットは削除と付け替えだけ、pre-push は feat/st* の枝だけ）。印の push で走る workflow が印の commit の定義なら、main に入っていない commit で書き換えた workflow に v* の印を打つと、その workflow が正式な鍵を読める。鍵が外に出ると、配った APK の更新を他人が作れる。D21（仮）が main に開かない理由（AI が書いた変更から鍵を読ませない）と同じ経路が、印の側に残る。 本人が鍵を置く手順（docs/release.md。D19）は Environment と Secrets を置く工程をルールセットより前に並べ、そのルールセットは印の作成に効かない。「ルールセットが無ければ公開しない」の判定も、書き換えられる同じ workflow の中にある。

<!-- prose impact.findings[IMP-02].title -->
印を打つ順を判定が見ないので、ベータ版の DB から同じ版の正式版へ移れないことがある

<!-- prose impact.findings[IMP-02].text -->
公開の判定は、正式版の印が main の祖先にあることと出所の版が合うことを見るが、同じ版のベータ版の印を祖先に持つことは見ない。正式版の印を最後のベータ版より前の commit に打つと、ベータ版で当てた移行を正式版が知らず、ベータ版の DB で正式版のサーバは止まる（ST48。DB は書き換えない）。 同じ原因（印を打つ順を判定が見ない）で、すでに v0.4.0 や v0.4.0-beta.3 がある版に後から v0.4.0-beta.2 を打つこともでき、印は消せないので、versionCode の小さい（携帯に入らない）版が公開されて残る。

<!-- prose impact.findings[IMP-03].title -->
公開に失敗した印も「配った印」に数えられ、その版の移行が凍結される

<!-- prose impact.findings[IMP-03].text -->
配った移行の凍結と送信契約の版の検査は、リリースがあるかでなく印 v* があるかで数える。印は消せないので、鍵が無い・組み立てが落ちたなどで公開されなかった印も、その版の移行と封筒の形を凍結する。

<!-- prose impact.findings[IMP-04].title -->
開発版が止まったことは、予定の実行のログにしか出ない

<!-- prose impact.findings[IMP-04].text -->
正式版を出した後に出所の版を上げ忘れると、開発版は組まずに終わり、理由は予定の実行のログに出るだけ。job は失敗にならないので、本人に知らせは届かない。

<!-- prose impact.findings[IMP-05].title -->
画面の版を package.json から読む 2 か所が、tasks の直す先に無い

<!-- prose impact.findings[IMP-05].text -->
tasks 3.2 は web/package.json の version を消すが、画面の単体試験の設定（web/vitest.config.ts）と e2e（web/e2e/st49-client-version.spec.ts）も pkg.version を読む。直さないと、試験の画面の印が undefined で始まり、e2e が落ちる。 この 2 か所は ST52 自身の PR の CI（vitest・画面の e2e）を落とす。

<!-- prose impact.findings[IMP-06].title -->
開発版の印 nightly を動かすと、手で git fetch --tags した clone が rc=1 になる

<!-- prose impact.findings[IMP-06].text -->
開発版の作り直しは印 nightly を付け替える。それを取った clone で git fetch --tags は rc=1 になる（git fetch は rc=0 で手元の印は古いまま）。ハーネスの台本は --tags を使っていないが、本人が手で取る場面は扱っていない。

<!-- prose impact.scope.covers[0].text -->
印から 3 チャンネルのリリースを作る仕組みと、PR ごとの配る形の組み立て。ST52 の間は下書きのリリースと使い捨ての鍵で確かめる範囲まで

<!-- prose impact.scope.covers[1].text -->
版番号の出所の 1 本化と、版の記録・組み立ての印・配った印の検査への写し

<!-- prose impact.scope.covers[2].text -->
配るサーバが同じ番号で画面を配ること（Linux の組み立てで e2e が見る）

<!-- prose impact.scope.preconditions[0].text -->
渡せる版番号は出所の版・-beta.N（N は 1〜98）・-dev.YYYYMMDD の 3 つの形だけ。出所の版の MAJOR は 0 の間だけ

<!-- prose impact.scope.preconditions[1].text -->
公開するのは main の祖先の印だけ。例外は緊急の修正の枝の正式版の印で、移行と封筒の形が元の印と同じときに限る

<!-- prose impact.scope.preconditions[2].text -->
開発版を公開するのは、本人が公開の設定を入れた後だけ。正式な鍵は印 v* の実行だけ、開発版の鍵は main の実行だけに渡す

<!-- prose impact.scope.preconditions[3].text -->
配る組み立ては綺麗な木の release で、携帯の APK は接続先と資格情報を持たないときだけ公開する。印の規則が配布元に入っていないと公開しない

<!-- prose impact.scope.preconditions[4].text -->
配った印の検査が数えるのは、今の木の HEAD の祖先にある v[0-9]* の印だけ

<!-- prose impact.scope.preconditions[5].text -->
versionCode の式が成り立つのは MINOR と PATCH が 99 以下のときだけ

<!-- prose impact.scope.preconditions[6].text -->
確かめの組み立ては下書きのリリースにだけ置く（印を作らず、公開しない）

<!-- prose impact.scope.preconditions[7].text -->
PR の配る形の組み立てはコードを触った PR だけで走り、文書だけ（docs/・openspec/・*.md）の PR では飛ばす

<!-- prose impact.scope.preconditions[8].text -->
版の記録は当て終えた版だけを残し、S-01 の実行時の役割には読むことだけを許す

<!-- prose impact.scope.preconditions[9].text -->
組み立ての印の版番号は、版番号を渡したときだけ渡した値になる。.dirty を見るのはその部品のソース（携帯と画面は根の Cargo.toml を含む）だけ

<!-- prose impact.scope.excludes[0].text -->
最初の本物の印と開発版の公開（ST53 の後）・本人の携帯とサーバの切り替え

<!-- prose impact.scope.excludes[1].text -->
Windows でサーバが DB に繋いで画面を配るところ（ST55）。ST52 の Windows は組み立てと起動の入口まで

<!-- prose impact.scope.excludes[2].text -->
開発版の DB の扱い（ST54）・PC の収集アプリの後戻り（ST58）

<!-- prose impact.scope.residual[0].text -->
印を push できる人が正式な鍵に届くかは確かめていない（IMP-01）

<!-- prose impact.scope.residual[1].text -->
ベータ版の DB から、最後のベータ版の移行を持たない正式版へ移れない（IMP-02）

<!-- prose impact.scope.residual[2].text -->
Environment が同じリポジトリの枝からの PR に値を渡さないこと（P12）と、下書きが印を作らないこと（P16）は公式文書だけ（確かさ 2）。tasks で走らせて確かめる

<!-- prose impact.scope.residual[3].text -->
開発版の毎日同じ versionCode の上書きが、adb 以外の入れ方で通るかは確かめていない

