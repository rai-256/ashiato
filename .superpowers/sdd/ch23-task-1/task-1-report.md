# Task 1 実装報告

## 実装内容

- `tools/check-vulns-issues.py` の `gh_json` に空応答の既定値を指定できる引数を追加した。
- `label list` のみ空の stdout を空のラベル一覧として扱い、ラベルが無い場合に作成処理へ進めるようにした。
- 自己試験の偽の `gh` を本物の `gh` に合わせ、ラベル無し時は rc=0 で空出力にした。
- ラベル作成順、既存ラベル、空の `issue view` 応答の3 Scenarioを自己試験へ追加した。

## テスト結果

- RED: `scripts/verify-run 1.1`（実装前） — FAIL。`== Scenario: 落ちた夜に、依存ごとに 1 件の issue が立つ` の後に `gh の応答: Expecting value` が出て、osv.dev 到達失敗ではないことを確認した。
- GREEN: `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` — rc=0、PASS。
- GREEN: `scripts/verify-run 1.1` — PASS。
- GREEN: `scripts/verify-run 1.2` — 2つの検証コマンドとも PASS。
- `git diff --check` — PASS。

## 変更ファイル

- `tools/check-vulns-issues.py`
- `tools/check-vulns.sh`
- `openspec/changes/ch23-vulns-nightly-issue-fix/evidence.jsonl`

## 自己レビュー

空応答を許可する呼び出しは `label list` だけであり、`issue list` と `issue view` は JSON 応答を要求する。偽の `gh` の空応答切り替えは新しい `GH_OPEN_ISSUE_EMPTY` 環境変数でのみ有効になる。追加した Scenario は `gh.log` の順序、ラベル再作成抑止、空応答時の rc=2 をそれぞれ確認している。

懸念事項なし。

## 修正報告（review 1 F1）

- `tools/check-vulns.sh` の「開いた issue のコメントの問い合わせが空の文字なら、rc=2 で落ちる」Scenario で gh のログを事前に空にした。
- stderr を一時ファイルへ捕捉し、`error: issue を記録できなかった（gh の応答:` の出力と `issue view 7` の呼び出しを検査するようにした。
- テスト: `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` — rc=0、PASS。
- 検証: `scripts/verify-run 1.2` — Scenario 名の検査 rc=0、自己試験 rc=0、証跡を `openspec/changes/ch23-vulns-nightly-issue-fix/evidence.jsonl` に記録。
