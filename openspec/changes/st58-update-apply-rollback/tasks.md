# ST58 実装タスク — 更新を入れ、移行が失敗したら自動で戻る

読む順: `deep.md`（**本人の答え Q1〜Q7**。第 1 回は推奨で先に書いてある）→ `defaults.md`（聞かない既定 C1〜C9・仮の候補 P1）→ このファイル → `specs/distribution/spec.md` → `specs/data-durability/spec.md` → `design.md` → `premises.md` → `CLAUDE.md`。

**前提**: requires の ST57（更新の確認・更新係・`GET /update-status`）・ST48（終了コード 3）・ST51（更新の履歴）・ST54（終了コード 4・系統）と、ST55（インストーラ）は archive 済みで main にある。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。
件数つきの検証は、試験を走らせた後に `-- --list`（gradle は結果の XML）の本数が下限以上かを同じコマンドの中で見る（0 本でも rc=0 になるため）。一時ファイルは `$TMPDIR` に書く（`/tmp` の固定名を使わない）。

**Windows で管理者の権限が要る確かめ（サービスの止め・開始の種類・ACL・戻し・`rollback` の口）は、下流の手元では走らない。** Windows の CI の job `install-windows` が新しい `tools/release/update-apply-install.sh` で毎回見る（ST55・ST57 と同じ形）。
下流の手元の検証は、(a) 更新の手順を組む純粋な関数と、サーバ・PC の収集アプリの Linux の `cargo test`、(b) `x86_64-pc-windows-gnu` への `cargo clippy`、(c) `update-apply-install.sh --self-test`（偽の `sc.exe`・`icacls`・`ashiato-server` を `PATH` の先に置き、確かめの論理だけを試す）の 3 つで閉じる。本物の Windows の通しは PR の CI で見る。
PC の入れ替え（版ごとの置き場からの起動・自動起動の台本・通知を押した後の窓）は、Windows の CI の job `collector-windows-runtime`（`tests/runtime_windows.rs`）が見る。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin / TypeScript はコメント（`// Scenario: 押す前は配布物を落とさない`）、bash は `echo "== Scenario: …"`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせる。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario**: `distribution` の ADDED 72 本と、MODIFIED で足した 10 本（`帯を押すと、サーバの更新の手順が開く`・`開発版の手順のリンクは印 nightly のページを指す`・`戻した更新は、帯と設定に出る`・`入らなかった更新は、帯と設定に出る`・`次に更新が通ると、戻した知らせは帯から消える`・`戻した知らせと更新ありと確かめられていないが同時なら 3 行出る`・`読めない更新の記録は画面に出さない`・`1 MB を超える更新の記録は画面に出さない`・`戻した・入らなかったの行を押しても何も起きない`・`戻したの値の行は入る`）、計 **82 本に ST58 が印を置く**。全部このファイルのどこかの Task が名指しする。
  MODIFIED で文を変えた既存の Scenario（`移行が失敗すると、サーバのサービスを開始せずに止まる`（初めてのインストールに限った）・`帯を押しても何も起きない`（「確かめられていない」の帯だけのときに替えた）・`更新の状態の口は応答を残させない`（`last_update` の欄）・data-durability の `説明書が更新の履歴と版の変化と生存信号の版を説明する`（`rolled_back` と「`failed` が無ければ」）・distribution の「S-01 の版が変わると、更新の履歴に 1 行残る」（Requirement の読み方の 1 文だけ。Scenario は変えない。Q9））は、
  今の印のある確かめ（`tools/release/install-check.sh`・`web/e2e/st57-update-notice.spec.ts`・`crates/server/src/st57_tests.rs`・`crates/server/src/export_update_history_tests.rs`）を新しい文に合わせて直す。**印を消さない**
- **本人の決定（下流は変えない）**: Q1 PC は収集アプリが自分で落として入れ替える / Q2 携帯はアプリが落として Android のインストール画面を開く / Q3 画面の帯を押すと「サーバの更新の手順」を開く / Q4 入らなかったことはインストーラの窓と画面の帯に出し、帯は次に更新が通るまで出す /
  Q5 更新の間はサーバが自分で起きないようにし、印を見てやり直す・待ち受けを開いた後は戻さない / Q6 `outcome` に `rolled_back` を足し、書けない回は控えて次の移行が写す / Q7 退避は更新が通ったらその回の 1 つだけを残す /
  Q8 携帯は更新の後、裏の位置の権限が無ければ位置を除いた種別（`specialUse`）で起こして位置以外を取り続ける（位置は開くまで止まる。D10） / Q9 待ち受けを開かずに戻した回は結果の行を書き直さず、「戻した」の 1 行に当てた版の名前を持たせる（D26）。
  聞かない既定: D1〜D9（`defaults.md` C1〜C9。D4 は C4 を直した —— 更新係は入れ替えの間だけ止める）。仮: D11〜D25（反転条件は design）。D10・D26 は本人の答え（Q8・Q9）。（第 1 回は推奨で先に書き、本人の答えが推奨と違えば上流が写してから下流へ渡す）
- **名前は統一する**（D14（仮））: 印と記録 `<状態の置き場>\update\state.json`・`outcomes.jsonl`・`history-pending.jsonl`、回の id `run_id`、退避 `backups\pre-migrate\<run_id>\`、前の版の退け先 `<プログラムの置き場>\previous\`、DB `ashiato_restore`・`ashiato_failed_<UTC>`、インストーラの口 `rollback`、
  サーバの口 `ashiato-server update-history record-rollback`・`ashiato-server update-history replay`、`GET /update-status` の欄 `last_update`、PC の口 `update`・置き場 `versions\<版>\`・`update-handoff.json`・`update-alive.json`・札 `update-rollback`、携帯の `UpdateInstall.kt`・`PackageReplacedReceiver.kt`・通知 ID 12・`noBackupFilesDir/update/`、
  試験の差し込み `ASHIATO_SETUP_TEST_STOP_AT`（`migrating`・`after-migrate`・`after-listen`・`restoring`）・`ASHIATO_SETUP_TEST_PG_MAJOR`・`ASHIATO_SERVER_TEST_MIGRATE_FAIL`・`ASHIATO_SERVER_TEST_EXIT_AFTER_LISTEN`・`ASHIATO_SERVER_TEST_NO_LISTEN`・`ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE`、共有の材料 `docs/contracts/update-apply-vectors.json`、道具 `tools/release/update-apply-install.sh`
- **試験の差し込みは試験の組み立て（`cfg(ashiato_test_build)`・携帯の試験の組み立て）でだけ読む**。配る組み立てに持たせない（`tools/release/check-artifacts.sh` が差し込みの名前の不在を見る —— 新しい名前を一覧に足す）
- **自動の試験と CI は本物の配布元（`api.github.com`・`github.com`・`release-assets.githubusercontent.com`）に問い合わせない**。受け手は loopback の手元の HTTP。試験の値に本物の資格情報を使わない
- **サーバの依存に HTTP・TLS の部品を足さない**（`tools/check-offsite.sh` が今のまま通る）。`last_update` はファイルを読むだけ
- **新しい依存を足さない**（PC の SHA-256 は既にある `sha2` を使う。無ければ `tools/check-licenses.sh`・`tools/check-vulns.sh` に通して足す。携帯は `java.security.MessageDigest`・`PackageInstaller`）
- **`unsafe` は書かない**。PC の新しい版のプロセスは窓が起こした子（`std::process::Child`）として止める
- **送信の封筒・送信契約の版・取り込みの口を変えない**（`tools/check-contract-version.sh` が落ちない）。移行は更新の履歴の CHECK に値を足す 1 本だけ（`migrations/<時刻>_update_history_rolled_back.sql` と `.down.sql`・`migrations/ORDER` の末尾へ 1 行）
- **DB の誤りの本文を、窓・更新の記録・控え・更新の履歴・ログに出さない**（ST51 と同じ。出すのは種別・落ちた版の名前・SQLSTATE だけ）
- **長いコマンドは `scripts/quiet-run`**（cargo test・gradle・エミュレータ・e2e）。試験の名前の接頭辞は `st58_`（Rust）・`St58`（Kotlin のクラス名）・`st58-`（e2e のファイル名）。サーバの試験は `crates/server/src/st58_tests.rs`
- 縦串・書き換え禁止・偽データ・録画のシナリオを足すなら Story ごとの新しいファイル（`tools/smoke.d/` など）に置き、共有の台本の本体に書き足さない。Windows の更新と戻しの確かめは `install-check.sh` に書き足さず、新しい `tools/release/update-apply-install.sh` に置く

## Task 1: 更新の履歴に「戻した」を書く（移行・`record-rollback`・控えの写し）

- [x] 1.1 移行 `migrations/<作成時刻>_update_history_rolled_back.sql`（と `.down.sql`）で、`core.update_history` の `outcome` の CHECK に `rolled_back` を足す（列と今の値は変えない。`.down.sql` は `rolled_back` の行があれば止まる）。`migrations/ORDER` の末尾へ 1 行。
  サーバに口 `ashiato-server update-history record-rollback`（D14（仮）・D18（仮）・D26。`DATABASE_OWNER_URL` で繋ぎ、引数で受けた新しい版・前の版・回の種類（移行が落ちた回は落ちた版の名前と SQLSTATE、待ち受けを開かなかった回はその印）から、戻した DB の最後の試みの行を読んで足す。移行が落ちた回は「落ちた試みの結果」と「戻した」の 2 行、待ち受けを開かなかった回は「戻した」の 1 行だけで、その行の当てた版に、自分の `MIGRATIONS` にあって戻した DB の版の記録に無い版の名前を並びの順に入れる（無ければ空。`applied` / `nothing_to_apply` の行は書かない）。1 つの取引で足す。CHECK が断る・表が無いときは表に何も書かずに、`--pending-out <控えのファイル>` に書けなかった行を 1 行 1 件の JSON で足して終了コード 5。標準出力で行を渡さない —— ログが標準出力に出る）と
  `ashiato-server update-history replay <控えのファイル>`（`written_at` の無い行だけを書いた順に、控えの時刻のまま足し、足した行に `written_at` を書き足す）を足す。
  試験（`crates/server/src/st58_tests.rs`・テスト用 DB）: `// Scenario: 戻したの値の行は入る`・`// Scenario: 戻すと、更新の履歴に落ちた試みの結果と戻したの 2 行が足される`（試みの行を持つ DB に `record-rollback` を当て、最後の 3 行の順と値を見る）・`// Scenario: rolled_back を受け付けない DB に戻すと控えに書き、次に通った移行が表へ写す`（ST58 の移行の前まで当てた DB で `record-rollback` が終了コード 5、控えを `replay` で写し、2 回目は増えない）・`// Scenario: 戻しても、試みの行は書き直さない`・`// Scenario: 待ち受けを開かずに戻すと、更新の履歴に当てた版の名前を持つ戻したの 1 行が足される`（版を 1 つ当てる前の DB を戻した DB として、その版を並びに持つ組み立てで `record-rollback` を当て、最後の 2 行と `applied` / `nothing_to_apply` が増えないことを見る）・`// Scenario: 当てる版の無い更新で待ち受けを開かずに戻すと、戻したの行の当てた版は空`・`// Scenario: 待ち受けを開かずに戻した後も、前の版は戻した先の版のまま`（戻した後に前の版の印で起動の書き込みを走らせて行が増えず、別の印の migrate の試みの行の前の版が戻した先の版）・`// Scenario: 待ち受けを開かずに rolled_back を受け付けない DB に戻すと、控えに戻したの 1 行を書く`（終了コード 5 で `--pending-out` のファイルが「戻した」の 1 行だけを持ち（ほかの行・ログの行が混ざらない）、当てた版の名前を持ち、それを `replay` で写す。D26）。
  ST51 の試験 `st51_form_rejects_unknown_via`（`update_history_tests.rs:89-93`）は `("S-01", "migrate", "rolled_back")` が断られることを確かめているので、この移行で落ちる。その組の値を、まだ決めていない別の値（`reverted` など）に替える（印 `決めていない値の行は入らない` は消さない。audit 2026-11-15 K1）。
  既にある `決めていない値の行は入らない`（`update_history_tests.rs`）と、書き出しの `説明書が更新の履歴と版の変化と生存信号の版を説明する`（`export_update_history_tests.rs`。説明書に `rolled_back` の説明を足し、試験を新しい AND に合わせる）が通る
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st58_history_ && test "$(cargo test -p ashiato-server st58_history_ -- --list 2>/dev/null | grep -c ": test$")" -ge 8'` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server update_history && test "$(cargo test -p ashiato-server update_history -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server export_update_history && test "$(cargo test -p ashiato-server export_update_history -- --list 2>/dev/null | grep -c ": test$")" -ge 1'` rc=0 / `tools/check-migrations.sh` rc=0 / `tools/check-immutable.sh` rc=0

## Task 2: サーバの `last_update` と、移行と起動の試験の差し込み

- [x] 2.1 `GET /update-status` に欄 `last_update` を足す（D14（仮）・D19（仮））: 状態の置き場の `update\outcomes.jsonl`（サービスの形は系統の定数の置き場、開発の形は `ASHIATO_STATE_DIR` か無し）の最後の 1 行を、1 MB を超えれば読まず、版・時刻・`result`・理由の種別・SQLSTATE（5 文字）の形を確かめてから返す（どれかが外れれば `null`）。`docs/openapi.json` を作り直す。
  `migrate` の退避を `pre-migrate-<時刻>.dump.part` に書き、`pg_dump` が通って中身があるときだけ `.dump` へ名前を変える（D2。試験 `st58_status_backup_` で、`pg_dump` が落ちた・止められた回に `.dump` が残らないことを見る）。
  `migrate` とサーバに、試験の組み立てでだけ読む差し込みを足す: `ASHIATO_SERVER_TEST_MIGRATE_FAIL`（`sqlstate:<符号>` で版を当てる取引をその SQLSTATE で落とす・`before-backup` で退避の前に終了コード 1・`during-backup` で退避を書く途中で止まる）・`ASHIATO_SERVER_TEST_NO_LISTEN`（待ち受けの前に終わる）・`ASHIATO_SERVER_TEST_EXIT_AFTER_LISTEN`（待ち受けの 5 秒後に終わる）。
  試験（`crates/server/src/st58_tests.rs`）: `// Scenario: 読めない更新の記録は画面に出さない`（版が `<script>`・最後の行が壊れている）・`// Scenario: 1 MB を超える更新の記録は画面に出さない`・`// Scenario: 戻した後も、新しい版を更新ありに出す`（更新係の結果 `0.5.0`・サーバの版 `0.4.0`・最後の行 `rolled_back` で `state` が `available`）と、`last_update` の 3 つの結果（`applied`・`rolled_back`・`not_applied`）を 1 本ずつ。
  既にある `更新の状態の口は応答を残させない`（`st57_tests.rs`）を `last_update` の欄に合わせて直す
  検証: `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st58_status_ && test "$(cargo test -p ashiato-server st58_status_ -- --list 2>/dev/null | grep -c ": test$")" -ge 5'` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server st57_'` rc=0 / `./tools/check-openapi.sh` rc=0 / `tools/check-offsite.sh` rc=0

## Task 3: 画面の帯（手順を開く・戻した・入らなかった）

- [x] 3.1 `web/src/update.ts`・`UpdateBand.tsx`・`SettingsView.tsx`（Q3・Q4 の推奨・D15（仮））: 「更新あり」の行を押すと、画面の中に「サーバの更新の手順」を開く（版・3 段・「移行が落ちたら自動で前の版に戻ります」・リンク `https://github.com/rai-256/ashiato/releases/tag/v<版>`、開発版は `…/tag/nightly`）。「確かめられていない」「戻した」「入らなかった」の行は押しても何もしない。
  `last_update` の最後の行が `rolled_back` / `not_applied` なら、帯に「更新 <新> は移行が落ちたので、<前> に戻しました（<落ちた版>・<理由>）」/「更新 <新> は入りませんでした（<理由>）。<前> のまま動いています」を出し、設定の「更新の確認」にも同じ行を出す。理由の文は D15（仮）の表（`update.ts` に置き、vitest で全部の種別と SQLSTATE の級を引く）。
  e2e（`web/e2e/st58-update-apply.spec.ts`。`/update-status` の応答を差し替える）: `// Scenario: 帯を押すと、サーバの更新の手順が開く`（外への要求 0 件を数える）・`// Scenario: 開発版の手順のリンクは印 nightly のページを指す`・`// Scenario: 戻した更新は、帯と設定に出る`・`// Scenario: 入らなかった更新は、帯と設定に出る`・`// Scenario: 次に更新が通ると、戻した知らせは帯から消える`・`// Scenario: 戻した知らせと更新ありと確かめられていないが同時なら 3 行出る`（360 px・文書の幅が窓の幅を超えない）・`// Scenario: 戻した・入らなかったの行を押しても何も起きない`。
  既にある `web/e2e/st57-update-notice.spec.ts` の `帯を押しても何も起きない` を新しい GIVEN（「確かめられていない」の帯だけ）に合わせて直す
  検証: `scripts/quiet-run e2e -- bash -c 'cd web && npm run test:e2e -- st58-update-apply.spec.ts st57-update-notice.spec.ts'` rc=0 / `bash -c 'cd web && npx tsc --noEmit && npx vitest run'` rc=0

## Task 4: インストーラの更新の計画（純粋な関数）

- [x] 4.1 `crates/installer` に更新の計画の純粋な関数を足す（D1・D2・D5・D15（仮）・D20（仮）・D23（仮））: 入っている版と自分の版から「走らせ直し / 更新 / 系統が違うので触らない」を決める・印の段から「前の版へ戻し終える手順」か「(7) の残りを進める手順」（段 `listening`）を組む・`migrate` の終了コードとこの回の置き場の中身から「DB を戻す / プログラムだけ戻す」を決める（0・2・3・4 以外で、この回の取り終えた退避があれば戻す。2・3・4 と、取り終えた退避が無い回は戻さない）・
  理由の種別と短い説明・`PG_VERSION` と組み立ての大きい版の比べ・保持で消す退避と DB の選び方（この回の置き場以外・`pre-migrate-*.dump`・`ashiato_failed_*`）・更新の記録の 1 行の形・`sc.exe` / `pg_restore` / `psql` の命令を組む関数（戻しは `ashiato_owner` で繋ぐ）。
  試験（`#[cfg(test)]`、名前は `st58_plan_`）: 終了コード 0〜4・137 と、この回の取り終えた退避のある・書きかけだけ・無いの全部の組み合わせ（18 件以上。0・2・3・4 以外で取り終えた退避があれば戻す）・段 `listening` の印では戻さないこと・印の段ごとの戻し終える手順（`started`〜`restored`）・保持の選び方（前の回の置き場 2 つ・じかの退避 1 つ・この回 1 つ）・大きい版 `17` / `18`・命令に DB の管理者の役割 `ashiato` が現れないこと・開発版のインストーラが正式版の置き場を選ばないこと
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup st58_plan_ && test "$(cargo test -p ashiato-setup st58_plan_ -- --list 2>/dev/null | grep -c ": test$")" -ge 20'` rc=0

## Task 5: インストーラの更新の手順・戻し・`rollback` の口

- [x] 5.1 `crates/installer/src/main.rs` に更新の手順を足す（D1〜D6・D16（仮）・D20（仮）・D23（仮））: 入っている版を読み、違えば（系統が同じなら）D1 の 8 段で更新する。`program_version_mismatch_message` で止まる形を更新の手順に置き換える（同じ版の走らせ直しは今のまま）。
  印と記録の置き場 `update\` を作り、ACL を Administrators と SYSTEM の書き・2 つのサービスのアカウントの読みにする（初回のインストールでも作る）。退避は回ごとの置き場を `MIGRATE_BACKUP_DIR` で渡す。戻しは D3（新しい版を `failed\` へ退け、`previous\` を戻し、`ashiato_restore` → 名前の入れ替え。役割は D25（仮））。戻した後は `failed\` の新しい版の `ashiato-server update-history record-rollback --pending-out <状態の置き場>\update\history-pending.jsonl` を走らせ、終了コード 5 は「控えに書いた」と読む（D18（仮））。戻した後、切ってなければ前の版の更新係を起こす。
  更新が通った後は、`update-history replay`・保持・`ashiato_failed_*` を消す・更新の記録に `applied`。入らなかった回は窓に D15（仮）の文を出して rc≠0、更新の記録に `rolled_back` / `not_applied`（大きい版の違いも `not_applied`・`pg_major_mismatch`）。印に段 `listening` を書いてから (7) の残りを進め、段 `listening` の印の走らせ直しと `rollback` は戻さずに残りを進める。口 `rollback`（印が無ければ「戻すものはありません」）。試験の組み立ての差し込み `ASHIATO_SETUP_TEST_STOP_AT`（`migrating`・`after-migrate`・`after-listen`・`restoring`）・`ASHIATO_SETUP_TEST_PG_MAJOR`。
  起こした命令の記録（ST55 の試験の組み立ての記録）に、戻しの命令の役割が出るようにする
  試験（`crates/installer` の `#[cfg(test)]`・`st58_plan_`）: 偽の `ashiato-server`（終了コード 5 で、渡された `--pending-out` のファイルに 1 行を足し、標準出力にログの行を出す）で戻しの後の段を走らせ、控えのファイルが JSON の行だけを持ち、インストーラが終了コード 5 を「控えに書いた」と読んで先へ進むことを見る（review/spec.md R35）
  検証: `cargo clippy -p ashiato-setup --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-setup'` rc=0 / `tools/release/check-artifacts.sh --self-test` rc=0

## Task 6: Windows の CI で、更新・戻し・途中で止まる・やり直しを通す

- [x] 6.1 新しい `tools/release/update-apply-install.sh`（D24（仮））: 同じ commit から版番号だけを替えた 2 つの試験の組み立ての zip を使い、古い方を入れてから新しい方で次を見る（各段の前に `echo "== Scenario: <名前>"`）。`--self-test` は偽の `sc.exe`・`icacls`・`ashiato-setup.exe`・`ashiato-server.exe`・`psql` を `PATH` の先に置いて、確かめの論理（件数の比べ・ACL の読み・出力の文の照合）がわざと壊した入力で落ちることを見る。
  `tools/release/build.sh` に 2 つ目の版の試験の組み立てを足し、CI の `package-windows` が両方を作り、`install-windows` が `update-apply-install.sh` を走らせる（`rust` の job に `tools/release/update-apply-install.sh --self-test`）。`--list` は印を置く Scenario の名前を 1 行ずつ出し、`install-windows` は走った `== Scenario:` の行と `comm` で突き合わせて欠けがあれば落ちる（ST55・ST57 の `install-check.sh` と同じ形）。
  印を置く Scenario: `新しい版のインストーラで走らせると、移行を当てて新しい版のサーバが起きる`・`更新の移行の間、サーバは待ち受けない`・`更新しても、秘密と設定と画面の合言葉は変わらない`・`同じ版で走らせ直すと、更新の手順に入らない`・`サーバのアカウントは、更新の途中の印を書けない`・`開発版のインストーラは、正式版のインストールを更新しない`・
  `更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる`・`戻した後、更新の間に溜まった未送信が入る`・`戻した後の「アプリと機能」の版とプログラムの置き場は前の版`・`戻しは DB の管理者の役割を使わない`・`戻した後、切っていない更新係は動いている`・`書きかけの退避は戻さない`・`正式版のインストーラは、開発版のインストールを更新しない`・`待ち受けを開いた後に止まった更新は、走らせ直しても DB を戻さない`・`待ち受けを開かずに戻すと、更新の履歴に当てた版の名前を持つ戻したの 1 行が足される`（Windows の通しでも見る）・`DB に新しい版の知らない版があると、戻さずに前の版のまま起きる`・`系統が違う DB には、戻さずに前の版のまま起きる`・`退避の前に落ちた回は、前の回の退避を戻さない`・
  `新しい版のサーバが待ち受けを開かなければ、戻して前の版で起きる`・`待ち受けを開いた後に新しい版が落ちても、DB は戻さない`・`更新の途中で止まると、サービスは自分で起きない`・`移行の後に止まった更新は、走らせ直すと戻してからやり直し、新しい版で起きる`・`移行の後に止まった更新は、前の版のインストーラで走らせ直すと前の版で起きる`・
  `戻しが落ちると、サーバを止めたまま、退避の場所とやり直しの口を示す`・`戻しをやり直す口で、前の版で起きる`・`印が無ければ、戻しの口は何も替えない`・`移行が落ちて戻すと、窓に戻したことと版と理由が出る`・`移行の前に止まると、窓に入らなかったことと種別が出る`・`更新が通ると、更新の記録に通った行が足される`・
  `更新が通ると、移行の前の退避は直前の 1 つだけになる`・`戻した回は退避を消さない`・`更新が通ると、前の回に残した落ちた DB を消す`・`大きい版の違う PostgreSQL を持つ配布物では、何も替えずに止まる`・`サーバの切った更新の確認は、更新でも戻しでも切ったまま`（P20 を確かさ 4 に上げる。`premises.md` の P20 の行を直す）。
  既にある `install-check.sh` の `移行が失敗すると、サーバのサービスを開始せずに止まる` を「前の版の無い PC に初めてインストールする」に合わせて直す
  検証: `tools/release/update-apply-install.sh --self-test` rc=0 / `tools/release/install-check.sh --self-test` rc=0 / `bash -c 'tools/release/build.sh --self-test'` rc=0 / `bash -c 'f="$1"; actionlint "$f" || docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7 "$f"' _ .github/workflows/ci.yml` rc=0 / `bash -c 'grep -q "update-apply-install.sh --self-test" .github/workflows/ci.yml && grep -q "update-apply-install.sh --list" .github/workflows/ci.yml'` rc=0 / 終了の条件: PR の CI の `install-windows` が緑（下流の手元では走らない）

## Task 7: 配布物の選び方と指紋（共有の crate と材料）

- [x] 7.1 `crates/update-check`（`ashiato-update-check`）に、版の一覧から自分の部品の配布物の場所と指紋を選ぶ純粋な関数を足す（D9・D11（仮））: 選んだ版の `assets[]` から部品の配布物の名前（携帯は APK・PC は PC の zip）を選び、`browser_download_url` が `https://github.com/rai-256/ashiato/releases/download/` で始まること、`digest` が `sha256:<64 桁の 16 進>` であることを確かめる。転送先が許す宛先（`https://release-assets.githubusercontent.com/`）かを返す関数と、落とす要求の見出しの集合（確かめと同じ 5 つの名前）。
  `docs/contracts/update-apply-vectors.json` を作る（`digest` の無い・形の違う・道の違う・宛先の違う・部品の無い・開発版の印 `nightly` の配布物を含めて 15 件以上）。Rust と Kotlin（Task 9.1）が全件を通す。
  試験（`#[cfg(test)]`、名前は `st58_asset_`）: 材料の全件・`// Scenario: 指紋の無い配布物は入れない`（選ぶ関数の側）
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-update-check st58_asset_ && test "$(cargo test -p ashiato-update-check st58_asset_ -- --list 2>/dev/null | grep -c ": test$")" -ge 3'` rc=0 / `bash -c 'test "$(python3 -c "import json;print(len(json.load(open(\"docs/contracts/update-apply-vectors.json\"))[\"cases\"]))")" -ge 15'` rc=0

## Task 8: PC の収集アプリの入れ替え

- [x] 8.1 `crates/collector-windows` に口 `update` と窓を足す（Q1・D7・D12（仮）・D13（仮）・D17（仮））: 札 `update` の通知を押すと `current_exe() update` で窓を開く。窓は版を示して `[y/N]` を聞き、`y` なら版の一覧を取り直し（確かめの数えと記録に入れない）、配布物を `versions\<版>\` に落として指紋を照らし、`update-handoff.json` で動いている収集アプリに止まるよう頼み、止まったのを待って新しい版を起こし、60 秒以内の `update-alive.json` を待つ。
  書かれたら自動起動の台本を新しい版の場所に書き換え、前の版より古い `versions\` を消す。書かれなければ子を止め、前の版を起こし直し、札 `update-rollback` で知らせる。見つけた最新の版が入っている版より新しくなければ「入っている版が最新です」。新しい版は送る仕組みと見回りを始めた後に `update-alive.json` を書く。
  `tools/check-no-time-server.sh` の許可に `Command::new(update_executable(&…)?)` の形を 1 つ足し、`--self-test` に許す形と、名前を引数に取る形を落とす植え付けを足す。試験の差し込み `ASHIATO_COLLECTOR_TEST_UPDATE_URL`（ST57）の受け手が配布物と転送も返す・`ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE`。
  試験（Linux。`crates/collector-windows/tests/st58_pc_update.rs`。loopback の受け手を試験の中で立て、実行ファイルの代わりに試験の組み立ての小さな実行ファイルを配布物にする）: `// Scenario: PC は y 以外の答えでは何も替えない`・`// Scenario: PC は入っている版より新しくない版を入れない`・`// Scenario: 押す前は配布物を落とさない`・`// Scenario: 配布物は決まった 2 つの宛先からだけ落とす`・`// Scenario: 指紋の無い配布物は入れない`・`// Scenario: 落とす要求は自分の版も端末の情報も持たない`・`// Scenario: 押したときの取り直しは、確かめの数えを動かさない`・`// Scenario: PC は入れ替えの前に積んだ未送信を、入れ替えた後に送る`・`// Scenario: PC の切った更新の確認は、入れ替えても切ったまま`・`// Scenario: PC は指紋の合わない配布物で入れ替えない`・`// Scenario: PC は前の版に戻しても、新しい版が積んだ未送信を消さない`（差し込み `ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE` で送信契約の版を上げた行を積む）。版ごとの置き場が 2 回続けて入れ替えても 1 段で、残るのは今の版と前の版の 2 つ（D12（仮））。
  `echo "== Scenario: 時刻サーバの検査は、入れ替えた新しい版を起こす形を許し、ほかの子のプロセスを許さない"` を `check-no-time-server.sh --self-test` に置く
  検証: `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows st58_pc_ && test "$(cargo test -p ashiato-collector-windows st58_pc_ -- --list 2>/dev/null | grep -c ": test$")" -ge 12'` rc=0 / `bash -c 'scripts/quiet-run ct -- cargo test -p ashiato-collector-windows'` rc=0（既存の試験が通る）/ `tools/check-no-time-server.sh` rc=0 / `tools/check-no-time-server.sh --self-test` rc=0 / `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0
- [x] 8.2 Windows の実行時の試験（`crates/collector-windows/tests/runtime_windows.rs` に st58 の関数を足す。CI の `collector-windows-runtime` が直列で走らせ、本数の下限を 13 から 19 に上げる）: `// Scenario: PC は新しい版の通知を押すと、入れ替えの窓を開く`（通知の受け手を呼ぶ）・`// Scenario: PC は y で新しい版に入れ替え、前の版の実行ファイルを残す`・`// Scenario: PC は新しい版が動き出してから、自動起動の台本を新しい版に向ける`・`// Scenario: PC は新しい版が 1 分以内に動き出さなければ、前の版に戻して起こし直す`・`// Scenario: PC は入れ替えの間、前の版と新しい版が同時に記録を積まない`・`// Scenario: PC は窓を閉じた後も、入れ替えた新しい版が動き続ける`（P24 を確かさ 4 に上げる）。
  版ごとの置き場からの起動で、通知の差出人と除外の登録が前の版と同じことも見る（D12（仮）の反転条件）
  検証: `cargo clippy -p ashiato-collector-windows --tests --target x86_64-pc-windows-gnu -- -D warnings` rc=0 / `bash -c 'grep -q "\-lt 19" .github/workflows/ci.yml'` rc=0 / 終了の条件: PR の CI の `collector-windows-runtime` が緑（下流の手元では走らない）

## Task 9: 携帯の収集アプリの入れ替えと、更新の後の再開

- [x] 9.1 `collector-android`（Q2・Q8・D10・D11（仮）・D21（仮）・D22（仮））: `REQUEST_INSTALL_PACKAGES` を宣言し、`UpdateInstall.kt`（版の一覧を取り直し、`update-apply-vectors.json` と同じ規則で APK と指紋を選び、`noBackupFilesDir/update/` に落として照らし、`PackageInstaller` の手続きで確認の画面を出す。結果を受けたら APK を消す。開いたときに残りを消す）・最初の画面の「更新あり」の行を押せるようにする（許されていなければ「提供元不明のアプリ」の許可の画面へ）・通知の押した先に「落とし始める」の印・
  `PackageReplacedReceiver.kt`（`ACTION_MY_PACKAGE_REPLACED`。送り先があれば 1 回きりの印のファイル `noBackupFilesDir/update/replaced-start` を置いて `LocationService` を起こす。D10 の表のとおり、`onCreate` の `foregroundServiceType()` は印があれば位置と裏の位置の権限が両方あるときだけ `location`、無ければ `specialUse`（印は読んだら消す。印が無ければ今のまま）。受け手から `specialUse` で起こしたサービスで、位置の権限があり裏の位置の権限が無い間は、位置のソースの取得できる状態を「取得できない（権限）」にして `collect()` を呼ばない（裏の位置の権限も許されたら今どおり取る）。位置の権限があって `specialUse` で起こしたら通知 ID 12 で 1 回知らせる。画面が起こす Intent に「前景から起こした」の印を付け、`onStartCommand` でその印があり、受け手から起こしたサービスで `specialUse` で位置の権限があれば `location` で `startForeground` を呼び直し、通知 ID 12 を消す）。
  試験（Robolectric。`St58PackageReplacedTest.kt`）: D10 の表の 5 行（印のファイルの有無 × 位置・裏の位置の権限）で選ぶ種別と、`specialUse` の間の位置のソースの取得できる状態（裏の位置の権限が無い間は取得できない・権限、裏の位置の権限も許された後は取得できる）を 1 本ずつ見る。`noBackupFilesDir` はバックアップの規則の外（`tools/check-android-backup-rules.sh` が通る）。
  試験（Robolectric。`app/src/test/.../St58UpdateInstallTest.kt`・`St58PackageReplacedTest.kt`。手元の受け手）: 材料の全件・`// Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く`（手続きに書いた中身の SHA-256）・`// Scenario: 携帯の更新の通知を押すと、落とし始める`・`// Scenario: 携帯は指紋の合わない APK でインストール画面を開かない`・`// Scenario: 携帯は提供元不明のアプリが許されていなければ、許可の画面を開く`・`// Scenario: 携帯でインストール画面を取り消すと、入っている版のまま動き続ける`（取り消しの結果を渡す）・`// Scenario: 携帯は入れ終えた後に APK を置き場に残さない`（成功の結果を渡し、開いたときの掃除も見る）・
  `// Scenario: 押す前は配布物を落とさない`・`// Scenario: 配布物は決まった 2 つの宛先からだけ落とす`・`// Scenario: 指紋の無い配布物は入れない`・`// Scenario: 落とす要求は自分の版も端末の情報も持たない`・`// Scenario: 押したときの取り直しは、確かめの数えを動かさない`（携帯の側。Rust の側と同じ名前の印を両方に置く）
  検証: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*St58*" && test "$(cat collector-android/app/build/test-results/testDebugUnitTest/TEST-*St58*.xml | grep -c "<testcase ")" -ge 22'` rc=0 / `tools/check-android-backup-rules.sh` rc=0
- [x] 9.2 計測テスト（エミュレータ。`app/src/androidTest/.../St58PackageReplacedInstrumentedTest.kt`）。APK を入れ替えるとアプリのプロセスが落ち、gradle の結果の XML に残らないので、段の台本 `tools/android-st58-replace.sh` を置く: 版番号だけを替えた試験の組み立ての APK を 2 つ組み（同じ試験の鍵で署名）、古い方を入れて設定し、`adb install -r` で新しい方を入れてから `adb shell am instrument --no-restart -w -e class dev.ashiato.collector.St58PackageReplacedInstrumentedTest#<段>` で確かめ、`OK (<n> tests)` を照らす（St56 と同じ形）: `// Scenario: 携帯は更新の後、開かなくても収集を再開する`・`// Scenario: 携帯は裏の位置の権限が無くても、更新の後に位置以外の収集を再開し、開くよう通知する`・`// Scenario: 位置の権限が無い携帯は、更新の後に位置以外の収集を再開し、通知を出さない`・`// Scenario: 通知の権限が無い携帯も、更新の後に位置以外の収集を再開する`・`// Scenario: 更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する`・`// Scenario: 送り先の無い携帯は、更新の後に収集を起こさない`・`// Scenario: 携帯の切った更新の確認は、入れ替えても切ったまま`（権限は段ごとに `adb shell pm grant` / `pm revoke` で整え、種別はサービスのログの種別の行で見る。段の確かめは **`am instrument --no-restart`** で始め、受け手が起こしたプロセスを止めない（台本は `adb shell pidof dev.ashiato.collector` で確かめの前後のプロセスの id が同じことも見る）。「開くと位置の収集を再開する」の段は、`adb shell am start` で画面を開き、裏の位置の権限を求める画面は UiAutomator で断り、`adb emu geo fix` で位置を流し込み、同じプロセスの id のまま種別の行が `specialUse` → `location` と続き、通知 ID 12 が消えたことを見る）。
  結果を `premises.md` の P6・P7・P26 に書き、確かさを 4 に上げる（P26 が外れたら D10 の「止めて画面から起こし直す」形に替える。受け手から `specialUse` で起こせないと分かったら、通知だけの形には戻さず、上流へ返して本人に聞き直す。D10）
  検証: `scripts/quiet-run emu -- tools/android-st58-replace.sh` rc=0（台本の中で 7 つの段の `OK (1 test)` を照らし、1 つでも欠ければ落ちる） / `tools/android-st58-replace.sh --self-test` rc=0（`OK` の行の欠けた出力で落ちる）

## Task 10: 文書

- [ ] 10.1 `docs/install.md` に「サーバの更新」の節（新しい版の zip のインストーラを走らせる・更新の間はサーバが止まる・移行が落ちたら自動で前の版に戻る・入らなかったときの窓の文と画面の帯・途中で止まったら新しい版の `ashiato-setup.exe` で走らせ直すか `ashiato-setup.exe rollback`・退避は直前の 1 つ・同梱の PostgreSQL の大きい版が違う配布物では止まる・ST58 より前の版のインストーラは印を読まない）。
  `docs/collector-setup.md` に PC の `update` の口と窓・携帯の「提供元不明のアプリ」の許可と更新の後の再開。`docs/release.md` に「`tools/release/pgsql.lock` の小さい版を上げる PR では、前の小さい版の zip から更新と戻しを通す」（design Risks・P25）。申し送り `docs/handoff/ST60.md` の ST58 D8 の節がある
  検証: `bash -c 'for h in "rollback" "前の版に戻" "update\\\\outcomes.jsonl" "大きい版"; do grep -q -- "$h" docs/install.md || exit 1; done && grep -q "提供元不明のアプリ" docs/collector-setup.md && grep -q "pgsql.lock" docs/release.md && grep -q "ST58 D8" docs/handoff/ST60.md'` rc=0

## Task 11: 全体

- [ ] 11.1 Scenario の印の突き合わせと、作業場の検査
  検証: `python3 scripts/check_scenarios.py . st58-update-apply-rollback` rc=0 / `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && scripts/quiet-run all -- cargo test --workspace'` rc=0 /
  `bash -c 'tools/check-offsite.sh && tools/check-updater-deps.sh && tools/check-contract-version.sh && tools/check-licenses.sh && tools/check-openapi.sh && tools/check-no-time-server.sh'` rc=0

## 人間の確認待ち（確認バッチ。PR の CI が緑になった後）

Scenario ではない —— 機械が全部の Scenario を見る。ここは本物の配布元・本人の PC と端末で、結果は確認バッチが `premises.md` に書く。

- [ ] 12.1 **本物の配布元で 1 回入れる**: 配布元に版の違う 2 つのリリース（ベータ版か開発版）が出た後、携帯の「更新あり」の行・PC の通知・画面の帯から新しい版を入れ、指紋が本物の `digest` と合って入ることを見る。**本物の `github.com` → `release-assets.githubusercontent.com` の転送を通るのはここが最初**（試験と CI は loopback の受け手。D11（仮）の反転条件）。いまは配布元のリリースが 0 件（P1）なので、出るまで待つ
  > 物理: realdata —— 本物の配布元のリリースは手元の受け手では作れない（試験・CI は本物の配布元に聞かない）
- [ ] 12.2 **本人の PC のサーバの更新**: ST57 までの版を入れた本人の PC で、新しい版の zip のインストーラを走らせて更新が通り、記録の件数と画面の合言葉がそのままで、退避が直前の 1 つになることを見る
  > 物理: realdata —— 本人の PC の前のインストールの状態と記録の量は CI で再現しきれない
