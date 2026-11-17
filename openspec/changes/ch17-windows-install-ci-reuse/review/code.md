<!-- 処置の判定: re-review -->
# CH17 code_verify — 独立検証（2026-11-17）

申告: 6/6（tasks の `[x]` は 1.1・2.1・2.2・2.3・3.1・3.2）・検証コマンド 14 本

| # | 検証コマンド（tasks.md） | 申告 | 実測 |
|---|---|---|---|
| 1.1a | `tools/release/windows-build-key.sh --self-test` | rc=0 | rc=0（最後の行は「ok: 変えた版 13 通りは全部落ちる」） 一致 |
| 1.1b | 自己試験の出力に鍵の 5 つの Scenario がある | rc=0 | rc=0 一致 |
| 1.1c | `windows-build-key.sh HEAD` が 64 桁の 16 進 | rc=0 | rc=0 一致 |
| 1.1d | git の木でない置き場で rc≠0 と空の出力 | rc=0 | rc=0 一致 |
| 2.2a | `--check-workflow` | rc=0 | rc=0 一致 |
| 2.2b | `--check-workflow` の出力に CI の定義の 8 つの Scenario がある | rc=0 | rc=0 一致 |
| 2.2c | 自己試験が `--check-workflow` の節を飛ばしていない | rc=0 | rc=0 一致 |
| 2.3a | `tools/release/check-guards.sh workflows` | rc=0 | rc=0 一致 |
| 2.3b | `tools/release/check-guards.sh --self-test` | rc=0 | rc=0 一致 |
| 2.3c | `tools/check-ci-ch14-step.sh` | rc=0 | rc=0 一致 |
| 2.3d | merge-base と比べて、`package-windows` の step 以外は同じ（python + yaml） | rc=0 | rc=0 一致 |
| 2.3e | `changes.sh code` の判定がそのまま | rc=0 | rc=0 一致 |
| 3.1 | `docs/testing.md` に `windows-build-key.sh` と `release-package` がある | rc=0 | rc=0 一致 |
| 3.2 | `docs/release.md` §11.1 に「使い回した」がある | rc=0 | rc=0 一致 |
| — | `python3 scripts/check_scenarios.py . ch17-windows-install-ci-reuse`（tasks には無い。merge gate が走らせる） | — | **rc=1（担保の無い Scenario 8 件）** 不一致 → R1 |

## 手 1: 固定値の独立な再計算

テストは鍵の期待値を直書きしていない（どれも 2 つの木の鍵が同じか違うかを比べる）。そのうえで、鍵そのものを python（`hashlib`・`git ls-tree -r --full-tree HEAD`・除くパスの判定は `startswith` と `endswith`）で別に計算し、
同じ `WINDOWS_BUILD_KEY_TOOLS` で台本の出力と比べた。台本も python も `fc43740ecbd79a8157595cd325471e393254db9684e014c0aa44aac85c1a1670` を出し、**一致**した。bash の glob（`docs/*` の `*` が `/` も越える）は、design D1 の「`docs/`・`openspec/` の下・`*.md`」と同じ集合を除いている。指摘なし。

## R1. CI の定義の 8 つの Scenario に、`check_scenarios.py` が拾える印が無い。merge gate が落ちる
- 成果物: tools/release/windows-build-key.sh / openspec/changes/ch17-windows-install-ci-reuse/specs/distribution/spec.md
- 根拠: `python3 scripts/check_scenarios.py . ch17-windows-install-ci-reuse` は rc=1 で、`[FAIL] 担保の無い Scenario` が 8 件出た（ADDED の CI の定義の 7 件と、MODIFIED の「使い回した Windows の配布物は、組んだ commit のソースの場所と印のまま上げる」）。
  同じ実行で `[warn] spec に無い Scenario を指す印:   (tools/release/windows-build-key.sh)` も出た。
  `check_scenarios.py:39` の `MARK`（`Scenario:\s*([^\n"']+?)\s*(?:$|"|'|\*/)`）を台本に当てると、拾えるのは鍵の 5 件と空白 1 件だけだった。CI の定義の Scenario 名は `scenario("<名前>", s_restore)`（windows-build-key.sh:231-238）の引数にしか無く、
  `print("== Scenario: " + name)` は名前を連結で組むので、印として読めない。
  `scripts/merge_gate.sh:178` は下流で `check_scenarios.py` の rc≠0 を `note`（`[FAIL]`）にするので、このままでは merge gate を通らない。
  `--check-workflow` を走らせれば 8 件とも実際に検査され `ok` を出す。担保はあるが、印が無い状態。
- kind: technical
- 提案: `scenario(...)` の呼び出しごとに `# Scenario: <名前>` のコメント（字面は spec と一字一句）を台本に置く。そのあと `check_scenarios.py` が rc=0 になることを確かめる。
- 処置: fixed 2.2 — （`scenario(...)` の呼び出しごとに `# Scenario: <名前>` の印を spec と一字一句で置き、`print` は `"== %s: %s" % ("Scenario", name)` にして空の印を拾わせない。`check_scenarios.py` は rc=0・`scenarios: OK`）

## R2. `--check-workflow` は、restore を組み立ての後ろへ動かした ci.yml を通す。この形では、引けても毎回ビルドし、検査は緑のまま
- 成果物: tools/release/windows-build-key.sh（`s_asis`・`s_skip`・`s_restore`）/ .github/workflows/ci.yml
- 根拠: 一時の複製で、`package-windows` の `actions/cache/restore`（id `reuse`）を、2 つの `package.sh windows` の後、最初の `upload-artifact` の直前へ動かした
  （並び: wbk → Rust の cache → pgsql の cache → ch14_ → package.sh ×2 → **reuse** → upload ×2 → save）。`tools/release/windows-build-key.sh --check-workflow <その版>` は **rc=0** で、
  「引けたときだけ組み立てを飛ばし、今と同じ名前で上げる」と「使い回した Windows の配布物は、組んだ commit のソースの場所と印のまま上げる」はどちらも `ok` を出した。
  この並びでは、`ch14_` と `package.sh` の `if: steps.reuse.outputs.cache-hit != 'true'` が restore より前に評価される。値はまだ空なので、毎回ビルドする。
  そのあと restore が当たると、組んだ置き場の上に前の組み立てを展開し、save は飛ばされる。
  `s_restore` が見る並びは「鍵の step より後」だけで、`s_asis` が見る範囲は「restore の後から最初の upload の前まで」だけ。組み立ての step が restore より後にあることを、どの関数も見ていない。
  33 分の短縮（この change の目的）が無言で消え、どの検査も落ちない。自己試験の 13 通りの壊しの中に、並びの壊しは無い。
- kind: technical
- 提案: `s_skip`（か `s_asis`）に「`ch14_` の step と 2 つの `package.sh windows` の step は restore より後にある」を足す。`mutate` に「restore を組み立ての後ろへ動かした版」を足し、`--check-workflow` が落ちることを自己試験で見る。
- 処置: fixed 2.2 — （`s_skip` に「`ch14_` と 2 つの `package.sh windows` は restore より後」と「restore の `cache-hit` を見る step は restore より後」を足し、`mutate` に restore を最初の upload の直前へ動かした版（n）を足した。指摘の版は `NG: step 7（ch14_ の試験か package.sh windows）が restore より前にある` で rc=1）

## R3. `--check-workflow` は、restore の `== 'pull_request'` を `!=` に変えた ci.yml を通す。この形では、PR は使い回さず、main への push が使い回す
- 成果物: tools/release/windows-build-key.sh（`s_restore`）/ specs の Requirement「WHEN 幹（`main`）への push … 使い回さずにビルドする」
- 根拠: `sed "s/github.event_name == 'pull_request' && steps.wbk/github.event_name != 'pull_request' && steps.wbk/"` で作った版（diff は ci.yml:485 の 1 行だけ）に、
  `tools/release/windows-build-key.sh --check-workflow` を当てると **rc=0** だった。`s_restore`（windows-build-key.sh:97-98）は、`if:` に `"pull_request"` の字面があるかだけを見ている。
  自己試験の壊し (d) は条件を丸ごと消す版だけで、向きの反転は見ていない。spec の Scenario「前の組み立ては、鍵が空でなく完全に一致した PR の run でだけ引く」と Requirement の main の SHALL を、字面の一部だけで固定している。
- kind: technical
- 提案: `s_restore` は `github.event_name == 'pull_request'` の完全な字面（`&&` で結んだ項の 1 つ）を見る。`mutate` に `!=` へ反転した版を足す。
- 処置: fixed 2.2 — （`s_restore` は `if:` を `&&` の項に割り、`github.event_name == 'pull_request'` がちょうど 1 項としてあること・`||` と否定が無いことを見る。`mutate` に `!=` へ反転した版（o）を足した。指摘の sed の版は rc=1）

## R4. 除くパスの一覧の広げすぎを、どの試験も止めない（`crates/installer/*` や `tools/release/package.sh` を足しても自己試験は通る）
- 成果物: tools/release/windows-build-key.sh（`EXCLUDE_PATHS`・自己試験）/ specs の Requirement「除くのは … だけ。名指ししていないファイルが 1 つでも変われば、鍵は変わる」
- 根拠: 台本の写しの `EXCLUDE_PATHS` に `'crates/installer/*'` を足した版は、`--self-test` が **rc=0** だった（`grep` で、写しの 24 行目に足した行があることを確かめた）。
  同じく `'tools/release/package.sh'` を足した版も **rc=0** だった。後者では、`tools/release/package.sh` の中身だけが違う 2 つの commit の鍵が、本物の台本では `d6773b6de057…` と `cf9fba8b7541…` で違い、
  写しではどちらも `608eea347c43…` で同じになった。つまり配る組み立ての手順を変えても、前の配布物を使い回す。
  自己試験は、組み立てに入るファイルを 11 個の標本で確かめるだけで、除くパスの一覧が spec の 6 項目と同じであることは見ていない。spec の「除くのは … だけ」（上限）は、どのテストも固定していない。
  広げすぎは「古い配布物で試験が通る」側に倒れる（design の Risks の 1 つ目）。今の一覧は spec と同じなので、いま壊れているものは無い。
- kind: technical
- 提案: 自己試験に「`EXCLUDE_PATHS` が spec の 6 項目とちょうど同じ」の確かめを足す（台本の配列を出して、固定の 6 行と比べる）。一覧を変えるときは、spec と自己試験を一緒に変えることになる。
- 処置: fixed D1 — （自己試験の「組み立てに入るファイルが 1 つでも違えば、鍵が変わる」で、`EXCLUDE_PATHS` が spec の名指しの 6 つちょうどであることを比べる。`'crates/installer/*'`・`'tools/release/package.sh'` を足した写しの `--self-test` はどちらも `NG: 除くパスが spec の 6 つと違う` で rc=1。design D1 に追記）

## R5. `--check-workflow` を走らせる CI の job が無く、ci.yml の使い回しの形は merge の後に誰も見ない
- 成果物: .github/workflows/ci.yml / tools/release/windows-build-key.sh / tasks.md 2.2（「Linux の手元と merge gate で走らせる」）/ design D2（ST58 が置き場を足したら「上げる置き場は全部、保存と引きの置き場に入る」が見る）
- 根拠: `grep -n "windows-build-key\|check-workflow"` を `.github/workflows/*.yml`・`scripts/merge_gate.sh`・`tools/verify-prep.sh`・`tools/pre-commit.sh`・`docs/flow-gates.md` に当てた。
  当たったのは ci.yml:465（Windows の `package-windows` の `--self-test`）と ci.yml:479（鍵を出す呼び出し）だけだった。`--check-workflow` を呼ぶ場所はどこにも無い。
  Linux の自己試験を並べる job（ci.yml:652-661 の `version.sh`・`check-guards.sh`・`package.sh`・`install-check.sh` などの `--self-test`）にも、`windows-build-key.sh` は無い。
  Windows の runner の `--self-test` は、`python3` か `yaml` が無ければ CI の定義の節を飛ばす（windows-build-key.sh:322-325）。windows-latest に `yaml` があるかは、手元では確かめられない。
  このため、後の Story が `package-windows` に `upload-artifact` を足して restore と save に入れ忘れても、その PR の CI では落ちないかもしれない。走っている ST58 が試験の形を 6 つに分ける件（deep.md C4）が、まさにこれに当たる。
  直す先は Linux の job への 1 行で、tasks の Global Constraints（「`ci.yml` で変えてよいのは `package-windows` の job の step だけ」）の外になる。
- kind: conflict
- 提案: Linux の自己試験の job（`check-guards.sh --self-test` の並び）に `tools/release/windows-build-key.sh --check-workflow` と `--self-test` を足す。範囲を広げるかを人間に返す。広げないなら、design D2 の「`--check-workflow` が見る」を「merge gate の時点で一度だけ見る」に書き直す。
- 処置: escalated —— deep.md 第 2 回 Q3 で答えが出た。本人が推奨「足す（chain の job に 2 行）」を選んだ（2026-11-17、`deep-answers-2.txt`）ので、答えに沿って直した（ci.yml の `chain` の job の自己試験の並び（`draft.sh --self-test` の後）に `./tools/release/windows-build-key.sh --self-test` と `--check-workflow` の 2 行を足した。`--check-workflow` は yaml が無ければ rc=2 で落ちる。tasks 2.2 の「merge gate で走らせる」を「`chain` の job で走らせる」に、Global Constraints に例外を、design D2・台本の冒頭の説明を直した）

## 手 2: ガードをわざと壊す

R2・R3・R4 のほかに、次を壊して確かめた（どれも期待どおり）。
- `tools/check-ci-ch14-step.sh`: 写しの ci.yml で `ch14_` の step を 2 つ目の `package.sh windows` の後ろへ動かすと **rc=1**、写しを戻すと **rc=0**。step に `if:` を足しても、並びの検査は効いたまま
- `--self-test` の 13 通りの壊し（a〜m）は、どれも `--check-workflow` を落とす（1.1a の出力「変えた版 13 通りは全部落ちる」）

## 手 3: Scenario と test

- 鍵の 5 件は、使い捨ての git の木で実際に鍵を出して比べている。spec の主張（ファイルの中身から鍵を作る）と同じ階層で、指摘なし
- CI の定義の 8 件は、ci.yml の YAML を静的に読む検査。spec の主張（「CI の定義」「手順を読む」）と同じ階層だが、印が無い（R1）。2 件は並びと向きの壊しを通す（R2・R3）
- 「使い回した run でも、インストールと更新の Scenario が 1 つ欠ければ CI が落ちる」: `s_install` が見るのは、`--list` の突き合わせの字面（`comm -23`・`test ! -s`）と `if:` が無いことだけ。「2 つの配布物で走らせる」ことは見ていない。
  ただし `install-windows` の job は 2.3d で変わっていないことを確かめた。今の job は release と test の 2 つの zip を走らせている（ci.yml:556-557・566-567）

## 手 4: 本人の決定がテストで固定されているか

`deep.md` の本人の答えは Q1（使い回しを認める）と Q2（使い回した配布物は組んだ commit を指す・`package-artifacts` は変えない）の 2 つで、数値は無い。
- Q1: `--self-test` の 1 つ目の Scenario（台本と文書だけの違いで鍵が同じ）が固定している。`install-check.sh` を外した写しで落ちることも自己試験が見る
- Q2「書き換えずに上げる」: `s_asis`（自己試験の壊し j・k）が固定しているが、並びの壊しは通す（R2）
- Q2「`package-artifacts` は変えない」: `check-guards.sh workflows` の Scenario「PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される」が、checkout と期待値の渡し方を固定している（2.3a・2.3b で rc=0）。それ以外の変化は、2.3d の merge-base との 1 回きりの比較だけが見る
- `docs/changes/CH17.md` の「試験の台本（tools/release/ など）だけが変わった push」: 除くパスの一覧の下限は固定、上限は固定していない（R4）

## 手 5: tasks の `[x]` と実体

6 件とも、本文の検証コマンドは実在し、全部 rc=0 だった（冒頭の表）。2.1 には自分の検証コマンドが無く、2.2 の `--check-workflow` が代わりに見ている。`cargo test --test` 型の指定は無い。
指摘なし。ただし merge gate が走らせる `check_scenarios.py` は rc=1（R1）。

## 手 6: 隙間

- 使い回した配布物を受け取る側に、head の commit と比べる箇所は無かった。
  - `install-check.sh`・`update-check-install.sh`・`check-artifacts.sh` を `rev-parse|GITHUB_SHA|head\.sha|BUILD_COMMIT|describe` で grep すると、当たりは 0 件だった
  - `check-artifacts.sh` は、配布物ごとに印と `SOURCE.txt` の commit を比べるだけで（:164-167）、配布物どうしは比べない
  - `package-artifacts` の期待値は `version.sh source built/Cargo.toml` と `built/LICENSE`（ci.yml:609）で、2 つとも鍵に入っている（自己試験の 2 つ目の Scenario が `LICENSE`・`Cargo.toml` を変えて鍵が変わることを見る）
- 除く `*.md`・`docs/`・`openspec/` を組み立てが読むか: `build.sh`・`package.sh`・`notices.sh`・`pgsql.sh`・各 `build.rs`・`crates/*/src` の `include_str!`／`include_bytes!` を grep した。当たりは 0 件。`web/src` の当たりは、コメントの中の `docs/` だけだった
- `wbk` の step は `-c http.extraheader` で認証を足すが、checkout は `persist-credentials: false`（ci.yml:460）なので、認証の header は重ならない
- 確かめた範囲の外: Actions の上での当たり外れ・キャッシュの保存の競合・windows-latest の `python3`／`yaml` の有無・同じ PR の前の push の head を空の clone から `git fetch` できるか。最後の 1 つは `docs/changes/CH17.md` の完了の判定 4（整合の確認 C2）が持つ。「捨てたものは復元できない」型の新しい経路は見つからなかった

## 手 7: 機械が承認した訂正

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/ch17-windows-install-ci-reuse` は `[]`。`correction-review.md` は作っていない）。

## security review（f62c86d..daf1a5e）

見た経路:
- PR の run → `wbk`（PR の head の木から鍵）→ `reuse`（`actions/cache/restore`、PR の置き場か main の置き場）→ `upload-artifact` → `package-artifacts` の `release-package` → `draft.sh` の下書き（3）
- main への push の run → `reuse` は `github.event_name == 'pull_request'` で飛ぶ → 毎回組む → `save`（main の置き場）（4）
- `release.yml` / `nightly.yml` → `windows-package-` / `cargo-windows-` のキャッシュを引かず、自分で組む（4）
- `wbk` の `GH_TOKEN`（`github.token`、`permissions: contents: read`）→ `git -c http.extraheader` の引数だけ。`GITHUB_OUTPUT`・`echo` には出ない（ci.yml:473-482、3）
- 新しい Rust の依存のビルドのキャッシュ（`target`、`restore-keys: cargo-windows-<rustc>-`）→ PR の置き場からも戻る → `package.sh windows`（3）
- 鍵の台本 → `git ls-tree` と `sha256sum` だけ。外から来る値を eval・パスへ入れない（`mktemp -d` と `trap rm -rf` は自己試験だけ。windows-build-key.sh:341-342）（2）
- 依存: 足したものは無い（`actions/cache/restore` と `actions/cache/save` は既存の `actions/cache` と同じ commit `0057852…` に固定）（2）
- 私的データ: 差分はライフログの記録を読まず、書きもしない。キャッシュに入るのは、すでに artifact として上げている配る zip 2 つだけ（2）

安全の決め手: 「Windows の組み立てのキャッシュを引くのは `ci.yml` の PR の run だけで、main の push と配布の workflow（`release.yml` / `nightly.yml`）は必ず自分で組む」—— 確かさ 4。
`/tmp/ch17sec/check.py` で全 workflow の `actions/cache*` の step を YAML で読んだ。確かめた点は 3 つ。`windows-package-` / `cargo-windows-` の鍵を使うのは `ci.yml` だけ。引く step は `/restore@`・`== 'pull_request'` 付き・`restore-keys` 無し。token を出力へ書く行が無い。
本物の ci.yml では `rc 0`。restore の条件を `!=` に変えた写しでは `NG: PR 以外でも引く: ci.yml:package-windows#5 …` / `rc 1` で落ちる。
これで main の置き場と配布の流れは PR から汚せない。ただし **同じ PR の中** はこの事実の外にある（R6）。

## R6. 同じ PR の前の push が保存した Windows の zip と `target` を、後の run が検証せずに使う。レビューした head と違うコードの組み立てが `release-package` と下書きに載る
- 成果物: .github/workflows/ci.yml（`reuse`・cargo の cache・`save`）/ docs/release.md §11.1 / design.md D3・Risks「キャッシュの中身の改竄」
- 根拠（確かさ 3。Actions の上では再現していない）:
  1. `pull_request` の run は PR の merge commit の ci.yml と台本で走る。保存は PR の置き場（`refs/pull/N/merge`）に入り、同じ PR の後の run が引ける（GitHub の文書「current branch か default branch のキャッシュを引ける」。design D2・D3 は、この引きを「同じ PR の前の push」の使い回しとして使っている）。
     前の push の commit は、`windows-build-key.sh` か ci.yml の `save` を書き換えられる。鍵の値は後の head の木（とそれを書いた本人が知っている道具の版）だけから決まる（windows-build-key.sh の冒頭の説明、ci.yml:479）。
     そのため前の push は、後の head が出す鍵の名前で、任意の中身の zip を保存できる
  2. 後の run は鍵が完全に一致すると組み立てを飛ばし（ci.yml:485-519）、引いた置き場をそのまま `package-windows-release` / `package-windows-test` として上げる（ci.yml:522-525）。`package-artifacts` は、それを `release-package` に入れる（ci.yml:608-611）
  3. 中身を確かめる箇所は無い。`check-artifacts.sh` は zip の中の印と `SOURCE.txt` の一致だけを見る（:164-167）。
     2 つとも zip の中身なので、組んだ側が head の commit を名乗ることもできる。`draft.sh` が見るのは、run が同じ repo の枝・`ci`・成功の 3 つだけ（draft.sh:25-31。`head_sha` も `windows-build-key` も grep で 0 件）。
     今回の docs/release.md §11.1 は、Windows の 2 つについて `head_sha` との照合を外した
  4. 差分の前は、キャッシュから来る中身は pgsql の zip だけで、`pgsql.lock` の sha256 で検証していた（pgsql.sh:19-39）。検証しないキャッシュが配る zip に入る経路は、この差分で増えた。
     新しい cargo の `target` のキャッシュも同じく PR の置き場から戻り（`restore-keys` 付き）、組み立ての中間物を検証しない。
     design の Risks「キャッシュの中身の改竄」が扱うのは、他の PR と main からの汚染だけで、同じ PR の前の push は扱っていない
  - 起きるには PR の枝へ push できる者（グラフの agent か本人）が要る。途中の push を force-push で消せば、人間が merge の前に見る最後の diff には残らない。
    下書きは非公開。それでも `docs/release.md` §11.1 の確かめでは、人が下書きの zip を落としてインストールする
- kind: conflict （D3「`package-artifacts` は変えない」は本人の決定 Q2 の範囲。直し方によっては、Q1 の「同じ PR の前の push を使い回す」も狭まる）
- 提案: `save`（と cargo の cache の保存）を `github.event_name == 'push'`（main）に限る。PR は main が組んだものだけを引く。Q1 の範囲は狭まる。
  狭めないなら、`package-windows` の出力（`cache-hit`）を `package-artifacts` に渡し、使い回した run では `release-package` を上げない（D3 の反転の条件が挙げる案）。
  どちらにするかを人間に返す。

範囲外: `release.yml` の pgsql のキャッシュ（`restore-keys: pgsql-`）は既存のもの。中身は pgsql.sh が sha256 で検証している。
- 処置: escalated —— deep.md 第 2 回 Q4 で答えが出た。本人が推奨「受け入れる（今のまま）」を選んだ（2026-11-17、`deep-answers-2.txt`）ので、答えに沿って（ci.yml は変えない。design の Risks に「同じ PR の前の push の保存物を確かめずに使う」（書けるのは PR の枝へ push できる者だけ・release/nightly は自分で組む）を足し、`docs/release.md` §11.1 の「使い回した run の見方」に、組んだ commit を PR の履歴で確かめてから zip を使う手順を足した）

## /code-review（f62c86d..1dbe541）

見た: Claude Code の組み込みの `/code-review`（7 件）。写しは scripts/code_review_record.py

## R7. No CI job runs `windows-build-key.sh --check-workflow` (the structural guard that keeps the reuse safe). The only CI call is `--self-test` on the Windows runner, and that silently skips the workflow check when `python3`/`yaml` is missing.
- 成果物: .github/workflows/ci.yml:465
- 根拠: A later PR adds `restore-keys: windows-package-` to the restore step, or removes the `cache-hit` guard from a `package.sh windows` step. The script's header says the check runs "on Linux locally and in the merge gate", but grep finds it wired into nothing (no Linux job, no merge-gate script). On windows-latest the self-test prints "飛ばした" and exits 0, so CI stays green while stale or partial artifacts get reused.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 2.2 — （R5 と同じ直し。深掘り第 2 回 Q3 で `chain` の job（ubuntu）に `--check-workflow` を足した。そこでは yaml が無ければ「飛ばした」でなく rc=2 で落ちる。`package-windows`（ci.yml:465 付近）の自己試験の step は消した）

## R8. `EXCLUDE_PATHS` copies the "not code" classification (`docs/*`, `openspec/*`, `*.md`) from `tools/release/changes.sh` (`^(docs/|openspec/)|\.md$`) instead of sharing one definition.
- 成果物: tools/release/windows-build-key.sh:17
- 根拠: Someone widens `changes.sh code` (for example to skip `.superpowers/`) but not the key script, or the other way round. The two lists drift silently: either reuse quietly stops happening, or a path that `package.sh` reads gets excluded from the key and a stale Windows zip is reused.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 共有すると `tools/release/changes.sh` を変えることになり、tasks の Global Constraints（`changes.sh` に触れない・`changes.sh code` の判定はそのまま）と design D1（`changes.sh` に足すと `deps` の判定が動き `vulns` の job が走る）に反する。指摘の危険な向き（鍵だけが広がり、組み立てが読むパスを除く）は R4 の直しで自己試験が止める（写しで `'tools/release/package.sh'` を足すと rc=1）。逆の向き（`changes.sh code` だけが広がる）は、その変更では `package-windows` 自体が走らないか、鍵が除かないパスが増えるだけで、使い回しが減る側に倒れ、古い配布物は使わない

## R9. Exclusion matching runs against `git ls-tree` output, which C-quotes non-ASCII or special paths (core.quotePath). A quoted path starts with `"` and ends with `.md"`, so neither `docs/*` nor `*.md` matches it.
- 成果物: tools/release/windows-build-key.sh:299
- 根拠: A doc with a Japanese file name, e.g. `docs/設計.md`, is listed as `"docs/\350...\.md"`. It is not excluded, so editing it changes the key and forces a full Windows rebuild that the design says should be skipped. Using `ls-tree -z` or `-c core.quotePath=false` would avoid this.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1 — （`git ls-tree -r -z` と `read -r -d ''` で読み、名前を引用符で包ませない。前の版では `docs/設計.md` を足した木の鍵が `1e5a6793e861…` → `817a9c4d7a40…` と変わり、今の版では同じ `1e5a6793e861…` のまま。ASCII の木の鍵は前と同じ。自己試験の 1 つ目の Scenario に `docs/設計.md`・`openspec/説明 書.txt` を足した。design D1 に追記）

## R10. The full key self-test (temp repo, about 20 commits and checkouts, about 15 key computations) runs on every `package-windows` run on the Windows runner, which bills at 2x. It runs even when the cache hits and nothing is built.
- 成果物: .github/workflows/ci.yml:465
- 根拠: Every PR run with code changes spends extra Windows minutes on a pure-bash self-test that would give the same result in a Linux job such as `chain`/`package-linux`. That works against the CH17 goal of saving Windows minutes; the repo has already run out of its Actions budget once (2026-09-19).（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D2 — （`package-windows` の自己試験の step を restore の後へ動かし、`if: steps.reuse.outputs.cache-hit != 'true'` を付けた。引けたときは同じ台本・同じ ci.yml（どちらも鍵に入る）の run が自己試験を通して保存しているので、飛ばしても確かめは減らない。Linux へ移すかは Q3。design D2 に追記）

## R11. Every build with a new key saves a new `windows-package-<key>` cache containing both zips (each bundles PostgreSQL). It also saves a new Windows Rust `target` cache whenever Cargo.lock changes. Almost every code push changes the key, so most of these entries are never read.
- 成果物: .github/workflows/ci.yml:525
- 根拠: Several active PRs each push a few code commits and each push writes hundreds of MB of package cache in its PR scope, while Windows `target` caches add several more GB. That hits the 10 GB repo cache limit and LRU eviction removes main's `windows-package-*` and `pgsql-*` entries, which are the ones PRs could actually reuse. Reuse then quietly stops working.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: 正しさの欠陥ではない。GitHub のキャッシュは置き場が溢れると最後に使った日時の古い順に消すので、誰も引かない保存物が先に消え、毎回引かれる `pgsql-` と、同じ PR が直前に保存したものは残る。消えたときは組み直す（CH17 の前と同じ）だけで、古い配布物を使うことは無い。今のキャッシュは `gh cache list` で `pgsql-` の 1 件（307.78 MiB）だけ。溢れて使い回しが効かないと分かったときの手は design D2 と D5 の反転条件（Rust のキャッシュを `~/.cargo` だけに絞る）が既に持つ

## R12. The step always runs `git fetch --depth=1 origin $RUN_SHA` and passes the token as a `-c http.extraheader=AUTHORIZATION: basic …` argument on the command line, even on push runs where RUN_SHA == HEAD and `git rev-parse HEAD:.github/workflows/ci.yml` would be enough.
- 成果物: .github/workflows/ci.yml:477
- 根拠: On push-to-main runs this adds a network round trip that can fail. A fetch failure leaves the key empty and nothing is saved for later PRs to reuse. The base64 token also appears in the process arguments of every child process on the runner, where `GIT_CONFIG_*` environment variables would keep it out.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D2 — （run の commit が checkout した HEAD と同じ（main への push）なら HEAD から ci.yml の blob を読み、fetch しない。PR の run だけ fetch し、token は `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` の環境変数で subshell の中だけに渡す。手元で step の run を走らせ、push 相当（RUN_SHA=HEAD）は fetch なしで鍵を出し、PR 相当（引けない sha・偽の token）は鍵を空にして警告を出した。design D2 に追記）

## R13. The key is built from `ls-tree` of the commit, but `package.sh` builds from the checked-out working tree. Nothing confirms that the two match, so on a dirty or modified working tree the key describes a different tree than the one actually built.
- 成果物: tools/release/windows-build-key.sh:296
- 根拠: Any step between checkout and package (now or added later, e.g. a codegen or `npm` step that rewrites tracked files) changes the build input but not the key. The saved cache entry then claims to be the build of commit X while its contents differ, and later PR runs with the same key reuse it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D2 — （upload の後・save の前に `id: tree` の step を足し、`git status --porcelain --untracked-files=no` が空のときだけ `clean=true` を出す。save の `if:` に `steps.tree.outputs.clean == 'true'` を足し、変わっていれば警告を出して保存しない。`s_save` がこの step と条件を見て、条件を外した版（p）で `--check-workflow` が落ちる。design D2 に追記）
