# CH13 — 作業箱（コンテナ隔離）の設定を置く（上流）

> 第 1 回の深掘りは推奨の答えで先に書き、2026-11-08 に本人の答え（Q1〜Q4。4 問とも推奨と違う）を `spec --answers Q1,Q2,Q3,Q4` で写した。
> 同じ日の第 2 回の合意で本人が「違和感がある」と返した（harness2 #161 で作業箱が箱の中の dockerd を持つ形に変わった）。この版はそれに合わせて書き直した（`deep.md` の各行の `→ 対応:`）。

## 先に: 作業箱を有効にする前の宿題

一覧の正本は `openspec/changes/ch13-workbox-config/enable-before.md`（下流が同じ本文で ashiato2 の issue を 1 つ立てる）。harness2 #157 / #161 で済んだ項目は一覧から消し、案内だけを置いた。一覧に残るのは次だけ:

- **上流で見つけたこと（有効にする前に直す）**: ホストの `PATH` に `/usr/sbin` が無いと、箱の中の dockerd が `iptables not found` で立たない。グラフを走らせる `hx-dev.service` の `PATH` にも無いので、このまま `tests` にすると、準備（`tools/verify-env.sh`）が毎回 docker を呼ぶので、箱に入る検証は全部（1,786 本のうち 1,751 本）BLOCKED_INFRA になる見込み（下流が harness2 に issue を 1 つ立てる）
- **測っていない**: e2e 1 本が検証の箱の 6g に収まるか（箱の中の DB の容器も数える）・`all` で 5 本のときの空き・agent の箱の中の cargo が 4g に収まるか
- **分類の一覧**: 既存の検証コマンドを harness2 と同じ判定（`host_only`）でホスト / 箱に分けた `command-classes.md`（下流が書き出す）

依頼の完了の判定の pgrep の確かめは tasks 3.1 に置いた: `HARNESS_WORKBOX` を外したシェルから `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` が 10 未満（上流の試しでは `1`。下流が値を書き足す）。

## 何をするか

harness2 の作業箱（#152・#161 で 2026-11-08 に main へ merge 済み）を ashiato2 で有効にできるように、`tools/workbox.json` を置く。中身は**依頼の版に 4 つの欄の直しを当てたもの**（正本は design.md の D1 の JSON）:

- `docker`・`prep_files`: 消した（#161 で「もう効かない」。検証の箱は箱の中の dockerd を持ち、準備も箱の中で走る）
- `docker_images`: `["postgres:17-alpine"]` を足した（箱の中で DB を立てるたびに pull しない）
- `host_only`: `tools/release/(build|install-check).sh`・`tools/android-side-by-side.sh`・`tools/usage-volume.sh` を足した
- ほかの欄は依頼の版と同じ

既定（off）の間、harness2 はこのファイルを読まず、ashiato2 の側も何も読まない。置いただけでは振る舞いは変わらない。

## 深掘り（第 1 回。本人の答え）と合意

- Q1 見つけた不足 → 依頼の「書き換えない」を外す。(1) JSON で直せるものは CH13 で直す（D1）/ (2) 根本は harness2 の issue（#157 / #161 で済んだ。D7）/ (3) 測るものだけを宿題の一覧と issue に（D2）
- Q2 後の Story に箱の制約を知らせるか → この change では何も置かない（D5）
- Q3 pgrep の確かめ → tasks に置く（D3・tasks 3.1）。鍵の確かめ（3.2）と、`host_only` の当たり方の確かめ（D8・tasks 4）も tasks に置く
- Q4 範囲 → 触るのは `tools/workbox.json` と change の中の文書だけ（D4。`skip_specs: true`）
- 仕様への合意: 第 1 回は合意する。第 2 回は違和感がある（harness2 #161 の形に合わせる。(1)〜(5)）→ この版で直した

A（止める問い）は無い。

## 仮決め（design.md の（仮）。台帳 `provisional.jsonl`）

- D9（仮）: 上流で見つけた `PATH` の件を harness2 の issue に 1 つ立て、宿題の一覧にも置く。`tools/workbox.json` の `env` では回避しない（Q1 (2) に従う）
- 前の D6（仮）（docker の正規表現の錨）・D7（仮）（harness2 の 3 つの issue）・D8（仮）（当たり方の確かめ）は台帳で resolve した（D6・D7 は前提が消え、D8 は違和感 (2) で本人が決めた）

## capability

- 無い（仕様を持たない change。`skip_specs: true`）。前倒しは無い

## tasks

5 Task・14 項目（5.4 を足した）。置く（1.1・1.2）/ off の間に何も変わらない（2.1〜2.4）/ pgrep と鍵（3.1・3.2）/ `host_only` の当たり方の確かめと分類の一覧（4.1・4.2）/ PR 本文と issue（5.1 PR 本文・5.2 harness2 に 1 つ・5.3 ashiato2 に 1 つ・5.4 PR 本文に issue の番号）。
docker と作業箱のイメージを要るのは 3.1 だけ。GitHub に出るのは 5.2・5.3（どれも同じ題があれば立てない）。

## レビュー

- 深掘りの問いの独立検査（`review/deep.md`）: 10 件。問いとして本人に渡したもの 4（R1・R6・R7・R8）、宿題・前提・仮決めへ 6
- spec-review（`review/spec.md`）: 第 1 回の先読みで 5 件（R1〜R5）、`spec --answers` の後で 6 件（R6〜R11）、第 2 回の違和感の後で 6 件（R12〜R17。technical 6）。全部 fixed（R2 の処置を escalated に・R9/R10 を仮でない形に直した・5.2 の検証で題を見る・2.4 で 2 節も見る・PATH の件の影響を「箱に入る検証の全部」に直した・5.4 を足した・字句）
- 答えの写し（`review/answers.md`）: Q1〜Q4 とも写った
- 走っている change との重なり: st53・st56・ch10 とも独立（触るファイルが重ならない）。st53 の actionlint と st56 の画面の e2e は、有効にした後は箱の中の dockerd で走る
