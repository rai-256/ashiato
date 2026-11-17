## CH18 — 作業箱の設定に圧縮ライブラリと CPU の上限を足す

軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いた。ashiato2 の振る舞いは変えず、harness2 が読む `tools/workbox.json` だけを直す。

### 何を
- `apt` の末尾に `liblzma-dev`・`libbz2-dev` を足す（箱の中でサーバの試験を組み立てると `-llzma` / `-lbz2` のリンクで止まるため）
- `"cpus": 6` を置く（箱 1 つの CPU の上限。`parallel_verify` 2 × 6 で、16 コアのうちホスト側に 4 を残す）
- ほかの欄は変えない。`HARNESS_WORKBOX` は設定しない（hx-dev.service に足すのは merge の後に人間が行う）

### 仕様
- 仕様の delta を持たない（`.openspec.yaml` の `skip_specs: true`）。作業箱が off の間、ashiato2 の側で観測できる振る舞いは何も変わらず、正典に作業箱の capability は無い（CH13 の D4 と同じ扱い）
- 壊してはいけないもの（ほかの欄・off の間）と直したこと（箱の中の包み・CPU の上限・全体テスト）は tasks の検証で固定する
- 1.3（箱の中の `cpu.max` と包み）は、直す前に FAIL を記録してから直す（上流で base の箱が `max 100000`・包み無しで落ちることを確かめた）

### 件数
- capability の前倒し: なし
- tasks: 2 Task・5 項目
- 仮決め: なし
- spec のレビューの指摘: 7 件（全部 technical・fixed。1.3 が証跡に載らない形だったのを直した・FAIL の記録を必須に・2.1 の一時フォルダの後始末・試験が落ちたときは直さず人間へ・`docs/briefs/CH18*` を許す・hx-dev.service は最後のレビューの席が見る・整形のバイト比較）

### 走っている change との重なり
- ch16・ch17・st58 は `tools/workbox.json` に触れない。有効にした後は、それらの検証も CPU 6 本の箱で走る

🤖 Generated with [Claude Code](https://claude.com/claude-code)
