# ST52 実装タスク — 3 つの部品を 1 つの版で、3 つのチャンネルに配る

読む順: `deep.md`（**本人の答え Q1〜Q11**。第 1 回は推奨で先に書いてある）→ このファイル → `specs/distribution/spec.md` → `specs/record-envelope/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST48・ST49・ST51 は archive 済みで main にある（下流の入口が main を取り込む）。版の記録（`crates/server/src/migration_ledger.rs`）・組み立ての印（`crates/build-stamp`・`web/build-stamp.ts`・`build.gradle.kts`）・
更新の履歴（`crates/server/src/update_history.rs`）・配った印の検査 2 本（`tools/check-released-migrations.sh`・`tools/check-contract-version.sh`）がそこにある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
件数つきの検証は、試験を走らせた後に `-- --list`（gradle は結果の XML）の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 携帯の versionCode は…`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 107 本**（`distribution` 84・`record-envelope` 23）。うち **ST52 が新しく印を置くのは 76 本**（`distribution` の ADDED 67 と、MODIFIED の
  `今の木の祖先でない印は、配った移行の検査で数えない`・`修正の枝の上では、枝の祖先の印を数える`、`record-envelope` の MODIFIED で足した 7 本）—— **全部このファイルのどこかの Task が名指しする**。
  残りの 31 本は MODIFIED で本文をそのまま写した Scenario で、印は ST48（版の記録の試験の 8 本と `check-released-migrations.sh --self-test` の 7 本）・ST49（`crates/build-stamp` と `check-build-stamp.sh` の 6 本）・ST50（`check-contract-version.sh --self-test` の 10 本）が持つ。**消さない**。
  そのうち `版の記録の 1 行は、名前・指紋・時刻・組み立ての版番号と commit を持つ` は THEN を変えた（Task 2.2 で試験の比べる値を直す）
- **本人の決定（下流は変えない）**: Q1 本人の携帯には収集アプリがまだ入っておらず記録も無い（入れ直しも署名の引き継ぎも要らない。最初に入れるのは ST56 の後の正式な鍵の版）/ Q2 正式な鍵は印 `v*` と main にだけ開いた Environment で CI が署名 / Q3 versionCode は版番号から計算 /
  Q4 ST54 まで、開発版に携帯の APK を入れない（開発版はサーバ 2 つと PC の収集アプリの 3 つ）/ Q5 ST52 は下書きのリリースで確かめ、本物の印と開発版の公開は ST53 の後 / Q6 出所は `Cargo.toml` の workspace の `version`、ベータは印から・開発版は日付から渡す /
  Q7 サーバは Windows と Linux の x86_64 / Q8 画面はサーバの実行ファイルに組み込む / Q9 コードを触った PR ごとに 3 部品を組む / Q10 開発版の印は `nightly`（`v*` の外）/ Q11 開発版は正式な鍵と別の鍵（控えは USB に 1 部。作って置くのは ST54）。
  未決（第 2 回の答え待ち。別の席が仮を否とした）: D1（Q12）・D22（Q13）。
  仮: D2（`v*` は消さない・動かさない）・D3（鍵は RSA 4096・25 年以上）・D4（本人のサーバは印の直後に移す）・D5（PC の後戻りは ST58）・D6（修正の枝は移行と封筒を変えない）・D21（配布用の Environment は印 `v*` にだけ開く）・D23（開発版の携帯の APK の道具は ST52 が作って使い捨ての鍵で確かめ、開発版のリリースに載せるのは ST54）。
  （第 1 回は推奨で先に書き、推奨と違った Q1・Q4 の答えを上流が写してから下流へ渡す）
- **名前は統一する**: 環境変数 `ASHIATO_BUILD_VERSION`（渡す版番号）・`ASHIATO_VERSION`（`build.rs` が出す、組み立ての版番号）・`ASHIATO_WEB_DIST`（取り込む画面の置き場）・`ASHIATO_RELEASE=1`（配る組み立て。PR の `package` も立てる）・`ASHIATO_APK_UNSIGNED=1`（携帯の APK を署名せずに組む。`release.yml` の組み立ての job だけ。`nightly.yml` は ST54 まで携帯の APK を組まない）・`ASHIATO_EXPECT_CERT_SHA256`（署名の指紋の期待値。Environment の変数 `ASHIATO_CERT_SHA256` から）、
  道具は `tools/release/{version,build,package,check-artifacts,sign-apk,make-key,draft,check-guards}.sh`・`tools/release/version-table.tsv` と `tools/check-version.sh`、workflow は `release.yml`・`nightly.yml`（と `ci.yml` の `package` の job）、
  Environment は `release`・`nightly`、リポジトリの変数は `ASHIATO_NIGHTLY_PUBLISH`、配布物の名前は design D17
- **workflow の YAML に判定の論理を書かない**（D8）。判定は `tools/release/*.sh` に置き、`--self-test` が使い捨ての git の木・偽の `gh`（`PATH` の先に置く台本）で確かめる
- **下流の実装の中では配布元に何も書かない**: 本物の印 `v*` を打たない・`nightly` を push しない・Environment とルールセットと下書きのリリースを作らない・本物の鍵を作らない（Q5・本人の手）。下流の検証は `--self-test`（使い捨ての git の木・偽の `gh`）で閉じる。
  配布元での確かめ（使い捨ての Environment・下書きのリリース・ルールセットの読み）は、PR の CI が緑になった後の確認バッチで行う（末尾の「人間の確認待ち」）
- **資格情報と鍵の合言葉を、ログ・標準出力・試験の期待値の出力に出さない**（C1。値は項目の名前だけで示す）
- **`vite preview` の配り方と既存の e2e を変えない**（壊してはいけないもの）。`tools/stack.sh` は画面を先に組んでサーバにも取り込ませるだけ（D20）
- **新しい Rust の依存の crate を足さない**（D13）。Android の依存も足さない
- **長いコマンドは `scripts/quiet-run`**（cargo test・gradle・`tools/smoke.sh`）。試験の名前の接頭辞は `st52_`。DB の試験は `crates/server/src/st52_web_tests.rs`（`lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;`）、本物の bin の試験は `crates/server/tests/server_startup.rs` の `st52_bin_` で始まる関数
- 縦串の節は Story ごとの新しいファイル（`tools/smoke.d/`）に置き、共有の台本の本体に書き足さない

## Task 1: 版番号の計算の台本（`tools/release/version.sh`）

- [ ] 1.1 `tools/release/version.sh` を作る（D11・D8・D9）。口は `source`・`check <版番号>`・`code <版番号>`・`order`・`plan <印>`・`nightly-plan`・`--self-test`。
  `plan` は `channel=`・`version=`・`version_code=`・`prerelease=` の行を出し、形・出所との一致・main の祖先（D6 の修正の枝の例外を含む。Task 8 で中身を足す。ここでは「祖先でなければ落とす」まで）を見る。
  `nightly-plan` は `build=yes|no`・`publish=yes|no`（環境変数 `ASHIATO_NIGHTLY_PUBLISH` が `on` のときだけ yes。`on` でなく予定の実行なら `build=no`。D15）・`version=` を出し、日付は `TZ=Asia/Tokyo`（試験は `ASHIATO_NOW`（UTC の ISO 8601）で時計を渡す）。
  `--self-test` は使い捨ての git の木で次の Scenario を走らせる:
  - `携帯の versionCode は…` の要件: `versionCode は版番号から決まり、ベータ < 正式 < 次の版の順になる`・`同じ版の開発版は、毎日同じ versionCode になる`・`桁を超える版番号では、versionCode の計算が失敗する`（表 `tools/release/version-table.tsv` の全行を `code` に当てる）
  - チャンネル: `正式版の印は、通常のリリースと判定される`・`ベータ版の印で、先行公開のリリースになる`・`印の版と出所の版が違えば公開しない`・
    `main の祖先でない commit の印は公開しない`・`決めた形でない印は、どのチャンネルにも配らない`
  - 開発版: `nightly と main の HEAD が同じ日は、開発版を作らない`・`main が進んだ日は、開発版を作り直す`・`予定の実行が落ちた次の回に、同じ変更を拾う`・`印 nightly がまだ無ければ、作り直す`・`公開の設定が入っていなければ、予定の実行は組まずに終える`・`公開の設定が入っていなければ、手で起こした作り直しも公開しない`・`出所の版の正式版を出した後は、出所の版を上げるまで開発版を組まない`（`nightly-plan` は起きた種類を `GITHUB_EVENT_NAME` で受ける。D15）
  検証: `tools/release/version.sh --self-test` rc=0 / `bash -c 'test "$(tools/release/version.sh code 0.3.0-beta.2)" = 30002 && test "$(tools/release/version.sh code 1.0.0)" = 1000099'` rc=0
- [ ] 1.2 `CLAUDE.md` の「構成」の `tools` の行に `release/`（配る道具）を足す
  検証: `bash -c 'grep -q "release/" CLAUDE.md'` rc=0

## Task 2: 版の出所を 1 つにする（Rust の 2 部品と版の記録）

- [ ] 2.1 `crates/build-stamp` に `version(source)`（D12）を足す。`ASHIATO_BUILD_VERSION` が空でなければ形（`<出所の版>`・`-beta.<1〜98>`・`-dev.<8 桁>`）と出所との一致を見て返し、違えば `Err`（出所の版と渡した版番号の両方を含む文）。
  `rerun_lines` に `cargo:rerun-if-env-changed=ASHIATO_BUILD_VERSION` を足す。試験（`crates/build-stamp/src/lib.rs` の `#[cfg(test)]`、名前は `st52_`）:
  `// Scenario: 版番号を渡した組み立ての印は、渡した版番号で始まる`・`// Scenario: 出所の版と違う版番号を渡すと、組み立てが失敗する`（関数の段）・`// Scenario: 決めた形でない版番号を渡すと、組み立てが失敗する`（関数の段。4 つの値を全部）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-build-stamp st52_ && test "$(cargo test -p ashiato-build-stamp st52_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0
- [ ] 2.2 `crates/server/build.rs` と `crates/collector-windows/build.rs` を `version(env!("CARGO_PKG_VERSION"))` の結果で印を組むように替え、`Err` なら `panic!`、`cargo:rustc-env=ASHIATO_VERSION=<版番号>` を出す。
  `migration_ledger::BUILD_VERSION` を `env!("ASHIATO_VERSION")` にする。`db_ahead` の `self:` の行と更新の履歴（ST51）は印と `BUILD_VERSION` を読むので、そのまま同じ版番号になる。
  `crates/server/tests/server_startup.rs` に、`ASHIATO_BUILD_VERSION=<出所>-beta.2` で組んだ bin を使う 3 本:
  `st52_bin_ledger_build_version`（`// Scenario: 渡した版番号で組んだサーバの版の記録は、渡した版番号を持つ`）・`st52_bin_db_ahead_self`（`// Scenario: 渡した版番号で組んだサーバが知らない版で止まると、self の行は渡した版番号を持つ`）・
  `st52_bin_update_history_version`（`// Scenario: 渡した版番号で組んだサーバの更新の履歴の今の版は、渡した版番号で始まる`）。
  ST48 の `migration_ledger_tests.rs` の `CARGO_PKG_VERSION` と比べている所（`版の記録の 1 行は、名前・指紋・時刻・組み立ての版番号と commit を持つ`）を `env!("ASHIATO_VERSION")` と比べるように直す。
  `crates/server/src/bin/openapi.rs` の試験（または `tools/check-openapi.sh` の節）に `// Scenario: API の契約の版は、渡した版番号に依らず出所の版`（`ASHIATO_BUILD_VERSION=<出所>-beta.2` で生成した契約の `info.version` が出所の版）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server --test server_startup st52_bin_ && test "$(cargo test -p ashiato-server --test server_startup st52_bin_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0 / `./tools/check-openapi.sh` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server migration_ledger'` rc=0（ST48 の既存の試験が通る）

## Task 3: 版の出所を 1 つにする（携帯と画面）と版の検査

- [ ] 3.1 `collector-android/app/build.gradle.kts`: `versionName` と `versionCode` の文字の値を消し、根の `Cargo.toml` の `[workspace.package] version` を `providers.fileContents` で読み、`ASHIATO_BUILD_VERSION`（`providers.environmentVariable`）を D12 と同じ規則で受け、versionCode を D11 の式で計算する（範囲外は `GradleException`）。
  開発版（`-dev.`）だけ `applicationIdSuffix = ".dev"`。`ASHIATO_APK_UNSIGNED=1` の release は `signingConfig` を当てない（D18）、無ければ今どおり debug 鍵（`ASHIATO_RELEASE=1` は署名に効かない）。組み立ての印（ST49 の `COMPONENT_VERSION`）の版番号もこの値にする。
  式は `build.gradle.kts` の中の関数に置く（D11）。2 つの式（`version.sh code` と gradle）が同じ値を出すことは、
  版番号と versionCode の表 `tools/release/version-table.tsv` を両方が読んで固定する: `version.sh --self-test` が表の全行を `code` に当て、gradle の試験の task `verifyVersionCodeTable`（`build.gradle.kts` に足す。表の全行を gradle の式に当て、違えば落ちる）が同じ表を当てる
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:verifyVersionCodeTable && test "$(grep -vc "^#" tools/release/version-table.tsv)" -ge 8'` rc=0 /
  `bash -c 'ASHIATO_BUILD_VERSION="$(tools/release/version.sh source)-beta.2" scripts/quiet-run apk -- ./collector-android/gradlew -p collector-android :app:assembleRelease && tools/release/check-artifacts.sh apk-version collector-android/app/build/outputs/apk/release/*.apk'` rc=0（`// Scenario: 組み立てた APK の versionCode は、渡した版番号から計算した値`。`check-artifacts.sh` の `apk-version` は Task 6 で作る。先に Task 6.2 の `apk-version` の口だけを作ってよい）/
  `tools/release/check-artifacts.sh --self-test-gradle-range` rc=0（`// Scenario: 桁を超える版番号では、携帯の組み立てが失敗する`。使い捨ての写しの木で出所を `0.100.0` にして `:app:assembleDebug` が rc≠0）
- [ ] 3.2 `web/build-stamp.ts` と `web/vite.config.ts`: 版番号を `package.json` でなく根の `Cargo.toml` から読み、`ASHIATO_BUILD_VERSION` を同じ規則で受ける（形が違えば `vite build` が落ちる）。`web/package.json` の `version` を消す。
  画面の印の「ソースの道」に根の `Cargo.toml` を足す（`:/web` と `:/Cargo.toml`）。`web/src/__tests__/build-stamp.test.ts` に `// Scenario: 携帯と画面の印の版番号も、根の出所の版から読む`（画面の側。偽の `Cargo.toml` の中身を渡す）と
  `// Scenario: 根の Cargo.toml を書き換えて commit していなければ、携帯と画面の印は .dirty を持つ`（画面の側。偽の git が `Cargo.toml` の未 commit の変更を返す）
  検証: `bash -c 'cd web && npx vitest run src/__tests__/build-stamp.test.ts'` rc=0 / `bash -c 'cd web && npm run build'` rc=0
- [ ] 3.3 `tools/check-version.sh`（`--self-test` つき。D12）: 手書きの版番号（`build.gradle.kts` の文字の `versionName = "`・`versionCode = <数>`、`web/package.json` の `version`）・出所の版の形（`0.<数>.<数>`）を見る。
  `--self-test` は使い捨ての木で `携帯の組み立ての設定に版番号を手で書くと、版の検査が落ちる`・`画面の package.json に版番号を書くと、版の検査が落ちる`・`出所の版の MAJOR が 0 でないと、版の検査が落ちる`・`出所の版が MAJOR.MINOR.PATCH の形でないと、版の検査が落ちる` を走らせる。
  `ci.yml` の `chain` の job に `./tools/check-version.sh --self-test` と `./tools/check-version.sh` を足す
  検証: `tools/check-version.sh --self-test` rc=0 / `tools/check-version.sh` rc=0
- [ ] 3.4 `tools/check-build-stamp.sh` を出所の変更に合わせる（C-01 の版番号を `build.gradle.kts` の `versionName` でなく `Cargo.toml` と比べる）。節 `release` を足し、使い捨ての写しの木で `ASHIATO_BUILD_VERSION=<出所>-beta.2` を渡して 4 つの部品を release で組み、
  印がどれも `<出所>-beta.2+<HEAD>` で `.dirty`・`.debug` が無く、組み立ての後の `git status --porcelain` が空なことを見る（`// Scenario: 渡した版番号は、4 つの部品の印に同じ値で入り、印は .dirty を持たない`）。
  同じ節で、渡さずに組んだ 4 つの印の版番号が出所の版と同じこと（`版番号を渡さない組み立てでは、4 つの部品の版番号が出所の版と同じ`）、出所と違う版番号でサーバの `cargo build` が落ちること（`出所の版と違う版番号を渡すと、組み立てが失敗する`）、
  `決めた形でない版番号を渡すと、組み立てが失敗する`（サーバの組み立てで 4 つの値）、`携帯と画面の印の版番号も、根の出所の版から読む`（携帯の側。出所を `0.9.0` にして commit した写しの木）・`根の Cargo.toml を書き換えて commit していなければ、携帯と画面の印は .dirty を持つ`（携帯の側。gradle の印のソースの道に根の `Cargo.toml` を足す）を見る。`ci.yml` の `build-stamp` の job に `./tools/check-build-stamp.sh release` を足し、`changes` の `stamp` の式に `web/build-stamp\.ts|web/vite\.config\.ts|tools/release/` を足す
  検証: `bash -c 'scripts/quiet-run stamp -- tools/check-build-stamp.sh release'` rc=0 / `bash -c 'scripts/quiet-run stamp -- tools/check-build-stamp.sh android'` rc=0（ST49 の節が出所の変更の後も通る）

## Task 4: 配った印の検査が祖先の印だけを数える（ST48・ST50 の台本。D1（Q12 の答え待ち））

- [ ] 4.1 `tools/check-released-migrations.sh`: 印の列挙を `git tag --merged HEAD --list 'v[0-9]*'` に替える。`--self-test` に `// Scenario` の echo つきで
  `今の木の祖先でない印は、配った移行の検査で数えない`・`修正の枝の上では、枝の祖先の印を数える`・`開発版の印は、配った印に数えない`（distribution。`nightly` だけの木で通る。送信契約の側は 4.2）を足す。既存の 7 本はそのまま
  検証: `tools/check-released-migrations.sh --self-test` rc=0 / `tools/check-released-migrations.sh` rc=0
- [ ] 4.2 `tools/check-contract-version.sh`: 同じく `--merged HEAD` に替え、「最後の印」を `tools/release/version.sh order` の最大で選ぶ（`--sort=-creatordate` を消す）。封筒の例の凍結（全部の印との比べ）も `--merged HEAD` の集合にする。
  `--self-test` に `最後の配った印は、打った日時でなく祖先の中で版番号の最も大きい印`・`最後の配った印は、ベータ版の番号を数として比べて選ぶ`・`同じ版のベータ版と正式版では、正式版を最後の配った印に選ぶ`・`開発版の印 nightly は、最後の配った印に数えない`・`開発版の印は、配った印に数えない`（送信契約の側）を足す。既存の 10 本はそのまま
  検証: `tools/check-contract-version.sh --self-test` rc=0 / `tools/check-contract-version.sh` rc=0
- [ ] 4.3 `docs/handoff/ST52.md` の 3 項目の下に「ST52 で処置した」を 1 行ずつ書く（型 `v[0-9]*` は変えず、数える印を祖先に絞った・版番号の出所を `Cargo.toml` から読むように替えた）
  検証: `bash -c 'test "$(grep -c "ST52 で処置した" docs/handoff/ST52.md)" -ge 3'` rc=0

## Task 5: サーバが組み込んだ画面を配る

- [ ] 5.1 `crates/server/build.rs` に画面の取り込み（D13）: `ASHIATO_WEB_DIST` の中のファイルの表を `OUT_DIR/web_assets.rs` に書く（空なら空の表。`rerun-if-env-changed` と、置き場があればその中のファイル）。`ASHIATO_RELEASE=1` で空なら `panic!`。
  新しい `crates/server/src/web_assets.rs` と `router()` の変更（`/api` を剥がす層・`fallback` に画面）。`crates/server/src/st52_web_tests.rs`（`router()` に偽の表を渡す口で試す。組み立ての表を差し替える口は試験の構成だけ）:
  - `st52_web_headers` — `// Scenario: 画面のファイルには、今と同じ守りの見出しが付く`
  - `st52_web_api_prefix` — `// Scenario: /api の道は、今の API と同じ応答と認可になる`
  - `st52_web_access_log` — `// Scenario: /api の道で読んだ画面の読み出しも、今と同じ読み出しの記録が残る`
  - `st52_web_collector_paths` — `// Scenario: 収集アプリの送り先の道は、今のまま受ける`
  - `st52_web_without_assets` — `// Scenario: 画面を組み込まない組み立てでは、/ は 404 で API は動く`
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st52_web_ && test "$(cargo test -p ashiato-server st52_web_ -- --list 2>/dev/null | grep -c ": test$")" -ge 5'` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server access_log'` rc=0（読み出しの記録が `/api` の道でも今と同じに当たる既存の試験）/ `./tools/check-openapi.sh` rc=0
- [ ] 5.2 `crates/server/tests/server_startup.rs` に `st52_bin_release_needs_web`（`// Scenario: 配る組み立てで画面が無ければ、サーバの組み立てが失敗する`。使い捨ての `CARGO_TARGET_DIR` で `ASHIATO_RELEASE=1`・`ASHIATO_WEB_DIST` 無しの `cargo build -p ashiato-server` が rc≠0）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server --test server_startup st52_bin_release && test "$(cargo test -p ashiato-server --test server_startup st52_bin_release -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0
- [ ] 5.3 `tools/stack.sh`（D20）: 画面の `vite build` をサーバの組み立ての前に移し、`ASHIATO_WEB_DIST` を渡してサーバを組む。`vite preview` は今の `WEB_PORT` のまま。
  `web/e2e/st52-served-ui.spec.ts`（`// Scenario: 配ったサーバだけで、Node を入れずに画面が開く`）: 環境変数 `BIND` から絶対 URL `http://$BIND/` を組んで開き（`baseURL` の `WEB_PORT` を使わない）、合言葉でログインし、`/api/coverage` の応答を待って稼働状況の格子が見える（`toBeVisible`）。
  開いた間の要求の URL がどれも `$BIND` の番号であることも見る
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && STACK_RESET=1 npm run test:e2e -- st52-served-ui.spec.ts'` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && STACK_RESET=1 npm run test:e2e -- login.spec.ts stack.spec.ts'` rc=0（既存の e2e が `vite preview` のまま通る）

## Task 6: 配る組み立ての台本と配布物の検査

- [ ] 6.1 `tools/release/build.sh <server-linux|server-windows|collector-windows|apk> <出力の置き場>`（D14・D17）: 画面を `vite build` してサーバに取り込む・release で組む・D17 の名前で包む。`ASHIATO_BUILD_VERSION` があれば渡す。
  Windows の 2 つは Windows の runner（と WSL からの Windows 側の cargo）で走る形にする（`bash` は Git Bash）。`--self-test` は Linux で `server-linux` と `apk` を組み、名前と中身を見る（Windows の 2 つの本物の通しは Task 9.2 の `package` の job が CI で見る）
  検証: `bash -c 'scripts/quiet-run pkg -- tools/release/build.sh --self-test'` rc=0
- [ ] 6.2 `tools/release/check-artifacts.sh <置き場> [--channel <release|beta|nightly>] [--allow-debug-key] [--expect-version <版番号>]`（D7・D23（仮））と口 `apk-version <apk>`: チャンネルの配布物がそろう（既定と `release`・`beta` は 4 つ、`nightly` は携帯の APK を除く 3 つで、APK があれば落とす）・印の版番号が 1 つ（と期待の値）・`.dirty` / `.debug` が無い・APK の `BASE_URL`・`API_TOKEN`・`USER_ID` が空・（`--allow-debug-key` でなければ）署名の指紋が `ASHIATO_EXPECT_CERT_SHA256` と同じ（`nightly` は APK が無いので指紋と `BuildConfig` を見ず、期待値は空でよい。D23（仮））。
  `--self-test` は、Linux で本物に組んだ 2 つ（Task 6.1 の `server-linux` と `apk`）と、Windows の 2 つの写し（Linux で組んだ C-02 の rlib から読んだ組み立ての印の文字列を、D17 の名前の zip に詰めたもの —— 検査は印の文字列と名前しか読まないので形は同じ）でそろえた置き場と、資格情報 `secret-token-for-test` を `-Pashiato.apiToken` で焼き込んだ APK・debug の APK・未 commit の変更で組んだサーバ）に当てる:
  `配布物が 1 つでも欠ければ公開しない`・`.dirty の印を持つ配布物があれば公開しない`・`.debug の印を持つ配布物があれば公開しない`・`資格情報を焼き込んだ APK があれば公開せず、値を示さない`（出力に `secret-token-for-test` が無いことも見る）・`綺麗な release で資格情報の無い配布物は、検査を通る`・
  `1 つのリリースの配布物は、決めた名前の 4 つで、同じ版番号を持つ`・`開発版の配布物の検査は、携帯の APK を除く 3 つで通る`（`server-linux` と Windows の 2 つの写しを `0.4.0-dev.20261102` で用意し、`ASHIATO_EXPECT_CERT_SHA256` を空にして `--channel nightly` が rc=0）・`開発版の置き場に携帯の APK があれば、検査は落ちてその名前を示す`（`ashiato-collector-dev-0.4.0-dev.20261102.apk` と `ashiato-collector-0.4.0-dev.20261102.apk` をそれぞれ足して rc≠0 と名前）
  検証: `bash -c 'scripts/quiet-run pkg -- tools/release/check-artifacts.sh --self-test'` rc=0

## Task 7: 携帯の APK の署名と鍵を作る台本

- [ ] 7.1 `tools/release/make-key.sh <official|dev> <出力の置き場>`（D3（仮））と `make-key.sh verify <keystore>`: 合言葉は標準入力から。`--self-test` は
  `鍵を作る台本は、決めた種類・長さ・有効期限の鍵だけを作る`・`鍵を作る台本は、決めた長さに満たない鍵を確かめで落とす`（`keytool` で 2048 bit の鍵を作って `verify` に当てる）。
  Play App Signing に持ち込める鍵の条件の一次情報（公式文書の URL と引用）を `premises.md` に P17 として足す（確かさ 2。条件が D3 より厳しければ実装を止めて上流へ返す）
  検証: `bash -c 'scripts/quiet-run key -- tools/release/make-key.sh --self-test'` rc=0 / `bash -c 'grep -q "^| P17 " openspec/changes/st52-release-channels/premises.md'` rc=0

- [ ] 7.2 `tools/release/sign-apk.sh <apk> <official|dev> <出力>`（D18）: 環境変数の鍵（base64）を `$TMPDIR` に戻して `apksigner sign` を当て、終わりに消す。鍵が無ければ rc≠0。
  開発版は識別子が `dev.ashiato.collector.dev` の APK にだけ、正式版はそうでない APK にだけ当てる（取り違えは rc≠0）。`--self-test` は使い捨ての鍵 2 つ（`make-key.sh` で作る）で:
  `正式版の APK は、正式な識別子と正式な鍵で署名されている`・`識別子とチャンネルの鍵を取り違えると、署名しない`・`開発版の APK は、.dev の識別子と開発版の鍵で署名されている`・`鍵が置き場に無ければ、署名しない APK を公開しない`・`PR の組み立ての APK は debug の鍵で署名される`（PR の形の `build.sh apk` の APK の証明書が `~/.android/debug.keystore` のもの）
  検証: `bash -c 'scripts/quiet-run sign -- tools/release/sign-apk.sh --self-test'` rc=0

## Task 8: 緊急の修正の枝の判定（D6（仮））

- [ ] 8.1 `tools/release/version.sh plan` に修正の枝の例外（D6）を足す: 印の commit が main の祖先でなく、`release/<MAJOR>.<MINOR>` の枝の上にあり、正式版の印で、枝の元の正式版の印と `migrations/ORDER`・`docs/contract/envelope.json` が同じときだけ通す。
  `--self-test` に `修正の枝の正式版の印は、移行と封筒の形が元の印と同じなら公開できる`・`修正の枝で移行を足した印は公開しない`・`修正の枝で封筒の形を変えた印は公開しない`・`修正の枝からベータ版は出さない` を足す
  検証: `tools/release/version.sh --self-test` rc=0

## Task 9: 守りの確かめの台本と workflow（PR ごとの組み立て・印のリリース・開発版）

- [ ] 9.1 `tools/release/check-guards.sh <rules|environments|workflows> [--repo <owner/name>]`（D2（仮）・D21（仮）・Q2）: `rules` は `gh api repos/<repo>/rules/tags/<印>` を読み、`deletion` と `update` が当たっていなければ rc≠0。
  `environments` は `release` の配る対象が印 `v*` だけ、`nightly` が枝 `main` だけかを見る。`workflows` は `.github/workflows/` を読み、`changes` の出し分けの式・`nightly.yml` の cron と `publish` の `if:`・書く権限の置き場（`publish` の job だけ）を見る。
  `--self-test` は偽の `gh` と偽の変わったファイルの一覧で `印の規則が入っていなければ、公開しない`・`印の規則が入っていれば、公開の前の確かめを通る`・`鍵の置き場は、印と main のそれぞれにだけ開いている`（開いているとき・開きすぎているときの両方）を走らせる
  （`workflows` の Scenario は 9.2・9.3 で workflow を書いてから足す）
  検証: `tools/release/check-guards.sh --self-test` rc=0
- [ ] 9.2 `ci.yml` に `package` の job（D14）: `changes` の `code` が true のときだけ。手順は `tools/release/package.sh <linux|windows> <置き場>`（`build.sh` を呼び、`ASHIATO_RELEASE=1`・`ASHIATO_BUILD_VERSION` を渡さない）に置き、Linux の job で `server-linux` と `apk`、Windows の job で `server-windows` と `collector-windows` を組み、
  最後の 1 つの job で 4 つを集めて `check-artifacts.sh --allow-debug-key --expect-version "$(tools/release/version.sh source)"` を通して `actions/upload-artifact` で置く。`permissions: contents: read` のまま。
  `check-guards.sh --self-test` に `コードを触った PR では、配る形の組み立てが走る`・`文書だけの PR では、配る形の組み立てを飛ばす` を、`package.sh --self-test` に `配る形を壊した PR は、その PR の CI で落ちる`（`build.sh` の 1 つを落とす偽の木で rc≠0）を足す
  検証: `tools/release/check-guards.sh --self-test` rc=0 / `tools/release/package.sh --self-test` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0 / `tools/release/check-guards.sh workflows` rc=0
- [ ] 9.3 `release.yml`・`nightly.yml`（D8・D9・D10・D14・D15・D18）。`nightly.yml` の `schedule` は `cron: "0 15 * * *"`。組み立ての job は `contents: read`（`release.yml` の携帯の APK は `ASHIATO_APK_UNSIGNED=1`。`nightly.yml` は ST54 まで携帯の APK を組まない。Q4・D23（仮））、`publish` だけが `contents: write` と `environment:`、`nightly` の `publish` は `if: vars.ASHIATO_NIGHTLY_PUBLISH == 'on'` も持つ。
  `release.yml` の `publish` は `check-guards.sh rules` → `sign-apk.sh` → `check-artifacts.sh`（`ASHIATO_EXPECT_CERT_SHA256` は Environment の変数 `ASHIATO_CERT_SHA256`）→ `gh release create`。`nightly.yml` の `publish` は `check-artifacts.sh --channel nightly` → D15 の入れ替え（署名しない）。
  `check-guards.sh workflows` は `nightly.yml` に携帯の APK の組み立て（`build.sh apk`）と `sign-apk.sh` の呼び出しが無いことも見る。`check-guards.sh --self-test` に `開発版の作り直しの workflow は、ST54 まで携帯の APK を組む手順を持たない`（APK の job を足した偽の `nightly.yml` で rc≠0）・`開発版の作り直しは、日本時間の 0:00 に起きる`・`公開の設定が無ければ、印 nightly を動かす手順は走らない` を足す
  検証: `tools/release/check-guards.sh --self-test` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/release.yml` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/nightly.yml` rc=0 / `tools/release/check-guards.sh workflows` rc=0

## Task 10: 下書きのリリースを作る台本（印を打たない）

- [ ] 10.1 `tools/release/draft.sh <run-id> <版番号>`（D16）: `gh run download` → `check-artifacts.sh --allow-debug-key` → `gh release create --draft [--prerelease] --target main`。`--self-test` は偽の `gh` で
  `確かめの組み立ては、印を作らずに下書きのリリースになる`・`確かめの組み立ても、配布物が欠ければ下書きを作らない` を走らせる。本物の配布元では走らせない（末尾の「人間の確認待ち」で走らせる）
  検証: `tools/release/draft.sh --self-test` rc=0

## Task 11: 手順書と申し送り

- [ ] 11.1 `docs/release.md`（D19）: 鍵を作る（`make-key.sh`）・控え（正式な鍵は扉 34 の USB と拠点外の 2 か所と紙の合言葉、開発版の鍵は正式な鍵と別で USB に 1 部 —— Q11。作って `nightly` に置くのは ST54 で、ここには手順だけ）・Environment `release`（印 `v*` だけ。D21（仮））と Secrets と指紋の変数 `ASHIATO_CERT_SHA256` / `nightly`（枝 `main`。ST52 では鍵を置かない）・ルールセット・
  最初の印を main の commit に打つ・開発版の公開を始める（`ASHIATO_NIGHTLY_PUBLISH`）・壊れた版は次の版で直す・修正の枝の切り方と消し方・本人の環境を移す（D4）・
  正式版を出した後に出所の版を次の版に上げる（Q6 の推奨。開発版が出したばかりの正式版より古く見えないように）・確認バッチの確かめの手順（下の「人間の確認待ち」のコマンド）。`docs/production-prep.md` の「バージョンの単一情報源」の行に ★ で「出所は `Cargo.toml` の workspace の `version`。ほかは読む（ST52）」と書く。
  `docs/handoff/ST53.md`（最初の印と開発版の公開（`ASHIATO_NIGHTLY_PUBLISH`）は ST53 の後に本人が始める・`docs/release.md` の手順・ST52 の完了の判定の残り（D22（Q13 の答え待ち）））・`docs/handoff/ST54.md`（開発版の DB の扱いの節を足す。review/deep.md R11。開発版の携帯の APK と鍵の節（Q4・Q11・D23（仮））は上流が書いた。実装が道具の名前を変えたら直す）・`docs/handoff/ST58.md`（PC の後戻り。R14・D5（仮））に 1 節ずつ足す
  検証: `bash -c 'for h in "鍵を作る" "USB に 1 部" "ルールセット" "最初の印" "ASHIATO_NIGHTLY_PUBLISH" "ASHIATO_CERT_SHA256" "修正の枝" "本人の環境を移す"; do grep -q "$h" docs/release.md || exit 1; done'` rc=0 / `bash -c 'grep -q "ASHIATO_NIGHTLY_PUBLISH" docs/handoff/ST53.md && grep -q "R11" docs/handoff/ST54.md && grep -q "D5" docs/handoff/ST58.md'` rc=0 / `python3 scripts/check_scenarios.py . st52-release-channels` rc=0

## 人間の確認待ち（確認バッチ。PR の CI が緑になった後）

下流の実装の中には PR の CI の実行が無いので、配布元での確かめはここで行う（review/spec.md R16）。Scenario ではない —— どれも前提（premises）の確かさを上げる確かめで、結果は確認バッチが `premises.md` に書く。

- [ ] 12.1 **下書きのリリース**（D16・P16）: この Story の PR の `package` の run で `tools/release/draft.sh <run-id> 0.1.0-beta.1` を走らせ、作る前と後で `git ls-remote --tags origin 'v*' | wc -l` が同じこと、下書きに 4 つの配布物が並び先行公開の印が付いていることを見る。見た後に下書きを消す。
  印が増えたら、その印を消さずに止めて上流へ返す（ルールセットはまだ無い）。P16 の確かさを 4 にする
- [ ] 12.2 **Environment の守り**（P12・D21（仮））: 使い捨ての Environment `st52-probe`（配る対象: 枝 `main` だけ）に使い捨ての Secret を置き、PR の枝から読もうとする workflow を 1 回だけ走らせて読めないことを見て、Environment を消す。P12 の確かさを 4 にする。読めたら鍵を置かず、Q2 を人間に聞き直す
- [ ] 12.3 **ルールセットの読み**（D2（仮））: `tools/release/check-guards.sh rules --repo rai-256/ashiato` を、Actions の `GITHUB_TOKEN` 相当の読みの権限で走らせ、規則が無いことを読めるか（rc≠0 で「規則が無い」と出るか、権限で読めないか）を見る。読めなければ D2 の反転条件で上流へ返す。結果を P18 として書く
- [ ] 12.4 **手順書**: `docs/release.md` を読み、鍵を作る・Environment とルールセットを入れる手順が、本人の手元でそのまま進められること（入れるのは ST53 の後の最初の印の前。ST52 の確認では入れなくてよい）
