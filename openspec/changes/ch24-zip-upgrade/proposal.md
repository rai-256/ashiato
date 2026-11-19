# CH24 — zip の部品を 2 から 9 に上げ、例外の表の理由を事実に合わせる

> 軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いてある。何を・なぜ・どこまでは受け皿 #338 の議論で本人が決めてある
> （`docs/changes/CH24.md` の「議論で決めたこと」。聞き直さない）。zip を読む・書く振る舞いは変えず、依存の版と例外の表だけを直す。

## Why

例外の表（`osv-scanner.toml`）は zip 2.6.1 の勧告を「上げる予定: 互換範囲外の修正版待ち」の理由で外している。だが直った版はもう出ている
（osv.dev に 2026-11-20 に問い合わせ: 2.6.1 には勧告 9 件、9.0.5 と 9.7.3 には 0 件。`premises.md`）。残っているのは 2 → 9 の大きな上げの作業だけで、
理由が事実と違う。今の使われ方（PC の更新は展開の前に指紋を照合・サーバは Takeout の zip をディスクへ展開せずに読む）ではすぐには困らないが、
重大の勧告を事実と違う理由で外したままにしない。

## What Changes

- サーバと PC の収集アプリの `Cargo.toml` の zip を `"2"` から `"9"` に上げ（`Cargo.lock` で 9.0.5 以上）、zip を使う 4 か所
  （`archive/open.rs`・`export/dump.rs`・`export/ledger.rs`・`update_apply.rs`）と試験を新しい API に合わせる。features は既定のまま（design D1）
- **上げる前に**、今の版（zip 2.6.1）で書いた書き出しの zip を試験に埋め込み、上げた後の版で同じ中身に読めること・AES の形が同じことを試験で固定する
  （`specs/data-durability`。ADDED の 1 Requirement。design D2）。今の試験は書くのも読むのも同じ版の部品なので、版の間の互換を見ていない
- 例外の表から zip の行を消す。本人が名指しした GHSA 3 件に加え、同じ zip 2.6.1 の RUSTSEC-2026-0367・0368・0369・0429 の 4 行も消す
  （9.0.0 で直っている。残すと検査の「勧告がもう出ない」で落ちる。design D3）
- 上げきれなければ、本人の決めた代わり（理由を `上げる予定: 2 → 9 の上げの作業が残る` に直し、期限を 90 日以内）を計画の直しとして人間に返す（design D4）

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `data-durability`: ADDED の 1 Requirement（前の版の zip の部品で書いた書き出しを同じに読み、同じ AES の形で書く）。既存の Requirement は変えない

## Impact

- 触れる予定のパス（`lane.json`）: 2 つの `Cargo.toml`・`Cargo.lock`・zip を使う 4 か所・7 つの試験のファイル・`osv-scanner.toml`・`tools/licenses/**`・`docs/changes/CH24.md`
  （と、この change の中）
- 依存: zip 9 の既定の features は新しい依存を引く（例: `lzma-rust2`・`ppmd-rust`）。許諾は `tools/check-licenses.sh` が見る。許す許諾の一覧（台本の中）は予定のパスの外なので、
  足りなければ直さずに返す。zip 9 は rust 1.88 以上を求め、手元は 1.99、CI は `stable`
- 触れないもの: API・送信の契約・移行・画面・携帯の収集アプリ・CI の workflow・例外の表の zip 以外の行
- 見つけたが直さないもの: `update_apply.rs` の展開は zip の `extract` を使わず自前で Unix の権限を写すので、RUSTSEC-2026-0367（SUID の保持）の直しは
  zip を上げても自前の展開には効かない。PC の収集アプリは Windows で動き、写すのは `#[cfg(unix)]` の枝だけ。振る舞いを変えない約束なので触らない（PR 本文に挙げる）
- 走っている change との重なり: `ch19-emulator-only-class`（`emulator-startup`）・`ch22-soak-install-online-ci`（`distribution`）は、`Cargo.lock`・`osv-scanner.toml`・
  zip を使う 4 か所・`data-durability` に触れない（2026-11-20 に change の中を `Cargo.lock` `osv-scanner` `data-durability` で探した）。差し戻すものは無い
