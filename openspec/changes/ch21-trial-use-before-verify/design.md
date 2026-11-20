# CH21 design — 確認バッチの前に、その回で足した機能だけを使ってみる席（/tsukau）で軽く確かめる

## Context

- 席の台本 `scripts/tsukau.py run`（harness2）は、差し込み口 `tools/tsukau/tsukau.json` を読み、`env_hook up` → `ready` を待つ（`ready_timeout_s`）→ `claude -p "/tsukau seat <実行のフォルダ>"`（cwd は `--repo`）
  → 時間・token の上限で止める → `LOG_<名前>` を写す → `env_hook down` → `result.json`・`RESULT.md`・`public.json` を書く。rc は 0（done/time/tokens）・2（引数・設定の誤り。実行のフォルダを作らない）・3（crashed/env_failed/interrupted）。
  最後の行に `{"v","stop","count","titles","run","cleanup"}` の JSON を出す（`.harness2/scripts/tsukau.py:636-675`）。env_hook には `TSUKAU_RUN`・`TSUKAU_EVIDENCE`・`TSUKAU_TARGET`・`TSUKAU_SCOPE` と呼び手の環境変数がそのまま渡る（`:447-450`）
- 確認バッチ（harness2 `scripts/verify_batch.sh`）は、`<repo>-verify` の作業ツリーで `origin/main` から `verify/<tag>` を作り、PR ごとに「verify: #<PR> <head> を確認バッチ <tag> に入れる」merge を重ね（`:117-160`）、
  `tools/verify-prep.sh <tag>` を呼ぶ（`:186-196`。落ちたら manifest なしで続ける）。その後に録画（`record-run verify`）、手順書、`docs/verify` の commit と push、PR 本文（manifest をそのまま載せる。`:241`）。
  作業ツリーに未コミットの変更があると次の確認バッチが止まる（`:113`）
- `tools/verify-prep.sh` は `set -euo pipefail`。main の checkout の `.env`（本物の合言葉）を作業ツリーへ写す（`:24-33`）
- 録画の専用環境（`tools/record-env.sh`）は、専用の compose project・空いている port・乱数の合言葉で `tools/stack.sh up` を立て、PC の収集アプリは Windows 側の cargo で組んで、網を切った Sandbox に入れる。
  録画（harness2 `record-run`）は、始める前に別の Sandbox が動いていれば撮らずに止まり、終わりと片付けでは動いている Sandbox を全部止める（premises.md）
- `wsb` の CLI（premises.md、確かさ 5）: `wsb start -c` で窓を開かずに起こせる・`wsb exec` は終了コードだけを返す（出力は共有のフォルダで受ける）・画面の写しと利用者としての操作には `wsb connect`（窓を開く）が要る・網をつなぐと私設網の機械に届く
- 配る zip は `tools/release/package.sh windows <置き場>` が組む（CI では `windows-latest` の bash）。この PC の Windows 側に Git Bash と VC の再頒布物はある（premises.md、確かさ 2。組み立ては走らせていない）

## Goals / Non-Goals

**Goals**: 確認バッチを作るたびに、その回に足した画面・PC の機能だけを、AI が本物（配る形）で軽く使い、件数と題を手順書に載せる。手で呼ぶ `/tsukau` も同じ差し込み口で立てる。

**Non-Goals**: 席の中身（触り方・候補の書き方・止まり方。harness2）・候補を直すこと・issue を作ること・携帯の相手・確認バッチに後から足す main の Story（`--story`）を範囲に入れること。

## Decisions

### D1（仮）. 差し込み口は `tools/tsukau/` に置き、置き場を明示する

- `tools/tsukau/tsukau.json`: `{"env_hook": "tools/tsukau/env-hook.sh", "targets": ["web", "pc"], "out_root": "~/.local/state/ashiato2/tsukau", "ready_timeout_s": 900}`。`down_timeout_s` は既定（300）
- `out_root` を明示するのは、既定（`~/.local/state/harness2/tsukau/<repo の名前>`）だと作業ツリーの名前（`ashiato2`・`ashiato2-verify`・使い捨ての名前）で置き場が分かれるから（review/deep.md R11）
- 手で呼ぶとき（`/tsukau`）は main の checkout か任意の作業ツリーから同じ差し込み口を使う。確認バッチの席は `--label verify-<tag>` で、実行のフォルダの `request.json` から tag を引ける（review/deep.md R18。公開に新しく出るものは無い）
- 反転条件: 本人が、確認バッチの tag ではない手がかり（日付・Story の ID）で手元の結果を探したいと言ったとき（`--label` か置き場の分け方を変える）

### D2. 範囲と相手は、確認バッチの木の merge から機械で決める（深掘り Q1）

- 入力: 確認バッチの作業ツリーの `git log --merges --first-parent origin/main..HEAD` のうち、件名が「verify: #<PR> <head> を確認バッチ <tag> に入れる」のもの。ID は `<head>` の `feat/(st|ch)<NN>-` から（harness2 `scripts/sid.py` と同じ読み方）
- その merge が入れたファイル = `git diff --name-only <merge>^1 <merge>`
- 相手の振り分け（パスの頭で。上から当てる）:

  | パス | 相手 |
  |---|---|
  | `web/src/`・`web/index.html`・`web/public/` | 画面 |
  | `crates/`（server・collector-windows・updater・update-check・installer・winsvc・build-stamp）・`migrations/` | PC |
  | それ以外（`collector-android/`・`docs/`・`tools/`・`web/e2e/`・`openspec/`・`.github/` など） | どれにも入れない |

- 範囲の文: 入れたものごとに「<ID> <表題>: <何のためか>」を 1 行。何のためか = 価値の節の最初の段落の 1 行目（120 字で切る）。雛形（「（」で始まる）なら表題だけ
- **同じ木からは同じ範囲**（AI に要約させない）。同じ範囲でないと D10 の「同じ中身」を比べられない
- サーバの変更は PC の相手に入れる（配る zip のサーバは画面を中に持つので、席は PC の相手の中でも画面に辿り着ける。review/deep.md R17）

### D3（仮）. PC の相手は Sandbox を 1 つだけ起こし、自分の id だけを止める（review/deep.md R6・R7）

- 起こす前に `wsb list` で動いている Sandbox があれば `UNAVAILABLE=別の Sandbox が動いている` を置く（待たない）。録画・本人・手で呼んだ席と取り合わない
- 起こした id を `<state>/sandbox-id` に残し、片付けはその id だけを `wsb stop --id` で止める（録画の「全部止める」を写さない）
- 同じ確認バッチでは、席（verify-prep の中）が録画より先に終わる並び（`verify_batch.sh:186-202`）。閉じ損ねたら片付けの「残りあり」として行に出る（D11）。そのとき同じバッチの PC の録画は撮れない（録画が自分で止まる）
- 片付けは止める前に、Sandbox の中の状態の置き場の `logs`・更新係の置き場の `checks.jsonl` と `logs`・PC の収集アプリの置き場のログ・システムのイベントの記録のうち ashiato のサービスに関わるものの抜き書きを共有のフォルダへ写させ、
  `$TSUKAU_EVIDENCE/pc-logs/` へ写す（Sandbox は閉じると中身ごと消える。#329 の根拠は更新係のログだった）。`LOG_<名前>` は使わない（台本は `LOG_` を down の前に写すので、Sandbox の中のログは down の中でしか取れない）
- Sandbox のメモリは録画と同じ 4096 MB
- 反転条件: 別の Sandbox が動いていて PC の席が回らない回が続くとき（待つ形にし、待つ上限を決める）・写したログが足りず、題の根拠を辿れなかったとき（写すものを足す）

### D4. 席は窓を開いた Sandbox を `tools/tsukau/pc.sh` で触る（深掘り Q4）

- 立て終わったら `wsb connect --id <id>` で窓を本人のデスクトップに開く（開かないと `ExistingLogin` で走らせられず、画面の写しが撮れない。premises.md）
- `tools/tsukau/pc.sh exec <PowerShell の文>`: `wsb exec --id <id> -r ExistingLogin` で走らせ、出力を共有のフォルダのファイルで受けて標準出力に出す（`wsb exec` は終了コードしか返さない）
- `tools/tsukau/pc.sh shot <置き場>`: Sandbox の中で画面の写しを撮り、`$TSUKAU_EVIDENCE` へ写す
- 案内（GUIDE）に、この 2 つの口・入れたサービスとログの場所・PC の収集アプリの起こし方・触らないものを書く

### D5（仮）. PC の配る zip は、席を呼ぶ前に確認バッチの木から手元で組み、ready の待ちに数えない（深掘り Q3・Q5）

- `tools/tsukau/build-pc.sh <置き場>`: 木を `C:\dev\ashiato2-tsukau-build\src` へ `rsync`（録画の写し先 `C:\dev\ashiato2-rec-build` とは分ける。手で呼んだ席と録画が同時に組んでも取り合わない）し、
  Windows 側の Git Bash で `tools/release/package.sh windows` を走らせ、配る形のサーバの zip と PC の収集アプリの zip を置き場へ写す。`target` と PostgreSQL の取得のキャッシュは `C:\dev\ashiato2-tsukau-build` に道具のキャッシュとして残す
- 確認バッチでは `verify-batch.sh` が PC の範囲があるときだけ、席を呼ぶ前に `dist/verify-<tag>/tsukau-pc/` へ組み、その置き場を環境変数 `ASHIATO_TSUKAU_PC_ZIPS` で env_hook に渡す。組み立ての上限は 40 分。組めなければ PC の行は「配る zip を組めず、回さなかった」
- 手で呼ぶときは、先に `tools/tsukau/build-pc.sh` で `dist/tsukau-pc/<HEAD のコミット>/` へ組む。env_hook はそこか `ASHIATO_TSUKAU_PC_ZIPS` を見て、無ければ `UNAVAILABLE=`（理由に `tools/tsukau/build-pc.sh` の名前）
- 組み立てを env_hook の `up` に入れない: 入れると `ready_timeout_s`（15 分）に数えられ、`target` が冷えた回は毎回「立てられなかった」になる
- 反転条件: 下流の Task 1 で測った組み立ての時間が、温まった `target` で 15 分を超えるか、組めないと分かったとき（前夜の版を使う形か、CI の組み立てを待つ形を本人に聞く。Q3 の案）

### D6. 網は Sandbox の中で塞ぎ、塞げたことを中から確かめてから渡す（深掘り Q2）

- 起こした直後（配る形を入れる前）に `wsb exec -r System` で、私設網と手元の網の宛先（RFC 1918 の私用の 3 つの範囲・私設網（tailscale）が使う CGNAT の範囲（RFC 6598）・IPv4 のリンクローカル・IPv6 の私用（ULA）とリンクローカルの、7 つの範囲）への外向きを塞ぐ規則を足す。範囲の字面（CIDR）は `tools/check-private.sh` が私設 IP として止めるので、文書にも台本にも字面で書かず、台本は数から組む（例: `"{0}.0.0.0/8" -f 10`）
- 確かめの宛先: この PC の私設網の IPv4（Windows 側の `tailscale.exe ip -4`。無ければ飛ばす）の 445、WSL の私設網の IPv4 の 22、この PC の手元の網の IPv4（既定の経路の口）の 445。どれか 1 つでも TCP でつながれば「塞げなかった」
- 外の確かめ: `api.github.com:443` に TCP でつながるか（`no-outside` か `outside-only` かを分けるだけで、外に届かなくても止めない）
- 塞げなかったら、その Sandbox を止め、網を切った設定で起こし直す（`INFO_NETWORK=disabled`）。私設網の宛先の IP は `INFO_` にも案内にも書かない（手元の事情）
- **守りの限り**: 規則は Sandbox の中にあるので、Sandbox の中の管理者（席の `pc.sh exec` は `ExistingLogin` で走る。インストーラは管理者で走る）は外せる。案内に「ファイアウォールの規則を変えない」を書くが、機械では止めない。
  席は案内の「触らない」を守る規範だけで動く（harness2 の `/tsukau`）。私設網へ届かせない守りを Sandbox の外（この PC の側）に置く形は、確かめていない（反転条件）
- **確かめていない**: Windows のファイアウォールでは塞ぐ規則が許す規則より強いので、RFC 1918 の範囲を塞ぐと、Sandbox の DNS の問い合わせ先（Hyper-V の既定の口。私用の範囲にあることが多い）も塞ぎうる。
  そのときは外の名前が引けず `no-outside` になる（止まらないが、#329 の型は見えない）。下流の Task 6 の実地で確かめ、`no-outside` なら既定の口の /32 を塞ぐ範囲から抜く
- 反転条件: 実地で `outside-only` にならない（DNS が引けない）・Sandbox の中の規則が効かないと分かったとき

### D7（仮）. 画面の相手は録画の専用環境と同じ立て方にする（review/deep.md R4・R11）

- 実行ごとの compose project `ashiato-tsukau-<実行の時刻>`・空いている port（DB・API・画面）・`openssl rand` の合言葉・`SEED=normal` の偽データで `tools/stack.sh up` を前景で立てる（`up` は前景に残り、台本が最後にプロセスグループを止める）
- 作業ツリーの `.env` は読まない。`tools/stack.sh` は cwd に `.env` があれば読む（`tools/stack.sh:28`）ので、env_hook は `stack.sh` を `.env` の無い木で走らせ、合言葉・port・project は環境変数で渡す。
  `.env` の無い木 = `<state>/tree` に `--repo` の HEAD を detached で取り出した作業ツリー（確認バッチでも、手で呼んだ main の checkout でも同じ。片付けで消す）。`--repo` の作業ツリーには何も書かない（D9 の確かめと、席の台本の「repo を書き換えた」の警告に引っかからない）
- サーバと画面は、確認バッチでは `verify-prep` が組んだもの（`ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` で渡す）を使い、手で呼んだときは env_hook が組む（数分。15 分の上限に収まる）
- 案内: 画面の URL・その回の画面の合言葉（`<state>` の外に出さない。案内は実行のフォルダにだけ写る）・本物のブラウザで開いて写しを撮るコマンド（`web/` の Playwright で。下流が実地で通るものを書く）・サーバのログの場所・触らないもの
- 片付け: `docker compose -p <project> down -v`。終わった後にその project のコンテナか volume が残れば rc≠0
- `tools/record-env.sh` は `source` しない（録画の変数の契約に縛られる）。立て方を写す。共有の部品に切り出すのは 3 つ目の写しが要るとき
- 反転条件: 画面の相手を立てる時間が 15 分の上限に近づく・偽データでは範囲の機能を触れない回が続くとき（本人に、立て方か偽データの量を聞く）

### D8. 毎回の大きさは 10 分・20 万 token、立てる上限 15 分（深掘り Q5）

- `--minutes 10 --tokens 200000`。値は `verify-batch.sh` の頭の定数に置く（後から変えられる）。手で呼ぶときは呼ぶ側が決める
- 1 回の呼び出しの打ち切り（`timeout`）は 900 + 600 + 300 + 120 = 1,920 秒。準備全体の延びは、範囲のある相手の数 ×（1,920 秒まで）＋ PC の組み立て（D5。40 分まで）
- 反転条件: 確認バッチの準備が 1 回 1 時間を超えることが続く・席が範囲を一通り触れずに `time` で終わることが続く（本人に大きさを聞き直す）

### D9（仮）. 席は使い捨ての作業ツリーで走らせる（review/deep.md R1・R4）

- `git worktree add --detach "$(mktemp -d …)" HEAD` → `python3 .harness2/scripts/init.py --worktree <その作業ツリー>`（`.claude/skills/tsukau` を繋ぐ。繋がないと席が起きない）→ `.env` は写さない
- `python3 <その作業ツリー>/scripts/tsukau.py run --repo <その作業ツリー> …` で呼ぶ（席の `claude -p` の cwd がその作業ツリーになる）
- 終わったら（どの止まり方でも。`trap`）`git worktree remove --force` と `git worktree prune`。席が書き換えたものは確認バッチの作業ツリーに残らない（`verify_batch.sh:113` で次のバッチが止まらない。手順書の commit に混ざらない）
- harness2 を繋げなかったら（`init.py` が非 0）、その相手の行は「席を呼べなかった」
- 反転条件: 使い捨ての作業ツリーを作って繋ぐのに分単位でかかる・席が確認バッチの木にしか無いもの（`dist/` の成果物）を読む必要があると分かったとき

### D10. 同じ中身の鍵と、前の結果の置き場（深掘り Q6）

- 鍵 = sha256（`origin/main` のコミット・相手・範囲の各行の ID と、その merge の 2 つ目の親（PR のいちばん新しいコミット）を並べたもの）
- 置き場: `<out_root>/verify-index/<鍵>.json` に `{"tag", "public", "cleanup"}` を書く（repo の外。手元だけ）
- 前の結果の止まり方が `done`・`time`・`tokens` のときだけ写す（落ちた回の結果を写し続けない）

### D11. 手順書の段落は決まった文言だけで組む（深掘り Q7・review/deep.md R10）

- 段落の頭: 「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）——」。相手ごとに「画面: …」「PC: …」の 1 行
- 止まり方 → 文言の表は spec の Requirement のとおり。`detail`・実行のフォルダのパス・候補の本文は写さない（手元の `RESULT.md` にある）
- 席の台本の最後の行の JSON を読めなければ、`public.json` を探さずに「席を呼べなかった」
- verify-prep 側は `tools/tsukau/verify-batch.sh` の失敗を `||` で受け、決まった 1 段落「使ってみる席: 呼べなかった（`tools/tsukau/verify-batch.sh` が非 0 で終わった）」を足す。準備の rc を変えない
- verify-batch の手元の出力（どの相手をどの範囲で呼んだか・実行のフォルダ）は `dist/verify-<tag>/tsukau.log` に書く（`dist/` は git が無視する）

### D12（仮）. 題は写す前に機械で伏せる（review/deep.md R5）

- 伏せる形: この PC の利用者名（`id -un`）・ホスト名（`hostname`）・私設網の名前（`tailscale status --json` の `MagicDNSSuffix`。取れなければ飛ばす）・確認バッチの作業ツリーの `.env` の値（8 字以上のもの）・
  絶対パスの形（`/home/`・`/mnt/`・`/Users/`・`C:\`・`\\` で始まる語）。当たれば題を「（伏せた）」に置き換え、件数には数える
- 守りの本体は席の規範（harness2 の SKILL.md「題に実データを書かない」）で、これは公開の repo に出す前の機械の網。網に掛からない私的な語（人の名前・場所の名前）は止められない
- 失わない側: 伏せても、題と本文は手元の実行のフォルダに残る（`RESULT.md`）。伏せた題は手元で読める
- 反転条件: 伏せる題が多すぎて、手順書の件数と題から何も分からない回が続く（伏せる形を狭めるか、席の規範を強める依頼を harness2 に出す）

### D13. 台本の自己試験を CI で走らせる

- `tools/tsukau/verify-batch.sh --self-test` と `tools/tsukau/env-hook.sh --self-test` を、CI の `rust` の job の自己試験の並び（`tools/release/update-apply-install.sh --self-test` の近く）に 1 行ずつ足す
- 自己試験は、偽の `scripts/tsukau.py`・偽の `wsb`・偽の `docker`・偽の起動の台本を一時の置き場に置いて走らせる（本物の Sandbox・docker・claude を使わない）
- `scripts/tsukau.py check`（harness2）は CI の runner に無い（`.harness2` は gitignore。`ci.yml:648-660` と同じ）。自己試験は `tsukau.json` の鍵・`env_hook` が実行できること・`targets` を自分で確かめ、
  `scripts/tsukau.py` があるときだけ（手元の merge gate）それも走らせる

### D14. CH21 だけの確認バッチでは席が回らないので、実地の 1 回は下流の Task の証跡にする（深掘り Q8）

- CH21 自身は道具と文書だけを変えるので、D2 の振り分けではどの相手にも入らない。CH21 だけの確認バッチの手順書には「回さなかった」の 2 行が載る
- Q8 の推奨の完了（「確認バッチで 1 回実地に走り、手順書に件数と題か警告が載る。PC の相手が立てられずに終わらない」）は、次の 2 つで満たす:
  1. 下流の最後の Task で、同じ差し込み口を本物で 1 回ずつ呼ぶ（画面と PC。`scripts/verify-run` の証跡に残る）。PC の止まり方が `env_failed` でないこと
  2. 確認バッチの手順書に席の段落が載っている（CH21 だけなら「回さなかった」。ほかの Story と一緒なら件数と題）
- 反転条件: 本人が「CH21 の確認バッチで席が回るのを見たい」と答えたとき（範囲を手で渡す口を足すか、画面か PC を変える Story と同じバッチに入れる）

## Risks / Trade-offs

- **外へ出るもの**（Q2）: 推奨の形では、PC の相手を立てるたびに、更新係と PC の収集アプリの配布元への問い合わせ（この PC の網の IP と時刻）と、席が開いた外のサイトへの接続が残る。取り消せない
- **私設網の守りは Sandbox の中の規則**（D6）: 中の管理者は外せる。守りは「確かめてから渡す」と案内の規範まで
- **本人の画面に窓が出る**（Q4）: PC を変えた回ごとに、Sandbox の窓が本人のデスクトップに出る。本人の PC で収集アプリが動いていれば、その間の前景の記録に Sandbox の窓が入りうる
- **準備が延び、錠を握る**（D8）: その間、記録と fix の撮り直しが待たされる
- **席の作業ツリーは HEAD を取り出す**: 確認バッチの木の `docs/verify` の commit より前に走るので、手順書の commit とは別のコミットを見る（手順書は席の後に作られる。席が見る木は手順書を持たない）
