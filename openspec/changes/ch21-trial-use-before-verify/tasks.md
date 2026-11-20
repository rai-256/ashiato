# CH21 実装タスク — 確認バッチの前に、その回で足した機能だけを使ってみる席（/tsukau）で軽く確かめる

読む順: `proposal.md` → このファイル → `design.md` → `specs/verify-environment/spec.md` → `docs/changes/CH21.md` → `premises.md` → `.harness2/scripts/tsukau.py` の冒頭（差し込み口の契約）→ `CLAUDE.md`。

## Global Constraints

- **製品のコードを変えない**: `crates/`・`collector-android/`・`web/`・`migrations/` に触れない。触ってよいのは `tools/tsukau/`（新しい）・`tools/verify-prep.sh`（席を呼ぶ 1 か所だけ）・
  `.github/workflows/ci.yml`（`rust` の job の `./tools/check-private.sh --self-test` の行の後に 2 行）・`docs/testing.md`・`docs/changes/CH21.md`・`openspec/changes/ch21-trial-use-before-verify/**` だけ
- **harness2 を変えない**: `scripts/`・`.claude/skills/`・`.harness2/` は harness2 への symlink。席の中身・`scripts/tsukau.py`・`verify_batch.sh` は読むだけ
- `tools/record-env.sh`・`tools/recording/`・`tools/release/`・`tools/check-private.sh`・`tools/check-secrets.sh` は読むか呼ぶだけ。`record-env.sh` を `source` しない（design D7）
- **公開の repo に出すのは件数・題・止まり方だけ**（依頼）: 手順書の段落に席の台本の 1 行目・`run`・`cleanup` の文・候補の本文・根拠・実行のフォルダのパスを書かない。issue を作らない。実行のフォルダは `~/.local/state/ashiato2/tsukau` の下（repo の外）
- **準備を止めない**（依頼）: `tools/tsukau/verify-batch.sh` は、席の台本・組み立て・作業ツリーのどれが落ちても rc=0 で段落を出す。`tools/verify-prep.sh` 側は `timeout` と `||` で受けて決まった段落にする
- **Sandbox の網は切る**（深掘り Q2 の推奨・design D6）: `.wsb` は `<Networking>Disable</Networking>`。本人が外につなぐ案を選んだら、この change の spec が先に直る（下流で網を開けない）
- **Scenario の印**: 自己試験は、確かめのすぐ上に `# Scenario: <名前>` のコメントを置く（`echo "== Scenario:` は本物で確かめる口だけ）。名前は spec の `#### Scenario:` と一字一句合わせる
- **自己試験は Linux で走る**（CI の `rust` の job）: 偽の `scripts/tsukau.py`・`wsb`・`docker`・`gh`・起動の台本・組み立ての台本・`git` の木は、`mktemp -d "${TMPDIR:-/tmp}/ch21-tsukau.XXXXXX"` の一時の置き場に作って終わりに消す（`/tmp` の固定名に書かない）。
  本物の Sandbox・docker・claude・ネットワークを使わない。偽の命令は受けた引数を呼ばれた順に 1 行ずつ記録のファイルへ書く。**偽の席の台本は本物と同じ 2 行を出す**（1 行目「使ってみた結果: <絶対パス>/RESULT.md」、最後の行に `v`・`stop`・`count`・`titles`・`run`・`cleanup` の JSON。`run` は `/home/…` の絶対パス）
- **差し込み（試験のための口）は環境変数で、名前は `TSUKAU_TEST_` で始める**（偽の `tsukau.py`・`wsb`・`docker`・起動の台本・組み立ての台本・`init.py` の場所・打ち切りの秒数・`out_root`）。自己試験の関数の中からだけ渡し、本番の呼び出し（`verify-prep.sh`・`tsukau.json`）は渡さない
- 定数は台本の頭に置く: 席の時間 10 分・token 200,000（深掘り Q5・design D8）・SIGTERM までの 1,560 秒と片付けの猶予 360 秒・PC の組み立ての上限 2,400 秒（design D5（仮））・準備の全体の上限 6,540 秒
- Sandbox は起こす前に動いているものを確かめ、止めるのは自分の id だけ（design D3）。本物の Sandbox を使う確かめ（Task 1・7・8）は、録画と本人が Sandbox を使っていないときに走らせる
- Windows に渡す `.ps1` は UTF-8 の BOM 付きで書く（Windows PowerShell 5.1 は BOM が無いと日本語の注釈で構文エラーになる）。`cmd.exe` はフルパス `/mnt/c/Windows/System32/cmd.exe` で呼ぶ
- 私設網や手元の網の IPv4 の字面（範囲を含む）・私設網のホスト名の形・秘密の形は、`tools/check-private.sh`・`tools/check-secrets.sh` が commit で止める。自己試験でその形が要るときは実行時に組む（数や文字片から）

## Task 1: PC の配る zip を手元で組む

- [ ] 1.1 `tools/tsukau/build-pc.sh <置き場>` を作る（design D5（仮））。木を `C:\dev\ashiato2-tsukau-build\src` へ `rsync -a --delete`（`target`・`.git`・`node_modules`・`web/dist`・`collector-android/local.properties` を除く）し、
  `C:\Program Files\Git\bin\bash.exe` で `tools/release/package.sh windows <Windows 側の置き場>` を走らせ（`CARGO_TARGET_DIR` と `PGSQL_CACHE_DIR` は `C:\dev\ashiato2-tsukau-build` の下）、
  `ashiato-server-*-windows-x86_64.zip` と `ashiato-collector-windows-*.zip` をちょうど 1 つずつ `<置き場>` へ写す。置き場を省くと `dist/tsukau-pc/<HEAD のコミット>/`。
  かかった秒数を最後に 1 行出す（`== build-pc: <秒> 秒`）。組めなければ rc≠0
  検証: `bash -c 'tools/tsukau/build-pc.sh "$TMPDIR/pc" && test "$(ls "$TMPDIR"/pc/ashiato-server-*-windows-x86_64.zip | wc -l)" -eq 1 && test "$(ls "$TMPDIR"/pc/ashiato-collector-windows-*.zip | wc -l)" -eq 1'` rc=0
- [ ] 1.2 1.1 の秒数を、`target` が冷えた 1 回目と温まった 2 回目の 2 つ測り、`premises.md` の「この PC の Windows 側に、配る zip を組む台本が前提にする道具がある」の行の結果に「build-pc: 冷えた <秒> 秒・温まった <秒> 秒」と書き、確かさを 4 にする。
  温まった回が 900 秒を超えたら、design D5 の反転条件に当たったと `review/` に書いて止める（A として人間に返す）
  検証: `bash -c 'grep -Eq "build-pc: 冷えた [0-9]+ 秒・温まった [0-9]+ 秒" openspec/changes/ch21-trial-use-before-verify/premises.md'` rc=0

## Task 2: 範囲と相手を決める

- [ ] 2.1 `tools/tsukau/verify-batch.sh` を作り、範囲と相手を決める部分（design D2・spec の範囲の Requirement）と `--self-test`・`--scopes`（求めた範囲を相手ごとに出すだけ。席は呼ばない）を置く。
  入力は cwd の確認バッチの作業ツリーと `<tag>`。`git log --merges --first-parent origin/main..HEAD` の件名「verify: #<PR> <head> を確認バッチ <tag> に入れる」、`git diff --name-only <merge>^1 <merge>`、
  `docs/stories/ST<NN>.md`・`docs/changes/CH<NN>.md` の front matter の `title` と「## 価値」の節の、`>` でも `（` でも始まらない最初の段落の 1 行目（120 字で切る）
  `--self-test` に置く印（一時の置き場に `git init` で `origin/main` と merge を重ねた木を作り、`--scopes` の出力を見る）:
  `# Scenario: 画面のファイルだけを変えた Story は、画面の相手の範囲に入る`・
  `# Scenario: サーバか PC のアプリか配る形の組み立てを変えたものは、PC の相手の範囲に入る`・
  `# Scenario: 両方を変えたものは、両方の範囲に入る`・
  `# Scenario: 携帯だけ・文書や道具だけ・試験だけを変えたものは、どの範囲にも入らない`・
  `# Scenario: 価値の節の注記と引用は飛ばし、無ければ表題だけを範囲に書く`・
  `# Scenario: 同じ木から 2 度求めると、同じ範囲になる`
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 6'` rc=0

## Task 3: 席を呼び、結果を段落にする

- [ ] 3.1 `tools/tsukau/verify-batch.sh <tag>` の本体を書く（design D8〜D11・D14・spec の呼び方・写すもの・同じ中身の Requirement）: 範囲のある相手ごとに、D10 の鍵で前の結果を探し、無ければ使い捨ての作業ツリー（D9）を作って harness2 を繋ぎ、
  `timeout -k 360 1560 python3 <作業ツリー>/scripts/tsukau.py run --repo <作業ツリー> --scope <範囲> --target <相手> --minutes 10 --tokens 200000 --label verify-<tag>` で呼ぶ。
  PC の範囲があれば、呼ぶ前に `timeout 2400 tools/tsukau/build-pc.sh "$PWD/dist/verify-<tag>/tsukau-pc"` で組み、絶対パスを `ASHIATO_TSUKAU_PC_ZIPS` で渡す。画面の相手には `$PWD/dist/verify-<tag>/ashiato-server` と `$PWD/dist/verify-<tag>/web` を `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` で渡す。
  呼ぶ前と後で、動いている Sandbox（`wsb list`）と `ashiato-tsukau-` の compose project を数え、打ち切った後に増えていれば「片付けに残りがある」。
  最後の行の JSON から、頭の段落と相手ごとの段落を決まった文言（spec と design D11）で組み、標準出力に段落だけを出す。手元の詳細（相手・範囲・実行のフォルダ・rc）は `dist/verify-<tag>/tsukau.log` へ。
  作業ツリーは `trap` で必ず消す。D10 の索引を `<out_root>/verify-index/<鍵>.json` に書く。どの道でも rc=0。`gh` を呼ばない
  `--self-test` に足す印（偽の `tsukau.py`・`init.py`・組み立ての台本・`wsb`・`docker`・`gh` と、`out_root` と打ち切りの秒数を `TSUKAU_TEST_` で渡す）:
  `# Scenario: 範囲のある相手だけを、決まった時間と token で呼ぶ`・
  `# Scenario: 席を呼ぶ作業ツリーは確認バッチの作業ツリーと別で、harness2 が繋がり、.env が無い`・
  `# Scenario: 席が作業ツリーを書き換えても、確認バッチの作業ツリーは変わらず、使い捨ての作業ツリーは消える`・
  `# Scenario: 画面の相手には、準備が組んだ置き場を絶対パスで渡す`・
  `# Scenario: 配る zip を組めなければ、PC の席は呼ばず、画面の席は呼ぶ`・
  `# Scenario: 時間の上限で打ち切り、片付けの間は待つ`・
  `# Scenario: 打ち切った後に立てたものが残っていれば、片付けに残りがあると書く`・
  `# Scenario: 席が終えた相手は、件数と題を手順書に載せる`・
  `# Scenario: 止まり方ごとに決まった文言だけを書く`・
  `# Scenario: 範囲が空の相手は、回さなかったと書く`・
  `# Scenario: issue を作らない`・
  `# Scenario: 同じ中身で作り直した回は、席を呼ばずに前の結果を写す`・
  `# Scenario: main が進んだか、PR のコミットが変われば、呼び直す`・
  `# Scenario: 前の回が落ちていたら、写さずに呼び直す`。
  「書く」Scenario は、それぞれ段落に `/home/`・`RESULT.md`・`.local/state`・`logs/env-down.log` が無いことを並べて見る
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 20'` rc=0
- [ ] 3.2 題を伏せて整える部分を書く（design D12（仮）・D15（仮）・spec の題の Requirement）: 利用者名・ホスト名・私設網の名前・`.env` の 8 字以上の値・絶対パスの形に当たる題と、
  台本を持つ使い捨ての作業ツリーに題を 1 行ずつ置き、`GIT_INDEX_FILE` の一時の index で stage して `tools/check-private.sh --staged`・`tools/check-secrets.sh --staged` に当て、落ちた行の題を「（伏せた）」に（検査を走らせられなければ全部を伏せる）。伏せなかった題は改行を空白・80 字・`@`→「＠」・`#<数字>`→「＃<数字>」。
  `--self-test` に足す印:
  `# Scenario: 私的な形を含む題は伏せて、件数には数える`・
  `# Scenario: 伏せた後の段落は、確認バッチの commit の検査を通る`（`tools/check-secrets.sh` が gitleaks を見つけられない木では、飛ばしたことを 1 行出して秘密の形の題だけを外す）・
  `# Scenario: 題は 1 行に整え、通知の形を替える`
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 23'` rc=0

## Task 4: 確認バッチの準備から呼ぶ

- [ ] 4.1 `tools/verify-prep.sh` の、`manifest.md` を書いた後（`echo "== 出来た: $out/"` の前）に、`# ---- 使ってみる席（CH21）` と `# ---- 使ってみる席の終わり` の注釈で挟んだ部分を置く:
  `timeout -k 60 "${TSUKAU_TEST_PREP_LIMIT_S:-6540}" tools/tsukau/verify-batch.sh "$tag"` の標準出力を受け、非 0 か空なら「使ってみる席: 呼べなかった（`tools/tsukau/verify-batch.sh` が非 0 で終わったか、上限で打ち切った）」にして、空行を挟んで `manifest.md` へ `>>` で足す。
  挟んだ部分は `run.sh`・`ashiato-server`・`web/`・`app-debug.apk` を書かない。
  `tools/tsukau/verify-batch.sh --self-test` に足す印（`tools/verify-prep.sh` から注釈で挟んだ部分を `sed` で切り出し、`set -euo pipefail` の下で、偽の `tools/tsukau/verify-batch.sh` と中身のある仮の `manifest.md` で走らせる）:
  `# Scenario: 席の呼び出しが落ちても、準備の rc は 0 のまま`（偽物は rc=1・出力なし）・
  `# Scenario: 席の呼び出しが終わらなくても、上限で打ち切って準備を続ける`（偽物は終わらない・上限 2 秒）・
  `# Scenario: 席の段落は、既存の段落の後に足すだけ`（`tools/verify-prep.sh` を読み、挟んだ部分が `cat > "$out/manifest.md"` の行より後にあり、挟んだ部分の `manifest.md` への書き込みが `>>` だけで、`run.sh`・`ashiato-server`・`app-debug.apk`・`web/` を書く行が無い）
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -n tools/verify-prep.sh` rc=0

## Task 5: 差し込み口と画面の相手

- [ ] 5.1 `tools/tsukau/tsukau.json`（design D1 の値）と `tools/tsukau/env-hook.sh`（`up` / `down` / `--self-test`。実行できる形）を作り、画面の相手（design D7・spec の画面の Requirement）を書く:
  `<state>/tree` に `--repo` の HEAD を detached で取り出し、その木に空いている port・`openssl rand -hex 24` の合言葉・`COMPOSE_PROJECT_NAME=ashiato-tsukau-<実行の時刻>` の `.env` と port の上書きを置き（`tools/record-env.sh` の `up` と同じ形）、`SEED=normal` で `<state>/tree/tools/stack.sh up` を前景で立て、
  画面の URL が応えたら、案内（`<state>/guide.md`）を書いて `ready` を `ready.tmp` → `mv` で置く（`GUIDE=`・`LOG_SERVER=`（サーバのログ）・`INFO_COMMIT=`）。
  `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` が渡されたらそれを使い、渡された置き場が無ければ `UNAVAILABLE=`。渡されていなければ `<state>/tree` で組む（かかった秒数を `env-up.log` に出す）。
  `down` はその project だけを `docker compose -p <project> down -v` し、`<state>/tree` を消し、残れば rc≠0。
  `--self-test` に置く印（偽の `docker` と、`<state>/tree/tools/stack.sh` の位置に置く偽の起動の台本を `TSUKAU_TEST_` の環境変数で渡す）:
  `# Scenario: 差し込み口の形が席の台本の契約に合う`・
  `# Scenario: 携帯は相手に持たない`・
  `# Scenario: 画面の相手は、本人の .env を持たない別の木から起動する`・
  `# Scenario: 画面の相手は、専用の project と本人のものでない合言葉で立つ`・
  `# Scenario: 渡されたサーバと画面の置き場がそのまま起動に届き、無ければ立てずに断る`・
  `# Scenario: 画面の案内は、URL と触らないものを書き、立てたコミットを残す`・
  `# Scenario: 片付けは、その回の project だけを消す`
  検証: `tools/tsukau/env-hook.sh --self-test` rc=0 / `python3 scripts/tsukau.py check` rc=0

## Task 6: PC の相手

- [ ] 6.1 `tools/tsukau/env-hook.sh` に PC の相手（design D3・D4・D6・spec の PC の Requirement）と `tools/tsukau/pc.sh`（`exec` / `shot`）を書く。`up` の順:
  `wsb list` で動いている Sandbox があれば `UNAVAILABLE=` → 配る zip（`ASHIATO_TSUKAU_PC_ZIPS` か `dist/tsukau-pc/<HEAD>/`）が無ければ `UNAVAILABLE=`（理由に `tools/tsukau/build-pc.sh`）→
  `.wsb`（`<Networking>Disable</Networking>`・共有のフォルダ 2 つ（zip は読むだけ・やり取りは書ける）・メモリ 4096 MB）で `wsb start` → id を `<state>/sandbox-id` へ →
  サーバの zip を開いてインストーラで入れる（`-r System`。合言葉の渡し方は `tools/release/install-check.sh` の `install_with` と同じ）→ サーバが loopback で応える → `wsb connect --id <id>` →
  PC の収集アプリの zip を開いて `-r ExistingLogin` で起こす → 案内を書いて `ready`（`GUIDE=`・`INFO_COMMIT=`）。
  `down`: Sandbox の中のログ（design D3）を共有のフォルダへ写す `exec` → `$TSUKAU_EVIDENCE/pc-logs/` へ写す → `wsb stop --id <自分の id>` → まだ動いていれば rc≠0。
  Sandbox の中で走らせる PowerShell は `tools/tsukau/pc/*.ps1`（BOM 付き）に置く。
  `--self-test` に足す印（偽の `wsb` を `TSUKAU_TEST_WSB` で渡す。偽の `wsb` は `list`・`start`（受けた設定を写す）・`exec`（`-r` と走らせた台本の名前を写す）・`connect`・`stop` を記録する）:
  `# Scenario: 別の Sandbox が動いていれば、立てずに断る`・
  `# Scenario: 配る zip が無ければ、立てずに断る`・
  `# Scenario: Sandbox は網を切って起こす`・
  `# Scenario: 立てるときは、サーバを入れ、窓を開き、利用者のセッションで PC の収集アプリを起こしてから案内を置く`・
  `# Scenario: PC の案内は、口と網と触らないものを書く`・
  `# Scenario: 片付けは、中のログを写してから自分の Sandbox だけを止める`・
  `# Scenario: 止めた後も自分の Sandbox が動いていれば、片付けは非 0 で終わる`
  検証: `tools/tsukau/env-hook.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/env-hook.sh)" -ge 14'` rc=0

## Task 7: 本物の Sandbox で PC の相手を立てて片付ける

- [ ] 7.1 `tools/tsukau/env-hook.sh real-check-pc` を作る: Task 1 で組んだ配る zip を置いて、一時の `<state>` と `$TSUKAU_EVIDENCE` で `up <repo> <state> pc` を走らせ、
  `ready` に `UNAVAILABLE=` が無いこと・Sandbox の中でサーバが loopback で応えること（`pc.sh exec` で healthz）・PC の収集アプリのプロセスがあることを見て、`down` を走らせ、rc=0 と、終わった後に動いている Sandbox が無いことを見る。
  見る前に `echo "== Scenario: 本物の Sandbox で、網を切ったまま配る形が入り、片付けで残らない"` を出す。網を切った Sandbox でインストールか loopback が通らなければ、design D6 の「確かめていない」に当たったと `review/` に書いて止める（A として人間に返す）。
  結果を `premises.md` に 1 行足す（網を切った Sandbox でインストールと loopback が通るか。確かさ 5）
  検証: `bash -c 'out="$(tools/tsukau/env-hook.sh real-check-pc)" && printf "%s\n" "$out" | grep -q "== Scenario: 本物の Sandbox で、網を切ったまま配る形が入り、片付けで残らない"'` rc=0

## Task 8: 同じ道を本物で 1 回通す・CI・文書

- [ ] 8.1 `tools/tsukau/real-run.sh` を作り、走らせる（design D14（仮））: 一時の置き場に、確認バッチの作業ツリーの写し（`git worktree add --detach … origin/main`）を作り、画面のファイル（`web/src/` の注釈 1 行）と PC のファイル（`crates/server/src/` の注釈 1 行）を変えた
  偽の `feat/ch99-tsukau-real` を「verify: #0 feat/ch99-tsukau-real を確認バッチ ch21-real に入れる」の merge で重ね、`tools/verify-prep.sh` の成果物のうち画面の相手が要るもの（`dist/verify-ch21-real/ashiato-server`・`web/`）を組み、
  `tools/tsukau/verify-batch.sh ch21-real` を本物で走らせる。終わったら写しの作業ツリーを消す。
  終わりの条件: 画面と PC の段落が「席が自分で終えた」「時間を使い切った」「token を使い切った」のどれかで始まり、「片付けに残りがある」が無く、`dist/verify-ch21-real/tsukau.log` が指す 2 つの実行のフォルダの `result.json` の `journal` が 1 以上。
  最後に相手ごとの止まり方・件数・片付けを 1 行ずつ出す（題は出さない）。見つかった候補は直さない
  検証: `tools/tsukau/real-run.sh` rc=0
- [ ] 8.2 `.github/workflows/ci.yml` の `rust` の job の `- run: ./tools/check-private.sh --self-test` の行の後に `- run: ./tools/tsukau/verify-batch.sh --self-test` と `- run: ./tools/tsukau/env-hook.sh --self-test` を足す（design D13。CH22 が足す位置と離す）
  検証: `bash -c 'grep -A2 "tools/check-private.sh --self-test" .github/workflows/ci.yml | grep -q "tools/tsukau/verify-batch.sh --self-test" && grep -q "tools/tsukau/env-hook.sh --self-test" .github/workflows/ci.yml'` rc=0
- [ ] 8.3 `docs/testing.md` に、使ってみる席の差し込み口（`tools/tsukau/`。何を立てるか・PC は網を切った Sandbox・確認バッチの準備から呼ぶこと・手で呼ぶときの `tools/tsukau/build-pc.sh` → `/tsukau`・結果の置き場・公開に出るものは件数と題と止まり方だけ）を 1 節で書く
  検証: `bash -c 'grep -q "tools/tsukau" docs/testing.md && grep -q "build-pc.sh" docs/testing.md'` rc=0

## Task 9: 全体

- [ ] 9.1 全体の確かめ
  検証: `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` rc=0 / `tools/tsukau/verify-batch.sh --self-test` rc=0 / `tools/tsukau/env-hook.sh --self-test` rc=0 / `python3 scripts/tsukau.py check` rc=0

## 人間の確認待ち

- 確認バッチの手順書の冒頭に「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」の段落と、相手ごとの段落（「画面: 」「PC: 」）が載り、読めること（CH21 だけの回では「回さなかった」の 2 つ）。
  PC を変えた Story と一緒の回なら、席の間に Sandbox の窓が本人のデスクトップに出ること（深掘り Q4）
