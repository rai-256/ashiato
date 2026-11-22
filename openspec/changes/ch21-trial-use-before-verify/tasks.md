# CH21 実装タスク — 確認バッチの前に、その回で足した機能を使ってみる席（/tsukau）で確かめ、違和感を直すか人間に回す

読む順: `proposal.md` → このファイル → `design.md` → `specs/verify-environment/spec.md` → `specs/distribution/spec.md`（CH22 の Requirement に 3 つ目の名指しを足す）→ `docs/changes/CH21.md` → `premises.md` → `.harness2/scripts/tsukau.py` の冒頭（差し込み口の契約）→ `CLAUDE.md`。

## Global Constraints

- **製品のコードを変えない**: `crates/`・`collector-android/`・`web/`・`migrations/` に触れない（携帯の相手の APK は `<state>/tree` の写しで組むだけ）。触ってよいのは `tools/tsukau/`（新しい）・`tools/verify-prep.sh`（席を呼ぶ 1 か所だけ）・
  `.github/workflows/ci.yml`（`rust` の job の `./tools/check-private.sh --self-test` の行の後に 3 行）・`docs/testing.md`・`docs/changes/CH21.md`・`openspec/changes/ch21-trial-use-before-verify/**` だけ
- **harness2 を変えない**: `scripts/`・`.claude/skills/`・`.harness2/` は harness2 への symlink。席の中身・`scripts/tsukau.py`・`verify_batch.sh`・処置（`verify_tsukau.py`）は読むか呼ぶだけ。処置は依頼として置く（Task 11。design D15）
- `tools/record-env.sh`・`tools/recording/`・`tools/release/`・`tools/check-private.sh`・`tools/check-secrets.sh` は読むか呼ぶだけ。`record-env.sh` を `source` しない（design D7）
- **公開の repo に出すのは止まり方・件数・番号つきの伏せた題だけ**（深掘り Q4・Q12）: 手順書の段落に席の台本の 1 行目・`run`・`cleanup` の文・候補の本文・根拠・実行のフォルダのパスを書かない。題は必ず `tools/tsukau/public-title.sh` を通す。
  席を呼ぶ台本は `gh` を呼ばず、確認バッチの作業ツリーのファイルを直さない（処置は harness2。design D14）。実行のフォルダは `~/.local/state/ashiato2/tsukau` の下（repo の外）、目録は `dist/verify-<tag>/tsukau.json`（git が無視する）
- **準備を止めない**（依頼）: `tools/tsukau/verify-batch.sh` は、席の台本・組み立て・作業ツリーのどれが落ちても rc=0 で段落を出す。`tools/verify-prep.sh` 側は `timeout` と `||` で受けて決まった段落にする
- **Sandbox は外にはつなぎ、私設網と手元の網を塞いで確かめてから入れる**（深掘り Q8 の本人の答え・design D6）: `.wsb` に `<Networking>Disable</Networking>` を書かず、何かを入れる前に Sandbox の中のファイアウォールで塞ぎ、確かめる。確かめられなければ外も塞ぎ、それでも届けば `UNAVAILABLE=`（手で緩めない）
- **携帯のエミュレータは起動の時から網を持たない箱の中で動かし、網を切り、自分の serial だけを操作する**（深掘り Q-G2 の本人の答え・design D22（仮）・D31・D35（仮））: エミュレータと adb のサーバは `unshare` の箱（中は loopback だけ）の中で起こし、`adb` は `nsenter` で箱に入って呼ぶ。箱を作れない・箱に loopback のほかの網があればエミュレータを起こさずに `UNAVAILABLE=`。エミュレータの網の引数（`restrict=on`）に頼らない。`adb` は `devices` のほかは全部 `-s <自分の serial>`。実機と、ほかのエミュレータ（本人・計測テスト）に入れない・止めない。網を切れたと確かめられなければ `UNAVAILABLE=`
- **実地は公開に何も出さない**（design D18）: `tools/tsukau/real-run.sh` は、使い捨ての木の `origin` を一時の bare repo にし、`gh` を記録する偽物にして走らせる。本物の GitHub の repo へ push しない・issue を作らない
- **Scenario の印**: 自己試験は、確かめのすぐ上に `# Scenario: <名前>` のコメントを置く（`echo "== Scenario:` は本物で確かめる口だけ）。名前は spec の `#### Scenario:` と一字一句合わせる
- **自己試験は Linux で走る**（CI の `rust` の job）: 偽の `scripts/tsukau.py`・`wsb`・`tailscale`・`powershell.exe`（Windows 側の一時の待ち受け）・`docker`・`gh`・`adb`・`emulator`・`avdmanager`・`unshare`・`nsenter`・`ip`・`socat`・`gradlew`・`ps`・起動の台本・組み立ての台本・`git` の木は、`mktemp -d "${TMPDIR:-/tmp}/ch21-tsukau.XXXXXX"` の一時の置き場に作って終わりに消す（`/tmp` の固定名に書かない）。
  本物の Sandbox・エミュレータ・docker・claude・ネットワークを使わない。偽の命令は受けた引数を呼ばれた順に 1 行ずつ記録のファイルへ書く。**偽の席の台本は本物と同じ 2 行を出す**（1 行目「使ってみた結果: <絶対パス>/RESULT.md」、最後の行に `v`・`stop`・`count`・`titles`・`run`・`cleanup` の JSON。`run` は `/home/…` の絶対パス）と、`run` の下に `candidates.jsonl` と、`candidates`（`id`・`title`）を持つ `result.json` を書く
- **差し込み（試験のための口）は環境変数で、名前は `TSUKAU_TEST_` で始める**（偽の `tsukau.py`・`wsb`・`tailscale`・`powershell.exe`・`docker`・`gradlew`・`ps`・`java`・起動の台本・組み立ての台本・`init.py` の場所・`/dev/kvm` の代わりの置き場・打ち切りの秒数・`out_root`）。自己試験の関数の中からだけ渡し、本番の呼び出し（`verify-prep.sh`・`tsukau.json`）は渡さない。
  `TSUKAU_RECHECK` は試験の口でなく処置との契約（design D27）。偽の `adb`・`emulator`・`avdmanager` は `TSUKAU_TEST_` でなく、一時の置き場に作った偽の `$ANDROID_HOME`（`platform-tools/adb`・`emulator/emulator`・`cmdline-tools/latest/bin/avdmanager`）に置いて `ANDROID_HOME` で渡す（席の `PATH` に `adb` が無いときに、差し込み口と `phone.sh` が本番と同じ道で道具を見つけることを試験で再現する。design D32）。自己試験は、走らせる環境の `ANDROID_HOME`・`ANDROID_SDK_ROOT` を偽の置き場に向けるか消し、席に渡る `PATH` を試験が組んだ置き場（偽の命令の置き場と `/usr/bin`・`/bin`）だけにする（ホストの `platform-tools` の `adb` で D33（仮）が効き、ほかの Scenario が環境次第で落ちるのを防ぐ。review/spec.md R35）
- **席の `PATH` に `adb` を置かない**（IMP-13 の本人の答え・design D32・D33（仮））: `verify-batch.sh` は `$ANDROID_HOME` と `$ANDROID_SDK_ROOT` の下の置き場を外した `PATH` で `tsukau.py` を呼ぶ。`env-hook.sh` と `phone.sh` は `adb`・`emulator`・`avdmanager` を `$ANDROID_HOME` の下から見つける（`env-hook.sh` は自分のプロセスの `PATH` の先に足してから `tools/emulator-start.sh` を読み込む。`emulator-start.sh` は `adb` を名前で呼ぶ）
- 定数は台本の頭に置く: 席の時間 10 分・token 200,000、確かめ直し 5 分・100,000（design D8・D27）・SIGTERM までの 1,860 秒と片付けの猶予 360 秒・PC の組み立ての上限 2,400 秒（design D5（仮））・準備の全体の上限 9,360 秒（3 つの相手）
- Sandbox は起こす前に動いているものを確かめ、止めるのは自分の id だけ（design D3）。本物の Sandbox を使う確かめ（Task 8・12）は、録画と本人が Sandbox を使っていないときに走らせる。本物のエミュレータを使う確かめ（Task 10）は `/dev/kvm` が要る（開けなければ `scripts/verify-run` が BLOCKED_INFRA にする）
- Windows に渡す `.ps1` は UTF-8 の BOM 付きで書く（Windows PowerShell 5.1 は BOM が無いと日本語の注釈で構文エラーになる）。`cmd.exe` はフルパス `/mnt/c/Windows/System32/cmd.exe` で呼ぶ
- 私設網や手元の網の IPv4 の字面（範囲を含む）・私設網のホスト名の形・秘密の形は、`tools/check-private.sh`・`tools/check-secrets.sh` が commit で止める。自己試験でその形が要るときは実行時に組む（数や文字片から）

## Task 1: PC の配る zip を手元で組む

- [ ] 1.1 `tools/tsukau/build-pc.sh <置き場>` を作る（design D5（仮））。木を `C:\dev\ashiato2-tsukau-build\src` へ `rsync -a --delete`（`target`・`.git`・`node_modules`・`web/dist`・`dist/`・`.env`・`.env.*`・`collector-android/local.properties` を除く。本物の合言葉を Windows 側の写しに置かない）し、
  `C:\Program Files\Git\bin\bash.exe` で `tools/release/package.sh windows <Windows 側の置き場>` を走らせ（`CARGO_TARGET_DIR` と `PGSQL_CACHE_DIR` は `C:\dev\ashiato2-tsukau-build` の下）、
  `ashiato-server-*-windows-x86_64.zip` と `ashiato-collector-windows-*.zip` をちょうど 1 つずつ `<置き場>` へ写す。置き場を省くと `dist/tsukau-pc/<HEAD のコミット>/`。
  かかった秒数を最後に 1 行出す（`== build-pc: <秒> 秒`）。組めなければ rc≠0
  検証: `bash -c 'tools/tsukau/build-pc.sh "$TMPDIR/pc" && test "$(ls "$TMPDIR"/pc/ashiato-server-*-windows-x86_64.zip | wc -l)" -eq 1 && test "$(ls "$TMPDIR"/pc/ashiato-collector-windows-*.zip | wc -l)" -eq 1 && test ! -e /mnt/c/dev/ashiato2-tsukau-build/src/.env && test ! -e /mnt/c/dev/ashiato2-tsukau-build/src/dist'` rc=0
- [ ] 1.2 1.1 の秒数を、`target` が冷えた 1 回目と温まった 2 回目の 2 つ測り、`premises.md` の「この PC の Windows 側に、配る zip を組む台本が前提にする道具がある」の行の結果に「build-pc: 冷えた <秒> 秒・温まった <秒> 秒」と書き、確かさを 4 にする。
  温まった回が 900 秒を超えたら、design D5 の反転条件に当たったと `review/` に書いて止める（A として人間に返す）
  検証: `bash -c 'grep -Eq "build-pc: 冷えた [0-9]+ 秒・温まった [0-9]+ 秒" openspec/changes/ch21-trial-use-before-verify/premises.md'` rc=0

## Task 2: 公開に出す題の台本

- [ ] 2.1 `tools/tsukau/public-title.sh`（標準入力の題 1 行ずつ → 同じ数の行。`--self-test`）を作る（design D12・spec の公開の題の Requirement）: 利用者名（`id -un`）・ホスト名・私設網の名前（`tailscale status --json` の `MagicDNSSuffix`。取れなければ飛ばす）・
  cwd の木の `.env` の 8 字以上の値・ホームや絶対パスの形・メールの形・IPv4 の形に当たる題と、台本を持つ使い捨ての作業ツリーに題を 1 行ずつ置き、`GIT_INDEX_FILE` の一時の index で stage して
  `tools/check-private.sh --staged`・`tools/check-secrets.sh --staged` に当てて落ちた行の題を「（題は手元の結果に）」に（検査を走らせられなければ全部を伏せる）。伏せなかった題は改行を空白・80 字・`@`→「＠」・`#<数字>`→「＃<数字>」。
  `--self-test` に置く印:
  `# Scenario: 私的な形を含む題は伏せる`・
  `# Scenario: 利用者名と .env の値を含む題は伏せる`・
  `# Scenario: 検査を走らせられなければ、全部の題を伏せる`（gitleaks の置き場を無い所に向ける差し込みを `TSUKAU_TEST_` で渡す）・
  `# Scenario: 伏せた後の題は、確認バッチの commit の検査を通る`・
  `# Scenario: 題は 1 行に整え、通知の形を替える`。
  自己試験は gitleaks を飛ばさない（`tools/check-secrets.sh` が取れなければ自己試験も rc≠0。CI の `rust` の job は gitleaks を取れる。期待を緩めると素通しの実装でも緑になる。review/spec.md R1）
  検証: `tools/tsukau/public-title.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/public-title.sh)" -ge 5'` rc=0

## Task 3: 範囲と相手を決める

- [ ] 3.1 `tools/tsukau/verify-batch.sh` を作り、範囲と相手を決める部分（design D2・spec の範囲の Requirement）と `--self-test`・`--scopes`（求めた範囲を相手ごとに出すだけ。席は呼ばない）を置く。
  入力は cwd の確認バッチの作業ツリーと `<tag>`。`git log --merges --first-parent origin/main..HEAD` の件名「verify: #<PR> <head> を確認バッチ <tag> に入れる」、`git diff --name-only <merge>^1 <merge>`、
  `docs/stories/ST<NN>.md`・`docs/changes/CH<NN>.md` の front matter の `title` と「## 価値」の節の、`>` でも `（` でも始まらない最初の段落の 1 行目（120 字で切る）。
  `--scopes` は相手を `web`・`pc`・`android` の順に出し、行ごとに ID と PR のいちばん新しいコミット（merge の 2 つ目の親）を持つ（D10 の突き合わせに使う）
  `--self-test` に置く印（一時の置き場に `git init` で `origin/main` と merge を重ねた木を作り、`--scopes` の出力を見る）:
  `# Scenario: 画面のファイルだけを変えた Story は、画面の相手の範囲に入る`・
  `# Scenario: 画面の入口と公開のファイルを変えたものも、画面の相手の範囲に入る`・
  `# Scenario: サーバか PC のアプリか配る形の組み立てを変えたものは、PC の相手の範囲に入る`・
  `# Scenario: 携帯の収集アプリを変えたものは、携帯の相手の範囲に入る`・
  `# Scenario: 両方を変えたものは、両方の範囲に入る`・
  `# Scenario: 文書や道具だけ・試験だけを変えたものは、どの範囲にも入らない`・
  `# Scenario: 価値の節の注記と引用は飛ばし、無ければ表題だけを範囲に書く`・
  `# Scenario: 同じ木から 2 度求めると、同じ範囲になる`
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 8'` rc=0

## Task 4: 席を呼び、段落と目録を書く

- [ ] 4.1 `tools/tsukau/verify-batch.sh <tag>` の本体を書く（design D8〜D11・D14・D21・spec の呼び方・目録・写すもの・新しく入ったものだけの Requirement）: 相手（`web`・`pc`・`android` の順）ごとに、範囲のものを D10 の試した記録（`<out_root>/verify-index/tried.jsonl`）と相手・ID・PR のいちばん新しいコミットの組で突き合わせ、席が終えた記録のあるものを外し、残りがあれば使い捨ての作業ツリー（D9）を作って harness2 を繋ぎ、
  `timeout -k 360 1860 python3 <作業ツリー>/scripts/tsukau.py run --repo <作業ツリー> --scope <範囲> --target <相手> --minutes 10 --tokens 200000 --label verify-<tag>` で呼ぶ。
  PC の範囲があれば、呼ぶ前に `timeout 2400 tools/tsukau/build-pc.sh "$PWD/dist/verify-<tag>/tsukau-pc"` で組み、絶対パスを `ASHIATO_TSUKAU_PC_ZIPS` で渡す。画面と携帯の相手には `$PWD/dist/verify-<tag>/ashiato-server` と `$PWD/dist/verify-<tag>/web` を `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` で渡す。
  `tsukau.py` は、`PATH` から `$ANDROID_HOME` と `$ANDROID_SDK_ROOT` の下の置き場を外した環境で呼ぶ（ほかの置き場の並びは変えない。値が無いか空の変数は数えない。design D32）。席に渡す `PATH` を出す `--print-seat-path` を足す（4.3 が使う）。外した `PATH` で `command -v adb` か `command -v emulator` が見つかれば、どの相手も呼ばず、段落は「席を呼べなかった」・目録は `not_run`・理由は `tsukau.log` にだけ（design D33（仮））。
  呼ぶ前と後で、動いている Sandbox（`wsb list`）・専用の AVD `ashiato-tsukau-api35` のエミュレータ（エミュレータのプロセスの引数。`adb` でほかの端末に問わない。design D22（仮））・`ashiato-tsukau-` の compose project を数え、打ち切った後に増えていれば「片付けに残りがある」。
  席が終えたら（`done`・`time`・`tokens`）、席に渡した範囲のものの組を試した記録へ追記する。外したものは段落の「前の回で試した: <ID>（<tag>）」と目録の `earlier` に書く。
  最後の行の JSON から、頭の段落と相手ごとの段落を決まった文言（spec と design D11）で組み、番号と題は最後の行の `run` の下の `result.json` の `candidates`（`id`・`title`）から取り、題は `tools/tsukau/public-title.sh` を通して「C<n>」と並べ、標準出力に段落だけを出す。
  目録 `dist/verify-<tag>/tsukau.json`（design D14 の形。`earlier` を持ち、`copied` は持たない）と手元の詳細 `dist/verify-<tag>/tsukau.log` を書く。作業ツリーは `trap` で必ず消す。どの道でも rc=0。`gh` を呼ばない
  `--self-test` に足す印（偽の `tsukau.py`・`init.py`・組み立ての台本・`wsb`・`docker`・`ps`・`gh` と、`out_root` と打ち切りの秒数を `TSUKAU_TEST_` で渡す。偽の `adb`・`emulator` は偽の `$ANDROID_HOME` に置き、その置き場を `PATH` の先にも足して渡す）:
  `# Scenario: 範囲のある相手だけを、決まった時間と token で呼ぶ`・
  `# Scenario: 席を呼ぶ作業ツリーは確認バッチの作業ツリーと別で、harness2 が繋がり、.env が無い`・
  `# Scenario: 席が作業ツリーを書き換えても、確認バッチの作業ツリーは変わらず、使い捨ての作業ツリーは消える`・
  `# Scenario: 席を呼ぶ PATH には Android の道具の置き場が無く、adb が名前で見つからない`・
  `# Scenario: Android の道具の置き場の外にも adb があれば、どの相手の席も呼ばない`・
  `# Scenario: Android の道具の置き場の外に emulator だけがあっても、どの相手の席も呼ばない`・
  `# Scenario: ANDROID_SDK_ROOT だけが指す置き場の道具も、席の PATH から外す`・
  `# Scenario: Android の SDK が無い環境では、席の PATH を変えずに画面の席を呼ぶ`・
  `# Scenario: 画面の相手には、準備が組んだ置き場を絶対パスで渡す`・
  `# Scenario: 携帯の相手にも、準備が組んだサーバと画面の置き場を絶対パスで渡す`・
  `# Scenario: PC の相手には、組んだ配る zip の置き場を絶対パスで渡す`・
  `# Scenario: harness2 を繋げなければ、席を呼ばずに「席を呼べなかった」と書く`・
  `# Scenario: 配る zip を組めなければ、PC の席は呼ばず、画面の席は呼ぶ`・
  `# Scenario: 時間の上限で打ち切り、片付けの間は待つ`・
  `# Scenario: 打ち切った後に立てたものが残っていれば、片付けに残りがあると書く`・
  `# Scenario: 打ち切った後に専用の AVD のエミュレータが残っていれば、片付けに残りがあると書く`・
  `# Scenario: 打ち切りの秒数は、立てる上限・席の時間・片付けの和で決まる`（`--print-limits` を足す）・
  `# Scenario: 打ち切った後でも、最後の行が読めて席が終えていれば結果を写す`・
  `# Scenario: 目録は相手ごとの状態と範囲と実行のフォルダを持つ`・
  `# Scenario: 目録は git が無視する場所にあり、候補の本文を持たない`・
  `# Scenario: 席を呼ぶ台本は、作業ツリーを直さず issue も作らない`・
  `# Scenario: 席が終えた相手は、件数と番号つきの題を手順書に載せる`（番号は席の台本の `id` の `C<n>`）・
  `# Scenario: 席が終えた相手の段落に、手元のパスは出ない`・
  `# Scenario: 止まり方ごとに決まった文言だけを書く`・
  `# Scenario: 携帯の段落は「携帯: 」で始まり、画面・PC・携帯の順に並ぶ`・
  `# Scenario: 範囲が空の相手は、回さなかったと書く`・
  `# Scenario: 席の段落の題は、公開の題の台本を通したものである`・
  `# Scenario: 前の回で試したものは外し、新しく入ったものだけを範囲にする`・
  `# Scenario: 目録の earlier の run は、前の回で席を呼んだ実行のフォルダである`・
  `# Scenario: 新しく入ったものが無ければ、呼ばずに回さなかったと書く`（merge を作り直しただけの回を作る）・
  `# Scenario: main が進んだだけでは、呼び直さない`・
  `# Scenario: PR のコミットが変われば、新しく入ったものとして呼び直す`・
  `# Scenario: 前の回が落ちていたら、試していないものとして呼び直す`。
  「書く」Scenario は、それぞれ段落に `/home/`・`RESULT.md`・`.local/state`・`logs/env-down.log` が無いことを並べて見る
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 41'` rc=0
- [ ] 4.2 確かめ直しの口を書く（design D27・spec の確かめ直しの Requirement）: `TSUKAU_RECHECK` のファイルがあれば、相手ごとに項目の候補の題（`run` の下の `result.json` の `candidates` から `id`（`C<n>`）で引く）を「直した違和感が直ったか: <題>」で範囲にし、
  `--minutes 5 --tokens 100000` で呼ぶ。確かめ直しの範囲のほかは呼ばず、試した記録を読まず足さない（design D21）。目録の `recheck` に項目を書き、段落に「直した候補を確かめ直した」を足す。項目の無い相手は呼ばず「回さなかった（直した候補が無い）」・`state: skipped`。
  `--self-test` に足す印:
  `# Scenario: 確かめ直しは、直した候補の題だけを範囲に、5 分と 10 万で呼ぶ`・
  `# Scenario: 確かめ直しは、試した記録で外さず足しもせず、目録に直した候補を書く`・
  `# Scenario: 確かめ直しで項目の無い相手は、回さなかったと書く`
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 44'` rc=0

- [ ] 4.3 `tools/tsukau/verify-batch.sh real-check-seat-path` を作る（design D32・review/spec.md R34）: `--print-seat-path` の `PATH` で本物の `claude -p` を起こし（`--permission-mode` は `HARNESS_PERMISSION_MODE` か `auto`。5 分・1 回だけ）、席の Bash で `command -v adb; command -v emulator; command -v git` を走らせてその出力をそのまま最後に返すよう頼み、
  返った出力に `adb` と `emulator` の置き場が無く、`git` の置き場があることを見る。見る前に `echo "== Scenario: 本物の席の Bash で、adb と emulator が名前で見つからない"` を出す。`adb` か `emulator` が見つかれば、design D32 の反転条件に当たったと `review/` に書いて止める（A として人間に返す）。
  結果を `premises.md` の「この PC で `adb` と `emulator` が名前で見つかるのは…」の行の結果の欄に、`real-check-seat-path` の名前と日付を添えて書き足す（上流の 2026-11-22 の実測で確かさは 5。`verify-batch.sh` の組む `PATH` で確かめ直す）
  検証: `bash -c 'out="$(tools/tsukau/verify-batch.sh real-check-seat-path)" && printf "%s\n" "$out" | grep -q "== Scenario: 本物の席の Bash で、adb と emulator が名前で見つからない"'` rc=0 / `bash -c 'grep -E "^\| この PC で \`adb\` と \`emulator\`" openspec/changes/ch21-trial-use-before-verify/premises.md | grep -q "real-check-seat-path"'` rc=0

## Task 5: 確認バッチの準備から呼ぶ

- [ ] 5.1 `tools/verify-prep.sh` の、`manifest.md` を書いた後（`echo "== 出来た: $out/"` の前）に、`# ---- 使ってみる席（CH21）` と `# ---- 使ってみる席の終わり` の注釈で挟んだ部分を置く:
  `timeout -k 60 "${TSUKAU_TEST_PREP_LIMIT_S:-9360}" tools/tsukau/verify-batch.sh "$tag"` の標準出力を受け、非 0 か空なら「使ってみる席: 呼べなかった（`tools/tsukau/verify-batch.sh` が非 0 で終わったか、上限で打ち切った）」にして、空行を挟んで `manifest.md` へ `>>` で足す。
  挟んだ部分は `run.sh`・`ashiato-server`・`web/`・`app-debug.apk` を書かない。
  `tools/tsukau/verify-batch.sh --self-test` に足す印（`tools/verify-prep.sh` から注釈で挟んだ部分を `sed` で切り出し、`set -euo pipefail` の下で、偽の `tools/tsukau/verify-batch.sh` と中身のある仮の `manifest.md` で走らせる）:
  `# Scenario: 席の呼び出しが落ちても、準備の rc は 0 のまま`（偽物は rc=1・出力なし）・
  `# Scenario: 席の呼び出しが終わらなくても、上限で打ち切って準備を続ける`（偽物は終わらない・上限 2 秒）・
  `# Scenario: 準備の席の部分の上限は、席の呼び出しの全体の上限と同じ`・
  `# Scenario: 席の段落は、既存の段落の後に足すだけ`（`tools/verify-prep.sh` を読み、挟んだ部分が `cat > "$out/manifest.md"` の行より後にあり、挟んだ部分の `manifest.md` への書き込みが `>>` だけで、`run.sh`・`ashiato-server`・`app-debug.apk`・`web/` を書く行が無い）
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -n tools/verify-prep.sh` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 48'` rc=0

## Task 6: 差し込み口と画面の相手

- [ ] 6.1 `tools/tsukau/tsukau.json`（design D1 の値。`seat_hide` は Task 12.2 まで足さない）と `tools/tsukau/env-hook.sh`（`up` / `down` / `--self-test`。実行できる形）を作り、画面の相手（design D7・spec の画面の Requirement）を書く:
  `<state>/tree` に `--repo` の HEAD を detached で取り出し、その木に空いている port・`openssl rand -hex 24` の合言葉・`COMPOSE_PROJECT_NAME=ashiato-tsukau-<実行の時刻>` の `.env` と port の上書きを置き（`tools/record-env.sh` の `up` と同じ形）、`SEED=normal` で `<state>/tree/tools/stack.sh up` を前景で立て、
  画面の URL が応えたら、案内（`<state>/guide.md`）を書いて `ready` を `ready.tmp` → `mv` で置く（`GUIDE=`・`LOG_SERVER=`（サーバのログ）・`INFO_COMMIT=`）。
  `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` が渡されたらそれを使い、渡された置き場が無ければ `UNAVAILABLE=`。渡されていなければ `<state>/tree` で組む（かかった秒数を `env-up.log` に出す）。
  `down` はその project だけを `docker compose -p <project> down -v` し、`<state>/tree` を消し、残れば rc≠0。env_hook は `dist/verify-*/tsukau.json` を書かない（目録は `verify-batch.sh` だけが書く）。
  `--self-test` に置く印（偽の `docker` と、`<state>/tree/tools/stack.sh` の位置に置く偽の起動の台本を `TSUKAU_TEST_` の環境変数で渡す）:
  `# Scenario: 差し込み口の形が席の台本の契約に合う`（鍵は 4 つと、あれば `seat_hide` だけ。12.2 の前と後のどちらでも通る）・
  `# Scenario: 相手は画面と PC と携帯の 3 つである`・
  `# Scenario: 差し込み口だけで立てて片付けても、処置へ渡す目録は作らない`・
  `# Scenario: 画面の相手は、本人の .env を持たない別の木から起動する`・
  `# Scenario: 画面の相手は、専用の project と本人のものでない合言葉で立つ`・
  `# Scenario: 画面の相手を立てても、--repo は変わらない`・
  `# Scenario: 渡されたサーバと画面の置き場がそのまま起動に届く`・
  `# Scenario: 渡された置き場が無ければ、立てずに断る`・
  `# Scenario: 手で呼んだときは、置き場が渡されなければ別の木で組む`（偽の `cargo`・`npm` を `TSUKAU_TEST_` で渡す）・
  `# Scenario: 画面の案内は、URL と触らないものを書き、立てたコミットを残す`・
  `# Scenario: 片付けは、その回の project だけを消す`・
  `# Scenario: Android の SDK が無くても、画面の相手は立つ`
  検証: `tools/tsukau/env-hook.sh --self-test` rc=0 / `python3 scripts/tsukau.py check` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/env-hook.sh)" -ge 12'` rc=0

## Task 7: PC の相手

- [ ] 7.1 `tools/tsukau/env-hook.sh` に PC の相手（design D3・D4・D6・spec の PC の Requirement）と `tools/tsukau/pc.sh`（`exec` / `shot`）を書く。`up` の順:
  `wsb list` で動いている Sandbox があれば `UNAVAILABLE=` → 配る zip（`ASHIATO_TSUKAU_PC_ZIPS` か `dist/tsukau-pc/<HEAD>/`）が無ければ `UNAVAILABLE=`（理由に `tools/tsukau/build-pc.sh`）→
  `.wsb`（`<Networking>` を書かない（網をつなぐ）・共有のフォルダ 2 つ（zip は読むだけ・やり取りは書ける）・メモリ 4096 MB）で `wsb start` → id を `<state>/sandbox-id` へ →
  一時の口を立てる（WSL 側でこの PC の私設網の宛先（`tailscale ip -4`・あれば `-6`）に、Windows 側でこの PC の手元の網のアドレスに（`powershell.exe` の TcpListener）、空いている port で待ち受ける。立てられなければ `UNAVAILABLE=`（理由に口の名前）。design D26（仮））→
  塞ぐ前の確かめ（Sandbox の中から一時の口につながる。届かなければ `UNAVAILABLE=`。何も入れない）→
  塞ぐ台本（`-r System`。外向きの既定を塞ぐにし、外の宛先と既定のゲートウェイの 53 番だけを通す規則を足す。外の範囲は `env-hook.sh` が bash で数から組んで渡す（`--print-net-rules` で出せる。私設網の IPv4 と IPv6・3 つの私用・リンクローカル・IPv6 の私用・マルチキャストを含めない。字面で書かない））→
  塞いだ後の確かめ（同じ一時の口が時間切れになる。拒否と時間切れを分けて読む。`api.github.com` を引いて HTTPS で届くか）→
  私設網か手元の網に届けば、全部塞ぐ台本（外を通す規則も外す）→ 確かめ直し → それでも届けば `UNAVAILABLE=`（何も入れない）→ 一時の口を閉じる →
  サーバの zip を開いて、その場で作った乱数の合言葉でインストーラで入れる（`-r System`。`--repo` の `.env` を読まない。合言葉の渡し方は `tools/release/install-check.sh` の `install_with` と同じ）→ サーバが loopback で応える → `wsb connect --id <id>` →
  PC の収集アプリの zip を開いて `-r ExistingLogin` で起こす → 案内（`pc.sh exec`・`pc.sh shot`・UI Automation と `SendKeys` で画面を操作する例・網の様子（私設網と手元の網は塞いである・全部塞いだ回・外に届かなかった回）・ファイアウォールの規則に触らない。design D4・D6）を書いて `ready`（`GUIDE=`・`INFO_COMMIT=`・`INFO_NET=`（塞いだ・全部塞いだ・外に届かない））。
  `down`: Sandbox の中のログ（design D3）を共有のフォルダへ写す `exec` → `$TSUKAU_EVIDENCE/pc-logs/` へ写す → `wsb stop --id <自分の id>` → まだ動いていれば rc≠0。
  Sandbox の中で走らせる PowerShell は `tools/tsukau/pc/*.ps1`（BOM 付き）に置く。
  `--self-test` に足す印（偽の `wsb`・`tailscale`・`powershell.exe` を `TSUKAU_TEST_` で渡す。偽の `wsb` は `list`・`start`（受けた設定を写す）・`exec`（`-r` と走らせた台本の名前を写す。確かめる台本には、試験が決めた答え（私設網・手元の網・外に届くか）を返す）・`connect`・`stop` を記録する）:
  `# Scenario: 別の Sandbox が動いていれば、立てずに断る`・
  `# Scenario: 配る zip が無ければ、立てずに断る`・
  `# Scenario: Sandbox は網をつないで起こす`・
  `# Scenario: 何かを入れる前に、塞いで確かめる`・
  `# Scenario: 通す外の宛先は、私設網と手元の網の範囲を含まない`（`--print-net-rules` の出力に、自己試験が数から組んだ見本を当てる）・
  `# Scenario: 一時の口を立てられなければ、何も入れずに断る`・
  `# Scenario: 塞ぐ前に一時の口に届かなければ、確かめられないとして何も入れずに断る`・
  `# Scenario: 私設網か手元の網に届いたら、外向きを全部塞いで確かめ直す`・
  `# Scenario: 全部塞いでも届くなら、何も入れずに断る`・
  `# Scenario: 外にだけ届かなければ、続けて案内に書く`・
  `# Scenario: 立てるときは、塞いで確かめ、サーバを入れ、窓を開き、利用者のセッションで PC の収集アプリを起こしてから案内を置く`・
  `# Scenario: PC の相手は、本人の合言葉でなく、その場の合言葉で入れる`・
  `# Scenario: Sandbox の中のコマンドは、利用者のセッションで走り、出力が返る`（`tools/tsukau/pc.sh exec` を偽の `wsb` で）・
  `# Scenario: PC の案内は、口と網と触らないものを書く`・
  `# Scenario: 片付けは、中のログを写してから自分の Sandbox だけを止める`・
  `# Scenario: 片付けは、自分の Sandbox だけを止める`・
  `# Scenario: 止めた後も自分の Sandbox が動いていれば、片付けは非 0 で終わる`
  検証: `tools/tsukau/env-hook.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/env-hook.sh)" -ge 29'` rc=0

## Task 8: 本物の Sandbox で PC の相手を立てて片付ける

- [ ] 8.1 `tools/tsukau/env-hook.sh real-check-pc` を作る: 始めに `tools/tsukau/build-pc.sh "$TMPDIR/pc"` で一時の置き場に配る zip を組んで `ASHIATO_TSUKAU_PC_ZIPS` で渡し（`target` が温まっていても数分かかる。review/spec.md R16）、一時の `<state>` と `$TSUKAU_EVIDENCE` で `up <repo> <state> pc` を走らせ、
  `ready` に `UNAVAILABLE=` が無く `INFO_NET=` が「塞いだ」であること・塞ぐ前には届いたこの PC の私設網と手元の網の一時の口に、塞いだ後の Sandbox の中から届かないこと（`tailscale ip -6` があれば IPv6 も）・`api.github.com` が引けて HTTPS で届くこと（`pc.sh exec` で）・Sandbox の中でサーバが loopback で応えること（`pc.sh exec` で healthz）・PC の収集アプリのプロセスがあることを見て、`down` を走らせ、rc=0 と、終わった後に動いている Sandbox が無いことを見る。
  見る前に `echo "== Scenario: 本物の Sandbox で、私設網と手元の網を塞いだまま配る形が入り、片付けで残らない"` を出す。塞いだ後に名前が引けない・外の HTTPS が通らない・インストールが通らない・私設網か手元の網に届く、のどれかなら、design D6 の反転条件に当たったと `review/` に書いて止める（A として人間に返す）。
  結果を `premises.md` の「Sandbox の中のファイアウォールで、私設網と手元の網への宛先だけを塞げる」と「Windows 側の手元の網のアドレスで、許可の窓を出さずに一時の待ち受けを立てられる」の行に書き、確かさを 5 にする
  検証: `bash -c 'out="$(tools/tsukau/env-hook.sh real-check-pc)" && printf "%s\n" "$out" | grep -q "== Scenario: 本物の Sandbox で、私設網と手元の網を塞いだまま配る形が入り、片付けで残らない"'` rc=0

## Task 9: 携帯の相手

- [ ] 9.1 `tools/tsukau/env-hook.sh` に携帯の相手（design D22（仮）・D30（仮）・D31・D35（仮）・D25・D32・spec の携帯の Requirement）と `tools/tsukau/phone.sh`（`shell` / `shot` / `ui`）を書く。
  `env-hook.sh` は頭で、自分のプロセスの `PATH` の先に `$ANDROID_HOME/platform-tools`・`$ANDROID_HOME/emulator`・`$ANDROID_HOME/cmdline-tools/latest/bin` を足してから `tools/emulator-start.sh` を読み込む（受けた `PATH` には `adb` が無い。design D32）。
  `phone.sh` は `adb` を `$ANDROID_HOME/platform-tools/adb` で、`<state>` に残した箱の PID に `nsenter --preserve-credentials --user --net --target <PID>` で入って呼び（無ければ「携帯の道具が見つからない」で rc≠0。箱の PID が無い・生きていなければ `adb` を呼ばずに rc≠0 で、この PC の adb のサーバへ落とさない）、serial は `<state>` から読んで `-s` に渡し、引数でも `ANDROID_SERIAL` でも受けない。最初の引数が `shell`・`shot`・`ui` でなければ `adb` を呼ばずに rc≠0（design D25）。`env-hook.sh` が道具を探して断るのは携帯の相手（`android`）だけで、画面と PC の相手は `$ANDROID_HOME` が無くても今のまま立てる（design D32）。`up` の順:
  `tools/emulator-start.sh` を読み込み `emu_check_kvm`（開けなければ `UNAVAILABLE=`。何も起こさない）→ SDK の `emulator`・`adb`・JDK が無ければ `UNAVAILABLE=` →
  エミュレータのプロセスの引数（`ps -eo pid,args` で `-avd ashiato-tsukau-api35` を語として含むもの）があれば `UNAVAILABLE=`（ほかの端末に `adb -s` で問わない。`emu_reject_existing_avd` は使わない）→
  網を持たない箱を作る（`unshare --user --map-root-user --net` で箱を持つプロセスを起こし、PID を `<state>` へ。箱の中で loopback を上げ、`ip -o link` に loopback のほかが無いことを見る（`/sys/class/net` は箱の外の網を見せるので使わない）。作れない・ほかの網がある・`socat` が無ければ `UNAVAILABLE=`。その回の環境も APK も作らない。箱の中の段は `nsenter … bash` で起こして `tools/emulator-start.sh` を読み込み、`EMU_PID` を `<state>` に書く。design D35（仮））→
  画面の相手と同じ形で `<state>/tree` にその回の専用の環境を立てる（Task 6 の部品を使う）→ その木で `-Pashiato.baseUrl=http://127.0.0.1:18787 -Pashiato.apiToken=<その回の API_TOKEN> -Pashiato.userId=<その回の利用者 id>` で `:app:assembleDebug`（design D30（仮）。`--repo` の `.env` と `~/.gradle/gradle.properties` の値を使わない）→
  AVD が無ければ `avdmanager` で作る（`tools/android-emulator.sh` と同じ image）→
  橋を渡す（箱の外で `<state>` の Unix のソケットからその回のサーバの API の port へ、箱の中で loopback のその port からソケットへ。`socat`）→
  箱の中で（以下の `adb`・`emulator` はどれも `nsenter` で箱に入って呼ぶ）空いている偶数の port で `emu_start ashiato-tsukau-api35 -port <port> -no-window -wipe-data -no-snapshot-save -no-boot-anim`・serial と PID を `<state>` へ → `emu_wait_boot` →
  切る前の確かめ（端末の中から、ホストの loopback の別名の宛先（字面で書かず数から組む）のその回のサーバの port に届く。届かなければ `UNAVAILABLE=`）→ `adb -s <serial> shell svc wifi disable`・`svc data disable` →
  切った後の確かめ（既定のネットワークが無い（`dumpsys connectivity` の「Active default network」が `none`。`ip route` は使わない）・同じ宛先に届かない。外へは問わない。外れれば `UNAVAILABLE=`。何も入れない。design D31・D26（仮））→ `adb -s <serial> reverse tcp:18787 tcp:<その回の API の port>` → 端末の中から `http://127.0.0.1:18787` に問い、その回のサーバが応えなければ `UNAVAILABLE=`（何も入れない。design D35（仮））→
  `install` → `am start` で起こす → 案内（`phone.sh shell`・`phone.sh shot`・`phone.sh ui`・席の `PATH` に `adb` は無く端末は `phone.sh` だけで触ること・`ui` で探して `input tap` で押す例・その回の画面の URL と合言葉・サーバのログの場所・網は切ってある・触らないもの）を書いて `ready`（`GUIDE=`・`LOG_SERVER=`・`INFO_COMMIT=`）。
  `down`: `adb -s <serial> logcat -d` を `$TSUKAU_EVIDENCE/phone-logs/` へ → `adb -s <serial> emu kill`（残れば `emu_stop_own`）（箱に入れなければ箱の中の `adb` を呼ばずに `<state>` の `EMU_PID` を止める）→ 橋と箱を持つプロセスを止め、ソケットを消す → `<state>` の `EMU_PID` か `-avd ashiato-tsukau-api35` を引数に持つプロセスが残れば rc≠0 → その回の compose project を `down -v` → `<state>/tree` を消す。
  `--self-test` に足す印（偽の `gradlew`・`ps`・`java`・`docker`・`unshare`・`nsenter`・`socat` と `/dev/kvm` の代わりの置き場を `TSUKAU_TEST_` で渡し（偽の `unshare` と `nsenter` は、箱の印の環境変数を付けて残りを走らせる。箱の中の `ip`（loopback を上げる・`ip -o link`）は偽の `ip` が受けて試験が決めた答えを返し、本物の `ip` は走らせない）、偽の `adb`・`emulator`・`avdmanager` は偽の `$ANDROID_HOME` に置いて、`PATH` には置かない。偽の `adb` は受けた引数を全部記録し、`devices` と網の確かめに試験が決めた答えを返す）:
  `# Scenario: KVM を開けなければ、何も起こさずに断る`・
  `# Scenario: エミュレータか SDK か JDK が無ければ、何も起こさずに断る`・
  `# Scenario: 網を持たない箱を作れなければ、何も起こさずに断る`・
  `# Scenario: 箱に loopback のほかの網があれば、エミュレータを起こさずに断る`・
  `# Scenario: 橋を渡す道具が無ければ、何も起こさずに断る`・
  `# Scenario: 箱の外のサーバへの道は、<state> のソケットの橋だけである`・
  `# Scenario: reverse の後に端末の中の 18787 でサーバが応えなければ、APK を入れずに断る`・
  `# Scenario: エミュレータと adb は、網を持たない箱の中で動く`・
  `# Scenario: 箱が無いか止まっていれば、phone.sh は adb を呼ばずに断る`・
  `# Scenario: 専用の AVD のエミュレータが動いていれば、立てずに断る`・
  `# Scenario: 携帯の相手の APK は、<state>/tree で組む`・
  `# Scenario: 携帯の相手の APK の接続先は、端末の中のその回のサーバである`・
  `# Scenario: 携帯の相手の APK に、本人の資格情報は入らない`・
  `# Scenario: エミュレータは画面を出さず、前のデータを消して起こす`・
  `# Scenario: 携帯の相手は、自分の serial だけを名指す`・
  `# Scenario: 網を切って確かめてから、サーバへの道を渡し、APK を入れて案内を置く`・
  `# Scenario: 切った後も既定のネットワークがあれば、APK を入れずに断る`・
  `# Scenario: 切った後もホストに届けば、APK を入れずに断る`・
  `# Scenario: 切る前に届かなければ、確かめられないとして APK を入れずに断る`・
  `# Scenario: 端末の中のコマンドは、自分の serial で走り、出力が返る`・
  `# Scenario: 受けた PATH に adb が無くても、差し込み口は SDK の置き場の道具で携帯の相手を立てる`・
  `# Scenario: phone.sh は serial を引数で受けず、ほかの端末を名指さない`・
  `# Scenario: 携帯の道具が見つからなければ、phone.sh は 1 行で断る`・
  `# Scenario: 携帯の案内は、口と網と触らないものを書く`・
  `# Scenario: 片付けは、端末のログを写してから自分のエミュレータを止める`・
  `# Scenario: 携帯の片付けは、ほかのエミュレータに触らない`・
  `# Scenario: 携帯の片付けは、その回の project と木を消す`・
  `# Scenario: 携帯の片付けは、橋と箱を持つプロセスを止め、ソケットを消す`・
  `# Scenario: 止めた後も自分のエミュレータが残っていれば、片付けは非 0 で終わる`
  検証: `tools/tsukau/env-hook.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/env-hook.sh)" -ge 58'` rc=0

## Task 10: 本物のエミュレータで携帯の相手を立てて片付ける

- [ ] 10.1 `tools/tsukau/env-hook.sh real-check-android` を作る: 一時の `<state>` と `$TSUKAU_EVIDENCE` で `up <repo> <state> android` を走らせ（サーバと画面は `<state>/tree` で組む。`up` から `down` までを 3 回続けて走らせ、3 回とも下を見る。design D35（仮））、
  `ready` に `UNAVAILABLE=` が無いこと・エミュレータのプロセス（`<state>` の `EMU_PID`）の `/proc/<PID>/ns/net` のリンク先が箱を持つプロセスと同じで、その中の網が loopback だけであること（`nsenter` で箱に入って `ip -o link`）・端末に既定のネットワークが無く、切る前には届いたホストの loopback の別名の宛先に届かないこと（`phone.sh shell` で）・端末の中の `http://127.0.0.1:18787` でその回のサーバが応えること・携帯の収集アプリのプロセスがあること（`pidof`）を見て、`down` を走らせ、rc=0 と、終わった後に AVD `ashiato-tsukau-api35` のエミュレータが無いことを見る。
  続けて、同じく立てた端末で `tools/tsukau/phone.sh shot`（PNG が 1 枚置かれる）・`phone.sh ui`（携帯の収集アプリのパッケージ名を含む XML）・`phone.sh shell 'input keyevent KEYCODE_HOME'`（rc=0）を走らせてから片付ける。
  見る前にそれぞれ `echo "== Scenario: 本物のエミュレータで、網を切ったまま APK が入ってサーバに届き、片付けで残らない"` と `echo "== Scenario: 本物のエミュレータで、席の口で端末を操作できる"` を出す。箱の中で `adb reverse` か橋の道が通らない（端末の中の `http://127.0.0.1:18787` でその回のサーバが応えない）・網を切れない・900 秒の中で立たない、のどれかなら、spec の本文（箱・Unix のソケットの橋・箱の中の adb のサーバ）を変えない範囲（`socat` の書き方・待ち受けの番地・reverse の port の取り方）で直して通らなければ、design D22（仮）・D31・D35（仮）の反転条件に当たったとして、`deep.md` に `(未回答)` の A として積み、`deep-questions-r<n>.json` を置いて終える（`.harness2/skills/story/task.md` の「A が残ったら」。Q-G2 の答え: 道が無ければ携帯の相手を回さない）。
  結果を `premises.md` の「網を切ったエミュレータで、adb reverse の道だけでサーバに届く」「`svc wifi disable` と `svc data disable` で…」「携帯の相手は…900 秒の中で立つ」（かかった秒数）「画面を出さない（`-no-window`）エミュレータで…」「網を持たない箱の中で、adb reverse の道で…」の 5 行と、「この PC の WSL で、利用者と網の名前空間を分けた箱…」「箱の中の loopback の port から…」の 2 行（計 7 行）に、箱の中で確かめた結果を、確かめ方の欄に `real-check-android`（3 回）の字面を入れて書き、確かさを 5 にする
  検証: `bash -c 'out="$(tools/tsukau/env-hook.sh real-check-android)" && printf "%s\n" "$out" | grep -q "== Scenario: 本物のエミュレータで、網を切ったまま APK が入ってサーバに届き、片付けで残らない" && printf "%s\n" "$out" | grep -q "== Scenario: 本物のエミュレータで、席の口で端末を操作できる"'` rc=0 / `bash -c 'test "$(grep -E "^\| (網を切ったエミュレータで|\`svc wifi disable\`|携帯の相手は、その回の環境|画面を出さない|網を持たない箱の中で、adb reverse|この PC の WSL で、利用者と網の名前空間|箱の中の loopback の port から)" openspec/changes/ch21-trial-use-before-verify/premises.md | grep "real-check-android" | grep -c "| 5 |$")" -eq 7'` rc=0

## Task 11: CI・文書・harness2 への依頼

- [ ] 11.1 `.github/workflows/ci.yml` の `rust` の job の `- run: ./tools/check-private.sh --self-test` の行の後に `- run: ./tools/tsukau/public-title.sh --self-test`・`- run: ./tools/tsukau/verify-batch.sh --self-test`・`- run: ./tools/tsukau/env-hook.sh --self-test` を足す（design D13。CH22 が足す位置と離す）
  検証: `bash -c 'grep -A3 "tools/check-private.sh --self-test" .github/workflows/ci.yml | grep -q "tools/tsukau/public-title.sh --self-test" && grep -q "tools/tsukau/verify-batch.sh --self-test" .github/workflows/ci.yml && grep -q "tools/tsukau/env-hook.sh --self-test" .github/workflows/ci.yml'` rc=0
- [ ] 11.2 `docs/testing.md` に、使ってみる席の差し込み口（`tools/tsukau/`。何を立てるか・PC は私設網と手元の網を塞いで外にはつないだ Sandbox・携帯は網を切った専用の AVD `ashiato-tsukau-api35` のエミュレータ・確認バッチの準備から呼ぶこと・目録 `dist/verify-<tag>/tsukau.json` と処置は harness2 の確認バッチ・手で呼ぶときの `tools/tsukau/build-pc.sh` → `/tsukau`・席の `PATH` に `adb` を置かず携帯は `phone.sh` だけで触ること（design D32）・結果の置き場・公開に出るものは止まり方と件数と `public-title.sh` を通した題だけ）を 1 節で書く
  検証: `bash -c 'grep -q "tools/tsukau" docs/testing.md && grep -q "build-pc.sh" docs/testing.md && grep -q "public-title.sh" docs/testing.md && grep -q "ashiato-tsukau-api35" docs/testing.md && grep -q "phone.sh" docs/testing.md'` rc=0
- [ ] 11.3 harness2 へ 2 つの依頼を置く: 処置（design D15）の `openspec/changes/ch21-trial-use-before-verify/harness2-request.json` と、席の環境から命令を隠す差し込み口の鍵（design D34（仮））の `openspec/changes/ch21-trial-use-before-verify/harness2-request-seat-path.json`。
  合意したものを書き換えずに、それぞれ `python3 scripts/intake.py save <ファイル>` で置き、標準出力の JSON（`number`・`url`）を `harness2-request.placed.json`・`harness2-request-seat-path.placed.json` に書く。同じ題の開いた依頼が harness2 にもうあれば、新しく置かずにその番号と URL を書く
  検証: `bash -c 'for f in harness2-request harness2-request-seat-path; do python3 -c "import json,sys; d=json.load(open(sys.argv[1])); assert isinstance(d[\"number\"], int) and d[\"url\"].rstrip(\"/\").endswith(\"/issues/%d\" % d[\"number\"])" openspec/changes/ch21-trial-use-before-verify/$f.placed.json || exit 1; done'` rc=0

## Task 12: 処置まで本物で通す（harness2 の処置が入った後）

- [ ] 12.1 `tools/tsukau/real-run.sh` を作る（design D18・spec の処置の契約と受け入れの Requirement）: 処置の入口（`.harness2/scripts/verify_tsukau.py`。design D15 の 1）が無ければ「確認バッチの処置（harness2）が無い」と 1 行出して rc=127（飛ばさない。`verify-run` が BLOCKED_INFRA として止める。design D18）。
  あれば、一時の置き場に bare repo を作り、`origin/main` を押した上に、画面のファイル（`web/src/` の画面の文言 1 つを崩す）と PC のファイル（`crates/server/src/` の注釈 1 行）を変えた偽の `feat/ch99-tsukau-real` と `docs/changes/CH99.md` を置き、
  その bare repo を `origin` に持つ使い捨ての確認バッチの木と、`feat/ch99-tsukau-real` の偽の Story の作業ツリーを作って「verify: #0 feat/ch99-tsukau-real を確認バッチ ch21-real に入れる」の merge で重ね、`tools/verify-prep.sh ch21-real` の席の部分まで（画面の相手が要る `dist/verify-ch21-real/ashiato-server`・`web/` を含む）を本物で走らせる。
  席の結果に、用意した候補を 2 つ足す（崩した文言の「この回が原因」と、main の版でも起きる「前から」。根拠の文は台本が決めた字面）。呼ばれた引数と本文を記録し `pr checks` に緑を返す偽の `gh` を `PATH` の先に置き、処置の台本を push 先・`gh`・Story の作業ツリー・CI の結果を差し替えて走らせる（D15 の 7）。
  終わったら使い捨ての木・作業ツリー・bare repo を消す（記録は手元に 1 つ残す）。見る前にそれぞれ `echo "== Scenario: <名前>"` を出す:
  `== Scenario: 本物で、用意した 2 つの候補はどちらも処置の結果を持つ`・
  `== Scenario: 本物で、直した候補は Story のブランチに直しが載り、席で確かめ直される`・
  `== Scenario: 本物の処置で公開の repo に向けて出たものは、題と番号だけ`・
  `== Scenario: harness2 の処置が無ければ、実地の確かめは落ちる`（処置の入口を見えなくした一時の木で、rc=127 と 1 行を見る）。
  最後に相手ごとの止まり方・件数・処置の内訳・片付けを 1 行ずつ出す（題は出さない）。用意した候補と偽の Story 以外が原因の候補は直さない
  検証: `tools/tsukau/real-run.sh` rc=0

- [ ] 12.2 harness2 の席の `PATH` の口（design D34（仮）。`tsukau.py` の差し込み口の鍵 `seat_hide`）が入った後に、`tools/tsukau/tsukau.json` に `"seat_hide": ["adb", "emulator"]` を足す。入る前は足さない（`tsukau.py check` が知らない鍵で落とす）。
  `tools/tsukau/check-seat-hide.sh` を作る: `.harness2/scripts/tsukau.py` に `seat_hide` が無ければ「席の PATH の口（harness2）が無い」と 1 行出して rc=127（`verify-run` が BLOCKED_INFRA として止める）。
  あれば `echo "== Scenario: harness2 の鍵が入った後は、差し込み口は席から adb と emulator を隠すよう名指す"` を出し、`tsukau.json` の `seat_hide` がちょうど `["adb", "emulator"]` で `python3 scripts/tsukau.py check` が rc=0 なら rc=0。harness2 が別の形で入れたら、その形に合わせてこの項目・spec の差し込み口の Requirement・design D34（仮）を直す
  検証: `bash -c 'out="$(tools/tsukau/check-seat-hide.sh)" && printf "%s\n" "$out" | grep -q "== Scenario: harness2 の鍵が入った後は、差し込み口は席から adb と emulator を隠すよう名指す"'` rc=0

## Task 13: 全体

- [ ] 13.1 全体の確かめ（12.2 の後に走らせる。12.2 が BLOCKED_INFRA のうちは走らせない）
  検証: `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` rc=0 / `tools/tsukau/public-title.sh --self-test` rc=0 / `tools/tsukau/verify-batch.sh --self-test` rc=0 / `tools/tsukau/env-hook.sh --self-test` rc=0 / `python3 scripts/tsukau.py check` rc=0

## 人間の確認待ち

- 確認バッチの手順書の冒頭に「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」の段落と、相手ごとの段落（「画面: 」「PC: 」「携帯: 」）が載り、読めること（CH21 だけの回では「回さなかった」の 3 つ）。
  PC を変えた Story と一緒の回なら、席の間に Sandbox の窓が本人のデスクトップに出ること（深掘り Q9）。harness2 の処置が入った後の回なら、直したものの 1 行と、判断が要る候補の問いが手順書に出ること
