## 6.1 2026-11-10T09:54:06+00:00
- 壊したもの: `.github/workflows/ci.yml:429` の `package-windows` の step から、末尾の `; cargo test -p ashiato-setup ch14_`（試験を実際に走らせる部分）を消し、`-- --list` で数えて下限を見るだけの step にした（試験が 1 本も走らない CI）。確かめた後に `/tmp` の控えから戻し、`git status --porcelain`・`git diff` が壊す前と同じく空であることを確かめた
- 実行: tools/check-ci-ch14-step.sh
- 結果: 落ちなかった
