# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: 依存の zip を 2 から 9 に上げ、使う 4 か所を新しい API に合わせ、osv-scanner.toml の 3 件の例外を外す（上げきれなければ理由を事実に直す）だけ。本人の決定で振る舞いは変えない（書き出しの AES の zip が今と同じに読めることを試験で確かめる）。API・送信の契約・移行には触れない。新しい版が足す依存の許諾文は tools/licenses に置く
- 触れる予定のパス: crates/server/Cargo.toml / crates/collector-windows/Cargo.toml / Cargo.lock / crates/server/src/archive/open.rs / crates/server/src/export/dump.rs / crates/server/src/export/ledger.rs / crates/collector-windows/src/update_apply.rs / crates/server/src/archive_tests.rs / crates/server/src/archive_flow_tests.rs / crates/server/src/media_export_tests.rs / crates/server/src/export_job_tests.rs / crates/server/src/export_tests.rs / crates/server/src/export_file_tests.rs / crates/collector-windows/tests/st58_pc_update.rs / osv-scanner.toml / tools/licenses/** / docs/changes/CH24.md

## gate で積んだ A（仮 D1 が「否」。2026-11-20）

merge gate（PR #347・upstream）が、仮で進めた D1（zip の features は既定のまま）を別の席が「否」とした（`provisional-review.md` の D1）ため落ちた。
D1 を台帳から外し（`provisional.py resolve`）、design の見出しの（仮）を外して、人間へ返す。コードと tasks は変えていない。
問いの HTML: `docs/briefs/CH24-deep-r1.html`（`deep-questions-r1.json`）

### Q-G1 [zip 9 の bzip2 の部品の許諾が許す一覧に無い。どう扱うか]

(未回答)

- **答え**: `deep-answers-1.txt`（2026-11-20）でも `(未回答)`。A なので推奨（案 (1)）を採ったとはみなさず、もう一度聞く

- **分類**: A（loss: discarded。案 (2) は bzip2 で縮めた項目を持つ Takeout の書庫を読めなくなる）
- **事実**: zip 9.7.3 の bzip2 は Rust で書き直した libbz2（`libbz2-rs-sys` 0.2.7）を引き、その許諾は `bzip2-1.0.6`。
  `tools/check-licenses.sh` の許す一覧（`ALLOW`）に無く、検査は `cargo metadata --all-features` の全部の部品を見るので、tasks 3.2 が必ず落ちる。
  9 の `bzip2-rs`（C の libbz2）だけを選んでも、cargo が依存と同じ名前の feature `bzip2` を立てて `bzip2/default` から `libbz2-rs-sys` を引く
  （2026-11-20、repo の外の使い捨ての crate で `cargo metadata --all-features` / `cargo tree` を走らせて確かめた。provisional-review.md D1 が挙げた「bzip2-rs を置く道」は成り立たない）。
  いまの 2.6.1 は `bzip2` 0.5.2 + `bzip2-sys`（`MIT/Apache-2.0`）。ほかに既定が引く `lzma-rust2`（Apache-2.0）・`ppmd-rust`（CC0-1.0 OR MIT-0）・`zlib-rs`（Zlib）は許される
- **選択肢**: (1) 既定のまま、許す一覧に `bzip2-1.0.6` を足す（推奨。`tools/check-licenses.sh` は予定のパスの外で、軽い路を外れる）/
  (2) zip 9 に上げ、bzip2 だけを外して残りの既定（aes-crypto・deflate64・deflate・lzma・ppmd・time・zstd・xz）を並べる /
  (3) zip は 2.6.1 のまま、例外の理由を事実に直して期限を 90 日以内に置く（`docs/changes/CH24.md` の「上げきれないとき」）
- **族**: 9 の既定の features が引く部品の許諾・勧告・組み立て（design D1 の反転条件）。許諾は bzip2 だけが外れ、ほかは許される。勧告は tasks 3.1 の `check-vulns.sh --base`、組み立ては Task 2 が見る。サーバと PC の収集アプリは同じ `zip = "2"` で、答えを両方に当てる
- **上流の漏れ**: premises —— 「既定の features は aes-crypto と deflate を含む」だけを確かめ、既定が引く部品の許諾を確かめずに design の反転条件へ回した
