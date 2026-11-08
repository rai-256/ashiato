# Task 2 実装報告

## 実装したこと

- `docs/changes/CH12.md` の「依頼（逐語）」から README と 5 件の ADR を追加した。
- `CLAUDE.md`、`AGENTS.md`、`.github/workflows/ci.yml` に指定の 1 行をそれぞれ追加した。
- 既存の `tools/check-adr.sh` が実際の ADR 題名と `AGENTS.md` の見出しを検査できるよう、抽出と cleanup の不具合を最小修正した。
- `verify-run` の証跡を `openspec/changes/ch12-memory-decisions-adr/evidence.jsonl` に記録した。

## テストと検証

- `tools/check-adr.sh --verbatim`: PASS
- `tools/check-adr.sh --self-test`: PASS
- `tools/check-adr.sh`: PASS
- `tools/check-private.sh`: PASS
- 3 ファイルの差分が各 1 行追加であることの検査: commit 後に PASS

## TDD Evidence

この Task は新しい実装コードではなく、逐語文書・設定行・検査台本の変更であるため、Scenario ベースのテスト追加は対象外。実装前に `tools/check-adr.sh --verbatim` が ADR 不在で FAIL することを確認し、追加後に PASS を確認した。

## 変更ファイル

- `docs/adr/README.md`
- `docs/adr/202610030000-worktree-port-allocation.md`
- `docs/adr/202610080000-foundation-for-satellites.md`
- `docs/adr/202610110000-pure-crate-split.md`
- `docs/adr/202610110001-ui-surface-only.md`
- `docs/adr/202610110002-stricter-lint.md`
- `CLAUDE.md`
- `AGENTS.md`
- `.github/workflows/ci.yml`
- `tools/check-adr.sh`
- `openspec/changes/ch12-memory-decisions-adr/evidence.jsonl`

## 自己レビュー

逐語検査、形の自己検査、private 検査、CI 差分行数検査を確認した。ADR 本文は `--verbatim` で原文との一致を確認している。

## 懸念

なし。

## Fix report（review 1）

### 修正内容

- F1: `--verbatim` が6塊を最後まで検査し、塊数・名前の重複を検査するよう修正。不一致は全件について行番号、依頼側の行、ADR側の行を出すようにした。
- F2: self-test の子プロセスでは `check-private.sh` を実行しないようにした。
- F3: self-test の基準木が本番検査を通ることを確認し、9ケースすべてで期待する error 文言を確認するようにした。

### テストと検証

- RED: 2つのADRを壊して現行 `--verbatim` を実行。最初のパスだけを出し、2つ目のパス・行番号・両側の行を出さないことを確認した。
- GREEN: `bash -n tools/check-adr.sh`、`tools/check-adr.sh --verbatim`、`scripts/quiet-run adr-self-test -- tools/check-adr.sh --self-test`（rc=0、約12秒）、`scripts/quiet-run adr-main -- tools/check-adr.sh`（rc=0、約58秒）。
- `scripts/verify-run 2.1` と `scripts/verify-run 2.2` はともに PASS。`tools/check-private.sh` と3ファイルの差分検査も PASS。

### 自己レビュー

F1〜F3以外の指摘・仕様・既存ファイルは変更していない。`git diff --check` も PASS。
