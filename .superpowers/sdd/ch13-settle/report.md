# CH13 の処置の fix report（settle が追記する）

## re-review 9 の後（R20・R21・R22）

- 変えたもの: `docs/briefs/CH13-pr.md` の 16 行目 `- 全 Task の ledger の `Ruling:`（逐語）:` を 1 行消した。
- 確かめ: `grep -n '全 Task の ledger' docs/briefs/CH13-pr.md` → `15:全 Task の ledger の `Ruling:`（逐語）:`（1 行だけ）。
- 戻る仕組み（3 回目の再発）: `.harness2/scripts/pr_body.py:67-69` の `rulings()` は前の本文の節から `"Ruling:" in ln` の行を拾う。
  台本が書く見出しの行（175 行目）が `Ruling:` を含むので、finish が本文を組み直すたびに見出しが箇条として戻る。
  この branch の直しは finish の次の再生成で必ず消える。根の直しは harness2 の `rulings()`（`Ruling:` で始まる行だけ拾う）。
  harness2 の main に直しは無い（`git log HEAD..origin/main -- scripts/pr_body.py` は空）。この席は harness2 に push・PR をしない。
