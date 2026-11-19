# CH20 — 配る形の更新の確認を、本物の配布元で毎晩確かめる

> 軽い路（`lane.json`）で申告したが、入口で当たらず（`lane-verdict.json` の `entry.why`）、通常の路に移った。
> 深掘り第 1 回（Q1〜Q10。`deep.md`）は**推奨の答えで先に書いた**（答え待ち）。本人の答えが推奨と違えば、その問いだけを写し直す。

## Why

更新の確認（サーバの更新係と PC の収集アプリ）の自動の試験は、どれも手元の偽の配布元を相手にしている。PR の CI の
`install-windows` は配る組み立ての更新係の外向きを塞いで走る（ST57 の D18）。そのため、**本物の配布元（`api.github.com` の
`rai-256/ashiato` のリリース）と組み合わせた振る舞いは、一度も機械で試されていない**。問いを立てる中で、偽の配布元と本物の配布元が
すでにずれていることが 3 つ見つかった（`premises.md` P9〜P11。下の「見つかった食い違い」）。

2026-11-19 に本人が配る形を使ったときに見た「更新の確認のエラー」の原因は、まだ分からない（P5）。本人の Windows には、確かめた記録が
1 行も無い。記録から見ると、配布元の答えから来たエラーではない見込みが高い。**この change は、そのエラーを直すことも捕まえることも
約束しない**（直すのは別の change。深掘り Q1・Q9）。約束するのは、本物の配布元と食い違えば翌朝までに機械が赤で知らせることだけ。

## What Changes

- 毎晩の検査を 1 つ足す（`.github/workflows/release-source-nightly.yml`。配る物の組み立てと公開の `nightly.yml` とは別の workflow。design D1）。
  入口は台本 `tools/release/release-source-check.sh`。次の 3 つを 1 回の実行で見る:
  1. **本物の今の状態**: 配る形のサーバ（インストーラと更新係）を Windows の runner に入れ、更新係に本物の配布元へ 1 回確かめさせる。PC の収集アプリは
     試験の組み立てで本物に 1 回当てる。開発版の道（印 `nightly`）は、試験の組み立ての更新係に開発版の版を与えて 1 回当てる（深掘り Q2・Q3・Q7）
  2. **本物が見せない状態**: その晩に本物から取った答えを手元の受け手から流し直す。版の名前だけを書き換えて「最新が自分と同じ版」「新しい版がある」を作り、
     2 つの部品の試験の組み立てに通す。`rai-256/ashiato` にリリースが無い間は、別の公開の repo の最新のリリース 1 件の形に、自分の配布物の名前を差し込む（Q4・Q6）
  3. **答えの形**: 本物の答えのうち製品が読む項目（状態コード・転送の行き先・本文の大きさ・道ごとの答えの形・各リリースの `tag_name`・`draft`・`assets[].name`・
     `browser_download_url`・`digest`）を、repo に置く基準（偽の配布元が返している形）と比べる（Q5）
- 判定: 2 つの部品がそれぞれの状態で正典どおりの結果を記録すれば緑。食い違えば赤。配布元に届かなかった夜は注意だけを出し、3 晩続けば赤（Q8）。
  開発版の道の `404` は正典どおり（「配布元が見つからない」）なら緑で、要約に「依頼と正典の食い違い」と出す（Q7）
- 1 回の実行で、部品が資格情報なしで本物に問い合わせるのは 3 回まで。検査の側の問い合わせ（本物の答えの取得・別の repo）は workflow の資格情報で 4 回に限る（design D4）。
  本物の答え・部品の記録・要約は成果物に 90 日残す（design D13（仮））
- 正典 `distribution` の「自動の試験は本物の配布元に問い合わせない」を、「PR の CI は問い合わせない。毎晩の検査だけは 1 回の実行で数回まで問い合わせてよい」に改める（Q10。MODIFIED）
- 台本の自己試験（記録した本物の答えと偽の受け手で、本物に問い合わせずに判定を確かめる）を PR の CI の `chain` の job で走らせる
- `docs/testing.md` と `docs/release.md` に、毎晩の検査の読み方（赤の理由・注意・artifact の場所）を書く

### 見つかった食い違い（この change では直さない。深掘り Q9）

| | 食い違い | この検査で捕まえる時 |
|---|---|---|
| (1) | 印 `nightly` の道の本物の答えはリリース 1 件のオブジェクト。Rust の 2 部品は配列しか読まず、偽の配布元は配列を返す（P9） | **初日から赤**。答えの形の比べ（別の repo の 1 件の形と、基準の「偽の配布元は配列」）と、開発版の流し直し（更新係と、開発版の版で組んだ PC の収集アプリ）の両方で |
| (2) | Rust の 2 部品は本文を 64 KiB までしか読まない（正典に無い決まり）。配布物 4 つのリリースは 8 件ほどで超える（P10） | **初日から赤**。流し直しに「一覧がいっぱい（20 件）」の場合を置き、別の repo の 1 件の形から 20 件の一覧を組んで通す（design D6（仮）） |
| (3) | PC の収集アプリは `5xx` と別の宛先への転送を「配布元に届かない」と記録する。正典は「読めない答え」（P11） | **初日から赤**。流し直しに `503` と別の宛先への `301` の場合を置く（design D6（仮））。本物が `5xx` を返した夜も同じ判定で見る |

直しの change が入るまで、毎晩の検査は赤のままになる（Q9 の推奨を採ったときの帰結）。

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `distribution`:
  - MODIFIED「更新を確かめる問い合わせは、版の一覧を取る決まった形だけを配布元へ送る」: 本物に問い合わせないのを PR の CI に限り、毎晩の検査だけは本物に問い合わせてよいと改める（Q10）。
    既存の 9 つの Scenario は名前も中身も変えない
  - ADDED「毎晩、本物の配布元との組み合わせを確かめる」ほか 3 つ（design と specs）

## Impact

- **触れるパス**: `.github/workflows/release-source-nightly.yml`（新）・`.github/workflows/ci.yml`（`chain` の job に自己試験と配線の検査の 2 行だけ）・
  `tools/release/release-source-check.sh`（新）・`tools/release/release_source.py`（新。答えの形の比べと流し直しの受け手）・
  `tools/release/release-source-check.d/`（新。自己試験の材料: 記録した本物の答え）・`docs/contracts/release-source-shape.json`（新。比べる基準）・
  `crates/collector-windows/tests/ch20_real_source.rs`（新。PC の収集アプリに 1 回確かめさせる入口。既定では走らない）・
  自己試験の材料の元は、上流で記録した本物の答え（`openspec/changes/ch20-real-release-source-nightly-check/material/`。2026-11-19 の `rai-256/ashiato` の一覧・印 `nightly` と `cli/cli` の 1 件の 2 つの道）・
  `docs/testing.md`・`docs/release.md`・`docs/changes/CH20.md`
- **触れないもの**: 製品のコード（`crates/update-check/**`・`crates/updater/src/**`・`crates/collector-windows/src/**`・`collector-android/**`）・
  `nightly.yml`・`release.yml`・`vulns-nightly.yml`・`ci.yml` の `install-windows` の job（外向きを塞ぐ D18 を含む）・
  既存の偽の配布元の試験と共有の材料（`docs/contracts/update-check-vectors.json`・`update-apply-vectors.json`）・
  `tools/release/update-check-install.sh`・`install-check.sh`・`build.sh`・`package.sh`
  （`lane.json` の申告の `crates/update-check/**` は製品のコードなので触れない。review/deep.md R14）
- 要件: `docs/requirements.md` の FR-116（「配布元へ版の一覧を取る問い合わせ以外の情報を送らない」）は、誰が問い合わせるかを言っておらず変わらない。
  改めるのは正典の Requirement の 1 つ（本文の 1 文を書き換え、上限の 1 文と Scenario 1 つを足す。既存の 9 つの Scenario は変えない）
- 走っている change との重なり: 無い（2026-11-19 の `openspec/changes/` に、archive 以外はこの change だけ）
- 費用: 毎晩 Windows の runner 1 本・40〜60 分（配る形の組み立てを含む。CH17 のキャッシュは PR の CI の鍵で、この workflow は使わない）。
  公開しない（権限は `contents: read`・`actions: read` だけ）
