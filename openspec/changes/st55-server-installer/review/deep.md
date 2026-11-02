# ST55 深掘りの点検（deep-review）

対象: `openspec/changes/st55-server-installer/deep-questions.json`（Q1〜Q11）・`premises.md`（P1〜P12・C1〜C11）・`premise-pgdata.ps1`。
手順 1〜8 は問いの一覧を見る前にやり直し、そのあと突き合わせた。本人の PC で確かめたもの（読むだけ。Docker Desktop は起こしていない）は、根拠にコマンドを書いた。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: FR-108（既定は Docker Desktop）と FR-109（すべて Docker の外の Windows のフォルダ）は、bind mount が通らないと両立しない → Q1・Q2 にある。FR-110「その PC の中にだけ」と FR-109「Windows のフォルダ」は、そのフォルダが同期されると食い違う → Q3 にある。FR-111 と ST28 の起動時の検査（`web_session.rs:86-95`）は Q5・Q6 にある。新しい衝突は見つからなかった
- 手順 2（扉の幅）: `doors: []`。関わりうる扉 20（原本は D-02 のファイル）・扉 24（バックアップの鍵は資格情報ストア）を読んだ。扉 20 は FR-109 と同じ向き。`core.media_original` は場所を持たず（`migrations/202610181200_media_capture.sql:53-`）、フォルダを移しても D-01 を書き直さない（Q3 の「後から移せる」は正しい）。扉 24 の鍵はこの Story が作らない。なお §5 の外部世界の主張で **EXT-O が 2 つある**（Health Connect と Docker Desktop。`docs/requirements.md:1567`・`:1580`）。FR-108 の「（EXT-O）」はどちらを指すか字面で決まらない（問いの外。要件の整合で直す）
- 手順 3（一方通行）: DB の本体の置き場（Q2）・データのフォルダ（Q3）は一覧にある。足りないのは R4・R5・R6・R14・R15
- 手順 4（日常）: R7・R8・R15
- 手順 5（既存コード）: R12・R13 と、Q10（`PG_DUMP`）・C9（相対の退避の置き場）は一覧どおり（`migrate_backup.rs:27-33`）
- 手順 6（過去の取りこぼし）: 「OS のサービス・コマンドの既定を実機で確かめる」→ R3・R8。「保存場所が丸ごと消えた／入れ直した」→ R10。「DB や写しを増やすなら削除が届くか」→ R4・R15。「完了の判定が本物の操作に乗る」→ Q9 にある。時計の族は、この Story で時刻を入力に状態を動かす決定が無い（退避の名前の時刻 `migrate_backup.rs:97` は並べるだけ）ので該当なし
- 手順 7（族）: R9・R10・R11
- 手順 8（前提）: R1・R2・R3・R6・R16

## R1. 前提の台本 premise-pgdata.ps1 は、書き込みの測定が必ず落ち、「再起動の後に行が無い」と読める出力を出す
- 成果物: openspec/changes/st55-server-installer/premise-pgdata.ps1
- 根拠: premise-pgdata.ps1:23 が `psql -c "create table t(a int); do $$ … commit; … $$;"` を **1 つの `-c`** で送る。1 つの問い合わせに文が 2 つあると暗黙のトランザクションの中で走り、DO の中の COMMIT は拒まれる。手元の `postgres:17-alpine`（使い捨てのコンテナ `ashiato-dr-st55-probe`）で再現: `ERROR: invalid transaction termination` rc=1、続く `select count(*) from t` は `relation "t" does not exist`（create table も巻き戻る）。`-c 'create table t2(a int)' -c 'do $$ … $$;'` と分けると rc=0・2000 行。台本は `| Out-Null` で誤りを捨てるので、出力は「2000 single-row commits: 0.x s」（速く見える）と「rows after restart: ERROR: relation "t" does not exist」（消えたように見える）になり、Windows のフォルダ・ボリュームの両方で同じ形になる。構文と引用は PowerShell 5.1（5.1.26100.9444）の `Parser::ParseFile` で誤り 0、`` `$`$ `` は `$$` に展開される（ここは正しい）
- kind: premise
- 提案: 23 行を `-c "create table t(a int)" -c "do `$`$ … `$`$;"` に分け、psql の終了コードを捨てずに `[$label] write: rc=N` として出す。Q1 の選択肢の「再起動の後も行が残った」は、この直しの後でないと判定に使えない
- 処置: fixed premise-pgdata.ps1 —— `create table` と DO を別の `-c` に分け、どちらも `rc=N` を出すようにした（誤りを捨てない）。PowerShell 5.1 の `Parser::ParseFile` で誤り 0

## R2. Q1 の測り方では Q2 の推奨の条件（停電・強制終了で書き込みが残るか）を確かめられない。待ちの判定にも競り合いがある
- 成果物: openspec/changes/st55-server-installer/premise-pgdata.ps1 / openspec/changes/st55-server-installer/deep-questions.json（Q1・Q2）
- 根拠: premise-pgdata.ps1:26 は `docker restart`（きれいな停止）だけで、Q2 の選択肢 1 の irreversible が名指す「停電・強制終了のときの耐久性」は測らない。fsync の回数（:25）は fsync が空振りしているかの目安にしかならない。Q2 の family「Docker を消して入れ直したとき」も、同じフォルダを**新しいコンテナ**に付け直す場合を試していない（ファイルの持ち主と権限が Docker の層に残るなら、入れ直した後に `data directory has invalid permissions` で起動しない場合がありうる）。また :18 の判定は、初期化用の仮サーバの「ready to accept connections」と「init process complete」が揃った時点で真になり、本番のサーバが待ち受ける前に :23 へ進みうる（手元のログ: 仮サーバの ready が 33 行目、complete が 49 行目、本番の ready が 56 行目）
- kind: premise
- 提案: 台本に (a)「`docker rm -f` の後に同じフォルダで新しいコンテナを起こし、行が残るか」を足す、(b) ready の判定を「ready の行が 2 つ」か `pg_isready` にする。premises の P3 には「(b) を走らせても耐久性（fsync が Windows の書き込みまで届くか）は 2 のまま」と書き、Q2 の推奨の条件を「初期化と付け直しが通ったとき」に言い直すか、耐久性が未確認であることを選択肢の irreversible に残す
- 処置: fixed deep-questions.json —— 台本の待ちを `pg_isready -h 127.0.0.1`（TCP。初期化用の仮サーバは開けない）にし、同じフォルダを新しいコンテナに付け直す測り方を足した。P3 に「耐久性はこの台本では測れない」を書き、Q1 の context・選択肢と Q2 の推奨の条件（初期化・再起動・付け直し）と irreversible（耐久性は未確認）に写した。Q1 は premise で人間に問う

## R3. 本人の PC の Docker Desktop は Ubuntu と連携する設定になっている。Q1 の手順と Q7 の文面が、その前提に合わない
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q1・Q7） / openspec/changes/st55-server-installer/premises.md（P11）
- 根拠: `%APPDATA%\Docker\settings-store.json`（`/mnt/c/Users/<user>/AppData/Roaming/Docker/settings-store.json`）に `IntegratedWslDistros: ['Ubuntu']`・`AutoStart: False`。Q1 の context は「起こす前に、Docker Desktop の設定の Resources → WSL integration で Ubuntu への連携が切れていることを確かめてください」と言うが、その画面は Docker Desktop を起こした後にしか開けず、起こした時点で連携が始まる。Q7 の選択肢 3 は「（いまは連携していないなら変わらない）」と書くが、いまは連携している。Q7 の推奨（連携には触らない・自動起動を聞いてから入れる）を本人が受けると、ログオンのたびに Docker Desktop が Ubuntu に連携し、WSL の中の dockerd（P11）と取り合う
- kind: premise
- 提案: P11 に `IntegratedWslDistros: ['Ubuntu']`・`AutoStart: False` を確かさ 5 で足す。Q1 は「起こす前に settings-store.json の `IntegratedWslDistros` を空にする（Docker Desktop は止めたまま）」の手順に直す。Q7 の context と選択肢 3 を実際の状態に合わせ、推奨が本人の PC で何を起こすか（連携が入ったまま自動起動が入る）を書く
- 処置: fixed deep-questions.json —— P11 に `AutoStart: False`・`IntegratedWslDistros: ['Ubuntu']` を足した。Q1 の準備を「止まっている間に settings-store.json の連携を切る」に直し、Q7 の context と選択肢に実際の状態と推奨が本人の PC で起こすことを書いた

## R4. Q2 の選択肢の「失うもの」が誤っている —— バックアップはまだ無く、写しを流す案の「最長 1 分」は代償を書いていない
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q2）
- 根拠: 選択肢 1 の irreversible「壊れたら最後のバックアップまで戻る」。毎日・毎週のバックアップ（FR-66 / FR-67）は ST30 の担当で、ST30 の change は無い（`ls openspec/changes | grep st30` が空）。サーバにも実装が無い（`grep -rln "pg_basebackup\|archive_command" crates/server/src` が空。あるのは移行の前の退避 `migrate_backup.rs` だけ）。いま壊れれば全部失う。選択肢 2 の「既定で最長 1 分」: PostgreSQL の `archive_timeout` の既定は 0（区切りを強制しない）。1 分にすると、書き込みがある間は 1 分ごとに 16 MB の区切りが写しに出る（公式文書: 早く閉じた区切りも満杯と同じ長さ）。生存信号が数分ごとに来る運用では、1 日最大で約 23 GB になる。また選択肢 2 は FR-66 / FR-67（ST30）の仕組みの一部をこの Story で作ることになり、写し（WAL とベースバックアップ）には本人が消した記録（ST22 / ST23）が保持の期間だけ残る
- kind: irreversible
- loss: discarded
- 提案: 選択肢 1 の irreversible を「ST30 のバックアップが入るまでは、壊れたら全部失う」に直す。選択肢 2 に「1 分の区切りの容量の代償（か `pg_receivewal` で流す形）」と「ST30 の範囲を前倒しする」「消した記録が写しに残る期間」を書く
- 処置: escalated —— Q2 の why に「バックアップ（ST30）はまだ無い」、選択肢 1 の irreversible に「ST30 が入るまでは壊れたら全部失う」、選択肢 2 に `archive_timeout` の既定 0・1 分で 1 日最大約 23 GB・`pg_receivewal`・ST30 の前倒し・消した記録が写しに残ることを書いた（Q2 は A のまま人間に問う）

## R5. Q3 の推奨は別のフォルダを選べるが、Windows のフォルダの権限を誰も狭めないので、選んだ場所によっては同じ PC の他のアカウントから DB・原本・合言葉が読める
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q3）
- 根拠: `crates/server/src/migrate_backup.rs:138`「Windows は利用者のプロファイルの ACL に従う」、権限を付けるのは `#[cfg(unix)]` だけ（`migrate_backup.rs:141`・`media.rs:41-43`・`export.rs:1188`）。本人の PC の `Get-Acl D:\`: `Authenticated Users Modify`（下へ継承）・`BUILTIN\Users ReadAndExecute`（下へ継承）。`D:\ashiato` を作るとこの継承を受け、他のアカウントから読める。`%LOCALAPPDATA%` はプロファイルの ACL なので当たらない。C2 の「本人だけが読める設定」もプロファイルの外では成り立たない
- kind: irreversible
- loss: exported
- 提案: Q3 の選択肢 1 の irreversible に「プロファイルの外を選ぶと、継承した権限で他のアカウントから読める」を足すか、選択肢の中身に「選んだフォルダの継承を切り、本人と SYSTEM だけにする」を入れる。聞かない既定にしてもよい（失わない側。費用は ACL を付ける処理 1 つ）
- 処置: escalated —— Q3（A・exported）の context と推奨の detail に写して人間に問う。選んだフォルダの継承を切り本人と SYSTEM だけにする、を聞かない既定にし（失わない側）、P16 に `D:\` の ACL を足した。Q3 の context と推奨の detail に写した

## R6. Q3 の context の取り込み待ちの場所がコードと合わず、取り込み待ちを外す理由が「外に出るか」でなく「持ち続けるか」で切られている
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q3） / openspec/changes/st55-server-installer/premises.md（P7・P8）
- 根拠: `crates/server/src/archive/config.rs:59-63` は既知フォルダでなく `%USERPROFILE%\Documents\ashiato\取り込み待ち` の字面を組む。本人の PC では `[Environment]::GetFolderPath("MyDocuments")` = `C:\Users\<user>\OneDrive\ドキュメント`、Desktop も `OneDrive\Desktop`（PC フォルダーのバックアップが入っている）、`User Shell Folders` の `Local AppData` は `C:\Users\<user>\AppData\Local` のまま。エクスプローラーの「ドキュメント」は OneDrive の側なので、context の「いまは『ドキュメント\ashiato\取り込み待ち』」は本人の PC では別の場所を指す。本人が「ドキュメント」の下に取り込み待ちを作って書庫を置くと、その書庫が OneDrive に上がる。書庫にはマップのタイムライン（端末にしか無いデータ。EXT-M、`crates/server/src/archive/timeline.rs`）が入る
- kind: premise
- loss: exported
- 提案: P8 は本人の PC について確かさ 5（上の 2 つの値）に上げる。Q3 の family に「取り込み待ちの場所」を兄弟として足し、同じ同期の確かめを当てるか、取り込み待ちの場所を既知フォルダで解くかを B（推奨つき）で置く
- 処置: escalated —— Q3（A・exported）と同じ族の Q12 として問いに足して人間に問う。P8 を本人の PC の値で確かさ 5 に上げ、Q3 の context の取り込み待ちの記述を直した。取り込み待ちの場所を Q12（B・推奨はデータのフォルダの中）として足し、Q3 と同じ族（同期・共有される場所）に入れた

## R7. 抜け: Docker Desktop を常駐させる日々の重さ（メモリ・窓・更新の知らせ）を問うていない
- 成果物: openspec/changes/st55-server-installer/deep-questions.json
- 根拠: 本人の `%USERPROFILE%\.wslconfig` は `memory=32GB`。Docker Desktop の WSL 2 の仕組みは、WSL の Ubuntu と同じ仮想マシンで動き、資源の上限はこの全体の設定でしか掛けられない（Docker の文書: WSL 2 の形では `.wslconfig` で上限を掛ける）。開発の DB とグラフが同じ上限を分け合う（メモ「メモリ不足の正体（2026-10-25）」の OOM と同じ枠）。ログオンのたびに起動すると、トレイ・ダッシュボードの窓・更新の知らせが毎日目に入る。一覧のどの問いも、これを挙げていない（Q7 は自動起動の可否だけ）
- kind: daily
- 提案: B の問いを 1 つ足す（インストーラが `.wslconfig` に触るか・Docker Desktop の「起動時にダッシュボードを開く」「更新を自動で落とす」をどうするか）。`.wslconfig` は開発の環境にも効くので、触らない側を推奨に置くならその理由を context に
- 処置: escalated —— Q13 として問いに足した（kind: daily）。Q13（B。`.wslconfig` には触らない・新しく入れたときだけダッシュボードを開く設定を切る・自動の更新はそのまま）を足した

## R8. Q11 の推奨（タスク スケジューラ）は、既定のままだと 72 時間で止め、電池に切り替わると止め、落ちても起こし直さない
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q11）
- 根拠: 本人の PC の PowerShell 5.1 で `New-ScheduledTaskSettingsSet` の既定: `ExecutionTimeLimit=PT72H`・`DisallowStartIfOnBatteries=True`・`StopIfGoingOnBatteries=True`・`RestartCount=0`。選択肢 1 の detail「落ちたときの起こし直しを Windows に任せられる」は明示して設定したときだけ成り立つ。過去の取りこぼし「OS のサービス・コマンドを情報源にする決定では、既定の起動種別・権限を実機で確かめる」（ST05 Q4）と同じ型
- kind: daily
- 提案: Q11 の context に上の既定を書き、選択肢 1 を「時間の上限なし・電池でも止めない・落ちたら N 回起こし直す」と明記する。premises に P を 1 つ足す（確かさ 4。上のコマンド）
- 処置: escalated —— Q11（B）の推奨と context に写して人間に問う。P13（確かさ 4）を足し、Q11 の context に既定を書き、推奨を「時間の上限なし・電池でも止めない・1 分おきに 3 回まで起こし直す」と明記した

## R9. 族の抜け: Q8 は「WSL の再起動」だけで、同じ族の「サインアウトが要る」「昇格した別のアカウント」が無い
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q8・Q4）
- 根拠: P4 の出所（https://docs.docker.com/desktop/setup/install/windows-install/）は、全利用者向けの導入で、管理者のアカウントと使う人のアカウントが違うときは使う人を `docker-users` に足し、サインアウトして入り直すよう求める。Q4 の context は「Docker Desktop と WSL を入れるところだけ管理者の権限」と言うが、標準のアカウントで別の管理者の資格情報を入れて昇格すると、昇格した側の `%LOCALAPPDATA%`・タスク スケジューラ・`HKCU` はその管理者のものになり、データのフォルダ（Q3）と自動起動（Q11）が本人でないプロファイルに置かれうる
- kind: technical
- 提案: Q8 の family に「docker-users への追加でサインアウトが要る」と「昇格したアカウントが本人と違う」を兄弟として足す。後者は「昇格は Docker Desktop と WSL の導入の子プロセスだけ、データのフォルダと自動起動は昇格しない側で作る」を聞かない既定にできる（失わない側）
- 処置: fixed premises.md C13 —— 昇格は導入の子のプロセスだけ、データのフォルダと起動は本人の側で作る、を聞かない既定にした。Q8 の context と Q7・Q8 の族に「docker-users とサインアウト」「昇格したアカウントが違う」を足した

## R10. 族の抜け: C1（既にある状態は上書きしない）の兄弟の「片方だけある」場合が決まっていない
- 成果物: openspec/changes/st55-server-installer/premises.md（C1） / openspec/changes/st55-server-installer/deep-questions.json
- 根拠: C1 は「DB の合言葉と API の合言葉は、無いときだけ作る」。DB の本体はあるのに合言葉のファイルだけが無い（消した・別のフォルダを選び直した）とき、新しく作った合言葉では既にある役割に繋がらない（`tools/env-init.sh:9-10` の注記が同じ失敗を書いている）。戻し方はある（コンテナの中の psql は合言葉なしで入れ、`tools/db-roles.sh:7-11` が役割の合言葉をそろえている）が、決まっていない。API の合言葉を作り直すと、ログインの世代の印（`web_session.rs:98` の鍵）が変わって全部のログインが切れ、収集アプリ（ST56）の資格情報も切れる。逆の「合言葉はあるが DB の本体が無い」（Q2 のボリューム案で Docker を消した）は Q2 の context が「戻す」とだけ書く。過去の取りこぼし「保存場所が丸ごと消えた／入れ直した場合を必ず問う」（ST08 Q12）
- kind: technical
- 提案: C1 の行を族に開き、(a) 本体あり・合言葉なし → 作り直してコンテナの中から役割をそろえる、API の合言葉を作り直したことを示す、(b) 合言葉あり・本体なし → Q2 の答えに従う、(c) 別のフォルダを選んで走らせ直した → 前のフォルダを示して止まる、を聞かない既定として書く（どれも失わない側）
- 処置: fixed premises.md C1 —— 片方だけある (a)(b)(c) を聞かない既定として書いた（どれも失わない・止まる側）

## R11. 族の抜け: データのフォルダに他のプログラムが触る場合（ウイルス対策・索引・Docker の更新）が Q2・Q3 に無い
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q2・Q3）
- 根拠: Q2 の選択肢 1 は PGDATA を NTFS の上に置く。Windows で PostgreSQL のデータの置き場をウイルス対策のリアルタイムの走査から外さないと、ファイルの名前の付け替え・削除が一時的に拒まれて落ちることがある（PostgreSQL の Windows での既知の運用の注意。知識・確かさ 1）。Q3 は同期ソフトだけを挙げ、Windows Defender と検索の索引（原本の写真を索引する）を挙げていない。Docker Desktop の更新でファイル共有の仕組みが替わると、Q1 で測った結果が保たれない
- kind: technical
- 提案: Q2 の family に「ウイルス対策の走査」「Docker Desktop の更新後」を兄弟として足し、既定（除外を入れるか・示すだけか、更新後に Q1 と同じ自己点検を起動時に走らせるか）を B で置く。premises に確かめ方つきで P を足す
- 処置: fixed premises.md C14 —— ウイルス対策の除外は入れず示すだけ、DB の起動に失敗したら理由を示して止まる、を聞かない既定にし、Q1・Q2 の族に兄弟として足した

## R12. 抜け: インストールした DB とサーバのポートが、本人の PC の開発のポートと被りうる（C6 は compose の project 名しか分けていない）
- 成果物: openspec/changes/st55-server-installer/premises.md（C6）
- 根拠: `tools/ports.sh` の規則は main が DB 55432・サーバ 18787、確認バッチが 55499・18787、Story が 55500+NN・19000+2×NN、ほかが 55600+n・19200+2×n。本人の WSL は `wslinfo --networking-mode` = nat で、WSL の中の待ち受けは Windows の 127.0.0.1 に転送される —— 本人の PC の PowerShell から `Test-NetConnection 127.0.0.1 -Port 55499` = True（確認バッチの DB に届いた）。インストールが同じ番号を取ると、どちらかが待ち受けに失敗するか、Windows のサーバが WSL の開発の DB の口に向く（合言葉が違うので書き込みは通らず止まる側に倒れるが、起動しない）。メモ「取り違えは被らない割り当てで防ぐ」
- kind: technical
- 提案: C6 に「インストールの DB とサーバのポートは `tools/ports.sh` のどの範囲とも被らない固定の番号にする」を足す（聞かない既定。失わない側）。スマホから見る `tailscale serve` の先（18787）をどちらに向けるかは C11 の範囲の外として明記
- 処置: fixed premises.md C6 —— ポートを `tools/ports.sh` のどの範囲とも被らない固定の番号にする、を足した。P15 に WSL の網（nat・127.0.0.1 から届く）を足した

## R13. 既存コード: サーバは設定を環境変数からしか読まない。Q6・Q11 の「設定のファイルから読む」は新しく作る前提で、その前提がどこにも書かれていない
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q6・Q11） / openspec/changes/st55-server-installer/premises.md
- 根拠: `crates/server/src/lib.rs:4365-4388` は `DATABASE_URL`・`API_TOKEN`・`WEB_PASSWORD`・`WEB_SESSION_MAX_AGE_DAYS`・`BIND` を `std::env::var` で読む。`grep -rn "dotenv\|\.toml\"\|settings.json" crates/server/src` は 0 件。Q11 の context「サーバの合言葉は `.cmd` に書かず、設定のファイルから読みます」と Q6 の推奨（ハッシュを設定に置く）は、どちらもまだ無い読み口に乗る。FR-109 の「設定」を Windows のフォルダに置くことも、この読み口が要る
- kind: technical
- 提案: premises に「サーバは設定のファイルを読まない（確かさ 3、上の行）」を足し、読み口を作るか（サーバが読む）・起動の係が環境変数に詰めて渡すか（サーバは直さない）を聞かない既定か B で決める。後者なら Q6 の推奨の「サーバのログインの照らし方を直す」と合わせて書く
- 処置: fixed premises.md C15 —— サーバは設定のファイルを読まず、起こす係が環境変数に詰める、を聞かない既定にした。P14 を足し、Q6・Q11 の context と Q6 の選択肢に写した

## R14. 抜け: DB を初めて作るときに固まる値（データのチェックサム・像の系統）が C7 の主版だけで止まっている
- 成果物: openspec/changes/st55-server-installer/premises.md（C7）
- 根拠: `docker-compose.yml:3` は `postgres:17-alpine`（musl の系統）。`initdb` の文字の並べ方（locale）と符号化は DB を作った時に固まり、変えるには全体の書き出しと入れ直しが要る。系統を alpine と Debian で取り替えると、文字の索引の並びが変わって作り直しが要る。データのチェックサムは PostgreSQL 17 の既定で無効で、Q2 の推奨のように Docker の層の耐久性が未確認なら、壊れたことに気づく手がかりになる（後から `pg_checksums` で止めて有効にできるので失うものは無い）
- kind: technical
- 提案: C7 を「主版と系統（alpine）を開発と同じに固定し、`POSTGRES_INITDB_ARGS=--data-checksums` で作る」に広げる（聞かない既定。失わない側）
- 処置: fixed premises.md C7 —— 系統（alpine）の固定と `--data-checksums` を足した

## R15. 移行の前の退避は数が増え続け、消した記録も残る。置き場を Windows のフォルダにした（C9）以上、保持の規則がどこにも無い
- 成果物: openspec/changes/st55-server-installer/premises.md（C8・C9）
- 根拠: `crates/server/src/migrate_backup.rs:86-` は `pre-migrate-{stamp}.dump`（:97）を毎回 1 つ作り、古いものを消す処理が無い（`grep -n "remove" migrate_backup.rs` で消すのは :123 の途中で落ちた空の退避だけ）。C8 で移行はインストールと更新の時だけだが、開発版（FR-107）は変更のある日ごとに出るので、更新を追うと毎日 1 つ DB 全体の写しが増える。退避には本人が消した記録（ST22 / ST23）が残る（過去の取りこぼし「DB や写しを増やす決定には、削除が届くかを問う」）
- kind: defer
- 提案: この Story では置き場だけを決め、保持の数と消した記録の扱いは ST58（更新）の深掘りへ申し送る、と C9 に 1 行足す（`docs/handoff/ST58.md` に載せる）
- 処置: deferred ST58 —— 移行の前の退避の保持の数と、退避に残る消した記録の扱いは ST58（更新）の深掘りで決める。C9 に書いた

## R16. C10 の根拠（ST52 Q1「記録を集めていない」）は携帯についての答えで、サーバの DB の中身は確かめていない
- 成果物: openspec/changes/st55-server-installer/premises.md（C10）
- 根拠: `openspec/changes/archive/2026-11-02-st52-release-channels/deep.md:10-12` の問いは「あなたの携帯の収集アプリを…」で、答えは「携帯にはまだ入っていない・記録を集めていない」。本人の PC には main の DB のコンテナ `ashiato2-db-1`（`Exited (255) 2 weeks ago`、`127.0.0.1:55432`、名前の無いボリューム）が残っている。PC の収集アプリの自動起動はスタートアップに無く、`%LOCALAPPDATA%\ashiato` も無い（収集している形跡は見つからない）
- kind: premise
- 提案: C10 の根拠を「携帯（ST52 Q1）と、PC の収集アプリの自動起動が無いこと」に書き直す。main の DB に残したい記録があるかは、中身を読まずに本人に 1 行で確かめる（Q1 の補足に足せる）
- 処置: fixed premises.md C10 —— 根拠を携帯（ST52 Q1）と PC の収集アプリの形跡が無いことに直し、main の開発の DB に残したい記録があるかを Q1 の補足で確かめるようにした

## R17. Q9 の「お使いの Windows のエディションを補足に」は機械で分かる
- 成果物: openspec/changes/st55-server-installer/deep-questions.json（Q9）
- 根拠: `Get-CimInstance Win32_OperatingSystem` の `Caption` = `Microsoft Windows 11 Pro`（本人の PC）。`HypervisorPresent=True`・`VirtualizationFirmwareEnabled=True`、`.wslconfig` に `nestedVirtualization=true`
- kind: technical
- 提案: Q9 の問いからエディションの依頼を外し、上の値を premises に P として置く（確かさ 5）。問いは「どこで確かめるか」だけにする

## 種類ごとの点検で該当の無かったもの

- 不要: 該当なし（Q4〜Q11 はどれも要件・扉で決まっていない。Q9 のエディションの一部だけ R17）
- 分類違い: 該当なし（Q2 discarded・Q3 exported は失うものを名付けられている。Q4〜Q11 の open に失うものは見つからなかった。Q5 の選択肢 3 で先に合言葉を決められる相手は同じ PC のプログラムで、データのフォルダもそもそも読めるので、増える流出は無い）
- 不可逆の根拠が無い: 該当なし（irreversible は Q2・Q3 だけで、どちらも loss がある。誤りは R4・R5）
- 文字で決めさせている画面の問い: 該当なし（画面の構造を問う問いは無い。インストーラは端末）
- 既定の無い仮の問い: 該当なし（Q4〜Q11 の全部に recommended がある。Q1 は premise）
- 処置: fixed deep-questions.json —— Q9 からエディションの依頼を外し、P15（確かさ 5）に置いた

