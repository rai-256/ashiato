# CH10 — エミュレータが起動できないとき android-emulator.sh が無期限に待たない（上流）

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。本人の答えはまだ無い。推奨と違う答えが返ったら、`spec --answers` がその問いだけを写す。

## 何をするか

2026-11-05、ST54 の ci_fix で `tools/android-emulator.sh` が、/dev/kvm を開けずに落ちたエミュレータを `adb wait-for-device` で約 20 分待ち続けた。
この change で、手元でエミュレータを起こす台本が、起こせないときに早く・理由つきで止まるようにする。正常に立つときの振る舞いは変えない。

## 深掘り（第 1 回。5 問とも B・推奨で先に書いた）

- Q1 範囲 → 推奨: 起動と待ちを 1 つの部品 `tools/emulator-start.sh` にし、採寸の台本 `tools/usage-volume.sh` も使う（D1（仮））
- Q2 端末が adb に現れるまでの上限 → 推奨: 120 秒（D2（仮））
- Q3 同じ AVD のエミュレータが既に動いているとき → 推奨: 起こさずに理由つきで止まる（D3（仮））。**正常時の振る舞いが変わる唯一の所**（`KEEP=1` の残りで続けて走らせるには先に止める）
- Q4 待つ相手・止める相手の取り違え → 推奨: 直さない（D4（仮））
- Q5 壊してはいけないもの → 推奨: `docs/changes/CH10.md` の一覧のとおり（D5（仮））

A（止める問い）は無い。

## 仮決め（design.md の（仮））

- D1（仮）〜D5（仮）: 上の Q1〜Q5 の推奨。反転条件は design.md の各 D

聞かずに決めた既定（`premises.md` の C1〜C10 → design の D6〜D15）と、AI の既定 D16（偽の SDK の試験の形）・D17（加速を使えない句で /dev/kvm の行も出す）。

## capability

- 新しい capability `emulator-startup`（開発の道具の約束。`test-hygiene`・`verify-environment` と同じ扱い）。前倒しは無い。MODIFIED は無い

## tasks

6 Task・11 項目。Scenario は 23 本（どれも `tools/test-android-emulator.sh` が印を出す。偽の SDK で本物の SDK も /dev/kvm も要らず、CI の検査の job で毎回走る）。
本物のエミュレータ（手元・kvm あり）での確かめは Task 6（/dev/null を被せて落ちる・上限で止めた後に過程が残らない・同じ AVD・正常に 6 段）。

## レビュー

- 深掘りの問いの独立検査（`review/deep.md`）: 11 件。問いの追加 1・外した問い 4（C へ）・前提の直し
- spec-review（`review/spec.md`）: 16 件（technical 14・premise 1・defer 1）。fixed 15・rejected 1（R12: D17 の語を実物の句に絞り、前提に依らなくした）。人間に返すものは無い
- 走っている Story との重なり: st54 の `tools/android-side-by-side.sh` が同じ形で無期限に待つ（範囲の外。st54 の merge の後に別の change で直す）。`ci.yml`・`docs/testing.md` は st53・st54・st55 と別の行

🤖 Generated with [Claude Code](https://claude.com/claude-code)
