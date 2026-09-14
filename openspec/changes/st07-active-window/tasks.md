# ST07 実装タスク — PC のアクティブウィンドウを集める

読む順: `deep.md`（**最優先。本人が決めた 8 件**）→ このファイル → `specs/desktop-collection/spec.md`
→ `design.md` → `docs/stories/ST07.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**この change は移行を 1 本も足さない**（design D2）。足す必要が出たら
**名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
**`collection-coverage` と `record-envelope` には触らない**（ST02 / ST03 の capability）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` と `tools/seed.sh` が前提。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: アプリを切り替えると 1 件増える`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、
  **印の無い Scenario を FAIL にする**。`scripts/merge_gate.sh` がこの検査を見るので、
  **印を置かないと tasks を全部チェックしても PR は止まる**
- 印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **この change が足す Scenario は 30 本**（すべて `desktop-collection`。spec-review が 7 本を足した後の数。review/code.md R12）
- 実機（Windows の実環境）でしか確かめられないものは、下の「人間の確認待ち」節に挙げる。
  **挙げたものだけが検査を通る**
- **D5 / D7 / D8 / D9 は（仮）決め。** 反転条件は `design.md` にある。
  実測で値を変えたら、その D 番号の（仮）を外すか反転条件を書き直す

## 1. 登録簿の担保

**移行は 1 本も足さない**（design D2）。`c02-window` は登録済みで、`external_id_kind` も
ST03 の移行（`202609120940_source_columns.sql`）が既に `'none'` にしている（2026-09-13 に実測）。
残すのは、**誰が倒したかに依存しない**担保 1 本だけ。

- [x] 1.1 全移行を当てた後に `c02-window` の `external_id_kind` が `'record'` ではないことを見る
  テストを 1 本置く（design D2）。**列の有無で分岐させない** —— 分岐すると「まだ列が無いから合格」で
  素通りする。検証: `cargo test c02_window_external_id_kind_is_not_record` rc=0

## 2. 送る形（契約）

- [x] 2.1 `docs/collector-contract.md` に **C-02 が送る `payload` の形**を追記する（design D1）——
  アプリ名（表示名）・実行ファイルのパス・プロセス名・ウィンドウ題名・URL・記録の種類
  （`foreground` / `idle` / `powered-off` / `excluded`）・範囲の終わり（`idle` と `powered-off` のみ）・
  **最後の入力からの経過時間**（`idle` のみ。design D9）・**除外した件数**（`excluded` のみ。design D11）。
  **`raw` は収集側が組んだ JSON を文字列のまま送る**ことを明記する。
  検証: `grep -c "c02-window" docs/collector-contract.md` が 1 以上、
  `python3 scripts/check_chain.py .` rc=0

## 3. 前景の変化を拾う

- [x] 3.1 `crates/collector-windows` に前景の読み取りを実装する。OS のイベント通知と
  1 秒間隔の見回りを併用する（design D5・**仮**）。検証:
  `cargo test --package ashiato-collector-windows foreground` rc=0
- [x] 3.2 アプリ名・ウィンドウ題名・URL のいずれかが変化したら記録を 1 件作る。
  Scenario: `アプリを切り替えると 1 件増える` / `同じアプリの中で題名が変われば 1 件増える` /
  `URL だけが変われば 1 件増える`。検証: `cargo test --package ashiato-collector-windows` rc=0
- [x] 3.3 URL は**前景がブラウザのときだけ** UI Automation でアドレスバーを読む（design D4 / Risks）。
  **補正しない**（design D12）。Scenario: `クエリとフラグメントが残る` / `表示されている文字列を補正しない`。
  検証: `cargo test url_is_not_normalized` rc=0
- [x] 3.4 題名だけの変化に最小滞留 5 秒を置く。**アプリと URL の変化は滞留に関わらず必ず 1 件**
  （design D8・**仮**）。Scenario: `題名が最小滞留より短く変わり続けても記録は増えない` /
  `アプリの切り替えは滞留時間に関わらず記録される` / `URL の変化は滞留時間に関わらず記録される`。
  検証: `cargo test min_dwell` rc=0

## 4. 離席（FR-81）

- [x] 4.1 最後の入力からの経過時間・画面ロック・スリープを読む。閾値は 5 分（design D9・**仮**）。
  **経過時間も記録に載せる**（閾値を変えたときに引き直せるようにするため）。
  Scenario: `離席の始まりと終わりが残る` / `画面ロックとスリープも残る`。
  検証: `cargo test idle_transitions` rc=0
- [x] 4.2 離席の記録を前景の記録と区別できる形にする（`payload` の記録の種類。design D1）。
  Scenario: `離席の記録は前景の記録と区別できる`。検証: `cargo test record_kind_is_distinguishable` rc=0

## 5. PC が止まっていた期間（FR-82）

- [x] 5.1 停止時刻の印をローカルに書き、**動作中は 1 分ごとに更新する**（design D10 / Risks）。
  検証: `cargo test last_seen_marker` rc=0
- [x] 5.2 起動時に、印と現在時刻から区間の記録を 1 件作る。**印が無ければ書かない**。
  Scenario: `起動時に止まっていた期間が 1 件残る` / `初回起動では生成しない`。
  検証: `cargo test powered_off_span` rc=0
- [x] 5.3 この記録が**意図的な停止（FR-34）と区別できる**ことを固定する（design D10）。
  Scenario: `意図的な停止と区別できる`。検証: `cargo test powered_off_is_not_stopped` rc=0

## 6. 除外（FR-83）

- [x] 6.1 除外する対象の登録の形（実行ファイルのパス / プロセス名 / ウィンドウ題名の一致規則）を作る。
  **既定は空**。検証: `cargo test exclusion_empty_by_default` rc=0
- [x] 6.2 除外の判定を**送る前**に行い、本文を持たない記録を 1 件書く（design D11）。
  Scenario: `除外に登録した対象の本文は残らない` / `除外した件数が残る` / `除外の登録が空なら何も除外されない`。
  検証: `cargo test exclusion` rc=0
- [x] 6.3 直列化の形を Rust 側のテストで固定する（**形が変わると同じ 1 件が別の鍵になる**。design D1）。
  **4 章・6 章で項目が出そろってから凍結する** —— 先に凍結すると 2 回書き換えることになり、
  形が変わるたびに同じ 1 件が別の鍵になる（spec-review R6）。
  検証: `cargo test payload_shape_is_pinned` rc=0
- [x] 6.4 **除外リストの初期登録の手順**を `crates/collector-windows/README.md` に書く
  （design Risks の「URL にトークンが入る」の守りがこれと PERM-9 の 2 つしかないため）。
  検証: `grep -c "除外" crates/collector-windows/README.md` が 1 以上

## 7. 生存信号（FR-78）

- [x] 7.1 想定間隔（登録簿の `c02-window` = 21600 秒）ごとに生存信号を送る。
  **注入した時計で間隔を進める単体テストで判定する** —— smoke は数十秒で終わるので、
  間隔を守らない実装でも必ず緑になる（spec-review R8）。
  Scenario: `想定間隔ごとに生存信号が届く`。
  検証: `cargo test heartbeat_interval_is_expected_gap` rc=0。
  併せて `tools/smoke.sh` に「生存信号が 1 件届く」手順を足して rc=0
- [x] 7.2 取得可否を**前景ウィンドウが取れること**と **UI Automation が応答すること**で判定し、
  満たされていない側を `blockers` に載せる（design D4）。
  Scenario: `前景を読めない状態は取得できないとして報告される`。検証: `cargo test capturable_blockers` rc=0
- [x] 7.3 前回の信号からの試行回数と成功回数を載せる（**成功は試行を超えない**）。
  Scenario: `試行回数と成功回数が載る`。検証: `cargo test attempts_successes` rc=0

## 8. 送信と保持

- [x] 8.1 追記のみの JSONL で未送信を保持し、到達できたら送る。**上限は置かない**（design D3）。
  Scenario: `到達できない間の記録が後から届く`。検証: `cargo test outbox_survives_outage` rc=0
- [x] 8.2 1 件ごとの結果の `accepted` だけを見て未送信から取り除く（`docs/collector-contract.md` §返る形）。
  検証: `cargo test outbox_uses_accepted_only` rc=0
- [x] 8.3 外部サービス上の識別子を付けずに送り、断られないことを確かめる。
  **`external_id` は null で送る（空文字は不可）** —— ST03 が `empty_external_id` を足しており、
  空文字は 400 で断られる（`docs/collector-contract.md` §返る形）。
  `source_updated_at` と `external_ref` はどちらも省略可なので送らない。
  Scenario: `識別子を持たない記録が受け付けられる`。検証: `tools/smoke.sh` rc=0

## 8b. 感度と時計

- [x] 8b.1 **記録に感度を明示せず、既定（`sensitivity=1` = 外部 AI に出してよい）に委ねる**
  （深掘り Q3。**本人が推奨と違う側を選んだ唯一の決定**）。
  **「題名と URL は私的だから」と厳しい側に倒さない。**
  Scenario: `既定の感度で格納される` / `収集側が厳しい側の感度を付けて送らない`。
  検証: `cargo test sensitivity_uses_collection_default` rc=0。
  併せて `tools/smoke.sh` で `psql -c "SELECT sensitivity FROM core.event WHERE
  logical_source='c02-window' LIMIT 1"` が `1` を返す
- [x] 8b.2 1 時間ごとに時計のずれの測定記録を出す（design D6）。
  Scenario: `1 時間ごとにずれの測定記録が残る`。検証: `cargo test clock_skew_is_measured` rc=0

## 9. ログと常駐

- [x] 9.1 ログにアプリ名・ウィンドウ題名・URL・原文を出さない。出すのは件数・ソース名・所要時間・
  エラーの種別だけ。Scenario: `送信の失敗がログに出ても題名と URL は出ない`。
  検証: `cargo test log_has_no_private_content` rc=0
- [x] 9.2 ログオン時に自動起動する（design D7・**仮**）。スタートアップフォルダへ
  `ashiato-collector.cmd` を 1 つ置く（`--install-autostart`。**読む 5 つの変数を全部書く**。
  レジストリは別の crate か `unsafe` を要するので採らない。design D15）。
  **トレイの常駐表示は ST07 では入れない**（design D7 の（仮）と反転条件。review/code.md R10）。
  検証: `grep -c "自動起動" crates/collector-windows/README.md` が 1 以上 / `cargo test startup_script_is_enough_to_start` rc=0

## 10. 仕上げ

- [x] 10.1 `openspec validate st07-active-window --strict` rc=0
- [x] 10.2 `python3 scripts/check_scenarios.py .` rc=0（**30 本すべてに印**）
- [x] 10.3 `python3 scripts/check_chain.py .` rc=0
- [x] 10.4 `python3 scripts/review_triage.py . st07-active-window` rc=0
- [x] 10.5 `tools/smoke.sh` rc=0 / `tools/check-immutable.sh` rc=0 / `tools/check-migrations.sh` rc=0
- [x] 10.6 `cargo test --workspace` rc=0 / `cargo clippy --workspace -- -D warnings` rc=0
- [x] 10.7 `docs/handoff/` を読み直す（PR 前の 2 回目）

## 11. 独立レビューの処置（`review/code.md`）

**レビューは指摘を出すだけで直さない。直したものをここに挙げる。** 1 件ずつ `review/code.md` の
`処置: fixed 11.<n>` から指される。検証は全部コマンドと終了コード。

- [x] 11.1 離席の区間と除外の数えを置き場（`engine.json`）に落とし、次の起動で閉じる。
  終了の合図（Ctrl-C・窓を閉じる）で `Runtime::stop` を呼ぶ（R1 / R4 / R33。design D21）。
  検証: `cargo test previous_state_is_closed_on_restart` rc=0 / `cargo test restart_closes_previous_away_and_excluded` rc=0
- [x] 11.2 除外の吐き出しで区間を閉じず、数えを 0 から数え直す（R2。design D18）。
  検証: `cargo test excluded_count_is_not_inflated_by_flush` rc=0
- [x] 11.3 自動起動の `.cmd` に読む変数を全部書き、その中身だけで設定が組めることを固定する（R3）。
  検証: `cargo test startup_script_is_enough_to_start` rc=0
- [x] 11.4 本物の HTTP で合言葉を送り 400 の本文を読むことを固定する。smoke は収集側の crate が
  組んだ本文を送る（R5 / R22）。検証: `cargo test http_transport_sends_bearer_and_reads_400_body` rc=0 /
  `tools/smoke.sh` rc=0（手順 37 / 39 が `examples/sample_body` を使う）
- [x] 11.5 ロック中の見回りを試行に数えない（R6）。検証: `cargo test locked_hours_are_not_failures` rc=0
- [x] 11.6 題名の最小滞留 5 秒を下からも縛る（R7）。検証: `cargo test min_dwell_is_five_seconds_from_both_sides` rc=0
- [x] 11.7 動作中に 1 分ごとに印が進むことを Runtime の層で固定する（R8）。
  検証: `cargo test marker_advances_every_minute_while_running` rc=0
- [x] 11.8 読んだ値の解釈を `winrules.rs` に出して Linux で確かめ、実機でしか決まらない 5 本を
  「人間の確認待ち」に足す（R9）。検証: `cargo test --package ashiato-collector-windows winrules` rc=0
- [x] 11.9 `Config` / `HttpTransport` / `Engine` / `Foreground` / `Runtime` の `Debug` に合言葉と本文を出さない（R11 / R35）。
  検証: `cargo test debug_hides_the_token` rc=0 / `cargo test debug_does_not_leak_private_content` rc=0
- [x] 11.10 Scenario の本数を 30 本に直す（R12）。検証: `grep -c "30 本" openspec/changes/st07-active-window/tasks.md` が 2 以上
- [x] 11.11 眠りに入った側にも経過時間を載せ、3 つの理由すべてで固定する。同義反復の assert を消す（R13 / R44）。
  検証: `cargo test idle_records_carry_elapsed_for_rethreshold` rc=0
- [x] 11.12 空文字の URL を「読めなかった」に倒す（R14）。検証: `cargo test empty_url_is_unavailable` rc=0
- [x] 11.13 印と数えを一時ファイル + 置き換えで書き、壊れていたら退避して起動を続ける（R18）。
  検証: `cargo test broken_counters_are_quarantined` rc=0 / `cargo test broken_marker_is_an_error` rc=0 /
  `cargo test atomic_write_replaces_and_leaves_no_tmp` rc=0
- [x] 11.14 生存信号 4 件・ずれの測定 24 件を Runtime を 1 日回して固定する（R19）。
  検証: `cargo test ticks_keep_heartbeat_and_skew_intervals_for_a_day` rc=0
- [x] 11.15 書き込みの失敗で見回りを止めない。`powered-off` に `boot_at` / `clean_stop` を載せる（R16。design D22 / D23）。
  検証: `cargo test write_failures_do_not_stop_collection` rc=0 / `cargo test clean_stop_is_carried_to_next_start` rc=0 /
  `cargo test start_records_powered_off_span` rc=0
- [x] 11.16 契機を単調時計で測り、飛びに `mono_gap_ms` を載せ、時計が飛んだらその場でずれを測る（R17。design D19）。
  検証: `cargo test clock_jumps_do_not_stop_the_intervals` rc=0
- [x] 11.17 読めない未送信の置き場を空として開かない（R20）。検証: `cargo test unreadable_outbox_is_an_error_not_empty` rc=0
- [x] 11.18 件数の合わない応答では何も取り除かない（R21）。検証: `cargo test mismatched_reply_length_removes_nothing` rc=0
- [x] 11.19 離席のまま自動でロックされたとき、入れ替わりを記録する（R23。design D14）。
  検証: `cargo test idle_then_lock_records_the_lock` rc=0
- [x] 11.20 経過時間が読めない状態を `blockers` に挙げ、読めないまま閾値ぶん経ったら区間を閉じる。
  変換できない値を 0 秒にしない（R24）。検証: `cargo test unreadable_idle` rc=0
- [x] 11.21 URL の読み取りの揺れを変化にせず、読めなかった記録に `url_unavailable` を付ける（R25）。
  検証: `cargo test url_flapping_is_not_a_change` rc=0
- [x] 11.22 区間の間に一度でも欠けたものを生存信号に残す（R26）。検証: `cargo test blockers_are_sticky_within_the_interval` rc=0
- [x] 11.23 置き場に既定を持たせず、絶対パスを要求する（R27）。検証: `cargo test config_requires_every_var_including_state_dir` rc=0
- [x] 11.24 除外の登録の書き間違い（知らない欄・`rules` の欠落・空の `value`）を断る（R28）。
  検証: `cargo test broken_registration_is_an_error_not_empty` rc=0 / `cargo test rules_hit_by_path_name_and_title` rc=0
- [x] 11.25 プロセス名が解決できない前景を「読めなかった」に倒す（R29）。
  検証: `cargo test process_name_is_none_not_empty` rc=0 /
  `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0
- [x] 11.26 壊れた行の退避を追記にし、件数をログに出す。断られた分の覚えを掃除する（R30）。
  検証: `cargo test broken_line_is_quarantined_not_dropped` rc=0
- [x] 11.27 未送信の追記と書き直しを同期する（R31）。検証: `cargo test atomic_write_replaces_and_leaves_no_tmp` rc=0
- [x] 11.28 二重に起動しない（R32。design D24）。
  検証: `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` rc=0
- [x] 11.29 CI の windows job で clippy を当てる（R34。design D20）。
  検証: `grep -c "cargo clippy -p ashiato-collector-windows" .github/workflows/ci.yml` が 1 以上
- [x] 11.30 環境変数を触る試験をやめる（R36）。検証: `cargo test env_adds_to_the_defaults` rc=0
- [x] 11.31 断られた分だけの当たり直し・1 回の件数の切り・閾値ちょうどの境界を固定する（R39）。
  検証: `cargo test rejected_only_backlog_is_retried` rc=0 / `cargo test batch_is_capped` rc=0 /
  `cargo test idle_threshold_boundary` rc=0
- [x] 11.32 滞留を満たした題名を切り替えで捨てない。眠っていた間を滞留に数えない（R40）。
  検証: `cargo test dwelled_title_survives_app_switch` rc=0 / `cargo test suspend_does_not_count_as_dwell` rc=0
- [x] 11.33 基準時刻の口に `date` ヘッダがあることを smoke で見る（R42）。検証: `tools/smoke.sh` rc=0（手順 40）

## 12. Windows の実行時テスト（2026-09-14。本人の決定: 実機の確認は最小にし、機械で確かめる）

**`platform.rs` は 0 テストで、11 の Scenario が「人間の確認待ち」だった。** テストが自分で窓を作り、
本物の `WindowsSource` に読ませて `Engine` に通す実行時テストを `crates/collector-windows/tests/runtime_windows.rs` に置く。
相手役は `tests/support/helper_window.ps1`（WinForms）と Edge。閾値は短くし（滞留 1 秒・離席 2 秒）、
本人の決めた 5 秒・5 分は `engine.rs` の単体が固定したまま。**Windows の上でだけ走る**（`#![cfg(windows)]`）。

- [x] 12.1 足場: 自分で出した窓が本物の `WindowsSource` から題名つきで読める。検証: Windows で `cargo test -p ashiato-collector-windows --test runtime_windows helper_window_is_observed` が rc=0
- [x] 12.2 アプリの切り替え（WinForms → Edge）で記録が 1 件増え、切り替えた後のアプリを持つ。検証: `... --test runtime_windows switching_app` が rc=0
- [x] 12.3 題名を最小の滞留より短い間隔で 10 回変えても記録は 1 件で、最後の題名を持つ。検証: `... --test runtime_windows rapid_title` が rc=0
- [x] 12.4 除外に登録したアプリ（Edge をプロセス名で）の題名・URL・アプリ名がどの記録にも無く、件数だけが残る。検証: `... --test runtime_windows excluded_app` が rc=0
- [x] 12.5 入力を止めて閾値を超え、再開すると、入った側と出た側が 1 件ずつ残り、本文を持たない。検証: `... --test runtime_windows idle_enter_and_leave` が rc=0
- [x] 12.6 前回の印を 1 時間前にして起動すると `powered-off` が 1 件、本物の `boot_at` つきで送られる。検証: `... --test runtime_windows powered_off_span` が rc=0
- [x] 12.7 Edge のアドレスバーを UI Automation で読み、クエリとフラグメントが残り、表示どおり（`http://` 無し）で記録され、同じタブで URL だけが変わると 1 件増える。検証: `... --test runtime_windows browser_url` が rc=0
- [x] 12.8 CI に `windows-latest` の job を足し、単体 86 本と実行時テストを Windows の上で走らせる。job は 25 分で打ち切り、走った本数が 7 未満なら落とす（R7 / R8）。検証: `.github/workflows/ci.yml` の `collector-windows-runtime` job が緑（**初回は 2 本落ちた** —— runner では Edge の題名が付く前に前景を読んでいた。頁の読み込みを待ってから読む形に直した）
- [x] 12.9 `cargo fmt --all --check` と `cargo clippy -p ashiato-collector-windows --all-targets -- -D warnings` が Windows の上で rc=0（テストも lint の対象）

**実測（2026-09-14、この PC）**: 7 本が 2 回連続で緑（32〜42 秒）。分かったこと —— (1) 相手役が標準入力を同期で待つと
メッセージポンプが止まり UI Automation が固まる、(2) `mshta.exe` は `about:` で即座に終了する、(3) `cmd.exe` の窓は
Windows Terminal が持つので題名で探せない、(4) Edge をもう 1 度起動すると新しいタブが開いて題名に「および他 1 ページ」が付く
（頁の script で `location.href` を変えると同じタブで URL だけ変わる）。

## 人間の確認待ち

**書式は `- Scenario: <名前>` の裸の形**（チェックボックスも番号も注釈も付けない）——
`check_scenarios.py` / `verify_checklist.py` / `verify_record.py` の 3 本ともこの形しか読まない
（spec-review R3）。やり方は次の行の引用に置く。

**2026-09-14 に 11 件から 2 件へ減らした。** 9 件は §12 の実行時テストが Windows の上で機械的に確かめる
（「前景を読めない」「URL を読めない」の 2 件は `heartbeat.rs` の単体が生存信号の導出を固定している）。
人間に残すのは、runner でも手元でも機械が再現できない**物理的な操作**だけ。本物の再起動をまたいだ
`powered-off`（12.6 は印を細工して確かめている）は、次の確認バッチで「触って違和感がないか」の問いとして見る。

- Scenario: 起動時に止まっていた期間が 1 件残る
  > 本物の再起動をまたぐ（12.6 は印を細工して確かめている。review/code-r2.md R5）。PC を落として翌日起動し、
  > その期間の記録が 1 件あり、`boot_at` が区間の始まりより後であることを見る
- Scenario: 画面ロックとスリープも残る
  > Win+L でロックして解除し、`reason: locked` の出入りが残ること（ロック中の前景が `LockApp.exe` に見えること）を見る。
  > 5 分放置して自動でロックされたときは `ended_by: superseded` の離席と `locked` の入りが残ることを見る（R9 / R23）
