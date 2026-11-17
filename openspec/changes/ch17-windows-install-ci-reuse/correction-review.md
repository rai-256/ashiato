# CH17 機械が承認した検証コマンドの訂正の確かめ（手 7）

## 2.3 2026-11-17T05:21:17+00:00
- 壊したもの: `chain` の job の、merge-base から既にある step `- run: ./tools/check-spdx.sh`（.github/workflows/ci.yml:649）を 1 行消した。新しいコマンドで検査の範囲が広がったのは `chain` なので（旧は 1 字比較、新は「2 step を足しただけ」を見る）、ここを壊した。足された 2 step 以外の変更が見逃されないかを確かめるため（壊す前は同じコマンドが `ok: …` rc=0）
- 実行: ./openspec/changes/ch17-windows-install-ci-reuse/check-ci-scope.sh
- 結果: 落ちた
- 出力: `NG: chain の step が、merge-base の並びに 2 step を足しただけでない` rc=1。`git checkout -- .github/workflows/ci.yml` で戻した。壊す前と後の `git status --porcelain`・`git diff` は同じ（どちらも空。cmp で一致を確かめた）。HEAD 97c03a9ea9d5bd2775468778a2400625ba8181b3
