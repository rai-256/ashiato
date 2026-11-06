# ST55 仮で進めた決定の確かめ（provisional_review）

対象: `provisional.jsonl` の D16・D17・D18・D10・D3・D26。問いを作った AI の「失わない」を信じず、design・tasks・spec・コードを grep で辿った。Windows の上では何も走らせていない。

## D16
- 判定: 確か
- 根拠: 画面は利用者を名乗らずに nil を読む（`crates/server/src/lib.rs:2599`・`:2662`・`:2965` の `unwrap_or_default()`）。開発の既定も nil（`.env.example:24`・`tools/env-init.sh:62`・`docs/archive-inbox.md:6`）。
  渡さないと取り込み器が起きない（`lib.rs:4433-4448` の `archive_disabled`）ので、nil を渡すのが失わない側で、費用は設定の 1 行（design.md:136-140・tasks.md:100）。
  収集アプリの利用者は設定から読み（`crates/collector-windows/src/config.rs:110`・`collector-android/app/build.gradle.kts:103`）、配った版のサーバへ繋ぐのは ST56 から（D18）なので、いま食い違う記録はできない。
  土台の決定は無い（要件の本文も変えない）。ST56 が収集アプリへ nil を渡すことが続きの前提になる（ST56 の申し送りに載せる話。この判定は変えない）

## D17
- 判定: 否
- 根拠: 条件 4（`rests_on` に土台の決定が無い）。D17 の「権限は置き場と同じ」は D3（仮）の ACL に乗るのに、`rests_on` が `[]`。
  しかも D3 の ACL（design.md:47）は `NT SERVICE\ashiato-db` に `db` だけ読み書きを付け、`logs\db` は根の `ashiato-server:(OI)(CI)M` と本人の `RX` を受け継ぐだけ。
  D17 の `log_directory='C:/ProgramData/ashiato/logs/db'`（design.md:144）に DB のサービスのアカウントは書けず、ログの収集プロセスがファイルを開けないので DB が起動しない（`grep -n 'ashiato-db:' design.md tasks.md specs/*/spec.md` → design.md:47 の 1 か所だけ）。
  D3 が反転条件（専用のローカルアカウント）で替わると、D17 のログの ACL も一緒に替わる。
  あわせて条件 1 の前提: Windows のサービスとして動く PostgreSQL は、ログの収集プロセスへの切り替えより前の出力（起動の最初のメッセージ・`postgresql.conf` の読み違い）と `pg_ctl` 自身のメッセージをイベントログへ書く（PostgreSQL の elog.c / pg_ctl.c の `write_eventlog`。この席の知識で確かさ 1）。
  文の字面と記録は含まない（`log_min_error_statement='panic'`・切り替えの後）ので外に出るものは無いと見るが、「イベントログには出さない」は字面どおりには成り立たない。spec.md:430-433 の Scenario（送った文と秘密が無い）はこの形でも通るので、design の文言を合わせればよい。
  処置の案: `rests_on` に D3 を足し、D3 の ACL に `logs\db` への `NT SERVICE\ashiato-db:(OI)(CI)M` を足す。spec.md:437-439 の「読みを持つのは 2 つのサービスのアカウントと本人だけ」とは合う

## D18
- 判定: 確か
- 根拠: 携帯の収集アプリはまだ入っていない（ST52 Q1。premises.md C18）。PC の収集アプリの自動起動の口はある（`crates/collector-windows/src/autostart.rs`・`main.rs:16`）が、本人の PC に仕込まれていないことは旧い前提で見ている（`git show 4f068c66:…/premises.md` の C10）。
  仕込まれていたとしても、PC の未送信は上限も破棄も持たない（`crates/collector-windows/src/outbox.rs:4`）ので、合わなくなっても捨てない。携帯の上限（FR-9。requirements.md:193）は ST56 で繋ぐまで効かない。
  失わない側の費用は手順書の書き直しだけ（design.md:148-152・tasks.md:162）で、範囲の中で要件を変えない。土台の決定は無い（D15 は本人の答え）

## D10
- 判定: 否
- 根拠: 条件 1（失わない側を選んでも `discarded` が起きる）。「DB はあるのに秘密が無い」で止まった後に出口が無い（design.md:95-101「出口…は作らない」）。
  止まっている間、携帯の未送信は FR-9 の上限（90 日・2 GB。requirements.md:193）で捨てられ、出口の無いまま元に戻す手は DB の置き場を消して入れ直すこと（記録を全部捨てる）だけ。design 自身が捨てることを認めている（design.md:99・specs/distribution/spec.md:271。「GO 資料に出す」）。
  `keeps` の「止まる側は何も書き換えない」は正しいが、この分岐で失わないとは言えない。出口（管理者が `postgres --single` で役割の合言葉を作り直す口。design.md:101 の反転条件）は ST55 の範囲の中で、要件を変えない —— 失わない側は「出口を作る」で、いまは失う側を仮で選んでいる。A として人間に返すか、出口を作る側に替える。
  あわせて: design の段の順（design.md:96 の (2)「秘密が無ければ作って設定に書く」→ (3)）のまま読むと、(3) の検査より前に (2) が新しい秘密を書き、次の走らせ直しでは「DB と秘密がある」に見えて食い違いを示せなくなる。tasks.md:108 の「見え方から次の段を決める純粋な関数」は止まる判定を先に置けるので、design の文を「(2) の前に判定する」に揃える

## D3
- 判定: 確か
- 根拠: 仮想アカウントで登録できないときに止まる側は、インストールが終わらないだけで、記録はまだ無く、共有のアカウントが置き場を読む形を作らない（`exported` が起きない）。反転条件も共有のアカウントへは倒さず専用のローカルアカウントへ替える（design.md:51）。
  費用は登録の失敗で止まる分岐と、下流の最初の確かめ（tasks.md:64 の 3.0 のサンドボックスでの P19・3.1 の CI の最初の項目）で、範囲の中で要件（FR-109）を変えない。本人の SID は上がる前に控える（D22。design.md:48）。
  ただし ACL に `logs\db` への `ashiato-db` の書きが無い（D17 の節）。失うものは無く CI の DB の起動で落ちるが、D3 の ACL の表を直すのはこの決定の側

## D26
- 判定: 確か
- 根拠: 本人の Q5 の答え（deep-answers-1.txt の Q5「残らなければ CI の『ログオンなしで起きる』までにする」）と同じ向き。CI の runner は再起動できない（tasks.md:178）ので、D26 は確かめられる範囲（開始の種類が自動・サービスのアカウント）に絞り、見ていないことを確認バッチに書く（design.md:196-203）—— 答えより弱めず、隠さない。
  動いていなかったときの失うもの: 携帯は ST56 まで繋がない（D18）、PC の未送信は上限を持たない（`outbox.rs:4`）、携帯を繋いだ後も 90 日の上限より先に本人の切り替えの最初の再起動で分かる。費用は確認バッチと手順書の 1 行ずつで、要件を変えない

## D28
- 判定: 否
- 根拠: 条件 4（`rests_on` に挙がっていない土台の決定がある）。
  一時の `hba_file` の行は `host all <DB の管理者の役割> 127.0.0.1/32 sspi map=<一時>`（design.md:225。tasks.md:114 も同じ）で、管理者の役割の名前が要る。ところが D1 の `initdb` は `-U` を渡さない（design.md:32）ので、管理者の役割の名前は「インストールを走らせた Windows の利用者の名前」になり（`initdb` の既定）、固定の名前を持たない（`tools/db-roles.sh:39` の `POSTGRES_USER:-ashiato` は開発の DB の名前で、配る DB には渡していない）。
  D4 の設定の鍵（design.md:55）にも管理者の役割の名前は無く、そもそも `repair` が動くのは設定のファイルが無いとき（design.md:216）なので、`repair` は名前を読む先を持たない。別の管理者 B が `repair` する Scenario（specs/distribution/spec.md:327）では、`pg_ident` が B の Windows のアカウントを A の名前の役割に結ぶ必要がある。反転条件（design.md:228 の `local all <DB の管理者の役割> trust`）も同じ名前を要るので、反転しても解けない。
  `grep -n -i 'admin\|管理者の役割\|-U ' design.md tasks.md` → 名前を決める・控える行は無い（design.md:41 の `-U` は `pg_ctl register` のサービスのアカウント）。土台は D1（`initdb` の引数）と D4（設定の鍵）で、どちらも `rests_on`（D27・D21）に無い。
  失うもの: 名前が引けなければ `repair` が (c) で止まり、出口の無い D10 の状態が続く（携帯の未送信は FR-9 の上限で捨てられる。design.md:238）—— D10 を否とした `discarded` がこの決定の下で戻ってくる。Windows の CI（tasks.md:117）で落ちて分かるが、そのとき直すのは D1 か D4 の側。
  直し方は ST55 の範囲の中で要件を変えない（例: D1 の `initdb` に固定の `-U` を渡す、または PostgreSQL 17 の `pg_ident` の役割の欄の `all` で役割の名前を問わずに結ぶ）。どちらを採るかを design に書き、`rests_on` に D1（と D4）を足す。
  あわせて（否の理由ではない）: (1) `ALTER ROLE … PASSWORD` の字面は `\getenv` でも `psql` が SQL に埋めてサーバへ送る（tools/db-roles.sh:45-57）。ログに載らないのは D6 の `log_statement='none'`・`log_min_error_statement='panic'` が `postgresql.conf` にあり（design.md:66-68）、一時の起動も同じ `postgresql.conf` を読むから —— D6 も土台なので `rests_on` に足す。(2) (c) の後で落ちたとき、`pg_ctl start` で起こした一時の `postgres` はサービスでないので (a) では止まらず、残ったまま次の `repair` の `pg_ctl start` が `postmaster.pid` で断られうる（design.md:218 は「(b) から作り直す」とだけ書く）。tasks.md:117 の `repair の途中で落ちても、もう一度 repair できる` は、`STOP_AT=repair-roles` が一時の `postgres` を残す形で走らせないと見えない。`repair` の最初に一時の形が残っていれば `pg_ctl stop -D` で止め、残った一時の `hba_file` を消す段を design に書く
  条件 1 のほか（keeps の範囲）: 一時の入口は上がった利用者の Windows のアカウントだけに `127.0.0.1` で開き、外に出るもの・捨てるものは無い（`exported` / `discarded` / `uncaptured` / `rewrite-all` は起きない）。条件 2: 費用（一時の `hba_file` と `pg_ident` を組む関数・Windows の CI の Scenario）は範囲の中で、spec の一時の例外（specs/distribution/spec.md:112）は Q8 で出口を作ると決めた側から来るもので D28 が足したものではない。条件 3: `follows` は無い

## D29
- 判定: 確か
- 根拠: 条件 1: プログラムの置き場への写しは無いファイルだけで（`crates/installer/src/main.rs:639-683`）、`sc.exe create` はサービスが無いときだけ（`main.rs:996-1001`）。記録・設定・外に出るものに触らず、`uncaptured` / `discarded` / `exported` / `rewrite-all` はどれも起きない。条件 2: 反転したときの費用（`New-Service` か写しの順を替える）は ST55 の範囲の中で、要件を変えない。条件 3: `follows` は無い。条件 4: 写した `pgsql` を指す `pg_ctl register`（D2）・置き場の名前（D14）・段の順（D10）は `rests_on` にある。
  あわせて（否の理由ではない。失うものは無いが、まっさらな機械では毎回落ちる）: (3) の `initdb` は `<program_dir>\pgsql\bin\initdb.exe` の絶対の場所で呼ぶ（`commands.rs:536`・`:588`。`main.rs:975`）のに、写しは後の (5)（`main.rs:992`）。zip を展開した置き場から初めて入れると `C:\Program Files\ashiato\pgsql\bin\initdb.exe` がまだ無く、`initdb.exe を起こせない` で止まる。走らせ直しも `PG_VERSION` が無いので同じ所でまた止まる。design の D29（design.md:248「D10 の (5) で…写す」と「`initdb`…は `<program_dir>\pgsql\bin` の絶対の場所で呼ぶ」）は、この 2 つが噛み合わない。`install-check.sh:437-438` は展開した置き場（`$WORK/unpacked`）から走らせるので、Windows の CI では最初の Scenario で落ちて見つかる（このブランチの CI はまだ走っていない。`gh run list -b feat/st55-server-installer` は空）。写しを (1) の後・(3) の前へ移すのは、D10 の段の順の直しで、この決定の向きは変わらない。
  もう 1 つ: 台帳の `chosen` は「道具は絶対の場所で呼ぶ」と書くが、絶対の場所なのは同梱の PostgreSQL の道具だけ。`sc.exe`・`icacls`・`reg`・`powershell.exe`・`whoami` は名前だけで呼んでいる（`main.rs:92` の `Command::new(&command.program)`）。Rust の `Command` は Windows で、システムの置き場より先に「いま動いている実行ファイルの置き場」を探す（`~/.rustup/toolchains/1.97.1-…/share/doc/rust/html/std/process/struct.Command.html` の探す順: 子の PATH → 実行ファイルの置き場 → `GetSystemDirectoryW` …）。上がったインストーラは展開した置き場（ダウンロードの下など、本人が書ける所）から動くので、そこに置かれた `sc.exe` が管理者で動く。記録は本人がもともと読める（D3）ので新しく外に出るものは無いと見るが、`chosen` の文言とは合わない。`%SystemRoot%\System32\<名前>` の絶対の場所で呼べば済む

## D31
- 判定: 否
- 根拠: 条件 1（失わない側を選んでも `discarded` が起きる）と条件 4（`rests_on` に D27 が無い）。
  D31 の狭めは「持ち主がサービスのアカウントのファイル（`db\17` の中など）へは ACL の変更が届かない」を前提にしている（design.md:266・`commands.rs:437-441` のコメント）。ところが `repair` は、DB のサービスが動いた後の `db\17` にこの一時の書きを足し（`main.rs:1318` の `DatabaseWrite::grant` → `commands.rs:473-479` の `icacls <db> /grant (OI)(CI)M`。`/T` は無く、継承で中へ届くことを当てにしている）、上がった札を落とした `pg_ctl start` の `postgres` に `db\17` を書かせる（`main.rs:1356`）。サービスが作った WAL・移行で作った表のファイルは `NT SERVICE\ashiato-db` が持ち主で、継承を切った `db` の ACL（`commands.rs:488-494`）に `Administrators` が無い。D31 の前提どおりなら、一時の書きはそれらに届かず、一時の `postgres` は起きないか `ALTER ROLE` の WAL を書けない。別の管理者 B の `repair` では、`initdb` が作ったファイル（持ち主は A）にも届かない。
  つまり D31 の前提が正しければ、出口の `repair`（D27。本人の Q8 の答え）は記録の入った DB で動かず、D10 で否とした状態（止まって出口が無く、携帯の未送信が FR-9 の上限で捨てられる。requirements.md:193・design.md:303）に戻る。前提が誤りなら、そもそも狭める理由が無い。どちらでも、`repair` の書きが届くかは確かめられていない（premises.md に ACL の届き方の前提が無い。確かさ 1）。
  反転条件（design.md:268）は「D28 の反転条件と一緒に見直す」と書くが、D28 の反転条件（`local … trust`。design.md:228 の系列）は接続の規則で、ファイルの書きは直さない。`logs\db` の側（review/code.md R32）は `-l settings\repair-db.log`・`logging_collector=off`（`commands.rs:861-866`）で直っているので、残っているのは `db\17` の中だけ。
  台帳の `chosen` は「DB を起こす前の db にだけ足す」で、design の但し書き（`repair` の `db` だけは足す）を含まない。字面どおりなら `repair` は `db` に書けず、同じ所で落ちる。
  処置の案: `repair` の (c) の前に、`db\17` の中の持ち主を取る（`takeown /r` か `icacls /setowner … /T`）か、`/T` を付けて書きを足し、(d) で DB のサービスの書きと本人の読みに戻す段を design に書く。その形を Windows の CI の `repair` の Scenario（tasks 6.2）で、移行を当てて記録を入れた後の DB で見る。D27 を `rests_on` に足す

## D32
- 判定: 確か
- 根拠: 条件 1: 作り直すのは `initdb` を始めた印があり、DB のサービスが無く、移行の完了記録も無いときだけ（`main.rs:954`）。役割は (5) の登録・(6) の起動の後の (7) でだけ作り（`main.rs:992-1020`）、印はその後に消す（`main.rs:1018`）ので、印が残っていてサービスが無い DB には、サーバが繋ぐ役割が無く記録が入らない。消す口でサービスを消した DB は役割の完了記録を持つので消さない（`roles_done`。`main.rs:952-957`）。止まる側は何も書き換えない（`state.rs:59-64`）。条件 2: 費用は印のファイル 1 つと案内の 1 文で、範囲の中。条件 3: `follows` は無い。条件 4: 土台の段の順（D10）と出口（D27）は `rests_on` にある。
  あわせて（否の理由ではない）: D32 が足した形「設定はあるが役割の完了記録が無い」（`state.rs:54-56` の `repair_allowed`）で `repair` が、完了記録を書いた後（`main.rs:1378-1383`）・`server.json.repair` を名前替えする前（`main.rs:1392-1393`）に落ちると、古い `server.json`（と、`owner.json` を置いた後なら新しい `owner.json`）が残って役割の合言葉と合わず、完了記録はあるので次の `repair` は「要りません」で断る（`repair_allowed` が偽）。走らせ直しは移行かサーバの接続で落ちる。手で完了記録を消せば戻る（記録は失わない）が、案内は無い。窓は名前替えの 2 回の間だけ。`server.json.repair` が残っていれば `repair` を受け付ける（`repair_allowed` に 1 つ足す）と閉じる

## D33
- 判定: 確か
- 根拠: 条件 1: 配る zip の `pgsql\bin` に DLL を 2 つ足すだけで、インストーラの振る舞いも記録も変えない。インストールの間はネットワークに出ない（D12）。条件 2: 費用は `tools/release/build.sh:100-113` の 1 関数と `check-artifacts.sh:77` の検査で、範囲の中。条件 3: `follows` は無い。
  2 つで足りるかを、この席で落としてある EDB の zip（`/tmp/st55-pg/x/pgsql/bin`）を `x86_64-w64-mingw32-objdump -p` で読んで確かめた: VC の実行時は `VCRUNTIME140.dll`（62 ファイル）・`MSVCP140.dll`（8）・`VCRUNTIME140_1.dll`（8）。`VCRUNTIME140_1.dll` を読むのは `stackbuilder.exe` と `wx*.dll` の 8 つだけで、どれも残さない（`tools/release/pgsql.lock:15` の `dlls`）。展開した zip の `pgsql/bin` に残るのは `postgres.exe` などで、2 つで足りる。`api-ms-win-crt-*` は Windows 10 以降の OS が持つ UCRT。
  条件 4 で 1 つ: `THIRD-PARTY-NOTICES.txt` に足す節（`build.sh:92`）は同梱の許諾の文の決定（D13）の上に乗るのに、`rests_on` は D12・D19 だけ。失うものは無いので否にはしないが、D13 を足すのがよい。Distributable Code の条項（design.md:286 の反転条件）は、この席では確かめていない

## D34
- 判定: 確か
- 根拠: 条件 1: 写す前に全部のファイルを比べ（`main.rs:654-665` で計画を作ってから `:667-671` で写す）、違うものが 1 つでもあれば何も写さずに止まる。写すのは `.copying` を経た無いファイルだけ（`main.rs:725-734`）。記録・動いている版・「アプリと機能」の版番号（(12) は止まった後なので書かない）に触らない。条件 2: 費用は zip の中身を 1 回読むことで、範囲の中。条件 3: `follows` は無い。条件 4: 土台の D29 は `rests_on` にある。
  あわせて（否の理由ではない）: 比べるのは (5)（`main.rs:992`）で、(1)〜(4) の後。状態の置き場が無く、プログラムの置き場だけに別の版が残っている機械（状態の置き場を手で消した後など）では、止まる前に (1) の ACL・(2) の新しい秘密・(3) の `initdb`（置き場に残っている古い版の `initdb.exe` で。`commands.rs:588`）を済ませてしまい、`keeps` の「何も替えずに止まる」は字面どおりにはならない。記録はまだ無いので失うものは無い。D29 の節の「写しを (3) より前へ」と一緒に、比べる段も (1) より前に置けば揃う
