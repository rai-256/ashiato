# CH24 機械が承認した検証コマンドの訂正の確かめ（code_verify の手 7）

`python3 scripts/plan_fix.py pending openspec/changes/ch24-zip-upgrade` の 2 件。どちらも新しい入口は `./openspec/changes/ch24-zip-upgrade/check-fixtures.sh`。
壊す前後で `git status --porcelain` と `git diff` はどちらも空で同じ（作業ツリーは HEAD dd6c62265 のまま）。

## 1.5 2026-11-20T14:57:42+00:00
- 壊したもの: 1.1 の埋め込みの囲みの中の期待値（書き出しの id）を、上げた後の作業ツリーで書き換えた。`crates/server/src/export_file_tests.rs:52` の `CH24_PREVIOUS_EXPORT_ID` を `…c10124` から `…c10125` にした（元に戻し済み）
- 実行: ./openspec/changes/ch24-zip-upgrade/check-fixtures.sh
- 結果: 落ちた

rc=1。出力は「上げる前の最後の版 1ec847af0 と比べる」の後に、`3c3` の diff と `囲みの中が変わった: crates/server/src/export_file_tests.rs`。

## 4.2 2026-11-20T16:14:45+00:00
- 壊したもの: 台本のループで最後に見るファイルの囲みの中の期待値を、上げた後の作業ツリーで書き換えた。`crates/collector-windows/src/update_apply.rs:435` の `CH24_EXTRACT_EXPECTED_UNIX` の 2 つ目（`C:\x`）を `true` から `false` にした（元に戻し済み）
- 実行: ./openspec/changes/ch24-zip-upgrade/check-fixtures.sh
- 結果: 落ちた

rc=1。3 つのファイルを順に見て、最後の `update_apply.rs` で `17c17` の diff と `囲みの中が変わった: crates/collector-windows/src/update_apply.rs` を出した。
補助の根拠として、4.2 の主張どおり「上げた後の commit での書き直し」も確かめた。使い捨ての clone（`/tmp`。終えて消した）で HEAD の上に
`crates/server/src/archive_tests.rs` の `CH24_CP437_NAME` を書き換えた commit を 1 つ積み、作業ツリーは clean にした。そのうえで同じ台本を走らせると rc=1（`囲みの中が変わった: crates/server/src/archive_tests.rs`）。
