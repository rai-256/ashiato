# CH20 実装タスク — 配る形の更新の確認を、本物の配布元で毎晩確かめる

読む順: `proposal.md` → このファイル → `design.md` → `specs/distribution/spec.md` → `docs/changes/CH20.md` → `premises.md` → `CLAUDE.md`。

深掘り第 1 回（Q1〜Q10。`deep.md`）の答えで書いた。製品のコードは変えない。検査（台本・workflow・試験の入口）だけを足す。

## Global Constraints

- **触ってよいのは次のパスだけ**: `.github/workflows/release-source-nightly.yml`（新）・`.github/workflows/ci.yml`（`chain` の job に 2 行だけ。Task 6）・
  `tools/release/release-source-check.sh`（新）・`tools/release/release_source.py`（新）・`tools/release/release-source-check.d/**`（新）・
  `docs/contracts/release-source-shape.json`（新）・`crates/collector-windows/tests/ch20_real_source.rs`（新）・`docs/testing.md`・`docs/release.md`
  （と `docs/changes/CH20.md`・`openspec/changes/ch20-real-release-source-nightly-check/**`）
- **製品のコードに触れない**: `crates/update-check/**`・`crates/updater/src/**`・`crates/updater/build.rs`・`crates/collector-windows/src/**`・`crates/collector-windows/build.rs`・
  `collector-android/**`・`crates/installer/**`・`crates/server/**`。3 つの食い違い（proposal の表）を直さない（深掘り Q9）。口が足りなければ直さずに止めて報告する
- **触れない台本と workflow**: `nightly.yml`・`release.yml`・`vulns-nightly.yml`・`ci.yml` の `chain` 以外の job（`install-windows` の外向きの遮断を含む）・
  `tools/release/update-check-install.sh`・`install-check.sh`・`build.sh`・`package.sh`・`check-guards.sh`・共有の材料 `docs/contracts/update-check-vectors.json`・`update-apply-vectors.json`
- **下流の検証と PR の CI は本物の配布元に問い合わせない**（正典 MODIFIED）。本物の答えの材料は上流で記録してある
  （`openspec/changes/ch20-real-release-source-nightly-check/material/`。2026-11-19 の `ashiato-list`（`200`・`[]`）・`ashiato-nightly`（`404`）・`cli-list`（`cli/cli` の `releases?per_page=1`）・
  `cli-tag`（その印 `v2.105.0` の 1 件）の `.status`・`.headers`・`.body`）。`tools/release/release-source-check.d/` へはこれを写して使い、取り直さない
- 判定の期待は正典から決める（spec・design D10）。部品のいまの振る舞いから写さない —— 写すと 3 つの食い違いが緑になる
- 試験の `# Scenario: <名前>` の印は、spec の `#### Scenario:` と一字一句合わせる（bash は `echo "== Scenario: <名前>"`）
- 一時ファイルは `"${TMPDIR:-/tmp}"` の下に `mktemp -d` で作り、終わりに消す（`/tmp` の固定名に書かない）。`release_source.py` は python の標準のものだけを使い `python3 -I` で呼ぶ。
  **例外は `--check-workflow` だけ**: workflow を `yaml` で読むので `-I` を付けずに呼び、`yaml` が無ければ 0 でない終了コードで落ちる（CH17 の `windows-build-key.sh --check-workflow` と同じ）
- `cargo` を走らせる検証と `collect --replay-only`（試験の組み立てを組む）は repo の根から `scripts/quiet-run <名前> -- …` で走らせる
- 赤の行の形は 1 行 1 件、タブ区切りで `NG<TAB><部品><TAB><場合><TAB>期待=<…><TAB>実際=<…>`。部品は `updater-release`（配る形の更新係）・`updater`（試験の組み立ての更新係）・
  `collector`（試験の組み立ての PC の収集アプリ）・`collector-dev`（開発版の版で組んだ PC の収集アプリ）・`shape`（答えの形）・`source`（届かない夜・比べに使う repo）・`collect`（記録の無い段）。
  場合は `real`・`real-dev`・`same`・`newer`・`nightly-object`・`full-20`・`status-503`・`redirect-offhost`。注意は `WARN<TAB>…`。
  要約の先頭に「既に分かっている食い違い」（部品と場合の組で名指し: `updater`/`collector-dev` の `nightly-object`・`updater`/`collector` の `full-20`・`collector` の `status-503`・`redirect-offhost`・
  `shape` の 1 件の道の答えの形）と「それ以外の赤」を分けて出す（spec）

## Task 1: 判定の本体と、比べる基準

- [ ] 1.1 `docs/contracts/release-source-shape.json` を置く（design D5。先頭の `"_note"` に「偽の配布元が返している形の写しで、正しい形の宣言ではない」と書く）。
  `tools/release/release_source.py` に、置き場（design D2 の形: `fetch/`・`replay/`・`records/`・`history/`）だけを読む `judge` を置く: 本物の状態の期待と届かなかった段の扱い（spec・design D10）・
  流し直しの期待（spec の場合ごとの期待）・答えの形の比べ（読む項目の有無と型・道ごとの答えの形・一覧の本文の上限・知らない状態コードは注意。読まない項目と値の変化は見ない）・
  記録の無い段（`records/mode` による。design D2）・180 秒の記録（`records/updater-release-real.timeout`）・届かない夜の数え方（予定の起動の実行だけ。design D8）。
  結果を標準出力の `NG` / `WARN` の行（標準出力にはこの 2 種の行だけを出す）と、置き場のファイルの要約（`summary.md`。先頭で既に分かっている食い違いとそれ以外の赤を分ける）と `summary.json`（`reached`・`ng`・`warn`・比べに使った repo の名前）に出し、赤なら rc=1、緑なら rc=0。
  `tools/release/release-source-check.sh judge <置き場>` がそれを呼ぶ。
  検証: `bash -n tools/release/release-source-check.sh` rc=0 /
  `python3 -I -m py_compile tools/release/release_source.py` rc=0
- [ ] 1.2 `tools/release/release-source-check.d/` に自己試験の材料を置く（自己試験と `--replay-fixture` はここだけを読む）: 本物の答え（change の `material/` から写す）と、そこから手で変えた答え（`reactions` を足す・`digest` を消す・`draft` を文字列にする・
  値だけを変える・一覧の本文を 70000 バイトにする・状態 `502`・`cli/cli` の一覧を `[]` にする）と、場合ごとの確かめた記録（正典どおり・正典と違う・本物の段で上限）と、
  `records/mode` の有無・180 秒の印と、前の実行の要約（予定の起動で届いた・届かない・手動の起動・読めない）。
  `release-source-check.sh --self-test` が、材料に `judge` を当てて次の Scenario を確かめる（各 Scenario の前に `echo "== Scenario: <名前>"`）:
  「部品の記録が無い段は赤になる」/「更新係のサービスが 180 秒のうちに確かめなければ赤」/「要約は既に分かっている食い違いとそれ以外の赤を分けて出す」/
  「空の一覧で「確かめた・新しい版なし」を記録すれば緑」/「流し直した同じ版を正典どおりに記録すれば緑」/「流し直した新しい版を正典どおりに記録すれば緑」/
  「開発版の道の 404 は「配布元が見つからない」なら緑で、食い違いを要約に出す」/「部品の記録が正典と違えば赤で、部品・場合・期待・実際を出す」/
  「別の宛先への 301 を「読めない答え」と記録しなければ赤」/「部品が本物の段で届かなかったと記録すれば、その段は期待と比べない」/
  「読む項目が揃い型が同じなら、読まない項目が増えても緑」/「読む項目の値が変わっても緑」/「読む項目が消えれば赤」/「読む項目の型が変われば赤」/
  「道ごとの答えの形が基準と違えば赤」/「一覧の本文が上限を超えれば赤」/「部品の知らない状態コードは形を比べずに注意を出す」/
  「自分の repo にリリースが無い間は、別の repo で項目を見たと要約に出す」/「比べに使う repo の答えを読めなければ赤」/
  「届かない夜は注意だけで、飛ばした判定の名前を出す」/「届かない夜も、本物に依らない流し直しは判定する」/「3 晩続けて届かなければ赤」/
  「手動の起動の実行は晩に数えない」/「前の実行を読めなければ、今夜だけで数える」。
  自己試験は、基準の本文の上限が `crates/update-check/src/lib.rs` の `MAX_RELEASES_JSON_BYTES`（`64 * 1024`）と同じ値であることも見る（読めなければ落ちる）。
  自己試験は、期待の表の `503` の期待「読めない答え」を「配布元に届かない」に替えた写しの `release_source.py` でも走らせ、
  「部品の記録が正典と違えば赤で、部品・場合・期待・実際を出す」が落ちることを見る（期待が効いていることの確かめ）。本物に問い合わせない。
  検証: `tools/release/release-source-check.sh --self-test` rc=0 /
  `bash -c 'out=$(tools/release/release-source-check.sh --self-test); for s in "部品の記録が無い段は赤になる" "更新係のサービスが 180 秒のうちに確かめなければ赤" "要約は既に分かっている食い違いとそれ以外の赤を分けて出す" "空の一覧で「確かめた・新しい版なし」を記録すれば緑" "流し直した同じ版を正典どおりに記録すれば緑" "流し直した新しい版を正典どおりに記録すれば緑" "開発版の道の 404 は「配布元が見つからない」なら緑で、食い違いを要約に出す" "部品の記録が正典と違えば赤で、部品・場合・期待・実際を出す" "別の宛先への 301 を「読めない答え」と記録しなければ赤" "部品が本物の段で届かなかったと記録すれば、その段は期待と比べない" "読む項目が揃い型が同じなら、読まない項目が増えても緑" "読む項目の値が変わっても緑" "読む項目が消えれば赤" "読む項目の型が変われば赤" "道ごとの答えの形が基準と違えば赤" "一覧の本文が上限を超えれば赤" "部品の知らない状態コードは形を比べずに注意を出す" "自分の repo にリリースが無い間は、別の repo で項目を見たと要約に出す" "比べに使う repo の答えを読めなければ赤" "届かない夜は注意だけで、飛ばした判定の名前を出す" "届かない夜も、本物に依らない流し直しは判定する" "3 晩続けて届かなければ赤" "手動の起動の実行は晩に数えない" "前の実行を読めなければ、今夜だけで数える"; do grep -qF "== Scenario: $s" <<<"$out" || { echo "無い: $s"; exit 1; }; done'` rc=0

## Task 2: 流し直しの答えを組み、手元の受け手から返す

- [ ] 2.1 `release_source.py` に、`fetch/` の本物の答えから spec の 6 つの場合の答えを組んで `replay/<場合>.body` に書く口と、それを loopback（`127.0.0.1` の空いた番号）で返す受け手を置く。
  `rai-256/ashiato` に数えるリリースが無い間は、`cli/cli` のリリース 1 件を土台にし、`tag_name` を書き換え、`assets` を土台の最初の配布物の写し 4 つ
  （`name` は自分の部品の配布物の名前、`browser_download_url` は取り先の版と名前の部分だけを書き換える。`digest` ほかは写しのまま）に置き換える。20 件の一覧の各リリースの本文は `tools/release/nightly-notes.md`。
  受け手は道に依らず場合の答えを返し、`503` と、`Location: https://example.invalid/` の `301` も返せる。受けた問い合わせ（方法・道・見出しの名前）を置き場に 1 行ずつ書く。
  `--self-test` に次の Scenario を足す（組んだ本文を `replay/` に置き、それと記録を `judge` に当てる）:
  「流し直しは版の名前と配布物だけを差し替え、ほかの項目を土台のまま残す」/「20 件の一覧の本文は部品の本文の上限を超える」/
  「印 nightly のリリース 1 件を読めなければ赤」/「20 件の一覧を読めなければ赤」。
  検証: `tools/release/release-source-check.sh --self-test` rc=0 /
  `bash -c 'out=$(tools/release/release-source-check.sh --self-test); for s in "流し直しは版の名前と配布物だけを差し替え、ほかの項目を土台のまま残す" "20 件の一覧の本文は部品の本文の上限を超える" "印 nightly のリリース 1 件を読めなければ赤" "20 件の一覧を読めなければ赤"; do grep -qF "== Scenario: $s" <<<"$out" || { echo "無い: $s"; exit 1; }; done'` rc=0

## Task 3: PC の収集アプリに 1 回確かめさせる入口

- [ ] 3.1 `crates/collector-windows/tests/ch20_real_source.rs` に `#[ignore]` の試験 `ch20_check_once` を 1 本置く（design D7）。環境 `ASHIATO_CH20_UPDATE_URL`・`ASHIATO_CH20_OUT` を読み
  （どちらかが無ければ理由を出して panic）、場合ごとの空の置き場に設定（`base_url`）と状態（案内を出した印と時刻）を書き、`ASHIATO_COLLECTOR_TEST_UPDATE_URL` に宛先を渡して
  `UpdateChecker::new(<置き場>).start(Utc::now())` を 1 回呼び、返った確かめた記録を `ASHIATO_CH20_OUT` に JSON 1 行で書く（記録が無ければ panic）。製品のコードに口を足さない。
  試験の上に「PR の CI では走らない（`#[ignore]`）。毎晩の検査が `--ignored --exact` で呼ぶ」と 1 行のコメントを書く。
  検証: `bash -c 'n=$(cargo test -p ashiato-collector-windows --test ch20_real_source -- --ignored --list 2>/dev/null | grep -c ": test$"); [ "$n" -eq 1 ]'` rc=0（`--list` で 1 本）/
  `scripts/quiet-run ch20-ignored -- bash -c 'out=$(cargo test -p ashiato-collector-windows --test ch20_real_source 2>&1); grep -q "0 passed; 0 failed; 1 ignored" <<<"$out"'` rc=0（既定では走らない）

## Task 4: 部品を走らせて記録を集める（`collect`）と、手順の一覧（`--plan`）

- [ ] 4.1 `release-source-check.sh collect <置き場>` を置く（design D2・D3・D6・D7）。最初に `records/mode` に `full` を書き、`GH_TOKEN`・`GITHUB_TOKEN` を unset する。
  (a) 配る形のサーバを `tools/release/build.sh server-windows` で組み、`ashiato-setup.exe install` で入れ（入れ方は `tools/release/install-check.sh:1225` の `install --profile … --downloads …` と標準入力の答え・`:311` のコンソールの包み（`conpty-run.ps1`）に合わせ、`install-check.sh` は呼ばない）、更新係のサービスが `C:\ProgramData\ashiato-updater\checks.jsonl` に書く最初の 1 行を
  180 秒まで待って `records/updater-release-real.json` に置く（来なければ `records/updater-release-real.timeout`）/ (b) 試験の組み立ての更新係を開発版の版で本物に 1 回（`real-dev`）/
  (c) 試験の組み立ての PC の収集アプリを本物に 1 回（`real`。Task 3 の入口）/ (d) Task 2 で組んだ答えを受け手から返し、試験の組み立ての更新係と PC の収集アプリに `same`・`newer`・`full-20`・`status-503`・`redirect-offhost` を、
  版の上書きで開発版にした更新係と、開発版の版で組んだ PC の収集アプリ（`collector-dev`）に `nightly-object` を通す。(a)〜(c) が部品の本物への問い合わせの全部（3 回）。
  途中の段が落ちても次の段へ進み、落ちた段は記録を置かない（`judge` が記録の無い段を赤にする）。
  `collect --replay-only <置き場>` は `records/mode` に `replay-only` を書き、(a)〜(c) を飛ばし、置き場に先に置いた `fetch/` から (d) だけを走らせる（Linux でも動く）。
  `judge` は `replay-only` の置き場では本物の 3 段を `WARN` 1 行ずつ（「本物の段を飛ばした」）にする。`--replay-fixture <置き場>` は `tools/release/release-source-check.d/` の本物の答えの写しを `fetch/` に置くだけの口（この Task で置く。change の `material/` は archive で動くので直に読まない）。
  `--plan` は部品が本物に問い合わせる段を 1 行ずつ（`updater-release real`・`updater real-dev`・`collector real`）出す。
  (a)〜(c) の実地（本物の配布元と Windows のサービス）は下流では走らせない。merge の後の最初の毎晩の実行で確かめる（`docs/changes/CH20.md` の確認バッチの問い）。
  検証: `bash -c '[ "$(tools/release/release-source-check.sh --plan | grep -c .)" -eq 3 ]'` rc=0 /
  `scripts/quiet-run ch20-replay -- bash -c 't=$(printf "\t"); d=$(mktemp -d); trap "rm -rf $d" EXIT; tools/release/release-source-check.sh --replay-fixture "$d" && tools/release/release-source-check.sh collect --replay-only "$d" || exit 1; tools/release/release-source-check.sh judge "$d" > "$d/out"; [ $? -eq 1 ] || exit 1; for l in "updater${t}nightly-object" "collector-dev${t}nightly-object" "updater${t}full-20" "collector${t}full-20" "collector${t}status-503" "collector${t}redirect-offhost"; do grep -qF "NG${t}${l}${t}" "$d/out" || { echo "無い: $l"; exit 1; }; done; [ "$(grep -cE "^NG${t}(updater|collector|collector-dev)${t}" "$d/out")" -eq 6 ] && [ "$(grep -c "本物の段を飛ばした" "$d/out")" -eq 3 ]'` rc=0
  （2026-11-19 の時点で、手元の流し直しの部品の行は 3 つの食い違い（proposal の表）の 6 行だけが赤になり、`same`・`newer` と更新係の `status-503`・`redirect-offhost` は緑になる。
  `shape` の行（1 件の道の答えの形）も赤で出るが、この検証は部品の行だけを数える。直しの change が入ったら、この検証の期待もその change が直す）

## Task 5: 本物の答えを取る（`fetch`）と、前の実行を読む（`history`）

- [ ] 5.1 `release-source-check.sh fetch <置き場>` を置く（design D4）: `GH_TOKEN` の資格情報で、`rai-256/ashiato` の一覧（`releases?per_page=20`）・印 `nightly`（`releases/tags/nightly`）・
  `cli/cli` の `releases?per_page=1` と、その印の `releases/tags/<印>` の 4 回を取り、状態コード・`Location`・本文をそのまま `fetch/` に置く。取り直さない。
  届かない（`403`・`429`・`5xx`・時間切れ・接続できない）ものは `.status` に `unreachable:<理由>` か状態コードを置く。`history <置き場>` は `gh run list --workflow release-source-nightly.yml --event schedule`
  （今の実行 `GITHUB_RUN_ID` を除いた完了した直近 2 回）と `gh run download <id> -n release-source-summary` で前の `summary.json` を `history/` に置き、読めなければ「読めない」を置く。
  `--self-test` に、PATH の先頭に置いた偽の `curl` と偽の `gh`（受けた引数を記録し、決めた答えを返す）で `fetch` と `history` を走らせる節を足す（本物に問い合わせない）:
  「検査の側が本物の答えを取るのは 1 回の実行で 4 回」（`echo "== Scenario: …"`）と、`history` が予定の起動の今の実行を除いた 2 回を読むこと・偽の `gh` が落ちると「読めない」を置くこと。
  検証: `tools/release/release-source-check.sh --self-test` rc=0 /
  `bash -c 'out=$(tools/release/release-source-check.sh --self-test); grep -qF "== Scenario: 検査の側が本物の答えを取るのは 1 回の実行で 4 回" <<<"$out" && grep -qF "== history の偽の命令の確かめ: ok" <<<"$out"'` rc=0

## Task 6: 毎晩の workflow と配線の検査

- [ ] 6.1 `.github/workflows/release-source-nightly.yml` を置く（design D1）: `schedule: cron: "0 16 * * *"` と `workflow_dispatch`、workflow の `permissions` は `contents: read`・`actions: read` だけ。
  job `check`（`windows-latest`・`timeout-minutes: 90`）の step: checkout（`persist-credentials: false`）→ Rust の toolchain と node（`nightly.yml` の `package-windows` と同じ pin）→
  `fetch`（環境に `GH_TOKEN: ${{ github.token }}`）→ `collect`（資格情報なし）→ `history`（`GH_TOKEN`。`if: always()`）→ `judge`（`if: always()`。`$GITHUB_STEP_SUMMARY` に要約を足す）→
  `upload-artifact`（`if: always()`。置き場の全部を名前 `release-source-raw`、`summary.json` を名前 `release-source-summary`、どちらも `retention-days: 90`）。
  `judge` の rc で job の成否が決まる（`judge` は `continue-on-error` を持たない）。使う action は既存の workflow と同じ commit に pin する。
  `release-source-check.sh --check-workflow [<毎晩の workflow>] [<ci.yml>] [<nightly.yml>]`（`python3` と `yaml` で読む。`yaml` が無ければ落ちる）を足し、配線の Scenario を確かめる（各 Scenario の前に `echo "== Scenario: <名前>"`）:
  「毎晩の検査は予定と手動の起動を持つ」/「毎晩の検査は読む権限だけで走る」/「毎晩の検査は公開の流れから独立する」/
  「部品が本物に問い合わせるのは 1 回の実行で 3 回まで」（`collect` を呼ぶ step が 1 つで、`--plan` の行数が 3）/「部品の問い合わせに workflow の資格情報を渡さない」/
  「判定と成果物の保存は、前の段が落ちても走る」/
  「PR ごとの CI は本物の配布元に問い合わせない」（`ci.yml` の `install-windows` が最初の `install-check.sh` より前に `update-check-install.sh --block-outbound` を持ち、どの job の `run` にも
  `release-source-check.sh collect`・`release-source-check.sh fetch`・`ch20_real_source`・`--ignored`・`--include-ignored` が無い）。
  `--self-test` に、壊した写し（(a) `permissions` に `contents: write` を足す (b) `nightly.yml` に毎晩の検査の job を足す (c) `collect` の step に `GH_TOKEN` を足す (d) `upload-artifact` の `if: always()` を消す
  (e) `ci.yml` の `--block-outbound` の step を消す (f) `ci.yml` の `rust` の job に `cargo test -- --ignored` を足す (g) `judge` の step に `continue-on-error: true` を足す
  (h) `judge` の `if: always()` を消す (i) `retention-days` を 30 にする (j) `workflow_dispatch` を消す (k) `ci.yml` の job に `release-source-check.sh fetch` を足す）で `--check-workflow` が
  0 でない終了コードで終わることを確かめる節を足す（`python3` か `yaml` が無ければ `== 配線の壊した写しの確かめを省いた（yaml が無い）` を 1 行出して続ける。CI の `chain`（ubuntu）では省かない）。
  検証: `tools/release/release-source-check.sh --check-workflow` rc=0 /
  `bash -c 'out=$(tools/release/release-source-check.sh --check-workflow); for s in "毎晩の検査は予定と手動の起動を持つ" "毎晩の検査は読む権限だけで走る" "毎晩の検査は公開の流れから独立する" "部品が本物に問い合わせるのは 1 回の実行で 3 回まで" "部品の問い合わせに workflow の資格情報を渡さない" "判定と成果物の保存は、前の段が落ちても走る" "PR ごとの CI は本物の配布元に問い合わせない"; do grep -qF "== Scenario: $s" <<<"$out" || { echo "無い: $s"; exit 1; }; done'` rc=0 /
  `bash -c 'out=$(tools/release/release-source-check.sh --self-test); ! grep -qF "== 配線の壊した写しの確かめを省いた" <<<"$out"'` rc=0
- [ ] 6.2 `.github/workflows/ci.yml` の `chain` の job の、`./tools/release/update-check-install.sh --self-test` の次に `./tools/release/release-source-check.sh --self-test` と
  `./tools/release/release-source-check.sh --check-workflow` の 2 行を足す。ほかの job と step は 1 字も変えない。
  検証: `bash -c 'grep -qF "./tools/release/release-source-check.sh --self-test" .github/workflows/ci.yml && grep -qF "./tools/release/release-source-check.sh --check-workflow" .github/workflows/ci.yml'` rc=0 /
  `bash -c 'base=$(git merge-base HEAD origin/main); [ "$(git diff "$base" -- .github/workflows/ci.yml | grep -c "^[+-][^+-]")" -eq 2 ] && ! git diff "$base" -- .github/workflows/ci.yml | grep -q "^-[^-]"'` rc=0（足した 2 行だけ）/
  `tools/release/check-guards.sh workflows` rc=0

## Task 7: 文書と、壊してはいけないもの

- [ ] 7.1 `docs/testing.md` の CI の job の表の近くに、毎晩の検査（workflow・本物に問い合わせる段が 3 つ・PR の CI では自己試験と配線の検査だけ）を、`docs/release.md` に
  赤の読み方（要約の「既に分かっている食い違い」と「それ以外の赤」・`WARN`・成果物 `release-source-raw` の中身と 90 日・基準 `release-source-shape.json` を直すのはいつか）を書く。
  検証: `bash -c 'grep -q "release-source-nightly" docs/testing.md && grep -q "release-source-shape.json" docs/release.md'` rc=0
- [ ] 7.2 いまの振る舞いが残ることを確かめる（壊してはいけないもの。`docs/changes/CH20.md`）: 製品のコードと触れない台本・workflow・共有の材料が 1 字も変わっていないこと、
  既存の偽の配布元の試験と ST57 の配線の自己試験が通ること。
  検証: `bash -c 'base=$(git merge-base HEAD origin/main); ! git diff --name-only "$base" -- crates/update-check crates/updater crates/collector-windows/src crates/collector-windows/build.rs collector-android crates/installer crates/server .github/workflows/nightly.yml .github/workflows/release.yml .github/workflows/vulns-nightly.yml tools/release/update-check-install.sh tools/release/install-check.sh tools/release/build.sh tools/release/package.sh docs/contracts/update-check-vectors.json docs/contracts/update-apply-vectors.json | grep -q .'` rc=0 /
  `tools/release/update-check-install.sh --self-test` rc=0 /
  `scripts/quiet-run st57 -- cargo test -p ashiato-updater --test st57_once` rc=0 /
  `bash -c '[ "$(cargo test -p ashiato-updater --test st57_once -- --list 2>/dev/null | grep -c ": test$")" -ge 1 ]'` rc=0 /
  `scripts/quiet-run st57pc -- cargo test -p ashiato-collector-windows --test st57_pc_update` rc=0 /
  `bash -c '[ "$(cargo test -p ashiato-collector-windows --test st57_pc_update -- --list 2>/dev/null | grep -c ": test$")" -ge 1 ]'` rc=0

## 人間の確認待ち（確認バッチ。merge の後の最初の毎晩の実行の後）

Scenario ではない —— 機械が全部の Scenario を見る。ここは本物の配布元と Windows のサービスの実地で、下流の検証は本物に問い合わせない（正典 MODIFIED）。結果は確認バッチが `premises.md` に書く。

- [ ] 8.1 **最初の毎晩の実行の要約を読む**: merge の後の最初の `release-source-nightly` の実行（待てなければ Actions の画面から手動で起動）の要約を開き、「既に分かっている食い違い」に (1) 印 `nightly` の 1 件を読めない（更新係・PC の収集アプリ・答えの形）・(2) 20 件の一覧を読めない・(3) PC の収集アプリの `503` と別の宛先への `301` が出て、「それ以外の赤」が無いかを見る。本物の状態の行で、配る形の更新係のサービスが 180 秒のうちに確かめ、正式版の道は「確かめた・新しい版なし」、開発版の道は「配布元が見つからない」と「依頼と正典の食い違い」の注意か。成果物 `release-source-raw` に本物の答えと部品の記録が残っているか。「それ以外の赤」があれば、本物との新しい食い違いか検査の誤りかを書く
  > 物理: realdata —— 本物の配布元の答えと、Windows に入れた更新係のサービスの実地は手元の受け手では作れない（試験・下流の検証は本物の配布元に聞かない）

