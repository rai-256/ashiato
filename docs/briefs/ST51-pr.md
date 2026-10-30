# ST51 — 版の変わり目が、更新の履歴として残る

## deep.md に積んだ未決（A）

- なし。finish の 1 回目で返した Q7（第 3 回。更新の後に端末の時計が戻ったとき、版の変化をどう数えるか。`review/code.md` R20）は、
  本人が「今のまま（発信の時刻の順）。時計が戻ったときの 2 行の見え方を受け入れる」を選んだ（推奨と同じ側。`deep-answers-4.txt`）。
  受け入れた見え方を collection-coverage の Scenario「更新の後に端末の時計が戻ると、更新 1 回が戻りと上げの 2 行に見える」に書き、
  試験 `st51_view_clock_back_after_update` に印を付けて固定した。design の危険の行にも Q7 を書き足した。要件へ戻すものは無い

## 仮決め（B）と Ruling

- **Task の ledger の `Ruling:`: なし**（どの Task にも ledger `progress.md` が作られておらず、`Ruling:` 行は 0 件）
- design の仮決め（台帳 `provisional.jsonl`。確認バッチで一覧に出る）:
  - D1（仮）収集アプリは、すべての生存信号に、その信号を積んだ時に動いていた版を載せる
  - D2（仮）S-01 は migrate の終わりと、版の検査を通った後・待ち受けの前の起動の両方で書く
  - D3（仮）更新の履歴は `core` の追記のみの表に置き、書き出しに分類して入れる
  - D4（仮）更新の履歴の列と生存信号の版の欄は、Requirement の表で名前・型・NULL の可否を固定する
  - D5（仮）収集アプリの版の変化は、生存信号から導くビューにする（変化の行を凍結しない）
  - D16（仮）試みの行は、結果の行と同じ条件（直前の通った行の版と自分の版が違う）でだけ書く。
    finish で、起動の取引が当てる処理の錠を **共有で** 待たずに取る形に変えた（`pg_try_advisory_xact_lock_shared`。同時に起動した S-01 どうしが互いを migrate と取り違えて `migrate_running` を出さない。`review/code.md` R28）

## Task

- 全 10 Task（20 項目）を埋めた。残した Task は無い（`ci` / `human` の残りも無い）

## finish で処置した指摘（`review/code.md` R16〜R28。code-verify・security review・/code-review）

- fixed: R16（同時の 2 つの起動の書き込みで 1 行を、`record_startup` 2 つで 10 回見る）・R17（`record_startup` が書かなかった理由を返し、試験が `migrate_running` を見る）・
  R18（アプリの UPDATE・DELETE は 42501 だけを通す。試験と `check-immutable.d` の ST51 の節）・R19（欄と原文で版が違う信号は欄で数える試験）・
  R23（SQLSTATE の取り出しを `sqlstate_of` に）・R24（表の有無の分岐を `table_ready` に）・R25（全行を出す書き出しの計画を `whole_plan` に）・
  R27（`collector_version_changes.jsonl` を行の全部の列で並べる）・R28（上記 D16）
- 壊して落ちるのを確かめた: 起動の取引から更新の履歴の鍵を外す → `st51_migrate_concurrent_one_row` FAILED / `grants.sql` の REVOKE を消す → `st51_rows_cannot_be_rewritten` FAILED
- rejected: R21（検査の前に落ちた回の試みなしの `failed` は design D8 の表の 7 行目どおり）・R22（同じ印の走らせ直しは版の変化でない。D2・D16（仮）。当てた版は ST48 の版の記録が持つ）・
  R26（件数と本文を別に引くのは書き出しの全部の表と同じ形。`pg_input_is_valid` は JSON でない原文で読み口を落とさない門）
- escalated: R20 → deep.md Q7 → 答え「今のまま」で Scenario に固定（上記）
- security review は指摘なし

## 通した検証コマンド

`evidence.jsonl` の最後の記録（全部 `PASS`・終了コード 0）:

| 項目 | コマンド（要約） | rc |
|---|---|---|
| 1.1 | `tools/check-migrations.sh` / `cargo build -p ashiato-server` | 0 / 0 |
| 1.2 | `cargo test -p ashiato-server st51_form_` / `st51_rows_` / `app_role_tests`（件数の下限つき） | 0 / 0 / 0 |
| 1.3 | `tools/check-immutable.sh` / `tools/check-shared-appends.sh` | 0 / 0 |
| 2.1〜2.2 | `cargo build` / `st51_migrate_`（≥19）/ `migration_ledger_`（≥20） | 0 |
| 2.3 | `--test server_startup st51_bin_migrate_`（≥5） | 0 |
| 3.1〜3.2 | `--test server_startup st51_bin_startup_`（≥3）/ `st48_`（≥10） | 0 |
| 4.1〜4.2 | `tools/check-openapi.sh` / `jq … has("client_version")` / `st51_hb_`（≥7） | 0 |
| 5.1 | `st51_view_`（≥13） | 0 |
| 6.1〜6.2 | `st51_export_`（≥5）/ `export_`（≥130）/ `export_file_excludes_ledgers` | 0 |
| 7.1〜7.2 | `cargo test -p ashiato-collector-windows --lib st51_c02_`（≥4）/ `--lib` | 0 |
| 8.1〜8.2 | gradle `*St51HeartbeatVersionTest*`（≥3）/ `*Heartbeat*`（≥20） | 0 |
| 9.1 | `st51_coverage_`（≥2）/ `outage_`（≥40） | 0 |
| 10.1〜10.2 | `check_chain.py` / `check_scenarios.py` / `check-migrations.sh` / `check-shared-appends.sh` | 0 |

finish の直しの後に走らせ直したもの（HEAD の作業ツリー）: `cargo test -p ashiato-server st51_` 55 passed（3 本増）・`--test server_startup st51_` 8 passed・`st48_` PASS・`migration_ledger_` PASS・
`export_` 191 passed・`cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0・`cargo fmt --check` rc=0・`tools/check-immutable.sh` rc=0・
`check_scenarios.py` OK・`check-shared-appends.sh` OK・`check_chain.py` OK・`review_triage.py` OK・`provisional.py check` rc=0

Q7 の答えを写した後（finish の 2 回目）: `cargo test -p ashiato-server --lib st51_view_` 16 passed・`check_scenarios.py` OK・`check_agreed_tests.py` OK・`check_chain.py` OK・`openspec validate --strict` valid・`review_triage.py` OK

## 未処置の申し送り

- `docs/handoff/ST58.md` の ST51 の節（退避から戻した後の 2 行の書き直し・落ちた版の名前と SQLSTATE の足し方・「戻した」の値を足すときの CHECK の作り替え）は ST58 が受ける。`check_chain.py` の未回収は 0 件
- なし（deep.md Q7 は答えを Scenario に写して閉じた）
