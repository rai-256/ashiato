# CH26 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
毎晩赤だった脆弱性の夜間の検査を緑に戻し、次に新しい勧告が出たら赤で気づけるようにする。挙がった 4 件は、上げられるものは上げ、上げられないものは理由と 90 日の期限を付けて例外にする。配布物の PostgreSQL は 17.12 にし、今まで使っていた DB はそのまま使える。

<!-- prose impact.overview -->
夜間の脆弱性検査が緑に戻り、次の勧告に気づけるようになる。配布物の PostgreSQL は 17.12 になり、今の DB はこの分析と台本の範囲ではそのまま使える。代わりに、libsqlite3-sys と chrono の 2 件は 90 日のあいだ検査で知らされなくなる。Windows の配布物どうしの更新は機械では確かめず、本人の PC で見る。

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

<!-- prose walkthroughs[W1].title -->
17.6 の配布版が入った PC を、17.12 の配布物で更新する

<!-- prose walkthroughs[W1].scene -->
本人の PC に PostgreSQL 17.6 を同梱した配布版のサーバが入っている。この change の後の配布物（17.12 を同梱）で更新する。

<!-- prose walkthroughs[W1].why -->
PostgreSQL は同じ大きい版の中では、データの形を変えない。インストーラが読むのも大きい版（17）だけなので、置き場の名前も変わらない。

<!-- prose walkthroughs[W1].figure.caption -->
更新で起きること（版の番号は lock の値。記録の件数は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
17.12 が同じ置き場で起き、今までの記録が画面に見える

<!-- prose walkthroughs[W1].cases[1].result -->
インストーラが 17.6 のプログラムに戻し、17.12 で取った退避を 17.6 で戻す。台本はこの戻しが通ることを確かめる

<!-- prose walkthroughs[W1].cases[2].result -->
更新の手順に入らない。確認バッチでは「入っていない」「同じ版」と答える

<!-- prose walkthroughs[W1].exceptions[0].text -->
Windows の配布物どうしの更新はこの PR では走らせない。機械が確かめるのは Linux の PostgreSQL の部分だけ（IMP-02）

<!-- prose walkthroughs[W1].exceptions[1].text -->
記録の件数は説明用の値。実際の本人の DB の中身は確かめていない

<!-- prose model.caption -->
4 件の片付け方

<!-- prose sections.makes.tldr -->
夜間の脆弱性検査を緑に戻し、同梱の PostgreSQL を 17.12 にする。製品の振る舞いは変えない。

<!-- prose sections.change.tldr -->
夜間の検査が緑に戻り、配布物の PostgreSQL が 17.6 から 17.12 になる。

<!-- prose sections.lost.tldr -->
例外にした 2 件は 90 日のあいだ夜間の検査で知らされなくなる。

<!-- prose sections.uses.tldr -->
次に検査が赤になったら、それは新しい勧告か例外の期限切れだと分かる。

<!-- prose sections.watch.tldr -->
Windows の配布物どうしの更新は機械では確かめず、本人の PC で見る。

<!-- prose sections.findings.tldr -->
重大な問題は見つけていない。中くらいの 2 件（例外の理由の確かさ・Windows の更新）を残す。

<!-- prose sections.rest.tldr -->
chrono の置き換えと rusqlite 0.41 への上げはしない。issue は本人が閉じる。

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
Windows の配布物どうし（17.6-1 の zip から 17.12-1 の zip）の更新と戻しは、この PR では走らせない。本人の PC に配布版が入っていれば、確認バッチで更新して記録が見えるかを答える。

<!-- prose features[close].summary -->
merge の後、夜間の検査が緑なのを見て本人が閉じる

<!-- prose features[close].tldr -->
merge の後に夜間の検査が緑で走ったら、本人が #334〜#337 を閉じる。

<!-- prose points[close.1].text -->
merge の後、夜間の検査が緑で走ったのを見て、本人が #334〜#337 を閉じる。確認バッチで「緑で走り、閉じたか」を聞く。

<!-- prose points[close.2].text -->
この PR が merge されても #334〜#337 は開いたまま残る。夜間の検査は、勧告が消えても issue を自分では閉じない。

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

<!-- prose impact.scope.covers[0].text -->
2026-11-19 の夜に挙がった crate の勧告 3 件と、同梱の PostgreSQL の 17.6-1 から 17.12-1 への小さい版の更新

<!-- prose impact.scope.preconditions[0].text -->
上げるのは直った版が Cargo.toml の依存の範囲の中にあるときだけで、そのとき動かすのは Cargo.lock のその crate だけ。範囲の外か直った版が無いときは例外の行にする

<!-- prose impact.scope.preconditions[1].text -->
置き場をそのまま使えるのは、同じ大きい版（17）の中で小さい版だけを替えるとき

<!-- prose impact.scope.excludes[0].text -->
大きい版の更新（18 など）。Windows の配布物どうしの更新を機械で走らせること

<!-- prose impact.scope.residual[0].text -->
libsqlite3-sys の勧告は PC の収集アプリに残る（IMP-01）。chrono は 90 日ごとに赤に戻る（IMP-03）

