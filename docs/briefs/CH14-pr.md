# CH14 — インストーラの合言葉の問いが文字化けする（上流）

> 第 1 回の深掘りの答えを待たずに、推奨の答えで仕様を先に書いた（2026-10-20 の流れ）。本人の答えが推奨と違えば、`spec --answers` がその問いだけを仕様へ写す。

## 深掘りで本人が決めること（第 1 回。全部 B（仮でよい）・推奨あり）

- Q1 直す範囲 —— 推奨: 依頼どおり 2 つ（問いの書き方を直す・最初の問いの前に条件を示す。検査の中身は変えない）→ design D1（仮）
- Q2 子の命令の出力の化け —— 推奨: 含めない（別の change にする）→ D2（仮）。見つけた口は proposal の Impact に書いた
- Q3 条件の示し方 —— 推奨: 最初の問いの前に 1 回だけ一覧を出し、断りに条件を添える → D3（仮）
- Q4 本物の窓で確かめる場所 —— 推奨: Windows の CI ＋ 確認バッチで 1 回見る → D6（仮）・D7（仮）
- Q5 問いの文をどこへ書くか —— 推奨: 窓へ直に書く（`CONOUT$` に `WriteConsoleW`）→ D4（仮）

失われるもの（`loss`）のある問い（A）は無い。

## 仮決め（B）

D1〜D4・D6・D7 が（仮）。どれも反転条件を design.md に書いてある。聞かずに決めた既定は D5（条件の文言は検査の定数から組む。review/deep.md R7）。

## 仕様の形

- capability の前倒し: 無い。`distribution` の Requirement「インストールの最後に、端末で画面の合言葉を決め、そのままログインできる」を MODIFIED にした。今の SHALL 5 行と Scenario 5 本は逐語で残し、SHALL を 4 行、Scenario を 6 本足した（インストールの最後・`password`・`repair` の 3 つの口に効く）
- 費用の見えるところ: Q5 の推奨を採ると、インストーラに `unsafe` の小さなモジュール（`console.rs`）と `windows-sys` の直の依存（rpassword が既に使う版）が入る
- 前提: 疑似コンソール（ConPTY）の試験の形は、本人の PC（Windows 11）で確かめた。親の出力が管でも、子は窓につながる。rpassword と同じ書き方では、本人が見たのと同じ『逕ｻ髱｢縺ｮ蜷郁ｨ闡』が出た（premises.md 9 行目。台本と結果は `premise-probe/`）。windows-latest の runner では未確認なので、`install-windows` のインストールの前の step で先に確かめる
- tasks: 6 Task・10 項目と、人間の確認待ち 1

## レビュー

- deep-review（問い）: 8 件。fixed 7・rejected 1（R8: 第 1 回は範囲を問う決まりなので Q1 を残した）
- spec-review: 10 件（technical 10。R1 は premise で出たが、ConPTY の前提を本人の PC で確かめた後に、レビューの席が technical に判定し直した）。処置は fixed 10（うち仮 3: D6・D7・D1）。内訳は review/spec.md
