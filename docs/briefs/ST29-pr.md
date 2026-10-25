# ST29 衛星を登録すると、衛星ごとの資格情報が発行される（実装）

change: `openspec/changes/st29-satellite-credentials/`。capability `satellite-platform`（新規）・`data-sensitivity`・`data-durability`（MODIFIED / ADDED）。

## 深掘りに積んだ未決（A）

なし（`deep.md` に `(未回答)` は無い。finish の処置で escalated も 0）。

## 仮決め（B）と Ruling

design の（仮）（台帳 `provisional.jsonl`。別の席の確かめは merge gate の前にグラフの `provisional_review` が行う）:

- **D1（仮）** 資格情報は 1 本ずつ番号を持ち、取り消した 1 本の行は消さずに取り消しの時刻を付ける
- **D2（仮）** 本体の前置きの形（`c03-` など）で始まる識別子は登録の時に断る
- **D3（仮）** モノの型（FR-86）の名前に ST29 の名前の形を当てない（ST40 で問う）
- **D4（仮）** 読み出しの記録の資格情報の種類に「衛星」を足し、衛星と 1 本の番号の欄を足す
- **D15（仮）** 長さ: 識別子 64・名前の部分 1〜64・衛星の名前 1〜40・資格情報の名前 40 文字まで
- **D16（仮）** 外していない衛星どうしで名前の重なりを断る
- **D17（仮）** 取り消しは確認をはさまず、取り消した 1 本は一覧に出さない
- **D18（仮）** 衛星の資格情報とログインの印を両方持つ求めは、衛星の求めとして 403
- **D19（仮）** `ashsat_` で始まる値が引けない求めは、ログインの印を見ずに 401
- **D20（仮）** 衛星の登録と資格情報の発行は別の操作（登録しただけでは 0 本）
- **D21（仮）** 正典の書き出しのファイルの一覧の書き漏れ（ST17 / ST21）を同じ Scenario の MODIFIED で直す

ledger の `Ruling:`（全 Task。`st29-task-7/progress.md` の 3 行が全部）:

- Task 7: parked — F1（7.1 の `CT export_ 60` の通し）— Ruling: 人間が受け入れた（2026-10-24）。専用 DB を作り直した通しで 131 passed / 1 failed。落ちた `export_ledger_migration_applies_twice` は ST33 の「移行は MIGRATIONS の末尾」の決めつけで、main では df75d2c1（ST13）が直し済み。ST29 のブランチが未取り込みなだけ
- Final review: parked — Minor 11 件（`review/code.md` の「Minor」節）— Ruling: merge 前に直さない — final reviewer が Minor とし、SDD の fix は 1 回だけ — 誤っていれば次の Story / change で拾う小さな手直し
- Final review: Ruling: Task 7 F1 の parked は解消 — ブランチの `export_ledger_tests.rs` は main の df75d2c1 と同じ字面、merge-tree で衝突なし — 誤りなら統合した木の CI で落ちる

## Task

10 Task・23 項目すべて `[x]`。残した Task は無い（`ci` / `human` の項目も無い）。

## レビュー（`review/code.md`）

- final review: R1（main と統合すると HEAD の口で全 route の試験が落ちる）・R2 を fixed（a3170df6）。Minor 11 件は park
  - **R1 の HEAD の分岐は、このブランチの契約に head の口が無いので、main と統合した木で初めて通る**
- code_verify: R3〜R5 → 全部 fixed（bed15ab7）
  - R3: 起動のたびに読み出しの記録の CHECK を付け直していたので、後の Story が種類か結果を足して行を書くと次の起動でサーバが上がらなかった → この版の語を含む定義がまだ無いときだけ付け替える（design D9 も同じ形へ）
  - R4: 「保存は値全体の SHA-256」を実装の外（DB の `sha256`）で確かめる
  - R5: 「行は消せない」の試験が共有の試験の DB に TRUNCATE を撃って、並んだ試験を deadlock させていた（完了の判定のテストが 3 回に 1 回落ちた）→ 使い捨ての DB へ。3 回続けて 43 passed
- security review: 指摘なし
- `/code-review`: 10 件 → fixed 4（R7 当て直しの全走査・R8 番号の片方だけの行・R13 取り消しの busy・R15 一覧の振り分け）、rejected 6（R6・R9・R10・R11・R12・R14。根拠は各節）
- finish の fix の scoped re-review: Critical / Important なし。Minor 4 件は park（`review/code.md` の末尾）

## 通した検証（`evidence.jsonl`。すべて exit 0 / PASS）

- 1.1 `tools/check-migrations.sh` / 1.2 `tools/check-immutable.sh`（出力に satellite）
- 5.3 `tools/check-log-private.sh` / 6.1 `cargo test -p ashiato-server every_route_refuses_satellite`
- 8.1 `tools/check-satellite-boundary.sh`（と `--self-test`）/ 9.1 `cd web && npm run lint && npm run test -- satellites`
- 10.1 `tools/check-openapi.sh` / 10.2 `tools/smoke.sh` / 10.3 `check_scenarios.py`・`review_triage.py`・`check_chain.py`
- 10.4 `cargo fmt --all --check`・`cargo clippy --workspace --all-targets -- -D warnings`・`cargo test --workspace`・web 一式（lint・test・build・e2e）・`check-immutable`・`check-log-private`・`check-satellite-boundary`・`check-openapi`・`smoke`

finish の fix（bed15ab7）の後に走らせ直したもの（すべて rc=0）: `cargo fmt -p ashiato-server -- --check`・`cargo clippy -p ashiato-server --all-targets -- -D warnings`・
`cargo test -p ashiato-server` の `satellite_`（43 ×3）・`access_log`（11）・`web_session`（19）・`export_satellite`（3）・`exempt_routes`（1）・
`tools/check-migrations.sh`・`tools/check-immutable.sh`・`cd web && npm run lint && npm run test -- satellites`（8）・`STACK_RESET=1 npm run test:e2e -- settings-satellites`（8 passed）。
前の移行に戻すと、足した断言（R8）と当て直しの試験（R3）が落ちることも確かめた。workspace 全体の `cargo test` は fix の後には走らせていない（CI が回す）。

## 申し送り（未処置のもの）

- `docs/handoff/ST29.md` の st28 Q4（収集側の合言葉が全記録を読める範囲）は ST44 へ移っている（`docs/stories/ST44.md:14`）が、`docs/handoff/ST29.md` に「ST44 へ移した」の追記が無い
- st17 C2・st33 C4（`/feelings` `/feelings/reminder` `/export*` を衛星の資格情報で断る）は `every_route_refuses_satellite` で閉じた
- `/selftest/panic`（環境変数で開けたときだけ生える口）は `route_layer` の後で、access_log の middleware の外にある（既存。この差分は触っていない）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
