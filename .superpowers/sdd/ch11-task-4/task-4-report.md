# Task 4: dependency-review に渡す許可の一覧

## 実装内容

- `tools/check-vulns.sh allow-ghsas` を追加した。
- `osv-scanner.toml` の例外表を読み、`GHSA-` の ID はそのまま、その他の ID は osv.dev の `GET /v1/vulns/<id>` の `aliases` から `GHSA-` の別名だけを収集する。
- すべての問い合わせが成功してから、重複を除いた許可一覧をカンマ区切りで標準出力へ出す。問い合わせ・応答の失敗時は一覧を出さず rc=2 とする。
- 自己試験に、例外表と別名から組む Scenario、および問い合わせ不能時に一覧を出さない Scenario を追加した。

## テスト結果

- RED: `scripts/quiet-run task4-red -- tools/check-vulns.sh --self-test` — rc=1。既存 Scenario 通過後、新しい `許可の一覧は例外の表と別名から組む` で停止した。実装前は `allow-ghsas` が未接続だったため期待どおり。
- GREEN: `scripts/quiet-run task4-green2 -- tools/check-vulns.sh --self-test` — rc=0。27 Scenario の印を含む台本が通過。
- 指定検証: `scripts/verify-run 4.1` — PASS、rc=0。証跡は `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl` に記録済み。
- `git diff --check` — PASS。
- `python3 scripts/check_scenarios.py .` は、この Story の未実装 Task に対応する 23 Scenario が未担保のため FAIL。Task 4 の自己試験追加分は印を持つ。

## TDD Evidence

- RED は上記のとおり、実装前に新しい Scenario で失敗することを確認した。
- GREEN は上記のとおり、実装後に `tools/check-vulns.sh --self-test` が rc=0 となった。

## 変更ファイル

- `tools/check-vulns.py`
- `tools/check-vulns.sh`
- `.superpowers/sdd/ch11-task-4/task-4-report.md`
- `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl`

## 自己レビュー

- 問い合わせに失敗した場合は収集済みの ID を出力しないことを確認した。
- RUSTSEC の別名から GHSA だけを選び、表に直接ある GHSA と重複除去して出力することを確認した。
- Task の範囲外の workflow 変更や例外表の変更は行っていない。

## 懸念

- なし。

## Fix report（review 1 / F1）

- 指摘対応: allow-ghsas の自己試験から偽 curl と `CURL` 環境変数による差し替えを削除した。別名収集の Scenario は実際の osv.dev の応答を検証し、問い合わせ不能の Scenario は `HTTPS_PROXY=http://127.0.0.1:9 NO_PROXY=` で rc=2 と一覧なしを検証するようにした。
- 実装変更: `tools/check-vulns.py` は常に実在の `curl` を使い、テスト専用の差し込み口を削除した。
- GREEN: `scripts/quiet-run task4-fix-red -- tools/check-vulns.sh --self-test` — rc=0。27 Scenario が通過し、allow-ghsas の実 osv.dev 経路と proxy 失敗経路を含む。
- 指定検証: `scripts/verify-run 4.1` — PASS、rc=0。証跡を `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl` に追記した。
- 追加確認: `git diff --check` — PASS。偽 curl、`FAIL_PROXY`、`CURL` の参照は対象ファイルに残っていない。
