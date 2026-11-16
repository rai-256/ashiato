<!-- 処置の判定: re-review -->
# ST58 code_verify（独立検証。2026-11-16・HEAD 9869168bf）

## 申告と実測

申告: **`[x]` 13 件 / 全 15 件**（1.1〜11.1。12.1・12.2 は人間の確認待ちで `[ ]`）。各項目の検証コマンドは下の表。

| 項目 | 検証コマンド | 申告 | 実測（この席で走らせた） |
|---|---|---|---|
| 1.1 | `cargo test -p ashiato-server st58_history_`（≥8） | PASS | rc=0・8 本 |
| 1.1 | `… update_history`（≥1）/ `… export_update_history`（≥1） | PASS | rc=0・31 本 / rc=0・5 本 |
| 1.1 | `tools/check-migrations.sh` / `tools/check-immutable.sh` | PASS | rc=0 / rc=0 |
| 2.1 | `… st58_status_`（≥5）/ `… st57_` | PASS | rc=0・8 本 / rc=0・17 本 |
| 2.1 | `tools/check-openapi.sh` / `tools/check-offsite.sh` | PASS | rc=0 / rc=0 |
| 3.1 | e2e `st58-update-apply.spec.ts st57-update-notice.spec.ts` | PASS | rc=0・21 passed |
| 3.1 | `npx tsc --noEmit && npx vitest run` | PASS | rc=0 |
| 4.1 | `cargo test -p ashiato-setup st58_plan_`（≥20） | PASS | rc=0・27 本 |
| 5.1 | clippy（windows-gnu）/ `cargo test -p ashiato-setup` / `check-artifacts.sh --self-test` | PASS | rc=0 / rc=0 / rc=0 |
| 6.1 | `update-apply-install.sh --self-test` / `install-check.sh --self-test` / `build.sh --self-test` / actionlint / ci.yml の grep | PASS | 全部 rc=0 |
| 6.1 | **終了の条件: PR の CI の `install-windows` が緑** | `[x]` | **未達**: `gh pr list --head feat/st58-update-apply-rollback` → `[]`・`gh run list --branch …` → 空・`git ls-remote origin feat/st58-…` → 空（R2・R10） |
| 7.1 | `cargo test -p ashiato-update-check st58_asset_`（≥3）/ 材料 ≥15 件 | PASS | rc=0・4 本 / rc=0 |
| 8.1 | `… st58_pc_`（≥12）/ 全体 / `check-no-time-server.sh`（+`--self-test`）/ clippy | PASS | rc=0・19 本 / rc=0 / rc=0・rc=0 / rc=0 |
| 8.2 | clippy `--tests`（windows-gnu）/ `grep -q "\-lt 19"` | PASS | rc=0 / rc=0 |
| 8.2 | **終了の条件: PR の CI の `collector-windows-runtime` が緑** | `[x]` | **未達**（6.1 と同じ。R10） |
| 9.1 | gradle `*St58*`（≥22）/ `check-android-backup-rules.sh` | PASS | rc=0・38 本 / rc=0 |
| 9.2 | `scripts/quiet-run emu -- tools/android-st58-replace.sh` / `--self-test` | PASS | rc=0・`OK (1 test)` 7 段（187 秒）/ rc=0 |
| 10.1 | 文書の grep | PASS | rc=0 |
| 11.1 | `check_scenarios.py` / fmt+clippy+`cargo test --workspace` / 検査 6 本 | PASS | rc=0 / rc=0 / rc=0 |

**一致**: 書かれた検証コマンドはすべて rc=0 で、申告と同じ。**不一致**: 6.1・8.2 の終了の条件（PR の CI）は満たされていない。
ST01 と同じく、ずれはコマンドの rc ではなく「主張の階層」と「テストの階層」の違いから出た。部品ごとの試験はすべて緑だが、**部品をつないだ所（インストーラ → サーバの口、インストーラ → 更新の記録 → 画面）が一度も一緒に走っていない**。

---

## R1. インストーラがサーバの `record-rollback` と `replay` を、サーバが受け付けない引数で呼ぶ。戻しは前の版のサーバを起こす前に止まり、通った更新もサーバの自動起動を「無効」のまま残して終わる
- 成果物: crates/installer/src/commands.rs / crates/installer/src/main.rs / crates/server/src/lib.rs
- 根拠:
  - 実行: `cargo build -p ashiato-server` の後、インストーラが組む形のまま本物の実行ファイルを呼んだ。
    `ashiato-server update-history record-rollback --pending-out <file>` → `Error: --new-version が必要です` **rc=1**。
    `ashiato-server update-history replay --pending <file>` → `Error: 引数が多すぎます` **rc=1**。
  - 組み立て: `commands.rs:1121-1131`（`record-rollback --pending-out` だけを渡す）・`commands.rs:1137-1146`（`replay --pending <file>`）。サーバの口は `lib.rs:475-497` で `--new-version` `--previous-version` `--kind` `--pending-out` をこの順に求め、`lib.rs:514-519` で `replay <file>` の 1 引数だけを受ける。`grep -rn -- '--new-version\|--previous-version\|"--kind"\|--failed-version' crates/installer` は **0 件**で、ほかの組み立て関数も無い。
  - 帰結（コードを読んだもの）:
    - 戻しは `main.rs:1428`（`record_rollback(...)?`）で Err になり、`main.rs:1431` の `start_previous_services` に届かない。サーバのサービスは `disabled` のまま止まり、印は段 `restored` で残る。`rollback` の口と、段 `restored` からの走らせ直し（`finish_restored`。`main.rs:1485` の `record_rollback(...)?`）も同じ命令を通るので抜けられない。
    - 通った更新は `main.rs:1294`（`replay` の `run_with_env(...)?`）で Err になる。サーバのサービスを `auto` に戻す `main.rs:1307` の前で止まり、`update_install` が `rollback()` を呼ぶ。段 `listening` なので `finish_update`（`main.rs:1667`）に入り、同じ `replay` でまた落ちる。結果、**どの更新も rc≠0 で終わり、サーバは動いているが開始の種類は `disabled` のまま**（次の再起動で起きない）。更新の記録に `applied` は書かれない。
  - 素通りした理由: インストーラ側の試験 `st58_record_rollback_reads_pending_exit_five_without_stdout_json`（`main.rs:2603-2630`）の偽サーバは引数を見ずに `$4` へ書いて `exit 5` する。インストーラの組む形を本物の口に一度も当てていない。サーバ側の試験（`st58_tests.rs`）は `record_rollback` / `replay_pending` の関数を直接呼び、CLI の口を通らない。Windows の通し（R2）はまだ一度も走っていない。
- kind: technical
- 提案: 引数の形をどちらかに揃える（インストーラが `--new-version` `--previous-version` `--kind`（落ちた版・SQLSTATE も）を渡す / `replay` の形を合わせる）。インストーラの `SetupCommand` を**本物の `ashiato-server` の CLI の解析に通す**試験を 1 本足す（Linux で走る。DB に繋ぐ前の引数の検査で rc を見れば足りる）。

## R2. Windows の通し `update-apply-install.sh` の判定は、誰も出さない文字列を待っている。Windows の CI で走らせても緑にならず、3 本目の Scenario で止まる
- 成果物: tools/release/update-apply-install.sh / crates/installer/src/commands.rs
- 根拠:
  - 判定（`assert_scenario_result`。`update-apply-install.sh:187-286`）が grep する文字列について、`crates tools .github` で出す側を探した（`grep -rlF`。この台本は除く）。`rollback=ok`・`failed_db=1`・`queued-record=accepted`・`updater=running`・`backup-retained=1`・`backups-retained=1`・`failed-db-cleaned=1`・`channel-isolated=ok`・`login=ok`・`settings-before=`・`program-version=`・`update-check=disabled`・`rollback=skipped-after-listen`・`rollback=no-stage`・`rollback-history`・`server-disabled` の **16 個すべてで、出す側が 0 件**だった。台本自身が trace に書くのは `healthz=`・`installed-version=`・`sc` と `icacls` の出力・`update-stage=absent`・`previous=present` とインストーラの出力だけ（`update-apply-install.sh:334-345`）。
  - 実行: 台本の関数を読み込み、`run_scenario` が実際に作る形の trace（`healthz=200`・`installed-version=0.4.0`・`STATE: 4 RUNNING`・`START_TYPE: 2`・`update-stage=absent`・`previous=present`・戻しの文）を `assert_scenario_result` に渡した。
    並びの 1 本目（`新しい版のインストーラで…新しい版のサーバが起きる`。版を `0.5.0` にした trace）→ rc=0。2 本目（`更新の移行の間、サーバは待ち受けない`。`STATE: 1 STOPPED`・`START_TYPE: 4`・`migrating` の trace）→ rc=0。
    **3 本目**（`更新しても、秘密と設定と画面の合言葉は変わらない`）→ `NG: 設定の指紋の比較が無い` **rc=1**。`run_windows_check` は並びの順に走り `die` で止まる（`:353-378`）ので、通しはここで止まり、残りの 31 本は走らない。
    同じ trace で `更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる` → `NG: 戻しの完了を確認できない` rc=1。
    `戻した後、更新の間に溜まった未送信が入る` → rc=1。`戻した後、切っていない更新係は動いている` → rc=1。`更新が通ると、移行の前の退避は直前の 1 つだけになる` → rc=1。`サーバの切った更新の確認は、更新でも戻しでも切ったまま` → rc=1。
  - ACL の判定 `assert_update_acl`（`:88-114`）は `NT SERVICE\ashiato-db` の読みを求める。インストーラが付けるのはサーバと**更新係**のアカウント（`commands.rs:1156-1157`）。実行: インストーラが付ける形（Administrators と SYSTEM の `M`・`ashiato-server` と `ashiato-updater` の `RX`）を `assert_update_acl` に渡した → `NG: 更新の印の ACL が必要な主体を満たさない` **rc=1**。本物の `icacls` が 1 行目に置く `C:\ProgramData\ashiato\update BUILTIN\Administrators:(…)` の形を渡すと、`NG: Administrators と SYSTEM 以外に更新の印の書き権限がある: C` rc=1（パスの `C:` を主体と読む）。self-test の偽の `icacls` はパスを付けずに `ashiato-db` を出すので通る（`:391-395`・`:418`）。
  - （読んだもの・未実行。Linux では走らせられない）最後の `windows_sc ashiato-server qc`（`:374`）は `sc.exe` の引数の順が逆に見える（`sc qc <名前>` が正しい形）。偽の `sc.exe` は引数を見ないので self-test は通る。
  - `gh pr list` は `[]`、`gh run list --branch feat/st58-update-apply-rollback` は空。`install-windows` は一度も走っていない。
- kind: technical
- 提案: 判定が要る事実（戻したか・退避の数・落ちた DB の有無・更新係の状態・合言葉でのログイン）は、台本が Windows の実際の状態（`sc query ashiato-updater`・`dir backups\pre-migrate`・`psql -l`・`/api/login`）から自分で取って trace に書く。インストーラの自己申告の文字列に頼らない。そのうえで PR の CI で `install-windows` を一度通す。

## R3. 同じ台本の「しない」の判定 3 本は、何も観測しなくても通る。self-test は Scenario の判定を 1 度も呼ばない
- 成果物: tools/release/update-apply-install.sh / crates/installer/src/main.rs
- 根拠:
  - 実行（R2 と同じ trace）: `書きかけの退避は戻さない` → **rc=0**。`退避の前に落ちた回は、前の回の退避を戻さない` → **rc=0**。`戻しは DB の管理者の役割を使わない` → **rc=0**。前の 2 本が見るのは `rc≠0` と `! grep -Eq 'rollback=ok|failed_db=1'`（`:228`・`:280`）。R2 のとおり `rollback=ok` は誰も出さないので、**rc≠0 で終わりさえすれば、実際に戻してしまっても通る**。
  - 管理者の役割の判定 `! grep -Fq 'role=ashiato '`（`:224`）が見る文字列は、インストーラの記録では `format!("role=ashiato_owner result=… {}", command.line())` の**固定の文字列**（`main.rs:1539`・`main.rs:1555`・`main.rs:1645`）。どの役割で繋いだかとは関係なく `ashiato_owner` と書くので、判定は役割を観測していない。
  - `grep -c assert_scenario_result tools/release/update-apply-install.sh` → **2**（定義と `run_scenario` の中の呼び出しだけ）。self-test（`:380-451`）は `assert_count`・`assert_output_contains`・`assert_update_acl` とフックの表しか試さない。実行して `OK: update-apply-install の自己試験` rc=0。34 本の判定はどれも、わざと壊した入力に当てられていない。
- kind: technical
- 提案: 「しない」の判定は、「した」ときに必ず出る観測（`ashiato_failed_*` の DB が増えた・退避の dump の更新時刻が変わった）を台本が自分で取り、その不在を見る。役割は接続の URL の利用者名か `pg_stat_activity` から取る。self-test に、各 Scenario の「通る trace」と「壊した trace」を 1 組ずつ置いて `assert_scenario_result` に通す。

## R4. 移行が落ちて戻した回の理由が、spec と画面の試験の想定と違う。落ちた版の欄に前のアプリの版が入り、SQLSTATE は出ず、`db_ahead` と系統違いも「戻した・移行が途中で終わった」になる
- 成果物: crates/installer/src/main.rs / web/e2e/st58-update-apply.spec.ts / specs/distribution/spec.md
- 根拠:
  - 実行: `main.rs` の末尾に一時の試験を足し（実行後に `git checkout` で戻した。`git status` と `git diff` が前後で同じ）、`update_failure_reason(e, "0.4.0")` を呼んだ。`e` はインストーラが作る形 `"移行に失敗した: <migrate の誤りの文>"`。
    - `…DB に自分の知らない版がある（自分より新しい DB）: 999912312359_x` → `rolled_back kind=aborted failed_version="0.4.0" sqlstate=None`
    - `…DB の系統が違う: db=dev self=official` → `rolled_back kind=aborted failed_version="0.4.0" sqlstate=None`
    - `…マイグレーション 202611152200_x の適用に失敗: … (SQLSTATE 23505)` → `rolled_back kind=aborted failed_version="0.4.0" sqlstate=None`
  - 原因: `main.rs:1132-1146`。誤りの文の部分一致（`"PostgreSQL"`・`"サーバが起きない"`・`"退避"`）で種別を決め、`failed_version` に `installed`（前のアプリの版）を入れ、`sqlstate` はいつも `None`。`migrate_exit_code`（`main.rs:1020`）は印に書くが、理由を決めるときには読まない。
  - spec: `specs/distribution/spec.md:428`（「移行が落ちたなら落ちた版の名前と SQLSTATE」）・`:459`（窓と更新の記録に「落ちた版の名前・`23505`・「制約に反した」」）・`:466`（DB に知らない版がある回は「入りませんでした」で、記録の最後の行は `not_applied`・種別 `db_ahead`）・`:775`・`:781`（帯）。画面の e2e は `/api/update-status` を `page.route` で差し替え、`failed_version: "V1"`・`sqlstate: "23505"`・`not_applied`/`db_ahead` を与えている（`st58-update-apply.spec.ts:5,14,17`）。**インストーラが実際に作る行はこの形にならない**。e2e は画面の文の組み立てだけを見ていて、Scenario の「戻した更新の理由が出る」は端から端まで担保されていない。
- kind: technical
- 提案: 理由は誤りの文の部分一致ではなく、`migrate` の終了コード（2・3・4 は `not_applied` と種別）と、`migrate` が出す 1 行（落ちた版の名前・SQLSTATE。spec D-01 の「種別・落ちた版の名前・SQLSTATE だけ」）から決める。インストーラの `append_outcome` が作る行を、サーバの `read_status` に通す試験を 1 本置く（R5 と合わせて）。

## R5. 段 `restored` から戻し終えた回は、理由の無い `rolled_back` の行を書く。サーバはその行を捨てるので、画面の帯に「戻しました」が出ない
- 成果物: crates/installer/src/main.rs / crates/server/src/update_status.rs
- 根拠:
  - 実行: `st58_tests.rs` に一時の試験を足し（実行後に戻した）、`outcomes.jsonl` に `{"result":"rolled_back",…,"reason":null}` の 1 行を置いて `read_status` を呼んだ → `last_update.is_some=false`（`last_update` は `null`）。
  - 書く側: `finish_restored`（`main.rs:1489-1495`）が `append_outcome(layout, "rolled_back", …, None)` と書く。読む側: `update_status.rs:287-291` の `match (&row.result[..], &row.reason)` で、`rolled_back` かつ `None` は `_ => None` に落ちる。
  - 帰結: 印が段 `restored` で残った回（戻しの途中で止まり、`rollback` の口か走らせ直しで `finish_restored` に入って戻し終えた回）は、Q4（「帯は次に更新が通るまで出す」）の知らせが画面に出ない。前の最後の行が `applied` なら、帯には何も出ない。いまは R1 で `finish_restored` が `append_outcome` に届かないので表に出ていないが、R1 を直すと出てくる。
- kind: technical
- 提案: `finish_restored` は印に残した理由（種別・落ちた版・SQLSTATE）を写して書く。印に理由を残していなければ、印を書く段で残す。

## R6. PC の入れ替えの試験の多くは、自分で書いたファイルを読み直すか、主張と逆を見ている。指紋の照合を消しても、印のある試験は 1 本も落ちない。60 秒はどの試験からも参照されていない
- 成果物: crates/collector-windows/tests/st58_pc_update.rs / crates/collector-windows/tests/runtime_windows.rs / crates/collector-windows/src/update_apply.rs
- 根拠:
  - 壊した: `update_apply.rs:216-219` の `anyhow::ensure!(actual_hex.eq_ignore_ascii_case(expected), …)` を消して常に `Ok`（実行後に戻した）。`cargo test -p ashiato-collector-windows st58_` → rc=101。落ちたのは**印の無い単体** `update_apply::tests::st58_update_digest_rejects_missing_or_wrong_fingerprint` の 1 本だけ。`// Scenario: PC は指紋の合わない配布物で入れ替えない` の試験 `st58_pc_update_rejects_wrong_digest`（`st58_pc_update.rs:191-212`）は、合わない指紋で `select_asset(...).is_some()` を確かめていて、**入れ替えないことを見ていない**（主張と逆の向き）。
  - 壊した: `WAIT_FOR_ALIVE` を 60 秒から 1 秒に、`<= 60` を `<= 1` に（`update_apply.rs:19`・`:285`）→ `st58_` は全部通った（rc=0。単体 3 本と st58_pc_ 19 本）。`grep -n '60\|WAIT_FOR_ALIVE' crates/collector-windows/tests/runtime_windows.rs` も st58 の関数に該当なし。Scenario「PC は新しい版が 1 分以内に動き出さなければ、前の版に戻して起こし直す」の 2 本の試験は、差し込みで `update-alive.json` が書かれないこと（`st58_pc_update.rs:327-336`）と、`Duration::ZERO` で待つと `false`（`runtime_windows.rs:101-116`）を見るだけ。戻すことも、前の版を起こし直すことも観測していない。
  - 同じ形（書いて読み直す・空の一時ディレクトリを見る）の試験: `st58_pc_update_refresh_is_separate_from_check_record`（`:156-165`。`押したときの取り直しは、確かめの数えを動かさない`）・`st58_pc_update_keeps_layout_for_unsent_records`（`:167-173`。`入れ替えの前に積んだ未送信を、入れ替えた後に送る`）・`st58_pc_update_keeps_new_version_records_on_rollback`（`:214-223`）・`st58_pc_update_does_not_download_before_confirmation`（`:108-113`）。どれも製品のコードを呼ばない。
  - 8.2 の `runtime_windows.rs` の st58 の 6 本（`:49-145`）は、プロセスを起こさず、窓も通知も使わない（XML の文字列・パスの組み立て・ファイルの読み書きだけ）。tasks 8.2 は「P24 を確かさ 4 に上げる」としているが、`窓を閉じた後も…動き続ける` の試験は `write_alive` で書いたファイルを読むだけ（`:134-145`）。
- kind: technical
- 提案: loopback の受け手の試験（`st58_pc_update_runs_against_loopback_with_a_real_staged_program` の形）を、指紋の合わない配布物・`y` 以外・新しい版が alive を書かない、の 3 つに広げ、`versions\` と自動起動の台本が変わらないこと、前の版の子が起きたことを観測する。60 秒は `wait_for_alive` に時計を差し込み、59 秒は待ち、61 秒は戻すことを見る。

## R7. 携帯の更新の試験は、共有の材料を読まず、いくつかの印は関係の無い試験に付いている。spec の「入れられませんでした」「許可した後に、もう一度押してください」の文は、アプリにも PC にも無い
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St58UpdateInstallTest.kt / collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateInstall.kt / crates/collector-windows/src/update_apply.rs
- 根拠:
  - `grep -rn 'update-apply-vectors' collector-android crates --include=*.kt --include=*.rs --include=*.kts` → 読み手は `crates/update-check/src/lib.rs:638` の 1 件だけ。tasks 7.1「Rust と Kotlin（Task 9.1）が全件を通す」と 9.1「材料の全件」は、Kotlin 側では満たされていない。
  - 印の先（`St58UpdateInstallTest.kt`）:
    - `// Scenario: 携帯は指紋の合わない APK でインストール画面を開かない` → `chooseAsset("0.5.0", "[]")` が `null`（`:57`）。指紋も画面も見ていない。
    - `// Scenario: 落とす要求は自分の版も端末の情報も持たない` → `chooseAsset("not-a-version", "[]")` が `null`（`:118`）。
    - `// Scenario: 押したときの取り直しは、確かめの数えを動かさない` → `chooseAsset("0.4.0", "[]")` が `null`（`:121`）。
    - `// Scenario: 押す前は配布物を落とさない` → `clean()` を呼んだ直後に置き場が空なことを見る（`:81`）。
    - `UpdateInstall.start()` / `install()`（`UpdateInstall.kt` の落とす・照らす・`PackageInstaller` に書く本体）を呼ぶ試験は無い（`grep -n 'start(\|install(' St58UpdateInstallTest.kt` に該当なし）。
  - 文言: `grep -rn '入れられませんでした' collector-android/app/src/main crates/collector-windows` → 0 件。`grep -rln '許可した後に' collector-android/app/src/main` → 0 件。spec `specs/distribution/spec.md:10`・`:31`（携帯の Scenario の THEN）・`:176`（PC の Scenario の THEN）が求める文が、製品に無い。`UpdateInstall.install()` は指紋が合わなければ `file.delete()` して黙って戻る。
  - `check_scenarios.py` の warn: `spec に無い Scenario を指す印: 携帯は更新の後に位置を除いた種別で動いていても、開くと位置の収集を再開する（St58PackageReplacedTest.kt）`。spec の名前は「更新の後に位置を除いた種別で…」で、先頭の「携帯は」が余分。
- kind: technical
- 提案: Kotlin の試験で `docs/contracts/update-apply-vectors.json` を読んで全件を通す。手元の受け手（`MockWebServer` か loopback）で `start()` を走らせ、指紋の合わない APK でセッションが作られないこと（Robolectric の `ShadowPackageInstaller`）と、行の文を見る。行と窓の「入れられませんでした（<理由>）」を作る。

## R8. 決めた値と分岐のうち 3 つは、書き換えても試験が落ちない（1 MB・終了コード 5・待ち受けの後は戻さない）
- 成果物: crates/server/src/update_status.rs / crates/server/src/lib.rs / crates/installer/src/main.rs
- 根拠（いずれも壊して走らせ、`git checkout` で戻した。前後の `git status --porcelain` と `git diff` は同じ）:
  - `MAX_OUTCOMES_JSONL_BYTES` を `1024 * 1024` から `1024` に（`update_status.rs:18`）→ `cargo test -p ashiato-server st58_` の `st58_status_` 8 本はすべて通った。試験は 1 MB を超える行を断ることしか見ず（`st58_tests.rs:263-279`）、1 MB 以下を読むことは見ていない。Scenario 名「1 MB を超える更新の記録は画面に出さない」の値は、下から固定されていない。
  - `record-rollback` の `std::process::exit(5)` を `exit(6)` に（`lib.rs:510`）→ `cargo test -p ashiato-server st58_` は 16 passed（rc=0）、`cargo test -p ashiato-setup st58_plan_` は 27 passed（rc=0）。サーバの終了コードとインストーラの `HISTORY_PENDING_WRITTEN` を突き合わせる試験は無い（R1 と同じ継ぎ目）。
  - Q5（待ち受けを開いた後は戻さない）: `rollback()` の `if stage == "listening" { return finish_update(...) }`（`main.rs:1330`）の条件を当たらない文字列にした（段 `listening` の印は `finish_update` に行かず、下の `match` で「未知の更新段です」の Err になる。DB を戻す向きには変わらない）→ `cargo test -p ashiato-setup` で落ちたのは同時に壊した保持の 2 本だけで、この分岐では何も落ちなかった。純粋な関数の側には `st58_plan_listening_never_restores`（`lib.rs:61-66`）があるが、`rollback()` は `recovery_plan` を呼ぶ前に段の文字列で分かれているので、その試験は `rollback()` の振り分けを見ていない。段 `listening` の印で走らせ直したときに残りを進めることは、どの試験も見ていない。
  - 対照: Q7（その回の 1 つだけを残す）は、`retain_backups` で全部の完了した退避を残すように壊すと `st58_plan_backup_selection_keeps_only_current_completed_dump` と `st58_plan_retention_includes_failed_databases_and_run_paths` が落ちた（固定されている）。
- kind: technical
- 提案: 1 MB ちょうど以下の行を読む試験を 1 本足す。終了コード 5 は R1 の CLI の試験で一緒に固定する。`rollback()` の段の振り分けは `recovery_plan` に寄せるか、段 `listening` の印で `rollback()` を呼んで DB を戻す命令が出ないことを見る試験を置く。

## R9. 配る組み立ての検査は、サーバの新しい試験の差し込み（`ASHIATO_SERVER_TEST_*`）を見ていない
- 成果物: tools/release/check-artifacts.sh
- 根拠:
  - 実行: 試験の組み立てのサーバ `target/debug/ashiato-server` の `strings` に入っている差し込みの名前は `ASHIATO_SERVER_TEST_EXIT_AFTER_LISTEN`・`ASHIATO_SERVER_TEST_MIGRATE_FAIL`・`ASHIATO_SERVER_TEST_NO_LISTEN` の 3 つだけ。`check-artifacts.sh` がサーバに当てる照合 `grep -Fq 'ASHIATO_SERVICE_TEST_'`（`:224`・`:309`）をこの実行ファイルに当てると **rc=1**（見つからず、通る）。
  - tasks の Global Constraints は「`tools/release/check-artifacts.sh` が差し込みの名前の不在を見る —— 新しい名前を一覧に足す」。足されたのは PC の `ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE`（`:244`）だけ。いまはサーバの差し込みが `#[cfg(ashiato_test_build)]` の中にある（`lib.rs:417`・`:4795`・`:4802`、`migrate_backup.rs:118`、`migration_ledger.rs:729`）ので、配る組み立てには入らない。（`lib.rs:4802` の `EXIT_AFTER_LISTEN` も `#[cfg(ashiato_test_build)]` の下にあることを表示して確かめた）。ただし cfg を 1 か所外しても、この検査は気づかない。
- kind: technical
- 提案: サーバの照合に `ASHIATO_SERVER_TEST_` を足し、self-test に植え付けを 1 つ足す。

## R10. 前提 P20 は「Windows CI の実物で確認」を根拠に確かさ 4 になっているが、その通しは一度も走っておらず、走っても緑にならない形になっている
- 成果物: openspec/changes/st58-update-apply-rollback/premises.md / openspec/changes/st58-update-apply-rollback/tasks.md
- 根拠:
  - `premises.md:36` の P20（戻しを DB の管理者なしで行える）は、根拠の列に「`tools/release/update-apply-install.sh` の Windows CI 通し」、判定に「行える（文書と台本の上、Windows CI の実物で確認）」と書き、確かさは **4**。tasks 6.1（`tasks.md:89`）が「P20 を確かさ 4 に上げる」と求めたもの。
  - `gh pr list --head feat/st58-update-apply-rollback --state all` → `[]`、`gh run list --branch feat/st58-update-apply-rollback` → 出力なし、`git ls-remote origin feat/st58-update-apply-rollback` → 出力なし。push 前の段なので CI が無いのは流れどおりだが、P20 の「Windows CI の実物で確認」は起きていない。R2 のとおり、通しは走っても 3 本目で止まる。R1 のとおり、戻しは `record-rollback` で止まる。
  - 対照: P24（`premises.md:40`）は「確かさ 2。Task 8.2 の Windows の実行時の試験で 4 に上げる」のまま残っていて、未確認として正しく書かれている。
- kind: technical
- 提案: P20 の確かさを 2〜3（文書と台本の上）に戻し、R1〜R3 を直した後の PR の CI で `install-windows` が緑になってから 4 に上げる。6.1・8.2 の「終了の条件: PR の CI」は、その結果を証跡に写すまで満たされていないものとして扱う。

---

## 手ごとの記録

- **手 1（固定値の再計算）: 該当なし。** 確かめた範囲: `st58_pc_update.rs`・`St58UpdateInstallTest.kt`・`st58_tests.rs`・`update_plan.rs`・`update-check/src/lib.rs` の 64 桁の 16 進の直書きを grep した。実装の出力を写した指紋は無かった（PC は `sha2::Sha256::digest`、Kotlin は `MessageDigest` で試験の中で計算して比べる。材料の指紋は形だけを見る値）。
- **手 2（ガードを壊す）:** R2・R3（Windows の通しの判定）・R9（配る組み立ての検査）。ほかに確かめて問題の無かったもの: `.down.sql` を使い捨ての DB（`st58_verify_down`。後で消した）で試した。`rolled_back` の行があると `ERROR: rolled_back の履歴があるため移行を戻せない` で rc=3、無ければ rc=0。`check-no-time-server.sh --self-test` は名前を引数に取る植え付けで落ちる形を持つ（rc=0）。ただし `.down.sql` の止まり方を試す試験は repo に無い（`grep -rln update_history_rolled_back crates tools` は 0 件）。
- **手 3（Scenario と test）:** `check_scenarios.py` rc=0（担保あり 3054/3055。残る 1 件は ST58 の外の `[wait]`）。印の先を読んだ範囲: `st58-update-apply.spec.ts` の 7 本（R4）、`update-apply-install.sh` の 34 本（R2・R3）、`st58_pc_update.rs` の 19 本と `runtime_windows.rs` の st58 の 6 本（R6）、`St58UpdateInstallTest.kt` と `St58PackageReplacedTest.kt`（R7）、`st58_tests.rs` の status の 8 本（R8）と history の 8 本（本体を読んだ）。history は `record_rollback` / `replay_pending` を関数として直接呼ぶ（CLI を通らない。R1）。**spec の THEN との突き合わせは未了**: `st58_history_failed_pair_is_ordered` は `failed` の行の結果・前の版・今の版だけを見て、spec `:491` が求める落ちた版の名前と `23505` を見ていない。`st58_history_startup_rollback_lists_applied_versions` は空の表から 1 行を数えるだけで、spec `:508` の「最後の 2 行は試み → 戻した」と「`applied`・`nothing_to_apply` が増えない」を見ていない（読んだもの。壊しての確認はしていない）。`St58PackageReplacedInstrumentedTest.kt` と `android-st58-replace.sh` は、いまの HEAD でエミュレータの 7 段がすべて `OK (1 test)` になることだけを確かめ、本体は読んでいない。
- **手 4（本人の決定）:** Q5 の分岐は固定されていない（R8）。Q7 は固定されている（R8 の対照）。Q6 の `rolled_back` は移行と `st58_history_accepts_rolled_back` で固定されているが、インストーラから書く経路は R1 で切れている。Q4（帯に出し続ける）は R4・R5 で、インストーラが書く行が画面に届く形になっていない。Q1・Q2・Q3・Q8・Q9 は値を持たない選択で、Q8 は 9.2 のエミュレータで、Q9 は `st58_history_startup_rollback_lists_applied_versions` で観測されている（Q9 の CLI の経路は R1）。Scenario 名に入っている 60 秒と 1 MB は R6・R8。
- **手 5（`[x]` と実体）:** 上の表のとおり、書かれた検証コマンドはすべて実在し rc=0。`cargo test <名前>` 型の本数は、いずれも下限以上。6.1・8.2 の終了の条件（PR の CI）は push 前の段なので未達で、その上で P20 だけが確かめた扱いになっている（R10）。9.2 が求めた P6・P7・P26 の確かさ 4 は `premises.md:8` に書かれ、この席でもエミュレータの 7 段が通った。
- **手 6（隙間）:** 「捨てたものは復元できない」型は R1（通った更新で `applied` の行が書かれず、サーバの自動起動が無効のまま残る）と R5（戻した知らせが画面から消える）。`append_outcome`（`main.rs:1762-1786`）は書けないときに黙って捨てる（`if let Ok(...)` と `let _ = writeln!`）が、Windows の置き場の権限を変えて試していないので指摘にはしない。
- **手 7（機械が承認した訂正）: 該当なし。** `python3 scripts/plan_fix.py pending openspec/changes/st58-update-apply-rollback` → `[]`。`correction-review.md` は作っていない。

壊した試験と一時の試験は、すべて `git checkout` で戻した。最後の `git status --porcelain` は、このファイルの差分だけ。DB のコンテナは `check-immutable.sh` の後に消えていたので、`tools/db.sh up -d --wait db` と `tools/db-roles.sh` で起こし直した。
