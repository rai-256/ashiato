# CH13 — 作業箱（コンテナ隔離）の設定を置く

> 第 1 回の深掘りの推奨の答えで先に書き、2026-11-08 に本人の答え（Q1〜Q4。4 問とも推奨と違う）を `spec --answers` で写した。
> 同じ日の第 2 回の合意で本人が「違和感がある」と返した（harness2 #161 で作業箱の形が変わった。`deep.md` の「仕様への違和感（第 2 回の合意）」）。この版はそれに合わせて書き直した。
> `docs/changes/CH13.md` の「価値」「完了の判定」「壊してはいけないもの」も、この段で合わせた（「依頼（逐語）」は書き換えない）。

## Why

並走する Story と change が同じ機械で検証を走らせ、共有の /tmp を試験の漏れが埋め（10-23）、メモリ不足で langgraph が OOM kill され（10-25）、
試験の `pgrep -f "/"` が利用者の全プロセスに当たった（11-07 CH10）。harness2 は検証と agent を thread ごとのコンテナ（作業箱）で走らせる仕組みを
用意した（#152 で 2026-11-08 に main へ merge 済み）。ashiato2 で使うには、repo に箱の設定 `tools/workbox.json` が要る。本人が 2026-11-08 に手元で確かめた中身がある。

上流の調べで、依頼の中身のままでは箱を有効にした日に落ちる・当たりすぎる検証が見つかった。本人は依頼の「書き換えない」を外した（Q1）。
その後 harness2 #161（issue #157）で、検証の箱は箱の中だけの dockerd を持ち、検証の準備も箱の中で走る形になった。docker の正規表現と `prep_files` で直す必要は無くなり、
依頼の `docker` と `prep_files` の鍵は「もう効かない」と出るようになった（違和感 (1)）。

## What Changes

- **`tools/workbox.json` を、依頼の版に直しを当てた中身で置く**（design D1。正本は D1 の JSON）。変えるのは 4 つの欄だけ: `docker` と `prep_files` を消す・`"docker_images": ["postgres:17-alpine"]` を足す・`host_only` に Windows とエミュレータを中で使う台本 3 本を足す（違和感 (1)(5)・Q1 (1)）。
  置いただけでは何も変わらない: harness2 は `HARNESS_WORKBOX` が `tests` か `all` のときだけ読み、既定は off（premises.md の 1 行目）
- **`host_only` が harness2 と同じ判定で当たる・当たらないことを確かめる**（Q3・違和感 (2)。D8）: change の中の表 `match-cases.json` と、`.harness2/scripts/workbox.py` の `host_only` をそのまま呼ぶ台本。既存の `openspec/**/tasks.md` の検証コマンドを全部、ホスト / 箱に分けた一覧 `command-classes.md` も書き出す
- **依頼の完了の判定の pgrep の確かめと、鍵の確かめを tasks に置く**（Q3・違和感 (4)。D3）。鍵は今の main の `load_config` で読み、もう効かない鍵を持たないことを見る
- **harness2 の issue**: 第 1 回で立てる予定だった根本の 3 つは harness2 #157 / #161 で済んだので立てない（D7）。上流で見つけた新しい件（ホストの `PATH` に `/usr/sbin` が無いと箱の中の dockerd が立たない）を 1 つ立てる（D9（仮）。下流の Task 5.2）
- **測るものだけを有効にする前の宿題に残す**（Q1 (3)・違和感 (3)。D2）: `enable-before.md`（archive の後も repo に残る）と PR 本文と ashiato2 の issue（下流が 1 つ立てる）。#161 で消えた項目は一覧から消し、#157 / #161 への案内を置く
- **仕様を持たない**（Q4。D4）。`.openspec.yaml` に `skip_specs: true`。正典の仕様にも、`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md` にも手を入れない
- **やらないこと**: 後の Story に箱の制約を知らせる規約や検査を置くこと（Q2。D5）、harness2 側を直すこと、メモリの前提を測ること

## Capabilities

### New Capabilities

（無い）

### Modified Capabilities

（無い）

仕様を持たない理由: `HARNESS_WORKBOX` が off の間、ashiato2 の側で観測できる振る舞いは 1 つも変わらない。箱の振る舞いの正本は harness2 の `scripts/workbox.py` で、
ashiato2 の正典に Requirement を置けば Scenario の印を持つ試験か台本が要り、触る範囲を越える（Q4）。当たり方の確かめは change の中の表と台本が持つ（D8）。
CH10 の `emulator-startup` が「止まるまでの時間・終了コード・文言は外から確かめられる振る舞い」として `skip_specs` を採らなかったのと逆で、ここでは ashiato2 の外から確かめられる振る舞いが無い。
`openspec validate --strict` は `.openspec.yaml` の `skip_specs: true` で差分 0 を受け付け、`openspec archive --yes`（`scripts/archive.sh` の呼び方）も同じ印で通る（2026-11-08 に openspec 1.12.0 の使い捨ての木で確かめた）。
深掘りの独立検査の R8 の処置にある「skip_specs はこのハーネスに無い」は、openspec のこの印を見落としていた（ハーネスの台本は使っていないが、openspec が読む）。

## Impact

- 道具: `tools/workbox.json`（新しいファイル 1 つ）。ashiato2 の台本・コード・CI はこのファイルを読まない（`git grep -i workbox` が 0 件。2026-11-08）
- change の中: `enable-before.md`・`match-cases.json`（上流）、`check-workbox-match.py`・`command-classes.md`（下流）
- 文書: 変えない（change の中と `docs/changes/CH13.md` だけ）
- 製品のコード・API・移行・画面・CI: 変えない
- 外向きの作用: 下流が GitHub の issue を立てる —— harness2 に 1 つ（D9（仮））、ashiato2 に 1 つ（題 `作業箱を有効にする前の宿題（CH13）`。D2）。どれも同じ題があれば立てない
- 依存: harness2 の main（#161 の `d65ff3b4a` 以降）の `scripts/workbox.py` と `scripts/evidence.py`。判定の関数（`host_only`）・設定の読み込み（`load_config` `KEYS` `RETIRED`）と取り出し（`items` `declared` `unwrap_quiet`）を下流の台本が呼ぶ。Task 3.1 は docker と作業箱のイメージと、ホストで pull する `postgres:17-alpine` を要る
- **走っている Story・change との重なり**（`openspec/changes/` の archive 以外）:
  - st53（`st53-license-bundle`）: 触るファイルは重ならない。tasks の workflow の検査（`actionlint "$f" || docker run rhysd/actionlint …`）は、有効にした後は箱の中の dockerd で走る（イメージは箱の中で pull する。`docker_images` には入れていない）。st53 へは何も差し戻さない
  - st56（`st56-collector-connection-setup`）: 触るファイルは重ならない。tasks 5.x の画面の e2e（`cd web && npm run test:e2e`）は、有効にした後は箱の中の dockerd で DB を立てて走る。差し戻さない
  - ch10（`ch10-emulator-start-timeout`）: 触るファイルは重ならない。`tools/android-emulator.sh` は `host_only` に当たり、新しく足す `tools/test-android-emulator.sh` は当たらない（偽の SDK の試験で、ホストは要らない。`match-cases.json` に置いた）。差し戻さない
  - どれも off の間は影響しない。merge の順は決めない
