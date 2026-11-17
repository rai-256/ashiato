# CH17 — Windows のインストールの試験の 1 周を縮める（台本だけの変更ではビルドし直さない）

> 軽い路（`lane.json` の `reason: same`）で始めたが、spec のレビュー（R1・R2）で正典 `distribution` の本文を改める必要が分かり、
> 通常の路へ移った。深掘り第 1 回（2026-11-17）で本人が Q1・Q2 に答えた（`deep.md`）。製品のコードと配布物の作り方は変えず、
> CI の Windows の配布物の job（`package-windows`）の回し方だけを変える。

## Why

ST58 の関門で、CI の 1 周が約 1 時間かかり、13 時間で 12 周した。内訳は `package-windows` のビルド 33 分・`install-windows` 17 分。
最後の 3 周は試験の台本（`tools/release/install-check.sh` など）しか変えていないのに、毎回 Windows の配布物をビルドし直していた。
本人は受け皿の依頼 #308 で「CI の中で縮める」と決めた（手元の Windows を試験に使う形は、並列とコンテナ化を保つためにやめた）。

## What Changes

- `package-windows` が、組み立ての入力から**組み立ての鍵**を計算する。鍵が同じ前の組み立ての配布物（配る形と試験の形の 2 つ）が
  GitHub Actions のキャッシュにあれば、ビルドせずにそれを同じ名前の成果物として上げる（job が上げる置き場の全部を保存して引く）。無ければ今と同じにビルドし、鍵で保存する（design D1〜D4）
- 鍵は「試験の台本と文書だけを除いた、追跡している全部のファイルの中身」と道具の版から作る。除くものは名指しし、名指ししていない
  ファイルが変われば鍵が変わる（ビルドし直す側に倒れる。design D1）
- 鍵を出す台本 `tools/release/windows-build-key.sh` を置き、自己試験を持たせる（`package-windows` で鍵を使う前に走らせる）
- `package-windows` がビルドするときは Rust のビルド（`~/.cargo` と `target`）をキャッシュから戻す（design D5）
- `install-windows` は今と同じ成果物を受け取り、インストールと更新の Scenario を全部走らせ、欠けたら落ちる（変えない）
- 使い回した Windows の配布物の `SOURCE.txt` と組み立ての印は、それを組んだ commit（同じ PR の前の push の head か main の commit）を指したまま上げる
  （深掘り Q2 の本人の答え。`package-artifacts` は変えない。design D3）
- `docs/testing.md` に、Windows の 2 つの job が使い回す条件と、使い回しを止める方法を書く。`docs/release.md` §11.1 の下書きの確かめに、使い回した run の見方を足す

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `distribution`: 要件を 1 つ足し（ADDED）、2 つを改める（MODIFIED。深掘り第 1 回 Q1・Q2 の本人の答え。FR-127 ★ 2026-11-17）
  - 「コードを触った PR ごとに、配る形の 3 部品を公開せずに組む」: Windows 向けは、鍵が同じ前の組み立てがあれば使う。置き場は CI の実行の成果物と CI のキャッシュ（公開しない）。
    3 つの Scenario は名前も中身も変えない（使い回しの後も成り立つ。鍵が組み立ての入力を含むので、配る形を壊す変更はビルドし直して落ちる）
  - 「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」: 使い回した Windows の配布物の `SOURCE.txt` と印は組んだ commit を指す。
    `package-artifacts` が head から取る期待値（版番号・`LICENSE`）と合うのは、`Cargo.toml` と `LICENSE` が鍵に入っているから。既存の Scenario は名前も中身も変えず
    （どれも「head を checkout して組んだとき」か「`SOURCE.txt` と印が同じ commit」の話で、使い回しの後も成り立つ）、使い回した配布物の Scenario を 1 つ足す

## Impact

- 触れるパス: `.github/workflows/ci.yml`・`tools/release/windows-build-key.sh`・`docs/testing.md`・`docs/release.md`（と、要れば `docs/adr/*.md`）。
  `tools/release/changes.sh` は触らない（`code` / `deps` の判定は `check-vulns.sh --self-test` が固定している。design D1）。
  正典の Scenario の名前を変えないので、印（`notices.sh`・`check-guards.sh`・`check-artifacts.sh`）にも触れない
- 変えないもの: `changes` の job の出し分け（`code` / `windows` / `android` / `stamp` / `deps`）・`package-windows` と `package-artifacts` の
  job の `if:` と checkout する commit（`tools/release/check-guards.sh workflows` が見る）・`ch14_` の試験の step が最初の
  `package.sh windows` より前にあること（`tools/check-ci-ch14-step.sh`）・成果物の名前（`package-windows-release` / `package-windows-test`）・
  `install-windows` の手順・他の job・製品のコード・配布物の作り方（`tools/release/build.sh` / `package.sh`）
- 走っている change との重なり（2026-11-17 の `origin/main`）:
  - **ST58（`st58-update-apply-rollback`。下流）**: `ci.yml` の `package-windows` で試験の形の成果物を 1 つから 6 つに分け、`install-windows` が読む試験の台本
    `tools/release/update-apply-install.sh` を足す。この change は保存物を「`package-windows` が上げる置き場の全部」で定義し、`--check-workflow` がそれを見る（design D2）ので、
    後から入る側が `ci.yml` を合わせれば保存物も揃う（揃わなければ `--check-workflow` が落ちる）。`update-apply-install.sh` は鍵から除く一覧に先回りして足さない
    （それだけを変えた PR はビルドし直す側に倒れる。足すかは ST58 の後に D1 の反転条件で決める）。ST58 には何も差し戻さない
  - **CH16（`ch16-install-check-ng-observations`。上流が main に入った）**: `install-check.sh`・`update-check-install.sh` に観測を足し、`tools/release/check-ng-observations.sh` を足す。
    前の 2 つは鍵から除く台本で、CH16 の変更だけの PR はこの change の使い回しが効く場面そのもの。新しい台本は除く一覧に無いのでビルドし直す側に倒れる
- 使い回した run の `release-package`（`package-artifacts` の成果物。`tools/release/draft.sh` の下書きが使う）には、使い回した Windows の
  配布物が入る。その `SOURCE.txt` と印の commit は、それを組んだ commit になる（本人の決定 Q2。spec の MODIFIED・design D3）
- 費用: GitHub Actions のキャッシュの置き場（repo あたり 10 GB、7 日使われないと消える）を、配布物と Rust のビルドで使う。分は使わない
