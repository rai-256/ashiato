# impact-review — ch24-zip-upgrade

seen: IMP-01 low, IMP-02 low, IMP-03 medium

## R1 — zip 9 が増やす「開けない書庫」の側が、失うものに入っていない（Takeout の取り込み）
- 対象: impact.lost[]（2 件目）・impact.findings（該当なし）・impact.overview・impact.situations[carryover].cannot
- 種類: 新しい帰結
- 指摘: 失うものの 2 件目は「CRC の合わない 0x7075 の書庫が `broken_zip` から読める側へ変わる」の一方向だけ。zip 9 は逆向きの厳しくなった検査も持つ。
  `zip-9.7.3/src/types.rs:357` が項目を開くとき（`find_content`）に局所ヘッダと中央ディレクトリを突き合わせ（`src/read/mod.rs:476-592`・`406-430`）、
  名前・`0x7075`・圧縮方式・暗号化の印・CRC32・圧縮後/展開後の大きさ（データ記述子待ちを除く）が食い違えば `invalid!` を返す。さらに印（bit 11）があって `0x7075` が名前と違う書庫も、`validate_language_encoding_upath`（`mod.rs:598-611`）が断る。
  2.6.1 の `find_content_seek`（`zip-2.6.1/src/read.rs:337-376`）は局所ヘッダをデータの開始位置を出すために読むだけで、突き合わせない。
  `open.rs:84-87` は `by_index` の Err を全部 `broken_zip` にする。つまり 2.6.1 で取り込めていた書庫が 9 で `broken_zip` になりうる。D7 の「失うものは無い」・
  overview の「できなくなることは 2 つ」・`situations[carryover].cannot`（見つけていない）は、この向きを調べずに言い切っている（review/spec.md R12 も 0x7075 の CRC の向きしか見ていない）。
  実物の Takeout の書庫がそうなるかは未確認なので、断定ではなく `unknowns` に置くのが正しい。取り込み（本人の主要な入口）が前に通った書庫で止まりうるので、重大度は少なくとも medium で、`unknowns` が要る。
- 直し方の案: IMP を 1 つ足し（根拠は上の 2 つの source の行、`basis: derived`、`unknowns` に実物の Takeout の局所/中央の一致を確かめていないこと）、`lost` と `overview` と `carryover.cannot` に反映する。試験で見るなら「局所と中央の名前/大きさが食い違う書庫を 2.6.1 が読み 9 が断る」固定を tasks 1.3 に足す。
- 処置: kept: 突き合わせは既定では走らない。`zip-9.7.3/src/read/zip_archive.rs:920-927` は `options.enforce_header_consistency` が立っているときだけ `validate_header_consistency` を呼び、`ZipReadOptions` は `#[derive(Default)]` で既定 false（`src/read/mod.rs:296-317`）、`by_index` は `ZipReadOptions::new()` を渡す（`zip_archive.rs:873-874`）。`find_content`（`types.rs:229-239`）は開始位置と終わりの桁あふれしか見ない。open.rs は `by_index` だけを使うので、この向きで新しく断る書庫は出ない（provisional-review.md D7 と同じ結論）。ただし `ZipArchive::new` の中央ディレクトリの読み込みで 9 が新しく断る書庫を全部は洗っていないので、go.json の `coverage.not_examined` に足した

## R2 — D8 の根拠の行が「行の検査が落ちる」を支えていない
- 対象: impact.dependencies[D8].evidence（E8）
- 種類: 根拠
- 指摘: E8 は `tools/check-vulns.py:294` の `print(f"新しく持ち込んだ: ...")`（`--base` の新規分の表示）。D8 の主張「zip の行を残したまま上げると、行の検査が落ちる」は
  `check-vulns.py:263`（`stale` の算出）と `302-303`（`勧告がもう出ない` を出して rc=1）が支える行で、294 行目ではない。結論は正しいが、挙げた行は別の枝（新規の持ち込み）を指している。
- 直し方の案: E8 の loc/quote を `check-vulns.py:302` の `勧告がもう出ない` に差し替える（または 263 を加える）。
- 処置: fixed: E8 を `tools/check-vulns.py:302`（`勧告がもう出ない`）に差し替えた

## R3 — 切り戻しの害が「見える」とは言えない（IMP-02 の harm）
- 対象: impact.findings[IMP-02].harm.who・impact.situations[rollback]
- 種類: どれくらい困るか
- 指摘: `harm.who` は「置き場に書き出しが 2 つ残るのが見える」。だが前の版が読めない書き出しは `is_untouched` が false なので `removable_previous`（`ledger.rs:216-219`）にも `leftover`（`job.rs:379-384`）にも入らず、
  状態の画面や API には何も出ない。本人が見るのは置き場のファイルを自分で開いたときだけで、再び上げるまで（9 が読めて消すまで）毎回の書き出しで 1 つずつ増え続ける可能性がある。
  「消さない側に倒れる」は正しいが「見える」は根拠が無く、増える量も書いていない。重大度 low 自体は、中身が失われないので妥当。
- 直し方の案: `harm.who` を「状態には出ない。置き場を自分で見たときだけ分かる」に直し、`unknown` に「戻したまま書き出しを重ねると残りが増える」を足す。
- 処置: fixed: R4 の確かめで IMP-02 は起きないと分かり dismissed にした。harm.who は「状態には出ず、置き場を自分で見たときだけ分かる」に直した（ledger.rs の leftover は untouched だけを出す）

## R4 — 「前の版で読める」を今この場で確かめられるのに、未確認のまま IMP-02 / D3 に置いている
- 対象: impact.dependencies[D3]・impact.findings[IMP-02].unknowns
- 種類: 閉じ方
- 指摘: IMP-02 の不明は「2.6.1 が zip 9 の書いた zip の注記を読めるか」。注記は EOCD の末尾で AES に依らず、2.6.1 の `ZipArchive::new`/`comment()` と 9 の書き出しの組を使い捨ての crate で走らせれば 1 回で分かる
  （design が D6/D7 で既に同じ手を使っている）。確かめられることを `unconfirmed` のまま残し、tasks にも 2.2 の python3 `zipfile` の下限（これは 2.6.1 の読みを見ない）しか無い。
  `interim` / `safe_now` に当たる手当ても IMP に書かれていない。
- 直し方の案: 2.6.1 の `ZipArchive` で上げた後に書いた zip を開いて `comment()` を比べる 1 手を tasks に足すか、D3 を確かめて `holds` を決める。
- 処置: fixed: 2026-11-20 に repo の外の使い捨ての 2 つの crate（zip =9.7.3 の既定の features で AES-256 + deflate + 注記 `ashiato-export 0193` の zip を書く／zip =2.6.1 で開く）を `cargo run --offline` で走らせた。結果: 2.6.1 は注記 `ashiato-export 0193` を読み、合言葉で 0 バイトの項目（AE-2）と 71 バイトの項目（AE-1）を解き、違う合言葉ではどちらも Err。IMP-02 を dismissed、依存 D3 を holds: yes（observed）にした

## R5 — IMP-03 は「外の道具で解けるか」の確かめ方が、このリポジトリで実行できる形になっていない
- 対象: impact.findings[IMP-03].verify・options
- 種類: 閉じ方
- 指摘: `verify` は `7z x -p`。premises（行 17）は 7z も pyzipper も手元に無いと確かめている。選択肢 2 の「7-Zip などを入れて」は誰がどこで入れるかを持たず、acceptance の 3 つ目も「決めている」だけで決める席が無い。
  また本文は「前の版の書き出しも確かめていない」ので、上げる前後の差としては何も増えていない（比較の基準が無い）。medium は、書き出しが退避の唯一の写しになる使い方がありうる点で妥当だが、
  「このまま受け入れる」を選んだ場合の `if_accepted` 相当（何が起きなくなったと言えるか）が書かれていない。
- 直し方の案: 選択肢 2 に「どの道具・どの層（CI か確認バッチか）」を 1 つ決めて書くか、受け入れた場合に前の版と同じ未確認であることを `if_accepted` として明記する。
- 処置: fixed: IMP-03 の verify を、手元に道具が無いことを前提に「確認バッチで本人の Windows の 7-Zip で解く」形に直し、選択肢 2 に層（確認バッチ）を書き、if_accepted（残るもの・範囲・上げる前と同じ未確認であること）を足した

## R6 — situations の `can` が、まだ存在しない Scenario / 試験を根拠に「できる」と言っている
- 対象: impact.situations[update].can・[carryover].can・[recovery].can（basis: spec）
- 種類: 言い切り
- 指摘: 3 件とも refs は本 change が新しく足す Scenario（「以前の版で書いた書き出しは、…消える」「…同じ中身で読める」）。これらは tasks 1.2 の試験が上げた後の版で通ることを前提にしており、
  いま `Cargo.lock` は 2.6.1 で、9 で通った実績は無い。`basis: spec` は「仕様に書いてある」であって「その場面でできると確かめた」ではない。`cannot` の「この分析では見つけていない」も、
  R1 のとおり局所/中央の突き合わせという見つけうるものを調べていない状態で書かれている。`mixed.cannot` は輸出の外の道具（IMP-03）で、この場面（PC の更新の古い版と新しい版）の `cannot` ではない。
- 直し方の案: `can` を「試験（tasks 1.x）で固定する予定」の形にして basis を unconfirmed に寄せるか、上げた後に通ったことを確かめてから spec とする。`mixed.cannot` は PC の更新側の未確認を書く。
- 処置: fixed: update / carryover / recovery の can を「仕様が求め、試験（tasks 1.1・1.2）が上げた後の版で確かめる」形にした。mixed.cannot は PC の更新の場面の内容に直し、外の道具の件は recovery.cannot に移した

## R7 — D6 の `holds: yes`（配る zip の名前が ASCII）は、台本の 2 つの作り方の片方しか見ていない
- 対象: impact.dependencies[D6]
- 種類: 根拠
- 指摘: E10 は `archive_collector` の呼び出し行（`build.sh:218`）で、名前を決めるのは `archive_windows`（`build.sh:237-`）。`zip -j` の枝は平らな基底名になるが、`powershell.exe` の枝は別の作り方で、
  どちらで配る zip が作られるかは E10 から言えない（CI/release のどちらが使うか未確認）。配る zip の項目は固定名の ASCII という点は両方で成り立ちそうだが、
  PowerShell の古い `Compress-Archive` は区切りを `\` で書く既知の癖があり、D6 の「断られない」が自前の確かめ（`..`/根/`\0`）でも通るかは枝ごとに見ていない。
- 直し方の案: `archive_windows` の両枝の項目名を evidence に加え、`holds` の根拠にする（または枝ごとに `unknown` を置く）。
- 処置: fixed: `archive_windows` の 2 つの枝（`build.sh:240` の `zip -q -j` と `build.sh:250` の `Compress-Archive -LiteralPath`）を根拠に足した。どちらもファイルを平らに渡すので項目は基底名だけで、`\` の癖が出る下のフォルダを持たない
