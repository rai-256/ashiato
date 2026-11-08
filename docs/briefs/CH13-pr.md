# CH13 — 作業箱（コンテナ隔離）の設定を置く（上流）

> 第 1 回の深掘りは推奨の答えで先に書き、2026-11-08 に本人の答え（Q1〜Q4。4 問とも推奨と違う）を `spec --answers Q1,Q2,Q3,Q4` で写した。

## 先に: 作業箱を有効にする前の宿題

一覧の正本は `openspec/changes/ch13-workbox-config/enable-before.md`（下流が同じ本文で ashiato2 の issue を 1 つ立てる）。JSON で直せる不足はこの change で直したので、一覧に残るのは次だけ:

- **測っていない**: e2e 1 本が検証の箱の 6g に収まるか・`all` で 5 本のときの空き・agent の箱の中の cargo が 4g に収まるか
- **harness2 の issue**（根本。下流が 3 つ立てる）: docker を渡すかを検証コマンドの字面で決めている・`load_config` が知らない鍵を黙って無視する・`apt` を変えるたびに古いイメージが溜まる
- **分類の一覧**: 既存の検証コマンドを harness2 と同じ判定で分けた `command-classes.md`（下流が書き出す）

依頼の完了の判定の pgrep の確かめは tasks 3.1 に置いた: `HARNESS_WORKBOX` を外したシェルから `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` が 10 未満（上流の試しでは `1`。下流が値を書き足す）。

## 何をするか

harness2 の作業箱（#152 で 2026-11-08 に main へ merge 済み）を ashiato2 で有効にできるように、`tools/workbox.json` を置く。中身は**依頼の版に Q1 (1) の直しを当てたもの**（正本は design.md の D1 の JSON）:

- `docker`: 呼び出しの位置に錨を置いた（台本の名前を grep するだけの検証に docker の socket を渡さない）。画面の e2e・`cargo test --workspace` / `server_startup`・`actionlint`・中で docker を使う台本を呼ぶ台本 3 本を足した
- `host_only`: `tools/release/(build|install-check).sh`・`tools/android-side-by-side.sh`・`tools/usage-volume.sh` を足した
- `prep_files`: `tools/db-roles.sql`・`docker-compose.override.yml` を足した
- ほかの欄は依頼の版と同じ

既定（off）の間、harness2 はこのファイルを読まず、ashiato2 の側も何も読まない。置いただけでは振る舞いは変わらない。

## 深掘り（第 1 回。本人の答え）

- Q1 見つけた不足 → 依頼の「書き換えない」を外す。(1) JSON で直せるものは CH13 で直す（D1・D6（仮））/ (2) 根本は harness2 の issue（D7（仮））/ (3) 測るものだけを宿題の一覧と issue に（D2）
- Q2 後の Story に箱の制約を知らせるか → この change では何も置かない。根本は harness2 の issue で直す（D5）
- Q3 pgrep の確かめ → tasks に置く。harness2 は merge 済みなので止まらない（D3・tasks 3.1）。鍵の突き合わせ（3.2）と、直した正規表現の当たり方の確かめ（D8（仮）・tasks 5）も tasks に置く
- Q4 範囲 → 触るのは `tools/workbox.json` と change の中の文書だけ（D4。`skip_specs: true`）

A（止める問い）は無い。仕様への合意（第 1 回）: 合意する（補足: Q1 の答えで変わる部分は書き直した後の版で見る）。

## 仮決め（design.md の（仮）。台帳 `provisional.jsonl`）

- D6（仮）: 正規表現の錨の形と、答えが名指ししていない同じ族（`npx playwright test`・絞り込みなしの `cargo test -p ashiato-server` と裸の `cargo test`・`docker` を直に呼ぶもの）を足す（Q1 に従う）
- D7（仮）: harness2 の issue は直す場所ごとに 3 つ（Q1 に従う）
- D8（仮）: 当たり方の確かめは change の中の表 `match-cases.json` と台本で、harness2 の `host_only`・`needs_docker` をそのまま呼ぶ（Q3 に従う）

## capability

- 無い（仕様を持たない change。`skip_specs: true`）。前倒しは無い

## tasks

5 Task・14 項目。置く（1.1・1.2）/ off の間に何も変わらない（2.1〜2.4）/ pgrep と鍵（3.1・3.2）/ 当たり方の確かめと分類の一覧（4.1・4.2）/ PR 本文と issue（5.1 PR 本文・5.2 harness2 に 3 つ・5.3 ashiato2 に 1 つ。宿題の一覧が 3.1・4.2 の値で埋まってから立てる）。
docker と作業箱のイメージを要るのは 3.1 だけ。GitHub に出るのは 5.2・5.3（どれも同じ題があれば立てない）。

## レビュー

- 深掘りの問いの独立検査（`review/deep.md`）: 10 件。問いとして本人に渡したもの 4（R1・R6・R7・R8）、宿題・前提・仮決めへ 6
- spec-review（`review/spec.md`）: 第 1 回の先読みで 5 件（technical 4・conflict 1）。全部 fixed
- 答えの写し（`review/answers.md`）: spec-review の観点 6。Q1〜Q4 とも写った。答え以外の変更は D6〜D8（仮）だけ
- spec-review（`spec --answers` の後）: 4 件（technical 4）。全部 fixed（5.1 の否定の検証を終了コード 3 で・2.4 の grep を項目ごとに・issue を立てる Task を最後へ・裸の `cargo test` を docker の欄に）
- 走っている change との重なり: st53・st56・ch10 とも独立（触るファイルが重ならない）。st53 の actionlint と st56 の画面の e2e は、この版で docker の欄に当たる
