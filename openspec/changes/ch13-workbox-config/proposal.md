# CH13 — 作業箱（コンテナ隔離）の設定を置く

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば、`spec --answers` がその問いだけを写す。`docs/changes/CH13.md` の「価値」「壊してはいけないもの」「完了の判定」も、この段で推奨の答えに合わせて書いた。

## Why

並走する Story と change が同じ機械で検証を走らせ、共有の /tmp を試験の漏れが埋め（10-23）、メモリ不足で langgraph が OOM kill され（10-25）、
試験の `pgrep -f "/"` が利用者の全プロセスに当たった（11-07 CH10）。harness2 は検証と agent を thread ごとのコンテナ（作業箱）で走らせる仕組みを
`feat/workbox` に用意した。ashiato2 で使うには、repo に箱の設定 `tools/workbox.json` が要る。本人が 2026-11-08 に手元で確かめた中身がある。

## What Changes

- **`tools/workbox.json` を依頼の中身のとおりに置く**（書き換えない。依頼の指定）。置いただけでは何も変わらない: harness2 は `HARNESS_WORKBOX` が `tests` か `all` のときだけ読み、既定は off（premises.md の 1 行目）
- **有効にする前の宿題の一覧を残す**（Q1 の推奨。D2（仮））。依頼の中身のままでは、箱を有効にした日に docker を受け取れずに落ちる検証（画面の e2e・`cargo test --workspace`・workflow の検査など）、
  ホストが要るのに箱に入る検証、当たりすぎて docker の socket を受け取る検証、測っていないメモリの前提が見つかった。中身は書き換えず、一覧を
  `openspec/changes/ch13-workbox-config/enable-before.md`（archive の後も repo に残る）に置き、PR 本文と issue（下流が 1 つ立てる）にも残す
- **依頼の完了の判定のうち「`workbox.py run . -- bash -c 'pgrep -fc /'` が 10 未満」を、この change の確かめから外す**（Q3 の推奨。D3（仮））。harness2 の main に `feat/workbox` が無い今は
  証跡が BLOCKED になって Task が閉じず、CH13 が harness2 の merge まで下流で止まる。この確かめは宿題の一覧に載せる
- **仕様を持たない**（Q4 の推奨。D4（仮））。`.openspec.yaml` に `skip_specs: true`。正典の仕様にも、`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md` にも手を入れない
- **やらないこと**: 依頼の欄に足りないものを足すこと（依頼の差し戻しになる）、後の Story に箱の制約を知らせる規約や検査を置くこと（Q2 の推奨。D5（仮））、harness2 側を直すこと

## Capabilities

### New Capabilities

（無い）

### Modified Capabilities

（無い）

仕様を持たない理由: `HARNESS_WORKBOX` が off の間、ashiato2 の側で観測できる振る舞いは 1 つも変わらない。箱の振る舞いの正本は harness2 の `scripts/workbox.py` で、
ashiato2 の試験で Scenario として固定できない（harness2 の main に無い）。CH10 の `emulator-startup` が「止まるまでの時間・終了コード・文言は外から確かめられる振る舞い」として
`skip_specs` を採らなかったのと逆で、ここでは外から確かめられる振る舞いが無い。
`openspec validate --strict` は `.openspec.yaml` の `skip_specs: true` で差分 0 を受け付け、`openspec archive --yes`（`scripts/archive.sh` の呼び方）も同じ印で通る（2026-11-08 に openspec 1.12.0 の使い捨ての木で確かめた）。
深掘りの独立検査の R8 の処置にある「skip_specs はこのハーネスに無い」は、openspec のこの印を見落としていた（ハーネスの台本は使っていないが、openspec が読む）。

## Impact

- 道具: `tools/workbox.json`（新しいファイル 1 つ）。ashiato2 の台本・コード・CI はこのファイルを読まない（`git grep -i workbox` が 0 件。2026-11-08）
- 文書: 変えない（change の中の `enable-before.md` と `docs/changes/CH13.md` だけ）
- 製品のコード・API・移行・画面・CI: 変えない
- 外向きの作用: 下流が GitHub の issue を 1 つ立てる（題 `作業箱を有効にする前の宿題（CH13）`。同じ題があれば立てない。Q1 の推奨。design D2（仮））
- 依存: harness2 の `feat/workbox`（未 merge。確かめた版 `b4797fbbb`）。merge までに鍵の名前が変わると、このファイルの欄は黙って効かなくなる（harness2 の `load_config` は知らない鍵を検査しない）。宿題の一覧に載せた
- **走っている Story・change との重なり**（`openspec/changes/` の archive 以外）:
  - st53（`st53-license-bundle`）: 触るファイルは重ならない。tasks の workflow の検査（`actionlint "$f" || docker run rhysd/actionlint …`）は、箱を有効にすると docker を受け取れずに落ちる側に入る。宿題の一覧に載せるだけで、st53 へ差し戻さない
  - st56（`st56-collector-connection-setup`）: 触るファイルは重ならない。tasks 5.x の画面の e2e（`cd web && npm run test:e2e`）が同じく宿題の一覧に入る。差し戻さない
  - ch10（`ch10-emulator-start-timeout`）: 触るファイルは重ならない。`tools/android-emulator.sh` は依頼の `host_only` に当たり、新しく足す `tools/test-android-emulator.sh` は当たらない（偽の SDK の試験で、ホストは要らない）。差し戻さない
  - どれも off の間は影響しない。merge の順は決めない
