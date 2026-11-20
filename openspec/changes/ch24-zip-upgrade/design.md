# CH24 design — zip の部品を 2 から 9 に上げる

## Context

- zip は 2 つの crate が `zip = "2"`（既定の features）で使い、`Cargo.lock` は 2.6.1 の 1 つだけ
- 使う所は 4 か所: サーバの `archive/open.rs`（Takeout の書庫を `ZipArchive` で読み、ディスクへ展開しない）・`export/dump.rs`（`ZipWriter` で
  deflate + AES-256 + 注記）・`export/ledger.rs`（置き場の書き出しの注記を読んで台帳と照合）・PC の収集アプリの `update_apply.rs`（指紋を照合した配布物を
  `by_index` と `enclosed_name` で自前に展開し、Unix では権限を写す）。試験は 7 つのファイルが `ZipWriter` / `ZipArchive` を直に使う
- 例外の表の zip の行は 7 つで、どれも理由は「上げる予定: 互換範囲外の修正版待ち」（`premises.md`）
- base の全体の脆弱性の検査は zip と無関係な 3 件（chrono・libsqlite3-sys・rustix）で既に落ちる。PR の検査（`--base`）は rc=0

## Goals / Non-Goals

- Goals: zip を 9.0.5 以上へ上げる・zip の行を例外の表から消す・振る舞いを変えない・前の版で書いた書き出しが読めることを試験で固定する
- Non-Goals: zip と無関係な勧告（全体の検査を落としている 3 件）を直す・例外の表のほかの行を変える・自前の展開を zip の `extract` に置き換える・
  `update_apply.rs` の権限の写し方を変える（下の D5）・features を絞って依存を減らす

## Decisions

### D1. zip の features は既定のまま、許す一覧に `bzip2-1.0.6` を足す（本人の決定。deep.md Q-G1）

`zip = "9"`（既定の features）にする。`default-features = false` で aes-crypto と deflate だけに絞れば依存は減るが、Takeout の zip を読める
圧縮の方式が今より狭くなりうる（2.6.1 の既定は bzip2・lzma・zstd なども読む）。振る舞いを変えない約束なので、絞らない。

既定の bzip2 は Rust で書き直した libbz2（`libbz2-rs-sys`）を引き、その許諾 `bzip2-1.0.6` は許諾の検査（`tools/check-licenses.sh`）の許す一覧に無い。
9 の `bzip2-rs`（C の libbz2）を選んでも `libbz2-rs-sys` を引くので、許諾を変えずに上げる道は無い（deep.md Q-G1 の事実）。本人の答え（2026-11-20）で、
許す一覧に `bzip2-1.0.6` を 1 つ足す。直すのは台本の既定の一覧（`ALLOW`）と、自己試験が一覧の文字列を丸ごと比べる行の 2 か所で、ほかの値は足さない・外さない。
一覧は正典の `distribution`（配布物に入る依存の許諾を、携帯の依存も含めて検査する）が値を並べて固定しているので、その Requirement を MODIFIED で直す（`specs/distribution`）。
`libbz2-rs-sys` は crate に `LICENSE` を持つので、配る物の依存の一覧（`tools/release/notices.sh`）はそれを写し、`tools/licenses/texts/` に全文を足す必要は無い。

反転条件: 既定の features が引く依存のうち `bzip2-1.0.6` の外の許諾が許す一覧に無い・その依存に勧告が当たる・組み立てが通らない。
そのときは features を絞る前・一覧に足す前に人間へ返す（読める方式が狭まるのは振る舞いの変化。一覧を広げるのは本人の決めること）。

### D2（仮）. 前の版で書いた書き出しは、試験のファイルにバイト列として埋め込む

上げる前の木（zip 2.6.1）で、試験の DB に決まった記録を置いて書き出しを 1 回走らせ、そのファイルのバイト列・項目ごとの SHA-256・書き出しの id・
合言葉を `crates/server/src/export_file_tests.rs` に定数（`CH24_PREVIOUS_EXPORT` など）として置く。別の材料のファイル（`crates/server/tests/fixtures/`）に置かないのは、
予定のパス（`lane.json`）の外だから。埋め込みを入れた commit の `Cargo.lock` が zip 2.6.1 であることを tasks 1.5 が git で確かめる
（上げた後に作り直すと互換を見なくなる）。
埋め込みは `// CH24-FIXTURE-BEGIN` と `// CH24-FIXTURE-END` の行で囲み、tasks 1.5 が「初めて入れた commit の囲みの中」と「HEAD の囲みの中」が
同じことも見る（`git log -S` は初めの commit しか拾わず、上げた後にバイト列だけを書き直しても気づかない。review R7）。
記録の置き方は、書いた量が 20 バイト未満の項目（0 行の `.jsonl`）と 20 バイト以上の項目の両方が出るようにする（AE-2 と AE-1 の両方の枝を比べるため。review R5）。
AES の形は zip の部品の読み方を通さず、試験の中でバイト列から読む: 中央ディレクトリ（`PK\x01\x02`）と局所ヘッダ（`PK\x03\x04`）の両方の
圧縮方式（`99`）・汎用の印の暗号化の bit（bit 0）・追加の欄 `0x9901`（vendor version・`AE` の印・強さ・実の圧縮方式）（review R6）。
手元に AES の zip を解ける別の道具（7-Zip など）は無いが、python3 の標準の `zipfile` は AES を解けなくても項目の名前と形（`compress_type`・`flag_bits`）を
別の実装として読める。上げた後の書き出しを試験が `CH24_DUMP_DIR` に書き、tasks 2.2 が python3 で全部の項目について `compress_type == 99` と
`flag_bits & 1` を見る（zip の部品の外の実装で、形が壊れていないことの下限）。
反転条件: 埋め込みが大きすぎて試験のファイルが読めなくなる（目安 64 KiB を超える）なら、書き出す記録を減らす。

### D3. 例外の表から消すのは zip の 7 行

本人が名指ししたのは GHSA 3 件だが、同じ zip 2.6.1 の RUSTSEC-2026-0367・0368・0369・0429 の 4 行も 9.0.0 で直っていて、上げた後は当たらない。
残すと、行の検査が「勧告がもう出ない」で落ちる（`dependency-vulnerability-scan` の正典）。同じ部品の同じ上げの帰結で、範囲は広げていない。

### D4. 上げきれないときは、自分で代わりに切り替えない

本人は「上げきれないときは、例外の理由を事実（`上げる予定: 2 → 9 の上げの作業が残る`）に直して期限を 90 日以内に置く」と決めている。
ただしその場合は完了の判定（1〜4）が変わり、tasks の検証も成り立たないので、Task agent は切り替えずに、上げきれない理由（落ちた試験・組み立ての誤り・
許諾・新しい勧告）と本人の決めた代わりを添えて返す。グラフが計画の直しとして扱う。
上げきれないとは: 予定のパスの中の直し（4 か所と 7 つの試験の、API に合わせる直し）で Task 2 の検証が通らないこと。

### D5. `update_apply.rs` の展開は自前のまま

RUSTSEC-2026-0367・0368・0369 は zip の `extract` の振る舞い（SUID の保持・道の外への symlink・symlink 越しの権限）の勧告。
`update_apply.rs` は `extract` を使わず、項目の名前を自前に確かめて道の外を断り（D6）、`#[cfg(unix)]` で `unix_mode` を写す。展開を `extract` に置き換えない。
zip 9 の `unix_mode()` は Unix の項目の特別な bit（SUID・SGID・sticky）を落として返す（`zip-9.7.3/src/format/functions.rs` の `clear_specials_bits`）。
これは勧告 RUSTSEC-2026-0367 の直しそのもので、`#[cfg(unix)]` の枝が写す権限から SUID などが消える。PC の収集アプリは Windows で動き、この枝は
配る物では走らない（Linux の試験でだけ走る）。DOS の項目の読み取り専用の扱いも書き方が変わった（2.6.1 は `&= 0o555`、9 は `&= !0o222`）が、
これも `#[cfg(unix)]` の枝だけに効く。自前の写し方のコードは変えず、この差を PR 本文に挙げる。
ディレクトリの判定（`is_dir`）は 2 つの版で同じ規則（名前の最後の文字が `/` か `\`。2.6.1 は文字、9 はバイトで見るが、どちらも ASCII の 1 文字）。

### D6. PC の更新の展開は、2.6.1 と同じ名前の確かめを自前で当てる

zip 9 の `enclosed_name` は判定が変わった（review R1・R11。2 つの版の source と、Windows 向けに組んだ使い捨ての .exe で確かめた）:
2.6.1 が断った絶対パス（`/etc/passwd`）・ドライブの前置き（`C:\x`）・根から始まる名前（`\x`）・Windows で前置きになる `a:b` を、9 は相対に直して受け付ける。
逆に、2.6.1 が通した Windows の装置の名前（`NUL`）・末尾の `.`（`a.`）を 9 は Windows で断り、どの OS でも `\` を区切りとみなすので Linux で `..\x` も断る。
`extract_zip` は 9 の `enclosed_name()` を使わず、2.6.1 と同じ確かめ（名前に `\0` を含む・`std::path::Path` の成分に前置きか根がある・`..` で根の外へ出る、のどれかなら
「zip の道が不正」で断る）を自前の関数で当て、展開する先は 2.6.1 と同じく `destination.join(PathBuf::from(名前))` にする。受け付ける名前と展開先は、どの OS でも 2.6.1 と同じ。
名前の文字列は 9 の `name()` から取り、Err なら断る。2.6.1 と文字列が違いうるのは UTF-8 の印の無い ASCII でない名前だけで、指紋を照合した配る zip の項目の名前は
`tools/release/build.sh` が決めた ASCII の固定のもの（`ashiato-collector.exe` など）なので当たらない。
正典に Scenario を置かない: 振る舞いを変えないことを試験（tasks 1.4）で固定するだけで、正典の約束を足さない（`distribution` は `ch22-soak-install-online-ci` が触っている）。
次に zip を上げる change は、この D6 と 1.4 の試験から同じ確かめを引き継ぐ（review R16）。

### D7（仮）. Takeout の書庫の項目の名前は、2.6.1 と同じ規則で読む

zip 9 は名前の読み方を変えた（review R3）: 2.6.1 は汎用の印の bit 11（UTF-8 の印）があれば UTF-8（不正なバイトは置換文字にする
`String::from_utf8_lossy`）、無ければ CP437 で読む（`zip-2.6.1/src/read.rs:1297-1300`）。9 は印を見ず、バイト列が UTF-8 として正しければ UTF-8、
でなければ CP437 で読み、`name()` は `ZipResult` を返す。項目の名前は記録の `inner_path` と台帳の照合（`core.archive_file`・保留の形の行）に残るので、
読み方が変わると、上げる前に取り込んだ記録と名前が揃わなくなる。
`open.rs` は `name()` を使わず、`name_raw()` と `flags().as_u16() & 0x0800` から 2.6.1 と同じ規則で名前を作る。CP437 の表は zip 9 が公開していないので、
`open.rs` に 128 字（0x80〜0xFF）の表を置く（0x00〜0x7F は ASCII のまま。2.6.1 の `cp437.rs` と同じ）。この読み方は失敗しないので、`name()` の Err の扱いは決めなくてよい。
正しい CRC の Info-ZIP Unicode Path（`0x7075`）を持つ項目は、9 が `name_raw()` を置き換えて bit 11 を立てるので、この読み方でも 2.6.1 と同じ名前になる（review R3 の再確認）。
CRC の合わない `0x7075` を持つ書庫だけは、2.6.1 では開けず `broken_zip` になり、9 では見出しの名前で読める（review R12）。読める側に変わるだけで、取り込んだ記録と名前は変わらず
失うものは無いので受け入れる。
正典に Scenario を置かない: 理由は D6 と同じ（試験 tasks 1.3 で固定する。次に zip を上げる change はこの D7 と 1.3 から引き継ぐ。review R16）。
実物の Takeout の書庫が UTF-8 の印を持つかは確かめていない（`premises.md`）が、2.6.1 と同じ規則で読む限り、どちらでも記録に残る名前は変わらない。
反転条件: 実物の Takeout の書庫が UTF-8 の印を持たず、2.6.1 の読み方で名前が化けていると分かったら、9 の読み方へ移すかを別の change で決める
（移すと上げる前に取り込んだ記録の `inner_path` と揃わなくなるので、記録の名前の直し方と一緒に決める）。

### D8. 書き出しの台帳の照合（注記を読む）は、前の版で書いた書き出しでも「書いたまま」と判定する

前の書き出しを消すかどうかは、台帳の照合が置き場のファイルを zip として開いて注記を読めるかで決まる（`ledger.rs` の `is_untouched_with`。false なら消さない側に倒れる）。
上げた後の版で、前の版で書いた書き出しを置いた状態から新しい書き出しを書き終えると、前のファイルが消えて 1 つだけ残ることを Scenario にする（review R4）。

## Risks

- zip 9 の API の変化（例: `set_comment` が結果を返す・`name()` が結果を返す）は組み立てで分かる。読み方の意味の変化のうち、項目の名前（D7）と
  道の判定（D6）は既存の試験では落ちないので、どちらも 2.6.1 の振る舞いを自前で残し、上げる前の版の結果を試験に固定する（tasks 1.3・1.4）。`is_dir` は同じ規則、`unix_mode` は `#[cfg(unix)]` の枝だけ（D5）
- 名前の試験の材料は zip の部品の外（python3 の `zipfile`・Info-ZIP の `zip`）で作ったバイト列を試験に埋め込む。部品の `ZipWriter` で作ると、書く側も上げで変わり、
  bit 11 を必ず付けるので、印の無い名前や道の外の名前を作れない
- PC の収集アプリは Windows で動く。CI の Windows の job は `cargo test -p ashiato-collector-windows --lib` を走らせるので、`update_apply.rs` の `#[cfg(test)]` に置く
  名前の試験（tasks 1.4）はそこで走る。手元でも `--target x86_64-pc-windows-gnu` の `--lib` が WSL の interop で走る（2026-11-20 に 435 本を数えた）。
  `runtime_windows` と `st58_pc_update` は展開の名前の判定を見ない（review R2）
