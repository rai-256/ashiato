# Task 1 の指摘（reviewer が返した番号・重さ・本文をグラフが書き起こす）

## review 1 @5590d40396f8941fa191ed5b944b7d4d9fbece82

（reviewer の本文: `task-1-review-1.md`）

### Spec Compliance

- ⚠️ Cannot verify from diff: 表を変えたので、gradle の verifyVersionCodeTable は Task 2.2 まで落ちる見込み。Task 2 の後に、gradle・Rust・TS の 3 つがこの表で通ることを確かめる。
- ⚠️ Cannot verify from diff: 実装者は RED（実装の前に試験が落ちること）を取っていないと自分で書いている。試験を先に書いた証跡が無い。

### Issues

#### Critical (Must Fix)
None

#### Important (Should Fix)
- F1. tools/check-version.sh:37 — self-test の最初の 2 つの Scenario（gradle の版番号・package.json の版番号）は、MAJOR のファイルがまだ無いことで落ちていて、何も確かめずに通っている。MAJOR の作成が :46-47 にあり、check_tree はそれより先に MAJOR を見る（:14-17）。直し方: MAJOR の作成を木の準備へ移し、各 Scenario の前に健全な木が通ることを確かめる。
- F2. tools/release/version.sh:63 — check_version の引数の中の $(major_file) の失敗は set -e に拾われず、空の major は (( value_major == major )) で 0 と評価される。実測: MAJOR が 'x' の写しの木で `version.sh check 0.2.0` は NG の文を出しながら rc=0 で受けた。直し方: 先に代入して失敗を伝え（local major; major=$(major_file) || exit 1）、MAJOR が壊れていれば check が落ちる試験を足す。

#### Minor (Nice to Have)
- F3. tools/release/version.sh:31 — major_file が既定の道を 2 回書いている（set -- の後に ${1:-$ROOT/…} でもう一度既定を書いている）。どちらか片方でよい。
- F4. tools/release/version-check-table.tsv:1 — 左を 1 にした「断る」行（例 0.2.0/0.0.1/1 ng）も、左を 1 にした「番号を持たない版」が通る行（例 0.0.1/0.0.1/1 ok）も無い。Rust・gradle・TS の揃いを確かめる表として、左が 0 でないときの規則を固定しない。
- F5. 報告のとおり RED の証跡が無い。F1 は、まさにその手順で捕まるはずの空の試験だった。

### Assessment

**Task quality:** Needs fixes
