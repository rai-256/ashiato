# ST55 の GO 資料の文（推敲用）

`<!-- prose … -->` の行は変えない・消さない・並べ替えない。各行の下の文だけを推敲する。

<!-- prose purpose -->
Windows でサーバのインストーラを 1 回走らせるだけで、Docker Desktop も WSL も入れずに、データベースとサーバが PC の起動時から動くようになる。最後に画面の合言葉を決めれば、そのまま画面にログインして使い始められる。

<!-- prose impact.overview -->
Windows でインストーラを 1 回走らせると、同梱の PostgreSQL 17 とサーバがサービスとして入り、ログオンしていなくても PC の起動時から動く（携帯から届くかは Tailscale の設定に依る）。画面の合言葉は最後に端末で決め、照らすための値だけを持つ。一方で、インストールした形では本人は記録の置き場に書けず（消すには管理者の操作が要る）、取り込み待ちの場所が変わり、合言葉を忘れたら管理者として決め直すことになる。開発の台本と Docker の DB はそのまま残るが、その記録は移さない。

<!-- prose impact.problem -->
いまサーバは開発の台本と WSL の中の Docker でしか動かず、PC を起こすたびに本人が起こしている。画面の合言葉は `.env` に元の値で書くしかなく、人に渡せる形が無い。Docker Desktop を前提にすると、利用者は数 GB の常駐と WSL を入れることになり、おまけと見て片付けると記録の置き場ごと消えうる。

<!-- prose uses[U1].title -->
本人が自分の環境を、配った版のサーバに切り替える

<!-- prose uses[U1].trigger -->
いまは PC を再起動するたびに Docker と開発の台本でサーバを起こしている。起こし忘れた日は画面が開かず、Docker Desktop も重い。配った版を、PC が起きたら勝手に動く形で使いたい。

<!-- prose uses[U1].outcome -->
Docker も開発の台本も使わずに、PC が起きている間はサーバが動く形になる。収集アプリを繋ぐのは ST56 から。

<!-- prose uses[U1].without -->
サーバは開発の台本と Docker（WSL の中）でしか動かず、画面の合言葉を決める口も無い。人に渡すこともできない。

<!-- prose uses[U1].steps[0].do -->
リリースから `ashiato-server-<版>-windows-x86_64.zip` を落として展開し、`ashiato-setup.exe` を開く

<!-- prose uses[U1].steps[0].see -->
端末の窓が開き、管理者の確認（UAC）が 1 回出る

<!-- prose uses[U1].steps[1].do -->
「はい」を押す

<!-- prose uses[U1].steps[1].see -->
新しい端末の窓で、置き場・DB・サービスを作る段が進む。C: の空きが 40 GB より少なければその知らせ

<!-- prose uses[U1].steps[2].do -->
最後に画面の合言葉を 2 回入れる

<!-- prose uses[U1].steps[2].see -->
既定のブラウザで `http://127.0.0.1:18700/` が開き、ログインの欄が出る

<!-- prose uses[U1].steps[3].do -->
決めた合言葉でログインする

<!-- prose uses[U1].steps[3].see -->
空の画面（Docker の DB の記録は移らない）。端末には Tailscale の転送先を 18700 にするコマンドと「無人で動かす」の入れ方

<!-- prose uses[U1].steps[4].do -->
PC を再起動し、ログオンせずに置いておく

<!-- prose uses[U1].steps[4].see -->
サービスは自動で起きている（携帯から届くかは Tailscale の設定次第）

<!-- prose uses[U2].title -->
画面の合言葉を忘れた

<!-- prose uses[U2].trigger -->
しばらく携帯のブラウザのログインだけで見ていて、PC のブラウザで合言葉を聞かれたが思い出せない。

<!-- prose uses[U2].outcome -->
記録と収集アプリの繋がりを保ったまま、画面にまた入れる。

<!-- prose uses[U2].without -->
設定は本人も書けない置き場にあるので、手で直す口が無い。

<!-- prose uses[U2].steps[0].do -->
`ashiato-setup.exe password` を開く

<!-- prose uses[U2].steps[0].see -->
管理者の確認が 1 回出る

<!-- prose uses[U2].steps[1].do -->
新しい合言葉を 2 回入れる

<!-- prose uses[U2].steps[1].see -->
サーバが起こし直され、ブラウザで画面が開く

<!-- prose uses[U2].steps[2].do -->
新しい合言葉でログインする

<!-- prose uses[U2].steps[2].see -->
記録はそのまま。携帯のブラウザのログインは切れていて、もう一度ログインが要る

<!-- prose walkthroughs[W1].title -->
インストールを走らせ直したとき

<!-- prose walkthroughs[W1].scene -->
本人がインストールを途中で閉じた・入れ直した・消した後にもう一度入れる。置き場に何が残っているかで、インストーラの振る舞いが分かれる。

<!-- prose walkthroughs[W1].why -->
秘密を上書きすると、DB に繋げなくなり、収集アプリの資格情報も切れる。秘密を先に書いてから DB を作るので、普通の途中の失敗は「秘密はあるのに DB が無い」側に倒れる。

<!-- prose walkthroughs[W1].figure.caption -->
走らせ直したときの置き場の状態と結果

<!-- prose walkthroughs[W1].cases[0].result -->
前の記録の上で続き、秘密も合言葉も変わらない。収集アプリの資格情報も切れない

<!-- prose walkthroughs[W1].cases[1].result -->
走らせ直しで DB を作り、同じ秘密で役割を作る

<!-- prose walkthroughs[W1].cases[2].result -->
走らせ直しは DB と秘密が食い違うと示して止まり、`repair` の口を示す。`repair` で記録を残したまま動かし直せるが、収集アプリは繋ぎ直しが要り、画面の合言葉も決め直す

<!-- prose walkthroughs[W1].exceptions[0].text -->
本人は置き場に書けないので、設定のファイルを消すには管理者として所有権を取る操作が要る

<!-- prose walkthroughs[W2].title -->
Takeout をどこに置けば入るか

<!-- prose walkthroughs[W2].scene -->
本人がインストールした形に切り替えた後、Google Takeout の書庫を落として取り込ませる。

<!-- prose walkthroughs[W2].why -->
サービスのアカウントで動くと、既定の場所は本人でなくサービスのプロファイルを指す。インストーラが本人の場所を控えて渡し、ドキュメント（OneDrive に上がりうる）を避けてプロファイルの直下にした。

<!-- prose walkthroughs[W2].figure.caption -->
置いた場所と、取り込まれるか

<!-- prose walkthroughs[W2].cases[0].result -->
サーバのアカウントがダウンロードを読み、取り込む。ダウンロードの書庫は動かさない

<!-- prose walkthroughs[W2].cases[1].result -->
インストールした形はそこを見ないので、取り込まれないまま残る

<!-- prose walkthroughs[W2].exceptions[0].text -->
`docs/archive-inbox.md` をインストールした形の場所に書き直す

<!-- prose walkthroughs[W3].title -->
置き場の中のファイルを触る

<!-- prose walkthroughs[W3].scene -->
本人が `C:\ProgramData\ashiato\export` の書き出しの zip を USB に写し、その後で古い原本を消して空きを作ろうとする。同じ PC の別の利用者も覗こうとする。

<!-- prose walkthroughs[W3].why -->
設定のファイルには DB の合言葉も入る。置き場は 2 つのサービスだけが書け、本人は読むだけにした（同じ PC のほかのプログラムが本人として動いても、書き換えられない）。

<!-- prose walkthroughs[W3].figure.caption -->
誰が何をしようとすると、どうなるか

<!-- prose walkthroughs[W3].cases[0].result -->
読みの権限で写せる

<!-- prose walkthroughs[W3].cases[1].result -->
消せない。管理者として所有権を取る手順（docs/install.md）が要る

<!-- prose walkthroughs[W3].exceptions[0].text -->
開発の形のサーバの置き場（本人の `%LOCALAPPDATA%`）は変わらない

<!-- prose model.caption -->
インストーラが置くもの（矢印は「誰が書けるか」）

<!-- prose features[install].summary -->
zip を展開して `ashiato-setup.exe` を開くと、管理者の確認が 1 回出て、DB とサーバが自動で起きる Windows のサービスとして入る。

<!-- prose points[install.1].text -->
入れるのはサーバとデータベースだけ。PC の収集アプリは今までどおり別の zip で入れる。インストーラはサーバの Windows の zip に同梱される。

<!-- prose points[install.2].text -->
2 つのサービスは開始の種類が「自動」で、利用者でなくサービス専用のアカウント（合言葉を持たない）で動く。サーバは DB が起きてから起き、異常に終わればどちらも起こし直される。

<!-- prose points[install.3].text -->
サーバは `ashiato-server service` の口で動き、DB が接続を受けるまで最長 120 秒待つ。起動のたびには移行を当てない（当てるのはインストールのときだけ）。

<!-- prose points[install.4].text -->
管理者の確認は、インストーラが開いた後に自分を管理者として起こし直すときの 1 回。開いた時点では上がらず、開いた利用者を「本人」として先に控える（別の管理者の資格情報で上がっても、その管理者は本人にならない）。

<!-- prose points[install.5].text -->
サービス専用のアカウントで登録できなければ、ほかのサービスと共有のアカウントへ倒さずに止まる。番号（55400・18700）が塞がっていても、登録せずに止まり、塞いでいるプログラムを示す。

<!-- prose points[install.6].text -->
インストールの間はネットワークに出ない。同梱する PostgreSQL は版と指紋を固定した公式の zip から要るものだけを取り、管理画面（pgAdmin）は入れない。配る組み立ては試験の差し込みを持たない。

<!-- prose features[db].summary -->
同梱の PostgreSQL 17 で、OS に依らない文字の並べ方・合言葉の認証・手元だけの待ち受けで作る。

<!-- prose points[db.1].text -->
DB は PostgreSQL 17（開発・試験の DB と同じメジャー版）で、`C:\ProgramData\ashiato\db\17` に置く。版ごとの置き場はいつかメジャー版を上げるときの移し方を残しておくため。

<!-- prose points[db.2].text -->
文字の並べ方は OS の地域の設定に任せず、符号の順（`builtin` の `C.UTF-8`）にする。日本語の Windows でも、開発の DB と同じ並びになる。作った後に並べ方を変えるには全部を入れ直すことになる。

<!-- prose points[db.3].text -->
どの接続にも合言葉を求め（`trust` を残さない）、`127.0.0.1` の 55400 番だけで待ち受ける。ログには文と値を書かない（製造準備の既定値と、エラーになった文の字面も書かない設定）。

<!-- prose points[db.4].text -->
インストーラが DB の 3 つの役割（管理者・表の所有者・実行時）を開発の DB と同じ SQL で作り、移行を表の所有者で当てる。移行の前の退避は `backups\pre-migrate` に同梱の `pg_dump` で取る。移行が失敗すると、サーバのサービスを開始せずに止まる。

<!-- prose features[place].summary -->
記録・原本・書き出し・退避・ログ・設定は全部 `C:\ProgramData\ashiato` に置き、書けるのは 2 つのサービスのアカウントだけ、本人は読むだけ。

<!-- prose points[place.1].text -->
状態の置き場は親から権限を受け継がず、2 つのサービスのアカウントに読み書き、インストールした本人に読みだけを付ける。ほかの利用者は読めない。

<!-- prose points[place.2].text -->
本人も置き場に書けない。書き出しの zip や写真の原本を開く・写すことはできるが、消す・動かすには管理者として所有権を取る（手順は `docs/install.md`）。

<!-- prose points[place.3].text -->
インストールした形では、書き出しの既定の置き場は `C:\ProgramData\ashiato\export`、写真の原本は `…\media`。開発の台本で起こすサーバの既定（本人の `%LOCALAPPDATA%`）は変えない。

<!-- prose points[place.4].text -->
PC の管理者は所有権を取れば置き場を読める。ほかの管理者からは守れない（要件の範囲の外の残るリスク）。

<!-- prose points[place.5].text -->
インストール・合言葉の決め直し・消すの間だけ、上がった利用者に書きを足し、終わりに外す。走らせ直しで本人は変わらない。

<!-- prose points[place.6].text -->
サービスにした DB とサーバのログは置き場の `logs` のファイルに書く。DB のログ（`logs\db`）は DB のサービスが書く。送った文の字面と秘密はイベントログに出さないが、DB がファイルへ切り替わる前の起動の数行とサービスの制御の知らせはイベントログに出る。DB は 7 日分、サーバは 30 日分残る。起動を拒んだ理由もサーバのログに残る。

<!-- prose features[secrets].summary -->
DB の合言葉・API の合言葉・ログインの印の鍵はインストーラが乱数で作り、置き場の設定のファイルにだけ置く。人は見ない。

<!-- prose points[secrets.1].text -->
秘密は OS の暗号の乱数で作り、インストールごとに違う。画面・端末・ログには出さない。

<!-- prose points[secrets.2].text -->
サービスとして動くサーバは、設定と秘密を `settings\server.json` から読む。サービスの登録簿（同じ PC のほかのアカウントが読める）には秘密を書かない。開発の台本で起こすサーバは今までどおり環境変数を読む。

<!-- prose points[secrets.3].text -->
設定のファイルが無い・読めない・形が違うと、サーバは理由をログに書いて起動しない。DB が 120 秒開かなければ、理由を書いて終わる（サービスの回復が起こし直す）。

<!-- prose points[secrets.4].text -->
API の合言葉は本人に見せない。いまの開発のサーバの `API_TOKEN` も引き継がない。収集アプリを配った版のサーバに繋ぐのは ST56 から。

<!-- prose features[password].summary -->
インストールの最後に端末で 2 回打って決め、ブラウザで画面が開く。忘れたら管理者として `ashiato-setup.exe password`。

<!-- prose points[password.1].text -->
インストールの最後に、端末で合言葉を 2 回入れる（表示されない）。決め終えるとサーバが起こし直され、既定のブラウザで `http://127.0.0.1:18700/` が開いて、その合言葉でログインできる。

<!-- prose points[password.2].text -->
16 文字より短い・`change-me` で始まる・2 回が一致しない・API の合言葉かログインの印の鍵と同じ、のどれかなら受け付けず、理由を示して入れ直させる。

<!-- prose points[password.3].text -->
合言葉の元の値はどこにも残さず、塩を付けた照らすための値（Argon2id）だけを設定に置く。設定に元の値があったり、照らすための値が無かったりすると、サーバは起動しない。

<!-- prose points[password.4].text -->
忘れたら `ashiato-setup.exe password` で決め直す。管理者の確認が 1 回出る。決め直すと、それまでのログイン（携帯のブラウザも）は全部切れる。記録とほかの秘密は変わらない。

<!-- prose points[password.5].text -->
ログインの印の材料はインストールした形では照らすための値に替わる（開発の形の印は ST44 のまま）。同じ合言葉で決め直しても塩が変わるので、印は切れる。

<!-- prose features[takeout].summary -->
サーバは本人のダウンロードを読み、`%USERPROFILE%\ashiato\取り込み待ち` を読み書きする。

<!-- prose points[takeout.1].text -->
インストーラは本人の「ダウンロード」にサーバのアカウントの読みだけを足し、取り込み待ちを本人のプロファイルの直下に作って読み書きを足す。ほかの本人のフォルダには権限を足さない。

<!-- prose points[takeout.2].text -->
インストールした形の取り込み待ちは「ドキュメント」の下でなく `%USERPROFILE%\ashiato\取り込み待ち`。ドキュメントは OneDrive に上がりうるので避けた。開発の形の既定（ドキュメントの下）に置いた書庫は、インストールした形では見ない。

<!-- prose points[takeout.3].text -->
取り込み待ちを本人が消しても、サーバが作り直す。取り込みの利用者は収集アプリ・画面と同じ既定の利用者で、インストールごとに作り直さない。

<!-- prose features[rerun].summary -->
走らせ直しは足りないものだけを足す。消すとプログラムとサービスは消え、記録の置き場は残る。

<!-- prose points[rerun.1].text -->
もう一度走らせると、既にある DB・秘密・設定・画面の合言葉は上書きせず、足りないもの（サービス・置き場・権限・プログラム）だけを足す。秘密を書いてから DB を作るので、途中で落ちても走らせ直しで DB ができる。

<!-- prose points[rerun.2].text -->
DB はあるのに設定のファイル（秘密）が無いと、走らせ直しは何も書き換えずに止まり、出口の `ashiato-setup.exe repair` を示す。走らせ直しのまま秘密を作り直すことはしない。

<!-- prose points[rerun.4].text -->
`repair` は管理者の確認を 1 回出し、2 つのサービスを止め、秘密を全部作り直して DB の 3 つの役割の合言葉を替えてから設定を置き、サービスを開始する。記録の行・移行の記録・原本・書き出しには触らない。止まった状態でなければ何も変えずに終わる。前の `repair` が途中で落ちて一時の DB が残っていれば、最初にそれを止めて片付けてから作り直す。本人は置き場の権限から読むので、別の管理者が走らせても本人は変わらない。

<!-- prose points[rerun.5].text -->
`repair` の後は API の合言葉が新しくなるので、収集アプリは繋ぎ直しが要る（前の合言葉は断られる）。画面の合言葉も最後に決め直す。

<!-- prose points[rerun.6].text -->
`repair` の間だけ、上がった利用者の Windows のアカウントだけが合言葉なしで入れる一時の入口を `127.0.0.1` の一時の番号に開き、終わりに閉じる（一時の接続の規則と番号は残らない）。入る先の DB の管理者の役割は、DB を作るときに固定した名前 `ashiato` で、設定のファイルが無くても分かる。別の管理者が走らせても同じ役割に入る。

<!-- prose points[rerun.3].text -->
Windows の「アプリと機能」に載り、消すとサービス・プログラム・登録と、ダウンロードと取り込み待ちに足した権限が消える。記録の置き場と取り込み待ちの中身は残り、その場所を示す。入れ直すと前の記録の上で続く。

<!-- prose features[notice].summary -->
Tailscale の設定は変えずに転送先と「無人で動かす」を示し、C: の空きが 40 GB より少なければ知らせる。

<!-- prose points[notice.1].text -->
インストーラは Tailscale に触らない。最後に、ログオンしていなくても携帯から届くための「無人で動かす（Run unattended）」の入れ方と、`tailscale serve` の転送先を 18700 にするコマンドを示す。

<!-- prose points[notice.2].text -->
置き場は `C:\ProgramData\ashiato` に固定（選ばせない）。C: の空きが初年度の見積もり（40 GB）より少なければ知らせ、インストールは続ける。

<!-- prose points[notice.3].text -->
配る zip からのインストールを Windows の CI で毎回確かめる（サービスの種類・置き場の権限・DB の設定・移行・ログイン・Takeout）。本物の管理者の確認と画面は確認バッチで Windows サンドボックスの中の録画で見る。

<!-- prose points[notice.4].text -->
「再起動するとログオンしなくても動いている」は、サンドボックスで再起動の後に状態が残るときだけ実物で見る。残らなければ CI の「自動・サービスのアカウント」で代え、確認バッチに見ていないと書く。

<!-- prose impact.scenarios[S1].result -->
アクセスが拒否される。消すには管理者として所有権を取る手順（docs/install.md）が要る

<!-- prose impact.scenarios[S1].lost -->
本人の権限だけで置き場のファイルを片付けること

<!-- prose impact.scenarios[S2].result -->
取り込まれず、置いた場所に残る。画面の取り込みの状態にも出ない

<!-- prose impact.scenarios[S2].lost -->
前の場所に置いた書庫が入ること（ダウンロードに置けば入る）

<!-- prose impact.scenarios[S3].result -->
インストーラは食い違いと置き場の場所と `repair` の口を示して止まる。本人が `repair` を走らせると、記録を残したまま秘密と役割の合言葉が作り直され、最後に画面の合言葉を決め直す

<!-- prose impact.scenarios[S3].lost -->
`repair` を走らせるまでサーバは止まり、携帯の未送信は溜まる（90 日・2 GB を超えた分から捨てられ始める）。`repair` の後は収集アプリの繋ぎ直しが要る

<!-- prose impact.scenarios[S4].result -->
管理者の確認で止まり、決め直しに進めない

<!-- prose impact.scenarios[S4].lost -->
管理者でない本人が自分で合言葉を取り戻すこと

<!-- prose impact.scenarios[S5].result -->
DB のサービスが起きられず、サーバは 120 秒待って終わる。回復が起こし直すかは IMP-06 に依る（推測。確かめていない）

<!-- prose impact.scenarios[S5].lost -->
その起動の間、携帯から記録を受け取ること（携帯は溜めて送る）

<!-- prose impact.rules[R1].legit -->
この分析では、正当な利用を止める場面を見つけていない（専用のアカウントが使えない Windows では入らない。CI の最初の項目で確かめる）

<!-- prose impact.rules[R2].legit -->
DB の起動が 120 秒より遅い PC では、サーバは一度終わる。その後に回復が起こし直すかは、登録の仕方に依り確かめていない（IMP-06）

<!-- prose impact.rules[R3].legit -->
本人も置き場のファイルを消す・動かせない（管理者の操作が要る）

<!-- prose impact.rules[R4].legit -->
この分析では、正当な利用を止める場面を見つけていない（インストーラが照らすための値を書くので、手で設定を書かない限り当たらない）

<!-- prose impact.rules[R5].legit -->
16 文字より短い合言葉を使いたい本人は、長くするまで決められない（入れ直させるだけで止めはしない）

<!-- prose impact.rules[R6].legit -->
管理者の資格情報を持たない本人は、忘れた合言葉の決め直しに進めない

<!-- prose impact.rules[R7].legit -->
この状態に当たった本人は、走らせ直しでは先へ進めず、`repair` を走らせることになる。`repair` の後は収集アプリの繋ぎ直しが要る

<!-- prose impact.rules[R8].legit -->
走らせ直しで合言葉を変えたい本人は、`password` の口を使う（走らせ直しでは変わらない）

<!-- prose impact.rules[R9].legit -->
その番号を使うほかのプログラムがある PC では、そのプログラムを止めるまで入らない（番号を選ぶ口は無い）。予約された番号の範囲はこの検査で見えない（IMP-02）

<!-- prose impact.rules[R10].legit -->
古い版のインストーラで、新しい版の移行が当たった DB に入れ直すと止まる（版の食い違いは ST48 の規則）

<!-- prose impact.rules[R11].legit -->
イベントビューアで起動の失敗を見る習慣の人は、置き場のログのファイルを開くことになる（本人は読める）

<!-- prose impact.rules[R12].legit -->
記録まで消したい本人は管理者として所有権を取って手で消す（消す口は作らない）

<!-- prose impact.rules[R13].legit -->
C: の小さい PC で、記録を別のドライブに置きたい本人は置けない

<!-- prose impact.rules[R14].legit -->
ドキュメントの下の前の取り込み待ちに置いた書庫は、インストールした形では入らない

<!-- prose impact.rules[R15].legit -->
この分析では、正当な利用を止める場面を見つけていない（本人は読める）

<!-- prose impact.findings[IMP-01].title -->
DB はあるのに設定のファイルが無いと、抜ける口が仕様に無い

<!-- prose impact.findings[IMP-01].text -->
走らせ直しは秘密を上書きしないために止まる。設定のファイルは本人も書けない置き場にある。当たる道は、管理者が消した・一部だけ戻した場合のほかに、この分析では見つけていない（ウイルス対策の隔離・ディスクの異常・ST58 の戻しは確かめていない）。止まっている間、携帯の未送信は上限（90 日・2 GB）を超えた分から捨てられ始める。

<!-- prose impact.findings[IMP-02].title -->
55400 が Windows の予約する番号の範囲に入りうる

<!-- prose impact.findings[IMP-02].text -->
55400 は Windows の動的な番号の範囲（49152〜65535）の中にある。Hyper-V・WSL・Docker のある PC では、Windows が起動のたびにこの範囲から番号の塊を予約することがあり、予約された番号は待ち受けられない。この範囲は外向きの接続の送り側の番号にも使われるので、DB が起きる前に別のプログラムが 55400 を一時的に取ることもありうる。インストーラの検査はインストールのときに待ち受け中の番号だけを見るので、予約も後の取り合いも見えない。本人の PC には WSL と Docker がある。18700 は範囲の外。直すなら、番号が設定・手順書・Tailscale の案内・ST56 の繋ぎ先に広がる前（配る前）が一番安い。

<!-- prose impact.findings[IMP-03].title -->
既定のブラウザが Chromium でないと、http://127.0.0.1 でログインの印が残らない恐れ

<!-- prose impact.findings[IMP-03].text -->
ログインの印は `Secure` 付きで送られ、インストーラは `http://127.0.0.1:18700/` を開く。Chromium（Edge・Chrome）が http の 127.0.0.1 でもこの印を残すことは e2e で確かめてあるが、Firefox など別のブラウザが既定のときは確かめていない。残らなければ、決めた合言葉でログインしても画面に入れない。

<!-- prose impact.findings[IMP-04].title -->
案内どおりに Tailscale の転送先を替えると、携帯からは新しい（空の）サーバに届く

<!-- prose impact.findings[IMP-04].text -->
いま携帯のブラウザは tailscale serve で main の開発のサーバ（18787）を見ている（台本の注釈による。実際の設定は確かめていない）。インストールの最後の案内どおりに転送先を 18700 に替えると、携帯は記録の無い新しいサーバに届き、開発の API の合言葉を持つ収集アプリは断られる。設計（D18（仮））は携帯の収集アプリはまだ入っておらず PC の収集アプリの自動起動も無いとしているが、この分析では確かめていない。本人の切り替えの前に tailscale serve と収集アプリの状態を見る。

<!-- prose impact.findings[IMP-05].title -->
署名の無いインストーラには、管理者の確認の前に SmartScreen の警告が出うる

<!-- prose impact.findings[IMP-05].text -->
完了の判定は「管理者の確認が 1 回出て」と言う。インターネットから落とした署名の無い実行ファイルには、管理者の確認の前に SmartScreen の警告が出る（「詳細情報」→「実行」で進める）。CI は落とした印の無いファイルで走るので、この警告を見ない。

<!-- prose impact.findings[IMP-06].title -->
サーバが理由を書いて自分で終わると、サービスの回復が起こし直さない恐れ

<!-- prose impact.findings[IMP-06].text -->
仕様は、設定を読めない・DB が 120 秒開かないときにサーバが理由をログに書いて終わり、サービスの回復が起こし直すとする。設計の登録の命令は回復の動作（`sc.exe failure`）だけで、停止を告げて終わったときにも回復を動かす印（`sc.exe failureflag … 1`）が無い。Windows の回復は停止を告げずに落ちたときだけ動く。サーバが停止を告げて終わる作りなら、DB の起動が遅い PC では次の再起動までサーバが止まったままになり、本人はログを見ないと気づかない（携帯は溜めて送る）。

<!-- prose impact.scope.covers[0].text -->
Windows の正式版・ベータ版のサーバの、配る zip からのインストール・走らせ直し・合言葉の決め直し・消す

<!-- prose impact.scope.covers[1].text -->
インストールした形で動くサーバの設定・秘密・置き場・ログ・取り込み・書き出しと原本の既定の置き場

<!-- prose impact.scope.preconditions[0].text -->
インストールする本人がその PC の管理者の資格情報を持つ（管理者の確認は 1 回だけ）

<!-- prose impact.scope.preconditions[1].text -->
サーバのサービスが状態の置き場に書けるのは、db を除く置き場だけ（db は DB のサービスだけ）

<!-- prose impact.scope.preconditions[2].text -->
秘密と合言葉の照らすための値は状態の置き場の設定にだけ置く

<!-- prose impact.scope.preconditions[3].text -->
移行を当てるのはインストールのときだけ（起動のたびには当てない）

<!-- prose impact.scope.preconditions[4].text -->
走らせ直しで足すのは足りないものだけ

<!-- prose impact.scope.preconditions[5].text -->
画面の合言葉は 2 回が一致したときだけ決まり、元の値は残らない

<!-- prose impact.scope.preconditions[6].text -->
取り込みが見るのはダウンロードと `%USERPROFILE%\ashiato\取り込み待ち` だけ

<!-- prose impact.scope.preconditions[7].text -->
DB は 127.0.0.1 の 55400 番だけで待ち受ける

<!-- prose impact.scope.preconditions[8].text -->
ログは状態の置き場のファイルに書く（DB がファイルへ切り替わる前の起動の数行とサービスの制御の知らせは、イベントログにも出る）

<!-- prose impact.scope.preconditions[9].text -->
配布物に入る PostgreSQL は要るものだけ

<!-- prose impact.scope.preconditions[10].text -->
書き出しの合言葉の扱い・断る条件は今のまま（インストールした形で変わるのは既定の置き場と権限だけ）

<!-- prose impact.scope.preconditions[11].text -->
記録の読み出しは画面のログインだけ（API の合言葉では読めない）は今のまま

<!-- prose impact.scope.preconditions[12].text -->
サービスとして動くときだけ設定のファイルを読む（開発の台本は環境変数のまま）

<!-- prose impact.scope.preconditions[13].text -->
Tailscale は「無人で動かす」が入っているときだけ、ログオンしていない間も携帯から届く（インストーラは触らない）

<!-- prose impact.scope.excludes[0].text -->
開発の形のサーバと開発・試験の DB（Docker）の振る舞いと置き場

<!-- prose impact.scope.excludes[1].text -->
PC の収集アプリ・Linux の配布物・開発版の並置・更新

<!-- prose impact.scope.residual[0].text -->
PC の管理者は所有権を取れば状態の置き場を読める

<!-- prose impact.scope.residual[1].text -->
DB はあるのに設定のファイルが無いと、`repair` を走らせるまでサーバは止まり、`repair` の後は収集アプリの繋ぎ直しが要る（IMP-01）

<!-- prose impact.scope.residual[2].text -->
55400 が予約された番号の範囲に入る PC では DB が起きられない恐れ（IMP-02。推測）

<!-- prose impact.scope.residual[3].text -->
再起動の後に動いていることを実物で見ないまま配りうる（D26（仮））

<!-- prose impact.scope.residual[4].text -->
`repair` の間だけ、上がった利用者が合言葉なしで入れる一時の入口が `127.0.0.1` に開く

