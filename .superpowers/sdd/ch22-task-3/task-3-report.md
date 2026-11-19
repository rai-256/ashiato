# Task 3: 文書 — 実装報告

## 実装内容

- `docs/testing.md` の CI job 表の `install-windows` 行に、インストールと更新の試験後に配る形を外につないで 5 分置く「置く段」と、起こし直し・停止・同じ誤りの続き・panic・配布元への確かめを観測する内容を追記した。
- 「release の台本」の一覧に `tools/release/soak-install.sh` を追加した。
- `soak-install.sh` が観測の部品を使い、自己試験を `rust` job で走らせることを文書化した。

## テストと結果

- `scripts/verify-run 3.1` — PASS、rc=0
- `bash -c 'grep -q "soak-install.sh" docs/testing.md && grep -q "置く段" docs/testing.md'` — PASS、rc=0
- `git diff --check` — PASS

## TDD Evidence

### RED

コマンド:

```text
bash -c 'grep -q "soak-install.sh" docs/testing.md && grep -q "置く段" docs/testing.md'
```

結果は `rc=1`。追記前の `docs/testing.md` に必要な文言が無かったため、期待どおり失敗した。

### GREEN

コマンド:

```text
scripts/verify-run 3.1
```

結果は `3.1 PASS rc=0`。追記後の指定検証が成功し、`openspec/changes/ch22-soak-install-online-ci/evidence.jsonl` に証跡を記録した。

## 変更ファイル

- `docs/testing.md`
- `openspec/changes/ch22-soak-install-online-ci/evidence.jsonl`
- `.superpowers/sdd/ch22-task-3/task-3-report.md`

## Self-review

- Task の対象である文書だけを変更し、製品コード・CI 定義・タスク本文は変更していない。
- `install-windows` の既存のインストール・更新試験の順序や、ST57 D18 の記述は変更していない。
- 指定検証と差分の空白検査は成功している。

## Issues / Concerns

なし。
