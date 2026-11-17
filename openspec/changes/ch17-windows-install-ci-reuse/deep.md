# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: CI の package-windows / install-windows の job の回し方（台本だけの変更では前に作った Windows の配布物を使い回す・Rust のビルドにキャッシュを効かせる）を変えるだけで、製品のコード・配布物の中身・API や送信の契約・移行には触れない。走らせる確かめ（install-check / update-check-install の Scenario）も変えない。使い回しの可否を決めるビルドの入力の鍵は、本人が「作るときに決める」とした範囲で、誤っても CI の設定を直せば戻る（取っていないデータ・捨てたデータ・外に出たものは無い）ので、spec の段で D（仮）と反転条件に書き、spec-review に確かめさせる。鍵を出す判定は changes.sh の正本に足すか、tools/release/ に新しい台本を置く
- 触れる予定のパス: .github/workflows/ci.yml / tools/release/changes.sh / tools/release/windows-build-key*.sh / docs/testing.md / docs/adr/*.md

## 第 1 回（2026-11-17 起票）—— spec のレビュー（review/spec.md）が出した 2 件

軽い路で spec を書いたが、spec-review が「正典の本文を変えないと書けない」と判定した（R1・R2。どちらも premise）。
正典を `MODIFIED` で変える change は軽い路を通れない（`scripts/lanes.py`）。答えで通常の路へ移る。問いは `deep-questions-r1.json`。

### Q1. PR ごとに Windows の配布物を組む、という要件を改めるか（R1）

正典 `distribution` の Requirement「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」の本文（コードを触った PR では Windows 向けも組む・
CI の実行の成果物としてだけ置く）が、使い回しと反対の SHALL を言う。推奨: 改める（使い回しを認める）。

本人の答え（`deep-answers-1.txt`。推奨の選択肢をはっきり選んだ。補足なし）: **改める（使い回しを認める）**

→ 正典 `distribution` の Requirement「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」を `## MODIFIED` で改める:
「組み立てる。ただし組み立ての入力（鍵）が同じ前の組み立ての Windows の配布物があれば、それを使う」「CI の実行の成果物と CI の
キャッシュにだけ置き、公開しない」。この Requirement の導出元は Story の価値（FR ではない）。requirements.md へは、Q2 の FR-127 の ★ が前置きとして使い回しを写す（整合の確認 C8）。
正典を改めるので通常の路へ移る（`lane-verdict.json` は機械が書く）。

### Q2. 使い回した配布物が指す commit（R2）

正典 `distribution` の Requirement「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」の本文（PR の組み立ては head の commit
そのものから組み、`SOURCE.txt`・印・組んだソースが同じ commit を指す）と design D3 が食い違う。推奨: 要件を改め、使い回した配布物は組んだ commit を指すとする
（`package-artifacts` は変えない）。

本人の答え（`deep-answers-1.txt`。推奨の選択肢をはっきり選んだ。補足なし）: **要件を改め、使い回した配布物は組んだ commit を指すとする**

→ 正典 `distribution` の Requirement「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」を `## MODIFIED` で改め、
「使い回した Windows の配布物の `SOURCE.txt` と組み立ての印は、それを組んだ commit を指す」を足す。design D3 の振る舞いを spec の
Scenario に移す。`package-artifacts` の job は変えない（本人の範囲のまま）。要件 FR-127 の ★ 2026-11-03（第 3 回）「PR の組み立ては
PR の head の commit そのものから組む」に例外を足すので、requirements.md の FR-127 に ★ 2026-11-17 補足を入れた。

## 要件へ戻すもの

- FR-127（★ 2026-11-17 補足。Q2）: 使い回した Windows の配布物の `SOURCE.txt` と印は、それを組んだ commit を指す（PR の head の commit から組むことの例外）

## spec の段へ渡すもの（整合の確認 `docs/audit/2026-11-17-ch17-fr127-consistency.md` の行き先 fix）

- C1: `MODIFIED` の範囲は名指しの 2 つの Requirement の本文だけでなく、正典 distribution の :2261（期待値を組んだ commit から取る）と Scenario 3 つ（:2319・:2329・:2340）に及ぶ。Scenario の名前を変えるなら、印 4 か所（`notices.sh:360`・`check-guards.sh:273,328`・`check-artifacts.sh:445`）も変わり、`lane.json` の予定のパスの外になる
- C2: 使い回した配布物の `SOURCE.txt` が指す commit（同じ PR の前の push の head）を、空の clone で `git fetch origin <commit>` できるかを完了の判定に足す（★ 2026-11-03 の未確認が ST53 の archive で宛先を失っている）
- C3: design D3 の反転条件に当たる手順が `docs/release.md:151`（run の head_sha と配布物の印を比べる）に既にある。`draft.sh:45` の説明も使い回した run では正しくない
- C4: 走っている st58 が `package-windows-test` を 6 つの成果物に分け、試験の台本 `update-apply-install.sh` を足す。D1（除く台本）・D2（保存する 2 つ）・CH17.md の「壊してはいけないもの」の成果物の名前が、st58 が先に main に入ると変わる
- K1: `package-artifacts` は版と `LICENSE` を head の木から取る（ci.yml:549-559）。使い回しが通るのは `Cargo.toml` と `LICENSE` が鍵に入っている間だけ。:2261 の `MODIFIED` にこの条件を書く

# 深掘り（第 2 回）

> 下流の最後の段（settle）で、review/code.md の R5・R7（検査を走らせる job が無い）と R6（同じ PR の前の push の保存物を確かめずに使う）は、本人の範囲（Windows の 2 つの job だけ）と第 1 回の答え（Q1・Q2）に触れないと直せないので、A として本人に返す。問いは `deep-questions-r2.json`・`docs/briefs/CH17-deep-r2.html`。

### Q3. 使い回しの形の検査（`--check-workflow`）を、Linux の CI の job で毎回走らせるか（R5・R7）

- 問い: `windows-build-key.sh --check-workflow` とその自己試験を Linux の `chain` の job に 2 行足して PR ごとに走らせるか。本人の範囲（Windows の 2 つの job だけを変える）を 2 行越える
- 答え（`deep-answers-2.txt`。推奨の選択肢をはっきり選んだ。補足なし）: **足す（chain の job に 2 行。Windows の job の自己試験は消す）**
- kind: premise
- 出所: review/code.md R5（code-verify）・R7（/code-review）
- 推奨: 足す（`chain` の job に 2 行。`package-windows` の自己試験の step は消す）
- 族「使い回しの形を守る検査が、後の変更で走らない」: `--check-workflow` → Q3／鍵の `--self-test` → Windows の job で組み立てるとき（R10 の直し。Q3 で移すかも決まる）／`check-guards.sh workflows`・`check-ci-ch14-step.sh` → 既に CI で走る

### Q4. 同じ PR の前の push が保存した組み立てを、後の run が確かめずに使う件（R6）

- 問い: 同じ PR の前の push が保存した Windows の配布物（と Rust の `target`）を、後の run がそのまま使う形を、今のまま受け入れるか
- 答え（`deep-answers-2.txt`。推奨の選択肢をはっきり選んだ。補足なし）: **受け入れる（今のまま。残る危険を design と docs/release.md に書く）**
- kind: premise
- 出所: review/code.md R6（security-review）
- 推奨: 受け入れる（今のまま。残る危険を design の Risks と docs/release.md §11.1 に書く）。ほかの案: 保存を main への push に限る（Q1 を狭める）／使い回した run では release-package を上げない（Q2 を改める）
- 族「PR から書けるキャッシュを、後の run が確かめずに使う」: 同じ PR の Windows の配布物・Rust の target → Q4／他の PR → 引けない（D2）／main の置き場 → PR から書けない／pgsql → sha256 で確かめる（既存）／release・nightly → 自分で組む

### 第 2 回の答えで決まったこと（下流の settle へ渡す）

- Q3 → 本人の範囲（Windows の 2 つの job だけ）を `chain` の job の 2 行だけ広げる。`chain` の job の自己試験の並びに
  `tools/release/windows-build-key.sh --self-test` と `--check-workflow` を足し、`package-windows` の自己試験の step を消す。
  design D2 の「`--check-workflow` が見る」はそのまま成り立つ（毎回の PR の CI で走る）。tasks 2.2 の「merge gate で走らせる」を
  「`chain` の job で走らせる」に直す。review/code.md の R5・R7 はこれで処置する
- Q4 → 今のまま受け入れる。design の Risks に「同じ PR の前の push が保存した Windows の配布物と Rust の `target` は、後の run が
  中身を確かめずに使う（書けるのは PR の枝へ push できる者だけ。本物のリリースと開発版はキャッシュを引かずに自分で組む）」を足し、
  `docs/release.md` §11.1 に「使い回した run の下書きの Windows の zip は、組んだ commit を PR の履歴で確かめてから使う」を足す。
  第 1 回の Q1・Q2 は変えない。review/code.md の R6 はこれで処置する（受け入れ）

## 要件へ戻すもの（第 2 回）

- なし。Q3 は CI の job の置き場、Q4 は残る危険の受け入れで、要件の本文（FR-127 ★ 2026-11-17 を含む）は変わらない
