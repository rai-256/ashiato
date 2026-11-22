# CH26 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
毎晩赤だった脆弱性の夜間の検査を緑に戻し、次に新しい勧告が出たら赤で気づけるようにする。挙がった 4 件は、上げられるものは上げ、上げられないものは理由と 90 日の期限を付けて例外にする。配布物の PostgreSQL は 17.12 にし、今まで使っていた DB はそのまま使える。PostgreSQL の版を上げる PR では、前の版の配布物から新しい版への更新と戻しを CI が Windows で通すので、本人が手で確かめなくてよい。

<!-- prose impact.overview -->
夜間の脆弱性検査が緑に戻り、次の勧告に気づけるようになる。配布物の PostgreSQL は 17.12 になり、今の DB をそのまま使うことを台本と CI で確かめる（まだ走らせていない）。lock を変える PR では、CI が前の版の配布物から今の版への更新と戻しを Windows で通す。代わりに、libsqlite3-sys と chrono の 2 件は 90 日のあいだ検査で知らされなくなり、lock を変えた PR で前の lock が取れない・大きい版が違うと CI が落ちる。

<!-- prose impact.problem -->
夜間の検査が 11-19 から毎晩赤で、立った 4 件の issue が手付かずのまま。新しい勧告が出ても赤に埋もれて気づけない。

<!-- prose uses[U1].title -->
夜間の検査の赤を、新しい勧告として読む

<!-- prose uses[U1].trigger -->
ある朝、夜間の脆弱性検査が赤で、issue が 1 件立っている。今は 11-19 から毎晩赤なので、新しいものかどうか見分けられない。

<!-- prose uses[U1].outcome -->
赤は「新しく見つかった」か「例外の期限が来た」のどちらかだと分かり、そのときに上げるか延ばすかを決められる

<!-- prose uses[U1].without -->
毎晩赤のままで、新しい勧告が出ても今の 4 件に埋もれて気づけない

<!-- prose uses[U1].steps[0].do -->
merge の後の夜間の検査の結果を見る

<!-- prose uses[U1].steps[0].see -->
緑。#334〜#337 の 4 件はもう挙がらない

<!-- prose uses[U1].steps[1].do -->
#334〜#337 を閉じる

<!-- prose uses[U1].steps[1].see -->
開いた脆弱性の issue が無くなる

<!-- prose uses[U1].steps[2].do -->
後日、検査が赤になったら issue を開く

<!-- prose uses[U1].steps[2].see -->
新しい勧告の ID か、例外の行の「期限切れ」

<!-- prose uses[U2].title -->
次に PostgreSQL の小さい版を上げる PR で、手を動かさずに更新を確かめる

<!-- prose uses[U2].trigger -->
EDB が 17.13 を出し、遅れの検査が赤になった。pgsql.lock を 17.13-1 に上げる PR を作る。前は前の版の zip を手で組んで更新と戻しを通す手順だった。

<!-- prose uses[U2].outcome -->
前の版の配布物から新しい版への更新と戻しが Windows で通ったと、PR の CI だけで分かる

<!-- prose uses[U2].without -->
前の版の zip を手で組み、Windows で更新と戻しを通して結果を PR に残す手間がかかる（docs/release.md の今の手順）

<!-- prose uses[U2].steps[0].do -->
pgsql.lock を 17.13-1 にした PR を出す

<!-- prose uses[U2].steps[0].see -->
CI の package-windows が古い側の試験の zip を分岐点の lock（17.12-1）で組む

<!-- prose uses[U2].steps[1].do -->
package-windows の出力を見る

<!-- prose uses[U2].steps[1].see -->
「古い側 17.12-1 → 新しい側 17.13-1」

<!-- prose uses[U2].steps[2].do -->
install-windows の結果を見る

<!-- prose uses[U2].steps[2].see -->
緑。更新と、移行が落ちたときの戻しの Scenario 群が通った

<!-- prose walkthroughs[W1].title -->
この PR の CI が、17.6 の配布物を 17.12 の配布物で更新する

<!-- prose walkthroughs[W1].scene -->
この change の PR（pgsql.lock を 17.6-1 から 17.12-1 に変える）の CI が、17.6 を同梱した配布物を入れた Windows を 17.12 の配布物で更新する。

<!-- prose walkthroughs[W1].why -->
PostgreSQL は同じ大きい版の中では、データの形を変えない。インストーラが読むのも大きい版（17）だけなので、置き場の名前も変わらない。

<!-- prose walkthroughs[W1].figure.caption -->
CI の Windows で起きること（版の番号は lock の値。記録の件数は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
17.12 が同じ置き場で起き、update-apply-install.sh の Scenario 群が通る

<!-- prose walkthroughs[W1].cases[1].result -->
インストーラが 17.6 のプログラムに戻し、17.12 で取った退避を 17.6 で戻す。CI の戻しの Scenario がこれを通す

<!-- prose walkthroughs[W1].cases[2].result -->
17.12 → 17.12 の更新になり、小さい版の違いを確かめない。この PR は version を変えるので、版の確かめがこれを見つけて CI を落とす（version を変えない lock の PR では見つけない）

<!-- prose walkthroughs[W1].exceptions[0].text -->
CI は更新の後に動いている PostgreSQL の版は見ない。入れ替え損ねれば、直したはずの脆弱性がその PC に残っても気づけない（D10（仮）・IMP-05）

<!-- prose walkthroughs[W1].exceptions[1].text -->
本人の PC に入っている配布版の移行はこの change で扱わない（ST64）

<!-- prose walkthroughs[W1].exceptions[2].text -->
記録の件数は説明用の値。CI の試験の記録の中身はここでは確かめていない

<!-- prose walkthroughs[W2].title -->
PR ごとに、CI が古い側の試験の zip をどの lock で組むか

<!-- prose walkthroughs[W2].scene -->
いろいろな PR が CI を走らせる。CI は PR の差分を見て、更新の試験の古い側をどの lock で組むかを決める。

<!-- prose walkthroughs[W2].why -->
変えたかを PR 自身の差分で決めるので、base の側だけが進んだ古い枝が版を下げる向きの更新を走らせない。取れないときに今の lock へ倒すと 17.12 → 17.12 で緑になるので、落とす。

<!-- prose walkthroughs[W2].figure.caption -->
PR の差分ごとの組み方（版の番号は説明用。17.6-1・17.12-1 はこの change の値）

<!-- prose walkthroughs[W2].cases[0].result -->
前の版から今の版への更新と戻しが Windows で走る

<!-- prose walkthroughs[W2].cases[1].result -->
今のまま組み、前の lock を取らない

<!-- prose walkthroughs[W2].cases[2].result -->
CI が落ち、PR は緑にならない。大きい版を上げる道は大きい版を上げる Story が作る

<!-- prose walkthroughs[W2].exceptions[0].text -->
開発版・互換の組（dev-old・compat-old）は前の lock に替えない

<!-- prose model.caption -->
4 件の片付け方

<!-- prose sections.makes.tldr -->
夜間の脆弱性検査を緑に戻し、同梱の PostgreSQL を 17.12 にする。製品の振る舞いは変えない。

<!-- prose sections.change.tldr -->
検査が緑に戻り、PostgreSQL が 17.12 になり、lock を変える PR の CI が更新を確かめる。

<!-- prose sections.lost.tldr -->
例外にした 2 件は 90 日のあいだ夜間の検査で知らされなくなる。

<!-- prose sections.uses.tldr -->
次に検査が赤になったら、まず新しい勧告か例外の期限切れを疑える。

<!-- prose sections.watch.tldr -->
CI は 2 つの zip の版を見るが、更新の後に動いている版は見ない。

<!-- prose sections.findings.tldr -->
重大な問題は見つけていない。中くらいの 2 件（例外の理由の確かさ・動いている版を見ない）を残す。

<!-- prose sections.rest.tldr -->
chrono と rusqlite の上げ・本人の PC の移行はしない。issue は本人が閉じる。

<!-- prose features[crate].summary -->
rustix は上げ、libsqlite3-sys と chrono は例外へ

<!-- prose features[crate].tldr -->
rustix だけを上げ、上げられない 2 件は理由と 90 日の期限で例外の表に書く。

<!-- prose points[crate.1].text -->
rustix は Cargo.lock だけで 1.1.6 以上に上げる。crate を名指しして上げ、ほかの依存の版は 1 つも動かさない。

<!-- prose points[crate.2].text -->
libsqlite3-sys の勧告は、PC の収集アプリの使い方に形は当たる（ブラウザの履歴を -wal ごと写して開く）。それでも上げずに「届かない:」で例外にする。理由は「履歴を書ける処理は、収集アプリの記憶をもともと読める」という読みで、この読みは確かめていない（確かさ 1）。

<!-- prose points[crate.3].text -->
chrono は直る版が無いので、例外の行は 90 日で切れて夜間の検査がまた赤になる。置き換える（jiff か time）まで、90 日ごとに本人の前に戻る。

<!-- prose points[crate.4].text -->
例外の表の今ある行は中身ごと残す。足すのは新しい行だけ。

<!-- prose features[pglock].summary -->
lock の版・URL・指紋を 17.12-1 にし、大きい版は 17 のまま

<!-- prose features[pglock].tldr -->
配布物の PostgreSQL を 17.12-1 に替える。大きい版・実行ファイル・置き場は変えない。

<!-- prose points[pglock.1].text -->
pgsql.lock の version・url・sha256 を 17.12-1 にそろえる。sha256 は実装のときに落とした zip から取る。

<!-- prose points[pglock.2].text -->
大きい版（17）・実行ファイルの一覧・許諾の置き場は変えない。インストーラが読むのは大きい版だけなので、DB の置き場 db\17 も変わらない。

<!-- prose points[pglock.3].text -->
同梱する DLL の一覧は 17.6 の zip で調べたもの。17.12 の zip で調べ直し、違えばそろえる。VC の実行時の DLL の取り込みが増えていたら、組み立ての台本の変更が要るので止めて昇格する。

<!-- prose features[pgswap].summary -->
17.6→17.12→17.6 の入れ替えと、退避を戻すことを Linux で走らせて確かめる

<!-- prose features[pgswap].tldr -->
17.6 で使っていた DB を 17.12 がそのまま使い、17.6 に戻しても使えることを機械で確かめる。

<!-- prose points[pgswap.1].text -->
17.6 で置き場を作ってサーバの移行を当て、行を入れる。同じ置き場を 17.12 で起こすと中身が同じで、移行をもう一度当てても足すものが無い。

<!-- prose points[pgswap.2].text -->
更新が落ちたとき、インストーラは 17.12 で取った退避を 17.6 の pg_restore で戻す。この戻しの道が通ることも、同じ台本で確かめる。17.12 で足した行は 17.6 に戻しても残る。

<!-- prose points[pgswap.3].text -->
Windows の配布物どうし（17.6-1 の zip から 17.12-1 の zip）の更新と、移行が落ちたときの戻しは、この PR の CI が Windows で通す（下の「lock を変える PR の CI」）。本人の PC では確かめない。本人の PC に入っている配布版の移行は ST64 が扱う。

<!-- prose features[ci].summary -->
lock を変えた PR だけ、古い側の試験の zip を前の lock で組む

<!-- prose features[ci].tldr -->
pgsql.lock を変えた PR だけ、CI が前の版の zip から今の版の zip への更新と戻しを Windows で通す。

<!-- prose points[ci.1].text -->
pgsql.lock を変えた PR では、更新の試験の古い側の zip（package-windows-test-old）だけを PR が変える前の lock で組み、新しい側とほかの試験の zip は PR の lock で組む。install-windows は今の台本（update-apply-install.sh）で、古い側を入れた上での更新と、移行が落ちたときの戻しを走らせる。

<!-- prose points[ci.2].text -->
変えたかは、PR の分岐点から head までの差分で決める。base の側だけが lock を上げた後の古い枝は「変えていない」になり、今のまま組む（小さい版を下げる向きの更新は走らせない）。lock を変えない PR・main への push・手動の run も今のまま。

<!-- prose points[ci.3].text -->
lock を変えた PR で、分岐点の lock が取れない・大きい版が違う・中身が同じ（改行だけを変えた）・組んだ zip の同梱の版が前と今の lock の版でない、のどれかなら、CI を緑にせず落とす。今の lock に倒すと 17.12 → 17.12 の更新で緑になり、確かめたつもりで何も確かめないため。大きい版を上げる PR（17 → 18）もここで落ちる。

<!-- prose points[ci.4].text -->
判定と版の確かめは台本 tools/release/pgsql-prev-lock.sh に置き、ci.yml は呼ぶだけにする。版は組んだ zip の許諾の文（THIRD-PARTY-NOTICES.txt）から読み、保存の前と、前の組み立てを使い回した run の両方で確かめる。出力に「古い側 17.6-1 → 新しい側 17.12-1」の形で版の組を出す。台本の自己試験と ci.yml の形の確かめは Linux の CI（chain）で毎回走る。

<!-- prose points[ci.5].text -->
CI は、更新した後・戻した後に動いている PostgreSQL の版は見ない。インストーラがプログラムの置き場ごと入れ替える前提で、2 つの zip の同梱の版だけを確かめる。もしインストーラが pgsql の置き場だけを入れ替え損ねても、CI は緑になりうる（IMP-05）。

<!-- prose points[ci.6].text -->
Windows の組み立ての使い回しの鍵に、古い側を組んだ前の lock の指紋（取らない run では none）を足す。前の lock だけが違う組み立てを誤って使い回さない。正典の鍵の Requirement を書き換える。

<!-- prose points[ci.7].text -->
lock を変える PR の CI は PostgreSQL の zip を 2 つ（約 370 MB ずつ）落とす。lock を変えない PR は今のまま。docs/release.md の小節は、手で通す手順から CI の結果の見方に書き換える。

<!-- prose features[close].summary -->
merge の後、夜間の検査が緑なのを見て本人が閉じる

<!-- prose features[close].tldr -->
merge の後に夜間の検査が緑で走ったら、本人が #334〜#337 を閉じる。

<!-- prose points[close.1].text -->
merge の後、夜間の検査が緑で走ったのを見て、本人が #334〜#337 を閉じる。確認バッチで「緑で走り、閉じたか」を聞く。

<!-- prose points[close.2].text -->
この PR が merge されても #334〜#337 は開いたまま残る。夜間の検査は、勧告が消えても issue を自分では閉じない。

<!-- prose impact.scenarios[S1].result -->
package-windows が古い側を組まずに落ち、PR の CI は緑にならない

<!-- prose impact.scenarios[S1].lost -->
大きい版を上げる PR を、今の CI のまま緑にすること

<!-- prose impact.rules[R1].legit -->
大きい版を上げる PR（17 → 18）は、大きい版を上げる Story が道を作るまで CI が緑にならない。改行だけを lock に入れた PR・分岐点を浅く fetch できない run（ネットワークの失敗など）も落ちる

<!-- prose impact.rules[R2].legit -->
この分析では、正当な PR を止める道を見つけていない。鍵に前の lock の指紋を足すと、lock を変えた PR は使い回せずに組み直す（約 25 分）だけ

<!-- prose impact.findings[IMP-01].title -->
libsqlite3-sys の勧告を外す理由は、確かめていない読みに乗っている

<!-- prose impact.findings[IMP-01].text -->
PC の収集アプリは勧告の当たる形（他の処理が書いた DB を -wal ごと開く）で履歴を読む。外す理由は「書ける処理は収集アプリの記憶をもともと読める」で、Firefox の profile が本人以外も書ける置き場にあれば成り立たない

<!-- prose impact.findings[IMP-02].title -->
Windows の配布物どうしの 17.6→17.12 の更新は、機械では確かめない

<!-- prose impact.findings[IMP-02].text -->
台本が確かめるのは Linux の PostgreSQL の置き場の入れ替えと、退避を戻すことだけ。インストーラが Windows でプログラムの置き場を入れ替える道は、本人の PC に配布版が入っているときの確認バッチでしか通らない

<!-- prose impact.findings[IMP-03].title -->
chrono の例外は 90 日ごとに夜間の検査を赤にする

<!-- prose impact.findings[IMP-03].text -->
chrono は直る版が無いので、置き換えるまで例外の期限が来るたびに検査が赤になり、issue が立つ

<!-- prose impact.findings[IMP-04].title -->
merge しても issue は自動では閉じない

<!-- prose impact.findings[IMP-04].text -->
夜間の検査は勧告が消えても issue を閉じない。#334〜#337 は本人が閉じるまで開いたまま残る

<!-- prose impact.findings[IMP-05].title -->
CI は、更新の後に動いている PostgreSQL の版を見ない

<!-- prose impact.findings[IMP-05].text -->
CI は 2 つの zip の同梱の版を確かめ、更新と戻しの Scenario 群を通すが、更新した後に起きている postgres.exe の版は読まない。インストーラが pgsql の置き場だけを入れ替え損ねても CI は緑になりうる

<!-- prose impact.findings[IMP-06].title -->
大きい版を上げる PR は、今の CI では緑にならない

<!-- prose impact.findings[IMP-06].text -->
lock の大きい版を変えた PR は、前の lock で古い側を組めず CI が落ちる。大きい版を上げる Story が道を作るまで、その PR は通らない

<!-- prose impact.findings[IMP-07].title -->
この上流の枝は main より古く、依存の表の元が CH24 の後の形でない

<!-- prose impact.findings[IMP-07].text -->
Cargo.lock と osv-scanner.toml は main（CH24 merge 済み）と違う。実装をこの枝の形の上で始めると、CH24 の変更を戻すか、元の版との比べが食い違う

<!-- prose impact.scope.covers[0].text -->
2026-11-19 の夜に挙がった crate の勧告 3 件と、同梱の PostgreSQL の 17.6-1 から 17.12-1 への小さい版の更新

<!-- prose impact.scope.covers[1].text -->
pgsql.lock を変える PR の CI で、古い側の試験の zip だけを分岐点の lock で組み、Windows で更新と戻しを通すこと

<!-- prose impact.scope.preconditions[0].text -->
上げるのは直った版が Cargo.toml の依存の範囲の中にあるときだけで、そのとき動かすのは Cargo.lock のその crate だけ。範囲の外か直った版が無いときは例外の行にする

<!-- prose impact.scope.preconditions[1].text -->
置き場をそのまま使えるのは、同じ大きい版（17）の中で小さい版だけを替えるとき

<!-- prose impact.scope.preconditions[2].text -->
CI が前の lock で古い側を組むのは、PR の run で、分岐点から head までの差分が lock を変え、大きい版が同じときだけ。lock を変えない PR・PR でない run は今のまま

<!-- prose impact.scope.preconditions[3].text -->
前の組み立てを使い回すのは、PR の run で鍵が完全に一致したときだけ。鍵に入れないのは名指しした試験の台本と文書だけで、前の lock の指紋は lock を変えた run だけ（ほかは none）

<!-- prose impact.scope.excludes[0].text -->
大きい版の更新（18 など。lock を変える PR の CI は落ちる）。本人の PC に入っている配布版の移行（ST64）。開発版・互換の組の古い側を前の lock で組むこと

<!-- prose impact.scope.residual[0].text -->
libsqlite3-sys の勧告は PC の収集アプリに残る（IMP-01）。chrono は 90 日ごとに赤に戻る（IMP-03）。CI は動いている PostgreSQL の版を見ない（IMP-05）

