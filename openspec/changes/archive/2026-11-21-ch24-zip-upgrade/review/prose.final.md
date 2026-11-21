# CH24 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
zip を読み書きする部品を、勧告の直った版（9 系）に上げる。例外の表に「修正版待ち」という事実と違う理由で残っていた zip の 7 行が消える。書き出し・Takeout の取り込み・PC の更新の展開の振る舞いは前と同じで、前の版で書いた書き出しも、上げた後の版で同じに読めることを試験で固定する。

<!-- prose impact.overview -->
zip の部品を 2.6.1 から 9 に上げ、例外の表から zip の 7 行を消し、許す許諾の一覧に bzip2-1.0.6 を足す。書き出し・Takeout の取り込み・PC の更新の展開は、9 で変わった読み方と判定を自前で 2.6.1 に揃えるので、試験で固定した範囲では前と同じに振る舞う。できなくなることとして見つけたのは、Linux でだけ走る権限の写しで特別な bit が落ちることと、CRC の合わない名前の欄を持つ書庫が `broken_zip` でなく読めるようになることの 2 つ。

<!-- prose impact.problem -->
例外の表が zip 2.6.1 の重大の勧告を「上げる予定: 互換範囲外の修正版待ち」で外している。直った版はもう出ており、理由が事実と違う。上げると zip を使う所の読み方が変わりうるので、上げても前の書き出しが読め、取り込みと更新の展開が同じに振る舞うことを先に固定したい。

<!-- prose uses[U1].title -->
脆弱性の警告を見直す

<!-- prose uses[U1].trigger -->
GitHub の依存の警告が 15 件あり、うち 9 件が zip。例外の表を見ると、理由は「修正版待ち」だが直った版はもう出ている。

<!-- prose uses[U1].outcome -->
例外の表に事実と違う理由の行が残らない。残る例外は、配る物に入らない部品など理由の通るものだけになる

<!-- prose uses[U1].without -->
重大の勧告が「修正版待ち」という事実と違う理由で外されたまま、期限（2027-02-06）まで残る

<!-- prose uses[U1].steps[0].do -->
この change の PR で、PR の脆弱性の検査の出力を見る

<!-- prose uses[U1].steps[0].see -->
「新しく持ち込んだ: 0 件」と、表で足した・変えた行が「なし」

<!-- prose uses[U1].steps[1].do -->
例外の表（osv-scanner.toml）を開く

<!-- prose uses[U1].steps[1].see -->
zip の 7 行が無く、残る行は zip と無関係なものだけ

<!-- prose uses[U2].title -->
上げた後に前の書き出しを扱う

<!-- prose uses[U2].trigger -->
上げた版のサーバに入れ替えた。置き場には上げる前に書いた書き出しの zip が残っている。

<!-- prose uses[U2].outcome -->
上げても、前に書いた書き出しの扱いが変わらないと分かる

<!-- prose uses[U2].without -->
試験は書くのも読むのも同じ版で、版をまたいで読めるかは確かめられていない

<!-- prose uses[U2].steps[0].do -->
書き出しをもう 1 回走らせる

<!-- prose uses[U2].steps[0].see -->
書き終えると前の書き出しのファイルが消え、新しい 1 つだけが残る（今と同じ）

<!-- prose uses[U2].steps[1].do -->
手元に残した前の書き出しを、上げた後の版の試験で読む（試験に埋め込んだ例）

<!-- prose uses[U2].steps[1].see -->
合言葉で全部の項目が同じ中身で読める

<!-- prose walkthroughs[W1].title -->
Takeout の項目の名前が、上げた後も同じ文字列になる

<!-- prose walkthroughs[W1].scene -->
本人が上げた後のサーバで Takeout の書庫を取り込む。書庫には UTF-8 の印のある名前と、印の無い日本語の名前がありうる。

<!-- prose walkthroughs[W1].why -->
zip 9 は名前を UTF-8 の印でなく、バイト列が UTF-8 として正しいかで読む。名前は記録と台帳に残るので、2.6.1 の規則を open.rs に置いて読み方を変えない。

<!-- prose walkthroughs[W1].figure.caption -->
名前の読み方（名前は説明用の値）

<!-- prose walkthroughs[W1].cases[0].result -->
どの版でも UTF-8 で読み、取り込み済みの記録の名前と揃う

<!-- prose walkthroughs[W1].cases[1].result -->
名前が 2.6.1 と違う文字列になり、上げる前に取り込んだ記録の inner_path と揃わない。この change は、これを避けるために 2.6.1 の規則を残す

<!-- prose walkthroughs[W1].cases[2].result -->
2.6.1 と同じ CP437 の文字列。取り込み済みの記録と揃う

<!-- prose walkthroughs[W1].exceptions[0].text -->
説明用の値。実物の Takeout の書庫が印を持つかは確かめていない

<!-- prose walkthroughs[W2].title -->
PC の更新の展開で、断る名前が変わらない

<!-- prose walkthroughs[W2].scene -->
PC の収集アプリが指紋を照合した更新の配布物を展開する。配る zip の名前は組み立ての台本が決めた ASCII の固定のもの。

<!-- prose walkthroughs[W2].why -->
zip 9 の `enclosed_name` は絶対パスや前置きを相対に直し、Windows の装置の名前を断る。振る舞いを変えない約束なので、2.6.1 と同じ確かめを自前の関数で当てる。

<!-- prose walkthroughs[W2].figure.caption -->
項目の名前ごとの結果（試験 1.4 が並べる名前の一部）

<!-- prose walkthroughs[W2].cases[0].result -->
どの判定でも受け付け、展開先も今と同じ

<!-- prose walkthroughs[W2].cases[1].result -->
2.6.1 が断った名前を受け付けるようになる。この change は 9 の判定を使わず、2.6.1 と同じに断る

<!-- prose walkthroughs[W2].cases[2].result -->
「zip の道が不正」で断る（2.6.1 と同じ）

<!-- prose walkthroughs[W2].exceptions[0].text -->
Windows での `NUL`・`a.` の結果は、試験 1.4 の Windows 向けの走りで 2.6.1 の値を得て固定する

<!-- prose model.caption -->
上げる部品と、前と同じに振る舞う 3 つの所

<!-- prose sections.makes.tldr -->
zip の部品を 2.6.1 から 9 に上げ、例外の表から zip の 7 行を消す。

<!-- prose sections.change.tldr -->
zip の版と例外の表と許諾の一覧が変わる。zip を使う所の振る舞いは前と同じ。

<!-- prose sections.lost.tldr -->
Linux でだけ走る権限の写しで特別な bit が落ち、壊れた名前の欄の書庫が読めるようになる。

<!-- prose sections.uses.tldr -->
脆弱性の警告と例外の表に、事実と違う理由の zip の行が残らない。

<!-- prose sections.watch.tldr -->
zip 9 は名前の読み方と道の判定を変えた。どちらも 2.6.1 の規則を自前で残す。

<!-- prose sections.findings.tldr -->
重大な問題は見つけていない。外の道具で書き出しを解けるかは確かめていない（IMP-03）。

<!-- prose sections.rest.tldr -->
zip と無関係な勧告は直さない。書き出しの埋め込み方は AI に任せた。

<!-- prose features[upgrade].summary -->
2 つの部品の zip を 9 にし、使う 4 か所を新しい API に合わせる

<!-- prose features[upgrade].tldr -->
サーバと PC の収集アプリの zip を 9.0.5 以上に上げる。使い方は新しい API に合わせるだけ。

<!-- prose points[upgrade.1].text -->
サーバと PC の収集アプリの `Cargo.toml` の zip を `"9"` にし、`Cargo.lock` の zip は 9.0.5 以上の 1 つの版になる。zip を使う 4 か所（書き出し・台帳の照合・Takeout の取り込み・PC の更新の展開）と試験は、新しい API に合わせて直すだけで意味を変えない。

<!-- prose points[upgrade.2].text -->
features は既定のまま。既定の bzip2 が新しい部品（Rust で書き直した libbz2）を引くので、許す許諾の一覧に 1 つ足す（下の「許諾の一覧」）。絞れば一覧は変えずに済むが、bzip2 などで縮めた Takeout の書庫を読めなくなる。

<!-- prose points[upgrade.3].text -->
予定のパスの中で直しても試験が通らないときでも、Task agent は例外の理由を直す側へ自分で切り替えない。落ちたものと、本人の決めた代わり（理由を「上げる予定: 2 → 9 の上げの作業が残る」に直し、期限を 90 日以内に置く）を添えて人間に返す。

<!-- prose features[export].summary -->
2.6.1 で書いた書き出しを試験に埋め込み、上げた後も同じに読めると固定する

<!-- prose features[export].tldr -->
前の版で書いた書き出しの zip を、上げた後の版で同じ中身に読めることを試験で固定する。

<!-- prose points[export.1].text -->
上げる前の木（zip 2.6.1）で、試験の DB に決まった記録を置いて書き出しを 1 回走らせ、そのバイト列・項目ごとの SHA-256・書き出しの id・合言葉を試験に埋め込む。上げた後の版で全部の項目が同じ中身で読め、違う合言葉では読めない。

<!-- prose points[export.2].text -->
置き場に前の版の書き出しがあり、台帳がそれを書いたままと判定すると、上げた後の版で次の書き出しを書き終えたときに前のファイルが消え、1 つだけ残る（今と同じ）。

<!-- prose points[export.3].text -->
上げた後に新しく書く書き出しの項目は、前の版と同じ AES の形（AES-256・中身は deflate・AE-1 / AE-2 の別も同じ）。形は zip の部品を通さず、バイト列から読んで比べる。

<!-- prose points[export.4].text -->
確かめるのは「新しい版が古い書き出しを読める」向きだけ。切り戻した前の版のサーバが、上げた後に書いた書き出しを読めるかは試験しない（形が同じことは見る）。

<!-- prose features[takeout].summary -->
zip 9 は名前の読み方を変えたので、2.6.1 と同じ規則を自前で残す

<!-- prose features[takeout].tldr -->
Takeout の書庫の項目の名前は、上げた後も 2.6.1 と同じ文字列で読む。

<!-- prose points[takeout.1].text -->
Takeout の書庫を開く所は、項目の名前を 2.6.1 と同じ規則（UTF-8 の印があれば UTF-8、無ければ CP437）で作る。ディスクへ展開せずに読む・ディレクトリを飛ばす・読めない zip を `broken_zip` として残す、は今と同じ。

<!-- prose points[takeout.2].text -->
名前の付け直しの欄（Info-ZIP Unicode Path）の検査値が合わない書庫だけは、2.6.1 では開けず `broken_zip` だったが、上げた後は見出しの名前で読める。読める側に変わるだけで、取り込み済みの記録の名前は変わらない。

<!-- prose points[takeout.3].text -->
実物の Takeout の書庫が UTF-8 の印を持つかは確かめていない。2.6.1 と同じ規則で読む限り、どちらでも記録に残る名前は上げる前と同じ。

<!-- prose features[pcupdate].summary -->
zip 9 は道の判定を変えたので、2.6.1 と同じ名前の確かめを自前で当てる

<!-- prose features[pcupdate].tldr -->
PC の更新を展開するとき、断る名前と受け付ける名前は上げた後も 2.6.1 と同じ。

<!-- prose points[pcupdate.1].text -->
更新の配布物は指紋を照合してから自前で展開する（zip の部品の `extract` に置き換えない）。名前に `\0` がある・前置きか根から始まる・`..` で外へ出る名前は「zip の道が不正」で断る。展開先は今と同じ。

<!-- prose points[pcupdate.2].text -->
zip 9 の判定のままだと、2.6.1 が断った `/etc/passwd`・`C:\x` を相対に直して受け付け、2.6.1 が通した `NUL`・`a.` を Windows で断る。どちらにも揃えず 2.6.1 の結果のとおりにし、名前ごとの結果を Windows と Linux の両方の試験で固定する。

<!-- prose points[pcupdate.3].text -->
Unix だけで走る権限の写しは、zip 9 では SUID などの特別な bit が落ちる。PC の収集アプリは Windows で動き、この枝は配る物では走らない。写し方のコードは変えず、差を PR 本文に挙げる。

<!-- prose features[exceptions].summary -->
勧告の直った版に上げるので、zip の 7 行を例外の表から消す

<!-- prose features[exceptions].tldr -->
例外の表から zip の 7 行（GHSA 3 件と RUSTSEC 4 件）を消す。ほかの行は変えない。

<!-- prose points[exceptions.1].text -->
GHSA-m665-v7pm-7grj・GHSA-q7v9-x93c-xqv3・GHSA-8hxf-qr8p-62x4 と RUSTSEC-2026-0367・0368・0369・0429 の 7 行を消す。PR の脆弱性の検査（新しく持ち込んだ勧告が 0 件・表で足した・変えた行が無し）が通る。

<!-- prose points[exceptions.2].text -->
全体の脆弱性の検査は zip と無関係な 3 件（chrono・libsqlite3-sys・rustix）で今も落ちる。この change の範囲の外で、直らない。

<!-- prose features[license].summary -->
既定の bzip2 が引く部品の許諾 bzip2-1.0.6 を、許諾の検査の一覧に足す

<!-- prose features[license].tldr -->
許諾の検査の許す一覧に bzip2-1.0.6 を 1 つ足し、17 の値を 18 にする。

<!-- prose points[license.1].text -->
許す一覧の末尾に `bzip2-1.0.6` を足す。直すのは台本の既定の一覧と、自己試験が一覧を丸ごと比べる行の 2 か所。正典の一覧も 18 の値に書き直す。

<!-- prose points[license.2].text -->
一覧の Requirement のほかの本文と Scenario は変えない。他人の部品の AGPL を落とす・自分の部品を決めた許諾の式で通す、は今と同じ。

<!-- prose points[license.3].text -->
上げた後に、`bzip2-1.0.6` のほかに一覧に無い許諾の部品を引いたら、一覧を足さずに人間に返す。

<!-- prose impact.rules[R1].legit -->
zip 9 の既定の bzip2 が引く部品（bzip2-1.0.6）。一覧に足すので止まらない

<!-- prose impact.rules[R2].legit -->
前の版で書いた書き出しも同じに消える（今と同じ振る舞い）

<!-- prose impact.rules[R3].legit -->
なし（合言葉を知っている本人は読める）

<!-- prose impact.rules[R4].legit -->
なし（上げた版には勧告が当たらない）

<!-- prose impact.findings[IMP-01].title -->
Linux の展開で SUID などが写らなくなる

<!-- prose impact.findings[IMP-01].text -->
PC の更新の展開の Unix だけの枝は、zip 9 では特別な bit の落ちた権限を写す。配る物（Windows）では走らない

<!-- prose impact.findings[IMP-02].title -->
切り戻した前の版が、上げた後の書き出しを照合できるかは試験しない

<!-- prose impact.findings[IMP-02].text -->
試験は「新しい版が前の版の書き出しを読む」向きだけを見る。前の版に戻したときに、上げた後に書いた書き出しを台帳で照合できるかは見ていない

<!-- prose impact.findings[IMP-03].title -->
上げた後の書き出しを外の道具で解けるかは確かめていない

<!-- prose impact.findings[IMP-03].text -->
試験は AES の形（見出しの値）と、上げた後の版で読めることを見る。7-Zip のような外の道具で合言葉から解けるかは、上げる前も後も確かめていない

<!-- prose impact.scope.covers[0].text -->
サーバの書き出し・台帳の照合・Takeout の取り込みと、PC の収集アプリの更新の展開

<!-- prose impact.scope.covers[1].text -->
例外の表の zip の 7 行と、許諾の検査の許す一覧

<!-- prose impact.scope.preconditions[0].text -->
前の版の書き出しの互換を試験で見るのは、2026-11 のサーバ（zip 2.6.1）で書いたものだけ

<!-- prose impact.scope.preconditions[1].text -->
許す一覧に足すのは bzip2-1.0.6 だけ。ほかの許諾を引いたら人間に返す

<!-- prose impact.scope.preconditions[2].text -->
Takeout の名前の読み方と更新の展開の判定は、2.6.1 の結果を試験に固定した名前の範囲で同じと確かめる

<!-- prose impact.scope.excludes[0].text -->
zip と無関係な勧告（chrono・libsqlite3-sys・rustix）と npm の 6 件

<!-- prose impact.scope.excludes[1].text -->
携帯の収集アプリ（zip を使わない）

<!-- prose impact.scope.residual[0].text -->
上げた後の書き出しを外の道具で解けるか（IMP-03）

