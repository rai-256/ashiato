# CH21 実装タスク — 確認バッチの前に、その回で足した機能だけを使ってみる席（/tsukau）で軽く確かめる

読む順: `proposal.md` → このファイル → `design.md` → `specs/verify-environment/spec.md` → `docs/changes/CH21.md` → `premises.md` → `.harness2/scripts/tsukau.py` の冒頭（差し込み口の契約）→ `CLAUDE.md`。

## Global Constraints

- **製品のコードを変えない**: `crates/`・`collector-android/`・`web/`・`migrations/` に触れない。触ってよいのは `tools/tsukau/`（新しい）・`tools/verify-prep.sh`（席を呼ぶ 1 か所だけ）・
  `.github/workflows/ci.yml`（`rust` の job の自己試験の並びに 2 行）・`docs/testing.md`・`docs/changes/CH21.md`・`openspec/changes/ch21-trial-use-before-verify/**` だけ
- **harness2 を変えない**: `scripts/`・`.claude/skills/`・`.harness2/` は harness2 への symlink。席の中身・`scripts/tsukau.py`・`verify_batch.sh` は読むだけ
- `tools/record-env.sh`・`tools/recording/`・`tools/release/` は読むだけ（`tools/release/package.sh windows` は呼ぶだけ）。`record-env.sh` を `source` しない（design D7）
- **公開の repo に出すのは件数・題・止まり方だけ**（依頼）: 手順書の段落に `detail`・候補の本文・根拠・実行のフォルダのパス・私設網の IP を書かない。issue を作らない。実行のフォルダは `~/.local/state/ashiato2/tsukau` の下（repo の外）
- **準備を止めない**（依頼）: `tools/tsukau/verify-batch.sh` は、席の台本・組み立て・作業ツリーのどれが落ちても rc=0 で段落を出す。`tools/verify-prep.sh` 側は、それでも非 0 なら `||` で決まった段落にする
- **Scenario の印**: 自己試験は、確かめのすぐ上に `# Scenario: <名前>` のコメントを置く（`echo "== Scenario:` は本物で確かめる口だけ）。名前は spec の `#### Scenario:` と一字一句合わせる
- **自己試験は Linux で走る**（CI の `rust` の job）: 偽の `scripts/tsukau.py`・`wsb`・`docker`・起動の台本・`git` の木は、`mktemp -d "${TMPDIR:-/tmp}/ch21-tsukau.XXXXXX"` の一時の置き場に作って終わりに消す（`/tmp` の固定名に書かない）。
  本物の Sandbox・docker・claude・ネットワークを使わない。偽の命令は受けた引数を呼ばれた順に 1 行ずつ記録のファイルへ書く
- **差し込み（試験のための口）は環境変数で、名前は `TSUKAU_TEST_` で始める**（偽の `tsukau.py` の場所・偽の `wsb` の場所・打ち切りの秒数・`out_root`）。自己試験の関数の中からだけ渡し、本番の呼び出し（`verify-prep.sh`・`tsukau.json`）は渡さない
- 定数は台本の頭に置く: 席の時間 10 分・token 200,000（深掘り Q5・design D8）・打ち切り 1,920 秒・PC の組み立ての上限 2,400 秒（design D5（仮））
- Sandbox は起こす前に動いているものを確かめ、止めるのは自分の id だけ（design D3）。本物の Sandbox を使う確かめ（Task 1・7・8）は、録画と本人が Sandbox を使っていないときに走らせる
- Windows に渡す `.ps1` は UTF-8 の BOM 付きで書く（Windows PowerShell 5.1 は BOM が無いと日本語の注釈で構文エラーになる）。`cmd.exe` はフルパス `/mnt/c/Windows/System32/cmd.exe` で呼ぶ

## Task 1: PC の配る zip を手元で組む

- [ ] 1.1 `tools/tsukau/build-pc.sh <置き場>` を作る（design D5（仮））。木を `C:\dev\ashiato2-tsukau-build\src` へ `rsync -a --delete`（`target`・`.git`・`node_modules`・`web/dist`・`collector-android/local.properties` を除く）し、
  `C:\Program Files\Git\bin\bash.exe` で `tools/release/package.sh windows <Windows 側の置き場>` を走らせ（`CARGO_TARGET_DIR` と `PGSQL_CACHE_DIR` は `C:\dev\ashiato2-tsukau-build` の下）、
  `ashiato-server-*-windows-x86_64.zip` と `ashiato-collector-windows-*.zip` をちょうど 1 つずつ `<置き場>` へ写す。置き場を省くと `dist/tsukau-pc/<HEAD のコミット>/`。
  かかった秒数を最後に 1 行出す（`== build-pc: <秒> 秒`）。組めなければ rc≠0
  検証: `bash -c 'tools/tsukau/build-pc.sh "$TMPDIR/pc" && test "$(ls "$TMPDIR"/pc/ashiato-server-*-windows-x86_64.zip | wc -l)" -eq 1 && test "$(ls "$TMPDIR"/pc/ashiato-collector-windows-*.zip | wc -l)" -eq 1'` rc=0
- [ ] 1.2 1.1 の秒数を、`target` が冷えた 1 回目と温まった 2 回目の 2 つ測り、`premises.md` の「この PC の Windows 側に、配る zip を組む台本が前提にする道具がある」の行の結果と確かさを書き直す（組めたら確かさ 4）。
  温まった回が 900 秒を超えたら、design D5 の反転条件に当たったと `review/` に書いて止める（A として人間に返す）
  検証: `bash -c 'grep -Eq "build-pc.*[0-9]+ 秒.*[0-9]+ 秒" openspec/changes/ch21-trial-use-before-verify/premises.md'` rc=0

## Task 2: 範囲と相手を決める

- [ ] 2.1 `tools/tsukau/verify-batch.sh` を作り、範囲と相手を決める部分（design D2）と `--self-test`・`--scopes`（求めた範囲を相手ごとに出すだけ。席は呼ばない）を置く。
  入力は cwd の確認バッチの作業ツリーと `<tag>`。`git log --merges --first-parent origin/main..HEAD` の件名「verify: #<PR> <head> を確認バッチ <tag> に入れる」、`git diff --name-only <merge>^1 <merge>`、
  `docs/stories/ST<NN>.md`・`docs/changes/CH<NN>.md` の front matter の `title` と「## 価値」の節の最初の段落の 1 行目（120 字で切る。「（」で始まれば表題だけ）
  `--self-test` に置く印（一時の置き場に `git init` で `origin/main` と merge を重ねた木を作り、`--scopes` の出力を見る）:
  `# Scenario: 画面のファイルだけを変えた Story は、画面の相手の範囲に入る`・
  `# Scenario: サーバか PC のアプリを変えたものは、PC の相手の範囲に入る`・
  `# Scenario: 両方を変えたものは、両方の範囲に入る`・
  `# Scenario: 携帯だけ・文書や道具だけを変えたものは、どの範囲にも入らない`・
  `# Scenario: 価値の節が雛形の change は、表題だけを範囲に書く`・
  `# Scenario: 同じ木から 2 度求めると、同じ範囲になる`
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 6'` rc=0

## Task 3: 席を呼び、結果を段落にする

- [ ] 3.1 `tools/tsukau/verify-batch.sh <tag>` の本体を書く（design D8〜D12）: 範囲のある相手ごとに、D10 の鍵で前の結果を探し、無ければ使い捨ての作業ツリー（D9）を作って harness2 を繋ぎ、
  `timeout 1920 python3 <作業ツリー>/scripts/tsukau.py run --repo <作業ツリー> --scope <範囲> --target <相手> --minutes 10 --tokens 200000 --label verify-<tag>` で呼ぶ。
  PC の範囲があれば、呼ぶ前に `timeout 2400 tools/tsukau/build-pc.sh dist/verify-<tag>/tsukau-pc` で組み、`ASHIATO_TSUKAU_PC_ZIPS` で渡す。画面の相手には `dist/verify-<tag>/ashiato-server` と `dist/verify-<tag>/web` を `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` で渡す。
  最後の行の JSON から段落を決まった文言（spec と design D11）で組み、題を D12 の形で伏せ、標準出力に段落だけを出す。手元の詳細（相手・範囲・実行のフォルダ・rc）は `dist/verify-<tag>/tsukau.log` へ。
  作業ツリーは `trap` で必ず消す。D10 の索引を `<out_root>/verify-index/<鍵>.json` に書く。どの道でも rc=0
  `--self-test` に足す印（偽の `tsukau.py` を `TSUKAU_TEST_TSUKAU` で、`out_root` を `TSUKAU_TEST_OUT_ROOT` で、打ち切りを `TSUKAU_TEST_TIMEOUT_S` で渡す）:
  `# Scenario: 範囲のある相手だけを、決まった時間と token で呼ぶ`・
  `# Scenario: 席は使い捨ての作業ツリーで走り、確認バッチの作業ツリーは書き換わらない`・
  `# Scenario: 時間の上限を越えた呼び出しは打ち切られる`・
  `# Scenario: 席が終えた相手は、件数と題を手順書に載せる`・
  `# Scenario: 止まり方ごとに決まった文言だけを書き、席の台本の理由は写さない`・
  `# Scenario: 範囲が空の相手は、回さなかったと 1 行書く`・
  `# Scenario: 私的な形を含む題は伏せて、件数には数える`・
  `# Scenario: 片付けに残りがあれば、その行に書く`・
  `# Scenario: 同じ中身で作り直した回は、席を呼ばずに前の結果を写す`・
  `# Scenario: main が進んだか、PR のコミットが変われば、呼び直す`・
  `# Scenario: 前の回が落ちていたら、写さずに呼び直す`。
  使い捨ての作業ツリーに harness2 を繋ぐ `init.py` は、自己試験では偽の `TSUKAU_TEST_INIT`（`.harness2` の symlink を置くだけ）にする。
  どの「書く」Scenario も、段落に `~/.local/state`・`/home/`・実行のフォルダのパスが無いことを並べて見る
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/verify-batch.sh)" -ge 17'` rc=0

## Task 4: 確認バッチの準備から呼ぶ

- [ ] 4.1 `tools/verify-prep.sh` の、`manifest.md` を書いた後（`echo "== 出来た: $out/"` の前）に、`# ---- 使ってみる席（CH21）` と `# ---- 使ってみる席の終わり` の注釈で挟んだ部分を置く:
  `tools/tsukau/verify-batch.sh "$tag"` の標準出力を受け、非 0 か空なら「使ってみる席: 呼べなかった（`tools/tsukau/verify-batch.sh` が非 0 で終わった）」にして、空行を挟んで `manifest.md` へ `>>` で足す。
  挟んだ部分は `run.sh`・`ashiato-server`・`web/`・`app-debug.apk` を書かない。
  `tools/tsukau/verify-batch.sh --self-test` に足す印（`tools/verify-prep.sh` から注釈で挟んだ部分を `sed` で切り出し、`set -euo pipefail` の下で、偽の `tools/tsukau/verify-batch.sh`（rc=1・出力なし）と仮の `manifest.md` で走らせる）:
  `# Scenario: 席の呼び出しが落ちても、準備の rc は 0 のまま`（rc=0・足した段落が「使ってみる席: 呼べなかった」で始まる・元の `manifest.md` の中身が頭にそのまま残る）・
  `# Scenario: 席の段落は、既存の段落の後に足すだけ`（`tools/verify-prep.sh` を読み、挟んだ部分が `cat > "$out/manifest.md"` の行より後にあり、挟んだ部分の `manifest.md` への書き込みが `>>` だけで、`run.sh`・`ashiato-server`・`app-debug.apk`・`web/` を書く行が無い）
  検証: `tools/tsukau/verify-batch.sh --self-test` rc=0 / `bash -n tools/verify-prep.sh` rc=0

## Task 5: 差し込み口と画面の相手

- [ ] 5.1 `tools/tsukau/tsukau.json`（design D1 の値）と `tools/tsukau/env-hook.sh`（`up` / `down` / `--self-test` / `probe-network`。実行できる形）を作り、画面の相手（design D7）を書く:
  `<state>/tree` に `--repo` の HEAD を detached で取り出し、空いている port・`openssl rand -hex 24` の合言葉・`ashiato-tsukau-<実行の時刻>` の project・`SEED=normal` を環境変数で渡して `tools/stack.sh up` を前景で立て、
  画面の healthz と画面の URL が応えたら、案内（`<state>/guide.md`）を書いて `ready` を `ready.tmp` → `mv` で置く（`GUIDE=`・`LOG_SERVER=`（サーバのログ）・`INFO_COMMIT=`）。
  `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` があれば使い、無ければ組む。案内は design D7 のとおり（触らないものに本人のサーバ・確認バッチの DB・main の checkout の `.env`）。
  `down` はその project だけを `docker compose -p <project> down -v` し、`<state>/tree` を消し、残れば rc≠0。
  `--self-test` に置く印（偽の `docker` と偽の起動の台本を `TSUKAU_TEST_` の環境変数で渡す）:
  `# Scenario: 画面の相手は、本人の .env を読まずに専用の project で立つ`・
  `# Scenario: 片付けは、その回の project だけを消す`。
  `tsukau.json` の確かめ（D13）も自己試験に置く: `# Scenario: 席の台本が差し込み口を受け付ける`（鍵が `env_hook`・`targets`・`out_root`・`ready_timeout_s` だけ・`env_hook` が実行できる・`targets` が `["web","pc"]`。`scripts/tsukau.py` があればそれの `check` も rc=0）・
  `# Scenario: 携帯を相手にした呼び出しは、立てる前に断られる`（`scripts/tsukau.py` があるときだけ。`out_root` を一時の置き場に向けた写しの差し込み口で rc=2 と、実行のフォルダが増えないこと）・
  `# Scenario: 別の作業ツリーから確かめても、置き場は同じ`（使い捨ての作業ツリーを作って、`out_root` の展開が同じこと）
  検証: `tools/tsukau/env-hook.sh --self-test` rc=0 / `python3 scripts/tsukau.py check` rc=0

## Task 6: PC の相手

- [ ] 6.1 `tools/tsukau/env-hook.sh` に PC の相手（design D3・D4・D6）と `tools/tsukau/pc.sh`（`exec` / `shot`）を書く。`up` の順:
  `wsb list` で動いている Sandbox があれば `UNAVAILABLE=` → 配る zip（`ASHIATO_TSUKAU_PC_ZIPS` か `dist/tsukau-pc/<HEAD>/`）が無ければ `UNAVAILABLE=`（理由に `tools/tsukau/build-pc.sh`）→
  `.wsb`（網あり・共有のフォルダ 2 つ（zip は読むだけ・やり取りは書ける）・メモリ 4096 MB）で `wsb start` → id を `<state>/sandbox-id` へ →
  `wsb exec -r System` で塞ぐ規則（design D6 の 7 つの範囲。CIDR の字面は `tools/check-private.sh` に止められるので、数から組む）→ 網の確かめ（私設網と手元の網の宛先に届かない・外に届くか）→ 届けば止めて網を切った `.wsb` で起こし直す →
  サーバの zip を開いてインストーラで入れる（合言葉の渡し方は `tools/release/install-check.sh` の `install_with` と同じ）→ healthz が 200 → PC の収集アプリの zip を開いて起こす →
  `wsb connect --id <id>` → 案内を書いて `ready`（`GUIDE=`・`INFO_NETWORK=`・`INFO_COMMIT=`）。
  `down`: Sandbox の中のログ（design D3 の 4 つ）を共有のフォルダへ写す `exec` → `$TSUKAU_EVIDENCE/pc-logs/` へ写す → `wsb stop --id <自分の id>` → まだ動いていれば rc≠0。
  Sandbox の中で走らせる PowerShell は `tools/tsukau/pc/*.ps1`（BOM 付き）に置く。`env-hook.sh probe-network` は、本物の Sandbox を網ありで起こし、塞いで確かめ、結果（宛先の種類ごとに届く・届かない・`outside-only` / `no-outside` / `blocked-failed`）を出して止める口（IP は出さない）。
  `--self-test` に足す印（偽の `wsb` を `TSUKAU_TEST_WSB` で渡す。偽の `wsb` は `list`・`start`・`exec`・`connect`・`stop` を記録し、網の確かめの答えを環境変数で返す）:
  `# Scenario: 別の Sandbox が動いていれば、立てずに断る`・
  `# Scenario: 配る zip が無ければ、立てずに断る`・
  `# Scenario: 立てるときは、入れて起こして窓を開いてから案内を置く`・
  `# Scenario: 片付けは、中のログを写してから自分の Sandbox だけを止める`・
  `# Scenario: 止めた後も自分の Sandbox が動いていれば、片付けは非 0 で終わる`・
  `# Scenario: 塞いだ後に私設網に届かず外に届けば、網ありで渡す`・
  `# Scenario: 塞いだ後も私設網に届けば、網を切って立て直す`
  検証: `tools/tsukau/env-hook.sh --self-test` rc=0 / `bash -c 'test "$(grep -c "^ *# Scenario: " tools/tsukau/env-hook.sh)" -ge 12'` rc=0

## Task 7: 本物の Sandbox で網を確かめる

- [ ] 7.1 この PC で `tools/tsukau/env-hook.sh probe-network` を走らせ、出力の前に `echo "== Scenario: 本物の Sandbox で、塞いだ後に私設網に届かない"` を出す。
  結果の行が `blocked-failed` なら、design D6 の「塞げなかった」の道（網を切る）が働くことを確かめ、D6 の反転条件に当たったと `review/` に書いて止める（A として人間に返す）。
  `no-outside` なら、DNS の問い合わせ先が塞がれていないかを見て、D6 の「確かめていない」のとおり塞ぐ範囲を直し、もう一度走らせる。結果を `premises.md` の「Sandbox の中のファイアウォールで、私設網と手元の網への宛先だけを塞げる」の行に書く（確かさ 5）
  検証: `bash -c 'out="$(tools/tsukau/env-hook.sh probe-network)" && printf "%s\n" "$out" | grep -q "== Scenario: 本物の Sandbox で、塞いだ後に私設網に届かない" && printf "%s\n" "$out" | grep -Eq "^result: (outside-only|no-outside)$" && ! printf "%s\n" "$out" | grep -Eq "(私設網|手元の網).*: 届く"'` rc=0

## Task 8: 本物で 1 回ずつ使ってみる・CI・文書

- [ ] 8.1 同じ差し込み口を本物で 1 回ずつ呼ぶ（design D14。確認バッチの準備と同じ大きさで）: `tools/tsukau/build-pc.sh` の後に
  `python3 scripts/tsukau.py run --scope "更新係: 入れて放っておく・更新を知らせる" --target pc --minutes 10 --tokens 200000 --label ch21-real` と
  `python3 scripts/tsukau.py run --scope "1 日の画面: 開いて 1 日の動きを見る" --target web --minutes 10 --tokens 200000 --label ch21-real`。
  どちらも最後の行の JSON の `stop` が `done`・`time`・`tokens` のどれかで、`cleanup` が「済み」であること、終わった後に動いている Sandbox と `ashiato-tsukau-` の compose project が無いことを見る。
  見つかった候補は直さない（件数と題をこの Task の記録に写すだけ）
  検証: `tools/tsukau/real-run.sh` rc=0（この 2 つを走らせて上の条件を見る台本。`tools/tsukau/` に置く。最後に相手ごとの止まり方・件数・片付けを 1 行ずつ出す。題は出さない）
- [ ] 8.2 `.github/workflows/ci.yml` の `rust` の job の自己試験の並び（`./tools/release/update-apply-install.sh --self-test` の行の近く）に `- run: ./tools/tsukau/verify-batch.sh --self-test` と `- run: ./tools/tsukau/env-hook.sh --self-test` を足す（design D13）
  検証: `bash -c 'grep -q "tools/tsukau/verify-batch.sh --self-test" .github/workflows/ci.yml && grep -q "tools/tsukau/env-hook.sh --self-test" .github/workflows/ci.yml'` rc=0
- [ ] 8.3 `docs/testing.md` に、使ってみる席の差し込み口（`tools/tsukau/`。何を立てるか・確認バッチの準備から呼ぶこと・手で呼ぶときの `tools/tsukau/build-pc.sh` → `/tsukau`・結果の置き場・公開に出るものは件数と題と止まり方だけ）を 1 節で書く
  検証: `bash -c 'grep -q "tools/tsukau" docs/testing.md && grep -q "build-pc.sh" docs/testing.md'` rc=0

## Task 9: 全体

- [ ] 9.1 全体の確かめ
  検証: `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` rc=0 / `tools/tsukau/verify-batch.sh --self-test` rc=0 / `tools/tsukau/env-hook.sh --self-test` rc=0 / `python3 scripts/tsukau.py check` rc=0

## 人間の確認待ち

- 確認バッチの手順書の冒頭に「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」の段落が載り、相手ごとの行が読めること（CH21 だけの回では「回さなかった」の 2 行）。
  PC を変えた Story と一緒の回なら、席の間に Sandbox の窓が本人のデスクトップに出ること（深掘り Q4）
