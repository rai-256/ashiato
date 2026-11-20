<!-- 処置の判定: re-review -->
# CH24 code_verify — zip の部品を 2 から 9 に上げる

HEAD dd6c62265（作業ツリーは clean）。base は `git merge-base origin/main HEAD` = 65eb1de27。`Cargo.lock` の zip は `9.0.5` の 1 つだけ。

## 申告: 14/14（全部 `[x]`）・検証コマンド一覧と実測

検証コマンドは tasks.md の各項目の `検証:` を逐語で取り、上から順に 1 回ずつ走らせた（ログは `/tmp/ch24v_log_<項目>.txt`）。

| 項目 | 検証コマンドの要点 | 申告 | 実測 | 一致 |
|---|---|---|---|---|
| 1.1 | `cargo test -p ashiato-server --lib ch24_`（3 本以上） | rc=0 | rc=0・`ch24_: 6` | 一致 |
| 1.2 | 同上（4 本以上） | rc=0 | rc=0・`ch24_: 6` | 一致 |
| 1.3 | `--lib ch24_takeout_`（2 本以上） | rc=0 | rc=0・`ch24_takeout_: 2` | 一致 |
| 1.4 | collector `--lib ch24_` を Linux と windows-gnu で（各 2 本以上） | rc=0 | rc=0・両方 2 本 | 一致 |
| 1.5 | `check-fixtures.sh`（訂正後） | rc=0 | rc=0・`OK: 3 つのファイルの囲みは zip 2.6.1 の木で書いた版のまま` | 一致 |
| 2.1 | 既定の features・zip が 1 版で 9.0.5 以上・clippy 3 本 | rc=0 | rc=0・`['9.0.5']`・両 crate `uses_default_features: True` | 一致 |
| 2.2 | `ch24_` 6 本以上 + python `zipfile` で全項目 `compress_type==99` と `flag_bits&1` | rc=0 | rc=0・`ch24_: 6`・18 項目 | 一致 |
| 2.3 | `check-tests-kept.sh` ×2 + `export archive media_export` 453 本以上 | rc=0 | rc=0・`453 passed`（174 秒） | 一致 |
| 2.4 | collector 全試験・`st58_pc_update` 16 本以上・`ch24_` 2 本以上 | rc=0 | rc=0・16 本・2 本 | 一致 |
| 2.5 | collector `--lib --target x86_64-pc-windows-gnu`・`ch24_` 2 本以上 | rc=0 | **1 回目 rc=1**（change の外の `clock::tests` 2 本。R5）、走らせ直すと rc=0・436 passed・`ch24_ windows: 2` | 一致（揺れあり） |
| 3.1 | 例外の表は zip の 7 行のほか不変・`check-vulns.sh --base` | rc=0 | rc=0・`新しく持ち込んだ: 0 件`・足した・変えた行 `なし` | 一致 |
| 3.2 | 一覧は `bzip2-1.0.6` を足すだけ・`check-licenses.sh rust`・`--self-test`・`check-updater-deps.sh` | rc=0 | rc=0・`ライセンス OK`・`self-test OK` | 一致 |
| 4.1 | diff と未追跡が予定のパスの中だけ | rc=0 | rc=0 | 一致 |
| 4.2 | `check-fixtures.sh`（訂正後） | rc=0 | rc=0 | 一致 |

`cargo test --test <名前>` は 2.4 の `--test st58_pc_update` だけで、`crates/collector-windows/tests/st58_pc_update.rs` は実在する。
`tasks.md` が挙げる試験の名前（`ch24_previous_export_reads_same` などの 8 本）は、どれも実在した。

## 手 1: 固定値を独立に再計算する — 全部一致

- **書き出しの埋め込み**（`export_file_tests.rs` の `CH24_PREVIOUS_EXPORT_*`）: zip の部品を使わずに python で読んだ（`cryptography` の AES-CTR と `hashlib.pbkdf2_hmac`（SHA-1・1000 回）で WinZip AES を解き、`zlib` の raw deflate で伸ばす）。
  結果は、項目の名前の並び 18 個・SHA-256 の 18 個・注記 `ashiato-export 4f17c2d4-…c10124` がどれも定数と一致した。違う合言葉 `wrong-password-for-ch24` は 18/18 の項目で検証値の照合に落ちた。
  中身が空の項目は 12 個（0 行の `.jsonl`）で、20 バイト以上の項目もある。
- **CP437 の表**（`open.rs` の `CP437`、`archive_tests.rs` の `CH24_RAW_CP437_80_FF` `CH24_CP437_NAME` `CH24_CP437_80_FF`）: python の `bytes(range(0x80,0x100)).decode('cp437')` と 128 字すべて一致した（0xFF は U+00A0）。
- **2.6.1 の結果**: repo の外の使い捨ての crate（`zip = "=2.6.1"`、`--offline`）で、埋め込みの zip をそのまま読んだ。
  - `enclosed_name()` の結果は、Linux で `[F,T,T,T,F,T,T,T,T,T,T,T]`。`--target x86_64-pc-windows-gnu` の .exe を WSL の interop で走らせると `[F,F,F,F,F,F,T,T,T,T,F,T]`。どちらも `CH24_EXTRACT_EXPECTED_UNIX` / `_WINDOWS` と一致した
  - Takeout の名前は、印の無い 2 項目が `CH24_CP437_NAME` / `CH24_RAW_CP437_80_FF`、印のある項目が `Takeout/マイ アクティビティ/a.json` で、試験の期待値と一致した
- **D5 / D7 の前提を 9.0.5 で**（design は 9.7.3 の source を引く）: `zip-9.0.5/src/format/functions.rs:26,45,58` に `clear_specials_bits` と `&= !0o222` があり、D5 の記述は 9.0.5 でも成り立つ。
  正しい CRC の `0x7075` を持つ印の無い項目は、9.0.5 の `name_raw()` が `マイ.json` で印 `0x800`、2.6.1 の `name()` も `マイ.json` だった。
  CRC の合わない項目は、2.6.1 が開けず 9.0.5 は見出しの名前で読んだ。D7 の記述どおり。

## 手 2: ガードをわざと壊す

どれも作業ツリーで 1 つずつ壊し、`/tmp` に退避したファイルで戻した。毎回、戻した後の `git status --porcelain` と `git diff` が空であることを確かめた。

| 壊したもの | 走らせたもの | 結果 |
|---|---|---|
| `open.rs` の `zip_name(&entry)` を 9 の `entry.name()` に差し替え | `cargo test -p ashiato-server --lib ch24_takeout_` | rc=101。`ch24_takeout_unflagged_name_reads_as_cp437` が落ちた |
| `ledger.rs:190` の注記の比較を常に不一致に | `--lib ch24_previous_export_replaced` | rc=101。`export_job_tests.rs:174` で落ちた（大きさと SHA-256 だけでは消えない） |
| `update_apply.rs` の `ParentDir` を `saturating_sub` に | collector `--lib ch24_`（Linux） | rc=101。`../x` で落ちた |
| `update_apply.rs` の `Prefix \| RootDir => return None` を `=> ()` に | collector `--lib ch24_`（Linux / windows-gnu） | **Linux rc=0**、windows-gnu rc=101（R2） |
| `dump.rs:170` の `AesMode::Aes256` を `Aes128` に | `--lib -- export archive media_export` と 2.2 の python | `ch24_` は全部通った。2.2 の python も通った。落ちたのは既存の `export_zip_every_entry_is_aes_256` の 1 本だけ（R1） |
| 許諾の一覧から `bzip2-1.0.6` を外す（`ALLOW_LICENSES=<base の 17 値>`） | `tools/check-licenses.sh rust` | rc=1。`NG libbz2-rs-sys: bzip2-1.0.6` |
| `check-licenses.sh` の `ALLOW` に `GPL-3.0-only` を足す | 3.2 の検証 | rc=1。`一覧に bzip2-1.0.6 を足すほかの変更がある` |
| 予定のパスの外に未追跡のファイル `crates/server/src/ch24v_probe.rs` を置く | 4.1 の検証 | rc=1。そのパスを出した |
| 囲みの中を書き換える（作業ツリーと、clone での commit） | `check-fixtures.sh` | rc=1（手 7） |

## 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . ch24-zip-upgrade` は rc=0（`scenarios: OK`。Scenario 3121 件 / 担保あり 3120 / 人間の確認待ち 1 は change の外）。
`specs/data-durability` の 4 本は、それぞれの試験に印が逐語である。

| Scenario | 試験 | 主張を観測しているか |
|---|---|---|
| 以前の版で書いた書き出しの zip を、同じ中身で読める | `ch24_previous_export_reads_same` | している（名前の並び・SHA-256・注記。手 1 で独立に一致） |
| 以前の版で書いた書き出しの zip は、違う合言葉では読めない | `ch24_previous_export_wrong_password_fails` | している（全項目で `by_index_decrypt` が Err） |
| 以前の版で書いた書き出しは、次の書き出しを書き終えると消える | `ch24_previous_export_replaced_after_next_export` | している（注記の比較を壊すと落ちる） |
| 新しく書いた書き出しの zip の項目は、以前の版と同じ AES の形で書かれる | `ch24_export_aes_shape_matches_previous` | **一部だけ**。強さ `3` と `AE` の印を読んでいない（R1） |

`specs/distribution` の Scenario「許可の一覧は、AGPL-3.0-only を外したほかは前と同じ」は、`check-licenses.sh --self-test` が一覧の文字列を丸ごと比べる。`ALLOW_LICENSES=<base の 17 値> tools/check-licenses.sh --self-test` は rc=1 で、その Scenario の行で止まった。

## 手 4: 本人の決定が test で固定されているか

`deep.md` と `docs/changes/CH24.md` の本人の決定から、値・する / しないを取った。

| 決定 | 書き換えたときに落ちるもの |
|---|---|
| 許す一覧に `bzip2-1.0.6` を足し、ほかは足さない・外さない（Q-G1） | `check-licenses.sh --self-test`（一覧を丸ごと比べる）。`bzip2-1.0.6` を外すと rc=1。`rust` の節も rc=1（`NG libbz2-rs-sys`）。ほかの値を足すと 3.2 の比較が rc=1 |
| zip を 9.0.5 以上にする | `Cargo.lock` だけを base の版（zip 2.6.1）に戻して `tools/check-vulns.sh --base HEAD` を走らせると、rc=1・`新しく持ち込んだ: 9 件`（zip 2.6.1 の RUSTSEC / GHSA）。merge の後に版を下げる PR は落ちる。`--base 65eb1de27`（zip 2.6.1 の base）では rc=0 で、この PR の中で下げた場合は 2.1 の 1 回きりの確かめしか見ない |
| 書き出しの AES-256・deflate・注記 | 既存の `export_zip_every_entry_is_aes_256` などと、CH24 の 4 本。ただし強さは CH24 の試験からは見ていない（R1） |
| 2.6.1 の名前の規則（D6・D7） | `ch24_extract_zip_name_rules`・`ch24_takeout_*`。Linux の `/etc/passwd` だけは理由を見ていない（R2） |
| **zip の既定の features のまま・bzip2 も読み続ける（Q-G1・D1）** | **無い**。値を変えても全部通る（R3） |

## 手 5: tasks の `[x]` と実体

全 14 項目のコマンドが実在し、rc=0 になった（上の表）。2.5 だけ 1 回目に change の外の試験が揺れた（R5）。
`[x]` で、挙がっている試験やコマンドが無いものは無かった。

## 手 6: 隙間 — 該当なし

確かめた範囲は 4 つ:
- Takeout の名前: `0x7075` の正しい CRC と合わない CRC（手 1。D7 の記述どおりで、名前の変わる経路は無かった）
- 埋め込みの書き出しを 9.0.5 で読む: 全項目と注記が読めた
- 台帳の照合: 前の版の書き出しは消える側に正しく倒れる。注記で決まることを手 2 で確かめた
- PC の更新の展開: 2.6.1 と同じ名前の判定。Linux と Windows の両方で 12 個の名前を確かめた

`unix_mode` の SUID などの bit が落ちる差（D5）は design が既知として挙げていて、`#[cfg(unix)]` の枝だけに効く。
実物の Takeout の書庫を 9 が `broken_zip` に倒すかどうかは、材料が無く走らせていないので、指摘にしない。

## 手 7: 機械が承認した検証コマンドの訂正 — 2 件とも落ちた

`plan_fix.py pending` は 2 件（1.5 `2026-11-20T14:57:42+00:00`、4.2 `2026-11-20T16:14:45+00:00`）。新しい入口はどちらも `check-fixtures.sh`。
囲みの中を壊すと、どちらも rc=1 で落ちた。4.2 は、上げた後の commit での書き直し（使い捨ての clone）でも rc=1 だった。詳細は `correction-review.md`。

---

## R1. Scenario「以前の版と同じ AES の形」の「強さが 3」を、CH24 の試験も 2.2 の python も見ていない
- 成果物: crates/server/src/export_file_tests.rs / openspec/changes/ch24-zip-upgrade/tasks.md（2.2）
- 根拠: `ch24_aes_extra`（`export_file_tests.rs` の `fn ch24_aes_extra`）は、`0x9901` の欄の +4..+5（vendor version）と +9..+10（実の圧縮方式）だけを返す。+6..+7 の `AE` の印と +8 の強さは読んでいない。
  `dump.rs:170` を `AesMode::Aes128` にして `cargo test -p ashiato-server --lib -- export archive media_export` を走らせると、rc=101・`452 passed; 1 failed` で、`ch24_export_aes_shape_matches_previous` は通った。
  落ちたのは既存の `export_zip_every_entry_is_aes_256` の 1 本だけで、これは zip の部品の `get_aes_verification_key_and_salt` で読む。spec は「部品を使わずにバイト列から読んで確かめる」と言っている。
  同じ壊し方で書いた `ch24.zip` に 2.2 の python の確かめを当てると、rc=0（18 項目）。部品の外で python で読むと、強さは `{1}`（埋め込みは `{3}`）だった。
  壊していない今の木で書いた `ch24.zip` を同じ python で読むと、全項目で `(99, 1, vendor, b'AE', 3, 8)` になり、名前ごとの vendor は埋め込みと同じだった。振る舞いは正しく、試験の観測が足りないだけ
- kind: technical
- 提案: `ch24_aes_extra` で +8 の強さと +6..+7 の `AE` も返し、試験で `strength == 3` を新旧の両方に当てる。2.2 の python の確かめにも `0x9901` の強さ `3` を足す
- 処置: fixed 1.1 / 2.2。`ch24_aes_extra` が AE の印と強さを読み、名前で組にした新旧の全項目で AES-256（強さ 3）・deflate（実の方式 8）を確かめるようにした。2.2 の Python の確かめも同じ AES extra の値を検査する。

## R2. PC の更新の展開の名前の試験は、Linux では `/etc/passwd` を断る分岐を観測していない（権限の拒否で Err になるだけ）
- 成果物: crates/collector-windows/src/update_apply.rs（`ch24_extract_zip_name_rules`）
- 根拠: 試験は `extract_zip(..).is_ok()` だけを比べ、Err の理由を見ていない。`/etc/passwd` は root の持ち物（`-rw-r--r-- root root`）で、試験は uid 1001 で走る。`enclosed_zip_path` の `Prefix(_) | RootDir => return None` を `=> ()` にすると、次のとおり。
  - `cargo test -p ashiato-collector-windows --lib ch24_` は **Linux で rc=0**（2 passed）。`/etc/passwd` に書けずに Err になったとみられ、期待の `false` と一致する
  - windows-gnu では rc=101（`assertion failed: /etc/passwd`）。ただしこのとき WSL の interop の .exe が `C:\etc\passwd` を作った（検証者が消した）
  - `ParentDir` の枝を壊すと、Linux で rc=101 になる一方、`$TMPDIR/x` を一時ディレクトリの外に書いた（消した）

  断る名前が断られなかったとき、Err の理由が名前の判定でなければ、試験は通るか、外へ書いてから落ちる
- kind: technical
- 提案: 断る名前は Err の文（`format!("{e:#}")`）に `zip の道が不正` を含むことまで見る。または、`enclosed_zip_path` を名前ごとに直に呼ぶ表の試験を足し、外へ書かないで判定を観測する
- 処置: fixed D6 — 展開を試す表に、書き込みの成否とは別に `enclosed_zip_path` 自体の受理・拒否を同じ名前の期待値で確かめる assert を加えた。

## R3. 本人の決定「zip の既定の features のまま・bzip2 も読み続ける」（Q-G1・D1）を固定する試験が無い
- 成果物: crates/server/Cargo.toml / crates/server/src/archive_tests.rs
- 根拠: `crates/server/Cargo.toml` の zip を `{ version = "9", default-features = false, features = ["aes-crypto", "deflate"] }` にした。
  そのうえで `cargo test --offline -p ashiato-server --lib -- archive ch24_` を走らせると、rc=0・`256 passed; 0 failed` だった。
  `cargo tree -p ashiato-server -i libbz2-rs-sys` は `did not match any packages` で、bzip2 を読めない木でも全部通る。
  `grep -rn "bzip\|ZIP_BZIP2" crates/server/src/*tests*.rs` は 0 件。この決定を見るのは、tasks 2.1 の 1 回きりの `cargo metadata` の `uses_default_features` だけで、試験にも CI にも無い
- kind: technical
- 提案: python の `zipfile.ZIP_BZIP2`（部品の外）で縮めた Takeout の項目を持つ zip を埋め込み、`open_archive` が中身を読める試験を `archive_tests.rs` に 1 本足す。lzma・zstd も既定が読む方式として、同じ形で並べてよい
- 処置: fixed D1 — 外部の `zipfile` で bzip2 圧縮した Takeout を `open_archive` が読める試験を `archive_tests.rs` に加え、既定の features が bzip2 を保つことを固定した。

## R4. `ch24_takeout_*` の `// Scenario:` の印が、spec に無い Scenario を指している
- 成果物: crates/server/src/archive_tests.rs
- 根拠: `python3 scripts/check_scenarios.py . ch24-zip-upgrade` が次の 2 件を出した。
  - `[warn] spec に無い Scenario を指す印: Takeout/マイアクティビティ/a.jsonをUTF-8のバイトで書いた印の無い書庫はCP437で読む`
  - 同じく `…UTF-8の印付きで書いた書庫はUTF-8のまま読む`

  design D7 は「正典に Scenario を置かない」と決めていて、印は宙に浮いている。あわせて、同じ囲みの `CH24_TAKEOUT_UNFLAGGED_ZIP_B64` と `CH24_CP437_80_FF` は `let _ =` で捨てられ、どの assert にも届いていない（tasks 1.3 が求めた「python で作り bit 11 を消した」材料はこれで、試験が読むのは別の `CH24_RAW_CP437_ZIP_B64`）
- kind: technical
- 提案: 印を `// Scenario:` でない説明の注記（design D7 を指す）に直す。使わない 2 つの定数は囲みの中なので消さず、試験で読む（印の無い 2 項目を `CH24_CP437_NAME` / `CH24_CP437_80_FF` と比べる）か、そのまま残す理由を注記する
- 処置: fixed D7 — 正典に無い `Scenario` 印を D7 を指す説明へ替え、2.6.1 が書いた deflate fixture の 2 項目を開いて CP437 の期待値まで検査するようにした。

## R5. collector の `clock::tests` の 2 本が、windows-gnu の `--lib`（tasks 2.5・CI の Windows の job と同じ）で 1 回落ち、走らせ直すと通った
- 成果物: crates/collector-windows/src/clock.rs（change の外。ST05）
- 根拠: 2.5 の検証の 1 回目は rc=1 だった。`clock_reference_pc_ahead_gives_positive_difference`（`skew=301046 width=32`、許す範囲は `300_000..301_032`）と `…_behind_gives_negative_difference`（`skew=-298953`）が落ちた。
  2 本とも許す範囲の端から十数ミリ秒外れた（`301046` は上の端 `301032` より 14 ms、`-298953` は上の端 `-298968` より 15 ms 外）。同じコマンドを走らせ直すと rc=0（436 passed）。落ちた原因は確かめていない。
  CH24 は `clock.rs` に触れていない（`git diff 65eb1de27 --stat` に無い）。持ち主は `docs/stories/ST05.md`（題が「端末時計のずれを測って残す」）。ST05 は `openspec/changes/archive/2026-10-05-st05-clock-skew` で閉じている。
  時刻の試験の揺れを扱った `archive/2026-11-07-ch09-load-tolerant-timing-tests` は `clock_reference` に触れていない（grep 0 件）。受け手の Story は無い
- kind: defer
- 提案: CH24 の確認バッチの問い（「PR の CI の Windows の job の `--lib` が通っているか」）で、clock の 2 本の落ちを CH24 の落ちと取り違えないよう添える。直すのは新しい change（`hx change`）で、落ちる理由を確かめてから許す範囲を決める
- 処置: deferred ST05 — CH24 が触れていない `clock::tests` の揺れであり、閉じた ST05 の時計試験を新しい change で原因から扱う。CH24 の Windows `--lib` は再実行で通り、CH24 の名前試験 2 本も通った。

## security review（65eb1de..4fb17bb）

見た経路:
- PC の更新: GitHub の版の一覧 → `download_asset` → `verify_digest`（`update_apply.rs:163`）→ `extract_zip`（`:168`）→ 新しい `enclosed_zip_path`（`:258-273`）→ `destination.join` → `File::create`。展開は指紋の照合の後だけ（2）。名前の確かめは、Windows と Linux の本物のファイルシステムで、展開先の外へ書ける名前が 0 件（4）
- PC の更新の権限: `#[cfg(unix)]` の `unix_mode` の写し。zip 9 は SUID・SGID・sticky を落とすので、2.6.1 より狭くなるだけ。配る物（Windows）では走らない（2）
- Takeout の書庫: 置き場の `.zip` → `open_archive`（`archive/open.rs:77-123`）→ `zip_name`（`name_raw` と CP437 の表）→ `ArchiveFile.path`（記録の `inner_path`）。名前はディスクの道・SQL の文字列・シェルには入らない（展開しない）。2.6.1 と同じ規則（3）
- 書庫の失敗: `ZipError` は `OpenError.source` にだけ残り、外に出るのは `kind()` の固定の文字列。zip 9 の `InvalidArchive(Cow)` の書式付きの文は型名・数・固定の文だけで、項目の名前を含まない（`zip-9.0.5/src` の `invalid!(` を grep）（3）
- 書き出し: `dump.rs:166-171` の deflate + AES-256 → `set_comment`（Result を返すようになっただけ。注記は `ashiato-export <id>` で中身を含まない）。zip 9 が書いた AES の項目を、別の版の実装（zip 2.6.1）で、平文が無い・違う合言葉で読めない・正しい合言葉で同じ中身、と確かめた（4）
- 試験に埋め込んだ前の版の書き出し（`export_file_tests.rs` の `CH24_PREVIOUS_EXPORT_B64`）: 解くと記録は試験が置いた 3 件（2001-02-20・`CH24 fixture`・東京駅の座標）だけ。開発の DB の記録は入っていない。合言葉の固定値は試験専用（4）
- 依存: 新しく入った 16 の crate（RustCrypto の `hmac`・`sha1`・`sha2`・`digest`・`cmov`・`ctutils`・`cpubits`・`hybrid-array` など、`typed-path`・`libbz2-rs-sys`・`lzma-rust2`・`ppmd-rust`）は、どれも crates.io の index から引かれ、`Cargo.lock` に checksum がある。どれも zip 9.0.5 の `Cargo.toml` が名指しする名前で、`repository` は RustCrypto・trifectatech・作者本人の repo。`build.rs` も proc-macro も持たない（3）
- 待ち受け・接続先・認可: この差分は API・router・listen・送信先に触れていない（`git diff --stat` に `lib.rs`・`net_guard.rs`・`sender.rs` が無い）（2）

安全の決め手: PC の更新の展開は、`enclosed_zip_path` が `Path::components` の Prefix・RootDir と、根を越える `..` を断るので、受け付けた名前を `destination.join` してもどの OS でも展開先の外に出ない —— 確かさ 4。repo の外の使い捨ての crate に `enclosed_zip_path` を写し、23 の名前（`C:\x`・`\\?\C:\x`・`\\server\share\x`・`C:x`・`\\.\PhysicalDrive0`・`a/..\..\x`・`a/C:\x`・`ab:c` など）について、展開先に本物のファイルを作り、親を `canonicalize` して展開先の下かを見た。Linux（`cargo run`）と Windows（windows-gnu の .exe を `cmd.exe` から）の両方で `ESCAPING_ACCEPTED=0`。Windows の `a/C:\x` は受け付けるが、作るときに os error 123 で失敗する（2.6.1 と同じ規則）。repo の試験は `cargo test -p ashiato-collector-windows --lib ch24_` が 2 passed、`cargo test -p ashiato-server --lib ch24_previous_export_wrong_password_fails` が 1 passed

指摘なし

範囲外（R 番号なし）:
- `update_apply.rs:151-166` の指紋（`asset.digest`）は、配る物と同じ GitHub の版の一覧から取っていて、配る物の出所を独立に確かめてはいない（ST58。この差分は触れていない）
- `archive/open.rs:113` の `read_to_end` には、項目ごとの大きさの上限も合計の上限も無い（展開後に大きくなる zip を読み込む。この差分の前から同じ）

## /code-review（65eb1de..977a149）

見た: Claude Code の組み込みの `/code-review`（6 件）。写しは scripts/code_review_record.py

## R6. AES の形の試験は、仕様が求める AES の強さ（`3` = 256 ビット）を一度も読まないので、強さが落ちても気づかない。`ch24_aes_extra` が返すのは vendor version（`extra[p+4..p+6]`）と実の圧縮方式（`extra[p+9..p+11]`）だけで、強さのバイト `extra[p+8]` を読んでいない。しかも項目は名前ではなく並び順で組にしている（L374 の `old.iter().zip(new.iter())`）。Fix: `extra[p+8] == 3` を確かめ、実の圧縮方式は以前の値と比べるのではなく `8` と直に比べ、組は名前で作る。
- 成果物: crates/server/src/export_file_tests.rs:176
- 根拠: zip の版を上げたことで、新しい書き出しが AES-128（強さ `1`）で書かれるようになったとする。`ch24_export_aes_shape_matches_previous` は (method, flags&1, vendor, actual) しか比べないので通ってしまい、Scenario「強さが `3`」は検証されないまま緑になる。新しい版で項目の並びが変わったときも、別の名前の項目どうしの AE-1 / AE-2 を比べることになる。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 / 2.2。R1 と同じ変更で、強さ・実の圧縮方式を直接に検査し、項目を名前で対応付けた。

## R7. UTF-8 の印が立った書庫で、名前に不正な UTF-8 が入っていると、黙って U+FFFD に置き換えて取り込むようになった。以前（zip 2.6.1）は名前を厳密に UTF-8 として読み、失敗すれば `by_index` が落ちて broken_zip として台帳に残っていた。新しい `zip_name` は `from_utf8_lossy` を使う。Fix: 印付きの分岐では `std::str::from_utf8` が失敗したら broken_zip の OpenError を返す（以前の振る舞いを保つ）。
- 成果物: crates/server/src/archive/open.rs:20
- 根拠: 印が立っていて、名前のバイトが違う 2 つの項目（どちらも不正な UTF-8）を持つ Takeout の zip を読む。どちらも同じ `Takeout/�.json` という path の ArchiveFile になり、以前は「読めない形の書庫」として台帳に残ったものが、名前の衝突した中身として取り込まれる。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: zip 2.6.1 の `read.rs:1297-1300` は UTF-8 印付きの名前に `String::from_utf8_lossy` を使う。現行の `zip_name` も同じ規則であり、`from_utf8` へ替えると D7 の「2.6.1 と同じ規則」を壊す。

## R8. Scenario「印の無い書庫は CP437 で読む」の試験は、2.6.1 の書き手で作った deflate の Takeout の fixture（`CH24_TAKEOUT_UNFLAGGED_ZIP_B64`）と、その期待値（`CH24_CP437_80_FF`）を `let _ =` で捨てている。実際に開いているのは手で組んだ raw の fixture だけになっている。Fix: `CH24_TAKEOUT_UNFLAGGED_ZIP_B64` を開き、2 つ目の項目の名前が `CH24_CP437_80_FF` と一致することも確かめる。
- 成果物: crates/server/src/archive_tests.rs:506
- 根拠: 以前の版が実際に書いた形（deflate、UTF-8 のバイトを CP437 で読み替えた名前）を今の版で読むと名前が変わる、という回帰が起きても、この試験は raw の fixture しか見ていないので緑のまま通る。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D7 — 未使用だった `CH24_TAKEOUT_UNFLAGGED_ZIP_B64` と `CH24_CP437_80_FF` を実際の assert へつないだ。

## R9. zip 9 には、より厳しい `enclosed_name`（Windows の道の規則を全環境で当て、区切りを含む要素も弾く）がある。それを使わず、2.6.1 の緩い検査を自前で写したので、Windows で問題のある名前が通り続ける。NTFS の代替データストリーム（`ab:c`）や予約名（`NUL`）は受け入れ、試験（`CH24_EXTRACT_EXPECTED_WINDOWS` の true）がその振る舞いを固定している。また `#[cfg(unix)]` の枝は SUID を含む mode をそのまま写す（RUSTSEC-2026-0367 と同じ型。design では「直さない」としている）。Fix: zip 9 の `entry.enclosed_name()` に切り替えるか、`:` を含む要素と予約名を弾く。
- 成果物: crates/collector-windows/src/update_apply.rs:258
- 根拠: Windows で、名前が `ashiato-collector.exe:evil` の項目を持つ配布物を展開すると、ファイルではなく代替データストリームに書かれる。`NUL` は NUL デバイスに書かれ、何も残らないまま成功する（指紋の照合が前にあるので、届くのは署名側の配布物に限られる）。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: D6 は 2.6.1 と同じ名前の判定を保つため `enclosed_name()` を使わないと明記し、`ab:c` と `NUL` の Windows の受理も fixture として固定している。提案のより厳しい判定は振る舞いの変更になる。

## R10. `ch24_zip64_offset` は、追加の欄の中で最初に現れる `01 00` を zip64 の見出しとみなしている。ところが AES の欄の vendor version（AE-1）も `01 00` なので、AES の欄が先に並ぶと誤った位置を読む。さらに、2 つの大きさがどちらも zip64 側にある前提で `+4+16` を固定している。Fix: 追加の欄を (id, size) の単位で順にたどり、id が `0x0001` の欄だけを読む。
- 成果物: crates/server/src/export_file_tests.rs:168
- 根拠: 中央ディレクトリの局所オフセットが 0xFFFFFFFF で、追加の欄が AES（`01 99 07 00 01 00 ...`）、zip64 の順に並ぶ書庫では、AES の欄の中の `01 00` を zip64 と取り違える。その結果オフセットが壊れ、`assert_eq!(&bytes[local..local + 4], b"PK\x03\x04")` が落ちるか、範囲外の参照で panic する。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed 1.1 — zip64 の extra field を id・size ごとにたどり、`0x0001` の field だけから、実際に 32 bit 値が飽和した size の後にある offset を読むようにした。AES field が先にあり size が zip64 でない例も試験した。

## R11. `zip = "9"` を既定の features のまま取り込んだため、外から届く Takeout の書庫を読むサーバに、新しい展開器（`ppmd-rust`・`lzma-rust2`）と `nix`・`typed-path` が入った。そのうえ、使わない bzip2 のために許諾の許可の一覧へ `bzip2-1.0.6` を足している。書き出しが使うのは deflate と AES だけ。Fix: `default-features = false, features = ["deflate", "aes-crypto", ...]` のように必要なものだけに絞り、収集アプリ側（`crates/collector-windows/Cargo.toml` L28）も同じにする。
- 成果物: crates/server/Cargo.toml:32
- 根拠: 攻撃者が置いた Takeout の zip の項目が PPMd（method 98）で圧縮されていると、今回新しく入った `ppmd-rust` の展開器がサーバの中で走る。以前の版では、この経路はそもそも存在しなかった。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: rejected: openspec/changes/ch24-zip-upgrade/deep.md:21-22 の本人の答え Q-G1 と D1 は、読み取り方式を狭めないため両 crate の zip 9 を既定の features のままにし、bzip2 の許諾を加えると決めている。features を絞る提案はその A の決定と矛盾する。
