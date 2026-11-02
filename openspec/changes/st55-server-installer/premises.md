# ST55 の前提（外の形式・振る舞い）

問いを立てる前に確かめたもの。確かさは story-upstream の 1〜5（1 言っただけ / 2 出所を示した / 3 辿った / 4 走らせた / 5 実物で再現した）。

失われるもの（`loss`）のある問いのうち、Q2（DB の本体の置き場）は P3 に依る。P3 は WSL の中の docker では 4（落ちる）まで確かめたが、
**配る形の Docker Desktop では確かめていない**（本人の PC に入っているが止まっていて、起こすと WSL の中の docker と並走している開発の DB に
障るおそれがあるので、このセッションでは起こさなかった）。そのため Q1 で本人に実物の結果（形だけ）を求める。Q3（データのフォルダ）は P8・P16 に依る（どちらも 5。本人の PC で deep-review が読んだ）。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | サーバ（`ashiato-server`）は Windows 向けに組み立てられ、起動の入口まで動く | FR-108 の `[NEEDS CLARIFICATION: S-01 を Windows 向けに組み立てられるかは試していない]` | ST52 の premises P1（2026-10-31、Windows 側の cargo で `cargo build --release -p ashiato-server`） | rc=0。`ashiato-server.exe` が起動して引数の検査まで動く。**DB に繋いで待ち受けるところは試していない** | 実物（ST52） | 4（DB に繋ぐところは未走行。tasks で 4 に上げる） |
| P2 | サーバの待ち受けは既定で `127.0.0.1:18787`、DB の宛先が loopback でなければ起動しない。コンテナに入れると守りが外れる | FR-108 の「サーバは Windows で直に動かす」 | `crates/server/src/net_guard.rs:31`（`DEFAULT_BIND`）・`crates/server/src/lib.rs:4388-4396`（`check_bind`・`db_host_is_local`） | そのとおり | 実物（repo） | 3 |
| P3 | Docker の中の PostgreSQL が、データの置き場（`PGDATA`）を Windows のフォルダ（bind mount）に置いて初期化でき、fsync が効く | Q2（DB の本体の置き場）・FR-109 の `[NEEDS CLARIFICATION: Docker Desktop で PostgreSQL のデータを Windows のフォルダに置いて初期化・fsync が通るかは試していない]` | (a) 2026-11-02、WSL の中の docker（29.7.2）で `/mnt/c/Users/Public/…`（9p・drvfs）を `postgres:17-alpine` の `PGDATA` に bind mount して起動。(b) Docker Desktop では `premise-pgdata.ps1`（この change に置いた。結果の形だけを出す。初期化・2,000 回の書き込み・fsync の速さ・再起動の後と新しいコンテナへの付け直しの後の行の数）を本人が走らせる（Q1） | (a) **初期化で落ちる**: `initdb: error: could not change permissions of directory "/var/lib/postgresql/data": Operation not permitted`。(b) 未走行。**(b) を走らせても、停電・強制終了のときの耐久性（fsync が Windows のディスクまで届くか）は確かめられない**（2 のまま） | 実物（WSL の docker）／Docker Desktop は未確認 | (a) 4・(b) 1（初期化と付け直しは Q1 で 5 に、耐久性は 1） |
| P4 | Docker Desktop は公式の配布元（`https://desktop.docker.com/win/main/amd64/Docker%20Desktop%20Installer.exe`）から入れられ、コマンドで `install [--user] --accept-license --backend=wsl-2 --quiet` を渡せる。全利用者向けの導入は管理者の権限が要り、利用者ごとの導入（`--user`）は要らない。WSL 2.1.5 以上と BIOS/UEFI の仮想化が要る | FR-108・Q4（インストーラの形）・Q7・Q8 | https://docs.docker.com/desktop/setup/install/windows-install/ （2026-11-02 に読んだ） | 文書どおり（未走行） | 公式文書 | 2 |
| P5 | `--accept-license` は Docker Subscription Service Agreement を、初回起動の時ではなく導入の時に受け入れる。Docker Desktop が無料なのは個人の利用・教育・非商用のオープンソース・小さな事業者（従業員 250 人未満かつ年商 1,000 万ドル未満）で、それ以外の業務の利用は有料の契約が要る | C3（規約を本人に見せて同意を取る）・「いずれ渡す相手」 | https://docs.docker.com/subscription/desktop-license/ （2026-11-02 に読んだ） | 文書どおり | 公式文書 | 2 |
| P6 | サーバの `migrate` は、当てる前に `pg_dump` を外のコマンドとして起こし、`--file <置き場>` に退避を書く。退避の置き場の既定は作業ディレクトリからの相対 `backups/pre-migrate` | Q10（Windows で `pg_dump` をどこから得るか）・FR-109 の「移行の前の退避」 | `crates/server/src/migrate_backup.rs:27-33`（`MIGRATE_BACKUP_DIR`・`PG_DUMP`）・`:101-111`（`Command::new`・`--file`） | そのとおり。Windows には `pg_dump` が無いので、何も渡さなければ最初の `migrate` が退避に失敗して当てずに止まる | 実物（repo） | 3 |
| P7 | 原本（写真）・書き出し・書庫の写しの置き場は、環境変数が無ければ `%LOCALAPPDATA%\ashiato\{media,export,archive-copies}`。取り込み待ちは `%USERPROFILE%\Documents\ashiato\取り込み待ち` | Q3（データのフォルダ） | `crates/server/src/media.rs:54`・`crates/server/src/export.rs:142`・`crates/server/src/archive/config.rs:58-68` | そのとおり | 実物（repo） | 3 |
| P8 | OneDrive の「PC フォルダーのバックアップ」を入れると「ドキュメント」「デスクトップ」がクラウドへ同期される場所に移る。`%LOCALAPPDATA%` は移らない | Q3・Q12（同期されるフォルダに DB・原本・書庫を置くと外へ出る） | deep-review が本人の PC で読んだ（2026-11-02）: `[Environment]::GetFolderPath("MyDocuments")` = `C:\Users\<user>\OneDrive\ドキュメント`、Desktop も `OneDrive\Desktop`、`User Shell Folders` の `Local AppData` = `C:\Users\<user>\AppData\Local` | そのとおり。コードの取り込み待ちの字面 `%USERPROFILE%\Documents\…` は、見えている「ドキュメント」と別の場所を指す | 実物（本人の PC） | 5 |
| P9 | サーバに外部へ送る部品（HTTP の送り手）を持たせない、を部品を持つかどうかで機械が止めている | Q4（実行環境を取りに行く部品をサーバの実行ファイルに入れられない） | `docs/requirements.md` FR-112 の ★（`openspec/specs/data-sensitivity/spec.md`・`tools/check-offsite.sh`） | そのとおり。Docker Desktop を落としてくる部品は、サーバとは別の実行ファイル（か台本）に置く | 実物（repo の文書） | 2 |
| P10 | 画面の合言葉は今、環境変数 `WEB_PASSWORD` で平文で渡し、16 文字以上・`change-me` で始まらない・API の合言葉と違う、を起動の時に確かめる。ログインの世代の印は `API_TOKEN` を鍵にした HMAC（合言葉が変わると全部のログインが切れる） | Q5・Q6（画面の合言葉を決める口と持ち方） | `crates/server/src/lib.rs:4370-4381`・`crates/server/src/web_session.rs:32-105` | そのとおり | 実物（repo） | 3 |
| P11 | 本人の PC には Docker Desktop が入っている（止まっている）。設定は `AutoStart: False`・`IntegratedWslDistros: ['Ubuntu']`（Ubuntu と連携する）。WSL の Ubuntu の中では別の docker（Ubuntu 26.04 の dockerd）が開発と確認の DB を動かしている（`ashiato2-st52-db-1` など） | Q1（起こす前の準備）・Q7（既に入っている PC での振る舞い）・Q9（確かめる場所） | `docker context ls`（`desktop-linux` の文脈あり）・`docker.exe info`（`dockerDesktopLinuxEngine` に繋がらない）・`docker ps`・`%APPDATA%\Docker\settings-store.json`（deep-review が読んだ） | そのとおり | 実物（本人の PC） | 5 |
| P12 | GitHub の Windows の実行環境（`windows-latest`）では Docker Desktop を動かせない（入れ子の仮想化が無く、Linux のコンテナを動かせない） | Q9（確かめる場所） | 知識（GitHub の runner の文書は Windows コンテナだけを挙げる） | — | 知識 | 1 |
| P13 | タスク スケジューラの既定は `ExecutionTimeLimit=PT72H`・`DisallowStartIfOnBatteries=True`・`StopIfGoingOnBatteries=True`・`RestartCount=0` | Q11（明示して外さないと、72 時間で止まり、電池で止まり、落ちても起こし直さない） | deep-review が本人の PC の PowerShell 5.1 で `New-ScheduledTaskSettingsSet` の既定を読んだ（2026-11-02） | そのとおり | 実物（本人の PC） | 4 |
| P14 | サーバは設定を環境変数からしか読まない（設定のファイルを読む口が無い） | Q6・Q11・C15（起こす係が設定のファイルを読んで環境変数に詰める） | `crates/server/src/lib.rs:4365-4388`（`std::env::var`）。`grep -rn "dotenv\|\.toml\"\|settings.json" crates/server/src` は 0 件 | そのとおり | 実物（repo） | 3 |
| P15 | 本人の PC は Windows 11 Pro。`HypervisorPresent=True`・`VirtualizationFirmwareEnabled=True`、`.wslconfig` に `nestedVirtualization=true`・`memory=32GB`。WSL の網は nat で、WSL の中の待ち受けは Windows の 127.0.0.1 から届く（55499 で `Test-NetConnection` が True） | Q9（Hyper-V の仮想マシンで確かめられる）・Q13（メモリの上限）・C6（ポートが被る） | deep-review が本人の PC で読んだ（`Get-CimInstance Win32_OperatingSystem`・`.wslconfig`・`wslinfo --networking-mode`） | そのとおり | 実物（本人の PC） | 5 |
| P16 | `D:\` は `Authenticated Users` が Modify、`BUILTIN\Users` が ReadAndExecute で、どちらも下へ継承される。サーバは Windows ではフォルダの権限を付けない（`#[cfg(unix)]` だけ） | Q3・C12（プロファイルの外を選ぶと他のアカウントから読める） | `Get-Acl D:\`（deep-review、本人の PC）・`crates/server/src/migrate_backup.rs:138-141`・`media.rs:41-43`・`export.rs:1188` | そのとおり | 実物（本人の PC・repo） | 5 |
## 触らなかったもの

- Docker Desktop を本人の PC で起こすこと（P3 の (b)）。起こすと WSL の連携の設定次第で、WSL の中の docker と取り合う
- Windows の「この PC を初期状態に戻す」でどのフォルダが残るか（データのフォルダの寿命）。この Story の範囲の外

## 聞かない既定（C。扉を開けたままにする側。spec の段で design の D 番号に写す）

| # | 既定 | なぜ聞かないか |
|---|---|---|
| C1 | 既にある状態（データのフォルダ・合言葉・設定）は、インストールを何度走らせても上書きしない。DB の合言葉と API の合言葉は、無いときだけ作る。片方だけあるとき: (a) DB の本体はあり合言葉が無い → 合言葉を作り直し、コンテナの中の psql（合言葉なしで入れる）で役割の合言葉をそろえる（`tools/db-roles.sh` と同じ）。API の合言葉を作り直したら、ログインが全部切れ、収集アプリの資格情報もやり直しになることを示す。(b) 合言葉はあり DB の本体が無い → Q2 の答えに従う（写しから戻すか、空で作ることを示して本人に確かめる）。(c) 前と別のフォルダを選んで走らせ直した → 前のフォルダの場所を示して止まる | 上書きすると DB に繋げなくなり、収集アプリの資格情報も切れる。上書きしない・止まる側は失わない（review/deep.md R10） |
| C2 | DB の合言葉（管理者・所有者 `ashiato_owner`・実行時 `ashiato_app` の 3 つ）と API の合言葉は、OS の暗号の乱数で 32 バイトから作り、画面にも記録にも出さない | FR-110。固定値を持たない（ST28 / D19） |
| C3 | Docker Desktop の規約（P5）は本人に示して同意を取ってから入れる（黙って `--accept-license` を渡さない） | 同意しない側に倒す。費用は 1 回の確認 |
| C4 | BIOS/UEFI の仮想化が無効なら、何も入れずに有効にする手順を示して止まる | FR-108 の逐語 |
| C5 | 消す操作（アンインストール）を作るなら、データのフォルダは消さない。消し方を示すだけ | 捨てない側。データを消す口は別の要件（FR の削除）が持つ |
| C6 | 入れた DB は、開発の `docker-compose.yml` と別の compose の定義・別の project 名で持つ。開発の compose と worktree ごとの DB には触らない。インストールの DB とサーバのポートは、`tools/ports.sh` のどの範囲（main 55432・18787、確認バッチ 55499、Story 55500+NN・19000+2×NN、ほか 55600+n・19200+2×n）とも被らない固定の番号にする（例: DB 55400・サーバ 18700）。待ち受けが塞がっていたら理由を示して止まる。スマホから見る `tailscale serve` の先をどちらに向けるかは C11 の範囲の外 | 「壊してはいけないもの」。WSL の中の待ち受けは Windows の 127.0.0.1 から届く（P15）。被らない割り当てで防ぐ（review/deep.md R12） |
| C7 | DB の像は PostgreSQL の主版（17）と系統（alpine。開発と同じ）で固定し、`POSTGRES_INITDB_ARGS=--data-checksums` で作る。主版・系統を変えるのは別の作業（データの移し替えが要る） | 主版が勝手に上がると古いデータの置き場を読めなくなり、系統を替えると文字の索引の並びが変わる。チェックサムは Docker の層の耐久性が未確認な間、壊れたことに気づく手がかりになる（review/deep.md R14） |
| C8 | ログオン時の起動では移行を当てない（DB を起こしてサーバを起こすだけ）。移行を当てるのは、インストールと更新（ST58）の時だけ | 起動のたびに退避が積み上がらない。版の食い違いはサーバが起動の時に止める（ST48） |
| C9 | 移行の前の退避の置き場（`MIGRATE_BACKUP_DIR`）は、データのフォルダの中の絶対の場所を渡す。退避の保持の数と、退避に残る消した記録の扱いは、この Story では決めず ST58（更新）の深掘りへ回す | P6。既定の相対の場所は作業ディレクトリ次第で、更新の最中に消えうる（FR-109 ★ C9）。退避は更新のたびに 1 つ増え、消す処理が無い（review/deep.md R15） |
| C10 | 既存の記録の引き継ぎ（WSL の開発の DB からの移し替え）は扱わない | 携帯は ST52 の Q1 で「まだ入っていない・記録を集めていない」と答えている。PC の収集アプリの自動起動も無く、`%LOCALAPPDATA%\ashiato` も無い（deep-review）。main の開発の DB（`ashiato2-db-1`、止まっている）に残したい記録があるかは、Q1 の補足で本人に確かめる（review/deep.md R16） |
| C11 | Tailscale での外からの経路（ST28）はインストーラが設定しない。収集アプリの繋ぎ先と資格情報を渡すのは ST56 | 範囲の外。待ち受けは loopback のまま |
| C12 | インストーラは、データのフォルダ（と取り込み待ち）を作るとき、継承を切って本人と SYSTEM だけが読み書きできるようにする。`%LOCALAPPDATA%` の中でも同じにする | P16。プロファイルの外は他のアカウントから読める。失わない側で、費用は権限を付ける処理 1 つ（review/deep.md R5） |
| C13 | 昇格（管理者の権限）は Docker Desktop と WSL を入れる子のプロセスだけ。データのフォルダ・設定・ログオン時の起動は、昇格しない本人の側で作る | 昇格した側の `%LOCALAPPDATA%`・タスク スケジューラは昇格したアカウントのもので、別の管理者の資格情報で昇格すると本人でないプロファイルに置かれる（review/deep.md R9） |
| C14 | ウイルス対策のリアルタイムの走査からの除外は、インストーラが入れず、入れ方を示すだけにする。起こす係は、DB の起動に失敗したら（Docker Desktop の更新でファイル共有の仕組みが替わった場合を含む）理由の種別を示して止まり、サーバを起こさない | 除外を入れるのは本人のセキュリティの判断。止まる側は書き換えない（review/deep.md R11） |
| C15 | サーバは設定のファイルを読まない（いまのまま環境変数だけ）。ログオン時に起こす係（`ashiato-setup` の起動の口）が設定のファイルを読み、環境変数に詰めて DB のコンテナとサーバを起こす | P14。サーバに読み口を足さない側が、サーバの起動の検査（ST28・ST48）をそのまま使える（review/deep.md R13） |
