# CH12 —— メモリにしか無かった判断を ADR に置く

change: `openspec/changes/ch12-memory-decisions-adr/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D2（仮）. 足すのは 6 ファイル・`CLAUDE.md` と `AGENTS.md` の 1 行ずつ・ADR の形の検査 `tools/check-adr.sh`・CI の chain job の 1 行（Q3。既定 C2 を改めた）
- D3（仮）. 食い違いは直さずに `docs/changes/CH12.md` の節で挙げる（Q1）
- D4（仮）. 完了の判定は repo の中のコマンドで確かめ、harness2 の側を待たない（Q2）
- D7（仮）. agent への案内は `CLAUDE.md` の表の 1 行と、`AGENTS.md` の 1 行の指示（Q4）

全 Task の ledger の `Ruling:`（逐語）:
- 全 Task の ledger に `Ruling:` の行は無い。

## Task

4 Task のうち、埋めた Task は 4 本。
残した項目は無い。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `bash -n tools/check-adr.sh` → rc=0（PASS・head fee620f）
- 2.1 `tools/check-adr.sh --verbatim` → rc=0（PASS・head 6ad6e50）
- 2.2 `tools/check-adr.sh` → rc=0（PASS・head 6ad6e50）
- 2.2 `tools/check-adr.sh --self-test` → rc=0（PASS・head 6ad6e50）
- 2.2 `tools/check-private.sh` → rc=0（PASS・head 6ad6e50）
- 2.2 `bash -c 'for f in CLAUDE.md AGENTS.md .github/workflows/ci.yml; do test "$(git diff --numstat origin/main...HEAD -- "$f" | cut -f1,2)" = "$(printf "1\t0")" || { echo "error: $f の差分が 1 行の追加でない"; exit 1; }; done'` → rc=0（PASS・head 6ad6e50）
- 3.1 `tools/check-adr.sh --backlog` → rc=0（PASS・head 0a87b17）
- 3.1 `tools/check-adr.sh --self-test` → rc=0（PASS・head 0a87b17）
- 4.1 `bash -c 'd=$(git diff --name-only origin/main...HEAD) || exit 1; test -n "$d" && test -z "$(printf "%s\n" "$d" | grep -vE "^(docs/adr/|CLAUDE\.md$|AGENTS\.md$|\.github/workflows/ci\.yml$|tools/check-adr\.sh$|openspec/|docs/changes/CH12\.md$|docs/briefs/|\.superpowers/)")"'` → rc=0（PASS・head 49b0021）
- 4.1 `python3 scripts/check_scenarios.py . ch12-memory-decisions-adr` → rc=0（PASS・head 49b0021）

## レビュー（`review/code.md`）

- 指摘 27 件（escalated 1・fixed 23・rejected 3）
- 処置の re-review: 6 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
