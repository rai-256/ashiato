# ST61 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
本人が版を上げる操作をしなくても、main に入った製品の変更ごとに、前と違う番号のサーバ・PC・携帯の組み立てが残る。同じ番号で中身の違う組み立ては作られず、直した版をインストーラが別の版として入れられる。

<!-- prose impact.overview -->
main に入った変更のうち、CI が通り製品が変わった commit に機械が番号を付け、番号ごとにサーバ・PC・携帯の組み立てを配布元の下書きに残す。番号付けが下書きに置く組み立ての間では、印や説明文が別の commit を指す物を置かないので、同じ番号で中身の違う物は並ばない（手元で番号を名乗らせる道 IMP-04 と、組み直しの再現性は残る）。代わりに、修正の枝から正式版を出す道と、PR の携帯の APK を本人の携帯へ上書きで入れる道は無くなる。

<!-- prose impact.problem -->
2026-11-19、版を手で上げないまま同じ 0.1.x で中身の違う組み立てができ、本人が直した版をサーバの PC に入れようとしても、インストーラが同じ版の走らせ直しとみなして入らなかった。

<!-- prose uses[U1].title -->
直したサーバを確実に入れる

<!-- prose uses[U1].trigger -->
サーバの不具合を直す Story が確認バッチで main に入った。11-19 のように、直したのに同じ版で入らないのは困る。

<!-- prose uses[U1].outcome -->
直した中身が 0.3.0 として入り、記録の部品の版でも直した後の記録だと見分けられる

<!-- prose uses[U1].without -->
同じ 0.1.x のまま組まれ、インストーラが走らせ直しとみなして、直した版が入らない

<!-- prose uses[U1].steps[0].do -->
確認バッチの PR を merge する

<!-- prose uses[U1].steps[0].see -->
main の commit は増えない。CI が通った後、配布元に印 build-0.3.0 と同じ名前の下書きができる

<!-- prose uses[U1].steps[1].do -->
配布元の下書き build-0.3.0 から、サーバの Windows 向けの zip を取る

<!-- prose uses[U1].steps[1].see -->
説明文に番号 0.3.0 と commit の 40 桁

<!-- prose uses[U1].steps[2].do -->
サーバの PC でインストーラを走らせる

<!-- prose uses[U1].steps[2].see -->
入っている 0.2.4 と違う版なので、更新の手順に入る

<!-- prose walkthroughs[W1].title -->
merge が続いた 1 日の番号

<!-- prose walkthroughs[W1].scene -->
前の番号 0.2.4 の後、1 日に 5 つの merge が続けて main に入る。

<!-- prose walkthroughs[W1].why -->
番号は CI が成功した commit に付き、前の番号からの差分で製品の変更を見る。取り消された run の commit には付けない。main の CI は前に成功した run からの差分で試すので、B の run が A のコードも試す。

<!-- prose walkthroughs[W1].figure.caption -->
merge ごとの CI と番号（番号と merge の並びは説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
前の番号から A の製品の変更があるので 0.2.5 が付き、A の中身が入る

<!-- prose walkthroughs[W1].cases[1].result -->
前の番号 0.3.0 から製品が変わっていないので付かない

<!-- prose walkthroughs[W1].cases[2].result -->
A には番号が無い。A だけの組み立ては残らない（A の中身は 0.2.5 に入る）

<!-- prose walkthroughs[W1].exceptions[0].text -->
説明用の値。実際の 1 日の merge の並びは確かめていない

<!-- prose walkthroughs[W2].title -->
PR の組み立てを試す

<!-- prose walkthroughs[W2].scene -->
本人が、直した PR の組み立てをサーバの PC と携帯で試そうとする。

<!-- prose walkthroughs[W2].why -->
番号を持たない組み立ては、本物の番号と重ならないよう 0.0.x を名乗る。インストーラは版が同じなら走らせ直しにし、Android は小さい versionCode を断る。

<!-- prose walkthroughs[W2].figure.caption -->
入っている物と入れる物ごとの結果

<!-- prose walkthroughs[W2].cases[0].result -->
サーバも携帯も新しい版として入る

<!-- prose walkthroughs[W2].cases[1].result -->
インストーラが走らせ直しとみなし、古いプログラムのまま（IMP-02）

<!-- prose walkthroughs[W2].cases[2].result -->
上書きを断られる。消して入れ直すと未送信が消えうる（IMP-01）

<!-- prose walkthroughs[W2].exceptions[0].text -->
署名の鍵が組み立てごとに同じかは確かめていない（ST64）

<!-- prose model.caption -->
この Story が回すもの

<!-- prose sections.makes.tldr -->
main の CI が通った製品の変更に機械が番号を付け、番号ごとにサーバ・PC・携帯を組んで残す。

<!-- prose sections.change.tldr -->
版を手で上げる代わりに、番号付けが番号を決める。携帯の versionCode の式も変わる。

<!-- prose sections.lost.tldr -->
修正の枝からの正式版と、PR の携帯の APK を本人の携帯へ上書きで入れる道が無くなる。

<!-- prose sections.uses.tldr -->
直した版を入れたいとき、番号の組み立てを取れば、前の版と違う版として確実に入る。

<!-- prose sections.watch.tldr -->
merge 1 つに番号 1 つではない。PR の組み立てはどれも 0.0.1 を名乗る。

<!-- prose sections.findings.tldr -->
重大な問題は見つけていない。PR の組み立ての入れ方など中程度が 3 件。

<!-- prose sections.rest.tldr -->
札・公開・移行は作らない（ST62〜ST64）。署名していない APK も残すことは AI に任せた。

<!-- prose features[when].summary -->
CI が通り、前の番号から製品が変わった main の commit にだけ付く

<!-- prose features[when].tldr -->
CI が成功し、前の番号から製品が変わった main の commit に番号が付く。

<!-- prose points[when.1].text -->
main への push の CI が成功で終わると、番号付けが走る。前に番号を付けた commit からの差分が製品に触れていれば、その commit に次の番号を 1 つ付ける。main に commit は積まない。

<!-- prose points[when.2].text -->
番号付けは CI と別の流れで、1 本ずつ流れ、取り消されない。終わった run の commit を全部の履歴で取り出して組む（main の HEAD は組まない）。

<!-- prose points[when.3].text -->
merge 1 つに番号 1 つではない。CI が次の merge に取り消された merge には番号が付かず、その中身は後の commit の番号に入る。文書だけの merge でも、その前に取り消された製品の merge があれば番号が付く。

<!-- prose points[when.4].text -->
そのため main への push の CI は、直前の push からでなく、前に成功した main の run の commit からの差分で重い job を出し分ける。取り消された merge のコードも、番号が付く前に試される。見つからなければ全部走らせる。

<!-- prose points[when.5].text -->
PR・fork・main 以外の枝の run では番号を付けない。番号を付けた commit より前の commit は、後から run が成功しても番号を付けない。同じ commit を走らせ直すと、付けた番号のまま返す。

<!-- prose features[how].summary -->
左は本人のファイル、真ん中は確認バッチ、右はそれ以外で 1 つ進む

<!-- prose features[how].tldr -->
左の数字は本人がファイルで決め、真ん中と右は機械が決める。最初は 0.2.0。

<!-- prose points[how.1].text -->
最初の番号は 0.2.0。今の形で使った 0.1.0・0.1.1 より大きいので、本人の PC と携帯の 0.1.x より新しいと比べられる。

<!-- prose points[how.2].text -->
前の番号から番号を付ける commit までの間に確認バッチの merge が 1 つでもあれば、真ん中を 1 上げて右を 0 にする。無ければ右を 1 上げる。確認バッチの merge は merge の題（verify/ の枝）で見分ける。

<!-- prose points[how.3].text -->
左の数字は tools/release/MAJOR（今は 0）。本人が PR で 1 に変えると、その後の最初の番号が 1.0.0 になる。

<!-- prose points[how.4].text -->
確認バッチは 30 日で約 21 回入るので、真ん中は年に 250 ほど進む（0.250.x）。真ん中の上限 999 に約 4 年で届き、届くと番号付けが止まる。届く前に左を上げるか式を見直す。

<!-- prose points[how.5].text -->
左のファイルを下げた・数字 1 つでない・履歴が浅い木では、番号を付けずに止まる。組み立てが落ちた番号は次の commit に回さず、欠番になる。

<!-- prose features[keep].summary -->
番号ごとに印と下書きのリリースを 1 つ作り、消さない

<!-- prose features[keep].tldr -->
番号ごとに印 build-<番号> を作り、組み立てを配布元の下書きに残す。消さない。

<!-- prose points[keep.1].text -->
番号を決めたら、組む前に印 build-<番号> を作る。その後でサーバ（Windows・Linux）・PC の収集アプリ・携帯の APK を組み、検査を通してから、同じ名前の下書きのリリース 1 つに置く。説明文に番号と commit を書く。

<!-- prose points[keep.2].text -->
番号の組み立ては全部残し、消さない（月に約 1.7 GB）。どの番号にも後から組み直さずに正式版などの札を付けられる。消し方は札を作る ST62 で決める。

<!-- prose points[keep.3].text -->
同じ番号の下書きに配布物が揃っていれば何もしない。欠けていればそれだけを足す。印か下書きの説明文が別の commit を指していれば、何も置かずに止まる。

<!-- prose points[keep.4].text -->
配布元へ書く権限は番号付けの job にだけ持たせ、その job は公開と印 v* の操作をしない。番号の組み立ては CI のキャッシュを使わず、保存もしない（PR の組み立てが番号の組み立てを引かない）。

<!-- prose points[keep.5].text -->
番号の印 build-* は配った印（v*）にも開発版の印にも数えない。配った移行の検査と送信契約の検査は、今のまま v* だけを数える。

<!-- prose features[nonum].summary -->
PR・手元・CI の試験の組み立ては、本物に使わない 0.0.x を名乗る

<!-- prose features[nonum].tldr -->
PR と手元と CI の試験の組み立ては 0.0.x を名乗り、本物の番号を名乗らない。

<!-- prose points[nonum.1].text -->
版番号を渡さずに組むと、4 つの部品（サーバ・画面・携帯・PC）が出所の版 0.0.1 を名乗る。番号を渡すのは番号付けの流れだけで、CI の配る形の組み立ての job は番号を渡さない。

<!-- prose points[nonum.2].text -->
渡せる版番号は 2 つの系だけ。出所の版の形（0.0.1 とその -beta・-dev）か、左が左の数字のファイルと同じ番号の形。それ以外は、どの部品の組み立ても落ちる。4 か所（bash・Rust・gradle・画面）が同じ表で確かめる。

<!-- prose points[nonum.3].text -->
渡した番号は 4 つの部品の組み立ての印・サーバの版の記録・更新の履歴に同じ値で入る。API の契約の版は渡した番号に依らず 0.0.1 のまま（変えると main に commit を積むことになる）。

<!-- prose points[nonum.4].text -->
CI のインストーラの更新の試験は組み立ての版を 0.4.0・0.5.0 などから 0.0.2〜0.0.5 に替えるだけ。更新・戻し・記録の振る舞いは変えない。

<!-- prose points[nonum.5].text -->
携帯の PR の APK は versionCode が 1 になり、本人の携帯の 0.1.x（10,199）にも番号の APK にも上書きで入らない。試すには、未送信を送り切ってから消して入れ直すか、番号の APK を使う。

<!-- prose points[nonum.7].text -->
旧い「3 つの部品の版番号は、1 つの出所から決まる」は外し、上の番号か出所の版から決まる形に置き換える（旧い本文は、出所の版と違う版番号を渡すと落ちた）。

<!-- prose points[nonum.6].text -->
手で起こす確かめの組み立ては、PR の組み立ての版 0.0.1 から 0.0.1-beta.1 を名乗る下書きにする。下書きの印の名前は build- で始めない（番号の記録に混ぜない）。

<!-- prose features[android].summary -->
versionCode の式の桁を広げ、1 年分の番号でも組み立てが落ちない

<!-- prose features[android].tldr -->
携帯の versionCode の式を左×1千万＋真ん中×1万＋右に変え、桁を広げる。

<!-- prose points[android.1].text -->
versionCode は 左×10,000,000 + 真ん中×10,000 + 右。後ろの -beta・-dev は値に入れない（同じ番号は同じ値）。手で付けない。

<!-- prose points[android.2].text -->
最初の番号 0.2.0 は 20,000 で、前の式の 0.1.0・0.1.1（10,099・10,199）より大きい。

<!-- prose points[android.3].text -->
真ん中 999・右 9,999・左 209 を超える版番号では、計算も携帯の組み立ても落ちる（Android の上限 2,100,000,000 の内側に収める）。

<!-- prose points[android.5].text -->
旧い「携帯の versionCode は版番号から計算する」（左×1,000,000 + 真ん中×10,000 + 右×100 + 末尾）は外し、上の式に置き換える。

<!-- prose points[android.4].text -->
この式は一度 APK を配ったら版をまたいで変えない。Android は小さい versionCode の上書きを断るので、配った値より小さくなる式には戻れない。

<!-- prose features[transition].summary -->
印 v* は番号の commit にだけ。開発版は最新の番号の commit から組む

<!-- prose features[transition].tldr -->
札（ST62・ST63）ができるまで、印 v* は番号を付けた commit にだけ付け、開発版はその commit から組む。

<!-- prose points[transition.1].text -->
印 v<番号>（と -beta.<N>）は、印の commit に番号の印 build-<番号> があり、3 つの数字が同じときだけ公開する。番号の無い commit・番号の違う印・main の祖先でない commit の印は公開しない。

<!-- prose points[transition.2].text -->
緊急の修正の枝 release/* の例外を外す。番号は main にだけ付くので、枝の上の commit は番号を持てない。急ぎの直しも main で直して次の番号を出す（別の道は ST62 が決める）。

<!-- prose points[transition.3].text -->
1 つのリリースの配布物は今のまま 4 つで同じ版番号。配布物の名前、浅い木で通さないこと、決めた形でない印は配らないことも今のまま。

<!-- prose points[transition.6].text -->
旧い「印の名前で 3 つのチャンネルに分けて配り…」と「開発版は、幹にその日の変更があった日だけ…」は、番号の形に合わせた名前の Requirement に書き直す（振る舞いの違いは上の 2 点）。

<!-- prose points[transition.7].text -->
「緊急の修正の枝は、配った版から切り…」を正典から外す。配った版はまだ無いので、移す枝も印も無い。

<!-- prose points[transition.4].text -->
開発版の作り直しは main の HEAD でなく、最も大きい番号の commit を <番号>-dev.<日付> で組む。印 nightly がその commit を指していれば組まない。番号がまだ無ければ組まない。

<!-- prose points[transition.5].text -->
開発版の公開の設定が入っていない間は、予定の実行は組まず、手で起こしても公開しない。日本時間 0:00 に起きること、開発版の鍵での署名は公開の job の中だけであることは今のまま。

<!-- prose impact.scenarios[S1].result -->
Android が小さい versionCode の上書きを断る

<!-- prose impact.scenarios[S1].lost -->
入れたままで試すこと。消して入れ直すと手元の未送信が消えうる

<!-- prose impact.scenarios[S2].result -->
インストーラは同じ版の走らせ直しとみなし、プログラムを入れ替えない

<!-- prose impact.scenarios[S2].lost -->
直した中身がサーバに入ること（11-19 の事故と同じ形）

<!-- prose impact.scenarios[S3].result -->
枝の印は公開されない。main で直すと、次の番号は確認前の変更も含む

<!-- prose impact.scenarios[S3].lost -->
配った版に直しだけを当てて出すこと

<!-- prose impact.scenarios[S4].result -->
番号付けが止まり、理由が出る

<!-- prose impact.scenarios[S4].lost -->
本人が左を上げるまで、番号の組み立てができない

<!-- prose impact.rules[R1].legit -->
なし（文書は組み立てに入らず、取り消された中身は後の番号に入る）

<!-- prose impact.rules[R2].legit -->
確認バッチが約 4 年続くと真ん中の上限 999 に届き、本人が左を上げるまで番号が付かない

<!-- prose impact.rules[R3].legit -->
なし（印を作り直したときだけ止まる）

<!-- prose impact.rules[R4].legit -->
PR と手元の組み立ては番号を名乗れない（0.0.1）

<!-- prose impact.rules[R5].legit -->
R2 と同じく、上限に届いたときだけ

<!-- prose impact.rules[R6].legit -->
配った版にだけ急ぎの直しを当てる道が無くなる

<!-- prose impact.rules[R7].legit -->
この Story が main に入ってから最初の番号が付くまで、開発版が作られない（公開の設定はまだ入っていない）

<!-- prose impact.rules[R8].legit -->
なし

<!-- prose impact.rules[R9].legit -->
なし

<!-- prose impact.findings[IMP-01].title -->
PR の携帯の APK は、本人の携帯に上書きで入らない

<!-- prose impact.findings[IMP-01].text -->
番号を持たない組み立ての APK は versionCode 1 になり、0.1.x と番号の APK の上に入らない。PR の携帯の変更を本人の携帯で試すには、消して入れ直すか番号の APK を待つ

<!-- prose impact.findings[IMP-02].title -->
PR のサーバの組み立ては、入れ替わらないか、版を下げる更新に入る

<!-- prose impact.findings[IMP-02].text -->
PR の組み立てはどれも 0.0.1 なので、0.0.1 が入った PC に別の PR の組み立てを入れると、インストーラは走らせ直しとみなしてプログラムを入れ替えない。11-19 と同じ形が PR の組み立てには残る。逆に、番号の版（0.2.x）の PC に PR の組み立てを入れると、インストーラは版の大小を見ないので、版を下げる更新に入る

<!-- prose impact.findings[IMP-03].title -->
番号付けの順番待ちが取り消されると、番号が遅れる

<!-- prose impact.findings[IMP-03].text -->
判定の job の同時実行の組が、条件を満たさない run（取り消された CI の run など）に席を取られないこと（P18）を確かめていない。外れると待っていた番号付けが取り消され、番号は次の成功まで付かない

<!-- prose impact.findings[IMP-04].title -->
手元で番号の形を渡すと、同じ番号で別の中身を組める

<!-- prose impact.findings[IMP-04].text -->
版の検査は左の数字が合えば番号の形を受け取り、番号の印を見ない。手元で ASHIATO_BUILD_VERSION=0.2.5 を渡せば、0.2.5 を名乗る別の中身を組める

<!-- prose impact.findings[IMP-05].title -->
番号の印 build-* は消せる・動かせる

<!-- prose impact.findings[IMP-05].text -->
配った印 v* の守り（消せず動かせない）は build-* に掛かっていない。push できる人が印を動かすと、番号と中身の対応が崩れる。番号付けは「印が別の commit を指せば止まる」で気づくだけ

<!-- prose impact.findings[IMP-06].title -->
配った版にだけ急ぎの直しを当てる手順を、ST62 まで仕様は定めていない

<!-- prose impact.findings[IMP-06].text -->
修正の枝の例外を外したので、仕様に書いてある急ぎの直しの手順は、main で直して次の番号を出すことだけ。main に配りたくない変更があると、直しと一緒に出る

<!-- prose impact.findings[IMP-07].title -->
約 4 年で真ん中の上限に届き、番号付けが止まる

<!-- prose impact.findings[IMP-07].text -->
確認バッチごとに真ん中が上がると年に約 250 進み、上限 999 に約 4 年で届く。届くと番号付けが止まり、本人が左を上げるか式を見直すまで番号が付かない

<!-- prose impact.scope.covers[0].text -->
main の push の CI が成功した、この repo の main の commit の番号付けと、その組み立て

<!-- prose impact.scope.covers[1].text -->
4 つの部品の版番号の出所と、携帯の versionCode の式

<!-- prose impact.scope.preconditions[0].text -->
番号は、CI が成功し、前の番号から製品（docs/・openspec/・*.md 以外）が変わった commit にだけ付く

<!-- prose impact.scope.preconditions[1].text -->
番号付けは 1 本ずつ流れ、条件を満たした run だけが順番待ちに入る（P18 は未確認）

<!-- prose impact.scope.preconditions[2].text -->
main の CI の出し分けの基準は main への push のときだけ変わる。PR は今のまま

<!-- prose impact.scope.preconditions[3].text -->
渡せる版番号は出所の版の形と、左がファイルと同じ番号の形だけ。0.0.0 は使わない

<!-- prose impact.scope.preconditions[4].text -->
印 v* は番号の main の commit にだけ。開発版は番号がある間だけ組む

<!-- prose impact.scope.preconditions[5].text -->
確かめの下書きは下書きにだけ置き、印を作らない

<!-- prose impact.scope.preconditions[6].text -->
インストーラの更新の手順は Windows のサーバだけ。この Story は試験の版の値だけを直す

<!-- prose impact.scope.preconditions[7].text -->
真ん中を上げるかは、前の番号の commit を除き、番号を付ける commit までの確認バッチの merge で決める

<!-- prose impact.scope.preconditions[8].text -->
配布元へ書く権限は番号付けの job だけ。下書きは公開せず、印 v* を作らない

<!-- prose impact.scope.preconditions[9].text -->
versionCode は真ん中 999・右 9,999・左 209 まで

<!-- prose impact.scope.excludes[0].text -->
札（正式版・ベータ版・開発版）を番号へ動かす操作と公開・正式な鍵の署名（ST62・ST63）

<!-- prose impact.scope.excludes[1].text -->
本人の PC と携帯の 0.1.x からの移行と、札の無い組み立てを入れない守り（ST64）

<!-- prose impact.scope.excludes[2].text -->
番号の組み立ての消し方（ST62）

<!-- prose impact.scope.residual[0].text -->
PR の組み立ては、携帯には上書きで入らず、サーバには入れ替わらないか版を下げる更新に入る（IMP-01・IMP-02）

<!-- prose impact.scope.residual[1].text -->
手元で番号を名乗る組み立てを作れる（IMP-04。ST64）

<!-- prose impact.scope.residual[2].text -->
番号付けの順番待ちと印の push の前提が未確認（IMP-03）

