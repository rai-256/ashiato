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

## R3. `--check-workflow` は、restore の `== 'pull_request'` を `!=` に変えた ci.yml を通す。この形では、PR は使い回さず、main への push が使い回す
- 成果物: tools/release/windows-build-key.sh（`s_restore`）/ specs の Requirement「WHEN 幹（`main`）への push … 使い回さずにビルドする」
- 根拠: `sed "s/github.event_name == 'pull_request' && steps.wbk/github.event_name != 'pull_request' && steps.wbk/"` で作った版（diff は ci.yml:485 の 1 行だけ）に、
  `tools/release/windows-build-key.sh --check-workflow` を当てると **rc=0** だった。`s_restore`（windows-build-key.sh:97-98）は、`if:` に `"pull_request"` の字面があるかだけを見ている。
  自己試験の壊し (d) は条件を丸ごと消す版だけで、向きの反転は見ていない。spec の Scenario「前の組み立ては、鍵が空でなく完全に一致した PR の run でだけ引く」と Requirement の main の SHALL を、字面の一部だけで固定している。
- kind: technical
- 提案: `s_restore` は `github.event_name == 'pull_request'` の完全な字面（`&&` で結んだ項の 1 つ）を見る。`mutate` に `!=` へ反転した版を足す。

## R4. 除くパスの一覧の広げすぎを、どの試験も止めない（`crates/installer/*` や `tools/release/package.sh` を足しても自己試験は通る）
- 成果物: tools/release/windows-build-key.sh（`EXCLUDE_PATHS`・自己試験）/ specs の Requirement「除くのは … だけ。名指ししていないファイルが 1 つでも変われば、鍵は変わる」
- 根拠: 台本の写しの `EXCLUDE_PATHS` に `'crates/installer/*'` を足した版は、`--self-test` が **rc=0** だった（`grep` で、写しの 24 行目に足した行があることを確かめた）。
  同じく `'tools/release/package.sh'` を足した版も **rc=0** だった。後者では、`tools/release/package.sh` の中身だけが違う 2 つの commit の鍵が、本物の台本では `d6773b6de057…` と `cf9fba8b7541…` で違い、
  写しではどちらも `608eea347c43…` で同じになった。つまり配る組み立ての手順を変えても、前の配布物を使い回す。
  自己試験は、組み立てに入るファイルを 11 個の標本で確かめるだけで、除くパスの一覧が spec の 6 項目と同じであることは見ていない。spec の「除くのは … だけ」（上限）は、どのテストも固定していない。
  広げすぎは「古い配布物で試験が通る」側に倒れる（design の Risks の 1 つ目）。今の一覧は spec と同じなので、いま壊れているものは無い。
- kind: technical
- 提案: 自己試験に「`EXCLUDE_PATHS` が spec の 6 項目とちょうど同じ」の確かめを足す（台本の配列を出して、固定の 6 行と比べる）。一覧を変えるときは、spec と自己試験を一緒に変えることになる。

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
