# CH24 — zip の部品を 2 から 9 に上げ、例外の表の理由を事実に合わせる

> 軽い路（`lane.json` の `reason: same`）で始め、深掘り gate の Q-G1 で通常の路へ昇格した。何を・なぜ・どこまでは受け皿 #338 の議論で本人が決めてある
> （`docs/changes/CH24.md` の「議論で決めたこと」。聞き直さない）。zip を読む・書く振る舞いは変えず、依存の版と例外の表と、許す許諾の一覧（`bzip2-1.0.6` を 1 つ足す。Q-G1）だけを直す。

## Why

例外の表（`osv-scanner.toml`）は zip 2.6.1 の勧告を「上げる予定: 互換範囲外の修正版待ち」の理由で外している。だが直った版はもう出ている
（osv.dev に 2026-11-20 に問い合わせ: 2.6.1 には勧告 9 件、9.0.5 と 9.7.3 には 0 件。`premises.md`）。残っているのは 2 → 9 の大きな上げの作業だけで、
理由が事実と違う。今の使われ方（PC の更新は展開の前に指紋を照合・サーバは Takeout の zip をディスクへ展開せずに読む）ではすぐには困らないが、
重大の勧告を事実と違う理由で外したままにしない。

## What Changes

- サーバと PC の収集アプリの `Cargo.toml` の zip を `"2"` から `"9"` に上げ（`Cargo.lock` で 9.0.5 以上）、zip を使う 4 か所
  （`archive/open.rs`・`export/dump.rs`・`export/ledger.rs`・`update_apply.rs`）と試験を新しい API に合わせる。features は既定のまま（design D1）
- 既定の bzip2 が引く `libbz2-rs-sys` の許諾 `bzip2-1.0.6` を、許諾の検査（`tools/check-licenses.sh`）の許す一覧に足す（本人の答え 2026-11-20。深掘り gate の Q-G1。
  `specs/distribution` の MODIFIED。design D1）。読める圧縮の方式は今より狭めない
- **上げる前に**、今の版（zip 2.6.1）で書いた書き出しの zip を試験に埋め込み、上げた後の版で同じ中身に読めること・AES の形が同じことを試験で固定する
  （`specs/data-durability`。ADDED の 1 Requirement。design D2）。今の試験は書くのも読むのも同じ版の部品なので、版の間の互換を見ていない
- zip 9 は項目の名前の読み方と道の判定を変えた。2.6.1 と同じに振る舞うよう、Takeout の名前は 2.6.1 と同じ規則で読み（design D7）、PC の更新の展開は 9 の判定を使わず
  2.6.1 と同じ名前の確かめを自前で当てる（design D6）。上げる前の結果を、zip の部品の外で作った材料で試験に固定する
- 例外の表から zip の行を消す。本人が名指しした GHSA 3 件に加え、同じ zip 2.6.1 の RUSTSEC-2026-0367・0368・0369・0429 の 4 行も消す
  （9.0.0 で直っている。残すと検査の「勧告がもう出ない」で落ちる。design D3）
- 上げきれなければ、本人の決めた代わり（理由を `上げる予定: 2 → 9 の上げの作業が残る` に直し、期限を 90 日以内）を計画の直しとして人間に返す（design D4）

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `data-durability`: ADDED の 1 Requirement（以前の版のサーバが書いた書き出しを同じ中身で読み、台帳の照合で書いたままと判定し、同じ AES の形で書く）。既存の Requirement は変えない
- `distribution`: MODIFIED の 1 Requirement（配布物に入る依存の許諾を、携帯の依存も含めて検査する）。許可の一覧に `bzip2-1.0.6` を足し、一覧の値を 17 から 18 にする（Q-G1）。ほかの本文と Scenario は変えない

## Impact

- 触れる予定のパス（`lane.json` の予定のパスと `tools/check-licenses.sh`。tasks 4.1）: 2 つの `Cargo.toml`・`Cargo.lock`・zip を使う 4 か所・7 つの試験のファイル・`osv-scanner.toml`・`tools/licenses/**`・
  `tools/check-licenses.sh`（許す一覧に `bzip2-1.0.6` を足すだけ。Q-G1）・`docs/changes/CH24.md`（と、この change の中）
- 依存: zip 9 の既定の features は新しい依存を引く（例: `lzma-rust2`・`ppmd-rust`）。許諾は `tools/check-licenses.sh` が見る。既定の bzip2 が引く `libbz2-rs-sys` の
  `bzip2-1.0.6` だけが一覧に無く、本人の答え（Q-G1）で足す。それ以外の許諾が足りなければ、一覧を直さずに返す（design D1 の反転条件）。zip 9 は rust 1.88 以上を求め、手元は 1.99、CI は `stable`
- 触れないもの: API・送信の契約・移行・画面・携帯の収集アプリ・CI の workflow・例外の表の zip 以外の行
- 見つけたが直さないもの: `update_apply.rs` の展開は zip の `extract` を使わず自前で Unix の権限を写すので、RUSTSEC-2026-0367（SUID の保持）の直しは
  zip を上げても自前の展開には効かない。PC の収集アプリは Windows で動き、写すのは `#[cfg(unix)]` の枝だけ。振る舞いを変えない約束なので触らない（PR 本文に挙げる）
- 走っている change との重なり: `ch19-emulator-only-class`（`emulator-startup`）・`ch22-soak-install-online-ci`（`distribution`）は、`Cargo.lock`・`osv-scanner.toml`・
  zip を使う 4 か所・`data-durability` に触れない（2026-11-20 に change の中を `Cargo.lock` `osv-scanner` `data-durability` で探した）。差し戻すものは無い。
  `ch22-soak-install-online-ci` は同じ `distribution` に delta を持つが、ADDED の 5 つと MODIFIED の「更新を確かめる問い合わせは、版の一覧を取る決まった形だけを配布元へ送る」で、
  この change が MODIFIED にする「配布物に入る依存の許諾を、携帯の依存も含めて検査する」には触れない（2026-11-20 に ch22 の delta の見出しを読んだ）。`tools/check-licenses.sh` にも触れない
