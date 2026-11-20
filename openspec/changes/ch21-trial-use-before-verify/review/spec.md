# CH21 spec の独立検査（spec-review、`spec --draft`）

検査したもの: `openspec/changes/ch21-trial-use-before-verify/`（proposal・specs/verify-environment・design・tasks・deep.md・deep-questions.json・premises.md・provisional.jsonl・review/deep.md）と `docs/changes/CH21.md`。
深掘り第 1 回は答え待ちなので、`deep.md` の「（答え待ち・推奨: …）」と `deep-questions.json` の推奨の選択肢を答えとして読んだ。
照らした実物: `.harness2/scripts/tsukau.py`（全文）・`.harness2/scripts/verify_batch.sh`・`.harness2/scripts/verify_checklist.py`・`.harness2/scripts/record-run`・
`tools/verify-prep.sh`・`tools/stack.sh`・`tools/ports.sh`・`tools/record-env.sh`・`tools/check-private.sh`・`tools/pre-commit.sh`・`tools/release/changes.sh`・
`.github/workflows/ci.yml`・正典 `openspec/specs/verify-environment/spec.md`・`openspec/specs/distribution/spec.md`・走っている change（origin/main の ch19 / ch22 / ch23 と `origin/feat/ch22-soak-install-online-ci`）。

## 機械の検査

- `openspec validate ch21-trial-use-before-verify --strict` → `Change 'ch21-trial-use-before-verify' is valid`（rc=0）
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。change なので観点 8 は無い）
- `python3 scripts/check_scenarios.py . ch21-trial-use-before-verify` → rc=1。この change の 32 本の Scenario が全部「担保の無い Scenario」（上流の段なので印がまだ無いのは想定どおり。指摘にしない）
- `python3 scripts/review_triage.py . ch21-trial-use-before-verify` → `triage: OK`（指摘 21 件 / 仮決め 6 件 / 要件へ戻すもの 0 件）
- `python3 scripts/spec_overlap.py candidates . ch21-trial-use-before-verify` → `[]`（0 件）
- 参考: `python3 scripts/provisional.py check …` → D12・D7・D3 が「provisional-review.md に別の席の確かめが無い」で rc=1（グラフが merge gate の前に書くもの。指摘にしない）

---

## R1. Q2 の推奨（外につなぐ）は、正典 distribution の「自動の試験は本物の配布元に問い合わせない」と、それを名指しの 2 つに限る CH22 の書き直しの両方とぶつかる
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q2）/ proposal.md / specs/verify-environment/spec.md
- 根拠:
  - `openspec/specs/distribution/spec.md:2523`「自動の試験は本物の配布元に問い合わせない。配る組み立てを動かす CI は、外への接続を塞いで走らせる」
  - 走っている CH22 の delta（`git show origin/main:openspec/changes/ch22-soak-install-online-ci/specs/distribution/spec.md:253`）は、これを「自動の試験が本物の配布元（`api.github.com`）へ繋ぐのを、次の 2 つに限る: (1) `tls_online_*` (2) install-windows の置く段」「ほかの自動の試験は本物の配布元に問い合わせず」に書き直す
  - CH21 の席は確認バッチの準備が毎回自動で呼び（spec「確認バッチの準備は、範囲のある相手ごとに 1 回…」）、Q2 の推奨では Sandbox の中の配る形の更新係と PC の収集アプリが `api.github.com` に問い合わせる（proposal.md:50、design.md:150）。Task 8.1 の実地も同じ（scope「更新係: 入れて放っておく・更新を知らせる」）
  - CH21 の proposal は distribution を Modified Capabilities に挙げず、Q2 の points・context・選択肢のどれもこの正典の約束に触れていない
  - さらに CH22 は「#329 を配る前に捕まえる」ために、配る形を外につながる状態で 5 分置く段を PR の CI に足す（CH22 proposal の What Changes）。Q2 の「網を切る」の代償「外とつなぐ不具合（#329 の型）は見えず」は、CH22 が入れば大半が CI で拾われることを書いていない
- kind: conflict
- loss: exported
- 提案: Q2 の context に、正典 distribution の約束と CH22 の書き直し・CH22 が #329 の型を CI で拾うことを書き足してから本人に問う。外につなぐ側が選ばれたら、CH21 は distribution の同じ Requirement を MODIFIED にして名指しの例外を 3 つ目として足す（そのときは CH22 の書き直しの上に積むので、観点 7 は前提になり `docs/changes/CH21.md` の requires に CH22 を足す）。網を切る側なら spec の網の Requirement を差し替える。
- 処置: escalated —— まだ渡していない第 1 回の Q2 に、正典 distribution の約束・CH22 の書き直し・CH22 の CI が #329 の型を見ることを書き、推奨を「網を切る」に移した（deep.md の「レビューから問いに入れたもの」）。下書きの仕様は網を切る形（design D6）。外につなぐ案が選ばれたら spec --answers で distribution を MODIFIED にし requires に CH22 を足す

## R2. Q2 の推奨は確かさの足りない前提に乗っており、選択肢の得が言い過ぎている（塞いだ後に外へ届くか・中の管理者が規則を外せること）
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q2）/ premises.md
- 根拠:
  - Q2 の推奨の gain は「本人のサーバには届かない」と言い切る。一方 design.md:80-81 は「規則は Sandbox の中にあるので、Sandbox の中の管理者（席の `pc.sh exec` は `ExistingLogin` で走る。インストーラは管理者で走る）は外せる…機械では止めない」と書く。harm.unknown は「塞げるか」しか挙げていない
  - design.md:82-83 は「RFC 1918 の範囲を塞ぐと、Sandbox の DNS の問い合わせ先（Hyper-V の既定の口）も塞ぎうる。そのときは外の名前が引けず `no-outside` になる（#329 の型は見えない）」とし、確かめていない。Q2 の得（#329 の型が見える）はこの前提に依るが、premises.md にも Q2 にもこの行が無い（premises.md の「塞げる」の行は確かさ 1 で、中身は塞げるかだけ）
  - 前提を確かめる台本（`env-hook.sh probe-network`、Task 7.1）は下流に置かれている。review/deep.md の R3・R13 では、同じ型の未確認を「問いの前に Sandbox を起こして確かさ 5 にする」で処置していた
- kind: premise
- loss: exported
- 提案: 問いを渡す前に、Sandbox を網ありで起こして 7 つの範囲を塞ぎ、私設網に届かないか・`api.github.com` の名前を引いて 443 に届くかを確かめて premises.md に確かさ 5 で書く（`no-outside` なら Q2 の推奨の形そのものを問い直す）。Q2 の推奨の gain を「規則は中の管理者なら外せる。守りは確かめてから渡すことと案内の規範まで」に直し、harm.unknown に DNS の件を足す。
- 処置: escalated —— Q2 の塞ぐ案の gain を直し、unknown に「中の管理者は規則を外せる」「塞いだ後に名前が引けるかは未確認」を足した。推奨（網を切る）はこの前提に乗らない（deep.md の「レビューから問いに入れたもの」）

## R3. 「外にはつなぐ」が Scenario に固定されていない —— `no-outside` を合格にしているので、推奨の得が失われても緑のまま
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md / tasks.md（7.1）
- 根拠:
  - spec.md:224-226「本物の Sandbox で、塞いだ後に私設網に届かない」の THEN は「結果の行は `outside-only` か `no-outside`」。Task 7.1 の検証も `grep -Eq "^result: (outside-only|no-outside)$"`
  - spec.md:210 は `no-outside` を正規の 3 つ目の状態にしている。design D6 は「外に届かなくても止めない」。手順書の行は止まり方しか書かないので、毎回 `no-outside` で回っても本人には見えない
  - Task 7.1 は `no-outside` のとき「塞ぐ範囲を直し、もう一度走らせる」とするが、直し方（既定の口の /32 を抜く。design.md:83）は spec の「7 つの範囲への外向きを塞がせる」（spec.md:208）と食い違い、spec にも Scenario にも無い
- kind: technical
- 提案: 本物の Scenario の THEN を `outside-only` だけにする（`no-outside` なら D6 の反転条件として A で人間へ返す）。DNS のために抜く宛先があるなら、それを spec の Requirement の本文と Scenario に書く（抜いた宛先にこの PC のサービスが届かないことも確かめる）。
- 処置: fixed D6 —— 推奨を網を切る形にしたので、網の確かめと `no-outside` の合格は spec から外した。本物の確かめは「網を切ったまま配る形が入り、片付けで残らない」（Task 7.1）

## R4. 網の確かめは「失敗したら開いたまま」になりうる（確かめられないとき・確かめる宛先が無いとき）
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md（網の Requirement）
- 根拠:
  - Requirement の名前は「確かめられなければ網を切って立て直す」だが、本文（spec.md:209）と Scenario（spec.md:214-222）は「1 つでも届けば」だけを扱う。確かめの `wsb exec` が非 0・出力のファイルが無い・規則を足す `exec` が落ちた、のときの振る舞いが無い
  - design.md:77 は「この PC の私設網の IPv4（`tailscale.exe ip -4`。無ければ飛ばす）」。宛先を全部飛ばせば「どれにも届かない」が自明に真になり `outside-only` で渡る
  - 塞ぐ前に宛先へ届くこと（陽性の対照）を確かめていないので、宛先がたまたま待ち受けていない（premises.md の「手元の網の `:80` は届かなかった（その宛先に何も無いだけかもしれない）」）ときも「塞げた」と読む
- kind: technical
- 提案: spec に「確かめられない（exec が落ちた・答えが読めない・確かめる宛先が 1 つも無い）なら、届いたときと同じく網を切って立て直す」を足し、偽の `wsb` が確かめの exec を失敗させる Scenario を置く。塞ぐ前に同じ宛先へ届くことを確かめ、届かない宛先は「確かめた」に数えない。
- 処置: fixed D6 —— 網を塞いで確かめる道を spec から外した（網を切る）。外につなぐ案が選ばれたときに確かめられなければ切る形を書く、と D6 に置いた

## R5. PC の収集アプリを窓を開く前に起こす順は、premises の実測と合わない（利用者のセッションの外で動く）
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md / design.md（D4）/ tasks.md（6.1）
- 根拠:
  - spec.md:172「配る形の PC の収集アプリを起こし、Sandbox の窓を本人のデスクトップに開いてから」、Scenario spec.md:191「`start`・サーバのインストールの `exec`・PC の収集アプリを起こす `exec`・`connect --id sb-1` の順」。Task 6.1 も「PC の収集アプリの zip を開いて起こす → `wsb connect`」
  - premises.md（確かさ 5）: 窓を開く前は `ExistingLogin` が「ログオン セッションが存在しない」で走らず、`System` では画面の写しが「handle is invalid」。つまり窓の前に起こせるのは `-r System`（利用者のデスクトップの無いセッション）だけ
  - PC の収集アプリは前景の窓・入力を集める（CLAUDE.md の構成表、`platform.rs`）。利用者のセッションの外では前景の窓を取れず、席は「入れて普通に使う」形を見られない。録画の実績（`.harness2/scripts/record-run:306`）も収集アプリは `LogonCommand`（利用者のセッション）で起こしている
- kind: technical
- 提案: 順を「start → 塞ぐ → 確かめる → サーバを入れる → `connect` → PC の収集アプリを `ExistingLogin` で起こす → ready」に直し、Scenario の順と、収集アプリを起こす `exec` の `-r ExistingLogin` を固定する。
- 処置: fixed D4 —— 順を start → サーバのインストール → `connect` → PC の収集アプリを `ExistingLogin` で起こす → ready に直し、Scenario の順と `-r ExistingLogin` を固定した

## R6. 偽の席の台本が、本物の `tsukau.py run` の出力の契約と違う形で書かれ、「写さない」の確かめがいつも緑になる
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md / design.md（D11）/ tasks.md（3.1）
- 根拠:
  - 本物（`.harness2/scripts/tsukau.py:671-672`）の標準出力は 2 行: 1 行目「使ってみた結果: <実行のフォルダの絶対パス>/RESULT.md」、最後の行 `{"v","stop","count","titles","run","cleanup"}`。`run` は `~` を展開した絶対パス（`/home/<利用者>/…`）、`cleanup` は「残りあり（down rc=1。logs/env-down.log）」のような文。`detail` は標準出力に出ない（result.json だけ）
  - spec の偽物は 1 行の `{"v":1,"stop":"done","count":0,"titles":[]}`（spec.md:75）で `run` も 1 行目も無い。「止まり方ごとに…」（spec.md:111）は本物が出さない `detail` に「/home/u/x」を入れて確かめている。「席が終えた…」の「`~/.local/state` の文字列は無い」（spec.md:108）は、本物では展開済みなので字面として現れない
  - 結果として、本物で漏れうる道（`run` の絶対パス・`cleanup` の文・1 行目のパス）を写してしまう実装でも、この Scenario 群は通る
  - 結果の読み元も食い違う: spec.md:94・97 は「`public.json` の件数・題・止まり方」「`public.json` を読めない」、design.md:121 は「最後の行の JSON を読めなければ、`public.json` を探さずに」、Task 3.1 は「最後の行の JSON から」
- kind: technical
- 提案: spec で読み元を「最後の行の JSON」に決め、偽物は本物と同じ 2 行（1 行目に `/home/…` の絶対パス、最後の行に `/home/…` を含む `run` と「残りあり（…logs/env-down.log）」の `cleanup`）を出す形にする。THEN は「段落に `/home/`・`RESULT.md`・`logs/env-down.log` が無い」で見る。`detail` を使う Scenario は外すか、本物の出し方に合わせる。
- 処置: fixed D11 —— 読み元を最後の行の JSON に決め、偽の席の台本を本物と同じ 2 行（`run` の絶対パス・`cleanup` の文つき）にし、THEN を「`/home/`・`RESULT.md`・`.local/state`・`logs/env-down.log` が無い」で見る形にした。`detail` を使う Scenario は外した

## R7. 打ち切りの扱いが本物と合わず、片付けの残りが Scenario に無い（rc 124 と `interrupted` の優先・打ち切った後の Sandbox）
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md / design.md（D8）/ tasks.md（3.1）
- 根拠:
  - 本物は SIGTERM を受けると `interrupted` にし、`env_down`（`down_timeout_s` 300 秒まで）と記録を済ませてから最後の行に `"stop":"interrupted"` を出して rc 3 で終わる（`tsukau.py:647-673`）。`timeout 1920` で切ると rc は 124 だが、出力には `interrupted` の JSON が残る。spec（spec.md:98）は「打ち切ったときは『時間の上限で打ち切った』」とだけで、どちらを優先するかが無い。偽物（spec.md:87「終わらない偽の席の台本」）は何も出さずに死ぬので、この差を見ない
  - `timeout` に `-k` を付けて SIGKILL まで送ると、`env_hook up` は `start_new_session=True`（`tsukau.py:456-458`）で別のプロセスグループにあり、台本の `env_down` も走らないので、Sandbox と compose の project が残る。残った Sandbox は同じバッチの PC の録画を止める（premises.md の録画の行）。打ち切った後の残りを確かめる Scenario が無い
- kind: technical
- 提案: spec に「打ち切りは SIGTERM だけで送り、台本の片付けを待つ（上限は 1,920 秒の中に収める）」「rc 124 なら JSON に依らず『時間の上限で打ち切った』」を書き、偽物は SIGTERM を受けて `interrupted` の JSON を出す形にする。打ち切った後に `<state>/sandbox-id` の Sandbox と `ashiato-tsukau-` の project が残っていれば行に「片付けに残りがある」を出す Scenario を足す。
- 処置: fixed D8 —— `timeout -k 360 1560`（SIGTERM を先に送り、片付けを待つ）にし、rc 124 は JSON に依らず「時間の上限で打ち切った」、打ち切り後に残った Sandbox・project は「片付けに残りがある」とする Scenario を足した

## R8. 「準備は止めない」の抜け道: verify-prep が席の呼び出し全体を時間で区切っていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md（止めないの Requirement）/ tasks.md（4.1）
- 根拠:
  - spec.md:132 は「どんな終わり方をしても（非 0・打ち切り・台本が無い）…rc=0 のまま続けさせ」だが、終わらないときが無い。Task 4.1 は `tools/tsukau/verify-batch.sh "$tag"` を `||` で受けるだけで、外側の上限が無い
  - 内側の上限は呼び出しごと（1,920 秒）と組み立て（2,400 秒）だけで、`git worktree add`・`init.py --worktree`・索引の読み書き・相手の間の処理は区切られない。準備は確認バッチの作業ツリーの錠の中で走り、台本に時間の上限は無い（review/deep.md R8 の根拠 `effects.py:155-158`）。止まれば記録と fix の撮り直しが錠を待ち続ける
  - Q5 の図（deep-questions.json）は「全体にも上限を置く」と書くが、spec と design D8 は「範囲のある相手の数 ×（1,920 秒まで）＋ PC の組み立て（40 分まで）」を延びの見積もりとして書くだけで、上限として持たない
- kind: technical
- 提案: verify-prep 側で `timeout -k <猶予> <全体の上限> tools/tsukau/verify-batch.sh "$tag"` にし、上限（2,400 + 2 × 1,920 + 余り）を spec の本文と、終わらない偽の `verify-batch.sh` で rc=0 と「使ってみる席: 呼べなかった」になる Scenario に書く。
- 処置: fixed D8 —— verify-prep が呼び出し全体を 6,540 秒で打ち切る形を spec に書き、終わらない偽物の Scenario を足した（Task 4.1）

## R9. 題を伏せる形が、確認バッチの commit を止める形（私設 IP・`*.ts.net`・秘密の形）と PR 本文で効く形（`@名前`・改行）を含まない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md（写すものの Requirement）/ design.md（D12）
- 根拠:
  - 手順書は `verify_batch.sh` が `git add docs/verify` → `git commit -q -m …`（`--no-verify` なし）で commit する。確認バッチの作業ツリー（`../ashiato2-verify`）は `core.hooksPath=.githooks` で、`.githooks/pre-commit` → `tools/pre-commit.sh` が `tools/check-private.sh --staged`（私設網の CGNAT・RFC 1918 の IPv4・`*.ts.net`・`Self.DNSName` のラベル）と `tools/check-secrets.sh --staged` を走らせる。`verify_batch.sh` は `set -euo pipefail`（:51）で `trap` による片付けを持たない（`grep -n trap` で 0 件）ので、commit が落ちるとバッチ全体が止まり、stage したままの `docs/verify` が次のバッチを「未コミットの変更がある」（:113）で止める
  - 手順書の段落は `verify_checklist.py:582-586` で note に入り `docs/verify/<tag>.json` として commit される。D12 の伏せる形（spec.md:100、design.md:127-128）は利用者名・`hostname`・`MagicDNSSuffix`・`.env` の値・絶対パスで、IPv4 の形・`*.ts.net` 一般・秘密の形を含まない。PC の相手は網を塞いだ Sandbox の中を触るので、「172.x.x.x に届かない」のような題は起こりうる
  - 題の 80 字・1 行の制限は `candidate` の口だけ（`tsukau.py:307`）。記録は `candidates.jsonl` を読み直して組む（`tsukau.py:574-575`）ので、手で書いた行の改行・長さは通る。改行は manifest の段落を割る（verify_checklist は空行で段落を分ける）
  - manifest は verify PR の本文にそのまま載る（`verify_batch.sh:241` の `cat "$manifest"`）。「@media の指定が効かない」のような題は、GitHub の利用者 `media` に通知が飛ぶ
- kind: technical
- loss: exported
- 提案: D12 の伏せる形を `tools/check-private.sh` の形の上位集合にする（同じ台本を題に当てる形でもよい）。秘密の形は `tools/check-secrets.sh` に通し、当たれば伏せる。題は写す前に改行を空白に・80 字で切り、`@` と `#<数字>` を無害な形（例: 全角の ＠）に替える。spec に「伏せた題を含む段落で確認バッチの commit の検査が通る」「IPv4 の形・`*.ts.net`・改行・`@名前` を含む題」の Scenario を足す。
- 処置: fixed D15 仮 —— commit の検査（check-private・check-secrets）そのものを題に当てて伏せ、改行・80 字・`@`・`#数字` を整える。台帳に載せた。Scenario 2 本を足した

## R10. 推奨の答えと Requirement の本文にあって、Scenario に無いもの（数値を含む）
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠:
  - Q3 の推奨「組めなければ警告にして席を回さない」: 文言「配る zip を組めず、回さなかった」（spec.md:98）を出す Scenario が無い（組み立てが落ちたとき PC の席の台本が呼ばれないこと・画面の相手は回ることも見ていない）
  - Q5 の推奨「立てる上限 15 分」: Requirement（spec.md:7）の `ready_timeout_s` 900 は、Scenario「席の台本が差し込み口を受け付ける」が見る `check` の出力（`tsukau.py:347` は `ok`・`env_hook`・`targets`・`out_root` だけ）に出ないので、どの Scenario も値を見ない。打ち切りの 1,920 秒も、Scenario は 2 秒に縮めた差し込みで見るだけで定数を見ない。組み立ての上限 2,400 秒（Task の定数・design D5）は spec に無い
  - 止まり方の文言のうち `time`・`tokens`・「`public.json` を読めない」「台本が無い」の行（spec.md:96-97）に Scenario が無い
  - 「issue を作らせない」（spec.md:101）、「案内に、Sandbox が外へつながることと、席が範囲の外の外のサイトを開かないことを書かせる」（spec.md:211）は本文だけ
- kind: technical
- 提案: 組み立てが落ちる偽の `build-pc.sh` で PC の行が「配る zip を組めず、回さなかった」になり席の台本が `--target pc` で呼ばれない Scenario、`tsukau.json` の `ready_timeout_s` が 900 であること・台本の頭の定数が 1,920 / 2,400 であることを読む Scenario、`time`・`tokens`・台本が無いときの文言の Scenario、偽の `gh` を PATH に置いて呼ばれないことを見る Scenario、案内の網の 2 文を見る THEN を足す。
- 処置: fixed D5 仮 —— 組めなければ PC の席を呼ばず画面は呼ぶ Scenario、`ready_timeout_s` 900 を読む Scenario、`time`・`tokens`・読めない最後の行の文言、`gh` を呼ばない Scenario を足した。上限 2,400 秒は spec の本文に移した。案内の網の文は網を切る形の Scenario（PC の案内）で見る

## R11. 偽物では落ちない確かめが 2 つある（塞ぐ規則の中身・`stack.sh` が読む `.env`）
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md / tasks.md（5.1・6.1）
- 根拠:
  - 網: 自己試験の偽の `wsb` は「網の確かめの答えを環境変数で返す」（Task 6.1）。Scenario「塞いだ後に私設網に届かず外に届けば、網ありで渡す」は塞ぐ `exec` が確かめより先に呼ばれた順だけを見る（spec.md:217）。塞ぐ規則が 7 つの範囲を持つか（spec.md:208）は、本物で 1 度だけ走る Task 7.1 にしか無く、CI では誰も見ない。後で `.ps1` から範囲を 1 つ落としても自己試験は緑
  - `.env`: `tools/stack.sh:22,28` は自分の木の根へ `cd` して、そこに `.env` があれば `set -a; . ./.env` で読み、渡した `WEB_PASSWORD` などを上書きする。Scenario「画面の相手は、本人の .env を読まずに専用の project で立つ」（spec.md:237-241）は偽の起動の台本が受けた環境変数を記録するだけなので、env_hook が `<state>/tree` でなく `--repo` の `tools/stack.sh`（手で main の checkout から呼べば `.env` がある）を起こしても、偽物には区別できず緑になる
  - CI で走らない: 差し込み口の 3 つの Scenario（「席の台本が差し込み口を受け付ける」「携帯を相手にした呼び出しは、立てる前に断られる」「別の作業ツリーから確かめても、置き場は同じ」）は、design D13・Task 5.1 で「`scripts/tsukau.py` があるときだけ」走る。CI の runner には `.harness2` が無い（`ci.yml` の `harness2` の段の注記）ので、CI では WHEN が走らないまま印だけが数えられ緑になる。CH21.md の完了の判定は「CI の `rust` の job で緑」をこれらの確かめに含めている
- kind: technical
- 提案: 偽の `wsb` が塞ぐ `exec` に渡された台本（`.ps1`）を写し、自己試験がそこから 7 つの範囲（数から組んだ CIDR）を読み出して比べる THEN を足す。起動の偽物は `<state>/tree/tools/stack.sh` の場所に置いて起こされた場所と、その木の根に `.env` が無いことを記録し、THEN で見る。`scripts/tsukau.py` が無いときは飛ばしたことを 1 行出し、CI では harness2 を取れたときに走らせる（`check_scenarios` の段と同じ取り方）か、完了の判定を「手元の merge gate で緑」に直す。
- 処置: fixed D13 —— 塞ぐ規則は外した（網を切る）。起動の偽物を `<state>/tree/tools/stack.sh` の位置に置いて、起こされた場所とその木の `.env` の有無を見る Scenario にした。差し込み口の Scenario は自己試験が `tsukau.json` を読む形にし（CI で走る）、`tsukau.py check` は完了の判定で手元に分けた

## R12. 確認バッチから画面の相手へ渡すサーバと画面の置き場が相対パスで、席の作業ツリーからは見えない
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md（3.1）/ design.md（D7・D9）
- 根拠:
  - Task 3.1「`dist/verify-<tag>/ashiato-server` と `dist/verify-<tag>/web` を `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` で渡す」。env_hook は `cwd=--repo`（使い捨ての作業ツリー。`tsukau.py:456-458`）で走り、`dist/` は git が無視するので使い捨ての作業ツリーには無い（design D9 の反転条件もそう書く）。`stack.sh` はさらに自分の木（`<state>/tree`）へ `cd` する
  - design D7 は「あれば使い、無ければ組む」。相対パスが解けないと、`<state>/tree` の空の `target` から release を組み、画面も `npm ci` から組むことになる。所要時間は premises.md に行が無く（「手で呼んだときは env_hook が組む（数分。15 分の上限に収まる）」は根拠なし）、900 秒を越えれば画面の相手は毎回 `env_failed`
  - 偽の起動の台本では置き場が解けたかを見ないので、自己試験は通る
- kind: technical
- 提案: 渡す値を絶対パスにし（`$(pwd)/dist/verify-<tag>/…`）、env_hook は渡された置き場が無ければ組まずに `UNAVAILABLE=` にする（確認バッチの道で組まない）と spec に書く。Scenario で、渡した置き場が `SERVER_BIN`・`WEB_DIST` として絶対パスのまま起動に届くことを見る。手で呼ぶときに組む時間は Task 1.2 と同じく測って premises.md に書く。
- 処置: fixed D7 仮 —— 絶対パス（`$PWD/dist/verify-<tag>/…`）で渡し、渡された置き場が無ければ組まずに `UNAVAILABLE=` とし、届いた値を見る Scenario を足した

## R13. 判定の道は「試験だけを変えたものは入れない」と言いながら、パスの頭で試験の置き場を入れてしまう。価値の節の読み方も実物の文書で崩れる
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md（範囲の Requirement）/ design.md（D2）
- 根拠:
  - spec.md:28-29 は、`web/src/` → 画面、`crates/` の下 → PC、「文書・道具・試験だけを変えたものは、どの相手にも入れない」。`web/src/__tests__/`（CLAUDE.md の構成表の画面の試験の置き場）と `crates/server/src/*_tests.rs`・`crates/*/tests/` は頭の規則で画面・PC に入り、本文の「試験だけ」と食い違う。Scenario（spec.md:51）は `web/e2e/` しか置かない
  - 配る zip の作り方を変える `tools/release/` は「道具」として外れるが、PC の相手が入れる物そのものが変わる（`.github/workflows/ci.yml` の `windows` の判定は `^tools/release/` を Windows の通しに入れている）
  - 「何のためか」= 価値の節の最初の段落の 1 行目、「（」で始まれば雛形（spec.md:31）。実物では `docs/changes/CH05.md` の価値の節が「（第 1 回の深掘りの推奨の答えで先に書き…」で始まり雛形と誤判定され、`CH12.md`・`CH13.md` は「> 第 1 回の深掘りの答え…」の引用で始まり、目的でない文が範囲に入る
- kind: technical
- 提案: 試験の置き場（`web/src/__tests__/`・`crates/**/tests/`・`*_tests.rs`）を除く規則と、`tools/release/` を PC に入れるかを spec の表に書き、それぞれの Scenario を足す。雛形の判定は `hx change` の雛形の字面そのもので見、引用の行（`>`）や注記の括弧で始まる段落は飛ばして次の段落を読む。
- 処置: fixed D2 —— 試験の置き場を除く規則と `tools/release/` を PC に入れることを spec に書き、価値の節は `>` と `（` で始まる段落を飛ばす形にした。Scenario を直した

## R14. D14 は Q8 の推奨「確認バッチで 1 回実地に走り」を、確認バッチを通らない直の呼び出しに読み替えている
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md（D14）/ tasks.md（8.1）/ docs/changes/CH21.md
- 根拠:
  - Q8 の推奨の points:「推奨の完了: 確認バッチで 1 回実地に走り、手順書に件数と題か警告が載る。PC の相手が立てられずに終わらない」
  - design D14 は「CH21 だけの確認バッチでは席が回らない」ので、Task 8.1（`tools/tsukau/real-run.sh`）で `scripts/tsukau.py run` を直に呼ぶことで満たすとする。この道は `verify-prep.sh` → `verify-batch.sh` → 使い捨ての作業ツリーと `init.py --worktree` → `dist/verify-<tag>/tsukau-pc` への組み立て → `ASHIATO_TSUKAU_*` の受け渡し → 段落の組み立て、を本物で 1 度も通らない（R12 の相対パスはまさにこの道でしか出ない）
  - Q8 の問いには「CH21 だけのバッチでは席が回らない」ことが書かれておらず、本人は推奨を選べば確認バッチで回ると読む
  - Task 8.1 の終了条件は `stop` が `done`・`time`・`tokens` で `cleanup` が済み。席が製品に届かずに「自分で終えた」でも通る
- kind: conflict
- 提案: Q8 の points に「CH21 だけの確認バッチでは席は回らない（『回さなかった』の 2 行）。実地は○○で代える」を書いてから問う。代える道は、`real-run.sh` が画面と PC を変える偽の commit を merge した使い捨ての確認バッチの木を作り、`tools/tsukau/verify-batch.sh <tag>` を本物で通す形にする。終了条件に `result.json` の `journal` ≥ 1 を足す。
- 処置: escalated —— まだ渡していない第 1 回の Q8 の推奨と context に「CH21 だけの確認バッチでは席は回らない。実地は同じ台本を使い捨ての確認バッチの木で通す」を書いた（deep.md の「レビューから問いに入れたもの」）。design D14（仮）・Task 8.1 を同じ道を通す形に直し、終わりの条件に `journal` ≥ 1 を足した

## R15. 段落は手順書の note に入るとき 1 行に潰れ、「相手ごとの行が読める」が満たされない
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md / docs/changes/CH21.md（完了の判定）
- 根拠:
  - `verify_checklist.py:583-586` は manifest を空行で段落に分け、段落の中の改行を空白に替えて 1 つの note にする
  - spec.md:93-94 は「最後の段落として 1 段落だけ」「相手ごとに 1 行」。手順書では「画面: …」「PC: …」と題の一覧が 1 行に続けて並ぶ
  - CH21.md の完了の判定「手順書の冒頭に『使ってみる席』の段落が載り、相手ごとの行が読める」と、tasks.md の人間の確認待ちが、この見え方を確かめさせる
- kind: technical
- 提案: 段落を相手ごとに分ける（頭の 1 段落 + 相手ごとの段落）か、1 段落のまま読める区切り（「／」など）で組むかを spec に書き、`verify_checklist.py` を通した note の字面で見る Scenario を置く。
- 処置: fixed D11 —— 頭の 1 段落と相手ごとの 1 段落に分けた（手順書の note が相手ごとに分かれる）

## R16. 仮の印の付いた D1・D5・D9 が台帳（provisional.jsonl）に無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/provisional.jsonl / design.md
- 根拠: `grep -n "（仮）" design.md` → D1・D3・D5・D7・D9・D12。`python3 scripts/provisional.py list` → D12・D7・D3 だけ。review/deep.md の処置は R1「fixed D9 仮」、R18「fixed D1 仮」。台帳に無い D<n> は別の席の確かめ（`provisional_review`）にも確認バッチの一覧にも出ない。一方、合意の引き継ぎ（観点 6）は design の「（仮）」の印で判定するので、D1・D5・D9 の変更は確かめられないまま引き継がれうる。D5 は本人に問う Q3 の答えそのもので、仮にする理由も書かれていない
- kind: technical
- 提案: D1・D9 を `provisional.py add` で台帳に載せる（`--source review/deep.md#R18` / `#R1`）。D5 は Q3 の答えに依る部分（手元で組む）と仮の部分（置き場・40 分の上限）を分け、仮の部分だけを台帳に載せるか、「（仮）」の印を外す。
- 処置: fixed D9 仮 —— D1・D5・D9・D14・D15 を台帳に載せた（D1 は review/deep.md#R18、D9 は review/deep.md#R1、D5 は review/spec.md#R10、D14 は review/spec.md#R14、D15 は review/spec.md#R9 を出所に）

## R17. design だけにある観測できる振る舞い
- 成果物: openspec/changes/ch21-trial-use-before-verify/design.md / specs/verify-environment/spec.md
- 根拠:
  - D5: 組み立ての上限 40 分（2,400 秒）と、手で呼ぶときに env_hook が配る zip を探す置き場 `dist/tsukau-pc/<HEAD のコミット>/`・`ASHIATO_TSUKAU_PC_ZIPS`（手で `/tsukau` を呼ぶ人に見える約束）
  - D6: 「私設網の宛先の IP は `INFO_` にも案内にも書かない」（実行のフォルダに何が残るか）
  - D9: 「harness2 を繋げなかったら（`init.py` が非 0）、その相手の行は『席を呼べなかった』」
  - D11: 手元の出力を `dist/verify-<tag>/tsukau.log` に書く・標準出力に段落だけ
  - D12: 伏せる絶対パスの形に `/Users/` があり、`.env` の値は 8 字以上のものだけ。spec.md:100 は `/Users/` を持たず、字数の条件も無い（spec と design が食い違う）
- kind: technical
- 提案: これらを spec の Requirement の本文と Scenario に移す（D12 は spec と design の字面を揃える）。`openspec archive` は main specs しか更新しないので、design に残すと正典から落ちる。
- 処置: fixed D5 —— 組み立ての上限・手で呼ぶときの zip の置き場と環境変数・harness2 を繋げなかったときの文言・`tsukau.log`・伏せる形（`/Users/`・8 字以上）を spec の本文に移した。私設網の IP を書かない件は網を切る形で消えた

## R18. Requirement の数が、proposal・CH21.md と spec で違う
- 成果物: openspec/changes/ch21-trial-use-before-verify/proposal.md / docs/changes/CH21.md
- 根拠: `grep -c "^### Requirement" specs/verify-environment/spec.md` → 9。proposal.md:39「Requirement を 8 つ ADDED（…PC の相手の立て方と網・…）」は PC と網を 1 つに数えている。CH21.md の完了の判定「archive で、正典 verify-environment に 8 つの Requirement が足される」は、このままでは archive で偽になる
- kind: technical
- 提案: proposal と CH21.md を 9 に直す（PC の相手の立て方と網を別に数える）。
- 処置: fixed proposal.md —— proposal と CH21.md を 9 つに直した

## R19. 主張を束ねた Scenario（片方だけ通っても緑になる）
- 成果物: openspec/changes/ch21-trial-use-before-verify/specs/verify-environment/spec.md
- 根拠:
  - 「席は使い捨ての作業ツリーで走り、確認バッチの作業ツリーは書き換わらない」（spec.md:80-84）: 違うパス・`.harness2` がある・`.env` が無い・`git status` が同じ・パスが消えた・`git worktree list` に無い、の 6 つ
  - 「画面の相手は、本人の .env を読まずに専用の project で立つ」（spec.md:237-241）: project 名・合言葉・案内に合言葉が無い・URL と「触らない」・`INFO_COMMIT=` の 5 つ
  - 「本物の Sandbox で、塞いだ後に私設網に届かない」（spec.md:224-226）: rc・宛先ごとの届かない・結果の行・後に動いている Sandbox が無い
  - 「立てるときは…」（spec.md:188-192）: 呼ぶ順と案内の中身
- kind: technical
- 提案: 少なくとも「`.env` を写さない」「確認バッチの作業ツリーを書き換えない」「使い捨ての作業ツリーを消す」と、「専用の project」「本人の合言葉を使わない」「案内の中身」を別の Scenario に分ける。
- 処置: fixed D9 仮 —— 作業ツリーの Scenario を「別で harness2 が繋がり .env が無い」と「書き換えても変わらず消える」に、画面の相手の Scenario を「.env の無い木」「専用の project と合言葉」「置き場」「案内」に、PC の立ての Scenario を「網を切る」「順」「案内」に分けた

## R20. CI の `rust` の job に足す位置が、走っている CH22 の足す位置と同じで、文字の上で衝突する
- 成果物: openspec/changes/ch21-trial-use-before-verify/tasks.md（8.2）/ proposal.md（Impact）
- 根拠: `git diff origin/main...origin/feat/ch22-soak-install-online-ci -- .github/workflows/ci.yml` は `- run: ./tools/release/update-apply-install.sh --self-test` の直後に `- run: ./tools/release/soak-install.sh --self-test` を足す。Task 8.2 も「`update-apply-install.sh --self-test` の行の近く」に 2 行を足す。同じ位置への挿入は git の merge で衝突し、確認バッチに同時に入ると衝突の解き直し（`conflict_fix`）になる。proposal.md:52 は「どちらも行を足すだけで、同じ行は変えない」として重なりを無しとしている
- kind: technical
- 提案: Task 8.2 の位置を CH22 と別の行の後（例: `./tools/check-boundaries.sh` の後の別の塊）に名指しで決め、proposal の重なりの記述を直す。
- 処置: fixed D13 —— 足す位置を `check-private.sh --self-test` の後にし、proposal の重なりの記述（CH22・CH23）を直した

---

## 観点ごとのまとめ

- 観点 1（deep の決定が写っているか）: Q1・Q6・Q7 は写っている。Q2 は R1・R2・R3・R4、Q3 は R10、Q4 は R5、Q5 は R8・R10、Q8 は R14 で欠けか食い違いがある。要件へ戻すものは無い（change）
- 観点 2（Scenario が検証可能か）: R6・R7・R10・R11・R19
- 観点 3（置き場）: R17。proposal の Capabilities と specs のディレクトリは一致する（`verify-environment`）
- 観点 4（tasks の検証）: どの `- [ ]` もコマンドと終了条件を持つ。R3（7.1 の合格の幅）・R12（3.1 の渡し方）・R14（8.1 の終了条件）・R20（8.2 の位置）
- 観点 5（change の文書）: `docs/changes/CH21.md` の完了の判定は書かれている。R14・R15・R18 で deep / spec と食い違う
- 観点 6: 該当なし（`spec --draft`。答えはまだ無い）
- 確かめて指摘にしなかったもの: 席の台本の契約のうち、rc の表（0 = done/time/tokens・2 = 引数と設定の誤りで実行のフォルダを作らない・3 = crashed/env_failed/interrupted）、env_hook に渡る環境変数（呼び手の環境 + `TSUKAU_RUN`・`TSUKAU_EVIDENCE`・`TSUKAU_TARGET`・`TSUKAU_SCOPE`）、cwd が `--repo` であること、`LOG_` は down の前に写すので Sandbox の中のログは down で取る（design D3）、`--target android` は argparse を通って差し込み口の `targets` で rc 2 になること、は design・spec の想定と一致した
- 観点 5（change）で確かめた範囲: 「壊してはいけないもの」の 5 つは、準備の成果物と段落（「席の段落は、既存の段落の後に足すだけ」）・準備の rc（「席の呼び出しが落ちても…」）・作業ツリー（「席は使い捨ての作業ツリーで…」）・本人の合言葉と DB と Sandbox（画面の相手の Scenario・「片付けは…自分の Sandbox だけを止める」）が Scenario を持つ。「録画: 席は録画より先に終わる」は呼ぶ位置（verify-prep の中）で決まり、Scenario は無い（`verify_batch.sh` の並びに依るので指摘にしない）。依頼（逐語）の範囲は越えていない（同じ中身の写し・伏せる・作業ツリーの分離は deep の問いか review/deep.md の処置に依る）
- 観点 3 で確かめた範囲: spec にある台本のパス・環境変数・`wsb` と `docker compose` の命令の名前は、この change が作る道具の外から見える口（呼び方）として扱い、実装の名前としては指摘にしなかった

## 観点 7: 該当なし（spec_overlap.py candidates が 0 件）

機械の候補の外で見たこと（判定の行ではない）: CH19（`emulator-startup`。`tools/android-emulator.sh` など）は capability も触るパスも重ならない。
CH23（`dependency-vulnerability-scan`）は capability は重ならないが、`docs/testing.md` を触る（CH23 proposal の 25・40 行の「毎晩の行に書き足す」）。CH21 の Task 8.3 も同じファイルに 1 節を足すので、同じファイルの別の場所への書き足しになる（proposal.md:53「CH23 とは重ならない」は不正確）。
CH22 とは delta の Requirement は重ならないが、CH21 の振る舞い（席が配る形を外につないで本物の配布元に問い合わせる）が、CH22 が書き直す distribution の Requirement に当たる（R1）。
CH21 がその Requirement を MODIFIED にすると決めたら、この節を CH22 の前提として判定し直し、requires に CH22 を足す。`.github/workflows/ci.yml` の文字の衝突は R20。
