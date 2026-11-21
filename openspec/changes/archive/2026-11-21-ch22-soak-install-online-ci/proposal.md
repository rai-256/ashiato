# CH22 — 配る形を外につながる状態で入れて置き、落ち続けや誤りの続きを PR の CI で捕まえる

> 通常の路（`lane-verdict.json` の `lane: normal`。正典 distribution の約束を MODIFIED で改め、深掘りで決めることが残っていた）。
> 何を・なぜ・どこまでは受け皿 #332 の議論で本人が決めてある（`docs/changes/CH22.md` の「議論で決めたこと」。聞き直さない）。
> 深掘り第 1 回（Q1〜Q5・Q7）は推奨の答えで先に書いている（`deep.md` の「答え待ち」）。

## Why

0.1.0 は、入れて放っておくだけで更新係が 10 秒ごとに落ちた（HTTPS の握手で根を指定せず panic した。#329、commit 97a43bcf6）。
CI の Windows のインストールの試験は、ST57 D18 で job の最初から更新係の外向きを塞ぎ、偽の受け手だけで試していたので、
配る組み立ての更新係は 1 度も本物の配布元と握手せずに配られた。普通に入れて置いておけば気づける不具合を、PR の CI が機械で捕まえるようにする。

## What Changes

- **置く段を足す**: CI の `install-windows` の、インストールと更新の試験の後の段（置く段）で、前の段の入れたもの（サービス・プログラムの置き場・状態の置き場・更新係の置き場）と
  PC の収集アプリの置き場を消し、更新係の外向きの塞ぎを外してから、配る形のサーバの zip を入れ、配る形の PC の収集アプリを設定せずに起こし、
  5 分置く（深掘り Q5・Q1。design D1〜D3・D7）
- **置く段の判定**（どれかが外れたら job を落とす。外れを許す一覧を持たない。Q7・design D12）:
  - 3 つのサービス（DB・サーバ・更新係）が置く間に 1 回でも起こし直された・止まった、置き終わりに実行中でない・healthz が 200 でない（Q2・D4・D5）
  - 1 つのログに同じ誤りの行が 3 回以上ある（Q3）・サーバのログに panic の行が 1 行でもある（D6（仮））
  - 更新係の確かめた記録に、置く段の始まりより後の「確かめた」が 1 行も無い（理由が全部「問い合わせの上限」なら警告だけ。Q4・D13（仮））
  - 設定せずに置いた PC の収集アプリが配布元へ 1 回でも問い合わせる（宛先なしのほかの記録の行がある）・送り先が設定されている（Q1。2026-11-21 本人の決定で変更・D7）
  - PC の収集アプリが、台本の止めた 1 回のほかに終わる・止めてから 150 秒以下で起こし直した・起こし直した後に現れない・置き終わりにいない（Q1・D7）
- 最初の走りで反転条件に当たったら、CH22 の中で spec・design・tasks を直す（design D15）
- 置く段の出力は、インストールの試験の台本と同じ観測の部品（CH16）で秘密を伏せてから出す（公開の CI のログに出る。design D9）
- 置く段と `install-windows` の job に時間の上限を付ける（design D8（仮））
- **正典を改める**: distribution の「更新を確かめる問い合わせは、版の一覧を取る決まった形だけを配布元へ送る」の最後の SHALL
  （「自動の試験は本物の配布元に問い合わせない。配る組み立てを動かす CI は、外への接続を塞いで走らせる」）を、
  本物の配布元へ繋ぐ自動の試験を名指しの 2 つ（握手の試験 `tls_online_*` と置く段）に限る形に改める（design D10）
- 製品のコード（`crates/`・`collector-android/`・`web/`）は変えない

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `distribution`: Requirement を 5 つ ADDED（置く段の順序と上限・サービスの起こし直し・誤りの行の続き・更新係が配布元に確かめた記録・PC の収集アプリ（設定せずに置き、問い合わせ 0 件））。
  既存の CI の約束（CH17 の「PR の CI は、組み立ての入力が同じ Windows の配布物を使い回し、インストールの試験は全部走らせる」）と同じ capability に置く。
  Requirement 1 つを MODIFIED（上の正典の改め。既存の Scenario 9 本は変えず、CI の定義を読む Scenario を 2 本足す。塞ぐ対象は今の D18 と同じ更新係の外向きに揃える（design D10））

## Impact

- 触れる予定のパス: `tools/release/soak-install.sh`（新しい）・`.github/workflows/ci.yml`（`install-windows` に置く段・job と段の時間の上限・
  D18 の注記の「job の終わりまで外さない」を「置く段で外すまで」に、`rust` の job に `soak-install.sh --self-test`）・`docs/testing.md`・
  `docs/changes/CH22.md`・この change。`lane.json` の予定のパス（`.github/workflows/ci.yml`・`tools/**`・`openspec/specs/distribution/spec.md`・`docs/**`）の中
- 触れないもの: 製品のコード・`tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/update-apply-install.sh`・
  `tools/release/check-ng-observations.sh`（使うだけ）・`collector-windows-runtime` の job の `tls_online_*` の手順（そのまま。正典の側を合わせる）
- **Windows の組み立ての鍵**: 新しい台本 `tools/release/soak-install.sh` は、CH17 の鍵から除く名指し（`install-check.sh`・`update-check-install.sh`・
  `conpty-run.ps1`）に入らないので、置く段の台本だけを直した push でも Windows の配布物を組み直す。CH17 の Requirement は改めずに受け入れる（design D1（仮））
- **CI の時間**: `install-windows`（今は約 61 分。2026-11-19 の run 44455083173）に、置く段の 5 分と入れ直しの 1〜2 分が足される。Windows の runner は 2 倍で数える
- **本物の配布元への問い合わせ**: 1 回の run で、握手の試験が 2 回・置く段が更新係 1 回（PC の収集アプリは送り先を設定しないので 0 回）。共有の runner の IP ごとの上限（1 時間 60 回）に当たったときは警告だけ（D13（仮））
- 走っている change との重なり: 2026-11-19 時点で `openspec/changes/` に走っているのは CH19（`tools/android-emulator.sh` だけを触る）。重なりは無い
- 文書の中の古い約束: `.github/workflows/ci.yml:607-608` の D18 の注記（「この規則は job の終わりまで外さない」）。ほかに同じ約束を言い直している文書・台本は無い（`grep -rn "本物の配布元\|問い合わせない"` で確かめた範囲）

## 持ち越し

- 長く置かないと出ない問題（毎晩 30 分置く形）は、要るようになったら別の change で足す（依頼の「やらないこと」）
- 本物の答えの形の比べ（CH20 の中身）はやらない
- 置く段が最初の走りでいまの不具合を見つけたら、段を緩めず、直しは別の change にする（Q7・design D12）
