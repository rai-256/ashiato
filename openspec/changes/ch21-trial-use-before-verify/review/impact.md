# impact-review — ch21-trial-use-before-verify

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 medium, IMP-05 high, IMP-06 low, IMP-07 medium, IMP-08 medium, IMP-09 medium

## R1 — IMP-05 の「いま進めて安全」の材料が観察であって手当てではない。既存の agent との差も書いていない
- 対象: impact.findings[IMP-05].if_accepted / .options[0] / .severity
- 種類: 閉じ方
- 指摘: `if_accepted.recheck` は「実地 1 回の journal に外のパスを読んだ跡が無いか」を見るだけである。`unknowns` 自身が「数回読まなかったとしても以後も読まないとは言えない」と書いており、これは進めて安全の根拠にならない。residual は「呼ぶかどうかを切り替える口は仕様に無い」まま。実物では、席の起動（`.harness2/scripts/tsukau.py:393-397`）は `--permission-mode auto` と Bash の kill フックだけで、読み出しを作業ツリーに限る指定は見当たらない。「案内だけで止めている」は E10 から言える。一方で、同じ host の WSL で auto の claude を走らせる口は、harness2 の他の agent にも既にあるはずである（私は確かめていない）。分析は、CH21 で新しく増える分（確認バッチごとの自動の呼び出し・Sandbox のログを読む選別の席）と既存の分を分けていない。high の根拠は「読まれた中身は取り戻せない・本人が journal を開かないと気づけない」で足りるので、high 自体は妥当（basis `guess` でも `unknowns` が足りている）。足りないのは、いま進めてよい理由として何を挙げているかである。
- 直し方の案: `if_accepted` に実在する手当て（既定で呼ばない切り替え、または .env の無い別の利用者）を置くか、「手当ては無く、本人が merge 前に決める」と書く。あわせて「新しく増えるのはどこか」を 1 行足す。
- 処置: fixed: IMP-05 の if_accepted の residual に「読み出しを止める手当ては無い。journal を見るのは観察で根拠ではない」、options[0] に「手当ては無い・観察」と書き、text に CH21 で新しく増える分（確認バッチごとの自動の呼び出し・選別の席）と、他の席との差を確かめていないことを足した。high のまま

## R2 — IMP-08 の未確認の片方は、実物で答えが出ている。「Story の PR」の呼び方も実態とずれる
- 対象: impact.findings[IMP-08].unknowns[1] / .text、dependencies[D12].assumes、lost[2].after
- 種類: 根拠
- 指摘: 「統合の CI の直し（ci_fix）の後に審査をやり直しているか」は未確認とされているが、`.harness2/scripts/verify_ci_route.py` は cherry-pick して push し、`scripts/ci_wait.py` で PR の CI を待つだけで、審査の席・merge_gate を呼ぶ語が 1 つも無い（冒頭 1-20 行と grep で確認）。処置は「ci_fix と同じ道」（harness2-request.json の what）なので、先例では審査はやり直されず、載せた直しの検査は CI だけである。また `verify_batch.sh:259` のとおり Story の PR は本人の merge 後に archive で**閉じられ**、main に入るのは verify PR である（D12 の assumes「本人が merge する Story の PR」は実態とずれる）。本人が直しの差分を見られる場所は verify PR の diff で、手順書は 1 行しか知らせない。medium は妥当（CI が緑・差分は PR に残る）。ただし「審査をやり直すか未確認」と書くと、先例が無いことを隠す。
- 直し方の案: unknowns[1] を「先例の ci_fix は CI だけで審査をやり直さない（verify_ci_route.py）」に直し、載せ先を verify PR の diff と書く。
- 処置: fixed: IMP-08 の text と unknowns を「先例の ci_fix は CI だけで審査をやり直さない（E14）・差分は verify PR の diff で見る」に直し、D12 の who / assumes / affects と lost[2] を verify PR と Story のブランチの言い方に直した

## R3 — harness2 の処置（まだ無い）に依る効果を、起きるものとして書いている
- 対象: situations[normal].can、lost[2].after、rules[R10・R11]（basis `derived`）、walkthroughs[W3].cases[0]（outcome `ok`）、sections.makes.tldr / sections.uses.tldr / before_after[4]
- 種類: 言い切り
- 指摘: `.harness2/scripts/` に `verify_tsukau.py` は無い（`ls` で確認）。「直したものの 1 行と本人への問いも載る」「3 件・1 回まで」「本人が見る前に直り、確かめ直しの結果も段落に載る」は、harness2-request.json の文面にしか無い。R11 の「4 件目からは本人に聞く」は依頼の scope の文をそのまま写したもので、basis は `derived` でなく `unconfirmed`/`guess` が正しい。`sections.makes.tldr` と `uses.tldr` は「直すか本人に聞くか依頼にする」「直した結果と判断の要る問いだけがある」と、条件（処置が入った後・処置が約束どおり動く）を落として言い切っている（U1 の steps には条件が付いている）。R12 で CH21 は処置が入るまで main に入らないが、入るのは「契約が動く」までで、選別の質・3 件の上限が約束どおりかは real-run の 2 候補では確かめられない（IMP-09 自身が言う）。
- 直し方の案: 処置に依る文は「（依頼。harness2 の処置が入った後）」を付け、R10・R11 の basis を `unconfirmed` に直す。tldr 2 本にも条件を足す。
- 処置: fixed: R10・R11 の basis を unconfirmed にし、situations.normal・lost[2]・before_after・W3 の ok の結果・sections の makes / uses の tldr に「依頼。harness2 の処置が入った後」の条件を付けた

## R4 — IMP-04 の「段落で気づく」は、決まった文言の規則と合わない
- 対象: impact.findings[IMP-04].options[0]・.harm.who・scenarios[S1]
- 種類: 言い切り
- 指摘: 選択肢 1 は「段落の『相手を立てられなかった』で気づく」、harm は「PC の段落と録画の欄が回らなかったことを示す」とある。しかし spec の「止まり方ごとに決まった文言」（specs/verify-environment/spec.md:166-168・R13 の legit）では、別の Sandbox が動いていたことも、Sandbox の起動の失敗も、同じ `env_failed`＝「相手を立てられなかった」に落ち、理由は手順書に出ない。残った Sandbox か、IMP-02（配る形が入らない）か、段落からは区別できず、手元の RESULT.md を開かないと分からない。理由が見えるのは、PC の録画が同じ回に撮られたときの record-run の文言「別の Sandbox が動いている」（`.harness2/scripts/record-run:317`）だけ。IMP-02 が起きていれば、残りの Sandbox を IMP-02 と取り違える。medium のままでよいが、「本人が自分で気づける」前提は成り立たない。
- 直し方の案: 選択肢 1 と harm の「気づく」を「段落からは原因が分からず、結果の手元の記録か録画の欄で分かる」に直し、`undefined` に「残りの原因を手順書に出す規則が無い」を足す。
- 処置: fixed: IMP-04 の options[0]・harm.who を「段落からは原因が分からず、手元の結果か録画の欄で分かる」に直し、undefined に「残りの原因を手順書に出す規則が無い」を足した

## R5 — IMP-03 は「録画が錠で待つ」とは言えない。錠を待つのは別の台本
- 対象: impact.findings[IMP-03].text・derived、lost[0].after、dependencies[D2]
- 種類: 根拠
- 指摘: 錠は `verify.py:108` で `verify_batch.sh` 全体を包む。録画（`record-run verify`）は同じ `verify_batch.sh` の中の準備の次の工程（`verify_batch.sh:196-`）で、錠を待つのでなく、準備が延びた分だけ**遅れて始まる**。錠を待つのは、別の経路の記録・fix・ci_fix・conflict_fix の後の作り直し（`verify.py:85`・`effects.py:181` の docstring）である。「録画と fix の撮り直しも待つ」と同列に書くと、延びる長さの数え方がずれる。7,140 秒は準備 1 回の上限で、処置が直して作り直すたびに準備（配る zip の 2,400 秒を含む）が走り直す回数の上限は CH21 の側に無い（harness2 次第）。unknowns は組み立てと処置の時間しか挙げていない。
- 直し方の案: 「録画は準備の後に続くので遅れ、錠で待つのは記録・fix・ci_fix・conflict_fix」と書き分け、作り直しの回数の上限が harness2 側の約束であることを unknowns に足す。
- 処置: fixed: IMP-03 の text と lost[0] を「記録・fix・ci_fix・conflict_fix は錠を待ち、録画は準備の後に遅れて始まる」に書き分け、unknowns に作り直しの回数の上限が harness2 側にあることを足した

## R6 — 依存の見落とし: 処置が作り直すたびに、準備（配る zip の組み立てを含む）が錠の中で走り直す
- 対象: impact.dependencies（T1 の読む側）、lost[0]
- 種類: 依存の見落とし
- 指摘: 依頼（harness2-request.json の what）は「直したものは Story のブランチへ載せてバッチを作り直し、準備に TSUKAU_RECHECK を渡す」とする。作り直しは `verify.py:batch`（`verify.py:108`）の同じ経路で `verify_batch.sh` 全体を錠の中でもう一度走らせるので、verify-prep の組み立て（サーバ・画面・配る zip の最大 2,400 秒）はやり直しになる。D10 の「同じ中身は写す」は、直した後の木が変わるので効かない（design D10 が自分で書いている）。T1 の読む側として「作り直しのたびに準備が走り直る」が挙がっていない。IMP-03 は延びることだけを言い、作り直しの回数の上限が CH21 の外（harness2 の約束）にあることは unknowns に無い。なお、持ち越し（処置の済んでいない候補）が処置の側の記憶に置かれる点（依頼の what）も、その記憶が落ちたら何が戻るかを依存に挙げていない。私は記憶の置き場の実物を確かめていない。
- 直し方の案: D2 の隣に「作り直しのたびに準備が走り直る（回数の上限は harness2）」を依存として足し、holds は `unknown` にする。
- 処置: fixed: 依存 D15（処置の作り直しのたびに準備が走り直る・持ち越しの記憶。holds unknown、IMP-03 へ）を足した

## R7 — IMP-09 の閉じ方が、本人の手作業に頼っている
- 対象: impact.findings[IMP-09].verify・.options[0]
- 種類: 閉じ方
- 指摘: medium は妥当（誤りは merge 前に見え、直しは CI を通る）。ただし options[0] の「誤検知も手元の結果には残る」と verify の「本人が手元の結果を見る」は、本人が開かなければ気づけない前提で、誤検知を本物と取り違えたときに見直す口が手順書に無い（`undefined` が認めている）。受け入れの理由としては、予定を書いただけである。
- 直し方の案: 選択肢 1 に「誤検知の見直しは本人が手元の結果を開いたときだけ」と書き、何回の確認バッチで見直すかを verify に決める。
- 処置: fixed: IMP-09 の options[0] に「誤検知の見直しは本人が手元の結果を開いたときだけ」と書き、verify を「処置が入った後の最初の 3 回の確認バッチで本人が開いて見る」に決めた

## R8 — IMP-05 の text に、辿れる先が本人の携帯の記録を含みうる点が出ていない
- 対象: impact.findings[IMP-05].text・.harm.link
- 種類: どれくらい困るか
- 指摘: design.md:17 のとおり、確認バッチの DB には確認の間に本人の携帯が送った記録が入りうる。IMP-05 の text と harm.link は「合言葉と DB」とだけ書き、DB の中身が私的な実データでありうることは memory[1] の why の「中身は偽データ」とも読み違えやすい（席が見る画面の中身は偽データだが、辿れる DB は偽でない）。重大度は変わらない。
- 直し方の案: IMP-05 の text と harm.link に「確認バッチの DB は本人の携帯の記録を含みうる（偽データでない）」を足す。
- 処置: fixed: IMP-05 の text と harm.link、memory[1] に「確認バッチの DB は本人の携帯の記録を含みうる（偽データでない）」を足した

