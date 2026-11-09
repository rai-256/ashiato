# CH12 の処置の fix report（settle が追記する）

## settle（re-review 4 の New Breakage: R25・R26）

- R25（fixed D3）: `design.md` の Risks の harness2 の ADR への参照の行を「上流の時点では行き先が無かった → 2026-11-08 に main に入り解けた（`1e140a4b3`）。いまはあることを CH12.md の節の末尾に書いた」にし、「PR 本文に書く」を消した。食い違いの行の置き場も CH12.md の節に直した
- R26（rejected）: `scripts/pr_body.py` を読んだ。`build` は食い違いの一覧の節を持たず、前の本文（`old`）から引き継ぐのは `Ruling:` の行だけ（`rulings`）。9e70f08 の diff で 8 件の一覧が消えたのが再現。手で CH12-pr.md に足しても次の `finish` で消えるので、成果物は直さず、design D3（仮）に 2026-11-09 の追記（一覧の置き場は `docs/changes/CH12.md` の節だけ。PR の差分に入る）を足し、CH12.md の完了の判定の文を合わせた
- 走らせたもの: `python3 scripts/review_triage.py . ch12-memory-decisions-adr --no-verdict` → `triage: OK` rc=0 / `tools/check-adr.sh` → rc=0

## settle（re-review 5 の New Breakage: R27）

- R27（fixed D3）: `design.md` の D3 の見出しを「食い違いは直さずに `docs/changes/CH12.md` の節で挙げる（Q1）」に変えた。本文の「これらを `docs/briefs/CH12-pr.md` に一覧で挙げる」を「CH12.md の `## ADR と repo の今の食い違い（直していない）` の節に一覧で挙げる（置き場はここだけ。理由は下の追記）」に、harness2 の `docs/adr/` の括弧の「PR 本文と CH12.md の節には」を「CH12.md の節には」に、「後で見つける置き場」の「同じ一覧を…にも置く」を「一覧は…に置く」に直した。2026-11-09 の追記（R26）は経緯として残した。tasks.md:10・16 は凍結した計画と本人の決定の写しなので触れていない（R27 の成果物は design.md だけ）
- 走らせたもの: `python3 scripts/review_triage.py . ch12-memory-decisions-adr --no-verdict` → `triage: OK` rc=0（初回は処置の行の `fixed D3（` の空白抜けで FAIL、直して OK）/ `tools/check-adr.sh` → rc=0
