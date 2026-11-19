# CH21 —— 確認バッチの前に Claude が配る形を入れて普通に使ってみる

change: `openspec/changes/ch21-trial-use-before-verify/`

第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた（2026-10-20 の流れ）。答えが推奨と違えば `spec --answers` が写し直す。

## 深掘りで本人が決めること（A）

- Q7 見たもの・候補の置き場と手順書に載せるもの（loss: exported）—— 推奨: 手元の `C:\dev\ashiato2-trial-use\<tag>\` に置き、手順書には件数・題 1 行（60 文字まで・私的な値の形は伏せる）・巡った目的の札・所要・終わり方・報告の場所だけ
- Q8 候補を issue にするか（loss: exported）—— 推奨: 人間が選ぶ。席は issue を作らず、各候補に /iwakan に貼れる違和感メモを添える

A は推奨を置いてあるが、本人は答えを省略できない（触らなければ `(未回答)`）。

## 仮決め（B）

深掘りの推奨（Q1〜Q6・Q9〜Q13）に従った仮と、design の（仮）:
- D1（仮）. capability は新しい `trial-use` にする
- D2（仮）. 配る形は、準備の中で手元の Windows で組む（Q2。倒し先は CI の artifact）
- D3（仮）. Claude は外（WSL）で動き、Sandbox の中の Edge と「頼みの箱」の 2 つの口で触る（Q3。倒し先は中に Node と Playwright）
- D4（仮）. Claude は席の台本が `claude -p` で起こす（グラフの外。Q12）
- D6（仮）. Claude に渡す材料と指示（Q4・Q5）
- D7（仮）. 候補の形と機械の確かめ（Q6）
- D8（仮）. 既知の照合は、使い終えた後に別の短い呼び出しで行う
- D9（仮）. 出所の比べ（Q11。zip のバイト列でなく、組み立てに入る木の hash）
- D10（仮）. 時間の上限は 45 分（Q9。組み立てには別に 60 分）
- D12（仮）. 置き場と手順書の段落（Q7 の推奨に、0 件の見分けのための札・所要・終わり方を足した）

## capability

新しい `trial-use`（ADDED 9 Requirement・40 Scenario。39 本は偽の Windows・Claude・gh の自己検査、1 本は確認バッチの「人間の確認待ち」（`> 物理: device`））。
既存の `verify-environment` には積まない（D1（仮））。前倒しは無い。

## tasks

11 Task（うち Task 1「確かめていない前提の実測」と Task 11「本物の Sandbox と Claude で 1 回通す」は手元の Windows が要る）。

## レビュー

- review/deep.md（questions の段）: 11 件 —— escalated 3（R1・R2・R10。Q7・Q8 として本人に問う）/ fixed 8
- review/spec.md: 25 件 —— escalated 2（R2・R21。Q7・Q13 の context に書いて本人に見せる）/ fixed 23（うち仮 2: R1 → D12、R4 → D10）
