# ST54 実装タスク — 開発版を正式版と並べて入れられる

読む順: `deep.md`（**本人の答え Q1〜Q10 と、第 2 回の Q11〜Q14**）→ このファイル → `specs/distribution/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST52 は archive 済みで main にある。配る道具（`tools/release/*.sh`・`.github/workflows/nightly.yml`）・組み立ての印と版番号（`crates/build-stamp`・`build.gradle.kts` の `resolvedBuildVersion`）・
版の記録（`crates/server/src/migration_ledger.rs`）・契約の問い合わせ（`crates/server/src/contract_version.rs`・`crates/collector-windows/src/contract_gate.rs`・`ContractGate.kt`）がそこにある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
件数つきの検証は、試験を走らせた後に `-- --list`（gradle は結果の XML）の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 開発版の migrate は、正式版の DB に 1 つも当てずに止まる`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 106 本**（MODIFIED の 2 つの Requirement に 13 本・名前を変えて移した 2 つの Requirement に 23 本・新しい 10 の Requirement に 70 本）。うち **ST54 が新しく印を置くのは 77 本**（署名・下書きと、名前を変えて移した 2 つで足した 7 本と、新しい 10 の Requirement の 70 本）—— **全部このファイルのどこかの Task が名指しする**。
  残りの 29 本は ST52 の本文をそのまま写した Scenario で、印は ST52 の試験（`version.sh`・`check-artifacts.sh`・`sign-apk.sh`・`make-key.sh`・`check-guards.sh`・`draft.sh` の `--self-test`）が持つ。**消さない**
- **ST52 の印のうち 3 つは消す**（REMOVED にした Requirement の Scenario で、移した先に無い。振る舞いが反対になる）: `開発版の配布物の検査は、携帯の APK を除く 3 つで通る`・`開発版の置き場に携帯の APK があれば、検査は落ちてその名前を示す`（`check-artifacts.sh --self-test`）・
  `開発版の作り直しの workflow は、ST54 まで携帯の APK を組む手順を持たない`（`check-guards.sh --self-test`）。試験そのものを新しい Scenario の向きに書き直す（Task 10）。古い印を残すと、`check_scenarios.py` が正典に無い印として拾う
- **本人の決定（下流は変えない）**: Q1 系統は組み立てのときの版番号で決める / Q2 PC の開発版は置き場の下に `dev`・端末識別子に `-dev`・除外は渡された置き場から読む（前景の窓もブラウザの履歴も。IMP-01）/ Q3 DB の系統の印があり、違う側は何も書かずに止まる /
  Q4 開発版の DB がずれたら止まって原因を示し、DB は書き換えず main 側で直す / Q5 印 `nightly` は動かす方式のまま（手順書に `git fetch --tags --force`）/ Q6 開発版の収集アプリは開発版のサーバにだけ送る /
  Q7 名前に「開発版」を付ける（アイコンは同じ）/ Q8 説明文の先頭に書き、公開のたびに書き直す / Q9 下書きのリリースと使い捨ての鍵で確かめる / Q10 開発版も全部の収集を動かす /
  Q11 開発版に載った移行の検査は動く印 `nightly` 1 つを基準に比べる（Q5 に乗る。D5）/ Q12 どちらの向きも系統の違うサーバへは何も送らず手元に残す（D7）/ Q13 送らずにいる間は「送り先が開発版／正式版のサーバです。送り先を確かめてください」と知らせる（D7）/
  Q14 開発版のサーバと PC の収集アプリの既定は D1 の系統で選び、値はこのまま（D8）/ Q15 系統の違うサーバからも、読む口（携帯と PC の `GET /outages`・携帯の `GET /feelings/reminder`）は今どおり引く（D13）。
  仮: D12（系統の違いの知らせの口と間隔。ST50 と同じ口・24 時間ごとの出し直し・別の札）・D14（系統は契約の版を読めた答えからだけ読み、開発版とそれ以外の 2 つ）。
- **名前は統一する**: 系統は `Channel::Dev` / `Channel::Official`（Rust）・`BuildConfig.CHANNEL`（`"dev"` / `"official"`）・`GET /contract` の応答の見出し `Ashiato-Channel`（`"dev"` / `"official"`。本文の欄にしない）。終了コード 4・`error: kind=channel_mismatch`。系統の違いの知らせの札は `channel`。
  開発版の名前: 自動起動 `ashiato-collector-dev.cmd`・差出人 `dev.ashiato.collector.dev`・表示名 `ashiato 開発版`・実行ファイル `ashiato-collector-windows-dev.exe`・アプリの名前 `あしあと。開発版`・待ち受け `127.0.0.1:18788`・クッキー `ashiato_session_dev`・退避 `backups/pre-migrate-dev`・DB のポート 55433。
  ファイル: `migrations/LINEAGE`・`tools/check-nightly-migrations.sh`・`tools/release/publish-nightly.sh`・`tools/release/nightly-notes.md`・`tools/android-side-by-side.sh`
- **workflow の YAML に判定の論理を書かない**（ST52 D8）。公開の手順は `tools/release/publish-nightly.sh` に置き、`--self-test` が偽の `gh`（`PATH` の先に置く台本）と使い捨ての鍵で確かめる
- **下流の実装の中では配布元に何も書かない**: 本物の開発版の鍵を作らない・`nightly` の Environment に何も置かない・印 `nightly` を push しない・`ASHIATO_NIGHTLY_PUBLISH` を入れない・下書きのリリースを作らない（Q9。末尾の「人間の確認待ち」）
- **資格情報と鍵の合言葉を、ログ・標準出力・試験の期待値の出力に出さない**
- **正式版の名前と既定を変えない**（壊してはいけないもの: 正式版の携帯の収集アプリの端末内の記録と未送信・正式版の PC の収集アプリの自動起動と通知）。系統が `Official` のときの値が今と同じことを、各 Task の試験が一緒に確かめる
- **新しい Rust の依存の crate・Android の依存を足さない**
- **長いコマンドは `scripts/quiet-run`**（cargo test・gradle・エミュレータ）。試験の名前の接頭辞は `st54_`。DB の試験は `crates/server/src/st54_tests.rs`（`lib.rs` の他の `*_tests` と同じ形で `#[cfg(test)] mod …;`）、本物の bin の試験は `crates/server/tests/server_startup.rs` の `st54_bin_` で始まる関数（開発版の bin は ST52 の `st52_bin_` と同じ組み方で `ASHIATO_BUILD_VERSION=<出所>-dev.20261102` を渡して組む）
- 縦串・書き換え禁止・偽データのシナリオを足すなら Story ごとの新しいファイル（`tools/smoke.d/` など）に置き、共有の台本の本体に書き足さない

## Task 1: 系統の判定（`crates/build-stamp`）

- [x] 1.1 `crates/build-stamp` に `pub enum Channel { Dev, Official }` と `pub fn channel(version: &str) -> Channel`（D1。版番号に `-dev.` を含めば `Dev`）と `Channel::as_str()`（`"dev"` / `"official"`）を足す。
  試験（`#[cfg(test)]`、名前は `st54_`）: `// Scenario: 版番号に -dev. を持つ組み立ては開発版になる`（`0.4.0-dev.20261102`・`0.4.0`・`0.4.0-beta.2`）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-build-stamp st54_ && test "$(cargo test -p ashiato-build-stamp st54_ -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0

## Task 2: サーバの系統（契約の問い合わせ・既定の待ち受け・クッキー・退避の置き場）

- [x] 2.1 `App` に `channel: Channel`（起動時に `channel(env!("ASHIATO_VERSION"))`。試験だけが差し替える）を足す。`GET /contract` の応答に見出し `Ashiato-Channel`（`"dev"` / `"official"`）を足し（本文は変えない）、`docs/openapi.json` を作り直す（D7）。
  `net_guard::default_bind(channel)`（開発版は `127.0.0.1:18788`）・`web_session::cookie_name(channel)`（開発版は `ashiato_session_dev`。発行と読み取りの両方）・`migrate_backup` の既定の置き場（開発版は `backups/pre-migrate-dev`）を系統で決める。`BIND`・`MIGRATE_BACKUP_DIR` を渡せばその値（D8。D1 の系統で選ぶ）。
  試験（`crates/server/src/st54_tests.rs`）: `// Scenario: 開発版のサーバは、契約の問い合わせに系統 dev を答える`・`// Scenario: 正式版のサーバは、契約の問い合わせに系統 official を答える`・
  `// Scenario: 開発版のサーバの既定は、正式版と別の待ち受け・クッキー・退避の置き場`・`// Scenario: 正式版のサーバの既定は、今のまま`・`// Scenario: 開発版のサーバの画面のログインは、開発版の名前のクッキーだけを使う`・
  `// Scenario: 待ち受けと退避の置き場を明示で渡せば、開発版でもその値を使う`・`// Scenario: 系統の名前の環境変数を渡しても、系統は変わらない`（サーバの側。`ASHIATO_CHANNEL=official` を渡しても `Ashiato-Channel` が `dev`）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st54_ && test "$(cargo test -p ashiato-server st54_ -- --list 2>/dev/null | grep -c ": test$")" -ge 7'` rc=0 / `./tools/check-openapi.sh` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st50_'` rc=0（ST50 の契約の問い合わせの試験を書き換えずに通る。本文は変えない）

## Task 3: DB の系統の守り（終了コード 4）

- [x] 3.1 `migration_ledger` に DB の系統の判定を足す（D3）: 版の記録の最初の行（`applied_at` の順、同じなら名前の順）の組み立ての版番号から系統を決める。版の記録が無いか行が 0 で ashiato の表を持つ DB は正式版、表が無い DB は系統なし。
  判定の言葉はコードの分かれ目に合わせる（版の記録の schema が無く `core` がある `BeforeLedger` は正式版、`Empty` と行が 0 の `Recorded` は系統なし）。記録を読んだ直後・`Verdict` を出す前に自分の系統と比べ、違えば `MigrateError::ChannelMismatch { db, me }`（退避・`BOOTSTRAP`・付与・更新の履歴より前）。`record_result` の更新の履歴を書かない早い返り（今は `Ahead | Unreadable`）に `ChannelMismatch` を足す。`migrate` とサーバの起動は終了コード 4、標準エラーは `error: kind=channel_mismatch` と `  db: <系統>`・`  self: <系統>`。
  `migrate` とサーバの起動の検査は系統を引数で受ける（試験が両方の系統を 1 つの試験の bin で走らせる）。
  試験（`st54_tests.rs`。関数の段）: `// Scenario: 開発版の migrate は、正式版の DB に 1 つも当てずに止まる`・`// Scenario: 正式版の migrate とサーバは、開発版の DB では止まる`・`// Scenario: 空の DB に開発版の migrate を当てると、開発版の DB になる`・
  `// Scenario: 版の記録の無い古い DB には、開発版の migrate は当てずに止まる`・`// Scenario: 版の記録の行が 1 つも無い DB には、どちらの系統の migrate も当てる`・`// Scenario: 系統が同じなら、今どおり当てて起動する`。
  THEN の「サーバの `/healthz` が 200」は、起動の検査（`startup_refusal` に当たる関数）が止めないことを見た後に、`contract_version_tests.rs` の `tee_app` の形で router を組んで `/healthz` を引いて見る（待ち受けは開かない）。
  本物の bin（`server_startup.rs`。開発版で組んだ bin）: `st54_bin_dev_server_on_official_db`（`// Scenario: 開発版のサーバは、正式版の DB では起動しない`。終了コード 4・待ち受けを開かない）・
  `st54_bin_channel_before_ahead`（`// Scenario: 系統が違い、知らない版もある DB では、系統の食い違いを示す`。終了コード 4 で 3 でない）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st54_ && test "$(cargo test -p ashiato-server st54_ -- --list 2>/dev/null | grep -c ": test$")" -ge 13'` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server --test server_startup st54_bin_ && test "$(cargo test -p ashiato-server --test server_startup st54_bin_ -- --list 2>/dev/null | grep -c ": test$")" -ge 2'` rc=0 /
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server migration_ledger'` rc=0（ST48 の既存の試験が通る）/
  `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server --test server_startup st48_'` rc=0
- [x] 3.2 更新の失敗からの戻し（ST58。FR-119）への申し送りを書き足す（D3。IMP-02）: `docs/handoff/ST58.md` の末尾に節 `## st54-dev-side-by-side D3 — 終了コード 4 も「移行の失敗」ではない（この回の退避は無い）` を足し、
  「`migrate` は DB の系統が自分と違うと、退避も版の適用も付与も更新の履歴の行も書かずに終了コード 4（標準エラーの 1 行目 `error: kind=channel_mismatch`）で止まる。FR-119 の戻しは終了コード 1 の回だけにし、終了コード 4 の回も 3 の回と同じく**退避から戻さず**、前の版のまま起動し直す（`rc≠0` を一律に移行の失敗と扱うと、前の回の退避を戻してそれ以後の記録を失う）」と、出所（この change の `design.md` D3・`go.json` の IMP-02）を書く。今ある節は変えない（書き足すだけ）
  検証: `bash -c 'f=docs/handoff/ST58.md; grep -q "^## st54-dev-side-by-side D3" "$f" && grep -q "終了コード 4" "$f" && grep -q "channel_mismatch" "$f" && grep -q "^## st48-db-version-guard R21" "$f"'` rc=0

## Task 4: 旧名の対応 `migrations/LINEAGE`（D4）

- [x] 4.1 `migrations/LINEAGE` を作る（説明の `#` 行だけ。形は spec の「移行の名前を変えた・消したときは…」）。`crates/server/build.rs` が `ORDER` と同じように組み込む（`rerun-if-changed` に足す）。
  `migration_ledger::compare` の前に、DB の名前の集合を LINEAGE で写す（`renamed` は今の名前へ、`removed` は外す）。版の記録の行は書き換えない。`renamed` の旧名の行の指紋は今の名前の中身と比べる。
  試験（`st54_tests.rs`。LINEAGE は試験が差し替えた表を渡す）: `// Scenario: 名前を変えた移行は、旧名の対応があれば、旧名で当てた DB でも起動し、当て直さない`・`// Scenario: 消した移行は、消したことの対応があれば、それを当てた DB でも起動する`・
  `// Scenario: 旧名の対応が無ければ、今どおり知らない版で止まり、その名前と当てた組み立てを示す`。`/healthz` は Task 3.1 と同じ見方（起動の検査を通した後に router で引く）
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st54_lineage && test "$(cargo test -p ashiato-server st54_lineage -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0
- [x] 4.2 `tools/check-migrations.sh` に LINEAGE の形の検査を足す（種類は 3 つ・名前の数・`renamed` の今の名前と `rewritten` の 2 つの名前が `ORDER` にある・`rewritten` の直しの移行が `ORDER` で直す移行より後）。`--self-test` に
  `echo "== Scenario: 旧名の対応の形が違えば、移行の形の検査が落ちる"`（`moved A B`・`renamed A`・`renamed A <ORDER に無い名前>`・`rewritten C D` で `D` が前、の 4 つで rc≠0 とその行の表示）を足す
  検証: `tools/check-migrations.sh --self-test` rc=0 / `tools/check-migrations.sh` rc=0

## Task 5: 開発版に載った移行の検査（D5。Q11）

- [x] 5.1 `tools/check-nightly-migrations.sh`（`--self-test` つき。形は `tools/check-released-migrations.sh` に合わせる）: 印 `nightly` が無ければ rc=0。あれば `git show nightly:migrations/ORDER` と各版の前進側の `.sql` を今の木と比べ、
  消した・名前を変えた版に `removed` / `renamed` が無い、中身を変えた版（`renamed` の版は今の名前の中身と `nightly` の旧名の中身を比べる）に `rewritten` が無いとき rc≠0 で版の名前（中身のときは「直しの移行を足して rewritten を書く」）を示す。`nightly` の版の `ORDER` を取り出せなければ rc≠0。浅い木では印を見る前に rc≠0（「履歴が浅い」。ST52 D1 と同じ止め）。
  `--self-test` は使い捨ての git の木で: `開発版に載った移行の名前を変えて旧名の対応が無ければ、検査が落ちる`・`開発版に載った移行の名前を変えても、旧名の対応があれば通る`・`名前を変えて中身も変えた移行は、旧名の対応だけでは落ちる`・`浅い木では、開発版に載った移行の検査は通らない`・`開発版に載った移行の中身を変えて直しの対応が無ければ、検査が落ちる`・
  `開発版に載った移行の中身を変えても、直しの移行の対応があれば通る`・`印 nightly が無ければ、開発版に載った移行の検査は通る`・`印 nightly の後に足した移行は、変えても通る`・`開発版に載った移行の戻し手順は、変えても通る`・`印 nightly の版の移行を取り出せなければ、検査は通らない`
  検証: `tools/check-nightly-migrations.sh --self-test` rc=0 / `tools/check-nightly-migrations.sh` rc=0
- [x] 5.2 `.github/workflows/ci.yml` の `rust` の job（完全な履歴）で、`check-released-migrations.sh` の隣に `tools/check-nightly-migrations.sh` を走らせる。その前に `git ls-remote --tags origin nightly` があるときだけ `git fetch --force --no-tags origin '+refs/tags/nightly:refs/tags/nightly'`（`--depth` を付けない）
  検証: `bash -c 'grep -q "check-nightly-migrations.sh" .github/workflows/ci.yml && grep -q "refs/tags/nightly:refs/tags/nightly" .github/workflows/ci.yml'` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0

## Task 6: PC の収集アプリの開発版の名前と置き場（D1・D2・D8）

- [x] 6.1 `Runtime` と `main.rs` が `channel(env!("ASHIATO_VERSION"))` を持つ（試験は差し替える）。`autostart::entry_name(channel)`（開発版は `ashiato-collector-dev.cmd`）・`toast::aumid(channel)`（開発版は `dev.ashiato.collector.dev`）・`toast::display_name(channel)`（開発版は `ashiato 開発版`）にし、正式版は今の値。
  `Config::from_lookup` に系統を渡し、開発版は `state_dir = <渡された置き場>/dev`・端末識別子の末尾に `-dev`（すでに `-dev` で終われば付けない）・`exclusions_path = <渡された置き場>/exclusions.json`。除外を読む口は 2 つとも `exclusions_path` を読む（D2）: `main.rs` の前景の窓の除外の読み込みと、ブラウザの履歴の取り込み（`HistoryCollector::new` が除外の登録の置き場を引数で受け、`Runtime::with_history` が設定の `exclusions_path` を渡す。今の `history/collect.rs:315` の `state_dir.join("exclusions.json")` をやめる）。自動起動の `.cmd` には渡された値（掘る前・付ける前）を書く。
  試験（`config.rs`・`autostart.rs`・`toast.rs`・`main.rs` の `#[cfg(test)]`、名前は `st54_`）: `// Scenario: 版番号を渡さずに組んだ部品は正式版として振る舞う`・`// Scenario: 開発版の PC の収集アプリは、開発版の自動起動の項目と差出人を使う`・
  `// Scenario: 開発版は、渡された置き場の下の dev を置き場にする`・`// Scenario: 開発版の自動起動の項目は、渡された置き場と端末識別子をそのまま書く`・`// Scenario: すでに -dev で終わる端末識別子には、重ねて付けない`・
  `// Scenario: 系統の名前の環境変数を渡しても、系統は変わらない`（PC の側。`ASHIATO_CHANNEL=official` を渡しても項目は `ashiato-collector-dev.cmd`）・・`// Scenario: 正式版は、渡された置き場と端末識別子をそのまま使う`・
  `// Scenario: 開発版は、渡された置き場の除外の登録を読む`（`main.rs` が使う、設定の `exclusions_path` から除外を読み込む関数を通して `Engine` に題名 `パスワード管理` の前景を通し、記録に題名が無い）・`// Scenario: 正式版と同じ置き場を渡しても、開発版と正式版は両方起動する`（二重起動の印を正式版の置き場と `<置き場>/dev` で両方取れ、2 つの未送信の道が別）
  試験（`runtime.rs` の `#[cfg(test)]`、名前は `st54_`）: `// Scenario: 開発版は、ブラウザの履歴の取り込みでも渡された置き場の除外の登録を読む`（開発版の系統で設定を読み、`exclusions.json` は**渡された置き場に直接**書く —— `write_rules` は `cfg.state_dir`（開発版では `<置き場>/dev`）に書くので使わない。取り込みの前に `<置き場>/dev/exclusions.json` が無いことを assert し、`history_runtime` と偽の読み手で `example.test/1`・`example.test/2` の訪問を取り込んで送り、送る本文に `example.test/1` が無く `example.test/2` がある。直していない `collect.rs:315` では落ちる）。
  `Runtime` に除外の登録の置き場を持たせる（今は `state_dir` だけ。`Runtime::new` で設定の `exclusions_path` を受け、`with_history` がそれを `HistoryCollector::new` に渡す）。`HistoryCollector::new` を呼ぶほかの口（`history/collect.rs:844` の試験の土台・`tests/runtime_windows.rs:1443`）と、`Config` を直書きする試験（`runtime.rs` の `cfg()`・`examples/browser_history_smoke.rs:172`・`tests/runtime_windows.rs:585`・`:654`）も合わせる。`tests/runtime_windows.rs` は `#![cfg(windows)]` で Linux の検証では組まれないので、Task 8 の Windows の検証で組めることを見る
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st54_ && test "$(cargo test -p ashiato-collector-windows st54_ -- --list 2>/dev/null | grep -c ": test$")" -ge 10'` rc=0 / `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows'` rc=0（既存の試験が通る）
- [x] 6.2 `tools/release/build.sh collector-windows` が、版番号に `-dev.` を含むとき zip の中の実行ファイルを `ashiato-collector-windows-dev.exe` にする（D8。版番号で分ける）。`--self-test` に
  `echo "== Scenario: 開発版の PC の収集アプリの配布物の実行ファイルは、-dev の名前を持つ"`（組み立てを偽の実行ファイルに差し替え、`0.4.0-dev.20261102` と `0.4.0` の zip の中の名前を読む）を足す
  検証: `tools/release/build.sh --self-test` rc=0

## Task 7: PC の収集アプリは同じ系統のサーバにだけ送り、違えば知らせる（D7・D12（仮）・D13。Q12・Q13・Q15）

- [x] 7.1 `Transport::get` の `Reply` に応答の見出しを足し、`/contract` の応答の見出し `Ashiato-Channel` を読む（無ければ `official`）。系統は `read_contract_answer` が `Version` と読んだ答えからだけ取る（404・読めない 200・401 などは今どおり。D14（仮））。`Runtime` の送る契機の頭（`runtime.rs:810-`）で、答えの系統が自分と違えばその契機は記録も生存信号も送らず、未送信を残し、`warn` で `channel_mismatch server=<相手> self=<自分>` を出す。
  違う系統の答えは `ContractGate` の契約の版の待ちに数えない（ST50 の「サーバを更新してください」の通知を出さない）。
  系統の違いの知らせ（D12（仮））: `contract_notice` と同じ決め方（入った契機で 1 回・前に出してから 24 時間ごと・同じ系統が答えた契機で消す・前に出した時刻を手元に持つ）を札 `channel` でもう 1 つ持ち（`ContractToaster` と同じ形の口。札 `contract`・`outage` とは別）、文は「送り先が開発版のサーバです。送り先を確かめてください」/「送り先が正式版のサーバです。送り先を確かめてください」（相手の系統で選ぶ）。`toast.rs` に札 `channel` の口を足す。
  試験（`runtime.rs` の `#[cfg(test)]`。偽の transport）: `// Scenario: 開発版の PC の収集アプリは、正式版のサーバに何も送らず手元に残す`・`// Scenario: 正式版の PC の収集アプリは、開発版のサーバに何も送らず手元に残す`・`// Scenario: 開発版の PC の収集アプリは、開発版のサーバには今どおり送る`・`// Scenario: 系統を答えない古いサーバは、正式版のサーバとみなす`・`// Scenario: 系統が違って送らない間は、サーバを更新してくださいの通知を出さない`（ST50 の通知の試験と同じ偽の時計と偽の通知の口）・
  `// Scenario: 系統が違って送らない間、PC の収集アプリは送り先を確かめるよう知らせる`・`// Scenario: 系統の違いの知らせは、続く間 24 時間ごとに出し直す`・`// Scenario: 送り先を直すと、PC の収集アプリは手元に残した記録を送り、系統の違いの知らせを消す`・`// Scenario: 契約の問い合わせが 404 なら、系統の違いの知らせでなく更新を促す`・`// Scenario: 契約の問い合わせが読めない 200 なら、系統の違いの知らせを出さない`・`// Scenario: 契約の問い合わせが 401 なら、系統の違いの知らせを出さない`・`// Scenario: 系統の違いの知らせの時刻は、収集アプリを起こし直しても引き継ぐ`・`// Scenario: 系統の違いの知らせを消しても、サーバを更新してくださいの知らせは消えない`
  読む口（`GET /outages` を引く `outage_notice`）は系統で止めない（D13。Q15）: `// Scenario: 系統の違うサーバからも、PC の収集アプリは途絶の判定を今どおり引く`（偽の transport に `/outages` が引かれたことを見る）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st54_channel && test "$(cargo test -p ashiato-collector-windows st54_channel -- --list 2>/dev/null | grep -c ": test$")" -ge 14'` rc=0 / `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st50_'` rc=0（ST50 の試験が通る）

## Task 8: Windows の上で、並べた 2 つの自動起動と差出人を確かめる

- [x] 8.1 `crates/collector-windows/tests/runtime_windows.rs` に `st54_side_by_side_autostart_and_aumid`（`// Scenario: 同じ PC に並べると、正式版と開発版の自動起動の項目と差出人が両方残る`）: 使い捨ての `APPDATA` を指して正式版の自動起動を仕込み、差出人を登録し（試験用の名前の前置きを付けた差出人で、本物の登録と混ざらないように `register_as` を使う）、
  続けて開発版の分を仕込み・登録して、スタートアップの 2 つのファイル・正式版の `.cmd` の中身が変わらないこと・登録簿の 2 つの鍵と表示名を見る。終わりに登録簿の試験用の鍵を消す
  検証: Windows で `cargo test -p ashiato-collector-windows --test runtime_windows st54_ -- --test-threads=1` が rc=0（CI の Windows の job）
  記録: 2026-11-05 の CI run 41958175380（head 6898088）の collector-windows-runtime で `st54_side_by_side_autostart_and_aumid ... ok`。手元の Windows（WSL から cmd.exe）は無人で画面がロックされているため、CI の結果で閉じる（人間が gate_help で付けた）

## Task 9: 携帯の開発版（名前・送り先・並べて入れる）

- [x] 9.1 `AndroidManifest.xml` の `android:label` を `${appLabel}` にし、`build.gradle.kts` が `-dev.` のときだけ `appLabel = "あしあと。開発版"`、ほかは `"あしあと。"` を `manifestPlaceholders` で渡す。`BuildConfig.CHANNEL`（`"dev"` / `"official"`）を足す（D1・D11）。
  `tools/release/check-artifacts.sh` に `apk-label <apk>`（`aapt2 dump badging` の `application-label`）と `apk-manifest <apk>`（求める権限・サービス・受け手の一覧から識別子を伏せたもの）を足し、`--self-test-android`（gradle で `<出所>-dev.20261102` と `<出所>` の debug を組む。`<出所>` は `tools/release/version.sh source`。gradle は出所と違う MAJOR.MINOR.PATCH を落とす）に
  `echo "== Scenario: 開発版のアプリの名前は「あしあと。開発版」で、正式版は「あしあと。」のまま"`・`echo "== Scenario: 開発版の携帯の収集アプリは、正式版と同じ権限と常駐の口を持つ"` を置く
  検証: `bash -c 'scripts/quiet-run apk -- tools/release/check-artifacts.sh --self-test-android'` rc=0
- [x] 9.2 `ContractGate.kt` の答え `Version` に `/contract` の応答の見出し `Ashiato-Channel` の系統を持たせ（見出しが無ければ `official`）、自分の `BuildConfig.CHANNEL` と違えば新しい答え `ChannelMismatch` にする。5 つの送り手（記録・生存信号・破棄の報告・原本の欠け・原本）は `ChannelMismatch` で何も送らず、`Log.w` に相手と自分の系統を残す。系統は `Version` の答えからだけ取る（404 `NoEndpoint`・`Unreadable`・`Transient` は今どおり。D14（仮））。読む口（`OutagePoller` の `GET /outages`・気分の通知の判断の `GET /feelings/reminder`）は系統で止めない（D13。Q15）。
  違う系統の答えは契約の版の待ち（ST50）に数えない。系統の違いの知らせ（D12（仮））: `ContractNotifier` と同じ決め方（入った契機で 1 回・24 時間ごと・同じ系統が答えた契機で消す・前に出した時刻を手元に持つ）を、同じ通知のチャンネル `contract` の別の通知の id でもう 1 つ持ち、画面の行（`MainActivity`）にも同じ文を出す。上限の 7 日前の知らせ（`contractHeadlineFor` の形）にも系統の違いの文を添える。
  試験（`app/src/test` の `ChannelGateTest`。Robolectric と偽の HTTP）: `// Scenario: 開発版の携帯の収集アプリは、正式版のサーバに何も送らず手元に残す`・`// Scenario: 正式版の携帯の収集アプリは、開発版のサーバに何も送らず手元に残す`・`// Scenario: 開発版の携帯の収集アプリは、開発版のサーバには今どおり送る`・
  `// Scenario: 系統が違って送らない間、携帯の収集アプリは画面の行と通知で送り先を確かめるよう知らせる`・`// Scenario: 送り先を直すと、携帯の収集アプリは手元に残した記録を送り、系統の違いの知らせを消す`・`// Scenario: 系統の違いの知らせは、携帯でも続く間 24 時間ごとに出し直す`・`// Scenario: 契約の問い合わせが 404 なら、携帯の収集アプリは系統の違いの知らせでなく更新を促す`・`// Scenario: 契約の問い合わせが読めない 200 なら、携帯の収集アプリは系統の違いの知らせを出さない`・`// Scenario: 携帯の上限の 7 日前の知らせに、系統の違いの案内を添える`・`// Scenario: 系統の違うサーバからも、携帯の収集アプリは途絶の判定を今どおり引く`（自分の系統は試験が渡す。ログは Robolectric の `ShadowLog`、通知は `ShadowNotificationManager` で見る）
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*ChannelGateTest*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*ChannelGateTest*.xml | grep -c "<testcase ")" -ge 10'` rc=0 /
  `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests *St50*` rc=0（ST50 の試験が通る）
  （訂正 2026-11-04: 検証の入口を「bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*ContractGate*"'」から上の入口に。受け入れ条件は変えない。理由: ST50 の携帯の試験は ContractGate の名を持たず（St50ContractNoticeTest・St50SenderTest・St50EnvelopeShapeTest）、*ContractGate* は main の時点から 1 本も当たらず No tests found で落ちる。受け入れ条件「ST50 の試験が通る」をそのまま確かめるため、ST50 の試験の名前で絞る（41 本 PASS） 実証: evidence.jsonl 2026-11-04T09:20:43+00:00 FAIL。承認: machine: evidence.jsonl 2026-11-04T09:21:00+00:00 PASS。記録: plan-corrections.md）
- [x] 9.3 `tools/android-side-by-side.sh`（D10・P2）: 端末が繋がっていなければ `tools/android-emulator.sh` と同じ AVD を画面なしで起動し（終わりに止める）、版番号 `<出所>`（`tools/release/version.sh source`）の debug（正式版）を入れて `run-as dev.ashiato.collector` で未送信に 1 件を置き（計測テストと同じ置き場の形）、件数を読む。
  続けて `<出所>-dev.20261102` の debug（開発版）を入れ、`pm list packages` に 2 つがあること・正式版の未送信の件数が同じことを見る。`echo "== Scenario: 開発版を入れても、正式版の端末内の記録は残る"`。結果は P2 の確かさを 4 にして `premises.md` に書く
  検証: `bash -c 'scripts/quiet-run emu -- tools/android-side-by-side.sh'` rc=0（`/dev/kvm` が要る。Codex の sandbox の中では `BLOCKED_INFRA` になり、外での実行を申請する —— `AGENTS.md`）

## Task 10: 開発版のリリースに携帯の APK を載せる（D6・D9・D10）

- [x] 10.1 `tools/release/check-artifacts.sh --channel nightly` を 4 つにする: 開発版の APK `ashiato-collector-dev-<版番号>.apk` を期待し、正式版の名前の APK があれば落とす。開発版の APK にも署名の指紋（`ASHIATO_EXPECT_CERT_SHA256`）と `BuildConfig` の空の検査を当てる。
  `--self-test` の ST52 の 2 つ（`開発版の配布物の検査は、携帯の APK を除く 3 つで通る`・`開発版の置き場に携帯の APK があれば、検査は落ちてその名前を示す`）を消し、
  `開発版のリリースの配布物は、開発版の APK を含む 4 つで、同じ版番号を持つ`（使い捨ての開発版の鍵で署名した APK）・`開発版の置き場に携帯の APK が無ければ、検査は落ちる`・`開発版の置き場に正式版の名前の APK があれば、検査は落ちてその名前を示す` を足す
  検証: `tools/release/check-artifacts.sh --self-test` rc=0
- [x] 10.2 `tools/release/nightly-notes.md`（1 行目は「この開発版はサポートしません。」で始まり、正式版と別のアプリとして並べて入ること・開発版のサーバは別の DB に繋ぐこと・`docs/release.md` を書く）と
  `tools/release/publish-nightly.sh <置き場> <版番号>`（D6・D9）: 開発版の鍵を Secrets（`ASHIATO_SIGNING_*`）から `$RUNNER_TEMP` に戻す → `sign-apk.sh <apk> dev <出力>`（`ASHIATO_EXPECT_DEV_CERT_SHA256`）→ 署名前の APK を外す → `check-artifacts.sh --channel nightly` →
  リリースが無ければ `gh release create nightly --prerelease --notes-file …` → `gh release upload --clobber` と要らない配布物の削除（ST52 D15 のまま）→ `gh release edit nightly --notes-file …` → `gh release view nightly --json body` の 1 行目を確かめる → `git push --force origin <commit>:refs/tags/nightly`。鍵は終わりに消す（`trap`）。鍵が無ければ「開発版の鍵が無い」と示して rc≠0。
  `--self-test`（偽の `gh` と偽の `git`、`sign-apk.sh --self-test` と同じ使い捨ての鍵）: `開発版の鍵の指紋の期待値が無ければ、開発版の APK に署名しない`・`開発版の鍵が置き場に無ければ、開発版は前の版のまま残る`・
  `開発版を公開するたびに、説明文は決まった文で書き直される`・`説明文の先頭にサポートしない旨が無ければ、公開の手順は落ちる`
  検証: `tools/release/publish-nightly.sh --self-test` rc=0 / `tools/release/sign-apk.sh --self-test` rc=0
- [x] 10.3 `nightly.yml`: `package-linux` に `tools/release/build.sh apk`（`ASHIATO_APK_UNSIGNED=1`・`ASHIATO_BUILD_VERSION`）を足す。`publish` の手順を `tools/release/publish-nightly.sh nightly/all "<版番号>"` の 1 行にし、env に Secrets（`ASHIATO_SIGNING_KEYSTORE_B64` など ST52 の `release.yml` と同じ 4 つの名前）と
  `ASHIATO_EXPECT_DEV_CERT_SHA256: ${{ vars.ASHIATO_CERT_SHA256 }}`・`ASHIATO_EXPECT_CERT_SHA256: ${{ vars.ASHIATO_CERT_SHA256 }}` を渡す。`environment: nightly`・`contents: write`・`if:` の公開の設定は今のまま。
  `check-guards.sh workflows` の nightly の検査を反転する（組み立ての job に `build.sh apk` がある・`publish-nightly.sh` は `environment: nightly` の `publish` の job からだけ呼ばれる・`ASHIATO_EXPECT_DEV_CERT_SHA256` が `vars.ASHIATO_CERT_SHA256` から渡る）。
  `--self-test` の `開発版の作り直しの workflow は、ST54 まで携帯の APK を組む手順を持たない` を消し、`開発版の作り直しの workflow は、携帯の APK を組み、公開の job の中でだけ開発版の鍵で署名する`（今の `nightly.yml` で rc=0、APK の手順を外した偽・署名を組み立ての job に置いた偽・指紋の変数が無い偽で rc≠0）を足す
  検証: `tools/release/check-guards.sh --self-test` rc=0 / `tools/release/check-guards.sh workflows` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/nightly.yml` rc=0
- [x] 10.4 `tools/release/draft.sh` に `--channel nightly`（D10）を足す: `gh run download` で手で起こした `nightly.yml` の run（公開の設定なし）の配布物を落とし、環境変数で渡した使い捨ての開発版の鍵で APK に署名し、`check-artifacts.sh --channel nightly` を通し、
  `gh release create --draft --prerelease --notes-file tools/release/nightly-notes.md` を出す（印を作らない）。`--self-test` に `開発版の確かめの組み立ては、開発版の鍵で署名した下書きになり、印を作らない`（偽の `gh` と使い捨ての開発版の鍵。下書きの要求に `--notes-file` があり、APK の指紋が使い捨ての鍵のもので、印の要求が無い）を足す
  検証: `tools/release/draft.sh --self-test` rc=0

## Task 11: 手順書と、印の突き合わせ

- [x] 11.1 `docs/release.md` に節を足す: 開発版の鍵を作る（`make-key.sh dev`。控えは USB に 1 部）と `nightly` の Environment に Secrets と変数 `ASHIATO_CERT_SHA256` を置く（ST53 の後、開発版の公開を始める前に本人が行う）・
  開発版を並べて入れる（携帯・PC。PC は正式版と別のフォルダに展開してよく、同じ `ASHIATO_STATE_DIR` を渡せば除外が効く）・開発版のサーバと DB（`ASHIATO_DB_PORT=55433 docker compose -p ashiato-nightly up -d db`・既定の待ち受け 18788・`DATABASE_URL` を開発版の DB に向ける）・
  開発版の DB が止まったら（`kind=channel_mismatch` と `kind=db_ahead` の読み方・`migrations/LINEAGE` の書き方・直しの移行はどちらの中身で当てた DB にも同じ結果になるように書く）・手元の clone で `git fetch --tags --force`（Q5）。
  `CLAUDE.md` の「構成」の `tools` の行に `android-side-by-side.sh` を、`migrations` の行に `LINEAGE` を足す
  検証: `bash -c 'for h in "make-key.sh dev" "ASHIATO_CERT_SHA256" "55433" "18788" "LINEAGE" "channel_mismatch" "git fetch --tags --force"; do grep -q -- "$h" docs/release.md || exit 1; done && grep -q "LINEAGE" CLAUDE.md'` rc=0
- [x] 11.2 印の突き合わせ: この change の Scenario が全部印を持ち、ST52 の消した 3 つの印が残っていない
  検証: `python3 scripts/check_scenarios.py . st54-dev-side-by-side` rc=0 / `bash -c '! grep -rn "携帯の APK を除く 3 つで通る\|ST54 まで携帯の APK を組む手順を持たない\|開発版の置き場に携帯の APK があれば" tools crates collector-android web'` rc=0

## 人間の確認待ち（確認バッチ。PR の CI が緑になった後）

下流の実装の中には配布元での実行が無いので、配布元での確かめはここで行う。Scenario ではない —— どれも完了の判定（D10）の読み替えと前提の確かさを上げる確かめで、結果は確認バッチが `premises.md` に書く。

- [ ] 12.1 **開発版の下書きのリリース**（D10・Q9）: `nightly.yml` を手で起こし（公開の設定は入れない。組むだけ）、その run で使い捨ての開発版の鍵を渡して `tools/release/draft.sh --channel nightly <run-id> <版番号>` を走らせる。
  作る前と後で `git ls-remote --tags origin` の行が同じこと、下書きに 4 つの配布物（`ashiato-collector-dev-<版番号>.apk` を含む）が並び、説明文の 1 行目が「この開発版はサポートしません。」で始まること、下書きの `ashiato-collector-dev-<版番号>.apk` を落として `apksigner verify --print-certs` の SHA-256 が使い捨ての開発版の鍵の指紋と一致し、`aapt2 dump badging` の識別子が `dev.ashiato.collector.dev` であることを見る。見た後に下書きを消す
- [ ] 12.2 **並べて入れる手順**: `docs/release.md` の開発版の節を読み、携帯・PC・サーバと DB を正式版と並べて立てる手順が、本人の手元でそのまま進められること（本物の開発版の鍵を作るのは ST53 の後。ST54 の確認では作らなくてよい）
