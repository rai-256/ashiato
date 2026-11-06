# code review — st55-server-installer

## final review（1941853..7c2330d）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘の写し。判定は **No（本物のインストールの経路は一度も走っていない）**。
範囲は ST55 の差分（6a8d707b..HEAD。ST44 の取り込み #231 は除く）。R1〜R9 の main.rs の行は final の controller が `sed -n 320,445p` で再現した。

## R1. 合言葉の入力を最初に読むが、install-windows の本物の zip の呼び出しは stdin を渡さないので必ず rc=1
- 成果物: crates/installer/src/main.rs / tools/release/install-check.sh
- 根拠: main.rs:357 `read_web_passwords` がどの命令より前。install-check.sh:352・:407 は `install >"$trace" 2>&1` だけ（self-test は `printf … |` で流す）
- kind: technical
- 処置: fixed 7.2 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R2. `ASHIATO_SETUP_TEST_REGISTER_FAIL=0` でも登録の失敗として止まる
- 成果物: crates/installer/src/main.rs
- 根拠: main.rs:327 `var_os("ASHIATO_SETUP_TEST_REGISTER_FAIL").is_some()`。install-check.sh:350 は `=0` を渡す
- kind: technical
- 処置: fixed 3.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R3. 本人の SID を控えず、管理者にも上がらない（Task 3.1 の昇格が配線されていない）
- 成果物: crates/installer/src/main.rs / commands.rs
- 根拠: `commands::elevation_command`・`owner_sid_from_whoami` は試験からしか呼ばれない。main.rs:701 `.unwrap_or("S-1-0")` で実在しない SID を ACL に使う
- kind: technical
- 処置: fixed 3.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R4. ACL の順序が壊れている（存在しない `db\17` への icacls・`:r` で一時の M が即消える・`/inheritance:r` で管理者が外れた後に完了記録を書く）
- 成果物: crates/installer/src/main.rs / commands.rs
- 根拠: main.rs:391 の `remove_temporary_write_acl_commands` が `db\17` を作る :401 より前。`temporary_write_acl_commands`（`/grant M`）の後に `acl_commands`（同じ SID に `/grant:r RX`）。`acl_commands` は `/inheritance:r` で Administrators・SYSTEM を外す（commands.rs の `acl_commands`）→ :427・:441 の `fs::write(.installer-*-complete)` が書けない（D21）
- kind: technical
- 処置: fixed 6.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R5. `--pwfile` を DB の置き場の中に置くので initdb が「空でない」で落ち、管理者の合言葉が `db\17` に残る
- 成果物: crates/installer/src/main.rs
- 根拠: main.rs:360 `layout.db_dir.join(".pwfile")` → :403 で書き → `initdb -D db_dir`。D1 は状態の置き場に置くと決めている（Task 4 F3 を Minor としたが本番も同じ）
- kind: technical
- 処置: fixed 4.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R6. プログラムの置き場への写しと、サーバのサービスの作成（`sc.exe create ashiato-server`）がどこにも無い
- 成果物: crates/installer/src/main.rs / commands.rs / design.md（D10）
- 根拠: `grep -n 'sc.exe create\|copy' crates/installer/src/*.rs` は uninstall の自分の写し（main.rs:472）だけ。commands.rs:251 の `sc.exe config ashiato-server` は無いサービスに当たる（1060）。`run_migrate`（main.rs:291）・`run_psql`（:233）は Program Files の下を叩き、initdb / pg_ctl は裸の名前（PATH に `pgsql\bin` は無い）。UninstallString も無い exe を指す。design.md:102 の D10 の段 (1)〜(13) にも写しと create が無い（plan の欠落）
- kind: technical
- 判断: `sc.exe config ashiato-server` と spec がサーバのサービスの存在を前提にしており、欠けた段を補うだけ。要件は変えない → design に D（仮）で段を足す
- 処置: fixed D29 仮 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R7. 配る DB の設定（127.0.0.1・55400・ログの 4 つ）が `postgresql.conf` に書かれない（申し送り C7 が未処置）
- 成果物: crates/installer/src/main.rs / db.rs
- 根拠: `db::postgresql_conf_append` は lib.rs の試験からしか呼ばれない（`grep -n postgresql_conf_append crates/installer/src/*.rs`）
- kind: technical
- 処置: fixed 4.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R8. `run_psql` に合言葉を渡さないので scram-sha-256 の DB に入れない
- 成果物: crates/installer/src/main.rs
- 根拠: main.rs:233-258 に PGPASSWORD が無い（`grep -n PGPASSWORD crates/installer/src/*.rs` は 0 件）
- kind: technical
- 処置: fixed 5.2 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R9. 走らせ直しと repair が登録の段で落ちる・`decide_install` の 2 つの入力が本番で効かない
- 成果物: crates/installer/src/main.rs
- 根拠: main.rs:415 `service_commands` を無条件に足す（2 回目の `pg_ctl register` は「既にある」）。repair は最後に `install()`（:673）。main.rs:337-342 で `verifier: false, services: false` を固定
- kind: technical
- 処置: fixed 6.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R10. install-windows は構造的に毎回赤（印 76 本のうち 24 本は `self_test()` でしか出ず、CI は self-test を走らせない）。試験の zip の経路も本物で落ちる所がある
- 成果物: tools/release/install-check.sh / .github/workflows/ci.yml
- 根拠: install-check.sh:628・:732・:742・:770 ほかの印は 436 行以降の `self_test()` の中。ci.yml:419-424 は `--list` と zip の経路の印を `comm -23` で比べる。`grep -rn ASHIATO_INSTALL_STATE_DIR crates/` は 0 件（`check_install_settings "$work/state"` が die）。:184-185 の `C:/Users/Public/...` の決め打ち、:384 の `export /grant:r` は偽の installer だけが出す、:161・:170-171 の icacls の引用、:159・:166 の `MIGRATE_BACKUP_DIR=`・`PG_DUMP=` が本物の記録（main.rs:312）と合わない
- kind: technical
- 処置: fixed 3.2 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R11. Scenario の印がその下の確かめと一致しない（Global Constraint「印は実際の確かめにだけ置く」違反）・差し込み 2 つが crate に無い
- 成果物: tools/release/install-check.sh / crates/server / crates/installer
- 根拠: install-check.sh:148（設定が無いと起動しない → `grep 'ashiato-server migrate'`）・:154・:156（DB が 120 秒開かない → `grep 'psql -U ashiato'`）・:158・:251・:255・:397-404（決め直し → ファイルの有無）。`ASHIATO_SERVICE_TEST_DB_DELAY_SEC`（Task 2.2）と `ASHIATO_SETUP_TEST_STOP_AT=initdb` は偽の installer にしか無い
- kind: technical
- 処置: fixed 10.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R12. `check_real_services` の DB の検査は本物で通らない（GUC でない `show`・`-U ashiato` は設計上認証できない・`|| true`）
- 成果物: tools/release/install-check.sh
- 根拠: install-check.sh:272 `show datlocprovider; show datlocale`、:272-277 `-U ashiato`（Q10・D1 で管理者の合言葉は pwfile を消した後どこにも無い）、:278 `|| true`。Task 4 F6・F7 は未解決のまま
- kind: technical
- 判断: `ashiato_app` と server.json の合言葉で `pg_database`・`pg_roles` を読む形に直す。D1・Q10 は変えない
- 処置: fixed 3.2 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R13. 秘密（server.json）を ACL を絞る前に書いている
- 成果物: crates/installer/src/main.rs
- 根拠: main.rs:374-388 の書き → :391 以降の ACL。D10 の (1) 置き場と ACL → (2) 秘密 の順に反し、`C:\ProgramData` の継承（Users の読み）のまま置かれる
- kind: technical
- 処置: fixed 6.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R14. 停止の知らせを取りこぼす（`notify_waiters` は待つ者がいないと何も残さない。DB 待ちの間の Stop が失われる）
- 成果物: crates/server/src/service.rs
- 根拠: service.rs:175-184。Task 2 re-review 1 の F1 前半は re-review 2 で abort と cleanup だけを見て ADDRESSED にされた
- kind: technical
- 処置: fixed 2.3 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R15. 開発の形でも起動の拒否を作業ディレクトリの `logs/server` に書く
- 成果物: crates/server/src/lib.rs / service.rs
- 根拠: lib.rs の `run_inner` の `crate::service::log_startup_refusal` 2 か所が無条件、service.rs:149-160 は `ASHIATO_SETTINGS_PATH` が無ければ相対の `logs/server`。D17（service の口だけ）に反する
- kind: technical
- 処置: fixed 2.4 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R16. 走らせ直しで本人の SID が変わる（設定の `ASHIATO_OWNER_SID` を読まない）
- 成果物: crates/installer/src/main.rs
- 根拠: `owner_sid_for_rerun` は使われていない。走らせ直しのたびにその回の `--owner-sid`（無ければ S-1-0）で `acl_commands` を付け直す（Scenario「別の利用者が走らせ直しても、本人は変わらない」）
- kind: technical
- 処置: fixed 6.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R17. CI の install-windows の最後の `python3 scripts/check_scenarios.py` は runner で解決できない
- 成果物: .github/workflows/ci.yml
- 根拠: ci.yml:427。ci.yml:446-448 自身が「scripts/ は .harness2/scripts への symlink で、.harness2 は runner には無い」と書く
- kind: technical
- 処置: fixed 3.2 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R18. 「アプリと機能」の UninstallString の引用が PowerShell 5.1 から外の命令に渡すと崩れる
- 成果物: crates/installer/src/commands.rs
- 根拠: commands.rs:260 `/d '"C:\...\ashiato-setup.exe" uninstall'`（7.3 より前は中の `"` が落ちる。Task 9 の ⚠️）
- kind: technical
- 処置: fixed 9.3 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R19. 管理者で起こし直した `password` の失敗が親に戻らない
- 成果物: crates/installer/src/main.rs
- 根拠: main.rs:538-545 の `Start-Process -Wait` に `-PassThru` と ExitCode の確認が無い
- kind: technical
- 処置: fixed 7.2 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R20. アンインストールの写しの結果が戻らない
- 成果物: crates/installer/src/main.rs
- 根拠: main.rs:470-480 で写しを spawn してすぐ Ok を返す
- kind: technical
- 処置: fixed 9.3 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R21. `run_inner` の `process::exit(2)` は SCM に Stopped を返さず、failure actions で再起動が続く
- 成果物: crates/server/src/lib.rs
- 根拠: 設定が不正なときの `process::exit(2)`（service の口）
- kind: technical
- 処置: fixed 2.2 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R22. 開発の口で verifier があるときの理由が `raw_password_present` で意味が逆
- 成果物: crates/server/src/lib.rs
- 根拠: Task 7 F10
- kind: technical
- 処置: fixed 7.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R23. `session_post` の Argon2 の照合を async の handler の中で同期のまま行う
- 成果物: crates/server/src/web_session.rs
- 根拠: Task 7 F11
- kind: technical
- 処置: fixed 7.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R24. `user_downloads` の `split_whitespace().last()` は空白を含むパスで壊れる
- 成果物: crates/installer/src/main.rs
- 根拠: main.rs:40（Task 8 F11）
- kind: technical
- 処置: fixed 8.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R25. install-check.sh の `find … -name '*pwfile*' -o -name '.pwfile'` の優先順位と探す範囲
- 成果物: tools/release/install-check.sh
- 根拠: install-check.sh:310（Task 4 F8）
- kind: technical
- 処置: fixed 3.2 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R26. 死んだコード（`secret_free_install_commands`・`repair_commands`・`install_phases`・`network_is_disabled_during_install`・`repair_owner_sid`・`decide_repair`）
- 成果物: crates/installer/src
- 根拠: 試験からしか呼ばれない。`secret_free_install_commands` は `_secrets` を捨てるので試験が何も確かめない
- kind: technical
- 処置: fixed 6.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R27. `DisplayVersion` が `CARGO_PKG_VERSION` で、渡した版番号（`ASHIATO_VERSION`）でない
- 成果物: crates/installer/src/commands.rs
- 根拠: commands.rs:261
- kind: technical
- 処置: fixed 9.3 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R28. VCRUNTIME140 / MSVCP140 を同梱しない判断が、まっさらな機械で未確認
- 成果物: tools/release/pgsql.lock
- 根拠: Task 1 の ⚠️。手元（Linux）では確かめられず、Windows の CI か確認バッチで見る
- kind: technical
- 処置: fixed D33 仮 —— 手元で EDB の zip（sha256 一致）を落として `x86_64-w64-mingw32-objdump -p` で見ると、postgres.exe・initdb.exe・pg_ctl.exe・psql.exe・pg_dump.exe・pg_isready.exe・libpq.dll ほかが VCRUNTIME140.dll を、icuuc67.dll が MSVCP140.dll も読み、zip の pgsql/bin には無かった（再現）。build.sh が Windows の runner の Visual Studio の再頒布の置き場から 2 つを pgsql/bin に写し、THIRD-PARTY-NOTICES.txt に節を足し、check-artifacts.sh が無い zip を落とす（自己テストに無い zip の場合を足した）

## R29. docs/install.md の repair の節が、新しい API_TOKEN をどこで読むかを書いていない
- 成果物: docs/install.md
- 根拠: Task 11 F4
- kind: technical
- 処置: fixed 11.1 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## R30. check-artifacts.sh に「サーバの配布物に PostgreSQL があれば THIRD-PARTY-NOTICES.txt に PostgreSQL の見出しがある」検査が無い（申し送り C12 の一部）
- 成果物: tools/release/check-artifacts.sh
- 根拠: docs/handoff/ST55.md の C12。`pgsql\LICENSES` は写すが見出しの検査は無い
- kind: technical
- 処置: fixed 3.3 —— fix 22e017ab..e92b0bca。scoped re-review で ADDRESSED

## final review の scoped re-review（22e017ab..e92b0bca）

fix の後の scoped re-review（SDD の `re-review-prompt.md`）。R1〜R30 は R28 を除き ADDRESSED。R28 は NOT ADDRESSED（手元で確かめられない）。
fix が新しく持ち込んだものを R31〜R39 に写す。**fix の機会は final では使い切った**ので、`処置:` の無いものは finish の 1 回の fix に渡す。
R31・R32・R35 は本物の Windows でしか確かめられない（PLAUSIBLE）。

## R31. サービスを登録する前に `NT SERVICE\<名前>` を icacls に渡しているので、まっさらな機械の最初のインストールが (1) で落ちうる
- 成果物: crates/installer/src/main.rs / commands.rs
- 根拠: main.rs:711 の `acl_commands`（commands.rs:338 の `NT SERVICE\ashiato-server`・`NT SERVICE\ashiato-db`）と main.rs:789 の `remove_database_write_acl_commands`（commands.rs:406）が、(5) の登録（main.rs:797-802）より前に走る。LSA は登録されていないサービスの仮想アカウント名を SID に引けない（1332）。消す口も `sc delete` の後に `/remove:g "NT SERVICE\ashiato-server"`（main.rs:934-959、commands.rs:188-208）。P19 の probe（sandbox-probe.sh:20-43）は登録してから使う順で、この形を試していない。PLAUSIBLE（Windows でしか確かめられない）
- 提案: 直し方は 2 つ。`sc.exe showsid <名前>` で `*S-1-5-80-…` を引いて使う（登録の前でも引ける）か、登録を ACL より前に動かす。消す口は SID で外すか、`sc delete` の前に外す
- kind: technical
- 処置: fixed 3.1 —— サービスの仮想アカウントの SID を名前から計算する `commands::service_sid`（sc.exe showsid と同じ。TrustedInstaller の既知の SID で試験）を足し、icacls には登録の前後とも `*S-1-5-80-…` で渡す（ACL・DB の書きの外し・取り込み待ち・消す口の `/remove:g`）。install-check.sh は消した後の残りを名前と SID の両方で見る

## R32. repair の一時の postgres は `logs\db` に書けず、起動しない
- 成果物: crates/installer/src/commands.rs / main.rs
- 根拠: `repair_start_command`（commands.rs:725-742）は `logging_collector` を上書きしないので、一時の起動も `postgresql.conf` の `logging_collector=on`・`log_directory=…/logs/db` を読む。管理者の札を落とした pg_ctl の利用者は `logs\db` に RX か何も無い（commands.rs:317-344）。DatabaseWrite が書きを足すのは `db` だけ（commands.rs:387-393、main.rs:1044）。syslogger は最初のログのファイルを開けないと FATAL → `pg_ctl start -w` が失敗し、repair の Scenario（install-check.sh:813-852・:1008-1014）が全部落ちる。`db\17` の中の持ち主がサービスのファイル（WAL など）へ ACL の変更が届かないことは commands.rs:353-355 自身が前提に書いている。D31（仮）の狭めが原因
- 提案: 一時の起動に `pg_ctl start -o "-c logging_collector=off"` を渡すか、一時の `log_directory`（上がった利用者が書ける置き場）を渡す。`db\17` の中の書きの届き方は D31（仮）の反転条件と一緒に見直す
- kind: technical
- 処置: fixed 6.2 —— `repair_start_command` に `-l <settings>\repair-db.log` と `-c logging_collector=off` を渡す（一時の書きのある設定の置き場に書く）。終わりに消す

## R33. UAC を断っても終了コード 0 で「成功」になる
- 成果物: crates/installer/src/commands.rs / main.rs
- 根拠: `elevation_script`（commands.rs:259-269）の `$p = Start-Process -Verb RunAs …; exit $p.ExitCode` は、Start-Process のエラーが文を終えるだけで残りが走り、`exit $null` で 0。`relaunch_elevated`（main.rs:203-216）は Ok と読む。install・password・repair・uninstall のどれも、断られたのに成功で終わる
- 提案: 式の頭に `$ErrorActionPreference='Stop';` を置くか、`if (-not $p) { exit 1 }` を足す。試験で組んだ式にそれがあることを見る
- kind: technical
- 処置: fixed 3.1 —— `elevation_script` の頭に `$ErrorActionPreference = 'Stop'`、`exit` の前に `if (-not $p) { exit 1 }`。試験で式の形を見る

## R34. fix が人間の決めた D10・D21・D24 の中身を書き換え、D29（仮）が台帳に無かった
- 成果物: openspec/changes/st55-server-installer/design.md / provisional.jsonl
- 根拠: D10 に「役割の完了記録が無く DB を一度も起こしていない DB の置き場を自動で消して作り直す」と止まり方「DB の役割を作る前に止まった DB です」（main.rs:755-767・docs/install.md）。D21 に一時の書きの置き場を狭める「実装の注」（R32 の原因）。D24 の「どちらの zip かを各 echo の行に書く」を節の見出しに替えた。D29（仮）が `provisional.jsonl` に無い。(10) と (11) の入れ替えは D8 に押された順序で実装の注とみてよい
- kind: conflict
- 処置: fixed D30 仮 —— final の controller が、D10 の自動の作り直しを D30（仮）、D21 の狭めを D31（仮）に切り出して反転条件を書き、D29・D30・D31 を `provisional.py add` で台帳に載せた（source は D29 が R6、D30・D31 が R34）。D24 の「echo の行 → 節の見出し」は手段の変更（印の名前を spec と一字一句合わせるため）で、PR 本文に書く

## R35. 別の管理者 B が走らせ直すと、判定の前の存在の検査を通らない
- 成果物: crates/installer/src/main.rs
- 根拠: `settings_exists`・`database_exists`（main.rs:659-668）は一時の書きを足す前に `Path::exists` で見る。B は `settings`・`db` に ACE を持たないので false に見えうる → fresh として扱い、(1) の `create_dir_all` と `is_dir` で「状態の置き場を作れない」で止まる（データは失わない）。Scenario「別の利用者が走らせ直しても、本人は変わらない」「別の管理者が repair しても」が Windows の CI で落ちうる。PLAUSIBLE
- 提案: 存在の検査の前に、上がった管理者として持ち主（Administrators）の権限で一時の読みを足すか、`exists` が「見えない」と「無い」を分ける（`metadata` の PermissionDenied を fresh と読まない）
- kind: technical
- 処置: fixed 6.1 —— 存在の検査を `present()`（`fs::metadata` の NotFound だけを「無い」と読み、読めないものは「ある」）にした（install・repair・uninstall）

## R36. 書き出しの確かめが、`export` が無いと状態の置き場を見るので、印が書き出しの置き場を確かめていないことがある
- 成果物: tools/release/install-check.sh
- 根拠: install-check.sh:701-702。印「書き出しは…ほかのアカウントは読めない」（Global Constraint「印は実際の確かめにだけ置く」）
- kind: technical
- 処置: fixed 8.2 —— install-check.sh の書き出しの確かめは、API の合言葉で `POST /export` を 1 回走らせ、`$STATE/export` に zip ができるのを待ってからその置き場の ACL を見る。状態の置き場へ倒す行を消した

## R37. 決め直しの昇格を断る差し込みが、本物の昇格の経路を通らない
- 成果物: crates/installer/src/main.rs / tools/release/install-check.sh
- 根拠: `ASHIATO_SETUP_TEST_ELEVATION=deny`（main.rs:984）は password() の先頭で返り、run_command の昇格（main.rs:1164-1171）を通らない。install-check.sh:963 の印は差し込みそのものしか確かめていない（R33 も捕まえない）
- kind: technical
- 処置: fixed 7.2 —— `ELEVATION=deny` は password() の先頭で返さず、`run_command` で `denied_elevation_script`（無い実行ファイルを起こす同じ形の式）を `relaunch_elevated` に通す。断りの扱い（R33）も本物の PowerShell で通る

## R38. 途中で切れた写しが直らない
- 成果物: crates/installer/src/main.rs
- 根拠: `copy_program_files` は既にあるファイルを写さない（main.rs:538-540）。前の回の写しが途中で切れると壊れたファイルが残り続ける
- 提案: 一時の名前に写して置き換えるか、大きさ・指紋が違えば写し直す
- kind: technical
- 処置: fixed D34 仮 —— 無いファイルだけを `<名前>.copying` に写してから名前を替える（半端なファイルを本来の名前で残さない）

## R39. fix の報告に試験の出力が無い（申告の rc=0 を出力で照らせない）
- 成果物: .superpowers/sdd/st55-final/fix-report.md
- 根拠: 走らせたコマンドの名前だけで出力が無い。code-verify の席（グラフの `code_verify`）が独立に走らせ直す
- kind: technical
- 処置: rejected: code_verify の席が作業ツリー 70da46e1 で全部の検証コマンドを独立に走らせ直し、実測の rc と本数を表に書いた（`review/code.md` の code_verify 節。27 項目とも rc=0）。finish の fix の後も `cargo test --workspace`・`install-check.sh --self-test` ほかを走らせ直し、出力を PR 本文に写す

## code_verify（70da46e1。独立検証）

検証の席（`code_verify`）が、作業ツリー 70da46e1 で tasks の検証コマンドを自分で走らせ、ガードと定数と配線をわざと壊して確かめた。
壊したものは毎回元に戻し、各回のあとに `git status --porcelain` が 0 行であることを見た（壊した 13 件とも 0 行）。この節の書き足しのほかは作業ツリーを変えていない。

### 申告: 27/30（12.1〜12.3 は人間の確認待ちで `[ ]`）・検証コマンドの一覧と実測

| task | 検証コマンド（要点） | 実測 |
|---|---|---|
| 1.1 | `grep sha256=… / version=17.` pgsql.lock | rc=0・一致 |
| 1.2 | `tools/release/pgsql.sh --self-test` | rc=0・一致 |
| 2.1 | windows-gnu の clippy（server）/ `check-licenses.sh` | rc=0 / rc=0・一致 |
| 2.2 | `cargo test st55_service_`（≥6）/ `--test server_startup st55_bin_`（≥1） | rc=0 n=10 / rc=0 n=1・一致 |
| 2.3 | `cargo test st55_stop_`（≥1） | rc=0 n=1・一致 |
| 2.4 | `cargo test st55_log_`（≥3）/ `check-log-private.sh` | rc=0 n=3 / rc=0・一致 |
| 3.0 | `tools/release/sandbox-probe.sh p19` | rc=0。ただし結果は `P16=UNAVAILABLE`・`P19=UNAVAILABLE`（`.superpowers/sdd/st55-task-3/p19-result.md`）。task の文言どおりの rc=0 で、実物は見ていない（R45） |
| 3.1 | `cargo test -p ashiato-setup st55_layout_`（≥8）/ clippy 2 本 | rc=0 n=16 / rc=0 / rc=0・一致 |
| 3.2 | `install-check.sh --self-test` / actionlint | rc=0 / rc=0（手元に actionlint が無く docker の側で通った）・一致 |
| 3.3 | `check-artifacts.sh --self-test` / `package.sh --self-test` | rc=0 / rc=0・一致 |
| 3.4 | actionlint / `check-guards.sh workflows` | rc=0 / rc=0・一致 |
| 4.1 | `st55_db_`（≥3）/ `check-pg-major.sh`（と `--self-test`）/ install-check self-test | rc=0 n=12 / rc=0 / rc=0 / rc=0・一致 |
| 5.1 | `tools/db-roles.sh` / `app_role_`（≥1）/ `server_startup_refuses_privileged_role`（≥1） | rc=0 / rc=0 n=6 / rc=0 n=1・一致 |
| 5.2 | `st55_secret_`（≥5）/ self-test / `check-secrets.sh` | rc=0 n=7 / rc=0 / rc=0・一致 |
| 6.1 | `st55_rerun_`（≥4）/ self-test | rc=0 n=7 / rc=0・一致 |
| 6.2 | `st55_repair_`（≥5）/ self-test | rc=0 n=8 / rc=0・一致 |
| 7.1 | `st55_verifier_`（≥6）/ `web_session` / `check-licenses.sh` | rc=0 n=7 / rc=0 / rc=0・一致 |
| 7.2 | `st55_password_`（≥5）/ self-test | rc=0 n=8 / rc=0・一致 |
| 8.1 | `st55_inbox_`（≥3）/ self-test | rc=0 n=4 / rc=0・一致 |
| 8.2 | `st55_place_`（≥2）/ `export_tests`・`media_tests` | rc=0 n=3 / rc=0・一致 |
| 9.1 | `check-ports.sh --self-test` / `st55_port_`（≥2） | rc=0 / rc=0 n=4・一致 |
| 9.2 | `st55_notice_`（≥3） | rc=0 n=3・一致 |
| 9.3 | `st55_uninstall_`（≥2）/ self-test | rc=0 n=4 / rc=0・一致 |
| 10.1 | self-test / `install-check.sh --check-coverage …` / `check_scenarios.py . st55-server-installer` | rc=0 / rc=0（「88 Scenario の coverage」）/ rc=0（`scenarios: OK`）・一致 |
| 11.1 | install.md ほかの grep | rc=0・一致 |
| 11.2 | handoff の grep | rc=0・一致 |
| 11.3 | fmt / clippy workspace / windows-gnu clippy 2 crate / `cargo test --workspace` / check-openapi・migrations・licenses・log-private・secrets | 全部 rc=0・一致 |
| （範囲の外） | **install-windows（Windows の本物の通し。`install-check.sh` の `--list` 72 本）** | **一度も走っていない**（`gh pr list --head feat/st55-server-installer` 空・`git ls-remote origin feat/st55-server-installer` 空・`gh run list` に ST55 の run なし）。R45 |

一致したのは手元の 3 層（cargo test / clippy / `--self-test`）だけで、spec の 131 本のうち `install-check.sh --list` の 72 本は、いまのところ何にも観測されていない。

### 手ごとの結果

- 手 1: 固定値を独立に計算した。`pgsql.lock` の `sha256=d378882a…6b3e` は、EDB から zip を落として python の `hashlib.sha256` で計算した値と一致（329891687 バイト）。秘密の長さ 43（`st55_secret_values_are_unique_base64url`）は python の `base64.urlsafe_b64encode(os.urandom(32)).rstrip(b'=')` の長さと一致。照らすための値（argon2id）とログインの印の材料 `ashiato-web-session-v2` を固定値で照らす試験は無く、手元に argon2 の別の実装も無いので、そこは未確認
- 手 2: 壊したもので落ちたか —— `pgsql.lock` の版を 16 に替えると `check-pg-major.sh` rc=1（落ちた）/ `ASHIATO_RELEASE=1` で組んだ `ashiato-setup` に `ASHIATO_SETUP_TEST_` は 0 件・組まない側は 2 件（差し込みの外しは効いている）/ install-check.sh:725 の `echo '== Scenario: 決め直した…'` を消すと `--self-test` rc=1（落ちた）・`--check-coverage` rc=0（こちらは名前の表と spec を比べるだけ）/ **install-check.sh:728 の確かめ（前の合言葉で入れないこと）を `true` にしても `--self-test` rc=0**（R45）
- 手 3: `check_scenarios.py` は rc=0。印の先を見て R41〜R43 を出した
- 手 4: R44
- 手 5: `[x]` 27 件のコマンドは全部実在し rc=0（`--test server_startup` は `crates/server/tests/server_startup.rs` が実在）。3.0 は rc=0 だが中身は UNAVAILABLE（R45）
- 手 6: R40（最優先）・R46
- 手 7: 該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st55-server-installer` が `[]`）

## R40. repair の途中で落ちると、次の走らせ直しが記録の入った DB を「作りかけ」と読んで消す（サービスが残っていれば、人に消させる案内を出す）
- 成果物: crates/installer/src/main.rs / design.md（D30（仮））
- 根拠: `repair()` は `server.json.repair` → `server.json` の名前替え（main.rs:1080）を、`.installer-roles-complete`・`.installer-migration-complete` の書き（main.rs:1083-1096）より前に行う。その間で落ちる（電源・強制終了・記録の書きの失敗）と、「DB あり・設定あり・役割の完了記録なし」が残る。`install()` はこれを D30（仮）の作りかけと読み、`database_exists && !roles_done` で `init_database`（main.rs:761）→ `remove_dir_all(&db_dir)`（main.rs:765）。完了記録は `settings` に、DB は `db\17` にあり、「完了記録が無い ⇒ 記録が無い」は成り立たない。`state::decide_install` の入力は 4 つ（settings・database・verifier・services）で、`roles_done`・`migration_marker` はその外にあるので、Task 6.1 の 16 通りの表はこの分岐を覆っていない。
  実行（Linux で再現。試験の組み立ての `target/debug/ashiato-setup`、偽の `sc.exe`・`icacls`・`whoami`・`netstat`・`powershell.exe` を `PATH` の先に置き、`C:\ProgramData\ashiato\db\17` に `PG_VERSION` と `base/16384`（記録の代わり）、`settings\server.json` を置き、完了記録は置かない）:
  (a) DB のサービスが無い（`sc.exe query` が 1060）: `ASHIATO_SETUP_TEST_TRACE=1 ASHIATO_SETUP_TEST_STOP_AT=initdb ashiato-setup install …` → `phase=Continue(RegisterServices)`・`icacls C:\ProgramData\ashiato\db /grant …` の後に「initdb の段で停止しました」rc=1。**`db\17` の中は `initdb-interrupted` だけになり、`base/16384` が消えた**（差し込みが無ければそのまま `initdb` で空の DB を作って進む）。
  (b) DB のサービスがある（`STATE : 1 STOPPED`）: rc=1 で「DB の役割を作る前に止まった DB です。**記録はまだありません。**C:\ProgramData\ashiato\db\17 を消してから走らせ直してください（docs/install.md）」—— 記録のある DB に偽の案内を出し、本人に消させる。
  D30（仮）の「失わない側: 記録はまだ 1 行も入っていない」（design.md:252）は、完了記録とサービスの有無から推しているだけで、DB の中身を見ていない
  install-check.sh の「設定を退けて」（:806・:847）は `mv "$SETTINGS"`（`server.json` の 1 ファイル）で `settings` の完了記録は残るので、Windows の通しはこの経路を通らない。`repair_cleanup_commands`（commands.rs:708-716）は `postmaster.pid` を見て `pg_ctl stop` するだけで `.installer-*` を消さないので、窓は名前替え（:1080）〜記録の書き（:1087・:1094）の間に絞られる。絞っても、落ちれば記録を失う。`provisional.jsonl` の D30 の `keeps`（「記録は 1 行も無い」）も同じ推しに乗っている
- kind: irreversible
- 提案: repair は完了記録を書いてから `server.json` を置く（名前替えを最後にする）。D30 の自動の作り直しは「この回の走らせで作った DB の置き場だけ」（例: initdb の前に書く「作りかけ」の印があるときだけ）に絞り、印の無い DB は消さず、案内から「記録はまだありません」を外す。D30（仮）の反転条件（いつも止まって手で消す）に当たるかを人間に返す
- 処置: fixed D32 仮 —— repair は完了記録を書いてから `server.json.repair` を置く（名前替えを最後に）。自動の作り直しは、インストーラが initdb の前に書く印 `.installer-initdb-started` があり、DB のサービスがまだ無く移行の完了記録も無い DB だけにし（印は設定の置き場でサーバのアカウントも書けるので印だけでは消さない）、印が無ければ消さずに「記録が入っているかもしれないので…消しません。repair で…」で止まる。repair はこの形（設定あり・役割の完了記録なし）も受け付ける。案内から「記録はまだありません」を外した（docs/install.md も）。D30（仮）は台帳で resolve

## R41. インストールした形の「照らすための値が無い・元の値がある・API の合言葉と同じなら起動しない」は、起動の検査を外してもどの試験も落ちない
- 成果物: crates/server/src/lib.rs / crates/server/src/st55_tests.rs / tools/release/install-check.sh
- 根拠: lib.rs:4445-4449 の `service_mode` の検査（`check_installed_password_configuration`・`check_web_password_verifier`）を `Ok(())` に替えて `cargo test -p ashiato-server st55_` && `--test server_startup` && `web_session` → rc=0（24 passed ほか、元に戻した）。印（data-sensitivity の「インストールした形で照らすための値が無いと起動しない」「インストールした形の設定に画面の合言葉の元の値があると起動しない」「画面の合言葉が API の合言葉と同じだと起動しない」の照らす形の側）は st55_tests.rs:36-65 の関数の戻り値の試験にだけあり、spec の WHEN（設定から除いて…サーバを起動する）・THEN（起動せず、理由をログに書いて終わる）を観測していない。`install-check.sh --list` にもこの 3 本は無い（spec と `--list` の comm で st55_tests.rs だけ）ので、Windows の通しでも捕まえない
- kind: technical
- 提案: `crates/server/tests/server_startup.rs`（既に実行ファイルを起こす `st55_bin_` がある）に、`ASHIATO_SERVICE_MODE=1` で照らすための値なし / `WEB_PASSWORD` 同居 / API の合言葉から作った値 の 3 通りで起こし、rc≠0 とログの理由を見る試験を置き、印をそちらへ移す
- 処置: fixed 7.1 —— tests/server_startup.rs に `st55_bin_service_mode_refuses_*` の 3 本（実行ファイルを `ASHIATO_SERVICE_MODE=1` で起こし、0 で終わらないこと・標準エラーとサーバのログのファイルの理由・秘密が出ないことを見る）を置き、印を移した

## R42. 照らす形のログインの照合を「常に通す」に壊しても、Linux の試験は全部通る（捕まえるのは一度も走っていない Windows の通しだけ）
- 成果物: crates/server/src/web_session.rs / crates/server/src/st55_tests.rs
- 根拠: web_session.rs:342 の `|verifier| verify_password_verifier(verifier, password)` を `|_verifier| true` に替えて `cargo test -p ashiato-server st55_` && `web_session` → rc=0（元に戻した）。印「インストールした形でも、照らすための値と合う合言葉でだけログインできる」（st55_tests.rs:23）は `verify_password_verifier` を直に呼ぶだけで、ログインの口（`session_post` → `WebLogin::accepts`）も「1 文字違う求めは 401」も見ていない（試験の違う合言葉は全く別の文字列）。install-check.sh:727-728 は捕まえる形だが未実行（R45）
- kind: technical
- 提案: `WebLogin::new_with_verifier` で組んだ App に `POST /session` を送り、合う合言葉で印が出て、1 文字違うと 401 になる試験を st55_tests.rs に置き、印をそちらへ
- 処置: fixed 7.1 —— `st55_verifier_login_route_accepts_only_matching_password`（照らす形の App に `POST /session`。1 文字違い・空は 401 で印なし、合う合言葉は 204 と印）を置き、印を移した。照合を `|_verifier| true` に壊すと落ちることを確かめた

## R43. サービスの口で既定の置き場（export・media・archive-copies）を当てる配線を外しても、どこも落ちない
- 成果物: crates/server/src/main.rs / crates/server/src/st55_tests.rs / tools/release/install-check.sh
- 根拠: main.rs:23 の `apply_settings_for_service(settings_path, &settings)` を `apply_settings(&settings)` に替えて `cargo test -p ashiato-server st55_` && `--test server_startup` → rc=0（元に戻した）。印の試験（st55_tests.rs:143-183）は純粋な関数の戻り値と環境変数を見るだけで、`st55_place_media_default_is_not_based_on_service_profile` の `!service_profile/ashiato.exists()` は何も書かない試験なので、配線が無くても必ず真。Windows の側も install-check.sh:701 の `[[ -d "$export" ]] || export="$STATE"` で、書き出しの置き場が無くても状態の置き場の ACL を見て通る（final R36 のまま）。配線が無いと、サービスは `LOCALAPPDATA`（サービスのプロファイル）の下に書き出しと原本を置く（export.rs:149・media.rs:61）
- kind: technical
- 提案: R41 と同じ実行ファイルの試験で、`service` の形の起動のあとに `ASHIATO_EXPORT_DIR` の実の値を見る（あるいは install-check で書き出しを 1 回走らせ、`$STATE/export` に zip ができることを見て、fallback を消す）
- 処置: fixed 8.2 —— main.rs の service の口の準備を `service::prepare_service` に移し、試験 `st55_place_service_entry_fills_default_places_and_keeps_owner_secret_out` がそれを通して 3 つの既定の置き場を見る。配線を `apply_settings` に壊すと落ちることを確かめた。Windows の側は R36 の書き出しで見る

## R44. 仕様で決めた数のうち 4 つは、値を変えても全部通る
- 成果物: crates/server/src/service.rs / crates/installer/src/password.rs / crates/server/src/st55_tests.rs / crates/installer/src/lib.rs
- 根拠: 1 つずつ書き換えて試験を走らせた（どれも元に戻した）。
  落ちなかった: DB を待つ上限 120 秒（service.rs:405 を 5 に。`cargo test -p ashiato-server st55_` rc=0。試験は 1.1 秒・5 ミリ秒の上限を自分で渡すだけ。install-check.sh:974-985 も `DB_DELAY_SEC=20` で起こして 15 秒以上（:978 の `-ge 15`）を見るだけで、`never` は上限を見ない）/ 繋ぎ直しの間隔 1 秒（service.rs:168 を 10 に。rc=0）/ サーバのログ 30 日（service.rs:276 を 29 に。`st55_log_` rc=0。試験は 31 日前が消え 29 日前が残ることだけを見るので 29 と 30 を区別しない。install-check.sh:640 は日付のファイルが 1 つあることだけ）/ 画面の合言葉の 16 文字（password.rs:4 を 15 に。`cargo test -p ashiato-setup st55_` rc=0。試験は 14 文字の `short-password`、install-check.sh:720 は 5 文字の `short`）。
  落ちた（固定されている）: 空き 40 GB（Q7。`< 39` で `st55_notice_warns_only_below_forty_gb` FAILED）・番号 55400・管理者の役割 `ashiato`（Q10）・DB のログ 7 日（`%a` を `%d` で FAILED）・`logs\db` の書きが `ashiato-db`（Q9。`ashiato-server` で FAILED）
- kind: technical
- 提案: 境目で試す —— 15 文字を断り 16 文字を通す / 30 日前を残し 31 日前を消す / `wait_for_db` に上限を渡さず本番の定数（`DB_WAIT`・`DB_RETRY` のような名前）を試験から参照する
- 処置: fixed 2.2 —— 本番の定数 `DB_WAIT`（120 秒）・`DB_RETRY`（1 秒）・`SERVER_LOG_KEEP_DAYS`（30）を service.rs に置いて試験で固定し、ログは 30 日前が残り 31 日前が消える境目、画面の合言葉は 15 文字を断り 16 文字を通す境目で試す

## R45. Windows の本物の通し（spec の 72 本）は一度も走っておらず、Linux の self-test は確かめの本体を見ていない
- 成果物: tools/release/install-check.sh / .github/workflows/ci.yml / premises.md（P19）
- 根拠: `gh pr list --head feat/st55-server-installer --state all` 空・`git ls-remote origin feat/st55-server-installer` 空・`gh run list` に ST55 の run なし。`sandbox-probe.sh p19` の結果は `P19=UNAVAILABLE`（P19 の確かさは 1 のまま。Task 3.0 は「先に実物で見る」だった）。`self_test()`（install-check.sh:1043-）の `expect_fail` は ACL の読み・状態の番号などの部品にしか当たらず、`check_release_zip`・`check_test_zip`（:456-1026）は走らない —— :728 の確かめを `true` にしても `--self-test` rc=0（手 2）。final の指摘で本物の Windows でしか確かめられないものも、コードは変わっていない: R31（`acl_commands` の `NT SERVICE\…`（main.rs:711）が (5) の登録より前。`grep -n showsid crates/installer/src/*.rs` 0 件）・R33（commands.rs:261 の式に `$ErrorActionPreference` も `-not $p` も無い）・R35（存在の検査は `Path::exists` のまま。`PermissionDenied` の扱い 0 件）
- kind: premise
- 提案: Global Constraint どおり PR の CI に委ねてよいが、finish は「27/27 rc=0」を 131 本の担保と読まない。merge の前に install-windows が少なくとも 1 回緑になること（R31・R35 で初回は赤の見込み）を merge gate で見る
- 処置: rejected: 主張（Windows の通しはまだ一度も走っていない・Linux の self-test は確かめの本体を見ない）はそのとおりで、提案の「merge の前に install-windows が緑」は機械が持っている —— merge は `merge_gate.sh` が head の CI が全部緑であることを見てからしか進まず（`.harness2/docs/flow-gates.md:194`）、この PR は `crates/` を変えるので install-windows が走る。PR 本文は「27/27 rc=0」を Windows の担保と書かず、初回の install-windows の赤を見込むと書く

## R46. 移行だけ・画面だけの PR では install-windows が走らない
- 成果物: .github/workflows/ci.yml
- 根拠: ci.yml:52 の正規表現 `^(crates/|Cargo\.)|^\.github/|^tools/release/|^tools/db-roles\.` を手元で当てると、`migrations/ORDER`・`migrations/202611040000_x.sql`・`web/src/App.tsx` はどれも `windows=false`（`crates/server/src/lib.rs` は true）。`crates/server/build.rs:24` は `../../migrations` を実行ファイルに埋め込み、サーバの zip は画面も持つので、移行だけの PR でも配る形は変わるのに、「配る形に移行を当ててサーバが起きる」「移行の前の退避は状態の置き場に、同梱の pg_dump で取られる」を Windows で見ない
- kind: technical
- 提案: install-windows の出し分けに `^migrations/` と `^web/` を足す（package-windows は `code` で既に走るので、増えるのは install-windows の 1 job）
- 処置: fixed 3.4 —— ci.yml の windows の出し分けに `^migrations/` と `^web/` を足した

## security review（6a8d707..70da46e）

範囲は `git diff 6a8d707b..70da46e1`（first-parent。ST44 の取り込み 6a8d707 より後の ST55 の分だけ）。

見た経路:
- 画面のログイン `POST /session` → `session_post`（web_session.rs:508-555）→ `WebLogin::accepts`（:339）→ 照らすための値があれば `verify_password_verifier` だけ（Argon2id）。空の入力も照らす値と照合するだけで通らない。印の材料は `secret_tag_for_verifier`（鍵 + 照らす値）（3）
- ルータの口: ST55 はルータに口を足していない（差分の `.route(`/`.nest(`/`.merge(`/`.fallback(` が 0 件）。新しい入口は `ashiato-server service --settings`（SCM からだけ）とインストーラの CLI だけ（4）
- サービスの起動: `server.json` → `load_settings`（service.rs:84）→ 全部の鍵を環境へ（:104-107）→ `run_inner` の起動の検査（lib.rs:4440-）。待ち受けは設定の `BIND=127.0.0.1:18700` を `net_guard::check_bind` に通す。DB は `listen_addresses='127.0.0.1'`・`scram-sha-256`（db.rs:33・:20）（3）
- 秘密の生成と置き場: `OsRng` 32 バイト → base64url（secrets.rs:126-131）→ `write_settings` は ACL を絞った後（main.rs:688-718。一時の `.initdb.pwfile` も同じ置き場で、Drop で消す :449-462）。DB の役割の合言葉は psql に環境変数と標準入力で渡し、引数に載せない（main.rs:572-584）（3）
- ログ: サービスの tracing は `logs\server` のファイルへ（service.rs:389-400）。要求の URL を書く層（`TraceLayer`）は無い（`grep TraceLayer` 0 件）。DB のログは `log_statement='none'`・`log_error_verbosity='terse'`・`log_min_error_statement='panic'`（db.rs:33。文の字面と DETAIL の値を書かない）（3）
- 取り込み待ちとダウンロード: インストーラ（昇格）が本人のプロファイルの下に作る → `ensure_real_directory`（main.rs:466）でリンクを断る → `icacls … /L`（commands.rs:142-150。リンクの行き先に権限を付けない）（3）
- 同梱の PostgreSQL: `pgsql.sh fetch` は取ってきたものも cache から戻したものも `sha256` を照らす（pgsql.sh:26-31・:38-43）。release.yml の `restore-keys: pgsql-` で別の版の cache が戻っても、指紋が違えば組まない（`pgsql.sh --self-test` の「指紋が違えば組まない」）（4）
- 消す口: 昇格した子の `uninstall()` → `temp_dir()` に自分の写し → それを起こす → 写しが `temp_dir()` に `.cmd` を書いて `cmd.exe` で起こす（main.rs:890-910・commands.rs:159-179）（4。下の R47）
- 依存: `argon2 0.5.3`・`password-hash 0.5.0`・`blake2 0.10.6`（RustCrypto）、`rpassword 7.5.4`・`rtoolbox 0.0.6`、`windows-service 0.7.0`・`widestring 1.2.1`。どれも crates.io の版を Cargo.lock で固定、名前の取り違えなし（2）

安全の決め手: **ST55 は画面と API の入口を足しておらず、認証の口は既存の middleware のまま** —— 確かさ 4。`git diff 6a8d707b..HEAD -- crates/server/src | grep -E '^\+.*\.(route|nest|merge|fallback)\('` → 0 件。
変わったのはログインの照合の中身だけ（照らすための値で照らす）で、こちらは 3 止まり（`POST /session` を照らす形の App に送る試験が無い。R42 と同じ欠け。未証明）。

## R47. 消す口は、管理者の権限のまま本人が書ける一時フォルダに exe と .cmd を置いて起こすので、本人の権限で動くプログラムが管理者の権限を取れる
- 成果物: crates/installer/src/main.rs / crates/installer/src/commands.rs
- 根拠: `uninstall` は先に `run_command` が管理者として起こし直し（main.rs:1164-1170。`uninstall_elevation_command`）、上がった子の `uninstall()` が `std::env::temp_dir().join("ashiato-setup-uninstall.exe")` へ写して起こす（main.rs:895-903）。写しは `executable.with_extension("cmd")`（同じ一時フォルダ）に `Set-Content` で `.cmd` を書き、`Start-Process cmd.exe` で起こす（commands.rs:159・:173-179）。中身は `ping 127.0.0.1 -n 3` で 3 秒待ってから `rmdir /s /q "C:\Program Files\ashiato"`。
  実行（Linux。試験の組み立ての `target/debug/ashiato-setup`、`PATH` の先に偽の `whoami`・`sc.exe`・`reg.exe`・`powershell.exe`、`TMPDIR=<一時>`、`USERPROFILE='C:\Users\alice'`。Linux では `is_elevated()` が真なので、上がった子と同じ道を通る）:
  `ASHIATO_SETUP_TEST_TRACE=1 ashiato-setup uninstall` → rc=0。一時フォルダに `ashiato-setup-uninstall.exe`（13526176 バイト）ができ、それが起こされて `command=uninstall` を 2 回出し、`powershell.exe … Set-Content -LiteralPath '<TMPDIR>/ashiato-setup-uninstall.cmd' … Start-Process cmd.exe -ArgumentList '/c', '"<TMPDIR>/ashiato-setup-uninstall.cmd"'` を組んだ（確かさ 4：置き場と起こす順）。
  Windows で同じ利用者が管理者の確認で上がるとき（家の PC の普通の形）、`Start-Process -Verb RunAs` の子は本人の環境を引き継ぐので `temp_dir()` は本人の `%LOCALAPPDATA%\Temp` になり、本人の権限（中の整合性）で動くプログラムが書ける。写しを起こす前に exe を差し替えるか、`cmd.exe` が 3 秒の `ping` の後に読み進める `.cmd` の後ろの行を書き換えると、その命令が管理者で動く（確かさ 2：Windows の振る舞いは走らせていない）
- kind: technical
- 提案: 写しと `.cmd` は、管理者だけが書ける置き場（例: `%ProgramData%\ashiato-uninstall\` を作って継承を切り Administrators と SYSTEM だけにする、または `%SystemRoot%\Temp`）に置く。`.cmd` を使わず、写しの側で親の終わりを待って自分で `Program Files\ashiato` を消す形にすれば、読み直される台本が残らない
- 処置: fixed 9.3 —— 写しと遅れて消す台本は `C:\ProgramData\ashiato-uninstall-<pid>-<時刻>` に置く（`create_dir` で新しく作り、継承を切って Administrators と SYSTEM だけにし、空であることを確かめ、写しは `create_new`）。台本は最後に置き場ごと消す

## R48. サーバのサービスのプロセスが、使わない DB の所有者の合言葉（`DATABASE_OWNER_URL`）を環境に持つ
- 成果物: crates/installer/src/secrets.rs / crates/server/src/service.rs / design.md（D4）
- 根拠: インストーラは `DATABASE_OWNER_URL`（表の所有者 `ashiato_owner` の合言葉入り）を `server.json` に置き（secrets.rs:46）、サービスの口は設定の鍵を全部そのまま環境へ入れる（main.rs:23 → service.rs:104-107）。サーバの起動（`run_inner`）はこの鍵を読まず、読むのは `run_migrate`（lib.rs:374）だけ。ST28 の分け方（実行時は `ashiato_app`。lib.rs:296 の「サーバの起動は所有者を呼ばない」）は DB の接続では保たれるが、サーバのプロセスを乗っ取られたとき（あるいはサーバのアカウントで設定を読めたとき）に、表の所有者として触発器（削除の門）を外せる資格が同じプロセスにある。`server.json` はサーバのアカウントが書ける根の置き場から受け継ぐ（commands.rs:328-343）ので、ファイルの側でも分かれていない（確かさ 3）
- kind: premise
- 提案: 少なくとも `apply_settings_for_service` で `DATABASE_OWNER_URL` を環境へ入れない（サーバの起動では要らない）。所有者の合言葉を移行のとき（インストーラ・ST58 の更新）だけ読む別のファイル（サーバのアカウントに読みを与えない）に分けるかは、D4 の決め直しとして人間に返す
- 処置: escalated —— deep.md Q11 で人間に返し、答えが出た。本人の答え（分ける。2026-11-04）を D4・D27 と spec（distribution の Scenario「表の所有者の合言葉は、サーバのサービスが読めないファイルにだけある」）に写し、インストーラは `settings\owner.json`（継承を切って Administrators と SYSTEM の F だけ・持ち主も Administrators、書きかけは先に管理者と SYSTEM だけにした新しいフォルダ `settings\.owner-staging-<印>` の中に作る —— 直に作って後から ACL を絞ると、絞る前に開かれた handle で中身を読める、と commit 後のセキュリティの hook が指摘）に置き、`server.json` から鍵を外した（secrets.rs `owner_settings_json`・main.rs `stage_owner_settings`・`place_owner_settings`）。試験は `st55_owner_settings_are_only_for_administrators_and_system`・`st55_secret_settings_have_api_and_session_keys`・`st55_rerun_reads_existing_database_secrets_from_settings` と install-check.sh の同名の Scenario

## 範囲外
- 移行の前の退避（`backups\pre-migrate` の `pg_dump`）は消した記録を含んだまま残る。ST55 はその既定の置き場を状態の置き場へ移しただけで、退避の寿命と削除の経路（ST23）は前からのもの

## /code-review（1941853..70da46e）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R49. The script runs `mkdir -p "$stage/pgsql"` and then `cp -a "$pgsql_dir" "$stage/pgsql"`. Because the destination folder already exists, cp copies the slimmed PostgreSQL into it as a subfolder, `pgsql/.pgsql-slim/`, so the zip has no `pgsql/bin/postgres.exe`.
- 成果物: tools/release/build.sh:65
- 根拠: `package.sh windows` builds a zip laid out as `pgsql/.pgsql-slim/bin/postgres.exe`. `check-artifacts.sh` stops with "Windows の zip に PostgreSQL が無い". The installer's `bundled_tool` (`C:\Program Files\ashiato\pgsql\bin\initdb.exe`/`pg_ctl.exe`/`pg_isready.exe`) does not exist, so every install fails at initdb. I reproduced the cp behaviour locally. CI has never run install-windows on this branch (review/code.md R45), so nothing has caught this.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.3 —— `cp -a "$pgsql_dir/." "$stage/pgsql/"` にし、写した後に `pgsql/bin/postgres.exe` が無ければ止まる

## R50. `copy_program_files` never overwrites a file that already exists. A rerun from a newer zip keeps the old `ashiato-server.exe`, `ashiato-setup.exe` and pgsql binaries, but step (12) still writes the new `DisplayVersion`.
- 成果物: crates/installer/src/main.rs:538
- 根拠: Install 0.4.0, then run `ashiato-setup.exe` from the 0.5.0 zip. `to.exists()` skips every binary, so the 0.4.0 server keeps running against the old schema. "Apps & features" shows 0.5.0. Any new migration is applied by the old `ashiato-server.exe migrate` (or skipped because the marker exists). A half-copied file left by an interrupted first copy is also never repaired.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D34 仮 —— 既にあるファイルが zip のものと違えば、何も替えず「別の版が入っています。版を替える口はまだ無い」で止まる（DisplayVersion も書かない）

## R51. `stop_service_and_wait_command` only stops a service whose status is exactly `Running`, and the loop that waits for `Stopped` has no timeout.
- 成果物: crates/installer/src/commands.rs:718
- 根拠: `password` runs while ashiato-server is `StartPending` (e.g. after a reboot it is still inside the 120 s DB wait, already holding the old verifier it read at process start). The stop is skipped, the new verifier is written, and then `sc start` fails with 1056 (already running). The command reports failure and the running server keeps the old password. Separately, if the service hangs in `StopPending`, `install`/`password`/`repair`/`uninstall` loop forever.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.2 —— `stop_service_and_wait_command` は StartPending なら Running になるのを待ってから止め、`SERVICE_STOP_TIMEOUT_SECS`（180 秒）を過ぎれば失敗で終わる

## R52. In `repair`, when `run_psql` (or anything else) fails after `repair_start_command`, the error returns without stopping the temporary postmaster. That postmaster is still running with the SSPI-trusting `repair.pg_hba.conf` on a loopback port.
- 成果物: crates/installer/src/main.rs:1072
- 根拠: The role SQL fails during `repair` (bad account mapping, psql error). `?` returns, and the Drop guards only reset ACLs. The temporary postgres keeps running and accepts SSPI logins as the DB superuser from the repairing admin. Its `postmaster.pid` also makes any later `sc start ashiato-db` fail. Only another `repair` (via `repair_cleanup_commands`) stops it; running `install` or starting the service by hand does not.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 6.2 —— 一時の DB を起こした後は `TemporaryPostmaster`（Drop で `pg_ctl stop`）が持ち、途中で返っても止める

## R53. The service accepts only `STOP`; it does not accept `SHUTDOWN`/`PRESHUTDOWN`, and it answers `Interrogate` with `NotImplemented`.
- 成果物: crates/winsvc/src/lib.rs:59
- 根拠: At OS shutdown or restart (e.g. Windows Update), the SCM sends no stop to ashiato-server, and the process is killed while an export is running. `exports.stop()` and the `.partial` cleanup never run, so partial export files are left in the state folder. An `Interrogate` from the SCM gets an error instead of the current status.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.2 —— winsvc は STOP・PRESHUTDOWN・SHUTDOWN を受けて同じ止め方を通し、Interrogate には NoError を返す

## R54. `request_stop` deletes the dot-prefixed partial files in `ASHIATO_EXPORT_DIR` immediately on the SCM thread, while the export task is still running. The same cleanup is already done after graceful shutdown.
- 成果物: crates/server/src/service.rs:232
- 根拠: A stop arrives during an export. The handler deletes the export's in-progress file (on Windows this hits a sharing violation, which is ignored, or the file becomes delete-pending). The still-running export then fails or writes into an unlinked file before `exports.stop()` aborts it. The early cleanup is racy and redundant with the one in `run_inner`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.3 —— `request_stop` は旗を立てるだけにし、書きかけの片付けは書き出しを止めた後の `run_inner` の 1 回だけにした

## R55. `build_installer` takes `ASHIATO_RELEASE` from the caller's environment (`${ASHIATO_RELEASE:-}`), but `build_server` forces `ASHIATO_RELEASE=1` for every non-test build. Running `build.sh server-windows` directly therefore gives a release server with a test-hook installer.
- 成果物: tools/release/build.sh:94
- 根拠: A developer, or an old workflow, runs `tools/release/build.sh server-windows out` without `ASHIATO_RELEASE=1`. `ashiato-setup.exe` is compiled with `cfg(ashiato_test_build)` and reads `ASHIATO_SETUP_TEST_*` (FREE_GB, STOP_AT, DOWNLOADS_DIR…), while the bundled server is the release build. The zip mixes the two kinds of build and is only caught if `check-artifacts.sh` is run afterwards.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.3 —— `build_installer` に同梱するサーバと同じ `server_release` を渡す（呼び手の環境の ASHIATO_RELEASE を読まない）

## R56. The windows release path no longer runs `env -u ASHIATO_BUILD_VERSION`, while the linux/apk paths still unset it. That contradicts the comment "版番号は build.sh が出所から解決する".
- 成果物: tools/release/package.sh:23
- 根拠: `package.sh windows` and `package.sh linux` run in a shell where `ASHIATO_BUILD_VERSION` is set (e.g. left over from a release step). The Windows zip gets that version while the Linux tarball and APK get the source version. `check-artifacts.sh --expect-version` then rejects one of them, or the release mixes versions.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.3 —— Windows の配る経路も `env -u ASHIATO_BUILD_VERSION` で build.sh を呼び、package.sh の自己テストに Windows の場合を足した

## R57. `check-pg-major.sh` is never invoked by CI or any script. It is only listed as an exempt Scenario source. It also reads the compose path from `COMPOSE_FILE`, which `env-init.sh` writes into `.env` as a colon-separated docker compose list.
- 成果物: tools/check-pg-major.sh:6
- 根拠: If docker-compose moves to `postgres:18` while `pgsql.lock` stays at 17.x, nothing fails: install-check hard-codes `17`. When run with `.env` sourced (`set -a; . ./.env`), `COMPOSE_FILE=docker-compose.yml:docker-compose.subnet.yml` makes `[[ -f ]]` fail with "docker-compose.yml が無い".（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.1 —— ci.yml の rust の job で `check-pg-major.sh --self-test` と本体を走らせ、compose の場所は `PG_MAJOR_COMPOSE_FILE` で差し替える（COMPOSE_FILE の並びを読まない。自己テストで見る）

## R58. `repair_ident_contents` wraps every non-alphanumeric character as `[c]`. For `^` this produces `[^]`, which is an invalid regex in PostgreSQL ARE syntax (a negated empty bracket).
- 成果物: crates/installer/src/commands.rs:766
- 根拠: An admin whose Windows account name contains `^` (allowed in Windows user names) runs `repair`. The ident map regex fails to compile, the SSPI login is rejected, `run_psql` fails, and repair cannot finish. This also triggers the previous finding: the temporary postmaster is left running.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 6.2 —— 英数字以外は ARE の `\uXXXX`（BMP の外は `\UXXXXXXXX`）にした。`^` が `[^]` にならず、空白も行の区切りにならない（試験で見る）

## finish の scoped re-review（70da46e1..9c41a46d）

finish の 1 回の fix の後の scoped re-review。R28・R40 は PARTIALLY、ほかの直した指摘は ADDRESSED。fix が新しく持ち込んだ・見落としたものは次の 3 つで、
**fix の機会は finish で使い切った**ので `.superpowers/sdd/st55-final/progress.md` に Ruling つきで park した（Q11 の答えの後にグラフが finish を呼び直すときに直す）。
- N1: Windows の runner（msvc・crt-static なし）で組む `ashiato-setup.exe`・`ashiato-server.exe` も VCRUNTIME140.dll を読む（R28 の残り。D33 の「Rust の 2 つは要らない」は誤り）
- N2: R40 で開けた repair の出口で、repair が `.installer-migration-complete` を無条件に書き、移行していない DB で install が移行を飛ばす
- N3: 消す口の写しの作成に失敗すると `C:\ProgramData\ashiato-uninstall-*` が残る

処置（Q11 の答えの後の 2 回目の finish。park を解いて直した）:
- N1: fixed D33 仮 —— `.cargo/config.toml` で `x86_64-pc-windows-msvc` に `+crt-static`。`check-artifacts.sh` は根の 2 つが `vcruntime*/msvcp*` の DLL を読む zip を落とし、self-test は DLL の名前を足した `ashiato-setup.exe` で落ちることを見る。D33 の「Rust の 2 つは要らない」を直した
- N2: fixed D32 仮 —— `repair` は移行の完了記録を書かない（あればそのまま、無ければ走らせ直しの (8) が移行を当てる）
- N3: fixed 9.3 —— 写しの置き場の ACL・中身の確かめ・写しの作成・起こすのが失敗したら、作った置き場を消す（リンクに替わっていればリンクだけ）

## 2 回目の finish の scoped re-review（d8a3a082..86cd0730）

Q11 の答えを写した fix と N1〜N3 の直しの後の scoped re-review（pr-review-toolkit:code-reviewer、別の文脈）。R48（Q11）・N1・N2 は ADDRESSED、N3 は PARTIALLY。
**fix の機会はこの finish で使い切った**ので、次の 3 つは `.superpowers/sdd/st55-final/progress.md` に Ruling つきで park し、PR 本文で人間に渡す（A ではない。どれも記録を失わない）。
- N4: 写しは起きたが遅れて消す台本を起こす前に失敗すると（サービスが 180 秒で止まらない など）、`C:\ProgramData\ashiato-uninstall-*`（写しと `.cmd`。管理者と SYSTEM だけ）が残る（main.rs の `uninstall` の親。確かさ 3）。直し方: 親で写しが非 0 のときも置き場を消す
- N5: `place_owner_settings` で rename の後の `icacls` が失敗すると、owner.json は受け継いだ項目のまま保護されず、`TemporaryWrite` の Drop の `/reset` で設定の置き場の形（サーバの M・本人の RX）を受け継ぎ直しうる（確かさ 2。icacls が失敗するときだけ）。直し方: 書きかけの置き場の中で rename の前に保護の ACL と持ち主を付ける
- N6: repair の R40 の出口（server.json があり役割の完了記録が無い）で、owner.json を置いた後・server.json を置く前に落ちると、新しい owner.json と古い server.json が残り、`repair_allowed` が偽で repair をやり直せない（確かさ 2。前からある記録と rename の間の窓に、外の命令の段が入った）

## final review（0548aad..45825ea）

squash（19c44ef28）と Task 13（D35 の DB の見張り）の後の whole-branch review（`superpowers:requesting-code-review` の `code-reviewer.md`、別の文脈）。判定は **With fixes**（Critical なし）。
N1・N2 は HEAD でも直っている。N4〜N6 は HEAD でもそのまま（前の Ruling どおり park のまま PR 本文で渡す）。I2・I3 は final の controller が該当行を読んで再現した。

## R59. Task 13（HEAD 45825ea）は Windows の CI を一度も通っていない
- 成果物: crates/server/src/service.rs / crates/winsvc/src/lib.rs / crates/installer/src/commands.rs / tools/release/install-check.sh
- 根拠: `origin/feat/st55-server-installer` は 90245585e のまま。最後の緑の install-windows（run 42235492040）は Task 13 の前の木。`sc.exe sdshow | Select-Object -Last 1` の SDDL の取り方・`sdset`（commands.rs の見張りの ACE）、見張りが本当に StartService を呼ぶか、主だけを殺す DB の回復の Scenario（install-check.sh:678-690）は Linux では確かめられない
- kind: premise
- 提案: push して install-windows が HEAD で緑になるのを待つ
- 処置: rejected: 主張（HEAD は Windows の通しを一度も通っていない）はそのとおりで、提案の「push して HEAD で緑」は機械が持っている —— push と PR はグラフの `publish`、merge は `merge_gate.sh` が head の CI が全部緑であることを見てからしか進まない（R45 と同じ）。この PR は `crates/` を変えるので install-windows が走る。PR 本文は Task 13 と R60〜R62 の Windows での振る舞いを未観測と書く

## R60. 設定を拒んだサーバが `failureflag 1` で 10 秒ごとに起こし直され続ける（R21 の直しが打ち消されている）
- 成果物: crates/installer/src/commands.rs / crates/server/src/main.rs / crates/server/src/service.rs / design.md（D11）
- 根拠: commands.rs:790-808 は両方のサービスに `failure … restart/10000 ×3` と `failureflag 1`。main.rs:44-63 の `run_windows_service` は設定の拒否（`SETTINGS_REFUSED`）・ランタイムの失敗・`run_service` のどの失敗（web_password・db_role・db_version・bind の拒否、DB 待ちの時間切れ）でも 1 を返す。`failureflag 1` では「STOPPED かつ終了コード非 0」で回復が動くので、起こし直しても直らない拒否が 10 秒ごとに繰り返され、サーバの依る `ashiato-db` も起こされる（repair が DB を止めた後の一時の postmaster と取り合いうる）。main.rs:21-23 のコメント（R21: 回復で起こし直しが続くのを防ぐ）と食い違う。spec の distribution:71 が回復に任せるのは「120 秒待っても DB に繋げない」だけ
- kind: technical
- 提案: 起こし直しても直らない拒否（設定・合言葉・役割・版・bind）は終了コード 0 で SERVICE_STOPPED を返し、DB 待ちの時間切れ（と runtime の失敗）だけ非 0 にする。試験で拒否の種類ごとの終了コードを固定する
- 処置: fixed 2.2 —— 拒否を型（`service::StartupRefused`）で返し、`service::ServiceEnd` が SCM への終了コードを決める。設定のファイルの拒否・設定の値の誤り（DATABASE_URL・API_TOKEN・期限・書庫・warmup。service の口だけ `refuse_setting` で型を付ける）・合言葉・鍵・bind・接続先・役割・版の拒否は 0、DB の時間切れ（`DatabaseUnavailable`）・実行環境の失敗・型の無い失敗は 1（`reset= 86400` で 1 日 3 回まで）。R21 のコメントを直し、`st55_service_unrecoverable_refusals_end_with_exit_code_zero`・`st55_service_database_timeout_and_other_failures_end_non_zero` で固定した。e4463b16f の `ensure_running` は終了コードに依らず冪等で正しいので残した

## R61. 正式版の配布（release.yml）で、Windows の成果物の版番号が tag の版でなく Cargo.toml の版になる
- 成果物: .github/workflows/release.yml / tools/release/package.sh
- 根拠: release.yml:63-69 は `package.sh windows` を `ASHIATO_BUILD_VERSION=<plan の版>` で呼ぶが、package.sh:19-25 は（R56 の直しで）`env -u ASHIATO_BUILD_VERSION` で build.sh を呼ぶので、build.sh:16-20 は `version.sh source` に倒れる。Linux の job は build.sh を直に呼び plan の版のまま。beta の tag（`v1.2.3-beta.1`）では publish の `check-artifacts.sh --expect-version` が Windows の zip で落ちる。main では release.yml は build.sh を直に呼んでいた（この枝で package.sh に替えた）
- kind: technical
- 提案: release.yml は build.sh を直に呼ぶ形に戻し（`ASHIATO_RELEASE=1`・`PGSQL_CACHE_DIR` を渡す）、server-windows と collector-windows の 2 本にする。または package.sh に版を渡す口を足す
- 処置: fixed 3.4 —— release.yml の Windows を `build.sh server-windows`・`build.sh collector-windows` の 2 本にし、Linux の job と同じく plan の版・`ASHIATO_RELEASE=1`・`PGSQL_CACHE_DIR`・`ASHIATO_SOURCE_REF` を渡す。package.sh は R56 のまま（PR の組み立てで残った版を外す）。proposal.md:66・tasks.md 3.4 の「release.yml も同じ package.sh を通る」はもう成り立たない（上流の文は書き換えていない）。actionlint は手元に無く、`check-guards.sh workflows`・`--self-test` が rc=0

## R62. DB の回復の Scenario が、何が DB を起こし直したか（D35 の見張りか）を確かめない
- 成果物: tools/release/install-check.sh
- 根拠: install-check.sh:678-690 は Task 13 で service_pid の比較を消し、`wait_state ashiato-db 4 60`・`wait_database 60`・ACL だけを見る。pg_ctl が主の死にまだ気づいていなければ状態 4 はすぐ通る。spec の Scenario を D35 で満たした仕組み（見張りの起こし直し）を固定していない
- kind: technical
- 提案: 主を終えた後に、サーバのログに `db_service_restarted` が書かれるのを待って確かめる
- 処置: fixed 13.1 —— 主を終える前にサーバのログの `db_service_restarted` の行数を控え、終えた後に 60 秒まで増えるのを待つ（`wait_db_restarted_by_server`。増えなければ診断を出して落ちる）。Windows の実地（install-windows）はまだ走らせていない（R59）

## R63. design.md D11 の回復の動作が実装と違う
- 成果物: openspec/changes/st55-server-installer/design.md
- 根拠: D11 は `restart/10000/restart/10000/restart/60000`、commands.rs:803 は `restart/10000` ×3（理由はコードのコメントだけ）。`failureflag 1` も D11 に無い
- kind: technical
- 提案: D11 に実装の注として 3 回とも 10 秒・`failureflag 1`・R60 の終了コードの方針を書く
- 処置: fixed D11 —— D11 に実装の注として、回復は 3 回とも 10 秒（理由は commands.rs のコメント）・両方のサービスに `failureflag 1`・R60 の終了コードの方針を書いた。決定の本文は変えていない

## R64. 見張りが塞ぐ呼び出し `start_db_service` を async の中で直に呼ぶ
- 成果物: crates/server/src/service.rs
- 根拠: service.rs:91 付近。問い合わせは spawn_blocking なのに起こしは直（Task 13 F6）
- kind: technical
- 処置: fixed 13.1 —— `start_db_service` も `spawn_blocking` で呼び、JoinError は失敗の理由にまとめる

## R65. 見張りの `db_service_start_failed` が同じ理由で 5 秒ごとに書かれる・`last_query_error` が成功で戻らない
- 成果物: crates/server/src/service.rs
- 根拠: service.rs:93 は理由が同じでも毎回書く（DB のサービスを無効にした場合など）。:99 の `last_query_error` は問い合わせが成功しても None に戻らないので、回復の後に同じ理由が再発しても書かれない（Task 13 F13）
- kind: technical
- 処置: fixed 13.1 —— `service::DbWatchErrorLog`（理由が変わったときだけ書く・成功で忘れる）を問い合わせと起こしの両方に使う。`st55_db_watch_error_log_writes_changes_and_forgets_on_success` で固定した

## R66. DB のサービス名を設定の置き場のパスから推している
- 成果物: crates/server/src/service.rs / crates/installer/src/secrets.rs
- 根拠: service.rs:49-55 の `db_service_name_from_settings_path` はパスに `ashiato-dev` を含むかで決める。インストーラが `server.json` に `ASHIATO_DB_SERVICE` を書けば推さずに済む
- kind: technical
- 処置: fixed 5.2 —— インストーラが `server.json` に `ASHIATO_DB_SERVICE`（`layout.db_service`。開発版は `ashiato-db-dev`）を書き、サーバはそれを読む。置き場のパスから推す `db_service_name_from_settings_path` を消した（鍵が無ければ `ashiato-db`）。installer の鍵の一覧の試験と、`st55_db_watch_settings_name_reaches_the_service_environment` で固定した

## R67. 拒否が 2 行書かれる
- 成果物: crates/server/src/main.rs
- 根拠: main.rs:57-58 は `run_service` が既に理由（`database_unavailable` など）を書いた後に、もう一度 `service_startup` を書く
- kind: technical
- 処置: fixed 2.2 —— `ServiceEnd::needs_log` が真（型の無い失敗）のときだけ `kind=service_startup` を書く。拒否と DB の時間切れは返す前に書いた 1 行だけ

## R68. server/tests/server_startup.rs の `#![cfg(unix)]` の理由が無い・server と winsvc の Windows 向け clippy が CI に無い
- 成果物: crates/server/tests/server_startup.rs / .github/workflows/ci.yml
- 根拠: server_startup.rs:5。ci.yml:157 の Windows 向け clippy は collector-windows だけで、winsvc と service の `cfg(windows)` の中身は package-windows のビルドでしか型を見ていない
- kind: technical
- 処置: fixed 3.4 —— server_startup.rs の `#![cfg(unix)]` に理由（unix の権限で偽の pg_dump を作る・部品の一部が cfg(unix) で Windows の target では組めない。ea3988d80 で足した）を書いた。ci.yml の collector-windows の job に `cargo clippy -p ashiato-server -p ashiato-winsvc -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` を足した。手元で走らせると installer の試験の `unwrap`（lib.rs の見張りの ACE の試験）で落ちたので、それも直した

## R69. pgsql.sh の `A && cp || B && cp || die` の連鎖（Task 1 F2）
- 成果物: tools/release/pgsql.sh
- 根拠: pgsql.sh:70-72。今は害が無いが、`cp` の失敗が次の枝に流れる
- kind: technical
- 処置: fixed 1.2 —— 許諾の文の写しを if/elif/else にし、`cp` の失敗はその場で落ちる

## R70. 同梱の DLL に `vcruntime140_1.dll` が要らないことを実物で確かめていない
- 成果物: tools/release/pgsql.lock / tools/release/build.sh
- 根拠: D33 の objdump の結果は VCRUNTIME140・MSVCP140 だけを挙げる。MSVC x64 の C++（ICU・libxml2）は `vcruntime140_1.dll` を読むことがある
- kind: technical
- 処置: fixed 1.1 —— 要らないことを実物で確かめた。指紋が pgsql.lock と合う EDB の zip（sha256 d378882a…）を展開し、`x86_64-w64-mingw32-objdump -p` を 7 つの実行ファイル・lock の 12 の DLL・`lib/*.dll` の 89 個に当てると、VCRUNTIME140.dll（MSVCP140.dll は icu*）だけ。`VCRUNTIME140_1.dll` を読むのは pgAdmin・StackBuilder の `wx*.dll`・`stackbuilder.exe` だけで、slim は写さない。結果を pgsql.lock のコメントに残した（DLL の追加は無し）

## final review の scoped re-review（45825ea..a185e38）

fix の後の scoped re-review（SDD の `re-review-prompt.md`、別の文脈）。R60〜R70 は全部 ADDRESSED、fix が持ち込んだ Critical / Important は無し。R59 は範囲外（上の処置）。
- Minor（処置しない）: install-check.sh の `db_restart_count` はログのファイル全体の行数を数える。日付でファイルが替わる 60 秒の窓だけ数え方がずれうるが、増えたかを見るだけなので実害は無い
- PR 本文へ: `proposal.md:66` と `tasks.md` 3.4 の「release.yml も同じ package.sh を通る」は R61 の直しで成り立たなくなった（上流の文は書き換えていない）
- 本物の Windows でしか確かめられない: 終了コード 0 / 1 で SCM の回復が止まる・動くか（R60）、見張りの `spawn_blocking` の起こし（R64）、`db_service_restarted` の待ち（R62）、新しい Windows 向け clippy の step（R68）。release.yml（R61）は tag を打つまで走らない

## code_verify（63b74d22b。独立検証・2 回目）

検証の席（`code_verify`）が、final review（R59〜R70）の直しの後の作業ツリー 63b74d22b で、tasks の検証コマンドを自分で走らせ、配線と定数をわざと壊して確かめた。
壊す確かめのうち M2・M4 は使い捨ての worktree（`git worktree add --detach` で HEAD。終わりに `git worktree remove`）、M1 は本体の作業ツリーで行い、戻した後の `git status --porcelain` は 0 行・`git diff` は 0 バイト（壊す前と同じ）。M3 は install-check.sh の写し（git が追わない `.superpowers/sdd/tmp/` の下）で行った。この節の書き足しのほかは作業ツリーを変えていない。
初めに M1 を worktree で走らせたときは testdb.rs:322・:332 の panic（worktree の DB の番号が本体と違い、所有者で繋げない）で 2 本落ちたので、結果に使わず、本体で単独に走らせ直した（下の M1）。

### 申告: 30/33（12.1〜12.3 は人間の確認待ちで `[ ]`）・検証コマンドの一覧と実測

| task | 検証コマンド（要点） | 実測 |
|---|---|---|
| 1.1 | `grep sha256=… / version=17.` pgsql.lock | rc=0・一致 |
| 1.2 | `pgsql.sh --self-test` | rc=0・一致 |
| 2.1 | windows-gnu の clippy（server）/ `check-licenses.sh` | rc=0 / rc=0・一致 |
| 2.2 | `st55_service_`（≥6）/ `--test server_startup st55_bin_`（≥1） | rc=0 n=13 / rc=0 n=4・一致 |
| 2.3 | `st55_stop_`（≥1） | rc=0 n=1・一致 |
| 2.4 | `st55_log_`（≥3）/ `check-log-private.sh` | rc=0 n=4 / rc=0・一致 |
| 3.0 | `sandbox-probe.sh p19` | rc=0。中身は `P16=UNAVAILABLE`・`P19=UNAVAILABLE`（前回と同じ。R45） |
| 3.1 | `st55_layout_`（≥8）/ clippy 2 本 | rc=0 n=18 / rc=0 / rc=0・一致 |
| 3.2 | `install-check.sh --self-test` / actionlint | rc=0 / rc=0・一致 |
| 3.3 | `check-artifacts.sh --self-test` / `package.sh --self-test` | rc=0 / rc=0・一致 |
| 3.4 | actionlint / `check-guards.sh workflows` | rc=0 / rc=0。本文の「release.yml も同じ package.sh を通る」は偽（R75） |
| 4.1 | `st55_db_`（≥3）/ `check-pg-major.sh`（と `--self-test`）/ self-test | rc=0 n=15 / rc=0 / rc=0 / rc=0・一致 |
| 5.1 | `db-roles.sh` / `app_role_`（≥1）/ `server_startup_refuses_privileged_role`（≥1） | rc=0 / rc=0 n=6 / rc=0 n=1・一致 |
| 5.2 | `st55_secret_`（≥5）/ self-test / `check-secrets.sh` | rc=0 n=7 / rc=0 / rc=0・一致 |
| 6.1 | `st55_rerun_`（≥4）/ self-test | rc=0 n=7 / rc=0・一致 |
| 6.2 | `st55_repair_`（≥5）/ self-test | rc=0 n=9 / rc=0・一致 |
| 7.1 | `st55_verifier_`（≥6）/ `web_session` / `check-licenses.sh` | rc=0 n=8 / rc=0 / rc=0・一致 |
| 7.2 | `st55_password_`（≥5）/ self-test | rc=0 n=9 / rc=0・一致 |
| 8.1 | `st55_inbox_`（≥3）/ self-test | rc=0 n=4 / rc=0・一致 |
| 8.2 | `st55_place_`（≥2）/ `export_tests`・`media_tests` | rc=0 n=4 / rc=0・一致 |
| 9.1 | `check-ports.sh --self-test` / `st55_port_`（≥2） | rc=0 / rc=0 n=4・一致 |
| 9.2 | `st55_notice_`（≥3） | rc=0 n=3・一致 |
| 9.3 | `st55_uninstall_`（≥2）/ self-test | rc=0 n=5 / rc=0・一致 |
| 10.1 | self-test / `--check-coverage` / `check_scenarios.py` | rc=0 / rc=0（「88 Scenario の coverage」）/ rc=0（`scenarios: OK`）。coverage の検査の外側は R73 |
| 11.1 | install.md ほかの grep | rc=0・一致 |
| 11.2 | handoff の grep | rc=0・一致 |
| 11.3 | fmt / clippy workspace / windows-gnu clippy / `cargo test --workspace` / check-openapi・migrations・licenses・log-private・secrets | 全部 rc=0・一致 |
| 13.1 | `-p ashiato-setup st55_db_watch_`（≥2） | rc=0 n=3・一致。照らす関数は本番の経路でない（R71） |
| 13.2 | `-p ashiato-server st55_db_watch_`（≥3）/ windows-gnu clippy（server・winsvc） | rc=0 n=7 / rc=0・一致 |
| 13.3 | self-test | rc=0・一致（実地は install-windows） |
| （範囲の外） | **install-windows（`install-check.sh --list` 73 本）を HEAD で** | **未実行**。`git ls-remote origin feat/st55-server-installer` は 90245585e、PR #243 の head も同じ。最後の緑（run 42235492040）は Task 13・R60〜R70 の直しより前の木。R45・R59 で処置済みなので新しい R にしない |

手元の 3 層（cargo test / clippy / `--self-test`）は 33 本の検証コマンドが全部 rc=0 で申告と一致した。Task 13 と R60 の振る舞い（見張りの起こし・SCM への終了コード）は、Linux では判断の関数の戻り値しか見ていない（R71・R72）。

### 手ごとの結果

- 手 1: 固定値を独立に計算した。(a) サービスの仮想アカウントの SID: python の `hashlib.sha1(name.upper().encode("utf-16-le"))` を `struct.unpack("<5I")` で並べると `TrustedInstaller` は `S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464` で、Windows の既知の値と `st55_layout_service_sid_matches_windows_virtual_account_sid`（lib.rs:199）の期待値に一致。(b) 照らすための値: インストーラの `password::verifier("correct-horse-battery-1")` を git が追わない置き場（`.superpowers/sdd/tmp/`）の使い捨ての crate で作り（`$argon2id$v=19$m=19456,t=2,p=1$…`）、OpenSSL 3.5.5 の `openssl kdf … ARGON2ID`（iter 2・memcost 19456・lanes 1・同じ salt）で計算し直すと一致、1 文字違いの合言葉では不一致。(c) 見張りの状態の番号（service.rs の `db_service_state`: 1/2/3/4）は windows-service 0.7.0 の `ServiceState`（`#[repr(u32)]`）と windows-sys 0.59.0 の `SERVICE_STOPPED = 1`・`SERVICE_START_PENDING = 2`・`SERVICE_STOP_PENDING = 3`・`SERVICE_RUNNING = 4` に一致。サービスの権限の数（`SERVICE_QUERY_STATUS = 4`・`SERVICE_START = 16`・`SERVICE_STOP = 32`・`SERVICE_CHANGE_CONFIG = 2`）は windows-sys で見たが、SDDL の字（`LC`・`RP`・`WP`・`DC`）との対応は原典が手元に無く未確認
- 手 2: M1（R72）・M2（R71）・M3（R73）・M4（R74）。どれも「落ちなかった」
- 手 3: `check_scenarios.py` は rc=0。squash（19c44ef28）以降に足された印 3 本（installer/src/lib.rs の「上がった方が別の管理者でも…」・main.rs:1554 の「設定を退けて止まった後に repair すると…」・st55_tests.rs:112 の「DB のサービスが異常に終わると起こし直される」）は、どれも主張より下の階層の試験に付いている（R73）
- 手 4: deep.md の答えで squash 以降に入ったもの —— Q11（`owner.json` は Administrators と SYSTEM だけ）は lib.rs:995 が `icacls … /inheritance:r /grant:r *S-1-5-32-544:F *S-1-5-18:F` を一字一句で固定しているので、変えれば落ちる。Q10（`ashiato`）・Q7（40 GB）・55400・DB のログ 7 日は前回の R44 で固定を確かめた。D35（2026-11-06 の本人の決定）と R60 の数は R74
- 手 5: `[x]` 30 件の検証コマンドは全部実在し rc=0（`--test server_startup` は crates/server/tests/server_startup.rs が実在）。3.4 の本文は R75
- 手 6: 記録を失う新しい経路は見つからなかった。確かめた範囲: repair・uninstall はサーバのサービスを DB より先に止める（main.rs:1353・:1357、commands.rs:217-218）ので、見張り（D35）が止めた DB を起こし直して一時の postmaster と取り合うことは無い / `password` と install の (10) はサーバを止めてから `ensure_running` で起こす（main.rs:1116-1119・:1311-1321） / R60 の `Failed`・`DatabaseUnavailable` は `reset= 86400` で 1 日 3 回までだが、止まっている間は携帯と PC の送信側が溜めるので捨てる型ではない
- 手 7: 該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st55-server-installer` が `[]`）

## R71. 見張りの ACE を DB のサービスに足す本番の式と、試験が照らす関数が別物（ACE の入れ方を壊しても全部通る）
- 成果物: crates/installer/src/commands.rs / crates/installer/src/lib.rs
- 根拠: `st55_db_watch_acl_inserts_ace_before_sacl_and_deduplicates`（lib.rs:418-427）が照らす `commands::add_db_watch_ace`（commands.rs:854）は、`grep -rn add_db_watch_ace crates/ tools/` で試験からしか呼ばれない。本番の `db_watch_acl_command`（commands.rs:844-851）は別に書いた PowerShell の式で、`sc.exe sdshow` の結果に `$s.Insert($d, $a)` で足す。
  M2: その式の `$s.Insert($d, $a)` を `$s.Insert(0, $a)`（ACE を `D:` より前に置く。壊れた SDDL）に替えて `cargo test -p ashiato-setup` → rc=0（82 passed ほか）、`install-check.sh --self-test` → rc=0。式を見る試験（lib.rs:405-416）は `"S:"`・`"Insert"`・`"-notmatch"` を含むかしか見ない。install-check の `assert_db_watch_acl`（:214-220）は Windows で `sdshow` の結果を読む側で、HEAD では一度も走っていない
- kind: technical
- 提案: インストーラが `sc.exe sdshow` の出力を Rust で受け取り、`add_db_watch_ace` で作った SDDL を引数にして `sc.exe sdset` を起こす形にし、試験が照らす関数を本番の経路にする（PowerShell の式を消す）
- 処置: fixed 13.1 —— PowerShell の式を消し、インストーラが `sc.exe sdshow ashiato-db` の出力を Rust で受け取り（`commands::sddl_from_sdshow`）、`commands::db_watch_sdset_command`（中は試験が照らす `add_db_watch_ace`）で作った SDDL を引数に `sc.exe sdset` を起こす形にした（main.rs の `grant_db_watch`。回復の設定の後に毎回呼ぶ）。試験は本番の関数の引数そのものを見る（lib.rs の `st55_db_watch_*` 3 本）。Windows での通しは install-check の `assert_db_watch_acl`（PR の CI）

## R72. 設定の値の拒否を「終了コード 0」に分ける配線を外しても、どの試験も落ちない（R60 の直しは関数の戻り値でしか固定されていない）
- 成果物: crates/server/src/lib.rs / crates/server/src/st55_tests.rs / tools/release/install-check.sh
- 根拠: M1: `run_inner` の `API_TOKEN` が短いときの `refuse_setting(service_mode, "api_token_short", …)`（lib.rs:4499-4503）を型の無い `anyhow::anyhow!(…)` に替えて `cargo test -p ashiato-server st55_` → rc=0（37 passed）、`--test server_startup` → rc=0（53 passed）。この配線が外れると service の口では `ServiceEnd::Failed` → 終了コード 1 → `failureflag 1` の回復が起こし直す（R60 が直した振る舞いに戻る）。
  `st55_service_unrecoverable_refusals_end_with_exit_code_zero`（st55_tests.rs:653-687）は `refuse_startup`・`refuse_setting` を直に呼んで `ServiceEnd::classify` に渡すので、`run_inner` のどの `?` がどちらを返すかは見ない。`run_windows_service` は `cfg(windows)` で Linux では組まれない。install-check.sh にも `WIN32_EXIT_CODE` や拒否の後に起こし直されないことの確かめは無い（`grep -n 'EXIT_CODE\|failureflag' tools/release/install-check.sh` は 0 件）
- kind: technical
- 提案: `st55_tests.rs` に env_lock つきで、`ASHIATO_SERVICE_MODE=1`・短い `API_TOKEN`（と `DATABASE_URL` 無し・`WEB_SESSION_MAX_AGE_DAYS` の誤り）で `run_inner()` を回し、`ServiceEnd::classify` が `Refused` になることを見る試験を置く。Windows の側は「設定のファイルが無いと…」の後に `sc.exe query ashiato-server` の `WIN32_EXIT_CODE` が 0 で、15 秒後も STOPPED のままであることを見る
- 処置: fixed 2.2 —— `st55_service_run_inner_setting_refusals_are_typed_as_refused` を足した（`ASHIATO_SERVICE_MODE=1` で `run_inner()` を回し、DATABASE_URL 無し・API_TOKEN 無し・短い API_TOKEN が `ServiceEnd::Refused`・終了コード 0 になる）。M1 と同じく `api_token_short` の `refuse_setting` を型の無い `anyhow!` に替えると落ちる（st55_tests.rs:790 で panic）ことを確かめて戻した。Windows の `WIN32_EXIT_CODE` の確かめは足していない

## R73. `--check-coverage` は `.rs` に印がある Scenario を全部免除するので、Windows でしか確かめられない Scenario を install-check から消しても通る（73 本のうち 37 本）
- 成果物: tools/release/install-check.sh / crates/server/src/st55_tests.rs / crates/installer/src/main.rs / crates/installer/src/lib.rs
- 根拠: `check_coverage()`（install-check.sh:88-）は git が持つ全部の `.rs` の `// Scenario:` を `exempt_scenarios` にしてから `--list` と比べる。`install-check.sh --list`（73 本）と `.rs` の印を `comm -12` すると 37 本が両方にある。
  M3: install-check.sh の写しから「DB のサービスが異常に終わると起こし直される」の 2 行（`--list` の名前と `echo`）を消して `--check-coverage openspec/changes/st55-server-installer` → **rc=0**。対照に、`.rs` に印の無い「サーバのサービスが異常に終わると起こし直される」を同じく消すと rc=1（`NG: --list に無い Scenario`）。CI の install-windows の `comm -23`（`--list` と出力の比べ）も、両方から消えれば通る。
  その Rust の印は主張より下の階層にある: st55_tests.rs:112 は `decide_db_watch(true, Ok(Stopped)) == Restart` だけ（spec の THEN は「1 分以内に実行中・55400 が接続を受ける」）、main.rs:1554 は `sh -c 'sleep 2 &'` が 1 秒以内に戻ること（spec の THEN は「記録を残したままログインできる」）
- kind: technical
- 提案: 免除を「`.rs` に印があるか」から、install-check で見なくてよい Scenario の明示の表（名前の一覧）に替える。あわせて、THEN がサービス・ACL・SCM の振る舞いの Scenario の Rust 側の印（st55_tests.rs:112・main.rs:1554 ほか）は外し、下支えの試験として残す
- 処置: fixed 10.1 —— 免除を「`.rs` に印があるか」から、install-check.sh の `covered_elsewhere`（15 本の名前の一覧）に替えた。`--check-coverage` は spec の Scenario が `--list` か一覧のどちらかにあること・一覧の名前が spec にあり試験の印を持ち `--list` と被らないことを見る。M3 と同じく「DB のサービスが異常に終わると起こし直される」を写しの `--list` から消すと rc=1（`NG: --list に無い Scenario`）、本体は rc=0（88 本）。下の階層の Rust の印（st55_tests.rs・main.rs）は下支えの試験として残した（免除の根拠ではなくなった）

## R74. D35 の見張りの間隔 5 秒と、回復の `reset= 86400` は、値を変えても全部通る
- 成果物: crates/server/src/service.rs / crates/installer/src/commands.rs / crates/installer/src/lib.rs
- 根拠: M4: commands.rs:798 の `"86400"` を `"3600"` に、service.rs:88 の `interval(Duration::from_secs(5))` を `from_secs(55)` に替えて `cargo test -p ashiato-setup` → rc=0（82 passed ほか）、`cargo test -p ashiato-server --lib st55_db_watch_` → rc=0（7 passed）。試験は `restart/10000/restart/10000/restart/10000`（lib.rs:372）と `failureflag … 1` を見るが、`reset=` の値を参照しない（`grep -n 86400 crates/installer/src/lib.rs` 0 件）。5 秒は `cfg(windows)` の `watch_db_service` の中の値で、Linux の試験からは参照できない。R60 の処置（「`reset= 86400` で 1 日 3 回まで」service.rs:586）と D35（「5 秒ごと」design.md:307）はこの 2 つの値に乗っている
- kind: technical
- 提案: 5 秒を `service::DB_WATCH_INTERVAL` の定数にして試験で固定する（R44 の `DB_WAIT`・`DB_RETRY` と同じ形）。lib.rs の回復の試験に `reset= 86400` を足す
- 処置: fixed 13.2 —— 5 秒を `service::DB_WATCH_INTERVAL` にして `st55_db_watch_interval_is_five_seconds` で固定し、lib.rs の回復の試験に `sc.exe failure <サービス> reset= 86400 actions=` を 2 つのサービスで足した

## R75. tasks 3.4 の `[x]` の本文「release.yml の Windows の組み立ても同じ package.sh を通る」は、いまの release.yml で成り立たない
- 成果物: openspec/changes/st55-server-installer/tasks.md / openspec/changes/st55-server-installer/proposal.md / .github/workflows/release.yml
- 根拠: `grep -n package.sh .github/workflows/release.yml` はコメントの 1 件（:63）だけで、Windows の job は `build.sh server-windows`・`build.sh collector-windows` を直に呼ぶ（R61 の直し）。tasks.md:86 と proposal.md:66 は「同じ package.sh を通る」のまま。検証コマンド（actionlint・`check-guards.sh workflows`）はこの主張を見ないので rc=0。final の scoped re-review は「PR 本文へ」として、上流の文は書き換えていない
- kind: technical
- 提案: tasks.md 3.4 と proposal.md:66 に、R61 で release.yml は build.sh を直に呼ぶ形に替えたこと（理由: package.sh は残った `ASHIATO_BUILD_VERSION` を外す）を注として足す
- 処置: fixed 3.4 —— tasks.md 3.4 と proposal.md の該当の文に、R61 で release.yml は build.sh を直に呼ぶ形に替えたこと（理由つき）を注として足した

## security review（0548aad..63b74d2）

見た経路:
- 画面のログイン（照らす形）: `POST /session` → `session_post`（web_session.rs:492-）→ `WebLogin::accepts` → Argon2id の照合 → 印。記録を読む口（`/sources` ほか）は今までどおり各ハンドラの `authorize` を通り、router（lib.rs:4304-4436）に新しい口は無い（4）
- サービスの口: `ashiato-server service --settings <場所>` → `prepare_service`（service.rs:340-）→ `server.json` の文字列のキーを環境へ（`DATABASE_OWNER_URL` は除く）→ `run_inner` の同じ検査（待ち受けは `net_guard` が 127.0.0.1 以外を拒む、DB は loopback だけ）（3）
- 試験の差し込み: `ASHIATO_SETUP_TEST_*`・`ASHIATO_SERVICE_TEST_*` は `cfg(ashiato_test_build)` の中だけで、配る組み立ては `ASHIATO_RELEASE=1`（package.sh:22・release.yml:69,76）、`check-artifacts.sh:210-211,274-275` が実物の文字列を見る（3）
- 秘密の生成と置き場: `secrets::secret()`（OsRng 32 バイト）→ `server.json`（根から受け継ぐ: サーバ M・本人 RX）/ `owner.json`（管理者と SYSTEM だけの書きかけの置き場で作って置く）。`psql` へは環境変数で渡し、命令の引数に載せない（main.rs:783-786）（3）
- initdb の合言葉: `PwFile::write`（main.rs:1025）→ `settings\.initdb.pwfile`（設定の置き場が根から受け継ぐ ACL のまま）→ `initdb --pwfile` → 消す（3。R76）
- repair: 一時の DB は `127.0.0.1:<一時の番号>`・`sspi` で実行した管理者だけ（commands.rs:1032-1095）、終わりに止めて hba/ident を消す（3）
- 取り込み: `inbox_acl_commands`（commands.rs:654-）がダウンロードにサーバの RX、取り込み待ちに M。リンクは `ensure_real_directory` と `icacls /L` で弾く。消す口が ACE を外す（commands.rs:262-）（3）
- インストーラの出力・ログ: `run` は命令の行だけを出し（合言葉は環境変数・ファイルにあるので行に載らない）、install-check の `assert_no_secret_in` が出力・サーバと DB のログに 5 つの秘密が無いことを見る（3）
- 依存: `argon2 0.5.3`・`rpassword 7.5.4`・`sha1 0.10`・`windows-service 0.7.0`（と `password-hash`・`blake2`・`rtoolbox`・`widestring`）は crates.io の正規の名前。同梱の PostgreSQL は `pgsql.lock` の sha256 をキャッシュから読むたびにも照らす（pgsql.sh:25-31）（2）
- 待ち受け: サーバ 127.0.0.1:18700（`BIND`）、DB `listen_addresses='127.0.0.1'`・55400・`-A scram-sha-256`（db.rs:13-37）。インストーラはネットワークに出ない（`open_browser` の 127.0.0.1 だけ）（2）

安全の決め手: インストールした形（照らす形）でも、記録を読む口は画面のログインの印でしか開かず、API の合言葉・照らすための値そのもの・偽の印では開かない —— 確かさ 4。
一時の試験（`App::with_login` + `WebLogin::new_with_verifier` で組んだ router に `tower::oneshot`。走らせた後に消した）を `cargo test -p ashiato-server --lib st55_secprobe -- --nocapture` で走らせ rc=0:
`GET /api/sources` 資格なし → 401 / `Authorization: Bearer <API_TOKEN>` だけ → 403 / `POST /api/session` に `WEB_PASSWORD_VERIFIER` の PHC 文字列そのもの → 401 / 偽の `ashiato_session` → 401 / 正しい合言葉でログインした印 → 200。

## R76. initdb に渡す DB の管理者（superuser）の合言葉を、本人とサーバのアカウントが読める設定の置き場にそのまま書く。その値はインストール後も `ashiato` の役割の合言葉として残る
- 成果物: crates/installer/src/main.rs / tools/db-roles.sql
- 根拠: main.rs:966 `pwfile_path = settings_dir.join(".initdb.pwfile")` → main.rs:1025 `PwFile::write` は素の `fs::write`（main.rs:504）。設定の置き場は根（`acl_commands` commands.rs:414-430: サーバの仮想アカウント `(OI)(CI)M`・本人 `(OI)(CI)RX`）から受け継ぎ、`temporary_write_acl_commands`（commands.rs:559-）は ACE を足すだけで継承を切らないので、このファイルは initdb が走る間、本人とサーバのアカウントが読める。
  `run_psql`（main.rs:783-784）は同じ `generated.admin_password` を `ADMIN_PW` に渡し、db-roles.sql:5 の `ALTER ROLE … PASSWORD :'admin_pw'` が同じ値を DB の管理者の役割に置き直すので、ファイルを消した後も合言葉は変わらない（repair を走らせるまで）。
  一方、表の所有者の合言葉（`owner.json`）は、まさにこの「設定の置き場に直に作ると本人とサーバのアカウントに読まれる」を理由に、管理者と SYSTEM だけの書きかけの置き場で作っている（main.rs:561-567 のコメント・review R48・deep Q11）。所有者より強い superuser の合言葉だけがその境の外にある。
  本人を別に渡したインストール（`--owner-sid`。install-check の「本人を渡したインストール」）では本人は標準の利用者になりうる。読めた者は `127.0.0.1:55400` に superuser で入り（pg_hba は scram）、`COPY … TO PROGRAM` で DB のサービスのアカウント（`db` に M を持つ）として命令を走らせられる（PostgreSQL の COPY の文書: PROGRAM は superuser か `pg_execute_server_program`）。Windows の ACL は手元で確かめていない（確かさ 3）
- kind: technical
- 提案: `.initdb.pwfile` を `owner.json` と同じく `admin_only_directory_acl_command` で絞った書きかけの置き場に作る。あわせて `run_psql` の `ADMIN_PW` を initdb の値と別の新しい乱数にし（`PGPASSWORD` は initdb の値のまま）、ファイルが漏れても役割を作った後は使えないようにする。install-check に「本人の利用者で `settings\.initdb.pwfile` を読めない」を足す

範囲外: なし
- 処置: fixed 5.2 —— (1) initdb の合言葉のファイルを `owner.json` と同じ形の管理者と SYSTEM だけの置き場（`settings\.initdb-staging-<印>`。`admin_only_staging_dir` に括り出し、`stage_owner_settings` も同じ関数を通る）に `create_new` で作り、成否に依らず置き場ごと消す。前の版の `settings\.initdb.pwfile` は始めに消す。(2) `run_psql` の `ADMIN_PW` を `secrets::new_admin_password()`（控えない使い捨ての乱数）にし、`PGPASSWORD` は initdb の値のまま。initdb の値は役割を作った後は使えない。install-check の `assert_no_pwfile` は `.initdb-staging-*` の残りも落とす（self-test に足した）。Windows の ACL の実地は PR の CI（install-windows）

## /code-review（0548aad..63b74d2）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R77. `recreatable` requires `!db_service_exists`, but on a fresh install the services are registered in step (1), before initdb (the R31 ordering). So on a fresh install the 'initdb started but roles not done' exit can never be taken, and the rerun is refused.
- 成果物: crates/installer/src/main.rs:1005
- 根拠: First install: services are registered, initdb writes PG_VERSION, then the DB fails to start within 120s (wait_for_database) and the install returns Err. On rerun: database_exists=true, roles_done=false, initdb_marker exists, but db_service_exists=true, so recreatable=false and install stops with roles_missing_message. The user is forced into `repair`, which regenerates every secret (API token and so on) and makes the collectors re-pair, for a DB that holds no records.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 再現した —— 初回で services を (1) で登録した後、initdb の後に DB が起きずに落ちると、走らせ直しは `db_service_exists` で `recreatable=false` になり `roles_missing_message`（repair を示す）で止まる。これは D32（仮）の決定そのもの（design.md「DB のサービスを登録した後に落ちたものも、消さずに `repair` を示す」。印は設定の置き場にありサーバのアカウントも書けるので、印だけで DB を消さないための条件）。repair が作り直す秘密（API の合言葉ほか）は、インストールが終わっていないので収集アプリの対がまだ無く、失うものも無い。作り直しの条件を緩めると、サーバのアカウントが印を書いて記録の入った DB を消させうる形に戻る

## R78. The LOGIN_CHECK permit is held by the request future, but the Argon2 check runs in spawn_blocking. If the client disconnects, the future is dropped and the permit released while the blocking check keeps running, and record_failure is never called.
- 成果物: crates/server/src/web_session.rs:548
- 根拠: An attacker sends POST /session repeatedly and closes each connection right away. Every request starts a tens-of-milliseconds Argon2id verify on a blocking thread while the semaphore is freed immediately, so many verifies run at once, saturating CPU and the blocking pool. Because the failures are never counted, the limiter (throttled) never engages. The 'one at a time' guarantee from R23 is lost.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— 1 つずつの印（permit）と失敗の数え上げを `spawn_blocking` の中へ移した。接続が切れて future が捨てられても、照合が終わって `record_failure` するまで次の照合は始まらない（blocking の仕事が panic したときも数える）

## R79. Deleting server logs older than 30 days only happens in ServerLog::new (at startup). When write_bytes_at reopens the file on a date change it does not prune, so a long-running service never deletes old logs.
- 成果物: crates/server/src/service.rs:426
- 根拠: The service runs for 90 days without restarting (normal for an always-on server) and server-YYYY-MM-DD.log files keep accumulating under logs\server, breaching the spec's 30-day retention. Separately, if remove_file fails on an old log (e.g. another process has it open), new() returns Err, run_service ends as Failed with exit 1, and recovery restarts the service every 10 seconds.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.4 —— 古いログの片付けを `ServerLog::prune` に括り出し、起動のときに加えて、日付が変わってファイルを開き直すたびに行う。消せないファイルは飛ばして次の機会に回し、ログの書きもサービスも止めない（`new` が Err を返さない）。`st55_log_prunes_old_files_when_the_date_changes_without_restart` を足した

## R80. wait_for_db retries every connect error the same way. Non-transient errors such as auth failure or a malformed URL also wait 120s and become DatabaseUnavailable (exit 1), so the failureflag-1 recovery restarts the service forever.
- 成果物: crates/server/src/service.rs:654
- 根拠: The ashiato_app password in server.json no longer matches the DB (half-finished repair, manual edit). Every attempt fails with 28P01 auth error, then TimedOut, then exit 1, then a restart 10s later, repeating forever: a loop that restarting cannot fix, which is exactly the situation R21/R60 tried to prevent.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.2 —— `wait_for_db` に `DbWaitError::is_permanent` を足し、接続の文字列の誤り（`Configuration`）・認証の失敗（28P01・28000）・DB が無い（3D000）は待たずに `WaitOutcome::Rejected` を返す。service の口はそれを `StartupRefused("database_rejected")`（終了コード 0）にしてログに書く。起動の途中の断り（57P03）や待ち受けが無いのは今までどおり待つ。`st55_service_wait_db_rejects_wrong_password_without_waiting`（本物の試験の DB に誤った合言葉で 1 回で Rejected）を足した

## R81. cleanup_partial_files deletes every dotfile in the export directory (not just `.partial-ashiato-export-*`) and stops at the first deletion error via `?`. It is also called right after exports.stop() (abort) without waiting for the task to actually stop.
- 成果物: crates/server/src/service.rs:517
- 根拠: If ASHIATO_EXPORT_DIR points at a user folder, unrelated hidden files such as `.keep` are deleted on every stop. And if the aborted export task is still mid-write on another worker, remove_file fails with a sharing violation on Windows, the loop exits early, the error is dropped by `let _`, and the remaining partial files stay behind.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.3 —— 片付けは `export::PARTIAL_PREFIX`（`.partial-ashiato-export-`）の名前だけを消し、1 つ消せなくても残りを消して最初の失敗を返す。run_inner は `exports.stop()` の後、走っている書き出しが無くなるまで最大 5 秒待ってから片付け、失敗はログに書く（`let _` で捨てない）。`st55_stop_cleanup_removes_only_partial_exports` を足した

## R82. The `service_mode` computed at the top of run_inner is ignored here and `ASHIATO_SERVICE_MODE` is read from the environment twice more, so the mode decision has three sources.
- 成果物: crates/server/src/lib.rs:4713
- 根拠: If the environment changed partway through (set_var/remove_var by a test or other code), the password checks and the graceful-shutdown/cleanup decisions could disagree. Using the `service_mode` variable in both places is enough.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.2 —— run_inner の後半の 2 か所も最初に読んだ `service_mode` を使う

## R83. add_db_watch_ace is a test-only Rust copy of the ACE insertion that actually ships, which is the PowerShell in db_watch_acl_command. The tests check only the Rust copy.
- 成果物: crates/installer/src/commands.rs:854
- 根拠: If the PowerShell SDDL handling (Select-Object -Last 1, the S: insertion point, -notmatch) breaks, the tests stay green and the server SID never gets LCRP on ashiato-db, so the DB watcher only ever records query failures. Either generate the PowerShell from the Rust function or point the tests at the command string.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 13.1 —— R71 と同じ直し（本番の経路が試験の照らす Rust の関数を通る）

## R84. A hand-written base64url encoder; the workspace already uses the `base64` crate (`URL_SAFE_NO_PAD` in server/web_session.rs).
- 成果物: crates/installer/src/secrets.rs:147
- 根拠: The hand-written bit operations and padding handling add maintenance and verification cost. `base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)` would replace it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 5.2 —— 手書きの符号化を消し、`base64::engine::general_purpose::URL_SAFE_NO_PAD`（server と同じ 0.22。Cargo.lock に既にある）にした。`st55_secret_values_are_unique_base64url` は通る

## R85. The installer's build-stamp inputs do not include tools/db-roles.sql, which main.rs embeds with include_bytes!.
- 成果物: crates/installer/build.rs:18
- 根拠: Change db-roles.sql without committing it and build: the installer exe embeds different SQL, but its stamp has no `.dirty`, so check-artifacts.sh's 'no .dirty stamp' check passes. The source the stamp claims and what is actually embedded disagree.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 3.3 —— installer/build.rs の印の入力を `INPUTS` にまとめ、`tools/db-roles.sql` を足した（印と `rerun-if-changed` の両方）

## R86. Server log files are named by UTC date (chrono::Utc::now().date_naive()), so the file switches at 09:00 JST, not at local midnight.
- 成果物: crates/server/src/service.rs:507
- 根拠: A Japanese user opening the log for 'today' to investigate finds events from 00:00 to 09:00 local time in the previous day's file, and the 30-day pruning also counts in UTC days.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.4 —— ログのファイルの日付を `service::log_day`（Asia/Tokyo。画面の日の区切り `coverage::DAY_TZ` と同じ固定のずれ）で決める。起動の拒否のログ・30 日の片付けも同じ日付。`st55_log_day_splits_at_tokyo_midnight` を足した

## finish の scoped re-review（9cc919b..b1d62ae）

fix の後の scoped re-review（SDD の `re-review-prompt.md`、別の文脈）。R71〜R86 は全部 ADDRESSED、R77 は REJECTION HOLDS（D32（仮）のとおり。repair が作り直す秘密に対の相手はまだ無い）。
- N7（Critical・Windows で未確認）: R76 の直しで、initdb の合言葉の置き場を管理者と SYSTEM だけにした。PostgreSQL の `initdb` は管理者として起こされると Administrators を deny-only にした制限付きの token で自分を起こし直し（restricted_token.c）、`--pwfile` をその子で読むので、置き場を読めずに新規のインストールと作り直しが止まる見込みが高い。前の版は設定の置き場に `TemporaryWrite` が走らせている利用者の SID を足していたので読めていた
  - 処置: fixed 5.2 —— 自分の直しが持ち込んだ退行なので park せずに直した。置き場を絞った後、走らせている利用者（`identity.invoking`）の SID に `(OI)(CI)R` だけを足す（`commands::staging_reader_grant_command`。試験 `st55_secret_initdb_staging_adds_only_invoking_reader`）。サーバのアカウントは引き続き読めない。本人を別に渡したインストール（`--owner-sid`）の本人も、走らせている管理者と別なら読めない。この直しは re-review を通していない（fix は 1 回・re-review は 1 回）。commit 後のセキュリティの hook は「利用者の SID に読みを足す」を HIGH としたが、直さない: 読めるのは走らせている管理者の SID だけで、本人（`--owner-sid`）が別の利用者なら読めず、同じなら元々管理者に上がれる人。読める間は initdb の数秒で、置き場ごと消す。hook の代案は使えない（initdb の `--pwfile` は標準入力を読まない・既定の整合性ラベルは書き込みしか止めない）。Windows での確かめは PR の CI の install-windows（初回のインストール）
- 文の上の傷（処置しない）: tasks.md 3.4・proposal.md の本文「同じ package.sh を通る」は残り、直後の注で打ち消す書き方になっている
