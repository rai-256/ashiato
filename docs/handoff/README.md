# 申し送り（handoff）—— 走っている Story へは差し戻さない

`docs/handoff/ST<NN>.md` は、**issue ができて凍結された Story** について、
他の Story のレビューや深掘りが見つけた事を置く場所。

## なぜ要るか（実測 2026-09-11）

ST03 の上流が ST02 の下流に「差し戻し 5 件」を送った結果、ST02 は tasks に 15 章を新設して
再実装し、独立レビューを 2 巡目としてかけ直し、深掘りが第 9 回まで伸びた。ST03 の側も
移行の番号をずらす PR と「ST02 が merge された事実に合わせる」PR を出した。**12 時間で 5 往復。**
ST03 の tasks 13.2 の検証は「ST02 側がまだ直っていないことを grep で確かめる」だった ——
他 Story の未完了を自分の完了条件にしていた。

## 規則

1. **issue ができた時点で、その Story の `tasks.md` は凍結。** `review_triage.py` は
   `deferred ST<NN>` の先に `openspec/changes/st<NN>-*/tasks.md` があれば FAIL にする
2. 走っている Story について見つけた事は、次のどちらか
   - **(i) 見つけた Story 自身の change で直す**（自分で直せるなら。`処置: fixed …`）
   - **(ii) 先行の merge 後に `fix/<slug>` の小さな change で拾う** —— `処置: followup ST<NN>` にして、
     ここ（`docs/handoff/ST<NN>.md`）に書く。先行の issue にもコメントで 1 行残す
3. **例外は A（失われるもの）だけ。** 先行 Story のデータが失われる型（`loss` が付く）なら、
   tasks ではなく先行 Story の deep の問い（`premise`）として立てる
4. 下流は handoff を **2 回だけ**読む —— 開始時（`/story` Step 2）と PR 前（Step 5）。途中では入れない

## 書式

`review_triage.py` は「この change 名と R 番号が書かれていること」だけを見る。

```markdown
# ST02 への申し送り

## st03-idempotent-ingest R57 — 「① 記録あり」を core.event から引く
- 何を: `coverage.rs` の「記録あり」を `coverage.event_count > 0` ではなく `core.event` から引く
- なぜ今入れないか: ST02 は issue #19 で凍結。走っている Story へ差し戻すと往復が生まれる
- いつ・どこで: ST02 の merge 後に `fix/coverage-has-record` で。担当は見つけた側（ST03）
- 根拠: openspec/changes/st03-idempotent-ingest/review/deep-r5.md R57
```
