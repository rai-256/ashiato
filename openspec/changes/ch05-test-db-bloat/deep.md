# 深掘り（第 1 回）

> 推奨の答えで先に仕様と GO 資料を書いた（2026-10-20 の流れ）。答えは `deep-answers-1.txt`（2026-10-25 record）。
> 本人は全問に答えた（`(推奨のまま: …)` は無い）。推奨と違ったのは Q7 だけ。仕様への合意（QA）は「合意する」。
> 第 2 回の問い Q8（premise。CH06 との重なり。`deep-questions-r2.json`）は `deep-answers-2.txt`（2026-10-25 record）で本人が答えた（推奨と同じ）。節は「## 第 2 回」。

### Q1. CH05 の範囲 —— 走っている CH04 との分け方

- 問い: 依頼の 1（試験の後片付けを殺されても残らない形にする）は、走っている CH04 がほぼ実装を終えています。CH05 は何をやりますか。
- 出所: review/deep.md R1・R3（CH04 との重なり・CH04 が禁じたトリガの外し）。壊してはいけないもの（トリガ・`ashiato` を使う 3 つの経路・`testdb` の形）は CH04 から引き継ぎ、`docs/changes/CH05.md` に書いた
- 答えの後にやること（record）: 推奨のとおりなら `docs/changes/CH05.md` の front matter を `requires: [CH04]` にする（下流の入口が CH04 の実装を取り込み、archive が CH04 の後になる。tasks 1.1 がこれに依る。review/spec.md R2）
- 答え: CH04 の上に積み、CH04 が持たないものだけをやる（本人の選択。推奨と同じ）

### Q2. 膨れた DB をいつ作り直すか（依頼の 2）

- 問い: worktree のテスト用 DB（`ashiato` database）を、機械はいつ作り直しますか。
- 答え: 検証の前（`tools/verify-env.sh`）に大きさを測り、閾値を超えていたら作り直す（本人の選択。推奨と同じ）

### Q3. 使っている途中で壊さない —— 何を見て作り直しを見送るか

- 問い: 「走っている試験がある DB は作り直さない」を、何で判定しますか。
- 答え: run の助言ロックか、`ashiato` への接続が 1 本でもあれば見送る（本人の選択。推奨と同じ）

### Q4. 作り直し方

- 問い: 作り直すとき、コンテナごと作り直しますか、中の `ashiato` database だけを作り直しますか。
- 答え: 中の `ashiato` database だけを作り直す（コンテナ・ネットワーク・volume はそのまま）（本人の選択。推奨と同じ）

### Q5. 作り直してよい DB の範囲

- 問い: 機械が作り直してよいのは、どの作業ツリーの DB ですか。
- 出所: review/deep.md R5
- 答え: Story・change の作業ツリー（`-st<NN>`・`-ch<NN>`・`-up-*`）だけ（本人の選択。推奨と同じ）

### Q6. 「殺した後の次の run の速さが変わらない」をどう確かめるか

- 問い: 完了の条件「体積の試験を途中で殺した後でも、次の `cargo test` の速さが変わらない」を、どの形の試験で確かめますか。
- 答え: 速さを決める量で見る（時間は記録するだけ）。殺す時点は 2 つ（本人の選択。推奨と同じ）

### Q7. ついでに直すか（範囲の外）

- 問い: 実物を見ていて見つかった近くの問題のうち、この change で一緒に直すものはありますか。
- 出所: review/deep.md R6。(1) を入れないと CH05 の下流の作業ツリーも DB を立てられない（review/spec.md R1。Q7 の context に書き足した）
- 答え: (1) 使われていないコンテナ・ネットワークを片付ける（volume は消さない）（本人の選択。**推奨（どれも入れない。(1) は別の change として申し送る）と違う**）
- 補足: なし
- これで決まったこと: CH05 は「膨れた `ashiato` を作り直す道」「殺した後の検査」に加えて、**使われていない docker のコンテナ・ネットワークの片付け**を持つ。
  片付けてよいのは選択肢の detail のとおり「作業ツリーが消えたことを機械で確かめられるもの」だけ。volume は消さない（浮いた volume 65 個は残す）。
  (2)（既存の 4 つの台本の `down -v` に見送りを当てる）と (3)（lock timeout・権限のエラーの調べ）は選ばれなかった —— 範囲の外のまま
- 仕様へ: `spec --answers Q7` が proposal・specs・design・tasks と `docs/changes/CH05.md` の完了の判定に写す
  （review/spec.md R1 の「CH05 の下流の作業ツリー自身が DB を立てられない」は、この片付けを下流の初めに置けば CH05 の中で解ける）

## 第 2 回（spec --answers Q7 の段で見つかったもの。2026-10-25）

| Q | 段 | 論点 | 失われるもの |
|---|---|---|---|
| Q8 | A（premise） | CH06（検証のたびに DB を空から作り直す）と CH05 の作り直し・見送りが重なる（review/spec.md R20） | —（Q2〜Q5 の前提） |

### Q8. CH06 と CH05 の作り直し・見送りが重なる（A / premise。推奨: CH05 は殺した後の検査と片付けだけにする）

- **なぜ人間の判断か**: Q2〜Q5 は「検証の準備で、膨れたときだけ・使っていなければ作り直す」前提で答えてもらった。2026-10-25 に main へ入った CH06 の上流は、同じ `tools/verify-env.sh` で、`HARNESS_VERIFY=1` なら毎回 DB を空から作り直し、接続も切る（CH06 Q2）。
  CH06 の後は Requirement 1・2 が検証の準備で働かず、向きも逆になる。本人の答えの扱いを本人に戻す
- **出所**: review/spec.md R20・R21（`origin/main:docs/changes/CH06.md`・`origin/main:openspec/changes/ch06-reset-verify-db/design.md` D5・D6・Non-Goals、`git worktree list` の `feat/ch06-reset-verify-db`）
- **問い**: `deep-questions-r2.json`（HTML は `docs/briefs/CH05-deep-r2.html`）
- **本人の答え**: CH05 は殺した後の検査と片付け（Q7）だけにする。作り直しと見送り（Requirement 1・2）は外し、CH06 に任せる（本人の選択。推奨と同じ）
- 補足: なし
- これで決まったこと: CH05 の範囲は「殺した後の次の回に残りが無いことの検査」（D6）と「消えた作業ツリーのコンテナ・ネットワークの片付け」（Q7・D9）だけになる。
  Q2〜Q5 の答え（膨れたら作り直す・使っている途中は見送る・中の database だけ・作り直してよい作業ツリーの範囲）は、検証のたびに DB を空から作り直す CH06 に任せて CH05 では作らない
  （本人の答えは消さず、前提が変わったので働く先が無くなった記録として残す）。`tools/test-db-rebuild.sh` と `tools/ports.sh` の作業ツリーの種類は作らない。
  `requires: [CH04, CH06]`。片付けは CH06 の `tools/verify-env.sh` の上に積み、CH06 の `verify-environment` の「ほかの compose の project のコンテナ・volume には触らない」を、消えた作業ツリーのものを除く形に MODIFIED で書き直す（review/spec.md R21）
- 仕様へ: `spec --answers Q8` が proposal・specs・design・tasks と `docs/changes/CH05.md` の完了の判定に写す。spec・design・tasks を書き直すので、仕様への合意はもう一度聞く（選択肢の detail のとおり）
- **効く先**: 推奨なら proposal・spec の Requirement 1・2 と Scenario 14 本・design D1〜D5・tasks Task 1・2 と 4.2 の見送りの確かめを外し、`docs/changes/CH05.md` の front matter を `requires: [CH04, CH06]` にし、CH06 の「ほかの project のコンテナには触らない」（`verify-environment`）を消えた作業ツリーを除く形に MODIFIED で書き直す（review/spec.md R21）。Q7 の片付け（D9）と殺した後の検査（D6）は残る

## 当初案を覆したもの

- Q8: 第 1 回の Q2〜Q5（膨れたら作り直す・見送る）の答えは、CH06 との重なりで CH05 の範囲から外れた（本人は推奨を選んだ。作り直しは CH06 が検証のたびに空から行う）
- Q7: 推奨は「どれも入れない（(1) は別の change へ申し送る）」。本人は (1) を選び、使われていないコンテナ・ネットワークの片付けを CH05 の範囲に入れた（volume は消さない）

## 要件へ戻すもの

- なし（change は要件を満たさない。`satisfies: []`。範囲の変更は `docs/changes/CH05.md` に書いた）
