# ST52 — 3 つの部品を 1 つの版で、3 つのチャンネルに配る

## Why

いまは配る仕組みが無い。版番号は 3 か所に手書きで（premises P10）、サーバは画面を自分で配れず（`vite preview` が別のプロセスで配る）、携帯の APK は組んだ PC の debug 鍵で署名されている。
この状態では「配れる形を壊した」ことに、配ろうとした日まで気づけない。ST53〜ST58（許諾・開発版の並置・インストール・設定・更新）は、どれも「印を付けると 3 部品が同じ版で並ぶ」ことを土台にする。
配る仕組みを先に通しておけば、配れる形を壊した Story はその PR の時点で落ち、更新の失敗も本人 1 人のうちに見つかる（FR-104・FR-105・FR-107・NFR-25・扉 34）。

## What Changes

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1・Q2・Q3・Q5・Q11 は A（失われるものがある）で、本人の答えが推奨と違えば `spec --answers` が書き直す。第 2 回の答えで推奨と違ったのは Q1 と Q4 で、この版はそれを写した（Q11 は推奨と同じ）。

- **版の出所を 1 つにする**（Q6 の推奨・NFR-25）: 根の `Cargo.toml` の `[workspace.package] version`（MAJOR.MINOR.PATCH。一般向けの配布までは `0.x`）だけを出所にする。
  携帯（gradle の `versionName`）と画面（vite）もここを読み、手書きの版番号を消す。ベータ版の `-beta.N` と開発版の `-dev.<YYYYMMDD>` は、ファイルを書き換えずに組み立ての時に渡す（書き換えると組み立ての印が `.dirty` になる）。
  渡した版番号は組み立ての印（ST49）にも、版の記録と更新の履歴（ST48・ST51）にもそのまま入る。3 部品の版番号が食い違えば CI が落ちる
- **携帯の versionCode を版番号から計算する**（Q3 の推奨）: `MAJOR×1,000,000 + MINOR×10,000 + PATCH×100 + (ベータは N・正式は 99・開発版は 0)`。範囲を超える版は CI が落とす
- **印（タグ）で 3 つのチャンネルに配る**（FR-105・Q10 の推奨）: `v<版>` は通常のリリース、`v<版>-beta.N` は先行公開、開発版は印 `nightly`（`v*` の外）を動かして 1 つのリリースを上書きする。
  リリースにはサーバ（画面を含む。Windows x86_64 と Linux x86_64 の 2 つ。Q7 の推奨）・携帯の APK・PC の収集アプリが同じ版で並び、3 つそろわなければ公開しない（C7）。開発版のリリースには ST54 まで携帯の APK を並べない（Q4。FR-104 ★ 2026-11-01）。
  印は main の祖先にだけ打てる（緊急の修正の枝 `release/<MAJOR>.<MINOR>` の正式版を除く。C2・C9）
- **開発版は幹に変更があった日だけ作り直す**（FR-107・C6）: 日本時間 0:00 に起き、`nightly` の印の commit と main の HEAD が違うときだけ組む。開発版はサーバと PC の収集アプリだけを並べ、携帯の APK は ST54 から（Q4）。開発版の APK を `dev.ashiato.collector.dev` で組んで開発版の鍵で署名する道具は ST52 が作り、使い捨ての鍵で確かめる（D23（仮））
- **最初の本物の印は ST52 では打たない**（Q5 の推奨）: ST52 はリリースの形を**下書きのリリース**（印を作らず、書き手にしか見えない）で確かめる。最初の `v0.1.0-beta.1` と開発版の公開は、許諾の同梱（ST53）の後に本人が main に打って始める。
  開発版の予定の実行は、本人が公開の設定を入れるまで何も公開しない
- **サーバが画面も配る**（FR-104 ★ 2026-10-25・Q8 の推奨）: 画面の組み立て済みのファイルをサーバの実行ファイルに組み込み、同じ番号で `/` に画面、`/api/…` に今の API を出す。収集アプリの送り先の道は変えない。
  開発の起動と画面の e2e が使う `vite preview` は今のまま残す
- **携帯の APK を正式な鍵で署名する**（Q2・Q11 の推奨・扉 34）: 正式版とベータ版は、印 `v*` にだけ開いた GitHub の Environment の正式な鍵で CI が署名する（Q2 の推奨の「印と main」の狭い側。D21（仮））。開発版は正式な鍵と別の鍵（Q11。開発版の Environment に置き、控えは USB に 1 部。作って置くのは、開発版に携帯の APK を載せる ST54）。
  PR の組み立ては debug 鍵のまま。鍵は Play App Signing に持ち込める種類と長さで作る（D3（仮））。鍵を作って置くのは本人で、台本と手順を ST52 が用意する
- **配る組み立ては綺麗で、資格情報を持たない**（C1・C8）: 印に `.dirty` / `.debug` があるもの、携帯の APK に接続先や資格情報が焼き込まれたものは公開しない
- **配れる形を PR ごとに確かめる**（Q9 の推奨）: コードを触った PR ごとに、公開せずに 3 部品の配布物を組み、版がそろうかを見る
- **配った印 `v*` は消さない・動かさない**（D2（仮））: GitHub のルールセットで拒み、公開の前にルールセットが入っていることを確かめる
- **配った印の検査（ST48・ST50）が数える印を、今の木の祖先にあるものに絞る**（D1（Q12 の答え待ち））: 緊急の修正の枝の印で、main の検査が誤って落ちる・送信契約の版の上げ忘れを見逃すことを防ぐ。
  「最後の印」は日付でなく、祖先にある印のうち版番号が最も大きいもの
- **緊急の修正の枝は移行と封筒の形を変えない**（D6（仮））: 修正の枝の正式版の印は、枝を切った元の印と `migrations/ORDER` と封筒の形の写しが同じときだけ公開する

**BREAKING ではない**: 収集アプリの送り先の道・API・記録の形は変えない。開発の起動（`tools/dev.sh` / `tools/stack.sh`）と e2e は `vite preview` のまま動く。
携帯のアプリ識別子 `dev.ashiato.collector` は変えない（扉 34）。本人の携帯には収集アプリがまだ入っておらず、記録も集めていない（Q1）。入れ直しも署名の引き継ぎも要らず、最初に入れるのは ST56 の後の正式な鍵の版（扉 34 ★ 2026-11-01 訂正）。

## Capabilities

### New Capabilities

（無し）

### Modified Capabilities

- `distribution`: 版の出所・versionCode・チャンネルと印・配る組み立ての検査・署名・開発版の作り直し・下書きのリリース・サーバが画面を配る・PR ごとの組み立て・印の取り下げの禁止・緊急の修正の枝（ADDED 11）。
  ST48 の「DB に当てた移行の版を、DB の中に記録する」を MODIFIED（当てた組み立ての版番号は印と同じ版番号。review/spec.md R1）と、「配った移行は、名前も中身も変えられない」を MODIFIED（今の木の祖先にある印だけを数える。D1（Q12 の答え待ち））
- `record-envelope`: ST49 の「組み立ての印は、版番号・commit・…を持ち、組み立てのたびに作り直す」を MODIFIED（印の版番号は 1 つの出所から決まり、配る組み立てでは渡した版番号）。
  ST50 の「封筒の形が変わったのに送信契約の版が上がっていなければ CI が落ちる」を MODIFIED（最後の配った印は、祖先にある印のうち版番号が最も大きいもの。D1（Q12 の答え待ち））

**capability の照合**: INDEX.md の capability 表は ST52 を `distribution` に割り当てていた。リリースとチャンネル・署名・開発版・サーバが画面を配ることは `distribution` で合っている。
組み立ての印の版番号（ST49）と送信契約の版の検査（ST50）は `record-envelope` にあり、ST52 はその 2 つの Requirement の「版番号の出所」と「数える印」を書き換える（申し送り `docs/handoff/ST52.md` の「型を変えるなら一緒に変える」「出所を 1 つにしたら、そこから読むように替える」）。
そのため `record-envelope` に ST52 を足し、INDEX.md の訂正の節に理由を書いた。後続の振る舞いの前倒しではない。

**requires**: ST48・ST49・ST51 は archive 済み（正典 `openspec/specs/distribution`・`record-envelope` に入っている）。ST50（送信契約の版の検査）は requires に無いが archive 済みで、MODIFIED する Requirement は正典にある。MODIFIED は正典の今の本文から写した。

## Impact

- 版の出所: `Cargo.toml`（出所のまま）、`collector-android/app/build.gradle.kts`（`versionName`・`versionCode` を出所と渡した版から計算。`applicationIdSuffix` を開発版だけ・`signingConfigs` を環境変数から）、
  `web/vite.config.ts` と `web/build-stamp.ts`（`package.json` でなく出所を読む）、`web/package.json`（`version` を消す。`private: true` なので npm は版を求めない）、
  `crates/build-stamp`（渡した版番号 `ASHIATO_BUILD_VERSION` を受ける）、`crates/server/build.rs`・`crates/collector-windows/build.rs`、`crates/server/src/migration_ledger.rs`（`BUILD_VERSION` を印と同じ版番号に）、`tools/stack.sh`（画面を先に組んでサーバにも取り込む。D20）
- サーバが画面を配る: `crates/server/src/lib.rs`（router に `/api` の剥がしと組み込んだ画面）、新しい `crates/server/src/web_assets.rs`、`crates/server/build.rs`（`ASHIATO_WEB_DIST` を組み込む）
- 配る道具: 新しい `tools/release/`（`version.sh`・`version-table.tsv`・`build.sh`・`package.sh`・`check-artifacts.sh`・`sign-apk.sh`・`make-key.sh`・`draft.sh`・`check-guards.sh`）、新しい `tools/check-version.sh`
- CI: `.github/workflows/ci.yml`（`package` の job。PR ごとに 3 部品を組む）、新しい `.github/workflows/release.yml`（印の push）・`.github/workflows/nightly.yml`（予定の実行）。どちらも `contents: write` を自分の job だけに持つ
- 検査の型の書き換え: `tools/check-released-migrations.sh`（ST48）・`tools/check-contract-version.sh`（ST50）の印の列挙を、祖先の印と版番号の順に替え、自己試験に修正の枝の場合を足す
- 文書: 新しい `docs/release.md`（鍵を作る・Environment とルールセットを入れる・最初の印を打つ手順）、`docs/production-prep.md`（バージョンの単一情報源の行）、`docs/requirements.md` の扉 34 と FR-104（record の段が ★ 2026-11-01 で改訂済み。Q2 の Environment・Q11 の開発版の鍵・Q1 の入れ直しの訂正・Q4 の開発版の例外）
- 申し送り: `docs/handoff/ST53.md`（最初の印と開発版の公開の始め方）、`docs/handoff/ST54.md`（開発版の DB の扱い。review/deep.md R11。開発版の携帯の APK と開発版の鍵は record の段が書いた節。Q4・Q11・D23（仮））、`docs/handoff/ST58.md`（PC の後戻り。R14・D5（仮））
- **走っている change との重なり**: `openspec/changes/` で archive 以外は `st51-update-history` だけで、main では archive 済み（#221）。ST51 が足した更新の履歴は、S-01 の組み立ての印をそのまま「今の版」に書く（正典 distribution「S-01 の版が変わると、更新の履歴に 1 行残る」）ので、
  版番号の出所の変更はそのまま更新の履歴の「今の版」に入る。ST51 の Requirement は変えない
