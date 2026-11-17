# CH16 — CI の Windows の試験の判定が落ちたら、何が起きていたかを必ずログに出す

> 軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いてある。何を・なぜ・どこまでは受け皿 #307 の議論で本人が決めてある
> （`docs/changes/CH16.md` の「議論で決めたこと」。聞き直さない）。製品の振る舞いは変えず、試験の台本が落ちたときに出すものだけを足す。

## Why

ST58 の関門で、CI の Windows のインストールの試験の判定が落ちても、ログには `NG: <理由>` の 1 行しか出なかった。
何が起きていたかを見るためだけに観測を足して CI をもう 1 周（約 1 時間）回し、その観測で、製品ではなく判定のほうの誤り
（`sc query` の出力の 4 行目にある状態を読めていない）だと分かった。最初から観測が出ていれば 2 周短くできた。

いまの `tools/release/install-check.sh` と `tools/release/update-check-install.sh` の `die` は `NG: $*` を出して終わるだけで、
判定が見た命令の出力（サービスの状態・設定・healthz・命令の記録）は捨てられる（`sc.exe qc … | grep -q … || die …` の形が多い）。
観測を出しているのは、待ちの時間切れの `service_diagnostics` など一部だけ。

## What Changes

- 2 つの台本の判定が NG になると、`NG: <理由>` の行に続けて、**その判定が見た観測**を CI のログ（標準エラー）に出す。
  観測は 3 つ: 判定が渡した値・直近に走らせた観測の命令の生の出力・共通の様子（3 つのサービスの状態と構成・healthz の番号・
  設定のキーの名前・命令の記録とサーバのログの末尾）。出し方は `die` に組み込み、判定ごとに書かせない（design D1〜D3）
- 観測に秘密の値を出さない（設定の秘密・試験の合言葉・ログインの印の値を伏せる。design D4）
- 観測を出さない判定を機械で落とす検査 `tools/release/check-ng-observations.sh` を足し、2 つの台本の `--self-test` から自分に当てる
  （CI はもう 2 つの `--self-test` を `chain` の job で走らせているので、`ci.yml` は変えない。design D5）
- 観測の塊の Windows の部分が壊れていないことを、CI の実地の確かめの始めに本物の命令で一度確かめる（design D7）
- `set -e` で判定の外の命令が落ちて止まる道は対象の外（design D8）
- 判定が OK / NG を出す条件・終了コード・標準出力（`== Scenario:` の行と `--list`）は変えない

## Capabilities

### New Capabilities

- `install-check-diagnostics`: Windows のインストールの試験の台本が、判定が落ちたときに何を CI のログに出すかの約束（開発の道具）。
  製品の capability（`distribution` など）には触れない

### Modified Capabilities

なし

## Impact

- 触れる予定のパス（`lane.json`）: `tools/release/install-check.sh`・`tools/release/update-check-install.sh`・
  `tools/release/check-ng-observations.sh`（新しい。`tools/release/check-*.sh` に当たる）・`docs/testing.md`（任意）・
  `docs/changes/CH16.md`・この change。`.github/workflows/ci.yml` は予定に入っているが、変えない見込み（design D5）
- 触れないもの: 製品のコード（`crates/`・`collector-android/`・`web/`）・CI の他の job・`tools/release/` のほかの台本・正典の `distribution`
- 走っている change との重なり: **ST58（`st58-update-apply-rollback`）**。
  - 今（2026-11-17、`origin/feat/st58-update-apply-rollback` の 33c2fed45）の ST58 の枝は、`install-check.sh` にも `update-check-install.sh` にも触れていない
    （`tools/release` で触れるのは `check-artifacts.sh`・`package.sh`・`update-apply-install.sh` と `ci.yml`）。ST58 が後から 2 つの台本に触れたら、
    後に merge する側が rebase で合わせる
  - ST58 が足す `tools/release/update-apply-install.sh` は main に無く、`lane.json` の予定のパスの外なので、**この change の検査の対象にしない**
    （検査は対象の台本を名指しで受け取る。design D5）。ST58 はその台本に自分で観測を出す `die` を持っている

## 持ち越し

- **ST58 の merge の後、`update-apply-install.sh` を `check-ng-observations.sh` の名指しに足すか**（本人の決定の「install 系の台本」に入る 3 つ目）。
  この change では決めない（main に無い台本で、予定のパスの外）。PR 本文に載せ、ST58 の merge の後に change として登録するかを人間が決める。
  走っている ST58 へは差し戻さない
