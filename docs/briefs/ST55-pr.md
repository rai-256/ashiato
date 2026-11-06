# ST55 サーバのインストーラ（Windows・同梱の PostgreSQL・サービス）

## 未決（A）—— なし

- 第 4 回の深掘り Q11（review/code.md R48）は答えが出た（2026-11-04。本人は推奨どおり「分ける」）。表の所有者の合言葉（`DATABASE_OWNER_URL`）は `settings\server.json` から外し、`settings\owner.json`（継承を切って Administrators と SYSTEM だけ・持ち主も Administrators）に置く形にした（design D4・D27、spec distribution の Scenario「表の所有者の合言葉は、サーバのサービスが読めないファイルにだけある」、e4352ed5・86cd0730）。
  書きかけは、先に管理者と SYSTEM だけにした新しいフォルダ `settings\.owner-staging-<印>` の中に作る（直に作ってから ACL を絞ると、絞る前に開いた handle で中身を読める —— commit 後のセキュリティの hook の指摘）。

## 仮決め（B / P）と Ruling

design の（仮）（台帳 `provisional.jsonl`。別の席の確かめはグラフの `provisional_review` が書く）:
- D3（仮）・D16（仮）・D18（仮）・D26（仮）・D29（仮）・D31（仮）（前の段から）
- **D32（仮）** —— 役割の完了記録の無い DB を自動で作り直すのは、インストーラが initdb の前に書いた印があり、DB のサービスがまだ無く、移行の完了記録も無いときだけ。それ以外は消さずに止まり、`repair` を示す。repair は完了記録を書いてから設定を置く（R40。D30（仮）を置き換え、台帳で resolve）
- **D33（仮）** —— 同梱の PostgreSQL が要る `vcruntime140.dll`・`msvcp140.dll` を、Windows の runner の Visual Studio の再頒布の置き場から `pgsql\bin` に同梱する（R28。EDB の zip を落として objdump で確かめた）
- **D34（仮）** —— プログラムの置き場に zip と違うファイルがあれば、何も替えずに止まる（版を替える口はまだ無い）。無いファイルは `.copying` から rename で置く（R38・R50）
- D24 の「どちらの zip かを各 echo の行に書く」は、印の名前を spec と一字一句合わせるため、節の見出しに書く形に替えた（手段の変更。R34）

ledger の Ruling（全 Task の `progress.md`。Task 1〜11・13 の ledger に `Ruling:` の行は無く、あるのは final の ledger の 3 行だけ）:
- Ruling: N1・N2・N3 は finish の fix の後の scoped re-review で見つかった。SDD の final review と同じく fix は 1 回だけなので、この回では直さず park する — Q11（R48）の答えの後にグラフが finish をもう一度呼ぶので、そこで直す（N1 は `.cargo/config.toml` の msvc の `crt-static` か DLL を根にも写す、N2 は repair の前に移行の完了記録の有無を控えて、無ければ書かない）— 直さずに merge すると、N1 は VC の再頒布パッケージの無い機械でインストーラが起動しない、N2 は repair の後にサーバが表の無い DB で動く（どちらも記録は失わない。merge gate は Q11 の (未回答) で止まる）

- Ruling: N4・N5・N6 は 2 回目の finish の fix の後の scoped re-review で見つかった。SDD の final review と同じく fix は 1 回だけなので直さず park し、PR 本文で人間に渡す — どれも記録を失わず、起きるのは命令の失敗か途中で落ちたときだけ（N5 は icacls の失敗、N6 は数個の命令の間の窓）— 直すなら change で: N4 は親で写しが非 0 でも置き場を消す、N5 は書きかけの置き場の中で rename の前に保護の ACL を付ける、N6 は owner.json を置く段を役割の完了記録より前に移す（または `repair_allowed` に「owner.json が server.json より新しい」を足す）

- Ruling: N7 は fix の後の scoped re-review で見つかった。SDD では fix は 1 回だけで残りは park だが、N7 はこの回の直し（R76）が持ち込んだ退行で、park すると新規のインストールが initdb で止まる（merge gate は install-windows の赤で止まるが、直すのが 1 行の ACL なので待たせる理由が無い）ので直した — その直しは re-review を通していない。Windows の確かめは PR の CI の install-windows（初回のインストール）が持つ — 赤なら、置き場の ACL（`staging_reader_grant_command`）を最初に疑う

上の 1 行目の N1〜N3 は、2 回目の finish（Q11 の答えの後）で直した:
- N1: `.cargo/config.toml` で `x86_64-pc-windows-msvc` に `+crt-static`。`check-artifacts.sh` は根の 2 つの exe が VC の実行時の DLL を読む zip を落とす（self-test で落ちることを確かめた）。design D33 を直した
- N2: `repair` は移行の完了記録を書かない（無ければ走らせ直しの段 (8) が移行を当てる）。design D32 に書いた
- N3: 消す口の写しの置き場は、ACL・空の確かめ・写しの作成・起こすのが失敗したら消す（re-review は PARTIALLY。残りは N4）

**park したもの**（`.superpowers/sdd/st55-final/progress.md`。2 回目の finish の scoped re-review）:
- N4（確かさ 3）: 写しが起きた後、遅れて消す台本を起こす前に失敗すると（サービスが 180 秒で止まらない など）、`C:\ProgramData\ashiato-uninstall-*`（管理者と SYSTEM だけ）が残る
- N5（確かさ 2）: owner.json を置いた後の icacls が失敗すると、保護されない形で残り、設定の置き場の `/reset` でサーバと本人の読みを受け継ぎ直しうる
- N6（確かさ 2）: repair の R40 の出口で、owner.json を置いた後・server.json を置く前に落ちると食い違いが残り、repair をやり直せない

## Task

- 埋めた Task: 30 / 33（1.1〜11.3・13.1〜13.3。Task 13 は D35 の DB のサービスの見張り。2026-11-06 の本人の決定）
- 残した Task: 12.1〜12.3（人間の確認待ち。確認バッチで Windows サンドボックスの録画・再起動の後・Tailscale の無人で動かす）—— `human` の正常な残り

## 検証（evidence.jsonl と finish の走らせ直し）

- evidence.jsonl: 1.1〜11.3 の 27 項目とも最後の結果が PASS（rc=0）
- 2 回目の finish（e4352ed5・86cd0730 の後）に `scripts/verify-run` で走らせ直したもの（どれも rc=0）: 3.2（`install-check.sh --self-test`・actionlint）・3.3（`check-artifacts.sh --self-test` 314 秒・`package.sh --self-test`）・5.2（`cargo test -p ashiato-setup st55_secret_`・`check-secrets.sh`）・6.1・6.2・9.3
  - 3.3 は 1 回目に FAIL（rc=1）: 未 commit の変更がある木で組んだので `.dirty` の印が付いた。commit した後のきれいな木で PASS
- ほかに: `cargo fmt --all -- --check` rc=0 / `cargo clippy --workspace --all-targets -- -D warnings` rc=0 / `cargo clippy -p ashiato-setup --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `cargo test -p ashiato-setup` rc=0（76 本）/ `cargo test -p ashiato-server st55_` rc=0 / `install-check.sh --check-coverage` rc=0（89 Scenario）/ `scripts/check_scenarios.py` rc=0 / `review_triage.py` rc=0
- 足した `check-artifacts.sh` の DLL の検査は、gnu で組んだ本物の `ashiato-setup.exe` では掛からない（0 件）ことも見た
- 1 回目の finish（464f10ab・f5f103a1・9c41a46d の後）: `cargo test --workspace` rc=0（408 秒）・`pgsql.sh --self-test`・`check-guards.sh workflows`・`check-pg-major.sh --self-test`・`check-licenses.sh`・`check-log-private.sh` rc=0。ガードを壊して落ちることも確かめた（照らす形の照合・`prepare_service`）
- **Windows の本物の通し（install-windows。spec の 72 本 + 今回の owner.json の Scenario）はまだ一度も走っていない。** 上の rc=0 は Linux の確かめで、Windows の担保ではない。owner.json の ACL・書きかけの置き場・crt-static も本物の Windows では未確認。この PR の CI が初めて走らせる（初回は赤の見込みもある）。merge は head の CI が全部緑でないと進まない

## 3 回目の finish（Task 13 の後。R59〜R86）

- whole-branch final review（R59〜R70）: R60〜R70 を直した（サービスの終了コード・release.yml の版・DB の見張りほか）。R59 は rejected（Windows の通しの緑は merge gate が head の CI で見る）。scoped re-review は全部 ADDRESSED
- **R61 の直しで、`release.yml` の Windows の組み立ては `package.sh` を通らなくなった**（`build.sh server-windows`・`collector-windows` を直に呼ぶ）。proposal・tasks 3.4 の「同じ package.sh を通る」には注を足した（R75）
- code_verify（R71〜R75）・security review（R76）・`/code-review`（R77〜R86）は、R77 を除いて直した（b1d62aef5）:
  - R71・R83: DB のサービスへ見張りの ACE を足す処理は、本番も試験が照らす Rust の関数を通る（`sc.exe sdshow` の結果を `db_watch_sdset_command` に通して `sdset`。PowerShell の式は消した）
  - **R76（security）**: initdb に渡す DB の管理者の合言葉のファイルを、本人とサーバのアカウントが読める設定の置き場に作っていた。`owner.json` と同じ管理者と SYSTEM だけの置き場に作り、役割を作るときに管理者の合言葉を控えない使い捨ての乱数に替える（initdb の値は役割の後は使えない）
  - R80: 合言葉の誤り（28P01・28000）・DB が無い（3D000）・接続の文字列の誤りは、120 秒待たずに終了コード 0 の拒否にする（回復が起こし直し続けない）
  - R72・R74: 設定の拒否が終了コード 0 になる配線、見張りの 5 秒、回復の `reset= 86400` を試験で固定した（R72 は配線を壊すと落ちることを確かめた）
  - R73: install-check の網羅の免除を「`.rs` に印があるか」から名前の一覧（`covered_elsewhere` 15 本）にした（Windows の通しの Scenario を `--list` から消すと落ちることを確かめた）
  - R78: ログインの照合の 1 つずつの印と失敗の数え上げを blocking の側へ移した（接続を切っても照合が並ばない）
  - R79・R86: サーバのログは日付が変わるたびに 30 日より古いものを消し、日付は Asia/Tokyo で区切る
  - R81: 停止のときの片付けは `.partial-ashiato-export-*` だけを消し、書き出しが止まるのを最大 5 秒待つ
  - R82・R84・R85: `service_mode` を 1 か所で読む・base64 は crate を使う・インストーラの組み立ての印に `tools/db-roles.sql` を入れる
  - **R77 は rejected**: 初回のインストールがサービスの登録の後・役割の前に落ちると、走らせ直しは repair を示して止まる。D32（仮）の決定どおり（印だけで DB を消さない条件）。インストールが終わっていないので、repair が作り直す秘密に対の相手はまだ無い
- この回の scoped re-review（別の文脈）: R71〜R86 は全部 ADDRESSED、R77 は REJECTION HOLDS。ただ、**R76 の直しが Critical を 1 件持ち込んだ**（N7）—— initdb は管理者として起こされると、管理者の権限を外した token で自分を起こし直すので、管理者と SYSTEM だけの置き場の合言葉のファイルを読めない。走らせている利用者の SID に読みだけを足して直した（上の Ruling。この直しは re-review を通していない）
- 検証（この回）: `cargo test --workspace --no-fail-fast --exclude ashiato-collector-windows` rc=0（266 秒）/ `cargo test -p ashiato-setup` rc=0（N7 の後に 83 本）/ `cargo clippy --workspace --all-targets -- -D warnings` rc=0 / `cargo clippy -p ashiato-server -p ashiato-winsvc -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0（N7 の後に ashiato-setup を両方で再度 rc=0）/ `install-check.sh --self-test` rc=0 / `--check-coverage` rc=0（88 本）/ `scripts/check_scenarios.py . st55-server-installer` rc=0 / check-openapi・migrations・licenses・log-private・secrets rc=0 / `review_triage.py` rc=0
  - **`ashiato-collector-windows` の `history_large_fetch_keeps_observing_foreground` は手元で 2 回とも落ちた**（「見回り 1 回が 2.6 秒 / 1.09 秒かかった」。1 回 1 秒以内を求める時間の試験で、機械の load average は約 5）。この枝は collector-windows を変えていない（`git diff origin/main -- crates/collector-windows` は空）ので、範囲の外として直していない。CI で落ちたら main でも同じかを見る
  - 新しい試験: `st55_service_wait_db_rejects_wrong_password_without_waiting`・`st55_service_run_inner_setting_refusals_are_typed_as_refused`・`st55_stop_cleanup_removes_only_partial_exports`・`st55_log_prunes_old_files_when_the_date_changes_without_restart`・`st55_log_day_splits_at_tokyo_midnight`・`st55_db_watch_interval_is_five_seconds`・`st55_secret_initdb_staging_adds_only_invoking_reader`
- **本物の Windows でしか確かめられないもの**（PR の CI の install-windows が初めて走らせる）: 見張りの ACE の `sdshow`→`sdset`（R71）・initdb の合言葉の置き場の ACL と制限付きの token での読み（R76・N7）・`database_rejected` の終了コード 0（R80）・R60〜R68 の SCM の振る舞い

## レビューの処置（review/code.md）

- review/code.md の指摘 124 件すべてに処置がある（`review_triage.py` rc=0）。1・2 回目の finish までの内訳: final review（R1〜R30）・code_verify（R40〜R46）・security review（R47・R48）・`/code-review`（R49〜R58）・final の re-review（R31〜R39）。finish の 2 回の scoped re-review の N1〜N6 は上の Ruling のとおり
- finish で直したもの: R28・R31〜R33・R35〜R38・R40〜R44・R46・R47・R49〜R58
- rejected: R39（code_verify が独立に走らせ直した）・R45（merge gate が head の CI の緑を見る）
- escalated: R48（Q11）→ 答えが出て、2 回目の finish で D4・spec・コードに写した
- 自動のセキュリティの見直し（hook）の指摘を受けて、作り直しの条件を「DB のサービスが無く移行の完了記録も無い」まで締め、repair の一時のログは前の回の残りを消せなければ止まり、消す口の台本は `create_new` で書く形にした。残るリスク: initdb を始めた印は設定の置き場（サーバのアカウントも書ける）にある。DB のサービスはサーバのアカウントでは作れも消せもしないので、印だけでは消えない

## 申し送り

- `docs/handoff/ST55.md` の C7・C12 は final review（R7・R30）で処置済み。ほかの未処置の申し送りは無い
- 版を替える口（ST57・ST58）へ: D34（仮）の「別の版なら止まる」は、その口が写し替え・サービスの止め・移行を受け持つ形に替える
