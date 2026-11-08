# Task 4: 範囲と Scenario の突き合わせ

## 実装内容

- `origin/main...HEAD` の差分が、Task 4 で許可されたパスだけで構成されていることを確認した。
- `docs/changes/CH12.md` の「ADR と repo の今の食い違い（直していない）」節は、8 件の不一致と harness2 の ADR 未配置を含み、Task 4 の D3 に示された一覧と一致していることを確認した。
- `tools/check-adr.sh` の Scenario 印（本番 11 本、自己検査 2 本）と change の全 Scenario を突合した。

## テスト結果

- `scripts/verify-run 4.1`: PASS / rc=0
  - 差分範囲チェック: PASS
  - `python3 scripts/check_scenarios.py . ch12-memory-decisions-adr`: PASS（`scenarios: OK`）
  - 証跡: `openspec/changes/ch12-memory-decisions-adr/evidence.jsonl`
- Scenario 突合の既存 warning（spec に無い印）は checker の許容 warning であり、Task 4 の change Scenario は未担保なし。

## TDD Evidence

この Task は新しい振る舞いを実装する Task ではなく、既存の差分範囲・文書一覧・Scenario 印を突き合わせる監査 Task であるため、RED の追加テストは該当しない。

- RED: N/A（新規テスト対象なし）
- GREEN: `scripts/verify-run 4.1` が rc=0、`scenarios: OK`

## 変更ファイル

- `.superpowers/sdd/ch12-task-4/task-4-report.md`
- `openspec/changes/ch12-memory-decisions-adr/evidence.jsonl`（`verify-run 4.1` の証跡追記）

## 自己レビュー

- Task 4 の本文にある検証コマンドを `scripts/verify-run 4.1` で実行した。
- `tasks.md`、ADR 本文、`docs/changes/CH12.md` の「依頼（逐語）」は変更していない。
- Scenario 名の追加や既存テスト期待値の変更は行っていない。

## 懸念・課題

なし。
