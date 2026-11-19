# CH24 spec-review（成果物どうしの整合）

対象: `docs/changes/CH24.md`、`openspec/changes/ch24-zip-upgrade/`（proposal・premises・design・specs/data-durability・tasks・lane.json・lane-verdict.json・deep.md）
照らした実物: `crates/collector-windows/src/update_apply.rs`（`extract_zip`）・`crates/collector-windows/tests/st58_pc_update.rs`・`crates/server/src/archive/open.rs`・`crates/server/src/archive/worker.rs`・`crates/server/src/export/dump.rs`・`crates/server/src/export/ledger.rs`・`crates/server/src/export/job.rs`・`crates/server/src/export_file_tests.rs`・`osv-scanner.toml`・`tools/check-vulns.py`・`tools/check-tests-kept.sh`・`tools/release/build.sh`・`.github/workflows/ci.yml`・`.harness2/scripts/evidence.py`、crate の source（`~/.cargo/registry` の zip-2.6.1 と zip-9.7.3）
正典: `openspec/specs/dependency-vulnerability-scan/spec.md`・`openspec/specs/data-durability/spec.md`
確かめた日: 2026-11-20（worktree `ashiato2-up-ch24`、HEAD 5116188d7、`origin/main` = 7817dc595 = merge-base）。軽い路（`reason: same`）

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch24-zip-upgrade --strict` | valid（rc=0） |
| `python3 scripts/check_chain.py .` | chain: OK（0 件 / 未回収 0 件 / warn 0 件） |
| `python3 scripts/check_scenarios.py . ch24-zip-upgrade` | rc=1。**この change の FAIL は 2 件で、どちらも足す 2 つの Scenario**（「前の版の zip の部品で書いた書き出しの zip を、上げた後の版で同じ中身に読める」「上げた後に書いた書き出しの zip の項目は、前の版と同じ AES の形で書かれる」）。実装の前なので試験はまだ無い。印は tasks 1.1 が名指ししている。warn はほかの change の既存の印だけ |
| `python3 scripts/review_triage.py . ch24-zip-upgrade` | triage: OK（指摘 0 / 仮決め 0 / 要件へ戻すもの 0。この review を書く前） |
| `python3 scripts/spec_overlap.py candidates . ch24-zip-upgrade` | `[]`（0 件） |
| `python3 .harness2/scripts/evidence.py lint openspec/changes/ch24-zip-upgrade` | `[OK]` |
| `evidence.py unverified … --task 1〜4` | どれも `[]` |

検証コマンドを書いたままの形で走らせた結果（base の木で）:

| 項目 | 結果 |
|---|---|
| 1.2 の python の部分（`HEAD:Cargo.lock` に替えて） | `['2.6.1']`・rc=0。引用は書いたまま走る |
| 2.1 の python の部分 | `AssertionError: ['2.6.1']`・rc=1（base で落ちるのは正しい） |
| 2.3 の `cargo test -p ashiato-server --lib -- export archive media_export --list` | 447 本（tasks の「base で 447 本」と一致） |
| 2.4 の `st58_pc_update` | `#[test]` は 16 本（tasks の下限と一致） |
| 3.1 の python の部分 | base では `AssertionError: zip の 7 行の外が変わった`・rc=1（正しい）。表は 71 行。7 行を除いた写しは 64 行 |
| 4.1 | rc=0（いまの差分は change の中だけ） |
| `evidence.declared` が取り出すコマンド | 2.3 と 2.4 は **2 本ずつ**取り出される（2 本目は括弧の中の `--list`）。R8 |

zip の部品の版による違いは、zip 2.6.1 と 9.7.3 の両方に依る使い捨ての crate を `/tmp` に作って確かめた（後で消した。Linux で走らせ、`x86_64-pc-windows-gnu` 向けに組んだ .exe も WSL の interop で走らせた）。結果は R1・R3 に写す。

## R1. zip 9 は `enclosed_name` の判定を変える。「道の外を指す項目を断る」を固定する試験も Scenario も無く、design の Risks（既存の試験が見る）が成り立たない
- 成果物: openspec/changes/ch24-zip-upgrade/design.md（Risks・D5）・tasks.md（Task 1・2.4）・docs/changes/CH24.md（壊してはいけないもの）
- 根拠:
  - CH24.md:94 は「道の外を指す項目を断る（`enclosed_name`）」を壊してはいけないものに挙げている。`update_apply.rs:235-236` は `enclosed_name()` が `None` なら `bail!("zip の道が不正")` で更新を止める
  - `st58_pc_update.rs` にも `update_apply.rs` の `#[cfg(test)]` にも、道の外を指す項目・絶対パスの項目を渡す試験は無い（`grep -n '\.\./\|道\|enclosed' ` が `update_apply.rs:235-236` しか返さない）。`openspec/specs/*/spec.md` にも Scenario は無い
  - 使い捨ての crate で、同じ zip の項目に 2 つの版の `enclosed_name()` を当てた（Windows 向けの .exe を interop で走らせた結果。PC の収集アプリが動くのはこちら）:

    | 項目の名前 | 2.6.1 | 9.7.3 |
    |---|---|---|
    | `/etc/passwd` | None（断る） | `etc\passwd`（受け付ける） |
    | `C:\x` | None | `x` |
    | `\x` | None | `x` |
    | `a:b` | None | `b` |
    | `NUL` | `NUL`（受け付ける） | None（断る） |
    | `a.` | `a.` | None |
    | `../x`・`..\x` | None | None |

    Linux でも `/etc/passwd` は 2.6.1 が None、9.7.3 が `etc/passwd`
  - つまり、2.6.1 が更新を止めた配布物（絶対パス・ドライブの前置き・`:` を含む項目）を 9 は相対に直して展開し、2.6.1 が通した名前（`NUL`・末尾の `.`）を 9 は断る。どちらも既存の試験では落ちない。design.md:58-59 の「読み方の意味の変化（項目の名前・ディレクトリの判定・`unix_mode`）は既存の試験が見る」は、この点では成り立たない
- kind: technical
- 提案: Task 1（上げる前）に、上の名前の並びを zip の部品で書いた zip を `extract_zip` に渡し、2.6.1 の結果（どれで更新が止まるか）を記録する試験を `update_apply.rs` の `#[cfg(test)]` に置く（予定のパスの中。`--lib` なので Windows の CI の job でも走る。R2）。
  Task 2 では、2.6.1 が断っていた名前（絶対パス・前置き）を断り続けるよう `extract_zip` に自前の確かめを足し、9 が新しく断る名前（`NUL`・末尾の `.`）は断る側のまま design に D<n> として書く（配る zip の項目の名前は `tools/release/build.sh` が決めた固定のものなので、断る側で失うものは無い）。
  `distribution` の正典に Scenario を足すなら、`ch22-soak-install-online-ci` が同じ capability を触るので観点 7 の候補になる。試験で固定するだけなら要らない
- 処置: fixed D6 — `extract_zip` は 9 の `enclosed_name()` を使わず、2.6.1 と同じ名前の確かめを自前で当てる（受け付ける名前と展開先はどの OS でも 2.6.1 と同じ。R11 の後の形）。上げる前の結果を tasks 1.4 の `ch24_extract_zip_name_rules`（`update_apply.rs` の `#[cfg(test)]`・zip の部品の外で作った材料・期待値も囲みの中）で固定する。design の Risks を直した。台帳に一度載せた D6 は、R11 の後の形で振る舞いが 2.6.1 と同じになったので `provisional.py resolve` で外した

## R2. Windows の CI は `extract_zip` を 1 度も走らせない。完了の判定 7・tasks 2.4・design の Risks が Windows の確かめを過大に言っている
- 成果物: docs/changes/CH24.md（完了の判定 7）・tasks.md（2.4）・design.md（Risks）
- 根拠:
  - `.github/workflows/ci.yml` の windows-latest の job が走らせるのは `cargo test -p ashiato-collector-windows --lib`（ci.yml:252）と `--test runtime_windows`（ci.yml:270）だけ。`grep -n st58_pc_update .github/workflows/*.yml` は 0 件。`runtime_windows.rs` は `update_executable`・`UpdateLayout`・トーストを見るが、`run_update` / `extract_zip` を呼ばない。`update_apply.rs` の `#[cfg(test)]` 4 本（402・412・421・437 行）も展開しない
  - zip 9.7.3 の `enclosed_name` には `#[cfg(windows)]` の枝がある（`zip-9.7.3/src/path.rs:259-262`）。R1 のとおり、Windows の結果は Linux と違う。Linux で走る `st58_pc_update` は Windows の枝を通らない
  - 配る PC の収集アプリの zip は、zip の部品ではなく Info-ZIP の `zip -j` か PowerShell の `Compress-Archive` で作る（`tools/release/build.sh:239-257`）。`st58_pc_update.rs` の 3 本（265・358・474 行）はどれも zip の部品の `ZipWriter` で書くので、上げた後も「同じ版で書いて同じ版で読む」形のまま。配る形の zip を読めるかは見ていない
  - 完了の判定 7 は確認バッチで「`runtime_windows`・Windows 向けの clippy が通っているか」を問うが、どちらも Windows の展開の振る舞いを見ない
- kind: technical
- 提案: R1 の試験を `update_apply.rs` の `#[cfg(test)]` に置けば、Windows の CI の `--lib` で走る。あわせて、Info-ZIP の `zip -j` で作った小さな zip（このリポジトリの手元に `/usr/bin/zip` がある）をバイト列で埋めて `extract_zip` で読む試験を足す。
  2.4 の検証に `cargo test -p ashiato-collector-windows --lib --target x86_64-pc-windows-gnu` を足す（手元では WSL の interop で .exe が走る。走った本数も数える）。完了の判定 7 の問いは「Windows の CI の `--lib` で R1 の試験が走って通ったか」に直す
- 処置: fixed tasks.md — 1.4 に Info-ZIP の `zip -j` で作った zip を読む `ch24_extract_zip_reads_info_zip` を足し、名前の試験は `--lib` に置いた（CI の Windows の job で走る）。1.4 と新しい 2.5 は `--target x86_64-pc-windows-gnu` の `--lib` も走らせる（手元で 435 本が走ることを確かめた。premises.md）。完了の判定 7 の問いを Windows の `--lib` に直し、design の Risks から過大な言い方を消した

## R3. zip 9 は項目の名前の読み方も変える。Takeout の項目の名前は記録の値と台帳に残るが、実物の Takeout がどちらに当たるかは確かめていない
- 成果物: openspec/changes/ch24-zip-upgrade/premises.md・design.md・docs/changes/CH24.md（壊してはいけないもの: Takeout の書庫の取り込み）
- 根拠:
  - 2.6.1 は UTF-8 の印（汎用の印の bit 11）があれば UTF-8、無ければ CP437 で名前を読む（`zip-2.6.1/src/read.rs:1297-1300`）。9.7.3 は印を見ず、バイト列が UTF-8 として正しければ UTF-8、そうでなければ CP437 で読む（`zip-9.7.3/src/types.rs:143-151`）
  - 使い捨ての crate で、`Takeout/マイ アクティビティ/a.json` を UTF-8 で書き、印だけを消した zip を読んだ。2.6.1 は `Takeout/πâ₧πéñ πéóπé»πâåπéúπâôπâåπéú/a.json`、9.7.3 は `Takeout/マイ アクティビティ/a.json`
  - `open.rs:93` の `entry.name()` が項目のパスになる。このパスは記録の payload の `inner_path`（`worker.rs:1064`）に入り、`core.archive_file` と保留の形の行の照合（`worker.rs:1222-1264`、`remove_pending_shape`）にも使う。分類は中身と拡張子で決めるので（`classify.rs:36`）、行き先は変わらない。変わるのは記録に残る値と照合
  - 既存の Takeout の試験はどれも zip の部品で書く（日本語の名前なら印が付く）ので、印の無い書庫を通らない。premises.md に、実物の Takeout の書庫が印を持つかの行は無い
  - 9 の `ZipFile::name()` は `ZipResult<Cow<str>>` を返す（`zip-9.7.3/src/read/zipfile.rs:97`。2.6.1 は `&str`）。`open.rs` で Err を `broken_zip` にするか、別の扱いにするかを決める D<n> が無い
- kind: technical
- 提案: premises.md に「実物の Takeout の書庫の項目は UTF-8 の印を持つ」の行を足す。確かめ方は、本人の Takeout の zip を 1 本、`python3 -c 'import zipfile,sys; print({i.flag_bits & 0x800 for i in zipfile.ZipFile(sys.argv[1]).infolist()})'` で見る。
  Task 1 では、印の無い UTF-8 の名前を持つ zip を `open_archive` で読んだときの名前を、上げる前の版で試験に記録する。`name()` の Err の扱いは design に書く（`broken_zip` に寄せるのが今の形に近い）。
  実物が印を持たなければ A になる。失う側は 2 つで、上げる前に取り込んだ記録と `inner_path` が揃わなくなるか、化けた名前を残し続けるか
- 処置: fixed D7 仮 — premise の向きは人間へ返さず（P の条件: 失わない側があり、費用は表 1 つ、範囲と要件を変えない）、2.6.1 と同じ規則（bit 11 があれば `from_utf8_lossy`、無ければ CP437）で名前を作ることにした（`name_raw()` と `flags().as_u16()` は zip 9 で公開されている。CP437 の表は `open.rs` に置く）。この読み方なら実物の Takeout が印を持つかに依らず記録の名前は変わらないので、失う側が無い。`name()` を使わないので Err の扱いは要らない。上げる前の結果を tasks 1.3 で固定する。premises.md に行を足した
- 再確認: 2026-11-20 kind を premise から technical に直した。理由: D7 の読み方（bit 11 があれば `from_utf8_lossy`、無ければ 2.6.1 の CP437 の表）を、zip 2.6.1 と 9.7.3 の両方に依る使い捨ての crate（`--offline`・9.7.3 は `default-features = false`）で、9.7.3 の公開の `name_raw()` と `flags().as_u16()` から字のとおりに組み、2.6.1 の `cp437.rs` を `#[path]` で取り込んで `ZipFile::name()`（2.6.1）と比べた。印の無い UTF-8 の名前（Info-ZIP の `zip -r` で作った `Takeout/マイ アクティビティ/a.json`。印も `0x7075` も付かない）・印のある名前・0x80〜0xFF の全部のバイトを持つ印の無い名前・正しい `0x7075`（Info-ZIP Unicode Path）を持つ印の無い名前の 6 項目で、食い違いは 0。`0x7075` は 9.7.3 が `name_raw()` を書き換え、`flags()` に bit 11 を立てる（`zip-9.7.3/src/extra_fields/extra_field.rs:313-325`）ので、2.6.1（`read.rs:1474-1481`）と同じ名前になる。実物の Takeout が印を持っても持たなくても、記録に残る名前は 2.6.1 と同じになり、この前提に依る決定は残っていない（D7 の反転条件は上げる前からの読み方の良し悪しで、この change の決定ではない）。残る差は、CRC の合わない `0x7075` を持つ書庫を 2.6.1 が開けず 9 が読むことだけで、印の前提には依らない（R12）

## R4. 前の版で書いた書き出しを台帳が「書いたまま」と判定することを、どの Scenario も見ていない。前の書き出しを消すかどうかはこの判定で決まる
- 成果物: openspec/changes/ch24-zip-upgrade/specs/data-durability/spec.md
- 根拠:
  - CH24.md:92 は「台帳の照合 `export/ledger.rs` も注記を読む」と名指しする。spec.md:12 も「書き出しの台帳の照合（注記を読む）もそれを読む」と書く
  - 製品が前の版の書き出しを読むのは、`ledger::is_untouched_with`（ledger.rs:170-195。`ZipArchive::new` と `comment()`）を `job.rs:383` が呼ぶ所だけ。これが false なら、前の書き出しは消さない側に倒れる（ledger.rs:164）。正典 data-durability:21-23 は「書き終えたら、前にこのシステムが書いた書き出しを消して 1 つだけ残す」と「本人が変えたファイルは消さない」を、この判定で分けている
  - 足す Scenario 1（spec.md:15-19）の WHEN は「試験が … 上げた後の版で開き」で、試験の中で zip の部品を直に使う形。tasks 1.1 も同じ。上げた後の版で、台帳の照合が前の版の書き出しを開けなくなっても、この Scenario は落ちない
- kind: technical
- 提案: Scenario を 1 つ足す。WHEN: 上げる前の版で書いた書き出し（埋め込んだバイト列を置き場に置き、大きさと SHA-256 を台帳の done の行に持つ）がある状態で、上げた後の版で新しい書き出しを書き終える。THEN: 前の書き出しのファイルは消え、置き場には新しい 1 つだけが残る。
  あるいは、置いたファイルについて台帳の照合（`is_untouched_with`）が true を返すことを試験で見る（spec には関数の名前を書かず、振る舞いで書く）
- 処置: fixed specs/data-durability/spec.md — Requirement に台帳の照合の SHALL を足し、Scenario「以前の版で書いた書き出しは、次の書き出しを書き終えると消える」を置いた。試験は tasks 1.2 の `ch24_previous_export_replaced_after_next_export`（`export_job_tests.rs`）。design D8

## R5. Scenario 2 は、2 つの zip の項目の名前の集まりが同じことを求める。書き出しのファイルが増えるたびに、正典のこの Requirement を書き直すことになる
- 成果物: openspec/changes/ch24-zip-upgrade/specs/data-durability/spec.md（spec.md:7-8・21-24）
- 根拠:
  - spec.md:23 は「2 つの zip の項目の名前の集まりは同じ」。片方は埋め込みで固定され、もう片方はその時点の木の書き出しになる
  - 書き出しの項目の一覧（`export_file_tests.rs:27` の `ENTRIES`。いまは 18 本）は、ST33 の後に 4 回増えた（`git log -G 'const ENTRIES: \[&str; [0-9]+\]'`: aa3f2337b・ab0806401・c1eea51d5・119bc6a19）。次に項目を足す Story はこの Scenario で落ち、archive の後の正典を MODIFIED で書き直すことになる
  - Requirement の本文（spec.md:8）は「同じ内容の同じ項目で」AE-1 / AE-2 の別が同じと書く。Scenario（spec.md:24）は「同じ名前の項目の」と書く。README.txt と manifest.json は、書いた時刻・書き出しの id・移行の版（`dump.rs:216-219`）を持つので、2 回の書き出しで中身が同じにならない。本文と Scenario で、比べる相手が違う
- kind: technical
- 提案: THEN を「上げた後の書き出しのどの項目も、強さ `3`・実の圧縮方式 `8`」と「両方の zip にある名前の項目は、AE-1 / AE-2 の別が同じ」に分ける。名前の集まりが同じことは求めない。本文の「同じ内容の同じ項目」は「同じ名前の項目」に揃える。
  AE-1 / AE-2 は書いた量が 20 バイト未満かで決まる（premises.md:11）。埋め込みには 20 バイト未満の項目（0 行の `.jsonl`）と 20 バイト以上の項目の両方が入るよう、記録の置き方を決めて tasks 1.1 に書く。片方しか無いと、片方の枝は比べられない
- 処置: fixed specs/data-durability/spec.md — 名前の集まりが同じことは求めず、THEN を「新しく書いた zip のどの項目も …」と「両方にある名前の項目は AE-1 / AE-2 の別が同じ」に分けた。本文も「同じ名前の項目」に揃えた。tasks 1.1 と design D2 に、20 バイト未満の項目と以上の項目の両方を出す記録の置き方を書いた

## R6. AES の形の確かめが中央ディレクトリの `0x9901` だけで、正典の「汎用の道具で開ける」は上げた後も zip の部品どうしでしか見ない
- 成果物: openspec/changes/ch24-zip-upgrade/specs/data-durability/spec.md（spec.md:22-24）・design.md（D2）
- 根拠:
  - 正典 data-durability:442-445「このシステムを使わずに中身を読める」（AES の zip を開ける汎用の道具で開く）の試験 `export_readme_readable_without_the_system`（export_file_tests.rs:813-）は、`ZipArchive`（zip の部品そのもの）で開いている。上げた後は、9 で書いて 9 で読む確かめになる
  - 足す Scenario 2 が別の実装の代わりに読むのは、中央ディレクトリの追加の欄 `0x9901` だけ（vendor version・強さ・実の圧縮方式）。汎用の道具が見る他の欄は比べない: 局所ヘッダの `0x9901`、圧縮方式 `99`、汎用の印（暗号化の bit 0・データ記述子の bit 3）、必要な版、`large_file(true)`（dump.rs:171）が付ける zip64 の欄。premises.md:12 は「手元に AES の zip を読める別の道具が無い」とするが、python3 の標準の `zipfile` は AES を解けなくても、中央ディレクトリと局所ヘッダの形（`compress_type == 99`・`flag_bits`・`extra`）は別の実装として読める
- kind: technical
- 提案: Scenario 2 の比べる欄に、局所ヘッダと中央ディレクトリの両方の `0x9901`・圧縮方式 `99`・汎用の印の暗号化の bit を足す。2 つの zip で同じことを見る。
  加えて、上げた後の書き出しを python3 の `zipfile` で開き、全部の項目の名前が読めて、`compress_type == 99` かつ `flag_bits & 1` であることを、検証の 1 本（tasks 2.2 の後ろ）にする。zip の部品の外の実装で、形が壊れていないことの下限になる
- 処置: fixed specs/data-durability/spec.md — 比べる欄に局所ヘッダと中央ディレクトリの圧縮方式 `99`・暗号化の bit を足した。tasks 2.2 に、上げた後の書き出しを `CH24_DUMP_DIR` に写して python3 の `zipfile` で全項目の `compress_type == 99` と `flag_bits & 1` を見る確かめを足した。design D2

## R7. tasks 1.2 は、上げた後に埋め込みを作り直しても通る
- 成果物: openspec/changes/ch24-zip-upgrade/tasks.md:33-34
- 根拠: `git log -S CH24_PREVIOUS_EXPORT … | tail -1` が返すのは、`CH24_PREVIOUS_EXPORT` という字が**初めて現れた** commit。`-S` は字の出る回数が変わった commit しか拾わないので、上げた後にバイト列だけを書き直した commit は拾わない（定数の名前の出る回数は変わらない）。design.md:31-32 と tasks.md:33・21 は「上げた後に作り直していないこと」を見ると書くが、見ているのは初めの commit の `Cargo.lock` だけ
- kind: technical
- 提案: 初めの commit `$c` と HEAD で、埋め込みの部分が同じであることも見る。たとえば、埋め込みを `// CH24-FIXTURE-BEGIN` と `// CH24-FIXTURE-END` で囲み、`diff <(git show "$c:crates/server/src/export_file_tests.rs" | sed -n '/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p') <(sed -n '/CH24-FIXTURE-BEGIN/,/CH24-FIXTURE-END/p' crates/server/src/export_file_tests.rs)` を 1.2 に足す。
  あるいは、`git log --format=%H -G '<埋め込みの行の型>' -- crates/server/src/export_file_tests.rs` が返す全部の commit について、`Cargo.lock` が 2.6.1 であることを見る
- 処置: fixed tasks.md — 埋め込みを `// CH24-FIXTURE-BEGIN` / `END` で囲み、tasks 1.5 が初めの commit の `Cargo.lock` と、その commit と HEAD の囲みの中が同じことを 3 つのファイルで見る。design D2

## R8. 試験の本数の下限が、どの検証でも機械に見られていない。2.3・2.4 の `--list` は本数に依らず rc=0 で、証跡には PASS と残る
- 成果物: openspec/changes/ch24-zip-upgrade/tasks.md（1.1・2.2・2.3・2.4・3.1）
- 根拠:
  - `evidence.declared` は 2.3 から `cargo test -p ashiato-server --lib -- export archive media_export --list` を、2.4 から `cargo test -p ashiato-collector-windows --test st58_pc_update -- --list` を検証のコマンドとして取り出す（この review で `tasks_phase.parse` と `evidence.declared` を当てた）。`--list` は 0 本でも rc=0 なので、「449 本以上」「16 本以上」は見られないまま PASS が残る
  - 1.1・2.2 の「`-- --list` で 2 本以上」は backtick の外にあり、走らない。`ch24_` の試験が 0 本でも `cargo test … ch24_` は rc=0（AGENTS.md の「0 本でも rc=0」）
  - 小さいもの: 3.1 は `out=$(tools/check-vulns.sh --base "$base") && printf …` なので、検査が rc≠0 のときは出力が捨てられ、何で落ちたかが証跡に残らない
- kind: technical
- 提案: 本数をコマンドの中で数え、下限を割ったら落とす。例:
  `bash -c 'n=$(cargo test -p ashiato-server --lib -- export archive media_export --list 2>/dev/null | grep -c ": test$"); echo "$n"; test "$n" -ge 449'`。
  1.1・2.2 も同じ形（`ch24_` で `-ge 2`）にし、括弧の中の `--list` の backtick は消す（2 本目として取り出されないように）。3.1 は `out=$(…); rc=$?; printf "%s\n" "$out"; test $rc -eq 0 && …` の形にする
- 処置: fixed tasks.md — どの検証も本数をコマンドの中で数え、下限を割ったら落とす形にした（1.1〜1.4・2.2〜2.5）。括弧の中の `--list` の backtick は消した（`evidence.declared` が各項目から 1 本だけ取り出すことを確かめた）。3.1 は検査が落ちても出力を残す形にした

## R9. 足す Requirement が zip の部品の名前と版を持ち、Scenario 1 は 3 つの主張を束ねている
- 成果物: openspec/changes/ch24-zip-upgrade/specs/data-durability/spec.md
- 根拠:
  - Requirement の名前と本文（spec.md:3・5・7-9）は「zip の部品の版を上げても」「上げる前の版（zip 2.6.1）」「部品の版が変われば」と書く。crate の名前と版で、実装の名前にあたる。archive の後は data-durability の正典に残り、次に zip を上げたときには「上げる前の版」が何を指すかがずれる
  - Scenario 1（spec.md:15-19）は THEN と AND 2 つで、前の版の中身を読める・注記を読める・違う合言葉では読めない、の 3 つを束ねている。違う合言葉で読めないことは、正典の「合言葉が無ければ zip の中身を読めない」（data-durability:54-56）とは別の書き出し（前の版で書いたもの）についての主張
- kind: technical
- 提案: Requirement を製品の振る舞いで書く。例:「サーバの版が変わっても、以前の版のサーバが書いた書き出しを同じ中身で読め、新しく書く書き出しは以前と同じ AES の形を持つ」。2.6.1 は「試験に埋め込む書き出しは 2026-11 の版（zip 2.6.1）で書いたもの」として、Scenario の WHEN か design D2 に置く。
  Scenario 1 は「違う合言葉では読めない」を別の Scenario に分ける
- 処置: fixed specs/data-durability/spec.md — Requirement を製品の振る舞い（以前の版のサーバが書いた書き出し）で書き、2.6.1 は本文の後ろの説明と design D2 に移した。「違う合言葉では読めない」を別の Scenario に分けた。試験の名前と tasks 1.1 も揃えた

## R10. lane.json と deep.md は「3 件の例外を外す」と書き、design D3・tasks 3.1 の 7 行と合わない
- 成果物: openspec/changes/ch24-zip-upgrade/lane.json（why）・deep.md:7
- 根拠: lane.json の why と deep.md:7 は「osv-scanner.toml の 3 件の例外を外す」。design.md:37-40・tasks.md:50・CH24.md:106 は 7 行（GHSA 3 件と RUSTSEC 4 件）。触るファイル（`osv-scanner.toml`）は同じなので、軽い路の裏取りは変わらない
- kind: technical
- 提案: lane.json は裏取り済みなので直さない。PR 本文で、消す行が 7 行であることと、その理由（D3: 同じ zip 2.6.1 の勧告で、残すと「勧告がもう出ない」で落ちる）を名指しする
- 処置: rejected: lane.json は裏取り済みで、deep.md は軽い路で機械が置く申告の写しなので、どちらも直さない。触るファイル（`osv-scanner.toml`）は同じで裏取りは変わらない。消す行が 7 行であることと理由（D3）は PR 本文（`docs/briefs/CH24-pr.md`）に名指しした

## 観点ごとの確かめ

- **観点 1（議論で決めたことが写っているか）**:
  - 「何を」: zip を 9.0.5 以上へ（tasks 2.1 の検証が `>=(9,0,5)` を見る）。GHSA 3 件を外す（tasks 3.1）
  - 「どこまで」: 4 か所（tasks 2.1）、AES の zip が今と同じに読める（specs の 2 つの Scenario）。振る舞いを変えないことの固定には穴がある（R1〜R4）
  - 「上げきれないとき」: design D4 は、本人の決めた代わり（理由を `上げる予定: 2 → 9 の上げの作業が残る` に直し、期限を 90 日以内に置く）を添えて、計画の直しとして返す。代わりの中身は変えておらず、本人の決定と矛盾しない。完了の判定 1〜4 と検証が成り立たなくなるので返す、という理由も CH24.md:110 と合う。
    ただし、zip 9 の判定の違い（R1）のように、組み立ても試験も通るのに振る舞いが変わるものは、D4 の「上げきれない」（tasks.md:18。Task 2 の検証が通らない）に当たらない。R1 の試験を置かなければ、D4 の出番が来ないまま通る
  - D3 の RUSTSEC 4 行: 正典 dependency-vulnerability-scan:73-74・117（PR の検査でも「勧告がもう出ない」で落ちる）に照らして、消すのが必然。表の 7 行は `osv-scanner.toml:107・171・199・247・251・255・259` にあり、理由はどれも `上げる予定: 互換範囲外の修正版待ち`、期限は 2027-02-06 で、premises と一致する。消した行は `== 例外の表で足した・変えた行` に出ない（`check-vulns.py:205-213` は足した行と変えた行しか出さない）。3.1 が `なし` を求めるのは正しい
  - 要件へ戻すもの: なし
- **観点 2**: R5・R9。WHEN / THEN はどちらの Scenario もバイト列と SHA-256 で観測できる
- **観点 3**: design の D1〜D5 に、specs に置くべき観測可能な振る舞いは無い。R3 の `name()` の Err の扱いは、決めれば D<n> になる（Takeout の振る舞いは正典の別の capability に属する）。specs の実装の名前は R9。proposal の Capabilities（data-durability）は specs/ のディレクトリと一致する。CH24.md の front matter の `dependency-vulnerability-scan` は、例外の表（データ）を変えるだけで delta を持たない。正典の振る舞いは変わらないので、食い違いではない
- **観点 4**: R7・R8。依存の順（Task 1 の埋め込み → Task 2 の上げ → Task 3 の表）は正しい。人間の確認待ちは無く、CH24.md:112 とも合う。Windows の確かめは R2
- **観点 5（change）**: 完了の判定は空でない。依頼（逐語）の範囲は、RUSTSEC 4 行（D3 に理由あり）の分だけ越える。「壊してはいけないもの」のうち、道の外の項目（R1・R2）・Takeout の名前（R3）・台帳の照合（R4）は Scenario にも試験にもなっていない。そのほかは既存の試験と `check-tests-kept.sh` が見る
- **観点 6**: 該当なし（`spec --answers` の呼び出しではない）
- **観点 7**: 該当なし（`spec_overlap.py candidates` が 0 件）

## R11. D6 の確かめは Linux で `..\x` を新しく断る。1.4 は Linux で 2.6.1 の「受け付ける」を固定し、2.1 はその期待値を直させないので、上げた後に 2.4 が落ちる
- 成果物: openspec/changes/ch24-zip-upgrade/design.md（D6）・tasks.md（1.4・2.1・2.4）・docs/changes/CH24.md（完了の判定 2）
- 根拠:
  - 2.6.1 の `enclosed_name` は `std::path::Path` の成分で見る（`zip-2.6.1/src/types.rs:606-621`）。Linux では `\` が区切りでないので `..\x` は `Normal` の 1 成分になり、受け付ける。
    9.7.3 は どの OS でも `typed_path::Utf8WindowsPath` で成分に分ける（`zip-9.7.3/src/path.rs:17-25`）ので、`..\x` は根の外へ出る `ParentDir` になり、断る（`path.rs:235-250`）
  - 2 つの版に依る使い捨ての crate で、D6 のとおり「2.6.1 の確かめ」と「9 の `enclosed_name()`」の両方で受け付けたものだけを通す関数を当てた（Linux）:

    | 名前 | 2.6.1 | 9.7.3 | D6 |
    |---|---|---|---|
    | `..\x` | `..\x`（受け付ける） | None | None（断る） |
    | `a\..\..\x` | `a\..\..\x` | None | None |
    | ほかの 1.4 の 8 つ（`/etc/passwd`・`C:\x`・`\x`・`a:b`・`../x`・`NUL`・`a.`・`ashiato-collector.exe`・`sub/a.txt`） | — | — | 2.6.1 と同じ |

    Windows 向けの .exe（WSL の interop）では、D6 が 2.6.1 と違うのは `NUL`・`a.`・`ab:c` だけ（`ab:c` は D6 の書く「`:` を含む成分」に当たり、1.4 の並びには無い）
  - tasks.md:48-49 は `..\x` を 1.4 の名前の並びに入れ、「Windows と Windows 以外で期待値を `cfg` で分ける」。Linux の 2.6.1 の期待値は「受け付ける」になる。
    tasks.md:61 は「Windows の期待値のうち `NUL`・`a.` の 2 つだけを直す（ほかの期待値は変えない）」。上げた後の 2.4（`cargo test -p ashiato-collector-windows`、Linux）で `ch24_extract_zip_name_rules` が落ち、
    直してよい期待値の外なので、Task agent は D4 の「上げきれない」に倒れるか、約束を破って直すかになる
  - design.md:73 の「2.6.1 より厳しくなるのは Windows の装置の名前・末尾の `.`・`:` を含む成分だけ」と CH24.md:89 の「D6 で新しく断る `NUL`・末尾の `.` を除く」も、Linux では成り立たない
- kind: technical
- 提案: どちらかに決めて design D6・tasks 2.1・CH24.md の完了の判定 2 を揃える。
  (a) 9 の `enclosed_name()` を `#[cfg(windows)]` でだけ当てる（Linux は 2.6.1 の確かめだけで、`..\x` の期待値が変わらない）。
  (b) 9 の `enclosed_name()` を当てない（2.6.1 の確かめだけ。受け付ける名前が 2.6.1 とどの OS でも同じになり、本人の「振る舞いは変えない」にいちばん近い）。
  (c) いまの D6 のまま、2.1 で直してよい期待値に Linux の `..\x` を足し、D6 の「厳しくなるのは」に `\` を含んで根の外へ出る名前（Linux）を足す
- 処置: fixed D6 — 提案の (b) を採った。9 の `enclosed_name()` を使わず 2.6.1 と同じ確かめだけにし、tasks 2.1 から期待値の直しを消した（Task 1 の期待値は変えない）。1.4 の並びに `a\..\..\x`・`ab:c` を足した。CH24.md の完了の判定 2 と proposal も揃えた

## R12. D7 の「2.6.1 と同じ規則」は印（bit 11）だけを写す。2.6.1 は Info-ZIP Unicode Path（`0x7075`）も読み、CRC の合わない `0x7075` を持つ書庫を開けない（`broken_zip`）が、9 は読む
- 成果物: openspec/changes/ch24-zip-upgrade/design.md（D7）・docs/changes/CH24.md（壊してはいけないもの: 読めない zip を `broken_zip` として台帳に残す）
- 根拠:
  - 2.6.1 は中央ディレクトリの追加の欄 `0x7075` を見つけると、CRC を確かめて名前を置き換え、印を立てる（`zip-2.6.1/src/read.rs:1474-1481`）。CRC が合わない・UTF-8 として正しくないと `Err` を返し、
    書庫ごと開けない（`read.rs:1335-1340` で `Io` 以外の誤りは返る）。9.7.3 は CRC が合わない・UTF-8 でない `0x7075` を無視して見出しの名前を使う（`zip-9.7.3/src/extra_fields/extra_field.rs:313-325`）
  - python3 の `zipfile` で、見出しの名前 `Takeout/b.json` に CRC の合わない `0x7075` を付けた zip を作り、使い捨ての crate で開いた: 2.6.1 は `ZipArchive::new` が `InvalidArchive("Could not find EOCD")`、
    9.7.3 は開けて名前は `Takeout/b.json`。`open.rs:74-78` は前者を `broken_zip` にするので、上げた後はこの書庫が読まれる
  - CRC の合う `0x7075` は、9.7.3 が `name_raw()` を書き換えて `flags()` に bit 11 を立てるので、D7 の読み方で 2.6.1 と同じ名前になる（確かめた。R3 の再確認）
  - design.md:84 の「この読み方は失敗しない」は名前の読み方としては正しいが、書庫を開けるかどうかの差には触れていない。1.3 の試験も `0x7075` を持つ書庫を通らない
- kind: technical
- 提案: D7 に 1 行足す: CRC の合わない `0x7075` を持つ書庫は、2.6.1 では `broken_zip`、9 では読める側に変わる。読める側に変わるだけで、取り込んだ記録と名前は変わらず、失うものは無いので受け入れる（実物の Takeout の印の有無には依らない）。
  あわせて 1.3 に、CRC の合う `0x7075` を持つ印の無い名前の試験を 1 本足すと、D7 が 9 の `flags()`（置き換えの後の値）を読んでいることを固定できる
- 処置: fixed D7 仮 — CRC の合わない `0x7075` を持つ書庫が `broken_zip` から読める側に変わることを D7 に書き、失うものが無いので受け入れるとした。正しい `0x7075` の名前の試験は足さない（R3 の再確認が 2.6.1 と同じ名前になることを確かめた。9 が `name_raw()` を置き換えるので D7 の実装は分岐を持たない）

## R13. Task 1 で足す試験の期待値（1.3 の名前の文字列・1.4 の受け付ける / 断る）は囲みの外にあり、上げた後に書き換えても何も落ちない
- 成果物: openspec/changes/ch24-zip-upgrade/tasks.md（Global Constraints・1.3・1.4・1.5・2.1・2.3）
- 根拠:
  - 1.5 が見るのは囲み（`CH24-FIXTURE-BEGIN` / `END`）の中だけ（tasks.md:54）。1.3 で囲むのは zip のバイト列で（tasks.md:45「バイト列を、囲みの行で囲んで埋め込む」）、期待値「2.6.1 で読んだ値をそのまま書く」は囲みの中と書かれていない。
    1.4 も囲むのは zip のバイト列（tasks.md:51）で、名前ごとの受け付ける / 断るの期待値は囲みの外。2.1 は 1.4 の期待値のうち 2 つを直させる（tasks.md:61）ので、期待値はもともと上げた後に触る場所にある
  - `tools/check-tests-kept.sh origin/main …`（2.3）は `git diff origin/main...HEAD` で減った印・名前・1 行の期待値を数える（`check-tests-kept.sh:15-21`）。ch24 の試験は origin/main に無く、差分には最後の形で「足した行」として出るので、
    Task 1 の commit と Task 2 の commit の間で期待値を書き換えても見えない。Global Constraints（tasks.md:17）は Task 1 で足す試験を「新しい API に合わせる直しだけ」の外に置いている
  - 1.1 の期待値（項目の名前の並び・SHA-256・書き出しの id）は囲みの中（tasks.md:34）なので、この穴は 1.3・1.4 だけ
- kind: technical
- 提案: 1.3 の期待値の文字列と、1.4 の 2.6.1 の期待値の表（名前と OS ごとの受け付ける / 断る）も囲みの中に置くと tasks に書く。2.1 で直す期待値は囲みの外に別の小さな表（D6 で変わる名前だけ）として置き、
  試験は「囲みの中の 2.6.1 の表に、囲みの外の D6 の上書きを当てたもの」と比べる。1.5 はそのままで、2.6.1 の期待値の書き換えを落とせる
- 処置: fixed tasks.md — Global Constraints・1.3・1.4 に、2.6.1 で読んだ期待値もバイト列と一緒に囲みの中に置くと書いた。R11 の (b) で Task 2 に直す期待値は無くなったので、囲みの外の上書きの表は要らない

## R14. 1.3 は自前に写す CP437 の表のうち、`マイ アクティビティ` が使う数字しか見ない
- 成果物: openspec/changes/ch24-zip-upgrade/tasks.md（1.3）・design.md（D7）
- 根拠: D7 は CP437 の 0x80〜0xFF の 128 字を `open.rs` に手で写す（design.md:83-84）。1.3 の期待値は `Takeout/マイ アクティビティ/a.json` の UTF-8 のバイト（0xE3・0x82・0x83 と その後の数字の十数種）の読み方だけで、
  ほかの 100 余りの字を写し違えても 1.3 も 2.2 も通る。使い捨ての crate で、0x80〜0xFF の全部のバイトを名前に持つ印の無い zip を作れて、2.6.1 の `name()` で読めることを確かめた（`ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº…`）
- kind: technical
- 提案: 1.3 の埋め込みに、0x80〜0xFF の全部のバイトを名前に持つ印の無い項目を 1 つ足し、2.6.1 で読んだ文字列を期待値にする（R13 のとおり囲みの中に置く）
- 処置: fixed tasks.md — 1.3 の印の無い書庫に、0x80〜0xFF の全部のバイトを名前に持つ項目を足し、2.6.1 で読んだ文字列を期待値にする

## R15. 書き直しの後に古くなった参照が 2 つ残っている
- 成果物: openspec/changes/ch24-zip-upgrade/design.md（D2）・premises.md
- 根拠:
  - design.md:31 と design.md:33 は「tasks 1.2 が git で確かめる」「tasks 1.2 が … 同じことも見る」と書く。git の確かめは 1.5 に移り（tasks.md:53-54）、いまの 1.2 は台帳の照合の試験
  - premises.md:19 は Scenario「上げた後に書いた書き出しの zip の項目は、前の版と同じ AES の形で書かれる」を名指す。specs の Scenario の名前は「新しく書いた書き出しの zip の項目は、以前の版と同じ AES の形で書かれる」（spec.md:30）に変わった
- kind: technical
- 提案: design D2 の 2 か所を「tasks 1.5」に、premises.md:19 の Scenario の名前を spec.md:30 の名前に直す
- 処置: fixed design.md — D2 の 2 か所を tasks 1.5 に直し、premises.md の Scenario の名前を spec.md のものに直した

## R16. D6・D7 は観測できる振る舞い（更新が断る名前・取り込んだ記録に残る名前）を決めるが、design にしか無く、archive で正典から落ちる
- 成果物: openspec/changes/ch24-zip-upgrade/design.md（D6・D7）・proposal.md（Capabilities）
- 根拠: D6 は、更新の配布物のどの項目の名前で更新を止めるかを決め、Windows では 2.6.1 より多くを断る（振る舞いの変化）。D7 は、Takeout の項目が記録の `inner_path` にどう残るかを決める。
  どちらも Scenario は無く、試験（1.3・1.4）に `// Scenario:` の印も無い。`openspec archive` は main specs しか更新しないので、次に zip を上げる change はこの 2 つを正典から見つけられない。
  proposal.md の Capabilities は `data-durability` だけで、D6 の振る舞いの属する capability（`distribution` か PC の更新の capability）と D7 の（Takeout の取り込みの capability）を挙げていない
- kind: technical
- 提案: D6（R11 の決め方の後の形）と D7 をそれぞれの capability の Scenario にする（`distribution` は `ch22-soak-install-online-ci` が触るので、足すなら観点 7 を見直す）。
  正典に置かないなら、その理由（「振る舞いを変えない」を試験で固定するだけで、正典の約束を足さない）を D6・D7 に書き、反転条件に「次に zip を上げるとき」を足す
- 処置: fixed D6 — D6・D7 に正典へ置かない理由（振る舞いを変えないことを試験で固定するだけで正典の約束を足さない・`distribution` は ch22 が触る）と、次に zip を上げる change が D6・D7 と 1.3・1.4 から引き継ぐことを書いた

## 再確認（2026-11-20）

worktree `ashiato2-up-ch24`、HEAD 5116188d7 + 作業ツリーの変更（`git diff HEAD`: CH24.md・design・premises・proposal・specs・tasks と、未追跡の provisional.jsonl・review/）。成果物は直していない。

機械の検査（この追記の前）:

| コマンド | 結果 |
|---|---|
| `openspec validate ch24-zip-upgrade --strict` | valid（rc=0） |
| `python3 scripts/check_chain.py .` | chain: OK（0 件 / 未回収 0 件 / warn 0 件） |
| `python3 scripts/check_scenarios.py . ch24-zip-upgrade` | FAIL: 担保の無い Scenario 4 件（足した 4 本。実装の前なので試験はまだ無い。名前は tasks 1.1・1.2 の名指しと逐語で一致） |
| `python3 scripts/review_triage.py . ch24-zip-upgrade` | FAIL 1 件: R3 が kind=premise で fixed（指摘 10 / 仮決め 2） |
| `python3 scripts/spec_overlap.py candidates . ch24-zip-upgrade` | `[]` |
| `python3 .harness2/scripts/evidence.py lint …` | `[OK]`。`evidence.declared` は 1.1〜4.1 のどの項目からも 1 本だけ取り出す（R8 の処置どおり） |

処置が写っているか:

- R1・R2: D6（仮）・tasks 1.4 / 2.5・CH24.md の完了の判定 7・design の Risks に写っている。ただし D6 は Linux で 2.6.1 と食い違う（R11）
- R3: D7（仮）・tasks 1.3・premises.md に写っている。kind を technical に直した（R3 の再確認）。残る差は R12
- R4: Requirement の 2 つ目の SHALL・Scenario 3・tasks 1.2・D8 に写っている。`is_untouched_with` は大きさ・注記（`ashiato-export <id>`）・SHA-256 を見る（`ledger.rs:170-195`）ので、Scenario 3 の試験は台帳の行の id を埋め込みの id に揃える必要がある（試験を書けば分かるので指摘にしない）
- R5・R6・R9: specs に写っている。名前の集まりの比べは消え、比べる欄に圧縮方式・暗号化の bit が入り、Requirement は製品の振る舞いで書かれ、違う合言葉は別の Scenario になった。tasks 2.2 の python3 の確かめもある
- R7: tasks 1.5。一時の clone（base に囲みの 3 ファイルを足した commit）で書いたままの 1.5 を走らせ、rc=0。囲みの中の 1 字を変えた commit を足すと `囲みの中が変わった` で rc=1。base の木では `囲みが無い` で rc=1
- R8: どの検証も本数を数えて下限で落とす形。下限の勘定: 1.1 は 3、1.2 は 4、1.3 は `ch24_takeout_` で 2、1.4 / 2.4 / 2.5 は collector-windows の `ch24_` で 2、2.2 はサーバの `ch24_` で 6（3+1+2）、
  2.3 は 447+6=453（足す 6 本は `export_file_tests`・`export_job_tests`・`archive_tests` の中にあり、絞り込み `export` `archive` に当たる。`lib.rs:42・103・106`）。どれも合っている。
  `grep -c` は 0 本で rc=1 になり、1.1〜1.3 は `&&` で、ほかは `|| exit 1` / `test` で落ちる
- R10: rejected。`docs/briefs/CH24-pr.md`（gitignore）に「消す例外の行は 7 行」の節があることを確かめた
- 書いたまま走らせた部分（base の木）: 1.5 は rc=1（囲みが無い）、2.1 の python3 は `AssertionError: ['2.6.1']`、3.1 の python3 は `zip の 7 行の外が変わった`、4.1 は rc=0。どれも base で落ちるべきものが落ちる
- 2.5 の前提: base の `ashiato_collector_windows-*.exe`（`--lib`、Windows 向け）を WSL の interop で走らせ、434 passed・1 ignored・0 failed（57 秒）。premises の「`--list` で 435 本」だけでなく、全部が通ることも確かめた
- 2.2 の `$TMPDIR` は verify-run が検証ごとに渡す（`evidence.py:425・481`）
- 予定のパス: tasks の触るファイル（`archive_tests.rs`・`export_job_tests.rs`・`update_apply.rs` など）はどれも lane.json の paths の中。4.1 の許す一覧とも一致する
- 正典: specs は ADDED の 1 Requirement だけで、既存の Requirement を変えない（proposal と一致）

確かめに使った使い捨ての crate（`/tmp` に置き、`cargo build --offline`。9.7.3 は手元に既定の features の依存が無いので `default-features = false`。名前の判定と読み方は features に依らない）は、終わってから消した。

追記の後の `review_triage.py`: 指摘 16 / 仮決め 2。R3 の FAIL は消え、FAIL は R11〜R16 の「処置が無い」6 件（書き手が処置を付ける）。
