# CH13 — 作業箱（コンテナ隔離）の設定を置く（上流）

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。本人の答えはまだ無い。推奨と違う答えが返ったら、`spec --answers` がその問いだけを写す。

## 先に: 作業箱を有効にする前の宿題

`tools/workbox.json` は依頼の中身のまま置く（書き換えない）。ただし依頼の中身のままでは、`HARNESS_WORKBOX` を `tests` / `all` にした日に落ちる・当たりすぎる検証がある。
一覧の正本は `openspec/changes/ch13-workbox-config/enable-before.md`（下流が同じ本文で issue も 1 つ立てる）。見出し:

0. 先に確かめること —— harness2 の merge の後の鍵の名前の突き合わせと、依頼の完了の判定から外した `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` が 10 未満（**`HARNESS_WORKBOX` を外したシェルから**。Q3）
1. docker が要るのに渡らない（画面の e2e・`cargo test --workspace`・actionlint・中で docker を呼ぶ台本 3 本）
2. ホストが要るのに `host_only` に当たらない（`powershell.exe` を中で呼ぶ release の台本・エミュレータを自分で起こす台本）
3. 当たりすぎ（正規表現に錨が無く、grep するだけの検証にも docker の socket が渡る）
4. `prep_files` の不足（`tools/db-roles.sql`・`docker-compose.override.yml`。`all` のときだけ）
5. 測っていない（e2e 1 本が 6g に収まるか・`all` で 5 本のときの空き・agent の箱の cargo が 4g に収まるか）
6. 後の Story・運用（新しい検証で同じ漏れ・イメージが溜まる）

## 何をするか

harness2 の作業箱（`feat/workbox`。未 merge、確かめた版 `b4797fbbb`）を ashiato2 で有効にできるように、`tools/workbox.json` を依頼の JSON のとおりに置く。
既定（off）の間、harness2 はこのファイルを読まず、ashiato2 の側も何も読まない。置いただけでは振る舞いは変わらない。

## 深掘り（第 1 回。4 問とも B・推奨で先に書いた）

- Q1 不足の一覧の置き場 → 推奨: PR 本文に挙げ、有効にする前の宿題として issue にも残す（D2（仮）。一覧の正本は `enable-before.md`、issue は下流の Task 3.2）
- Q2 後の Story に箱の制約を知らせるか → 推奨: この change では何も置かない（D5（仮））
- Q3 完了の判定の pgrep の確かめ → 推奨: tasks に置かず、PR 本文と宿題に回す（D3（仮））。harness2 の main に無い今は BLOCKED で Task が閉じないため
- Q4 範囲 → 推奨: ファイル 1 つだけ。正典にも文書にも手を入れない（D4（仮）。`.openspec.yaml` の `skip_specs: true`）

A（止める問い）は無い。

## 仮決め（design.md の（仮））

- D2（仮）〜D5（仮）: 上の Q1〜Q4 の推奨。反転条件は design.md の各 D
- D1: 中身は依頼の JSON と「JSON として読んで等しい」で確かめる（依頼が唯一の正本）

## capability

- 無い（仕様を持たない change。`skip_specs: true`。off の間 ashiato2 で観測できる振る舞いが無く、箱の振る舞いの正本は harness2 にある）。前倒しは無い

## tasks

3 Task・8 項目。置く（1.1・1.2）/ off の間に何も変わらない（2.1 読むものが無い・2.2 触ったファイルの範囲・2.3 リポジトリの検査 6 本・2.4 宿題の一覧）/ PR 本文と issue（3.1・3.2）。
docker・エミュレータ・harness2 の作業箱はどれも要らない。3.2 だけが GitHub に出る（同じ題があれば立てない）。

## レビュー

- 深掘りの問いの独立検査（`review/deep.md`）: 10 件。問いの追加 2・問い直し 1・前提の追記・宿題の一覧へ 6
- spec-review（`review/spec.md`）: 5 件（technical 4・conflict 1）。全部 fixed（検証コマンドの backtick・推奨の issue の写し方・PR 本文の pgrep・処置の形・2.2 の作業ツリー）。人間に返すものは無い
- 走っている change との重なり: st53・st56・ch10 とも独立（触るファイルが重ならない）。st53 の actionlint と st56 の画面の e2e は宿題の一覧に載せるだけで、差し戻さない
