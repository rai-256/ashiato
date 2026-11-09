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

## 独立検査の指摘から立てた問い

`review/deep.md` の指摘のうち、本人に問いとして渡したもの（処置は `escalated`）: R1 → Q3 / R6 → Q1（宿題の一覧。答えの (3)）/ R7 → Q1 / R8 → Q4。

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

## 仕様への違和感（第 2 回の合意）

- QA: 違和感がある（`deep-answers-2.txt`。`agreement.json` に機械が記録、status: rejected）
- 補足（逐語）: 前提が変わった。harness2 #161（2026-11-08 13:55Z merge、issue #157）で、作業箱は検証の箱ごとに箱の中だけの rootless の dockerd を持ち、検証の準備（tools/verify-env.sh）も同じ箱の中で走るようになった。字面の正規表現で docker を渡す形は無くなり、`workbox.needs_docker` は消え、`docker` と `prep_files` の鍵は「もう効かない」と出る（知らない鍵は読み込みで止まる）。仕様を次のとおり新しい形に合わせて直す。
  - (1) tools/workbox.json: 依頼の版から `docker` と `prep_files` を消し、`"docker_images": ["postgres:17-alpine"]` を足す。`host_only` の追加分（tools/release/(build|install-check)\.sh・tools/android-side-by-side\.sh・tools/usage-volume\.sh）は残す。ほかの欄は依頼の版のまま。
    → 対応: design D1 の JSON を書き直した（`docker` と `prep_files` を消し、`docker_images: ["postgres:17-alpine"]` を足し、`host_only` の 3 本は残した。ほかの欄は依頼の版のまま）。差分の表も 4 つの欄に直した。tasks 1.2 の検証を、この 4 つの欄だけが変わったことを見る形にした。前の D6（仮）（錨と族の足し算）は前提の欄が消えたので D6 として畳み、台帳で resolve した。`docs/changes/CH13.md` の価値・完了の判定も合わせた
  - (2) 当たり方の確かめ（Task 4.1・4.2）は `workbox.host_only` だけで分ける（`host` / `box` の 2 つ）。`needs_docker` を呼ばない。分類の一覧も 2 つの節にする。
    → 対応: design D8 を `host_only` だけで `host` / `box` に分ける形に書き直し（本人が決めたので（仮）を外し、台帳の D8 は resolve）、tasks 4.1（`needs_docker` を呼ばない）と 4.2（`## ホストで走る（host_only）` / `## 箱で走る` の 2 節）を直した。`match-cases.json` の `want: docker` の 17 行を `box` にした（35 行。D1 で全部一致・依頼の版では足した `host_only` の 4 行が外れることを上流で確かめた）
  - (3) enable-before.md から、#161 で消えたもの（1 の docker が渡らない・3 の当たりすぎ・4 の prep_files の不足・6 の後の Story の漏れと古いイメージ）を消し、harness2 #157 / #161 への案内に置き換える。残すのは、測っていないもの（e2e 1 本が 6g に収まるか・all で 5 本のときの空き・agent の箱の cargo が 4g に収まるか。DB の容器も箱のメモリに数えられるようになった）と、2 のホストが要る台本のうち host_only に足した分の確かめ。
    → 対応: `enable-before.md` を書き直した。#161 で消えたもの（docker が渡らない・当たりすぎ・`prep_files` の不足・後の Story の漏れ・知らない鍵・古いイメージ）は「harness2 #157 / #161 で済んだもの」の節の案内に置き換えた（design D7。第 1 回で立てる予定だった harness2 の 3 つの issue は立てない。台帳の D7 は resolve）。測るもの 3 つ（DB の容器も箱のメモリに数える旨を足した）と、`host_only` に足した台本の確かめの節は残した。加えて、上流で新しく見つけた件（ホストの `PATH` に `/usr/sbin` が無いと箱の中の dockerd が `iptables not found` で立たない。`hx-dev.service` の `PATH` にも無い。premises.md 第 2 回の追記、確かさ 5）を「2. 上流で見つけたこと」に置き、harness2 の issue を 1 つ立てる D9（仮）と tasks 5.2 にした（Q1 (2) の「根本は harness2 の issue」に従う。`tools/workbox.json` の `env` では回避しない —— (5) が `env` を依頼の版のままとしている）
  - (4) pgrep の確かめ（Task 3.1）は残す。準備が無い `workbox.py run` では dockerd は立たないので、10 未満のままでよい。
    → 対応: tasks 3.1 は残した。D1 の中身を指して上流で走らせ `1`（rc=0）。`docker` を先に呼ぶと dockerd が立って `4`（premises.md）。design D3 の「箱の中には docker が無いので落ちる」を、入れ子に箱を起こそうとする形に直した。鍵の確かめ（3.2）は、説明の文字列を見る形から、今の main の `load_config` で読めて `RETIRED` の鍵を持たないことを見る形にした
  - (5) 依頼の版の JSON は差し戻し済みの扱い（Q1 の答え）。変えてよい欄は `docker`（消す）・`prep_files`（消す）・`docker_images`（足す）・`host_only`（後ろに足す）。
    → 対応: design D1 と tasks 1.2 で、変えてよい 4 つの欄（`docker` 消す・`prep_files` 消す・`docker_images` 足す・`host_only` 後ろに足す）の外が依頼の版と等しいことを機械で見る。「依頼（逐語）」は書き換えていない

> 記録の注: この違和感は第 1 回の答えのうち Q1 (1) の docker の正規表現・`prep_files` の直しと、Q3 の `needs_docker` での当たり方の確かめ、Q1 (2) の harness2 の issue（字面の判定・古いイメージ）を、harness2 #161 の形に合わせて置き換える。Q1 (1) の `host_only` の追加・Q1 (3) の測るもの・Q3 の pgrep の確かめ・Q4 の範囲は残る（補足の (1)〜(5)）。直すのは次の `spec`（各行の下に `→ 対応:` を書く）。

## 仕様への合意（第 3 回）

- QA: 合意する（`deep-answers-3.txt`。`agreement.json` に機械が記録、status: accepted）
- 補足（逐語。下流への申し送り）: 2 点だけ下流への申し送り。(1) PATH の件（design D9・enable-before.md の 2.）は harness2 #164（2026-11-09 merge 済み。dockerd-up が rootlesskit を起こすときに PATH の前へ /usr/local/sbin:/usr/sbin:/sbin を足す）で直り、本体（~/dev/harness2）にも取り込み済み。Task 5.2 の題の issue は記録として rai-256/harness2#165 を立てて閉じてあるので、5.2 は新しく立てずにその番号を書く。enable-before.md の 2. は「#164 で済み」として [x] にしてよい。(2) 「e2e 1 本が 6g に収まるか」は 2026-11-08 に実測済み（release ビルドからの e2e login 7 本、箱の memory.peak 2.63GiB、上限 6g。同じ worktree で 2 本同時もホストの空き最小 19.8GB で通った）。宿題の一覧にはこの結果を書き、済みの扱いにする。

> 記録の注: 合意した仕様（digest b07ef5af…）は変えていない（変えると合意が外れる）。申し送りは下流が扱う。(1) の 5.2 は、いまの手順（同じ題の issue が `--state all` で既にあればその番号を書く）のままで #165 を書く（題の一致を 2026-11-09 に確かめた。CLOSED）。enable-before.md の 2. の [x] と、(2) の実測の結果を宿題の一覧に書くのも下流（Task 2.4 の検証が見る文言は消さない）。
