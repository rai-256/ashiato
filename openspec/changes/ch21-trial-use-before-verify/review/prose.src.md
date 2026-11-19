# CH21 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
確認バッチの手順書が本人に届く前に、Claude がまっさらな Windows Sandbox に配る形（サーバと PC の収集アプリの zip）を入れ、筋書きなしで普通に使ってみる。本人は手順書の冒頭で、気になったことの件数と題を先に読み、詳しくは手元の報告で写しと根拠を見てから、残すものだけを /iwakan に貼れる。

<!-- prose impact.overview -->
確認バッチの準備の最後に、Claude がまっさらな Windows Sandbox に配る形を入れて最大 45 分使い、気になったことを根拠つきで手元の報告に残し、手順書の冒頭に件数と題を載せる。走るのは、手元に Windows Sandbox と Claude がある機械で、配る形の出所が前の回から変わったか、前の回が使い終えていない（警告で終わった）確認バッチだけ。できなくなることとして、席が走る回は手順書が届くのが最大およそ 1 時間 50 分遅れ（IMP-02）、その間は手元の Sandbox を席が使う。席が Sandbox を閉じられないと録画と次の回の席が止まる（IMP-01）。

<!-- prose impact.problem -->
2026-11-19、本人が配る形を入れて普通に使ったら、更新の確認がエラーを出し続けた。いまの試験はどれも Scenario を 1 つずつ確かめ、e2e の相手は開発用の起動なので、「入れて普通に使うとどう見えるか」を見る席が無く、本人が最初の利用者になっている。

<!-- prose uses[U1].title -->
手順書を開いたら、先に使ってみた結果が載っている

<!-- prose uses[U1].trigger -->
ST の確認バッチの手順書が届いた。自分で配る形を入れて試す前に、引っかかる所が無いか知りたい。

<!-- prose uses[U1].outcome -->
自分が最初の利用者にならずに、どこを重点に確かめるか決めてから確認バッチに入れる

<!-- prose uses[U1].without -->
配る形を自分で入れて使い、違和感を自分で踏んで初めて分かる（11-19 の更新の確認のエラーのように）

<!-- prose uses[U1].steps[0].do -->
手順書の冒頭の「成果物と起動」の最後の段落を読む

<!-- prose uses[U1].steps[0].see -->
「使ってみた（Windows Sandbox・38 分・一通り終えた）: 違和感の候補 2 件。①更新の確認が何度もエラーを出す ②取り込みの後に件数が出るまで 15 秒（Sandbox で測った）。巡った目的: install・password・login・import・update-check …。報告: C:\dev\ashiato2-trial-use\<tag>\report.html」

<!-- prose uses[U1].steps[1].do -->
Windows で報告の HTML を開く

<!-- prose uses[U1].steps[1].see -->
候補ごとの画面の写し・直前の操作・期待したことと見えたこと・もう 1 回で起きたか、と違和感メモ

<!-- prose uses[U1].steps[2].do -->
①の違和感メモを「コピー」して /iwakan に貼る

<!-- prose uses[U1].steps[2].see -->
「違和感」の issue が 1 件できる（②は Sandbox の遅さと見て貼らない）

<!-- prose walkthroughs[W1].title -->
同じ出所の作り直しでは走らない

<!-- prose walkthroughs[W1].scene -->
確認バッチが gate の後に作り直される。前の回（tag A）は一通り使い終えて候補 2 件だった。

<!-- prose walkthroughs[W1].why -->
確認バッチは gate のたびや fix の後に作り直され、そのたびに 45 分と Claude の利用枠を使うのは重い。組み立てに入るファイルの中身を比べ、同じなら前の回の結果を写す。

<!-- prose walkthroughs[W1].figure.caption -->
作り直したときに席がすること（A・B・C は説明用の tag）

<!-- prose walkthroughs[W1].cases[0].result -->
出所の比べに入らないので走らず、A の結果が写される。見え方が変わっていても気づかない

<!-- prose walkthroughs[W1].cases[1].result -->
Sandbox を起こして使い直す

<!-- prose walkthroughs[W1].exceptions[0].text -->
出所の外（配布元・同梱の PostgreSQL の取り直し）の変化は比べない。見つかったら比べる値に入れる（反転条件）

<!-- prose walkthroughs[W2].title -->
写しを撮り損ねた候補は数えない

<!-- prose walkthroughs[W2].scene -->
Claude が更新の確認でエラーを 2 回見て候補に書いたが、2 回目の写しを撮る前に画面が変わった。

<!-- prose walkthroughs[W2].why -->
誤検知で本人を疲れさせないため、機械が確かめられる根拠が揃ったものだけを数える。

<!-- prose walkthroughs[W2].figure.caption -->
根拠ごとの扱い

<!-- prose walkthroughs[W2].cases[0].result -->
件数に入り、題が手順書に並ぶ

<!-- prose walkthroughs[W2].cases[1].result -->
本物の不具合でも手順書の件数に出ず、報告の別の節にだけ残る

<!-- prose walkthroughs[W2].exceptions[0].text -->
既知の issue と同じと照合した候補も数えない（「既知: #<番号>」の印で報告に残る）

<!-- prose model.caption -->
席が 1 回の確認バッチの中でしていること

<!-- prose features[prep].summary -->
準備の最後に 1 行で呼ばれ、どう終わっても段落を 1 つ足すだけ

<!-- prose points[prep.call].text -->
確認バッチの準備（`tools/verify-prep.sh`）が、いまの成果物と manifest を作り終えた後に席を 1 回呼ぶ。席の結果は manifest の最後に「使ってみた」で始まる 1 段落として足される

<!-- prose points[prep.never-stop].text -->
席がどう終わっても（組めない・Claude が動けない・Sandbox が無い）、準備は止まらず、既存の段落はそのまま残る。代わりに、理由を 1 つ名指しした警告の段落が載る

<!-- prose points[prep.slow].text -->
席が走る回（出所が変わった回と、前の回が警告で終わった回）は、手順書が届くのが遅れる。初回は組み立て（最大 60 分）と使う時間（最大 45 分）で最大およそ 1 時間 50 分、2 回目からは差分のビルドと 45 分と数分

<!-- prose points[prep.claude].text -->
Claude は席の台本が直に `claude -p` で起こす（グラフの外）。利用上限で動けなかった回は警告にして、同じ回の中で起こし直さない

<!-- prose points[prep.harness].text -->
ハーネス（harness2）の側は変えない。手順書の形・録画・/iwakan・ラベル「違和感」の意味はいまのまま

<!-- prose features[build].summary -->
確認バッチの木から zip を 2 つ組み、出所が前と同じなら走らない

<!-- prose points[build.two].text -->
席は、準備が走っている木の HEAD から、サーバの zip と PC の収集アプリの zip の 2 つを手元の Windows で組み、その 2 つだけを Sandbox に渡す。携帯の APK は渡さない

<!-- prose points[build.fail].text -->
組めなかった・60 分の上限を越えたときは、Sandbox を起こさずに「組めなかった」の警告で終える

<!-- prose points[build.skip].text -->
組み立てに入るファイルの中身が前の回と同じなら、Sandbox を起こさず、前の回の段落を「前の回（<tag>）の結果」として写す。比べるのは zip のバイト列ではなく、組み立てに入るファイルの木

<!-- prose points[build.rerun].text -->
前の回が使い終えていない（組めなかった・Sandbox が動いていた・Claude が動けなかった）なら、同じ出所でも走る。`probe-` で始まる手で試す回は、前の回の記録を読みも書きもしない

<!-- prose features[sandbox].summary -->
まっさらな Sandbox を網を開けて起こし、Claude が目的だけを持って最大 45 分使う

<!-- prose points[sandbox.own].text -->
席は自分で起こした Sandbox だけを扱う。正常に終えても・失敗しても・打ち切っても、席の終わりまでに閉じる。網は開けて起こす（本人の PC と同じに、更新の確認が配布元に問い合わせられる）

<!-- prose points[sandbox.busy].text -->
Sandbox が既に動いている（本人か録画のもの）・別の席が走っているときは、起こさず閉じず、走らせなかったことを警告に書く。閉じても残った Sandbox も段落に書く

<!-- prose points[sandbox.material].text -->
Claude に渡すのは、利用者の目的の一覧・2 つの zip・合成の書き出しだけ。仕様・Scenario・コード・前の回の報告・開いている issue は見せない。Claude はリポジトリの外で、ブラウザを操る道具・Sandbox への頼みの道具・候補の書き込みだけを許されて動く

<!-- prose points[sandbox.goals].text -->
目的の一覧は手順を書かず、依頼の対象ごとの札（install・password・login・import・browse・settings・update-check・uninstall・reinstall・pc-collector）を 1 つずつ持つ。どの順でどこを触るかは Claude が決める

<!-- prose points[sandbox.days].text -->
「数日分の記録」は、利用者の口（取り込みに合成の書き出しを渡す・PC の収集アプリを数分動かす）だけで作る。時計やタイムゾーンを変える頼みは断る（時刻を読むだけの頼みは通す）

<!-- prose points[sandbox.limit].text -->
Claude が使う時間は 45 分まで。越えたら止め、それまでに書いた候補と写しで報告を作る

<!-- prose points[sandbox.healthz].text -->
使っている間に中のサーバが一度も応えなかったら、「インストールまで届かなかった」を候補とは別の警告として書く。一度でも応えた後にアンインストールした回は、届いたとみなす

<!-- prose features[cand].summary -->
利用者に見えたもので、根拠が 4 つ揃ったものだけを数える

<!-- prose points[cand.evidence].text -->
候補として数えるのは、4 つの根拠（画面の写し・直前の操作・期待したことと見えたこと・もう 1 回で起きたか）が全部揃ったものだけ。欠けた候補は手順書の数に入らず、手元の報告の「根拠が欠けたもの」に残る

<!-- prose points[cand.marks].text -->
もう 1 回で起きなかったものは「1 回だけ」、Sandbox に固有の違いに依るものは「Sandbox だから」、待ち時間の候補の題には「（Sandbox で測った）」が付く。「1 回だけ」でないものが先に並ぶ

<!-- prose points[cand.known].text -->
使い終えた後に、開いている「違和感」の issue と照合し、同じと分かった候補には「既知: #<番号>」を付けて数から外す。使っている間の Claude には issue を見せない

<!-- prose points[cand.cap].text -->
手順書に出すのは上位 10 件まで。件数は全件の数を書き、手元の報告には全件が残る

<!-- prose features[out].summary -->
見たものは全部手元、公開の手順書には件数と題と場所だけ

<!-- prose points[out.local].text -->
画面の写し・Sandbox から取り出したログ（サーバ・同梱の DB・更新係・インストーラ）・候補の全件・Claude の操作の記録は、リポジトリの外の手元の置き場に置き、1 枚の報告の HTML にまとめる。警告で終わった回も、そこまでの写しで報告を作る

<!-- prose points[out.title].text -->
手順書の段落に載るのは、件数・題・巡った目的の札・所要・終わり方・報告の場所だけ。手順書は公開の repo に commit され、PR 本文にも写る。題は Claude の文なので、1 行 60 文字に切り、私的な値の形は伏せる

<!-- prose points[out.zero].text -->
候補が 0 件でも、0 件と巡った目的の札を書く（走って何も見なかったのか、見て何も無かったのかを分ける）

<!-- prose points[out.memo].text -->
席は issue を作らない。GitHub には一覧を読むだけ。手元の報告の各候補に、そのまま /iwakan に貼れる違和感メモ（コピーのボタンつき）が付き、残すものは本人が選んで貼る

<!-- prose points[out.done].text -->
この change は、確認バッチで 1 回実地に走り、手順書の冒頭に結果（0 件でも巡った目的）か警告が載れば終わり。11-19 の違和感（更新の確認がエラーを出し続ける）を拾えたかは記録するが、判定には使わない

<!-- prose impact.scenarios[S1].result -->
席は起こさず、「別の Sandbox が動いていた」の警告の段落が載る。録画も同じ理由で止まる（いまと同じ）

<!-- prose impact.scenarios[S1].lost -->
その回の使ってみた結果

<!-- prose impact.scenarios[S2].result -->
件数に入らず、手元の報告の「根拠が欠けたもの」に残る

<!-- prose impact.scenarios[S2].lost -->
手順書の冒頭でその不具合に気づくこと

<!-- prose impact.scenarios[S3].result -->
走らず、A の件数と題が「前の回（A）の結果」として写される

<!-- prose impact.scenarios[S3].lost -->
配信の変化による見え方の違いに気づくこと

<!-- prose impact.rules[R1].legit -->
本人が自分の Sandbox を開いたまま確認バッチが作られると、その回は使ってみた結果が出ない（警告の段落だけ）

<!-- prose impact.rules[R2].legit -->
時刻を読むだけの頼みは届くので、Claude が表示の時刻を確かめることは止まらない

<!-- prose impact.rules[R3].legit -->
本物の不具合でも、写しを撮り損ねると手順書の件数に出ず、手元の報告の別の節を開かないと見えない

<!-- prose impact.rules[R4].legit -->
出所の外（配布元の配信・同梱の PostgreSQL の取り直し）だけが変わったときも走らず、見え方の変化を見逃す

<!-- prose impact.rules[R5].legit -->
残したい候補は本人が報告を開き、違和感メモを /iwakan に貼る手間が 1 回要る

<!-- prose impact.rules[R6].legit -->
一時の失敗でも、その回は結果が出ない（次の回は同じ出所でも走り直す）

<!-- prose impact.rules[R7].legit -->
CI・作業箱では元から使えないので、止まる正当な利用は見つけていない

<!-- prose impact.findings[IMP-01].title -->
席の Sandbox が残ると、録画と次の回の席が止まり続ける

<!-- prose impact.findings[IMP-01].text -->
録画は始めに他の Sandbox を見て、動いていれば止まる。席が自分の Sandbox を閉じられなかった（閉じても残った・席のプロセスごと落ちた）回は、録画が撮れず、その確認バッチの「違和感」の問いに動画が付かない。残った Sandbox は次の回の席も「別の Sandbox が動いていた」として閉じずに見送るので、本人が気づいて閉じるまで、次の確認バッチ以降も席と録画が止まる

<!-- prose impact.findings[IMP-02].title -->
席が走る回は、手順書と同じ錠を待つ作業が最大およそ 1 時間 50 分遅れる

<!-- prose impact.findings[IMP-02].text -->
グラフは準備の台本を時間の上限なしで、確認バッチの作業ツリーの錠を握ったまま待つ。席の組み立て（初回 60 分まで）と使う時間（45 分まで）の間、この確認バッチの手順書が届かず、同じ錠を取る別の確認バッチ・録画の撮り直し・fix も待たされる。その間に手元が落ちると、段は初めからやり直しになる

<!-- prose impact.findings[IMP-03].title -->
席の Claude が、並走する agent と同じ利用枠を使う

<!-- prose impact.findings[IMP-03].text -->
席は本人の Claude を最大 45 分と照合の 1 回使う。グラフは同時に複数の thread を走らせるので、確認バッチの間に並走する Story の agent が利用上限に当たりやすくなる

<!-- prose impact.findings[IMP-04].title -->
インストーラが Sandbox で通らなければ、席は毎回警告で終わる

<!-- prose impact.findings[IMP-04].text -->
配るインストーラ（サービスの登録・昇格）が Sandbox の中で最後まで通るかは確かめていない。通らなければ、毎回「インストールまで届かなかった」が載るだけになる。警告で終わった回の後は同じ出所でも走り直す（R9）ので、出所が変わらなくても確認バッチのたびに組み立てと Claude の 45 分を使う（IMP-02・IMP-03 も同じ分広がる）。tasks はその場合も残りを作って閉じる

<!-- prose impact.findings[IMP-05].title -->
準備の台本に足す行に `install` の英字を入れると CI が落ちる

<!-- prose impact.findings[IMP-05].text -->
APK の入れ方の自己検査は、準備の台本の install_on_emulators 以外の行に `adb` か `install` の語があれば落ちる。予定の行には無いが、実装で警告の文を変えるときに触れうる

<!-- prose impact.scope.covers[0].text -->
確認バッチの準備が、手元の Windows Sandbox で配る 2 つの zip を Claude に使わせること

<!-- prose impact.scope.preconditions[0].text -->
手元に Windows の PowerShell と Windows Sandbox があり、Claude が `claude -p` で動くこと。無ければ走らず警告だけ

<!-- prose impact.scope.preconditions[1].text -->
渡すのは確認バッチの木から組んだサーバと PC の収集アプリの zip だけ・Claude の材料は目的と zip と合成の書き出しだけ

<!-- prose impact.scope.preconditions[2].text -->
候補は利用者に見えたもので根拠が 4 つ揃ったものだけ。手順書には件数と題と場所だけ。GitHub には一覧を読むだけ

<!-- prose impact.scope.preconditions[3].text -->
出所が前の回から変わったとき・前の回が使い終えていないときだけ走る

<!-- prose impact.scope.excludes[0].text -->
携帯の収集アプリ（APK）

<!-- prose impact.scope.excludes[1].text -->
候補の issue 化・ハーネスの変更

<!-- prose impact.scope.excludes[2].text -->
本人や録画が起こした Sandbox（席は自分で起こしたものだけを閉じる）

<!-- prose impact.scope.residual[0].text -->
席の Sandbox が残ると、本人が閉じるまで録画と次の回の席が止まる（IMP-01）

<!-- prose impact.scope.residual[1].text -->
席が走る回は手順書が遅れ、利用枠を使う（IMP-02・IMP-03）

<!-- prose impact.scope.residual[2].text -->
インストーラが Sandbox で通るかは未確認（IMP-04）

<!-- prose impact.scope.residual[3].text -->
題は Claude の文で、言い回しまでは機械で縛れない（Q7）

