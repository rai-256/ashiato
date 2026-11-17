# CH18 — 作業箱の設定に圧縮ライブラリと CPU の上限を足す

> 軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いてある。ashiato2 の振る舞いは変えず、harness2 が読む作業箱の設定 1 つだけを直す。

## Why

作業箱（`HARNESS_WORKBOX=tests`。CH13 で `tools/workbox.json` を置いた）は、まだ有効になっていない。2026-11-17 に箱の中で
`cargo test -p ashiato-server` を組み立てると `rust-lld: error: unable to find library -llzma` / `-lbz2` で止まった（圧縮の依存が動的にリンクする）。
このまま有効にすると、その日からサーバの試験が全部落ちる。また全体テストは、同時に重いコマンドが走っていると遅くなっていた
（30 分を超えた回は同時に平均 0.83 本、10 分以内の回は 0.01 本）。機械は 16 コアで、箱 2 つ（`parallel_verify` の既定）× 6 本にしてホスト側（agent・langgraph）に残す。

## What Changes

- `tools/workbox.json` の `apt` の末尾に `liblzma-dev`・`libbz2-dev` を足す
- `tools/workbox.json` に `"cpus": 6` を置く（検証の箱 1 つの CPU の上限。harness2 の `scripts/workbox.py` が `docker run --cpus 6` にする）
- ほかの欄は変えない（`parallel_verify` は 2 のまま）。`HARNESS_WORKBOX` は設定しない（hx-dev.service に足すのは merge の後に人間が行う）
- 仕様を持たない（`.openspec.yaml` の `skip_specs: true`）。理由: `HARNESS_WORKBOX` が off の間、ashiato2 の側で観測できる振る舞いは何も変わらず、
  有効にした後の振る舞いの正本は harness2 の `scripts/workbox.py`。ashiato2 の正典には作業箱の capability が無い（CH13 の D4 と同じ扱い）。
  壊してはいけないもの（ほかの欄・off の間）と、直したことの確かめ（箱の中の包み・CPU の上限・全体テスト）は tasks の検証が固定する

## Capabilities

### New Capabilities

なし

### Modified Capabilities

なし

## Impact

- 触れる予定のパス（`lane.json`）: `tools/workbox.json`・`docs/changes/CH18.md`・`openspec/changes/ch18-workbox-libs-cpus/**`
- 読むもの: harness2 の `scripts/workbox.py`（`load_config` が鍵を確かめ、`apt` をイメージに足し、`cpus` を `--cpus` にする）。イメージは `apt` が変わると作り直される（初回の箱の起動が数分延びる）
- 触れないもの: 製品のコード・試験・CI・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`・`openspec/specs/`
- 走っている change との重なり: `ch16-install-check-ng-observations`・`ch17-windows-install-ci-reuse`・`st58-update-apply-rollback` は `tools/workbox.json` に触れない（2026-11-17 に change の中を `workbox` で探して 0 件）。
  有効にした後は、どれの検証も箱の中で CPU 6 本の上限で走る（この change が merge され、人間が有効にしてから）。差し戻すものは無い
