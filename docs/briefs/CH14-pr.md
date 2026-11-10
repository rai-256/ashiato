# CH14 — インストーラの合言葉の問いが文字化けする（上流）

> 第 1 回の深掘りの答えを待たずに、推奨の答えで仕様を先に書いた（2026-10-20 の流れ）。2026-11-10 の本人の答えで、推奨と違った Q2 を `spec --answers Q2` で仕様へ写した（下の「第 1 回の答えで直したこと」）。

## 深掘りで本人が決めること（第 1 回。全部 B（仮でよい）・推奨あり）

- Q1 直す範囲 —— 推奨: 依頼どおり 2 つ（問いの書き方を直す・最初の問いの前に条件を示す。検査の中身は変えない）→ design D1（仮）
- Q2 子の命令の出力の化け —— **本人の答え: 表示も値の口も含める**（推奨「含めない」と違う）→ D2。読み方は D8（仮）、確かめ方は D9（仮）
- Q3 条件の示し方 —— 推奨: 最初の問いの前に 1 回だけ一覧を出し、断りに条件を添える → D3（仮）
- Q4 本物の窓で確かめる場所 —— 推奨: Windows の CI ＋ 確認バッチで 1 回見る → D6（仮）・D7（仮）
- Q5 問いの文をどこへ書くか —— 推奨: 窓へ直に書く（`CONOUT$` に `WriteConsoleW`）→ D4（仮）

失われるもの（`loss`）のある問い（A）は無い。

## 第 1 回の答えで直したこと（Q2）

- spec: `distribution` に ADDED の Requirement「インストーラは、子の命令の出力を窓のコード頁で読み、Windows の文字コードで表せる名前なら英字でなくても止まらない」と Scenario 9 本を足した（MODIFIED の Requirement と 11 本は変えていない）
- design: D2 を本人の答えに書き直し、D8（仮）（UTF-8 として正しければそのまま、そうでなければ窓のコード頁で読む。読めない並びは U+FFFD）と D9（仮）（Windows の CI の `package-windows` で `ch14_` の単体の試験・install-check に英字でない名前の管理者の repair）を足した。D7 の反転条件の tasks の番号の参照を直した
- tasks: Task 5（子の出力の読み）・Task 6（Windows の CI で確かめる）を足し、文書・全体の確かめを Task 7・8、人間の確認待ちを 9.1 に付け直した
- premises: 子の命令は引き継いだ窓のコード頁で書く（本人の PC で確かめた。確かさ 5）と、runner の `chcp 932`・日本語の利用者名・PostgreSQL の突き合わせ（確かさ 1。CI が確かめ、外れたら D9 の反転条件）
- 合意: 答え以外の変更は D7・D8・D9（どれも（仮））に依る（`review/answers.md`）

## 仮決め（B）

D1・D3・D4・D6〜D9 が（仮）。どれも反転条件を design.md に書いてある。聞かずに決めた既定は D5（条件の文言は検査の定数から組む。review/deep.md R7）。

## 仕様の形

- capability の前倒し: 無い。`distribution` の Requirement「インストールの最後に、端末で画面の合言葉を決め、そのままログインできる」を MODIFIED にした。今の SHALL 5 行と Scenario 5 本は逐語で残し、SHALL を 4 行、Scenario を 6 本足した（インストールの最後・`password`・`repair` の 3 つの口に効く）
- 費用の見えるところ: Q5 の推奨を採ると、インストーラに `unsafe` の小さなモジュール（`console.rs`）と `windows-sys` の直の依存（rpassword が既に使う版）が入る
- 前提: 疑似コンソール（ConPTY）の試験の形は、本人の PC（Windows 11）で確かめた。親の出力が管でも、子は窓につながる。rpassword と同じ書き方では、本人が見たのと同じ『逕ｻ髱｢縺ｮ蜷郁ｨ闡』が出た（premises.md 9 行目。台本と結果は `premise-probe/`）。windows-latest の runner では未確認なので、`install-windows` のインストールの前の step で先に確かめる
- tasks: 8 Task・14 項目と、人間の確認待ち 1

## レビュー

- deep-review（問い）: 8 件。fixed 7・rejected 1（R8: 第 1 回は範囲を問う決まりなので Q1 を残した）
- spec-review（spec --answers Q2 の観点 6 を含む）: R11〜R26 の 16 件を足し、どれも fixed（D8・D9（仮）に関わるものを含む）。最初の 10 件（technical 10。R1 は premise で出たが、ConPTY の前提を本人の PC で確かめた後に、レビューの席が technical に判定し直した）。処置は fixed 10（うち仮 3: D6・D7・D1）。内訳は review/spec.md
