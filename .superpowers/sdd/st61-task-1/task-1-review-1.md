### Spec Compliance

- ✅ 1.1 と 1.2 の項目はすべて diff にある。`tools/release/MAJOR` は `0`。`Cargo.toml` と `Cargo.lock` の workspace の 7 crate は `0.0.1`。`version.sh` には `major`・`check`（D9）・`code`（D3。上限を超えると、超えた部分を出して落ちる）がある。`version-table.tsv` は brief が挙げた全行を持ち、`version-check-table.tsv` は必須の 7 行を含む 17 行で、`--self-test` が全行を当てる。Scenario の印 4 本（`tools/release/version.sh` の self_test）と 3 本（`tools/check-version.sh`）は spec と一字一句同じ。旧い印「出所の版の MAJOR が 0 でないと…」は消えている。
- ⚠️ 表を変えたので、gradle の `verifyVersionCodeTable`（`collector-android/app/build.gradle.kts` がこの表を読む）は、Task 2.2 が式を D3 にするまで落ちる見込み。controller は Task 2 の後に、gradle・Rust・TS の 3 つがこの表で通ることを確かめる。
- ⚠️ 実装者の報告は、RED（実装の前に試験が落ちること）を取っていないと自分で書いている。試験を先に書いた証跡が無い。

### Strengths
- `check_rule` を `check_rule <版> <出所> <左>` の純粋な形に切り出し、self_test が `version-check-table.tsv` の全行を、Cargo.toml や MAJOR のファイルを介さずに当てている（`tools/release/version.sh:51-60`、self_test の表のループ）。Rust・gradle・TS が同じ表で揃いを確かめる土台になっている。
- 上限を超えたときの文に「超えた部分」を入れ、試験は rc だけでなく `超えた` の文字も見ている（`tools/release/version.sh` の version_code と、self_test の `0.1000.0 0.1.10000 210.0.0` のループ）。
- `plan()` は、これまで check_version が担っていた「印の 3 つの数字が出所の版と同じ」の検査を明示して足し、既存の Scenario「印の版と出所の版が違えば公開しない」を保っている（`tools/release/version.sh` の plan 内）。

### Issues

#### Critical (Must Fix)
（なし）

#### Important (Should Fix)
- F1. `tools/check-version.sh:37-45` — self-test の最初の 2 つの Scenario（「携帯の組み立ての設定に版番号を手で書くと、版の検査が落ちる」「画面の package.json に版番号を書くと、版の検査が落ちる」）が、何も確かめずに通っている。`check_tree` は出所の版の次に MAJOR のファイルを見る（`:14-17`）。ところが使い捨ての木の `tools/release/MAJOR` は、この 2 つの Scenario の後の `mkdir -p "$tree/tools/release"`（`:46-47`）で初めて作られる。だから 2 つとも、gradle や package.json の検査の前に「MAJOR が無い」で落ちている。gradle の検査を消しても self-test は緑のまま。直し方: `MAJOR` を作る処理を木の準備（`:34-36`）へ移し、各 Scenario の前に「健全な木なら check_tree が通る」ことを一度確かめる。
- F2. `tools/release/version.sh:63`（`check_version`）と `:56` — MAJOR のファイルの誤りが握りつぶされる。`check_rule "$1" "$(source_version)" "$(major_file)"` では、引数の中の `$(...)` の失敗は `set -e` に拾われない。`major_file` が die しても major は空のまま渡る。さらに `(( value_major == major ))` は変数名で参照しているので、空は 0 と評価される。実測: MAJOR を `x` にした写しの木で `version.sh check 0.2.0` を走らせると、`NG: 左の数字のファイルが数字 1 つでない: x` を出しながら `0.2.0` を出力し、rc=0 で終わった。組み立てが MAJOR の壊れた木で番号を受けてしまう。直し方: `local major; major=$(major_file) || exit 1` のように先に代入して失敗を伝える。self_test に「MAJOR が壊れていれば check が落ちる」試験を足す。

#### Minor (Nice to Have)
- F3. `tools/release/version.sh:31-33`（`major_file`）— 既定の道を 2 回書いている。`set -- "$ROOT/…"` の後に、`${1:-$ROOT/…}` でもう一度既定を書いている。どちらか片方でよい。
- F4. `tools/release/version-check-table.tsv` — 左を 1 にした「断る」行が無い（例 `0.2.0	0.0.1	1	ng`）。左を 1 にした「番号を持たない版」が通る行も無い（例 `0.0.1	0.0.1	1	ok`）。これらが無いと、左が 0 でないときの規則（`value_major == major` の比べと、番号を持たない版の分岐）を表が固定しない。Rust・gradle・TS の 3 つの実装の揃いも、この表で確かめるので、ここで足しておくと効く。
- F5. 実装者の報告どおり、RED（実装の前に試験が落ちること）の証跡が無い。F1 は、まさにその手順で捕まるはずの空の試験だった。

### Assessment

**Task quality:** Needs fixes

**Reasoning:** 規則（D9）と式（D3）、2 つの表、Scenario の印は spec どおり。ただし `check-version.sh` の self-test のうち 2 本は MAJOR のファイルが無いことで落ちていて、gradle や package.json の検査を確かめていない（F1）。また `version.sh check` は MAJOR が壊れていても rc=0 で番号を受ける（F2。実測で確かめた）。
