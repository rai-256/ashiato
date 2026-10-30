# CH08 —— 消した記録の一覧に包まれた滞在を残し、作り直しで隠れた滞在の本文も消す（ST23 Q7・Q8）

change: `openspec/changes/ch08-erasure-nested-and-late-stays/`（capability `record-deletion`・`browsing-views`）。依頼と完了の判定は `docs/changes/CH08.md`。

## deep.md に積んだ未決（A）

なし。

## 仮決め（B）と Ruling

全 Task の ledger（`.superpowers/sdd/ch08-task-*/`）に `Ruling:` の行は無い（Task の review の Important は、どれも re-review で ADDRESSED）。

design の（仮）:
- D2〜D8: 上流で決めた仮（作り直しが隠す重なる滞在の見分け・L の台帳と元の操作・戻す口の断り方・起動時の手当ての対象・入れ子の項目・確認の口の件数・画面の文面）
- **D10**（final review R2）: 範囲の行が無い本文を消した滞在の窓（その日全体）は、1 日の並びの入れ子の親にしない
- **D11**（code review R12。今回）: 本人が消したことか戻したことのある滞在（削除の台帳に行がある）は、作り直しで隠れても本文を消さない。起動時の手当て（D5 の 3）と同じ条件。
  CH08 の前の戻す口で戻されて一覧に残った B が、CH08 の後の最初の作り直しで確認なしに本文を消されるのを防ぐ（Q2）
- **D12**（code review R19。今回）: 窓だけに覆われる、本文を消していない消した滞在は、黙って落とさず自分の「消した」の区間として並べる（窓の区間と重なって 2 行並ぶ）

台帳（`provisional.jsonl`）は D10・D11・D12。別の席の確かめはグラフの `provisional_review` が書く。

## Task

7 Task・9 項目すべて `[x]`。残した Task は無い。人間の確認待ちは 0。

## 通した検証

Task の証跡（`evidence.jsonl`）: 1.1〜7.2 の検証コマンドは、どれも最新の結果が PASS（rc=0）。
7.1 の `cargo test -p ashiato-server` は Task のときは rc=0 だった。

finish の直しの後（この commit の木）:
- `cargo test -p ashiato-server purge_` → rc=0（149 passed）
- `cargo test -p ashiato-server` → **rc=101（1750 passed / 1 failed）**。落ちたのは `archive_scan_interval_change_takes_effect_without_restart` だけで、単独では通る（実時間で走査を待つ ST13 の試験。CH08 は触れていない。下の申し送り）
- `cargo clippy -p ashiato-server --all-targets -- -D warnings` → rc=0、`cargo fmt --check` → rc=0
- `check_scenarios.py` → `scenarios: OK`、`openspec validate --strict` → valid、`check-shared-appends.sh` → rc=0、`review_triage.py` → OK
- 新しい試験 3 本（`purge_hidden_restored_stay_keeps_its_body`・`purge_nested_day_window_does_not_drop_an_erased_stay`・`purge_remedy_startup_path_empties_every_users_overlapping_stay`）は、直しを外すと 3 本とも落ちることを確かめた

## レビュー（`review/code.md`）

- final review R1〜R11: fixed 8（R2 は D10 仮）・rejected 3（前の段で処置済み）
- code-verify R12〜R15: fixed 3（R12 は D11 仮）・followup 1（R14）
- security review: 指摘なし
- /code-review R16〜R25: fixed 4（R19 は D12 仮。R24 は Rust の側だけ）・rejected 6。
  R16 は一時の試験で再現しなかった（作り直しは本文ごと消した時間の両端の位置に後着の印を付けてから滞在を作るので、戻した滞在は P に触れない）
- 直しの scoped re-review（1 回）: 指摘なし

## 未処置の申し送り

- `docs/handoff/ST13.md` —— `archive_scan_interval_change_takes_effect_without_restart` が全体の試験の負荷の下で落ちる（R14）。走査の回数を実時間で 5 秒しか待たない。取り込み器の側の change で拾う

🤖 Generated with [Claude Code](https://claude.com/claude-code)
