# CH02 書き出しの「次の記録」の引きを行数の 2 乗で遅くならないようにする（下流・実装）

change: `openspec/changes/ch02-export-next-record-window/`。capability `data-durability`（MODIFIED 1・ADDED 1）。依頼の逐語と完了の判定は `docs/changes/CH02.md`。

`export/excluded.rs` の除く範囲の判定で、「次の記録」を行ごとの相関副問い合わせから、鍵の列だけを並べた 1 つの窓（`span_next`。逆向きの並びで頭を固定した `RANGE` 枠の `min`）に替えた。
消した滞在が 0 件なら記録の表を読まず、消した時間の終わりより後は組ごとの最初の 1 行だけを窓に入れる。移行・索引・API・画面の変更は無い。

## 未決（A）

なし。

## 本人に見てほしい点

- **意図した挙動の変更は 1 点だけ**: 同じ時刻に並んだ記録の先のほうが、除かれる側に変わる（Q1）。除く側にだけ増え、狭まらない
  （security review が乱数の配置 6 周 × `snapshot_at` 4 通りで `旧が除く ⊆ 新が除く` を確かめた）
- **ST33 D13（件数の数え 3 秒）の判定**: 120 万行・消した滞在ありの 1 通りで、件数の数え（`summary_seconds`）が **25.5 秒**（design の「計測の記録」の表）。
  形を替えるかは本人が決める（D8）。替えるなら、書き出し 1 回で窓を文ごとに組み直している分（/code-review R6。全体 146.5 秒のうち約 49 秒）もあわせて候補になる
- **ST09 の上での `export_` の試験は確認バッチの統合の木に任せた**（tasks 6.2 の訂正。2026-10-23 に本人が承認。`plan-corrections.md`）

## 仮決め（B）と Ruling

全 Task の ledger（`.superpowers/sdd/ch02-task-*/progress.md`）は無く、Task の報告・指摘にも `Ruling:` の行は無い。
`design.md` の（仮）と反転条件:

- **D3** 窓の前に時刻だけで絞る（0 件の門・`max(end_at)` 以下と組ごとの後の最初の 1 行）。反転条件: 突き合わせで読み違いが残るなら外して全行に窓を当てる
- **D7** 線形の固定は読んだ行の数を N と 4N で比べる（新 4.0 倍・旧 47.9 倍。上限 8 倍）。窓の枠は断片の文字列で固定。反転条件: 読んだ行の数が計画で揺れるならバッファの数に替える
- **D8** 時間は量の試験で 1 万・10 万・120 万行 × 消した滞在あり・無しを 1 回測り、ST33 D13 の判定に渡す（上の「見てほしい点」）
- **D9** `docs/changes/CH02.md` の `capability` は空のまま
- **D10** ST09 の上での試験の走らせ方（6.2 の訂正で、確認バッチの統合の木に任せる形に替わった）

## Task

6 Task・18 項目すべて `[x]`。残した Task は無い（画面を持たないので e2e・人間の確認待ちも無い）。

## 通した検証（`evidence.jsonl`）

| 項目 | コマンド | 結果 |
|---|---|---|
| 1.3 | `CT ch02_span_legacy` | PASS（2026-10-22） |
| 2.2 | `CT ch02_span_` / `CT export_excluded` | PASS |
| 3.4 | `CT ch02_same_` | PASS |
| 4.4 | `CT ch02_scale_` | PASS |
| 5.1 / 5.2 | `CT export_volume_one_run_reports_time_and_memory` / design の表に空欄なし | PASS（5.2 は 1 回 FAIL の後に表を埋めて PASS） |
| 6.1 | `CT export_excluded` / `export_file` / `export_job` / `export_ledger` / `export_tests` / `ch02_` | PASS |
| 6.2 | `CT export_`（148 本）・`git status --short` が空 | PASS（2026-10-23T09:33。訂正前の `CT st09_exp_` は 0 本で FAIL） |
| 6.3 | `cargo test -p ashiato-server` / clippy `-D warnings` / `check_scenarios.py` | PASS（2026-10-23T09:13〜09:23）。下の申し送り 1 のとおり、共有の試験 DB では 1 本落ちることがある |

`CT` は tasks.md の Global Constraints の定義（`.env` を読んで `scripts/quiet-run ct -- cargo test -p ashiato-server <名前>`、`-- --list` の本数を下限と比べる）。
finish の直し（R12 の doc・R13 の使わない列の削除）の後に `CT ch02_`（19 passed）/ `CT export_excluded`（24 passed）/ `CT export_file`（44 passed・1542 秒）/ clippy `-D warnings` を走らせ直し、全部 rc=0（2026-10-24）。

## レビュー

`review/code.md`:
- final review: Ready to merge（Minor 4。fixed 3・rejected 1）
- code_verify: 変異 9 通り（M1〜M8）で決定の固定を確かめた。R5（6.3 の crate 全体）だけ不一致 —— 原因は CH02 の外（申し送り 1）
- security review: 指摘なし
- /code-review: 10 件 —— fixed 2（R12 doc・R13）・rejected 8（R6 は定数倍で実測済み、R7〜R11・R14 は CH02 の差分の外 → 申し送り 2、R15 は repo の慣行）

## 未処置の申し送り（持ち主が無い。`hx change` で起こすかを本人が決める）

1. **`day_tests::st25_reasons_before_collection_started` が共有の試験 DB で落ち続ける**（code_verify R5）。`c01-location` の `collection_started_on` が
   ほかの試験の取り込みで 2000 年まで下がると、2001-01-05 が「開始の後」になる。まっさらな DB と CI では緑。案: その試験だけの論理ソースか、まっさらな DB に向ける
2. **`tools/ports.sh` の台帳（c7a8737f で main に入ったもの。CH02 の差分の外）** ——
   R7 台帳の書き込み（`printf … > tmp && mv`）の失敗を拾わず、記録していない番号を返す（tools/ports.sh:36。読んで確かめた）／
   R8 台帳と lock が git の共通ディレクトリにあり、Codex の sandbox からは書けず `testdb.rs` が panic しうる／
   R9 `--path-format` は git 2.31 以上（この host は 2.53 で起きない）／
   R10 `_ports_root` が論理パスで、symlink 経由と実パスで別の番号になりうる（tools/ports.sh:17）／
   R11 `crates/server/tests/server_startup.rs` の `with_worktree_port` が `testdb.rs` の呼び出しの写しで、毎回 bash と git を起こす／
   R14 `tools/record-env.sh:130` のコメントが「録画の worktree は 55432」のまま

🤖 Generated with [Claude Code](https://claude.com/claude-code)
