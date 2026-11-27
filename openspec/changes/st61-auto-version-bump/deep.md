# 深掘り（第 1 回）

> 推奨の答えで先に仕様と GO 資料を書いた（2026-10-20 の流れ）。答えは `deep-answers-1.txt`（2026-11-22）。
> 4 問とも本人は推奨に触れず（`(推奨のまま: …)`。本人の選択とは区別する）、補足も無い。仕様は推奨で書いたままで、直すものは無い。

### Q1. 番号を付ける commit

- 問い: main のどの commit に番号を付けるか
- 答え: (推奨のまま: CI が通り、前の番号から製品が変わった commit に付ける)

### Q2. 携帯の上書きの番号（versionCode）

- 問い: 携帯の上書きの番号の式を、今変えるか
- 答え: (推奨のまま: 桁を広げる（左×10,000,000 + 真ん中×10,000 + 右）)

### Q3. 真ん中の数字を上げる時

- 問い: 真ん中の数字を、いつ上げるか
- 答え: (推奨のまま: 確認バッチが入ったら上げる)

### Q4. 番号を持たない組み立て

- 問い: 番号を持たない組み立てに、どの版番号を書くか
- 答え: (推奨のまま: 本物に使わない 0.0.x（1 以上）を名乗る)

## 高リスクの印（QR）

- 答え: 高リスクのままにする（本人の選択。`agreement.json` の `risk.decision: kept`）

## 仕様への合意（QA）

- 答え: 合意する（`agreement.json` が持つ。IMP-01〜07 は medium / low で、書いた残るもののまま受け入れた）

## 要件へ戻すもの（第 1 回の答えで record が戻す。spec の段が置いた。review/spec.md R11）

FR-134 の未決（NEEDS CLARIFICATION と 2026-11-21 の補足の 3 つの「決める」）は、この回の Q1〜Q3 が閉じる。推奨のままの答えでも閉じる（★ 訂正を入れる）。

- Q1 → FR-134 の本文「WHEN 変更が main に merge される」と補足「文書だけの merge でも番号を進めるか」。推奨のままなら「CI が通り、前の番号から製品が変わった main の commit に」と直す。あわせて Story ST61 の題（「merge するたびに」）と完了の判定 1 つ目を `docs/stories/stories.json` から直す
- Q2 → FR-134 の補足「式を変えるか」（versionCode の式）
- Q3 → FR-134 の NEEDS CLARIFICATION「真ん中の数字（MINOR）をいつ上げるか」
- Q4 → 要件の本文は変えない（FR-134 の「同じ番号で中身の違う組み立てを作らない」の実装）。「元にした main の番号を名乗る」を選んだときだけ FR-134 と食い違うので直す

### record の段で戻したもの（2026-11-23。答えは 4 問とも推奨のまま）

- FR-134: 本文を「CI が通り、前の番号から製品が変わった main の commit に」へ ★ 2026-11-23 訂正（Q1）。未決の 3 つ —— NEEDS CLARIFICATION「真ん中の数字をいつ上げるか」（Q3: 確認バッチが入ったら）・補足の「式を変えるか」（Q2: 桁を広げる）・「文書だけの merge でも番号を進めるか」（Q1: 進めない）—— を同じ ★ で閉じた。Q4 は本文と食い違わない（本物に使わない 0.0.x を名乗る）ので本文は変えず、★ に実装の形だけを記した
- Story ST61: `docs/stories/stories.json` の題と完了の判定 1 つ目を Q1 に合わせて直し（3 つ目の「1 年分の merge の番号でも落ちない」は Q2・Q3 の答えでも成り立つので変えない）、`make_story.py` で `ST61.md` を貼り直した
- 整合の確認 `docs/audit/2026-11-23-st61-fr134-consistency.md`（C6・K1。行き先はすべて記入済み）: 要件 §5 の方針の行・ST61 の価値と judged・INDEX.md の題・`docs/release.md`・`docs/production-prep.md` の「merge ごとに」を改訂後の FR-134 に合わせ、FR-134 に「0.x の間に真ん中の上限に届けば、左を上げずに式を見直す」を ★ で足した

# 深掘り（第 2 回）

> /code-review の R10（review/code.md）: 枝の commit 58f7a4d8c の題が「wip」で、CLAUDE.md の commit の規則（`feat:` / `fix:` / `docs:` / `chore:` + 日本語 1 行、本文に「なぜ」）に合わない。直すには履歴を書き換える（後の commit の番号が全部変わる）ので、settle の席は直さず本人に返す。問いは `deep-questions-r2.json`・`docs/briefs/ST61-deep-r2.html`。

### Q5. 題が「wip」の commit を、merge の前に書き直すか（R10）

- 問い: 枝の途中の commit 58f7a4d8c（番号の印と置き場の台本の途中の作業。本文なし）の題を規則の形に書き直すか、そのまま merge するか。
- 答え: そのまま merge する（書き直さない）（本人の選択。第 2 回 `deep-answers-2.txt`。推奨と同じ・補足なし）
- loss: discarded
- 出所: review/code.md R10・CLAUDE.md「git」
- 推奨: そのまま merge する（書き直さない）。中身とその理由は直後の 53a74e0ce の題で読め、書き直すと枝の commit の番号が全部変わってグラフの記録とずれる
- 族「規則の形に合わない commit の題が main に入る」: Task agent の題「wip」→ 未決 Q5／グラフが付ける題（`chore(ST61): Task N を完了にする`）→ グラフ（harness2）の決め・範囲の外／これからの題を commit-msg の hook で止める → 範囲の外（ハーネスの改良の依頼）

### 第 2 回の答えで戻したもの（2026-11-27）

- 要件・Story へ戻すものは無い（commit の題の扱いで、FR-134 や ST61 の判定に効かない）。枝の commit は書き直さず、58f7a4d8c の題「wip」は main に残す（本人が受け入れた）
- 族の兄弟（これからの題を commit-msg の hook で止める）はこの Story の範囲の外のまま

# 深掘り（第 3 回）

> final review 2 の R24（review/code.md）: 前の番号の commit と差分が空の commit（revert で戻った commit など）にも番号が付く。仮の決定 D4 の解釈に関わる premise なので、settle の席は直さず本人に返す。問いは `deep-questions-r3.json`・`docs/briefs/ST61-deep-r3.html`。

### Q6. 前の番号の commit と差分が空の commit に、番号を付けるか（R24）

- 問い: 0.2.1 の後に製品に触れる merge B が入って run が取り消され、B を戻す revert C が続いたとき、差分が空の C に 0.2.2 を付けるか、付けないか。
- 答え: 付けない（差分が空なら「変わっていない」とする）（本人の選択。第 3 回 `deep-answers-3.txt`。推奨と同じ・補足なし）
- loss: exported
- 出所: review/code.md R24・design.md D4（仮）
- 推奨: 付けない（差分が空なら「変わっていない」とする）。D4 の文言「前の番号から製品が変わった commit に付ける」どおりになり、中身が同じで番号が違う版ができない。いまの実装は空の差分で `changes.sh code` に空行が 1 行渡り、「製品に触れた」と読む（実測: `changes.sh code <<<""` は rc=0、`printf '' | changes.sh code` は rc=1）
- 族「前の番号の commit と製品が変わらない commit に番号が付く」: 文書だけの merge → Q1・D4（付けない）／差分が空の commit → 未決 Q6／前の番号の commit の走らせ直し → D4 の 4.・D7（同じ番号）／祖先の run が後から成功 → D4 の 2.（付けない）

### 第 3 回の答えで戻したもの（2026-11-27）

- 要件・Story へ戻すものは無い。FR-134 の本文（★ 2026-11-23）は既に「前の番号から製品が変わった main の commit に」で、答えはこの文言どおりに読む側を選んだ（差分が空なら製品は変わっていない）。ST61 の題と完了の判定も同じ文言のまま成り立つ
- 仕様へ写すもの（次の `spec --answers Q6` の段）: design.md D4（仮）の 3. を「前の番号の commit との差分が空なら変わっていない」と明記し、入力が空のときの結果を Scenario に書く（upstream_miss の prevention）。直しは version.sh の number の 1 か所（`changed` が空なら変わっていない）と自己試験の Scenario 1 本。PR の CI の側（ci.yml の空の差分で code=true）は安全側なので変えない（R24 の直しの範囲）
- 族の兄弟（文書だけの merge・走らせ直し・祖先の run の後からの成功）は D4 の 1.・2.・4.・D7 で決まっていて、この答えで変わらない
