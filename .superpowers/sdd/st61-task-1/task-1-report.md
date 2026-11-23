# Task 1 report
実装: tools/release/MAJOR(0)・Cargo.toml/Cargo.lock の workspace 7 crate を 0.0.1・version.sh に major / check_rule(D9) / code(D3) ・version-table.tsv 書き直し・version-check-table.tsv 新設・version_test.sh・check-version.sh（0.0.x・MAJOR 検査と新 Scenario 印）。
plan() は今の印の配り方を保つため、3 つの数字が出所と同じ印だけ許す検査を足した（後の Task が番号の印へ作り直す）。
検証: verify-run 1.1・1.2 PASS（evidence.jsonl 参照）。RED は先に取っていない（前の中断した試行に commit が無く、実装と試験を同時に書いた）。

## Fix round 1
- F1: tools/check-version.sh の self_test で MAJOR を木の準備へ移し、各 Scenario の前に健全な木が通ることを確かめる（携帯・package.json の 2 つが MAJOR 欠けで偽に通っていた）。
- F2: tools/release/version.sh の check_version を `major=$(major_file) || exit 1` に直し、MAJOR が `x`・空の写しの木で `check` が落ちる試験を self_test に足した。
- 実行: `tools/check-version.sh --self-test` rc=0、`tools/release/version.sh --self-test` rc=0、`tools/check-version.sh` rc=0。
- RED は取っていない（試験を先に書いた証跡は無い）。F2 は修正前に rc=0 で受けた実測が findings にある。
- 表の変更に伴う gradle の verifyVersionCodeTable は Task 2.2 以降で確かめる。

## Fix report（round 2）
- F1: `tools/check-version.sh` の self_test は、MAJOR を木の準備（:34）で作り、各 Scenario の前に健全な木が通ることを確かめる（:40, :47, :53）。108252b69 で直し済み。
- F2: `tools/release/version.sh` の `check_version` は `major=$(major_file) || exit 1` で代入を分けた。self-test に MAJOR が `x` / 空の木で `check` が落ちる試験を追加（108252b69）。
- RED: 旧い形（`"$(major_file)"` を引数に直書き）に戻して `tools/release/version.sh --self-test` → rc=1（落ちる）。直した形に戻すと rc=0。
- GREEN: `scripts/verify-run 1.1` / `1.2` を現在の木で走らせ直し、全 PASS（evidence.jsonl に記録）。
- 未解決の ⚠️ 2 件は Task の中で確かめない（表の gradle/Rust/TS 側は Task 2、RED の取り方は上の通り今回示した）。

## Fix report（round 3）
- F1・F2 は 108252b69（round 2）で直し済み。現在の木で再確認: `tools/check-version.sh --self-test`・`tools/release/version.sh --self-test`・`tools/check-version.sh` が全て rc=0。コードの変更は無い。
- 木に残っていた未 commit は `.superpowers/sdd/` の記録のみ（コードなし）。まとめて commit した。
- ⚠️ 2 件（gradle/Rust/TS の表の確認・RED の証跡）は前回と同じ Ruling: Task の中では確かめない（Task 2 / final review）。
