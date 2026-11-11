# ST57 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
サーバ・携帯・PC の収集アプリに新しい版が出たら、本人が画面・携帯・PC の通知で気づけるようになる。確かめる通信は版の一覧を取るだけで、記録は 1 行も外へ出さない。確かめたくない部品は部品ごとに切れる。

<!-- prose impact.overview -->
3 つの部品（サーバの更新係・携帯・PC の収集アプリ）が既定で 1 日 1 回、配布元に版の一覧だけを取りに行き、新しい版を画面の帯・携帯の画面と通知・PC の通知で知らせる。この Story の後は、確認を切らない限り、3 つの部品から配布元へ毎日、問い合わせの元の IP と時刻が出る。サーバ本体は外へ送る部品を持たないまま（更新係は別のプログラム）。この分析では、PC で切った確認が前の版への切り戻しで戻りうること（IMP-01）と、サーバを前の版へ戻すと更新係を切る口・消す口が無くなること（IMP-02）、ST55 で入れた PC に新しい版の zip で走らせ直しても更新係は足されないこと（IMP-06。いまは消して入れ直す）を見つけた。

<!-- prose impact.problem -->
本人は配布元に新しい版を出しても、サーバ・携帯・PC の収集アプリのどれが古いままかに気づかない。とくに携帯の収集アプリは設定のときにしか画面を開かないので、直した不具合が手元に届いていないことに長く気づけない。

<!-- prose uses[U1].title -->
画面を開いたら、サーバに新しい版が出ていると分かる

<!-- prose uses[U1].trigger -->
いつものように稼働状況の画面を開く。配布元に 0.5.0 を出したことを本人は覚えていない。

<!-- prose uses[U1].outcome -->
サーバが古い版に取り残されていると分かり、入れ替える時期を決められる

<!-- prose uses[U1].without -->
配布元のページを自分で見に行かない限り、新しい版が出たことに気づかない

<!-- prose uses[U1].steps[0].do -->
稼働状況の画面を開く

<!-- prose uses[U1].steps[0].see -->
見出しの上に 1 行「更新あり 0.5.0（11/10 に見つけた）」

<!-- prose uses[U1].steps[1].do -->
設定の画面を開く

<!-- prose uses[U1].steps[1].see -->
「更新の確認」の節に「有効・新しい版あり」・入っている版 0.4.0・見つけた最新の版 0.5.0・最後に確かめた 11/10 09:12

<!-- prose uses[U1].steps[2].do -->
帯を押す

<!-- prose uses[U1].steps[2].see -->
何も起きない。入れるのは新しい zip を落としてインストーラを走らせ直す（押して入れる口は後の Story）

<!-- prose uses[U2].title -->
サーバが止まっている間も、携帯は自分の新しい版を知る

<!-- prose uses[U2].trigger -->
PC を数日つけていない（サーバも止まっている）。携帯の収集アプリには直したい不具合があり、新しい版を待っている。

<!-- prose uses[U2].outcome -->
サーバの状態によらず、携帯の新しい版に気づける

<!-- prose uses[U2].without -->
携帯の新しい版に気づくのは、配布元を自分で見たときだけになる

<!-- prose uses[U2].steps[0].do -->
何もしない（携帯の収集アプリが 1 日 1 回、自分で配布元に確かめる）

<!-- prose uses[U2].steps[0].see -->
通知「新しい版 0.5.0 が出ています」が 1 回

<!-- prose uses[U2].steps[1].do -->
通知を押す

<!-- prose uses[U2].steps[1].see -->
アプリの最初の画面に「更新あり 0.5.0（11/10 に見つけた）」の行

<!-- prose walkthroughs[W1].title -->
入っている版によって、どの版が「更新あり」になるか

<!-- prose walkthroughs[W1].scene -->
配布元の一覧に v0.4.1（正式版）・v0.5.0-beta.1（ベータ版）・nightly（開発版 0.5.0-dev.20261110）が並んでいる。入っている版の違う部品が、それぞれ一覧を読む。

<!-- prose walkthroughs[W1].why -->
入っている版の形（正式版・ベータ版・開発版）がチャンネルを決め、そのチャンネルのリリースだけを数える。ベータ版を追う設定は作らない。

<!-- prose walkthroughs[W1].figure.caption -->
入っている版 → 見つけた最新の版（版番号は説明用の値）

<!-- prose walkthroughs[W1].cases[0].result -->
どちらが出ても「更新あり」になる

<!-- prose walkthroughs[W1].cases[1].result -->
以後はベータ版を知らせない。ベータ版に戻りたいときは、配布元を自分で見る

<!-- prose walkthroughs[W1].exceptions[0].text -->
下書きのリリースと、自分の部品の配布物を持たないリリースは数えない

<!-- prose walkthroughs[W1].exceptions[1].text -->
説明用の値。実際の配布元のリリースはいま 0 件で確かめていない

<!-- prose walkthroughs[W2].title -->
PC で確認を切った後、起こし直しや前の版への切り戻しで切ったままか

<!-- prose walkthroughs[W2].scene -->
本人が PC の収集アプリで `update-check off` を走らせた。その後、PC を再起動する。別の日には、不具合で PC の収集アプリを 1 つ前の版に戻し、また新しい版に入れ直す。

<!-- prose walkthroughs[W2].why -->
PC の収集アプリは起動のたびに settings.json を自分の知っている欄だけで書き直す（今のコード）。前の版は切った値の欄を知らない。

<!-- prose walkthroughs[W2].figure.caption -->
切った値が置かれる settings.json の中身の移り変わり（仕様と今のコードから導いた筋書き）

<!-- prose walkthroughs[W2].cases[0].result -->
新しい版が起動時の書き直しで切った値を保てば、切ったまま

<!-- prose walkthroughs[W2].cases[1].result -->
切った値が消え、本人が気づかないまま確かめ始める（入れ替えの通知も、前に出したので出ない）

<!-- prose walkthroughs[W2].exceptions[0].text -->
携帯は前の版へ戻す入れ方が普通は無い（Android は版を下げる入れ替えを断る）ので、この筋書きは PC だけ

<!-- prose walkthroughs[W3].title -->
確かめられない日が続いたときに出るもの

<!-- prose walkthroughs[W3].scene -->
11/03 を最後に、サーバの更新係が配布元に届かない（網が切れている）。毎日確かめては失敗している。

<!-- prose walkthroughs[W3].why -->
確かめられない間は「更新あり」も出ないので、黙っていると「新しい版が無い」のと見分けられない。7 日を境に、確かめられていないことを同じ所に出す。

<!-- prose walkthroughs[W3].figure.caption -->
最後に確かめられた日からの日数と、画面の帯（日付は説明用の値）

<!-- prose walkthroughs[W3].cases[0].result -->
帯が消える。次に 7 日続けば、また出る

<!-- prose walkthroughs[W3].cases[1].result -->
OS は知らせない。48 時間で理由が「更新係が動いていない」に変わり、7 日で帯に出る

<!-- prose walkthroughs[W3].exceptions[0].text -->
携帯と PC は通知を、確かめられない日が続く 1 回の間に 1 度だけ出す

<!-- prose model.caption -->
3 つの部品がそれぞれ配布元に聞き、自分の所に出す

<!-- prose features[who].summary -->
サーバの更新係・携帯・PC の収集アプリの 3 つが、それぞれ配布元に直に聞く

<!-- prose points[who.1].text -->
3 つの部品がそれぞれ自分の版を確かめる。収集アプリはサーバに聞かないので、サーバが止まっていても自分の新しい版を知る

<!-- prose points[who.2].text -->
確かめるたびに、配布元（GitHub）には問い合わせの元の IP と時刻が残る。携帯は繋いでいる網で IP が変わるので、毎日の居場所の網が配布元に見えうる。中身は版の一覧の取得だけで、記録は出ない

<!-- prose points[who.3].text -->
携帯は Wi-Fi でも携帯の回線でも確かめる。1 回の通信は圧縮して 140 KB より小さい見込み

<!-- prose points[who.4].text -->
1 つの部品の確認を切っても、ほかの部品は確かめ続ける（サーバを切っても PC は確かめる）

<!-- prose features[query].summary -->
版の一覧を取る決まった形の GET だけ。自分の版も端末の情報も送らない

<!-- prose points[query.1].text -->
問い合わせは GET・本文なしで、見出しは固定の名前と値だけ（User-Agent は ashiato-update-check）。自分の版は送らず、取った一覧を手元で選ぶ。試験が受け手で中身を数えて固定する

<!-- prose points[query.2].text -->
配布元から見分けられるのは「開発版かどうか」だけ（開発版は印 nightly の 1 件を取るので道が違う）

<!-- prose points[query.3].text -->
宛先は組み立ての時の定数。配る組み立ては宛先を変える口を持たず、別の宛先への転送も追わない。差し替えは試験の組み立てだけで、自動の試験は本物の配布元に聞かない

<!-- prose points[query.4].text -->
配った版の宛先・読み方・証明書の確かめ方は、その版からは直せない。壊れたら 7 日で「確かめられていない」が出るのが気づく口になる

<!-- prose features[pick].summary -->
入っている版のチャンネル（正式版・ベータ版・開発版）で数えるリリースを決める

<!-- prose points[pick.1].text -->
正式版は正式版だけ、ベータ版はベータ版と正式版、開発版は印 nightly だけを数え、その中で最も新しい版を「見つけた最新の版」にする（例は W1）

<!-- prose points[pick.2].text -->
ベータ版の人が正式版を入れると、以後はベータ版を知らせない（ベータ版を追う設定は作らない）

<!-- prose points[pick.3].text -->
結果は「見つけた最新の版」で持ち、「更新あり」かは読む側がそのときの自分の版と比べる。入れ替えた後に古い結果で「更新あり」が残らない。3 つの部品は同じ材料で選び方を試す

<!-- prose points[pick.4].text -->
確かめるたびに 1 行を追記だけの記録に残す（時刻・成否・理由・見つけた版）。理由は 6 つに分ける。携帯の記録はクラウドのバックアップと端末の移し替えから外す

<!-- prose features[when].summary -->
最後に確かめてから 24 時間が過ぎた後の最初の機会に 1 回。最初の 1 回は切る口を見せた後

<!-- prose points[when.1].text -->
最後に確かめてから 24 時間が過ぎた後の最初の機会（起動か定期の時）に 1 回だけ。失敗した回も数えるので、同じ日に繰り返さない

<!-- prose points[when.2].text -->
最後に確かめた時刻が今より未来（時計が戻った）なら、期限が来たとみなして確かめる

<!-- prose points[when.3].text -->
最初の問い合わせは、切る口を本人が見た後にだけ出す: サーバはインストールの最後の表示の後、収集アプリは送り先を設定した後。確認の無い前の版から入れ替えた収集アプリは、切り方を通知で 1 回示してから、次の機会に確かめる

<!-- prose features[updater].summary -->
サーバと別のプログラム・別のサービス。DB にも状態の置き場にも触れない

<!-- prose points[updater.1].text -->
サーバの版は、別の実行ファイル ashiato-updater.exe がサービス専用のアカウントで確かめる。DB の部品を持たず、状態の置き場を読めない。サーバは更新係の部品に依らず、外へ送る部品を持たないまま

<!-- prose points[updater.2].text -->
更新係は結果を自分の置き場（C:\ProgramData\ashiato-updater）に書き、サーバはそこを読むだけ（書けない）

<!-- prose points[updater.3].text -->
サーバの Windows の zip に更新係が入り、インストールで 3 つめのサービスとして登録される。配布物の依存の一覧に更新係の部品も載る

<!-- prose points[updater.4].text -->
Linux 向けのサーバと開発の形のサーバは更新係を持たない。設定に「動いていない」と出て、帯は出ない

<!-- prose features[off].summary -->
部品ごとに切れる。サーバはインストーラの命令、携帯は設定のスイッチ、PC は命令

<!-- prose points[off.1].text -->
サーバは `ashiato-setup.exe update-check off` / `on`（管理者の確認 1 回）。切ると更新係のサービスを止めて無効にする。画面から切る口は作らない

<!-- prose points[off.2].text -->
携帯は設定の画面の「新しい版を確かめる」のスイッチ。PC は `update-check off` / `on` と、setup の最後の問い「新しい版を確かめますか [Y/n]」（既定は確かめる）。設定の文字列には入れないので、携帯と PC を別々に切れる

<!-- prose points[off.3].text -->
既定は有効。切らない限り、入れた日から確かめる

<!-- prose points[off.4].text -->
サーバの切った確認はインストーラの走らせ直しでも新しい版への入れ替えでも戻さない。`repair` は更新係に触れない

<!-- prose points[off.5].text -->
更新係の無い前のインストールで走らせ直すと、更新係が足され、確認は有効になる。消す口は更新係のサービスも消し、更新係の置き場（確かめた記録）は残す

<!-- prose features[web].summary -->
どの画面でも見出しの上の帯に 1 行。設定の画面に「更新の確認」の節

<!-- prose points[web.1].text -->
新しい版があれば、どの画面でも見出しの上に「更新あり 0.5.0（11/10 に見つけた）」の帯が 1 行出る。無ければ帯は出ず、見出しの位置も変わらない

<!-- prose points[web.2].text -->
帯を押しても何も起きない（落として入れる口は後の Story）

<!-- prose points[web.3].text -->
帯を出さないとき: 確認を切ってある（設定に「切ってある」）・まだ確かめていない（設定に「まだ確かめていない」）・結果を読めない（設定に「結果を読めない」）・更新係が無い（設定に「動いていない」）

<!-- prose points[web.4].text -->
帯と節の中身は、サーバの新しい口 GET /update-status から取る。画面のログインが要り、API の合言葉では答えない。画面から外の宛先へは要求を出さない

<!-- prose points[web.5].text -->
更新係の結果のファイルは信用しない入力として読む: 64 KB を超える・版や時刻の形でない値を持てば、中身を画面に出さず「結果を読めない」にする

<!-- prose points[web.6].text -->
帯は 360 px の幅で横に流れず、文字と地のコントラストは 3:1 以上

<!-- prose features[notify].summary -->
携帯は最初の画面の行と通知、PC は通知。どちらも新しい版ごとに 1 回

<!-- prose points[notify.1].text -->
携帯は最初の画面に「更新あり <版>（<月>/<日> に見つけた）」の行を出し、まだ知らせていない版なら通知「新しい版 <版> が出ています」を 1 回出す。通知を押すと最初の画面が開く

<!-- prose points[notify.2].text -->
PC は画面を持たないので、通知「新しい版 <版> が出ています」を新しい版ごとに 1 回だけ出す。起こし直しても同じ版では出さない。文はほかの知らせと分ける

<!-- prose points[notify.3].text -->
PC の通知を見逃すと、次の新しい版まで PC には出ない（画面にはサーバの帯だけが出続ける）

<!-- prose features[stale].summary -->
7 日確かめられなければ「更新を確かめられていない」を、「更新あり」と同じ所に出す

<!-- prose points[stale.1].text -->
確認が有効で、最後に確かめられてから 7 日が過ぎたら、最後に確かめられた日と理由を付けて出す: 画面の帯・携帯の最初の画面の行と通知 1 回・PC の通知 1 回（例は W3）

<!-- prose points[stale.2].text -->
携帯と PC の通知は、確かめられない日が続く 1 回の間に 1 度だけ。確かめられたら、次に続いたときにまた出す。確認を切った部品は出さない

<!-- prose points[stale.3].text -->
サーバの更新係が 48 時間より長く確かめていなければ、理由は「更新係が動いていない」になる（OS はサービスが止まったことを知らせない）

<!-- prose points[stale.4].text -->
新しい版と確かめられていないが同時なら、帯は 2 行になる

<!-- prose impact.scenarios[S1].result -->
更新係の実行ファイルは消えるが、サービス ashiato-updater の登録は残り、起動に失敗し続ける。前の版のインストーラには update-check の口も無い

<!-- prose impact.scenarios[S1].lost -->
前の版のインストーラだけで更新係を切る・消すこと

<!-- prose impact.scenarios[S2].result -->
切った値（settings.json）は消え、入れ替えの通知の印（update-check.json の intro_shown）は残る。新しい版に入れ直すと、既定の「有効」で、通知を出さずに確かめ始める

<!-- prose impact.scenarios[S2].lost -->
本人が切ったことを、版の行き来の後も保つこと

<!-- prose impact.rules[R1].legit -->
社外へ出たくない端末も切らない限り外へ出る（切る口で止められる）

<!-- prose impact.rules[R2].legit -->
この分析では、宛先を変えたい正当な利用を見つけていない（本人 1 人の運用。リポジトリの名前の変更は同じ api.github.com の中の転送で追う）

<!-- prose impact.rules[R3].legit -->
この Story の範囲では、更新係が記録を読む正当な理由は無い（入れる役は後の Story が設計する）

<!-- prose impact.rules[R4].legit -->
サーバを前の版へ戻した人は、前の版のインストーラにこの命令が無いので、そのインストーラでは切れない

<!-- prose impact.rules[R5].legit -->
PC の収集アプリを前の版へ戻すと、前の版が settings.json を書き直して切った値が消える

<!-- prose impact.rules[R6].legit -->
この分析では、API の合言葉で更新の状態を読みたい利用を見つけていない

<!-- prose impact.rules[R7].legit -->
前の版のインストーラで消すと、この規則は働かない（IMP-02）

<!-- prose impact.rules[R8].legit -->
網が戻った直後でも、24 時間たつまで確かめ直さない（翌日まで待つ）

<!-- prose impact.rules[R9].legit -->
通知を見逃すと、次の新しい版まで携帯と PC には通知が出ない（携帯は最初の画面の行、画面はサーバの帯で見える）

<!-- prose impact.rules[R10].legit -->
この分析では、共有のアカウントで動かしたい正当な利用を見つけていない

<!-- prose impact.findings[IMP-01].title -->
PC で切った確認が、settings.json の書き直しで戻りうる

<!-- prose impact.findings[IMP-01].text -->
PC の収集アプリは起動のたびに settings.json を自分の知っている欄だけで書き直す。前の版へ戻すと切った値が消え、新しい版に入れ直すと本人が気づかないまま確かめ始める。新しい版の書き直しが切った値を書き戻すことも、起こし直しを含む Scenario では固定していない

<!-- prose impact.findings[IMP-02].title -->
サーバを前の版へ戻すと、更新係を切る口と消す口が無くなる

<!-- prose impact.findings[IMP-02].text -->
前の版のインストーラは更新係を知らない。前の版で消すと、DB とサーバのサービスとプログラムの置き場は消えるが、更新係のサービスの登録が残り、実行ファイルの無いまま起動に失敗し続ける。前の版のインストーラには update-check の口も無い（前の版で走らせ直すと、版違いで何も替えずに止まる）

<!-- prose impact.findings[IMP-03].title -->
帯が出ている間、稼働状況の高さの予算に帯の分が足される

<!-- prose impact.findings[IMP-03].text -->
稼働状況の正典は「開いた直後に 2 ソースの直近 4 週」「5 ソースがひとスクロール以内」を求め、予算から除くのは直近の書庫の箱だけ。帯（1〜2 行）が出ると中身がその分だけ下がる

<!-- prose impact.findings[IMP-04].title -->
配る zip を入れる録画で、更新係が本物の配布元に問い合わせうる

<!-- prose impact.findings[IMP-04].text -->
サーバの zip を入れる録画があれば、録画のたびに更新係が本物の配布元に 1 回問い合わせうる。確かめると、scenarios.d にサーバの zip を入れる台本は見つからず、Windows の録画は外への接続を切って走る

<!-- prose impact.findings[IMP-05].title -->
携帯の回線の共有の IP で、問い合わせの上限に当たりうる

<!-- prose impact.findings[IMP-05].text -->
配布元の上限は IP ごとに 1 時間 60 回。携帯の回線では多くの利用者が 1 つの IP を共有することがあり、ほかの人の問い合わせと合わせて上限に当たると、その日は「問い合わせの上限」で確かめられない。続けば 7 日で「確かめられていない」が出る

<!-- prose impact.findings[IMP-06].title -->
ST55 で入れた PC に新しい版の zip で走らせ直しても、更新係は足されない

<!-- prose impact.findings[IMP-06].text -->
いまのインストーラは、プログラムの置き場に違う版のファイルがあると「別の版が入っています。版を替える口はまだ無い」で何も替えずに止まる。仕様の「更新係の無いインストールを走らせ直すと、更新係が足される」は同じ版の zip で試す形で、本人の実際の道（ST55 の版の上に新しい版の zip）では通らない。design の移行の計画（走らせ直しで足される）もこの道を前提にしている

<!-- prose impact.scope.covers[0].text -->
Windows のサーバ（更新係）・携帯の収集アプリ・PC の収集アプリの更新の確認と知らせ

<!-- prose impact.scope.preconditions[0].text -->
確かめるのは、その部品の確認が有効なときだけ。最初の問い合わせは切る口を見せた後

<!-- prose impact.scope.preconditions[1].text -->
数えるのは自分のチャンネルのリリースのうち、自分の部品の配布物を持つものだけ（下書きを除く）

<!-- prose impact.scope.preconditions[2].text -->
宛先の差し替えは試験の組み立てだけ。送るのは版の一覧の取得だけ

<!-- prose impact.scope.preconditions[3].text -->
携帯は前景の収集のサービスが動いているときだけ確かめる

<!-- prose impact.scope.preconditions[4].text -->
収集アプリが聞くのは配布元だけ（サーバには聞かない）。サーバの版を聞くのは更新係だけで、更新係が書けるのは自分の置き場だけ

<!-- prose impact.scope.preconditions[5].text -->
記録は追記だけ。切る口はサーバではインストーラの命令だけ、携帯と PC ではその端末の中の値だけ

<!-- prose impact.scope.preconditions[6].text -->
帯を出すのは新しい版か 7 日確かめられないときだけ。通知は新しい版ごと・確かめられない日が続く 1 回ごとに 1 度だけ

<!-- prose impact.scope.preconditions[7].text -->
インストールは管理者の確認 1 回だけ。走らせ直しは足りないものだけを足す。配布物は要るものだけを持ち、依存の一覧に更新係の部品を足す

<!-- prose impact.scope.excludes[0].text -->
Linux 向けのサーバ・開発の形のサーバの更新係（設定に「動いていない」と出るだけ）

<!-- prose impact.scope.excludes[1].text -->
新しい版を落として入れる操作（後の Story）

<!-- prose impact.scope.residual[0].text -->
確かめるたびに配布元に IP と時刻が残る（切れば止まる）

<!-- prose impact.scope.residual[1].text -->
前の版への切り戻しで、PC の切った確認が戻りうる・サーバの更新係の登録が残る（IMP-01・IMP-02）。既存のサーバに更新係を足すには、いまは消して入れ直す（IMP-06）

