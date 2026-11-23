## Task 1: 版番号の規則と versionCode の式（bash の正本と対応表）

- [ ] 1.1 `tools/release/MAJOR`（中身 `0`）を足し、根の `Cargo.toml` の `[workspace.package] version` を `0.0.1` にする（`Cargo.lock` の workspace の crate の版も追従させる）。`tools/release/version.sh` に `major`（ファイルを読み、数字 1 つでなければ落ちる）を足し、
  `check <版番号>` を D9 の規則にする（番号を持たない版 = 3 つの数字が出所の版と同じ / 番号 = 左が `MAJOR` と同じで `<左>.<真ん中>` が `0.0` でない。後ろの形は今のまま。`0.0.0` は落ちる）。`code` を D3 の式にし、真ん中 999・右 9,999・左 209 を超えれば超えた部分を示して落ちる。
  `tools/release/version-table.tsv` を新しい式で書き直す（`0.0.1`→1・`0.2.0`→20000・`0.2.7`→20007・`0.250.9999`→2509999・`1.0.0`→10000000・`0.4.0-beta.2`→40000・`0.4.0-dev.20261101`→40000、拒む `0.1000.0`・`0.1.10000`・`210.0.0`・`0.4.0-beta.99`）。`version_test.sh` の値も直す。
  受ける・断る版番号の表 `tools/release/version-check-table.tsv`（`<版番号>\t<出所の版>\t<左>\tok|ng`。少なくとも `0.0.1`/`0.0.1`/`0` ok・`0.4.0-beta.2` ok・`0.0.1-beta.1` ok・`1.4.0` ng・`0.0.2`（出所 `0.0.1`）ng・`0.0.0` ng・`0.4.0-rc.1` ng）を作り、`check` に全行を当てる（D9。review/spec.md R17）。
  `--self-test` に印: `versionCode は版番号の 3 つの数字から決まる`・`後ろに -beta や -dev が付いても、同じ番号は同じ versionCode になる`・`最初の番号は、前の式の 0.1.x の APK より大きい`（20000 > 10199）、値を変えた `桁を超える版番号では、versionCode の計算が失敗する`
  検証: `tools/release/version.sh --self-test` rc=0 / `tools/release/version_test.sh` rc=0 / `bash -c 'test "$(tools/release/version.sh code 0.2.0)" = 20000 && test "$(tools/release/version.sh code 1.0.0)" = 10000000 && ! tools/release/version.sh code 0.1.10000 2>/dev/null'` rc=0
- [ ] 1.2 `tools/check-version.sh` を D9 に合わせる（出所の版は `0.0.<1 以上>`、`tools/release/MAJOR` は数字 1 つ）。`--self-test` に印 `出所の版が 0.0.x でないと、版の検査が落ちる`（`0.1.0`・`0.0.0`）・`左の数字のファイルが数字 1 つでないと、版の検査が落ちる`（`0.1`・空）と、
  中身を変えた `出所の版が MAJOR.MINOR.PATCH の形でないと、版の検査が落ちる`（`0.0.1-beta.1`）。旧い印 `出所の版の MAJOR が 0 でないと、版の検査が落ちる` は消す（REMOVED の Requirement の Scenario。`1.0.0` は上の 0.0.x の Scenario が持つ）
  検証: `tools/check-version.sh --self-test` rc=0 / `tools/check-version.sh` rc=0
