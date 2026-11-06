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

ledger の Ruling（全 Task の `progress.md`。Task 1〜11 の ledger に `Ruling:` の行は無く、あるのは final の ledger の 2 行だけ）:
- Ruling: N1・N2・N3 は finish の fix の後の scoped re-review で見つかった。SDD の final review と同じく fix は 1 回だけなので、この回では直さず park する — Q11（R48）の答えの後にグラフが finish をもう一度呼ぶので、そこで直す（N1 は `.cargo/config.toml` の msvc の `crt-static` か DLL を根にも写す、N2 は repair の前に移行の完了記録の有無を控えて、無ければ書かない）— 直さずに merge すると、N1 は VC の再頒布パッケージの無い機械でインストーラが起動しない、N2 は repair の後にサーバが表の無い DB で動く（どちらも記録は失わない。merge gate は Q11 の (未回答) で止まる）

- Ruling: N4・N5・N6 は 2 回目の finish の fix の後の scoped re-review で見つかった。SDD の final review と同じく fix は 1 回だけなので直さず park し、PR 本文で人間に渡す — どれも記録を失わず、起きるのは命令の失敗か途中で落ちたときだけ（N5 は icacls の失敗、N6 は数個の命令の間の窓）— 直すなら change で: N4 は親で写しが非 0 でも置き場を消す、N5 は書きかけの置き場の中で rename の前に保護の ACL を付ける、N6 は owner.json を置く段を役割の完了記録より前に移す（または `repair_allowed` に「owner.json が server.json より新しい」を足す）

上の 1 行目の N1〜N3 は、2 回目の finish（Q11 の答えの後）で直した:
- N1: `.cargo/config.toml` で `x86_64-pc-windows-msvc` に `+crt-static`。`check-artifacts.sh` は根の 2 つの exe が VC の実行時の DLL を読む zip を落とす（self-test で落ちることを確かめた）。design D33 を直した
- N2: `repair` は移行の完了記録を書かない（無ければ走らせ直しの段 (8) が移行を当てる）。design D32 に書いた
- N3: 消す口の写しの置き場は、ACL・空の確かめ・写しの作成・起こすのが失敗したら消す（re-review は PARTIALLY。残りは N4）

**park したもの**（`.superpowers/sdd/st55-final/progress.md`。2 回目の finish の scoped re-review）:
- N4（確かさ 3）: 写しが起きた後、遅れて消す台本を起こす前に失敗すると（サービスが 180 秒で止まらない など）、`C:\ProgramData\ashiato-uninstall-*`（管理者と SYSTEM だけ）が残る
- N5（確かさ 2）: owner.json を置いた後の icacls が失敗すると、保護されない形で残り、設定の置き場の `/reset` でサーバと本人の読みを受け継ぎ直しうる
- N6（確かさ 2）: repair の R40 の出口で、owner.json を置いた後・server.json を置く前に落ちると食い違いが残り、repair をやり直せない

## Task

- 埋めた Task: 27 / 30（1.1〜11.3）
- 残した Task: 12.1〜12.3（人間の確認待ち。確認バッチで Windows サンドボックスの録画・再起動の後・Tailscale の無人で動かす）—— `human` の正常な残り

## 検証（evidence.jsonl と finish の走らせ直し）

- evidence.jsonl: 1.1〜11.3 の 27 項目とも最後の結果が PASS（rc=0）
- 2 回目の finish（e4352ed5・86cd0730 の後）に `scripts/verify-run` で走らせ直したもの（どれも rc=0）: 3.2（`install-check.sh --self-test`・actionlint）・3.3（`check-artifacts.sh --self-test` 314 秒・`package.sh --self-test`）・5.2（`cargo test -p ashiato-setup st55_secret_`・`check-secrets.sh`）・6.1・6.2・9.3
  - 3.3 は 1 回目に FAIL（rc=1）: 未 commit の変更がある木で組んだので `.dirty` の印が付いた。commit した後のきれいな木で PASS
- ほかに: `cargo fmt --all -- --check` rc=0 / `cargo clippy --workspace --all-targets -- -D warnings` rc=0 / `cargo clippy -p ashiato-setup --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `cargo test -p ashiato-setup` rc=0（76 本）/ `cargo test -p ashiato-server st55_` rc=0 / `install-check.sh --check-coverage` rc=0（89 Scenario）/ `scripts/check_scenarios.py` rc=0 / `review_triage.py` rc=0
- 足した `check-artifacts.sh` の DLL の検査は、gnu で組んだ本物の `ashiato-setup.exe` では掛からない（0 件）ことも見た
- 1 回目の finish（464f10ab・f5f103a1・9c41a46d の後）: `cargo test --workspace` rc=0（408 秒）・`pgsql.sh --self-test`・`check-guards.sh workflows`・`check-pg-major.sh --self-test`・`check-licenses.sh`・`check-log-private.sh` rc=0。ガードを壊して落ちることも確かめた（照らす形の照合・`prepare_service`）
- **Windows の本物の通し（install-windows。spec の 72 本 + 今回の owner.json の Scenario）はまだ一度も走っていない。** 上の rc=0 は Linux の確かめで、Windows の担保ではない。owner.json の ACL・書きかけの置き場・crt-static も本物の Windows では未確認。この PR の CI が初めて走らせる（初回は赤の見込みもある）。merge は head の CI が全部緑でないと進まない

## レビューの処置（review/code.md）

- final review（R1〜R30）・code_verify（R40〜R46）・security review（R47・R48）・`/code-review`（R49〜R58）・final の re-review（R31〜R39）の 96 件すべてに処置がある。finish の 2 回の scoped re-review の N1〜N6 は上の Ruling のとおり
- finish で直したもの: R28・R31〜R33・R35〜R38・R40〜R44・R46・R47・R49〜R58
- rejected: R39（code_verify が独立に走らせ直した）・R45（merge gate が head の CI の緑を見る）
- escalated: R48（Q11）→ 答えが出て、2 回目の finish で D4・spec・コードに写した
- 自動のセキュリティの見直し（hook）の指摘を受けて、作り直しの条件を「DB のサービスが無く移行の完了記録も無い」まで締め、repair の一時のログは前の回の残りを消せなければ止まり、消す口の台本は `create_new` で書く形にした。残るリスク: initdb を始めた印は設定の置き場（サーバのアカウントも書ける）にある。DB のサービスはサーバのアカウントでは作れも消せもしないので、印だけでは消えない

## 申し送り

- `docs/handoff/ST55.md` の C7・C12 は final review（R7・R30）で処置済み。ほかの未処置の申し送りは無い
- 版を替える口（ST57・ST58）へ: D34（仮）の「別の版なら止まる」は、その口が写し替え・サービスの止め・移行を受け持つ形に替える
