# 機械が承認した検証コマンドの訂正の、壊しての確かめ

## 5.3 2026-10-22T11:45:58+00:00
- 壊したもの: `docs/handoff/ST23.md` の申し送りの節 `## ch01-st17-review-fixes D9 — 本文を消した滞在を紐づけ先にする気分`（docs/handoff/ST23.md:109 から末尾まで）を作業ツリーで一時的に削った（ST23 の brief に CH01 の申し送りが届かない状態）。走らせた後に元へ戻し、`git status --porcelain` と `git diff` が壊す前と同じ（どちらも空）ことを `cmp` で確かめた
- 実行: tools/check-handoff-reaches-brief.sh ST23 ch01-st17-review-fixes
- 結果: 落ちた
