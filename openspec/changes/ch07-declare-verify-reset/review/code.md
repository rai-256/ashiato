# CH07 下流のコードレビュー

## final review（1954021..d81a260）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。review package は
`.superpowers/sdd/tasks/review-19540213..d81a2601.diff`（11 commit）。全 Task の ledger（`progress.md` は無く、deferred-minor / parked / Ruling の行は 0）と、
全 Task の指摘の記録の Minor / ⚠️ を指した。判定は **Yes**（Critical 0 / Important 0 / Minor 3）。

申し送り（`docs/handoff/CH07.md`）は無い。

⚠️ の確かめ（final reviewer）: Task 1 の CH06 C4 は hunk の外で不変 —— OK。Task 3 の証跡の head 7d252409 —— 3.1 の tree の
`check-verify-env-reset.sh` の blob が e66fa37b と一致し、4.3（7b9db6d1）の tree の 2 本の blob は d81a2601 と一致 —— OK。
Task 4 の D5 の注記（`tools/verify-env.sh:4`）—— OK。4.2 の `.harness2` の SHA `aaecd2c6…` は log と `evidence.jsonl` にあり、いまの `.harness2` の HEAD と一致 —— OK。
Task の Minor（Task 1 F2・Task 2 F2・Task 3 F1 / F2）はそのまま残す（理由は final reviewer の triage: 冪等・3./4./10./11. の段が壊し方を弁別する・tasks 3.1 の明示）。

## R1. `--self-test` の --plan の段のラベルが、印があると「印ありHARNESS_VERIFY=1」に展開される
- 成果物: tools/check-verify-env-reset.sh:471（d81a2601）
- 根拠: `${mark:+印あり}${mark:-印なし}` —— `${mark:-…}` は mark が空でなければ mark の値になる。`--self-test` の出力で確認（Task 2 F1 と同じ）。判定には影響しない
- kind: technical
- 処置: fixed 2.1 —— `$([[ -n $mark ]] && echo 印あり || echo 印なし)` に（5a5bb003。`--self-test` rc=0、出力が「demo（CI なし・印あり）…」）

## R2. 本体の注記が「design D8 の 1.〜8.」のままで、ch07 の 9.〜11. と合っていない
- 成果物: tools/check-verify-env-reset.sh:16・:5（d81a2601）
- 根拠: body() の上の注記。9.〜11. は ch07 で足した（Task 4 F2 の積み残し）。:5 の「CI と手元で同じ結果」は :9 の例外と並べないと読み違える
- kind: technical
- 処置: fixed 3.2 —— 「1.〜11.。1.〜8. は design D8、9.〜11. は ch07」と「（例外は下）」（5a5bb003）

## R3. 10. の途中で abort すると、`$tmp` が消えて tmpfs の DB のコンテナが残る
- 成果物: tools/check-verify-env-reset.sh の 10.（volume を持たない DB のコンテナ）
- 根拠: 後始末は trap の `$tmp` の削除だけ。残ったコンテナは次の準備が作り直す（design D3）ので収束する
- kind: technical
- 処置: rejected: 次の `tools/verify-env.sh` が宣言なしで作り直して収束する（design D3）。害は残らないので直さない

## scoped re-review（d81a260..5a5bb00）

R1・R2 とも ADDRESSED、fix の差分に新しい破損なし（`.superpowers/sdd/tasks/review-d81a2601..5a5bb003.diff`、報告 `.superpowers/sdd/ch07-final-fix-report.md`）。
