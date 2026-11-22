## Why

版を手で上げる決まりは重くて使われず、2026-11-19 に同じ 0.1.0 で中身の違う組み立てができて、直した版をインストーラが入れられなかった（依頼 #352）。
main に merge された変更に機械が番号を付け、番号ごとに 1 つの組み立てを残す形にして、「同じ番号で中身の違う組み立て」を作れなくする（FR-134・FR-104・NFR-25）。

## What Changes

- **番号を付ける commit**（deep.md Q1。答え待ち・推奨で書いた）: main への push の CI の run が成功で終わり、前に番号を付けた commit から製品（`docs/`・`openspec/`・`*.md` 以外）が変わっていれば、その commit に次の番号を付ける。
  取り消された・落ちた run の中身は、次に番号が付く commit の番号に入る。
  **FR-134 の今の本文（「merge のたびに」）とずれる。** Q1 の答えが推奨のままなら、record の段で FR-134 を ★ 訂正で「製品が変わった後の main の、CI が通った commit に」と直す（この段では requirements.md を直さない）
- **番号の決め方**（Q3。推奨: 確認バッチが入ったら真ん中を上げる）: 左の数字は repo のファイル `tools/release/MAJOR` を本人が PR で変える。真ん中と右は機械が決める。最初の番号は 0.2.0（今の 0.1.0・0.1.1 より大きい。FR-117・FR-137）
- **番号の記録と組み立ての置き場**: 番号は印 `build-<番号>`（その commit を指す）で記録し、番号ごとに組み立てを配布元の**下書きのリリース**に全部残す（D1（仮））。携帯の APK は署名していないものも一緒に残す（D2（仮））。公開はしない（公開は ST62 の札）
- **携帯の versionCode の式を変える**（Q2。推奨: 桁を広げる `左×10,000,000 + 真ん中×10,000 + 右`）。**BREAKING**（式の変更。配った APK はまだ無い —— premises P9）
- **番号を持たない組み立て**（Q4。推奨: 0.0.x）: PR・手元の組み立ては出所の版 `0.0.1`、CI の更新の試験の組み立ては `0.0.2`〜`0.0.5` を名乗る。根の `Cargo.toml` の版は `0.0.1` にする。**BREAKING**（今の出所の版 0.1.1 は番号を持たない組み立ての版でなくなる）
- **移る間（ST62・ST63 まで）**: 印 `v<番号>` は、その番号を付けた commit を指すときだけ公開する。開発版の作り直しは main の HEAD でなく最新の番号の commit を `<番号>-dev.<日付>` で組む
- **main の CI の差分の基準**: main への push の CI は、前に成功で終わった main の run の commit からの差分で出し分ける（取り消された merge のコードを、番号を付ける前の CI が試す）
- **REMOVED**: 「緊急の修正の枝は、配った版から切り、移行と封筒の形を変えない」。番号は main にだけ付くので、枝の上の印は番号を持てない（FR-105 ★ 2026-11-21 で要件から外れた）

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `distribution`: 版番号の出所・versionCode の式・印の判定・開発版の作り直し・確かめの下書きの版を書き直し、番号付け（付ける commit・決め方・記録と置き場・流れ）と番号を持たない組み立ての版・main の CI の差分の基準を足す。CI の更新の試験の組み立ての版を書く 8 つの Requirement の Scenario の版を 0.0.x に直す。緊急の修正の枝を外す

capability の照合（`docs/stories/INDEX.md`）: 版番号の進め方は `distribution` に割り当て済み（ST61 の行）。capability の前倒しは無い。
**要件の前倒し**（review/spec.md R12）: 移る間の開発版（最新の番号の commit を組む。D11）は FR-107 の新しい本文（ST63 の担当）の一部を先に実装する。修正の枝を外すのは、FR-105 の未決「急ぎの直しの出し方」（ST62 の担当）を決めるものではなく、ST62 が決めるまでの間は main で直して次の番号を出す手順にしておくだけ（docs/handoff/ST62.md に、未決が開いたままであることを書く）。どちらも、番号の形の上で今の開発版と印の組み立てを緑に保つため（premises の C7）。
**Story の題と完了の判定**（review/spec.md R11）: 題の「merge するたびに」と完了の判定 1 つ目は、Q1 の推奨（製品が変わった後の、CI が通った commit に付ける）とずれる。record の段で、FR-134 の ★ 訂正と一緒に `stories.json` から直す（design D4 の「要件へ戻すもの」）。

## Impact

- **コード**: `tools/release/version.sh`（版の検査・versionCode・番号付け・印の判定・開発版の判定）・`tools/release/version-table.tsv`・新しい台本（番号の記録と置き場）・`tools/release/draft.sh`・`tools/release/update-apply-install.sh`・`tools/check-build-stamp.sh`・`tools/check-openapi.sh`・`crates/build-stamp/src/lib.rs`・`collector-android/app/build.gradle.kts`・`web/build-stamp.ts`・根の `Cargo.toml`（版 0.0.1）・`docs/openapi.json`（`info.version`）・新しいファイル `tools/release/MAJOR`
- **CI**: `.github/workflows/ci.yml`（main の差分の基準・更新の試験の版）・新しい workflow（番号付け）・`release.yml`（印の判定）・`nightly.yml`（最新の番号）
- **文書**: `docs/production-prep.md` の「バージョンの単一情報源」・配る手順（左の数字を変える手順）
- **走っている change との重なり**: `openspec/changes/` に走っている change は無い（2026-11-22）。後続の ST62（札）・ST63（開発版の札）・ST64（札の無い組み立ての守り）はこの Story の番号と下書きの置き場を前提にする
- **できなくなること**: 修正の枝からの正式版（REMOVED。配った版はまだ無い —— P9）。印 `v*` を番号の無い commit に付けての公開
- **正典に残す「修正の枝」の言葉**: 配った移行の検査（「配った版にある移行は…」の祖先の印だけを数える文と Scenario「修正の枝の上では、枝の祖先の印を数える」）と record-envelope の導出元の「緊急の修正の枝」は残す。祖先の印だけを数える振る舞いは修正の枝が無くても正しく、ST62 が急ぎの直しの道を決めるときに読み直す
- **requires**: ST52（版の出所・印の判定）・ST54（開発版）はどちらも archive 済み
