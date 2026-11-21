# CH24 仮で進めた決定の確かめ

## D1
- 判定: 否
- 根拠: 条件 2（cost が範囲を越える）に当たる。選んだ側（既定の features のまま）は、許諾の検査を必ず落とす。
  - zip 9.7.3 の既定の `bzip2` は `["dep:bzip2", "bzip2/default", "_bzip2_any"]`（`~/.cargo/registry/src/*/zip-9.7.3/Cargo.toml`。bzip2 は `0.6`、default-features = false）。bzip2 0.6.1 の既定は `[dep:libbz2-rs-sys]`（`cargo info bzip2@0.6`）
  - repo の外の使い捨ての crate で `zip = "9"` を `cargo metadata` で解決した（2026-11-20）: `libbz2-rs-sys 0.2.7 bzip2-1.0.6`。ほか `lzma-rust2 Apache-2.0`・`ppmd-rust CC0-1.0 OR MIT-0`・`zlib-rs Zlib` は許される
  - `bzip2-1.0.6` は `tools/check-licenses.sh:9` の `ALLOW` に無い。検査は `cargo metadata --all-features` の全部のパッケージを見る（`tools/check-licenses.sh:170`）ので、tasks 3.2 が落ちる。いまの 2.6.1 は `bzip2 0.5.2` + `bzip2-sys`（`MIT/Apache-2.0`。`Cargo.lock:404-416`）で、この許諾は引いていない
  - 一覧を直すのは `tools/check-licenses.sh` で、`lane.json` の予定のパスの外。tasks.md:13・24 は一覧に触れないと決めている。design D1 の反転条件（「許す一覧に無い」→ 人間へ返す）は、条件付きではなく必ず起きる
  - `keeps`（読める圧縮の方式が今より狭まらない）を崩さずに許諾を通す道は、どちらの選択肢にも無い。`default-features = false` にして 9 の既定の並び（aes-crypto・deflate64・deflate・lzma・ppmd・time・zstd・xz）を書き、`bzip2` の代わりに `bzip2-rs`（`bzip2/bzip2-sys`。いまと同じ C の libbz2、`MIT/Apache-2.0`）を置く道。aes-crypto と deflate に絞る側は bzip2・lzma・zstd などの項目を持つ Takeout の書庫を読めなくなる（拒む: `discarded`）
  - 確かめた範囲で、ほかに失うものは無かった: 9 の既定は 2.6.1 の既定（aes-crypto・bzip2・deflate64・deflate・lzma・time・zstd・xz）をすべて含み、ppmd が増えるだけ（読める方式の対応は `zip-2.6.1/src/compression.rs:101-117` と `zip-9.7.3/src/compression.rs:189-219`）。`follows` / `rests_on` は空で、土台にしている決定は無い
  - premises.md にこの許諾の前提の行が無い（「既定の features は aes-crypto と deflate を含む」の行だけ）

## D2
- 判定: 確か
- 根拠: 条件 1: どちらの置き方でも、上げる前の木で書いた書き出しのバイト列は残る。中身は試験の DB の決まった記録と試験の合言葉だけで、注記は `ashiato-export {export_id}`（`crates/server/src/export/dump.rs:229`）。外に出たり捨てたりするものは無い。条件 2: 埋め込み先の `crates/server/src/export_file_tests.rs` は `lane.json` の予定のパスの中にあり、`crates/server/tests/fixtures/` は外（tasks.md:13）なので、選んだ側が範囲の中に収まる側。条件 3: `follows` は null。条件 4: 埋め込みに依って立つのは D8 の Scenario（前の版の書き出しを置いて新しく書く）で、D2 が D8 に依るのではない。tasks 1.5・4.2 が初めの commit の `Cargo.lock` の zip が 2.6.1 であることと、囲みの中が書き直されていないことを見る（tasks.md:56-57・89-90）。
  注: `export_file_tests.rs` はいま 64,317 バイト（`ls -la`）で、design D2 の反転条件「目安 64 KiB を超える」に、埋め込む前からほぼ達している。数十 KiB を足せば必ず超える。この目安が試験のファイルの大きさを指すなら、反転条件はすぐに満たされる。そのときの処置（書き出す記録を減らす）でも失うものは無く判定は変わらないが、目安が埋め込みの大きさのことかファイルの大きさのことかは design で書き分けるのがよい

## D7
- 判定: 確か
- 根拠: 条件 1: いまは `open.rs:93` の `entry.name()` だけが名前を作り（`by_name` は使わず `by_index` で回す。open.rs:79-80）、2.6.1 の規則は `zip-2.6.1/src/read.rs:1297-1300`（bit 11 があれば `from_utf8_lossy`、無ければ CP437）。9 の `name_raw()` は 0x7075 を当てた後の `file_name_raw` を返し（`zip-9.7.3/src/read/zipfile.rs:80-82`）、CRC が合う 0x7075 があれば名前を置き換えて `LanguageEncoding` を立てる（`zip-9.7.3/src/extra_fields/extra_field.rs:313-324`）ので、`name_raw()` と `flags()` の bit 11 から 2.6.1 と同じ規則で作れば、2.6.1 が開けた書庫の名前はどれも同じになる。2.6.1 が開けず 9 が開けるもの（CRC の合わない 0x7075、UTF-8 でない 0x7075、0x7075 が 2 つ）は読める側に変わるだけ。
  9 の局所ヘッダと中央ディレクトリの食い違いの検査（0x7075 の成分・拡張子・名前・時刻の不一致で `invalid!`。`zip-9.7.3/src/read/mod.rs:563-760`）は `ZipReadOptions::enforce_header_consistency` が立っているときだけ走る。既定は false（`#[derive(Default)]`。`read/mod.rs:298-317`）で、`by_index` は `ZipReadOptions::new()` を渡す（`read/zip_archive.rs:873-874`）ので、`open.rs` の読み方では新しく断る書庫は出ない。`is_dir` はどちらの版でも名前の最後のバイトを見る（design D5）。
  条件 2: CP437 の表と十数行は lane の `open.rs` の中に収まり、要件の本文は変えない。条件 3: `follows` は null。条件 4: `rests_on` は空で、土台にしている決定は無い（試験 tasks 1.3 の期待値は 2.6.1 で取るので、tasks 1.5・4.2 の確かめに依るが、どちらも決定ではない）
