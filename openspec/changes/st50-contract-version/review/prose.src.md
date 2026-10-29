# ST50 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
サーバ（S-01）の更新を後回しにしたまま収集アプリだけ新しくなっても、記録が黙って欠けずに携帯と PC の手元に残り、「サーバを更新してください」と知らせが出る。サーバを更新すると、残っていた記録がそのまま入る。送る形を変えたのに版を上げ忘れたら、CI が止める。

<!-- prose impact.overview -->
収集アプリは送る前に S-01 が受け付ける契約の版を確かめ、受け付けられない行を送らずに手元に残し、「サーバを更新してください」と知らせる。S-01 は新しすぎる版を 426 で、知らない欄を unknown_field で断り、収集アプリはどちらも捨てない。その代わり、新しい収集アプリを ST50 より前の S-01 に繋ぐと、S-01 を更新するまで記録も生存信号も届かなくなる。携帯では 90 日・2 GB の上限を超えた分は捨てられる。PC の未送信には上限を置かない（仮の決定 D13。PC のディスクが尽きた場合は見ていない）。

<!-- prose impact.problem -->
S-01 の更新は手動なので、収集アプリだけが新しい組み合わせは日常に起きる。今の S-01 は知らない欄を読み飛ばして 200 を返し、収集アプリは受け付けられた行を消すので、新しい欄（ST41 の終わりの時刻・ST49 の版など）だけが二度と戻らない。

<!-- prose walkthroughs[W1].title -->
収集アプリを先に新しくした

<!-- prose walkthroughs[W1].scene -->
本人が、携帯と PC の収集アプリを ST50 の版に入れ替えた。S-01 は手で更新するので、まだ ST50 より前のまま。

<!-- prose walkthroughs[W1].why -->
新しい収集アプリは、S-01 に受け付ける版を聞いてから送る。確かめる口を持たない S-01 は 404 を返すので「受け付けない」と読み、欄が黙って消えるのを避けるために何も送らない。

<!-- prose walkthroughs[W1].figure.caption -->
入れ替えの順と、送られるもの（版の数は説明用）

<!-- prose walkthroughs[W1].cases[0].result -->
ST50 より前の収集アプリは見出しを持たないので版 1 として受け付けられ、途切れない

<!-- prose walkthroughs[W1].cases[1].result -->
その数日は記録が届かず途絶と出るが、行は手元に残り、S-01 の更新後にそのまま入る

<!-- prose walkthroughs[W1].cases[2].result -->
携帯の 90 日を超えた分は捨てられる（PC は捨てない）。83 日目に音の鳴る知らせが出る

<!-- prose walkthroughs[W1].exceptions[0].text -->
説明用の順。実際に本人が入れ替える順は運用の手順（`docs/collector-contract.md` の版の節）にだけ書かれる

<!-- prose walkthroughs[W1].exceptions[1].text -->
圏外や合言葉の不一致では、行は残るが更新は促さない

<!-- prose walkthroughs[W2].title -->
S-01 を前の版へ戻した

<!-- prose walkthroughs[W2].scene -->
本人が S-01 を契約の版 2 に更新し、収集アプリも版 2 にした。不具合が見つかったので、S-01 だけを版 1 の組み立てに戻した。

<!-- prose walkthroughs[W2].why -->
行の版は積んだ時の値のまま変わらず、S-01 は自分より新しい版を受け付けない。新しい版の行を古い形に直して送る作りは無い。

<!-- prose walkthroughs[W2].figure.caption -->
戻した後の携帯の溜め（版の数は説明用）

<!-- prose walkthroughs[W2].cases[0].result -->
戻した S-01 も版 1 を受け付けるので、今どおり送られる

<!-- prose walkthroughs[W2].cases[1].result -->
版 1 の S-01 には送れず、S-01 を版 2 以上に戻し直すまで残る。携帯では 90 日を超えた分が捨てられる

<!-- prose walkthroughs[W2].exceptions[0].text -->
説明用の値。ST58 の自動の戻し（FR-119）は移行が失敗した更新の途中に起きるので、この筋書きの入口は主に手で戻す場合（ST58 はまだ上流が無く、確かめていない）

<!-- prose walkthroughs[W3].title -->
収集アプリにだけ欄を足した

<!-- prose walkthroughs[W3].scene -->
ある change が携帯の記録に最上位の欄を 1 つ足し、写しを作り直して版も上げた。S-01 の型には同じ欄を足し忘れた。

<!-- prose walkthroughs[W3].why -->
S-01 は知らない欄を自分の型で判定して断る。CI の写しは S-01 の型と収集アプリの欄を別々に記録し、版の上げ忘れを見るが、収集アプリの欄が S-01 の型に含まれるかは比べない。

<!-- prose walkthroughs[W3].figure.caption -->
同じリリースの S-01 と携帯（版 2 は説明用）

<!-- prose walkthroughs[W3].cases[0].result -->
同じ版のリリース同士で記録が入る

<!-- prose walkthroughs[W3].cases[1].result -->
S-01 を最新にしても断られ続け、知らせは誤った案内になる。直した S-01 を出すまで携帯の行は 90 日の上限に向かう

<!-- prose walkthroughs[W3].exceptions[0].text -->
説明用の筋書き。今の収集アプリ 2 本が送る最上位の欄は、どれも S-01 の型にあることを確かめた

<!-- prose model.caption -->
送る前に版を確かめ、受け付けられる行だけを送る（版の数は説明用）

<!-- prose features[version].summary -->
送る形の版を整数 1 つで持ち、S-01 は「どの版まで受け付けるか」を答える

<!-- prose points[version.1].text -->
送信契約の版は、スキーマ版（`schema_version`）とは別の整数 1 つ。S-01 は自分の版以下の全部を受け付ける。今の値は 1

<!-- prose points[version.2].text -->
`GET /contract` が `{"contract_version": <整数>}` を返す。取り込みと同じ API の合言葉で守り、合言葉が無ければ 401、画面のログインと衛星は 403

<!-- prose points[version.3].text -->
口の道・答えの形・見出しの名前・426 と本文の鍵は、最初の配った印の後は変えない。配った収集アプリはそれに従い続けるので、変えると新しい S-01 にも送らなくなる

<!-- prose features[reject].summary -->
新しすぎる版と知らない欄を、収集アプリが捨てない形で断る

<!-- prose points[reject.1].text -->
収集アプリの要求は見出し `Ashiato-Contract-Version` で版を名乗る。S-01 は自分より新しい版を 426 で断り、何も格納しない。見出しの無い要求は版 1 として今どおり受け付ける

<!-- prose points[reject.2].text -->
止めるのは記録だけでなく、収集アプリが送るもの全部（生存信号・破棄の報告・原本の欠け・原本）。止まっている間は生存信号も届かないので、S-01 の稼働状況には途絶と出る

<!-- prose points[reject.3].text -->
S-01 が知らない最上位の欄を持つ 1 件は、`malformed`（携帯が記録ごと捨てる種別）でなく `unknown_field` で断る。同じまとめ送りのほかの 1 件は今どおり入る

<!-- prose points[reject.4].text -->
知らない欄かどうかは S-01 自身が知る欄で決める（要求が名乗る版では決めない）。`payload` の中は見ない

<!-- prose points[reject.5].text -->
画面のログインと衛星の要求には、版の検査も知らない欄の検査も掛けない（画面は S-01 が配るので常に同じ版）

<!-- prose features[record].summary -->
収集アプリの記録は、溜めに積んだ時の版を 1 件ごとに持ち、書き出しにも出る

<!-- prose points[record.1].text -->
収集アプリは積む時点の版を 1 行ごとに焼き込み、記録の 1 件に `contract_version` として載せる。S-01 はそれを記録の表に残す。欄の無い 1 件は版 1

<!-- prose points[record.2].text -->
記録の版は、送った時の要求の見出しではなく、作った時の版。更新の前に積み、更新の後に送った記録は古い版のまま

<!-- prose points[record.3].text -->
収集アプリから届かなかった記録（画面・S-01 の中の書庫・滞在）の版は空。版だけが違う同じ記録は今どおり 1 件に畳まれ、最初の値が残る

<!-- prose points[record.4].text -->
ST50 より前に収集アプリから入った記録（論理ソースが `c01-` / `c02-`）とその前の版は、移行で版 1 に埋める。前の版の表の版は書き換えられない

<!-- prose points[record.5].text -->
外部サービス由来の更新で中身が入れ替わると、版も新しい 1 件の値になり、前の値は前の版の表に残る

<!-- prose points[record.6].text -->
書き出しの記録と前の版のファイルに列 `contract_version` が増え、説明書は「送信契約の版（スキーマ版とは別）」と説明する。今の列は消さない

<!-- prose features[released].summary -->
配った印の版は受け付け続け、送る形を変えたのに版を上げ忘れたら CI が落ちる

<!-- prose points[released.1].text -->
配った印（タグ `v*`。ベータ版を含む）で出した版は、以後のすべての S-01 が受け付ける。版 1 の封筒と口の形の例を試験の材料に置き、受け付けることを試験で固定する

<!-- prose points[released.2].text -->
版 1 は最初の印 `v*` で閉じる。それまでに足す欄（ST41 の終わりの時刻・ST49 の版・ST50 の版）は全部版 1。ベータ版の印も「配った」に入るので、最初のベータ版を打った後に欄を足す change は版を 2 に上げる

<!-- prose points[released.3].text -->
`tools/check-contract-version.sh` が、S-01 の型と収集アプリ 2 本が実際に直列化する欄から封筒の形の写しを作り、最後の配った印の写しと比べる。欄の追加・削除・型・必須の変化があるのに版が上がっていなければ落ちる

<!-- prose points[released.4].text -->
S-01・携帯・PC の 3 つの版の値の食い違い、版の履歴の行の欠け、印はあるのに写しを取り出せないことでも落ちる。意味だけの変化は機械で見えず、版の履歴に理由を書く規則にとどまる

<!-- prose features[hold].summary -->
送る契機ごとに S-01 の版を聞き、受け付けられる版の行だけを送る

<!-- prose points[hold.1].text -->
送る契機ごとに `GET /contract` を 1 回引き、版がその答え以下の行だけを、今の規則（溜めた順・まとめ送り）で送る。新しい版の行は捨てず、飛ばす印も付けずに残す

<!-- prose points[hold.2].text -->
確かめる口を持たない S-01（ST50 より前）には、版 1 の行も含めて何も送らない。新しい収集アプリを古い S-01 に繋ぐと、S-01 を更新するまで記録も生存信号も届かない

<!-- prose points[hold.3].text -->
ST50 より前に積んだ版を持たない行は版 1 として送る。426 と、1 件ごとの `unknown_field`・`contract_unsupported` は捨てずに残し、次の契機で確かめ直す

<!-- prose points[hold.4].text -->
携帯は、版で残した記録にも今の上限（積んでから 90 日・2 GB）を掛け、超えた分を捨てて破棄として報告する。PC の未送信には上限を置かず、捨てない

<!-- prose points[hold.5].text -->
S-01 を更新すると、次の契機から残っていた行が今の送り方で送られる。止まっていた間に付いた「飛ばす」印は捨て、先頭で待たせない。原本は、その記録を積んだ時の版で止まり、欠けにはならない

<!-- prose features[notify].summary -->
版で行を残している間、携帯は画面と通知、PC は通知で知らせる

<!-- prose points[notify.1].text -->
携帯: 画面に「サーバを更新してください」と、残している最も古い行を積んでからの日数を出す。通知は止まった時に 1 回、止まっている間は前に出してから 24 時間ごとに出し直す（置き換えて積み重ねない。再起動しても 24 時間経つまで出し直さない）

<!-- prose points[notify.2].text -->
PC: 画面が無いので Windows の通知だけ。文言に何日分を残しているかを出し、頻度は携帯と同じ。途切れの通知とは別の札で、互いに置き換えない

<!-- prose points[notify.3].text -->
更新を促すのは、版で止まった時（新しい版の行が残る・404・426・unknown_field / contract_unsupported）だけ。圏外・合言葉の不一致・サーバの失敗、それに 200 で読めない答え（公衆の Wi-Fi の入口の画面）では、行は残すが促さない

<!-- prose points[notify.4].text -->
止まった状態を抜けると通知と画面の行は消える。止まっている間の携帯の上限の 7 日前の知らせ（83 日）には「サーバを更新してください」が添えられる

<!-- prose impact.scenarios[S1].result -->
新しい欄を持つ記録は断られ続け、携帯に残る。「サーバを更新してください」が出るが、更新できる新しい S-01 は無い

<!-- prose impact.scenarios[S1].lost -->
直した S-01 を出すまで、その記録が S-01 に入らない。携帯では 90 日を超えた分が捨てられる

<!-- prose impact.scenarios[S2].result -->
版 1 で積んだ行は送られ、版 2 で積んだ行は残る。「サーバを更新してください」が出る

<!-- prose impact.scenarios[S2].lost -->
S-01 を版 2 以上へ戻し直すまで、版 2 の行が入らない。携帯では 90 日を超えた分が捨てられる

<!-- prose impact.scenarios[S3].result -->
記録も生存信号も届かない。携帯と PC に「サーバを更新してください」が出て、S-01 の稼働状況には途絶と出る（途絶の知らせも出うる）

<!-- prose impact.scenarios[S3].lost -->
S-01 を更新するまで記録が S-01 で見えない。携帯では 90 日を超えた分が捨てられる

<!-- prose impact.rules[R1].legit -->
S-01 を前の組み立てへ戻した本人の、戻す前に積んだ新しい版の行も止まる

<!-- prose impact.rules[R2].legit -->
収集アプリにだけ欄を足したリリースでは、同じリリースの最新の S-01 にも送れない

<!-- prose impact.rules[R3].legit -->
収集アプリは見出しを載せた行の最大にするので、正当な送り手を止める経路は見つけていない

<!-- prose impact.rules[R4].legit -->
収集アプリを先に更新した本人の、版 1 の記録と生存信号も S-01 を更新するまで止まる

<!-- prose impact.rules[R5].legit -->
版を上げて履歴の行を書けば通るので、正当な変更を止める経路は見つけていない（逆に見逃す経路は IMP-04）

<!-- prose impact.rules[R6].legit -->
DB に直に書く操作だけが止まる。取り込みと外部の更新は前の版を積むので止まらない

<!-- prose impact.rules[R7].legit -->
収集アプリは合言葉を持つので止まらない。画面と衛星は版を使わない

<!-- prose impact.rules[R8].legit -->
200 の読めない答えが S-01 の古さから来ているときも促さない（行は残るので失われない。D22 の反転条件）

<!-- prose impact.rules[R9].legit -->
足すだけなので、正当な利用を止める経路は見つけていない

<!-- prose impact.findings[IMP-01].title -->
収集アプリにだけ足した欄は、最新の S-01 にも断られ続ける

<!-- prose impact.findings[IMP-01].text -->
CI は版の上げ忘れを見るが、収集アプリが送る最上位の欄が S-01 の型に含まれるかは見ない。足し忘れると、同じリリースの S-01 が unknown_field で断り続け、「サーバを更新してください」は誤った案内になる

<!-- prose impact.findings[IMP-02].title -->
S-01 を前の版へ戻すと、新しい版で積んだ行は戻し直すまで送れない

<!-- prose impact.findings[IMP-02].text -->
行は積んだ時の版を持ち、S-01 は自分より新しい版を受け付けない。S-01 だけを契約の版の小さい組み立てへ戻すと、その間の新しい版の行は残り、携帯では 90 日を超えた分が捨てられる

<!-- prose impact.findings[IMP-03].title -->
収集アプリを先に入れ替えると、途絶の知らせと更新の知らせが同時に出る

<!-- prose impact.findings[IMP-03].text -->
S-01 を先に入れ替える順は design と送信契約の文書にだけある。逆の順だと、S-01 を更新するまで生存信号も届かず、稼働状況は途絶と出て、途絶の知らせと「サーバを更新してください」が並ぶ

<!-- prose impact.findings[IMP-04].title -->
正式版の修正の印の後は、版の上げ忘れを見逃しうる

<!-- prose impact.findings[IMP-04].text -->
検査は作成日の一番新しい印を「最後の配った印」として比べる。FR-105 は正式版にだけ修正を当てる枝を一時的に許すので、その印が幹のベータ版の印より後に打たれると、幹の欄の変化を古い版の写しと比べ、版の上げ忘れを通しうる

<!-- prose impact.findings[IMP-05].title -->
画面も配る古い S-01 では、更新の知らせが出ないまま行が溜まりうる

<!-- prose impact.findings[IMP-05].text -->
ST52 で S-01 が画面も配り、知らない道に画面の入口（200 の HTML）を返す形になると、ST50 より前のその S-01 は GET /contract に 404 でなく 200 の HTML を返しうる。収集アプリは送らずに残すが、200 の読めない答えでは更新を促さない（D22）ので、本人は止まった理由を知らされず、携帯は 90 日で捨てる（83 日の知らせは出る）

<!-- prose impact.scope.covers[0].text -->
収集アプリ 2 本が S-01 へ送るもの全部（記録・生存信号・破棄の報告・原本の欠け・原本）

<!-- prose impact.scope.covers[1].text -->
収集アプリの記録の版と、その書き出し

<!-- prose impact.scope.covers[2].text -->
配った印の後の、封筒の欄の変化と版の上げ忘れ

<!-- prose impact.scope.preconditions[0].text -->
版の検査・知らない欄の検査・記録の版は、収集アプリの資格情報（API の合言葉）で届く要求だけに掛かる

<!-- prose impact.scope.preconditions[1].text -->
知らない欄の検査は最上位の欄だけを見る（payload の中は見ない）

<!-- prose impact.scope.preconditions[2].text -->
送るのは S-01 の答えた版以下の行だけ。携帯の上限は今どおり掛かり、PC には上限を置かない

<!-- prose impact.scope.preconditions[3].text -->
更新を促すのは版で止まった時だけ（圏外・合言葉・サーバの失敗・200 の読めない答えでは促さない）

<!-- prose impact.scope.preconditions[4].text -->
版の比較は配った印（タグ v*）があるときだけ。印が無い間は欄を足しても版 1 のまま通る

<!-- prose impact.scope.preconditions[5].text -->
確かめる口は合言葉を持つ収集アプリだけに答える

<!-- prose impact.scope.excludes[0].text -->
画面のログインと衛星の要求

<!-- prose impact.scope.excludes[1].text -->
収集アプリ以外が書いた記録の版（空のまま）

<!-- prose impact.scope.excludes[2].text -->
意味だけの変化（欄の形が同じ）の検出。版の履歴に理由を書く規則にとどまる

<!-- prose impact.scope.excludes[3].text -->
書き出しの今の列の変更（足すだけ）

<!-- prose impact.scope.residual[0].text -->
携帯で版により残した行は、90 日・2 GB を超えると捨てられる

<!-- prose impact.scope.residual[1].text -->
収集アプリにだけ欄を足すと、最新の S-01 にも断られ続ける（IMP-01）

<!-- prose impact.scope.residual[2].text -->
S-01 を戻すと、新しい版の行は戻し直すまで残る（IMP-02）

<!-- prose impact.scope.residual[3].text -->
修正の枝の印の後は、版の上げ忘れを見逃しうる（IMP-04）

<!-- prose impact.scope.residual[4].text -->
画面も配る古い S-01 では、更新の知らせが出ないまま行が溜まりうる（IMP-05）

