## 6.1 2026-11-10T09:54:06+00:00
- 壊したもの: `.github/workflows/ci.yml:428` の `package-windows` の step から、末尾の `; cargo test -p ashiato-setup ch14_`（試験を実際に走らせる部分）を消し、`-- --list` で数えて下限を見るだけの step にした（試験が 1 本も走らない CI）。別に (b) その step を最初の `tools/release/package.sh windows` の step の後ろへ動かす・(c) 下限を `-ge 11` から `-ge 1` に下げる、も試した。どれも確かめた後に `/tmp` の控えから戻し、`git status --porcelain`・`git diff` の md5 が壊す前と同じであることを確かめた（壊す前から `.superpowers/sdd/ch14-settle/` の 2 件が ` D` で、それも変わらない）
- 実行: tools/check-ci-ch14-step.sh
- 結果: 落ちた
