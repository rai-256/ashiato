# CH13 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
ハーネスの作業箱（検証と agent を thread ごとのコンテナで走らせる仕組み）を、ashiato2 でいつでも有効にできるように、箱の設定ファイルを置く。中身は依頼の版を harness2 #161 の形（箱の中に自前の dockerd を持つ）に合わせて直したもの。置いただけでは何も変わらない。上流で見つけた「ホストの PATH の件」は harness2 の issue に、測っていないメモリの前提は「有効にする前の宿題」に残す。

<!-- prose impact.overview -->
設定ファイル `tools/workbox.json` が 1 つ増える。`HARNESS_WORKBOX` が off（既定）の間は誰も読まないので、検証・CI・agent の振る舞いはこの分析で見た範囲では変わらない。できなくなることはこの分析では見つけていない。有効にしたとき、今ある検証のうち Windows とエミュレータを使う台本がホストで走ることは表と全数の分類で確かめる。一方、ホストの PATH の件が直るまでは、有効にすると箱で走る検証が軒並み止まる見込みで、測っていないメモリの前提も残る（harness2 の issue と宿題の一覧）。

<!-- prose impact.problem -->
並走する Story の検証が同じ機械を取り合い、試験の漏れが共有の /tmp を埋め（10-23）、メモリ不足で langgraph が落ち（10-25）、試験の `pgrep -f "/"` が利用者の全プロセスに当たった（11-07 CH10）。作業箱で隔離するには、repo に箱の設定が要る。

<!-- prose uses[U1].title -->
並走で /tmp やメモリがまた溢れたので、箱を有効にしようとする

<!-- prose uses[U1].trigger -->
Story を 5 本並べて走らせていたら、試験の漏れで /tmp が埋まり、langgraph がまた OOM で落ちた。作業箱で検証を隔離したい。

<!-- prose uses[U1].outcome -->
有効にする前に何が直り、何を測ればよいかが 1 枚で分かる。`tests` にした後、ホストが要る台本がホストで走ることは表で確かめてある

<!-- prose uses[U1].without -->
設定ファイルが無く、有効にできない。依頼の版のまま置いていたら、読み込みのたびに「もう効かない鍵」が出るうえ、Windows とエミュレータを使う台本 3 本が箱に入って落ちた。PATH の件は有効にした日に初めて分かった

<!-- prose uses[U1].steps[0].do -->
GitHub の issue「作業箱を有効にする前の宿題（CH13）」を開く（PR 本文の先頭からも同じ一覧へ飛べる）

<!-- prose uses[U1].steps[0].see -->
「上流で見つけたこと」に PATH の件と harness2 の issue の番号、「測っていない」にメモリの 3 つ。PR 本文には pgrep の結果（`pgrep: 1` のような値）がある

<!-- prose uses[U1].steps[1].do -->
harness2 の issue（PATH の件）が閉じたかを見る

<!-- prose uses[U1].steps[1].see -->
閉じていなければ、グラフを走らせる `hx-dev.service` の `PATH` に `/usr/sbin` が無いので、`tests` にすると箱の中の dockerd が立たず、箱で走る検証が軒並み BLOCKED_INFRA になる見込み

<!-- prose uses[U1].steps[2].do -->
閉じた後で `HARNESS_WORKBOX=tests`（検証だけ）にする

<!-- prose uses[U1].steps[2].see -->
画面の e2e や DB を使う検証は箱の中の dockerd で走り、Windows やエミュレータを使う台本（`host_only` に当たるもの）はホストで走る

<!-- prose uses[U1].steps[3].do -->
`all` にする前に、宿題の 3 つを測る

<!-- prose uses[U1].steps[3].see -->
ashiato2 の側に止める口は無い。環境変数を変えた時点から箱が使われる

<!-- prose walkthroughs[W1].title -->
箱を有効にした日に、どの検証がどこで走るか

<!-- prose walkthroughs[W1].scene -->
本人が `HARNESS_WORKBOX=tests` にした後、下流が tasks の検証を走らせる。ハーネスは検証コマンドの字面を `host_only` の正規表現に当て、当たればホスト、当たらなければ箱で走らせる。

<!-- prose walkthroughs[W1].why -->
箱に入れるかは `host_only` の正規表現だけで決まり、検証コマンドの字面に当てる（中で呼ぶ台本は見ない）。docker を渡すかの判定は harness2 #161 で無くなったので、この版で直したのは `host_only` の足し算だけ。

<!-- prose walkthroughs[W1].figure.caption -->
同じ検証コマンドが、依頼の版とこの版でどこで走るか（match-cases.json の行から）

<!-- prose walkthroughs[W1].cases[0].result -->
この版は `host_only` に名前を足したので、ホストで走る

<!-- prose walkthroughs[W1].cases[1].result -->
どちらの版でも箱で走る。docker は箱の中の dockerd が受け持つ（harness2 #161）。ただし PATH の件（W2）が直っていることが前提

<!-- prose walkthroughs[W1].cases[2].result -->
`host_only` に名前が無ければ箱に入り、箱の中で落ちる。そのときは `host_only` に名前を足す

<!-- prose walkthroughs[W1].exceptions[0].text -->
表の 4 行は match-cases.json の 35 行から選んだもの。全行の当たり方は下流の Task 4.1 が harness2 の関数で確かめる

<!-- prose walkthroughs[W2].title -->
ホストの PATH に /usr/sbin が無いまま有効にすると

<!-- prose walkthroughs[W2].scene -->
本人が、グラフを走らせる `hx-dev.service` の設定を変えずに `HARNESS_WORKBOX=tests` にする。検証の準備（`tools/verify-env.sh`）は毎回 docker を呼んで DB を立てる。

<!-- prose walkthroughs[W2].why -->
harness2 の `dockerd-up` は `PATH` に足すのが自分の bin だけで、`/usr/sbin` にある iptables を見つけられない。直す場所は harness2 なので、この change は issue を 1 つ立て、宿題の一覧に置く。ashiato2 の `tools/workbox.json` の `env` では回避しない。

<!-- prose walkthroughs[W2].figure.caption -->
箱の中の dockerd が立つか（上流でこの機械で走らせた結果）

<!-- prose walkthroughs[W2].cases[0].result -->
箱はホストの PATH を受け継ぐので dockerd が立たない。準備が毎回 docker を呼ぶので、docker を使わない検証も止まる

<!-- prose walkthroughs[W2].cases[1].result -->
dockerd が立ち、準備も検証も箱の中で走る

<!-- prose walkthroughs[W2].exceptions[0].text -->
「軒並み BLOCKED_INFRA」は見込み。上流で確かめたのは箱の中の `docker images` が落ちることと、準備が docker を呼ぶことまで（2026-11-08 の走査で箱に入る検証は 1,786 本のうち 1,751 本）

<!-- prose model.caption -->
置くものと、それが効く時

<!-- prose features[place].summary -->
依頼の版を harness2 #161 の形に合わせて直した JSON を `tools/workbox.json` に置く

<!-- prose points[place.1].text -->
置く中身は依頼の版に直しを当てたもので、正本は design の D1 の JSON の塊。下流は塊を機械で取り出して書く。依頼から変えるのは 4 つの欄だけ: `docker` と `prep_files` を消す・`docker_images` に `postgres:17-alpine` を足す・`host_only` の後ろに 3 本を足す。ほかの欄（`apt` `memory` `tmp_size` `env` `parallel_verify` `agent_memory`）は依頼の版のまま（機械で確かめる）

<!-- prose points[place.2].text -->
置いただけでは何も変わらない。ハーネスがこのファイルを読むのは `HARNESS_WORKBOX` が `tests` か `all` のときだけで、既定は off。ashiato2 の台本・コード・CI もこのファイルを読まない

<!-- prose points[place.3].text -->
`docker` と `prep_files` を消すのは、harness2 #161 で効かなくなったから。検証の箱は箱の中に自前の dockerd を持ち、検証の準備も同じ箱で走る。残すと読み込みのたびに「もう効かない」と出て、効いていると読み違える。`docker_images` の `postgres:17-alpine` は、箱の中で DB を立てるたびに pull しないため（ホストで 1 回 pull・save した tar を箱に渡す）

<!-- prose points[place.4].text -->
箱に入れるかは `host_only` の正規表現を検証コマンドの字面に当てて決まる（中で呼ぶ台本は見ない）。この版は、中で `powershell.exe` やエミュレータを使う台本 3 本（`tools/release/(build|install-check).sh`・`tools/android-side-by-side.sh`・`tools/usage-volume.sh`）を足した。後の Story がホストの要る台本を新しい名前で書いたら、`host_only` に名前を足す（規約や検査は置かない）

<!-- prose features[homework].summary -->
PATH の件を harness2 の issue に、測っていないものを宿題の一覧に残す

<!-- prose points[homework.1].text -->
有効にする前の宿題の一覧（change の中の `enable-before.md`。archive の後も repo に残る）に載せるのは、上流で見つけた PATH の件と、測っていないもの 3 つ: e2e 1 本が 6g に収まるか・`all` で 5 本のときの空き・agent の箱の cargo が 4g に収まるか。箱の中の dockerd が立てる DB の容器も箱のメモリに数えられる。PR 本文の先頭に案内を置き、同じ一覧を本文にした ashiato2 の issue を下流が 1 つ立てる

<!-- prose points[homework.2].text -->
上流で見つけた: ホストの `PATH` に `/usr/sbin` が無いと、箱の中の dockerd が `iptables not found` で立たない。グラフを走らせる `hx-dev.service` の `PATH` にも無い。このまま `tests` にすると、準備が毎回 docker を呼ぶので、箱で走る検証は軒並み BLOCKED_INFRA になる見込み。直す場所は harness2 なので、下流が harness2 に issue を 1 つ立てる（AI の仮の決定）。ashiato2 の設定の `env` では回避しない

<!-- prose points[homework.3].text -->
外向きの作用は、下流の agent が GitHub に立てる issue 2 つだけ（harness2 に PATH の件・ashiato2 に宿題の一覧。同じ題があれば立てない）。agent が網に出られなければ、PR 本文に「merge の後に人間が立てる」1 行を置く形に切り替え、本人に返す

<!-- prose points[homework.4].text -->
第 1 回の深掘りで一覧に載せたもののうち、docker が渡らない・当たりすぎ・`prep_files` の不足・後の Story の漏れ・知らない鍵を黙って無視する・古いイメージが溜まる、は harness2 #157 / #161 で済んだ。一覧からは消し、案内だけを置く。第 1 回で立てる予定だった harness2 の issue 3 つは立てない

<!-- prose features[done].summary -->
中身・off の間に変わらないこと・箱の中の pgrep・鍵・当たり方・issue で閉じる

<!-- prose points[done.1].text -->
閉じる条件: 中身が D1 の塊と等しく、依頼の版から変えたのが 4 つの欄だけ・ashiato2 に読むものが無い・触ったのがこのファイルと change の文書だけ・リポジトリの検査 6 本が通る・pgrep が 10 未満・鍵が今の harness2 で読める・当たり方の表が合う・分類の一覧を書き出した・PR 本文と issue 2 つに残した。docker と作業箱のイメージが要るのは pgrep の確かめだけ

<!-- prose points[done.2].text -->
依頼の完了の判定の「箱の中で `pgrep -fc /` が 10 未満」は、この change の tasks で確かめる。`docker` を呼ばないので箱の中の dockerd は立たず、数は箱の init と pgrep だけ（上流の試しで `1`。`docker` を先に呼ぶと `4`）。`HARNESS_WORKBOX` を外したシェルからでないと成り立たない（有効の下では、このコマンド自身が検証の箱に包まれ、箱の中から箱を入れ子に起こそうとする）。結果は PR 本文に書く

<!-- prose points[done.3].text -->
鍵の確かめは、今の harness2 の `load_config` で読めて（知らない鍵があれば止まる）、もう効かない鍵（`docker` `prep_files`）を 1 つも持たないこと。`load_config` は効かない鍵を止めずに知らせるだけなので、そこは台本で落とす

<!-- prose points[done.4].text -->
当たり方は、harness2 の判定の関数 `host_only` をそのまま呼ぶ台本で、表（35 行以上）に当て、ホスト / 箱 の 2 つに分けて確かめる。正規表現を写し直さない。依頼の版に当てると、足した `host_only` の 4 行が外れて落ちることも確かめる（台本が表を見分けている証拠）。同じ台本で、今ある検証コマンドを全部ホスト / 箱に分けた一覧を書き出す（上流の走査ではホスト 35・箱 1,751）

<!-- prose features[scope].summary -->
仕様を持たない。触るのは設定ファイル 1 つと change の中の文書だけ

<!-- prose points[scope.1].text -->
仕様を持たない change（`skip_specs: true`）。off の間に外から確かめられる振る舞いが無いので、正典の仕様・`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`・CI を変えない。change の中に置くのは宿題の一覧・当たり方の表・確かめの台本・分類の一覧で、どれも archive で change と一緒に移る

<!-- prose points[scope.2].text -->
後の Story に箱の制約を知らせる規約や検査は、この change では置かない。docker の字面の判定は harness2 #161 で無くなり、残る字面の判定は `host_only` だけ。それは宿題の一覧に 1 行置く

<!-- prose impact.scope.covers[0].text -->
repo に箱の設定ファイルを置くこと・今ある検証コマンドの当たり方を確かめること・PATH の件と宿題を issue と一覧に残すこと

<!-- prose impact.scope.preconditions[0].text -->
`HARNESS_WORKBOX` が off（既定）のときに限って、振る舞いが変わらない

<!-- prose impact.scope.preconditions[1].text -->
鍵と当たり方の確かめは、harness2 の main の #161 以降の版（`d65ff3b4a`）に対して

<!-- prose impact.scope.excludes[0].text -->
箱を有効にすることと、箱の中で検証が通るかを確かめること（pgrep の確かめを除く）

<!-- prose impact.scope.excludes[1].text -->
後の Story への規約・検査

<!-- prose impact.scope.residual[0].text -->
PATH の件は harness2 で直るまで残る。有効にするかは本人の環境変数の操作で決まり、ashiato2 の側に止める口は無い

<!-- prose impact.scope.residual[1].text -->
ホストが要る台本を新しい名前で書くと箱に入る（`host_only` は字面の判定）

<!-- prose impact.scope.residual[2].text -->
メモリの前提（e2e 1 本が 6g・all で 5 本のときの空き・agent の箱の cargo が 4g）は測っていない

