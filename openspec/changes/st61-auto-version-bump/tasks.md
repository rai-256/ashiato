# ST61 実装タスク — main に merge するたびに版番号が自動で進む

読む順: `deep.md`（Q1〜Q4。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST52・ST54 は archive 済みで main にある。版の検査（`tools/release/version.sh`・`crates/build-stamp/src/lib.rs`・`collector-android/app/build.gradle.kts`・`web/build-stamp.ts`・`tools/check-version.sh`・`tools/check-build-stamp.sh`）・
配る組み立て（`tools/release/{build,package,check-artifacts,draft,publish-nightly,check-guards}.sh`）・workflow（`ci.yml`・`release.yml`・`nightly.yml`）・更新の試験（`tools/release/update-apply-install.sh`）がそこにある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。件数つきの検証は、試験を走らせた後に `-- --list` の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

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

- [x] 1.1 `tools/release/MAJOR`（中身 `0`）を足し、根の `Cargo.toml` の `[workspace.package] version` を `0.0.1` にする（`Cargo.lock` の workspace の crate の版も追従させる）。`tools/release/version.sh` に `major`（ファイルを読み、数字 1 つでなければ落ちる）を足し、
  `check <版番号>` を D9 の規則にする（番号を持たない版 = 3 つの数字が出所の版と同じ / 番号 = 左が `MAJOR` と同じで `<左>.<真ん中>` が `0.0` でない。後ろの形は今のまま。`0.0.0` は落ちる）。`code` を D3 の式にし、真ん中 999・右 9,999・左 209 を超えれば超えた部分を示して落ちる。
  `tools/release/version-table.tsv` を新しい式で書き直す（`0.0.1`→1・`0.2.0`→20000・`0.2.7`→20007・`0.250.9999`→2509999・`1.0.0`→10000000・`0.4.0-beta.2`→40000・`0.4.0-dev.20261101`→40000、拒む `0.1000.0`・`0.1.10000`・`210.0.0`・`0.4.0-beta.99`）。`version_test.sh` の値も直す。
  受ける・断る版番号の表 `tools/release/version-check-table.tsv`（`<版番号>\t<出所の版>\t<左>\tok|ng`。少なくとも `0.0.1`/`0.0.1`/`0` ok・`0.4.0-beta.2` ok・`0.0.1-beta.1` ok・`1.4.0` ng・`0.0.2`（出所 `0.0.1`）ng・`0.0.0` ng・`0.4.0-rc.1` ng）を作り、`check` に全行を当てる（D9。review/spec.md R17）。
  `--self-test` に印: `versionCode は版番号の 3 つの数字から決まる`・`後ろに -beta や -dev が付いても、同じ番号は同じ versionCode になる`・`最初の番号は、前の式の 0.1.x の APK より大きい`（20000 > 10199）、値を変えた `桁を超える版番号では、versionCode の計算が失敗する`
  検証: `tools/release/version.sh --self-test` rc=0 / `tools/release/version_test.sh` rc=0 / `bash -c 'test "$(tools/release/version.sh code 0.2.0)" = 20000 && test "$(tools/release/version.sh code 1.0.0)" = 10000000 && ! tools/release/version.sh code 0.1.10000 2>/dev/null'` rc=0
- [x] 1.2 `tools/check-version.sh` を D9 に合わせる（出所の版は `0.0.<1 以上>`、`tools/release/MAJOR` は数字 1 つ）。`--self-test` に印 `出所の版が 0.0.x でないと、版の検査が落ちる`（`0.1.0`・`0.0.0`）・`左の数字のファイルが数字 1 つでないと、版の検査が落ちる`（`0.1`・空）と、
  中身を変えた `出所の版が MAJOR.MINOR.PATCH の形でないと、版の検査が落ちる`（`0.0.1-beta.1`）。旧い印 `出所の版の MAJOR が 0 でないと、版の検査が落ちる` は消す（REMOVED の Requirement の Scenario。`1.0.0` は上の 0.0.x の Scenario が持つ）
  検証: `tools/check-version.sh --self-test` rc=0 / `tools/check-version.sh` rc=0

## Task 2: 版の規則を Rust・gradle・画面にそろえる

- [x] 2.1 `crates/build-stamp/src/lib.rs` の `version(source)` を D9 の規則にする（左の数字は `tools/release/MAJOR` から読む。読む道は `CARGO_MANIFEST_DIR` からの根。`cargo:rerun-if-changed` に足す）。誤りの文に、左の数字のファイルの値・出所の版・渡した版番号を含める。
  試験（`#[cfg(test)]`、名前は `st61_`）に `// Scenario: 渡した番号の左の数字が左の数字のファイルと違うと、組み立てが失敗する`（`1.4.0`）・`// Scenario: 出所の版と違う 0.0.x を渡すと、組み立てが失敗する`（出所 `0.0.1` に `0.0.2`。旧い印 `出所の版と違う版番号を渡すと…` を、`check-build-stamp.sh` のものも含めてこの名前に替える）と、値を足した `// Scenario: 決めた形でない版番号を渡すと、組み立てが失敗する`（`0.0.0` を足した 5 つ）。`version-check-table.tsv` の全行を当てる試験も足す
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-build-stamp st61_ && test "$(cargo test -p ashiato-build-stamp st61_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0
- [x] 2.2 `collector-android/app/build.gradle.kts`: 版の規則を D9 に、versionCode を D3 の式にする（`tools/release/MAJOR` を `providers.fileContents` で読む）。`verifyVersionCodeTable` は Task 1 の表をそのまま当てる。
  `tools/release/check-artifacts.sh --self-test-gradle-range` を、出所 `0.0.1` の写しの木に `ASHIATO_BUILD_VERSION=0.1000.0` を渡す形に直す（`// Scenario: 桁を超える版番号では、携帯の組み立てが失敗する`）。
  `組み立てた APK の versionCode は、渡した版番号から計算した値` の期待値を 40000 にする。`version-check-table.tsv` の全行を当てる task `verifyVersionCheckTable` を足し、`// Scenario: 渡した番号の左の数字が左の数字のファイルと違うと、組み立てが失敗する` の印を置く
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:verifyVersionCodeTable :app:verifyVersionCheckTable && test "$(grep -vc "^#" tools/release/version-table.tsv)" -ge 10'` rc=0 /
  `tools/release/check-artifacts.sh --self-test-gradle-range` rc=0 /
  `bash -c 'ASHIATO_BUILD_VERSION=0.4.0-beta.2 scripts/quiet-run apk -- ./collector-android/gradlew -p collector-android :app:assembleRelease && tools/release/check-artifacts.sh apk-version collector-android/app/build/outputs/apk/release/*.apk'` rc=0
- [x] 2.3 `web/build-stamp.ts`: 版の規則を D9 にする（`tools/release/MAJOR` を読む）。`web/src/__tests__/build-stamp.test.ts` に `version-check-table.tsv` の全行を当てる試験を足し、`// Scenario: 渡した番号の左の数字が左の数字のファイルと違うと、組み立てが失敗する` の印を置く
  検証: `bash -c 'cd web && npx vitest run src/__tests__/build-stamp.test.ts'` rc=0 / `bash -c 'cd web && npm run build'` rc=0
- [x] 2.4 `docs/openapi.json` を作り直す（`info.version` が `0.0.1`）。`tools/check-openapi.sh` はそのまま通る（`// Scenario: API の契約の版は、渡した版番号に依らず出所の版`）。
  `tools/check-build-stamp.sh` の `release` の節を、出所 `0.0.1`・`MAJOR` `0` の写しの木で `0.4.0-beta.2` を渡す形に直し、`版番号を渡さない組み立てでは、4 つの部品の版番号が出所の版と同じ` と ADDED の `版番号を渡さずに組んだ部品の版番号は 0.0.1`（この repo の木で渡さずに組む）の印を置く
  検証: `tools/check-openapi.sh` rc=0 / `bash -c 'scripts/quiet-run stamp -- tools/check-build-stamp.sh release'` rc=0 / `bash -c 'scripts/quiet-run stamp -- tools/check-build-stamp.sh android'` rc=0

## Task 3: 番号の判定（`version.sh number`）

- [x] 3.1 `tools/release/version.sh number <commit>` を足す（D4・D5・D7・D13・D14・D15）。出力は `number=<番号>`・`reuse=yes|no`（その commit に既に印があれば yes）か `number=`（付けない）と理由の行。
  浅い木は「履歴が浅い」で落ちる。commit が前の番号の commit の子孫で main の first-parent の上に無ければ付けない。差分の判定は `tools/release/changes.sh code` を呼ぶ（判定を写さない）。
  `--self-test` は使い捨ての git の木（`main`・`--no-ff` の merge・題 `Merge pull request #<n> from o/verify/<日付>`・印 `build-*`）で次の印を走らせる:
  `製品に触れた merge の commit の run が成功すると、次の番号が付く`・`文書だけの merge の commit には番号を付けない`・`道具だけの merge の commit にも番号が付く`・`取り消された merge の中身は、後の文書だけの merge の番号に入る`・
  `番号を付けた commit の祖先の run が後から成功しても、番号を付けない`・`同じ commit の run を走らせ直しても、番号は変わらない`・`欠番の commit を走らせ直すと、後の番号が付いていても、その番号を返す`・`番号の印が無い木では、文書だけの merge に番号を付けない`・`最初の番号は 0.2.0`・`間に確認バッチの merge が無ければ、右の数字が 1 上がる`・
  `間に確認バッチの merge があれば、真ん中が上がり右が 0 に戻る`・`確認バッチの merge 自身の番号は、真ん中が上がる`・`前の番号の commit が確認バッチの merge でも、次は右が上がる`・`左の数字のファイルを上げると、真ん中と右が 0 になる`・`左の数字のファイルを下げると、番号を付けずに止まる`・`左の数字のファイルが数字 1 つでなければ、番号を付けずに止まる`・`真ん中か左が上限を超える番号は付けない`・`携帯の versionCode に収まらない番号は付けない`・
  `組み立てが落ちた番号は、次の commit に回さない`・`浅い木では、番号付けの判定は通らない`
  検証: `tools/release/version.sh --self-test` rc=0 / `bash -c 'out=$(tools/release/version.sh --self-test 2>&1); n=0; while IFS= read -r s; do grep -qxF "== Scenario: $s" <<<"$out" && n=$((n+1)); done < <(sed -n "/^### Requirement: main の commit には/,/^### Requirement: 番号を付けた組み立て/s/^#### Scenario: //p" openspec/changes/st61-auto-version-bump/specs/distribution/spec.md | grep -v "main の commit は増えない\|成功した run だけが"); test "$n" -ge 20'` rc=0（2 つの Requirement の Scenario の名前の全文で数える）

## Task 4: 番号の印と下書きへの置き場（`tools/release/number.sh`）

- [x] 4.1 `tools/release/number.sh tag <番号> <commit>`（印 `build-<番号>` を作る。同じ commit を指していれば何もしない、別の commit なら落ちる）と `tools/release/number.sh place <番号> <commit> <置き場>`
  （`check-artifacts.sh <置き場> --allow-debug-key --expect-version <番号>` を通し、`gh release view build-<番号>` で今の配布物を読み、無ければ `gh release create build-<番号> --draft --title <番号> --target <commit> --notes "ashiato-build: <番号> <commit>"`、欠けた配布物だけを `gh release upload` で足す。上書き（`--clobber`）・公開・`v*` の操作をしない）を作る（D1（仮）・D2（仮）・D7）。
  `place` は下書きの説明文の `ashiato-build:` の commit が渡した commit と違えば何も書かずに落ち、置く前の `check-artifacts.sh` が落ちれば何も書かない（review/spec.md R3・R10）。
  `--self-test` は偽の `gh` と使い捨ての git の木で次の印を走らせる: `番号の印を作る手順は、印を作る要求だけを出す`・`番号を付けると、下書き 1 つに配布物を置く要求だけを出す`・`配布物の検査に落ちた置き場では、配布元へ何も書かない`・`下書きの説明文の commit が違えば、何も置かずに止まる`・`同じ番号の下書きに配布物が揃っていれば、置き直さない`・`下書きに欠けた配布物があれば、それだけを足す`・
  `番号の印が別の commit を指していれば、何も置かずに止まる`・`番号を付けても main の commit は増えない`（台本の前後で `main` の commit が同じ）
  検証: `tools/release/number.sh --self-test` rc=0
- [x] 4.2 `tools/release/build.sh apk` に、署名していない APK を `ashiato-collector-<版番号>-unsigned.apk` の名前で同じ置き場に足す口（環境変数 `ASHIATO_APK_ALSO_UNSIGNED=1`。番号付けの組み立てだけが立てる）を足し、`check-artifacts.sh` が番号の置き場で 5 つ（4 つ + 署名していない APK）を受けるようにする（D2（仮））。
  `check-artifacts.sh --self-test` に `番号を付けた配布物の印は、番号と commit を持つ`（番号 `0.2.1` を渡して組んだ 4 つの印が `0.2.1+<commit>` で `.dirty` を持たない）と `署名していない APK は、debug の鍵の APK と同じ組み立て`（2 つの APK の印と versionCode 20001 が同じで、署名していない方は署名を持たない）を足す
  検証: `bash -c 'scripts/quiet-run pkg -- tools/release/check-artifacts.sh --self-test'` rc=0
- [x] 4.3 `tools/check-released-migrations.sh` と `tools/check-contract-version.sh` の自己試験に、番号の印 `build-0.2.0`・`build-0.2.1` だけがある木で配った印が 0 個として通る `番号の印は、配った印に数えない` を足す（今の「配った印は `v[0-9]*`」の数え方のままで通ることを固定する）
  検証: `tools/check-released-migrations.sh --self-test` rc=0 / `tools/check-contract-version.sh --self-test` rc=0

## Task 5: 番号付けの workflow（`number.yml`）

- [x] 5.1 `.github/workflows/number.yml` を足す（D10・D12）: `on: workflow_run`（`workflows: [ci]`・`types: [completed]`・`branches: [main]`）、既定の `permissions: contents: read`。`concurrency: { group: number, cancel-in-progress: false }` は workflow でなく判定の job に置く（review/spec.md R1）。
  判定の job（条件: `github.event.workflow_run.event == 'push' && github.event.workflow_run.conclusion == 'success' && github.event.workflow_run.head_repository.full_name == github.repository`。`head_sha` を `fetch-depth: 0` で取り出し `version.sh number` → `number.sh tag`。この job は `contents: write`）、
  組み立ての 2 つの job（判定の job の後 `needs`。Linux: `build.sh server-linux` と `apk`（`ASHIATO_APK_ALSO_UNSIGNED=1`）、Windows: `build.sh server-windows` と `collector-windows`。`ASHIATO_RELEASE=1`・`ASHIATO_BUILD_VERSION=<番号>`。`head_sha` を取り出す。`actions/cache` で前の組み立てを引かず、保存もしない）、
  置く job（`number.sh place`。`contents: write`）。`reuse=yes` で下書きが揃っていれば組み立ての job を飛ばす
  `check-guards.sh workflows` に number.yml の検査を足し、`--self-test` に印 `main の push の成功した run だけが番号付けを起こす`・`番号付けの workflow は、前の組み立てを使い回さず、書く権限をその job にだけ持つ`・`番号付けは、CI と別の、取り消さない 1 本ずつの流れにある`・`番号付けは、終わった run の commit を全部の履歴で取り出す`・`番号付けを起こさない run は、待っている番号付けを取り消さない`（それぞれ、条件を 1 つ外した偽の number.yml で rc≠0。組を workflow の単位に置いた偽の number.yml も rc≠0）
  検証: `tools/release/check-guards.sh --self-test` rc=0 / `tools/release/check-guards.sh workflows` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/number.yml` rc=0

## Task 6: main への push の CI の差分の基準

- [x] 6.1 `tools/release/changes.sh base`（D8）: 環境変数で渡した「前に成功した main の push の run の commit」（`gh api` の `actions/workflows/ci.yml/runs?branch=main&event=push&status=success` の最初の `head_sha`。読み出しは workflow が渡す）を、木に無い・空なら `all` を返し、あればその commit を返す。
  `ci.yml` の `changes` の job で、push のときだけその commit を `BASE` にする（PR は今のまま）。`changes` の job に `permissions: { contents: read, actions: read }`。
  `changes.sh --self-test` に印 `取り消された run の後の文書だけの merge でも、コードの job が走る`（使い捨ての木で、基準からの差分が製品に触れて `code=true`）・`前に成功した main の run が見つからなければ、全部走らせる`（空・木に無い commit で `all`）
  検証: `tools/release/changes.sh --self-test` rc=0 / `tools/release/check-guards.sh workflows` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0

## Task 7: 更新の試験の組み立てを 0.0.x にする

- [x] 7.1 `ci.yml` の `package-windows` の更新の試験の組み立ての版を `0.0.2:old 0.0.3:new 0.0.2-dev.20261115:dev-old 0.0.3-dev.20261116:dev-new 0.0.4:compat-old 0.0.5:compat-new` にし（D6）、`tools/release/update-apply-install.sh` の版の値（前 `0.0.2`・新 `0.0.3`・互換 `0.0.4`→`0.0.5`・開発版、窓と記録の文の期待値）を同じに直す。
  `tools/release/install-check.sh`・`update-check-install.sh`・`soak-install.sh` に 0.4.0 などの版の値が無いことを確かめる。`check-guards.sh workflows` に、更新の試験の組み立ての版が `0.0.` で始まることを見る検査と、印 `CI の更新の試験の組み立ては、0.0.x だけを名乗る`（`0.4.0` を足した偽の ci.yml で rc≠0）、配る形の組み立ての job が番号を渡さないことを見る検査と印 `配る形の組み立ての job は、番号を版番号に渡さない`（`ASHIATO_BUILD_VERSION: 0.2.0` を足した偽の ci.yml で rc≠0。D6・review/spec.md R9）を足す。
  Scenario「DB に新しい版の知らない版があると、戻さずに前の版のまま起きる」は互換の組（`0.0.4` のインストールに `0.0.5` のインストーラ）で走らせる（spec の版に合わせる）。
  MODIFIED の 8 つの Requirement の Scenario（版を `0.0.x` にしたもの）は `update-apply-install.sh` の今の印のまま
  検証: `tools/release/check-guards.sh --self-test` rc=0 / `tools/release/check-guards.sh workflows` rc=0 / `bash -c '! grep -nE "\b0\.(4|5|6)\.[0-9]" tools/release/update-apply-install.sh tools/release/install-check.sh tools/release/update-check-install.sh tools/release/soak-install.sh'` rc=0 / `bash -c 'tools/release/update-apply-install.sh --list | grep -c . | xargs test 34 -le'` rc=0

## Task 8: 印 v* の判定を番号の commit に限る（移る間）

- [x] 8.1 `tools/release/version.sh plan <印>` を D11 にする: 印の commit に `build-<番号>` の印があり、印の 3 つの数字がその番号と同じときだけ通す。番号の無い commit は「印の commit に番号が無い」で落ちる。
  修正の枝 `release/*` の例外を消す（main の祖先でなければ落ちる）。`--self-test` の 4 つの修正の枝の Scenario（REMOVED）を消し、印 `印の版とその commit の番号が違えば公開しない`・`番号の無い commit の印は公開しない`・`修正の枝の印も、main の祖先でなければ公開しない` を足し、
  `浅い木では、印の判定は通らない`・`正式版の印は、通常のリリースと判定される`・`ベータ版の印で、先行公開のリリースになる`・`main の祖先でない commit の印は公開しない` の木に番号の印を足す。`docs/release.md` の修正の枝の節を消し、壊れた版は main で直して次の番号に印を付ける手順にする
  検証: `tools/release/version.sh --self-test` rc=0 / `tools/release/check-no-release-branch.sh` rc=0（修正の枝の分岐 `refs/remotes/origin/release/*` を消した）

  （訂正 2026-11-23: 検証の入口を「bash -c '! grep -nF "release/" tools/release/version.sh'」から上の入口に。受け入れ条件は変えない。理由: version.sh は tools/release/ のパスを 48 か所で正当に持つので grep -F release/ は原理的に通らない。消すのは修正の枝の分岐（release/<数字>・origin 上の release/）で、それだけを見る台本に置き換える 実証: evidence.jsonl 2026-11-23T03:37:24+00:00 FAIL。承認: machine: evidence.jsonl 2026-11-23T03:37:33+00:00 PASS。記録: plan-corrections.md）
## Task 9: 開発版の作り直しを最新の番号の commit から組む（移る間）

- [x] 9.1 `tools/release/version.sh nightly-plan` を D11 にする: 最新の番号の commit を選び `commit=<40 桁>` の行を足して出す。番号が無ければ `build=no` と「番号がまだ無い」。版番号は `<番号>-dev.<日本時間の YYYYMMDD>`。出所の版の正式版の判定を消す。
  `--self-test` の印: `nightly が最新の番号の commit を指していれば、開発版を作らない`・`nightly より新しい番号があれば、その番号の commit で開発版を作り直す`・`番号がまだ無ければ、開発版を組まない` を足し、
  `浅い木では、開発版の作り直しの判定は通らない`・`予定の実行が落ちた次の回に、同じ変更を拾う`・`印 nightly がまだ無ければ、作り直す`・`公開の設定が入っていなければ、予定の実行は組まずに終える`・`公開の設定が入っていなければ、手で起こした作り直しも公開しない` の木に番号の印を足す。旧い 3 つ（REMOVED の Requirement の名前）の印を消す
  検証: `tools/release/version.sh --self-test` rc=0
- [x] 9.2 `nightly.yml`: 組み立ての job と公開の job が `plan` の `commit` を `ref:` に取り出し、`publish-nightly.sh` が印 `nightly` をその commit へ動かす（`publish-nightly.sh` に commit を渡す口を足し、`--self-test` で偽の `gh` が受けた `nightly` の先がその commit であることを見る）。
  `check-guards.sh workflows` と `--self-test` に印 `開発版の作り直しの workflow は、判定が返した commit を組む`（`ref:` を外した偽の nightly.yml で rc≠0）を足す
  検証: `tools/release/publish-nightly.sh --self-test` rc=0 / `tools/release/check-guards.sh --self-test` rc=0 / `tools/release/check-guards.sh workflows` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/nightly.yml` rc=0

## Task 10: 確かめの下書きの版

- [x] 10.1 `tools/release/draft.sh` の自己試験の版を `0.0.1-beta.1` にし、下書きの印の名前が `build-` で始まれば落ちる守りを足す（D7）。`確かめの組み立ては、印を作らずに下書きのリリースになる` の AND（`build-` で始まらない）を見る
  検証: `tools/release/draft.sh --self-test` rc=0

## Task 11: 手順書と申し送り

- [ ] 11.1 `docs/release.md` に「番号」の節（番号の付き方 D4・D5・左の数字を本人が変える手順 D9・番号の組み立ての置き場と欠番 D1・D7・番号付けが止まったとき（左を下げた・上限に届いた D14）の直し方）を足す。
  `docs/production-prep.md` の「バージョンの単一情報源」の行に ★ で「番号は main の番号付けが付け、左の数字は `tools/release/MAJOR`。`Cargo.toml` は番号を持たない組み立ての `0.0.1`（ST61）」と書く。
  `docs/handoff/ST62.md`（`## st61-auto-version-bump R3`）と `docs/handoff/ST64.md`（`## st61-auto-version-bump R9`）は上流が書いた。実装で台本や印の名前が変わったら、その節を直す
  検証: `bash -c 'for h in "tools/release/MAJOR" "build-" "欠番" "左の数字"; do grep -q "$h" docs/release.md || exit 1; done && grep -q "ST61" docs/production-prep.md && grep -q "st61-auto-version-bump R3" docs/handoff/ST62.md && grep -q "st61-auto-version-bump R9" docs/handoff/ST64.md'` rc=0 / `python3 scripts/check_scenarios.py . st61-auto-version-bump` rc=0

## 人間の確認待ち（確認バッチ。PR が main に入った後）

下流の実装の中では配布元に書かないので、配布元での確かめはここで行う。Scenario ではない —— 前提の確かさを上げる確かめと、完了の判定の実地の確かめ。

- [ ] 12.1 **下書きは認証なしの一覧に出ない**（P5・D1（仮））: 使い捨ての下書き 1 つを作り、認証なしの `curl https://api.github.com/repos/rai-256/ashiato/releases?per_page=20` に出ないことを見て、消す。P5 の確かさを 4 にする。出たら D1 の反転条件で上流へ返す
- [ ] 12.2 **最初の番号**: この Story が main に入った後、最初に製品に触れる merge の CI が通ると、印 `build-0.2.0` と下書き `build-0.2.0` に 5 つの配布物が並び、main の commit が増えていないこと（完了の判定の 1 つ目）
- [ ] 12.3 **確認バッチの merge**: 次の確認バッチの merge の後の番号の真ん中が 1 上がり、右が 0 になること（D5）
- [ ] 12.4 **main の差分の基準**（D8・review/spec.md R16）: main に入った後の最初の文書だけの merge の run の `changes` の job の出力に、基準の commit（前に成功した main の push の run の `head_sha`）が出て、全部走らせるに落ちていないこと
- [ ] 12.5 **番号付けの順番待ち**（P18・P19・P20・D10）: main に入った後、取り消された CI の run の直後に成功した run があった回で、番号付けの判定の job が取り消されずに番号の印を作ったこと（印 `build-*` の push と、その印の下書きができたこと）。外れたら D10 の反転条件で上流へ返す
