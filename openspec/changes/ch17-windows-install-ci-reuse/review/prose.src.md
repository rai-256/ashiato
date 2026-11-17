# CH17 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
試験の台本と文書だけを直した PR を push したとき、CI が Windows の配布物を組み直さず、組み立ての入力が同じ前の配布物でインストールと更新の試験だけを走らせる。組み直すときも Rust の依存をキャッシュから戻す。台本を直して CI を待つ 1 周が、約 1 時間から試験の分（約 17 分と少し）に縮む。

<!-- prose impact.overview -->
試験の台本（3 つ）と文書だけが違う PR の push では、CI が Windows の配布物を組まずに、組み立ての鍵が同じ前の配布物を CI のキャッシュから引いて上げ、インストールと更新の試験は今と同じく全部走らせる。組むときも Rust の依存をキャッシュから戻す。効くのは PR の run で、同じ鍵の保存物がキャッシュに残っているときだけで、main への push と Android だけの変更などでは今と同じく組む。できなくなるのは、使い回した run で release-package の 4 つの配布物が同じ commit を指すことで、Windows の 2 つは組んだ前の commit を指す。もう 1 つ、使い回した run ではインストーラの文字化けの試験（ch14_）がその push では走らず、同じ鍵の前の run の結果に頼る。

<!-- prose impact.problem -->
ST58 の関門で、CI の 1 周が約 1 時間（Windows の配布物の組み立て 33 分・インストールの試験 17 分）かかり、13 時間で 12 周した。最後の 3 周は試験の台本しか変えていないのに、毎回 Windows の配布物を組み直していた。

<!-- prose uses[U1].title -->
Windows のインストールの試験の台本を直して、すぐ結果を見る

<!-- prose uses[U1].trigger -->
関門でインストールの試験が落ち、直すのは試験の台本（install-check.sh）の 1 行だけ。今は push のたびに配布物を組み直すので、結果まで約 1 時間待つ。

<!-- prose uses[U1].outcome -->
直した台本で試験が通るかが、組み立ての約 33 分を待たずに分かる。13 時間で 12 周した ST58 のような関門で、台本だけの周が速く回る。

<!-- prose uses[U1].without -->
台本の 1 行の変更でも毎回 Windows の配布物を組み直し、1 周に約 1 時間かかる。

<!-- prose uses[U1].steps[0].do -->
台本の 1 行を直して PR に push する

<!-- prose uses[U1].steps[0].see -->
CI の Windows の配布物の job で、鍵を出す step の後の組み立ての step が skipped になり、2 つの成果物が今と同じ名前で上がる

<!-- prose uses[U1].steps[1].do -->
インストールの試験の job の結果を見る

<!-- prose uses[U1].steps[1].see -->
配る形と試験の形の 2 つでインストールと更新の Scenario が全部走り、欠けがあれば落ちている

<!-- prose walkthroughs[W1].title -->
何を変えた push なら組み直さないか

<!-- prose walkthroughs[W1].scene -->
本人が同じ PR に、変えるファイルの違う push を順に重ねる。1 回目の push で Windows の配布物が組まれ、鍵 K1 で保存されている。

<!-- prose walkthroughs[W1].why -->
鍵は「名指しして除くもの以外の全部」から作る。数え漏らした入力が変わっても使い回して古い配布物で試験が通ることを避けるため、知らない変更は組み直す側に倒す。

<!-- prose walkthroughs[W1].figure.caption -->
2 回目の push で変えたもの → Windows の配布物の job がすること（K1・K2 は説明用の名前）

<!-- prose walkthroughs[W1].cases[0].result -->
鍵が K1 のままなので組まずに引き、インストールの試験は全部走る

<!-- prose walkthroughs[W1].cases[1].result -->
鍵が変わるので組み直す。壊れていればこの PR の CI で落ちる

<!-- prose walkthroughs[W1].cases[2].result -->
Windows の配布物に入らない変更でも鍵が変わり、約 33 分の組み立てが走る（D1 の反転条件で後から除ける）

<!-- prose walkthroughs[W1].exceptions[0].text -->
どの行も説明用で、実際の run で確かめた時間ではない。短縮の実測は完了の判定 3・4

<!-- prose walkthroughs[W2].title -->
使い回した run の release-package の commit

<!-- prose walkthroughs[W2].scene -->
本人が PR に 2 回 push した（1 回目の head が A、2 回目の head が B。2 回目は試験の台本だけ）。2 回目の run の release-package で、下書きのリリースの配布物を確かめる。

<!-- prose walkthroughs[W2].why -->
深掘り Q2 で本人が、使い回した配布物は組んだ commit を指すとした。配布物を集める job は変えないので、Linux と APK は head、Windows は組んだ commit になる。

<!-- prose walkthroughs[W2].figure.caption -->
2 回目の run（head B）の release-package の中身が指す commit

<!-- prose walkthroughs[W2].cases[0].result -->
Windows の 2 つは SOURCE.txt も印も A で揃っているので通る

<!-- prose walkthroughs[W2].cases[1].result -->
Windows の 2 つは A で head_sha の B と違い、今の手順のままだと食い違いに見える。手順に使い回した run の見方を足す（tasks 3.2）

<!-- prose walkthroughs[W2].exceptions[0].text -->
A を空の clone から取れるかは確かめていない（完了の判定 4）

<!-- prose model.caption -->
Windows の配布物の job が、鍵で「組む」か「前のものを使う」かを分ける

<!-- prose features[reuse].summary -->
鍵が同じ前の配布物が CI のキャッシュにあれば、組まずにそれを今と同じ名前で上げる

<!-- prose points[reuse.1].text -->
PR の run で、鍵が空でなく、同じ鍵の前の配布物（配る形と試験の形の 2 つ）が CI のキャッシュにあれば、組み立ての手順を飛ばし、引いたものを書き換えずに今と同じ成果物の名前で上げる。

<!-- prose points[reuse.2].text -->
引けなかったときは今と同じ手順で組み、組み立てが成功したときだけ同じ鍵で保存する。保存と引きの置き場は、job が上げる置き場の全部（一部だけを使い回さない）。

<!-- prose points[reuse.3].text -->
main への push と、鍵を出せなかった run は使い回さずに組む。鍵の一部だけが一致する前の組み立ても使わない（前方一致で古いものを引かない）。

<!-- prose points[reuse.4].text -->
使い回したときは、インストーラの文字化けの試験（ch14_）も飛ばす。その試験の入力は鍵に入っていて、同じ鍵の前の run で通ってから保存されている。

<!-- prose features[key].summary -->
名指しして除くもの以外の全部と、走っている CI の定義と道具の版から作る

<!-- prose points[key.1].text -->
鍵は、追跡している全部のファイルの中身（git の blob の id）から、試験の台本 3 つ（install-check.sh・update-check-install.sh・conpty-run.ps1）と docs/・openspec/・*.md を除いたものに、走っている run の CI の定義と道具の版（Rust・Node・runner の image の名前と版）を足して作る。

<!-- prose points[key.2].text -->
名指ししていないファイルが 1 つでも変わる・増えると鍵が変わり、組み直す側に倒れる。鍵を出せない木では鍵を出さずに落ち、CI は組む側に回る。

<!-- prose points[key.3].text -->
Android の収集アプリだけ・Linux の道具だけの変更でも、Windows の配布物は組み直す（除くものは試験の台本と文書だけ）。runner の image の版が上がる週ごとにも鍵が変わり、その後の最初の run は組み直す。

<!-- prose points[key.4].text -->
走っている ST58 が足す試験の台本 update-apply-install.sh は、除くものに先回りして足さない。その台本だけを変えた PR は組み直す（足すかは ST58 の後に決める）。

<!-- prose features[tests].summary -->
使い回したかどうかに依らず、インストールと更新の Scenario を全部走らせて突き合わせる

<!-- prose points[tests.1].text -->
インストールの試験の job は変えない。配る形と試験の形の 2 つの配布物でインストールと更新の試験を走らせ、走った Scenario を台本の --list の全部と突き合わせ、1 つでも欠ければ CI を落とす。

<!-- prose points[tests.2].text -->
配る形を壊す変更（コード・依存・組み立ての台本・CI の定義など）は鍵を変えるので、その PR の CI で組み直され、組めなければ落ちる。文書だけの PR は今と同じく組み立てを飛ばす。

<!-- prose features[source].summary -->
使い回した Windows の配布物は、それを組んだ commit を指したまま上げる

<!-- prose points[source.1].text -->
使い回した Windows の配布物の SOURCE.txt と組み立ての印は書き換えず、どちらもそれを組んだ commit（同じ PR の前の push の head か main の commit。ほかの PR の組み立ては使わない）を指す。配布物の検査は配布物ごとに SOURCE.txt と印の一致を見るので通る。

<!-- prose points[source.2].text -->
使い回した run の release-package（下書きのリリースの材料）では、Linux の tar と携帯の APK は head の commit、Windows の 2 つの zip は前の commit を指す。1 つの run の中で配布物の commit が揃わない。

<!-- prose points[source.3].text -->
SOURCE.txt が指す前の push の head を、枝を push し直した後に空の clone から取れるかは確かめていない。完了の判定 4 で確かめ、取れなければ結果を記録して人間に返す。PR の配布物は公開しない。

<!-- prose points[source.4].text -->
配布物の検査は期待する版番号と LICENSE を head から取る。Cargo.toml と LICENSE が鍵に入っているので、使い回すのはその 2 つが head と同じときだけで、期待値と使い回した配布物の中身が合う。

<!-- prose points[source.5].text -->
下書きのリリースで配布物を手で確かめる手順（docs/release.md §11.1）に、使い回した run では Windows の 2 つについて SOURCE.txt と印が同じ commit を指すことを見る、を足す。下書きを作る台本は変えない。

<!-- prose points[source.6].text -->
正式版・ベータ版・開発版の配布物は使い回さない（それぞれの workflow が自分で組む）。SOURCE.txt が自分を組んだ commit や印を指すのは今のまま。

<!-- prose features[cache].summary -->
組むときは Rust の依存を戻す。置き場は repo あたり 10 GB の Actions のキャッシュ

<!-- prose points[cache.1].text -->
組むとき（前の配布物を引けなかったとき）だけ、~/.cargo の依存と target を、Rust の版と Cargo.lock の hash の鍵で戻す。Rust の版が同じなら前の鍵からも戻し、cargo が変わった所だけ組み直す。

<!-- prose points[cache.2].text -->
置き場は GitHub Actions のキャッシュ（repo あたり 10 GB、7 日使われないと消える）。配布物 2 つと Rust のビルドで使う。溢れたら古いものから消え、組み直すだけになる（遅くなるが、誤った配布物は使わない）。

<!-- prose features[keep].summary -->
触るのは Windows の配布物の job の step と、鍵の台本と、文書だけ

<!-- prose points[keep.1].text -->
変えないもの: job の出し分け・Windows の配布物の job の if と checkout する commit・文字化けの試験が最初の組み立てより前にあること・成果物の名前・インストールの試験の job・配布物を集めて検査する job・他の job・製品のコード・配布物の作り方。

<!-- prose points[keep.2].text -->
改める 2 つの要件の既存の Scenario は、名前も中身も変えない（使い回しの後も成り立つ）。印を持つ台本にも触れない。

<!-- prose impact.rules[R1].legit -->
正当な利用は止めない。使い回せないときは今と同じに組むので、時間が掛かるだけ

<!-- prose impact.rules[R2].legit -->
今と同じ判定で、使い回したかどうかで分けない

<!-- prose impact.rules[R3].legit -->
今と同じ。配る形を壊す変更は鍵を変えるので、使い回しで落ちるべきものが通ることはこの分析では見つけていない

<!-- prose impact.rules[R4].legit -->
使い回した Windows の配布物は SOURCE.txt と印が組んだ commit で揃い、Cargo.toml と LICENSE が鍵にある間は期待値とも合うので、この規則では止まらない。使い回した run はまだ走っていないので、完了の判定 3・4 の run で確かめる

<!-- prose impact.findings[IMP-01].title -->
使い回した配布物の SOURCE.txt の commit を後から取れるか確かめていない

<!-- prose impact.findings[IMP-01].text -->
使い回した Windows の配布物の SOURCE.txt は、同じ PR の前の push の head を指す。枝を push し直した後に、その commit を空の clone から取れるかは確かめていない。取れなければ、SOURCE.txt が取れない commit を指す。

<!-- prose impact.findings[IMP-02].title -->
上げる置き場を後から足した change の合わせ忘れを、CI の中では見ない

<!-- prose impact.findings[IMP-02].text -->
「上げる配布物は全部、保存と引きの置き場に入る」を確かめる --check-workflow は、この change の tasks の検証としてだけ走る。CI の chain の job にも Windows の runner にも入らず、merge gate も別の change に対して走らせる口を持たない。後から package-windows の上げる置き場を足す・名前を変える change は、その change の tasks が書かない限り、誰もこの検査を走らせない。走っている ST58 の枝の ci.yml は、試験の形を 1 つの step の中のループで 6 つ組み、package-windows-test の名前を無くして 6 つの名前で上げる。保存と引きに足し忘れると、2 回目の push から、使い回した run で欠けた置き場が空のまま上がり、インストールの試験の job が配布物を見つけられずに落ちる見込み（原因が見えにくい）。

<!-- prose impact.findings[IMP-03].title -->
下書きの説明文の run 番号は、使い回した Windows の配布物の組み立ての run ではない

<!-- prose impact.findings[IMP-03].text -->
draft.sh の下書きの説明文は「配布物は PR の package の組み立て（run <N>）」と書く。使い回した run では、Windows の 2 つは別の run の組み立て。§11.1 の手順は tasks 3.2 で直すが、draft.sh は変えないので説明文はそのまま。

<!-- prose impact.findings[IMP-04].title -->
キャッシュの置き場 10 GB を Windows の配布物と Rust のビルドで使い、ほかの保存物を押し出すかもしれない

<!-- prose impact.findings[IMP-04].text -->
Windows の配布物 2 つ（PostgreSQL を含む）と Rust の target を、組み直すたびに新しい鍵で保存する。PR ごと・Cargo.lock ごとに積み、10 GB を超えると古いものから消える。PostgreSQL の zip のキャッシュが消えると、外の配布元から取り直す。使い回したい保存物が先に消えると、短縮が効かない。 Rust のキャッシュは組んだ PR の run ごとにその PR の置き場へ保存され、PR の置き場はほかの PR から読めない。並走する PR の保存物が古い順の押し出しで、全 PR が引ける main の保存物を先に押し出す向きがある。

<!-- prose impact.findings[IMP-05].title -->
使い回した run では、文字化けの試験の結果を前の run に頼る

<!-- prose impact.findings[IMP-05].text -->
使い回した run は ch14_ の試験を飛ばす。入力は鍵に入るが、結果が runner の環境（コンソールのコードページなど）で揺れるなら、前の run で通ったことはこの run で通ることを言わない。鍵に runner の image の版が入るので、引ける前の run は同じ版の image で走ったものに限られる。

<!-- prose impact.scope.covers[0].text -->
コードを触った PR の CI の、Windows の配布物の job（使い回しと Rust のキャッシュ）。インストールの試験の job は今のまま

<!-- prose impact.scope.preconditions[0].text -->
使い回すのは PR の run で、鍵が空でなく、同じ鍵の保存物が CI のキャッシュに残っているときだけ（7 日・10 GB）

<!-- prose impact.scope.preconditions[1].text -->
コードを触った PR だけが組み立ての job を走らせる。文書だけの PR は今と同じく job ごと飛ぶ

<!-- prose impact.scope.preconditions[2].text -->
配布物の検査の期待値と使い回した配布物が合うのは、Cargo.toml と LICENSE が鍵に入っている間だけ

<!-- prose impact.scope.excludes[0].text -->
main への push（毎回組んで保存する）

<!-- prose impact.scope.excludes[1].text -->
正式版・ベータ版・開発版の組み立て（それぞれの workflow が自分で組む）

<!-- prose impact.scope.excludes[2].text -->
Android だけ・Linux の道具だけの変更での使い回し

<!-- prose impact.scope.residual[0].text -->
使い回した run の release-package で、Windows の 2 つが前の commit を指す（IMP-01・IMP-03）

<!-- prose impact.scope.residual[1].text -->
後から上げる置き場を足した change の合わせ忘れを CI の中で見ない（IMP-02）

<!-- prose impact.scope.residual[2].text -->
キャッシュの置き場の押し出し（IMP-04）

<!-- prose impact.scope.residual[3].text -->
使い回した run で ch14_ の結果を前の run に頼る（IMP-05）

