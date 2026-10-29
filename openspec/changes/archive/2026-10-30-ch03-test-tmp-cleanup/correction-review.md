## 8.3 2026-10-25T09:57:27+00:00
- 壊したもの: 分岐点からある試験 `broken_marker_is_an_error`（crates/collector-windows/src/marker.rs:146-156。`#[test]` と関数の全体）を消した。check-test-names.sh は commit 済みの差分（`origin/main...HEAD`）しか見ないため、使い捨ての worktree（`git worktree add --detach`）で HEAD の上に分岐していない commit を 1 つ作って走らせた。worktree は消し、ブランチの ref・作業ツリー・`git status --porcelain`・`git diff` は壊す前と同じ（確かめ済み）
- 実行: openspec/changes/ch03-test-tmp-cleanup/check-test-names.sh
- 結果: 落ちた
