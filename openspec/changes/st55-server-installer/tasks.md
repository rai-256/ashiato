# ST55 実装タスク — サーバを入れて、そのまま使い始められる

読む順: `deep.md`（**本人の答え Q1〜Q7**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `specs/data-sensitivity/spec.md` → `specs/data-durability/spec.md` → `specs/device-collection/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST52（配布物の形・版の出所・`tools/release/`）と ST28（DB の役割・画面のログイン）は archive 済みで main にある。ST44（ログインの印の鍵 `WEB_SESSION_KEY`）は下流が gate を通った SHA を、下流の入口が取り込む
（`python3 scripts/story_deps.py included ST55 ST44` rc=0 を確かめてから始める）。`data-sensitivity` の MODIFIED は ST44 の delta の Requirement を書き換える。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
件数つきの検証は、試験を走らせた後に `-- --list` の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

**Windows で管理者の権限が要る確かめ（サービスの登録・ACL・UAC の外の通し）は、下流の手元では走らない。** それは Windows の CI の job `install-windows`（D24）が `tools/release/install-check.sh` で毎回見る。
下流の手元の検証は、(a) 命令を組む純粋な関数の Linux の `cargo test`、(b) `x86_64-pc-windows-gnu` への `cargo clippy`、(c) `install-check.sh --self-test`（偽の `sc.exe`・`icacls`・`psql` を `PATH` の先に置き、確かめの論理だけを試す）の 3 つで閉じる。
本物の Windows の通しは、PR の CI（merge gate が CI の緑を見る）と、末尾の「人間の確認待ち」で見る。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: …`）、bash は `echo "== Scenario: …"`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario**: `distribution` の ADDED 80 本・`data-sensitivity` の MODIFIED で足した 4 本・`data-durability` の MODIFIED で足した 2 本・`device-collection` の MODIFIED で足した 1 本、計 **87 本に ST55 が印を置く**。全部このファイルのどこかの Task が名指しする。
  MODIFIED で本文をそのまま写した Scenario（`data-sensitivity` の 17 本は ST44。うち WHEN を形で絞った「画面の合言葉が API の合言葉と同じだと起動しない」は Task 7.1 で照らす形の試験にも印を置く、`data-durability` の書き出しの置き場の Scenario は ST29 の系列、`device-collection` の原本の置き場の Scenario は ST09）の印は、それぞれの Story が持つ。**消さない**
- **本人の決定（下流は変えない）**: Q1 入れるのはサーバとデータベースだけ（PC の収集アプリは別の zip）/ Q2 インストーラは `ashiato-setup.exe`（Rust）をサーバの zip に入れる / Q3 画面の合言葉はインストーラの端末で 2 回打たせて決め、忘れたら `ashiato-setup.exe password` /
  Q4 消す口を作り、記録の置き場は残す / Q5 CI に加えて確認バッチで Windows サンドボックスの中を録画 / Q6 Tailscale に触らず案内する / Q7 置き場は `C:\ProgramData\ashiato` に固定し、空きが 40 GB より少なければ知らせる /
  Q8 DB はあるのに秘密が無くて止まったときの出口 `ashiato-setup.exe repair` をこの Story で作る（D10・D27） / Q9 `logs\db` に DB のサービスの書きを足し、DB の起動の最初の数行と `pg_ctl` の知らせがイベントログに出るのは受け入れる（送った文の字面と秘密は出さない。D3・D17） / Q10 DB の管理者の役割の名前は DB を作るときに `ashiato` に固定する（`initdb -U ashiato`。設定にも別の置き場にも控えない。D1・D28）。
  仮: D3（置き場の ACL と仮想アカウント。登録できなければ止まる）・D26（再起動はサンドボックスで残るときだけ実物で見る）・D16（取り込みの利用者は nil）・D18（本人の切り替えで `API_TOKEN` を引き継がない）。
  （第 1 回は推奨で先に書き、本人の答えが推奨と違えば上流が写してから下流へ渡す）
- **名前は統一する**（D14）: サービス `ashiato-db`・`ashiato-server`（開発版は `-dev`）、アカウント `NT SERVICE\ashiato-db`・`NT SERVICE\ashiato-server`、プログラムの置き場 `C:\Program Files\ashiato`、状態の置き場 `C:\ProgramData\ashiato`（`db\17`・`media`・`export`・`backups\pre-migrate`・`archive-copies`・`logs\db`・`logs\server`・`settings`）、
  設定のファイル `settings\server.json`、取り込み待ち `%USERPROFILE%\ashiato\取り込み待ち`、番号 DB 55400・サーバ 18700、設定の鍵 `WEB_PASSWORD_VERIFIER`、インストーラの口 `install`（既定）・`password`・`repair`・`uninstall`、試験の差し込み `ASHIATO_SETUP_TEST_<名前>`（インストーラ）・`ASHIATO_SERVICE_TEST_<名前>`（サーバの `service` の口）と命令の記録 `ASHIATO_SETUP_TEST_TRACE=1`（どれも試験の組み立てだけ。D24）、設定の本人 `ASHIATO_OWNER_SID`、
  crate `crates/installer`（`[package] name = "ashiato-setup"`・bin `ashiato-setup`）、道具 `tools/release/pgsql.sh`・`tools/release/pgsql.lock`・`tools/release/install-check.sh`・`tools/release/sandbox-probe.sh`・`tools/db-roles.sql`・`tools/check-pg-major.sh`、CI の job `install-windows`
- **秘密（DB の合言葉・API の合言葉・ログインの印の鍵・画面の合言葉とその照らすための値）を、端末の出力・ログ・試験の期待値の出力・サービスの登録簿・命令の引数に出さない**（値は項目の名前だけで示す）
- **開発の形を変えない**（壊してはいけないもの）: `docker-compose.yml` の DB・`tools/ports.sh` の割り当て・`.env` と環境変数で起こすサーバ・`net_guard.rs` の loopback の守り・ST28 の画面のログイン・ST44 の `v1` のログインの印。`tools/db-roles.sh` は SQL を `tools/db-roles.sql` に出すだけで流す SQL は同じ
- **新しい Rust の依存は `windows-service`・`argon2`・`rpassword` の 3 つだけ**（D2・D8・D23）。`windows-service` は `cfg(windows)` だけ。`tools/check-licenses.sh` に通す。網の依存（`ureq` など）をインストーラに足さない（D12）
- **`unsafe` は書かない**（作業場の `unsafe_code = "forbid"`）。`windows-service` の展開が `unsafe` を出すなら、`crates/winsvc` に閉じ込める（D2。Task 2.1 で決める）
- **試験の差し込みと命令の記録（`ASHIATO_SETUP_TEST_*`・`ASHIATO_SERVICE_TEST_*`）は試験の組み立て（`package.sh windows --test-build`）でだけ読み、配る組み立て（`ASHIATO_RELEASE=1`）には入れない**（D24）。差し込みか記録に依る Scenario は試験の zip で、それ以外は配る zip で走らせる
- **長いコマンドは `scripts/quiet-run`**（cargo test）。試験の名前の接頭辞は `st55_`。サーバの DB の試験は `crates/server/src/st55_tests.rs`（`lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;`）、インストーラの試験は `crates/installer/src/` の `#[cfg(test)]`
- 縦串・書き換え禁止・偽データの節は Story ごとの新しいファイル（`tools/smoke.d/` など）に置き、共有の台本の本体に書き足さない

## Task 1: 同梱する PostgreSQL を版と指紋で固定して取り、要るものを切り出す

- [x] 1.1 `tools/release/pgsql.lock`（D19）を作る: `version=17.6-1`・`url=`・`sha256=`（EDB の zip を 1 回落として `sha256sum` で控える）・残す実行ファイルの一覧。中の許諾の文の場所と、残す実行ファイルが読む DLL の一覧（`x86_64-w64-mingw32-objdump -p <exe> | grep "DLL Name"` を推移で辿る）を `pgsql.lock` に書く。
  `premises.md` に P20（許諾の文の実在と DLL の一覧。確かさ 5）を足す
  検証: `bash -c 'grep -qE "^sha256=[0-9a-f]{64}$" tools/release/pgsql.lock && grep -q "^version=17\." tools/release/pgsql.lock'` rc=0
- [x] 1.2 `tools/release/pgsql.sh`（D19）: `fetch <置き場>`（指紋を比べ、違えば rc≠0 で「指紋が違う」）・`slim <展開した置き場> <出力>`（`bin` の残す実行ファイルと DLL・`lib`・`share`・許諾の文だけを写す）・`--self-test`。
  `--self-test` は小さな偽の zip（`bin/postgres.exe` などの空のファイル・`pgAdmin 4/`・`StackBuilder/`・`include/`・`doc/` を持つ）で `同梱する PostgreSQL の指紋が違えば組まない`（指紋を差し替えた lock で `fetch` が rc≠0 と「指紋が違う」）と、`slim` の出力に `pgAdmin 4`・`StackBuilder`・`include`・`doc` が無いことを見る
  検証: `tools/release/pgsql.sh --self-test` rc=0
## Task 2: サーバの `service` の口（設定のファイル・サービスの制御・DB を待つ・ログのファイル）

- [x] 2.1 `windows-service` を `cfg(windows)` の依存に足し、`x86_64-pc-windows-gnu` で `define_windows_service!` を使う小さな版を組んで、作業場の `unsafe_code = "forbid"` に当たるかを見る。当たれば `crates/winsvc`（作業場の lint を継がず、`#![deny(unsafe_op_in_unsafe_fn)]` とサービスの入口だけを持つ）に分ける。どちらにしたかを `design.md` の D2 の下に 1 行で書く
  検証: `cargo clippy -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `tools/check-licenses.sh` rc=0
- [x] 2.2 `crates/server/src/service.rs`（D4・D11）: `--settings <場所>` の JSON を読んで、tokio を起こす前に鍵を自分のプロセスの環境変数に置き、DB へ 1 秒ごとに最長 120 秒繋ぎ直してから今の起動の検査に入る。設定が無い・読めない・形が違うときは理由（`settings_missing`・`settings_unreadable`・`settings_malformed`）をログに書いて終わる。
  `main.rs` に `service` の口を足す（Linux では「Windows でだけ使える」で rc≠0）。読み方と待ち方は純粋な関数に分け、Linux で試す（`crates/server/src/st55_tests.rs`）:
  設定の読み方（無い・読めない・形が違う の 3 つと、出力に秘密の値が無いこと）・DB の待ち方（DB の口を 20 秒後に開く偽の接続で待つ）の関数の段の試験（印は置かず、下支え）・
  `// Scenario: 開発の台本で起こすサーバは、今までどおり環境変数を読む`（`crates/server/tests/server_startup.rs` の `st55_bin_env_unchanged`。`.env` で起こして `/healthz` 200）。関数の段の試験には 120 秒の上限で理由を出して終わることも置く。
  サービスの段の確かめは Task 3.2 の `install-check.sh` が持つ: `設定のファイルが無いと、理由を書いて起動しない`（配る zip。設定を退けて開始）・`DB の起動が遅れても、サーバのサービスは待ってから起きる`・`DB が 120 秒開かなければ、理由を書いて終わる`（試験の zip。`ASHIATO_SERVICE_TEST_DB_DELAY_SEC=20`・`=never`）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st55_service_ && test "$(cargo test -p ashiato-server st55_service_ -- --list 2>/dev/null | grep -c ": test$")" -ge 6'` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server --test server_startup st55_bin_ && test "$(cargo test -p ashiato-server --test server_startup st55_bin_ -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0
- [x] 2.3 停止の知らせ（D2）: router の graceful shutdown・取り込み器と書き出しへの打ち切り・書き途中の `.` で始まるファイルの片付け。試験は停止の知らせを送る口を Linux でも呼べる形にして `// Scenario: サービスの停止を受けると、書き途中の書き出しを残さずに終わる`（`ASHIATO_EXPORT_SELFTEST_DELAY_MS` で書き出しを遅らせ、途中で止め、置き場に `.` で始まるファイルが無く、次の起動で書き出しの状態が走っていない）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st55_stop_ && test "$(cargo test -p ashiato-server st55_stop_ -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0
- [x] 2.4 ログの書き手（D17）: `service` の口では `logs\server\server-<日付>.log` に書き、日付が変わったら開き直し、起動のときに 30 日より古いファイルを消す。起動の拒否（`eprintln!` の所）も同じ書き手に書く。新しい依存を足さない。
  試験（Linux。置き場は一時フォルダ）: 日付の切り替え・30 日の片付け（31 日前のファイルが消え 29 日前が残る。`// Scenario: ログは DB が 7 日分、サーバが 30 日分残る` の印を置き、DB の 7 日は Task 4.1 の DB の設定の試験と `install-check.sh` が見る）・起動の拒否の理由が書かれる
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-server st55_log_ && test "$(cargo test -p ashiato-server st55_log_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0 / `tools/check-log-private.sh` rc=0

## Task 3: インストーラの骨（crate・置き場の表・上がり方・ACL・サービスの登録）・Windows の CI の通し・配布物への同梱

- [x] 3.0 **P19 を先に実物で見る**（design の Risks・review/spec.md R3）: `tools/release/sandbox-probe.sh p19` は、WSL から Windows サンドボックスを `.wsb`（同梱の PostgreSQL の `pgsql` を読みだけで写し、結果の置き場を書けるように写す。`LogonCommand` で確かめの PowerShell を走らせる）で起こし、
  サンドボックスの利用者が上がった札を持つか・`pg_ctl register -U "NT SERVICE\<名前>"` が通るか・その SID に ACL を付けた置き場に `initdb` の後の `postgres` が書けるか、を結果のファイルに書いて閉じる。`premises.md` の P19（と P16 の利用者の札）を結果で書き直す。
  外れたら D3 の反転条件として上流へ返す（下流は先へ進まない）。サンドボックスが起きない・利用者が上がった札を持たないなら、その旨を P19 に書き、PR の CI（3.2 の最初の項目）に任せる
  検証: `tools/release/sandbox-probe.sh p19` rc=0（サンドボックスで確かめられないときも、その旨を結果のファイルに書いて rc=0。P19 が成り立たないと分かったときは rc=1 で、上流へ返す）
- [x] 3.1 `crates/installer`（D20・D14・D22）: 作業場の member、`build.rs` で組み立ての印、`main.rs` の先頭に理由つきの `#![allow(clippy::print_stdout)]`。`layout.rs` に D14 の表（版番号の `-dev.` で列を選ぶ）。
  上がる前に本人の SID・`USERPROFILE`・ダウンロードの実の場所を控え、`powershell Start-Process -Verb RunAs` で自分を起こし直す命令を組む（既に管理者なら起こし直さない）。ACL を付ける `icacls` の命令（D3（仮）。`db` と `logs\db` は `NT SERVICE\ashiato-db` の書き・D21）とサービスの登録の命令（`pg_ctl register`・`sc.exe config`・`sc.exe failure`。D2・D11）を組む純粋な関数を置く。
  起こした命令を記録する口（`ASHIATO_SETUP_TEST_TRACE=1`。試験の組み立てだけ）と、引数の読み方（知らない引数で何も作らずに rc≠0）もここで置く。
  試験（Linux の `cargo test -p ashiato-setup`）: 表の選び方（正式・ベータ・開発版）・起こし直しの命令に秘密が無い・`icacls` の命令に `Users`・`Authenticated Users`・`Everyone`・`Administrators` が無い・`logs\db` の書きが `NT SERVICE\ashiato-db` で `ashiato-server` でない・登録の命令が共有のアカウントを名指ししない・
  `// Scenario: 上がっていない状態から開くと、管理者に上がる起こし直しは 1 回だけ`・`// Scenario: 上がる前の段は、開いた利用者を本人として控える`（`whoami /user` の出力を渡した純粋な関数）・`// Scenario: インストーラは状態の置き場を選ぶ口を持たない`（引数の読み方の段。`install-check.sh` にも置く）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_layout_ && test "$(cargo test -p ashiato-setup st55_layout_ -- --list 2>/dev/null | grep -c ": test$")" -ge 8'` rc=0 /
  `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `cargo clippy -p ashiato-setup --all-targets -- -D warnings` rc=0
- [x] 3.2 `tools/release/install-check.sh`（D24）と `.github/workflows/ci.yml` の job `install-windows`（`windows-latest`。`changes` の出し分けに `crates/installer`・`tools/release`・`tools/db-roles.*` を入れる）: Git Bash で、zip を展開してインストーラを走らせ、spec の Scenario ごとに `echo "== Scenario: <名前>（配る zip | 試験の zip）"` と確かめを置く台本。
  始めに上がった札（`whoami /groups` の High Mandatory Level）を見て、無ければ全部の前に落ちる（P17）。次に P19（仮想アカウントの登録と書き込み）を見る。利用者の地域の設定は `Set-Culture ja-JP` にしてから入れる。ここでは
  `配る zip からのインストールで、2 つのサービスが自動で起きる形で入る`・`インストールは Docker も WSL も使わない`（試験の zip の命令の記録）・`サービスは利用者でなくサービス専用のアカウントで動く`・`DB のサービスが異常に終わると起こし直される`・`設定のファイルが無いと、理由を書いて起動しない`・`DB の起動が遅れても、サーバのサービスは待ってから起きる`・`DB が 120 秒開かなければ、理由を書いて終わる`・`サービス専用のアカウントで登録できなければ、共有のアカウントへ倒さず止まる`（`ASHIATO_SETUP_TEST_REGISTER_FAIL=1`）・
  `サービスを止めて起こし直すと、サーバは DB の後に起きて答える`・`サーバのサービスが異常に終わると起こし直される`・`サービスの登録簿に秘密が無い`・`状態の置き場はほかのアカウントから読めない`・`本人は状態の置き場を読めるが書けない`・`別の利用者は状態の置き場を読めない`（`net user` で作った管理者でない使い捨ての利用者で `Start-Process -Credential`）・
  `DB の置き場にはサーバのアカウントが書けない`・`上がった方が別の管理者でも、本人は上がる前に控えた利用者になる`（使い捨ての利用者の SID を `--owner-sid` で渡した管理者の実行で、ACL にその利用者があり管理者が無い）・`インストーラは状態の置き場を選ぶ口を持たない`・`インストールが終わると、インストールの間の書きの権限が残らない` を置く。
  `--self-test` は、偽の `sc.exe`・`icacls`・`whoami`・`reg`・`psql`・`curl` を `PATH` の先に置いた使い捨ての置き場で、各確かめが合う出力で通り、1 つずつ外れた出力で rc≠0 になることを見る（Linux で走る）
  検証: `tools/release/install-check.sh --self-test` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0

- [x] 3.3 `tools/release/build.sh server-windows` と `package.sh windows` が、`ashiato-setup.exe`（3.1 の crate を同じ版番号で組む）と `pgsql.sh fetch`+`slim` の出力（`pgsql\`）と `tools/db-roles.sql` を zip に入れる。Linux の配布物は変えない。`package.sh windows --test-build` は `ASHIATO_RELEASE` 無しで同じ中身の試験の zip を組む（D24）。
  `check-artifacts.sh` に、Windows の zip の中身の検査（spec の目録）・`ashiato-setup.exe` の組み立ての印の版番号がサーバと同じ・配る組み立ての `ashiato-setup.exe` と `ashiato-server.exe` に `ASHIATO_SETUP_TEST_`・`ASHIATO_SERVICE_TEST_` の文字列が無い・Linux の配布物に `pgsql` と `ashiato-setup` が無い、を足す。
  `check-artifacts.sh --self-test` に `Windows の配布物にインストーラと PostgreSQL が入り、管理画面は入らない`・`インストーラとサーバの版番号はそろう`（`0.4.0-beta.1` の印の写し）・`Linux の配布物は PostgreSQL を持たない`・`配る組み立ては試験の差し込みを持たない`（差し込みの名前を含む写しで rc≠0）を足す（足りない・余計な・版の違う写しで rc≠0）
  検証: `tools/release/check-artifacts.sh --self-test` rc=0 / `tools/release/package.sh --self-test` rc=0
- [x] 3.4 CI の `package-windows` が `pgsql.sh fetch` の zip を `actions/cache`（鍵は版と指紋）に置き、配る zip と試験の zip の 2 つを `install-windows` に渡す。`release.yml` の Windows の組み立ても同じ `package.sh` を通る
  検証: `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0 / `tools/release/check-guards.sh workflows` rc=0

## Task 4: 配る DB を作り、設定する

- [x] 4.1 インストーラの段 (3)(4)（D1・D5・D6・D17）: `initdb` の命令（`-U ashiato --locale-provider=builtin --builtin-locale=C.UTF-8 --encoding=UTF8 --data-checksums -A scram-sha-256 --pwfile=<一時>`）と `postgresql.conf` の末尾に足す行を組む純粋な関数、`--pwfile` の一時のファイルを消す片付け。
  試験（Linux）: 命令の引数に合言葉が無い・`trust` が無い・`-U ashiato` がある（Q10）・足す行に `listen_addresses='127.0.0.1'`・`port=55400`・ログの 4 つ（`log_min_error_statement='panic'` を含む）・`logging_collector=on`・`log_directory` が状態の置き場の `logs/db`・`log_truncate_on_rotation=on`。
  `install-check.sh` に `配る DB は OS に依らない並べ方で作られ、符号の順に並ぶ`（あわせて管理者の役割（`rolsuper`）が `ashiato` の 1 つだけであることを見る。Q10・D1）・`配る DB は合言葉なしの接続を通さない`・`配る DB は手元の 55400 番だけで待ち受ける`・`配る DB はログに文と値を書かない`（`log_min_error_statement=panic` も）・`DB のサービスは logs の db に書け、DB のログのファイルができる`（配る zip）・`送った文と秘密はイベントログに出ない`（イベントログの出し元 `PostgreSQL` の項目は起動の知らせがあってよく、文と秘密の値だけを探す）・`ログのファイルはほかのアカウントから読めない`・`ログは DB が 7 日分、サーバが 30 日分残る`・`DB を作るときの一時の合言葉のファイルが残らない` を足す。
  `配る DB と開発の DB は同じメジャー版` は `tools/check-pg-major.sh`（新しい台本。`pgsql.lock` の `version` と `docker-compose.yml` の `postgres:<N>` のメジャーを比べ、`--self-test` で食い違いに rc≠0）に置き、`install-check.sh` でも同梱の `postgres --version` を見る
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_db_ && test "$(cargo test -p ashiato-setup st55_db_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0 / `tools/check-pg-major.sh` rc=0 / `tools/check-pg-major.sh --self-test` rc=0 / `tools/release/install-check.sh --self-test` rc=0

## Task 5: 秘密・DB の役割・移行

- [x] 5.1 `tools/db-roles.sql` に `tools/db-roles.sh` の SQL を出し、`db-roles.sh` はそれを `psql` に流す形に替える（D7。流す SQL は同じ）。合言葉は `psql -v` の変数で渡す
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; tools/db-roles.sh'` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server app_role_ && test "$(cargo test -p ashiato-server app_role_ -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server --test server_startup server_startup_refuses_privileged_role && test "$(cargo test -p ashiato-server --test server_startup server_startup_refuses_privileged_role -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0（ST28 の既存の試験が通る）
- [x] 5.2 インストーラの段 (2)(7)(8)（D7・D11）: 秘密を `OsRng` の 32 バイトの base64url で作り、`settings\server.json`（D4 の鍵。`ASHIATO_ARCHIVE_USER_ID` は D16（仮）の nil）に書く。同梱の `psql` で `-U ashiato`（D1 の固定の管理者の役割。`admin` の変数も `ashiato`）として `db-roles.sql` を流し、`ashiato-server migrate` を表の所有者で、`MIGRATE_BACKUP_DIR`・`PG_DUMP` を絶対の場所で渡して当てる。移行が失敗したらサーバのサービスを開始せずに rc≠0。
  試験（Linux）: 秘密の長さと文字の種類・2 回作ると違う・ログインの印の鍵と API の合言葉が 16 文字以上で違う・設定の JSON の鍵の一覧・`psql` と `migrate` の命令の引数に秘密が無い・`psql` の命令が `-U ashiato` で入る・`MIGRATE_BACKUP_DIR` と `PG_DUMP` が絶対の場所。
  `install-check.sh` に `インストールごとに秘密が違う`（同じ runner で消して入れ直し、状態の置き場も消した 2 回の設定の秘密を比べる）・`インストーラの出力とログに秘密が出ない`・`作った秘密でサーバの起動の検査が通る`・`配る形に移行を当ててサーバが起きる`・`実行時の役割は管理者でも表の所有者でもない`・
  `移行の前の退避は状態の置き場に、同梱の pg_dump で取られる`・`移行が失敗すると、サーバのサービスを開始せずに止まる`（`ASHIATO_SETUP_TEST_MIGRATE_FAIL=1`）・`サービスの起動では移行を当てない`・`インストールの間、ネットワークに出ない`（D12。展開した置き場とプログラムの置き場の実行ファイルの外向きを Windows のファイアウォールで止めて入れる）を足す
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_secret_ && test "$(cargo test -p ashiato-setup st55_secret_ -- --list 2>/dev/null | grep -c ": test$")" -ge 5'` rc=0 / `tools/release/install-check.sh --self-test` rc=0 / `tools/check-secrets.sh` rc=0

## Task 6: 走らせ直しと `repair` の口（D10・D27）

- [x] 6.1 インストーラの段の順（D10）と「既にあるか」の判定を、状態の置き場の見え方（設定の有無・`db\17\PG_VERSION` の有無・照らすための値の有無・サービスの有無）から次の段を決める純粋な関数にする。DB があるのに設定が無ければ何も書き換えずに rc≠0（DB と秘密の食い違いと置き場の場所と `repair` の口を示す）。
  試験（Linux。見え方の全部の組み合わせ 16 通りの表）: 次の段・止まるか・上書きしない
  `install-check.sh` に `走らせ直しても秘密と DB は変わらない`・`DB を作る途中で落ちても、走らせ直しで DB ができる`（`ASHIATO_SETUP_TEST_STOP_AT=initdb`）・`DB はあるのに秘密が無いと、何も書き換えずに止まる`・`合言葉を決める前に閉じたら、走らせ直しで合言葉の段からやり直す`（標準入力を空で閉じる）・`別の利用者が走らせ直しても、本人は変わらない`（使い捨ての管理者 B で走らせ直す）を足す。一時の書きの権限（D21）を段の終わりと走らせ直しの最初に外すことも、ここの表の試験に入れる
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_rerun_ && test "$(cargo test -p ashiato-setup st55_rerun_ -- --list 2>/dev/null | grep -c ": test$")" -ge 4'` rc=0 / `tools/release/install-check.sh --self-test` rc=0
- [x] 6.2 `repair` の口（Q8・Q10・D27・D28）: 上がり方は `password` と同じ（D22・D21）。止まった状態（`db\17\PG_VERSION` があり設定が無い）でだけ動き、それ以外は何も書かずに rc≠0 で「`repair` は要らない」と走らせ直しか `password` を示す。
  段は D27 の (a)〜(e): サービスを止める → 秘密を作り直して `settings\server.json.repair` に書く → `pg_ctl start -o` で一時の形（`127.0.0.1`・空いている一時の番号・一時の `hba_file` の `sspi` と `pg_ident` で上がった利用者だけを DB の管理者の役割 `ashiato`（D1）に結ぶ）に起こし、`psql -U ashiato` で入って 3 つの役割の `ALTER ROLE … PASSWORD` を環境変数と `\getenv` で流す →
  一時の形を止めて一時の `hba_file` を消し、`server.json.repair` を `server.json` へ名前を替える → 走らせ直しの (5)〜(13)。出力の最後に収集アプリの繋ぎ直しが要ることと `docs/install.md` を示す（値は示さない）。役割の段で止まる差し込み `ASHIATO_SETUP_TEST_STOP_AT=repair-roles` を試験の組み立てだけに置く。
  試験（Linux）: 口の判定（止まった状態・設定がある・DB が無い の 3 つ）・置き場の ACL の読み（`icacls` の出力）から本人の SID を 1 つ選ぶ（サービスのアカウントを除く・2 つ以上なら止まる）・段の順（`server.json` を置くのが役割の段の後）・一時の `hba_file` に `trust` が無く `sspi` の行が 1 つだけでその役割が `ashiato`・`psql` が `-U ashiato` で入り、名前を設定のファイルから読まない（設定が無くても組める）・`pg_ctl` と `psql` の命令の引数に秘密が無い・出力に秘密が無い。
  `install-check.sh` に `設定を退けて止まった後に repair すると、記録を残したままログインできる`（配る zip）・`repair の後は前の API の合言葉では送れず、繋ぎ直しの案内が出る`（配る zip）・`止まった状態でなければ repair は何も変えずに終わる`（配る zip）・`repair の後に合言葉なしの入口と一時の番号が残らない`（配る zip。`pg_hba.conf` の有効な行と `netstat -ano` の `postgres` の待ち受け）・`別の管理者が repair しても、本人は変わらない`（使い捨ての管理者 B で `repair`）・`repair の途中で落ちても、もう一度 repair できる`（試験の zip。`ASHIATO_SETUP_TEST_STOP_AT=repair-roles`）を足す。
  一時の形が上がった札で通らなければ D28 の反転条件（一時の `unix_socket_directories` と `local … trust`）に替え、どちらも通らなければ上流へ返す
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_repair_ && test "$(cargo test -p ashiato-setup st55_repair_ -- --list 2>/dev/null | grep -c ": test$")" -ge 5'` rc=0 / `tools/release/install-check.sh --self-test` rc=0

## Task 7: 画面の合言葉（照らすための値・決める口・決め直す口）

- [x] 7.1 サーバ（D8）: `argon2` を依存に足し、`WEB_PASSWORD_VERIFIER` があれば照らす形で動く（`WEB_PASSWORD` と両方あれば起動しない・形が違えば起動しない）。ログインの印の材料は `"ashiato-web-session-v2" || <照らすための値>`、`WEB_SESSION_KEY` が画面の合言葉と同じでないかは照らすための値と照らして見る。元の値の形（`v1`）は変えない。
  照らす形かどうかは口で決める（`service` の口は照らす形だけ・それ以外は元の値の形だけ。D8）。照らす形の起動で `API_TOKEN` と `WEB_SESSION_KEY` を照らすための値と照らし、合えば起動しない。
  `crates/server/src/st55_tests.rs` に `// Scenario: インストールした形でも、照らすための値と合う合言葉でだけログインできる`・`// Scenario: インストールした形で照らすための値が無いと起動しない`（無い・形が違う）・`// Scenario: インストールした形の設定に画面の合言葉の元の値があると起動しない`・`// Scenario: 画面の合言葉が API の合言葉と同じだと起動しない`（照らす形の側。API の合言葉から作った照らすための値）と、照らす形で合言葉を決め直すと前の印が 401 になる試験を置く。ST28・ST44 の画面のログインの試験は変えずに通す
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st55_verifier_ && test "$(cargo test -p ashiato-server st55_verifier_ -- --list 2>/dev/null | grep -c ": test$")" -ge 6'` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server web_session'` rc=0 / `tools/check-licenses.sh` rc=0
- [x] 7.2 インストーラ（Q3・D8・D23）: 段 (11) で 2 回入れさせ（端末なら `rpassword`、端末でなければ標準入力の 2 行）、16 文字未満・`change-me` で始まる・API の合言葉と同じ・ログインの印の鍵と同じ・一致しない を理由つきで断って入れ直させ、照らすための値だけを設定に書き、サーバのサービスを起こし直して既定のブラウザで `http://127.0.0.1:18700/` を開く。
  `password` の口は上がった後に同じ検査で決め直し、サーバのサービスを起こし直す（ほかの鍵を変えない）。試験（Linux）: 検査の 5 つの理由・設定に元の値が無い・`password` の口がほかの鍵を変えない。
  `install-check.sh` に `決めた合言葉で、そのまま画面にログインできる`・`短い・既定の雛形・一致しない合言葉は受け付けない`・`API の合言葉やログインの印の鍵と同じ値は受け付けない`・`合言葉を決め終えると、既定のブラウザで画面を開く`（試験の zip の命令の記録）・`合言葉の元の値は、どこにも残らない`・`インストールした形の設定は、画面の合言葉の元の値を持たない`・
  `決め直した合言葉でログインでき、前の合言葉ではできない`・`決め直すと、それまでのログインは切れる`・`決め直しても、ほかの秘密と記録は変わらない`・`管理者に上がらなければ決め直せない`（試験の zip。`ASHIATO_SETUP_TEST_ELEVATION=deny` で確認を断ったときと同じ戻り値にし、rc≠0・設定の指紋が変わらない）を足す
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_password_ && test "$(cargo test -p ashiato-setup st55_password_ -- --list 2>/dev/null | grep -c ": test$")" -ge 5'` rc=0 / `tools/release/install-check.sh --self-test` rc=0

## Task 8: 取り込み待ちとダウンロード・インストールした形の既定の置き場

- [x] 8.1 インストーラの段 (9)（D9・D16（仮））: 本人のダウンロード（登録簿の `User Shell Folders` の `{374DE290-…}` を展開）に `NT SERVICE\ashiato-server` の `RX`、`%USERPROFILE%\ashiato\取り込み待ち` を作って `M`、親の `%USERPROFILE%\ashiato` にそのフォルダだけの `AD` と `RX`。同期されるフォルダ（「ドキュメント」・「デスクトップ」・OneDrive の下）を取り込み待ちに選ばない。
  試験（Linux）: 登録簿の値の展開（`%USERPROFILE%` を含む値・OneDrive の下の値）・取り込み待ちの場所が同期されるフォルダの下にならない・付ける `icacls` の命令。
  `install-check.sh` に `ダウンロードに置いた Takeout が、インストールした形で取り込まれる`（`tools/fixtures` の小さな Takeout の書庫を runner の利用者のダウンロードに置き、形の確認の印を置き、見回りの間隔は同梱の `psql` で登録簿の `s01-archive-inbox` の `fetch_interval_sec` を下限まで短くして待ち（環境変数 `ASHIATO_ARCHIVE_SCAN_SEC` はサーバが起動を拒むので使わない）、画面のログインで読み出しに出る・ログに `archive_disabled` が無い）・
  `取り込み待ちは本人のプロファイルの直下にでき、サーバが読み書きできる`・`取り込み待ちを消しても、サーバが作り直す`・`ダウンロードにはサーバのアカウントの読みだけが足される` を足す
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_inbox_ && test "$(cargo test -p ashiato-setup st55_inbox_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0 / `tools/release/install-check.sh --self-test` rc=0
- [x] 8.2 サーバ（`data-durability`・`device-collection` の MODIFIED・D14）: `service` の口で動くとき、`ASHIATO_EXPORT_DIR`・`ASHIATO_MEDIA_DIR`・`ASHIATO_ARCHIVE_COPY_DIR` が無ければ設定のファイルの場所から組んだ状態の置き場の `export`・`media`・`archive-copies` を既定にする（インストーラはこの 3 つを設定に書かない。D4）（開発の形の既定は変えない）。
  `crates/server/src/st55_tests.rs` に `// Scenario: インストールした形では、指定が無ければ状態の置き場の export に書く`・`// Scenario: インストールした形では、設定が無ければ状態の置き場の media に置く`（状態の置き場を一時フォルダにした `service` の形で、サービスのプロファイル（`LOCALAPPDATA` を別の一時フォルダにする）の下に何もできない）。
  `export_tests.rs:73`・`media_tests.rs:211` の印つきの試験は変えずに通す。`install-check.sh` に `インストールした形の書き出しは、本人が読めて、ほかのアカウントは読めない` を足す
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st55_place_ && test "$(cargo test -p ashiato-server st55_place_ -- --list 2>/dev/null | grep -c ": test$")" -ge 2'` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server export_tests && scripts/quiet-run ct -- cargo test -p ashiato-server media_tests'` rc=0

## Task 9: 番号・案内・空き・消す口

- [x] 9.1 番号（D5）: 登録の前に 55400・18700 を `netstat -ano` の出力から見て、塞がっていれば番号と持ち主のプロセス名を示して rc≠0。`tools/check-ports.sh --self-test` に `配る番号は開発の台本の割り当てと被らない`（`tools/ports.sh` の範囲を全部挙げて 18700・55400・18701・55401 と比べる）を足す。
  試験（Linux）: `netstat -ano` の出力の読み方（IPv4・IPv6・別の番号）。`install-check.sh` に `番号が塞がっていると、登録せずに止まる`（runner で `127.0.0.1:55400` を塞ぐ小さな待ち受けを立てて入れる）を足す
  検証: `tools/check-ports.sh --self-test` rc=0 / `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_port_ && test "$(cargo test -p ashiato-setup st55_port_ -- --list 2>/dev/null | grep -c ": test$")" -ge 2'` rc=0
- [x] 9.2 案内と空き（Q6・Q7）: 段 (13) で Tailscale の「無人で動かす」と `tailscale serve --bg --https=443 http://127.0.0.1:18700` 相当のコマンドを示す（Tailscale の命令を走らせない）。始めに状態の置き場のドライブの空きを見て 40 GB より少なければ空きと見積もりを示す（止めない。試験の差し込み `ASHIATO_SETUP_TEST_FREE_GB`）。
  試験（Linux）: 案内の文に 18700 があり、インストーラの命令の一覧に `tailscale` が無い・空きの判定（39 GB で知らせる・40 GB で知らせない）。
  `install-check.sh` に `最後に Tailscale の転送先を示し、Tailscale の設定は変えない`（試験の zip。出力の文と、命令の記録に `tailscale` が無いこと）・`空きが見積もりより少なければ知らせ、インストールは続ける` を足す
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_notice_ && test "$(cargo test -p ashiato-setup st55_notice_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0
- [x] 9.3 「アプリと機能」と消す口（Q4・D25）: 段 (12) で `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato`（表示名・版番号・`UninstallString`）。`uninstall` の口はサービスを止めて消し、ダウンロードと取り込み待ちの ACE を外し、登録を消し、プログラムの置き場を消し（自分は写しから消す）、状態の置き場と取り込み待ちの中身は残して場所と `docs/install.md` を示す。
  試験（Linux）: 消す命令の一覧に状態の置き場と取り込み待ちの中身を消すものが無い。
  `install-check.sh` に `インストールすると「アプリと機能」に載る`・`消すとサービスとプログラムと登録が消える`・`消しても記録の置き場は残り、その場所を示す`・`消した後に入れ直すと、前の記録の上で続く`・`消すとダウンロードと取り込み待ちに足した権限が外れる` を足す
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_uninstall_ && test "$(cargo test -p ashiato-setup st55_uninstall_ -- --list 2>/dev/null | grep -c ": test$")" -ge 2'` rc=0 / `tools/release/install-check.sh --self-test` rc=0

## Task 10: Windows の CI の通しの残り

- [x] 10.1 `install-check.sh` に、まだ置いていない `起動を拒んだ理由が、サーバのログのファイルに残る`（設定のログインの印の鍵を 15 文字に差し替える）を足し、`install-windows` の job の終わりに `python3 scripts/check_scenarios.py . st55-server-installer` を走らせる。
  job の出力に、置いた全部の `== Scenario:` の行が出ることを `install-check.sh --list` と突き合わせる（走らずに飛ばした Scenario があれば job が落ちる）
  検証: `tools/release/install-check.sh --self-test` rc=0 / `tools/release/install-check.sh --check-coverage openspec/changes/st55-server-installer` rc=0（spec の Scenario のうち cargo test・`pgsql.sh`・`check-artifacts.sh`・`check-ports.sh`・`check-pg-major.sh` が印を持つものを除いた全部が `--list` にある） / `python3 scripts/check_scenarios.py . st55-server-installer` rc=0

## Task 11: 手順書と申し送り

- [x] 11.1 `docs/install.md`（新しい）: 入れる（zip を展開して `ashiato-setup.exe`。SmartScreen の警告・UAC 1 回）・画面の合言葉を決める・忘れたら `password`・消す・記録を手で消す（管理者として所有権を取る `takeown /r` と `icacls` の手順）・Tailscale の「無人で動かす」と転送先・空き・秘密が欠けて止まったときの `repair` と、その後の収集アプリの繋ぎ直し（D27）・残るリスク（管理者は所有権を取れば読める・止まってから `repair` するまでの間の携帯の上限）。
  `docs/release.md` §7 を D18（仮）の手順に書き直す（旧い手順は消す）。`docs/archive-inbox.md` にインストールした形の取り込み待ち（D9）を足す。`docs/production-prep.md` の既定値の棚卸しに、配る DB の行（D1・D6・D17）を ★ で足す。`CLAUDE.md` の「構成」に `crates/installer` の行を足す
  検証: `bash -c 'for h in "ashiato-setup.exe" "password" "repair" "uninstall" "takeown" "無人で動かす" "18700"; do grep -q "$h" docs/install.md || exit 1; done && grep -q "ashiato-setup" docs/release.md && grep -q "取り込み待ち" docs/archive-inbox.md && grep -q "crates/installer" CLAUDE.md'` rc=0
- [x] 11.2 申し送り（D13・D14）: `docs/handoff/ST53.md`（同梱した PostgreSQL と DLL の許諾を FR-127 の一覧と NFR-26 の検査へ。`pgsql\LICENSES\` の場所）・`docs/handoff/ST30.md`（インストールした形のバックアップの置き場は `C:\ProgramData\ashiato\backups`。サーバのサービスのアカウントが書く）・
  `docs/handoff/ST54.md`（D14 の開発版の列を確かめる）・`docs/handoff/ST57.md`・`docs/handoff/ST58.md`（サービスの名前・置き場の並び・`layout.rs`・インストールの走らせ直しの段）に 1 節ずつ足す
  検証: `bash -c 'grep -q "pgsql" docs/handoff/ST53.md && grep -q "ProgramData" docs/handoff/ST30.md && grep -q "ashiato-db-dev" docs/handoff/ST54.md && grep -q "layout.rs" docs/handoff/ST57.md && grep -q "layout.rs" docs/handoff/ST58.md'` rc=0
- [x] 11.3 全体: `cargo fmt --all -- --check`・`cargo clippy --workspace --all-targets -- -D warnings`・Windows の 2 crate の `--target x86_64-pc-windows-gnu` の clippy・`cargo test --workspace`・`tools/check-openapi.sh`・`tools/check-migrations.sh`・`tools/check-licenses.sh`・`tools/check-log-private.sh`・`tools/check-secrets.sh`
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy -p ashiato-setup -p ashiato-server --all-targets --target x86_64-pc-windows-gnu -- -D warnings && scripts/quiet-run all -- cargo test --workspace'` rc=0 /
  `bash -c 'tools/check-openapi.sh && tools/check-migrations.sh && tools/check-licenses.sh && tools/check-log-private.sh && tools/check-secrets.sh'` rc=0

## Task 13: DB のサービスの見張り（D35）

- [ ] 13.1 インストーラ（D35）: サービスの登録の後、`ashiato-db` のサービスの DACL に `NT SERVICE\ashiato-server` の SID の `(A;;LCRP;;;<SID>)` を足す命令（`sc.exe sdshow` の結果に足して `sc.exe sdset`。既にあれば足さない）を `configure_service_commands` の後に置く。
  試験（Linux。`crates/installer/src/lib.rs`）: 足す ACE が `LC`・`RP` だけで、`WP`（止める）・`DC`（設定）を含まないこと・既にある SDDL に重ねて足さないこと
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st55_db_watch_ && test "$(cargo test -p ashiato-setup st55_db_watch_ -- --list 2>/dev/null | grep -c ": test$")" -ge 2'` rc=0
- [ ] 13.2 サーバ（D35）: `service` の口で 5 秒ごとに `ashiato-db` の状態を読み、サーバが動いている間に `STOPPED` なら `StartService` で起こし、`kind=db_service_restarted` をログに書く。判断は純粋な関数（`crates/server/src/service.rs`）、Windows の呼び出しは `crates/winsvc`。
  試験（Linux。`crates/server/src/st55_tests.rs`）: `STOPPED` → 起こす・`RUNNING`/`START_PENDING`/`STOP_PENDING` → 何もしない・問い合わせが失敗 → 何もせず理由をログに書く
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st55_db_watch_ && test "$(cargo test -p ashiato-server st55_db_watch_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0 /
  `cargo clippy -p ashiato-server -p ashiato-winsvc --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0
- [ ] 13.3 `install-check.sh` の `DB のサービスが異常に終わると起こし直される` を spec の WHEN に戻す: pg_ctl（サービスの PID）でなく **PostgreSQL の主だけ**（`postmaster.pid` の先頭行）を `taskkill /F` で終わらせ、60 秒以内に `ashiato-db` が実行中になり 55400 が接続を受けることを見る。あわせて `sc.exe sdshow ashiato-db` にサーバの SID の `LCRP` があり、`WP` が無いことを見る
  検証: `tools/release/install-check.sh --self-test` rc=0（実地は PR の CI の `install-windows`）

## 人間の確認待ち（確認バッチ。PR の CI が緑になった後）

Scenario ではない —— Windows の CI が全部の Scenario を見る。ここは CI では見られない本物の UAC・本物の端末・再起動・Tailscale の確かめで、結果は確認バッチが `premises.md`（P16・P18）に書く。

- [ ] 12.1 **Windows サンドボックスの中のインストールの録画**（Q5 の推奨・`tools/recording/scenarios.d/` に ST55 の台本を新しく置く）: まっさらなサンドボックスに配る zip を写し、`ashiato-setup.exe` を開いて、管理者の確認が 1 回出る・合言葉を 2 回入れて決める・ブラウザで画面が開いてログインできる、までを 1 本に撮る。
  サンドボックスの利用者で管理者の確認が出なければ、録画で見るのは合言葉を決めてログインするまで（P16 の確かさを書く）
  > 物理: device —— 本物の管理者の確認（UAC）の画面と、端末で伏せ字の入力をする操作は、CI の runner（UAC が無い・端末が無い）では出ない
- [ ] 12.2 **再起動の後にログオンせずに動いている**（D26（仮））: サンドボックスの中で Windows を再起動して状態が残るなら、再起動してログオンせずに、ホストから（サンドボックスの網へ）`/healthz` を叩く。残らなければ、CI の「サービスは利用者でなくサービス専用のアカウントで動く」までとし、完了の判定の再起動の項を実物で見ていないことを P16 と確認バッチの結果に書く（本人の切り替えの最初の再起動で実物が通る）
  > 物理: device —— CI の runner は再起動できない
- [ ] 12.3 **Tailscale の無人で動かす**（P18）: 本人の PC で `tailscale debug prefs` の `ForceDaemon` を読み、インストーラが示した手順で「無人で動かす」を入れた後に、ログオフした状態で携帯から `/healthz` に届くかを見る（本人の環境を切り替えるとき。D18（仮））
  > 物理: device —— 本人の網（tailnet）と携帯は本人の手元にしか無い
