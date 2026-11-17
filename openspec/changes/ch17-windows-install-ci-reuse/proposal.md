# CH17 — Windows のインストールの試験の 1 周を縮める（台本だけの変更ではビルドし直さない）

> 軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いてある。製品のコードと配布物の作り方は変えず、
> CI の Windows の 2 つの job（`package-windows`・`install-windows`）の回し方だけを変える。

## Why

ST58 の関門で、CI の 1 周が約 1 時間かかり、13 時間で 12 周した。内訳は `package-windows` のビルド 33 分・`install-windows` 17 分。
最後の 3 周は試験の台本（`tools/release/install-check.sh` など）しか変えていないのに、毎回 Windows の配布物をビルドし直していた。
本人は受け皿の依頼 #308 で「CI の中で縮める」と決めた（手元の Windows を試験に使う形は、並列とコンテナ化を保つためにやめた）。

## What Changes

- `package-windows` が、組み立ての入力から**組み立ての鍵**を計算する。鍵が同じ前の組み立ての配布物（配る形と試験の形の 2 つ）が
  GitHub Actions のキャッシュにあれば、ビルドせずにそれを同じ名前の成果物として上げる。無ければ今と同じにビルドし、鍵で保存する（design D1〜D4）
- 鍵は「試験の台本と文書だけを除いた、追跡している全部のファイルの中身」と道具の版から作る。除くものは名指しし、名指ししていない
  ファイルが変われば鍵が変わる（ビルドし直す側に倒れる。design D1）
- 鍵を出す台本 `tools/release/windows-build-key.sh` を置き、自己試験を持たせる（`package-windows` で鍵を使う前に走らせる）
- `package-windows` がビルドするときは Rust のビルド（`~/.cargo` と `target`）をキャッシュから戻す（design D5）
- `install-windows` は今と同じ成果物を受け取り、インストールと更新の Scenario を全部走らせ、欠けたら落ちる（変えない）
- `docs/testing.md` に、Windows の 2 つの job が使い回す条件と、使い回しを止める方法を書く

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `distribution`: 要件を 1 つ足す（ADDED のみ。既存の Requirement は変えない）。「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」の
  3 つの Scenario（コードを触った PR で組み立ての job が走る・文書だけの PR で飛ぶ・配る形を壊した PR が落ちる）は、使い回しの後も成り立つ
  （鍵が組み立ての入力の全部を含むので、配る形を壊す変更は必ず鍵を変え、ビルドし直して落ちる。design D1）

## Impact

- 触れる予定のパス（`lane.json`）: `.github/workflows/ci.yml`・`tools/release/windows-build-key*.sh`・`docs/testing.md`（と、要れば `docs/adr/*.md`）。
  `tools/release/changes.sh` は予定に入っているが触らない（`code` / `deps` の判定は `check-vulns.sh --self-test` が固定している。design D1）
- 変えないもの: `changes` の job の出し分け（`code` / `windows` / `android` / `stamp` / `deps`）・`package-windows` と `package-artifacts` の
  job の `if:` と checkout する commit（`tools/release/check-guards.sh workflows` が見る）・`ch14_` の試験の step が最初の
  `package.sh windows` より前にあること（`tools/check-ci-ch14-step.sh`）・成果物の名前（`package-windows-release` / `package-windows-test`）・
  `install-windows` の手順・他の job・製品のコード・配布物の作り方（`tools/release/build.sh` / `package.sh`）
- 走っている change との重なり: `openspec/changes/` で archive 前なのはこの change だけ（2026-11-17）。`ci.yml` を同時に触る change は無い
- 使い回した run の `release-package`（`package-artifacts` の成果物。`tools/release/draft.sh` の下書きが使う）には、使い回した Windows の
  配布物が入る。その印の commit は、ビルドした前の commit になる（中身は同じ入力から組んだもの。design D3・Risks）
- 費用: GitHub Actions のキャッシュの置き場（repo あたり 10 GB、7 日使われないと消える）を、配布物と Rust のビルドで使う。分は使わない
