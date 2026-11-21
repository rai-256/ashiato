# CH22 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
PR の CI が、配る形のサーバと PC の収集アプリを外につながる状態で初めから入れ、5 分置いて様子を見る。落ち続け・止まったまま・同じ誤りの続き・配布元に聞けない、のどれかがあれば、配る前に CI が赤になる。0.1.0 の「入れて放っておくだけで更新係が 10 秒ごとに落ちる」は、この段で配る前に見つかる。

<!-- prose impact.overview -->
PR の CI の Windows のインストールの試験に、配る形を外につながる状態で入れて 5 分置く段が付く。落ち続け・止まったまま・同じ誤りの続き・サーバの panic・配布元に聞けない、のどれかで job が赤になる。ただし配布元の答えが「問い合わせの上限」（403・429）だけの回は警告で通り、その回は一覧の読み取りまでは確かめない。代わりに、配布元（api.github.com）が落ちている日は製品と関係の無い PR でも赤になり、走らせ直すまで merge できない。見るのは 5 分の間に出る壊れ方だけで、長く置かないと出ない問題と、配布元の答えの形の正しさは、この分析の範囲では見ていない。

<!-- prose impact.problem -->
0.1.0 は、入れて放っておくだけで更新係が 10 秒ごとに落ちた。CI は job の最初から更新係の外向きを塞ぎ、偽の受け手だけで試していたので、配る形の更新係は 1 度も本物の配布元と握手しないまま緑で配られた（#329）。

<!-- prose uses[U1].title -->
更新係の握手を壊した PR を、配る前に止める

<!-- prose uses[U1].trigger -->
HTTPS の部品を入れ替える PR を作った。手元の試験と偽の受け手の試験は全部緑。でも 0.1.0 のように、配ってから本物の配布元との握手で落ちないかが気になる。

<!-- prose uses[U1].outcome -->
配る前に、入れて放っておくだけで落ち続ける不具合だと分かり、直してから配れる

<!-- prose uses[U1].without -->
CI は塞いだまま偽の受け手で試すので緑になり、配った後に本人の PC で更新係が 10 秒ごとに落ちて初めて気づく（0.1.0 の #329）

<!-- prose uses[U1].steps[0].do -->
PR の CI の結果を開く

<!-- prose uses[U1].steps[0].see -->
Windows のインストールの試験の job だけが赤

<!-- prose uses[U1].steps[1].do -->
job のログの置く段を開く

<!-- prose uses[U1].steps[1].see -->
ashiato-updater の起こし直しが 28 回、更新係の誤りのログに同じ誤りの行が 28 行（行は秘密を伏せて出る）。確かめた記録に「確かめた」の行が無い

<!-- prose uses[U1].steps[2].do -->
同じ PR を直して push し直す

<!-- prose uses[U1].steps[2].see -->
置く段が緑になり、起こし直し 0 回・確かめた記録「確かめた。新しい版なし」の行がログに出る

<!-- prose walkthroughs[W1].title -->
0.1.0 型の落ち続けと、正常な置き方

<!-- prose walkthroughs[W1].scene -->
置く段が配る形のサーバの zip を入れて 5 分置く。更新係が本物の配布元との握手で panic する組み立てと、正常な組み立てを比べる。

<!-- prose walkthroughs[W1].why -->
何もしないで置いて落ちる・止まるのは、それだけで不具合とみなす（Q2）。起こし直しの数だけでは止まったままを見逃すので、置き終わりの状態も見る。

<!-- prose walkthroughs[W1].figure.caption -->
5 分置いた後に置く段が見るもの（回数は説明用の値）

<!-- prose walkthroughs[W1].cases[0].result -->
回復が 10 秒ごとに起こし直すので、起こし直しの回数・同じ誤りの行・確かめた記録の 3 つで落ちる

<!-- prose walkthroughs[W1].cases[1].result -->
起こし直し 0 回、誤りの行なし、確かめた行 1 行で通る。緑の回もこれらの数をログに出す

<!-- prose walkthroughs[W1].cases[2].result -->
起こし直されないので回数は 0 だが、置き終わりの状態の判定で落ちる

<!-- prose walkthroughs[W1].exceptions[0].text -->
回数・行数は説明用の値。回復が 4 回目以降も起こし直すかは確かめていない（3 回で止まっても置き終わりの状態の判定で落ちる）

<!-- prose walkthroughs[W2].title -->
確かめた記録の理由と、置く段の結果

<!-- prose walkthroughs[W2].scene -->
置き終わりに、更新係の確かめた記録の、置く段の始まりより後の行を読む。

<!-- prose walkthroughs[W2].why -->
記録が 1 行も無いまま緑になる道（24 時間の間隔・塞ぎの外し漏れ）を塞ぐため、成功の行を求める。理由の語は外の揺れと ashiato の誤りを分けないので、上限のほかは全部赤にする（Q4）。

<!-- prose walkthroughs[W2].figure.caption -->
置く段の始まりより後の行と結果

<!-- prose walkthroughs[W2].cases[0].result -->
「確かめた」の行が 1 行あり、緑

<!-- prose walkthroughs[W2].cases[1].result -->
「読めない答え」になり赤。走らせ直して済ませる

<!-- prose walkthroughs[W2].cases[2].result -->
警告だけで緑。この回は一覧の読み取りまでは確かめていない（IMP-02）

<!-- prose walkthroughs[W2].exceptions[0].text -->
説明用の行。「問い合わせの上限」には配布元の 403 も入る（IMP-01）

<!-- prose model.caption -->
Windows のインストールの試験の job の並び（この change の後）

<!-- prose features[order].summary -->
前の段の残りを全部消し、塞ぎを外してから、配る形を初めから入れる

<!-- prose points[order.1].text -->
置く段はインストールと更新の試験を全部終えた後に走る。前の段は今のとおり、更新係の外向きを塞いだまま走る（最初のインストールより前に塞ぐ D18 は残る）

<!-- prose points[order.2].text -->
順は「消す → 塞ぎを外す → 入れる → PC の収集アプリを起こす → 5 分置く」。決定の字義（入れた後に外す）と順が違う。更新係は入れた直後に 1 回だけ確かめ、次は 24 時間後なので、入れた後に外すと 5 分の間に配布元へ聞かない

<!-- prose points[order.3].text -->
消すのはサービスの登録・プログラムの置き場・状態の置き場・更新係の置き場・PC の収集アプリの置き場。アンインストールの口は状態と更新係の置き場を残すので、台本が自分で消す

<!-- prose points[order.4].text -->
消した後に前の段の残りがある、または塞ぎの規則を消せないときは、配る zip を入れずに落ちる

<!-- prose points[order.5].text -->
数える窓は 2 つある。確かめた記録は「置く段の始まり」（塞ぎを消す直前）から、起こし直し・止まり・誤りの行は「置く間」（入れ終わって healthz が 200 になってから 5 分）だけを数える

<!-- prose features[services].summary -->
3 つのサービスが置く間に 1 回でも起こし直されたら・止まったら赤

<!-- prose points[services.1].text -->
DB・サーバ・更新係の 3 つのサービスの状態とプロセスの番号を、置く間ずっと見る。1 回でも実行中でなくなる・番号が変わる・異常な終わりの記録があれば落ち、どのサービスが何回かを出す

<!-- prose points[services.2].text -->
起こし直しを数えるだけでは足りない。サーバが起動を断ると「止まった」として返るので起こし直されない。置き終わりに 3 つとも実行中で healthz が 200 であることも見る

<!-- prose points[services.3].text -->
置く間より前の終わりの記録（前の段がわざとサービスを終わらせた試験・入れる最中）は数えない。システムの記録を読めない runner なら警告を出し、状態とプロセスの番号だけで判定する

<!-- prose features[errors].summary -->
1 つのログに同じ誤りが 3 回以上で赤。サーバの panic は 1 行で赤

<!-- prose points[errors.1].text -->
更新係・サーバ・DB・PC の収集アプリの 4 つのログから、置く間に書かれた誤りの行を拾う。日時と DB のプロセスの番号を除いて同じ行を同じ誤りとし、1 つのログに 3 回以上あれば落ちる。1〜2 回の行は出力に出すだけ

<!-- prose points[errors.2].text -->
サーバの panic の行は 1 行で落ちる。サーバは作業を分けて動かすので、分けた作業の panic ではプロセスが止まらず、起こし直しとしては見えない。更新係と PC の収集アプリの panic は、プロセスの終わりとして見る

<!-- prose points[errors.3].text -->
警告の段の行は数えない。置く段の始まりより前の行、入れる最中（置く間より前）の行も数えない

<!-- prose features[checks].summary -->
更新係と PC の収集アプリのそれぞれに、置く段の始まりより後の「確かめた」が 1 行は要る

<!-- prose points[checks.1].text -->
置き終わりに、更新係と PC の収集アプリの確かめた記録を読む。置く段の始まりより後に「確かめた」（成功）の行がそれぞれ 1 行以上あれば通る。いまの配布元は空の一覧を返すので、正常なら「確かめた。新しい版なし」になる

<!-- prose points[checks.2].text -->
配布元が落ちている日（5xx・時間切れ）も赤になる。理由の語は ashiato の誤りか外の揺れかで分かれていないので、外の揺れだけを許すことはできない。赤なら走らせ直す

<!-- prose points[checks.3].text -->
理由が全部「問い合わせの上限」なら落とさず、CI の警告を出して通る（共有の runner の IP の都合）

<!-- prose features[collector].summary -->
設定せずに起こし、途中で 1 回起こし直して、配布元に確かめさせる

<!-- prose points[collector.1].text -->
配る形の PC の収集アプリを、送り先を設定せずに起こし、決まった時間の後に 1 回止めて起こし直す。本人が翌日 PC を起こした場面にあたる。置き終わりにプロセスが動いていること、誤りの行が続かないことも見る

<!-- prose points[collector.2].text -->
起こし直しが要るのは、初めて起きたときは更新の案内を出すだけで配布元に聞かないから。起こし直しより前の「確かめた」は数えない。runner で通知の差出人を登録できないと確かめる係が付かず、製品が正しくても赤になる（その行の有無を毎回出す）

<!-- prose features[canon].summary -->
正典の「問い合わせない」を、名指しの 2 つの例外の形に改める

<!-- prose points[canon.1].text -->
自動の試験が本物の配布元へ繋ぐのは、握手の試験 tls_online_*（更新係と PC の収集アプリで 1 回ずつ）と置く段の 2 つだけ。ほかの段は今のとおり更新係の外向きを塞いで走る。問い合わせの形（版の一覧を取る決まった GET だけ）の約束と既存の 9 本の Scenario は変えない

<!-- prose points[canon.2].text -->
正典の「外への接続を塞ぐ」を「更新係の外への接続を塞ぐ」に揃える。今の CI の塞ぎが更新係だけなのに合わせたもので、塞ぐ範囲は今と同じ

<!-- prose features[ops].summary -->
赤のときの扱い・時間・ログで秘密を伏せること

<!-- prose points[ops.1].text -->
置く段が最初の走りでいまの不具合を見つけて赤になっても、段は緩めず、許す一覧も持たない。直すのは別の change にし、CH22 はその change の後に merge する。段の側の誤り（台本・runner の環境）なら CH22 の中で直す

<!-- prose points[ops.2].text -->
置く段が出す行（起こし直しの回数・誤りの行・確かめた記録・落ちたときの観測）は公開の CI ログに出るので、インストールの試験と同じ観測の部品で秘密を伏せてから出す。観測を出さずに落ちる道は持たない

<!-- prose points[ops.3].text -->
置く段と Windows のインストールの試験の job に時間の上限を付ける（固まったときに既定の 6 時間を 2 倍で数えない）

<!-- prose impact.scenarios[S1].result -->
Windows のインストールの試験が赤。グラフの merge の関門も止まる

<!-- prose impact.scenarios[S1].lost -->
配布元が戻って走らせ直すまで merge できない（1 回あたり 70 分前後の runner の時間）

<!-- prose impact.scenarios[S2].result -->
そのサービスの起こし直し 1 回で赤

<!-- prose impact.scenarios[S2].lost -->
走らせ直すまで merge できない

<!-- prose impact.scenarios[S3].result -->
製品が正しくても赤

<!-- prose impact.scenarios[S3].lost -->
CH22 が merge できない（D12 で段を緩めないので、その行が不具合かを調べる）

<!-- prose impact.scenarios[S4].result -->
製品が正しくても赤

<!-- prose impact.scenarios[S4].lost -->
CH22 が merge できない

<!-- prose impact.rules[R1].legit -->
配布元が落ちている日（5xx・時間切れ）は、製品に関係の無い PR も赤になり merge が止まる

<!-- prose impact.rules[R2].legit -->
この分析では、正当な利用を止める道を見つけていない（消せない runner は段の側の誤りとして扱う）

<!-- prose impact.rules[R3].legit -->
runner の都合（OS の更新・資源の不足）でサービスが 1 回落ちる回も赤になる。起きるかは確かめていない

<!-- prose impact.rules[R4].legit -->
何もしないで置いた正常な状態でも同じ誤りを 3 回書く行があれば、製品が正しくても赤になる。正常でどれだけ書くかは見ていない

<!-- prose impact.rules[R5].legit -->
runner で通知の差出人を登録できないと確かめる係が付かず、製品が正しくても赤になる

<!-- prose impact.rules[R6].legit -->
毎晩の試験など 3 つ目を足すときは、この文を改める change が要る（意図した手間）

<!-- prose impact.findings[IMP-01].title -->
配布元の 403 も「問い合わせの上限」になり、警告だけで通る

<!-- prose impact.findings[IMP-01].text -->
更新係と PC の収集アプリは 403 と 429 をどちらも「問い合わせの上限」と書く。置く段は理由が全部上限なら警告だけで通すので、配布元が上限以外の理由で断った 403 も緑になる。ただし 403 を受け取った時点で握手は終わっているので、0.1.0 型の握手の不具合はこの道でも捕まえる

<!-- prose impact.findings[IMP-02].title -->
上限の警告が常になると、一覧の読み取りを確かめない回が続く

<!-- prose impact.findings[IMP-02].text -->
共有の runner の IP は他の利用者と上限を分け合う。上限の警告で通る回は握手までは確かめるが、版の一覧の読み取りと版の選び方は確かめない。その回が多ければ、置く段は一覧の読み取りの壊れ方を捕まえにくい

<!-- prose impact.findings[IMP-03].title -->
リリースを公開すると、コードを変えない PR の置く段の結果も変わりうる

<!-- prose impact.findings[IMP-03].text -->
いまの配布元は空の一覧を返し、置く段は「新しい版なし」を見る。リリースを公開すると、更新係は一覧を読み、版を選ぶ道を通る。そこで「読めない答え」になれば、その後の Windows の通しが走る PR は全部赤になる。いま進めて安全なのは、公開のリリースが 0 件で、置く段が読むのは空の一覧だけだから（公開するまでこの道は通らない）

<!-- prose impact.findings[IMP-04].title -->
置く段の台本だけを直しても、Windows の配布物を組み直す

<!-- prose impact.findings[IMP-04].text -->
組み立ての鍵が除くのは名指しの試験の台本 3 つだけで、新しい台本はそこに無い。置く段の台本だけを直しても配布物を組み直し、その分 CI が延びる

<!-- prose impact.scope.covers[0].text -->
Windows の通しが走る PR と push の CI で、配る形（正式版）のサーバと PC の収集アプリを、初めから入れて 5 分置いたときの壊れ方

<!-- prose impact.scope.preconditions[0].text -->
置く段が走るのは、Windows の通しが走る変更（crates・Cargo・.github・tools/release・migrations・web）のときだけ

<!-- prose impact.scope.preconditions[1].text -->
起こし直し・止まり・誤りの行は置く間（healthz が 200 になってから 5 分）だけ、確かめた記録は置く段の始まりより後だけを数える。誤りの行は警告の段を除く

<!-- prose impact.scope.preconditions[2].text -->
本物の配布元へ繋ぐのは握手の試験と置く段だけで、塞ぐのは更新係の外向きだけ

<!-- prose impact.scope.preconditions[3].text -->
PC の収集アプリは、台本が止めた 1 回だけを起こし直しとして許し、確かめた記録は台本の起こし直しより後だけを数える

<!-- prose impact.scope.excludes[0].text -->
長く置かないと出ない問題・配布元の答えの形を比べること・開発版の配る形・携帯の収集アプリ・PC の収集アプリの送る道

<!-- prose impact.scope.residual[0].text -->
配布元が落ちている日は PR が赤になり、走らせ直しが要る

<!-- prose impact.scope.residual[1].text -->
上限の警告の回は一覧の読み取りを確かめない（IMP-01・IMP-02）

<!-- prose impact.scope.residual[2].text -->
確かさ 1 の前提（正常なログの誤りの行・システムの記録を読めるか・通知の登録）は、最初の走りまで分からない

