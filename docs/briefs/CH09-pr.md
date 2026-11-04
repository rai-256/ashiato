# CH09 上流 — 実時間で「〜秒以内」を求める試験を混み具合で落ちない形にする

change: `openspec/changes/ch09-load-tolerant-timing-tests/`。依頼・壊してはいけないもの・完了の判定は `docs/changes/CH09.md`。
第 1 回の深掘りの答えを待たずに、推奨の答え（Q0〜Q10）で先に書いた（`spec --draft`）。本人の答え（2026-11-04）が推奨と違ったのは Q2 だけで、`spec --answers Q2` が写した。

## 深掘りで本人が決めたこと

A（`loss` のある問い）は無い。Q0〜Q10 はどれも試験のコードと台本だけに効き、後から書き直せば戻る。全問に推奨を置き、本人は Q2 だけ推奨と違う答えを選んだ:

- Q2（洗い出しの範囲）: **全部探して全部直す**（推奨は「依頼の 3 か所を直し、Windows だけの試験は載せるだけ」）。Windows の実行時テスト・画面の e2e・携帯の計測テスト・縦串と検査の台本まで直し、その確かめもこの change に入る
- 仕様への合意（QA）: 合意する（補足なし）

## 仮決め（B。design の（仮））

- D1（Q1）1 の試験は止めた読み ＋ 見張り 30 秒 ＋ 見回り 1 回に積む件数 ≤ 2,000 で確かめる。帳面の書き込みを待つ退行は捕まえない
- D2（Q8）2 の試験は届かなければ同じ歩を進め直し、10 回の時計の経過を **30 秒未満**で見る（推奨の「例: 60 秒」を、30 秒ごとの読み直しの退行も捕まえる値にした。review/spec.md R5）
- D3（Q2 の本人の答え）直すのは試験と台本の全部（`crates/`・Windows の実行時テスト・`web/src/__tests__`・`web/e2e`・携帯の単体試験と計測テスト・`tools/*.sh`・`tools/smoke.d/*.sh`）。決まった時間の窓は延ばさない。手元で走らない場所は CI を証にする
- D15（Q2・Q5）画面の e2e は playwright の設定で待ち 30 秒・試験 120 秒。試験の中の 30 秒より短い `timeout:` は消す
- D16（Q2・Q5）台本の 30 秒より短い回数の待ちは `SECONDS` の期限 30 秒。30 秒以上の回数の待ちは残す。共有の台本はある行を直すだけ
- D17（Q2・Q5）計測テストの条件の待ち（定数・`Tasks.await`）は 30 秒。Espresso の待ちは変えない
- D4（Q5）条件の待ちは実時間の期限 30 秒。試験の中の時刻を兼ねるループは回数のまま
- D5（Q4）観測できる条件のある固定の眠りは条件の待ちに。無いものは残して載せる
- D6（Q3）打ち切りの試験は相手 60 秒・上限 30 秒。計算の形の試験は直さない
- D7（Q6）`tools/load-repeat.sh`。直す前に再現した負荷を残し、直した後は同じ負荷で 10 回
- D8（Q7）製品の不具合の疑いは直さず一覧に
- D9（Q9）画面の単体試験は待ち 10 秒・試験 30 秒
- D10（Q0）製品のコードの差分 0 行を検査で見る
- D11（Q10）`tools/break-check.sh`。使い捨ての作業木に壊す差分を当てる
- 聞かずに決めたもの（C）: D12（「起きない」の窓・下限は直さず分類）・D13（試験の名前・印・本数を減らさない。期待の行を替える 2 本は `agreed-test-changes.md`）・D14（一覧の置き場と検査）

## capability

- 新: `test-timing`（ADDED 5 Requirement・17 Scenario）。`test-hygiene` の Purpose が一時フォルダに限るので分けた。前倒しは無い
- MODIFIED: `external-ingestion`（見回りの間隔の変更の Scenario の確かめ方。Q8）・`test-database`（statement timeout の Scenario の長さ。Q3）。Requirement の本文は変えない

## tasks

13 Task・22 項目（Q2 で Task 8〜11 の 4 つを足した: Windows の実行時テスト・画面の e2e・携帯の計測テスト・縦串と検査の台本）。直す前の再現（Task 1.2）を、1 と 2 の試験を書き換える前に取る順。

## レビュー

- `review/deep.md`: 7 件（全部 fixed。R6 は D4 D9 仮）
- `review/spec.md`: 14 件 —— fixed 13・rejected 1（R14。前例 `openspec/specs/test-hygiene/spec.md:73` と同じ形で残す）
- `review/spec.md`（`spec --answers Q2` の確かめ）: 8 件（R15〜R22）—— 全部 fixed。e2e の上限の読み方・CI の job の名前・一覧の型を D14 の 8 つに寄せる・録画の台本は載せるだけ・台本の `for` の拾い方・計測テストの `CountDownLatch`・`SETTLE_MS` の起きないを見る窓の分け方・携帯の録画の待ち
- `review/answers.md`: Q2 は写った。答え以外の変更は D3・D15・D16・D17（どれも仮）
- 走っている st53（各ファイルの 1 行目の SPDX だけ）・st55（`runtime.rs` の別の箇所・`vitest.config.ts` の `define`）と同じファイルを触る。差し戻さない

🤖 Generated with [Claude Code](https://claude.com/claude-code)
