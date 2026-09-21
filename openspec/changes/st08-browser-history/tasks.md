# ST08 実装タスク — PC のブラウザ履歴を集める

読む順: `deep.md`（**最優先。本人が決めた 3 件 + 第 2 回の 2 件と、context で見せた R2 / R5〜R9**）→ このファイル →
`specs/desktop-collection/spec.md` → `design.md` → `docs/stories/ST08.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D7）。**名前は作成時刻 `YYYYMMDDHHMM_browser_history_record_id.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。**サーバの取り込みのコードは変えない**（design D8）。
**`collection-coverage` と `record-envelope` には触らない。**

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` と `tools/seed.sh` が前提。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust はコメント（`// Scenario: 2 回続けて取得しても行が増えない`）、
  bash は `echo`。`scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする
- 印の名前は spec の `#### Scenario:` と**一字一句合わせる**（空白は無視される）
- **印を置くテストは、その Scenario の THEN を確かめるものにする**（review/spec.md R16）。THEN が「格納されている」なら
  取り込み口まで通す検査（サーバの結合テストか `tools/smoke.sh` の psql）に置く
- **この change の delta は 75 本**（`desktop-collection`。うち ST08 で新しく足した Scenario は 62 本。
  残る 13 本は MODIFIED で写した ST07 の既存分で、印は ST07 のテストに既にある）
- **ウィンドウの記録の形を変えない**（design D13）。`cargo test payload_shape_is_pinned` は最初から最後まで緑のまま
- **D3 / D10 / D15 は（仮）決め**を含む。反転条件は `design.md` にある。**D3 と D14 の反転条件のうち spec を破る手は本人に返す**
- **識別子の形（design D6 / D9）は第 2 回 Q4 / Q5 の本人の答えに従う**（`deep.md`）。答えより前に実装を始めない
- **ログに URL・ページの題名・プロファイルの表示名・原文を出さない**（spec の MODIFIED）。帳面にも URL と題名を書かない（design D10）
- 同じ名前のテストで絞る検証（`cargo test history_vanished` など）は、**`-- --list` で 1 本以上あることも見る**（0 本でも rc=0 になるため。R17）

## 1. 足場

- [x] 1.1 `crates/collector-windows/Cargo.toml` に `rusqlite`（`bundled`）を**`cfg(windows)` の外に**足し、
  `history/` モジュール（`locate` / `read` / `contract` / `ledger` / `fetch`）の空の骨組みを置く（design D2）。
  検証: `cargo build -p ashiato-collector-windows` rc=0 / `tools/check-licenses.sh` rc=0
- [ ] 1.2 CI の windows 向け型検査（`collector-windows` job）が `bundled` の C をコンパイルできるようにする
  （mingw の C コンパイラを入れる。design D2）。
  検証: `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` が手元と CI で rc=0

## 2. 登録簿と取り込み（サーバ側。取り込みのコードは変えない）

- [ ] 2.1 移行 `YYYYMMDDHHMM_browser_history_record_id.sql`（と `.down.sql`）を作り、`c02-browser-history` の
  `external_id_kind` を**このソースの記録が 0 件のときだけ** `'record'` にする（design D7）。`MIGRATIONS` の末尾に足す。
  テスト: (a) 全移行を当てると `'record'` / (b) 記録が 1 件ある状態で値を `'none'` に戻して移行を当て直しても `'none'` のまま /
  (c) `c02-window` は `'record'` にならない / **(d) 識別子なしの `c02-browser-history` の要求が `missing_external_id` で断られる**（R16）。
  Scenario: `識別子を欠いた履歴の記録は断られる`（(d) に印）。
  検証: `cargo test -p ashiato-server browser_history_record_id` rc=0 / `tools/check-migrations.sh` rc=0 /
  `cargo test c02_window_external_id_kind_is_not_record` rc=0
- [ ] 2.2 サーバの結合テストで、`c02-browser-history` に同じ識別子の到着を送り分けて固定する（design D6 / D8 / D15）:
  (a) 題名だけ違う到着で行が 1 のまま新しい題名を持つ / (b) 前の題名の版が 1 つ積む / (c) 滞在時間だけ違う到着で同じく版が積む /
  (d) `source_updated_at` の古い到着が後から届いても題名が書き戻らない / (e) 番号だけ同じで訪問時刻の違う識別子は別の行になる。
  Scenario: `題名が変わった訪問は 1 行のまま新しい題名を持つ` / `題名が変わった訪問の前の版が残る` /
  `閉じたタブの滞在時間が後の取得で更新され、前の版が残る` / `未送信の再送で古い題名が新しい題名を書き戻さない` /
  `番号が振り直された後の訪問は、前の訪問と別の記録になる`。
  検証: `cargo test -p ashiato-server browser_history_update` rc=0（`-- --list` で 5 本）

## 3. 送る形（契約）

- [ ] 3.1 `VisitPayload`（`visit` / `vanished` / `excluded` / `profiles`）と `IngestRequest::of_visit` を作る（design D4 / D5 / D6 / D15）。
  `event_time` はマイクロ秒、`tz_basis = "collected-at"`、`source_updated_at` は読んだ時刻、`external_id` は D6 の形
  （**第 2 回 Q5 の答えに従う**）。`IngestRequest` の `source_updated_at` は `skip_serializing_if` で足す（design D13）。
  Scenario: `訪問時刻がマイクロ秒で残る` / `タイムゾーンが取得時のものだと本文から分かる` / `識別子から URL と訪問時刻とプロファイルが読み取れない` /
  `記録の本文にブラウザとプロファイルがある`。
  検証: `cargo test visit_time_keeps_micros` rc=0 / `cargo test visit_external_id_is_pinned` rc=0 /
  `cargo test visit_payload_shape_is_pinned` rc=0 / `cargo test window_request_body_is_unchanged` rc=0 / `cargo test payload_shape_is_pinned` rc=0
- [ ] 3.2 起点の換算（Chromium の 1601 年起点・Firefox の 1970 年起点）を純粋な関数にし、境界（0・負）を単体で固定する。
  検証: `cargo test history_epoch_conversion` rc=0
- [ ] 3.3 `docs/collector-contract.md` に **C-02 の履歴の `payload` の形・識別子の作り方・`source_updated_at` に読んだ時刻を載せる理由**を追記する（design D4 / D6 / D15）。
  検証: `grep -c "c02-browser-history" docs/collector-contract.md` が 1 以上 / `grep -c "v1:" docs/collector-contract.md` が 1 以上 /
  `grep -c "source_updated_at" docs/collector-contract.md` が 3 以上（既存 2 + 追記）/ `python3 scripts/check_chain.py .` rc=0

## 4. 置き場を探して読む

- [ ] 4.1 置き場の組み立て（design D1 の表を**全 6 行**。`User Data` の直下 1 段で `History` を持つディレクトリ、
  Firefox は `profiles.ini` の `Path=` と `Profiles\` の走査の和）を純粋な関数にし、一時ディレクトリに作った木で確かめる。
  Scenario: `6 つのブラウザの既知の置き場にあるプロファイルが全部見つかる` / `Firefox の一覧にある既定の外の置き場も見つかる` /
  `複数のブラウザと複数のプロファイルの履歴が全部入る`。
  検証: `cargo test history_locate` rc=0（`-- --list` で 3 本以上。6 種を列挙する assert を含む）
- [ ] 4.2 表示名との対応（Chromium 系は `Local State`、Firefox は `profiles.ini`）を読み、**初回と対応が変わったときだけ** `profiles` 記録を作る（design D1 / D4）。
  Scenario: `プロファイルの表示名との対応が残る` / `プロファイルの表示名を変えると新しい対応が残る`。
  検証: `cargo test history_profiles_map` rc=0
- [ ] 4.3 写しを取って読み取り専用で開き、Chromium 系（`visits` + `urls`）と Firefox（`moz_historyvisits` + `moz_places`）を訪問 1 件ごとに読む。
  写しは読み終えたら消す（design D2）。単体は `rusqlite` で両方の表の形を作って確かめる。
  Scenario: `同じページを 2 回訪問すると 2 件になる` / `ページの題名と滞在時間が載る` / `遷移の種類とどこから来たかが残る` /
  `URL のクエリとフラグメントが残る` / `履歴 DB の URL の文字列を補正しない`。
  検証: `cargo test history_read_chromium` rc=0 / `cargo test history_read_firefox` rc=0 / `cargo test history_copy_is_removed` rc=0
- [ ] 4.4 同期で入った他端末の訪問に `originator_cache_guid` / `originator_visit_id` を載せ、端末は読んだ PC、識別子は PC 側の番号
  （**第 2 回 Q4 の答えに従う**。design D9）。
  Scenario: `他の端末の訪問は発生元の印を持つ` / `他の端末の訪問の記録の端末は、読んだ PC である` / `PC 自身の訪問は発生元の印を持たない`。
  検証: `cargo test history_foreign_visits` rc=0

## 5. 帳面と取得

- [ ] 5.1 帳面（プロファイルごと。識別子 → 送った内容のハッシュ・訪問時刻・他端末か・除外したか、前回の最大番号、前回の表示名の対応）を作る。
  **URL と題名を書かない。** 一時ファイル + 置き換えで書き、壊れていたら退避して空から始める（design D10）。
  検証: `cargo test history_ledger_has_no_url_or_title` rc=0 / `cargo test history_ledger_broken_is_quarantined` rc=0
- [ ] 5.2 取得のたびに全部読み、**まだ送っていない訪問と内容が変わった訪問だけ**を積む（design D3）。
  Scenario: `初回の取得で過去の履歴が入る` / `前回の取得の後に古い時刻で入った訪問も取り込まれる`。
  検証: `cargo test history_fetch_sends_only_new_or_changed` rc=0
- [ ] 5.3 同じ履歴 DB で 2 回取得して行の件数が変わらないことを、取り込み口まで通して見る。
  Scenario: `2 回続けて取得しても行が増えない`。
  検証: `tools/smoke.sh` に手順を足し（`examples/` に Chromium の形の小さな履歴 DB を作る台本を置く）、
  `SELECT count(*) FROM core.event WHERE logical_source='c02-browser-history'` が 2 回目の前後で同じで rc=0
- [ ] 5.4 契機: 起動時に前回の成功から 24 時間以上なら取得、動作中は前回の成功から 24 時間、失敗は成功を進めず 1 分後に試し直す（design D3。**仮は 1 分だけ**）。
  注入した時計で確かめる。
  Scenario: `起動時に前回の成功から 24 時間以上経っていれば取得する` / `前回の成功から 24 時間経たないうちは取得しない` /
  `動作中に前回の成功から 24 時間経つと取得する`。
  検証: `cargo test history_schedule` rc=0（`-- --list` で 3 本以上）
- [ ] 5.5 **成功とみなすのは未送信に積み終えて帳面を書いた後**。取り込み口を止めた状態で取得し、戻した後に格納されていることを取り込み口まで通して見る（R16）。
  Scenario: `取り込み口が止まっている間に取得した履歴が後から届く`。
  検証: `cargo test history_success_only_after_outbox` rc=0 /
  `tools/smoke.sh` に「サーバを止めて取得 → 起動 → 送信 → psql で件数」の手順を足して rc=0
- [ ] 5.6 読みは見回りとは別のスレッド。**読みに 3 分かかる読み手を差し込んでも見回りが続き、`c02-window` に `suspended` が入らず、
  履歴の記録を 1 件も `c02-window` に作らない**ことを Runtime の層で固定する（R15）。
  Scenario: `履歴の取得でウィンドウのソースの記録は増えない`。
  検証: `cargo test history_slow_read_does_not_disturb_window` rc=0

## 6. 消えた事実（Q2）

- [ ] 6.1 帳面にあって今回の読みに無い識別子を `vanished` に載せ、手がかり（`age_days` / `foreign` / `table_recreated` / `profile_gone`）を付け、
  帳面から外す。1 件に 1,000 件まで（design D10・**仮**）。**URL と題名を載せない。経路を名指しする値を置かない。**
  Scenario: `履歴から 1 件消すと次の取得で「消えた」記録が残る` / `消えた訪問の、訪問から取得までの日数が本文にある` /
  `同期で入った訪問が消えたことが本文にある` / `表が作り直されたことが本文にある` / `プロファイルが無くなったことが本文にある` /
  `消えた経路を名指しする値を持たない` / `消えた記録に URL と題名が載らない`。
  検証: `cargo test history_vanished` rc=0（`-- --list` で 7 本以上）
- [ ] 6.2 **読めなかったプロファイルでは判定しない**。やり直しで同じ「消えた」が 2 件目にならない（識別子が中身から決まる。design D6 / D10）。
  Scenario: `読めなかったプロファイルでは消えた記録を出さない` / `取得をやり直しても「消えた」記録は増えない`。
  検証: `cargo test history_vanished_skips_unreadable_profile` rc=0 / `cargo test history_vanished_is_idempotent_on_retry` rc=0
- [ ] 6.3 「消えた」記録を送った後も元の訪問の行が変わらないことを、取り込み口まで通して見る。
  Scenario: `消えた訪問の記録は残っている`。
  検証: `tools/smoke.sh` に「`vanished` を送った前後で元の訪問の行の `raw` が同じ」を psql で見る手順を足して rc=0

## 7. 除外（FR-83 を履歴にも）

- [ ] 7.1 `Rule` に `url-contains` と `browser-profile` を足す（`deny_unknown_fields` のまま）。`url-contains` がウィンドウに当たったら
  **その前景の変化を丸ごと除外して件数に数える**。`browser-profile` はウィンドウに当てない（design D11）。
  README の除外の登録の手順に 2 つを書き足す。
  Scenario: `URL の部分一致に当たった前景は本文を残さない` / `URL の部分一致に当たった前景は除外の件数に数えられる`。
  検証: `cargo test rules_hit_url_and_profile` rc=0 / `cargo test url_rule_excludes_whole_foreground` rc=0 /
  `cargo test broken_registration_is_an_error_not_empty` rc=0 /
  `grep -c "url-contains" crates/collector-windows/README.md` が 1 以上 / `grep -c "browser-profile" crates/collector-windows/README.md` が 1 以上
- [ ] 7.2 履歴への写像（プロセスの登録 → そのブラウザの全プロファイル、題名 → ページの題名、URL → 訪問の URL、`browser-profile` → そのプロファイル）を実装し、
  **送る前**に落とす。除外した訪問は帳面に書き、取得 1 回・プロファイル 1 つにつき `excluded` を 1 件（design D11）。
  Scenario: `ブラウザのプロセスを除外するとその全プロファイルの履歴が送られない` / `履歴で除外した件数が残る` /
  `題名の部分一致の登録はページの題名に当たる` / `URL の部分一致の登録は履歴にも効く` /
  `プロファイルを指す登録はそのプロファイルの履歴だけを除く` / `除外した訪問は取得のたびに数え直されない` /
  `取得をやり直しても除外の件数は増えない`。
  検証: `cargo test history_exclusion` rc=0（`-- --list` で 7 本以上）
- [ ] 7.3 登録を後から足した / 外したときの振る舞いを固定する（design D11 / R8）。
  Scenario: `登録を後から足すと、既に送った訪問の変わった内容は送られない` / `登録を外すと、まだ履歴にある除外済みの訪問が次の取得で送られる`。
  検証: `cargo test history_exclusion_added_later` rc=0 / `cargo test history_exclusion_removed_later` rc=0

## 8. 生存信号（2 本目）

- [ ] 8.1 `c02-browser-history` 用の `Schedule`（86400 秒）と数え（`counters-browser-history.json`）を持ち、ウィンドウとは別の件として送る。
  `blockers` は `history-unreadable:<browser>:<profile_dir>` / `history-none-found`、区間の間の和（design D12）。
  注入した時計で Runtime を 2 日回して確かめる。
  Scenario: `ブラウザ履歴のソースにも想定間隔ごとに生存信号が届く` / `ブラウザ履歴の生存信号はウィンドウの生存信号と別の件である` /
  `読めないプロファイルが 1 つでもあれば取得できないとして報告される` / `読めなかったブラウザとプロファイルが満たされていないものに挙がる` /
  `履歴が 1 つも見つからなければ取得できないとして報告される`。
  検証: `cargo test history_heartbeat` rc=0（`-- --list` で 5 本以上）/ `cargo test ticks_keep_heartbeat_and_skew_intervals_for_a_day` rc=0
- [ ] 8.2 **区間に読みが 1 回も無いときは、信号を出す前に写しを取って開けるかを確かめる**（`SELECT 1` まで。design D12 / R4）。起動直後に 1 回出す。
  Scenario: `区間に読みが無くても、開けるかを確かめてから報告する`。
  検証: `cargo test history_heartbeat_probes_when_not_read` rc=0 / `cargo test history_heartbeat_on_start` rc=0

## 9. 感度とログ

- [ ] 9.1 履歴の記録に感度を明示せず、既定に委ねる（ST07 Q3 / ST08 Q1）。
  Scenario: `ブラウザ履歴の記録も既定の感度で格納される`。
  検証: `tools/smoke.sh` で `psql -c "SELECT DISTINCT sensitivity FROM core.event WHERE logical_source='c02-browser-history'"` が `1` だけを返して rc=0 /
  `cargo test history_sensitivity_uses_collection_default` rc=0
- [ ] 9.2 履歴の取得のログに URL・ページの題名・プロファイルの表示名・原文を出さない（件数・ソース名・所要時間・エラーの種別だけ）。
  Scenario: `履歴の読み取りの失敗がログに出ても URL と題名は出ない`。
  検証: `cargo test history_log_has_no_private_content` rc=0

## 10. Windows の実行時テスト（design D14）

`crates/collector-windows/tests/runtime_windows.rs` に足す。**一時プロファイル**で開き、本人の実物のプロファイルは使わない。
**Chrome と Firefox も飛ばさない**（runner に無ければ job で入れる。R10）。

- [ ] 10.1 Edge を一時プロファイルで開いてページを 2 つ訪問し、**Edge を開いたまま**本物の置き場探し・写し・読み手・取得を通して、
  2 つの訪問が URL と訪問時刻つきの記録として積まれる（取得契機は注入した時計で起こす）。
  Scenario: `ブラウザが動いている間も取得できる`。
  検証: Windows で `cargo test -p ashiato-collector-windows --test runtime_windows browser_history_while_running` が rc=0
- [ ] 10.2 Chrome と Firefox でも同じことを確かめる。
  検証: Windows で `... --test runtime_windows browser_history_chrome` rc=0 / `... --test runtime_windows browser_history_firefox` rc=0
- [ ] 10.3 「前日に見たページが翌日の取得で入っている」を取り込み口まで通して見る: 10.1 の記録を smoke と同じ手順でサーバへ送り、
  psql で URL と訪問時刻の行があることを見る（Windows の job ではサーバを立てないので、`tools/smoke.sh` の側で
  履歴 DB を読んで取得契機を 1 日進める形にする）。
  Scenario: `前日に見たページが翌日の取得で入っている`。
  検証: `tools/smoke.sh` rc=0（手順に「取得契機を 1 日進めて psql で行を見る」がある）
- [ ] 10.4 CI の `collector-windows-runtime` job の走った本数の下限を **既存 + 3**（10.1 / 10.2 の 3 本）に上げる。飛ばしを下限に数えない。
  検証: `.github/workflows/ci.yml` の下限の数が既存より 3 大きい（`git diff` で見る）/ `collector-windows-runtime` job が緑

## 11. 仕上げ

- [ ] 11.1 `openspec validate st08-browser-history --strict` rc=0
- [ ] 11.2 `python3 scripts/check_scenarios.py .` rc=0（**75 本すべてに印**）
- [ ] 11.3 `python3 scripts/check_chain.py .` rc=0
- [ ] 11.4 `python3 scripts/review_triage.py . st08-browser-history` rc=0
- [ ] 11.5 `tools/smoke.sh` rc=0 / `tools/check-immutable.sh` rc=0 / `tools/check-migrations.sh` rc=0 / `tools/check-licenses.sh` rc=0
- [ ] 11.6 `cargo test --workspace` rc=0 / `cargo clippy --workspace --all-targets -- -D warnings` rc=0 / `cargo fmt --all --check` rc=0
- [ ] 11.7 `docs/handoff/` を読み直す（PR 前の 2 回目）

## 人間の確認待ち

**書式は `- Scenario: <名前>` の裸の形**（`check_scenarios.py` / `verify_checklist.py` / `verify_record.py` がこの形しか読まない）。

**無し。** 75 本はすべて単体・結合・Windows の実行時テストが持つ（本人の決定 2026-09-14: 人間の確認は正しさのテストではない）。
確認バッチでは Story ごとの 1 問「触ってみて違和感は無かったか」だけを聞く ——
本人の実物のプロファイル（普段使っているブラウザ。深掘り Q1 の補足は空だった）で 1 日置いて、
稼働状況の画面と記録を見てもらう。
