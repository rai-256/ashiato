# CH16 実装タスク — 判定が落ちたら観測を必ずログに出す

読む順: `proposal.md` → このファイル → `design.md` → `specs/install-check-diagnostics/spec.md` → `docs/changes/CH16.md` → `CLAUDE.md`。

軽い路（`lane.json` の `reason: same`）。実装の後に、実際の diff が予定のパスの中にあることを機械が見る。

## Global Constraints

- **触ってよいのは `lane.json` の予定のパスだけ**: `tools/release/install-check.sh`・`tools/release/update-check-install.sh`・
  `tools/release/check-ng-observations.sh`（新しく作る。`tools/release/check-*.sh` に当たる）・`docs/testing.md`・`docs/changes/CH16.md`・
  `openspec/changes/ch16-install-check-ng-observations/**`。**`.github/workflows/ci.yml` は変えない**（2 つの `--self-test` は既に `chain` の job で走る。design D5。
  lane の予定のパスには入っているので、3.1 の検証が変えていないことを見る）。
  `tools/release/` のほかの台本・製品のコード（`crates/`・`collector-android/`・`web/`）・`openspec/specs/` には触れない（触れると通常の路へ昇格する）
- **判定の条件を変えない**（design D6）: どの判定も、今 OK の入力で OK、今 NG の入力で NG のまま。`NG: <理由>` の行の文・終了コード 1・
  標準出力（`== Scenario:` の行）・`--list` の出力を変えない。観測の塊は標準エラーにだけ出す。判定の本体（`check_release_zip`・`check_test_zip` の中の判定の行）は
  書き換えない（足すのは design D7 の確かめだけ）
- **今の自己試験の確かめを消さない**: 変更前の `self_test` にある `|| die` と `expect_fail` の確かめは全部残す
- **秘密を出さない**（design D4）: 観測の塊に、設定の秘密・台本の画面の合言葉・`Authorization:` の値・`ashiato_session=` の値を出さない。
  設定はキーの名前だけ。伏せる値を集められないときは命令の記録とファイルとログの末尾を出さない
- **観測を取る命令の失敗で判定を変えない**: 観測の部品の中の命令は全部 `|| true` 相当で受け、`NG_OBSERVE_TIMEOUT` 秒（既定 30）で打ち切る（design D3）。
  2 つの台本は `set -euo pipefail` で動く。口の中で命令が落ちても `NG:` の行と終了コード 1 は変わらない
- **`set -e` の効き方**: `self_test` の本体では効く。落ちるはずの判定を `( … ) 2>"$err"` と裸で書くと、自己試験がそこで黙って rc=1 で終わる。
  落ちるはずの判定は `if ( … ) …; then die …; fi` か `( … ) … || rc=$?` で受ける。`$(…)`・`if ( … )`・`||` の左では効かないので、
  確かめは全部 `|| die '<何が外れたか>'` で書き、命令の失敗に頼らない
- **自己試験の一時ファイルは `mktemp -d "${TMPDIR:-/tmp}/…XXXXXX"` の置き場に置き、終わりに消す**（今の `self_test` と同じ）。`/tmp` の固定名に書かない
- **自己試験に `echo '== Scenario:` の印を置かない**（`install-check.sh --self-test` が落とす。印は実地の確かめにだけ）。
  この change の Scenario の印は、確かめのすぐ上に `# Scenario: <名前>` のコメントで置く（spec の `#### Scenario:` と一字一句）
- **ST58 との重なり**: 今（2026-11-17、`origin/feat/st58-update-apply-rollback` の 33c2fed45）の ST58 の枝は 2 つの台本に触れていない。
  ST58 が後から触れたら、後に merge する側が rebase で合わせる。ST58 の `update-apply-install.sh` は対象にしない（design D5）

## Task 1: 観測を出す部品と、出さない道を落とす検査

- [x] 1.1 `tools/release/check-ng-observations.sh` を作る。2 つの役を持つ（design D5）:
  - **`source` されたとき**: 観測を出す部品を定義する —— `ng_report`・`die`・`die_seen`（design D1）、観測の命令を包む関数と記録する口 `ng_observed`
    （design D2）、共通の様子（design D3。`NG_STATE_DIR`・`NG_SETTINGS_FILES`・`NG_HEALTHZ_URL`・`NG_OBSERVED_DIR` を渡されたものだけ）、伏せる関数（design D4。
    伏せる値は台本が定義する `ng_secret_values` から集め、定義が無ければ値の伏せは見出しの形だけにして記録とファイルの末尾は出さない）
  - **直に走らせたとき**: `check-ng-observations.sh <台本>…` で spec の 4 つの決まりを見て、外れた決まりの名前と `<台本>:<行の番号>` を出して rc=1。
    ヒアドキュメントの中は見ない。`# ng-observations: <理由>` の印の行は決まり 1〜3 から除く（理由が空なら外れ）
  - **`--self-test`**: `${TMPDIR:-/tmp}` の置き場で次を確かめる（偽の命令は置き場の `bin` に置き、`PATH` の先に足す。「命令が無い」環境は、要る道具
    （`bash`・`grep`・`sed`・`awk`・`tail`・`cat`・`mktemp`・`timeout`・`tr`・`date`・`rm` など）だけを `ln -s` で並べた置き場を `PATH` にして作る）:
    `# Scenario: 観測の口の外で NG の行を出す台本を、検査が落とす` /
    `# Scenario: 観測の口の外で 0 でない exit をする台本を、検査が落とす` /
    `# Scenario: 記録しない口で観測の命令を呼ぶ台本を、検査が落とす` /
    `# Scenario: 観測を出す部品を読まない・口を定義し直す台本を、検査が落とす` /
    `# Scenario: 理由の印を付けた行は除き、理由の無い印は外れにする`
    （写しは、部品を `source` する行を持ち、それぞれ 1 つの決まりにだけ外れる。外れの無い写しが rc=0 になることも見る）/
    `# Scenario: 観測の命令を記録しても、判定が見る出力は変わらない`（行末に CR を持つ標準出力と標準エラーを出して rc=3 の偽の `sc.exe` を、
    包んだ呼び出しと `command sc.exe` で受け、標準出力・標準エラー・rc をそれぞれ比べる）/
    `# Scenario: 記録する命令の出力をパイプが途中で閉じても、終了コードは命令のものになる`（`set -o pipefail` で、200 KB を出して rc=0 の偽の `sc.exe` を
    包んで `| grep -q '<最初の行>'` に流し、rc=0）/
    `# Scenario: 観測を取る命令が無い・落ちても、NG の行と終了コードは変わらず、取れなかったことを書く` /
    `# Scenario: 観測を取る命令が戻らなくても、時間の上限で打ち切って NG で終わる`（`sleep 600` する偽の `sc.exe`・`NG_OBSERVE_TIMEOUT=1`・`NG_STATE_DIR` を渡し、
    `die` が 30 秒より前に rc=1 で終わる）/
    `# Scenario: 設定に無い秘密も、見出しの形で伏せる` /
    `# Scenario: 一度伏せた値は、設定が変わった後も伏せ続ける`（記録を書いた後に設定の値を変え、前の値が標準エラーにも記録のファイルにも無い）/
    `# Scenario: 待ちのループの同じ記録は 1 件にまとまり、判定が見た出力を押し出さない`（同じ偽の出力の `sc.exe` を 30 回呼んだ後に別の出力の命令を 1 回呼んで NG。どちらの出力も塊にあり、`（同じ記録 ×` の行がある）/
    `# Scenario: 設定のファイルがまだ無くても、命令の記録は出す`（`NG_SETTINGS_FILES` の先が無い状態で NG。命令の記録の行があり、`（伏せる値を読めないので出さない）` は無い）/
    `# Scenario: 伏せる値を読めなければ、命令の記録とファイルの末尾を出さない`
  検証: `tools/release/check-ng-observations.sh --self-test` rc=0 /
  `bash -c 'd=$(mktemp -d "${TMPDIR:-/tmp}/ch16-1-1.XXXXXX"); trap "rm -rf \"$d\"" EXIT; printf "%s\n" "#!/usr/bin/env bash" ". tools/release/check-ng-observations.sh" "f() { echo \"NG: x\" >&2; }" >"$d/bad.sh"; ! tools/release/check-ng-observations.sh "$d/bad.sh" >"$d/out" 2>&1 && grep -q "bad.sh:3" "$d/out"'` rc=0（決まり 1 だけに外れる写しを行の番号つきで落とす）/
  `bash -c 'd=$(mktemp -d "${TMPDIR:-/tmp}/ch16-1-1.XXXXXX"); trap "rm -rf \"$d\"" EXIT; printf "%s\n" "#!/usr/bin/env bash" ". tools/release/check-ng-observations.sh" "f() { exit 3; }" >"$d/bad.sh"; ! tools/release/check-ng-observations.sh "$d/bad.sh" >"$d/out" 2>&1 && grep -q "bad.sh:3" "$d/out"'` rc=0（決まり 2 だけに外れる写し）

## Task 2: `install-check.sh` を観測を出す部品に寄せる

- [ ] 2.1 `tools/release/install-check.sh` の頭で `check-ng-observations.sh` を `source` し、今の `die` の定義を外す（部品の `die` を使う）。
  `ng_secret_values` を、`secret_values` と試験の画面の合言葉（`PW1` など台本が持つ値）を出す関数として置く（設定のファイルが無ければ字面の値だけで rc=0、あるのに読めなければ rc≠0。字面の合言葉はどちらでも出す。design D4）。
  `common_setup` で `NG_STATE_DIR`・`NG_SETTINGS_FILES`・`NG_HEALTHZ_URL`・`NG_OBSERVED_DIR` を渡す（design D3）。値を受けて判定する部品（`assert_acl_only`・`assert_read_only_perm`・
  `assert_modify_perm`・`assert_service_config`・`assert_restart_on_failure`・`assert_db_watch_acl`・`assert_loopback_listener_only`・`assert_no_trust`）は、
  外れたとき受けた値を `die_seen` で渡す（`assert_no_secret_in` は値を渡さない —— 秘密そのものを含む）。
  `common_teardown` は `ng_report` で出して片付けを続ける。`install_with` の時間切れは `ng_report … 9` で出して `exit 1`（印を付ける。design D1）。`console_probe` の `exit 2` に印を付ける。
  `common_setup` の末尾に design D7 の確かめを置く（`# Scenario: CI の Windows で、観測の塊を本物の命令で一度出してから実地の確かめに入る`）。
  `self_test` に次を足す（偽の命令は `PATH` の先の置き場に置く）:
  `# Scenario: 判定が NG になると、NG の行に続けて判定が見た命令の出力が出る`（ST58 の形 —— 4 行目に `        STATE              : 4  RUNNING` を持つ `sc.exe query` の偽の出力で、
  `[[ "$(state_code ashiato-server)" == 1 ]] || die …` を部分殻で走らせ、rc=1・標準エラーの 1 行目が `NG: `・見出しの 2 行の間にその行がある・標準出力に塊の行が無い）/
  `# Scenario: 判定が渡した値は、命令を走らせない判定でも観測に出る`（`assert_acl_only` に余計な主体を持つ偽の行を渡す）/
  `# Scenario: 実地の確かめの中の NG は、サービスの状態・healthz・設定のキー・命令の記録とサーバのログの末尾を出す`（`NG_*` を自己試験の置き場に向け、偽の `sc.exe`・偽の `curl`・
  設定のファイル・`trace`・サーバのログを置く）/
  `# Scenario: 観測に秘密の値が出ない`（設定のファイルの 5 つの秘密と画面の合言葉を、命令の記録・偽の命令の出力・`die_seen` の値に混ぜる。標準エラーにどの値も無く、設定のキーの名前はある）/
  `# Scenario: 2 つの台本は検査を通る`（`check-ng-observations.sh "$0"` と `check-ng-observations.sh --self-test`）
  検証: `tools/release/install-check.sh --self-test` rc=0 /
  `tools/release/check-ng-observations.sh tools/release/install-check.sh` rc=0 /
  `bash -c 'b=$(git merge-base HEAD origin/main) && diff <(bash <(git show "$b:tools/release/install-check.sh") --list) <(tools/release/install-check.sh --list)'` rc=0（`--list` が 1 行も変わらない）/
  `bash -c 'b=$(git merge-base HEAD origin/main) && git show "$b:tools/release/install-check.sh" | awk "/^self_test\\(\\)/{on=1} on" | grep -E "\\|\\| die|expect_fail " | sed "s/^[[:space:]]*//" | while IFS= read -r l; do grep -Fq -- "$l" tools/release/install-check.sh || { echo "消えた: $l"; exit 1; }; done'` rc=0（今の自己試験の確かめが残る）

## Task 3: `update-check-install.sh` を観測を出す部品に寄せる

- [ ] 3.1 `tools/release/update-check-install.sh` の頭で `check-ng-observations.sh` を `source` し、今の `die` の定義を外す。`windows_sc`・`windows_icacls`・`windows_netsh` は
  記録する口を通す（System32 の場所から呼ぶのは変えない）。`ng_secret_values` を置く（状態の置き場の設定の秘密と、字面の 2 つの画面の合言葉
  `update-check-test-password-0123`・`update-check-release-password-0123`。設定が無ければ rc=0、あるのに読めなければ rc≠0 で、合言葉はどちらでも出す）。
  実地の確かめの始めで design D3 の 4 つの名前を渡す（`NG_HEALTHZ_URL` はサーバの 18700。偽の配布元の `URL` は渡さない）。
  `assert_service`・`assert_disabled_service` は外れたとき受けた `sc qc` の出力を `die_seen` で渡す。
  `self_test` に、偽の `sc.exe`（今の自己試験の `$fake` の置き場）で `assert_service` を外し、標準エラーの `NG:` の行より後に偽の出力の行があることと、
  `check-ng-observations.sh "$0"` が通ることを足す（`# Scenario: 判定が NG になると、NG の行に続けて判定が見た命令の出力が出る`・`# Scenario: 2 つの台本は検査を通る`）。
  `docs/testing.md` の release の台本の節に、判定は `die` / `die_seen` で落とし観測は部品が出すこと・出さない道は `tools/release/check-ng-observations.sh` が落とすことを 2〜3 行で書く
  検証: `tools/release/update-check-install.sh --self-test` rc=0 /
  `tools/release/check-ng-observations.sh tools/release/install-check.sh tools/release/update-check-install.sh` rc=0 /
  `bash -c 'b=$(git merge-base HEAD origin/main) && diff <(bash <(git show "$b:tools/release/update-check-install.sh") --list) <(tools/release/update-check-install.sh --list)'` rc=0 /
  `bash -c 'b=$(git merge-base HEAD origin/main) && git show "$b:tools/release/update-check-install.sh" | awk "/^self_test\\(\\)/{on=1} on" | grep -E "\\|\\| die|expect_fail " | sed "s/^[[:space:]]*//" | while IFS= read -r l; do grep -Fq -- "$l" tools/release/update-check-install.sh || { echo "消えた: $l"; exit 1; }; done'` rc=0 /
  `bash -c 'git diff --quiet "$(git merge-base HEAD origin/main)" -- .github/workflows/ci.yml'` rc=0（`ci.yml` を変えていない）/
  `bash -c 'grep -q "check-ng-observations.sh" docs/testing.md'` rc=0
  終了の条件: PR の CI の `chain`（2 つの `--self-test`）と `install-windows`（実地の確かめと design D7 の確かめ。下流の手元では走らない）が緑
