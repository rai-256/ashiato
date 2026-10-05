# ST08 実装の独立検証（code-verify）

対象: worktree `/home/yosis/dev/ashiato2-st08`（`feat/st08-browser-history` / HEAD `96ac2f2`）
DB: `docker compose up -d --wait db`（healthy）で実行。作業ツリーは検証の前後とも clean（`git status --short` が空）。

## 申告と実測

申告: 実装フェーズの 36 タスク（`- [x]` 36 件 / `- [ ]` 2 件 = 10.1・10.2）すべて完了、検証も通した。
`.git/harness/ST08-run.json` は `reason: impl_done / left_impl: 0`。

| 走らせたもの | 実測 | 申告との一致 |
|---|---|---|
| `cargo test --workspace` | rc=0（server 338 / collector 132） | 一致 |
| `cargo clippy --workspace --all-targets -- -D warnings` | rc=0 | 一致 |
| `cargo fmt --all --check` | rc=0 | 一致 |
| `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings`（1.2） | rc=0 | 一致 |
| `openspec validate st08-browser-history --strict`（11.1） | rc=0 | 一致 |
| `python3 scripts/check_scenarios.py . st08-browser-history`（11.2） | rc=0（495/495） | 一致 |
| `python3 scripts/check_scenarios.py .`（11.2 が書いている形） | **rc=1**（担保なし 148 件。すべて st12 / st06 由来） | **不一致**（R19） |
| `python3 scripts/check_chain.py .`（11.3） | rc=0 | 一致 |
| `python3 scripts/review_triage.py . st08-browser-history`（11.4） | rc=0 | 一致 |
| `tools/smoke.sh`（5.3 / 5.5 / 6.3 / 9.1 / 10.3 / 11.5） | rc=0 | 一致（ただし中身は R8 / R9） |
| `tools/check-immutable.sh` / `check-migrations.sh`（11.5） | rc=0 / rc=0 | 一致 |
| `tools/check-licenses.sh`（1.1 / 11.5） | **rc=1**（`web/node_modules が無い（この検査は空振りしている）`） | **不一致**（R20） |
| `cargo test window_request_body_is_unchanged`（3.1） | rc=0 だが **0 tests**（リポジトリ全体に定義が無い） | **不一致**（R4） |
| `cargo test history_heartbeat -- --list`（8.1 は「5 本以上」） | **4 本** | **不一致**（R11） |
| `cargo test browser_history_update -- --list`（2.2 は「5 本」） | 5 本（ただし同一本体を呼ぶ 5 関数） | 形式は一致 / 実体は R7 |
| `cargo test history_vanished` / `history_exclusion` / `history_locate` / `history_schedule` の本数 | 9 / 9 / 3 / 3（下限を満たす） | 一致 |
| ガードを壊す（外部識別子・移行の `NOT EXISTS`・置き場の表） | **3 件とも 1 本も落ちない** | **不一致**（R5 / R6 / R14） |
| 固定値の独立再計算（smoke の Chromium 起点・external_id） | **2 件とも実装と食い違う** | **不一致**（R8） |
| `crates/collector-windows/tests/` の差分（10.4） | **差分なし・7 本のまま**。CI の下限は 10 に上げてある | **不一致**（R10） |
| リモートの CI | `origin/feat/st08-browser-history` が**存在しない**（一度も push されていない） | CI は 1 度も走っていない |

手 3（Scenario と test の突き合わせ）: `check_scenarios.py` は st08 スコープで OK だが、
印の先が Scenario の THEN を観測していないものが **R1 / R2 / R3 / R11 / R12 / R13 / R18** に挙げた範囲で 20 本以上ある。

---

## R1. `history` モジュールが収集の本体から一度も呼ばれていない。履歴は 1 件も取得・送信されない

- 処置: escalated — deep.md Q6（R1）へ premise / loss: uncaptured として追記。

- 成果物: crates/collector-windows/src/history/（locate / read / ledger / fetch / contract）、crates/collector-windows/src/runtime.rs、crates/collector-windows/src/engine.rs、crates/collector-windows/src/main.rs
- 根拠:
  ```
  $ grep -rn "use crate::history\|history::" --include=*.rs crates/ | grep -v "^crates/collector-windows/src/history/"
  crates/collector-windows/src/contract.rs:268:        visit: &crate::history::contract::Visit,
  crates/collector-windows/src/contract.rs:453:        let visit = crate::history::contract::Visit::new(   ← mod tests の中
  $ grep -rn "HistorySchedule\|HistoryWorker\|select_new_or_changed\|detect_vanished\|apply_history_exclusions\|queue_then_save\|read_chromium\|read_firefox\|probe_readable\|LedgerStore\|locate(" --include=*.rs crates/ | grep -v "^crates/collector-windows/src/history/"
  （出力なし）
  ```
  `IngestRequest::of_visit`（contract.rs:267）も、呼び出しは contract.rs:461 のテストだけ。
  `runtime.rs` / `engine.rs` / `main.rs` / `platform.rs` に `history` の語が 1 つも無い。
  実装されたのは**純粋関数の集合と、その単体テスト**であり、取得契機 → 置き場探し → 写し → 読み →
  除外 → 契約 → outbox → 帳面 の線が 1 本も繋がっていない。
  spec の「WHEN PC 側の収集がブラウザ履歴の取得契機に達する」を満たす経路が存在しない。
- kind: irreversible
- loss: uncaptured
  （Chromium 系は 90 日を過ぎた履歴を手元の DB から消す。spec の導出元にあるとおり
  「取らなかったブラウザ・プロファイルの履歴は後から作れない」。走らせても 1 件も取らない状態で
  merge すると、その日数ぶんが恒久的に失われる）

## R2. `ReadVisit` → `Visit` の変換が存在せず、滞在時間・遷移の種類・どこから来たかは常に `None`

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/read.rs、crates/collector-windows/src/history/contract.rs
- 根拠: `grep -rn "ReadVisit" --include=*.rs crates/ | grep -v history/read.rs` が空。
  `history/contract.rs:31-33` は `duration_ms: None` / `transition: None` / `referrer: None` を直書きし、
  これらを設定する関数は `with_originator`（発生元だけ）以外に無い（`grep -rn "duration_ms\|referrer"` で
  代入箇所が 1 つも無いことを確認）。
  spec の THEN「その記録の本文に、ページの題名と滞在時間がある」「その記録は遷移の種類と、
  どの訪問から来たかを持つ」は、現在の実装では必ず偽になる。
  印を置いた `history_read_chromium`（read.rs:140）は `ReadVisit` の段階で `url` と `title` を見るだけで、
  `duration_us` も `transition` も assert していない（記録の本文までは 1 行も通っていない）。
- kind: technical
- 提案: `ReadVisit` から `Visit` を作る関数を足し、`duration_ms` / `transition` / `referrer` を
  埋めたうえで `visit_payload_shape_is_pinned` の固定文字列に載せる。

## R3. `vanished` / `excluded` / `profiles` の記録を組み立てるコードが無い（tasks 3.1 が書いた 4 種のうち `visit` だけ）

- 処置: escalated — deep.md Q6（R3）へ premise / loss: uncaptured として追記。

- 成果物: crates/collector-windows/src/history/contract.rs、crates/collector-windows/src/history/fetch.rs、crates/collector-windows/src/history/locate.rs
- 根拠: `grep -rn "of_vanished\|of_excluded\|of_profiles"` が空。
  `VisitPayload.kind` は `"visit"` のリテラル固定（contract.rs:24）。
  `VanishedVisit`（fetch.rs:45）は `kind` も持たず、`IngestRequest` へ変換する経路が無い。
  履歴の `excluded` は `apply_history_exclusions` が `usize` の件数を返すだけで、
  `logical_source = c02-browser-history` の記録にする箇所が無い（`contract.rs:45` の
  `RecordKind::Excluded` はウィンドウ用）。
  `profiles` の記録は型すら無い（`profile_names`（locate.rs:44）は `BTreeMap` を返すだけ）。
  tasks 4.2 の「**初回と対応が変わったときだけ** `profiles` 記録を作る」に対応するコードが無い。
  これで次の Scenario は担保が無い: `履歴から 1 件消すと次の取得で「消えた」記録が残る` /
  `消えた訪問の、訪問から取得までの日数が本文にある` / `同期で入った訪問が消えたことが本文にある` /
  `表が作り直されたことが本文にある` / `プロファイルが無くなったことが本文にある` /
  `消えた記録に URL と題名が載らない` / `履歴で除外した件数が残る` /
  `プロファイルの表示名との対応が残る` / `プロファイルの表示名を変えると新しい対応が残る`。
- kind: irreversible
- loss: uncaptured
  （深掘り Q2 の本人の答えは「消えたことを記録として残す」。spec が書くとおり
  「『消えた』事実は次の取得で比べた時にしか分からず、比べなかった期間の削除は後から特定できない」）

## R4. tasks 3.1 が検証に挙げる `window_request_body_is_unchanged` はリポジトリに存在しない（0 本で rc=0）

- 処置: fixed 3.1

- 成果物: openspec/changes/st08-browser-history/tasks.md（3.1）、crates/collector-windows/src/contract.rs
- 根拠:
  ```
  $ cargo test --workspace window_request_body_is_unchanged -- --list
  0 tests, 0 benchmarks （rc=0）
  $ grep -rn "window_request_body_is_unchanged" --include=*.rs .
  （出力なし）
  ```
  tasks.md §0 が「同じ名前のテストで絞る検証は、`-- --list` で 1 本以上あることも見る（0 本でも rc=0 に
  なるため。R17）」と自分で書いている型に、3.1 自身が落ちている。
  design D13 の「ウィンドウの要求の本文を変えない」を固定するテストが無いまま、
  `IngestRequest` に `source_updated_at` 欄が増えている（contract.rs:227）。
- kind: technical
- 提案: `IngestRequest::of` の直列化結果を丸ごと固定するテストを足すか、3.1 の検証行を実在の名前に直す。

## R5. 外部識別子の式がどのテストでも固定されていない。訪問番号だけのハッシュに変えても 132 本全部緑

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/contract.rs:36-44、同 tests（`visit_external_id_is_pinned`）
- 根拠: `Visit::new` の
  `for part in [browser, profile, id.as_str(), payload.at.as_str(), url]` を
  `for part in [id.as_str()]` に書き換えて実行:
  ```
  $ cargo test -p ashiato-collector-windows --lib
  test result: ok. 132 passed; 0 failed   （rc=0）
  ```
  （確認後、原状へ戻し `git status --short` が空であることを確認）
  spec は「**訪問の番号だけから作らない**」と明記し、Scenario `識別子から URL と訪問時刻と
  プロファイルが読み取れない` の AND は「同じ URL の別の訪問の識別子と、形を表す接頭辞のほかに
  共通する部分を持たない」を要求している。
  `visit_external_id_is_pinned`（contract.rs:101）は名前に反して固定値を持たず、
  `starts_with("v1:")` と `!contains("example")` 等の否定だけで、**2 件目の識別子と比べていない**。
  第 2 回 Q5 の本人の答え（組全体を 1 つのハッシュ）は、値を変えてもどのテストも落ちない。
  独立に再計算した基準値（python `hashlib`、長さ 8 バイト BE + バイト列の順）:
  `chrome / Default / 1 / 2026-09-08T02:00:00.000000Z / https://example.test/yesterday`
  → `v1:d5f356c63f07cc38a7708be2c917e50c5201f9a307e0d4fb91aef88ce049f930`
- kind: technical
- 提案: (a) 既知の入力 1 組に対する 64 桁の固定値を assert する、
  (b) 同じ URL の 2 つの訪問の識別子が `v1:` 以外を共有しないことを assert する。

## R6. 移行の `NOT EXISTS` の番人を外しても server の 338 本が全部緑。tasks 2.1 (b) のテストが無い

- 処置: fixed 2.1 — `351387a` の `browser_history_record_id_is_kept_once_records_exist`。code-verify 2 回目が番人を外して FAILED を再現（R47）。

- 成果物: migrations/202609211400_browser_history_record_id.sql、crates/server/src/registry_tests.rs
- 根拠: 移行を
  ```sql
  UPDATE core.source SET external_id_kind = 'record' WHERE logical_source = 'c02-browser-history';
  ```
  （`AND external_id_kind <> 'record' AND NOT EXISTS (SELECT 1 FROM core.event ...)` を削除）に
  差し替えて実行:
  ```
  $ cargo test -p ashiato-server
  test result: ok. 338 passed; 0 failed   （rc=0）
  ```
  （確認後、原状へ戻し `git status --short` が空であることを確認）
  tasks 2.1 は「(b) 記録が 1 件ある状態で値を `'none'` に戻して移行を当て直しても `'none'` のまま」を
  要求しているが、`registry_tests.rs` にその形のテストは無い（`browser_history_record_id` で絞ると 1 本だけ。
  それは (a) の `browser_history_record_id_is_required`）。
  design と移行のコメントが「鍵を変えると既存行との対応が切れる（deep Q3）」と書いている、
  まさにその番人が無検査。
- kind: technical
- 提案: `core.event` に `c02-browser-history` を 1 件入れ、`external_id_kind` を `'none'` に戻して
  `migrate()` を当て直し、`'none'` のままであることを見るテストを足す。

## R7. tasks 2.2 の「5 本」は、同じ本体を呼ぶ 5 つの関数で満たしている

- 処置: fixed 2.2 — 共有の helper を 5 本の独立したテスト（(a)〜(e) をそれぞれ 1 本で検査）に割った。`scripts/verify-run 2.2` PASS（5 passed）。

- 成果物: crates/server/src/registry_tests.rs:198-218
- 根拠: `browser_history_update` / `..._title_keeps_one_row` / `..._title_keeps_version` /
  `..._duration_keeps_version` / `..._stale_does_not_rewind` の 5 本はいずれも本体が
  `assert_browser_history_update().await;` の 1 行で、同じ検査を 5 回走らせている。
  `cargo test -p ashiato-server browser_history_update -- --list` は 5 本を返すが、
  検査している事柄は 1 つ。§0 の R17（本数を見る）を、本数の水増しで通している。
  （なお本体は (a)〜(e) を順に見ており、検査内容そのものは足りている）
- kind: technical
- 提案: 5 本に割るなら fixture を共有した独立のテストにする。1 本のままなら tasks 2.2 の
  「`-- --list` で 5 本」を「5 つの assert を含む 1 本」に直す。

## R8. smoke.sh の ST08 は本物の取得を 1 度も通していない。作った履歴 DB を読まず、手書き JSON を 2 回 POST するだけ

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: tools/smoke.sh:836-881、crates/collector-windows/examples/browser_history_smoke.rs
- 根拠: `grep -n "HISTORY_DB" tools/smoke.sh` →
  ```
  841:HISTORY_DB=$(mktemp)
  842:rm -f "$HISTORY_DB"
  843:cargo run -q ... --example browser_history_smoke -- "$HISTORY_DB"
  844:[ -s "$HISTORY_DB" ] || { ... }
  881:rm -f "$HISTORY_DB"
  ```
  作った履歴 DB は**サイズを見るだけ**で、読み手にも取得にも渡っていない。送っているのは
  smoke.sh の中に直書きした JSON。したがって Scenario `2 回続けて取得しても行が増えない`
  （「続けて 2 回**取得**する」）は、同じ本文を 2 回 POST した server 側の冪等（ST03 が既に持つ）に
  すり替わっている。
  独立に再計算して、この台本が実装と食い違うことも確認した:
  - 台本の `visit_time = 13402627200000000`（Chromium 起点 1601-01-01）を python で換算 →
    **2025-09-18T00:00:00Z**。smoke.sh が assert する `event_time` は `2026-09-08 02:00:00+00`。
    DB を読まないので誰も気づかない。
  - smoke.sh が送る `external_id` は `v1:visit:aaaa…`。実装の `Visit::new` が作るのは
    `v1:` + SHA-256 の 64 桁 hex（`visit:` の節は無い）。`docs/collector-contract.md` の追記も
    `v1:` + ハッシュと書いており、smoke.sh の形だけが別。
- kind: technical
- 提案: smoke.sh から `browser_history_smoke` の DB を実際に読んで送るところまで通す。
  通さないなら Scenario `2 回続けて取得しても行が増えない` / `前日に見たページが翌日の取得で入っている`
  の印を smoke.sh から外す。

## R9. tasks 5.5 / 10.3 が「smoke に足す」と書いた手順が smoke.sh に無い

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: openspec/changes/st08-browser-history/tasks.md（5.5 / 10.3）、tools/smoke.sh
- 根拠: `git diff main...HEAD -- tools/smoke.sh` の追加 44 行に、
  - 5.5 の「サーバを止めて取得 → 起動 → 送信 → psql で件数」に当たる手順が無い
    （`grep -n "止め\|停止\|kill" tools/smoke.sh` の ST08 区画に該当なし。12 行目の `kill` は trap の cleanup）
  - 10.3 の「取得契機を 1 日進めて psql で行を見る」に当たる手順が無い
    （`event_time` を直書きした 1 件を POST し、その `payload->>'url'` を SELECT しているだけ）
  それでも 5.5 / 10.3 は `[x]`。
- kind: technical

## R10. tasks 10.4 で CI の下限を 7 → 10 に上げたが、10.1 / 10.2 のテストが無い。`collector-windows-runtime` は必ず落ちる

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: .github/workflows/ci.yml:126、crates/collector-windows/tests/runtime_windows.rs
- 根拠:
  ```
  $ git diff --stat main...HEAD -- crates/collector-windows/tests/
  （差分なし）
  $ grep -c "#\[test\]" crates/collector-windows/tests/runtime_windows.rs
  7
  $ grep -n "fn browser_history" crates/collector-windows/tests/runtime_windows.rs
  （出力なし）
  ```
  ci.yml は `if ([int]$Matches[1] -lt 10) { Write-Error "実行時テストが ... 本しか走っていない（10 本のはず）"; exit 1 }`。
  実際に走るのは 7 本なので、この job は確実に失敗する。10.4 自身の検証は
  「`collector-windows-runtime` job が緑」と書いてあり、これは成立し得ない。
  加えて `origin/feat/st08-browser-history` が存在せず（`git log origin/... ` が
  `unknown revision`）、CI はこのブランチで一度も走っていない。「検証も通した」に CI は含まれていない。
- kind: technical
- 提案: 10.1 / 10.2 が未着手である以上、下限の引き上げは 10.1 / 10.2 と同じ commit に置く。
  いま入れるなら 10.4 を `[ ]` に戻す。

## R11. 生存信号の 2 本目が組み立てられていない。`counters-browser-history.json` は存在せず、`history_heartbeat` は 4 本（tasks は 5 本以上）

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/heartbeat.rs:27-47・303-340、crates/collector-windows/src/runtime.rs
- 根拠:
  ```
  $ grep -rn "history_signal\|history_blocker\|HISTORY_EXPECTED_GAP_SEC" --include=*.rs crates/
  heartbeat.rs:38 / :313（テスト）   heartbeat.rs:27 / :333,:334（テスト）   heartbeat.rs:35 / :310,:323（テスト）
  $ grep -rn "counters-browser-history" crates/ tools/
  （出力なし）
  $ cargo test --workspace history_heartbeat -- --list
  heartbeat::tests::history_heartbeat_is_daily_and_separate
  heartbeat::tests::history_heartbeat_on_start
  heartbeat::tests::history_heartbeat_reports_unreadable_and_missing
  history::read::tests::history_heartbeat_probes_when_not_read
  （4 本。tasks 8.1 は「5 本以上」）
  ```
  `history_signal` / `history_blocker` は本体（runtime.rs）から呼ばれず、`Runtime` は履歴の
  `Schedule` を持っていない。tasks 8.1 の「注入した時計で Runtime を 2 日回して確かめる」に当たる
  テストは無い（`ticks_keep_heartbeat_and_skew_intervals_for_a_day` は ST07 の既存テストで、差分なし）。
  印の先の中身:
  - `ブラウザ履歴のソースにも想定間隔ごとに生存信号が届く` → `history_signal(...).logical_source` を 1 回見るだけ。
    区間ごとに届くことを見ていない。
  - `ブラウザ履歴の生存信号はウィンドウの生存信号と別の件である` → THEN は「生存信号が **2 件** 届いている」。
    テストは 1 件しか作らない。
  - `履歴が 1 つも見つからなければ取得できないとして報告される` → テストは `NONE_FOUND` を**自分で**
    `BTreeSet` に入れて `capturable == false` を見るだけ。「履歴が 1 つも見つからない」から
    `NONE_FOUND` を立てるコードが存在しない。
- kind: technical

## R12. 中身を確かめないテストが 2 本ある（どちらも Scenario の印付き）

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/exclusion.rs:322-334
- 根拠:
  ```rust
  // Scenario: 取得をやり直しても除外の件数は増えない
  #[test]
  fn history_exclusion_reuses_same_id_on_retry() {
      let id = "v1:excluded:hash";
      assert_eq!(id, "v1:excluded:hash");
  }

  #[test]
  fn history_exclusion_has_no_private_body() {
      let raw = serde_json::json!({"kind":"excluded","excluded_count":1});
      assert!(raw.get("url").is_none() && raw.get("title").is_none());
  }
  ```
  前者は文字列リテラルの自己比較で、製品コードを 1 行も呼ばない。後者はテストの中で作った
  JSON リテラルに url / title が無いことを見ているだけ。
  同じ区画の `history_exclusion_counts_new_match_once`（:311）も、名前に反して
  `hits_history(...) == true` を 1 回見るだけで、印を置いた `履歴で除外した件数が残る`（THEN は
  「その件数から 3 件を読み取れる」）と `除外した訪問は取得のたびに数え直されない`（THEN は
  「1 回だけ数えられている」）のどちらも観測していない。
- kind: technical

## R13. tasks 5.6 / R15 の「Runtime の層で `c02-window` に `suspended` が入らない」を検査していない

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/fetch.rs:315-342
- 根拠: 印を置いた `history_slow_read_does_not_disturb_window` は、`std::thread` を 1 本起こして
  `is_finished()` を 3 回見て `join()` するだけ。`Runtime` も `c02-window` も `suspended` も
  1 度も現れない（`grep -n "suspended\|Runtime" crates/collector-windows/src/history/fetch.rs` が空）。
  tasks 5.6 の本文は「**読みに 3 分かかる読み手を差し込んでも見回りが続き、`c02-window` に
  `suspended` が入らず、履歴の記録を 1 件も `c02-window` に作らない**ことを Runtime の層で固定する」。
  Scenario `履歴の取得でウィンドウのソースの記録は増えない` の 2 つの THEN（件数・眠っていた時間）が
  どちらも未検査。
- kind: technical

## R14. design D1 の置き場の表（6 行）が固定されていない。Edge / Opera のパスを壊しても `history_locate` 3 本が緑

- 処置: fixed 4.1

- 成果物: crates/collector-windows/src/history/locate.rs:24-33・163-205
- 根拠: `Browser::base` の
  `Self::Edge => local.join("Microsoft/Edge/User Data")` を `local.join("WRONG/Edge/User Data")` に、
  `Self::Opera => roaming.join("Opera Software/Opera Stable")` を `roaming.join("WRONG/Opera Stable")` に
  書き換えて実行:
  ```
  $ cargo test -p ashiato-collector-windows --lib history_locate
  test result: ok. 3 passed; 0 failed   （rc=0）
  ```
  （確認後、原状へ戻し `git status --short` が空であることを確認）
  テストの `fixture()`（locate.rs:230）が assert 側と**同じ `Browser::base()`** で木を作るので、
  パスが何であれ `found.len() == 6` は必ず通る。tasks 4.1 が要求する
  「6 種を列挙する assert を含む」は、本数の assert であって置き場の assert ではない。
  実機で置き場が 1 つ間違っていれば、そのブラウザの履歴は「見つからない」まま 90 日で消える。
- kind: technical
- 提案: `base()` の 6 行の戻り値そのものを固定する assert を足す（fixture を通さない）。

## R15. WAL の履歴 DB では写しに 1 行も入らない（`-wal` を写していない）

- 処置: escalated — deep.md Q6（R15）へ premise / loss: uncaptured として追記。

- 成果物: crates/collector-windows/src/history/read.rs:87-99（`with_copy`）
- 根拠: python の sqlite3 で、ブラウザが開いたまま（接続を閉じず checkpoint されていない）状態を再現:
  ```
  mode: ('wal',)
  files: ['wal.sqlite', 'wal.sqlite-shm', 'wal.sqlite-wal']
  copy read error: no such table: visits
  ```
  `with_copy` は `std::fs::copy(source, &copy)` で**本体 1 ファイルだけ**を写す。
  Firefox の `places.sqlite` は既定で WAL。Chromium 系も近年の版は History を WAL で開く。
  この状態では読みが丸ごと失敗し（= そのプロファイルが「読めない」扱い）、
  部分的に checkpoint された途中の状態なら**古いスナップショットを黙って読む**。
  印を置いた `history_copy_is_removed`（read.rs:188）は `rusqlite::Connection::open` の既定
  （journal_mode=delete）で作った DB を使うので、この経路を 1 度も通らない。
  Scenario `ブラウザが動いている間も取得できる` の印がこのテストに乗っている。
- kind: irreversible
- loss: uncaptured

## R16. 読み手が unwind すると、履歴 DB の写しが `%TEMP%` に残る（除外は写しの後なので、除外したプロファイルの URL も残る）

- 処置: escalated — deep.md Q6（R16）へ premise / loss: exported として追記。

- 成果物: crates/collector-windows/src/history/read.rs:87-99
- 根拠: `with_copy` に panic する閉包を渡して実測（一時的にテストを足して実行し、直後に原状へ戻した）:
  ```
  COPY_STILL_EXISTS=true path=/tmp/ashiato-history-copy-db0ba529-ba8a-4724-a412-2bc949d5b5bd.sqlite
  test result: ok. 1 passed
  ```
  `let result = read(&copy);` が unwind すると `std::fs::remove_file(copy)` に到達しない
  （`Drop` による後片付けが無い）。プロセスが落ちた場合も同じ。
  除外（`apply_history_exclusions`）は `Visit` を組んだ**後**に当たるので、この写しには
  `browser-profile` や `url-contains` で除外したはずの URL・題名がそのまま入っている。
  写しは OS の一時ディレクトリにあり、PERM-2 の感度も FR-83 の除外も効かない。
  `history_copy_is_removed` は正常系だけを見ている。
- kind: irreversible
- loss: exported

## R17. 既存（ST07）のウィンドウの感度テストが履歴用に置き換えられ、ウィンドウ側の担保が消えた

- 処置: fixed 9.1

- 成果物: crates/collector-windows/src/contract.rs:448-465
- 根拠: `git diff main...HEAD -- crates/collector-windows/src/contract.rs`:
  ```diff
  -    fn sensitivity_uses_collection_default() {
  -        let p = WindowPayload::new(RecordKind::Foreground, at());
  -        let req = IngestRequest::of(&p, ...)
  +    fn history_sensitivity_uses_collection_default() {
  +        let visit = crate::history::contract::Visit::new(...);
  +        let req = IngestRequest::of_visit(&visit, ...)
  ```
  テストの doc コメントに残る `Scenario: 収集側が厳しい側の感度を付けて送らない`（MODIFIED で
  写された ST07 の既存 Scenario。WHEN は「PC 側の収集が記録を送る」）は、いま
  `IngestRequest::of`（ウィンドウ）を 1 度も通らない。新しい Scenario を足すのに既存の担保を潰している。
  tasks 9.1 の検証も `cargo test history_sensitivity_uses_collection_default` だけを挙げている。
- kind: technical
- 提案: 元の `sensitivity_uses_collection_default` を戻し、履歴用は別のテストとして足す。

## R18. `probe_readable` は成功側しか見ていない。Scenario の WHEN（開けない）と THEN（取得できない状態）が未検査

- 処置: fixed 8.2

- 成果物: crates/collector-windows/src/history/read.rs:104-113・200-207
- 根拠: 印を置いた `history_heartbeat_probes_when_not_read` の本体は
  `assert!(probe_readable(&db).is_ok());` の 1 行。
  Scenario は「見つかったプロファイルの 1 つの履歴 DB が**開けない**状態で ... THEN その生存信号は
  **取得できない状態を示す**」。開けない場合を試しておらず、`probe_readable` の `Err` を
  `history_blocker::unreadable(..)` に変えるコードも存在しない（R11 の grep のとおり）。
  同じ Scenario にもう 1 つ印が乗っている `history_heartbeat_on_start`（heartbeat.rs:322）は
  `Schedule::with_interval(86400).due(t(0))` を見るだけで、写しにも `SELECT 1` にも触れていない。
- kind: technical

## R19. tasks 11.2 の `python3 scripts/check_scenarios.py .` は rc=1

- 処置: fixed 11.2 — `python3 scripts/check_scenarios.py .` rc=0 を再現（2026-10-05。st06 / st12 側が埋まった）。

- 成果物: openspec/changes/st08-browser-history/tasks.md（11.2）
- 根拠:
  ```
  $ python3 scripts/check_scenarios.py .
  scenarios: FAIL (担保なし 148 件 / 名無しの確認待ち 0 件)   rc=1
  $ python3 scripts/check_scenarios.py . st08-browser-history
  Scenario 495 件 / 印 582 個 / 担保あり 495     scenarios: OK   rc=0
  ```
  148 件はすべて st12-archive-ingestion / st06-app-usage 由来で、ST08 が壊したものではない。
  ただし 11.2 は「`python3 scripts/check_scenarios.py .` rc=0」と書いた形のまま `[x]` になっている。
  （main でも同じ FAIL が出るので ST08 の回帰ではない）
- kind: technical
- 提案: 11.2 の本文を `... . st08-browser-history` に直す。

## R20. `tools/check-licenses.sh` は rc=1（1.1 / 11.5 の検証が満たされていない）

- 処置: fixed 11.5 — `tools/check-licenses.sh` rc=0 を再現（2026-10-05。node_modules を揃えた後）。

- 成果物: openspec/changes/st08-browser-history/tasks.md（1.1 / 11.5）
- 根拠:
  ```
  $ tools/check-licenses.sh ; echo rc=$?
  == Rust の依存
    314 件を確認 / 不許可 0 件
  == Node の依存
    NG web/node_modules が無い（この検査は空振りしている）
  rc=1
  ```
  `rusqlite`（`bundled`）を足したのは 1.1 で、その検証に `tools/check-licenses.sh rc=0` が挙がっている。
  Rust 側は 314 件・不許可 0 件で通っているので、新しい依存そのものは問題ない。
  落ちているのは Node 側の空振り検知で、手元では `cd web && npm ci` が要る（CI では web job が持つ）。
  申告の「検証も通した」に、この rc=1 は含まれていない。
- kind: technical

---

# `pr-review-toolkit` の 3 agent から（R21〜）

`code-reviewer` / `pr-test-analyzer` / `silent-failure-hunter` を `git diff origin/main...HEAD` に当てた。
延べ 51 件のうち、上の R1〜R20 と重なるものを畳んで残ったのが以下。

## R21. 外部識別子の形が design D6 の逐語と違う（種別の接頭辞が無く、組も違う）

- 処置: escalated — deep.md Q6（R21）へ premise / loss: rewrite-all として追記。

- 成果物: `crates/collector-windows/src/history/contract.rs:35-44` / `docs/collector-contract.md:218-226`
- 根拠: D6 は `v1:visit:<sha256(family \x1f browser \x1f profile_dir \x1f visit_id \x1f visit_time_raw \x1f url)>`。
  実装は `format!("v1:{:x}", hasher.finalize())` で **`visit:` の種別が無い**。ハッシュの入力も
  `family` と `visit_time_raw` が入らず、区切りは `\x1f` でなく長さ前置。`tools/smoke.sh:846` は
  `v1:visit:…` を使っており、design / 実装 / 縦串の 3 者が三様。`docs/collector-contract.md` の追記は
  実装側に合わせて書かれていて、design と食い違ったまま文書化されている。
- kind: irreversible
- loss: rewrite-all

> 種別の接頭辞が無いと `vanished` / `excluded` / `profiles`（R3）の識別子空間と分かれない。
> `gates.sql` が `external_id` の書き換えを拒むので、1 行でも入った後に直すと既存行との対応が切れる。
> **R3 のとおり他の 3 種がまだ無いので、いまなら直せる。**

## R22. `exe-path` の除外登録がブラウザ履歴に一度も当たらない

- 処置: escalated — deep.md Q6（R22）へ premise / loss: exported として追記。

- 成果物: `crates/collector-windows/src/exclusion.rs:134-136`
- 根拠: `Rule::ExePath { value } | Rule::ProcessName { value } => value.eq_ignore_ascii_case(process)`。
  `process` は `"chrome.exe"` 等の裸のプロセス名（同 `:121-129`）だが、`exe-path` の `value` は
  フルパスの完全一致（`exclusion.rs:67` の前景側、`README.md:78` の例 `C:\Program Files\...\KeePassXC.exe`）。
  `C:\...\chrome.exe` が `chrome.exe` と一致することはない。design D11 の表は `exe-path` / `process-name`
  の**両方**を全プロファイルに写すと決めている。確かめるテスト `history_exclusion_process_covers_profiles`
  （`exclusion.rs:311`）は `ProcessName` しか使っていない。
- kind: irreversible
- loss: exported

> 本人が `exe-path` でブラウザを除外したつもりでも履歴は素通りし、URL と題名が
> **既定の感度（PERM-3 = 外部 AI 可。扉 #15）**で入る。効かなかったことはログにも生存信号にも出ない。

## R23. 履歴 DB の写しが置き場ではなく `%TEMP%` に作られ、削除の失敗を握りつぶす

- 処置: escalated — deep.md Q6（R23）へ premise / loss: exported として追記。

- 成果物: `crates/collector-windows/src/history/read.rs:91-98`
- 根拠: `std::env::temp_dir().join(...)` へ写し、`std::fs::remove_file(copy).ok()` で消す。
  design D2 は「**置き場の**一時ディレクトリへ写し」「写しは読み終えたら消す（**私的な内容を置き場に残さない**）」。
  `probe_readable`（`read.rs:104`）は生存信号のたびにこれを呼ぶので頻度も高い。
- kind: irreversible
- loss: exported

> R16（unwind で残る）と同じ経路だが、こちらは**正常系でも置き場の外に置いている**ことと、
> **削除失敗が誰にも伝わらない**こと。最大 90 日ぶんの URL と題名の完全な複製。

## R24. `table_recreated` / `profile_gone` が production では決して立たない

- 処置: escalated — deep.md Q6（R24）へ premise / loss: uncaptured として追記。

- 成果物: `crates/collector-windows/src/history/fetch.rs:62-65` / `ledger.rs`
- 根拠: `ledger.max_visit_id` と引数を比べているが、`Ledger::max_visit_id` に**書き込むのはテストだけ**
  （`fetch.rs:353`）。`read_chromium` / `read_firefox` は最大番号を返さず、`Ledger` に設定する関数も無い。
  design D10 の「Chromium は `sqlite_sequence` の値でも見る」は実装されていない。`profile_gone` も同様。
- kind: irreversible
- loss: uncaptured

> 全期間の削除で表が作り直されたとき、数万件の `vanished` が全部 `table_recreated: false` で残る。
> Q2 で本人が選んだのは「経路は判定しないが手がかりは載せる」なので、
> 手がかりが常に false だと「本人が消した」と読める**誤った事実**が積まれる。`vanished` は訂正できない。

## R25. `locate` が「読めない」を「そのブラウザは無い」に化けさせる

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: `crates/collector-windows/src/history/locate.rs:50, 67, 107-109, 131`
- 根拠: `let Ok(entries) = std::fs::read_dir(base) else { return; }` など 5 か所。戻り値は `Vec` / `BTreeMap` で
  エラーは呼び出し元へ伝わらない。隠れるのは `User Data` への権限拒否・I/O エラー・`profiles.ini` の
  読取り失敗・`Local State` の JSON 破損。
- kind: technical

> design D12 の `history-unreadable:<browser>:<profile_dir>` blocker が**原理的に立たない**。
> 「1 つでも読めなければ取得できない」（Q1 で本人に見せた約束）が機械の側で実現できていない。

## R26. `visits JOIN urls` で `urls` 行を失った訪問が黙って落ち、件数の検算が無い

- 処置: escalated — deep.md 第 4 回 Q8（loss: uncaptured。URL の行が無い訪問を捨てるか入れるか）。

- 成果物: `crates/collector-windows/src/history/read.rs:41, 63`
- 根拠: `... FROM visits v JOIN urls u ON u.id=v.url ORDER BY v.id`。読んだ件数と
  `SELECT count(*) FROM visits` を突き合わせる箇所が無い。
- kind: technical
- loss: uncaptured

> 落ちた訪問は `seen` に入らないので、**帳面にあれば次の取得で「消えた」に化ける**
> （本人は消していないのに `vanished` が残る）。取りこぼした件数を誰も知らない。

## R27. 壊れた帳面の退避が無言

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: `crates/collector-windows/src/history/ledger.rs:64-70`
- 根拠: `path.with_extension("broken.ledger")` へ `rename` するだけで、ログも blocker も数えも付かない。
  ウィンドウ側は同じ型の事故に `telemetry::line("counters_quarantined", ...)`（`runtime.rs:128-130`）を出している。
- kind: technical

> 帳面が壊れた回は「消えた」の比較基準が消える。D10 の見積もりは「1 回ぶん」だが、実際には
> **壊れた瞬間から次の読みまでに消えた訪問は永久に検出されない**。本人が知る手段が無い。

## R28. 履歴側の失敗が 1 行もログに出ない（`telemetry::history_line` が呼ばれていない）

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: `crates/collector-windows/src/telemetry.rs` / `crates/collector-windows/src/history/`
- 根拠: `grep -rn "telemetry\|info(" crates/collector-windows/src/history/` → 0 件。
  `grep -rn "history_line" --include=*.rs crates/ | grep -v src/telemetry.rs` → 0 件。
- kind: technical

> tasks 9.2 の Scenario「履歴の読み取りの失敗がログに出ても URL と題名は出ない」は
> **ログが存在しないので真空で成立**している。漏らさない決定は守られているが、漏らす前に出す口が無い。

## R29. Firefox のプロファイル表示名が構造的に読めない（`profiles.ini` の場所が 1 段違う）

- 処置: fixed 4.2

- 成果物: `crates/collector-windows/src/history/locate.rs:30, 44-50`
- 根拠: `Browser::Firefox.base()` は `roaming/Mozilla/Firefox/Profiles`。`profile_names` は
  `base.join("profiles.ini")` を読むので `…/Firefox/Profiles/profiles.ini` を探すが、実際は
  `…/Firefox/profiles.ini`（`locate()` 自身は `locate.rs:90` でそちらを読んでいる）。
  Firefox の `Name=` は常に空になる。単体 `history_profiles_map` は Chrome しか通していない。
- kind: technical

## R30. `profiles.ini` の `IsRelative` をファイル全体で 1 つと解釈している

- 処置: fixed 4.1

- 成果物: `crates/collector-windows/src/history/locate.rs:134`
- 根拠: `let relative = !text.lines().any(|line| line.trim() == "IsRelative=0");`。
  `IsRelative` は `[ProfileN]` セクションごとの値。絶対パスの profile が 1 つでもあると、
  以降すべての `Path=` が絶対扱いになる（逆も同様）。design D1 は「`Path=`（相対 / 絶対）の和」を要求。
- kind: technical

## R31. `read.rs` の `duration_us` が NULL の行で型変換エラーになる

- 処置: fixed 4.3

- 成果物: `crates/collector-windows/src/history/read.rs:53`
- 根拠: `duration_us: Some(r.get(6)?)`。Chromium の `visit_duration` は NULL を取りうる。
  `ReadVisit` は `is_known_to_sync` も持たない（design D4 の項目）。
- kind: technical

## R32. 除外の「件数」系と「後から足す」の Scenario が、主張を確かめていない

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: `crates/collector-windows/src/exclusion.rs:303-308` / `crates/collector-windows/src/history/fetch.rs:443-456`
- 根拠: `history_exclusion_counts_new_match_once` は 2 つの印を持つが本体は
  `assert!(history_rules().hits_history(...))` だけで、件数も再計上も見ていない。
  `history_exclusion_added_later` は `mark_queued` した訪問と**同一の**訪問をそのまま渡しており、
  spec の WHEN「その訪問の**題名が変わった後**に取得契機に達する」を作っていない。
- kind: technical

## R33. `history_read_chromium` が滞在時間と遷移の種類を assert していない

- 処置: fixed 4.3

- 成果物: `crates/collector-windows/src/history/read.rs:139-157`
- 根拠: 5 つの Scenario 印を持つが assert は `v.len()==2` / `url` / `title` / `from_visit` の 4 つ。
  `duration_us`（投入 7・8）と `transition`（投入 3・4）は検査されない。`SELECT` の列番号
  （`r.get(4)` / `r.get(6)`）がずれてもテストは緑のまま。
- kind: technical

## R34. `history_foreign_visits` が `device_id` を見ていない

- 処置: fixed 4.4

- 成果物: `crates/collector-windows/src/history/contract.rs:132-148`
- 根拠: Scenario の THEN は「その記録の端末は、履歴を読んだ PC である」だが assert は
  `foreign.external_id == local.external_id`。`device_id` は `IngestRequest::of_visit` が決めるのに、
  同期訪問を `of_visit` へ通す assert が無い。`with_originator` は `payload` の 2 欄を差すだけなので
  `external_id` が一致するのは実装上の恒等式。
- kind: technical

## R35. `history/` だけ周囲の流儀（design の D 番号・`仮` の明示・実測値）が抜けている

- 処置: escalated — 以前の rejected（「Q6 待ち」）は Q6 の回答で成り立たなくなった（R48）。deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: `crates/collector-windows/src/history/{locate,read,ledger,fetch,contract}.rs`
- 根拠: `grep -rn "design D\|仮\|R[0-9]" crates/collector-windows/src/history/*.rs` → `mod.rs:6` の 1 行のみ。
  同じクレートの既存ファイルは `autostart.rs:2`（`design D7・**仮**`）、`platform.rs:19`、`engine.rs:8`、
  `clock.rs:77` のように出所と「仮」を必ず書いている。ずれている箇所: `fetch.rs:9-10` の
  `HISTORY_INTERVAL` / `HISTORY_RETRY_INTERVAL`、`fetch.rs:138` の `vanished_chunks` の 1,000 件
  （D10 で**仮**かつ反転条件付き）、`ledger.rs` 全体。
- kind: technical

## R36. `origin/feat/st08-browser-history` が存在せず、この実装で CI が一度も走っていない

- 処置: fixed 10.4

- 成果物: （リポジトリ運用）
- 根拠: `git ls-remote --heads origin feat/st08-browser-history` → 0 件。
- kind: technical

> `tasks.md` 10.4 の検証「`collector-windows-runtime` job が緑」は、**job が一度も動いていない**
> 状態で `[x]` になっている（R10 のとおり、動けば必ず落ちる）。

---

## final review（7d08921..3b86eb4）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`。review package `.superpowers/sdd/tasks/review-7d08921..3b86eb4.diff`）。
判定: **No**。前回の検証（96ac2f2）の後に入ったのは `ef3d145`（locate / read / contract の小修正）・tasks 見出しの整形・Q6 の問いと答えだけで、
**Q6「決めたとおりに繋ぎ直し、識別子も設計どおりに直す」は実装に入っていない**。tasks 3〜10 の `[x]` は実態を表していない。
良い点（reviewer）: 移行の冪等の番人と対称な `.down.sql`、サーバの取り込みを変えない結合テスト、`queue_then_save` の順序、
帳面に URL と題名を持たない形、`window_request_body_is_unchanged` / `window_sensitivity_uses_collection_default` の追加（R4 / R17 解消）。

## R37. 履歴の収集経路が今も Runtime に繋がっていない（R1 / Q6 が未実装）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/runtime.rs、crates/collector-windows/src/history/
- 根拠: `grep -rn "history::" --include=*.rs crates/collector-windows/src | grep -v src/history/` → crates/collector-windows/src/contract.rs:268（`of_visit`）とそのテストだけ。`HistoryWorker` / `HistorySchedule` / `LedgerStore` / `locate` / `read_*` に本番の呼び手が無い
- kind: technical
- loss: uncaptured

## R38. 訪問の識別子が design D6 の式と違う（R21 が未処置）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/contract.rs
- 根拠: crates/collector-windows/src/history/contract.rs:38 が `v1:<sha256(len‖browser, len‖profile, len‖visit_id, len‖at, len‖url)>`。design.md:141 は `v1:visit:<sha256(family \x1f browser \x1f profile_dir \x1f visit_id \x1f visit_time_raw \x1f url)>`。`visit_external_id_is_pinned` は式を固定していない（R5）
- kind: technical
- loss: rewrite-all

## R39. `vanished` / `excluded` / `profiles` の記録と `ReadVisit` → `Visit` の変換が無い（R2 / R3）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/contract.rs、crates/collector-windows/src/history/fetch.rs
- 根拠: crates/collector-windows/src/history/contract.rs の `kind` は `visit` 固定。`duration_ms` / `transition` / `referrer` を設定する経路が無く常に `None`（contract.rs:31-33）。D4 の `family` / `profile_dir` / `visit_id` / `visit_time_raw` / `transition_core` を持たない
- kind: technical
- loss: uncaptured

## R40. CI の `collector-windows-runtime` の下限 10 に対してテストは 7 本で、job が必ず落ちる（R10）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: .github/workflows/ci.yml、crates/collector-windows/tests/runtime_windows.rs
- 根拠: .github/workflows/ci.yml:123 が `-lt 10`。`tests/runtime_windows.rs` に `browser_history_*` は 0 本（10.1 / 10.2 は `[ ]` のまま 10.4 だけ `[x]`）
- kind: technical

## R41. `exe-path` の除外登録がブラウザ履歴に当たらない（R22）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/exclusion.rs
- 根拠: crates/collector-windows/src/exclusion.rs:134 が `ExePath` の値（フルパス）を `"chrome.exe"` 等のプロセス名と `eq_ignore_ascii_case` で比べる。一致しないので、`exe-path` でブラウザを除外した本人の URL と題名が送られる
- kind: technical
- loss: exported

## R42. 写しが `-wal` / `-journal` を取らない（R15）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/read.rs
- 根拠: crates/collector-windows/src/history/read.rs:89 の `with_copy` が本体だけを写す。design D2 は `History-journal` / `History-wal`（Firefox は `places.sqlite-wal`）も写すと決めている
- kind: technical

## R43. 写しを `%TEMP%` に作り、削除の失敗を握りつぶし、unwind で写しが残る（R16 / R23）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/read.rs
- 根拠: crates/collector-windows/src/history/read.rs:93-100。design D2 は置き場の一時ディレクトリ。除外したプロファイルの URL もディスクに残りうる
- kind: technical

## R44. `table_recreated` / `profile_gone` を計算する呼び手が無い（R24）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/fetch.rs
- 根拠: `detect_vanished` は手がかりを引数で受けるだけで、本番で計算するコードが無い（R37 と同根）
- kind: technical

## R45. Scenario の印が付いたテストが THEN を観測していない（R11 / R12 / R13 / R32 / R34）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/fetch.rs、crates/collector-windows/src/exclusion.rs、crates/collector-windows/src/heartbeat.rs、crates/collector-windows/src/history/contract.rs
- 根拠: `history_slow_read_does_not_disturb_window` は Runtime も `c02-window` も見ない（tasks 5.6 は「Runtime の層で」）。`history_exclusion_counts_new_match_once` は件数を見ない。`history_heartbeat_*` は汎用の `Schedule` と定数だけで `counters-browser-history.json` が無い。`history_foreign_visits` は `device_id` を見ない
- kind: technical

## R46. smoke.sh の ST08 は本物の取得を通さず、固定値も食い違う（R8 / R9）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: tools/smoke.sh
- 根拠: tools/smoke.sh:885-927 は作った履歴 DB を読まず手書き JSON を POST する。`visit_time=13402627200000000` は 2025-09-18 だが smoke は 2026-09-08 を期待する
- kind: technical

## R47. 移行の `NOT EXISTS` の番人を固定するテスト（tasks 2.1 (b)）が無い（R6）

- 処置: fixed 2.1 — `351387a` の `browser_history_record_id_is_kept_once_records_exist`。code-verify 2 回目が番人を外して FAILED を再現（R47）。

- 成果物: crates/server/src/registry_tests.rs
- 根拠: `registry_tests.rs` にあるのは 2.1 の (a) と (d) だけ。番人を外しても server のテストは全部緑（R6 の実測）
- kind: technical

## R48. tasks 3〜10 の `[x]` と、「R1 / Q6 待ち」を理由にした rejected が実態と食い違う

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: openspec/changes/st08-browser-history/tasks.md、openspec/changes/st08-browser-history/review/code.md
- 根拠: deep.md の Q6「何が変わるか」が「tasks の 3〜10 章をやり直す（完了の印は検証の証跡で付け直す）」。R5 / R8 / R9 / R10 / R11 / R12 / R13 / R25 / R27 / R28 / R32 の rejected の理由は Q6 の回答で成り立たなくなった
- kind: technical

## R49. Minor: 負の時刻 1 行で読み全体が失敗・`JOIN` の欠落と退避が無言・本番の `.expect`

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/read.rs、crates/collector-windows/src/history/ledger.rs、crates/collector-windows/src/history/fetch.rs
- 根拠: crates/collector-windows/src/history/read.rs:40-55（負の時刻で `InvalidQuery`）、crates/collector-windows/src/history/ledger.rs:69（`*.broken.ledger` を上書き・ログ無し。R26 / R27 / R28）、crates/collector-windows/src/history/fetch.rs:118 と :180 の `.expect`
- kind: technical

---

## code-verify（2 回目。HEAD `351387a` ＋ 作業ツリーの未コミット差分）

対象: worktree `/home/yosis/dev/ashiato2-st08`（`feat/st08-browser-history`）。HEAD は `351387a`（origin は `6768425`。3 commit が未 push）。
**作業ツリーに未コミットの差分がある**（`crates/collector-windows/src/history/{contract,ledger,locate,read}.rs` と `telemetry.rs`。+1232 / −269。
更新時刻 09:09〜09:13、検証の開始時に ST08 を触っているプロセスは無かった）。作業ツリーはビルドできない（R50）ので、
**テストは HEAD を `git archive HEAD` で書き出した複製（`/home/yosis/dev/.st08-verify`）で走らせた**。server は作業ツリーと HEAD で差が無いので作業ツリーで走らせた。
DB: `docker compose up -d --wait db` ＋ `tools/db-roles.sh`（smoke の `down -v` の後に作り直した）。ガードを壊した複製は毎回 `git show HEAD:<file> | diff -q` で原状を確かめた。

### 申告と実測

申告: tasks.md は `- [x]` 36 件 / `- [ ]` 2 件（10.1・10.2）。deep.md 第 3 回 Q6 の答えは「決めたとおりに繋ぎ直し、識別子も設計どおりに直す」で、
「tasks の 3〜10 章をやり直す（完了の印は検証の証跡で付け直す）」と書いている。

| 走らせたもの | 実測 | 申告との一致 |
|---|---|---|
| `cargo build -p ashiato-collector-windows --lib --tests`（作業ツリー） | **rc=101**（error 9 件: `E0425/E0432/E0433 Visit` が無い、`E0599 Opened::ledger` が無い、`E0308`） | **不一致**（R50） |
| `cargo fmt --all --check`（作業ツリー / HEAD） | **rc=1** / rc=0 | 作業ツリーは不一致（R50） |
| `tools/smoke.sh`（作業ツリー / HEAD） | **rc=101**（collector のコンパイル失敗）/ rc=0 | 作業ツリーは不一致（R50）。HEAD の中身は R46 のまま |
| `cargo test -p ashiato-collector-windows`（HEAD） | rc=0（lib 142 本） | 一致 |
| `cargo test -p ashiato-server`（.env を読み込んで） | rc=0（409 + 7 本） | 一致 |
| `cargo clippy --workspace --all-targets -- -D warnings`（HEAD） | rc=0 | 一致 |
| `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings`（HEAD・1.2） | rc=0 | 一致 |
| tasks に名前のある `cargo test <名前> -- --list` 33 種（collector）＋ 3 種（server） | すべて 1 本以上（`browser_history_update` 5 / `history_heartbeat` 5 / `history_vanished` 9 / `history_exclusion` 9 / `history_locate` 4 / `history_schedule` 3） | 本数は一致 |
| `openspec validate st08-browser-history --strict`（11.1） | rc=0 | 一致 |
| `python3 scripts/check_scenarios.py .` / `… . st08-browser-history`（11.2） | rc=0 / rc=0 | 一致（前回 R19 の rc=1 は解消）。中身は R54 |
| `python3 scripts/check_chain.py .`（11.3） | rc=0 | 一致 |
| `python3 scripts/review_triage.py . st08-browser-history`（11.4） | **rc=1**（`triage: FAIL (32 件)`。R37〜R49 に処置が無い） | **不一致**（R52） |
| `tools/check-immutable.sh` / `tools/check-migrations.sh` / `tools/check-licenses.sh`（11.5） | rc=0 / rc=0 / rc=0（Rust 315 件・Node 257 件。前回 R20 の rc=1 は解消） | 一致 |
| PR #14 の CI（head `6768425`） | **`collector-windows-runtime` fail**（「実行時テストが 7 本しか走っていない（10 本のはず）」）/ **`chain` fail**（`tools/check-db-secret.sh`: `NG tools/record-env.sh:58`） | **不一致**（R52 / R53） |
| 本番の呼び手 `grep -rn "history::" crates/collector-windows/src \| grep -v src/history/`（HEAD・作業ツリー） | どちらも `contract.rs:268`（`of_visit`）とそのテストだけ | Q6 の答えは未実装（R51） |
| ガードを壊す: 移行の `NOT EXISTS` を外す | `browser_history_record_id_is_kept_once_records_exist` が FAILED（原状で 2 本 ok） | 一致（R47 / R6 は解消） |
| ガードを壊す: 識別子を `for part in [id.as_str()]` に（HEAD） | **142 本全部 ok** | **不一致**（R38 / R5 のまま。作業ツリーの版は手 1） |
| ガードを壊す: Edge の置き場（`locate.rs:27` だけ）を `Microsoft/EdgeX` に | 1 本 FAILED | 一致（R14 は解消） |
| 一時テスト: `exe-path` = `C:\Program Files\Google\Chrome\Application\chrome.exe` で `hits_history("chrome",…)` | **FAILED**（当たらない） | **不一致**（R41 のまま。作業ツリーも `exclusion.rs` に差分なし） |
| 一時テスト: 時計が 1 年先の成功の後に戻る | **365 日 1 度も `due` にならない** | 新規（R55） |

### 手 1: 固定値を独立に再計算する

- HEAD: `visit_external_id_is_pinned` は今も固定値を持たない（`starts_with("v1:")` と否定の `contains` だけ）。上の表のとおり式を番号だけにしても 142 本緑 —— **R38 / R5 のまま**。
  `visit_payload_shape_is_pinned` が 1 文字単位で固定している形は `profile`（D4 は `profile_dir`）で、`family` / `visit_id` / `visit_time_raw` が無い —— **R39 のまま**（設計と違う形を固定している）
- 作業ツリー（未コミット・ビルド不可）の 3 つの固定値は python の `hashlib.sha256` で独立に再計算して**一致した**:
  `chromium\x1fchrome\x1fDefault\x1f7\x1f13402627200000001\x1fhttps://example.test/a?q=x` → `981e93fe…ecefa1`、
  `chrome\x1fDefault\x1fv1:visit:a\x1fv1:visit:b` → `1803175b…fd469d8`、`chrome\x1fDefault\x1e個人\x1fProfile 1` → `5c4f4852…e8db`。
  **commit されてビルドが通れば R38 は塞がる**が、現時点では走らない（R50）
- smoke: 台本の `visit_time = 13402627200000000` は python で `2025-09-18 00:00:00`、smoke の期待は `2026-09-08 02:00:00+00`、
  識別子は手書きの `v1:visit:aaaa…`。`$HISTORY_DB` は `[ -s ]` と `rm` にしか使われない —— **R46 のまま**（HEAD の smoke は rc=0 で通る）

### 手 2: ガードをわざと壊す

上の表の 4 件。新しく塞がったのは移行の番人（R47）と置き場の表（R14）。識別子（R38）と `exe-path`（R41）は壊しても / 当てても落ちない。
`check-licenses.sh` は今回 Rust・Node を数えて rc=0（Android は「対象外」と明示して見ていない。前回までと同じ範囲）。

### 手 3: Scenario と test

`check_scenarios.py` は rc=0。印の先が THEN を観測していないものは R54（前回 R45 に挙がったものを除いた新しい分と、10.1 が `[ ]` なのに担保ありになる 1 件）。

### 手 4: 本人の決定が test で固定されているか

HEAD の複製で 1 つずつ書き換えて `cargo test -p ashiato-collector-windows --lib` を走らせた:
`HISTORY_INTERVAL` 24h→23h / `HISTORY_RETRY_INTERVAL` 1min→5min / `chunks(1000)`→`999` / `HISTORY_EXPECTED_GAP_SEC` 86400→21600 は**それぞれ 1 本落ちる**。
`SecondsFormat::Micros`→`Millis` は 2 本落ちる。値の固定はある。
ただし**どれも本番の経路から呼ばれていない**（`HISTORY_EXPECTED_GAP_SEC` の参照は `heartbeat.rs:310/316/323` の tests だけ、`HistorySchedule` は呼び手が無い）ので、
固定されているのは「使われない定数の値」である（R51 / R11 と同根）。第 2 回 Q5（組全体のハッシュ）は HEAD では固定されていない（手 1）。

### 手 5: tasks の `[x]` と実体 → R52

### 手 6: 隙間 → R55（時計が戻る）。プロセスの再起動・権限の拒否・件数の不一致は前回の R25 / R26 / R27 から状態が変わっていない（`locate.rs` の差分は名前の関数の追加だけ、`fetch.rs` は無変更）。

---

## R50. 作業ツリーに未コミットの差分（5 ファイル・+1232 / −269）があり、ビルドできない。fmt も smoke も落ちる

- 処置: fixed 11.6 — 未コミットの差分（ビルド不可）は捨てずに `refs/wip/st08-final-partial`（`eb4e889`）と `.superpowers/sdd/st08-task-3/final-review-partial.patch` に退避し、作業ツリーを HEAD に戻した。D6 の識別子・`-wal` の写しなどの下書きとして Task 3〜5 のやり直しで参照できる。

- 成果物: crates/collector-windows/src/history/contract.rs / ledger.rs / locate.rs / read.rs、crates/collector-windows/src/telemetry.rs
- 根拠: `git status --short` が 5 ファイルの ` M`。`cargo build -p ashiato-collector-windows --lib --tests` rc=101
  （`error[E0425]: cannot find type Visit in module crate::history::contract`、`error[E0599]: no method named ledger found for struct Opened`（fetch.rs:310）など 9 件。
  `fetch.rs` は `contract::Visit` と `LedgerStore::ledger()` を使うが、差分が両方を消した）。`cargo fmt --all --check` rc=1。
  作業ツリーで `tools/smoke.sh` rc=101（`browser_history_smoke` の例のビルドで同じ error）。
  差分は Q6 の一部（D6 の識別子・`-wal`/`-journal` の写し・失敗の種別のログ）に当たるが、`runtime.rs` / `engine.rs` / `main.rs` / `exclusion.rs` / `fetch.rs` には触れていない
- kind: technical
- 提案: 差分を持ち主の Task として続けて commit まで通すか、捨てるかをグラフ側で決める。このまま `finish` / `publish` に進めると、
  作業ツリーで走る検証（11.5 / 11.6 / Task gate）は全部落ちる。

## R51. Q6 の答え（繋ぎ直す）が HEAD にも作業ツリーにも入っていない。履歴は今も 1 件も取得されない

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/runtime.rs、crates/collector-windows/src/engine.rs、crates/collector-windows/src/main.rs、crates/collector-windows/src/history/
- 根拠: `grep -rn "history::\|use crate::history" --include=*.rs crates/collector-windows/src | grep -v src/history/` は HEAD・作業ツリーとも
  `contract.rs:268` / `:494` / `:514`（`of_visit` とそのテスト）だけ。`grep -n history crates/collector-windows/src/runtime.rs crates/collector-windows/src/main.rs` は空。
  Q6 の答えは 2026-10-05 08:57 に記録（`3b86eb4`）され、その後の commit は `95a7263`（指摘）と `351387a`（2.1 (b) のテスト）だけ
- kind: technical
- loss: uncaptured （Chromium 系は 90 日を過ぎた履歴を手元から消す。R1 / R37 と同じ。Q6 で本人が「繋ぎ直す」を選んだので、もう問いではない）
- 提案: tasks 3〜10 の `[x]` を外し（R52）、Q6 の範囲を Task として回す。最初の Task は Runtime から取得契機 → 置き場 → 写し → 読み → 除外 → 契約 → outbox → 帳面を 1 本通すこと。

## R52. tasks の `[x]` のうち、本文の検証が今の木で成り立たないもの（R48 の実測の内訳）

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: openspec/changes/st08-browser-history/tasks.md、.github/workflows/ci.yml、tools/smoke.sh
- 根拠:
  - **10.4**「`collector-windows-runtime` job が緑」→ PR #14（head `6768425`）の job 111448515935 が fail。ログ 689 行目
    `Write-Error: 実行時テストが 7 本しか走っていない（10 本のはず）`。`grep -c "#\[test\]" crates/collector-windows/tests/runtime_windows.rs` = 7、`fn browser_history` は 0
  - **11.4** `python3 scripts/review_triage.py . st08-browser-history` → rc=1（`triage: FAIL (32 件)`）
  - **8.1**「数え（`counters-browser-history.json`）」→ `grep -rn counters-browser-history crates/` が空
  - **3.1**「`VisitPayload`（`visit` / `vanished` / `excluded` / `profiles`）…`external_id` は D6 の形」→ HEAD の `kind` は `"visit"` 固定、式は D6 と違う（手 1）
  - **4.2**「`profiles` 記録を作る」→ HEAD に `profiles` 記録を作るコードが無い（`grep -rn '"profiles"\|fn profiles' crates/collector-windows/src` が空）
  - **5.5**「サーバを止めて取得 → 起動 → 送信」→ smoke.sh の ST08 節（885〜927 行）に停止・起動の手順が無い
  - **5.3 / 6.3 / 9.1 / 10.3** → smoke は履歴 DB を読まない（手 1。R46）
  - **11.5 / 11.6** → HEAD では rc=0、作業ツリーでは smoke・fmt・build が落ちる（R50）
- kind: technical
- 提案: deep.md Q6 の「完了の印は検証の証跡で付け直す」どおり、3〜10 の印を外す（`task_mark.py` の側で）。10.4 は 10.1 / 10.2 と同時でなければ付けられない。

## R53. CI の `chain` job が main 由来の `tools/check-db-secret.sh` で落ちている（ST08 の変更ではない）

- 処置: followup ST28 — main 由来（`tools/record-env.sh:58` は `bd7ce65`。ST08 の差分に無い）。docs/handoff/ST28.md に書いた。

- 成果物: tools/record-env.sh、tools/check-db-secret.sh
- 根拠: 手元の `tools/check-db-secret.sh` rc=1（`NG tools/record-env.sh:58` / `error: DB の合言葉が字面で書かれている`）。
  `git diff --quiet HEAD origin/main -- tools/record-env.sh tools/check-db-secret.sh` は差が無く、`tools/record-env.sh` の最終変更は `bd7ce65`（ST22・ST06・ST28・ST05 の録画）。
  PR #14 の job 111448487698 は同じ行で落ちている
- kind: defer
- 提案: main 側の `fix/` で直す（ST08 の change で直さない）。直るまで ST08 の `merge_gate.sh` は CI で落ちる。

## R54. Scenario の印が THEN を観測していないもの（R45 に無い分）。10.1 が `[ ]` でも担保ありに数えられる

- 処置: escalated — deep.md 第 3 回 Q6 で回答済み（決めたとおりに繋ぎ直し、識別子も設計どおりに直す）。Q6 の「何が変わるか」どおり tasks 3〜11 の impl の印を外し、Task 3 から回し直す（deep.md「Q6 の答えが覆う、その後の指摘」）。

- 成果物: crates/collector-windows/src/history/read.rs、crates/collector-windows/src/history/fetch.rs、crates/collector-windows/src/heartbeat.rs
- 根拠:
  - `ブラウザが動いている間も取得できる`（THEN: 記録が入っている）の唯一の印は `read.rs:203` の `history_copy_is_removed`。
    Linux で `rusqlite::Connection::open` を開いたまま写しを取り、**写しが消えたこと**だけを assert する。Chromium が Windows で取る排他ロックも、記録も見ない。
    その Scenario を担保するはずの 10.1（`browser_history_while_running`）は `[ ]` で存在しないのに、`check_scenarios.py` は rc=0
  - `取り込み口が止まっている間に取得した履歴が後から届く`（THEN: 後から格納される）→ `fetch.rs:281` の `history_success_only_after_outbox` は帳面の中身だけを見る。取り込み口も outbox も通らない
  - `ブラウザ履歴のソースにも想定間隔ごとに生存信号が届く` → `heartbeat.rs:308` は `assert_eq!(HISTORY_EXPECTED_GAP_SEC, 86_400)` と、信号 1 件の `logical_source` だけ。「届く」も「間隔ごと」も見ない
  - `区間に読みが無くても、開けるかを確かめてから報告する` → `heartbeat.rs` の `history_heartbeat_on_start` は `Schedule::due(t(0))` だけ。開けるかの確認と報告の順序を見ない
- kind: technical
- 提案: `ブラウザが動いている間も取得できる` の印は 10.1 のテストに移し、単体側から外す（単体に残すと 10.1 が無くても緑になる）。
  残り 3 件は Runtime に繋いだ後（R51）、Runtime の層か smoke に印を移す。

## R55. 時計が先に進んでいた間に成功すると、戻した後は戻った幅だけ取得しない（1 年なら 365 日）

- 処置: escalated — deep.md 第 4 回 Q7（loss: uncaptured。時計が戻ったときに取得を待つか）。

- 成果物: crates/collector-windows/src/history/fetch.rs:158-164（`HistorySchedule::due`）
- 根拠: HEAD の複製に一時テストを足して実行（実行後に原状へ戻し `diff -q` で確認）:
  `succeeded(2027-10-05)` の後、`2026-10-05` から 365 日ぶん毎日 `due()` を呼ぶ → `PROBE days_blocked=365`、テスト FAILED。
  `due` は `now - last >= 24h` だけで、`last > now`（時計が戻った）を扱わない。生存信号の `Schedule::due`（heartbeat.rs:211）も同じ形だが、
  あちらは失っても次の信号で戻る。履歴は Chromium が 90 日で消すので、**戻った幅が 90 日を超えれば、その間の訪問は後から取れない**。
  `HistorySchedule` は `Serialize` / `Deserialize` を持ち、繋いだ後は前回の成功が保存されて再起動をまたぐ
- kind: technical
- loss: uncaptured
- 提案: `last_success > now` なら取得する（あるいは `last_success` を `now` に丸める）。注入した時計で「戻った直後に取得する」を固定するテストを足す。

## final review（7d08921..2d02000）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`。review package `.superpowers/sdd/tasks/review-7d08921..2d02000.diff`、77 commit）。
判定: **No**。Task 3 からの回し直しで R37 / R38 / R39 / R42 / R44 / R46 / R47 / R51 は解消（`main.rs` → `Runtime::with_history` → `HistoryCollector::tick` → `HistoryWorker` → outbox が繋がり、識別子は D6 の式どおり）。
`cargo test -p ashiato-collector-windows` rc=0（179 passed）。ただし deep.md 第 4 回 Q7 / Q8 の答えが design / tasks / spec に写されないまま回し直されたので、どちらも未実装（使い捨ての worktree で再現）。
良い点（reviewer）: 読みの完了を待たない形（D3）を Runtime の層で固定、4 種の記録と `-wal` / `-journal` の写し、`profile_gone` の実在確認、成功の順序（帳面の後に `last_success`）、生存信号 2 本、smoke が本物の Runtime と読み手で psql まで通す。

## R56. deep.md Q7（前回の成功が今より先ならすぐ取得する）が未実装

- 処置: fixed D3 — `HistorySchedule::due` に `last > now`（0ed3835）。`history_schedule_runtime_fetches_at_once_when_last_success_is_ahead` が Runtime の層で固定。D15 に時計が戻ったときの題名の更新を 1 行

- 成果物: crates/collector-windows/src/history/fetch.rs:160-166（`HistorySchedule::due`）
- 根拠: `due` は `now - last >= 24h` だけ。`last_success` を 400 日目に置いて 0〜364 日目に毎日 `due()` → `days_blocked=365`（reviewer の一時テスト。R55 の実測と同じ）。
  Q7 の「効く先」design D3 / tasks 5.4 に答えが写っていない（`grep -n 'Q7' design.md tasks.md specs/` が空）
- kind: technical
  （答えは deep.md Q7 で出ている。実装するだけ）
- 提案: `last.is_some_and(|l| l > now)` なら due。Runtime の層で「戻った直後に取得する」を固定。design D3 に Q7 を写し、D15（`source_updated_at` が過去になり題名の更新が古い到着として捨てられうる）に一行足す

## R57. deep.md Q8（URL の行が無い訪問は URL 無しで入れ、「消えた」にしない）が未実装

- 処置: fixed D6 — `LEFT JOIN` と `ReadVisit.url: Option<String>`、URL 無しの識別子は D6 の組から URL を抜いた 5 要素、帳面の `slot` で「消えた」にしない（0ed3835）。URL で除外した訪問の除外の引き継ぎ（c964792、D11）。サーバの取り込みは変えていない（smoke で 200 と格納）

- 成果物: crates/collector-windows/src/history/read.rs:66-67（`FROM visits v JOIN urls u`）、:103（`JOIN moz_places p`）、`ReadVisit.url: String`、history/contract.rs の識別子と除外
- 根拠: `urls` に無い `url=99` の訪問を 1 行入れると 2 件のはずが 1 件しか読まれない（reviewer の一時テスト）。帳面にあればこの後 `detect_vanished` で「消えた」に化ける（R26）。
  Q8 の「効く先」design D2 / D10 に答えが写っていない
- kind: technical
  （答えは deep.md Q8 で出ている）
- 提案: `LEFT JOIN` と `url: Option<String>`。D6 の識別子の組での URL 無しの表し方を design D6 に書く。「URL の行が無い訪問が URL と題名を省いた visit として送られ、vanished に出ない」をテストで固定

## R58. `exe-path` の除外登録がブラウザ履歴に当たらない（R41 が未解消）

- 処置: fixed D11 — `ExePath` はファイル名部分で比べる（0ed3835）。`history_exclusion_exe_path_covers_all_profiles`

- 成果物: crates/collector-windows/src/exclusion.rs:134 付近（`hits_history`）
- 根拠: `ExePath` の値（フルパス）を `"chrome.exe"` と `eq_ignore_ascii_case` で比べる。`exe-path: C:\Program Files\Google\Chrome\Application\chrome.exe` で `hits_history("chrome", …)` が false（reviewer の一時テスト）
- 影響: README の手順どおり exe-path で除外した人の URL と題名が送られる（D11 の表に反する。決め直す問いではなく、決めたとおりに当たっていない不具合）
- kind: technical
- 提案: 値のファイル名部分（`\` / `/` の最後）で比べる。runtime の `history_exclusion_*` に exe-path の 1 本を足す

## R59. 読めないプロファイルが 1 つ常にあると、全プロファイルを 1 分ごとに永久に読み直す

- 処置: fixed D3 仮 — 不正な行は飛ばして `skipped` に数える。試し直しは 1 分から倍々で 1 時間まで（反転条件は D3）（0ed3835）

- 成果物: crates/collector-windows/src/history/collect.rs:306（`ensure!(unreadable == 0)`）、history/read.rs:86 / :112（負の `visit_time` で `ok_or(InvalidQuery)?`。R49 の残り）
- 根拠: 1 プロファイルの失敗で `schedule.failed` → 1 分後に再試行し、`last_success` が進まない。負の時刻 1 行・表の無い `History`・壊れた古いプロファイルのどれでも、全 DB の写しと全件読みが 1 日 1,440 回走る
- kind: technical
- 提案: 行 1 つの不正でプロファイル全体を落とさない（その行を飛ばして数える）。再試行の間隔を失敗回数で延ばすのは D3 の（仮）の範囲

## R60. 写しの置き場と後始末が design D2 と違う（R43 が未解消）

- 処置: fixed D2 — 写しは `state_dir/browser-history/tmp/<uuid>` に drop guard、起動時に掃除、削除の失敗は種別と件数だけログ（0ed3835）。`history_copy_lives_under_tmp_and_is_removed_on_panic`

- 成果物: crates/collector-windows/src/history/read.rs:138-142
- 根拠: 写しを `std::env::temp_dir()` に作り、`remove_dir_all(...).ok()` で削除の失敗を握りつぶす。読み手が panic すると（`HistoryWorker::join` が捕まえて収集は続く）URL を含む写しが %TEMP% に残る
- kind: technical
- 提案: drop guard にし、置き場（`state_dir/browser-history/tmp`）の下に作る。起動時に残骸を掃除し、削除の失敗は種別だけログに出す

## R61. 10.1 / 10.2 は未検証なのに 10.4 は `[x]`。CI は今の head を一度も走らせていない

- 処置: rejected: push と CI はこの段の作業ではない —— `scripts/merge_gate.sh:7` が head を push し、**その head の** CI（`collector-windows-runtime` を含む）が緑になるまで待ち、赤なら draft に戻す。10.1 / 10.2 は `[ ]` のまま（ci の残り）

- 成果物: openspec/changes/st08-browser-history/tasks.md:189-190、evidence.jsonl:69、.github/workflows/ci.yml
- 根拠: evidence.jsonl:69 は 10.1 が FAIL（Linux で 0 本）。`origin/feat/st08-browser-history` は `6768425` のままで未 push が 33 commit。10.4 の検証「`collector-windows-runtime` job が緑」を今の head で見た者がいない。Chrome / Firefox の choco 導入と `-lt 10` の下限は `windows-latest` で未実行
- kind: technical
- 提案: push して job の結果で 10.1 / 10.2 / 10.4 を付け直す

## R62. 単体が今も `Scenario: ブラウザが動いている間も取得できる` の印を持つ（R54 が未解消）

- 処置: fixed 10.1 — read.rs の単体 2 本から印を外した（0ed3835）。印は `tests/runtime_windows.rs` の 3 本だけ。`python3 scripts/check_scenarios.py . st08-browser-history` rc=0

- 成果物: crates/collector-windows/src/history/read.rs:293、:375
- 根拠: Linux で写しが消えるのを見るだけの単体に印があり、`check_scenarios.py` は Windows の 3 本（10.1 / 10.2）が一度も通っていなくても担保ありに数える
- kind: technical
- 提案: 単体側の印を外す（担保は `tests/runtime_windows.rs` の 3 本）

## R63. Minor: README の `browser-profile` の例が表示名に見える・帳面の退避が無言・本番の `.expect`・smoke の手組み JSON

- 処置: fixed D4 — README（`profile` はディレクトリ名）・帳面の退避の件数ログ・`.expect` の除去・smoke の手組み JSON（0ed3835）

- 成果物: crates/collector-windows/README.md:70、history/ledger.rs（`*.broken.ledger`）、history/fetch.rs:128 / :206、tools/smoke.sh（ST08 の最初の節）
- 根拠: README の例 `"profile": "Work"` は表示名に見えるが照合先は `profile_dir`（`Default` / `Profile 1`）で、表示名の登録は黙って当たらない。
  帳面の退避がログ無しで上書き（R49 の残り）。`.expect` は `micros()` が作った値の再パース。smoke の手組み JSON は `"profile":"Default"` で D4 は `profile_dir`
- kind: technical
- 提案: README に「ディレクトリ名」と明記。退避を種別だけログに出す。`.expect` を消す。手組み JSON を D4 の形に揃える

## R64. Minor: D10 の Chromium `sqlite_sequence` による作り直し判定が未実装（最大番号の比較だけ）

- 処置: fixed D10 — `sqlite_sequence` を読み帳面の `visit_sequence` と比べる（0ed3835）。`history_vanished_marks_recreated_table_by_sequence`

- 成果物: crates/collector-windows/src/history/（`table_recreated` の算出）
- 根拠: Task 6 の F2 の後半。最大番号が下がらない作り直しは見逃す
- kind: technical

## R65. Minor: Firefox の帳面が、ini の絶対パスのプロファイルと `Profiles\` 直下の同名ディレクトリで衝突する

- 処置: followup ST08 — 直していない（Minor・失うもの無し。訪問は毎回読まれ同じ識別子で畳まれる）。帳面の鍵だけ分ければ `profile_dir` と D6 を変えずに止められる（re-review）。docs/handoff/ST08.md

- 成果物: crates/collector-windows/src/history/locate.rs（`scan_ini` の `directory` = `file_name`）
- 根拠: 2 つが同じ `(browser, directory)` の帳面を共有し、互いの訪問を毎回「消えた」と判定する
- kind: technical

## scoped re-review（73ab551..c964792）

席: re-review（SDD の `re-review-prompt.md`。package `.superpowers/sdd/tasks/review-73ab551..c964792.diff`）。
判定: R56〜R60・R62〜R64 は ADDRESSED、R65 は意図して未対処（Minor）。fix が持ち込んだ Critical / Important は無い。Minor 2 件を R66 / R67 に写す。2 回目の fix は出さない（SDD の Final Review）。

## R66. URL 付きで送った訪問が後から URL の行を失うと、URL 無しの識別子で 2 件目として送られる

- 処置: followup ST08 — Minor。失うものは無く（`slot` で「消えた」の誤判定は防いでいる）、取り込み口に同じ訪問の行が 2 つ残りうる。Q8 の答え（URL 無しで入れる）の文面どおり。docs/handoff/ST08.md

- 成果物: crates/collector-windows/src/history/fetch.rs（`still_present`）、crates/collector-windows/src/history/collect.rs（`select_new_or_changed`）、design.md D6
- 根拠: 古い識別子は `still_present` で「まだある」と扱われ、新しい URL 無しの識別子が新規として選ばれる。片付ける記録は来ない。D6 は「別の識別子になる」とだけ書く
- kind: technical
- 提案: D6 に「同じ `slot` がすでに送られていれば送らない」を足すか、件数を数える Story へ申し送る

## R67. Minor: `CLEANUP_FAILURES` がプロセス全体の static・`collect_rows` が `SqliteFailure` 以外をすべて行の不正として飛ばす

- 処置: followup ST08 — Minor。本番の動きに実害は無い（列の番号は固定）。テストの理論上の不安定だけ。docs/handoff/ST08.md

- 成果物: crates/collector-windows/src/history/read.rs（`CLEANUP_FAILURES`、`collect_rows`）
- 根拠: 並んで走るテストで `history_bad_row_does_not_fail_the_profile` の `take_log_counts()` に別のテストの後始末の失敗が混ざりうる。`InvalidColumnIndex` などのコードの誤りも黙って飛ぶ
- kind: technical

## code-verify（3 回目。HEAD `291b271`）

対象: worktree `/home/yosis/dev/ashiato2-st08`（`feat/st08-browser-history` / HEAD `291b271`）。作業ツリーは検証の前後とも clean（`git status --short` が空）。
DB: `docker compose up -d --wait db` ＋ `tools/db-roles.sh`（smoke の後始末で止まったので、移行の変異の前に起こし直した。終わりに `stop`）。
ガードの変異・一時テストは **HEAD を `git archive HEAD` で書き出した複製**（`/home/yosis/dev/.st08-verify3`）で走らせ、1 件ごとに原状へ戻した。複製は検証の後に消した。
Windows の実測は、この PC の Windows 側の cargo（msvc）で、作業ツリーを `C:\dev\ashiato2-rt-st08cv3` に同期して走らせた（検証の後に消した）。

### 申告と実測

申告: tasks.md は `- [x]` 36 件 / `- [ ]` 2 件（10.1・10.2）。`review_triage.py` は指摘 97 件すべてに処置あり。

| 走らせたもの | 実測 | 申告との一致 |
|---|---|---|
| `cargo test --workspace`（.env を読み込んで） | rc=0（collector lib 197 本 / server 409 本 + 7 本。`runtime_windows` は Linux では 0 本） | 一致 |
| `cargo clippy --workspace --all-targets -- -D warnings` / `cargo fmt --all --check`（11.6） | rc=0 / rc=0 | 一致 |
| `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings`（1.2） | rc=0 | 一致 |
| `openspec validate st08-browser-history --strict`（11.1） | rc=0 | 一致 |
| `python3 scripts/check_scenarios.py .` / `… . st08-browser-history`（11.2） | rc=0 / rc=0（642/642。delta は `#### Scenario:` 75 本） | 一致。中身は手 3 |
| `python3 scripts/check_chain.py .`（11.3） | rc=0 | 一致 |
| `python3 scripts/review_triage.py . st08-browser-history`（11.4） | rc=0（`指摘 97 件 / 仮決め 3 件`） | 一致 |
| `tools/smoke.sh`（5.3 / 5.5 / 6.3 / 9.1 / 10.3 / 11.5） | rc=0（ST08 の 8 節とも通る。うち 3 節は本物の `Runtime` と読み手で psql まで） | 一致（手組み JSON の時刻は R73） |
| `tools/check-immutable.sh` / `check-migrations.sh` / `check-licenses.sh`（11.5） | rc=0 / rc=0 / rc=0（Rust 315 件・Node 257 件。Android は「対象外」と明示） | 一致 |
| tasks に名前のある `cargo test <名前> -- --list` 36 種（collector 34・server 2） | すべて 1 本以上。下限のあるもの: `history_heartbeat` 10（≥5）/ `history_vanished` 22（≥7）/ `history_exclusion` 14（≥7）/ `history_locate` 6（≥3）/ `history_schedule` 10（≥3）/ `browser_history_update` 5。`window_request_body_is_unchanged` は 1 本（前回 R4 の 0 本は解消） | 一致 |
| 3.3 の grep（`c02-browser-history` / `v1:` / `source_updated_at`） | 1 / 1 / 3 | 一致 |
| 7.1 の grep（README の `url-contains` / `browser-profile`） | 2 / 2 | 一致 |
| 10.4 `git diff main...HEAD -- .github/workflows/ci.yml` | 下限 `-lt 7` → `-lt 10`、Chrome / Firefox の choco 導入 | 前半は一致。**「job が緑」は未確認**: `origin/feat/st08-browser-history` は `6768425` のままで 37 commit が未 push（R61 で rejected 済み。状態は同じなので新しい R にしない） |
| 10.1 / 10.2（`[ ]`）を Windows 実機で: `cargo test -p ashiato-collector-windows --test runtime_windows browser_history -- --test-threads=1` | `browser_history_while_running` ok / `browser_history_chrome` ok / `browser_history_firefox` FAILED（この PC に Firefox が無い。`runtime_windows.rs:1067` の `expect`） | 申告なし（`[ ]`）。Edge・Chrome は**動いているブラウザの履歴を本物の読み手で取れる**ことを確かめた。Firefox は手元では確かめられない |
| 固定値の独立再計算（python `hashlib` / `datetime`） | 識別子 5 つ・時刻 2 つが一致 / smoke の手組み JSON の時刻 2 つが不一致 | 手 1・R73 |
| ガードの変異 23 件（collector 22 ＋ 移行 1） | 19 件はどれかのテストが落ちる / **4 件は 197 本全部緑** | 手 2・R72 |
| 初回の大きな取り込み（Windows 実機・実時計の `Runtime::tick`） | 10 万件で見回りが 107 秒止まる / **15 万件で 148 秒止まり、`c02-window` に `suspended` が 2 件入る** | 新規（R68） |

### 手 1: 固定値を独立に再計算する

python の `hashlib.sha256` で `\x1f` 区切りの組から計算し直した。**5 つとも一致**:

| テスト | 組 | python | テストの期待値 |
|---|---|---|---|
| `visit_external_id_is_pinned` | `chromium·chrome·Default·7·13402627200000001·https://example.test/a?q=x` | `981e93fe…79ecefa1` | 一致 |
| 同上（vanished / excluded） | `chrome·Default·v1:visit:a·v1:visit:b` | `1803175b…7fd469d8` | 一致 |
| 同上（profiles） | `chrome·Default\x1e個人·Profile 1` | `5c4f4852…74b8e8db` | 一致 |
| `visit_without_url_row_omits_url_and_title` | `chromium·chrome·Default·7·13402627200000001`（URL を省いた 5 つ） | `c8aa63ce…56acd5d7a` | 一致 |
| 同上（空の URL の 6 つの組） | 末尾が空文字 | `04e70926…30c3d370` | `assert_ne!` のとおり別の値 |

時刻: `13402627200000001`（1601 年起点 µs）→ `2025-09-18 00:00:00.000001+00`、`1758153600000000`（1970 年起点）→ `2025-09-18 00:00:00+00`、
`805306368 & 0xff = 0`（`link`）。`visit_time_keeps_micros` / `visit_payload_shape_is_pinned` の期待値と一致する。
smoke の本物の取得の節（`browser_history_smoke make` の `13402627200000000`）の期待 `2025-09-18 00:00:00+00` も一致。
**手組み JSON の 2 節だけが食い違う** → R73。

### 手 2: ガードをわざと壊す

複製で 1 件ずつ書き換え、`cargo test -p ashiato-collector-windows --lib` を走らせた（原状は 197 passed）。

| # | 壊したもの | 結果 |
|---|---|---|
| M01 | Q7: `due` の `last > now \|\|` を外す | 2 本 FAILED（`history_schedule_fetches_at_once_when_last_success_is_ahead`・Runtime の同名） |
| M02 | Q7: 試し直しの時刻が先にあるときの `\|\| retry - now > …` を外す | 1 本 FAILED |
| M03 / M04 | Q8: Chromium / Firefox を `LEFT JOIN` → `JOIN` | それぞれ 1 本 FAILED（`history_read_keeps_visits_whose_url_row_is_missing`） |
| M05 | Q8: `still_present` の `slot` の照合を無効にする | 2 本 FAILED |
| M06 | D11: URL を失った訪問の除外の引き継ぎを無効にする | 1 本 FAILED（`history_exclusion_survives_lost_url_row`） |
| M07 | D11: `exe-path` をファイル名でなく全体で比べる | 1 本 FAILED（`history_exclusion_exe_path_covers_all_profiles`。前回 R58 は解消） |
| M08 | Q4: 識別子を発生元の番号で作る | 1 本 FAILED |
| M09〜M12 | 24h→23h / 試し直し 1 分→2 分 / 上限 1h→2h / `chunks(1000)`→`999` | 3 / 2 / 1 / 1 本 FAILED |
| **M13** | `apply` の `ensure!(unreadable == 0, …)` を外す（読めないプロファイルがあっても成功にする） | **197 本全部 ok**（R72） |
| M14 | 生存信号 86400→21600 | 2 本 FAILED |
| **M15** | 除外の登録が読めないとき `unwrap_or_default()`（空の登録で送る） | **197 本全部 ok**（R72） |
| **M16** | `profile_dir_absent` の `try_exists().unwrap_or(true)` → `unwrap_or(false)` | **197 本全部 ok**（R72） |
| **M17** | `with_history` の起動時の写しの掃除（`sweep_copies`）を呼ばない | **197 本全部 ok**（R72） |
| M18 | 区間に読みが無いときの確かめ（`start_probe`）を外す | 3 本 FAILED |
| M19 | 除外を外した訪問を送り直す条件を外す | 1 本 FAILED（`history_exclusion_removed_later`） |
| M20 | `source_updated_at` を載せない | 1 本 FAILED |
| M22 | Q1: Vivaldi の置き場を `VivaldiX/User Data` に | 1 本 FAILED（`browser_bases_match_the_documented_locations`） |
| M23 | Q2: `vanished` を積まない | 8 本 FAILED |
| 移行 | `…_browser_history_record_id.sql` の `NOT EXISTS` を外す（server） | `browser_history_record_id_is_kept_once_records_exist` FAILED（原状は 2 本 ok。DB を起こした状態で対照を取った） |

### 手 3: Scenario と test

`check_scenarios.py` は rc=0（642/642）。印の先を見て、主張の階層とテストの階層が違うもの:

- `履歴の取得でウィンドウのソースの記録は増えない`（spec 81〜85 行。AND「取得の間の時間が、ウィンドウのソースに PC が眠っていた時間として記録されていない」）
  の印 `history_slow_read_does_not_disturb_window` は**読み**（別スレッド）を 3 分止めるだけで、訪問は 1 件。**積み込み**（見回りのスレッドで 1 件ずつ `sync_data`）は測っていない。
  Windows 実機で 15 万件を積むと AND が破れる → R68
- `ブラウザが動いている間も取得できる` は `tests/runtime_windows.rs` の 3 本だけが印を持つ（前回 R62 は解消）。Windows 実機で Edge・Chrome の 2 本は rc=0。Firefox はこの PC に無く未確認
- `check_scenarios.py` の `[warn] spec に無い Scenario を指す印` が ST08 のコードに 7 個ある（`locate.rs` の `6つのブラウザの既知の置き場が設計表どおりである` など）。
  数えに入らない名前なので害は無いが、印の形をしていて Scenario でない。指摘にはしない

### 手 4: 本人の決定が test で固定されているか

`deep.md` の答えを 1 件ずつ壊した（手 2 の M 番号）:
Q1（6 つの置き場）M22 / Q2（消えた記録）M23 / Q4（PC 側の番号）M08 / Q5（組全体のハッシュ）は手 1 の独立計算と一致する固定値 / Q7 M01・M02 / Q8 M03〜M05・M06 /
24 時間 M09・試し直し 1 分 M10・上限 1 時間 M11・1,000 件 M12・生存信号 86,400 秒 M14 —— **どれも書き換えると 1 本以上落ちる**。
前回の手 4 の「固定されているのは使われない定数」は解消（`HistorySchedule` / `HistoryBeat` が `Runtime` から使われ、Runtime の層のテストも落ちる）。
Q3（版を残す）はサーバの `browser_history_update` 5 本が持つ。今回サーバのコードは壊していない。

### 手 5: tasks の `[x]` と実体

`[x]` 36 件の検証コマンドは、10.4 の「job が緑」を除いてすべて実在し rc=0（上の表）。10.4 の後半は R61（rejected）のまま —— 新しい R にしない。

### 手 6: 隙間

R68（大きな積み込みで見回りが止まり、偽の `suspended` が入る）・R69（積み残しの後ろでウィンドウの記録が待つ）・R70（積めない間も生存信号が「取得できる」）・R71（在るプロファイルを「無くなった」とする）。
時計が戻る（Q7）は M01 / M02 で固定済み。読めないプロファイル・URL の行の欠落は手 2 のとおり固定済み。

---

## R68. 初回の大きな取り込みは見回りのスレッドで 1 件ずつ同期して積むので、Windows で 15 万件なら 148 秒止まり、`c02-window` に偽の「眠っていた」が入る

- 成果物: crates/collector-windows/src/history/collect.rs（`HistoryCollector::tick` → `apply` → `queue_then_save`）/ crates/collector-windows/src/runtime.rs:525-536（`maybe_history` の `queue` が `events.add`）/ crates/collector-windows/src/outbox.rs:81-94（`add` が 1 件ごとに open・追記・`sync_data`）/ runtime.rs:46・293（`SUSPEND_GAP_SEC = 120`）
- 根拠:
  - Windows 実機（この PC。Windows 側の cargo）で、HEAD の複製の `runtime::tests` に一時テストを足し、偽の読み手が N 件を返す `Runtime` を**実時計の `tick()`**で 1 秒ごとに回した:
    N=1,000 → 最長の 1 回 0 秒・`suspended` 0 件 / N=100,000 → **最長 107 秒** / N=150,000 → **最長 148 秒、`outbox.jsonl` の `c02-window` に
    `{"kind":"idle","reason":"suspended","transition":"enter","mono_gap_ms":149624}` と `leave`（`range_end` 08:20:05）の 2 件**
  - 同じテストを Linux で: N=20,000 → 3.4 秒 / 100,000 → 18.2 秒 / 400,000 → 51 秒（件数に比例）。Windows の NTFS で「追記して `Flush(true)`」を 2,000 回 → 1 回 1.79〜1.92 ms（PowerShell で 2 回測った）
  - 読み（別スレッド）は待たないが、`tick` が読みの終わりを見つけた回に `apply` が**見回りのスレッドで**全件を `queue`（= `Outbox::add`、1 件 1 回の `sync_data`）する。
    止まっている間は前景を 1 度も観測せず（その間の前景の変化は取れない）、次の見回りで `wall_gap >= 120` が「眠っていた」を積む。収集した記録は書き換えられない
  - design D3 は「Firefox が何年分も持っていると最初の読みは数十秒かかりうるので、見回りを止めると眠りの判定に化ける」として読みを別スレッドにしたが、積み込みが同じ型で残っている。
    帳面が壊れて退避した後（D10「空から始める」）も全件を積み直すので、初回だけの話ではない
- kind: technical
- loss: uncaptured （止まっている間の前景の変化。あわせて事実でない「眠っていた」区間が書き換えられない記録として残る）
- 提案: 1 回の取得の分をまとめて追記し、同期は 1 回にする（`Outbox::add_many`）か、積み込みを数千件ずつ見回りに分ける。
  「15 万件を積む間も `c02-window` に `suspended` が入らない」を Runtime の層（実時計か、積み込みの所要時間を見回りの時計に反映する形）で固定する。
- 処置: escalated — deep.md 第 5 回 Q9（loss: uncaptured。大きな取り込みの間、見回りを止めてよいか）。`docs/briefs/ST08-deep-r5.html`

## R69. 初回の取り込みの後ろで、ウィンドウの記録の送信が件数 ÷ 200 × 5 分待つ（2,000 件で 50 分、10 万件で約 42 時間）

- 成果物: crates/collector-windows/src/sender.rs:84-98（`snapshot` の先頭から `MAX_BATCH = 200`）/ crates/collector-windows/src/runtime.rs:43（`SEND_INTERVAL_SEC = 300`）/ history/collect.rs（履歴を同じ `events` の outbox に積む）
- 根拠: 複製の一時テストで、2,000 件の履歴を積んだ直後に前景のアプリを変えて 1 件のウィンドウの記録を作り、`rt.send()` を繰り返した → **10 回目の送信で初めてその記録が出る**
  （`PROBE sends_until_window=10 interval_sec=300`）。送信は 5 分に 1 回・200 件なので 50 分。同じ形で 10 万件なら 500 回 ≒ 41.7 時間。
  未送信の置き場は 1 つで、送る順は積んだ順。この間、`remove` のたびに置き場の全行を書き直す（`outbox.rs:111-131`）
- kind: daily （失うものは無い。初回と帳面の退避の後に、PC の前景の記録が画面に出るまで最大で日単位で遅れる）
- 提案: 履歴の取り込みの間も、ウィンドウの記録を先に送る（ソースごとに 1 回の送信の枠を分ける・履歴を別の置き場にする）か、
  初回だけ送信の間隔を詰める。どちらにするかは design D3 の（仮）に足して反転条件を書く。
- 処置: fixed D3 仮 — 1 回の送信でウィンドウの記録を履歴より先に載せる（`Outboxable::sends_late`。どちらの中でも積んだ順）。反転条件は D3。`history_backlog_does_not_hold_back_window_records`（並べ替えを外すと落ちることを確かめた）

## R70. 積み込みが失敗し続けている間も、履歴の生存信号は「取得できる」を報告する（除外の登録が壊れた 3 日間、送った訪問 0 件・信号 4 回とも `capturable=true`）

- 成果物: crates/collector-windows/src/history/collect.rs（`tick` が `apply` の前に `finished_reads` を「読めた」で作る / `apply` の `Exclusions::load(...)?`）/ crates/collector-windows/src/runtime.rs:127-160（`HistoryBeat::record` は読めたかだけを数える）
- 根拠: 複製の一時テストで、起動の後に `exclusions.json` を壊れた形（`[{"kind":"no-such-kind","value":"x"}]`）にして 3 日回した:
  `PROBE visits_sent=0 reads=66`、生存信号は 4 件とも `capturable=true blockers=[] attempts=successes`（2 / 24 / 20 / 21）。
  `apply` は毎回 `除外の登録を読めないので履歴を送らない` で失敗し、ログは `history_fetch_failed`（種別 `failed`）だけ。
  同じことは `apply` の中のどの失敗（帳面の保存・`last_success.json` の書き込み・未送信の置き場への追記）でも起きる —— 生存信号は「写しが読めたか」しか見ない。
  ウィンドウの側は起動時に読んだ登録で動き続けるので、壊れたことに気づく経路が無い（再起動すれば起動が止まる）。
  Chromium 系は 90 日を過ぎた訪問を手元から消すので、この状態が続くとその間の訪問は後から取れない。spec の生存信号の Requirement（440〜451 行）は「読めなかった」しか書いておらず、
  「読めたが積めない」状態を想定していない
- kind: technical
- loss: uncaptured
- 提案: 積み込みの失敗も「取得できない」に数え、`blockers` に種別だけを載せる（例 `history-not-queued`。URL・パスは載せない）。
  既定は厳しい側（Q1 の R8「1 つでも読めなければ取得できない」と同じ向き）なので問いにはしない。spec の生存信号の Requirement に 1 行と、Runtime の層のテストを足す。
- 処置: escalated — deep.md 第 5 回 Q10（loss: uncaptured。積めない間の生存信号を「取得できる」にしてよいか）。`docs/briefs/ST08-deep-r5.html`

## R71. Opera の主プロファイルと、Firefox の既定の外に置いたプロファイルは、ディレクトリが在っても「無くなった」と判定され、`profile_gone: true` の「消えた」記録が送られる

- 成果物: crates/collector-windows/src/history/collect.rs（`FsHistoryReader::profile_dir_absent` / `queue_gone_profiles`）/ history/locate.rs:42-51（`Browser::base`）
- 根拠:
  - `profile_dir_absent` は `base/<directory>` と `base/_side_profiles/<directory>` しか見ない。Opera の主プロファイルは `Opera Stable` の直下（directory = `Default`）、
    Firefox は `base` が `Mozilla/Firefox/Profiles` で、`profiles.ini` の絶対パスのプロファイルはその外にある。
    複製の一時テスト: `Opera Stable` が在る状態で `profile_dir_absent(Opera, "Default")` → `true`、
    在る `D/ffprof.work`（ini の絶対パス相当）で `profile_dir_absent(Firefox, "ffprof.work")` → `true`
  - 本物の読み手（`FsHistoryReader`）と `HistoryCollector` で端から端まで: Opera の `History` に 1 訪問 → 1 回目は `visit` と `profiles` を積む →
    `History` を一時的に別名にして（`Opera Stable` は在る）24 時間後に取得 → `{"kind":"vanished",…,"browser":"opera","profile_dir":"Default",…,"profile_gone":true}` が積まれ、取得は成功扱い（試し直さない）
  - 同じ状態の Chrome（`User Data\Default` は在り `History` だけ無い）は「無くなった」にならず、帳面も残る。ブラウザによって同じ状態の扱いが違う。
    逆に Chrome の `User Data` ごと消えた（アンインストールでデータも消した）ときは、`read_dir(base)` が失敗して「無いとは言えない」となり、`profile_gone` は一度も出ない（`chrome_userdata_exists=false absent=false`）
  - 「消えた」は収集した記録として残り、書き換えられない。帳面から外れた訪問は、`History` が戻ると同じ識別子で送り直され取り込み口で畳まれる（行は失われない）が、事実でない「無くなった」は残る
- kind: technical
- 提案: 「無くなった」の確かめを、見つけたときの実際のパス（`Profile.path` の親）で行い、帳面にそのパスの形（ディレクトリ名でなく、置き場からの相対か ini の絶対かの印）を持つ。
  Opera の主プロファイル・Firefox の ini の絶対パス・`User Data` ごと消えた、の 3 つを `history_vanished` に足す。
- 処置: fixed D10 — `profile_dir_absent` を見つけるときと同じ場所（Opera の主プロファイルは置き場そのもの・Firefox は `profiles.ini` のパス）で確かめ、置き場ごと消えたら無くなった、探す先の根が無い / ini が読めないなら「在る」（`locate::ini_profile_dirs` を共有）。`history_vanished_opera_main_profile_is_its_place` / `history_vanished_firefox_absolute_profile_is_checked_at_its_path` / `history_vanished_whole_place_removed_is_gone`（それぞれの分岐を外すと落ちる）

## R72. 4 つのガードが、壊しても collector の 197 本が全部緑のまま（読めないプロファイルでも成功にする / 除外の登録が読めないとき空で送る / 在るか不明を無いと読む / 起動時に写しを掃除しない）

- 成果物: crates/collector-windows/src/history/collect.rs（`apply` の `ensure!(unreadable == 0)`・`Exclusions::load(...).context(...)?`・`profile_dir_absent` の `unwrap_or(true)`）/ crates/collector-windows/src/runtime.rs:233-244（`with_history` の `sweep_copies`）
- 根拠: 手 2 の M13 / M15 / M16 / M17。複製で 1 件ずつ書き換えて `cargo test -p ashiato-collector-windows --lib` → 4 件とも `197 passed; 0 failed`。
  - M15 は**除外した URL と題名が送られる**向きの変更（`exported`）で、spec の除外の Requirement と D11「既定は厳しい側」の担保が無い。ウィンドウの側の同じ規則は `broken_registration_is_an_error_not_empty` が持つが、履歴の経路には無い
  - M13 は design D3「失敗した取得は成功を進めない」の、読めないプロファイルの場合の担保が無い（`history_vanished_skips_unreadable_profile` は「消えた」を出さないことだけを見る）
  - M16 は R71 と同じ関数の安全側の既定（確かめられないなら「在る」）が固定されていない
  - M17 は design D2 / R60 の「起動時に残骸を消す」。`history_copy_leftovers_are_swept` は `read::sweep_copies` を直接呼ぶだけで、起動の経路から呼ばれていることを見ない
- kind: technical
- 提案: 4 本を Runtime か `HistoryCollector` の層に足す（壊れた `exclusions.json` で訪問が 1 件も送られない / 読めないプロファイルがあると 1 分後に試し直す /
  `try_exists` が失敗する置き場で「消えた」が出ない / 起動時に `browser-history/tmp` の残骸が消える）。
- 処置: fixed D11 — 4 本を足した: `history_exclusion_broken_registration_sends_nothing`（M15）/ `history_unreadable_profile_does_not_advance_success`（M13）/ `history_vanished_unknown_existence_is_present`（M16。unix のみ —— ENOTDIR で問い合わせを失敗させる）/ `history_copy_leftovers_are_swept_at_startup`（M17。Runtime の起動経路）。4 件とも変異を当てて 1 本落ちることを確かめた

## R73. Minor: smoke の手組み JSON の `visit_time_raw` と `at` が食い違う（`13402627200000000` は 2025-09-18、`at` は 2026-09-08）

- 成果物: tools/smoke.sh:891（`history_raw`）/ :932（`orphan_raw`）
- 根拠: python の `datetime(1601,1,1) + timedelta(µs)` で `13402627200000000` → `2025-09-18 00:00:00+00`、`13402630800000000` → `2025-09-18 01:00:00+00`。
  本文の `at` はそれぞれ `2026-09-08T02:00:00`・`03:00:00`（その時刻の 1601 年起点 µs は `13433306400000000`）。収集側の `Visit::from_read` は `at` を `visit_time_raw` から作るので、この組は実装が作らない形。
  smoke は取り込み口の側（行が増えない・原文が変わらない・感度・URL の無い訪問の格納）しか見ないので rc=0 になる（前回 R46 / R63 の残り。本物の取得の節は一致している）
- kind: technical
- 提案: 手組みの 2 本を `browser_history_smoke make` の値（`13402627200000000` ⇔ `2025-09-18T00:00:00.000000Z`）に揃えるか、手組みをやめて本物の取得の節へ寄せる。
- 処置: fixed 11.5 — 手組みの `visit_time_raw` を `at` に揃えた（`13433306400000000` ⇔ 2026-09-08T02:00、`13433310000000000` ⇔ 03:00。python で再計算）。`tools/smoke.sh` rc=0

### R68〜R73 の処置の scoped re-review（`/story ST08 finish`。2026-10-05）

独立の reviewer（`pr-review-toolkit:code-reviewer`）が、R69 / R71 / R72 / R73 の修正の差分だけを見た。**指摘なし**（Critical / Important 0 件）。
参考の Minor 1 件: `history_vanished_unknown_existence_is_present` は `#[cfg(unix)]` なので、Windows では M16 が固定されていない（処置の記述どおり。指摘にしない）。
R68 / R70 は直さず deep.md 第 5 回 Q9 / Q10 へ返した。
