# CH03 テストの一時ファイルの後始末

change: `openspec/changes/ch03-test-tmp-cleanup/`（capability `test-hygiene`・Scenario 18 本）。依頼と完了の判定は `docs/changes/CH03.md`。
Rust の試験は `tempfile::TempDir`、Kotlin の単体試験は `@get:Rule` の `TemporaryFolder` で一時フォルダを持ち、落ちても消える。
手で作る書き方は `tools/check-tmp-hygiene.sh`（CI の chain job と commit 前）が止め、CI の rust・android の試験の段は
`tools/tmp-leak-check.sh` から走って、試験の後に一時フォルダへ残ったものがあれば落ちる。

## deep.md に積んだ未決（A）

- **Q9（第 2 回・premise。`review/code.md` R17）8.3 の検証コマンドの入口を、証跡の機械が拾えない** —— (未回答)。
  問いは `openspec/changes/ch03-test-tmp-cleanup/deep-questions-r2.json`（HTML `docs/briefs/CH03-deep-r2.html`）。
  tasks 8.3 の `openspec/changes/ch03-test-tmp-cleanup/check-test-names.sh` は、harness2 の `scripts/evidence.py:67` の CMD_LIKE（`./` `tools/` `scripts/` …）に当たらないため、
  verify-run が走らせられない。旧コマンドの FAIL の証跡も作れないので、plan_fix の機械の訂正も成り立たない。
  推奨は harness2 の CMD_LIKE に `openspec/` を足すこと。台本は HEAD で手で走らせて rc=0（rust 1805→1806・kotlin 626→628、分岐点の試験の名前は全部 HEAD でも試験）

## 仮決め（B）と Ruling

- Task の ledger（`.superpowers/sdd/ch03-task-*/progress.md`）は無く、`Ruling:` と parked の行も無い
- design の（仮）: D3（検査の範囲は置き場で決める）・D4（落とす書き方と印）・D5（CI の chain job と commit 前。commit 前は index の中身を見る `--staged`。R30）・
  D6（根は `/tmp` の下の `mktemp -d`。落ちた回・中断した回は `target/tmp-leak/<rust|android>-…` へ移し、種類ごとに 5 個まで残す。R1・R23・R24・R33）・
  **D9（仮・新）** 負荷で揺れる `archive_scan_interval_change_takes_effect_without_restart` は CH03 で直さない（R21）。
  反転条件: 分岐点の木で、同じ負荷の全体の走りが 3 回とも通れば、CH03 の差分が原因として CH03 で直す
- finish の scoped re-review の後に park したもの（`review/code.md` の末尾）:
  - R20 の残り: 名前の集合は名前だけを比べる。そのため、同じ名前の試験が別のファイルにあると（HEAD で rust 6・kotlin 6）、片方の注釈を外しても通る —— Ruling: 本数の比べ（分岐点より減らない）がそこを覆う。名前に path を含めるのは 8.3 の次の直しで行う
  - Minor: trap は TERM でも rc=130 を返す。`--no-run` が落ちた道は trap を外さずに return する（すぐ終わるので害は無い）。1 行に書いた `#[test] fn x()` を拾わない（今の木には無い）。R29 の Drop は `poll_outages` の `#[cfg(test)] wait()` との重ねがけ

## Task

8 Task・25 項目をすべて埋めた。残した Task は無い。

例外として 2 つを宣言する（R13）。どちらも振る舞いと期待値の意味は変わらない:
- 製品のファイルに入れたのは `#[cfg(test)]` の項目（`history/collect.rs` の `threads_finished`・`outage_notice.rs` の `Drop`）と、`media.rs:568` の `// tmp-ok:` のコメント 1 行だけ
- assert の書き換えは `day_tests.rs` の `st25_reasons_before_collection_started` の日付 1 か所だけ（合意は `agreed-test-changes.md`）

## 検証（`evidence.jsonl` の各項目の最後の記録）

- 9394a0b0: 4.1（`LK rust -p ashiato-collector-windows --lib` 283 本・残り無し）・8.4（check_scenarios・fmt+clippy・detekt・log-private+shared-appends）がどれも rc=0
- 39e5cf39: 1.3・4.2・7.1〜7.4・8.1 がどれも rc=0
- 571854cd: 8.1・8.2（rust 1788・android 629、残り無し）が rc=0
- 2026-10-23〜25 の各 Task の HEAD: 1.1〜1.5・2.x・3.x・5.x・6.x が rc=0（後で触ったのは tools と collector-windows の試験だけ）
- **8.2 は最後の HEAD では取り直していない。** code_verify が 4dec22bc で 2 回走らせ、2 回とも `archive_scan_interval_change_takes_effect_without_restart` の 1 本だけで rc=101 になった（R21・D9）
- **8.3 は機械の証跡が無い**（上の Q9）。最後の記録は d798b111 の PASS

## 未処置の申し送り

- **揺れる試験の change を起票する**（R16・R21）: `archive_scan_interval_change_takes_effect_without_restart`（`crates/server/src/archive_scan_interval_tests.rs:156-165`、25ms × 200 回の待ち）は、全体の走りの負荷（load average 3.9）の下で「走査が N 回に届かない」で落ちる。
  CI の rust job がこの台本を通すようになったので、CI の赤の元になりうる。この試験を持つ Story / change は無い。`hx change` で起票する（このセッションは起票していない）
- harness2 への申し送りが 2 つある:
  - (1) `evidence.py` の CMD_LIKE に `openspec/` が無い（Q9）。plan_fix.py が新しい入口を書く前に、evidence.py が拾えるかを確かめていない
  - (2) deep.md の答えが「答え: …（本人の選択）」の形のため、`provisional.py` の `answered()` が本人の答えとして読まず、`--follows Q5` が通らない（final の申し送り）
- finish の途中で、同じ CH03 finish のセッションが 2 本、同じ worktree を同時に触っていた（20:24 と 20:26 に起動）。片方がわざと壊して組んだ試験のバイナリが、mtime を戻したせいで古いまま使われた。`cargo clean -p ashiato-collector-windows` で解消した。グラフが finish を重ねて起こしていないか、harness2 で見てほしい

## レビュー

final（R1〜R17）・code_verify（R18〜R22）・security-review（R23）・/code-review（R24〜R33）の処置は `review/code.md`。`review_triage.py` は OK。
finish の fix は 1 回（4dec22bc..49f4da1f）、scoped re-review は 1 回。ADDRESSED は 14 件、PARTIAL は 1 件（R20。上の park）。fix が持ち込んだ退行 1 件（check-test-names.sh の本数の比べが `set -e` の下で効かなくなっていた）は 39e5cf39 で戻した。

🤖 Generated with [Claude Code](https://claude.com/claude-code)
