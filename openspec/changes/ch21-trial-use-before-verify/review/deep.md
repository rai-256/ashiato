# CH21 深掘りの問いの独立検査（deep-review、第 3 版の第 1 回）

検査したもの: `openspec/changes/ch21-trial-use-before-verify/deep-questions.json`（Q1〜Q13、format 2）と `premises.md`。
問いの一覧を見る前に、手順 1〜8 をやり直した。読んだものは次のとおり。
- 依頼: `docs/changes/CH21.md`（「どこまで」と「本人の決定」は問い直さない）
- 席の契約: `.harness2/scripts/tsukau.py` 冒頭、`.harness2/README.md`「使ってみる席」
- 確認バッチ: `.harness2/graph/harness_graph/verify.py`、`.harness2/scripts/verify_batch.sh`・`verify_checklist.py`・`verify_iwakan.py`・`verify_ci_route.py`・`record-run`、`.harness2/skills/verify/SKILL.md`
- ashiato2 側: `tools/verify-prep.sh`・`tools/stack.sh`・`.github/workflows/ci.yml`
- 第 2 版: `git show e2c9031f8^:` の review/deep.md と provisional.jsonl

## 手順ごとの結果（要約）

- **手順 1（要件どうしの衝突）**: change なので `satisfies: []`。代わりに、依頼の項目どうしと、依頼と既存の台本を突き合わせた。
  - 依頼の「同じ fix で直す」は、本人が答える前には働かない（`verify.py:422`）。これは Q2 が名指しで問うている
  - 依頼の「直した後に席でもう一度確かめる」と、既存の fix の振る舞いが食い違う。fix は準備を丸ごと走らせ直す（`verify.py:359-361`）が、手順書は録画しか差し替えない（`verify.py:371`）。Q10 と Q11 の推奨も互いに食い違う（R5）
  - 依頼の「手で呼ぶときも」と Q5 の推奨が食い違う（R12）
- **手順 2（扉の幅）**: 次の 4 つの読み方が問いに無い。
  - 「前からある違和感」かどうかの決め方（R8）
  - 本人に回した問いの答えの行き先（R9）
  - 依頼の「相手」の読み方。Q6 は依頼を第 2 版の文で引いている（R13）
  - PC の相手の zip の出所。依頼が決めていて、Q7 の 2 つの案には幅が無い。実際の幅（統合の CI が組んだ zip）は案に無い（R10）
- **手順 3（新たに立つ一方通行）**: 外へ出るもの（exported）として、Q4 が扱うのは手順書の問いと違和感の issue だけ。次のものが無い。
  - 直しの道で公開の repo に出るもの。Story の issue への書き込み、commit の本文、足した試験（R1）
  - 席の作業ツリーにある本物の合言葉（R2）
  - 前からある違和感が、バッチのたびに issue として繰り返し立つこと（R7）
  - Q4 の推奨のリンクに入る、ホームのパス（R15）

  取っていないデータ（uncaptured）として、Sandbox の中のログが閉じると消えること（R4）
- **手順 4（日常に影響する選択）**: Q9（窓）・Q10（大きさ）・Q11（作り直し）はある。次のものが無い。
  - 直しと作り直しを繰り返す輪の上限を何で数えるか（R6）
  - 違和感の issue が毎日繰り返し立つこと（R7）
  - 選別で外したものを本人がどう知るか（R8）

  実行のフォルダが溜まる容量は、依頼にもブリーフにも保存期限が無いので、SKILL.md の規則に従って指摘にしない
- **手順 5（既存のコードが依頼を満たしていない箇所）**: grep と行で確かめた。
  - `tools/verify-prep.sh:14` は `set -euo pipefail`。席の失敗を拾わなければ、準備ごと落ちて manifest が消える（`verify_batch.sh:189-193`）。依頼が「止めない」と決めているので、問いではなく design の既定でよい
  - `verify-prep.sh:23-33` は本物の `.env` を写す。席は `tsukau.py:501` で `cwd=self.repo` として動く（R2）
  - `verify.py:359-361`・`:371` の fix の後の扱い（R5）
  - `verify_batch.sh:113` は、作業ツリーが汚れていると次のバッチを止める（R14）
  - `.harness2/skills/verify/SKILL.md:41` で、fix は Story の issue に書き込む（R1）
- **手順 6（過去の取りこぼし）**: `upstream_misses.py report` の 84 の観点のうち、当てはまったのは次のもの。
  - 54「送らずに止める決定は、止めたことを本人がどう知るか」→ R8
  - 7「後処理が失敗すると、印の無いデータが外へ出る」→ R1・R15。伏せる検査が漏れた題と、直しの commit
  - 39「後のために残すなら、どう見つけるか」→ R4・R14
  - 81「前の成果物を使い回すときは、書ける者と中身の確かめ」→ R10。CI の成果物を使う案
  - 9「OS のコマンドは実機で確かめる」→ premises で確かさ 5（Sandbox の窓・網）。指摘なし
  - 84「新しい置き場は保持も見る」も当たるが、要件にもブリーフにも保存期限が無いので指摘にしない
- **手順 7（族の網羅）**:
  - 「公開に出るもの」の族は、手順書の問い・issue（Q4）、警告（Q12）までしか無い。直しの道（R1）とリンクのパス（R15）が欠けている
  - 「候補の行き先」の族は、直す（Q2）・迷う（Q3）までしか無い。前からと決めたもの（R7・R8）、外したもの（R8）、本人の答えの後（R9）、直せなかった・載せられなかったとき（R6）が欠けている
  - 「席の環境」の族（取り合い・汚れ・ログ・合言葉）は、第 2 版で P と C に置いたものが、立て直しで全部消えた（R2・R4・R14）
- **手順 8（前提の確かさ）**:
  - premises.md 冒頭は「Q4 が依る前提は確かさ 5」と書く。しかし Q4 の harm は確かさ 2 の行（偽データ）に乗っていて、その行は実物と部分的に合わない（R3）
  - Q10 の「立てる上限 15 分」は、確かさ 2 の行（zip を手元で組む時間を測っていない）に乗っている（R11）
  - Q8 の推奨の側（網を切る）は確かさ 1 の行に乗る。ただ失うものは無く、通らなければ A で返すと書いてあるので、指摘しない

## 一覧と突き合わせた結果

- 抜け: R1・R2・R4・R6・R7・R8・R9・R14
- 不要（形の見直し）: R10（Q7）
- 分類違い: R12（Q5。本人の文を名指しせずに狭めている）
- 前提: R3・R11・R13
- 食い違い: R5（Q10 × Q11）
- 公開に出るもの: R15（Q4 の推奨）
- 説明の無い言葉: R16
- 画面の構造を文字で問うている問い: 無い（画面を持たない change）
- 既定の無い B: 無い（Q1・Q3・Q5〜Q7・Q9〜Q13 は、どれも案のどれか 1 つに `recommended` がある）
- `kind: irreversible` の `loss`: Q4・Q8 の `exported` は妥当（つないだ時点・push した時点で戻せない）

---

## 処置の置き方（questions の段）

`escalated` は本人への問い（第 1 回の 1 枚）に入れたもの。deep.md は第 1 回ではグラフの draft が書くので、「レビューから問いに入れたもの」の節（R → Q の対応）は spec の段が deep.md に書く。R2・R4・R14 の既定（D1・D3・D7・D9 にあたるもの）は、spec の段が design の D<n>（仮）と台帳（provisional.py）に置く。

## R1. 直しの道で、違和感の中身が公開の repo に出る。Q4 は手順書の問いと issue しか扱っていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q2・Q4）
- 根拠:
  - `.harness2/skills/verify/SKILL.md:41`: 確認バッチの fix は、通らなかったものを「元の Story の issue にコメントし、fix タスクにする」。直しは commit し、その Story の test を足す
  - 依頼は、候補を「人間が『通らなかった』と答えたときと同じ経路」で直すとしている
  - Q2 の推奨は、統合の CI の直しと同じ道を通る。直しの commit を原因の Story のブランチへ cherry-pick して push する（`.harness2/scripts/verify_ci_route.py:4-5`）
  - commit の本文には「なぜ」を書く決まり（`CLAUDE.md` の git の節）。repo は公開（premises.md の行 5、確かさ 5）
  - Q4 の decide が対象にしているのは「手順書の問いと違和感の issue」だけ
- kind: irreversible
- loss: exported
- 提案: Q4 の範囲に、直しの道で出るものを足す。具体的には、Story の issue へのコメント、commit の本文、試験の名前と期待値。足さないなら、兄弟の A の問いを 1 つ立てる。失わない側が高くつかなければ、P にしてよい。その場合の P は「直しの commit・コメントには題と候補の番号だけを書き、根拠は手元の結果を指す」。
  あわせて Q2 の兄弟（B）を足す。本人が見る前に直したものを、手順書に 1 行で知らせるか。
- 処置: escalated —— 本人への問い Q4 にした。Q4 の decide・points・irreversible に、直しの commit の本文・Story の issue へのコメント・足す試験の名前を足した。Q2 の兄弟（見る前に直したものを手順書に 1 行で知らせる）は Q2 の points と Q1 の context に推奨の既定として書いた

## R2. 席の作業ツリーにある本物の合言葉を席から遠ざける決定が、第 3 版から消えた
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json / premises.md
- 根拠:
  - `tools/verify-prep.sh:23-33`: 確認バッチの作業ツリーへ、main の checkout の `.env`（本物の画面の合言葉と API の合言葉）を写す
  - `.harness2/scripts/tsukau.py:501`: 席の `claude -p` は `cwd=self.repo` で動く。`:397` で `--permission-mode auto`。全文は実行のフォルダの `logs/claude.jsonl` に残る
  - `verify-prep.sh:147`: 確認バッチの DB には、確認の間に本人の電話が送った記録が入り、確認の後も残る。fix の後に準備を走らせ直すと（`verify.py:359-361`）、本人が確認を始めた後の DB で席が回りうる
  - 第 2 版は、これを失わない側の P として台帳に載せていた。D7（専用の project・空いている port・乱数の合言葉・`.env` を写さない）と D9（使い捨ての作業ツリー）。`git show e2c9031f8^:openspec/changes/ch21-trial-use-before-verify/provisional.jsonl` で確かめた。第 3 版は design と台帳を消し、問いにも premises にも無い
  - Q8 の context が案内で名指すのは「本人のサーバ・本人の PC の配布版・確認バッチの DB」だけで、`.env` は無い
- kind: irreversible
- loss: exported
- 提案: spec の段で、第 2 版の D7・D9 を P として台帳に戻す。席が読みうる本物の合言葉と、本人の記録が入った DB を、席の入力（モデルへの入力・題）から外す側にする。戻せないなら A の問いを足す。Q4 の harm と premises.md も、この前提に合わせて直す（R3）。
- 処置: escalated —— 本人への問い Q8 にした。Q8 の context に、どの案でも共通の推奨の既定（使い捨ての作業場所・本物の .env を写さない・乱数の合言葉と専用の DB）を書いた。spec の段で D（仮）と台帳に置く

## R3. premises の「偽データで、本人の本物の記録ではない」は実物と部分的に合わない。Q4 の harm がそれに乗っている
- 成果物: openspec/changes/ch21-trial-use-before-verify/premises.md / deep-questions.json（Q4）
- 根拠:
  - premises.md の行「確認バッチの起動（run.sh）が入れるのは偽データ」は確かさ 2（`verify-prep.sh:131` を読んだだけ）
  - 同じファイルの `verify-prep.sh:147` に「この回に電話が送った記録」がある。この回の DB には本人の本物の記録が入りうる
  - `verify-prep.sh:23-33` は本物の `.env` を写す
  - Q4 の推奨の `harm.link`「本人の本物の記録とは結びつかない」と points 3 は、この行に乗っている
  - premises.md 冒頭は「Q4 が依る前提は実物で確かめた（確かさ 5）」と書く。しかし Q4 は確かさ 2 の行（この行と、`tsukau.py` の public.json の契約）にも乗っている
- kind: premise
- 提案:
  - premises.md の行を直す。偽データが入るのは run.sh が初めて立てたときで、確認の間には本人の電話の記録が入りうること、写すのは本物の `.env` であること、を書く
  - 冒頭の「確かさ 5」を、Q4 が依る行の確かさに合わせて書き直す
  - Q4 の `harm.link` を「本物の記録と結びつくかは、席が使う DB と合言葉の決め方（R2）に依る」に直し、確かめていない分を `unknown` へ移す
- 処置: fixed premises.md —— DB の行を「立てたとき偽データ・確認の間に本人の携帯の記録が入りうる・本物の .env を写す」に直し、冒頭の確かさを Q4 が依る行ごとに書き分けた。Q4 の harm.link を「Q8 の DB の決め方に依る」に直し、context に書いた

## R4. Sandbox の中のログが、閉じると一緒に消える。選別の席の根拠が残らない
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠:
  - `.harness2/scripts/tsukau.py:64`: `LOG_<NAME>` は down の前にホストのパスから写す。Sandbox の中のファイルは、env_hook が外へ出さない限り届かない
  - premises.md の行「`wsb exec` は終了コードしか返さない（出力は共有フォルダのファイルで受け取る）」
  - 依頼「候補が本物かどうかは、別の席が根拠を見て決める」。#329 の根拠は更新係のログだった（premises.md の #329 の行）
  - 第 2 版は、これを D3 の P（止める前に、サービス・更新係・収集アプリのログとイベントの抜き書きを実行のフォルダへ写す）に置いていた。第 3 版では消えている
- kind: irreversible
- loss: uncaptured
- 提案: spec の段で、第 2 版の D3 を P として戻す（取っておく側）。Q3 の points に「選別の席が読む根拠には、Sandbox から写したログが入る」と書く。
- 処置: escalated —— 本人への問い Q8 にした（context の共通の既定に「閉じる前に中のログを外へ写す」）。Q3 の points に「Sandbox から写したログを含む」を書いた。spec の段で D（仮）と台帳に置く

## R5. 直した後の確かめ直しで、Q10 と Q11 の推奨が食い違う。既存の fix の後にも席が丸ごと回り、その結果は手順書に届かない
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q10・Q11）
- 根拠:
  - Q10 の points 3 は「直した後の確かめ直しは、直した候補だけを範囲に、同じ上限の半分で回す」
  - Q11 の推奨は「中身（範囲・入った PR のいちばん新しいコミット・main のコミット）が同じなら前の結果を写す」
  - Q2 の推奨の道では、直しが Story のブランチに載るので PR の head が変わる。Q11 の比べ方では「違う中身」になり、ふつうの大きさで全部を回し直す。Q10 の「直した候補だけ・半分」と合わない。前の回の候補（別の tag）を次の回がどう引くかも、どちらにも無い
  - 本人の「通らなかった」から入る既存の fix は、直しを `verify/<tag>` に積み、準備を `tools/verify-prep.sh <tag>` で走らせ直す（`verify.py:358-361`）。このとき PR の head も main も変わらないので、Q11 の比べ方では「同じ中身」になる。前の結果が写り、直した後の木を席が確かめない
  - fix の後の手順書は録画・残るリスク・最終仕様書しか差し替えない（`verify.py:371`、`verify_checklist.py:520-548` の `--refresh-recordings`）。manifest の段落（席の結果）は差し替わらない
  - 準備が落ちると、fix の node は StepFailed で止まる（`effects.py:155-164`。agent の再試行は TransientError だけ）
- kind: conflict
- 提案: Q10 と Q11 を 1 つの規則にそろえる。比べる中身に `verify/<tag>` の HEAD を入れるか、直しの後の確かめ直しを別の口（直した候補の番号を渡す）にする。fix の後の席の結果を手順書に写すこと（`--refresh-recordings` の拡張）は、Q1 の harness2 への依頼の中身として points か context に書く。
- 処置: escalated —— 本人への問い Q10・Q11 にした。Q11 の比べる中身に確認バッチの木のいちばん新しいコミットを入れ、直した後は回し直すと書いた（直した候補だけの確かめ直しは Q10）。fix の後に席の結果を手順書へ差し替えることは Q1 の context（harness2 への依頼の中身）に書いた

## R6. 直す → 作り直す → 席がまた見つける、の輪を何で数えて止めるかが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q2・Q3）
- 根拠:
  - Q3 の推奨は「1 回のバッチで直すのは 3 件まで、同じ候補の直しは 1 回まで」
  - Q2 の推奨の道では、直すたびにバッチを作り直す（`verify.py` の ci_fix → batch）。作り直すと新しい tag の新しいバッチになるので、「1 回のバッチ」で数えると上限が作り直しのたびに戻る
  - 統合の CI の直しは、thread の state で数えている（`verify.py:51`、`CI_FIX_ATTEMPTS = 2`。`route_batch` の `len(state.get("ci_fixes"))`）
  - Story のブランチへ載せる台本は、Story の作業ツリーが無い・汚れている・手元が origin と違うときは載せない（`verify_ci_route.py:16-18`）。そのとき CI の直しは人間を待つ（ci_help）。席の直しで同じことが起きたときの行き先は、問いに無い
- kind: daily
- 提案: Q3 に足す。上限を数える単位（確認バッチの thread、または同じ Story の組）と、載せられなかったときの行き先を書く。推奨の既定の例は「thread で数える」「載せられなければ本人の問いへ」。B のままでよい。
- 処置: escalated —— 本人への問い Q3 にした。上限を確認バッチの流れ 1 本で数える（作り直しても数え直さない）こと、Story のブランチへ載せられなければ本人の問いへ回すことを decide・points・context に書いた

## R7. 前からある違和感が、作り直しのたびに公開の repo へ新しい issue として立つ
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json
- 根拠:
  - `verify_iwakan.py:38`・`:93`: 同じ issue を 2 本作らない印は「tag と問い」ごと。tag が変われば別の issue になる
  - 確認バッチは 1 日に 1〜5 回作り直される（premises.md、確かさ 4）。Q11 の推奨も、main か PR が進めば席を回し直す
  - 前からある違和感は、どの回の範囲にも残り続けるので、毎回見つかりうる
  - 依頼「そのバッチと関係ない、前からある違和感 → 受け皿の依頼（issue）にする」。issue は公開の repo に立ち、題が残る（Q4）
- kind: daily
- 提案: 推奨つきの B を足す（Q4 の兄弟）。前の回の結果と、開いている「違和感」の issue に同じものがあれば、新しく立てずにそちらへ寄せるか。何で同じとみなすか（題・種類・範囲）も決める。推奨の例は「開いている issue の題と、前の回の実行のフォルダの候補に当てて、同じなら立てない」。
- 処置: escalated —— 本人への問い Q3 にした。context に「開いている違和感の issue に題と範囲が同じものがあれば立てずに寄せる」を推奨の既定として書いた

## R8. 「前からある」の決め方と、選別で外したものを本人がどう知るかが無い。前からと誤ると、退行が本人を通らずに main に入る
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q3）
- 根拠:
  - 依頼は、行き先を 3 つ（直す・本人に回す・前からなら issue）に分ける。そのうち「関係ない、前から」をどう決めるか（main の版で再現する、差分に触れているかで見る、など）は Q3 に無い。Q3 が決めるのは「迷ったら」だけ
  - `verify.py:340-347` と `wait_merged`: 「通らなかった」が 0 のバッチは機械が merge する。違和感は issue にして止めない。したがって、そのバッチが原因の候補を「前から」と誤ると、本人の問いにも直しにも乗らず、issue だけ立って merge される
  - 選別で「本物でない」とした候補は、実行のフォルダ（repo の外）にしか残らない（`tsukau.py:35-48`）。`upstream_misses` の観点 54（止めたことを本人がどう知るか）
- kind: daily
- 提案: Q3 に兄弟の B を足す。1 つ目は「前から」の決め方。推奨の例は「その回の差分が触れた機能に当たるなら、前からとはみなさず本人へ」。2 つ目は、外したもの・前からとしたものの件数を手順書に 1 行で書くか（題を出すなら Q4 の検査を通す）。
- 処置: escalated —— 本人への問い Q3 にした。points と context に「差分が触れた機能は前からとみなさず本人へ」「外した件数を手順書に 1 行」を書き、図の「前から」の行を直した

## R9. 本人に回した問いの答えが、それぞれどこへ行くかが無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q1）
- 根拠:
  - 依頼「直すかどうかが判断になるもの → 手順書の問いとして人間に回す」
  - 今の答えの行き先は 2 つしかない。「通らなかった」は fix へ（`verify.py:338`）。「違和感」の問いは `verify_iwakan.py` で issue にする（`verify.py:340-341`）
  - 「確認できない（まだ）」は持ち越しの一覧に残る。この一覧は tasks.md の「人間の確認待ち」の項目を前提にしている（`verify_checklist.py:595`。tasks.md に無い題は error）。席の候補には tasks.md の項目が無い
  - Q1 の推奨は、問いを足す口を harness2 に依頼するとしている。しかし、どの答えを用意し、それぞれがどこへ行くかは書いていない
- kind: technical
- 提案: Q1 の context に、harness2 への依頼に入れる答えの形を書く。それとも推奨つきの B を足す。例: 「直す（通らなかったと同じく fix へ）」「依頼にする（違和感の issue へ）」「このままでよい（記録だけ）」「確認できない（次の回の席の結果に任せ、持ち越しにしない）」。
- 処置: fixed deep-questions.json —— Q1 の context に、harness2 への依頼に入れる答えの形（直す・依頼にする・このままでよい・確認できない）と行き先を書いた

## R10. Q7 の 2 つの案には幅が無い（依頼が決めている）。残る幅「統合の CI が組んだ zip を使う」が案に無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q7）
- 根拠:
  - 依頼は「PC（Windows Sandbox に配る zip を入れる）」で、範囲は「その回のバッチで足した機能」。Q7 の案 2（前夜の版）は、cost に自分で「範囲を触れない」と書いていて、依頼を満たさない
  - 確認バッチの PR は draft で作られ、`pull_request` の CI が走る（`.github/workflows/ci.yml:2-4`）。`package-windows` の job は、PR の head（`verify/<tag>`）を checkout して配る zip を組み、成果物として上げる（`ci.yml:467-474`・`:541`・`:559-560`。`code` が変わった回だけ）
  - Q1 の推奨は、処置を harness2 の確認バッチに置く。すると、席を統合の CI の後に回し、CI が組んだ zip を使う並びも選べる
  - `upstream_misses` の観点 81（前の成果物を使い回すときは、書ける者と中身の確かめを context に書く）
- kind: technical
- 提案: Q7 を作り直す。案は「手元で組む（準備の中。時間は測っていない）」と「統合の CI が組んだ zip を使う（席は CI の後。CI が赤なら回さない）」にする。CI の成果物を使う案の context には、成果物を書ける者（その run だけ）と、zip の中身の確かめ方を書く。作り直さないなら、Q7 は依頼で決まっているので問いから外し、design の既定にする。
- 処置: fixed deep-questions.json —— Q7 を作り直した。案は「準備の中で手元で組む（推奨）」と「確認バッチの CI が組んだ zip を使う」。前夜の版は依頼を満たさないので案から外し、why に書いた。CI の案の context に成果物の確かめ方を書いた

## R11. Q10 の「立てる上限 15 分」は、測っていない zip の組み立て時間に乗っている
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q7・Q10）/ premises.md
- 根拠:
  - premises.md の行「手元で zip 全体を組めるか・何分かかるかは確かめていない」（確かさ 2）と、道具の有無の行（確かさ 2。組み立ては走らせていない）
  - Q7 の推奨は手元で組み、Q10 の推奨は立てる上限を 15 分とする（`tsukau.py` の `ready_timeout_s`。差し込み口の `up` が `ready` を置くまで）
  - 組み立てを `up` の中に置けば 15 分に数えられ、組み立てがそれを越えると PC の相手は毎回 `env_failed` になる
  - 第 2 版は、組み立てを席の前に別に置いていた（D5。上限 2,400 秒で、ready の待ちに数えない）
- kind: premise
- 提案: Q10 の points に、組み立てを立てる時間に数えるか（`up` の中か、その前か）と、まだ測っていないことを書く。組み立てを前に置くなら、その上限も B の既定として書く。
- 処置: fixed deep-questions.json —— Q10 の points に、zip の組み立ては立てる時間の前に別に置き上限 40 分（まだ測っていない）と書いた

## R12. Q5 の推奨は、依頼の太字の文を手で呼んだときについて狭めているのに、名指ししていない
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q5）
- 根拠:
  - 依頼（逐語）は「**見つけた違和感の候補は、見つけて書くだけで終えず、機械が直すか人間に回す。** 手で好きなときに呼ぶ（`/tsukau`）ときも同じ差し込み口を使う」
  - 本人の指摘（2026-11-20）は「手順書に件数を載せるだけでは足りない」
  - Q5 の推奨「結果を残すだけ（今のまま）」は、手で呼んだときを「見つけて書くだけ」で終える。Q5 の why は差し込み口の文だけを引き、太字の文を変えることには触れていない
- kind: conflict
- 提案: Q5 の decide か why に、「依頼の『見つけて書くだけで終えず』を、手で呼んだときには当てない」と名指しで書く。依頼の文を変える問いなので、Q2 と同じく `premise`（A。答えを省けない）にする。
- 処置: escalated —— Q5 を premise（答えを省けない）にし、decide と why で依頼の「見つけて書くだけで終えず」を手で呼んだときには当てないことを名指しした

## R13. Q6 の context は、依頼を第 2 版の文で引いている
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q6）
- 根拠:
  - Q6 の context は、依頼を「相手は少なくとも PC、画面も入れられるなら入れる」と引く
  - 今の `docs/changes/CH21.md` の「どこまで」は「相手は画面（web）と PC（…）。携帯は外（後で足せる作り）」
  - 同じ context の「すでに main に入っていて確認だけ残っている Story は範囲に入らない」は、依頼の「その回に入った Story / change」を狭める。確認バッチに後から足した Story（`verify_batch.sh` の `--story`）も、その回で人間が確かめる Story である。狭めていることが書かれていない
- kind: premise
- 提案: 引用を今の依頼の文に直す。`--story` で足した Story を範囲から外すことは、狭めていると明記するか、範囲に入れる（範囲は、確認バッチの Story の一覧から読む）案と比べる。
- 処置: fixed deep-questions.json —— Q6 の context の引用を今の依頼の文に直し、確認バッチに後から足した Story を範囲から外すのは依頼を狭めていると明記した（入れるには準備に Story の一覧を渡す口が要る）

## R14. 第 2 版で P と C に置いた環境の既定が、第 3 版では問いにも premises にも無い
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json / premises.md
- 根拠（第 2 版の台帳と review/deep.md。`git show e2c9031f8^:openspec/changes/ch21-trial-use-before-verify/provisional.jsonl`）:
  - D9: 席を使い捨ての作業ツリーで走らせる。`verify_batch.sh:113` は、作業ツリーに未コミットの変更があると次のバッチ全体を止める。依頼の「準備は止めない」とぶつかる
  - D3: Sandbox の取り合いと片付け。録画は、別の Sandbox が動いていれば撮らない（`record-run:317`）。片付けでは動いている Sandbox を止める（`record-run:570-573`）。席の片付けが閉じ損ねると、同じバッチの録画が撮れない
  - D1: `out_root` を明示し、`--label verify-<tag>` で走らせる。既定の置き場は作業ツリーの名前で分かれる（`tsukau.py:245`）
  - D15: 題の `@` と `#数字` を全角に替える（公開の repo で通知と issue へのリンクになる）。第 3 版の Q4 の伏せる検査は「ホームのパス・IP・メールの形」だけ
- kind: technical
- 提案: spec の段で、D1・D3・D9・D15 を C または P として design と台帳に戻す。D15 の `@` と `#数字` は、Q4 の推奨の「伏せる検査」の points に足す。
- 処置: fixed deep-questions.json —— D15（`@` と `#数字` を全角に）は Q4 の context の伏せる検査に足した。D9 の使い捨ての作業場所は Q8 の context の共通の既定に書いた。D1（out_root と --label）・D3（Sandbox の取り合いと片付け）は spec の段で C / P として design と台帳に戻す（下の「処置の置き方」）

## R15. Q4 の推奨は、手元の結果へのリンク（ホームのパス）を公開の手順書に書くことになる
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q4）
- 根拠:
  - Q4 の推奨は「根拠は手元の実行のフォルダの結果（RESULT.md）へのリンクで開く」。手順書の問い（`docs/verify/<tag>.json`）は commit されて公開される（premises.md の行 5）
  - 実行のフォルダの既定は `~/.local/state/harness2/tsukau/<repo の名前>`（`tsukau.py:245`）。リンクを書くと、ホームのパス（利用者名）が公開に出る
  - 同じ推奨は、題からは「ホームのパス」を伏せるとしている
  - 今の録画の場所は `C:\dev\ashiato2-recordings\…` で、利用者名を含まない（main の `docs/verify/*.json` を grep して確かめた）
- kind: irreversible
- loss: exported
- 提案: Q4 の推奨の `irreversible` と `harm` に、リンクのパスを足す。あるいは、リンクは commit しない HTML（`docs/briefs/`。git が無視する。`.gitignore:34`）にだけ描き、JSON には回の名前と候補の番号だけを書く形にする。
- 処置: escalated —— 本人への問い Q4 にした。推奨を「手元の結果へのリンクは git が無視する HTML にだけ描き、公開の JSON には回の名前と候補の番号だけ」に直し、context に実行のフォルダのパスで利用者名が出ることを書いた

## R16. Q8 の要点と context が、説明なしで内輪の言葉を使っている
- 成果物: openspec/changes/ch21-trial-use-before-verify/deep-questions.json（Q8）
- 根拠:
  - points 1・3 の「CH22」「本人の決定 #332」
  - context の「正典 distribution」「requires に CH22」「CH21 は CH22 の後に積む」
  - Q2 の context の「ci_fix」は、たたんだ詳細の中なので許せる。しかし points 3 の「統合の CI の直し」は、その場で何をする直しかが分からない
- kind: technical
- 提案: その場で言い換える。例: 「CH22（PR の CI で、配る形を外につないで 5 分置く変更）」「#332（その依頼）」「正典 distribution（配る形の決まりの仕様）」「requires に CH22（CH22 が入るまで着手しない）」「統合の CI の直し（確認バッチの CI が赤のとき、AI が原因の Story を直してそのブランチに載せる道）」。
- 処置: fixed deep-questions.json —— Q8 の CH22・#332・正典 distribution・requires、Q2 の「統合の CI の直し」をその場で言い換えた

