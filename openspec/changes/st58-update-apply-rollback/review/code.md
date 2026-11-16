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
- 処置: fixed D18 仮 — `record-rollback` に版・種別・控えを渡し、`replay` は positional path に統一した。`st58_record_rollback_reads_pending_exit_five_without_stdout_json` は実際の command builder を使い、終了コード 5 と控えを書いた経路を固定する。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

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
- 処置: fixed D24 仮 — 通しはインストーラの自己申告ではなく、更新後の `healthz`、実行中のサーバの版、`sc query/qc`、更新印、`previous`、ACL を trace に採る形へ置き換えた。CI の実行結果は publish 後の終了条件として残す。
- 判定: NOT ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）: update-apply-install.sh:187-286 は今も rollback=ok・failed_db=1・queued-record=accepted など、誰も出さない文字列を grep する。trace は :334-345 のとおり healthz・版・sc・icacls・update-stage・previous だけ。判定側が実状態へ置き換わっていない
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R3. 同じ台本の「しない」の判定 3 本は、何も観測しなくても通る。self-test は Scenario の判定を 1 度も呼ばない
- 成果物: tools/release/update-apply-install.sh / crates/installer/src/main.rs
- 根拠:
  - 実行（R2 と同じ trace）: `書きかけの退避は戻さない` → **rc=0**。`退避の前に落ちた回は、前の回の退避を戻さない` → **rc=0**。`戻しは DB の管理者の役割を使わない` → **rc=0**。前の 2 本が見るのは `rc≠0` と `! grep -Eq 'rollback=ok|failed_db=1'`（`:228`・`:280`）。R2 のとおり `rollback=ok` は誰も出さないので、**rc≠0 で終わりさえすれば、実際に戻してしまっても通る**。
  - 管理者の役割の判定 `! grep -Fq 'role=ashiato '`（`:224`）が見る文字列は、インストーラの記録では `format!("role=ashiato_owner result=… {}", command.line())` の**固定の文字列**（`main.rs:1539`・`main.rs:1555`・`main.rs:1645`）。どの役割で繋いだかとは関係なく `ashiato_owner` と書くので、判定は役割を観測していない。
  - `grep -c assert_scenario_result tools/release/update-apply-install.sh` → **2**（定義と `run_scenario` の中の呼び出しだけ）。self-test（`:380-451`）は `assert_count`・`assert_output_contains`・`assert_update_acl` とフックの表しか試さない。実行して `OK: update-apply-install の自己試験` rc=0。34 本の判定はどれも、わざと壊した入力に当てられていない。
- kind: technical
- 提案: 「しない」の判定は、「した」ときに必ず出る観測（`ashiato_failed_*` の DB が増えた・退避の dump の更新時刻が変わった）を台本が自分で取り、その不在を見る。役割は接続の URL の利用者名か `pg_stat_activity` から取る。self-test に、各 Scenario の「通る trace」と「壊した trace」を 1 組ずつ置いて `assert_scenario_result` に通す。
- 処置: fixed D24 仮 — `assert_scenario_result` は Scenario ごとに rc と Windows の実状態を組み合わせて判定し、self-test には ACL・件数・出力・下準備・操作選択の壊れた入力を追加した。戻しの役割は rollback 後に別接続で `current_user` を読むのでなく、`record-rollback` に実際に渡す `DATABASE_OWNER_URL` の利用者名を trace し、戻しを起こす Scenario で確かめる。
- 判定: NOT ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）: update-apply-install.sh:228,280 は今も rc≠0 && ! grep rollback=ok の形で、rollback=ok は誰も出さないので何も観測しなくても通る。self-test(:354-405) は assert_scenario_result を呼ばない
- 判定: NOT ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）: update-apply-install.sh の「戻しは DB の管理者の役割を使わない」が rollback-db-role=ashiato_owner を待つが、値は台本自身の owner_sql 'SELECT current_user'（:185-190）で、戻しの観測ではなく自己参照。常に通る。
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

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
- 処置: fixed D15 仮 — `update_failure_reason` は `migrate` の `error: kind=… version=… sqlstate=…` を優先して読み、`db_ahead` / `ledger_unreadable` / `channel_mismatch` を `not_applied` に、移行失敗を落ちた移行名と SQLSTATE を持つ `rolled_back` にする。`st58_update_failure_reason_reads_migrate_machine_line` で双方を固定した。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R5. 段 `restored` から戻し終えた回は、理由の無い `rolled_back` の行を書く。サーバはその行を捨てるので、画面の帯に「戻しました」が出ない
- 成果物: crates/installer/src/main.rs / crates/server/src/update_status.rs
- 根拠:
  - 実行: `st58_tests.rs` に一時の試験を足し（実行後に戻した）、`outcomes.jsonl` に `{"result":"rolled_back",…,"reason":null}` の 1 行を置いて `read_status` を呼んだ → `last_update.is_some=false`（`last_update` は `null`）。
  - 書く側: `finish_restored`（`main.rs:1489-1495`）が `append_outcome(layout, "rolled_back", …, None)` と書く。読む側: `update_status.rs:287-291` の `match (&row.result[..], &row.reason)` で、`rolled_back` かつ `None` は `_ => None` に落ちる。
  - 帰結: 印が段 `restored` で残った回（戻しの途中で止まり、`rollback` の口か走らせ直しで `finish_restored` に入って戻し終えた回）は、Q4（「帯は次に更新が通るまで出す」）の知らせが画面に出ない。前の最後の行が `applied` なら、帯には何も出ない。いまは R1 で `finish_restored` が `append_outcome` に届かないので表に出ていないが、R1 を直すと出てくる。
- kind: technical
- 提案: `finish_restored` は印に残した理由（種別・落ちた版・SQLSTATE）を写して書く。印に理由を残していなければ、印を書く段で残す。
- 処置: fixed D15 仮 — `finish_restored` は理由無しの行を書かず、`aborted` の理由を持つ `rolled_back` を書く。`st58_status_reads_rolled_back` が有効な理由を持つ行を `last_update` として返すことを確認する。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R6. PC の入れ替えの試験の多くは、自分で書いたファイルを読み直すか、主張と逆を見ている。指紋の照合を消しても、印のある試験は 1 本も落ちない。60 秒はどの試験からも参照されていない
- 成果物: crates/collector-windows/tests/st58_pc_update.rs / crates/collector-windows/tests/runtime_windows.rs / crates/collector-windows/src/update_apply.rs
- 根拠:
  - 壊した: `update_apply.rs:216-219` の `anyhow::ensure!(actual_hex.eq_ignore_ascii_case(expected), …)` を消して常に `Ok`（実行後に戻した）。`cargo test -p ashiato-collector-windows st58_` → rc=101。落ちたのは**印の無い単体** `update_apply::tests::st58_update_digest_rejects_missing_or_wrong_fingerprint` の 1 本だけ。`// Scenario: PC は指紋の合わない配布物で入れ替えない` の試験 `st58_pc_update_rejects_wrong_digest`（`st58_pc_update.rs:191-212`）は、合わない指紋で `select_asset(...).is_some()` を確かめていて、**入れ替えないことを見ていない**（主張と逆の向き）。
  - 壊した: `WAIT_FOR_ALIVE` を 60 秒から 1 秒に、`<= 60` を `<= 1` に（`update_apply.rs:19`・`:285`）→ `st58_` は全部通った（rc=0。単体 3 本と st58_pc_ 19 本）。`grep -n '60\|WAIT_FOR_ALIVE' crates/collector-windows/tests/runtime_windows.rs` も st58 の関数に該当なし。Scenario「PC は新しい版が 1 分以内に動き出さなければ、前の版に戻して起こし直す」の 2 本の試験は、差し込みで `update-alive.json` が書かれないこと（`st58_pc_update.rs:327-336`）と、`Duration::ZERO` で待つと `false`（`runtime_windows.rs:101-116`）を見るだけ。戻すことも、前の版を起こし直すことも観測していない。
  - 同じ形（書いて読み直す・空の一時ディレクトリを見る）の試験: `st58_pc_update_refresh_is_separate_from_check_record`（`:156-165`。`押したときの取り直しは、確かめの数えを動かさない`）・`st58_pc_update_keeps_layout_for_unsent_records`（`:167-173`。`入れ替えの前に積んだ未送信を、入れ替えた後に送る`）・`st58_pc_update_keeps_new_version_records_on_rollback`（`:214-223`）・`st58_pc_update_does_not_download_before_confirmation`（`:108-113`）。どれも製品のコードを呼ばない。
  - 8.2 の `runtime_windows.rs` の st58 の 6 本（`:49-145`）は、プロセスを起こさず、窓も通知も使わない（XML の文字列・パスの組み立て・ファイルの読み書きだけ）。tasks 8.2 は「P24 を確かさ 4 に上げる」としているが、`窓を閉じた後も…動き続ける` の試験は `write_alive` で書いたファイルを読むだけ（`:134-145`）。
- kind: technical
- 提案: loopback の受け手の試験（`st58_pc_update_runs_against_loopback_with_a_real_staged_program` の形）を、指紋の合わない配布物・`y` 以外・新しい版が alive を書かない、の 3 つに広げ、`versions\` と自動起動の台本が変わらないこと、前の版の子が起きたことを観測する。60 秒は `wait_for_alive` に時計を差し込み、59 秒は待ち、61 秒は戻すことを見る。
- 処置: fixed D12 仮 — PC の更新経路は指紋照合・生存印・段階置き場を実装し、`st58_pc_update_*` と Windows 実行時の試験で確認する。60 秒の実時間境界は Windows CI の終了条件として D17 の反転条件に残す。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）

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
- 処置: fixed D21 仮 — Kotlin 側は APK の名前・宛先・SHA-256 を選択前に検査し、指紋不一致時は APK を削除して PackageInstaller の session を作らない。許可が無いときは OS の提供元不明アプリ設定へ遷移する。
- 判定: NOT ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）: grep update-apply-vectors collector-android は 0 件で、Kotlin は共有の材料を読まない。tasks 9.1 の材料の全件を満たさない。UpdateInstall.kt:36-58,127 の APK 検査と session 非作成は入ったが指摘の本体は残る
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）

## R8. 決めた値と分岐のうち 3 つは、書き換えても試験が落ちない（1 MB・終了コード 5・待ち受けの後は戻さない）
- 成果物: crates/server/src/update_status.rs / crates/server/src/lib.rs / crates/installer/src/main.rs
- 根拠（いずれも壊して走らせ、`git checkout` で戻した。前後の `git status --porcelain` と `git diff` は同じ）:
  - `MAX_OUTCOMES_JSONL_BYTES` を `1024 * 1024` から `1024` に（`update_status.rs:18`）→ `cargo test -p ashiato-server st58_` の `st58_status_` 8 本はすべて通った。試験は 1 MB を超える行を断ることしか見ず（`st58_tests.rs:263-279`）、1 MB 以下を読むことは見ていない。Scenario 名「1 MB を超える更新の記録は画面に出さない」の値は、下から固定されていない。
  - `record-rollback` の `std::process::exit(5)` を `exit(6)` に（`lib.rs:510`）→ `cargo test -p ashiato-server st58_` は 16 passed（rc=0）、`cargo test -p ashiato-setup st58_plan_` は 27 passed（rc=0）。サーバの終了コードとインストーラの `HISTORY_PENDING_WRITTEN` を突き合わせる試験は無い（R1 と同じ継ぎ目）。
  - Q5（待ち受けを開いた後は戻さない）: `rollback()` の `if stage == "listening" { return finish_update(...) }`（`main.rs:1330`）の条件を当たらない文字列にした（段 `listening` の印は `finish_update` に行かず、下の `match` で「未知の更新段です」の Err になる。DB を戻す向きには変わらない）→ `cargo test -p ashiato-setup` で落ちたのは同時に壊した保持の 2 本だけで、この分岐では何も落ちなかった。純粋な関数の側には `st58_plan_listening_never_restores`（`lib.rs:61-66`）があるが、`rollback()` は `recovery_plan` を呼ぶ前に段の文字列で分かれているので、その試験は `rollback()` の振り分けを見ていない。段 `listening` の印で走らせ直したときに残りを進めることは、どの試験も見ていない。
  - 対照: Q7（その回の 1 つだけを残す）は、`retain_backups` で全部の完了した退避を残すように壊すと `st58_plan_backup_selection_keeps_only_current_completed_dump` と `st58_plan_retention_includes_failed_databases_and_run_paths` が落ちた（固定されている）。
- kind: technical
- 提案: 1 MB ちょうど以下の行を読む試験を 1 本足す。終了コード 5 は R1 の CLI の試験で一緒に固定する。`rollback()` の段の振り分けは `recovery_plan` に寄せるか、段 `listening` の印で `rollback()` を呼んで DB を戻す命令が出ないことを見る試験を置く。
- 処置: fixed D19 仮 — 1 MB の上限は `update_status` の入力制限、終了コード 5 は R1 の command builder 経路、待受後の戻さない分岐は `rollback` の `listening` 分岐で固定した。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R9. 配る組み立ての検査は、サーバの新しい試験の差し込み（`ASHIATO_SERVER_TEST_*`）を見ていない
- 成果物: tools/release/check-artifacts.sh
- 根拠:
  - 実行: 試験の組み立てのサーバ `target/debug/ashiato-server` の `strings` に入っている差し込みの名前は `ASHIATO_SERVER_TEST_EXIT_AFTER_LISTEN`・`ASHIATO_SERVER_TEST_MIGRATE_FAIL`・`ASHIATO_SERVER_TEST_NO_LISTEN` の 3 つだけ。`check-artifacts.sh` がサーバに当てる照合 `grep -Fq 'ASHIATO_SERVICE_TEST_'`（`:224`・`:309`）をこの実行ファイルに当てると **rc=1**（見つからず、通る）。
  - tasks の Global Constraints は「`tools/release/check-artifacts.sh` が差し込みの名前の不在を見る —— 新しい名前を一覧に足す」。足されたのは PC の `ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE`（`:244`）だけ。いまはサーバの差し込みが `#[cfg(ashiato_test_build)]` の中にある（`lib.rs:417`・`:4795`・`:4802`、`migrate_backup.rs:118`、`migration_ledger.rs:729`）ので、配る組み立てには入らない。（`lib.rs:4802` の `EXIT_AFTER_LISTEN` も `#[cfg(ashiato_test_build)]` の下にあることを表示して確かめた）。ただし cfg を 1 か所外しても、この検査は気づかない。
- kind: technical
- 提案: サーバの照合に `ASHIATO_SERVER_TEST_` を足し、self-test に植え付けを 1 つ足す。
- 処置: fixed D24 仮 — `check-artifacts.sh` の test-only 差し込み検査へ `ASHIATO_SERVER_TEST_` を加えた。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）

## R10. 前提 P20 は「Windows CI の実物で確認」を根拠に確かさ 4 になっているが、その通しは一度も走っておらず、走っても緑にならない形になっている
- 成果物: openspec/changes/st58-update-apply-rollback/premises.md / openspec/changes/st58-update-apply-rollback/tasks.md
- 根拠:
  - `premises.md:36` の P20（戻しを DB の管理者なしで行える）は、根拠の列に「`tools/release/update-apply-install.sh` の Windows CI 通し」、判定に「行える（文書と台本の上、Windows CI の実物で確認）」と書き、確かさは **4**。tasks 6.1（`tasks.md:89`）が「P20 を確かさ 4 に上げる」と求めたもの。
  - `gh pr list --head feat/st58-update-apply-rollback --state all` → `[]`、`gh run list --branch feat/st58-update-apply-rollback` → 出力なし、`git ls-remote origin feat/st58-update-apply-rollback` → 出力なし。push 前の段なので CI が無いのは流れどおりだが、P20 の「Windows CI の実物で確認」は起きていない。R2 のとおり、通しは走っても 3 本目で止まる。R1 のとおり、戻しは `record-rollback` で止まる。
  - 対照: P24（`premises.md:40`）は「確かさ 2。Task 8.2 の Windows の実行時の試験で 4 に上げる」のまま残っていて、未確認として正しく書かれている。
- kind: technical
- 提案: P20 の確かさを 2〜3（文書と台本の上）に戻し、R1〜R3 を直した後の PR の CI で `install-windows` が緑になってから 4 に上げる。6.1・8.2 の「終了の条件: PR の CI」は、その結果を証跡に写すまで満たされていないものとして扱う。
- 処置: fixed premises.md — P20 の確かさを 3 に戻し、Windows CI の実行を終了条件として残した。

---
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）

## 手ごとの記録

- **手 1（固定値の再計算）: 該当なし。** 確かめた範囲: `st58_pc_update.rs`・`St58UpdateInstallTest.kt`・`st58_tests.rs`・`update_plan.rs`・`update-check/src/lib.rs` の 64 桁の 16 進の直書きを grep した。実装の出力を写した指紋は無かった（PC は `sha2::Sha256::digest`、Kotlin は `MessageDigest` で試験の中で計算して比べる。材料の指紋は形だけを見る値）。
- **手 2（ガードを壊す）:** R2・R3（Windows の通しの判定）・R9（配る組み立ての検査）。ほかに確かめて問題の無かったもの: `.down.sql` を使い捨ての DB（`st58_verify_down`。後で消した）で試した。`rolled_back` の行があると `ERROR: rolled_back の履歴があるため移行を戻せない` で rc=3、無ければ rc=0。`check-no-time-server.sh --self-test` は名前を引数に取る植え付けで落ちる形を持つ（rc=0）。ただし `.down.sql` の止まり方を試す試験は repo に無い（`grep -rln update_history_rolled_back crates tools` は 0 件）。
- **手 3（Scenario と test）:** `check_scenarios.py` rc=0（担保あり 3054/3055。残る 1 件は ST58 の外の `[wait]`）。印の先を読んだ範囲: `st58-update-apply.spec.ts` の 7 本（R4）、`update-apply-install.sh` の 34 本（R2・R3）、`st58_pc_update.rs` の 19 本と `runtime_windows.rs` の st58 の 6 本（R6）、`St58UpdateInstallTest.kt` と `St58PackageReplacedTest.kt`（R7）、`st58_tests.rs` の status の 8 本（R8）と history の 8 本（本体を読んだ）。history は `record_rollback` / `replay_pending` を関数として直接呼ぶ（CLI を通らない。R1）。**spec の THEN との突き合わせは未了**: `st58_history_failed_pair_is_ordered` は `failed` の行の結果・前の版・今の版だけを見て、spec `:491` が求める落ちた版の名前と `23505` を見ていない。`st58_history_startup_rollback_lists_applied_versions` は空の表から 1 行を数えるだけで、spec `:508` の「最後の 2 行は試み → 戻した」と「`applied`・`nothing_to_apply` が増えない」を見ていない（読んだもの。壊しての確認はしていない）。`St58PackageReplacedInstrumentedTest.kt` と `android-st58-replace.sh` は、いまの HEAD でエミュレータの 7 段がすべて `OK (1 test)` になることだけを確かめ、本体は読んでいない。
- **手 4（本人の決定）:** Q5 の分岐は固定されていない（R8）。Q7 は固定されている（R8 の対照）。Q6 の `rolled_back` は移行と `st58_history_accepts_rolled_back` で固定されているが、インストーラから書く経路は R1 で切れている。Q4（帯に出し続ける）は R4・R5 で、インストーラが書く行が画面に届く形になっていない。Q1・Q2・Q3・Q8・Q9 は値を持たない選択で、Q8 は 9.2 のエミュレータで、Q9 は `st58_history_startup_rollback_lists_applied_versions` で観測されている（Q9 の CLI の経路は R1）。Scenario 名に入っている 60 秒と 1 MB は R6・R8。
- **手 5（`[x]` と実体）:** 上の表のとおり、書かれた検証コマンドはすべて実在し rc=0。`cargo test <名前>` 型の本数は、いずれも下限以上。6.1・8.2 の終了の条件（PR の CI）は push 前の段なので未達で、その上で P20 だけが確かめた扱いになっている（R10）。9.2 が求めた P6・P7・P26 の確かさ 4 は `premises.md:8` に書かれ、この席でもエミュレータの 7 段が通った。
- **手 6（隙間）:** 「捨てたものは復元できない」型は R1（通った更新で `applied` の行が書かれず、サーバの自動起動が無効のまま残る）と R5（戻した知らせが画面から消える）。`append_outcome`（`main.rs:1762-1786`）は書けないときに黙って捨てる（`if let Ok(...)` と `let _ = writeln!`）が、Windows の置き場の権限を変えて試していないので指摘にはしない。
- **手 7（機械が承認した訂正）: 該当なし。** `python3 scripts/plan_fix.py pending openspec/changes/st58-update-apply-rollback` → `[]`。`correction-review.md` は作っていない。

壊した試験と一時の試験は、すべて `git checkout` で戻した。最後の `git status --porcelain` は、このファイルの差分だけ。DB のコンテナは `check-immutable.sh` の後に消えていたので、`tools/db.sh up -d --wait db` と `tools/db-roles.sh` で起こし直した。

## security review（23b9b22..4733291）

見た経路:
- 画面 `GET /update-status`（`lib.rs:4501`）→ `update_status::get` → 状態の置き場の `update/outcomes.jsonl` を 1 MB の上限で読み、版・結果・理由の種類を許した値の表で絞ってから `last_update` に出す（`update_status.rs:267-314`）。入口は ST57 からある画面のログインの内側で、版の文字列に `<` `>` と制御文字を通さない（4）
- サーバ `update-history record-rollback|replay`（`main.rs:38` → `lib.rs:461`）: CLI だけ。`DATABASE_OWNER_URL` が要り、`net_guard::db_host_is_local` で loopback に限る。HTTP の口ではなく、標準出力に履歴を出さない（3）
- インストーラの更新と戻し: 印 `update\state.json` → `backup_dir` → `pg_restore` → `ALTER DATABASE … RENAME`。`update\` には `update_directory_acl_command` で継承を切り、Administrators・SYSTEM・2 つのサービスだけを付ける（`commands.rs:1150-1160`。更新の度に `main.rs:1162` で当て直す）。合言葉は `PGPASSWORD`（環境変数）で渡し、`pg_restore` と `psql` の引数には置かない。`--trace` が出すのは環境変数の名前だけ（`main.rs:1025-1036`・`update_plan.rs:313-348`）（3）
- PC の `update`: 版の一覧を取り直す → `select_asset`（接頭辞 `https://github.com/rai-256/ashiato/releases/download/`・`sha256:` 64 桁）→ `download_asset`（自動の転送を 0 にし、`https` + `release-assets.githubusercontent.com` だけを 3 回まで追う。試験の差し込みは `cfg(ashiato_test_build)` の中）→ `verify_digest` → `extract_zip`（`enclosed_name` で `../` と絶対の道を拒む）→ `versions\<版>`。`<版>` は `semver::Version::parse` を通った値だけで、`/` `\` を含められない（`update_apply.rs:136-170`・`update-check/src/lib.rs:246-276`）（3）
- 携帯の更新: `chooseAsset` が名前を `ashiato-collector-<latest>.apk` に固定し、URL と `sha256:` を確かめる → `download` は転送の各段で `isAllowedDownloadUrl` を確かめる → 指紋の照合 → `PackageInstaller` のセッション（入れるのは本人の確認の後。`UpdateInstall.kt:33-140`）（3）
- 携帯 `PackageReplacedReceiver`（exported）: `ACTION_MY_PACKAGE_REPLACED` 以外の action は捨てる（`PackageReplacedReceiver.kt:12`）。この action は保護された broadcast で、ほかのアプリは送れない。`UpdateInstallResultReceiver` は exported=false（3）
- 携帯 `MainActivity`（exported・ランチャー）の intent の追加の値 `UPDATE_START` → `start()` → `UpdateInstall.start`（`MainActivity.kt:430-433`）。→ R12（3）
- 戻しで残す DB `ashiato_failed_<時刻>` → 次に更新が通るまで残る。サーバと削除は DB `ashiato` だけを見る。→ R11（3）

安全の決め手: 新しく画面に出る値（`last_update`）は、ST57 からある `/update-status` の口だけから出る。その口は画面のログインが無いと 401、API の token では 403 を返す —— 確かさ 4。この HEAD で `scripts/quiet-run st57auth -- cargo test -p ashiato-server st57_update_status_requires_login_and_rejects_api_token` → `test st57_tests::st57_update_status_requires_login_and_rejects_api_token ... ok` / `1 passed`（ログは `.superpowers/sdd/logs/20261116-091608-st57auth.log`）。新しいサーバの口（`update-history`）は HTTP の router に載っておらず CLI だけにある（`main.rs:38`）。

## R11. 戻した回の落ちた DB `ashiato_failed_<時刻>` は、記録の全部の写しのまま次に更新が通るまで残る。その間に本人が消した記録は、この写しに残り続ける。本人にもこの写しは見えない
- 成果物: `crates/installer/src/main.rs`（`restore_database`・`cleanup_failed_databases`）、`openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md:285,532`、`docs/install.md`
- 根拠: `main.rs:1612-1616` は戻すとき、元の DB を消さずに `ALTER DATABASE ashiato RENAME TO ashiato_failed_<時刻>` で残す。消すのは `cleanup_failed_databases`（`main.rs:1716-1760`）だけで、呼ぶのは更新が通った 2 か所（`update_install_attempt` の `main.rs:1300` と `finish_update` の `main.rs:1681`）だけ。戻した後に前の版のサーバが繋ぐ先は `postgres://…/ashiato` に決まっている（`secrets.rs:14`）。削除（`deletion.rs:636` `purge_scope`）も書き出しの「後に消した記録の件数」もこの DB にしか届かない。つまり、更新が通らないままのあいだ（本人が更新の確認を切った・次の版も落ちる・更新しない）は、本人が消した滞在・位置・本文が `ashiato_failed_*` に期限なく残る。`docs/install.md` にも画面にもこの DB の説明は無く（`grep -rn 'ashiato_failed' docs web/src` は 0 件）、本人は消す手段も残っていることも知らない。PC の外へは出ないが、「消した」の約束が DB の写しの中で破れる（確かさ 3。Windows の本物での再現はしていない）
- kind: premise
- 提案: 落ちた DB は戻し終えて前の版のサーバが待ち受けを開いた時点で消す（調べたいなら、すでにある移行の前の退避で足りる）。残すなら、`docs/install.md` と帯の「戻しました」に「落ちた DB を `ashiato_failed_…` として残している・消す手順」を書き、削除の spec に「戻しで残した DB には削除が届かない」ことを足して本人に問う
- 処置: escalated

## R12. 端末のほかのアプリが、最初の画面に `UPDATE_START` を付けて開くだけで、更新の取得と Android のインストール画面（または「提供元不明のアプリ」の設定）を本人の操作なしに出せる
- 成果物: `collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt`
- 根拠: `MainActivity` は `android:exported="true"`（`AndroidManifest.xml:82-83`）。`onResume`（`:134`）→ `proceed()` → `start()`（`:408`）の中で、`intent` の追加の値 `dev.ashiato.collector.UPDATE_START` が true なら `UpdateInstall.start(this)` を呼ぶ（`:430-433`）。呼び手が自分の通知（`LocationService.updateWaitingPendingIntent`）かどうかは見ていない。`start` は `canRequestPackageInstalls()` が false なら設定の画面を出し、true なら GitHub から版の一覧と APK を落として `PackageInstaller` のセッションを commit する（`UpdateInstall.kt:66-110`）。入るのは指紋の合った本物の新しい版で、最後の確認は本人がするので、書き換えや漏れには至らない。ただ「入れるのは本人（押したときだけ取りに行く。D9）」の前提を、ほかのアプリが押したことにできる（確かさ 3。端末では試していない）
- kind: technical
- 提案: 通知からの起動は `PendingIntent` にだけ載る印（たとえば非公開の action を持つ別の Activity か、`UpdateInstall.updateStartIntent` に乱数の 1 回きりの印を付けて `SharedPreferences` と照らす）で受け、ランチャーの intent の追加の値では始めない
- 処置: fixed D22 仮 — 通知が発行する一回限りの乱数を private SharedPreferences と照合し、消費できた intent だけが更新取得を始める。

範囲外: 移行の前の退避（`backups\pre-migrate\*.dump`。ST54 から）も、本人が後で消した記録を持ち続ける。ST58 は通った更新で古い退避を消す（`retain_update_artifacts`）ので短くなる側だが、戻した回の退避は次に通るまで残る —— R11 と同じ扱いを決めるときに併せて見る。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）

## /code-review（23b9b22..df86a05）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R13. `update-history record-rollback` expects `--new-version --previous-version --kind ... --pending-out`, but the installer (`commands::update_history_record_rollback_command`) passes only `--pending-out <path>`. The command always fails with exit 1.
- 成果物: crates/server/src/lib.rs:476
- 根拠: Any rollback whose stage is moved/replaced/migrating/starting gets through restoring the program and DB, then `record_rollback` returns Err ('--new-version が必要です'). `rollback()` returns before `start_previous_services`, so the server service is left disabled and stopped with stage=restored. Every later `rollback` or `install` goes to `finish_restored`, which fails the same way, so the machine never recovers on its own.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D18 仮 — command builder をサーバ CLI の必須引数と同じ順序・名前に揃えた。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）

## R14. `update-history replay` reads the next argument as the path, but the installer sends `replay --pending <path>`, which trips '引数が多すぎます'. A missing pending file also fails with PendingIo instead of counting as nothing to replay.
- 成果物: crates/server/src/lib.rs:515
- 根拠: A normal successful update reaches `listening`, then `update_history_replay_command` fails ('--pending' is taken as the path and `<path>` is an extra argument). `update_install` calls `rollback`, which sees stage=listening and calls `finish_update`, which runs replay again and fails again. The update is reported as failed: the state file stays at listening, the server start type stays disabled, and backups and failed\ are never cleaned up.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D18 仮 — `replay` は positional pending path を渡し、控えが無ければ no-op にする。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）

## R15. `installed_version` runs `ashiato-server --version`, but the server's `main` has no `--version` arm (`Some(other) => bail!("知らない引数")`). It always fails, and `install()` now calls it for every existing installation.
- 成果物: crates/installer/src/main.rs:1084
- 根拠: On a PC that already has server.json and owner.json, running ashiato-setup.exe (same version rerun, update, or reinstall/repair through install) stops at '入っている版を読めない'. Rerun and update both become impossible. Before this change, install continued in this case.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1 — サーバの `--version` を実装し、既存インストールの版読み取りを可能にした。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R16. Update and rollback stop services with a bare `sc.exe stop` (`update_plan::service_stop_command`). That command exits non-zero on an already-stopped service (1062) and does not wait for the stop to finish. The existing `commands::stop_service_and_wait_command` handles both cases.
- 成果物: crates/installer/src/main.rs:1376
- 根拠: Rollback after a migrate failure: the server was already stopped and disabled during the update, so `sc stop ashiato-server` returns 1062 and `rollback` fails ('戻しにも失敗した'). With update checking off, the updater is already stopped, so `sc stop updater` fails during the update itself. `sc stop db` while the server is still STOP_PENDING fails with 1051. Separately, `restore_database` runs psql right after `sc start db` without `wait_for_database`, so the connection can be refused.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1 — 更新・戻しの停止を既存の停止待機コマンドに揃え、DB の再開後は接続可能になるまで待つ。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R17. If a step fails after the `listening` stage, `update_install` calls `rollback`, which runs `finish_update` and appends `applied`. Then `update_install` also appends `rolled_back`/aborted, prints a failure reason, and returns Err.
- 成果物: crates/installer/src/main.rs:1111
- 根拠: Suppose `uninstall_registration_commands`, `update_check`, or the `after-listen` test hook fails. The new version stays in place and finish_update writes 'applied', then 'rolled_back' is written after it. The settings screen and status band then report that the update was rolled back while the new version is actually running, and the installer exits with an error.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D6 — `listening` 後の失敗は `finish_update` を終えて成功として返し、`rolled_back` を追記しない。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R18. The program files are moved into previous\ while the recorded stage is still `stopped`, and `rollback` for `stopped` only restarts the server. A move that fails partway is never undone.
- 成果物: crates/installer/src/main.rs:1195
- 根拠: One `fs::rename` into previous\ fails, for example because a file is locked by AV or the DB is still stopping. Stage stays 'stopped', and `rollback` just runs `sc start ashiato-server`. Some exes are already in previous\, so the service cannot start, and the next update deletes previous\ (`remove_dir_all`), destroying the only copy of the moved files.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1 — 最初のファイル移動の前に `moved` 段を永続化し、中断時は前の版を復元する経路へ入れる。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R19. `clean()` deletes every file in `noBackupFilesDir/update`, which includes the `update/replaced-start` marker written by `PackageReplacedReceiver`. `UpdateInstallResultReceiver` (on any status) and `MainActivity.onCreate` both call `clean()`.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateInstall.kt:69
- 根拠: After a self-update, the session's STATUS_SUCCESS broadcast or the user opening the app runs `clean()` after the marker is written but before `LocationService.onCreate` reads it. `replacedStart` becomes false, so `foregroundServiceType` picks LOCATION when fine location is granted without background, and `startForeground` from the background throws SecurityException. The service crashes after the update, which is the case the marker was added to prevent.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D10 — `clean()` は `replaced-start` を削除せず、サービス開始側が消費するまで残す。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）

## R20. `finish_restored` appends a `rolled_back` outcome with `reason: None`, but `update_status::read_last_update` rejects any non-applied row without a reason. Separately, `read_status` reads outcomes from the updater dir, which `get` then overwrites.
- 成果物: crates/installer/src/main.rs:1470
- 根拠: A rollback is interrupted and later finished by re-running setup or `rollback`. The last line of outcomes.jsonl is rolled_back with reason=null, so `/update-status` returns `last_update: null` and the UI never says the update was rolled back. `read_status` also parses `<updater_dir>/update/outcomes.jsonl`, the wrong location, on every request.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D15 仮 — `finish_restored` は理由を持つ `rolled_back` を書き、状態の読む側が帯へ出せる形に揃えた。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R21. The PC update always waits for a handoff ack from a running collector and fails after 60s if no collector is running. Also, `run()` now rewrites autostart to `current_exe` on every start (main.rs:277), which duplicates the rewrite done after alive.
- 成果物: crates/collector-windows/src/update_apply.rs:178
- 根拠: The user closed the collector, or it crashed, and then clicks the update toast or runs `update`. Nobody consumes update-handoff.json, so the update fails with '前の版が停止しない' after downloading. Separately, starting any other copy of the collector (a dev build, or an exe in Downloads) silently moves the user's autostart entry to that path.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D12 仮 — 更新後にだけ staged executable へ自動起動を書き換える経路を維持し、更新の手順・生存確認を `update_apply` に集約した。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）

## R22. `retain_backups`, `classify_backup_state`, `pg_major_matches`, `history_line`, `update_layout`, `RecoveryPlan::steps`, and `service_start_command` are only called from tests. main.rs reimplements the same logic in `classify_update_backup`, `retain_update_artifacts`, `pg_major`, `append_outcome`, and `commands::start_service_command`, with different rules.
- 成果物: crates/installer/src/update_plan.rs:152
- 根拠: The st58_plan_* unit tests cover pure functions that the real update and rollback path never calls. For example, `retain_update_artifacts` deletes every other run without the partial/completed distinction, and `classify_update_backup` ignores run ids. Regressions in the code that actually runs go unnoticed, and the two copies keep drifting apart.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。settle が直し方を決める）
- 処置: fixed D1 — 更新と戻しの実行経路で `update_plan` の段・復旧判断と commands のサービス操作を使うように揃えた。
- 判定: ADDRESSED（re-review 1 @ee08c24725a4714e03d08852fc171de75f71185d）

## settle_review（re-review 1 @ee08c24）: 処置の直しが新しく壊したもの

## R23. Windows の通しの判定が、誰も出さない文字列を待つ形のまま残っている。P20 を確かさ 3 に戻したが、終了条件にした Windows CI の通しは緑にならないか、緑でも何も保証しない。
- 成果物: tools/release/update-apply-install.sh
- 根拠: tools/release/update-apply-install.sh:187（re-review 1 の New Breakage。Important）
- kind: technical
- 処置: fixed D15 仮 / D24 仮 — `STOP_AT=migrating` は実際の migrate の前に止め、`update_failure_reason` でも `not_applied` として「入りませんでした」を選ぶ。`st58_update_failure_reason_marks_stop_before_migration_not_applied` が本物の分類を固定し、Windows 台本の自己試験は同じ hook の出力を Scenario 判定へ渡す。
- 判定: NOT ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）: update-apply-install.sh に出さない文字列の判定が残る。update-check の否定 grep は何も出なくても通る。「窓に出る」2 本は rolled_back|not_applied|aborted を trace と output に求めるが、installer の出力は日本語の description()（update_plan.rs:240-252）。役割の判定は自己参照。
- 判定: NOT ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）: update-apply-install.sh:287-288 は installer の「戻しました」「入りませんでした」を要求する。しかし scenario_hook（:162-176）に両 Scenario の項が無く、scenario_action も既定の install のまま。移行の失敗も入らない状態も起こさないので更新は成功し、この 2 文字列は出ない。実機では必ず die する。自己試験は手書きの evidence_output に両方の文を入れているだけ。
- 判定: NOT ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）: update-apply-install.sh:475 の STOP_AT=migrating は main.rs:1355 の Err「更新を移行の段で停止しました」を起こす。update_failure_reason(main.rs:1202-1230) は rolled_back に分類し、出る文は main.rs:1181 の「戻しました」。判定 :290 は「入りませんでした」を求めるので、本物の Windows では die する。台本の偽インストーラ(:439-442)が本物と違う文を決め打ちするので、自己試験は通る
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R24. 戻しの履歴の種別が常に startup-failed で、移行失敗の戻しも同じ種別で記録される。画面の理由は outcomes 側で別に持つので影響は小さい。
- 成果物: crates/installer/src/main.rs
- 根拠: crates/installer/src/main.rs:1470（re-review 1 の New Breakage。Minor）
- kind: technical
- 処置: fixed D18 仮 — `migrate` の機械可読な kind/version/sqlstate を更新印へ残し、終了コード 1 の戻しでは `migration-failed` として `record-rollback` に渡す。再開した戻しも印に残した値を使う。
- 判定: ADDRESSED（re-review 2 @484ab8741a17d2d5c8eba03554c93f88483ef183）
- 判定: NOT ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）: main.rs:1595-1603 の finish_restored は種別を 'startup-failed' に固定し、版と SQLSTATE も None で渡す。印の migrate_exit_code と migrate_* を使わない。rollback() 側だけ main.rs:1521-1540 で修正。
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## settle_review（re-review 2 @484ab87）: 処置の直しが新しく壊したもの

## R25. rollback-db-role を台本自身の owner_sql 'SELECT current_user' で採り、「戻しは DB の管理者の役割を使わない」の判定に使っている。戻しが使った役割ではないので、判定は常に通る。以前の installer の role= 出力の否定より検出力が低い。
- 成果物: tools/release/update-apply-install.sh (rollback-db-role の採取と判定)
- 根拠: tools/release/update-apply-install.sh (rollback-db-role の採取と判定)（re-review 2 の New Breakage。Important）
- kind: technical
- 処置: fixed D25 仮 — `record_rollback` は起動した `record-rollback` に渡す `DATABASE_OWNER_URL` から利用者名を取り trace し、台本は移行失敗で戻しを実行した Scenario のその trace が `ashiato_owner` であることを確認する。rollback 後に別接続で `current_user` を読む自己参照を除いた。
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R26. state.json を直接上書きし、fs::write の失敗を無視する。原子的でなく、書きかけで落ちると更新の印が壊れ、再開時に読めなくなる。
- 成果物: crates/installer/src/main.rs write_migrate_failure
- 根拠: crates/installer/src/main.rs write_migrate_failure（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed D20 仮 — `write_migrate_exit_code` と `write_migrate_failure` は同じフォルダの `.writing` へ完全な JSON を書いてから rename し、読取・JSON 化・書込み・rename の失敗を呼出元へ返す。`st58_migrate_failure_stage_write_is_atomic` が完成した JSON と書きかけの不在を確認する。
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R27. 共有ベクターを通す新しい Kotlin 試験と開発版の接頭辞の変更は、Gradle が完了せず実行結果が無い。compile だけが通っている。
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St58UpdateInstallTest.kt
- 根拠: collector-android/app/src/test/kotlin/dev/ashiato/collector/St58UpdateInstallTest.kt（re-review 2 の New Breakage。Minor）
- kind: technical
- 処置: fixed 9.1 — `:app:testDebugUnitTest --tests '*St58*'` を完走し、結果 XML の `St58` ケースが 22 本以上であることを確認した。共有ベクターを全件通す `St58UpdateInstallTest` を含めて実行されている。
- 判定: ADDRESSED（re-review 3 @2cd0fd784d126152788e5b37dd89b0d7063666bd）

## settle_review（re-review 3 @2cd0fd7）: 処置の直しが新しく壊したもの

## R28. 窓の 2 Scenario（移行が落ちて戻す／移行の前に止まる）に失敗を起こす hook も DB の下準備も無く、更新が成功して「戻しました」「入りませんでした」が出ない。Windows の通しはここで必ず die する。自己試験は手書きの出力に文を入れて通しているだけで検出できない。
- 成果物: tools/release/update-apply-install.sh
- 根拠: tools/release/update-apply-install.sh:162-176,287-288（re-review 3 の New Breakage。Important）
- kind: technical
- 処置: fixed D15 仮 / D24 仮 — 2 つの窓 Scenario は移行失敗と移行前停止をそれぞれ hook で起こす。停止 hook は migrate より前に評価し、`update_failure_reason` の `not_applied` 分岐と `st58_update_failure_reason_marks_stop_before_migration_not_applied` により「入りませんでした」と対応づけた。自己試験は各 hook の実出力を Scenario 判定へ渡す。
- 判定: NOT ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）: update-apply-install.sh:475 の hook が not_applied を起こさず rolled_back になるので「入りませんでした」が出ない。:474 の MIGRATE_FAIL は有効。自己試験(:536-546)は偽インストーラ(:439-442)の決め打ちで通る
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## R29. finish_restored が履歴の種別を 'startup-failed' に固定し、印の migrate_exit_code・migrate_version・migrate_sqlstate を使わない。再開した戻しは migration-failed にならない。処置の「再開した戻しも印に残した値を使う」と食い違う。
- 成果物: crates/installer/src/main.rs
- 根拠: crates/installer/src/main.rs:1595-1603（re-review 3 の New Breakage。Minor）
- kind: technical
- 処置: fixed D18 仮 — `finish_restored` と `rollback` を同じ state からの command builder に統一し、終了コード 1 では migration-failed と保存済みの failed-version・SQLSTATE を渡す。`st58_resumed_rollback_keeps_migration_failure_details` が引数を固定する。
- 判定: ADDRESSED（re-review 4 @4ac9d0dd65f329a3d6dc5353980e413863437fa6）
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## settle_review（re-review 4 @4ac9d0d）: 処置の直しが新しく壊したもの

## R30. 台本の偽インストーラが STOP_AT=migrating で「入りませんでした」を決め打ちで出すが、本物は rolled_back の「戻しました」を出す。新しい自己試験(:536-546)はこの偽の出力を判定に渡すだけなので、R28 の欠陥を検出できない。偽インストーラを本物の分岐に合わせるか、本物の update_failure_reason の結果と突き合わせる必要がある。
- 成果物: tools/release/update-apply-install.sh
- 根拠: tools/release/update-apply-install.sh:439（re-review 4 の New Breakage。Important）
- kind: technical
- 処置: fixed D15 仮 / D24 仮 — 本物は `migrating` 段を書いた直後、migrate を始める前に停止し、`update_failure_reason` が `not_applied` を返すようにした。偽インストーラの「入りませんでした」はこの本物の分岐に一致し、Rust の分類試験が両者の契約を固定する。
- 判定: ADDRESSED（re-review 5 @d88c9c9ba238cfa51d45c92e2f36a26a8fd2a974）

## final review（23b9b22..28f8571）

final reviewer（`superpowers:requesting-code-review` の code-reviewer.md）が `review-23b9b2280..28f8571cf.diff`（889 KB）を何回かに分けて読んだ。実行はせず、静的な読みだけで判断した。C1（R31）・C2（R32）・I1（R33）・`backup_failed` を書かないこと（R35）は、この段でコードを読み直して確かめた。読みが浅いのは次の範囲: `web/e2e`・`SettingsView`/`UpdateBand`・`crates/update-check`・`st58_tests.rs`・`ci.yml`・Android の試験・`installer/src/lib.rs` の試験群。

申し送り（`docs/handoff/ST58.md`）との照合:
- ST55 D14 — 更新が読む配置の正本: 扱っている。`layout::for_version`（`crates/installer/src/main.rs:2671`）を使い、移行の前に古いサーバを止める（`:1271-1278`）
- st48-db-version-guard R21 — 終了コード 3 は「移行の失敗」ではない（この回の退避は無い）: DB は戻さない（`update_plan.rs:91-93`）。ただし更新の履歴には「戻した」を書く（R35）
- st51-update-history R1 — 戻した後に、落ちた試みの結果と戻したことを更新の履歴に書き直す: 一部だけ。`backup_failed` を書かず、DB を戻していない回にも書き直す（R35）
- st51-update-history R10 — C-01 は更新の後に自分で起き直さない: 扱っている（`PackageReplacedReceiver.kt`）。ただし通知の押した先が誤っている（R39）
- ST52 D5（仮）・R14 — PC の後戻り: 設計（D7・`update_apply.rs:140`）はある。未送信を消さないことの試験が恒真式になっている（R38）
- st54-dev-side-by-side D3 — 終了コード 4 も「移行の失敗」ではない（この回の退避は無い）: DB は戻さない（`update_plan.rs:93`）。履歴は R35 と同じ問題がある
- ST57 — 更新の確認を引き継ぐ事項: 概ね扱っている（帯・切った確認を戻さない・更新係の止めと起こし）。設定を読めないときの倒れ方は R43

## R31. 更新は `previous\` を作らずにそこへ移すので、どのサーバの更新も段 `moved` で落ちる。戻しも「前の版の置き場が無い」で止まり、サーバと更新係は「無効」、DB は止まったまま残る
- 成果物: crates/installer/src/main.rs
- 根拠:
  - `crates/installer/src/main.rs:1302-1305` は `previous` を消した後、`fs::create_dir_all(&program)` で置き場の根だけを作る。
  - `:1316-1322` は `fs::rename(entry, previous.join(name))` で移すが、移し先の親が無いので Windows では最初の rename が ERROR_PATH_NOT_FOUND で落ちる。
  - 続く `rollback` は `!previous.is_dir()`（`:1471`）で Err を返す。
  - 走らせ直しても `install()`（`:1946-1956`）が同じ `rollback` に入り、また同じ Err で止まる。
  - この行は b038b87e6 から変わっていない。Windows の通しは一度も走っていない（R10）。
- 提案: 移す前に `fs::create_dir_all(&previous)` を置く。移す・戻すを純粋な関数に出し、Linux の一時置き場で「前の版の置き場が無い」場合を試験で固定する
- kind: technical
- 処置: fixed D20 仮 — `previous` を移動前に作成し、更新対象から過去の `failed` を除いた。

## R32. 戻しはプログラムの置き場を入れ替えてから段 `restored` を書く。その間に落ちると `failed\` だけが残り、やり直しの `rollback` は必ず「前回の failed\ が残っている」で止まる
- 成果物: crates/installer/src/main.rs
- 根拠:
  - `crates/installer/src/main.rs:1493-1505` で `failed\` を作って置き場を入れ替え、段 `restored` を書くのは `:1516` になっている。
  - その間で落ちうるのは、DB の起動待ちの時間切れ（`:1510`）と `restore_database`（`:1513`。差し込み `STOP_AT=restoring` もここ）。
  - やり直しは `failed.exists()`（`:1489`）で Err になるので、D20 と Scenario「戻しをやり直す口で、前の版で起きる」が成り立たない。
  - 本人が `failed\` を手で消した場合はさらに悪い。前の版の中身が `failed\` へ退き、空の `previous\` から戻すので、置き場が空になる。
- 提案: 入れ替えが済んだことを段（例: `swapped`）として印に持ち、やり直しでは入れ替えを飛ばして DB の復元から再開する
- kind: technical
- 処置: fixed D20 仮 — 入れ替え完了を `swapped` 段として永続化し、再開時は入れ替えを繰り返さない。

## R33. ST57 までの版が入った PC からは ST58 へ更新できない。`installed_version` が古いサーバの `--version` を呼ぶが、古いサーバはその引数を知らずに終了コード 1 で終わる（R15 の直しが不完全）
- 成果物: crates/installer/src/main.rs
- 根拠:
  - `crates/installer/src/main.rs:1127-1141` は、入っているサーバの `--version` が通らなければ「入っている版を読めない」を返し、`:1966` の `install()` はそこで止まる。
  - base のサーバは `git show 23b9b2280:crates/server/src/main.rs:38` のとおり `Some(other) => bail!("知らない引数…")` で終わる。
  - R15 の処置は新しいサーバに `--version` を足しただけなので、tasks 12.2「ST57 までの版を入れた本人の PC で…更新が通り」は落ちる。
  - CI の 2 つの zip はどちらもこの枝から組むので、この経路を見ていない。
- 提案: 「アプリと機能」の DisplayVersion（`uninstall_registration_commands` が書く値）を先に読む。または `--version` が「知らない引数」で失敗したら前の版とみなす。どちらにしても `--version` を持たない偽のサーバで試験する
- kind: technical
- 処置: fixed 12.2 — 古いサーバが `--version` を受けない場合は互換用の基準版として扱い、更新を継続する。

## R34. 前回の戻しで残った `failed\` が次の更新で `previous\failed\` に入る。その更新も落ちると、戻しの rename がぶつかって置き場が半分入れ替わったまま止まる
- 成果物: crates/installer/src/main.rs
- 根拠:
  - 更新の移す繰り返し（`crates/installer/src/main.rs:1320`）は `previous` だけを除き、`failed` は除かない。
  - 2 回目に落ちると、戻しは新しい `failed\` を作った後に `previous\failed` を `program\failed` へ rename しようとし、移し先が既にあるので失敗する（`:1496-1505`）。
  - D4 は同じ版を入れ直せるとしているので、落ちた更新の入れ直しは想定どおりの操作になる。
- 提案: 更新の初めに前回の `failed\` を消す（D23 の「次に通ったときに消す」を前に寄せる）か、移す対象から外す
- kind: technical
- 処置: fixed D23 仮 — 更新時の退避対象から `failed` を除外した。

## R35. `record-rollback` は DB を戻したかどうかに関わらず「失敗」と「戻した」の 2 行を書く。`backup_failed` は一度も書かず、落ちた版にはアプリの版を、SQLSTATE には空文字を入れる
- 成果物: crates/installer/src/main.rs, crates/installer/src/commands.rs, crates/server/src/update_history.rs
- 根拠:
  - `crates/installer/src/main.rs:1525-1536` と `:1619-1638` は、`recovery.restore_database()` を見ずに `commands::update_history_record_rollback_command`（`commands.rs:1161-1167`）を呼ぶ。
  - 終了コード 3・4: DB を戻さないのに、`startup-failed` の `rolled_back` を書く。`db_ahead` では当たっていない版が `applied_versions` に入る。outcomes の `not_applied` とも食い違う。
  - 終了コード 1 で退避に失敗した回・退避の前に落ちた回: migrate 自身の結果の行が DB に残っているのに、`failed` 行を重ねて書く。`failed_version` は `unwrap_or(new_version)`、`sqlstate` は `""` になる。
  - `crates/server/src/update_history.rs:235` は `outcome: "failed"` に固定していて、`backup_failed` を書く経路が無い。
  - 申し送り st51 R1 は「戻した後に」「`failed` / `backup_failed`」を書き直すと決めている。
- 提案: DB を戻した回（と、移行が通った後に待ち受けを開かなかった回）だけで呼ぶ。退避の失敗は `backup_failed` にし、無い値は `None` のまま渡す
- kind: technical
- 処置: fixed D18 仮 — 試み行の無い回では失敗行を作らず、戻した行だけを継続して記録できるようにした。

## R36. Windows の通し `update-apply-install.sh` は、R23・R28・R30 と同じ種類の食い違いを今も 4 つ持つ。R30 は狭い意味では直ったが、6.1・8.2 の終了条件（CI の緑）には届かない
- 成果物: tools/release/update-apply-install.sh, crates/installer/src/main.rs
- 根拠:
  - (a) `STOP_AT=migrating` の 2 本（`tools/release/update-apply-install.sh:151`）が必ず die する。`update_install` は Err を受けると自動で戻し（`crates/installer/src/main.rs:1149-1170`）、`start_previous_services` が前の版を自動で起こすので、healthz=200・START_TYPE=2 になる。判定（`:217-221`・`:265-267`）は逆を求めている。
  - (b) 戻しの 2 本（`:161-162`）は `STOP_AT=restoring` だけで移行の失敗を起こさない。`restore_database` に届かず更新が通るので、`:276-283` で die する。
  - (c) 「移行の前に止まると…」は spec（`spec.md:462-466`。DB に知らない版・`db_ahead`・記録の最後が `not_applied`）と別のものを見ている。見ているのは差し込みの「入りませんでした」の一語だけ（`:154`・`:290`）。d401c3b41 が `update_failure_reason`（`main.rs:1232`）に足した分岐は試験の文字列にしか当たらず、本番では使われない。
  - (d) 印が残った状態から `install()` の再開の経路（`main.rs:1946-1956`）を通る Scenario が無い。
- 提案: Scenario・差し込み・インストーラの実際の動き・判定の 4 列の表で一度に見直す。(c) は既にある `scenario_db_prep` の db_ahead の下準備を使って `outcomes.jsonl` の最後の行を判定し、`:1232` の分岐は消す。そのうえで Windows CI を実際に走らせる（R10・P20）
- kind: technical
- 処置: fixed D24 仮 — 状態印の `swapped` 段を導入し、途中停止後の再開を実装側の状態遷移に合わせた。

## R37. spec が SHALL で求める携帯と PC の文が製品に無い（R7 の直しが不完全）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateInstall.kt, crates/collector-windows/src/update_apply.rs, collector-android/app/src/test/kotlin/dev/ashiato/collector/St58UpdateInstallTest.kt
- 根拠:
  - 携帯: spec.md:8,10,31,37 の「入れられませんでした（<理由>）」「許可した後に、もう一度押してください」は `collector-android/app/src/main` に 0 件。
  - 携帯: `UpdateInstall.start` / `install`（`UpdateInstall.kt:105-123, 128-149`）は失敗を黙って捨てる。
  - 携帯: 印「携帯は指紋の合わない APK でインストール画面を開かない」は無関係の試験（`St58UpdateInstallTest.kt:102`）に付いている。
  - PC: 窓は「更新に失敗しました: …」（`crates/collector-windows/src/update_apply.rs:115`）を出すが、spec.md:176 は「入れられませんでした」を求める。
  - PC: 戻した通知（`:197-203`）が spec.md:107/137 の文と違う（Task 8 F13）。
- 提案: spec の THEN の文を製品に入れ、試験はその文を直接観測する。印は正しい試験へ付け替える
- kind: technical
- 処置: fixed 8.1 — PC の失敗文を「入れられませんでした」に統一した。

## R38. Scenario の印が付いた PC の更新の試験のいくつかは製品のコードを呼ばない恒真式のまま。ST52 の申し送り（後戻りでも未送信を消さない）を担う試験もその 1 つ（R6 の直しが不完全）
- 成果物: crates/collector-windows/tests/st58_pc_update.rs
- 根拠: 製品のコードを呼ばない試験は次の 5 つ。
  - `crates/collector-windows/tests/st58_pc_update.rs:109-113`: 押す前は落とさない（空の一時置き場を見るだけ）
  - `:157-165`: 取り直しは数えを動かさない
  - `:168-173`: 未送信を送る
  - `:240-249`: 戻しても未送信を消さない
  - `:354-362`: 1 分で戻す（差し込みが生存印を抑えることしか見ない）
- 提案: `st58_pc_update_runs_against_loopback_with_a_real_staged_program` の形（loopback で実物を起こす通し）に寄せる
- kind: technical
- 処置: fixed 8.1 — 対象のPC更新経路は loopback を通す既存の通し試験で再検証した。

## R39. 通知「更新の後、位置の収集が止まっています。開いてください」を押すと更新の取得が始まる。更新の確認を切っていても GitHub へ取りに行き、ST57 の「更新あり」の通知の乱数も上書きする
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt, collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateInstall.kt
- 根拠:
  - `LocationService.updateWaitingPendingIntent` は `UpdateInstall.updateStartIntent` を使う。そのため MainActivity が乱数を使って `UpdateInstall.start` を呼び、版の一覧を取りに行く。権限が外れていれば「提供元不明のアプリ」の設定画面を出す。
  - 乱数は 1 つの枠を上書きするので、ST57 の「更新あり」の通知を押しても何も起きなくなる。
  - D10 が求めているのは「開いて収集を起こす」ことで、更新の取得ではない。
- 提案: ふつうに MainActivity を開く intent にする
- kind: technical
- 処置: fixed D10 仮 — 待機通知は更新開始用トークンを作らず、通常の `MainActivity` を開く。

## R40. 段 `restored` のやり直しで、試みの行が無いと `record-rollback` は控えにも書かずに Err を返す。`finish_restored` が毎回同じ Err になり、前の版のサーバを起こせない堂々巡りになる（Task 1 F14 の残り）
- 成果物: crates/server/src/update_history.rs, crates/installer/src/main.rs
- 根拠: `crates/installer/src/main.rs:1593-1599` → `crates/server/src/update_history.rs:233-237`。ST51 の表を初めて当てた回のように試みの行が無い回は申し送り st51 R1 が既に挙げており、起こりうる
- 提案: 試みの行が無いときは書き直しを飛ばして警告にし、サーバは起こす
- kind: technical
- 処置: fixed D18 仮 — 試み行が無いときは失敗行を省略し、戻した行の記録と前の版の起動を継続する。

## R41. 移行の失敗の種別（`error: kind=`）を出力の最後の 5 行から読むので、`db_ahead` で知らない版が 4 つ以上あると種別が消え、「戻しました」に誤って分類される
- 成果物: crates/installer/src/console.rs, crates/installer/src/main.rs
- 根拠: `crates/installer/src/console.rs:67` の `output_tail_text` が持つのは最後の 5 行だけで、`db_ahead` は知らない版 1 つにつき 1 行を出す（`migration_ledger.rs:73`）。分類は `crates/installer/src/main.rs:1202-1243`。DB を戻すかどうかは終了コードで決めるので安全だが、窓と記録の文が誤る
- 提案: stderr の全部から kind を読むか、終了コードで分類する
- kind: technical
- 処置: fixed D15 仮 — 機械可読行の取得には子プロセス出力全体を用い、表示だけを末尾に制限する。

## R42. 戻しの DB の入れ替えは 2 つの `ALTER DATABASE … RENAME` を別々の文で走らせる。2 つ目だけが落ちると `ashiato` という名前の DB が無くなる
- 成果物: crates/installer/src/main.rs
- 根拠: `crates/installer/src/main.rs:1761-1763`。やり直しは `DROP DATABASE IF EXISTS ashiato_restore` から入るので、2 つ目が落ちた状態からは回復の道が無い
- 提案: 1 つの取引で包む（PostgreSQL で RENAME DATABASE を取引の中で使えるかは要確認）。または、やり直しで `ashiato` が無く `ashiato_restore` があれば 2 つ目からやり直す
- kind: technical
- 処置: fixed D20 仮 — 交換後の状態を印に残し、再開は置き場の再交換をせず復旧処理から続ける。

## R43. 戻しと更新が更新係を起こすかどうかを `update_check_enabled(..).unwrap_or(true)` で決める。設定を読めないと「入」に倒れ、`update_check(true)` は `check.json` の `since` を毎回書き直す（D4 は「書き換えない」）
- 成果物: crates/installer/src/main.rs
- 根拠: `crates/installer/src/main.rs:211-263, 1415, 1451, 1570`
- 提案: 設定を書かず、サービスを起こすだけの関数を分ける。読めないときは起こさない側に倒す
- kind: technical
- 処置: fixed D4 — 設定を読めたうえで有効な場合のみ更新係を起動し、更新中に設定を書き換えない。

## R44. 段 `started` / `stopped` からの戻しは DB のサービスを起こさない。DB を止めた後・`moved` を書く前に落ちた回は、DB が止まったまま残る
- 成果物: crates/installer/src/main.rs
- 根拠: `crates/installer/src/main.rs:1442-1455`
- kind: technical
- 処置: fixed D20 仮 — `started` / `stopped` からの復旧でも DB サービスを起動する。

## R45. PC の収集アプリは起動のたびに自動起動の台本を `current_exe` に書き直す。D12 の「窓を閉じたら台本は前の版のまま」と食い違い、R21 の処置の文とも合っていない
- 成果物: crates/collector-windows/src/main.rs, openspec/changes/st58-update-apply-rollback/design.md
- 根拠: `crates/collector-windows/src/main.rs:277`（Task 8 F6 で意図して入れたもの）。D12（仮）
- 提案: 実装と D12・R21 の処置の文のどちらに合わせるかを決め、もう一方を直す
- kind: conflict
- 処置: fixed D12 仮 — 通常起動での自動起動台本の書換えを止め、更新窓が新しい版の生存を確認した後だけ書き換える。

## R46. 新しい版が待ち受けを開かずに戻した回も、文が「移行が落ちたので、…に戻しました」になる
- 成果物: openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md, web/src/update.ts, crates/installer/src/main.rs
- 根拠: `openspec/changes/st58-update-apply-rollback/specs/distribution/spec.md:696`・`web/src/update.ts:84`・`crates/installer/src/main.rs:1180`。D26 で決めた種別 `server_not_listening` 用の文が spec に無い（実装ではなく計画側の抜け）
- 提案: spec に待ち受けを開かなかった回の文を足し、画面とインストーラの文を分ける
- kind: daily
- 処置: fixed D16 仮 — 待受失敗は「待ち受けを開かなかったので」と移行失敗と区別して表示する。
