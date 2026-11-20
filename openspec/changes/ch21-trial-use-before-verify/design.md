# CH21 design — 確認バッチの前に、その回で足した機能だけを使ってみる席（/tsukau）で軽く確かめる

## Context

- 席の台本 `scripts/tsukau.py run`（harness2）は、差し込み口 `tools/tsukau/tsukau.json` を読み、`env_hook up` → `ready` を待つ（`ready_timeout_s`）→ `claude -p "/tsukau seat <実行のフォルダ>"`（cwd は `--repo`）
  → 時間・token の上限で止める → `LOG_<名前>` を写す → `env_hook down` → `result.json`・`RESULT.md`・`public.json` を書く。rc は 0（done/time/tokens）・2（引数・設定の誤り。実行のフォルダを作らない）・3（crashed/env_failed/interrupted）。
  標準出力は 2 行: 1 行目「使ってみた結果: <実行のフォルダの絶対パス>/RESULT.md」、最後の行 `{"v","stop","count","titles","run","cleanup"}`（`.harness2/scripts/tsukau.py:671-672`）。`detail` は標準出力に出ない。
  SIGTERM を受けると `interrupted` にして、片付けと記録を済ませてから終わる（`:647-673`）。env_hook には `TSUKAU_RUN`・`TSUKAU_EVIDENCE`・`TSUKAU_TARGET`・`TSUKAU_SCOPE` と呼び手の環境変数がそのまま渡り、cwd は `--repo`（`:447-458`）
- 確認バッチ（harness2 `scripts/verify_batch.sh`）は、`<repo>-verify` の作業ツリーで `origin/main` から `verify/<tag>` を作り、PR ごとに「verify: #<PR> <head> を確認バッチ <tag> に入れる」merge を重ね（`:117-160`）、
  `tools/verify-prep.sh <tag>` を呼ぶ（`:186-196`。落ちたら manifest なしで続ける）。その後に録画、手順書（`verify_checklist.py` が manifest を空行で段落に分け、段落ごとに 1 つの note にする）、
  `docs/verify` の commit（pre-commit の `check-private`・`check-secrets` を通る）と push、PR 本文（manifest をそのまま載せる。`:241`）。作業ツリーに未コミットの変更があると次の確認バッチが止まる（`:113`）
- `tools/verify-prep.sh` は `set -euo pipefail`。main の checkout の `.env`（本物の合言葉）を作業ツリーへ写す（`:24-33`）
- `tools/stack.sh` は cwd の木の根に `.env` があれば読み、渡した環境変数を上書きする（`tools/stack.sh:28`）
- 録画の専用環境（`tools/record-env.sh`）は、専用の compose project・空いている port・乱数の合言葉で `tools/stack.sh up` を立て、PC の収集アプリは Windows 側の cargo で組んで、網を切った Sandbox に入れる。
  録画（harness2 `record-run`）は、始める前に別の Sandbox が動いていれば撮らずに止まり、終わりと片付けでは動いている Sandbox を全部止める（premises.md）
- `wsb` の CLI（premises.md、確かさ 5）: `wsb start -c` で窓を開かずに起こせる・`wsb exec` は終了コードだけを返す（出力は共有のフォルダで受ける）・`ExistingLogin` で走らせる・画面の写しを撮るには `wsb connect`（窓を開く）が要る
- 配る zip は `tools/release/package.sh windows <置き場>` が組む（CI では `windows-latest` の bash）。この PC の Windows 側に Git Bash と VC の再頒布物はある（premises.md、確かさ 2。組み立ては走らせていない）
- 正典 distribution は「自動の試験は本物の配布元に問い合わせない」を持ち、走っている CH22 がそれを名指しの 2 つ（`tls_online_*` と CI の置く段）に限る形に書き直している（本人の決定 #332）。CH22 の置く段は、配る形を外につないで置き、#329 の型を PR の CI で見る

## Goals / Non-Goals

**Goals**: 確認バッチを作るたびに、その回に足した画面・PC の機能だけを、AI が本物（配る形）で軽く使い、件数と題を手順書に載せる。手で呼ぶ `/tsukau` も同じ差し込み口で立てる。

**Non-Goals**: 席の中身（触り方・候補の書き方・止まり方。harness2）・候補を直すこと・issue を作ること・携帯の相手・確認バッチに後から足す main の Story（`--story`）を範囲に入れること・外とつないで初めて出る不具合を見ること（CH22 の CI に任せる。深掘り Q2 の推奨）。

## Decisions

### D1（仮）. 差し込み口は `tools/tsukau/` に置き、置き場を明示する

- `tools/tsukau/tsukau.json`: `{"env_hook": "tools/tsukau/env-hook.sh", "targets": ["web", "pc"], "out_root": "~/.local/state/ashiato2/tsukau", "ready_timeout_s": 900}`。`down_timeout_s` は既定（300）
- `out_root` を明示するのは、既定（`~/.local/state/harness2/tsukau/<repo の名前>`）だと作業ツリーの名前（`ashiato2`・`ashiato2-verify`・使い捨ての名前）で置き場が分かれるから（review/deep.md R11）
- 確認バッチの席は `--label verify-<tag>` で、実行のフォルダの `request.json` から tag を引ける（review/deep.md R18。公開に新しく出るものは無い）
- 反転条件: 本人が、確認バッチの tag ではない手がかり（日付・Story の ID）で手元の結果を探したいと言ったとき（`--label` か置き場の分け方を変える）

### D2. 範囲と相手は、確認バッチの木の merge から機械で決める（深掘り Q1）

- 入力: 確認バッチの作業ツリーの `git log --merges --first-parent origin/main..HEAD` のうち、件名が「verify: #<PR> <head> を確認バッチ <tag> に入れる」のもの。ID は `<head>` の `feat/(st|ch)<NN>-` から（harness2 `scripts/sid.py` と同じ読み方）
- その merge が入れたファイル = `git diff --name-only <merge>^1 <merge>`。振り分けの表と試験の置き場の除き方は spec のとおり
- `tools/release/` を PC に入れるのは、配る zip の中身（同梱物・インストーラ）が変わり、PC の相手が入れる物そのものが変わるから（review/spec.md R13。CI の `windows` の判定も `^tools/release/` を Windows の通しに入れている）
- 範囲の文の「何のためか」は、価値の節の段落のうち引用（`>`）と注記（`（`）を飛ばした最初の段落。change の価値の節は雛形のままのことも、深掘りの注記や引用で始まることもある（`docs/changes/CH05.md`・`CH12.md`・`CH13.md`）
- **同じ木からは同じ範囲**（AI に要約させない）。同じ範囲でないと D10 の「同じ中身」を比べられない
- サーバの変更は PC の相手に入れる（配る zip のサーバは画面を中に持つので、席は PC の相手の中でも画面に辿り着ける。review/deep.md R17）

### D3（仮）. PC の相手は Sandbox を 1 つだけ起こし、自分の id だけを止める（review/deep.md R6・R7）

- 起こす前に `wsb list` で動いている Sandbox があれば `UNAVAILABLE=別の Sandbox が動いている` を置く（待たない）。録画・本人・手で呼んだ席と取り合わない
- 起こした id を `<state>/sandbox-id` に残し、片付けはその id だけを `wsb stop --id` で止める（録画の「全部止める」を写さない）
- 同じ確認バッチでは、席（verify-prep の中）が録画より先に終わる並び（`verify_batch.sh:186-202`）。閉じ損ねたら片付けの「残りあり」として段落に出る（D11）。そのとき同じバッチの PC の録画は撮れない（録画が自分で止まる）
- 片付けは止める前に、Sandbox の中のログを共有のフォルダへ写させ、`$TSUKAU_EVIDENCE/pc-logs/` へ写す（Sandbox は閉じると中身ごと消える。#329 の根拠は更新係のログだった）。`LOG_<名前>` は使わない（台本は `LOG_` を down の前に写すので、Sandbox の中のログは down の中でしか取れない）
- Sandbox のメモリは録画と同じ 4096 MB
- 反転条件: 別の Sandbox が動いていて PC の席が回らない回が続くとき（待つ形にし、待つ上限を決める）・写したログが足りず、題の根拠を辿れなかったとき（写すものを足す）

### D4. 席は窓を開いた Sandbox を `tools/tsukau/pc.sh` で触る（深掘り Q4）

- 順: `start` → サーバのインストール（`-r System`）→ サーバが応える → `wsb connect --id <id>`（窓を本人のデスクトップに開く）→ PC の収集アプリを `-r ExistingLogin` で起こす → `ready`
- 窓を開く前は `ExistingLogin` で走らず、`System` では利用者のデスクトップが無い（premises.md）。PC の収集アプリは前景の窓と入力を集めるので、利用者のセッションで起こさないと「入れて普通に使う」形にならない（review/spec.md R5。録画も `LogonCommand` で起こしている）
- `tools/tsukau/pc.sh exec <PowerShell の文>`: `wsb exec --id <id> -r ExistingLogin` で走らせ、出力を共有のフォルダのファイルで受けて標準出力に出す（`wsb exec` は終了コードしか返さない）
- `tools/tsukau/pc.sh shot <置き場>`: Sandbox の中で画面の写しを撮り、`$TSUKAU_EVIDENCE` へ写す

### D5（仮）. PC の配る zip は、席を呼ぶ前に確認バッチの木から手元で組み、ready の待ちに数えない（深掘り Q3・Q5）

- `tools/tsukau/build-pc.sh <置き場>`: 木を `C:\dev\ashiato2-tsukau-build\src` へ `rsync`（録画の写し先 `C:\dev\ashiato2-rec-build` とは分ける。手で呼んだ席と録画が同時に組んでも取り合わない）し、
  Windows 側の Git Bash で `tools/release/package.sh windows` を走らせ、配る形のサーバの zip と PC の収集アプリの zip を置き場へ写す。`target` と PostgreSQL の取得のキャッシュは `C:\dev\ashiato2-tsukau-build` に道具のキャッシュとして残す
- 組み立てを env_hook の `up` に入れない: 入れると `ready_timeout_s`（15 分）に数えられ、`target` が冷えた回は毎回「立てられなかった」になる。上限 2,400 秒と置き場の約束は spec
- 反転条件: 下流の Task 1 で測った組み立ての時間が、温まった `target` で 15 分を超えるか、組めないと分かったとき（前夜の版を使う形か、CI の組み立てを待つ形を本人に聞く。Q3 の案）。40 分の上限も同じ実測で見直す

### D6. Sandbox の網は切る（深掘り Q2 の推奨）

- `.wsb` に `<Networking>Disable</Networking>` を書く。外へも私設網へも手元の網へも何も出ない
- 推奨を「網を切る」に置いたのは、外につなぐ 2 案が、正典 distribution の「自動の試験が本物の配布元へ繋ぐのを名指しの 2 つに限る」（CH22 の書き直し。本人の決定 #332）に 3 つ目を足すことになり、
  CH22 の後に積む（requires に CH22）必要があるから（review/spec.md R1）。外とつないで初めて出る不具合（#329 の型）は CH22 の置く段が PR の CI で見る
- 案内に、網が切ってあり、更新係と PC の収集アプリの「つながらない」は網を切ったためだと書く（席が毎回同じ候補を書く雑音を減らす）
- **確かめていない**: 網を切った Sandbox で、インストーラ・サーバ・DB と、中のサーバと PC の収集アプリの loopback の通信が通るか（premises.md）。下流の Task 7（本物の Sandbox）で確かめる。通らなければ A として人間に返す
- 本人が外につなぐ案を選んだら: `spec --answers` が網の Requirement を「塞いで、塞げたことを中から確かめてから渡す（確かめられなければ切る）」に差し替え、distribution の約束を MODIFIED で改め、requires に CH22 を足す（review/spec.md R2〜R4 の前提の実測も要る）

### D7（仮）. 画面の相手は録画の専用環境と同じ立て方にする（review/deep.md R4・R11）

- 実行ごとの compose project `ashiato-tsukau-<実行の時刻>`・空いている port（DB・API・画面）・`openssl rand` の合言葉・`SEED=normal` の偽データで `tools/stack.sh up` を前景で立てる（`up` は前景に残り、台本が最後にプロセスグループを止める）
- `stack.sh` は `<state>/tree`（`--repo` の HEAD を detached で取り出した作業ツリー。片付けで消す）で走らせ、その木に乱数の合言葉と `COMPOSE_PROJECT_NAME` を持つ `.env` と port の上書き（`docker-compose.override.yml`）を置く（`tools/record-env.sh` の `up` と同じ形。`env-init.sh` は `COMPOSE_PROJECT_NAME` の行がある `.env` には触らない）。`--repo` には何も書かない（D9 の確かめと、席の台本の「repo を書き換えた」の警告に引っかからない）
- サーバと画面は、確認バッチでは `verify-prep` が組んだものを絶対パスで受け、無ければ組まずに断る（確認バッチの道で組むと 15 分の上限を越えうる。review/spec.md R12）。手で呼んだときは `<state>/tree` で組む（時間は下流が測って premises.md に書く）
- 案内: 画面の URL・その回の画面の合言葉（`<state>` の外に出さない。案内は実行のフォルダにだけ写る）・本物のブラウザで開いて写しを撮るコマンド（`web/` の Playwright で。下流が実地で通るものを書く）・サーバのログの場所・触らないもの
- `tools/record-env.sh` は `source` しない（録画の変数の契約に縛られる）。立て方を写す。共有の部品に切り出すのは 3 つ目の写しが要るとき
- 反転条件: 画面の相手を立てる時間が 15 分の上限に近づく・偽データでは範囲の機能を触れない回が続くとき（本人に、立て方か偽データの量を聞く）

### D8. 毎回の大きさは 10 分・20 万 token、立てる上限 15 分（深掘り Q5）

- `--minutes 10 --tokens 200000`。値は `verify-batch.sh` の頭の定数に置く（後から変えられる）。手で呼ぶときは呼ぶ側が決める
- 1 回の呼び出しは `timeout -k 360 1560`: 立てる上限 900 + 席 600 + 60 秒で SIGTERM、台本の片付け（`down_timeout_s` 300）を待って 1,920 秒で止める（review/spec.md R7。SIGKILL を先に送ると env_hook が別のプロセスグループに残り、Sandbox と project が残る）
- 確認バッチの準備は呼び出し全体を 6,540 秒で打ち切る（review/spec.md R8）
- 反転条件: 確認バッチの準備が 1 回 1 時間を超えることが続く・席が範囲を一通り触れずに `time` で終わることが続く（本人に大きさを聞き直す）

### D9（仮）. 席は使い捨ての作業ツリーで走らせる（review/deep.md R1・R4）

- `git worktree add --detach "$(mktemp -d …)" HEAD` → `python3 .harness2/scripts/init.py --worktree <その作業ツリー>`（`.claude/skills/tsukau` を繋ぐ。繋がないと席が起きない）→ `.env` は写さない
- `python3 <その作業ツリー>/scripts/tsukau.py run --repo <その作業ツリー> …` で呼ぶ（席の `claude -p` の cwd がその作業ツリーになる）
- 終わったら（どの止まり方でも。`trap`）`git worktree remove --force` と `git worktree prune`。席が書き換えたものは確認バッチの作業ツリーに残らない（`verify_batch.sh:113` で次のバッチが止まらない。手順書の commit に混ざらない）
- 反転条件: 使い捨ての作業ツリーを作って繋ぐのに分単位でかかる・席が確認バッチの木にしか無いもの（`dist/` の成果物）を読む必要があると分かったとき

### D10. 同じ中身の鍵と、前の結果の置き場（深掘り Q6）

- 鍵 = sha256（`origin/main` のコミット・相手・範囲の各行の ID と、その merge の 2 つ目の親（PR のいちばん新しいコミット）を並べたもの）
- 置き場: `<out_root>/verify-index/<鍵>.json` に `{"tag", "stop", "count", "titles", "cleanup_ok"}` を書く（repo の外。手元だけ）

### D11. 手順書の段落は決まった文言だけで組む（深掘り Q7・review/deep.md R10・review/spec.md R6・R15）

- 頭の 1 段落と相手ごとの 1 段落に分ける。`verify_checklist.py` は段落の中の改行を空白に替えるので、1 段落に相手を並べると 1 行に潰れる
- 読み元は席の台本の最後の行の JSON だけ（`public.json` は探さない）。`run`・`cleanup` の文・1 行目は写さない
- verify-prep 側は `timeout` と `||` で受け、決まった 1 段落「使ってみる席: 呼べなかった（`tools/tsukau/verify-batch.sh` が非 0 で終わったか、上限で打ち切った）」を足す。準備の rc を変えない

### D12（仮）. 題は写す前に機械で伏せる（review/deep.md R5）

- 伏せる形: この PC の利用者名（`id -un`）・ホスト名（`hostname`）・私設網の名前（`tailscale status --json` の `MagicDNSSuffix`。取れなければ飛ばす）・確認バッチの作業ツリーの `.env` の値（8 字以上のもの）・
  絶対パスの形（`/home/`・`/mnt/`・`/Users/`・`C:\`・`\\` で始まる語）。当たれば題を「（伏せた）」に置き換え、件数には数える
- 守りの本体は席の規範（harness2 の SKILL.md「題に実データを書かない」）で、これは公開の repo に出す前の機械の網。網に掛からない私的な語（人の名前・場所の名前）は止められない
- 失わない側: 伏せても、題と本文は手元の実行のフォルダに残る（`RESULT.md`）。伏せた題は手元で読める
- 反転条件: 伏せる題が多すぎて、手順書の件数と題から何も分からない回が続く（伏せる形を狭めるか、席の規範を強める依頼を harness2 に出す）

### D13. 台本の自己試験を CI で走らせる

- `tools/tsukau/verify-batch.sh --self-test` と `tools/tsukau/env-hook.sh --self-test` を、CI の `rust` の job の `./tools/check-private.sh --self-test` の行の後に 1 行ずつ足す（CH22 が足す `update-apply-install.sh` の近くと離す。review/spec.md R20）
- 自己試験は、偽の `scripts/tsukau.py`・偽の `wsb`・偽の `docker`・偽の `gh`・偽の起動の台本を一時の置き場に置いて走らせる（本物の Sandbox・docker・claude を使わない）
- 差し込み口の形は自己試験が `tsukau.json` を読んで確かめる（CI の runner に `.harness2` は無い。`ci.yml:648-660`）。席の台本自身の `check` は手元（下流の Task 9・merge gate）で走らせる

### D14（仮）. CH21 だけの確認バッチでは席が回らないので、実地の 1 回は同じ道を使い捨ての確認バッチの木で通す（深掘り Q8）

- CH21 自身は道具と文書だけを変えるので、D2 の振り分けではどの相手にも入らない。CH21 だけの確認バッチの手順書には「回さなかった」の 2 行が載る（Q8 の context に書いた）
- 実地: `tools/tsukau/real-run.sh` が、`origin/main` に、画面のファイルと PC のファイルを 1 行ずつ変えた偽の `feat/ch99-tsukau-real` を「確認バッチ <tag> に入れる」merge で重ねた使い捨ての木を作り、
  `tools/tsukau/verify-batch.sh` を本物で通す（使い捨ての作業ツリー・`init.py`・組み立て・絶対パスの受け渡し・段落の組み立てを本物で通る。review/spec.md R14）。
  終わりの条件: 画面と PC の段落が「席が自分で終えた」「時間を使い切った」「token を使い切った」のどれかで始まり、「片付けに残りがある」が無く、手元の `result.json` の `journal` が 1 以上
- 反転条件: 本人が「CH21 の確認バッチで席が回るのを見たい」と答えたとき（範囲を手で渡す口を足すか、画面か PC を変える Story と同じバッチに入れる）

### D15（仮）. 題は確認バッチの commit を止める形も伏せ、1 行に整える（review/spec.md R9）

- 手順書は pre-commit の `check-private`・`check-secrets` を通って commit される。題 1 つで commit が落ちると確認バッチ全体が止まり、stage したままのファイルが次のバッチも止める
- 伏せる判定は `tools/check-private.sh` と `tools/check-secrets.sh` そのものを題に当てる。どちらも自分の repo の根へ `cd` してその index を見るので、台本を持つ使い捨ての作業ツリー（D9 と同じ作り方。席を呼ばなかった回は検査のためだけに作る）に題を 1 行ずつ置き、`GIT_INDEX_FILE` の一時の index で stage して当て、落ちた行の題を伏せる（`check-private.sh --self-test` と同じ当て方）。確認バッチの作業ツリーには何も置かない（途中で止まると未追跡のファイルが次のバッチを止める）。形を写して持たない（写すと検査の側の変更とずれる）
- 検査を走らせられない（gitleaks を取れない など。`check-secrets.sh` は取れなければ落ちる）ときは、全部の題を伏せる（伏せる側に倒す）
- 伏せなかった題は、改行を空白・80 字で切る・`@` を「＠」・`#<数字>` の `#` を「＃」に替える（席の台本の 80 字・1 行の制限は `candidate` の口だけで、手で書いた行は通る。`tsukau.py:307`・`:574-575`）
- 失わない側: 伏せた題も本文も手元の `RESULT.md` に残る
- 反転条件: 検査に当てる時間が 1 回の準備で目立つ（数十秒を超える）とき（形を写して持つ）

## Risks / Trade-offs

- **外とつないで初めて出る不具合はこの席では見えない**（D6）: CH22 の CI に任せる。本人が外につなぐ案を選べば、distribution の約束を改める
- **本人の画面に窓が出る**（Q4）: PC を変えた回ごとに、Sandbox の窓が本人のデスクトップに出る。本人の PC で収集アプリが動いていれば、その間の前景の記録に Sandbox の窓が入りうる
- **準備が延び、錠を握る**（D8）: その間、記録と fix の撮り直しが待たされる（最大 6,540 秒）
- **席の作業ツリーは HEAD を取り出す**: 確認バッチの木の `docs/verify` の commit より前に走るので、手順書の commit とは別のコミットを見る（席が見る木は手順書を持たない）
