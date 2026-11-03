# Task 5 実装報告

## 実装内容

- `tools/check-nightly-migrations.sh` を追加した。
  - 浅い履歴を `nightly` の確認前に拒否する。
  - `nightly` が無ければ対象なしとして通す。
  - `nightly:migrations/ORDER` と前進側 `.sql` を取得し、現木と比較する。
  - `migrations/LINEAGE` の `removed` / `renamed` / `rewritten` を検査に反映する。
  - 10 個の指定 Scenario を使い捨て git 木で検査する `--self-test` を追加した。
- `.github/workflows/ci.yml` の `rust` job に、`nightly` が存在するときだけ指定 refspec を force fetch し、検査を実行する手順を追加した。

## テスト結果

- `tools/check-nightly-migrations.sh --self-test`: PASS（10/10 Scenario）
- `tools/check-nightly-migrations.sh`: PASS（nightly 無し）
- `bash -c 'grep -q "check-nightly-migrations.sh" .github/workflows/ci.yml && grep -q "refs/tags/nightly:refs/tags/nightly" .github/workflows/ci.yml'`: PASS
- actionlint の検証コマンド: PASS（ローカル `actionlint` は無く、指定 Docker fallback を使用）
- `bash -n tools/check-nightly-migrations.sh`: PASS
- `git diff --check`: PASS
- `scripts/verify-run 5.1`: self-test は検証準備の `tools/verify-env.sh` rc=1 で `BLOCKED_INFRA`、本体検査は PASS。証跡は `evidence.jsonl` に記録済み。
- `scripts/verify-run 5.2`: 2 検証とも PASS。

## TDD Evidence

- RED: 実装前に `tools/check-nightly-migrations.sh --self-test` を実行し、`No such file or directory`、rc=127 を確認した。
- GREEN: 実装後に `tools/check-nightly-migrations.sh --self-test` を実行し、10 個の Scenario と `check-nightly-migrations self-test OK` を確認した。

## 変更ファイル

- `tools/check-nightly-migrations.sh`
- `.github/workflows/ci.yml`
- `openspec/changes/st54-dev-side-by-side/evidence.jsonl`
- `.superpowers/sdd/st54-task-5/task-5-report.md`

## 自己レビュー

- 前進側 SQL は一時ファイル経由の `cmp` で比較し、戻し手順は比較対象にしていない。
- `nightly` の ORDER または対象 SQL を取得できない場合は非 0 で停止する。
- `check_scenarios.py` は本 Task 外の未実装 Scenario 47 件を報告したため、他 Task の責務を変更せず、そのままにした。

## Fix report（review 1）

- F1: `印 nightly の版の移行を取り出せなければ、検査は通らない` の self-test で、ORDER 無しの commit に `nightly` tag を付け、その後に現行木の ORDER を復元するよう修正した。これにより `git show nightly:migrations/ORDER` の失敗分岐を直接検証する。
- F2: self-test の `expect` に任意個の必須出力パターンを追加し、名前変更・名前変更と中身変更・中身変更の 3 Scenario で、仕様にある移行名と `直しの移行を足して rewritten を書く` を検証するよう修正した。
- F3 は final review で確認する対象のため変更していない。

### Fix verification

- `bash -n tools/check-nightly-migrations.sh && tools/check-nightly-migrations.sh --self-test`: PASS（10 Scenario、`check-nightly-migrations self-test OK`）。
- `scripts/verify-run 5.1`: PASS（self-test、本体検査）。
- `scripts/verify-run 5.2`: PASS（workflow の存在確認、actionlint fallback）。
- 実行コマンド: `scripts/quiet-run task5-verify -- bash -c 'scripts/verify-run 5.1 && scripts/verify-run 5.2'`、rc=0。
