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
