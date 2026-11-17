# spec-review — ch17-windows-install-ci-reuse（CH17・軽い路 `reason: same`）

## 機械の検査（先に写す）

```
$ openspec validate ch17-windows-install-ci-reuse --strict
Change 'ch17-windows-install-ci-reuse' is valid

$ python3 scripts/check_chain.py .
chain: OK (0 件 / 未回収 0 件 / warn 0 件)

$ python3 scripts/check_scenarios.py . ch17-windows-install-ci-reuse
  [FAIL] 担保の無い Scenario: 試験の台本と文書だけが違う 2 つの木は、同じ組み立ての鍵になる
  [FAIL] 担保の無い Scenario: 組み立てに入るファイルが 1 つでも違えば、鍵が変わる
  [FAIL] 担保の無い Scenario: 名指ししていないファイルが増えれば、鍵が変わる
  [FAIL] 担保の無い Scenario: 道具の版が違えば、鍵が変わる
  [FAIL] 担保の無い Scenario: 鍵を出せない木では、鍵を出さずに落ちる
  [FAIL] 担保の無い Scenario: Windows の配布物の job は、鍵が完全に一致した PR の run でだけ組み立てを飛ばす
  [FAIL] 担保の無い Scenario: 使い回した run でも、インストールと更新の Scenario が 1 つ欠ければ CI が落ちる
scenarios: FAIL (担保なし 7 件 / 名無しの確認待ち 0 件)
（ほかに既存の warn 11 件。この change と関係しない）

$ python3 scripts/review_triage.py . ch17-windows-install-ci-reuse
triage: OK（指摘 0 件 / 仮決め 0 件 / 要件へ戻すもの 0 件）

$ python3 scripts/spec_overlap.py candidates . ch17-windows-install-ci-reuse
[]
```

check_scenarios の 7 件は、印を置く台本（`tools/release/windows-build-key.sh`）がまだ無いので落ちている。spec の段では想定どおり。
tasks の 1.1 と 2.2 が印（`echo "== Scenario: …"`）を置く。R は立てない。

Task 2.3 の検証コマンドを今の木で走らせた結果（変更前の基準）: `tools/release/check-guards.sh workflows` rc=0、`--self-test` rc=0、
`tools/check-ci-ch14-step.sh` rc=0、`changes.sh` の 1 行 rc=0、`ci.yml` の yaml 比較 rc=0。どれも書いたとおりに走る。

## 名指しの 3 点の判定

1. **正典の Requirement「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」: A / 昇格が要る（R1）。**
   正典の本文は、この change の ADDED と同じ条件に対して反対の SHALL を言っている。3 つの Scenario は使い回しの後も成り立つが、
   食い違っているのは Scenario ではなく本文。本文を MODIFIED で書き換える必要があり、軽い路では書けない（`scripts/lanes.py:24`・`:60`）。
   「Scenario は変わらない」（delta の spec.md:13、proposal.md:31-33）は、本文と Scenario の差を使って言い換えで逃げている。
   名付けられる loss は無い。後から直しても失うものは無く、人間に返す理由は昇格の方にある。そのため kind は `premise` にした（理由は R1）。
2. **design D3（使い回した run の `release-package` に、前の commit で組んだ Windows の配布物が入る）: loss は無いが、P（仮）では閉じられない（R2）。**
   - loss が無い理由: 下書きは公開されない（exported でない）。本物のリリースと開発版は自分でビルドする。保存物は 7 日で消え、計算し直せば戻る。
   - 仮で閉じられない理由: 正典の Requirement「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」の本文
     （spec.md:2259-2260「PR の組み立てでは PR の head の commit」「PR の組み立てを、PR の head の commit そのものから組む」）と食い違う。
     P の条件は「要件を変えない」で、それを満たさない。
   - 直し方は 2 つあり、どちらも軽い路の外に出る: 正典を MODIFIED で変える（昇格）か、`package-artifacts` を変える（本人の範囲「他の job は変えない」を越える）。
   - 黙って入る理由: CI はこの食い違いを検出しない。`check-artifacts.sh` は配布物ごとに印と `SOURCE.txt` の一致を見るだけで、配布物どうしの commit や head の commit とは比べない。
3. **design D1 の鍵: 今の木では足りる（除いたものは Windows の配布物の組み立てに入っていない）。ただし spec と反転条件に穴がある（R3・R4・R5・R6）。**
   - 実物で辿った結果: `build.sh`・`package.sh`・`notices.sh`・`pgsql.sh`・`version.sh`（PR の経路）は、3 つの台本・`docs/`・`openspec/`・`*.md` のどれも読まない。
     `conpty-run.ps1` を読むのは `install-check.sh` だけ（`install-check.sh:310`・`:1473-1476`）。
   - `crates/*/build.rs` が読むのは `migrations/`・`ASHIATO_WEB_DIST`・git の HEAD だけ。
   - `include_str!` / `include_bytes!` で製品が組み込むのは `tools/db-roles.sql`・`migrations/`・`grants.sql` など。これらは鍵に入る。
   - `docs/` を読む `include_str!` / `read_to_string` は試験のコードにだけある（R5）。
   - 鍵の外の入力として、runner の Visual Studio から写す VC の実行時の DLL がある。今は ImageVersion が覆っているが、D1 の反転条件は ImageVersion を外す（R4）。
   - 鍵の作り方そのものにも穴がある: `git ls-tree -r` が cwd に依る（R8）。pull_request の run が走らせる workflow の定義が鍵に入らない（R11）。

---

## R1. 正典「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」の本文が、ADDED の Requirement と反対のことを言う。MODIFIED が要る（A / 昇格が要る）
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/specs/distribution/spec.md
- 根拠:
  - 正典の本文（`openspec/specs/distribution/spec.md:784`）:「WHEN コード（`docs/`・`openspec/`・`*.md` 以外）を触った PR が CI を走らせる THE SYSTEM SHALL サーバ（…Windows 向けと Linux 向け）…を、配るときと同じ手順で…組み立てる」。
    delta の spec.md:5 は、同じ「コードを触った PR」に対して「Windows の配布物をビルドせずに…使い」と言う。
  - `printf 'tools/release/install-check.sh\n' | tools/release/changes.sh code` は rc=0。台本だけの PR も正典の言う「コードを触った PR」で、
    その PR の CI は Windows 向けを組まない。main の保存物を引けば、その PR の CI では一度も組まない（design D2）。
  - 正典の spec.md:786「CI の実行の成果物としてだけ置く」に対して、この change は配布物を Actions のキャッシュにも置く（design D2）。
  - delta の spec.md:13 と proposal.md:31-33 は「3 つの Scenario は変わらない」とだけ主張していて、本文の 2 つの SHALL には触れていない。
    validate はこれを落とさない（観点 2 の「本文だけが言っていて Scenario が言っていないこと」）。
  - 軽い路は、正典を MODIFIED / REMOVED / RENAMED で変える change を通常の路へ外す（`scripts/lanes.py:24`、`SPEC_CHANGE` は `:60`）。
- kind: premise
- 提案: 正典の本文を `## MODIFIED Requirements` で書き換える。例:「組み立てる。ただし組み立ての鍵が同じ前の組み立ての Windows の配布物があれば、それを使う」
  「CI の実行の成果物と CI のキャッシュにだけ置き、公開しない」。そのうえで通常の路へ昇格する。
  loss は名付けられない（後から直しても失うデータは無い）。人間へ返す理由は「既存の Requirement は変えない」という軽い路の前提が崩れたことなので、kind を premise にした。
- 処置: escalated —— 第 1 回 Q1（deep-questions-r1.json）。正典の本文を MODIFIED で改めるかは本人の範囲の外。答えで通常の路へ

## R2. D3 は正典「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」の本文と食い違う。loss は無いが、仮で閉じられない
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/design.md（D3）, proposal.md:44-45
- 根拠:
  - 正典の `openspec/specs/distribution/spec.md:2259`:「PR の組み立てでは PR の head の commit になる」。
  - 同じく `:2260`:「PR の組み立てを、PR の head の commit そのものから組む…`SOURCE.txt` の commit・組み立ての印・組んだソースが同じ commit を指す」。
  - 同じく `:2261`:「配布物の検査で、期待する版番号と…`LICENSE` を、配布物を組んだのと同じ commit の中身から取る」。
    使い回した run の `package-artifacts` は、期待値を head（`built/`）から取る。配布物を組んだ前の commit からではない。
    中身は鍵が同じなので一致するが、本文の言う commit とは違う。
  - 同じく `:715`:「手で起こした確かめの組み立て（PR の組み立ての配布物を使う）」。
  - D3 のもとでは、使い回した run の `release-package` の Windows の 2 つの zip の `SOURCE.txt` と印は、前の commit（main の push の run のこともある）を指す。PR の head ではない。
  - CI はこれを検出しない。`check-artifacts.sh:164-167` は配布物ごとに `SOURCE.txt` と印の一致を見るだけ。`:284-305` も Windows の zip の中の 3 つの印どうしを比べるだけで、
    Linux の配布物や head の commit とは比べない。正典の Scenario（`:2319-2345`）も緑のまま通る（観点 2 の「本文だけ」）。
  - `docs/changes/CH17.md` の「壊してはいけないもの」はこの性質を挙げていない。spec にも D3 の振る舞いの Scenario が無く、design にだけある（観点 3: 観測できる振る舞いが design に置かれている）。
  - loss の 4 つを当てた結果:
    - exported でない: `draft.sh` は `--draft` で、本物のリリースは `release.yml`、開発版は `nightly.yml` が自分でビルドする。
    - uncaptured / discarded でない: 何も取りこぼさず、何も捨てない。
    - rewrite-all でない: 保存物は 7 日で消え、`ci.yml` を戻せば戻る。
- kind: premise
- 提案: 本人の範囲（他の job は変えない）の中で閉じる道は、正典 `:2259-2260` の本文を MODIFIED で「使い回した配布物は、それを組んだ commit を指す」に変えることだけ。
  これは R1 と同じ昇格になる。もう 1 つの道は、`package-artifacts` を変える（使い回した run では `release-package` を上げない、など）。こちらは範囲を越えるので、本人に確かめる。
  どちらを選んでも、D3 の振る舞いを spec の Scenario に移す。
- 処置: escalated —— 第 1 回 Q2（deep-questions-r1.json）。正典を改めるか、package-artifacts を変える（範囲を広げる）かを本人に聞く

## R3. 鍵から除くものの一覧が spec に無く、除き過ぎた実装が全部の Scenario を通る
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/specs/distribution/spec.md
- 根拠:
  - delta の spec.md:6 は「試験の台本と文書として名指ししたファイルを除く」とだけ言い、何を名指しするかを書いていない。一覧は design D1 と tasks 1.1 にだけある。
  - 鍵の Scenario（spec.md:15-38）は、「同じになる」側を 3 つの台本と `docs/` でしか見ていない。「変わる」側も名指しした 6 つのファイルと `tools/release/` の新しい台本でしか見ていない。
    そのため、`collector-android/`・`tools/licenses/`・`tools/release/pgsql.lock`・`LICENSE` を除く実装も、5 つの Scenario を全部通る。
    例えば `tools/licenses/` は `notices.sh` の入力で、`LICENSE` は `build.sh:48` が zip に入れる。
  - 自己試験の効き目の確かめ（tasks 1.1 の 36 行）は、除くものを 1 つ外す方向（`install-check.sh`）しか見ない。除くものを足す方向の誤りは検出しない。
- kind: technical
- 提案: Requirement の本文に、除くものの閉じた一覧を書く（3 つの台本・`docs/` と `openspec/` の下・`*.md`）。
  さらに Scenario「組み立てに入るファイルが 1 つでも違えば、鍵が変わる」の組に、`tools/licenses/` の 1 ファイル・`LICENSE`・`tools/release/pgsql.lock`・
  `collector-android/` の 1 ファイルを足す（D1 の「含むもの」と揃える）。
- 処置: fixed specs/distribution/spec.md —— Requirement の本文に除くものの閉じた一覧を書き、「鍵が変わる」の組に pgsql.lock・tools/licenses/・LICENSE・collector-android/ を足した（tasks 1.1 も）

## R4. D1 の反転条件「ImageVersion を外す」は、鍵の外にある本物の入力（VC の実行時の DLL）を落とす
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/design.md（D1 の反転条件、47-48 行）
- 根拠:
  - `tools/release/build.sh:155-168` の `bundle_vc_runtime` は、runner の Visual Studio（`vswhere -latest` → `VC/Redist/MSVC/*/x64/Microsoft.VC14*.CRT`）から
    `vcruntime140.dll`・`msvcp140.dll` を写す。`:198` がそれを `pgsql/bin` に向けて呼び、`:109` が `pgsql/` ごと zip の置き場に写す。
    この版は git の木に無く、`rustc -V`・`node -v` にも出ない。
  - 今は `ImageOS`・`ImageVersion` が覆っている。反転条件どおり ImageVersion を外すと、VS の更新で DLL が変わっても鍵が変わらず、古い DLL の入った zip を使い回す。
  - Windows の runner では native（MSVC）の target で組む（`ci.yml:473` のコメント「native の target で組む（gnu の target は使わない）」）。そのため MSVC のリンカの版も同じ扱いになる。
- kind: technical
- 提案: 反転条件を「ImageVersion を外し、代わりに VC の再頒布の置き場の版（`VC/Redist/MSVC/<版>`）と MSVC の toolset の版を道具の版の行に入れる」に直す。
  鍵の道具の版の列挙（D1 の 38 行）にも、ImageVersion が覆っているものとして VC の実行時の DLL を名指しする。
- 処置: fixed D1 —— ImageVersion が VC の実行時の DLL と MSVC のリンカを覆うと書き、反転条件を「外すなら VC の再頒布と toolset の版を入れる」に直した

## R5. D1 の前提の文「`include_str!` / `include_bytes!` は `docs/` を読まない」は、試験のコードについては偽
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/design.md（D1 の 35 行）
- 根拠:
  - `crates/update-check/src/lib.rs:562-564` は `include_str!("../../../docs/contracts/update-check-vectors.json")`。`#[cfg(test)]` の中（`:270`）にある。
  - `crates/collector-windows/src/setup_string.rs:80-85` と `crates/collector-windows/src/contract.rs:532-534` も `docs/contracts/` と `docs/contract/envelope.json` を読む（どちらも `#[cfg(test)]`）。
  - 配る実行ファイルには入らないので、鍵の保証は今の木では成り立つ。
  - D4 で飛ばす `ch14_` の試験（`ashiato-setup`）は `docs/` を読まない（`crates/installer` に `docs/` の include は無い）。
  - ただし文は「確かめた」と言っていて、その範囲が書いていない。いずれ試験でないコードが `docs/contract/` を読むようになっても、それを止める仕組みが無い。
    `docs/contract/**` は `tools/lanes.json` の `except.docs` で送信の契約として扱われている。
- kind: technical
- 提案: D1 の文を「配る組み立て（`cfg(test)` でないコード・`build.rs`・`build.sh` 系）は読まない」に限る。
  反転条件に「`docs/contract/` を試験でないコードが読むようになったら、除くものから外す」を足す。
  機械で止めたいなら、自己試験に「`crates/*/src` の `cfg(test)` の外と `build.rs` に `docs/` を読む `include_str!` が無い」を足す。
- 処置: fixed D1 —— 前提の文を配る組み立てに限り、試験のコードが docs/contracts を読むことと、反転条件（試験でないコードが読むようになったら除くものから外す）を書いた

## R6. 道具の版を CI が鍵の台本へ渡すことを、どの Scenario も見ていない
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/specs/distribution/spec.md, tasks.md（2.2）
- 根拠:
  - Requirement の本文（spec.md:6）は「組み立てに使う道具の版（Rust・Node・runner の image）から作る」と言う。
  - しかし Scenario「道具の版が違えば、鍵が変わる」（spec.md:30-33）が見るのは、台本が `WINDOWS_BUILD_KEY_TOOLS` の文字列を鍵に混ぜることだけ。
  - tasks 2.2 の `--check-workflow` の確かめの項目（54-58 行）には、鍵の step が `WINDOWS_BUILD_KEY_TOOLS` に `rustc -V`・`node -v`・`ImageOS`・`ImageVersion` を渡すことが無い。
  - そのため、CI が空の文字列を渡しても、7 つの Scenario は全部緑になる。そのときは Rust の版が上がっても使い回す。
- kind: technical
- 提案: CI の定義の Scenario に「鍵を出す step は、道具の版として Rust・Node・runner の image の版を渡す」を足す。
  `--check-workflow` でそれを確かめ、`--self-test` に、その 4 つの 1 つを消した `ci.yml` の写しで落ちることを足す。
- 処置: fixed specs/distribution/spec.md —— Scenario「鍵を出す手順は、道具の版と走っている CI の定義を鍵に入れる」を足し、tasks 2.2 の --check-workflow と自己試験 (g) で見る

## R7. 「鍵を出せないときは使い回さずにビルドする」の CI の側に Scenario が無い。保存の条件も否定の自己試験が無い。1 つの Scenario に主張が 3 つある
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/specs/distribution/spec.md, tasks.md（2.2）
- 根拠:
  - 本文 spec.md:8「WHEN 鍵を出せない THE SYSTEM SHALL 使い回さずにビルドする」を見る Scenario は、台本の Scenario（spec.md:35-38。rc≠0 と空の出力）だけ。
    CI の側（鍵が空なら restore と save を走らせない）は、tasks 2.1 の文（47 行・50 行）にしか無い。
    tasks 2.2 の `--check-workflow` は restore と save の「鍵が空でない」の条件を見ず、自己試験の (a)〜(d)（59-60 行）にも save の改変が無い。
  - save の非空の条件が抜けると、`windows-package-` の鍵で保存される。restore の非空の条件も抜けると、鍵を出せない run が、無関係の組み立てを引く。
  - spec.md:40-45 の Scenario は THEN と AND 2 つで、別々の主張を 3 つ束ねている（引き方・飛ばし方・保存の条件）。
- kind: technical
- 提案: Scenario を 3 つに分ける。
  - 引き方: 完全一致・PR の run だけ・鍵が空でないときだけ。
  - 飛ばし方: 引けたときだけ組み立てを飛ばし、同じ名前で上げる。
  - 保存の条件: 引けず・鍵が空でなく・成功したときだけ、同じ鍵で保存する。

  `--self-test` に、(e) save の `success()` を消した写しと、(f) restore・save の鍵の非空の条件を消した写しを足す。
- 処置: fixed specs/distribution/spec.md —— CI の Scenario を引き方・飛ばし方・保存の条件の 3 つに分け、鍵が空でない条件を入れた。tasks 2.2 の自己試験に (e)(f) を足した

## R8. 「git の木でない置き場」の確かめは、このハーネスの TMPDIR では成り立たない。`git ls-tree -r` は cwd に依る
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/tasks.md（1.1 の 35 行・41 行、Global Constraints の 22 行）
- 根拠:
  - この worktree の `TMPDIR` は `/home/yosis/dev/ashiato2-up-ch17/.superpowers/sdd/tmp/a` で、git の木の中にある。
  - `d=$(mktemp -d)` の中（`.../.superpowers/sdd/tmp/a/tmp.*`）で走らせた結果:
    - `git ls-tree -r HEAD >/dev/null; echo $?` → `0`。
    - `git rev-parse --show-toplevel` → `/home/yosis/dev/ashiato2-up-ch17`（rc=0）。
    - `git ls-tree -r HEAD | wc -l` → `0`。

    git は親の repo を見つけ、cwd の下の項目だけを出す。
  - そのため、正しく作った台本でも、tasks 1.1 の 4 つ目の検証（41 行）と自己試験の Scenario「鍵を出せない木では、鍵を出さずに落ちる」は、この環境では鍵を出して rc=0 になる。検証のほうが落ちる。
    Global Constraints（22 行）は、一時の置き場を `${TMPDIR:-/tmp}` の下にすることを求めている。
  - 同じ理由で、D1 の「鍵 = `git ls-tree -r <commit>` の行」は、台本を repo の根以外から走らせると別の鍵になる。
- kind: technical
- 提案: 台本は `git -C "$(git rev-parse --show-toplevel)" ls-tree -r --full-tree` で読む（`rev-parse` が失敗したら落ちる）。
  「git の木でない置き場」の確かめは `GIT_CEILING_DIRECTORIES="$d"`（か `GIT_DIR=/nonexistent`）を付けて走らせる。tasks 1.1 の 41 行の検証も同じように直す。
- 処置: fixed 1.1 —— 台本は repo の根で ls-tree --full-tree を読み、git の木でない置き場の確かめは GIT_CEILING_DIRECTORIES を付ける（tasks 1.1 の検証も）。cwd に依らないことも自己試験で見る

## R9. 本人の「何を」の後半（Rust のビルドにキャッシュを効かせる）に、検証も完了の判定も無い
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/tasks.md（2.1・2.2）, docs/changes/CH17.md（完了の判定）
- 根拠:
  - `docs/changes/CH17.md` の「議論で決めたこと」の何を:「あわせて package-windows の Rust のビルドにキャッシュを効かせる」。価値も「ビルドするときも Rust の依存を組み直さない」と言う。
  - それを実装するのは design D5 と tasks 2.1 の 48-49 行だけ。
  - spec に Scenario が無い。tasks 2.2 の `--check-workflow` の項目にも Rust のキャッシュの step が無い。完了の判定 1〜4 にも、ビルドする run の時間が縮むことが無い。
  - そのため、step を書き忘れても、鍵の誤り（例: `rustc -V` を入れない）があっても、全部の検査が緑になる。
- kind: technical
- 提案: `--check-workflow` に「引けなかったときだけ、`~/.cargo/registry`・`~/.cargo/git`・`target` を、`rustc -V` と `Cargo.lock` の hash を持つ鍵で戻す」を足す。
  完了の判定に、Rust のキャッシュが戻ったことの確かめを足す。例:「2 回目にビルドする run の `actions/cache` の step が cache hit（restore-keys の一致を含む）を出す」。
- 処置: fixed specs/distribution/spec.md —— Rust のキャッシュの SHALL と Scenario「ビルドするときは、Rust の依存のビルドを戻す」を足し、tasks 2.2 の --check-workflow と自己試験 (h) で見る。完了の判定 5 を足した

## R10. 完了の判定は同じ commit の再実行しか見ず、本題（台本だけを変えた別の commit で使い回す）を確かめない
- 成果物: docs/changes/CH17.md（完了の判定 3・4）, design.md:110-111
- 根拠:
  - 完了の判定 3 は「同じ PR の CI を同じ commit でもう一度走らせると」。同じ commit なら、鍵から何も除かなくても一致する。D1 の「除くもの」が効くかどうかは、本物の Actions では一度も確かめない。
  - design の「確かめ」（110-111 行）は「その後に台本だけを変えた push で確かめる」と書いているが、完了の判定にはその項が無い。
  - 価値（CH17.md の「試験の台本と文書だけを変えた PR の push では…ビルドし直さず」）に直に当たる判定が無い。
- kind: technical
- 提案: 完了の判定に足す:「同じ PR に `tools/release/install-check.sh` だけを変えた commit を push すると、`package-windows` がビルドを飛ばし、`install-windows` が Scenario を全部走らせて通る」。
  確認バッチの問い（4）も、その run を指すように直す。
- 処置: fixed design.md —— design の「確かめ」と docs/changes/CH17.md の完了の判定 4 に、台本だけを変えた別の commit の PR で使い回すことの確かめを足し、確認バッチの問いをその run に向けた

## R11. pull_request の run は merge の commit の workflow で走るが、鍵には head の `ci.yml` が入る
- 成果物: openspec/changes/ch17-windows-install-ci-reuse/design.md（D1 の 36 行・D2）
- 根拠（GitHub の振る舞いの部分は未確認。repo の中の裏付けは下の 1 点目だけ）:
  - 正典 `openspec/specs/distribution/spec.md:2260-2261` は「GitHub が作る、head とその時の main を合わせた merge の commit」と言い、
    「PR の CI の配布物を検査する job は、ほかの job と同じく head とその時の main を合わせた木で走り」とも言う。
    pull_request の run の既定の木は merge の commit になる。`package-windows` だけが `ref:` で head の commit を checkout する（`ci.yml:458-460`）。
  - pull_request の run の workflow の定義（`ci.yml` そのもの）も merge の commit から読まれる。ここは GitHub の一般の振る舞いで、この repo では確かめていない（未確認）。
  - D1 の鍵は head の木の `ci.yml` の blob を使う。main で `package-windows` の step の `env` や手順が変わった後に、古い main から切った PR へ台本だけを push すると、
    鍵は前の run と同じのまま、走る workflow だけが新しい。新しい手順で組んだ配布物は作られない。
  - node・Rust の版の変更は道具の版の行で拾える。拾えないのは `env:` や step の中身の変更。
- kind: technical
- 提案: まず、pull_request の run がどの commit の `ci.yml` で走るかを確かめる。merge の commit なら、走っている workflow の定義を鍵に入れる。
  `package-windows` の checkout は head だけなので、merge の commit の `ci.yml` は手元に無い。取り方は別に要る（例: その commit を追加で fetch して blob を読む）。
  入れないなら、入れない理由と、それで使い回してよい範囲を D1 に書く。

---

## 観点 1: deep の決定が正典に写っているか

該当なし（確かめた範囲: `deep.md` は軽い路の申告だけで、本人の答えが無い。代わりに `docs/changes/CH17.md` の「議論で決めたこと」の 3 項目を突き合わせた）。

- 何を（使い回し）: design D1・D2 と spec に写っている。
- 何を（Rust のキャッシュ）: 検証が無い（R9）。
- どこまで（他の job は変えない）: D3 が守っているが、正典と食い違う（R2）。
- 本人の決定に数値は無い。

## 観点 2: Scenario が検証可能か

R3・R6・R7（本文だけが言っていて Scenario が言っていないこと、主張の束ね）。
正典側の本文と Scenario の差は R1・R2。
ほかの Scenario は、観測できる言葉（鍵が同じ / 違う・rc・標準出力が空・`ci.yml` の step の条件）で書かれている。

## 観点 3: 置き場の誤り

- design D3 は、観測できる振る舞い（`release-package` の Windows の配布物の `SOURCE.txt` と印の commit）を design にだけ置いている（R2 に含めた）。
- D2 の「台本の自己試験が落ちたら job を落とす」も spec に無い。軽いので R にはしない。
- proposal の Capabilities（`distribution`）と `specs/distribution/` は一致する。CH17.md の `capability: [distribution]` とも一致する。
  change は `docs/stories/INDEX.md` の表に載らない（`grep CH17` は 0 件）。

## 観点 4: tasks が検証を持つか

- 1.1・2.2・2.3・3.1 はコマンドと終了条件を持つ。2.1 は検証を持たないが、2.2・2.3 が同じ `ci.yml` を確かめる。
- 1.1 の検証の 1 つは、この環境で書いたとおりには通らない（R8）。
- Rust のキャッシュに検証が無い（R9）。
- 依存順は Task 1（台本）→ Task 2（`ci.yml` と `--check-workflow`）→ Task 3（文書）で正しい。
- 人間の確認待ちは要らない（完了の判定 2・3 は CI の run で機械で分かる）。
- Global Constraints が許すパス（`.github/workflows/ci.yml`・`tools/release/windows-build-key*.sh`・`docs/testing.md`・`docs/adr/*.md` と change の置き場）は、`lane.json` の paths の中にある。
  Task 1〜3 が触るのもその中だけ。

## 観点 5: change の文書（`docs/changes/CH17.md`）

- 完了の判定は書かれている。ただし本題の確かめが無い（R10）。
- 「壊してはいけないもの」には、PR の配布物の `SOURCE.txt` と印が head の commit を指すこと（正典 spec.md:2259-2260）が無い。この change はまさにそれを崩す（R2）。
- 「依頼（逐語）」の範囲: specs は越えていない。後半（Rust のキャッシュ）は spec にも検証にも無い（R9）。

## 観点 6: 答えが写ったか

該当なし（`spec --answers` の呼び出しではない）。

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）
- 処置: fixed D1 —— 走っている run の CI の定義（github.sha の ci.yml の blob）を鍵に入れ、fetch できなければ鍵を空にしてビルドする。GitHub の振る舞いは未確認（確かさ 1）と書いた。tasks 2.1・2.2 と Scenario も

