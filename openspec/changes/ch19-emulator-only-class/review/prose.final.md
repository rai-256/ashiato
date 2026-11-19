# CH19 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
計測テストの 1 クラスを直している間は、そのクラスだけをエミュレータで走らせて結果を数分で見られる（全体の 17 分を毎回待たない）。通ったら最後に全体を 1 回走らせる、という回し方を、エージェントも本人も同じ口（`ONLY=`）で取れるようになる。

<!-- prose impact.overview -->
計測テストの台本に `ONLY` の口が増え、1 クラスだけを走らせられるようになる。`ONLY` を渡さなければ今の全体のまま。この分析で見つけた、できなくなることは 1 つで、ふつうのクラスを絞って走らせた後は、前の全体の実行の結果の XML が残らない（St56BackupTest の経路では古い XML が残る）。注意は 2 つあり、`ONLY` が環境に export されたままだと、rc だけを見る全体の検証（verify-run を含む）が黙って 1 クラスに絞られうること（IMP-01）と、後の Story が全体に特別な手順の段を足すと、そのクラスは `ONLY` では落ちうること（IMP-02）。

<!-- prose impact.problem -->
ST56 Task 5 で、計測テストの 1 クラスを直すのに全体（1 回 17 分。gradle 自体は 20 秒で落ちていた）を 27 回まわし、implement が 485 分かかった。台本に 1 クラスだけを走らせる口が無かった。

<!-- prose uses[U1].title -->
落ちた 1 クラスを直して、そこだけ確かめる

<!-- prose uses[U1].trigger -->
Task の全体の検証で `MediaPremisesInstrumentedTest` だけが落ちた。直したが、確かめるたびに 17 分待つのは長い。

<!-- prose uses[U1].outcome -->
直す回のたびに全体を待たずに済み、完了の判定は今までどおり全体の証跡で決まる

<!-- prose uses[U1].without -->
ST56 Task 5 のように、1 クラスを直すのに全体を 27 回まわす（implement が 485 分）

<!-- prose uses[U1].steps[0].do -->
`ONLY=dev.ashiato.collector.MediaPremisesInstrumentedTest scripts/quiet-run emu1 -- tools/android-emulator.sh`

<!-- prose uses[U1].steps[0].see -->
エミュレータが立ち、そのクラスだけが走る。結果は `androidTest-results-stage1` の 1 つ

<!-- prose uses[U1].steps[1].do -->
落ちたら直して同じコマンドをもう一度

<!-- prose uses[U1].steps[1].see -->
rc≠0 と gradle の失敗。通れば rc=0

<!-- prose uses[U1].steps[2].do -->
通ったら `scripts/verify-run <項目>`（全体の `tools/android-emulator.sh`）

<!-- prose uses[U1].steps[2].see -->
全体の 8 段と入れ替えの台本が走り、証跡が残る

<!-- prose uses[U2].title -->
バックアップの試験を直す（ST56 Task 5 の場面）

<!-- prose uses[U2].trigger -->
`St56BackupTest` の復元の確認が「未送信が戻らない」で落ちた。

<!-- prose uses[U2].outcome -->
往復を含むこのクラスも、全体を待たずに繰り返せる

<!-- prose uses[U2].without -->
全体の 2〜3 段目に届くまで毎回ほかの段を待つ

<!-- prose uses[U2].steps[0].do -->
`ONLY=dev.ashiato.collector.St56BackupTest scripts/quiet-run emu56 -- tools/android-emulator.sh`

<!-- prose uses[U2].steps[0].see -->
準備 → 往復 → 確かめだけが走り、`OK (1 test)` か `error: St56BackupTest の復元の確かめが通らない`

<!-- prose walkthroughs[W1].title -->
ONLY に何を渡すと何が走るか

<!-- prose walkthroughs[W1].scene -->
エージェントが計測テストの 1 クラスを直していて、ONLY に色々なクラス名を渡す。

<!-- prose walkthroughs[W1].why -->
台本はエミュレータを起こす前には値の形だけを見て、クラスが実在するかは gradle が走った後の結果で見る。専用の台本のクラスは名前で分かるので、起こす前に止められる。

<!-- prose walkthroughs[W1].figure.caption -->
ONLY の値ごとの動き（クラス名は実在のもの、結果は仕様から）

<!-- prose walkthroughs[W1].cases[0].result -->
そのクラスだけが走り、通れば rc=0

<!-- prose walkthroughs[W1].cases[1].result -->
形は正しいので起こしてから、そのクラスの結果が無いことで rc=1（数分かかる）

<!-- prose walkthroughs[W1].cases[2].result -->
起こさずにすぐ rc=1。使うべき台本の名前が出る

<!-- prose walkthroughs[W1].exceptions[0].text -->
表の値は説明のための組み合わせ。打ち間違いの例の名前は実在しない

<!-- prose model.caption -->
計測テストの台本の分かれ道（ONLY の有無と値）

<!-- prose features[full].summary -->
`ONLY` が無いか空なら、命令も順も段の数も今と同じ。

<!-- prose points[full.1].text -->
`ONLY` を渡さない・空で渡すと、今と同じ命令を同じ順に送り、今と同じ数の段の結果を残し、最後に入れ替えの台本を走らせる。全体の経路の行は字下げも含めて書き換えない。

<!-- prose points[full.2].text -->
「前の版と同じ命令の列か」を比べる確認は、比べる元の版を明示して渡したときだけ走る。CI の普段の実行では「比べる元の版が無いので飛ばす」と出して通る。効くのはこの change の PR の証跡（Task 1.1・3.1 が元の版を渡し、飛ばしていないことを見る）。

<!-- prose features[narrow].summary -->
全体と同じ準備の後、gradle をそのクラスに絞って 1 回だけ。

<!-- prose points[narrow.1].text -->
`ONLY=dev.ashiato.collector.MediaPremisesInstrumentedTest` のように完全修飾のクラス名を 1 つ渡すと、全体と同じにエミュレータを起こし、アニメーションを切り、写真と動画を入れ、アプリを外し、止めて権限を戻してから、gradle の計測テストをそのクラスに絞って 1 回だけ走らせる。残る結果の置き場は `androidTest-results-stage1` の 1 つ。

<!-- prose points[narrow.2].text -->
クラスのテストが落ちたら rc≠0 で止まり、エミュレータを止める。`KEEP=1` を一緒に渡せば、通った後もエミュレータを止めない（全体のときと同じ）。

<!-- prose points[narrow.3].text -->
走らせる前に、前の実行の結果（`collector-android/app/build/outputs/androidTest-results`）を消す。走った後、渡したクラスの結果が 1 本も無ければ `error: ONLY=<クラス> のテストが 1 本も走っていない` で rc=1。クラス名の打ち間違いはエミュレータを起こした後のこの確認で初めて分かる（形だけは起こす前に見る）。

<!-- prose points[narrow.4].text -->
絞った実行で通っても、全体で通るとは限らない。全体の 1 段目は多くのクラスを権限を戻さずに続けて走らせるが、絞った実行は毎回まっさらな権限から始める。だから最後に全体を走らせる手順（AGENTS.md）とセットで使う。

<!-- prose features[st56].summary -->
`St56BackupTest` は gradle 1 回では通らないので、全体と同じ準備・往復・確かめをする。

<!-- prose points[st56.1].text -->
`ONLY=dev.ashiato.collector.St56BackupTest` は gradle の計測テストを呼ばず、全体の 2〜3 段目と同じ手順（アプリと試験のアプリを入れる → バックアップを有効にする → 準備 → `tools/android-backup-roundtrip.sh` で往復 → 確かめて `OK (1 test)` を照らす）だけを同じ順に走らせる。議論で決めた「1 段で」は、このクラスに限りこの手順と読む。

<!-- prose points[st56.2].text -->
この経路は段の結果の置き場を作らない（`am instrument` は XML を書かない）。判定は `OK (1 test)` の照合だけで、通らなければ全体と同じ `error: St56BackupTest の復元の確かめが通らない` で rc=1。

<!-- prose features[guard].summary -->
専用の台本で走るクラスと形の違う値は、エミュレータを起こさずに rc=1。

<!-- prose points[guard.1].text -->
入れ替えの試験と採寸の試験は gradle の 1 段では前提が無く必ず落ちるので、起こさずに `error: ONLY=` で始まる行にその台本の名前を出して rc=1 で止める。

<!-- prose points[guard.2].text -->
値は完全修飾のクラス名 1 つだけ。メソッド 1 つだけ・複数のクラスを一度に、は受け付けない（起こさずに rc=1）。

<!-- prose features[guide].summary -->
直している途中は絞り、最後に全体を verify-run で。

<!-- prose points[guide.1].text -->
AGENTS.md の「長いコマンド」に 1 項目足す: 直している途中は落ちたところだけに絞って走らせ（エミュレータは `ONLY=dev.ashiato.collector.<クラス> scripts/quiet-run emu -- tools/android-emulator.sh`、`cargo test` は名前で絞って `-- --list` で 1 本以上を確かめる）、通ったら最後に全体を `scripts/verify-run <項目>` で走らせる。

<!-- prose points[guide.2].text -->
既存の Story / change の tasks.md の検証コマンドは全体のまま書き換えない。絞った実行は Task の完了の証跡にしない（完了は全体の verify-run で決まる）。

<!-- prose impact.scenarios[S1].result -->
前の全体の結果は消え、今回の 1 クラスの結果だけが残る

<!-- prose impact.scenarios[S1].lost -->
前の全体の結果を見返すこと（全体を走らせ直せば作り直される）

<!-- prose impact.rules[R1].legit -->
入れ替えと採寸の試験を直す人は、今までどおり専用の台本を使う。1 メソッドだけ・複数クラスを一度に絞りたい人は、クラス 1 つずつ渡す

<!-- prose impact.rules[R2].legit -->
そのクラスのテストが 1 本でも走れば結果が出るので、正当な実行は止まらない（class= だけで絞り、annotation では除かない）

<!-- prose impact.rules[R3].legit -->
前の全体の実行の結果を後から読みたい人も、読めなくなる

<!-- prose impact.rules[R4].legit -->
全体のときと同じ判定で、正当な実行は止めない

<!-- prose impact.rules[R5].legit -->
今と同じ振る舞いで、新しく止めるものは無い

<!-- prose impact.findings[IMP-01].title -->
`ONLY` が export されたままだと、rc だけを見る全体の verify-run が黙って 1 クラスに絞られうる

<!-- prose impact.findings[IMP-01].text -->
`scripts/verify-run` は呼ばれた環境をそのまま検証のコマンドに渡す。`export ONLY=…` したシェル（シェルの状態が続く executor）から全体の項目を verify-run すると、台本は 1 クラスだけを走らせ、通れば PASS の証跡が残りうる。「最後に全体を走らせる」手順の意味が崩れる

<!-- prose impact.findings[IMP-02].title -->
後の Story が全体に特別な手順の段を足すと、そのクラスは `ONLY` では理由を出さずに落ちうる

<!-- prose impact.findings[IMP-02].text -->
`ONLY` の経路が特別扱いするのは St56BackupTest と専用の台本の 2 クラスだけ。後で St56 のような往復の手順を全体に足したクラスは、`ONLY` では gradle 1 回で走り、落ちうる（St56 は実物で gradle 1 回では通らなかった）。台本はその理由を出さない

<!-- prose impact.scope.covers[0].text -->
手元とエージェントが `tools/android-emulator.sh` を呼ぶ計測テストの実行

<!-- prose impact.scope.preconditions[0].text -->
`ONLY` は完全修飾のクラス名 1 つだけ。空は渡していないのと同じ

<!-- prose impact.scope.preconditions[1].text -->
St56BackupTest のときだけは gradle を呼ばず、全体と同じ往復の手順を走らせる

<!-- prose impact.scope.preconditions[2].text -->
`ONLY` にクラスを渡したときは、そのクラスだけが 1 段で走り、全体のほかの段・バックアップの往復・入れ替えの台本は走らない

<!-- prose impact.scope.excludes[0].text -->
入れ替えの試験と採寸の試験（専用の台本で走らせる）

<!-- prose impact.scope.excludes[1].text -->
CI の計測テストの job（台本を呼ばない）

<!-- prose impact.scope.residual[0].text -->
絞った実行で通っても全体で通るとは限らない（最後の全体の実行で確かめる）

<!-- prose impact.scope.residual[1].text -->
export された `ONLY` が全体の検証を絞りうる（IMP-01）

