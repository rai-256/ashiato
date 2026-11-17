## CH17 — Windows のインストールの試験の 1 周を縮める（台本だけの変更ではビルドし直さない）

受け皿の依頼 #308 から。軽い路で始め、spec のレビュー（R1・R2）で正典 `distribution` の本文を改める必要が分かったため、通常の路へ移った。

### 深掘りで本人が決めたこと（第 1 回・2026-11-17。どちらも推奨の選択肢を補足なしで選んだ）

- **Q1**: 「コードを触った PR では毎回 Windows の配布物を組む」を改め、使い回しを認める
  → 正典の Requirement「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」を MODIFIED で改めた。
  Windows 向けは、組み立ての入力（鍵）が同じ前の組み立てがあれば使う。置き場は CI の実行の成果物と CI のキャッシュで、どちらも公開しない
- **Q2**: 使い回した配布物の `SOURCE.txt` と印は、それを組んだ commit（同じ PR の前の push の head か main の commit）を指す
  → 正典の Requirement「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」を MODIFIED で改め、Scenario を 1 つ足した。
  `package-artifacts` は変えない。要件 FR-127 には ★ 2026-11-17 を入れた。design D3 は本人の決定になった（（仮）を外した）

正典の既存の Scenario は名前も中身も変えていない。使い回しの後も成り立つので、印（`notices.sh`・`check-guards.sh`・`check-artifacts.sh`）にも触れない。

### 仮決め（design の（仮））

- D1（仮）: 鍵は、名指しして除くもの（試験の台本 3 つと文書）以外の全部と、道具の版・走っている CI の定義から作る。`Cargo.toml` と `LICENSE` は外さない
- D2（仮）: 保存物は actions/cache の完全一致で引き、PR の run でだけ使う。保存物は `package-windows` が上げる置き場の全部
- D4（仮）: 使い回したときは `ch14_` の試験も飛ばす
- D5（仮）: Rust のビルドのキャッシュ（Rust の版までの前方一致）

### 整合の確認から写したもの（`docs/audit/2026-11-17-ch17-fr127-consistency.md`）

- C2: 使い回した配布物が指す commit を空の clone で取れるかを、完了の判定 4 で確かめる（未確認）
- C3: `docs/release.md` §11.1 の下書きの確かめに、使い回した run の見方を足す（tasks 3.2）
- C4: 走っている ST58 は試験の形を 6 つに分ける。保存物を「上げる置き場の全部」で定義し、`--check-workflow` で見る
- K1: 版番号の出所の `Cargo.toml` を鍵に入れることを、Scenario で固定する

### 走っている change との重なり

- ST58（下流）: `ci.yml` の `package-windows` の成果物が増える。どちらが先に入っても、後から入る側が合わせる。ST58 には何も差し戻さない
- CH16（上流は merge 済み）: 鍵から除く 2 つの台本に観測を足す。CH16 の変更だけの PR は、この change の使い回しが効く場面そのもの

### 件数

- capability の前倒し: なし（`distribution` だけ。ADDED 1・MODIFIED 2）
- tasks: 3 Task・6 項目
- spec のレビューの指摘: 13 件
  - escalated 2（R1・R2 → 第 1 回 Q1・Q2 で本人が答えた）
  - fixed 11（spec 5・D1 3・design 1・tasks 1.1 と 2.2 で 2）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
