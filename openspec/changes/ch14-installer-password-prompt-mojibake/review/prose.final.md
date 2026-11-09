# CH14 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
日本語の Windows でインストーラが合言葉を聞くとき、問いの文が化けずに読めるようになる。最初の問いの前に「16 文字以上・change-me で始めない」などの条件が出るので、何文字要るかを知ってから打てる。

<!-- prose impact.overview -->
日本語の Windows の窓で、合言葉の問いが化けずに読めるようになり、最初の問いの前に条件が 1 回出る（インストールの最後・password・repair の 3 つの口で、標準入力が窓のとき）。検査の中身と、管で流すときの読み方は変えない。本人ができなくなることは、この分析では見つけていない（長い条件の行の折り返しは確かめていない）。設計のままでは CI の unsafe の検査が落ち（IMP-01）、窓の試験が長い行の折り返しで落ちうる（IMP-03）。

<!-- prose impact.problem -->
2026-11-09、本人が日本語の Windows でインストーラを開くと、昇格して開き直した窓で、合言葉の問いだけが化けた。何を聞かれているか読めず、短い合言葉を打つと「短い」とだけ断られて、何文字要るか分からなかった。CI は合言葉を管で流すので、この経路を一度も通っておらず、見逃していた。

<!-- prose uses[U1].title -->
日本語の Windows で合言葉を決め直す

<!-- prose uses[U1].trigger -->
本人が日本語の Windows で ashiato-setup.exe password を開く。管理者の確認を通すと、新しい窓が開く。

<!-- prose uses[U1].outcome -->
問いが読めて、何文字要るかを最初から知ったうえで合言葉を決められる

<!-- prose uses[U1].without -->
問いが『逕ｻ髱｢縺ｮ蜷郁ｨ闡』と化けて何を聞かれているか分からない。断られても「短い」としか出ず、何文字要るか分からない

<!-- prose uses[U1].steps[0].do -->
管理者の確認を通す

<!-- prose uses[U1].steps[0].see -->
開き直した窓に「画面の合言葉を決めます。16 文字以上・change-me で始めない・…。入れた文字は表示されません」の 1 行と、その後に「画面の合言葉: 」

<!-- prose uses[U1].steps[1].do -->
15 文字の合言葉を 2 回打つ

<!-- prose uses[U1].steps[1].see -->
打った文字は出ない。「合言葉を受け付けない: 短い（16 文字以上）。入れ直してください」と出て、「画面の合言葉: 」に戻る

<!-- prose uses[U1].steps[2].do -->
20 文字の合言葉を 2 回打つ

<!-- prose uses[U1].steps[2].see -->
受け付けられ、サーバが起こし直されて、その合言葉で画面にログインできる

<!-- prose walkthroughs[W1].title -->
同じ問いの文が、書き方で化けたり化けなかったりする

<!-- prose walkthroughs[W1].scene -->
コード頁 932 の窓（日本語の Windows）で、インストーラが「画面の合言葉: 」を窓に出す。

<!-- prose walkthroughs[W1].why -->
rpassword は問いの文を窓へバイトのまま書き、窓はバイトを自分のコード頁で読む。Rust の標準出力は窓のとき文字として書くので、コード頁に依らない。この change は問いも文字として窓へ書き、rpassword は読みだけに使う。

<!-- prose walkthroughs[W1].figure.caption -->
書き方ごとの窓の表示（2026-11-09 に本人の PC の疑似コンソールで再現）

<!-- prose walkthroughs[W1].cases[0].result -->
化けて読めない

<!-- prose walkthroughs[W1].cases[1].result -->
いまも化けない

<!-- prose walkthroughs[W1].cases[2].result -->
読める。向け替えても窓に出る

<!-- prose walkthroughs[W1].exceptions[0].text -->
GitHub の runner で疑似コンソールが使えるかはまだ確かめていない

<!-- prose model.caption -->
合言葉を聞く 1 つの関数と、文字の行き先（この change の後）

<!-- prose features[prompt].summary -->
問いの文を窓へ直に書き、打った文字は rpassword が見せずに読む

<!-- prose points[prompt.1].text -->
標準入力が窓のとき、「画面の合言葉: 」「画面の合言葉（もう一度）: 」を窓へ直に書く。窓のコード頁が 932 でも化けない

<!-- prose points[prompt.2].text -->
インストールの最後・password の口・repair の口は、どれも同じ 1 つの関数を通る。直すのは 1 か所で、3 つの口に効く

<!-- prose points[prompt.3].text -->
標準出力と標準エラーをファイルへ向け替えても、問いは窓に出る。ファイルには入らない（今の rpassword と同じ性質を残す）

<!-- prose points[prompt.4].text -->
打った合言葉は窓に出ない。元の値は設定・ログ・インストーラの出力のどこにも残らない（今と同じ）

<!-- prose points[prompt.5].text -->
標準入力が管のとき（CI）は今のまま。問いの文は出さず、標準入力の 2 行を 2 回入れた合言葉として読む

<!-- prose features[rules].summary -->
最初の問いの前に 1 回だけ条件を示し、短い・雛形の断りには条件を添える。検査の中身は変えない

<!-- prose points[rules.1].text -->
最初の問いの前に、条件の 1 行（16 文字以上・change-me で始めない・自動で作った API の合言葉やログインの印の鍵と同じにしない）を 1 回だけ出す。3 つの口のどれでも出る

<!-- prose points[rules.2].text -->
短い・雛形で断るときは、理由に条件を添える。条件の数（16）と語（change-me）は検査と同じ定数から組むので、検査と文言がずれない

<!-- prose points[rules.3].text -->
断られても条件の行は出し直さない。断りの行だけが出る（条件は最初の 1 回だけ）

<!-- prose points[rules.4].text -->
「入れた文字は表示されません」は窓のときだけ添える。管（CI）では付けない

<!-- prose features[test].summary -->
Windows の CI が疑似コンソールで窓を作って毎回確かめ、本人が日本語の Windows で 1 回見る

<!-- prose points[test.1].text -->
Windows の CI（install-windows）が、疑似コンソールで窓を作ってコード頁を 932 に替え、password の口で合言葉を 2 回打つ。問いと条件が読めること・条件が問いより前にあること・決め直せること・打った文字が出ないこと・向け替えても問いが窓に出ることを見る

<!-- prose points[test.2].text -->
試験は先に、rpassword と同じ書き方の子が化けて見えることを確かめる。化けを見分けられない窓（UTF-8 の窓など）で「化けなかった」と言わないためである

<!-- prose points[test.3].text -->
GitHub の Windows の runner で疑似コンソールが使えるかは、まだ走らせていない（本人の PC では通った）。使えなければ下流が止まり、窓の Scenario 4 本の置き場を人間に問い直す

<!-- prose points[test.5].text -->
条件の 1 行は表示幅で約 168 桁あり、試験の疑似コンソール（120 桁）では折り返す。試験は 5 つの語を 1 行の中に探すので、折り返しの表し方しだいで落ちうる（IMP-03）

<!-- prose points[test.4].text -->
確認バッチで本人が日本語の Windows で password の口を開き、昇格して開き直した窓で問いと条件が読めること・打った文字が出ないこと・15 文字で「短い（16 文字以上）」と断られることを 1 回見る（CI の runner は昇格して開き直す窓を作らない）

<!-- prose features[scope].summary -->
検査の中身・管での読み方は変えない。子の命令の出力の化けは別の change

<!-- prose points[scope.1].text -->
合言葉の検査（16 文字・change-me・API の合言葉・ログインの印の鍵・2 回の一致）と、断りの行の形は変えない

<!-- prose points[scope.2].text -->
インストーラが子の命令（sc.exe・icacls・whoami・reg.exe・tasklist）の出力を読む口も cp932 で化けうるが、この change では直さない。日本語の利用者名や日本語のダウンロードの場所では、repair やインストールが止まる見込みがある（本人の PC は英字の利用者名で、今は当たらない）

<!-- prose points[scope.3].text -->
設計は窓へ書く部品にだけ unsafe を許す（console.rs）。ただし CI の検査（tools/check-unsafe-scope.sh）は、unsafe を許す属性を PC の収集アプリの secret.rs の外に置くと落とす。tasks はこの検査に触れていない。検査の許す場所を広げるか、unsafe の要らない書き方にするかを、仕様の段で決め直す必要がある（IMP-01）

<!-- prose impact.rules[R1].legit -->
規則の中身はこの change で変えない。条件が先に出るので、本人は断られる前に知ることができる

<!-- prose impact.findings[IMP-01].title -->
窓へ書く部品の unsafe を、CI の unsafe の検査が落とす

<!-- prose impact.findings[IMP-01].text -->
design D4 は console.rs の先頭に allow(unsafe_code) を置く。CI の tools/check-unsafe-scope.sh は、unsafe_code を許す属性を crates/collector-windows/src/secret.rs の外に置くと落とす（ST56 の決定）。tasks はこの検査に触れていないので、書いたとおりに作ると CI が落ちて merge できない。

<!-- prose impact.findings[IMP-02].title -->
runner で疑似コンソールが使えなければ、窓の Scenario 4 本の置き場が無くなる

<!-- prose impact.findings[IMP-02].text -->
窓の試験は、GitHub の Windows の runner で疑似コンソールが使えることを前提にする。本人の PC では通ったが、runner ではまだ走らせていない。使えなければ下流が止まり、試験の置き場を人間に問い直す（design D6 の反転条件）。install-check は CI で release と test の zip に 2 回走るので、窓の節も 2 回走り、password の口の起こし直し（最大 180 秒待つ）が 1 回あたり 2 回増える。

<!-- prose impact.findings[IMP-03].title -->
条件の 1 行が疑似コンソールの幅を超え、窓の試験が「1 行」を探して落ちうる

<!-- prose impact.findings[IMP-03].text -->
D3 の条件の行は表示幅で約 168 桁ある。試験の疑似コンソールは 120 桁で作る（premise-probe/probe.ps1 の下敷き）。tasks 4.2 は 5 つの語を全部含む 1 行を窓の文字から探す。折り返しで改行や桁の移動が入ると、表示は正しくても試験が落ちうる。design の Risks は「短い行だけを探す」と書いており、4.2 と食い違う。

<!-- prose impact.scope.covers[0].text -->
標準入力が窓のときの、インストールの最後・password・repair の合言葉の問いと条件の表示

<!-- prose impact.scope.preconditions[0].text -->
問いを窓へ出すのは、標準入力が端末（Windows の窓）につながっているときだけ。管のときは問いを出さず、条件の行だけを標準出力に出す

<!-- prose impact.scope.preconditions[1].text -->
条件は最初の問いの前に 1 回だけ。断るたびには出し直さない

<!-- prose impact.scope.excludes[0].text -->
子の命令の出力の化け（失敗の理由の表示・whoami の利用者名・reg.exe のダウンロードの場所）

<!-- prose impact.scope.residual[0].text -->
日本語の利用者名・日本語のダウンロードの場所の PC では、repair やインストールが子の命令の出力の化けで止まる見込みが残る

<!-- prose impact.scope.residual[1].text -->
CI の unsafe の検査との食い違い（IMP-01）・runner で疑似コンソールが使えるか（IMP-02）・長い条件の行の折り返しで試験が落ちうる（IMP-03）

