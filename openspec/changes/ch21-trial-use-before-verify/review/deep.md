# CH21 深掘りの問いの独立検査（deep-review、第 1 回）

検査したもの: `openspec/changes/ch21-trial-use-before-verify/deep-questions.json`（Q1〜Q6、format 2）。
問いの一覧を見る前に、手順 1〜8 を依頼（`docs/changes/CH21.md`。「どこまで」は本人の決定として扱い、問い直さない）・
`premises.md`・harness2 の契約（`.harness2/scripts/tsukau.py` 冒頭・README「使ってみる席」・`skills/tsukau/SKILL.md`）・
呼び出し元（`tools/verify-prep.sh`・`.harness2/scripts/verify_batch.sh`・`.harness2/graph/harness_graph/verify.py`）・
Sandbox の実績（`.harness2/scripts/record-run`・`tools/record-env.sh`・`tools/recording/README.md`）から洗い出し、その後で一覧と突き合わせた。

## 手順ごとの結果（要約）

- 手順 1（要件どうしの衝突）: change なので `satisfies: []`。代わりに依頼の項目どうし・依頼と既存の台本を突き合わせた。
  「席が落ちても準備は止めない」と、確認バッチの作業ツリーが汚れたら次のバッチが止まる既存の検査がぶつかる（R1）。
  Q1（毎回両方）と Q5（変えたパスで相手を選ぶ）も食い違う（R17）
- 手順 2（扉の幅）: 「画面も入れられるなら入れる」は依頼が決めている（R17）。「手順書に写すのは件数・題・止まり方だけ」を Q6 が問い直している（R18）。
  「毎回呼ぶ」を Q4 が狭めているのに、狭めていると書いていない（R19）
- 手順 3（新たに立つ一方通行）: 外へ出るものとして、Sandbox の網（Q2）以外に次の 3 つが一覧に無い。席が読んだもの（本物の合言葉・本人の配布版）がモデルへの入力になる（R4）、
  公開の repo に出す題（R5）、Sandbox を閉じると一緒に消えるログ（R6、取り込めなかったデータ）。
  Q2 の「外に出るもの」は書き足りない（R13・R14）
- 手順 4（日常に影響する選択）: 一覧にあるのは Q3（時間と token）だけ。Sandbox の取り合い（R7）、準備が延びて確認バッチの作業ツリーの錠を握り続けること（R8）、
  Sandbox の窓が本人の PC に出て本人の記録に混ざること（R9）、席が落ちたときの警告と、落ち続けたとき（R10）が無い
- 手順 5（既存コードが依頼を満たしていない箇所）: `grep` と台本を読んで確かめた。`verify_batch.sh:113` の作業ツリーの検査（R1）、
  `verify-prep.sh:28-29` の本物の `.env` の写し（R4）、`tsukau.py:245` の置き場の既定が作業ツリーの名前で分かれること（R11）、
  `verify_batch.sh:189-193`（verify-prep 全体が落ちると manifest ごと消え、警告も残らない。R10・R12）
- 手順 6（過去の取りこぼし）: `upstream_misses.py report` の観点のうち当てはまったのは次のもの。
  7（解消するまで待つなら上限を決める）→ R7。10（OS のコマンドは実機で確かめる）→ R3。26（port を決め打ちしない）→ R11。
  40（後で見つける手がかり）→ R18。55（止めたことを本人がどう知るか）→ R10。38（写しに本物の記録が入るか）→ R4。
  85（新しい置き場の保持）にも当たるが、SKILL.md:236（要件にもブリーフにも無い保存期限は問わない）に従い、指摘にしない
- 手順 7（族の網羅）: Q2（網）の族のうち、Sandbox が手元の網・私設網に届くか（R13）と、中から外へ出るもの全体（R14）が欠けている。
  席が落ちる場合の族（env_failed・crashed・time/tokens・interrupted・rc 2・片付けの残り・repo の書き換え・範囲が空）は一覧に 1 つも無い（R1・R7・R10）
- 手順 8（前提の確かさ）: 確かさ 1 の行 7 の上に、`loss` のある Q2 が載っている（R13）。行 4（確かさ 3）の上に、依頼が必須とした PC の相手が載っている（R3）。
  行 5 は確かさ 2 なのに中身が実物と違う（R15）。外の形式に依る前提として、配る zip を手元で組めるかが表に無い（R2）

## 処置の置き方（questions の段）

処置の印が付いていない指摘は 2 種類ある。1 つは、扉を開けたままにする既定（C）と、伏せる側に倒す仮（P）。spec の段が design の D<n>（仮）と台帳（provisional.py）に置き、`fixed D<n> 仮` に書き換える。もう 1 つは本人への問いにしたもの。record の後に答えを指して書き換える。

---

## R1. 席が確認バッチの作業ツリーを書き換えると、次の確認バッチの準備が止まる（依頼の「止めない」とぶつかる）
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: `.harness2/scripts/tsukau.py:501`（`claude -p` は `cwd=self.repo` で、確認バッチから呼べば `<repo>-verify` になる）・`:397`（`--permission-mode auto`）・`:576-577`（前後の `git status` の差は warnings に数えるだけで、戻さない）／`.harness2/scripts/verify_batch.sh:113`（作業ツリーに未コミットの変更があると `error` で exit 1。次のバッチ全体が止まる）・`:216`（`git add docs/verify` で手順書を commit・push する。席が `docs/verify/` の下に書けば、それも公開の repo に入る）／`.gitignore:37,40,41`（無視されるのは `dist/`・`web/test-results/`・`web/playwright-report/` だけ）
- kind: conflict
- 提案: 問いを 1 つ足すか、扉を開けたままにする既定（C）として design に置く。席の後に verify-prep が作業ツリーを席の前の状態へ戻す（増えた変更は警告に書いてから捨てる）か、席を別の使い捨ての作業ツリーで走らせるか。どちらにしても「席が repo を書き換えた」を警告の族に入れる（R10）。
- 処置: fixed D9 仮 —— 席は確認バッチの作業ツリーの HEAD を detached で取り出した使い捨ての作業ツリーで走らせ、harness2 を繋ぎ、終わったら消す（spec「確認バッチの準備は、範囲のある相手ごとに 1 回、使い捨ての作業ツリーで…」）

## R2. PC の相手に入れる「配る zip」を、どこで・何から組むかが、問いにも前提の表にも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json / premises.md
- 根拠: `tools/verify-prep.sh:39-74` が作るのは Linux のサーバ・画面・APK だけで、Windows の zip は作らない／Windows の配る zip は `tools/release/package.sh windows` → `build.sh server-windows`・`collector-windows`（`pgsql.sh` での PostgreSQL の取得、`ASHIATO_VC_CRT_DIR` の VC ランタイムの同梱）で組み、実績は CI の `windows-latest` だけ（`.github/workflows/release.yml:61`）／録画の実績（`tools/record-env.sh:192-206`・`tools/recording/README.md`）は、PC の収集アプリの exe だけを Windows 側の cargo で組んで、網を切った Sandbox に入れたもの。サーバは WSL 側にある。インストーラ・同梱の DB・更新係を Sandbox に入れた実績は無い／前の CH21 はこれを問いにしていた（`git show bd3dee549^:openspec/changes/ch21-trial-use-before-verify/deep.md` の Q1「どの配る形を入れるか」と Q2「配る zip をどこから持ってくるか」）
- kind: premise
- 提案: premises.md に行を足す。確認バッチの木から Windows の配る zip（サーバ・PC の収集アプリ）を手元（WSL から Windows 側）で組めるか、何分かかるかを、実物で走らせて確かさ 4 で書く。組めないなら、PC の相手の立て方を問いにする（例: 夜の版を使うと、その回の機能が入らない）。どの zip を入れるかも、推奨つきの B で足す。
- 処置: fixed deep-questions.json — 問いに足した（Q3「配る zip の出所」）。premises.md に行を足した（手元で zip 全体を組む時間は未測。下流の最初の Task で測る。組めなければ警告にして席を回さない）

## R3. 席が Sandbox の中を見て触れるかが確かさ 3 のままなのに、PC を必須の相手にしている
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md
- 根拠: premises.md 行 4（「出力を返すか、画面の写しを撮れるかは確かめていない（Sandbox を起こしていない）」）／依頼の「どこまで」（相手は少なくとも PC）／`upstream_misses.py report` の観点 10（OS のコマンドを情報源にする決定は、実機で確かめて前提の欄に書く）
- kind: premise
- 提案: 問いを出す前に Sandbox を 1 回起こし、`wsb exec` の出力が返るか、画面の写しが取れるかを確かめて確かさ 4 にする。取れなければ、PC の相手で席が見られるもの（ログだけか、画面もか）を問いにする。Q3 の「10 分」も、それで足りるかが変わる。
- 処置: fixed premises.md — 2026-11-20 にこの PC で Sandbox を起こして確かめた（premises.md 行 4・5、確かさ 5）。`wsb exec` は終了コードだけ返し出力は共有フォルダで受ける。画面の写し・操作には窓を開く（`wsb connect`）必要がある → 問いに足した（Q4「Sandbox の窓」）

## R4. 席が触ってはいけない本人の本物（本物の合言葉・本人の PC の配布版・確認バッチの DB）をどう遠ざけるかが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠:
  - `tools/verify-prep.sh:28-29`: main の checkout の `.env`（本物の画面の合言葉と API の合言葉）を、確認バッチの作業ツリーへ写す
  - `.harness2/scripts/tsukau.py:501`: 席はその作業ツリーを cwd にして動き、全文は `logs/claude.jsonl` に残る（冒頭 43 行）。読んだものはモデルへの入力になる
  - `tools/record-env.sh:112`: 録画の専用環境は合言葉を毎回乱数にしている
  - `tools/verify-prep.sh:147`: 確認バッチの DB には、確認の間に本人の電話が送った記録が入り、確認の後も消えずに残る
  - `docs/changes/CH21.md` の「なぜ」: 本人は配る形を自分の PC に入れて使っている
  - `.harness2/skills/tsukau/SKILL.md`「しないこと」: 守りは「案内が触らないとしたものに触らない」だけ
  - Q1 の points 2 は「別に立てたサーバと偽データ」と書くが、合言葉・本人の配布版・確認バッチの DB は名指していない
- kind: irreversible
- loss: exported
- 提案: 問いを足す。失わない側が安ければ、P として台帳に載せる。画面の相手は record-env と同じく、乱数の合言葉・専用の compose project・空いている port で立て、作業ツリーの `.env` を使わない。案内（GUIDE）の「触らないもの」に、本人の配布版（`C:\ProgramData\ashiato`）・main の `.env`・確認バッチの DB（`ashiato-verify-<tag>`）を名指す。席の権限で塞げない分は harness2 へ申し送る。
- 処置: fixed D7 仮 —— 失わない側（本人の合言葉と DB を席に渡さない）を選んで台帳に載せた。画面の相手は専用の project・空いている port・乱数の合言葉で `.env` の無い木から立て、席の作業ツリーに `.env` を写さない（D9）。案内の「触らないもの」に本人のサーバ・本人の PC の配布版・確認バッチの DB・main の `.env` を名指す

## R5. 公開の repo に出る「題」を、写す前に機械で確かめるかが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: 依頼「手順書と PR 本文に写すのは件数・題・止まり方だけ」／`.harness2/scripts/verify_batch.sh:216`（`docs/verify` を commit・push）・`:241`（manifest をそのまま PR 本文へ）／題は席（AI）が書き、守りは `.harness2/skills/tsukau/SKILL.md:70`（「題に実データを書かない」）の規範だけ／前の CH21 でも同じ点が spec レビューで挙がっていた（`git show bd3dee549^:…/deep.md` の「レビューから本人の問いへ回したもの」、review/spec.md の R2）
- kind: irreversible
- loss: exported
- 提案: 問いを足す（依頼の「題を写す」は変えず、写す前の確かめ方だけを問う）。例: 本人の利用者名・ホスト名・私設網の名前・`.env` の値・パスの形を含む題は、伏せて件数だけにする。失わない側（機械で確かめる）が安ければ P。
- 処置: fixed D12 仮 —— 題に利用者名・ホスト名・私設網の名前・`.env` の値・絶対パスの形があれば「（伏せた）」にして件数だけ数える。台帳（provisional.jsonl）に載せた

## R6. Sandbox の中のログ（サービス・更新係・インストーラ）を、閉じる前に外へ写すかが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: `.harness2/scripts/tsukau.py:64`（`LOG_<NAME>` は down の前に写す。何を挙げるかは差し込み口が決める）／Sandbox は閉じると中身ごと消える（録画は、書ける写し先 `C:\rec\out` から結果を取り出している。`record-run:303-304, :350-351`）／#329 の根拠は更新係のログだった（premises.md 行 6）。更新係は `checks.jsonl` に書く（`crates/updater/src/lib.rs:22`）
- kind: irreversible
- loss: uncaptured
- 提案: 写すもの（状態の置き場の logs・更新係の `checks.jsonl`・イベントログの抜き書き）を決める問いを足す。写す側が安ければ P として台帳に載せる。写さないと、題を見て根拠を開いたときに、ログが Sandbox ごと消えている。
- 処置: fixed D3 仮 —— 失わない側（取っておく側）を選んで台帳に載せた。片付けで、止める前に Sandbox の中のログ・更新係の `checks.jsonl`・PC の収集アプリのログ・イベントの抜き書きを `$TSUKAU_EVIDENCE/pc-logs/` へ写す（`LOG_` は down より前に写されるので使わない）

## R7. Sandbox の取り合い（本人の Sandbox・手で呼んだ /tsukau・録画）と、閉じ損ねたときの扱いが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠:
  - `.harness2/scripts/record-run:263`: Sandbox は同時に 1 つしか動かない
  - `record-run:317`: 録画は、始める前に別の Sandbox が動いていれば撮らずに止まる
  - `record-run:281-290`・`:348`・`:571-573`: 録画は、自分のシナリオの後と片付けで、動いている Sandbox を全部止める
  - `verify_batch.sh:189` → `:202`: 同じバッチでは、verify-prep（席）の後に録画が走り、録画が撮れなくても warn で続く
- 帰結は 3 つある。
  - (a) 席の片付けが Sandbox を閉じ損ねると、同じバッチの PC の録画が撮れない
  - (b) 手で /tsukau pc を走らせている間に録画が走ると、録画は止まる。録画の片付けは、手の席の Sandbox ごと止める
  - (c) 本人が自分で Sandbox を使っているときに、席が起こせない
- kind: daily
- 提案: 推奨つきの B を足す。Sandbox が使われているとき、席は待つ（上限は何分か）か、飛ばして警告にするか（`upstream_misses` の観点 7）。片付けは自分が起こした Sandbox だけを id で止める（録画の「全部止める」を写さない）ことを、C として design に置く。
- 処置: fixed D3 仮 —— 起こす前に別の Sandbox が動いていれば `UNAVAILABLE=`（待たない）。片付けは自分の id だけを止める

## R8. 確認バッチの準備がどれだけ延び、その間に何が待たされるかが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠:
  - Q3 の points 1: 立てる時間は席の時間に数えず、準備が延びる、とだけ書いてある
  - `.harness2/scripts/tsukau.py:55`: `ready_timeout_s` の既定は 900 秒
  - `.harness2/graph/harness_graph/verify.py:108`: verify_batch.sh を、確認バッチの作業ツリーの錠の中で走らせる
  - `.harness2/graph/harness_graph/effects.py:178-183`: 同じ錠を、記録と fix の撮り直しも取る
  - `effects.py:155-158`: 台本に時間の上限は無い
  - 1 回の準備には、zip の組み立て（R2。何分か測っていない）、Sandbox を起こして入れる時間、席の時間 × 相手の数、片付けが足される。これが 1 日 1〜5 回ある（premises.md 行 8）
- kind: daily
- 提案: Q3 に「立てる時間の上限（`ready_timeout_s`）」と「1 回の準備全体の上限」を足すか、推奨つきの B を別に立てる。R2 の実測の数字を Q3 の points に書く（「準備は延びる」でなく「約 N 分延びる」）。
- 処置: escalated —— 本人への問い Q5 にした（deep.md の「レビューから問いに入れたもの」）。Q5 に立てる時間の上限と、待たされるもの（確認バッチの作業場所の錠）を書き、図を足した。準備全体の上限は相手の数 ×（立てる上限 + 席の時間 + 片付けの上限）で決まる形を spec の段で design に置く

## R9. Sandbox の窓が本人の PC に毎日出て、本人の記録に混ざりうる
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: 録画の実績は、`cmd.exe /c start <.wsb>` で窓を本人のデスクトップに開く形（`record-run:344`）。メモリは 4096 MB（`:300`）／PC の収集アプリは前景の窓を集める（`CLAUDE.md` の構成表）。本人の PC で収集アプリが動いていれば、前景に出た Sandbox の窓が、本人が使ったのではない時間として記録に入る／録画が Sandbox を起こすのは PC のシナリオがある回だけだが、席は PC を変えた回ごとに起こす（Q5 の推奨）
- kind: daily
- 提案: 推奨つきの B を足す。窓を出す起こし方か、窓を出さない起こし方（wsb の CLI で起こして画面に繋がない。できるかは premises で確かめる）か。本人が PC を使っている時間を避けるか。
- 処置: escalated —— 本人への問い Q4 にした（deep.md の「レビューから問いに入れたもの」）。問いに足した（Q4「Sandbox の窓」）。窓が本人のデスクトップに出ること・前景の記録に入りうることを points と代償に書いた

## R10. 席が落ちた・回らなかったときの警告の形と、落ち続けたときの扱いが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: 依頼は「警告として手順書に書く」まで／`tsukau.py` の止まり方は、rc 3（crashed・env_failed・interrupted）、rc 2（差し込み口の誤り。実行のフォルダを作らず、public.json も無い）、片付けの「残りあり」、repo の書き換え（R1）／`tools/verify-prep.sh:14` は `set -euo pipefail` で、verify-prep が落ちると `verify_batch.sh:189-193` が manifest ごと捨てる（警告も手順書に載らない）／`detail` にはログの場所や `UNAVAILABLE=` の理由がそのまま入りうる（tsukau.py:526）
- kind: daily
- 提案: 推奨つきの B を足す。警告に書くもの（止まり方だけか、`detail` も書くか。`detail` は公開の repo に出る）。範囲が空で回さなかった回も一行書くか。同じ止まり方が続いたとき（Sandbox が使えなくなった、など）に、手順書の警告のほかにも知らせるか。rc 2（設定の誤り）を毎回の警告で済ませず、`tsukau.py check` を CI の検査にするかは C で design に置く。
- 処置: escalated —— 本人への問い Q7 にした（deep.md の「レビューから問いに入れたもの」）。問いに足した（Q7「警告の書き方」。決まった文言だけを推奨）。verify-prep 全体を落とさない（席の呼び出しの失敗は捕まえて警告にし、準備の rc を変えない）は Q8 の守るものに入れた。`tsukau.py check` を CI の検査にするのは C として spec の段で design に置く
  spec の段: `tsukau.py check` 相当は自己試験に置き、CI の `rust` の job で走らせる（fixed D13）。verify-prep を止めないのは spec の「席が落ちても時間切れでも…」の Requirement（fixed D11）

## R11. 手で呼ぶ /tsukau とバッチの席が、置き場と立てる物で食い違う・取り合う
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: `.harness2/scripts/tsukau.py:245`（`out_root` の既定は `~/.local/state/harness2/tsukau/<repo の名前>`）。確認バッチは `<repo>-verify` で走る（`verify_batch.sh:108`）ので `ashiato2-verify`、main の checkout から手で呼ぶと `ashiato2`、ほかの worktree からならその名前に分かれる／手とバッチが同時に走ると、画面の相手の port・compose project と、PC の Sandbox（R7）を取り合う。run.sh の port は決め打ち（verify-prep.sh の manifest、5180・18787）／`upstream_misses` の観点 26
- kind: technical
- 提案: 扉を開けたままにする既定（C）として design に置く。`tools/tsukau/tsukau.json` に `out_root` を明示する。立てる物の名前と port は、実行ごとに被らないように決める（record-env の「空いている port」と同じ）。手で呼んだときの相手のコミットを `INFO_` に残す。
- 処置: fixed D1 —— `tsukau.json` に `out_root` を明示し、画面の相手は実行ごとの project と空いている port で立てる（D7）。`INFO_COMMIT=` を残す

## R12. change の第 1 回で問うはずの範囲（完了の判定・壊してはいけないもの）が無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠: `.claude/skills/story-upstream/SKILL.md:54`（第 1 回で範囲を問う。何を変え、何を変えないか、どこまで直したら終わりか）／`docs/changes/CH21.md` の「壊してはいけないもの」「完了の判定」は空／前の CH21 は Q12・Q13 で問うていた（`git show bd3dee549^:…/deep.md`）／壊してはいけないものの候補: verify-prep の既存の成果物（run.sh・manifest の段落・APK・`c01-clock` の数え）と、準備の rc（落ちると manifest ごと消える。`verify_batch.sh:189-193`）
- kind: technical
- 提案: 推奨つきの B を 1 つ足す。完了の判定の例は、確認バッチで 1 回実地に走り、手順書に件数・題か警告が載ること、PC の相手が env_failed で終わらないこと。壊してはいけないものの例は、既存の manifest の段落と準備の rc。
- 処置: fixed deep-questions.json — 問いに足した（Q8「どこまでで終わりか」）

## R13. Q2 の「どれくらい困るか」が確かさ 1 の前提に乗っているのに、premises.md は「Q2 は前提の確かさに依らない」と書いている
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md / deep-questions.json（Q2）
- 根拠: premises.md 冒頭 2〜3 行（「その決定は下の前提の確かさに依らない」）と行 7（Sandbox から私設網に届くか: 確かさ 1・「Q2 の代償」）が食い違う／Q2 の options[0].harm.unknown に同じことが書いてある。届くなら、網につなぐ案で起きるのは「GitHub への問い合わせ」だけでなく、Sandbox の中の席と中のソフトが本人の本物のサーバ（本物の記録）に届くことになる。外に出るもの（exported）の中身が変わる
- kind: premise
- loss: exported
- 提案: 網をつないだ Sandbox を 1 回起こし、私設網の宛先と手元の網に届くかを確かめて（確かさ 4）、Q2 の irreversible と harm に書く。premises.md 冒頭の「依らない」を消す。届くなら、Q2 に「網はつなぐが、手元の網と私設網への宛先は塞ぐ」案を足せるかを確かめる。
- 処置: escalated —— 本人への問い Q2 にした（deep.md の「レビューから問いに入れたもの」）。Sandbox を網ありで起こして確かめた（premises.md 行 6、確かさ 5: 私設網の WSL と Windows に届いた）。premises.md 冒頭の「依らない」を消した。Q2 に「外にはつなぎ、私設網と手元の網を塞ぐ」案を足して推奨にし、塞げるかは確かさ 1 として unknown に書き、塞げなければ切る側に倒すと書いた

## R14. Q2 の不可逆の記述が、外へ出るものを GitHub への問い合わせに限っている
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q2）
- 根拠: PC の収集アプリはブラウザのアドレスバーを集める（`CLAUDE.md` の構成表の `browsers.rs`）。その回の範囲がブラウザの収集なら、席は Sandbox の中のブラウザで外のサイトを開く。それでも Q2 の `irreversible` は「問い合わせ（IP と時刻）が GitHub に残る」、`harm.who` は「GitHub（配布元）」だけ／Windows 自身の外への通信は premises に無い
- kind: irreversible
- loss: exported
- 提案: Q2 の `irreversible` と `harm` に「席がその回の範囲で開く外のサイト（この PC の網の IP で届く）」を足す。Windows 自身の通信は `unknown` に「確かめていない」と書く。`exported` という分類は妥当（つないだ時点で問い合わせが外へ出て戻せない）。足りないのは中身の記述。
- 処置: escalated —— 本人への問い Q2 にした（deep.md の「レビューから問いに入れたもの」）。Q2 の points・irreversible・harm.who に「席が範囲の中で開く外のサイト」を足した

## R15. premises.md 行 5 の「走り出しに動いている Sandbox を全部止める」「録画が席の Sandbox を止める」は実物と違う
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md
- 根拠: `.harness2/scripts/record-run:265`（「本人が自分で起こした Sandbox には触らない —— 始める前に動いていたら撮らずに止まる」）・`:317`（別の Sandbox が動いていれば fail）。全部を止めるのは、自分のシナリオの後（`:348`）と片付け（`:571-573`）の `sandbox_stop`
- kind: premise
- 提案: 行 5 を「始める前に別の Sandbox があれば録画が止まる。終わりと片付けでは動いている Sandbox を全部止める」に直す。帰結（同じバッチなら席の後に録画が走る、閉じ損ねると録画が撮れない、手の席は録画の片付けで止まる）は R7 の問いに書く。
- 処置: fixed premises.md — premises.md の録画の行を実物どおりに直した（始める前に別の Sandbox があれば撮らずに止まる／終わりと片付けで全部止める）

## R16. Q1 の「#329 はこの形で見つかった」は依頼と合わない
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q1）
- 根拠: `docs/changes/CH21.md` の依頼「本人が配る形を入れて普通に使ったら、違和感…がすぐ見つかった（#329）」。見つけたのは本人の PC で、Sandbox ではない。Sandbox に配る zip を入れた実績は無い（R2）
- kind: premise
- 提案: Q1 の points 1 を「#329 は本人が配る形を自分の PC に入れて見つけた。Sandbox に配る zip を入れるのは今回が初めて」に直す。
- 処置: fixed deep-questions.json — Q1 を削った。premises.md の #329 の行を「本人が自分の PC に入れて見つけた」に直した

## R17. Q1 は依頼が決めている。しかも Q5 と食い違う（不要）
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q1・Q5）
- 根拠:
  - `docs/changes/CH21.md` の「どこまで」は「画面（web）も入れられるなら入れる」。Q1 の why も、入れられること（record-env と同じ立て方）を示している。残る幅は「入れられるか」の技術だけで、人間の選択ではない
  - Q1 は「毎回の相手を両方にするか」と問い、得と代償も「相手の数だけ token を使う」と、毎回両方を回す前提で書いている
  - Q5 の推奨は、変えたパスで相手を選ぶ（画面だけの回は画面だけ）
  - 配る zip のサーバは画面を中に持つ（`tools/release/build.sh:73` の `build_web`、`crates/server/build.rs:137-151`）ので、PC の相手の中でも画面に辿り着ける。points はこれに触れていない
- kind: conflict
- 提案: Q1 を削る。「差し込み口は web と pc を持ち、回ごとの相手は Q5 で決める」を design に置く。残すなら Q5 と同じ前提に書き直し、PC の相手の中のサーバも画面を持つことを points に書く。
- 処置: escalated —— 本人への問い Q1 にした（deep.md の「レビューから問いに入れたもの」）。元の Q1 を削った。相手は新しい Q1（範囲と相手の決め方）で回ごとに決める形にし、context に「差し込み口は画面と PC の両方を持つ」「PC の相手の中のサーバも画面を持つ」を書いた

## R18. Q6 は依頼の「写すのは件数・題・止まり方だけ」を問い直している。公開に何も足さない手がかりがある（不要）
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q6）
- 根拠: `docs/changes/CH21.md` の「どこまで」／`.harness2/scripts/tsukau.py:688`（`--label` は request.json に残る）。回の名前（tag）はすでに手順書と PR に出ている／`upstream_misses` の観点 40（後で見つける手がかり）
- kind: conflict
- 提案: Q6 を削る。`--label verify-<tag>` で走らせ、置き場を `tsukau.json` で固定し（R11）、tag から実行のフォルダを引ける形を design に置く（公開に新しく出るものが無い）。残すなら、decide に「依頼の『だけ』を変える問い」と書く。
- 処置: fixed D1 仮 —— `--label verify-<tag>` で走らせ、置き場を `tsukau.json` で固定する。公開に新しく出るものは無い

## R19. Q4 の推奨は依頼の「毎回呼ぶ」を狭めているのに、そう書いていない。比べる「中身」にも抜けがある
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q4）
- 根拠: 依頼「小さい時間と token で毎回呼ぶ」、本人の決定「毎回簡単な動作確認もしてほしい」（`docs/changes/CH21.md`）／Q4 の points 2 は「入った Story・change と、その head の版」で比べるが、確認バッチは毎回 origin/main から作り直す（`verify_batch.sh` の冒頭）。main が進むと、同じ Story の組でも配る物は変わる
- kind: conflict
- 提案: Q4 の points に「依頼は毎回。作り直しで中身が同じときを 1 回と数えるかを決める」と書く。比べる中身に main 側の版（origin/main のコミット）を入れるかを、案の代償に書く。
- 処置: escalated —— 本人への問い Q6 にした（deep.md の「レビューから問いに入れたもの」）。Q6 の decide に「依頼は毎回。同じ中身を 1 回と数えるか」と書き、比べる中身に main のコミットを入れた

## R20. Q2 の「どれくらい困るか」と context に、元の資料に無い言い切りがある
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q2）
- 根拠:
  - harm.link の「本人の家の網の IP」: 「家の」は依頼・premises・コードのどれにも無い
  - context の「本人が実機に入れた配布版も、同じ問い合わせを毎日している」: 更新係の間隔は 1 時間（`crates/updater/src/lib.rs:305`）。本人の配布版で更新の確認が有効のままかは、どこにも無い
  - context の「網を切っても Sandbox の中でつながる」: 録画の実績はサーバを Sandbox の外に置いた形で、中でサーバと収集アプリがつながることは確かめていない（`tools/recording/README.md`）
- kind: premise
- 提案: 「家の」を外し、「この PC の網の IP」にする。間隔は「1 時間ごと（`updater/src/lib.rs:305`）」と書く。本人の配布版の設定と、網を切った Sandbox の中でつながるかは、unknown に「確かめていない」として移す。
- 処置: fixed deep-questions.json — 「家の」を外して「この PC の網の IP」にした。「毎日」の文を消した（premises.md に 1 時間ごとと書いた）。「網を切っても中でつながる」は確かめていないと context に書いた

## R21. 説明の無い言葉
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q3・Q4・Q5）
- 根拠: Q4 points 1 の「関門」、Q4 points 2 の「head の版」、Q5 points 2 の「表題と「価値」の 1 行」（依頼の文書の節の名前）、Q3 decide の「token（重み付き）」
- kind: technical
- 提案: その場で言い換える。例:「関門」→「自動の検査と見直し」、「head の版」→「その PR のいちばん新しいコミット」、「価値」→「依頼の文書の『何のために』の 1 行」、「重み付き」→「読み返しの分を 1/10 に数えた量」。
- 処置: fixed deep-questions.json — 「関門」→「自動の検査と見直し」、「head の版」→「いちばん新しいコミット」、「価値」→「何のためか（依頼の文書の 1 行）」、「重み付き」→「読み返しの分を 1/10 に数えた量」に言い換えた
