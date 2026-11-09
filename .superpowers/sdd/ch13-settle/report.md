# CH13 の処置の fix report（settle が追記する）

## re-review 11 の後（R20・R21・R22・R23）—— escalated

- 変えたもの: `docs/briefs/CH13-pr.md` は変えていない。`deep.md` に「下流（settle）で立てた問い」Q-R20 を `(未回答)` で足し、
  `deep-questions-r1.json` と `docs/briefs/CH13-deep-r1.html`（`ask_wizard.py` rc=0。止める 1）を置いた。code.md の R20〜R23 の処置を `escalated` に書き直した。
- 直さなかった理由（再現）: `.harness2/scripts/pr_body.py` の `main` は `old = docs/briefs/CH13-pr.md` を読み（196 行目）、`rulings()` が
  その「## 仮決め（B）と Ruling」の節から `"Ruling:" in ln` の行を拾う（67〜69 行目）。台本が書く見出しの行（175 行目）は `Ruling:` を含むので、
  ブランチでどう消しても、次の finish で見出しの行から箇条が作られる。消す直し（84b7cba・d3edcb8・2a244db）が 3 回とも finish（e2b3b06・3a87c39・8faa3ab）で戻った。
- harness2 の確かめ: `git -C ~/dev/harness2 fetch && git log HEAD..origin/main -- scripts/pr_body.py` → 空。open の PR・issue にも該当なし（`gh pr list` / `gh issue list --search rulings` → `[]`）。
- 根の直しの案（harness2。この席はしない）: 69 行目の条件を `ln.lstrip("- ").startswith("Ruling:")` にする。
- 確かめ: `python3 scripts/review_triage.py . ch13-workbox-config --no-verdict` → `triage: OK`（rc=0）。
