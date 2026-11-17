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
