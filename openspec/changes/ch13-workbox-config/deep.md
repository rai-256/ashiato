# 深掘り（第 1 回）

> 推奨の答えで先に仕様と GO 資料を書いた（2026-10-20 の流れ）。2026-11-08 に本人の答え（`deep-answers-1.txt`）を受け、全問を本人の答えに書き換えた。
> 4 問とも推奨と違う（Q2・Q4 は推奨と同じ選択肢に補足つき）。仕様へは `spec --answers Q1,Q2,Q3,Q4` で写す。

### Q1. 見つけた不足の一覧をどこに残すか

- 問い: 依頼の中身のままでは、`HARNESS_WORKBOX` を有効にした日に箱の中で落ちる・当たりすぎる検証が見つかりました。この一覧をどこに残し、有効にする前に誰が見るようにしますか。
- 答え: (選択なし)（本人は選択肢を選ばず、補足で答えた。推奨と違う）
- 補足（逐語）: 依頼の「tools/workbox.json を書き換えない」を外す（依頼は本人のもの。差し戻しとして扱ってよい）。見つけた不足を 3 つに分ける。 (1) JSON で直せるものは CH13 の中で直す: docker の正規表現に錨を置き、台本の名前を grep するだけの検証に socket が渡らないようにする（当たりすぎ・R4）/ prep_files に tools/db-roles.sql と docker-compose.override.yml を足す（R5）/ docker の欄に画面の e2e（npm run test:e2e）・cargo test --workspace（と -p ashiato-server --test server_startup）・actionlint の検査を足す（R2 の対症療法）/ host_only に tools/release/(build|install-check)\.sh・tools/android-side-by-side\.sh・tools/usage-volume\.sh を足す（R3）。中で docker を使う台本を呼ぶ台本（load-repeat.sh・st25-seed-check.sh・check-st23-derived-erasure.sh）も docker の欄に足す。 (2) 根本は harness2 側の issue にする: docker を渡すかを検証コマンドの字面で決める設計（中で呼ぶ台本を見ない。後の Story で同じ漏れが繰り返される）/ load_config が知らない鍵を黙って無視する（R9）/ apt を変えるたびに溜まる古いイメージの掃除（R10）。 (3) 測るものだけ有効にする前の宿題として enable-before.md と ashiato2 の issue に残す: e2e 1 本が 6g に収まるか・all で 5 本のときの空き・agent の箱の中の cargo が 4g に収まるか（R6）と、pgrep の確かめ（Q3）。直した項目は一覧から消し、harness2 の issue への案内を置く。

### Q2. 箱の制約を後の Story に知らせるか

- 問い: 後の Story が tasks に書く検証コマンドが、箱の中で docker を受け取れない形（字面に正規表現が当たらない）にならないよう、この change で何か置きますか。
- 答え: この change では何も置かない（本人の選択。補足つきなので推奨と違うものとして扱う）
- 補足（逐語）: 根本は Q1 (2) の harness2 の issue で直す（字面で判定しない形になれば、後の Story に制約を知らせる必要が無くなる）。

### Q3. 完了の判定の pgrep の確かめをどこに置くか

- 問い: 完了の判定の「`workbox.py run . -- bash -c 'pgrep -fc /'` が 10 未満」を、この change のどこで確かめますか。
- 答え: tasks に置き、harness2 の merge まで CH13 を止める（本人の選択。推奨と違う）
- 補足（逐語）: 前提が変わった。harness2 の feat/workbox は 2026-11-08 07:27Z に main へ merge 済み（harness2 #152、a1ef6bcf5）なので止まらない。pgrep の確かめ（HARNESS_WORKBOX を外したシェルから `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` が 10 未満）を tasks に置く。enable-before.md の「0. 先に確かめること」の鍵の突き合わせ（R9）も main の workbox.py で今やる。加えて、 Q1 (1) で直す正規表現は、harness2 と同じ判定（feat/workbox の workbox.py:121 の `re.search(p, cmd)`）で当たる・当たらないを確かめる検証を tasks に置く: docker が要るコマンド（e2e・cargo test --workspace・server_startup・actionlint・中で docker を呼ぶ台本）が docker の欄に当たる / 台本の名前を grep するだけのコマンドが当たらない（錨が効く）/ powershell・エミュレータを使う台本が host_only に当たる。既存の openspec/**/tasks.md の検証コマンドを全部この判定で分類した一覧も enable-before.md に添える。

### Q4. この change が触る範囲（仕様を持つか）

- 問い: この change は `tools/workbox.json` を置くことのほかに、正典の仕様（検証の環境の約束）や文書に手を入れますか。
- 答え: ファイル 1 つだけ。正典にも文書にも手を入れない（本人の選択。補足つきなので推奨と違うものとして扱う）
- 補足（逐語）: 触るのは tools/workbox.json と change の中の文書だけ、は変えない。ただし中身は Q1 (1) のとおり依頼の版から直す。完了の判定の「依頼の JSON と同じ中身」は「依頼の版に Q1 (1) の直しを当てたもの」に変える。

## 第 1 回の答えで決まったこと（record のまとめ。2026-11-08）

- **依頼の「`tools/workbox.json` を書き換えない」を外す**（Q1。本人が依頼の持ち主として差し戻した）。置く中身は「依頼の版に Q1 (1) の直しを当てたもの」（Q4）
- 見つけた不足は 3 つに分ける（Q1）:
  1. JSON で直す（CH13 の中）: docker の正規表現に錨（台本の名前を grep するだけの検証に socket を渡さない。R4）・`prep_files` に `tools/db-roles.sql` と `docker-compose.override.yml`（R5）・docker の欄に e2e（`npm run test:e2e`）・`cargo test --workspace`（と `-p ashiato-server --test server_startup`）・actionlint の検査（R2 の対症療法）・中で docker を使う台本を呼ぶ台本（`load-repeat.sh` `st25-seed-check.sh` `check-st23-derived-erasure.sh`）・`host_only` に `tools/release/(build|install-check)\.sh` `tools/android-side-by-side\.sh` `tools/usage-volume\.sh`（R3）
  2. harness2 側の issue（根本）: docker を渡すかを検証コマンドの字面で決める設計・`load_config` が知らない鍵を黙って無視する（R9）・apt を変えるたびに溜まる古いイメージの掃除（R10）
  3. 有効にする前の宿題（`enable-before.md` と ashiato2 の issue）: e2e 1 本が 6g に収まるか・all で 5 本のときの空き・agent の箱の中の cargo が 4g に収まるか（R6）と pgrep の確かめ。直した項目は一覧から消し、harness2 の issue への案内を置く
- 後の Story に箱の制約を知らせるものは置かない（Q2。根本は harness2 の issue で直す）
- **前提の変化**（Q3 の補足）: harness2 の `feat/workbox` は 2026-11-08 07:27Z に main へ merge 済み（harness2 #152、a1ef6bcf5）。pgrep の確かめ（`HARNESS_WORKBOX` を外したシェルから `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` が 10 未満）を tasks に置く。`enable-before.md` の「0. 先に確かめること」の鍵の突き合わせ（R9）も main の `workbox.py` で今やる
- tasks に、直した正規表現を harness2 と同じ判定（`workbox.py` の `re.search(p, cmd)`）で当てる検証を置く（Q3）: docker が要るコマンドが docker の欄に当たる / 台本の名前を grep するだけのコマンドが当たらない / powershell・エミュレータを使う台本が `host_only` に当たる。既存の `openspec/**/tasks.md` の検証コマンドを全部この判定で分類した一覧を `enable-before.md` に添える
- 触るのは `tools/workbox.json` と change の中の文書だけ、は変えない（Q4）。完了の判定の「依頼の JSON と同じ中身」は「依頼の版に Q1 (1) の直しを当てたもの」に変える

## 仕様への合意（第 1 回）

- QA: 合意する（`agreement.json` に機械が記録）
- 補足（逐語）: Q1 の答えで変わる部分（完了の判定・enable-before.md・tasks）は、書き直した後の版で見る。
