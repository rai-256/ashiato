# CH21 実装タスク — 確認バッチの前に Claude が配る形を入れて普通に使ってみる

読む順: `deep.md`（**本人の答え Q1〜Q13**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/trial-use/spec.md` → `design.md`（D1〜D15）→
`premises.md` → `docs/changes/CH21.md` → `tools/verify-prep.sh`・`tools/release/package.sh`・`tools/release/build.sh`・`tools/release/sandbox-probe.sh`・`tools/record-env.sh` の `archive_prepare`・
`.harness2/scripts/record-run` の `sandbox_pids` / `wsb_xml` / `windows_record`（写す元。**harness2 は変えない**）・`.harness2/scripts/iwakan_widget.py` の `memo()`（違和感メモの形）→ `CLAUDE.md`。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** bash は `echo "Scenario: …"`（その場合を確かめる直前に出す）。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。
  印の名前は spec の `#### Scenario:` と**一字一句合わせる**。印は、その Scenario が言う道筋（席の入口か本体を走らせる）を本当に通る試験にだけ付ける
- **この change の Scenario は 39 本**（`trial-use` の ADDED 9 つ: 4・3・5・4・4・7・5・3・4 本）。38 本は `tools/trial-use/check-trial-use.sh --self-test` に置き、
  1 本（「本物の確認バッチで、手順書の冒頭に使ってみた段落が載る」）は下の「人間の確認待ち」に置く。**38 本とも、このファイルのどこかの Task が名指しする**
- **自己検査は Windows も Claude も GitHub も要らない形にする**（design D14）。偽の `powershell.exe`・Sandbox のプロセスの問い・`wsb`・組み立て・`claude`・時間の包み・`gh` を一時の場所の `PATH` の先頭に置くか、
  `TRIAL_USE_*` の差し替えの変数で渡す。一時の場所は `mktemp -d`（`$TMPDIR` の下）で、`trap` で消す。`/tmp` の固定名に書かない。手元の置き場は自己検査では `TRIAL_USE_HOME` で一時の場所にする
- **harness2（`.harness2/` の下・symlink の先）を変えない**（Q12）。録画の Sandbox の扱いが要るなら ashiato2 の側に写す
- **issue を作らない・GitHub に書き込まない**（Q8。A）。`gh` は `issue list`（読むだけ）だけを使う
- **候補の本文・根拠・画像・ログの中身・Claude の自由な文（題を除く）を、リポジトリの追跡するファイルと manifest に書かない**（Q7。A）。置き場は `TRIAL_USE_HOME`（既定 `/mnt/c/dev/ashiato2-trial-use`）
- **Claude の資格情報を Sandbox に持ち込まない。Sandbox の時計を動かさない**（Q3・Q5）
- **`tools/verify-prep.sh` に足すのは席を呼ぶ 1 行だけ**（design D5）。既存の段の順・成果物（サーバ・画面・APK・`run.sh`）・manifest の既存の段落を変えない
- **変えないもの**: `crates/`・`migrations/`・`web/`・`collector-android/`・`tools/release/`（呼ぶだけ）・`tools/recording/`・`tools/record-env.sh`・`tools/stack.sh`・ラベル「違和感」とその説明（Task 10.1 の検証が見る）
- **本人の決定（下流は変えない）**: Q1〜Q13（第 1 回は推奨で先に書いた。本人の答えが違えば上流がここを書き直してから下流へ渡す）。
  Q7「手元に置き、手順書には件数・題 1 行・場所だけ」と Q8「人間が選ぶ。issue は作らない」は A
- **本物の Windows・Sandbox・Claude の利用枠が要る検証は Task 1 と Task 11 だけ**（作業箱の外、手元の機械で走る）。ほかは CI と同じく偽もので走る
- 走っている Story・change のブランチには触らない（proposal の Impact）

## Task 1: 確かめていない前提を実測する（倒し先を決める）

`premises.md` の確かさ 1〜2 の 3 行を、本物の手元の Windows と Sandbox で確かめ、結果で design の D2・D3 の倒し先を決める。**この Task の結果で後の Task の作り方が変わる。**
**手元の Windows と Sandbox が要る**（作業箱の中では走らない）。`tools/workbox.json` の `host_only` に `tools/trial-use/(probe|seat)\\.sh` を足すのもこの Task。

- [ ] 1.1 `tools/workbox.json` の `host_only` に `"tools/trial-use/(probe|seat)\\.sh"` を足す。`tools/trial-use/probe.sh <build|install|drive>`（新しいファイル）を書く。どれも結果を `$TRIAL_USE_HOME/probe/<名前>.txt` に
  `PASS` / `FAIL: <理由>` / `UNAVAILABLE: <理由>` の 1 行で書き、`PASS` か `FAIL` を書けたら rc=0、確かめられなかったら（Windows が無い・Sandbox を起こせない）rc=1（`verify-run` では BLOCKED_INFRA として止まる）。
  `sandbox-probe.sh` の作り方を写す。`install` は `build` の zip を使う（無ければ先に `build` を走らせる）。
  `build`: この木を `git clone --local --no-hardlinks` で `C:\dev\ashiato2-trial-build\src\` に写し、Windows の Git の bash で `tools/release/package.sh windows` を走らせ、2 つの zip ができるか（design D2）。
  `install`: 組んだサーバの zip を Sandbox の中で展開してインストーラを起こし、サービスが立って `http://127.0.0.1:18700/healthz` が応えるか（昇格の確認が出たかも書く）。
  `drive`: Sandbox の中の Edge を遠隔のデバッグの口つきで起こして網の口へ転送し、外（WSL）からその口に繋いで中の画面の題を読めるか（design D3）。終わりにどれも Sandbox を閉じる。
  検証: `bash -c 'grep -q "tools/trial-use/(probe|seat)" tools/workbox.json && python3 -m json.tool tools/workbox.json >/dev/null'` rc=0 / `tools/trial-use/probe.sh build` rc=0 / `tools/trial-use/probe.sh install` rc=0 / `tools/trial-use/probe.sh drive` rc=0
- [ ] 1.2 結果を `premises.md` に写す（確かめた 3 行の確かさを 5 に・結果を `PASS` / `FAIL: <理由>` に）。`FAIL` の行があれば design の倒し先に倒す:
  `build` が FAIL → D2 の CI の artifact の案（倒したことを D2 に追記）。`drive` が FAIL → D3 の第 2 案（中に Node と Playwright）。`install` が FAIL → 後の Task はそのまま作り、
  席はインストールまで届かなかったことを警告に書く（spec）。倒したら `python3 scripts/provisional.py add` で台帳に載せる（`--follows Q2` / `--follows Q3`）。
  検証: `bash -c 'f=openspec/changes/ch21-trial-use-before-verify/premises.md; [ "$(grep -E "^\| (手元の Windows に Git|配る zip のインストーラ|手元の側（WSL）の Claude)" "$f" | grep -c "| 5 |$")" -eq 3 ]'` rc=0

## Task 2: 席の入口と準備との境目

- [ ] 2.1 `tools/trial-use/seat.sh <tag> <manifest のパス>`（新しいファイル）を書く（design D5）: どう終わっても rc=0。Windows の PowerShell と Sandbox の実行ファイルの有無を先に見て、無ければ本体を呼ばずに理由の段落を書く。
  本体（既定 `tools/trial-use/run.sh --platform windows`、自己検査は `TRIAL_USE_BODY` で差し替える）を呼び、本体が書いた段落（`$TRIAL_USE_HOME/<tag>/paragraph.md`）があればそれを、
  無ければ「本体が落ちた（rc=<n>）」の警告を、「使ってみた」で始まる 1 段落として manifest の末尾に `>>` で足す。
  `tools/verify-prep.sh` の manifest の heredoc の後・`== 出来た` の前に `./tools/trial-use/seat.sh "$tag" "$out/manifest.md" || echo "warn: 使ってみる席の入口が落ちた"` を 1 行足す。
  `tools/trial-use/check-trial-use.sh`（新しいファイル。`set -euo pipefail`、使い方の注記、`--self-test` の入口。失敗はどの場合かを出して最後に rc=1、全部通れば `OK` の 1 行で rc=0）に 3 本:
  `echo "Scenario: 席の本体が落ちても、入口は rc=0 で既存の段落を変えず警告の段落を足す"`（既存の形の manifest を置いた一時の木で `seat.sh` を呼び、呼ぶ前の manifest が呼んだ後の先頭と `cmp -n` で同じ）/
  `echo "Scenario: Windows Sandbox が無い機械では、席を走らせず理由を書く"`（PowerShell が無い・Sandbox の実行ファイルが無いの 2 通り）/
  `echo "Scenario: 準備の台本は、席を既存の段の後に 1 か所でだけ呼び、その失敗で止まらない"`（入口の行が 1 つ・manifest の heredoc の終わりより後・同じ行に `||`）。
  検証: `tools/trial-use/check-trial-use.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/trial-use/check-trial-use.sh)" -ge 3 ]'` rc=0 / `bash -c '[ "$(grep -c "tools/trial-use/seat.sh" tools/verify-prep.sh)" -eq 1 ]'` rc=0

## Task 3: 配る形を組む

- [ ] 3.1 `tools/trial-use/run.sh` に、錠（design D11 の `flock -n`。この Task では錠を取るだけ）と組み立ての段を書く（design D2・D10。Task 1 で倒したなら倒した先）: この木の HEAD を写して組み、サーバの zip と PC の収集アプリの zip の 2 つだけを
  材料の置き場（`$TRIAL_USE_HOME/<tag>/in/`）へ写す。組み立ての命令は `TRIAL_USE_PACKAGE`、上限は `TRIAL_USE_BUILD_LIMIT`（既定 3600 秒）で差し替えられる。
  組めない・上限を越えたら Sandbox を起こさずに段落を書いて終える。`--self-test` に 3 本:
  `echo "Scenario: 席が Sandbox に渡すのは、この木から組んだ 2 つの zip だけ"` / `echo "Scenario: 配る形を組めなければ、Sandbox を起こさずに警告を書く"` / `echo "Scenario: 組み立てが上限を越えれば、組めなかったとして終える"`。
  検証: `tools/trial-use/check-trial-use.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/trial-use/check-trial-use.sh)" -ge 6 ]'` rc=0

## Task 4: Sandbox を起こし、必ず閉じる

- [ ] 4.1 `tools/trial-use/run.sh` に Sandbox の段を書く（design D11）: 錠が取れなければ警告、動いている Sandbox を問い（あれば起こさず閉じず警告）、`.wsb`（網は既定＝開ける・`C:\trial\in` と `C:\trial\bin` は読みだけ・`C:\trial\out` は書ける・
  LogonCommand は `C:\trial\bin\runner.ps1`）を書いて起こし、増えたプロセスを控え、`trap` で正常・失敗・打ち切りのどれでも閉じ、閉じた後に問い直して残っていれば段落に書く。
  閉じる前に `runner.ps1` にログ（サーバ・同梱の DB・更新係・インストーラ）を `C:\trial\out\logs\` に写させ、`healthz` の応えを書かせる。
  `tools/trial-use/runner.ps1`（中で動く。頼みの箱 `C:\trial\out\box\` を順に実行し、結果と写しを書く。Edge を遠隔のデバッグの口つきで起こして網の口へ転送する —— Task 1 の結果の形で）を書く。`--self-test` に 4 本:
  `echo "Scenario: 別の席が走っていれば、Sandbox を起こさず警告を書く"` / `echo "Scenario: Sandbox が既に動いていれば、起こさず閉じず警告を書く"` / `echo "Scenario: 閉じても残った Sandbox は、段落に書く"` /
  `echo "Scenario: 席の Sandbox の設定は網を開けている"`。
  検証: `tools/trial-use/check-trial-use.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/trial-use/check-trial-use.sh)" -ge 10 ]'` rc=0

## Task 5: 材料・頼みの道具・報告の下地

- [ ] 5.1 `tools/trial-use/goals.md`（目的の一覧。札 `install` `password` `login` `import` `browse` `settings` `update-check` `uninstall` `reinstall` `pc-collector` を 1 つずつ持つ目的を利用者の言葉で。手順・順・部品の名前を書かない。design D6）・
  `tools/trial-use/prompt.md`（指示。候補の基準と 4 つの根拠・もう 1 回確かめる・Sandbox に固有の違い・好みの話は挙げない・時計を変えない・巡った札を書く）・
  `tools/trial-use/make-export.py`（合成の Takeout の形の zip。席を走らせた日の前の 7 日分の YouTube の視聴と検索。`archive_prepare` の形と同じ）・
  `tools/trial-use/box.sh`（外の Claude が使う「頼みを書いて結果を待つ」道具。時計を変える命令を含む頼みは届けずに非 0）を書く。`--self-test` に 2 本:
  `echo "Scenario: 目的の一覧は、依頼の対象の札を全部持ち、手順を書かない"` / `echo "Scenario: Sandbox の時計を変える頼みは断る"`。
  検証: `tools/trial-use/check-trial-use.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/trial-use/check-trial-use.sh)" -ge 12 ]'` rc=0 /
  `bash -c 'python3 tools/trial-use/make-export.py "$TMPDIR/e.zip" && python3 -c "import zipfile,sys; n=zipfile.ZipFile(sys.argv[1]).namelist(); sys.exit(0 if any(\"YouTube\" in x for x in n) else 1)" "$TMPDIR/e.zip"'` rc=0
- [ ] 5.2 `tools/trial-use/report.py`（新しいファイル。stdlib のみ。design D7・D12・D13）を書く: `candidates/*.json` を 4 つの根拠で確かめ（欠けたものは「根拠が欠けたもの」へ）、印（「1 回だけ」「既知: #<番号>」「Sandbox だから」「（Sandbox で測った）」）、
  題を 1 行 60 文字に切り `tools/check-private.sh` の型を伏せる、並べて手順書には上位 10 件。手元の `report.html`（全件・元の題・根拠・写し・違和感メモとコピーのボタン・根拠が欠けたもの・巡った札・取り出せなかったログ・照合を飛ばしたか）と、
  段落 `paragraph.md`（「使ってみた」で始まり、件数・題・巡った札・所要・終わり方の決まった語・報告の場所だけ）を書く。`report.py --self-test`（固定の候補の置き場に当てる単体。Scenario の印は付けない —— 印は 7.1 で席を通して付ける）。
  検証: `python3 tools/trial-use/report.py --self-test` rc=0

## Task 6: Claude を起こし、時間の上限で止める

- [ ] 6.1 `tools/trial-use/run.sh` に Claude の段を書く（design D4・D6・D10）: 材料の置き場（`in/` には目的・2 つの zip・合成の書き出しだけ。`runner.ps1` は `bin/`）を置き、リポジトリの外の一時の作業場所に材料を写し、
  `claude -p` を限った道具で起こす（`TRIAL_USE_CLAUDE` で差し替え）。`timeout --kill-after=30s "${TRIAL_USE_LIMIT:-2700}"`（包みは `TRIAL_USE_TIMEOUT` で差し替え）で止め、それまでの候補と写しで `report.py` を呼ぶ。
  Claude が動けなければ警告にして起こし直さない。使い終えた後に `runner.ps1` の `healthz` の結果を読み、届いていなければ段落に書く。操作の記録を `transcript.jsonl` に残す。`--self-test` に 7 本:
  `echo "Scenario: 席が起こした Sandbox は、どう終わっても閉じる"` / `echo "Scenario: 時間の上限を越えると、それまでの候補で報告を作って閉じる"` / `echo "Scenario: 上限を指定しなければ、Claude に許す時間は 2700 秒である"` /
  `echo "Scenario: Claude が動けなければ警告を書き、起こし直さない"` / `echo "Scenario: インストールまで届かなければ、候補とは別の警告として書く"` /
  `echo "Scenario: 材料の置き場と Claude の作業場所には、目的と zip と合成の書き出しのほかに何も無い"` / `echo "Scenario: Claude はリポジトリの外で、限った道具だけで起こされる"`。
  検証: `tools/trial-use/check-trial-use.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/trial-use/check-trial-use.sh)" -ge 19 ]'` rc=0

## Task 7: 候補と報告と段落を、席を通して確かめる

- [ ] 7.1 6.1 までの席を偽の Claude（決まった候補と写しを書く）で走らせ、`report.py` が席の中で正しく働くことを `--self-test` に 12 本で確かめる（足りない所は `run.sh` か `report.py` を直す）:
  `echo "Scenario: 4 つの根拠のどれが欠けても、候補は数えず報告の別の節に残す"` / `echo "Scenario: もう 1 回で起きなかった候補には「1 回だけ」の印が付く"` / `echo "Scenario: 「1 回だけ」でない候補が先に並ぶ"` /
  `echo "Scenario: Sandbox に固有の違いと待ち時間には印が付く"` / `echo "Scenario: 題は 1 行 60 文字までで、私的な値の形は伏せる"` / `echo "Scenario: 候補が 10 件を越えても、手順書には 10 件、手元には全件"` /
  `echo "Scenario: 手順書の段落には件数と題と札と場所だけが載る"`（目印の文字列が段落にも `git status --porcelain` の追跡するファイルの差分にも無い）/ `echo "Scenario: 見たものとログは、手元の置き場に残る"` /
  `echo "Scenario: 取り出せなかったログは、報告に書く"` / `echo "Scenario: 警告で終わった回も、そこまでの写しで報告を作る"` / `echo "Scenario: 候補が 0 件でも、巡った目的を書く"` /
  `echo "Scenario: 各候補に、そのまま貼れる違和感メモが付く"`。
  検証: `tools/trial-use/check-trial-use.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/trial-use/check-trial-use.sh)" -ge 31 ]'` rc=0

## Task 8: 既知の照合と、GitHub に書き込まないこと

- [ ] 8.1 `tools/trial-use/run.sh` に既知の照合の段を書く（design D8）: 使い終えた後に `gh issue list --label 違和感 --state open --json number,title,body`（読むだけ）で一覧を取り、
  候補と一覧だけを渡した 2 回目の `claude -p`（道具なし・JSON の出力。`TRIAL_USE_MATCH` で差し替え）で同じものの番号を受け取り、候補に `known` を書き足してから `report.py` を呼ぶ。読めなければ飛ばし、報告に書く。`--self-test` に 3 本:
  `echo "Scenario: 既知と照合した候補は数えず、印を付けて手元に残す"` / `echo "Scenario: 席が打つ GitHub の命令は、一覧を読むことだけ"`（偽の `gh` が書き留めた命令がすべて `issue list` で始まる）/
  `echo "Scenario: 一覧を読めなければ、照合を飛ばして報告に書く"`。
  検証: `tools/trial-use/check-trial-use.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/trial-use/check-trial-use.sh)" -ge 34 ]'` rc=0

## Task 9: 配る形の出所が同じなら走らない

- [ ] 9.1 `tools/trial-use/sources.txt`（出所の一覧。design D9）を置き、`tools/trial-use/run.sh` の最初（錠の後・組み立ての前）に出所の比べを書く: 一覧の `git rev-parse HEAD:<path>` を並べた sha256 を `$TRIAL_USE_HOME/last.json` と比べ、
  同じで前の回が走れていれば、Sandbox を起こさずに前の回の段落を「前の回（<前の tag>）の結果」として写す。走ったら `last.json` を書き直す（警告で終わった回も書き、`outcome` で区別する）。tag が `probe-` で始まる回は読みも書きもしない。
  `--self-test` に、一覧が `tools/release/build.sh`・`package.sh` の読むパス（`$root/<path>`・`git show HEAD:<path>`）を全部含むことの突き合わせ（印なし）と、4 本:
  `echo "Scenario: 出所が同じなら、走らずに前の回の結果を写す"` / `echo "Scenario: 出所が変われば走る"`（一時の repo で commit を 1 つ足す）/ `echo "Scenario: 前の回が走れていなければ、同じ出所でも走る"` /
  `echo "Scenario: 手で試す回は、前の回の記録に触らない"`。
  検証: `tools/trial-use/check-trial-use.sh --self-test` rc=0 / `bash -c '[ "$(grep -c "echo \"Scenario: " tools/trial-use/check-trial-use.sh)" -ge 38 ]'` rc=0 / `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` rc=0

## Task 10: CI・文書・ADR・変えないもの

- [ ] 10.1 `.github/workflows/ci.yml` の `chain` job（出し分けが無く、md だけの変更でも走る）に `- run: ./tools/trial-use/check-trial-use.sh --self-test` を 1 行足す。
  `docs/testing.md` の §7（人間の確認）に「確認バッチの準備は、配る形を Sandbox に入れて Claude が使ってみる席を走らせる（`tools/trial-use/`。CH21）」の 1 段落を足す。
  `docs/adr/` に ADR を 1 つ書き（形と一覧への行は `docs/adr/README.md`。design D15）、`tools/trial-use/README.md` に置き場・入口・環境変数（`TRIAL_USE_HOME` `TRIAL_USE_LIMIT` `TRIAL_USE_BUILD_LIMIT`）・手で 1 回走らせる方法（`probe-` の tag）を書く。
  検証: `bash -c 'sed -n "/^  chain:/,/^  [a-z-]*:$/p" .github/workflows/ci.yml | grep -q "tools/trial-use/check-trial-use.sh --self-test" && grep -q "tools/trial-use/" docs/testing.md && grep -rlq "使ってみる" docs/adr/ && [ -f tools/trial-use/README.md ]'` rc=0 / `tools/check-private.sh` rc=0 /
  `bash -c '! git diff --name-only "$(git merge-base origin/main HEAD)" HEAD | grep -E "^(crates|migrations|web|collector-android|tools/release|tools/recording)/|^tools/(record-env|stack)\.sh$"'` rc=0（harness2 は symlink の先なので、この diff には出ない。`.harness2` の先の repo に変更を作らない）

## Task 11: 本物の Sandbox と Claude で 1 回通す

- [ ] 11.1 **手元の Windows・Sandbox・Claude の利用枠が要る**（作業箱の外）。手元の機械で、席を本物のまま 1 回走らせる（確認バッチの外。tag は `probe-<YYYYMMDD>` なので前の回の記録に触らない）。
  段落が「使ってみた」で始まり、手元の置き場に `report.html` と写しが 1 枚以上あり、インストールまで届いたこと（`logs/` の `healthz` の結果）を見る。
  11-19 の違和感（更新の確認がエラーを出し続ける）が候補に出たかは記録するが、合否に使わない（Q13）。結果（件数・巡った札・所要・終わり方）をこの Task の ledger に書く。
  検証: `bash -c 't="probe-$(date +%Y%m%d)"; m="$TMPDIR/manifest.md"; d="/mnt/c/dev/ashiato2-trial-use/$t"; : >"$m"; tools/trial-use/seat.sh "$t" "$m" && grep -q "^使ってみた" "$m" && [ -f "$d/report.html" ] && [ -n "$(ls -A "$d/shots")" ] && ! grep -q "インストールまで届かなかった" "$m"'` rc=0

## 人間の確認待ち

**書式は `- Scenario: <名前>` の裸の形**（`check_scenarios.py` / `verify_checklist.py` / `verify_record.py` がこの形しか読まない）。

- Scenario: 本物の確認バッチで、手順書の冒頭に使ってみた段落が載る
  > 物理: device —— 本物の Windows Sandbox と Claude の利用枠で、確認バッチ（`scripts/hx verify`）の中で走る。CI の runner には Sandbox も Claude も無い（11.1 は確認バッチの外での 1 回）。
  > 手順書の冒頭の段落に、件数と題（0 件なら巡った目的の札）と手元の報告の場所か、走らせなかった・落ちた理由があること、報告の HTML を開けることを見る
