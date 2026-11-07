# CH09 —— 実時間で「〜秒以内」を求める試験を、混み具合で落ちない形にする

change: `openspec/changes/ch09-load-tolerant-timing-tests/`（新 capability `test-timing`、MODIFIED `external-ingestion`・`test-database`）。依頼・壊してはいけないもの・完了の判定は `docs/changes/CH09.md`。
**製品のコードは 0 行**（Q0。`check-product-unchanged.sh` が試験モジュールの範囲の外を数えて見る）。変えたのは試験・試験の補助・台本・設定の上限だけ。

## deep.md に積んだ未決（A）

なし。

## 仮決め（B）と Ruling

全 Task の ledger（`.superpowers/sdd/ch09-task-*/`）に `Ruling:` の行は無い（Task の review の指摘は final review と re-review で処置した）。
finish の scoped re-review で park した Minor 2 件の Ruling（`review/code.md` の末尾の節）:
- R20 の「1 回で届いた歩が 1 つ以上」は、DB を待つ間の時計の自動の進みとの競争に、仕組みの上では左右されうる。Ruling: 間隔を +1 秒に効かせる壊す差分で落ちる（10 歩とも 2 回）ことを確かめた。`break-proof.sh` へ足して繰り返し確かめるのは次の change で
- 期限の検査が拾わない形（構造体の欄・型注記つきの束縛・定数・3 行以上に折れた右辺・`tokio::time::timeout`・`$(( $(date +%s) + N ))`）。Ruling: いまの木に該当なし。広げるのは該当が出たとき

design の（仮）:
- D1〜D6・D9・D15〜D17: 上流で決めた仮（試験の確かめ方・洗い出しの範囲・待ちの上限 30 秒・打ち切りの相手と上限・画面の上限・台本と計測テストの待ち）
- **D18**（code review R17。今回）: 見回り 1 回に積む 2,000 件は試験で `assert_eq!(QUEUE_PER_TICK, 2_000)` として固定する。打ち切りの相手 60 秒・上限 30 秒は一覧の検査に足さない（archive の後は、上限の比べの行を `check_agreed_tests.py` が守る。相手の `pg_sleep(60)` は守られない）

台帳（`provisional.jsonl`）は D18。別の席の確かめはグラフの `provisional_review` が書く。

## Task

13 Task・22 項目すべて `[x]`。残した Task は無い。人間の確認待ちは 0。

## 通した検証

`evidence.jsonl`（finish の直しの後の木 5c7b8b6c0 で取り直したもの）:
- 1.1 `tools/load-repeat.sh --self-test`（KILL の例を含む）・2.1 `tools/break-check.sh --self-test`・3.1・3.3（collector-windows の 2 本）・4.1・4.3（`archive_scan_interval_`）・6.1 `npx vitest run`（513 本）・7.1（携帯の単体試験）・13.1（製品不変の検査）→ rc=0
- 3.2 `break-proof.sh history`・4.2 `break-proof.sh scan` → 壊した木で期待どおり落ちる（rc=0）
- 10.1 `tools/android-emulator.sh`（計測テスト全部。R26・R27 の待ちを含む）→ rc=0（1023 秒）
- 13.3 `load-proof.sh after` → rc=0（history は DB の負荷・scan は host の負荷で、どちらも `pass 10/10`。`load-after.txt` を取り直した）
- 12.1・13.2 は本文に backtick のコマンドが無いので記録されない。手元で `check-inventory.sh`（`--self-test` を含む。166 ファイル）・`check-test-names.sh` → rc=0

finish の側で走らせたもの（rc=0）: `cargo fmt --check`・`cargo clippy --workspace --all-targets -D warnings`・windows-gnu の clippy・`cargo test --test server_startup`（39 本）・`compileDebugAndroidTestKotlin`、間隔を +1 秒に効かせる壊す差分（`変えた 1 秒が効いていない` で落ちる）。
走らせていないもの: Windows の実行時テスト（8.1。finish では `runtime_windows.rs` を変えていない。code_verify の時点で 15 本中 12 本 ok・3 本は Edge の窓が見つからない環境の落ち —— 分岐点の版でも同じに落ちる）・e2e（9.1。finish では変えていない。code_verify の時点で 244 passed）・smoke（11.1。同じく code_verify の時点で rc=0）・`cargo test --workspace`（13.4。code_verify の時点で 2,256 passed）。

## レビュー（`review/code.md`）

- final review R1〜R13: 全部 fixed（1 回の fix の後、re-review で ADDRESSED）
- code_verify R14〜R18・/code-review R19〜R28（security review は指摘なし）: 全部 fixed（R17 は D18 仮、R22 は「永久に止まる」の部分だけ rejected —— `assert!(sec < 86_400 + 60_000)` が 6 万回で落とす）。主な直し:
  - 製品不変の検査が、試験モジュールの閉じの後ろの製品の行も数える（R14）
  - 一覧の検査が、Rust の実時間の期限・Kotlin の `get`／`Tasks.await`・台本の `SECONDS` の期限・期限より先に尽きる回数を見る（R15・R19・R24）。強めた検査で `server_startup.rs` の 20 秒の期限 5 か所が見つかったので、30 秒にした
  - 間隔の変更の試験・見張り・止めた読みの試験が、ゆるめた分をまた見るようにした（R20〜R23）。画面の上限は、実際に効いている `task.timeout` で見る（R16）
  - 負荷の台本は、KILL で止められても負荷を残さない（R18）。壊した木は、受け継いだ `CARGO_TARGET_DIR` で組み立てない（R25）
  - 計測テストで 30 秒を毎回使い切る待ち 2 か所を、条件の待ちと短い窓に分けた（R26・R27）
- scoped re-review（1 回）: R14〜R28 は全部 ADDRESSED、新しい Critical / Important なし。Minor 3 件のうち 1 件（13.3 の取り直し）は済ませ、2 件は上の Ruling で park した

## 合意した試験の書き換え

`agreed-test-changes.md` に、期待の行を替えた Scenario を理由つきで挙げた（`check_agreed_tests.py` → OK）。

## 未処置の申し送り

- 上の park 2 件（R20 の壊す差分を `break-proof.sh` へ足す・期限の検査の拾う形を広げる）
- Windows の机の上に残る `msedge.exe`（相手役 `Edge` の `Drop` が子の木だけを止める形。分岐点にもある。CH09 の範囲外。code_verify の 8.1 の脚注）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
