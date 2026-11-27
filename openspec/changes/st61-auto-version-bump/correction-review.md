## 8.1 2026-11-23T03:37:33+00:00
- 壊したもの: tools/release/version.sh:113 の `git merge-base --is-ancestor "$commit" "$main" || die "印の commit が main の祖先でない"` を、base（e72125187 の version.sh:98-105）の修正の枝 `release/<左>.<真ん中>` の例外（`git for-each-ref --contains "$commit" 'refs/heads/release/*' 'refs/remotes/origin/release/*'` に当たれば main の祖先でなくても通す）に一時的に戻した。作業ツリーで壊し、`git checkout -- tools/release/version.sh` で戻した。前後の `git status --porcelain` と `git diff` は同じ（どちらも空）
- 実行: tools/release/check-no-release-branch.sh
- 結果: 落ちた

## 2.2 2026-11-23T13:46:06+00:00
- 壊したもの: collector-android/app/build.gradle.kts:83 の `val buildVersionInput = buildVersionProvider.get().trim()` を `val buildVersionInput = ""` にした（`ASHIATO_BUILD_VERSION` を無視し、出所の版 0.0.1 で組む）。作業ツリーで壊し、`git checkout -- collector-android/app/build.gradle.kts` で戻した。前後の `git status --porcelain`（`?? …/correction-review.md` の 1 行）と `git diff`（空）は同じ（cmp で一致）
- 実行: tools/release/check-artifacts.sh apk-build 0.4.0-beta.2
- 結果: 落ちた
