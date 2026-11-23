# ST61 実装タスク — main に merge するたびに版番号が自動で進む

読む順: `deep.md`（Q1〜Q4。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST52・ST54 は archive 済みで main にある。版の検査（`tools/release/version.sh`・`crates/build-stamp/src/lib.rs`・`collector-android/app/build.gradle.kts`・`web/build-stamp.ts`・`tools/check-version.sh`・`tools/check-build-stamp.sh`）・
配る組み立て（`tools/release/{build,package,check-artifacts,draft,publish-nightly,check-guards}.sh`）・workflow（`ci.yml`・`release.yml`・`nightly.yml`）・更新の試験（`tools/release/update-apply-install.sh`）がそこにある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。件数つきの検証は、試験を走らせた後に `-- --list` の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

> **この plan は `openspec/changes/st61-auto-version-bump/tasks.md` の Task 1 だけの切り出し**（story グラフの `task_slice.py` が作る写し）。
> 正本は `openspec/changes/st61-auto-version-bump/tasks.md`。`[x]` はそちらに付ける。ほかの Task はこの plan には無い —— 別の node が回す。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント、bash は `echo "== Scenario: …"`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と一字一句合わせる
- **この change の Scenario は 128 本**（すべて `distribution`）。新しい名前の Scenario は下の Task が全部名指しする。残りは今の台本・試験が印を持つ。そのうち中身（GIVEN の版・期待する値）を変えた Scenario は、印の場所の試験の値を直す（Task が名指しする）。印を消さない
- **本人の決定（第 1 回は推奨で書いた。答えが違えば上流が写し直してから下流へ渡す）**: Q1 番号は CI が通り前の番号から製品が変わった commit に付ける（D4（仮））/ Q2 versionCode は `左×10,000,000 + 真ん中×10,000 + 右`（D3（仮））/ Q3 間に確認バッチの merge があれば真ん中を上げる（D5）/ Q4 番号を持たない組み立ては 0.0.x（D6）。
  仮: D1（番号の組み立ては全部、下書きのリリースに残す）・D2（署名していない APK も残す）。D3・D4・D6 も（仮）で、第 1 回の答えが推奨と違えば `spec --answers` の段が design を直す
- **名前は統一する**: 左の数字のファイル `tools/release/MAJOR`・番号の印 `build-<番号>`・下書きの説明文の行 `ashiato-build: <番号> <commit の 40 桁>`・署名していない APK `ashiato-collector-<番号>-unsigned.apk`・番号付けの workflow `.github/workflows/number.yml`・
  台本の口 `tools/release/version.sh number <commit>`（番号の判定）・`tools/release/number.sh`（印を作り下書きに置く）。環境変数は今の `ASHIATO_BUILD_VERSION`・`ASHIATO_APK_UNSIGNED`・`ASHIATO_RELEASE`
- **workflow の YAML に判定の論理を書かない**（ST52 D8 の型）。判定は `tools/release/*.sh` に置き、`--self-test` が使い捨ての git の木・偽の `gh`（`PATH` の先に置く台本）で確かめる。workflow の形は `tools/release/check-guards.sh workflows` が読む
- **下流の実装の中では配布元に何も書かない**: 印 `build-*`・`v*`・`nightly` を push しない・下書きを作らない。配布元での確かめは末尾の「人間の確認待ち」で行う
- **版の検査の 4 か所（bash・Rust・gradle・TypeScript）は同じ規則**（D9）。versionCode の表（`tools/release/version-table.tsv`）と、受ける・断る版番号の表（`tools/release/version-check-table.tsv`）を 4 か所の試験が読む
- **main の HEAD を組まない**（番号付けは run の commit、開発版は最新の番号の commit を組む。D10・D11）
- **新しい Rust・Android・npm の依存を足さない**
- **長いコマンドは `scripts/quiet-run`**（cargo test・gradle）。Rust の試験の名前の接頭辞は `st61_`

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
