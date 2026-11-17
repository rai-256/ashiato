# CH17 実装タスク — Windows のインストールの試験の 1 周を縮める

読む順: `proposal.md` → このファイル → `design.md` → `specs/distribution/spec.md` → `docs/changes/CH17.md` → `CLAUDE.md`。

軽い路で始め、深掘り第 1 回（Q1・Q2。`deep.md`）で正典 `distribution` の 2 つの Requirement を MODIFIED で改めて通常の路へ移った。製品のコードと配布物の作り方は変えない。

## Global Constraints

- **触ってよいのは次のパスだけ**: `.github/workflows/ci.yml`・`tools/release/windows-build-key.sh`・`docs/testing.md`・`docs/release.md`・`docs/adr/*.md`
  （と `docs/changes/CH17.md`・`openspec/changes/ch17-windows-install-ci-reuse/**`）。**`tools/release/changes.sh`・`tools/release/check-guards.sh`・`tools/release/check-artifacts.sh`・
  `tools/release/notices.sh`・`tools/check-ci-ch14-step.sh`・`tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/build.sh`・`tools/release/package.sh`・
  `tools/release/draft.sh`・他の workflow（`release.yml`・`nightly.yml`）・製品のコードには触れない**（本人の範囲は CI の Windows の配布物の job だけ。Q2 で `package-artifacts` も変えないと決めた）
- **正典（`openspec/specs/`）を直に書かない**。この change の spec（ADDED 1・MODIFIED 2）は archive で正典に入る。MODIFIED の 2 つの Requirement の既存の Scenario は名前を変えていないので、
  その印（`notices.sh`・`check-guards.sh`・`check-artifacts.sh`）には触れない
- **`ci.yml` で変えてよいのは `package-windows` の job の step だけ**。`package-windows` の job の `if:`・`needs:`・`runs-on:`・checkout の `ref:` と、
  `changes`・`install-windows`・`package-artifacts` ほか全部の job は変えない（`install-windows` は今と同じ成果物の名前を受け取るので変える必要が無い）
- **壊してはいけないもの**（`docs/changes/CH17.md`）: `changes` の job の出し分け・`tools/release/check-guards.sh workflows`・`tools/check-ci-ch14-step.sh` が通ること・
  成果物の名前（`package-windows-release` / `package-windows-test`）・`install-windows` のインストールと更新の Scenario の全数の突き合わせ・
  ビルドするときの手順（`ch14_` の試験 → `package.sh windows` → `package.sh windows … --test-build`、`PGSQL_CACHE_DIR` を渡す）
- **使う action は今の pin のまま**: `actions/cache/restore` と `actions/cache/save` は、今の `actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0` と同じ commit に pin する。
  新しい action（`Swatinem/rust-cache` など）を足さない
- 試験の `# Scenario: <名前>` の印は、spec の `#### Scenario:` と一字一句合わせる（bash は `echo "== Scenario: <名前>"` か `#`）
- 台本の一時ファイルは `"${TMPDIR:-/tmp}"` の下に `mktemp -d` で作り、終わりに消す（`/tmp` の固定名に書かない）

## Task 1: 組み立ての鍵を出す台本

- [x] 1.1 `tools/release/windows-build-key.sh` を置く（design D1）。`windows-build-key.sh [<commit>]`（既定 `HEAD`）は、repo の根（`git rev-parse --show-toplevel`。引けなければ落ちる）で `git -C <根> ls-tree -r --full-tree <commit>` を読み、除くパス
  （`tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/conpty-run.ps1`・`docs/` と `openspec/` の下・`*.md`）を落とした行と、
  道具の版の行（環境変数 `WINDOWS_BUILD_KEY_TOOLS` に渡された文字列。CI では `rustc -V`・`node -v`・`ImageOS`・`ImageVersion` を並べて渡す）を合わせた sha256 を 1 行で出す。
  git の木でない・commit が引けないときは、何も出さずに 0 でない終了コードで終わる。除くパスは台本の先頭に 1 か所で列挙し、理由（その台本を組み立てが読まない）をコメントに書く。
  `--self-test` は使い捨ての git の木を作り、spec の鍵の 5 つの Scenario を確かめる（各 Scenario の前に `echo "== Scenario: <名前>"`）:
  「試験の台本と文書だけが違う 2 つの木は、同じ組み立ての鍵になる」（3 つの台本と `docs/x.md` の変更）/
  「組み立てに入るファイルが 1 つでも違えば、鍵が変わる」（`crates/server/src/lib.rs`・`migrations/ORDER`・`web/src/App.tsx`・`tools/release/build.sh`・`tools/release/pgsql.lock`・`tools/licenses/` の 1 ファイル・`LICENSE`・`Cargo.toml`・`collector-android/` の 1 ファイル・`Cargo.lock`・`.github/workflows/ci.yml` を 1 つずつ）/
  「名指ししていないファイルが増えれば、鍵が変わる」（`tools/release/` に新しい台本）/
  「道具の版が違えば、鍵が変わる」（`WINDOWS_BUILD_KEY_TOOLS` の Rust の版だけを変える）/
  「鍵を出せない木では、鍵を出さずに落ちる」（git の木でない置き場で `GIT_CEILING_DIRECTORIES` をその置き場の親にして走らせ、rc≠0 と空の標準出力。一時の置き場が git の木の中にあっても親の木を見ない。review R8）。
  自己試験は、台本を repo の根でない cwd（使い捨ての木の下の `crates/`）から走らせても同じ鍵になることも見る。
  自己試験は、除くパスの列挙から 1 つ（`install-check.sh`）を外した版の台本でも走らせ、1 つ目の Scenario が落ちることを見る（除くことが効いていることの確かめ）。
  Windows の Git Bash でも動くよう、`sha256sum` と `git` と bash の組み込みだけを使う（python を使わない）。
  検証: `tools/release/windows-build-key.sh --self-test` rc=0 /
  `bash -c 'out=$(tools/release/windows-build-key.sh --self-test); for s in "試験の台本と文書だけが違う 2 つの木は、同じ組み立ての鍵になる" "組み立てに入るファイルが 1 つでも違えば、鍵が変わる" "名指ししていないファイルが増えれば、鍵が変わる" "道具の版が違えば、鍵が変わる" "鍵を出せない木では、鍵を出さずに落ちる"; do grep -qF "== Scenario: $s" <<<"$out" || exit 1; done'` rc=0 /
  `bash -c 'k=$(tools/release/windows-build-key.sh HEAD); [[ "$k" =~ ^[0-9a-f]{64}$ ]]'` rc=0 /
  `bash -c 's="$PWD/tools/release/windows-build-key.sh"; d=$(mktemp -d); cd "$d" && ! out=$(GIT_CEILING_DIRECTORIES="$(dirname "$d")" "$s" HEAD 2>/dev/null) && [ -z "$out" ]; rc=$?; cd / && rm -rf "$d"; exit $rc'` rc=0（git の木でない置き場）

## Task 2: Windows の配布物の job が鍵で使い回し、Rust のビルドをキャッシュから戻す

- [x] 2.1 `.github/workflows/ci.yml` の `package-windows` の step を design D2・D4・D5 のとおりに変える: toolchain と node の後に、鍵の台本の自己試験（`tools/release/windows-build-key.sh --self-test`。
  落ちたら job を落とす）→ 鍵を出す step（`WINDOWS_BUILD_KEY_TOOLS` に `rustc -V`・`node -v`・`$ImageOS`・`$ImageVersion` と、`${{ github.sha }}` を `git fetch --depth=1` で取ってその commit の `.github/workflows/ci.yml` の blob の id（design D1。走っている run の CI の定義）。fetch か台本が落ちたら鍵を空にして続ける）→
  `pull_request` の run で鍵が空でないときだけ `actions/cache/restore`（鍵 `windows-package-<鍵>`、`restore-keys` 無し、置き場は `${{ runner.temp }}/package-windows` と
  `${{ runner.temp }}/package-windows-test`）→ 引けなかったときだけ Rust のキャッシュ（`actions/cache`。`~/.cargo/registry`・`~/.cargo/git`・`target`、鍵 `cargo-windows-<rustc -V>-<hashFiles('Cargo.lock')>`、
  `restore-keys: cargo-windows-<rustc -V>-`）・PostgreSQL のキャッシュ（今のまま）・`ch14_` の試験・2 つの `package.sh windows` → 2 つの `upload-artifact`（今の名前のまま。引けたときも引けなかったときも。引いた置き場を書き換えずに上げる）→
  引けなかったときで、鍵が空でなく、ここまでが成功したときだけ `actions/cache/save`（同じ鍵・同じ置き場）。restore と save の置き場は、`package-windows` の `upload-artifact` の `path` の全部と同じにする（design D2。今は 2 つ）。job の `if:`・checkout・step の並び（`ch14_` → 2 つの `package.sh windows`）は変えない。
  既存の step のコメントは残し、足した step には何のための step かを 1 行のコメントで書く。
- [x] 2.2 `tools/release/windows-build-key.sh --check-workflow [<ci.yml>]`（既定 `.github/workflows/ci.yml`。`python3` と `yaml` で読む。この口は Linux の手元と merge gate で走らせ、Windows の runner では走らせない）を足し、
  spec の CI の定義の 8 つの Scenario を確かめる（各 Scenario の前に `echo "== Scenario: <名前>"`）:
  「前の組み立ては、鍵が空でなく完全に一致した PR の run でだけ引く」（`package-windows` の配布物の restore が `restore-keys` を持たず、鍵が `windows-package-` と鍵の step の出力から成り、`if:` が `pull_request` と鍵が空でないことを条件に持つ）/
  「引けたときだけ組み立てを飛ばし、今と同じ名前で上げる」（`ch14_` の試験と 2 つの `package.sh windows` の step の `if:` が restore の `cache-hit` が `'true'` でないことを条件に持つ／2 つの `upload-artifact` が `if:` で飛ばされず今の名前を持つ）/
  「上げる配布物は全部、保存と引きの置き場に入る」（`package-windows` の `upload-artifact` の `path` が 1 つ残らず、restore と save の `path` に入っている）/
  「使い回した Windows の配布物は、組んだ commit のソースの場所と印のまま上げる」（restore より後で `upload-artifact` より前の step のうち、`run:` を持つものと `actions/cache` でないものは全部 `if:` に restore の `cache-hit` が `'true'` でないことを持つ／`upload-artifact` の `path` が restore の `path` と同じ）/
  「組み立てた配布物は、鍵が空でなく組み立てが成功したときだけ保存する」（`actions/cache/save` が `cache-hit` でないこと・鍵が空でないこと・`success()` を条件に持ち、restore と同じ鍵と置き場を持つ）/
  「鍵を出す手順は、道具の版と走っている CI の定義を鍵に入れる」（鍵の step の `WINDOWS_BUILD_KEY_TOOLS` が `rustc -V`・`node -v`・`ImageOS`・`ImageVersion`・`github.sha` の `ci.yml` を持つ）/
  「ビルドするときは、Rust の依存のビルドを戻す」（Rust の `actions/cache` が restore の `cache-hit` でないときだけ走り、`~/.cargo/registry`・`~/.cargo/git`・`target` を持ち、鍵が `rustc -V` と `hashFiles('Cargo.lock')` を含み、`restore-keys` が Rust の版までの前方一致）/
  「使い回した run でも、インストールと更新の Scenario が 1 つ欠ければ CI が落ちる」（`install-windows` が `install-check.sh --list` と `update-check-install.sh --list` の全部との突き合わせ
  （`comm -23` と `test ! -s`）を 2 つとも持ち、それらの step が `if:` を持たない）。
  `--self-test` に、`--check-workflow` が落ちることの確かめを足す: 今の `ci.yml` の写しから (a) restore に `restore-keys` を足した版、(b) `package.sh windows` の step の `if:` を消した版、
  (c) `install-windows` の Scenario の突き合わせの step を消した版、(d) restore の `pull_request` の条件を消した版、(e) save の `success()` を消した版、(f) restore と save の鍵が空でない条件を消した版、
  (g) 鍵の step から `ImageVersion` を消した版、(h) Rust のキャッシュの step を消した版、(i) `upload-artifact` を 1 つ足し、その `path` を restore と save に入れない版、
  (j) restore と `upload-artifact` の間に `if:` を持たない `run:` の step（引いた置き場に書く）を足した版、(k) `upload-artifact` の 1 つの `path` を restore の `path` と違う置き場に変え、その置き場を save にだけ足した版（引いた置き場をそのまま上げていない）を作り、どれも `--check-workflow` が 0 でない終了コードで終わること
  （`--self-test` が `python3` を要るのはこの節だけ。`python3` か `yaml` が無ければこの節を飛ばしたと 1 行出して続ける —— Windows の runner の自己試験はこの節を飛ばす）。
  検証: `tools/release/windows-build-key.sh --check-workflow` rc=0 /
  `bash -c 'out=$(tools/release/windows-build-key.sh --check-workflow); for s in "前の組み立ては、鍵が空でなく完全に一致した PR の run でだけ引く" "引けたときだけ組み立てを飛ばし、今と同じ名前で上げる" "上げる配布物は全部、保存と引きの置き場に入る" "使い回した Windows の配布物は、組んだ commit のソースの場所と印のまま上げる" "組み立てた配布物は、鍵が空でなく組み立てが成功したときだけ保存する" "鍵を出す手順は、道具の版と走っている CI の定義を鍵に入れる" "ビルドするときは、Rust の依存のビルドを戻す" "使い回した run でも、インストールと更新の Scenario が 1 つ欠ければ CI が落ちる"; do grep -qF "== Scenario: $s" <<<"$out" || exit 1; done'` rc=0 /
  `bash -c 'out=$(tools/release/windows-build-key.sh --self-test); ! grep -q "飛ばした" <<<"$out"'` rc=0（手元では `--check-workflow` の自己試験も走ったこと）
- [x] 2.3 今の振る舞いが残ることを確かめる（壊してはいけないもの）。`ci.yml` で変わったのが `package-windows` の job の中だけであることも見る。
  検証: `tools/release/check-guards.sh workflows` rc=0 /
  `tools/release/check-guards.sh --self-test` rc=0 /
  `tools/check-ci-ch14-step.sh` rc=0 /
  `python3 -c 'import subprocess as sp, yaml; base=sp.run(["git","merge-base","HEAD","origin/main"],capture_output=True,text=True,check=True).stdout.strip(); o=yaml.safe_load(sp.run(["git","show",base+":.github/workflows/ci.yml"],capture_output=True,text=True,check=True).stdout); n=yaml.safe_load(open(".github/workflows/ci.yml",encoding="utf-8")); oj=o.pop("jobs"); nj=n.pop("jobs"); assert o==n and oj.keys()==nj.keys(); assert all(oj[k]==nj[k] for k in oj if k!="package-windows"); a=dict(oj["package-windows"]); b=dict(nj["package-windows"]); a.pop("steps"); b.pop("steps"); assert a==b'` rc=0（`package-windows` の step 以外は 1 字も変わっていない）/
  `bash -c 'printf "%s\n" tools/release/install-check.sh | tools/release/changes.sh code && ! printf "%s\n" docs/x.md | tools/release/changes.sh code'` rc=0（出し分けの判定はそのまま）

## Task 3: 文書

- [x] 3.1 `docs/testing.md` の CI の job の表の `package-windows` / `install-windows` の行（無ければ表の近くの段落）に、使い回す条件（鍵の作り方・除くもの・PR の run だけ・完全一致）・
  Rust のキャッシュ・使い回しを止めたいとき（鍵の台本の除くものを空にする／キャッシュを `gh cache delete` で消す）・使い回した run の `release-package` の Windows の配布物の印は前の commit になること（design D3）を書く。
  依存の名前は書かない。
  検証: `bash -c 'grep -q "windows-build-key.sh" docs/testing.md && grep -q "release-package" docs/testing.md'` rc=0
- [x] 3.2 `docs/release.md` §11.1（下書きのリリースで許諾の 3 点を見る）の「run の `head_sha` と配布物の組み立ての印の commit が同じ」の後に、使い回した run の見方を足す（design D3・整合の確認 C3）:
  Windows の配布物を使い回した run（`package-windows` の組み立ての step が飛ばされた run）では、Windows の 2 つの zip の印と `SOURCE.txt` は、それを組んだ commit（同じ PR の前の push の head か main の commit）を指す。
  そのときは 2 つの zip について、`SOURCE.txt` の `Commit:` と組み立ての印の commit が同じで `.dirty` が無いことを見る。`tools/release/draft.sh` は変えない。
  検証: `bash -c 'sed -n "/^### 11.1/,/^### 11.2/p" docs/release.md | grep -q "使い回した"'` rc=0
