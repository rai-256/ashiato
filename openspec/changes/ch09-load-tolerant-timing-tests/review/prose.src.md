# CH09 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
マシンが混んでいる（並走する Story の下流が重なる）ときに、変更と関係のない既存の試験が「〜秒以内」で落ちて関門が止まることを無くす。試験が見ている性質（見回りが履歴の取得を待たない・間隔の変更が再起動なしで効く）は弱めず、実時間の長さに依らない確かめ方に替える。製品のコード・画面・API・移行は変わらない。

<!-- prose impact.overview -->
同じ形の試験を、関門で走る 3 か所（Rust・画面の単体試験・携帯の単体試験）で、実時間の長さに依らない形に替える。マシンが混んでいても、これらの試験が「〜秒以内」で落ちることは減る。代わりに、試験は見回りの速さと、間隔の変更が効くまでの数秒の遅れを確かめなくなる。本当に壊れた試験は落ちるまでが延びる。直したことを示す負荷の台本は、走らせている間ほかの Story の関門も遅くする。

<!-- prose impact.problem -->
並走する Story の関門（cargo test --workspace）が、その Story と関係のない既存の試験で止まる。2026-11-04 の ST55 では 2 度。落ちたのは load average 9〜14 のときで、同じコードを取り直すと通った。

<!-- prose uses[U1].title -->
並走の多い日に、関門が無関係な試験で止まらない

<!-- prose uses[U1].trigger -->
ST55 の Task 11 の関門（cargo test --workspace）が、ST55 と関係のない見回りの試験で「見回り 1 回が 1.335917022s かかった」と落ちた。load average は 14。同じコードを取り直すと通る。

<!-- prose uses[U1].outcome -->
関門が止まったときは、変更に関係する試験が落ちたと考えてよくなる。直さなかった試験は一覧で分かる

<!-- prose uses[U1].without -->
証跡の最新が FAIL になり、診断の席か人間が取り直す。並走が続く限り、別の Story の関門が同じ理由で止まり続ける

<!-- prose uses[U1].steps[0].do -->
CH09 が main に入った後、関門の Story を main に rebase して cargo test --workspace を走らせる（並走で load average 9〜14）

<!-- prose uses[U1].steps[0].see -->
見回りの試験は、止めた読みの間に見回りが返ったことを数で見るので、混み具合で落ちない。走査の間隔の試験は、取り込み器が遅れた歩を進め直して 10 回を数える

<!-- prose uses[U1].steps[1].do -->
PR 本文の洗い出しの一覧を読む

<!-- prose uses[U1].steps[1].see -->
同じ形の試験の場所・どの形に直したか・直さなかったものとその理由（計算の形を時間で見る stay_tests.rs など）

<!-- prose walkthroughs[W1].title -->
見回りの試験: 混んでいても落ちず、待つ実装なら必ず落ちる

<!-- prose walkthroughs[W1].scene -->
PC の収集アプリが 30,000 件の訪問を持つブラウザの履歴を読んでいる間も、前景の見回り（1 秒ごと）が止まらないことを確かめる。

<!-- prose walkthroughs[W1].why -->
落ちていたのは「待った」からではなく、見回りの側が積む 2,000 件の手間が混み具合で延びたため。待つかどうかは、読みを止めた間に見回りが返るかで直接分かる。

<!-- prose walkthroughs[W1].figure.caption -->
同じ試験を、混み具合と実装の正しさで分けたときの結果（秒数は説明用の値）

<!-- prose walkthroughs[W1].cases[0].result -->
読みは試験が放すまで止まっている。その間に見回りが 5 回以上返り、前景の切り替えが記録に積まれる。放した後は見回り 1 回で積まれる記録が 2,000 件以下

<!-- prose walkthroughs[W1].cases[1].result -->
最初の見回りが返らない。30 秒後に見張りが読みを放し、試験は「見張りが読みを放した」で落ちる

<!-- prose walkthroughs[W1].exceptions[0].text -->
見回り 1 回が遅くなる（速さの）退行は捕まえなくなる。速さは要件でない

<!-- prose walkthroughs[W2].title -->
走査の間隔の試験: 取り込み器が遅れた歩を進め直す

<!-- prose walkthroughs[W2].scene -->
取り込み器が 120 秒の間隔で動いているところで、登録簿の間隔を 1 秒に変え、止めた時計を 1,001ms ずつ進めて走査が 10 回入ることを確かめる。

<!-- prose walkthroughs[W2].why -->
上限を延ばすだけでは、時計を進めた歩が眠りの前に過ぎる抜けが残る（往復 40ms の模型で、上限 5 秒でも 30 秒でも同じ所で止まった）。進め直しは時計を 1 秒ずつ使うだけで、古い間隔の退行は捕まえたまま。

<!-- prose walkthroughs[W2].figure.caption -->
混んでいる回の 1 歩（ms は説明用の値）

<!-- prose walkthroughs[W2].cases[0].result -->
抜けた歩を進め直すので、10 回は時計の 10〜20 秒で入る。30 秒未満で通る

<!-- prose walkthroughs[W2].cases[1].result -->
10 回には時計で 1,200 秒かかる。時計の経過 30 秒か実時間 30 秒で「待った時間」「最後に見た数」を出して落ちる

<!-- prose walkthroughs[W2].cases[2].result -->
10 回に 39 秒以上かかり、30 秒未満を満たさず落ちる

<!-- prose walkthroughs[W2].exceptions[0].text -->
説明用の値。40ms の往復は模型の値で、実際の往復の長さは計っていない

<!-- prose model.caption -->
試験の確かめ方を 3 つの型に替え、2 つの台本で直ったことを示す

<!-- prose features[watch].summary -->
見回りの試験 2 本は、相手を止めておき、その間に見回りが返るかで判定する。返らなければ 30 秒の見張りが落とす

<!-- prose points[watch.1].text -->
1 の試験は、偽の履歴の読み手を試験が放すまで止める。その間に見回りを 5 回以上回し、前景を切り替え、切り替えが記録に積まれていることを見る。放した後は、見回り 1 回で積まれた記録が 2,000 件以下であることを件数で見る。積み込みを見回りに分けている・積みこぼさない・積み込みの間に眠りが入らない、の確かめは残す

<!-- prose points[watch.2].text -->
同じ形の clock_worker_slow_read_does_not_hold_the_patrol（基準の読み取りが長引いても前景の切り替えが残る）も、「30 秒未満」の比較を同じ見張りに替える。正典の文は時間を言っていないので、変えた理由を agreed-test-changes.md に書く

<!-- prose points[watch.3].text -->
止めた読みでは、見回りが帳面の書き込み（Saving）を待つ退行は捕まらない。止める口を製品のファイルに足さない決め（Q0）のため。見回り 1 回が遅くなる（速さの）退行も捕まえなくなる

<!-- prose features[wait].summary -->
回数 × 短い眠りの待ちを、実時間の期限に替える。普段の所要時間は変わらない

<!-- prose points[wait.1].text -->
サーバ・PC の収集アプリ・携帯の収集アプリの試験の、相手が進むのを待つ箇所は、条件が満たされた時点で進み、満たされなければ実時間 30 秒で落ちる。落ちたときは待った時間と最後に見た値を出す

<!-- prose points[wait.2].text -->
画面の単体試験は、数字の書かれていない待ち（waitFor / findBy）の上限を 10 秒、試験 1 本の上限を 30 秒にする。設定 1 か所で変える

<!-- prose points[wait.3].text -->
走査の間隔の試験は、上限を延ばすだけでは直らない。抜けた歩を進め直し、10 回の時計の経過を「10 秒ちょうど」でなく「30 秒未満」で見る。正典の Scenario の文もそれに合わせて書き直す。古い間隔のまま眠る退行と 30 秒ごとにしか読み直さない退行は捕まるが、数秒の遅れは捕まえない

<!-- prose points[wait.4].text -->
試験の中の時刻を兼ねるループ（1 回ごとに試験の時刻を進めるもの）は回数のまま残す。期限に変えると時刻が 1 時間で効く規則をまたぐ。残す行には timing-ok の印と理由を書く

<!-- prose points[wait.5].text -->
相手が進んだと仮定して決まった時間だけ眠る箇所は、観測できる条件があれば条件の待ちに替え、無ければ残して一覧に載せる。走査の間隔の試験の settle(500)・settle(25)・advance_secs(600, 25) は観測の口が無いので残る

<!-- prose points[wait.6].text -->
本当に壊れた試験は、落ちるまでが延びる（待ち 1 か所ごとに最大 30 秒、画面は 10 秒）

<!-- prose features[timeout].summary -->
打ち切りは相手と上限を一緒に延ばし、計算の形は直さず一覧に載せる

<!-- prose points[timeout.1].text -->
試験の DB の打ち切り（上限 1 秒）は、問い合わせを pg_sleep(60) にし、30 秒を待たずに SQLSTATE 57014 で失敗することを見る。正典 test-database の Scenario を書き直す。時刻合わせの子プロセスの打ち切り（200ms）も sleep 60・30 秒未満にする。打ち切りが効けば所要時間は変わらない

<!-- prose points[timeout.2].text -->
1 日の作り直しが 3 秒未満なら「隣の日をたどっていない」と見る stay_tests.rs は直さない（数で見る口が製品に無い）。「起きない」を 200ms の窓で見る携帯の試験と、経過の下限を見るログインの試験も直さない（混むと見逃す側へ弱まるだけで、落ちはしない）

<!-- prose features[proof].summary -->
負荷の下で続けて通ることと、確かめたい性質を壊すと落ちることを、書いたまま走る台本で確かめる

<!-- prose points[proof.1].text -->
tools/load-repeat.sh は全コアに busy loop の負荷を（手元か DB のコンテナの中に）かけ、命令を決まった回数続けて走らせる。直す前の 2 本の試験で落ちる回が出ることを load-before.txt に、直した後に 10 回ずつ通ることを load-after.txt に残す。負荷のための道具は入れない

<!-- prose points[proof.2].text -->
tools/break-check.sh は使い捨ての作業木に壊す差分を当てて命令を走らせ、決まった文言で落ちることを見る。元の作業木は書き換えず、終わるときに使い捨ての作業木を消す。見回りが読みを待つ差分と、取り込み器が変えた間隔を読み直さない差分の 2 つを change の中に置く

<!-- prose points[proof.3].text -->
負荷の台本は、並走する Story と同じ機械の全コアを埋める。DB のコンテナの中にかけても、CPU は同じ機械のもの。走らせている間は、ほかの Story の関門も遅くなる（IMP-01）

<!-- prose features[guard].summary -->
差分を試験の側に限り、既存の試験の名前・印・本数を守ることを機械で見る

<!-- prose points[guard.1].text -->
製品のファイルの、試験のモジュール（#[cfg(test)] mod tests）の外は 1 行も変えない。試験用の差し込み口も足さない。分岐点からの差分を数える検査が見る

<!-- prose points[guard.2].text -->
直す途中で製品が本当に待っている・遅すぎると分かっても、直さない。その試験は直さず一覧に「製品の不具合の疑い」と根拠を書き、PR 本文で別の change に出す（登録は人間が hx change で行う）

<!-- prose points[guard.3].text -->
既存の試験の名前・Scenario の印は消さず、本数は分岐点より減らさない。1 の試験と 2 の試験の名前も変えない

<!-- prose points[guard.4].text -->
洗い出す範囲は、関門で走る 3 か所（crates/ の Rust・web/src/__tests__・collector-android/app/src/test）。Windows でだけ組まれる試験は探して一覧に載せるだけ。画面の e2e・携帯の計測テスト・tools の台本は探さない。一覧（場所・型・方針・理由）は inventory.md に書き、候補を持つファイルが全部載っていることを検査が見て、PR 本文に写す

<!-- prose impact.scenarios[S1].result -->
Task 3.1・3.2 の検証が落ち、試験の土台の呼び方を合わせる

<!-- prose impact.scenarios[S1].lost -->
検証のやり直しの手間だけ。製品と記録には効かない

<!-- prose impact.scenarios[S2].result -->
ほかの Story の関門が遅くなる。CH09 が入る前の木では、同じ見回りの試験・走査の間隔の試験が落ちうる

<!-- prose impact.scenarios[S2].lost -->
ほかの Story の証跡が FAIL になり、取り直しの手間が増える

<!-- prose impact.rules[R1].legit -->
この分析では、正当な実装を落とす場面を見つけていない。見回り 1 回は混んだときでも 1〜2 秒（tools/tmp-leak-check.sh の実測）で、30 秒より 1 桁以上短い

<!-- prose impact.rules[R2].legit -->
負荷がもっと重く、正しい実装が 30 秒を超えると落ちる。load 9〜14 で落ちた実測（見回り 1.34 秒・5 秒の待ちで走査が届かない）からは、この分析では届かないと見ているが、走らせて確かめていない

<!-- prose impact.rules[R3].legit -->
CH09 は上限の値を変えない。試験の問い合わせの長さを変えるだけで、この分析では正当な利用を止める場面を見つけていない

<!-- prose impact.rules[R4].legit -->
並走の st54 が試験の土台を変えた後は、CH09 の試験が rebase まで組めず、break-check が落ちうる（IMP-03）

<!-- prose impact.rules[R5].legit -->
製品の不具合が見つかっても、CH09 の差分では製品のコードを変えない（Q7 で決めたとおり。別の change に出す）

<!-- prose impact.rules[R6].legit -->
CH09 はこの規則に触れない。Scenario の文の 1 本だけを書き直す

<!-- prose impact.findings[IMP-01].title -->
負荷の台本が、並走する Story の関門を遅くし、落としうる

<!-- prose impact.findings[IMP-01].text -->
load-repeat.sh は全コアを busy loop で埋める。DB のコンテナの中にかけても CPU は同じ機械のもの。負荷は Task 1.1 の self-test・1.2（直す前）・9.3（直した後）で走り、9.3 は検証の命令なので gate や fix のたびに走り直す。その間、ほかの Story の関門も遅くなり、CH09 が入る前の木では同じ試験が混み具合で落ちうる。CH09 が止めようとしている止まり方を、CH09 自身が起こす

<!-- prose impact.findings[IMP-02].title -->
負荷の台本が KILL で殺されると、全コアの負荷が残りうる

<!-- prose impact.findings[IMP-02].text -->
仕様は EXIT・INT・TERM で負荷を止めると決めているが、KILL では trap が走らない。HUP も trap の対象に無い。手元の busy loop は親を失っても回り続け、DB のコンテナの中の busy loop はコンテナが止まるまで回る。印 ch09-load-<pid> は DB のコンテナの中の過程にしか付かないので、手元に残った負荷を印で探すことはできない。残ると全コアが埋まったままになり、すべての Story の関門が遅くなる

<!-- prose impact.findings[IMP-03].title -->
並走の st54 が試験の土台を変えると、CH09 の 1 の試験を rebase で直す

<!-- prose impact.findings[IMP-03].text -->
st54 は HistoryCollector::new に除外の登録の置き場を引数で渡す形に変え、runtime.rs の mod tests の cfg()・試験の土台と history/collect.rs:844 の試験の土台も合わせる。CH09 が書き直す 1 の試験も同じ mod tests の土台を使うので、後から入る側が rebase で呼び方を直す。壊す差分（Stage::Reading の周り）は st54 の変える行と別の場所で、当たり方への影響はこの分析では見つけていない

<!-- prose impact.findings[IMP-04].title -->
Windows でだけ組まれる試験は CI で走る（設計の前提と違う）

<!-- prose impact.findings[IMP-04].text -->
design D3 は tests/runtime_windows.rs を「関門でも CI でも走らない」とし、直さず一覧に載せるだけにした。実際は CI の collector-windows-runtime が、Windows を変えた PR で走らせる。中の「10 秒以内」などの待ちは残る。CI は関門ではないので関門が止まる問題には効かないが、CI の runner が混むか・その待ちで落ちた記録があるかは確かめていない

<!-- prose impact.scope.covers[0].text -->
関門で走る試験（cargo test --workspace・画面の単体試験・携帯の単体試験）の、実時間の長さで判定する箇所

<!-- prose impact.scope.preconditions[0].text -->
落ちる原因が混み具合（実時間の長さ）であること。製品の不具合なら直さず一覧に載せる

<!-- prose impact.scope.preconditions[1].text -->
条件の待ちの期限 30 秒は、試験の中の時刻を兼ねるループを除く

<!-- prose impact.scope.preconditions[2].text -->
見回りの試験の見張りは、見回りが返らないときだけ働く。見回り 1 回の実時間は見ない

<!-- prose impact.scope.preconditions[3].text -->
負荷の台本は INT・TERM で止めたときに負荷を消す。break-check は元の作業木を書き換えない

<!-- prose impact.scope.preconditions[4].text -->
試験の DB の上限の既定 60 秒と環境変数の扱いは変えない。取り込み器の間隔の規則（300 秒・環境変数を断る）も変えない

<!-- prose impact.scope.preconditions[5].text -->
差分は試験のファイル・試験のモジュール・web/vitest.config.ts・tools/・docs/・openspec/ に限る

<!-- prose impact.scope.excludes[0].text -->
画面の e2e・携帯の計測テスト・tools の台本・Windows でだけ組まれる試験の直し

<!-- prose impact.scope.excludes[1].text -->
計算の形・「起きない」の窓・下限を時間で見る試験の直し

<!-- prose impact.scope.residual[0].text -->
見回りが帳面の書き込みを待つ退行と、見回りの速さの退行は試験で捕まらない

<!-- prose impact.scope.residual[1].text -->
負荷の台本が並走の Story を遅くする・KILL で負荷が残りうる（IMP-01・IMP-02）

