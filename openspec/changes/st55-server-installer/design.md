# ST55 design — サーバを入れて、そのまま使い始められる

## Context

動機は `proposal.md` の Why。いまの状態（premises の P1〜P19）:

- サーバ（`ashiato-server`）は Windows 向けに組めて、起動の入口まで動く（P1）。起動口は引数なしと `migrate` だけで、Windows のサービスの制御を受ける口が無い（P15）。設定は環境変数からしか読まない（P13）
- 配る形の DB は無い。開発と試験の DB は `docker-compose.yml` の `postgres:17-alpine`（WSL の中の docker）。DB の 3 つの役割は `tools/db-roles.sh` が `docker compose exec` の `psql` で作る（review/deep.md R7）
- 移行の前の退避の既定は作業の場所からの相対と PATH の `pg_dump`（P10）。取り込み待ち・ダウンロード・原本・書き出しの既定は、動いているアカウントの `USERPROFILE` / `LOCALAPPDATA` から組む（P11）。置き場の権限を付けるのは Unix だけ（P12）
- 画面の合言葉は環境変数 `WEB_PASSWORD` の元の値で持ち、ログインの印は元の値の上の HMAC（P14・`web_session.rs:98-105`）。走っている ST44 がログインの印の鍵を `WEB_SESSION_KEY` に替える（ST44 design D8（仮））
- `C:\ProgramData` の下に作ったフォルダは `Users` に読みを受け継ぐ（P6。実物）。サービスの登録簿は `Users` が読める（P7）。本人の PC では 5432 が塞がっている（P8）
- 作業場の lint は `unsafe_code = "forbid"` と `print_stdout = "deny"`（根の `Cargo.toml`）
- 配る道具（ST52）: `tools/release/package.sh`・`check-artifacts.sh`、CI の `package-windows`（`windows-latest`）。許諾の一覧（`THIRD-PARTY-NOTICES.txt`）は ST53 がまだ作っていない

## Goals / Non-Goals

**Goals**: Docker も WSL も無い Windows で、配る zip から 1 回の管理者の確認で DB とサーバを自動で起きるサービスにし、状態を FR-109 の置き場と権限に置き、秘密を自動で作り、最後に決めた画面の合言葉でそのままログインできる。これを Windows の CI で毎回確かめる。

**Non-Goals**:
- PC の収集アプリを入れること・収集アプリの繋ぎ先と資格情報の設定（Q1 の推奨。ST56・FR-128）
- 開発版の並置（別の名前・置き場・番号を実際に入れて確かめること。ST54・FR-106）。ST55 は名前の表（D14）を用意するだけ
- 更新・移行の失敗の戻し・更新係（ST57・ST58）。ST55 は「インストールの走らせ直し」と、更新係が乗る名前と置き場（D14）まで
- 毎日のバックアップ（ST30・FR-70）。置き場 `backups` を用意するだけ（`docs/handoff/ST30.md`）
- 許諾の一覧（FR-127）への統合（ST53。C13 の残り。`docs/handoff/ST53.md`）
- 画面から合言葉を変える口（ST59・FR-133）
- Docker の DB から記録を移す口（D15）・Tailscale の設定（Q6）・Linux の配布（ST60）

## Decisions

### D1. 配る DB は `builtin` の `C.UTF-8`・`UTF8`・チェックサムで作り、版ごとの置き場 `db\17` に置く

`initdb -U ashiato --locale-provider=builtin --builtin-locale=C.UTF-8 --encoding=UTF8 --data-checksums -A scram-sha-256 --pwfile=<一時のファイル> -D C:\ProgramData\ashiato\db\17`。
- DB の管理者の役割の名前は `ashiato` に固定する（本人の答え `deep.md` Q10。2026-11-03。開発の DB の `tools/db-roles.sh` の `POSTGRES_USER` の既定と同じ）。`-U` を渡さないと、名前はインストールを走らせた Windows の利用者の名前になり、`repair`（D27・D28）が設定のファイルの無いときに名前を知る先が無い。インストールの役割の段（D10 の (7)。D7）も `repair` の一時の接続の規則（D28）も、この固定の名前で入る。名前は設定のファイル（D4）にも別の置き場にも控えない
- 理由: premises P4・C1。後から並べ方を変えるには全部を書き出して入れ直す（rewrite-all）が、符号の順の側は失わない（言語の順は列や照会ごとに指定できる）。文字の種類の判定（`lower()`・正規表現。`export/excluded.rs:400-410`）も開発の DB と同じ規則になる
- 選ばなかった案: OS の地域の設定に任せる（開発と並びが変わり、Windows の更新で並べ方の表が変わると索引が黙って壊れうる）・`icu`（同梱する DLL と版が増える）
- Windows の CI で、記録を入れる前に `datlocprovider='b'`・`datlocale='C.UTF-8'`・`UTF8`・`'a' < 'B'` が偽・`data_checksums=on`・管理者の役割（`rolsuper`）が `ashiato` の 1 つだけ を見る（P4 の Windows の側を 4 に上げる）。`--pwfile` の一時のファイルは状態の置き場の中に作り、`initdb` の後に消す

### D2. サーバに `ashiato-server service` の口を足し、DB は同梱の `pg_ctl register` でサービスにする

実装: `windows-service` の入口は `crates/winsvc` に分離する（作業場の `unsafe_code = "forbid"` との衝突を閉じ込める）。

- サーバ: `crates/server/src/service.rs`（`#[cfg(windows)]`）がサービスの制御（開始・停止）を受ける。依存は `windows-service`（MIT / Apache-2.0）。停止の知らせで今の router の graceful shutdown を起こし、取り込み器と書き出しに打ち切りを伝え、書き途中の `.` で始まるファイルを消してから終わる
- `unsafe_code = "forbid"` との両立: `windows-service` の `define_windows_service!` が利用側の crate に `unsafe` の展開を出すなら、サービスの入口だけを持つ小さな crate（`crates/winsvc`。作業場の lint を継がない）に分け、`ashiato-server` からはその安全な関数だけを呼ぶ。下流の最初の Task で `cargo build --target x86_64-pc-windows-gnu` を通して決める
- DB: `pg_ctl register -N ashiato-db -U "NT SERVICE\ashiato-db" -D <db\17> -S auto -w`（`-P` は付けない。仮想アカウントは合言葉を持たない）。登録の後、`sc.exe config ashiato-db obj= "NT SERVICE\ashiato-db"` で確かめ直す
- 選ばなかった案: サービスを包む道具（WinSW・NSSM）を同梱する。許諾と更新の部品が増える（C2）
- 反転条件: `pg_ctl register` が仮想アカウントを受けない（P19 が外れる）なら、`sc.exe create` で `pg_ctl runservice` を登録する形に替える（同じ `pg_ctl` の口）

### D3（仮）. 状態の置き場は継承を切り、2 つの仮想アカウントに読み書き、本人に読みだけ。仮想アカウントで登録できなければ止まる

`C:\ProgramData\ashiato` に `icacls <置き場> /inheritance:r /grant:r "NT SERVICE\ashiato-server:(OI)(CI)M" "<本人の SID>:(OI)(CI)RX"`、`db` と `logs\db` は `NT SERVICE\ashiato-db:(OI)(CI)M` と本人の読みに替え、`ashiato-server` の書きを外す（`logs\db` は DB のログの書き先。D17・deep.md Q9）。
`Users`・`Authenticated Users`・`Everyone`・`Administrators`・`SYSTEM` は付けない（管理者は所有権を取れば読める。ほかの管理者からは守れない —— 残るリスク）。
- 本人の SID は、管理者に上がる前のインストーラが `whoami /user` で控えて、上がった方へ引数で渡す（D22）
- 仮想アカウントで登録できなければ、共有のアカウントへ倒さず rc≠0 で止まる（review/deep.md R8）
- 反転条件: Windows の CI の最初の項目（tasks 3.1）で、仮想アカウントの SID に ACL を付けられない・`postgres` がその SID で置き場に書けないと分かったら、サービスごとの専用のローカルアカウント（合言葉を乱数で作り、設定に置く）へ替える。共有のアカウントへは倒さない

### D4. サービスとして動くサーバは、設定と秘密を `settings\server.json` から読む。表の所有者の合言葉だけは `settings\owner.json` に分ける

- 形は JSON（`serde_json` は既に依存にある）。中身は今の環境変数と同じ名前の鍵（`DATABASE_URL`・`API_TOKEN`・`WEB_SESSION_KEY`・`BIND`・`ASHIATO_INBOX_DIR`・`ASHIATO_DOWNLOADS_DIR`・`ASHIATO_ARCHIVE_USER_ID`・`MIGRATE_BACKUP_DIR`・`PG_DUMP`）と、画面の合言葉の照らすための値 `WEB_PASSWORD_VERIFIER`（D8）と、本人の SID `ASHIATO_OWNER_SID`（走らせ直しで本人を変えないため。D21）
- 原本・書き出し・書庫の写しの置き場（`ASHIATO_MEDIA_DIR`・`ASHIATO_EXPORT_DIR`・`ASHIATO_ARCHIVE_COPY_DIR`）はインストーラが書かない。`service` の口のサーバが、設定のファイルの置き場（`settings` の親 = 状態の置き場）から `media`・`export`・`archive-copies` を既定に組む（`data-durability`・`device-collection` の MODIFIED の「インストールした形で指定が無ければ」。review/spec.md R8）。設定に書かれていればそれが優先する（今の「環境で指定すればそこ」と同じ）
- `service` の口は、tokio の実行環境を起こす前（1 本のスレッドの間）に設定のファイルを読み、鍵を自分のプロセスの環境変数に置いてから、今の `run()` と同じ道を通る。読み方を 2 つに分けない（開発の台本の振る舞いは変わらない）
- 設定のファイルの場所は、サービスの起動の引数 `--settings C:\ProgramData\ashiato\settings\server.json`（場所は秘密でない）
- 表の所有者の接続 `DATABASE_OWNER_URL`（移行を当て、削除の門のトリガを外せる役割の合言葉）は `server.json` に置かず、`settings\owner.json`（鍵はこれ 1 つ）に置く（deep.md Q11 の答え。review/code.md R48）。
  `owner.json` は継承を切って `Administrators`・`SYSTEM` の F だけにし、持ち主も `Administrators` にする（サーバ・DB のサービスのアカウントと本人に何も付けない）。
  インストーラは書きかけを、設定の置き場に新しく作ったフォルダ（`settings\.owner-staging-<印>`。継承を切って `Administrators`・`SYSTEM` だけにし、空であることを確かめる）の中に `create_new` で作って中身を書き、`owner.json` へ名前を替えて置き、明示の ACL と持ち主を付けてからフォルダを消す（設定の置き場に直に作ると、ACL を絞る前に本人やサーバのアカウントが開いた handle で、後から書いた中身を読める。前の回の書きかけの置き場は先に消す）。`repair` も同じ形で書き、(d) で置く。
  置く順は `owner.json` → `server.json`（「設定がある」は 2 つが揃ったとき。片方だけなら D10 の「秘密が無い」と同じに扱い、`repair` が受け付ける）。初めて書く前にサーバのサービスを止める
- 読むのは移行の段（インストーラが `ashiato-server migrate` の環境へ渡す）・走らせ直し（役割を作り直すとき）・`repair`、のちの版を替える口（ST57・ST58）。サーバの `service` の口は `DATABASE_OWNER_URL` を環境へ入れない（`service::SERVICE_EXCLUDED_KEYS`。手で `server.json` に書かれても持たない）
- 管理者は読める（D3 と同じく、ほかの管理者からは守れない）。守るのは、サーバのプロセスを乗っ取られたとき・サーバのアカウントで設定を読めたときに、所有者として削除の門を外されること
- 選ばなかった案: サービスの登録簿の環境変数（P7。ほかのアカウントが読める）・Windows の資格情報マネージャ（サービスの仮想アカウントの保管庫の扱いを確かめていない）

### D5. 配る番号は DB 55400・サーバ 18700（開発版は 55401・18701）、どちらも `127.0.0.1` だけ

`tools/ports.sh` の割り当て（55432・55499・55500〜・55600〜・18787・19000〜・19200〜）と 5432 のどれとも被らない。インストーラは登録の前に 2 つの番号を `Get-NetTCPConnection` 相当（`netstat -ano` を読む）で見て、塞がっていれば持ち主のプロセス名を示して止まる。
`tools/ports.sh` の `55400` は docker の網の /24 を決める差の起点で、待ち受けには使わない（衝突しない）。開発版の番号の確かめは ST54。

### D6. 配る DB の設定は `postgresql.auto.conf` でなく `postgresql.conf` の末尾に書く

`listen_addresses='127.0.0.1'`・`port=55400`・`log_statement='none'`・`log_min_duration_statement=-1`・`log_error_verbosity='terse'`・`log_min_error_statement='panic'`（エラーになった文の字面をログに書かない。既定の `error` だと文ごと書く。review/spec.md R18）・D17 のログの設定。
`log_min_error_statement` は開発の DB（`docker-compose.yml`）の 3 つの設定に無いので、製造準備の棚卸し（`docs/production-prep.md`）に配る DB の行として足す。開発の DB には足さない（壊してはいけないものの 3 つの設定を変えない）`initdb` の後、サービスを登録する前に書く。
`ALTER SYSTEM` で書く `postgresql.auto.conf` は、後で人が `ALTER SYSTEM` を打つと上書きされうるので使わない。

### D7. 秘密は OS の暗号の乱数で作り、DB の 3 つの役割はインストーラが同梱の `psql` で `tools/db-roles.sh` と同じ SQL で作る

- 秘密（DB の管理者・`ashiato_owner`・`ashiato_app` の合言葉、`API_TOKEN`、`WEB_SESSION_KEY`）は 32 バイトの乱数の base64url（43 文字）。`rand` の `OsRng`
- 役割の SQL は `tools/db-roles.sql`（新しいファイル）に出し、`tools/db-roles.sh` もそれを `psql` に流す形に替える（台本を 2 つに分けない）。インストーラは同じファイルを配布物に入れて同梱の `psql` に流す。合言葉は `psql` の変数（`-v`）で渡し、命令の引数に載せない（`PGPASSWORD` は管理者の接続にだけ使う。管理者の役割の名前は D1 の固定の `ashiato` で、`db-roles.sql` の `admin` の変数にも同じ名前を渡す）
- 移行の前の退避の置き場 `MIGRATE_BACKUP_DIR=C:\ProgramData\ashiato\backups\pre-migrate`・`PG_DUMP=C:\Program Files\ashiato\pgsql\bin\pg_dump.exe` を設定に書き、インストーラの `migrate` にも同じ値を渡す（申し送り K1）。退避の保持の数は ST58

### D8. インストールした形の画面の合言葉は Argon2id の照らすための値だけで持ち、ログインの印の材料もその値にする

- 照らすための値: Argon2id（`argon2` crate。MIT / Apache-2.0）、PHC 文字列（`$argon2id$v=19$m=19456,t=2,p=1$<塩>$<値>`。OWASP の最小の推奨）。設定の鍵は `WEB_PASSWORD_VERIFIER`
- 照らす形かどうかは**口で決める**（review/spec.md R6）: `service` の口では照らす形だけで、`WEB_PASSWORD_VERIFIER` が無い・形が違う・`WEB_PASSWORD` がある、のどれでも起動しない。サービスでない口（開発の台本）は今の `WEB_PASSWORD` の形だけで、`WEB_PASSWORD_VERIFIER` があれば起動しない（曖昧にしない）
- 照らす形の起動の検査: `API_TOKEN` と `WEB_SESSION_KEY` をそれぞれ照らすための値と照らし、合えば（画面の合言葉と同じなら）起動しない（Argon2id 2 回。起動のときだけ。review/spec.md R7）
- ログインの印の材料: 照らす形では `"ashiato-web-session-v2" || WEB_PASSWORD_VERIFIER`（塩を含むので、同じ合言葉で決め直しても印が変わり、それまでのログインは切れる）。元の値の形は ST44 のまま（`v1`）。鍵は ST44 の `WEB_SESSION_KEY`
- `WEB_SESSION_KEY` が画面の合言葉と同じでないかの検査（ST44）は、照らす形では鍵を照らすための値と照らして行う
- 長さ・`change-me`・API の合言葉とログインの印の鍵との違いはインストーラが決める時に見る（`data-sensitivity` の MODIFIED）
- ST44 の D8（仮）の「印の内容は変えない」は、照らす形でだけ外れる。ST44 へは差し戻さない（ST55 の Task で `web_session.rs` を直す）
- 選ばなかった案: 元の値を設定に置く（状態の置き場は本人とサービスしか読めないが、人が決める合言葉はほかでも使い回されうる）・bcrypt（72 バイトで切れる）

### D9. 取り込み待ちは `%USERPROFILE%\ashiato\取り込み待ち`、ダウンロードは `shell:Downloads` の実の場所

インストーラ（上がる前）が本人の `USERPROFILE` と、登録簿 `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders` の `{374DE290-123F-4565-9164-39C4925E467B}` を展開した場所を控え、上がった方へ渡す（D22）。
上がった方が `icacls <ダウンロード> /grant "NT SERVICE\ashiato-server:(OI)(CI)RX"`、取り込み待ちを作って `M` を足し、親の `%USERPROFILE%\ashiato` にそのフォルダだけの「フォルダを作る」（`AD`）と読み（`RX`）を足す —— 本人が取り込み待ちを消しても、取り込み器が作り直せる（正典 external-ingestion「専用のフォルダが無ければ作る」。review/spec.md R10）。ダウンロードが OneDrive の下なら、そのまま読む（読むだけで、上げるのは本人の操作）。
「ドキュメント」の下に作ると書庫が OneDrive に上がる（外に出る）ので、出ない側に倒す（premises P9・C9）。`docs/archive-inbox.md` をこの形に書き直す。

### D10. 走らせ直しは足りないものだけ足し、秘密を先に書いてから `initdb` する。DB があるのに秘密が無ければ止まり、`repair` の口を示す

> 仮で進めていたが、別の席（`provisional-review.md` の D10）が否とし、`deep.md` の Q8 で本人が「この Story で出口を作る（`ashiato-setup.exe repair`）」を選んだ（2026-11-03）。出口は D27

インストーラの段: (1) 置き場と ACL → (2) 秘密が無ければ作って設定に書く → (3) DB の置き場が空なら `initdb` → (4) DB の設定 → (5) プログラムの置き場への写しとサービスの登録（無ければ。写しと `sc.exe create` は D29（仮））→ (6) DB の開始 → (7) 役割（無ければ）→ (8) 移行 → (9) 取り込み待ちとダウンロードの権限 → (11) 画面の合言葉（照らすための値が無ければ）→ (10) サーバの開始 → (12) 「アプリと機能」→ (13) 案内（Tailscale・空き）。
(11) を (10) の前に置く: `service` の口のサーバは照らすための値が無いと起動しない（D8）ので、合言葉を決めてからサーバを開始する（final review R6〜R9 の直しで実行順に合わせた。番号は Scenario と tasks の参照のため変えない）。
(1) の ACL は状態の置き場を初めて作るときだけ付ける（サービスが書いたファイルができた後は変えない。D21）。`initdb` の一時の合言葉のファイルは `settings` に置き、成否に依らず消す（D1）。役割を作る前に落ちた DB の扱いは D30（仮）。
各段は「既にあるか」を見て、あれば触らない。DB の置き場に `PG_VERSION` があるのに設定のファイルが無ければ、**(2) より前に**判定して何も書かずに rc≠0 で止まる（(2) が先に新しい秘密を書くと、次の走らせ直しで「DB と秘密がある」に見えて食い違いを示せなくなる。provisional-review.md の D10）。
止まるときの出力に、DB と秘密の食い違い・状態の置き場の場所と、出口の `ashiato-setup.exe repair`（D27）を示す。走らせ直しの口のまま秘密を作り直すことはしない（食い違いを本人が見ずに直ると、収集アプリの資格情報が黙って切れる）。

### D11. サービスは「自動」・回復で起こし直し・サーバは DB に依る。移行はインストールのときだけ

`sc.exe config ashiato-server depend= ashiato-db start= auto`・`sc.exe failure ashiato-server reset= 86400 actions= restart/10000/restart/10000/restart/60000`（DB も同じ）。
サーバの `service` の口は、DB へ 1 秒ごとに繋ぎ直し、最長 120 秒待つ（その間はサービスの状態を「開始中」で告げる）。120 秒で繋げなければ理由をログに書いて終わる（回復が起こし直す）。
起動のたびには移行を当てない。版の食い違いはサーバが起動の時に止める（ST48）。

実装の注（final review R63・R60）:
- 回復の動作は 3 回とも 10 秒（`actions= restart/10000/restart/10000/restart/10000`、`reset= 86400`。DB も同じ）。3 回目を 60 秒にすると、強制終了の直後に PostgreSQL の子プロセスや待ち受けの解放と競って失敗した回復の次の試みが、実地の検査（install-windows）の 90 秒の内に来ない。失敗した回復も同じ短い間隔で続け、サービスを止めたままにしない（`crates/installer/src/commands.rs` の `configure_service_commands`）
- 両方のサービスに `sc.exe failureflag <サービス> 1` を当てる。`failure` だけでは異常終了（プロセスの消滅）でしか回復が動かず、サービスが自分で「止まった」を非 0 の終了コードで返したときに動かない
- `failureflag 1` は「止まった かつ 終了コードが非 0」で回復を動かすので、サーバの `service` の口は終了コードで回復の要否を告げる（`service::ServiceEnd`）。起こし直しても直らない拒否（設定のファイルが無い・読めない・形が違う、設定の値の誤り、合言葉・ログインの鍵・待ち受け・DB の接続先・役割・DB の版の拒否）は理由をサーバのログに書いて **0** で終わり、回復に繰り返させない。120 秒待っても DB に繋げない（`database_unavailable`）・実行環境を作れない・型の無い失敗（待った後の切断・起動後の待ち受けの失敗など）は **1** で終わり、回復が起こし直す（1 日 3 回まで）

### D12. インストーラはネットワークに出ない。ウイルス対策の除外は入れない

インストーラの crate は網の依存を持たない（`ureq` などを入れない）。PostgreSQL は配布物に同梱する。外への接続を止めた状態でもインストールが終わることを Windows の CI で見る（Windows のファイアウォールで、展開した置き場とプログラムの置き場の実行ファイルの外向きを止める）。

### D13. 同梱する PostgreSQL の許諾の文を配布物に入れ、`check-artifacts.sh` で見る

EDB の zip の中の PostgreSQL の許諾の文（`pgsql\doc\...` の `COPYRIGHT` 相当。tasks 1.1 で実在を確かめる）と、同梱する DLL（OpenSSL・ICU・zlib・libxml2 など、`bin` に残すもの）の許諾を `pgsql\LICENSES\` に入れる。
FR-127 の一覧（`THIRD-PARTY-NOTICES.txt`）への統合と NFR-26 の検査（`tools/check-licenses.sh` は同梱のバイナリを見ない）は ST53 へ申し送る（`docs/handoff/ST53.md`）。

### D14. サービスの名前と置き場の並び

| | 正式版・ベータ版 | 開発版（ST54 が確かめる） |
|---|---|---|
| DB のサービス | `ashiato-db`（`NT SERVICE\ashiato-db`） | `ashiato-db-dev` |
| サーバのサービス | `ashiato-server` | `ashiato-server-dev` |
| プログラムの置き場 | `C:\Program Files\ashiato` | `C:\Program Files\ashiato-dev` |
| 状態の置き場 | `C:\ProgramData\ashiato` | `C:\ProgramData\ashiato-dev` |
| 番号 | DB 55400・サーバ 18700 | DB 55401・サーバ 18701 |
| 取り込み待ち | `%USERPROFILE%\ashiato\取り込み待ち` | `%USERPROFILE%\ashiato-dev\取り込み待ち` |

状態の置き場の中: `db\17`・`media`・`export`・`backups\pre-migrate`（と ST30 の毎日のバックアップ）・`archive-copies`・`logs\db`・`logs\server`・`settings`。
インストーラは自分の組み立ての印の版番号（`-dev.` を含むか）で列を選ぶ。表は `crates/installer/src/layout.rs` の 1 か所に置き、更新係（ST57）と更新（ST58）が読む。更新係の置き場は ST57 が足す。
正典の既定の置き場（`data-durability` の書き出し・`device-collection` の原本）は、インストールした形でこの表の `export`・`media` にする（MODIFIED。申し送り C6）。印つきの試験 `export_tests.rs:73`・`media_tests.rs:211` は今の既定（開発の形）のまま残し、インストールした形の既定の試験を足す。

### D15. Docker の DB から記録を移す口は作らない

FR-108 の ★ 2026-11-03（Q5。移したい記録は無い）。

### D16（仮）. 取り込みの利用者は、収集アプリと画面と同じ既定の利用者（nil の UUID）を渡す

設定の `ASHIATO_ARCHIVE_USER_ID=00000000-0000-0000-0000-000000000000`。インストールごとに作り直さない。
- 渡し忘れると取り込み器は `archive_disabled` を書いて黙って起きず（`lib.rs:4433-4448`）、別の値だと記録が画面に出ないまま門が書き換えを拒む（rewrite-all）
- 反転条件: 複数の利用者（FR の利用者ごとの分け）を持つ Story が来たら、インストールのときに利用者を作る形に替える（それまでの記録は既定の利用者のまま）

### D17. DB とサーバのログは状態の置き場の `logs` にファイルで書き、送った文の字面と秘密はイベントログに出さない（起動の最初の数行と `pg_ctl` の知らせは出る）

> 仮で進めていたが、別の席（`provisional-review.md` の D17）が否とし、`deep.md` の Q9 で本人が「`logs\db` に DB のサービスの書きを足し、起動の数行がイベントログに出るのは受け入れる」を選んだ（2026-11-03）

- 権限は D3 に乗る: `logs\db` は D3 の ACL で `NT SERVICE\ashiato-db` が書き（`ashiato-server` は書かない）、`logs\server` は根の `NT SERVICE\ashiato-server` の書きを受け継ぐ。どちらも本人は読みだけ
- DB: `logging_collector=on`・`log_destination='stderr'`・`log_directory='C:/ProgramData/ashiato/logs/db'`・`log_filename='postgresql-%a.log'`・`log_rotation_age=1d`・`log_truncate_on_rotation=on`（曜日のファイルを上書きするので 7 日分）
- イベントログに出るもの: サービスとして動く PostgreSQL は、ログの収集へ切り替わる前の出力（起動の最初の知らせ・設定の読み違い）と `pg_ctl` 自身の知らせをアプリケーションのイベントログへ書く（出し元 `PostgreSQL`）。これは止めない（Q9）。送った文の字面と秘密は、D6 の `log_statement='none'`・`log_min_error_statement='panic'` でログの行にそもそも載らず、切り替え前の出力にも含まれない（確かさ 1。Windows の CI の `送った文と秘密はイベントログに出ない` で見る）
- サーバ: `service` の口では `tracing_subscriber` の書き先を `logs\server\server-<日付>.log` に替え（新しい依存を足さず、日付が変わったら開き直す自前の書き手）、起動のときに 30 日より古いファイルを消す。起動の拒否（`eprintln!` の所）も同じ書き手に書く。サーバはイベントログに書かない
- 反転条件: Windows の CI で、切り替え前の出力に送った文の字面か秘密が載ると分かったら、その出力を作る設定（`log_min_messages` など）を絞る。本人が「イベントログで見たい」と言えば、起動の拒否の 1 行（理由の種類だけ。値を含まない）をイベントログにも出す

### D18（仮）. 本人の切り替えでは、いまの開発の `API_TOKEN` と繋ぎ先を引き継がない

`docs/release.md` §7 を「インストーラで入れる → 画面の合言葉を決める → 画面にログインする。収集アプリを配った版のサーバに繋ぐのは ST56 から」に書き直す。
- 携帯の収集アプリはまだ入っておらず（ST52 Q1）、PC の収集アプリの自動起動も無いので、合わなくなって捨てる記録は今は無い
- 反転条件: ST56 より前に本人が収集アプリを配った版のサーバへ繋ぎたいと言ったら、設定の `API_TOKEN` を管理者として読み出す手順を `docs/install.md` に足す

### D19. 同梱する PostgreSQL は `tools/release/pgsql.sh` が版と SHA-256 を固定して取り、要るものを切り出す

`tools/release/pgsql.lock`（新しいファイル）に `version=17.6-1`・`url=https://get.enterprisedb.com/postgresql/postgresql-17.6-1-windows-x64-binaries.zip`・`sha256=<tasks 1.1 で控える>`。
`pgsql.sh fetch <置き場>` は落として指紋を比べ、違えば rc≠0。`pgsql.sh slim <展開した置き場> <出力>` は `bin`（要る実行ファイルと DLL）・`lib`・`share` と許諾の文だけを写す。
残す実行ファイルの一覧も `pgsql.lock` に置く（`postgres.exe`・`initdb.exe`・`pg_ctl.exe`・`psql.exe`・`pg_dump.exe`・`pg_restore.exe`・`pg_isready.exe`）。DLL は実行ファイルが読むものを残す（tasks 1.1 で `dumpbin /dependents` 相当を当てて一覧を作る）。
組み立ての間は網に出てよい（インストールの間は出ない。D12）。CI は落とした zip を `actions/cache` に版と指紋の鍵で置く。マイナー版の差し替え（FR-132）は `pgsql.lock` の 3 行を変える PR。

### D20. インストーラは新しい crate `crates/installer`（bin `ashiato-setup`）にし、この crate だけ `print_stdout` を許す

作業場の member にし、`[package] name = "ashiato-setup"`（bin の名前と同じ。tasks の `-p ashiato-setup` はこの名前。review/spec.md R21）、版は根の `Cargo.toml` の 1 つの出所（ST52）。`build.rs` は `ashiato-build-stamp` で印を焼く。
端末に案内と問いを書くのが役目なので、`[lints]` で作業場の lint を継いだ上で、`main.rs` の先頭に `#![allow(clippy::print_stdout)]` を理由のコメント付きで置く（ほかの crate は今のまま）。
画面は端末の文字だけ（Q2 の推奨）。Windows でしか動かない部分（`icacls`・`sc.exe`・`pg_ctl` を呼ぶ所）は、命令を組む純粋な関数（Linux の `cargo test` で試す）と、呼ぶ薄い層に分ける。

### D21. 置き場に書く段の間だけ、上がった利用者に書きを足し、終わりに外す

状態の置き場の持ち主は、作った上がった利用者の `Administrators`（Windows の既定）で、持ち主は ACL を書き換えられる。上がったインストーラは、書く段（(1)〜(11)・`password`・`repair`・`uninstall`）の間だけ、自分の SID に、その段が書く置き場（`settings`・`db\17`・`backups`・`logs`）の `M` を足し、段の終わりと、途中で落ちた走らせ直しの最初に外す。
`initdb` と `pg_ctl` は管理者の権限を落とした札（利用者の SID は残る）で動くので、この一時の書きで足りる。別の管理者 B で走らせ直したときも、同じ形で B に一時の読み書きを足し、本人は設定の `ASHIATO_OWNER_SID` から読んで変えない（review/spec.md R9）。
外したことを Windows の CI で ACL を読んで見る（spec の「インストールが終わると、インストールの間の書きの権限が残らない」）。
- 一時の書きを足す置き場の狭め（final review R4 の直し）は D31（仮）
- 選ばなかった案: `initdb` を DB のサービスのアカウントで走らせる（仮想アカウントの札を作る口が無い）・`Administrators` に恒久で書きを付ける（FR-109 が崩れる）

### D22. 上がる前に本人を控え、自分を管理者として起こし直す

`ashiato-setup.exe`（引数なし）は上がらずに起き、本人の SID・`USERPROFILE`・ダウンロードの実の場所を控え、`powershell -NoProfile -Command Start-Process -Verb RunAs -FilePath <自分> -ArgumentList 'install','--owner-sid',<SID>,'--profile',<場所>,'--downloads',<場所>'` で自分を管理者として起こし直す（確認はここの 1 回だけ）。
上がった方は新しい端末の窓で動き、終わったら「Enter で閉じる」を出す。上がった方が既に管理者のとき（CI）は、起こし直さずにその場で続ける。`password`・`repair`・`uninstall` の口も同じ形。
- 選ばなかった案: 実行ファイルに「管理者として動く」の印（manifest の `requireAdministrator`）を付ける —— 開いた時点で上がるので、別の管理者の資格情報で上がると本人を取り違える
- `ShellExecute` を直に呼ぶと `unsafe` が要る（作業場の lint）ので、`powershell` の `Start-Process` を使う

### D23. 端末でないときは、標準入力の 2 行を画面の合言葉として読む

Windows の CI でインストールを最後まで通すため。端末かどうかは `std::io::IsTerminal`。端末のときは入力を表示しない読み方を `rpassword`（Apache-2.0。`unsafe` は依存の crate の中に閉じ、作業場の lint に当たらない）で行う。
端末でないときに読んだ 2 行も、端末のときと同じ検査（D8・spec の「インストールの最後に…」）を通る。

### D24. Windows の CI の job `install-windows` が、配る zip と試験の zip からのインストールを毎回確かめる

`ci.yml` の `package-windows` が、配る組み立て（`ASHIATO_RELEASE=1`）の zip と、同じ `package.sh windows --test-build` で組む**試験の組み立て**（`ASHIATO_RELEASE` 無し。差し込みと命令の記録を読む）の zip の 2 つを作る。
`install-windows`（`windows-latest`）は `tools/release/install-check.sh` を Git Bash で走らせ、差し込みにも記録にも依らない Scenario を配る zip で、差し込み（`ASHIATO_SETUP_TEST_<名前>`・サーバの `ASHIATO_SERVICE_TEST_<名前>`）か命令の記録（`ASHIATO_SETUP_TEST_TRACE=1`）に依る Scenario を試験の zip で走らせる（review/spec.md R2）。
どちらの zip で走らせたかは台本の節の見出し（`=== 配る zip`・`=== 試験の zip`）で示す（各 Scenario の `echo` の行は spec の名前と一字一句合わせるため、後ろに何も付けない）。`check-artifacts.sh` が配る zip の 2 つの実行ファイルに差し込みの名前の文字列が無いことを見る。
- 台本は始めに、上がった札（High Mandatory Level）で動いていることを `whoami /groups` で見て、違えば全部の前に落ちる（premises P17 が外れたことを 1 行で示す。review/spec.md R3）。次に P19（仮想アカウントの登録と書き込み）を見る
- 利用者の地域の設定は `Set-Culture ja-JP` で日本語にしてからインストールする（D1 の確かめが OS の設定に倒れないことを見る）
- 別の利用者（管理者でない A）は `net user` で作り、`Start-Process -Credential` で動かす。終わりに消す
- job は `crates/installer`・`crates/server`・`migrations`・`tools/release`・`tools/db-roles.*` を触った PR で走る（`changes` の出し分け）
- Scenario の印（`echo '== Scenario: …'`）は、この job が走らせる実地の確かめ（配る zip と試験の zip の経路）にだけ置き、`--self-test` には置かない（final review R10）。偽の命令に合わせた確かめに印を置くと、印が実物を確かめていないまま緑になる。`--self-test`（chain の job で毎回走る）は、確かめの部品の判定と「`--list` の全部が実地の確かめに 1 つずつ印を持つ」ことを見るので、job の `comm -23` は構造的には欠けない

### D26（仮）. 再起動の後に動いていることは、Windows サンドボックスで状態が残るときだけ実物で見る。残らなければ CI の「自動・サービスのアカウント」で代える

ST55 の完了の判定 1 つ目の「PC を再起動するとログオンしなくても動いている」を、Q5 の推奨の逃げ道（サンドボックスで再起動の後に状態が残らない）では実物で確かめられない（review/spec.md R19）。
そのときは、開始の種類が「自動」で、サービスが利用者でないアカウントで動き、ログオンした利用者の札に依らないこと（CI）で代え、確認バッチの 12.2 に「再起動は見ていない」と書いて本人に見せる。
- 失わない側: 本人の環境の切り替え（`docs/release.md` §7）の最初の再起動で実物が通る。そこで動いていなければ、携帯は溜めて送る（記録は失わない）
- 反転条件: サンドボックスで再起動の後に状態が残ると分かれば、12.2 で再起動まで見る。本人が「実物の再起動を見てから配りたい」と言えば、Q5 の案 2（Hyper-V の仮想マシン）へ替える

### D25. 消す口は `ashiato-setup.exe uninstall`。自分の実行ファイルは消した後の 1 回の再起動で消える

サービスを止めて消し（`sc.exe delete`）、ダウンロードと取り込み待ちの ACE を外し、「アプリと機能」の鍵を消し、プログラムの置き場を消す。走っている `ashiato-setup.exe` 自身は消せないので、`%TEMP%` に写して写しから消し、写しは終わった後に `cmd /c` の遅らせた `del` で消す。
状態の置き場と取り込み待ちの中身は消さず、その場所と `docs/install.md` の「記録を手で消す」を出す。

### D27. 出口は `ashiato-setup.exe repair`。DB の役割の合言葉と秘密を作り直し、記録には触らない

本人の答え（`deep.md` Q8。2026-11-03）。D10 で止まった状態（DB の置き場に `PG_VERSION` があり、設定のファイルが無い）からだけ動く口。
- 上がり方は `password`・`uninstall` と同じ（D22。管理者の確認 1 回・上がる前に本人を控える）。書く段の間だけの書きの権限も同じ（D21）。止まった状態でなければ（設定のファイルがある・DB が無い）、何も書かずに rc≠0 で「`repair` は要らない」と、走らせ直しか `password` を示して終わる
- 段: (a) 2 つのサービスを止める → (b) 秘密を全部（DB の 3 つの役割の合言葉・`API_TOKEN`・`WEB_SESSION_KEY`）作り直し、`settings\server.json.repair`（同じ置き場・同じ権限）に、表の所有者の合言葉は `owner.json` の書きかけ（D4 の書きかけの置き場）に書く（本人は、設定が無いので状態の置き場の根の ACL から読む —— 2 つのサービスのアカウントのほかに読みを持つ 1 つの SID。上がる前に控えた利用者が別の管理者でも、本人にしない。D21・spec「別の管理者が repair しても、本人は変わらない」。読みを持つ SID が 1 つでなければ何も書かずに rc≠0）→ (c) DB を合言葉なしで入れる形で一時に起こし、3 つの役割の合言葉を新しい秘密に替える → (d) 一時の形を止め、`owner.json` の書きかけを `owner.json` へ、次に `server.json.repair` を `server.json` へ名前を替えて置き、ふだんの設定でサービスの DB を開始 → (e) 走らせ直しの (5)〜(13) を同じ判定で通る（画面の合言葉は照らすための値が無いので (11) で決め直す。移行は版が合っていれば当たるものが無い。(9) の取り込み待ちとダウンロードは、本人の SID のプロファイル（登録簿の `ProfileList`）で「既にあるか」を見て、あれば触らない）
- 途中で落ちても、`server.json` を置くのは役割の合言葉を替えた後なので、状態は「DB はあるのに秘密が無い」のまま（走らせ直しは止まり、`repair` をもう一度走らせれば (b) から作り直す。残った `server.json.repair` は (b) が上書きする）。「秘密はあるのに役割の合言葉が古い」には倒れない
- (c) の一時の形は D28（管理者の役割の名前は D1 の固定の `ashiato`。Q10）
- `repair` の最初（(a) の後・(b) の前）に、前の `repair` が (c) の後で落ちて一時の形が残っていれば片付ける: DB の置き場の `postmaster.pid` があり DB のサービスが止まっていれば、一時に起こした `postgres` とみなして `pg_ctl stop -D <DB の置き場> -m fast` で止め、残った一時の接続の規則（一時の `hba_file` と `pg_ident`）を消す。一時の `postgres` はサービスでないので (a) では止まらず、残ると次の `pg_ctl start` が `postmaster.pid` で断られる（provisional-review.md の D28。答えに依らない）。tasks 6.2 の「`repair` の途中で落ちても、もう一度 `repair` できる」は、`STOP_AT=repair-roles` が一時の `postgres` を残したまま落ちる形で走らせて見る
- 記録の行・移行の記録・原本・書き出しには触らない。収集アプリの資格情報（`API_TOKEN`）は変わるので、出力の最後に「収集アプリは繋ぎ直しが要る」と `docs/install.md` の繋ぎ直しの節の場所を示す（値は示さない。収集アプリを配った版のサーバに繋ぐのは ST56 から。D18（仮））
- Windows の CI で「設定を退ける → 走らせ直しが止まる → `repair` → 記録の件数と中身の指紋が同じ・新しい合言葉でログインできる・前の `API_TOKEN` は断られる」を見る（tasks 6.2）

### D28. `repair` の一時の形は、`pg_ctl start` に一時の `hba_file`（`sspi` で上がった利用者だけ）を渡し、固定の名前 `ashiato` の管理者の役割に入る

> 仮で進めたが、別の席（`provisional-review.md` の D28）が否とした: 一時の `hba_file` と `pg_ident` は DB の管理者の役割の名前を要るのに、D1 の `initdb` は `-U` を渡さないので名前はインストールを走らせた Windows の利用者の名前になって固定されず、D4 の設定の鍵にも無く、`repair` は設定のファイルが無いときに動くので名前を読む先が無い（別の管理者 B の `repair` では B を A の名前の役割に結べない。反転条件の `local … trust` も同じ名前を要る）。名前が引けなければ `repair` は (c) で止まり、出口の無い D10 の状態に戻る（携帯の未送信が FR-9 の上限で捨てられる。discarded）。土台の D1・D4・D6 を台帳の rests_on に挙げていなかった。台帳から外し、`deep.md` の Q10 で人間に返した。本人は「DB を作るときに名前を固定する（`initdb -U ashiato`）」を選んだ（Q10。2026-11-03）。この節はその答えに乗る決定で、仮ではない。土台は D1（`-U ashiato`）・D4（設定に名前を控えない）・D6（一時の起動も同じ `postgresql.conf` を読み、文の字面をログに書かない）と D27・D21。前の `repair` の一時の形が残っていたときの片付けは答えに依らないので D27 に書いた

`pg_ctl start`（上がった札を落として `postgres` を起こす。D21）に、`-o` で `listen_addresses='127.0.0.1'`・空いている一時の番号・一時の `hba_file`（`host all ashiato 127.0.0.1/32 sspi map=<一時>`、`pg_ident` で上がった利用者の Windows のアカウントだけを管理者の役割 `ashiato`（D1）に結ぶ）を渡す。`psql` も `-U ashiato` で入る（名前は D1 の固定の値で、設定のファイルから読まない。別の管理者 B の `repair` でも、`pg_ident` が B のアカウントを同じ `ashiato` に結ぶ）。合言葉なしで入れるのは上がった利用者だけで、ほかのアカウントには開かない。spec の DB の Requirement はこの一時の入口を `repair` の間だけの例外とし、終わりに閉じたこと（一時の `hba_file` が無い・待ち受けが 55400 だけ）を Scenario で見る。`ALTER ROLE … PASSWORD` の値は `tools/db-roles.sh` と同じく環境変数で渡して `\getenv` で読む（命令の引数・SQL の字面に載せない）。一時の `hba_file` は (d) で消す
- `postgres --single` を直に起こす形（Q8 の選択肢の書き方）は使わない: PostgreSQL は管理者の札での起動を断る作りで（確かさ 1。知識）、上がったインストーラからは起こせないとみなす。どちらの形でも、本人に見える振る舞い（記録を残したまま役割と秘密が作り直される）は同じ
- 失わない側: 一時の形は `repair` の中だけで、記録にも本人に見える振る舞いにも触らない。形を替えても `repair` の口と Scenario は変わらない
- 反転条件: Windows の CI で一時の形の `sspi` の照合が通らない（上がった札と `pg_ctl` が落とした札で Windows のアカウントが食い違う など）と分かったら、`unix_socket_directories` を状態の置き場の中の一時のフォルダ（上がった利用者と DB のアカウントだけが読み書き）にし、`local all ashiato trust` で入る形に替える。どちらも通らなければ上流へ返す（出口の形を変えるかは本人に聞く）

### D29（仮）. インストーラはプログラムの置き場へ写してから、サーバのサービスを `sc.exe create` で作る

D10 の (5) で、zip を展開した置き場（インストーラ自身の置き場）から `ashiato-server.exe`・`ashiato-setup.exe`・`db-roles.sql`・`pgsql\` を `layout.program_dir`（`C:\Program Files\ashiato`。D14）へ写す（既にあるものは写さない）。続けて DB は同梱の `<program_dir>\pgsql\bin\pg_ctl.exe register`（D2）、サーバは
`sc.exe create <server_service> binPath= "\"<program_dir>\ashiato-server.exe\" service --settings \"<state_dir>\settings\server.json\"" obj= "NT SERVICE\<server_service>" start= auto depend= <db_service>` で作り、どちらも無いときだけ登録する（走らせ直しと `repair` で重ねない）。`initdb`・`pg_ctl`・`psql`・`pg_dump` は `<program_dir>\pgsql\bin` の絶対の場所で呼ぶ（PATH に頼らない）。命令は PowerShell を通さず引数のまま渡す（引用の崩れを避ける）。
- 理由: spec の Scenario と D11 の `sc.exe config ashiato-server` は、サーバのサービスとプログラムの置き場の実行ファイルがあることを前提にしていたが、D10 の段に作る段が無かった（final review R6）。`pg_ctl register` は自分の場所をサービスに書くので、写した後の `pgsql` から呼ぶ。`UninstallString`（D25）も写したインストーラを指す
- 失わない側: 写しと作成はインストールの中だけで、要件も本人に見える振る舞いも変えない。既にある実行ファイルを上書きしないので、更新（ST57・ST58）の形は先取りしない
- 反転条件: 本物の Windows の CI（`install-windows`）で、仮想アカウントを `sc.exe create … obj=` で名指しできない・`binPath=` の引用が崩れる・動いているサービスの実行ファイルの写しで止まる、などで別の形が要ると分かったら、`New-Service`（PowerShell）で作ってから `sc.exe config obj=` で仮想アカウントへ替える形か、写しを止めた後に行う形に替え、この節を書き直す

### D30（仮）. 役割を作る前に落ちた DB は、DB のサービスを一度も起こしていなければ作り直し、起こしていれば止まって消す手順を示す

**finish（review/code.md R40）で D32（仮）に置き換えた。** 「完了記録とサービスの有無」から「記録は無い」を推していたが、repair の途中で落ちた DB（記録が入っている）も同じに見えた。台帳では resolve 済み。以下は経緯。

final review R5・R9 の直しで、D10 に無かった振る舞いとして足した（re-review N4）。管理者の合言葉はどこにも残らない（Q10・D1）ので、役割を作る前に落ちた DB に役割を作り直す口が無い。
DB のサービスを一度も起こしていない（`.installer-roles-complete` が無く、DB を起こした印も無い）DB の置き場は自動で消して `initdb` からやり直す。起こしていれば止まって `db\17` を消す手順（`docs/install.md`）と止まった理由「DB の役割を作る前に止まった DB です」を示す。
- 失わない側: 消すのは役割の前・サービスの起動の前の DB だけで、記録はまだ 1 行も入っていない
- 反転条件: 本人が自動で消すことを望まないなら、作り直しをやめ、いつも止まって手で消す手順を示す形に戻す

### D31. 一時の書きは、インストーラが持ち主の置き場（`settings`・`backups\pre-migrate`）と、DB を起こす前の `db` にだけ足す

D21 の本文は書く段の間 `settings`・`db\17`・`backups`・`logs` に一時の `M` を足すと書く。final review R4 の直しで次のように狭めた（re-review N4）。
一時の書きは、インストーラだけが書きその中のファイルの持ち主もインストーラである `settings`・`backups\pre-migrate` に足して `/reset` で外し、`db` には `initdb` と DB の設定を書く間（DB のサービスを起こす前）だけ足す。持ち主がサービスのアカウントのファイル（`logs`・`db\17` の中・根の下にサービスが作るもの）へは ACL の変更が届かない見込みなので、そこへは足さない（`repair` の `db` だけは D28 の一時の形のために足す。Windows の CI で落ちれば D28 の反転条件と一緒に見直す）
- 失わない側: 一時の書きの置き場を変えるだけで、恒久の ACL（FR-109）も記録も変えない
- 反転条件: Windows の CI で、`repair` の一時の DB の起動が `logs\db` に書けずに落ちる（review/code.md R32）・持ち主がサービスのファイルへも ACL の変更が届く、などと分かったら、D21 の本文の置き場（`logs` を含む）に戻すか、`repair` の一時の起動にログの置き場を別に渡す

### D32（仮）. 自動で作り直すのは、このインストーラが `initdb` を始めた印のある DB だけ。印が無ければ消さずに止まり、`repair` を示す

D30（仮）を置き換える（review/code.md R40）。インストーラは `initdb` の前に `settings\.installer-initdb-started` を書き、役割を作り終えて `.installer-roles-complete` を書いた後に消す。
走らせ直しで「DB あり・役割の完了記録なし」のとき、印があり、DB のサービスがまだ無く、移行の完了記録も無ければ（印は設定の置き場にあり、サーバのアカウントも書けるので、印だけでは消さない）、D30 と同じく `db\17` を消して `initdb` からやり直す。印が無ければ消さずに「DB の役割の完了記録がありません。記録が入っているかもしれないので、… は消しません。ashiato-setup.exe repair で DB の役割を作り直してください」で止まる。
`repair` はこの形（設定はあるが役割の完了記録が無い）も受け付ける（`state::repair_allowed`）。`repair` は役割の完了記録を書いてから `server.json.repair` を `server.json` へ置く（名前替えを最後にする。置いた後・記録の前に落ちると、記録の入った DB が「役割の前の DB」に見えた）。
移行の完了記録は `repair` が書かない（あればそのまま、無ければ走らせ直しの (8) が移行を当てる。移行は当たっていない版だけを当てるので、当たっている DB でも害が無い）。この出口の DB（役割の前に落ちた DB）は移行していないので、書くと移行を飛ばして表の無い DB でサーバを起こす（finish の scoped re-review N2）。
- 失わない側: 消すのは、このインストーラ自身が `initdb` を始め、役割をまだ作っていない DB だけ（役割が無ければサーバは繋げず、記録は入らない）。それ以外の DB は消さない（DB のサービスを登録した後に落ちたものも、消さずに `repair` を示す）。費用は印のファイル 1 つと、案内の 1 文
- 反転条件: 印が残ったまま役割ができている形（役割の SQL が通った直後・完了記録の前に落ちた）で、作り直しが本人に困ると分かったら、印があっても止まって `repair` を示す形にする

### D33（仮）. 同梱の PostgreSQL が要る VC の実行時の DLL（`vcruntime140.dll`・`msvcp140.dll`）を `pgsql\bin` に同梱する

EDB の Windows 版（MSVC の組み立て）の `postgres.exe`・`initdb.exe`・`pg_ctl.exe`・`psql.exe`・`pg_dump.exe`・`pg_isready.exe`・`libpq.dll`・`libssl`・`libcrypto`・`libxml2`・`liblz4`・ICU は `VCRUNTIME140.dll` を、ICU（`icuuc67.dll`）は `MSVCP140.dll` も読む（finish で zip を落として `x86_64-w64-mingw32-objdump -p` で確かめた。zip の `pgsql\bin` には無く、pgAdmin の python の下にだけある）。
まっさらな Windows には Visual C++ の再頒布パッケージが入っていないことがあり、そのときは `initdb` から起きない（review/code.md R28。Windows の CI の runner には Visual Studio が入っているので、CI では捕まらない）。
`tools/release/build.sh` は Windows の runner の Visual Studio の再頒布の置き場（`vswhere` で引く `VC\Redist\MSVC\<版>\x64\Microsoft.VC14x.CRT`）から 2 つを `pgsql\bin` に写し（`ASHIATO_VC_CRT_DIR` で置き場を渡せる）、`THIRD-PARTY-NOTICES.txt` に節を足す。`check-artifacts.sh` は 2 つと節が無い zip を落とす。
根の Rust の 2 つ（`ashiato-setup.exe`・`ashiato-server.exe`）も、Windows の runner（`x86_64-pc-windows-msvc`）で組むと既定では `VCRUNTIME140.dll` を読む（DLL は `pgsql\bin` にだけあるので根の 2 つからは見えない。finish の scoped re-review N1）。`.cargo/config.toml` で msvc に `+crt-static` を付けて実行時を静的に持たせ、`check-artifacts.sh` は根の 2 つが VC の実行時の DLL を読む zip を落とす。
- 失わない側: 配る zip に DLL を 2 つ足すだけで、インストーラの振る舞いも要件も変えない（D12 のネットワークに出ない形のまま）。費用は約 0.7 MB
- 反転条件: Visual Studio の Distributable Code の条項で app-local の同梱が許されない・runner の置き場に無い、と分かったら、`vc_redist.x64.exe` を zip に入れてインストーラが黙って入れる形か、インストーラが最初に DLL の有無を見て入れ方を示して止まる形にする

### D34（仮）. プログラムの置き場に別の組み立てのファイルがあれば、何も替えずに止まる

D29（仮）の「既にあるものは写さない」を次のように補う（review/code.md R38・R50）。写すのは無いファイルだけで、`<名前>.copying` に写してから名前を替える（途中で切れても、半端なファイルを本来の名前で残さない）。
既にあるファイルは中身を zip のものと比べ、違えば「プログラムの置き場に、この zip と違う … があります（別の版が入っています）。版を替える口はまだ無いので、何も替えずに止めます」で止まる（「アプリと機能」の版番号も書かない）。インストールした置き場の `ashiato-setup.exe` から走らせ直すときは比べない（同じ置き場）。
- 失わない側: 止まるだけで、記録・設定・動いている版を替えない。古い実行ファイルのまま新しい版の番号を書くこと（R50）を防ぐ。費用は走らせ直しで配る zip の中身（約 180 MB）を 1 回読むこと
- 反転条件: 版を替える口（ST57・ST58 の更新）を作るとき、その口が写し替えとサービスの止め・移行を受け持つ形にし、この止まり方をその口への案内に替える

### D35. サーバのサービスが DB のサービスを見張り、止まっていれば起こし直す（2026-11-06 本人の決定）

spec の `DB のサービスが異常に終わると起こし直される`（WHEN は PostgreSQL の主のプロセスを外から終わらせる）は、D2・D11 だけでは満たせない。
同梱の `pg_ctl runservice` は主が落ちると SCM へ **正常な停止**（`WIN32_EXIT_CODE 0`）を返し、SCM の回復（`failure`・`failureflag 1`）は非 0 の終わりでしか動かないので、DB は止まったまま残る
（実測 run 42195954119 の診断: `STATE 1 STOPPED`・`WIN32_EXIT_CODE 0`・SCM のイベントは 7036 の停止だけ）。本人は 2026-11-06 に「仕様はそのまま、作りを直す」を選んだ。

- インストーラは `ashiato-db` のサービスの DACL に、`NT SERVICE\ashiato-server` の SID へ **状態を読む・開始する**（SDDL の `LC`・`RP`）だけの ACE を足す（`sc.exe sdshow` に足して `sc.exe sdset`）。止める・設定を変える権限は渡さない
- サーバの `service` の口は、5 秒ごとに `ashiato-db` の状態を読み、**サーバが動いている間に DB が `STOPPED`** なら `StartService` で起こし、理由（`kind=db_service_restarted`）をサーバのログに書く。
  DB を人が止めるには先に依存するサーバを止める（`net stop` は依存を先に止め、`sc stop` は依存が動いていれば拒む）ので、サーバが動いていて DB が止まっているのは意図しない停止とみなせる。インストーラの `repair`・`uninstall` は先にサーバを止める
- 起こすか否かの判断（サーバの状態・DB の状態 → 何もしない / 起こす）は純粋な関数にして Linux で試す。Windows の呼び出し（`OpenService`・`QueryServiceStatus`・`StartService`）は `crates/winsvc` に閉じる
- 失わない側: SCM の回復（D11）はそのまま残す（pg_ctl 自身が異常に終わった場合はこれが起こす）。見張りは足すだけ。費用は 5 秒ごとの状態の問い合わせ
- 反転条件: 同梱の PostgreSQL のサービスの入口を自前の口に替え、主の異常な終わりを非 0 で SCM へ返せるようにしたら、見張りを外して SCM の回復だけにする

## Risks / Trade-offs

- [仮想アカウントで `pg_ctl register` が通り、その SID で置き場に書けること（P19）は走らせていない（確かさ 1）。下流の手元では管理者の Windows が無く、PR の CI はレビューの後でしか走らない（`.githooks/pre-push`）ので、外れたことは全部のタスクの後に分かりうる] → tasks 3.0 で、下流の最初に、WSL から本人の PC の Windows サンドボックスを起こし（`.wsb` の `LogonCommand`。PC の収集アプリの録画と同じ置き場）、P19 の確かめだけを機械で走らせる（`tools/release/sandbox-probe.sh p19`。結果は写した置き場のファイル）。サンドボックスの利用者が上がった札を持たず確かめられなければ、そう書いて PR の CI に任せる。外れたら D3 の反転条件（専用のローカルアカウント）。共有のアカウントへは倒さない
- [`windows-service` の `define_windows_service!` の展開が利用側に `unsafe` を出す（D2）] → 出すなら `crates/winsvc`（作業場の lint を継がない小さな crate）にサービスの入口だけを閉じ込める（tasks 2.1 で決める）
- [管理者は所有権を取れば状態の置き場を読める] → 残るリスク（FR-109 の範囲の外）。GO 資料と `docs/install.md` に書く
- [ログオンしていない間に携帯から届くかは Tailscale の設定に依る（P18。確かさ 1）] → インストーラは触らず、最後に示す（Q6）。CI では見ない。確認バッチの人間の確認で見る
- [Windows サンドボックスの中で UAC が出るか・再起動の後に状態が残るか（P16。確かさ 1）] → Q5 の推奨のとおり、出ない・残らないなら、録画で見るのは合言葉を決めてログインするまで・再起動は CI の「ログオンなしで起きる」まで
- [配布物が約 180 MB 大きくなる（P5）] → 要るものだけを切り出す（D19）。GitHub のリリースの 1 ファイルの上限（2 GB）の内
- [DB があるのに秘密が無い状態で止まる（D10）] → 出口は `repair`（D27）。止まってから本人が `repair` を走らせるまでの間は、携帯の未送信が上限（90 日・2 GB。FR-9）に向かって溜まる。止まったことと `repair` を走らせ直しの出力で示す
- [`repair` の一時の形（`pg_ctl` に一時の `hba_file` と `sspi`）が上がった札で通るかは走らせていない（確かさ 1）] → tasks 6.2 の Windows の CI で見る。外れたら D28 の反転条件
- [ST44 の `web_session.rs` の上に積む] → requires に ST44 を入れた。下流の入口が ST44 の gate 済みの SHA を取り込む（`deps.json`）。ST44 の spec が変わったら merge gate が「依存の spec:」で落とす

## Migration Plan

- 開発の形（環境変数で起こすサーバ・Docker の DB）は変えない。`tools/db-roles.sh` は SQL を `tools/db-roles.sql` に出すだけで、流す SQL は同じ
- 本人の環境の切り替えは ST55 の確認の後、`docs/release.md` §7 の新しい手順で本人が行う（D18（仮））。Docker の DB の記録は移さない（D15）
- 戻すとき: `ashiato-setup.exe uninstall` で消す。状態の置き場は残るので、入れ直せば続く

## Open Questions

（無し。Q1〜Q7 は推奨で書き、第 1 回の答えで確かめた。Q8・Q9 は第 2 回の答えを D10・D27・D3・D17 に、Q10 は第 3 回の答えを D1・D28 に、Q11 は第 4 回の答えを D4・D27 に写した）
