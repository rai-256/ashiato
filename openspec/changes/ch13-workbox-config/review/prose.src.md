# CH13 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
ハーネスの作業箱（検証と agent を thread ごとのコンテナで走らせる仕組み）を、ashiato2 でいつでも有効にできるように、箱の設定ファイルを依頼の中身のまま置く。置いただけでは何も変わらない。依頼の中身のまま有効にすると落ちる検証が見つかったので、その一覧を「有効にする前の宿題」として残す。

<!-- prose impact.overview -->
設定ファイル `tools/workbox.json` が 1 つ増える。`HARNESS_WORKBOX` が off（既定）の間は誰も読まないので、検証・CI・agent の振る舞いはこの分析で見た範囲では変わらない。できなくなることはこの分析では見つけていない。ただし依頼の中身のまま箱を有効にすると、画面の e2e などが箱の中で落ちる。その一覧を有効にする前の宿題として残す。

<!-- prose impact.problem -->
並走する Story の検証が同じ機械を取り合い、試験の漏れが共有の /tmp を埋め（10-23）、メモリ不足で langgraph が落ち（10-25）、試験の `pgrep -f "/"` が利用者の全プロセスに当たった（11-07 CH10）。作業箱で隔離するには、repo に箱の設定が要る。

<!-- prose uses[U1].title -->
並走で /tmp やメモリがまた溢れたので、箱を有効にしようとする

<!-- prose uses[U1].trigger -->
Story を 5 本並べて走らせていたら、試験の漏れで /tmp が埋まり、langgraph がまた OOM で落ちた。作業箱で検証を隔離したい。

<!-- prose uses[U1].outcome -->
有効にした日に画面を持つ Story の検証が一斉に落ちる、という事態を、有効にする前に知って順番を決められる

<!-- prose uses[U1].without -->
設定ファイルだけが main にあり、有効にした日に e2e や workspace の試験が落ちてから原因を探すことになる

<!-- prose uses[U1].steps[0].do -->
GitHub の issue「作業箱を有効にする前の宿題（CH13）」を開く（PR 本文の先頭からも同じ一覧へ飛べる）

<!-- prose uses[U1].steps[0].see -->
0〜6 の見出しの宿題。0 は「harness2 の鍵の名前と突き合わせる」「pgrep の確かめを HARNESS_WORKBOX を外したシェルから走らせる」

<!-- prose uses[U1].steps[1].do -->
1 の「docker が要るのに渡らない」を読む

<!-- prose uses[U1].steps[1].see -->
画面の e2e（`cd web && npm run test:e2e`）・`cargo test --workspace`・workflow の検査が、箱の中で docker を受け取れずに落ちること。どのファイルの何行目が理由かも書いてある

<!-- prose uses[U1].steps[2].do -->
直す作業を別の change として登録し、それが済んでから `HARNESS_WORKBOX=tests` にする

<!-- prose uses[U1].steps[2].see -->
ashiato2 の側に止める口は無い。環境変数を変えた時点から箱が使われる

<!-- prose walkthroughs[W1].title -->
依頼の中身のまま有効にした日の、画面の e2e

<!-- prose walkthroughs[W1].scene -->
本人が `HARNESS_WORKBOX=tests` にした後、画面を持つ Story の下流が、tasks の検証 `cd web && npm run test:e2e` を走らせる。

<!-- prose walkthroughs[W1].why -->
作業箱は、検証コマンドの文字列に `docker` の欄の正規表現が当たるかだけを見て docker を渡す。コマンドが中で呼ぶ台本は見ない。依頼の中身は、直に呼ばれる台本の名前だけを挙げている。

<!-- prose walkthroughs[W1].figure.caption -->
箱が docker を渡すかは、検証コマンドの字面だけで決まる

<!-- prose walkthroughs[W1].cases[0].result -->
字面が `docker` の欄の正規表現に当たるので docker が渡る

<!-- prose walkthroughs[W1].cases[1].result -->
字面に `tools/stack.sh` が出ないので docker が渡らず、`docker compose up` で落ちる

<!-- prose walkthroughs[W1].exceptions[0].text -->
この change では直さない。宿題の一覧の 1 に載せるだけ（依頼が中身の書き換えを禁じている）

<!-- prose model.caption -->
置くものと、それが効く時

<!-- prose features[place].summary -->
依頼の JSON を 1 字も変えずに `tools/workbox.json` に置く

<!-- prose points[place.1].text -->
依頼（`docs/changes/CH13.md` の逐語）の JSON の塊を機械で取り出し、そのまま `tools/workbox.json` に書く。中身は JSON として読んで依頼と等しいことで確かめる

<!-- prose points[place.2].text -->
置いただけでは何も変わらない。ハーネスがこのファイルを読むのは `HARNESS_WORKBOX` が `tests` か `all` のときだけで、既定は off。ashiato2 の台本・コード・CI もこのファイルを読まない

<!-- prose features[homework].summary -->
依頼の中身のままでは箱の中で落ちる検証の一覧を、3 か所に残す

<!-- prose points[homework.1].text -->
一覧の正本は change の中の `enable-before.md`（archive の後も repo に残る）。PR 本文の先頭に案内を置き、同じ一覧を本文にした issue を下流が 1 つ立てる

<!-- prose points[homework.2].text -->
依頼の中身のまま有効にすると、画面の e2e・`cargo test --workspace`・workflow の検査などが docker を受け取れずに落ちる。逆に、台本の名前を grep するだけの検証にも docker が渡る。設定は直さず、一覧に載せるだけ

<!-- prose points[homework.3].text -->
下流の agent が GitHub に issue を立てる（外向きの作用はこの 1 つだけ。同じ題があれば立てない）。agent が網に出られなければ、PR 本文に「merge の後に人間が立てる」1 行を置く形に切り替え、本人に返す

<!-- prose features[done].summary -->
中身の一致・JSON として読める・ほかを変えていない、で閉じる

<!-- prose points[done.1].text -->
閉じる条件は、中身が依頼と等しい・JSON として読める・ashiato2 に読むものが無い・触ったのがこのファイルと change の文書だけ・リポジトリの検査 6 本が通る、の 5 つ。どれも docker も作業箱も要らない

<!-- prose points[done.2].text -->
依頼の完了の判定にあった「作業箱の中で `pgrep -fc /` が 10 未満」は、この change の確かめから外した。harness2 の main に作業箱がまだ無く、置くと CH13 が harness2 の merge まで閉じないため。この確かめは宿題の 0 に載せ、有効にするときに行う

<!-- prose features[scope].summary -->
ファイル 1 つ。仕様にも文書にも手を入れない

<!-- prose points[scope.1].text -->
仕様を持たない change（`skip_specs: true`）。off の間に外から確かめられる振る舞いが無いので、正典の仕様・`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`・CI を変えない

<!-- prose points[scope.2].text -->
後の Story が箱の中で落ちる検証を書かないための規約や検査は、この change では置かない。箱を有効にするときにまとめて決める（宿題の 6）

<!-- prose impact.scope.covers[0].text -->
repo に箱の設定ファイルを置くことと、有効にする前の宿題を残すことだけ

<!-- prose impact.scope.preconditions[0].text -->
`HARNESS_WORKBOX` が off（既定）のときに限って、振る舞いが変わらない

<!-- prose impact.scope.excludes[0].text -->
箱を有効にすることと、箱の中で検証が通るかを確かめること（pgrep の確かめを含む）

<!-- prose impact.scope.excludes[1].text -->
後の Story への規約・検査

<!-- prose impact.scope.residual[0].text -->
依頼の中身のまま有効にすると、箱の中で落ちる・当たりすぎる検証がある。有効にするかは本人の環境変数の操作で決まり、ashiato2 の側に止める口は無い（宿題の一覧が読まれることに頼る）

<!-- prose impact.scope.residual[1].text -->
メモリの前提（e2e 1 本が 6g・all で 5 本のときの空き・agent の箱の cargo が 4g）は測っていない

