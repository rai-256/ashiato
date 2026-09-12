# ST07 実装の独立レビュー（code-verify）

**やり方**: 申告された検証コマンドを全部走らせ直し、そのうえで
(1) 直書きの期待値を別言語で再計算し、(2) 守るものを潰した複製で検査が赤くなるかを見て、
(3) spec の 30 Scenario を印の先のテストまで 1 件ずつ追い、(4) `deep.md` の本人の決定 9 件の値を
書き換えて落ちるテストがあるかを確かめ、(5) `tasks.md` の `[x]` の検証方法を実在確認し、
(6) プロセスの立て直し・権限・応答が読めない経路を突いた。

**壊した複製はすべて `git checkout --` で戻し、検証用に足した一時テストは削除した**
（最終の `git status --short` は空。`git log -1` = `57e7071`）。
**この change のコードも成果物も 1 行も編集していない。**

## 申告 32/33 と、走らせ直した結果

| 申告 | 実測 | 一致 |
|---|---|---|
| tasks 33 件のうち 32 件 `[x]`・残 1 件は 9.3 | `[x]=32 / [ ]=1`（9.3） | 一致 |
| `cargo test --workspace` rc=0（collector 51 / server 144） | rc=0 / 51 本 / 144 本 | 一致 |
| `cargo clippy --workspace --all-targets -- -D warnings` rc=0 | rc=0 | 一致 |
| `cargo fmt --all --check` rc=0 | rc=0 | 一致 |
| `cargo check -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu` rc=0 | rc=0 | 一致 |
| `tools/smoke.sh` rc=0 | rc=0（== 39 まで通過） | 一致 |
| `tools/check-immutable.sh` / `check-migrations.sh` / `check-boundaries.sh` / `check-openapi.sh` rc=0 | 4 本とも rc=0 | 一致 |
| `python3 scripts/check_scenarios.py . st07-active-window` rc=0 | rc=0（Scenario 60 / 担保あり 60 / 人間の確認待ち 0） | 一致 |
| `python3 scripts/check_chain.py .` rc=0 | rc=0 | 一致 |
| `openspec validate st07-active-window --strict` rc=0 | rc=0 | 一致 |
| `python3 scripts/review_triage.py . st07-active-window` rc=0 | rc=0（指摘 22 / 仮決め 4 / 要件へ戻すもの 6） | 一致 |
| 移行は 1 本も足していない | `git show --stat` に `migrations/` 無し。DB 実測で `c02-window \| none \| 21600` | 一致 |
| **「spec の 30 Scenario すべてに印か『人間の確認待ち』」** | 印は 30/30 あるが、**`tasks.md` は「23 本」と書いている**（R12） | **不一致** |
| （申告に無い）`scripts/merge_gate.sh` の tasks 検査 | **9.3 で FAIL する**（R10） | — |

**捏造も空テストも無かった。** 41 本ある `#[test]` はすべて実際に assert しており、
印の付いた Scenario に対応するテストは全部存在して緑になる。
ずれは**「主張の階層」と「テストの階層」**、および**プロセスが落ちたとき**に出た。


## 再検証（2026-09-13）

HEAD `57e7071`・作業ツリーの差分は review/code.md（未追跡）だけの状態で、上の表と R1〜R16 を走らせ直した。

- 表の検証コマンド: `cargo test --workspace` rc=0（collector 51 / server 144）/ clippy・fmt・windows-gnu の `cargo check` rc=0 / `check-immutable`・`check-migrations`・`check-boundaries`・`check-openapi` rc=0 / `check_scenarios.py` rc=0（60 / 60 / 待ち 0）/ `check_chain.py`・`openspec validate --strict` rc=0 / `tools/smoke.sh` rc=0（== 39 まで）。**すべて一致**
- `review_triage.py . st07-active-window` は**いまは rc=1**（`処置が無い` 16 件）—— この未追跡の code.md に `処置:` がまだ無いため。code.md を一時的に外すと rc=0（指摘 22）で、表の行とは矛盾しない
- R1〜R16: 一時テスト（`runtime.rs` / `sender.rs` の末尾に足した `zz_reverify*`）・定数を振った複製・本物のサーバ（127.0.0.1:18999）・バイナリの素の起動で 1 件ずつ当て直した。**16 件すべて再現した**（R14 は判定の層まで。実機の UI Automation の挙動は確かめていない）
- 前回の記述との食い違い（本文は触っていない）: 冒頭の「41 本ある `#[test]`」は HEAD では 51 本 / 冒頭の「最終の `git status --short` は空」は、実際には review/code.md が未追跡で残る / R9 の表の「R16 のとおり」は R14 / R16 の `runtime.rs:186-208` は 172-194
- やり残しを埋めて足した指摘: R17（時計の巻き戻り）/ R18（状態ファイルの書きかけ）/ R19（契機の配線に担保が無い）
- 後始末: 壊した複製はすべて `git checkout --` で戻し、一時テストは消した。smoke の後に `docker compose up -d --wait db` で DB を起こし直した。最後の `git status --short` は `?? openspec/changes/st07-active-window/review/code.md` だけ

---

## R1. 除外の件数が、プロセスが落ちると最大 5 分ぶん無言で消える

- 成果物: crates/collector-windows/src/engine.rs:330-357 / crates/collector-windows/src/runtime.rs:153-160
- 根拠: 一時テストで `Runtime` を 2 回立てて実測（除外対象を前景に置き、題名を 10 回変えてから
  プロセスを落とし、立て直す）。
  ```
  落ちる前の未送信 (0, 0)
  再起動後に送られた種類: ["clock-skew","excluded","powered-off","clock-skew","excluded"]
  除外の記録 2 件   count=1 / count=1
  ```
  **10 回の変化のうち記録に残ったのは 2 件（各 count=1）で、8 回は消えた。**
  `Engine.excluded`（`ExcludedSpan { since, count, key }`）はメモリだけにあり、
  `Outbox` と違って `state_dir` に落ちない。`engine.rs:328` の doc は
  「**除外の数えを抱えたまま落ちると、その件数は後から作れない**」と書いており、
  その受け皿として `Runtime::stop`（runtime.rs:153）がある。
  しかし `grep -rn '\.stop(' crates/collector-windows/src/` → **0 件**。
  `main.rs:59-62` は `loop { rt.tick(...)?; wait_for_change(...) }` で停止契機を持たず、
  ログオフ・シャットダウン・`tick` の Err のどれでも `stop` は呼ばれない。
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 irreversible（loss: uncaptured）。欠陥の指摘で、選び方の対立ではない —— **直す側に失われるものが無く、直さない側だけが失う**（どちらの読みでも何かが失われる A の型ではない）。merge 前で実データは 1 件も無いので、直せば何も失われていない。人間に問う選択肢が立たないので technical として直した。**PR 本文の冒頭に、この訂正を一覧で出す**
- 当初の loss: uncaptured —— 除外された変化の回数は後から作れない（deep Q5 / FR-83）
- 提案: `ExcludedSpan` を `state_dir` に落とす（`counters.json` と同じ形で十分）か、
  除外の変化を数えた時点で本文なしの 1 件を `Outbox` へ積む。`stop` を呼ぶ経路
  （`SetConsoleCtrlHandler` 相当が無ければ、せめて `tick` の Err 時に `stop` を試す）を足す。
- 再検証（2026-09-13）: 再現した —— 一時テスト（`Runtime` を 2 回立てる。除外対象の題名を 10 回変えてから `stop` を呼ばずに落とし、立て直す）で `送られた種類 ["clock-skew","excluded","powered-off","clock-skew","excluded"]` / `excluded_count [1, 1]`。HEAD の `crates/collector-windows/src/` に `.stop(` は 0 件
- 処置: fixed 11.1

## R2. 前景の変化が 0 回でも、除外の記録が 5 分ごとに 1 件（count=1）増える

- 成果物: crates/collector-windows/src/engine.rs:330-357（`flush` / `close_excluded`）
- 根拠: 除外対象を 30 分間ずっと前景に置き、**題名も URL も 1 度も変えず**に
  1 秒刻みで 1801 回 tick した実測（一時テスト）。
  ```
  excluded 記録 7 件（前景の変化は 0 回。30 分）
     at=00:00:00.000Z range_end=00:00:00.000Z count=1   ← 長さ 0 の区間
     at=00:00:01.000Z range_end=00:05:00.000Z count=1
     at=00:05:01.000Z range_end=00:10:00.000Z count=1
     …（以下 5 分ごとに 1 件、いずれも count=1）
  ```
  原因: `close_excluded` が `self.excluded.take()` を**無条件に**呼ぶ（engine.rs:348）。
  そのため `flush` の直後は必ず `self.excluded == None` になり、
  続く `if let Some(span) = &mut self.excluded { span.since = now; span.count = 0; }`
  （engine.rs:333-336。コメントは「除外の対象を見続けている間も数えは続く（次の区間として
  数え直す）」）は**到達不能な死んだ枝**。次の観測で `excluded = None` → 新しい区間が
  `count = 1` で作られる。
  spec の Scenario「除外した件数が残る」（**3 回起きる → 3 回を読み取れる**）と design D18
  （「その対象から出たとき 1 件書く」「送信の契機ごとにも吐き出す」）が前提にしている
  「件数＝変化の回数」が、`Engine` 単体（`excluded_count_is_kept`）では成り立つが
  **`Runtime` を通すと成り立たない**。担保しているテストは `flush` を 1 回しか呼ばず
  （`excluded_count_is_flushed_while_still_foreground`）、その 1 回の後に
  「観測を続ける」ことをしていないので、この経路を観測していない。
- kind: technical
- 提案: `close_excluded` を「`take()` せず、出力した後に `since`/`count` だけ戻す」形にし、
  `Runtime` 経由で「変化 0 回の 30 分 → 除外の記録の合計件数が変化の回数と一致する」テストを 1 本置く。
  併せて長さ 0 の区間（`at == range_end`）を作らない。
- 再検証（2026-09-13）: 再現した —— 同じ条件（変化 0 回・1 秒刻み 1801 tick）で excluded 7 件、全件 `count=1`、先頭は `at == range_end`（00:00:00.000Z）。
  同根: pr-review の「`Engine::flush` の除外リセットが到達不能」
- 処置: fixed 11.2

## R3. 自動起動の `.cmd` が接続先・合言葉・利用者を持たないので、次のログオンで必ず落ちる

- 成果物: crates/collector-windows/src/autostart.rs:23-31 / crates/collector-windows/src/main.rs:21 /
  crates/collector-windows/README.md（§自動起動）
- 根拠: `startup_script` が書く中身は
  ```
  @echo off
  rem ashiato C-02（ST07 / design D7）。消せば自動起動は止まる。
  set ASHIATO_STATE_DIR=<dir>
  start "" /min "<exe>"
  ```
  —— `ASHIATO_STATE_DIR` だけ。`Config::from_env`（config.rs:52-66）は
  `ASHIATO_BASE_URL` / `ASHIATO_API_TOKEN` / `ASHIATO_USER_ID` / `ASHIATO_DEVICE_ID` の
  **1 つでも欠ければ Err** で、`main.rs:21` がそれを最初に呼ぶので即終了する。
  README の「動かす」は `$env:…`（セッション限りのスコープ）で 4 つを設定する手順を示し、
  §自動起動にはそれを永続化する指示が 1 行も無い。`start "" /min` なので窓は最小化で開き、
  即座に閉じる → **利用者からは何も起きなかったように見える。**
  唯一のテスト `startup_script_has_exe_and_state_dir`（autostart.rs:57-66）は
  exe と `ASHIATO_STATE_DIR` と `/min` があることだけを見ており、**起動できるかを見ていない。**
  D7 / NFR-12 がこれを入れた理由は「起動を忘れた期間の記録は後から作れない」。
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 irreversible（loss: uncaptured）。欠陥の指摘で、選び方の対立ではない —— **直す側に失われるものが無く、直さない側だけが失う**（どちらの読みでも何かが失われる A の型ではない）。merge 前で実データは 1 件も無いので、直せば何も失われていない。人間に問う選択肢が立たないので technical として直した。**PR 本文の冒頭に、この訂正を一覧で出す**
- 当初の loss: uncaptured —— 起動しなかった期間の前景・題名・URL は後から作れない
- 提案: `--install-autostart` のとき 4 つの環境変数も `.cmd` に書く（合言葉が平文で
  スタートアップに載るのが嫌なら `state_dir` の設定ファイルから読む形に変える）か、
  `setx` で利用者スコープに永続化する。README に永続化の手順を書く。
  「生成した `.cmd` を素の環境で実行して `Config::from_env` が通る」検査を 1 本置く。
- 再検証（2026-09-13）: 再現した —— `env -i ASHIATO_STATE_DIR=/tmp/zz-st07 ./target/debug/ashiato-collector-windows` → rc=1 `Error: ASHIATO_USER_ID が無い`（`--install-autostart` を付けても同じ。`Config::from_env` が main.rs:21 で `run` より先に走る）。README に環境変数を永続化する記述（`setx` など）は 0 件
- 処置: fixed 11.3

## R4. 離席中に落ちると、その離席は「入った」だけが残り、永久に閉じない

- 成果物: crates/collector-windows/src/engine.rs:64-76（`Engine.away`）/ runtime.rs:69-105（`Runtime::new`）
- 根拠: 一時テストで、離席（閾値 300 秒超）に入った後にプロセスを落とし、立て直してから
  入力を戻して 400 回 tick した実測。
  ```
  idle の記録 7 件
     …
     transition="enter" at="2026-09-13T00:05:00.000Z" range_end=null   ← 対応する leave が無い
  last-seen.txt = 2026-09-13T00:19:20.000Z
     powered-off at="00:11:40.000Z" range_end="00:13:20.000Z"
  ```
  `enter` 側は `Outbox`（ディスク）に積まれて送られるが、`Engine.away` はメモリだけなので
  立て直した `Engine` は `away: None` から始まり、入力が戻っても `leave` を作らない。
  spec の Scenario「離席の始まりと終わりが残る」（**入った時刻と出た時刻がどちらも記録に残る**）は
  `idle_transitions`（engine.rs）が 1 プロセス内でだけ担保している。
  `powered-off` が空白を埋めるが、`idle` の区間の終わりはそこから引けない
  （`powered-off` の始まりは印の更新時刻で、離席の終わりではない）。
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 irreversible（loss: uncaptured）。欠陥の指摘で、選び方の対立ではない —— **直す側に失われるものが無く、直さない側だけが失う**（どちらの読みでも何かが失われる A の型ではない）。merge 前で実データは 1 件も無いので、直せば何も失われていない。人間に問う選択肢が立たないので technical として直した。**PR 本文の冒頭に、この訂正を一覧で出す**
- 当初の loss: uncaptured —— 離席の終わり（いつ戻ったか）は後から作れない
- 提案: `away` を `state_dir` に落として立て直しで読み戻す。読み戻せたら、
  起動時に `powered-off` と一緒に「印の時刻で閉じた `leave`」を出す
  （閉じた理由が違うことが読めるよう `reason` を分けるか、`range_end` の出どころを載せる）。
- 再検証（2026-09-13）: 再現した —— 一時テストで `enter`（reason=idle, range_end=null）の後に落とし、立て直して入力を戻したまま 400 秒回しても、対応する `leave` は 0 件。出たのは `powered-off`（00:11:00〜00:13:20）だけ
- 処置: fixed 11.1

## R5. 本物の HTTP を叩く経路（`HttpTransport`）にテストが 1 本も無く、`Bearer` を外しても全緑

- 成果物: crates/collector-windows/src/sender.rs:169-206
- 根拠: `authorization` ヘッダの行（sender.rs:190）を削った複製で
  `cargo test -p ashiato-collector-windows --lib` → **`test result: ok. 51 passed; 0 failed`**。
  `Transport` trait のテストはすべて `FakeTransport` / `AcceptAll` で、
  `HttpTransport` を通るテストは 0 本。`tools/smoke.sh` の ST07 の節（== 37〜39）も
  **`jq` で本文を手書きして `curl` で送っている**ので、収集側が組んだ要求は 1 度も本物の
  取り込み口を通らない。
  実害の大きさを確かめるため、本物のサーバ（`127.0.0.1:18999`）を立てて
  `HttpTransport` + `IngestRequest::of` + `heartbeat::signal` を通す一時テストを書いて実測した:
  ```
  INGEST    status=200 body=[{"id":"0198be8e-…","duplicate":false,"accepted":true,"error":null}]
  HEARTBEAT status=200 body=[{"id":"a7e3342b-…","duplicate":false,"accepted":true,"error":null}]
  REFERENCE now = Ok(2026-09-12T16:34:31Z)
  ```
  → **いまは通る。** ただし壊れたときの見え方が悪い: サーバは `Bearer ` 接頭辞と定数時間一致を
  要求し（server/src/lib.rs:131-151）、外れると 401。`ureq` の `StatusCode` は
  `Reply { status: 401, body: "" }` になり、`sender.rs` はそれを「契約から外れた応答」として
  **何も取り除かずに** `send_reply_unreadable` を 1 行吐くだけなので、
  未送信が無限に伸びるだけで誰も気づかない。
- kind: technical
- 提案: 本物のサーバを立てて `HttpTransport` で 1 件通す試験を 1 本置く（`smoke.sh` の節を
  `jq` の手書きから **収集側の crate が組んだ本文**に差し替えるのがいちばん安い）。
  併せて 401 / 403 を「網の失敗」ではなく**資格情報の不一致**として区別し、ログの種別を分ける。
- 再検証（2026-09-13）: 再現した —— `authorization` の行を削った複製で `ok. 51 passed`。本物のサーバ（127.0.0.1:18999）に `HttpTransport` で ingest 200 / heartbeat 200（いまは通る）。合言葉違いは `Reply { status: 401, body: "" }`（curl では本文 12 バイトが返っている）で、`Sender::flush` を 3 回回すと未送信 1 のまま `kind=send_reply_unreadable … error=401` が 3 行出るだけ。
  同根: pr-review の「400・401 の本文を `ureq` が捨てる」
- 処置: fixed 11.4

## R6. 画面ロック中の見回りが「取得の失敗」として数えられ、`capturable=true` と `successes=0` が同じ信号に載る

- 成果物: crates/collector-windows/src/runtime.rs:130（`counters.record(obs.foreground.is_some())`）/
  runtime.rs:246-259（`capability_of`）
- 根拠: 一時テストで、6 時間ロックしたまま（`foreground: None, locked: true`）10 秒刻みで tick した実測。
  ```
  beat emitted_at="2026-09-13T00:00:00.000Z" capturable=true attempts=1    successes=1
  beat emitted_at="2026-09-13T06:00:00.000Z" capturable=true attempts=2160 successes=0
  ```
  `capability_of` は「**ロック中は『取れない』ではない**。ここを取得不能に数えると、
  成功条件 1（NFR-13）の分子が『席にいた日』に化ける」と明記して `locked` を
  `capturable=true` に倒しているのに、直前の `counters.record` は同じ観測を
  `foreground.is_some()`（ロック中は `None`）で数えるので**失敗に入る**。
  spec の Requirement「前回の生存信号からの取得の試行回数と成功回数を生存信号に載せる」は
  形としては満たすが、載る数が意味を失う（`attempts=2160 / successes=0` と `capturable=true` が同居）。
  `attempts` / `successes` は `core.heartbeat` に凍結され（移行 0004）、区間ごとの取得率として
  画面に出る（`crates/server/src/coverage.rs:115-131` の `Interval`）。
  担保しているテストは `attempts_successes`（heartbeat.rs。`Counters` 単体で `i % 3` を数えるだけ）で、
  **ロック中の観測がどちらに数えられるかを観測していない。**
- kind: technical
- 提案: `counters.record` の成功条件を `capability_of(&obs).capturable` に揃える
  （ロック中は「試行していない」として数えないのが `capability_of` の意図と整合する）。
  「6 時間ロックしたままの生存信号が `capturable=true` かつ `successes == attempts`」を
  `Runtime` 経由のテストで固定する。
- 再検証（2026-09-13）: 再現した —— ロックのまま 6 時間（10 秒刻み）回し、2 通目が `capturable=true attempts=2160 successes=0`。（今回は起点からロックで回したので 1 通目も `successes=0`。前回の 1 通目 `successes=1` は起点で前景を読めた条件の差で、主張には効かない）
- 処置: fixed 11.5

## R7. 本人が決めた「5 秒」がどのテストからも参照されていない（0 秒にしても 51 本全緑）

- 成果物: crates/collector-windows/src/engine.rs:23（`MIN_DWELL_SEC`）
- 根拠: 値を振って `cargo test -p ashiato-collector-windows --lib` を走らせた実測。
  ```
  MIN_DWELL=0 -> test result: ok. 51 passed
  MIN_DWELL=1..6 -> いずれも ok. 51 passed
  MIN_DWELL=7 -> FAILED. 50 passed; 1 failed（title_change_makes_one_record_after_dwell）
  MIN_DWELL=3600 -> FAILED. 49 passed; 2 failed
  ```
  **緑になる範囲は 0〜6 秒。** deep Q6 で本人が選んだ「5 秒」は上からしか縛られていない。
  テストが実際に固定しているのは **「同じ題名を 2 回続けて観測するまで記録しない」という
  1 観測ぶんの遅れ**であって、5 秒の滞留ではない
  （`min_dwell_drops_title_only_churn` は 1 秒刻みで題名を変えるので、`pending` が毎回
  差し替わることだけで `made == 0` になる。`min_dwell = 0` でも通る）。
  deep Q6 の効く先は NFR-5 の容量（1 日 5,990〜15,200 件の枠）なので、
  0 に倒れると**題名が 2 秒ごとに変わるアプリが全件記録される**（枠を使い切る側）。
- kind: technical
- 提案: 「題名が `MIN_DWELL_SEC` 未満でとどまった場合は記録しない」を、
  滞留時間を明示した `with_thresholds` で**下からも**固定する
  （例: `min_dwell = 5` で 4 秒とどまった題名は落ち、6 秒とどまった題名は残る）。
- 再検証（2026-09-13）: 再現した —— `MIN_DWELL_SEC` = 0 / 6 → `ok. 51 passed`、7 → `FAILED 50/1`（`title_change_makes_one_record_after_dwell`）
- 処置: fixed 11.6

## R8. 「動作中は 1 分ごとに印を更新する」を見るテストが 1 本も無く、間隔を 1 日にしても全緑

- 成果物: crates/collector-windows/src/marker.rs:17（`TOUCH_INTERVAL_SEC`）/ runtime.rs:138-143 /
  openspec/changes/st07-active-window/tasks.md（5.1「**動作中は 1 分ごとに更新する**」検証: `cargo test last_seen_marker`）
- 根拠: 値を振った実測。
  ```
  TOUCH=1 -> ok. 51 passed      TOUCH=3600 -> ok. 51 passed      TOUCH=86400 -> ok. 51 passed
  POLL=60 -> ok. 51 passed      POLL=600  -> ok. 51 passed
  IDLE=60 -> ok. 51 passed      IDLE=340  -> ok. 51 passed        IDLE=341 -> FAILED（3 本）
  MAX_BATCH=1000000 -> ok. 51 passed
  ```
  `last_seen_marker`（marker.rs:97-108）は `Marker::touch` / `read` を直接叩くだけで、
  **`Runtime::tick` が印を進めることを観測していない。**
  design Risks は「ずれは更新間隔（1 分）以内に収まるので、日単位の判定には影響しない」と
  言い切っているが、その 1 分はどのテストからも参照されていない。
  印が進まなくなると `powered-off` の始まりは**前回の起動時刻**になり、
  「PC が止まっていた」の記録が**PC が動いて記録も出ていた時間帯を覆う**（記録が嘘になる）。
  `IDLE_THRESHOLD_SEC` は 60〜340 秒が全緑（deep Q7 は B なので可逆。design D9 の仮決めとして許容範囲）。
- kind: technical
- 提案: `Runtime::tick` を 3 分ぶん回して `last-seen.txt` が
  `TOUCH_INTERVAL_SEC` の刻みで進むことを見るテストを 1 本。
  `POLL_INTERVAL_SEC` は `wait_for_change`（`cfg(windows)`）でしか使われないので、
  固定できないなら D5 の（仮）にその旨を書く。
- 再検証（2026-09-13）: 再現した —— `TOUCH_INTERVAL_SEC` = 1 / 86400、`POLL_INTERVAL_SEC` = 600、`IDLE_THRESHOLD_SEC` = 60 / 340 → いずれも `ok. 51 passed`、`IDLE_THRESHOLD_SEC` = 341 → `FAILED 48/3`
- 処置: fixed 11.7

## R9. `platform.rs` は 0 テストで、その中身に依存する Scenario 5 本が「人間の確認待ち」にも無い

- 成果物: crates/collector-windows/src/platform.rs（185 行・`#[cfg(test)]` 0 個）/
  openspec/changes/st07-active-window/tasks.md（§人間の確認待ち）
- 根拠: `grep -c '#\[test\]' crates/collector-windows/src/platform.rs` → 0。
  `check_scenarios.py` は **人間の確認待ち 0 件**と報告する（30/30 に印がある）。
  印の先を 1 件ずつ追うと、30 本のうち 26 本は「OS を触らない層」（`engine.rs` など）で
  偽の `Observation` を食わせたテストが担保しており、その主張が実機で真になるかは
  `platform.rs` にしか無い。tasks の §人間の確認待ちに挙がっているのは 6 本だけで、
  **次の 5 本は機械にも人間にも掛かっていない**:
  | Scenario | 実機で決まるもの |
  |---|---|
  | クエリとフラグメントが残る | UI Automation が拾う `Edit` がアドレスバーで、省略されていないこと |
  | 画面ロックとスリープも残る | `LOCK_SCREEN = ["lockapp.exe","logonui.exe"]`（platform.rs:38）がロック中の前景と一致すること。ロック中に `get_active_window` がロック前のアプリを返すなら `locked=false` のまま `AwayReason::Idle` に化ける |
  | 前景を読めない状態は取得できないとして報告される | `active_win_pos_rs::get_active_window()` の Err と「前景が無い」の区別 |
  | URL を読めない状態も取得できないとして報告される | R16 のとおり、空文字は「読めた」に倒れる |
  | URL だけが変われば 1 件増える | `read_url` が同じ窓で URL の変化を追えること（`get_focused_element` から親を 16 段たどる） |
  `lock_and_suspend_are_recorded`（engine.rs）は `locked: true` を**テストが直接指定**しているので、
  ロック判定そのものは 1 度も試されていない。
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 premise。本人の決定の前提ではなく、tasks の「人間の確認待ち」の網羅と `platform.rs` の判断の置き場の穴。5 本を確認待ちへ足し、判断を `winrules.rs` に出して Linux で確かめた。本人の答えは 1 つも動かない
- 提案: 上の 5 本を tasks の §人間の確認待ちへ足す（`- Scenario: <名前>` の裸の形。
  `verify_checklist.py` はこの節しか読まない）。併せて `platform.rs` から
  OS に触らない判定（`LOCK_SCREEN` の照合・`top_level_window` の段数・
  pid 不一致の扱い）を関数に切って `cfg(windows)` の外でテストする。
- 再検証（2026-09-13）: 再現した —— `grep -c '#\[test\]' platform.rs` → 0。表の 5 本はいずれも tasks.md §人間の確認待ちに 0 件、`check_scenarios.py` は `人間の確認待ち 0`。なお表の 4 行目「R16 のとおり」は R14 を指す（番号の誤記。本文は触っていない）
- 処置: fixed 11.8

## R10. `scripts/merge_gate.sh` は 9.3 で FAIL する（9.3 が「人間の確認待ち」に無い）

- 成果物: openspec/changes/st07-active-window/tasks.md（9.3 / §人間の確認待ち）
- 根拠: `merge_gate.sh:67-75` と同じ判定を走らせた実測。
  ```
  open=[9.3]
  9.3: 確認待ちに無い（merge_gate が note する）→ [FAIL] tasks 9.3 が未完で、人間の確認待ちにも無い
  ```
  9.3 の本文は「検証: 実機でトレイに出ることを見る（**確認バッチ**）」と書いているが、
  `verify_checklist.py:60-78` は `tasks.md` の **§人間の確認待ちの節しか読まない**ので、
  この項目は確認バッチの問いにもならない。**未実装と明記しただけでは gate を通らず、
  確認バッチも拾わない**という二重の落ちどころ。
- kind: technical
- 提案: 9.3 を §人間の確認待ちへ移す（`- [ ] 9.3 トレイに常駐して「動いている」ことを見せる`
  の形。`verify_checklist.py` はこの形も読む）。移せないなら design D7 の（仮）として
  「ST07 では入れない」を決め、tasks から落とす。
- 再検証（2026-09-13）: 再現した —— `merge_gate.sh:70-75` と同じ抜き出しを HEAD の tasks.md に当てて `open=[9.3]` → `[FAIL] tasks 9.3 が未完で、人間の確認待ちにも無い`。`verify_checklist.py` は §人間の確認待ちの節しか読まない（:67）
- 処置: fixed D7

## R11. 合言葉が `Debug` にそのまま出る（doc コメントは「`Debug` にも出ない」と主張している）

- 成果物: crates/collector-windows/src/sender.rs:169-177 / crates/collector-windows/src/config.rs:33-45
- 根拠: 一時テストで実測。
  ```
  HttpTransport Debug = HttpTransport { base_url: "http://x", token: "SUPER-SECRET-TOKEN" }
  Config Debug        = Config { base_url: "http://x", api_token: "SUPER-SECRET-TOKEN", … }
  ```
  sender.rs:174 の doc は「**合言葉はログに出さない**（`Debug` にも出ないよう `telemetry` 以外へ
  渡さない）」と書いているが、`#[derive(Debug)]` がそのまま出す。
  `Transport: std::fmt::Debug` が supertrait なので `Runtime`（`#[derive(Debug)]`）を
  `{:?}` すれば連鎖して出る。いま `{:?}` する経路は無いが、`anyhow` の context に
  1 度混ぜれば PERM-8 の合言葉がログに落ちる。spec の
  「ログに出すものを、件数・ソース名・所要時間・エラーの種別に限る」に対する担保は
  `telemetry.rs` の `line()` の引数型だけで、**`Debug` 経路は塞がれていない。**
- kind: technical
- 提案: `HttpTransport` と `Config` の `Debug` を手で書き（合言葉は `"***"`）、
  「`format!("{:?}")` に合言葉が現れない」テストを 1 本置く。
- 再検証（2026-09-13）: 再現した —— 一時テストで `HttpTransport { base_url: "http://x", token: "SUPER-SECRET-TOKEN" }` / `Config { …, api_token: "SUPER-SECRET-TOKEN", … }`
- 処置: fixed 11.9

## R12. `tasks.md` の「Scenario は 23 本」が、spec の 30 本と合っていない

- 成果物: openspec/changes/st07-active-window/tasks.md:21 / :152 /
  openspec/changes/st07-active-window/specs/desktop-collection/spec.md
- 根拠: `grep -c '^#### Scenario:' …/specs/desktop-collection/spec.md` → **30**
  （Requirement は 12 本）。tasks は 2 か所で「23 本」と書いている
  （0 章「この change が足す Scenario は 23 本」/ 10.2「**23 本すべてに印**」）。
  `review/spec.md` の機械検査の引用が `[FAIL] 担保の無い Scenario: 23 件` なので、
  **spec-review が 7 本を足した後に tasks の数字が更新されていない**。
  10.2 は `[x]` になっているが、その本文の主張（23 本）は事実と違う。
- kind: technical
- 提案: 0 章と 10.2 の「23 本」を「30 本」に直す。数を本文に書くなら
  `check_scenarios.py` の出力（この change ぶんの件数）と合わせる。
- 再検証（2026-09-13）: 再現した —— spec の `#### Scenario:` 30 件（Requirement 12）。tasks.md:21 と :152 が「23 本」。review/spec.md:20 の引用は「23 件」
- 処置: fixed 11.10

## R13. `suspended` の「入った」記録に、最後の入力からの経過時間が載らない

- 成果物: crates/collector-windows/src/engine.rs:313-327（`report_suspend`）
- 根拠: `report_suspend` の `enter` は `WindowPayload::new(RecordKind::Idle, from)` に
  `transition` と `reason` だけを足し、**`idle_ms` を設定しない**（engine.rs:319-322）。
  spec の Requirement は「THE SYSTEM SHALL その記録に、最後の入力からの経過時間を
  併せて載せる」、Scenario「閾値を後から引き直せる形で残る」は
  「**その記録は最後の入力からの経過時間を持つ**」と言っている。
  担保している `idle_records_carry_elapsed_for_rethreshold`（engine.rs）は
  `reason = Idle` の出入りだけを見ており、`suspended` を観測していない。
  `lock_and_suspend_are_recorded` は `report_suspend` を呼ぶが `kind` と `range_end` しか見ない。
  `leave` 側には区間長が載るので値そのものは引き直せるが、**Scenario の主張が
  3 種類のうち 1 種類で偽**になっている。
- kind: technical
- 提案: `report_suspend` の `enter` に `idle_ms` を載せる（眠りに入った時点の経過時間、
  取れなければ 0 ではなく欠落のままにする理由を doc に書く）。
  `idle_records_carry_elapsed_for_rethreshold` を 3 つの `reason` すべてで回す。
- 再検証（2026-09-13）: 再現した —— `report_suspend` の出力は enter `idle_ms=None` / leave `idle_ms=Some(3900000)`
- 処置: fixed 11.11

## R14. UI Automation が空文字を返すと「URL を読めた」に倒れ、生存信号は満点のまま URL が消える

- 成果物: crates/collector-windows/src/platform.rs:120-129 / runtime.rs:246-259
- 根拠: `read_url` は `Ok(value) => UrlRead::Read(value)`（platform.rs:127）で、
  **空文字も `Read("")`** として返す（doc も「空なら『読めなかった』ではなく空として残す」と
  明記）。`capability_of` は `UrlRead::Unavailable` のときだけ `uiautomation` を `blockers` に
  載せるので、空文字は `capturable = true`。結果:
  - `Shown.url = Some("")` になり、その後ブラウザ内で URL だけが変わっても
    `same_except_title` が真のまま（`url` がどちらも `Some("")`）→ **題名だけの変化として
    最小滞留に掛かる**。spec の「URL の変化は滞留時間に関わらず記録される」が崩れる。
  - 生存信号は「URL を読み取る経路が応答している」と報告し続ける。
    spec の Requirement「取得できる状態の判定に … **URL を読み取る経路が応答すること**の
    両方を含める」が、半分壊れた状態を検知しない。
  深掘り Q4 は URL の欠落を `loss: uncaptured` としており、**取れなかった URL は後から作れない。**
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 irreversible（loss: uncaptured）。欠陥の指摘で、選び方の対立ではない —— **直す側に失われるものが無く、直さない側だけが失う**（どちらの読みでも何かが失われる A の型ではない）。merge 前で実データは 1 件も無いので、直せば何も失われていない。人間に問う選択肢が立たないので technical として直した。**PR 本文の冒頭に、この訂正を一覧で出す**
- 当初の loss: uncaptured —— 読み取らなかった URL は後から作れない（deep Q4）
- 提案: `read_url` で空文字を `UrlRead::Unavailable` に倒す（アドレスバーが空の瞬間は
  短く、取りこぼすより「読めなかった」と報告する側が安い）。
  「`Read("")` のとき `blockers` に `uiautomation` が載る」と
  「`Read("")` → `Read("example.com/a")` が 1 件になる」の 2 本をテストにする。
- 再検証（2026-09-13）: 再現した（収集側の判定の層まで）—— `capability_of(Read(""))` = `Capability { capturable: true, blockers: [] }`、`Read("")` のまま題名だけ変わると 0 件（滞留待ち）。**UI Automation が実機で空文字を返すかは Linux では確かめられない**（platform.rs:125-127 の分岐を読んだだけ）
- 処置: fixed 11.12

## R15. 時計のずれの基準が取り込み口の `date` なので、同じ PC 構成では常に 0 になり、どちらだったかが記録に残らない

- 成果物: crates/collector-windows/src/clock.rs:79-105（`HttpDateClock`）/ design.md D17
- 根拠: `HttpDateClock::now()` は `{base_url}/healthz` の `date` ヘッダを基準にする。
  本物のサーバで実測すると `date: Sat, 12 Sep 2026 16:34:04 GMT` が返り、経路は動く
  （`REFERENCE now = Ok(2026-09-12T16:34:31Z)`）。
  しかし `docs/requirements.md` §5 は「S-01 / D-01 / D-02 はオンプレ」「**C-02 が S-01 と
  同じ PC かどうかは定めない**」で、同じ PC なら基準は**その PC の時計そのもの**になり
  `skew_ms` は構造的に 0 前後に張り付く。design D6 はこれを承知で
  「同じ PC なら差は 0 で費用も 0」と書いているが、**記録の側に基準の出どころが 1 つも載らない**
  （`WindowPayload` は `skew_ms` だけ）。読む側は
  「独立な基準と比べて 0 だった」と「自分と比べて 0 だった」を区別できず、
  扉 #5 の「**破れたことを後から知る**」が同じ PC 構成では達成されない。
  `clock_skew_is_measured`（clock.rs）は `measure()` の算術と契機だけを見ており、
  基準の独立性には触れていない。
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 premise。D6 / D17 は C（聞かない）に落とした技術判断で、本人の決定の前提ではない。基準の出どころを列として持つ（「列を持つ」扉を開けたままにする既定）で閉じた。本人の答え（第 2 回 Q9 の補足: サーバと PC は同じ想定・オンプレ）は動かない
- 提案: 記録に基準の出どころ（`base_url` のホスト、または「同一ホストだった」の印）を 1 項目足す。
  0 が「測れた 0」なのか「自分と比べた 0」なのかが後から判るようにする。
  値は後から引き直せないので day one で。
- 再検証（2026-09-13）: 再現した —— 本物のサーバの `/healthz` が `date` を返し、`HttpDateClock::now()` = `Ok(2026-09-12T22:14:59Z)`、同じ瞬間の手元は `22:14:59.713`。`WindowPayload` に基準の出どころの欄は無い。**追加で分かったこと**: 基準が秒で切り捨てられるので、同じホストでも `skew_ms` は 0 ではなく 0〜999 ms の正の偏りを持つ（「0 に張り付く」より正確には「1 秒未満の正の値に張り付く」）
- 処置: fixed D17

## R16. `tick` が Err を返すと無言で終了し、そのあと誰も立て直さない（`powered-off` が「PC が切れていた」に化ける）

- 成果物: crates/collector-windows/src/main.rs:58-62 / runtime.rs:130-131 / tools/check-panic-log.sh
- 根拠: `main.rs` は `loop { rt.tick(&mut source, Utc::now())?; … }`。
  `tick` は毎回 `self.counter_store.save(&self.counters)?`（runtime.rs:131。**1 秒ごとに
  `counters.json` を書く**）と `self.marker.touch(now)?` と `self.events.add(req)?` を通るので、
  ウイルス対策の一時ロック・ディスク満杯・権限の変更のどれでも `Err` → `run` が返る →
  プロセス終了。`start "" /min` の窓も同時に消えるので、**ログもエラーも残らない。**
  `tools/check-panic-log.sh` は server（`/selftest/panic`）だけを見ており、
  収集側には panic hook も再試行もトレイ（9.3 は未実装）も無い。
  気づく手段は生存信号の不在（最短 6 時間）だけで、次の起動時には
  `powered-off` が 1 件書かれる —— **「PC を閉じていた」と「C-02 が死んでいた」は
  同じ `kind: "powered-off"` になる。** deep Q8 の「効く先」は
  「FR-82（Q1）が PC の停止期間を残すので、通知が鳴った理由が後から
  『PC を閉じていた』か『C-02 が死んでいた』かに分かれる」と書いているが、
  **実装ではこの 2 つが分かれない。**
  併せて `maybe_measure_skew`（runtime.rs:186-208）は基準時刻が取れないとき `mark` しないので、
  サーバが落ちている間は**毎秒 `/healthz` を叩き、毎秒ログを 1 行出す。**
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 premise。指摘の中身は「深掘り Q8 の効く先（通知が鳴った理由が後から分かれる）が実装で成り立っていない」。**前提そのものは正しく、実装が満たしていなかった**ので、実装を前提に合わせた（`boot_at` / `clean_stop` / 止まらない見回り）。本人の答え（通知を許容する）の根拠が戻るだけで、答えは動かない。**PR 本文で本人に見せる**
- 提案: (a) `tick` の Err を種別で分け、書き込み失敗などは記録して回り続ける
  （記録そのものを落とすより安い）。(b) `.cmd` から `powershell` の再起動輪など
  最低限の立て直しを入れるか、少なくともログをファイルへ落とす。
  (c) 終了の仕方（正常終了 / エラー終了 / 電源断）を印に書き分け、
  `powered-off` の記録に「前回どう終わったか」を 1 項目載せる（Q8 の主張を実装で成立させる）。
  (d) `maybe_measure_skew` は測れなかったときも次の契機まで待つ。
- 再検証（2026-09-13）: 再現した —— 一時テストで `counters.json` を書けなくすると `tick` = `Err("数えを書けない: Is a directory (os error 21)")`（main.rs:60 の `?` でプロセスが終わる）。基準時刻が取れない間は 10 tick で基準を 10 回叩いた。`tools/check-panic-log.sh` は server の `/selftest/panic` だけ、収集側に panic hook は 0 件。本文の `maybe_measure_skew（runtime.rs:186-208）` は HEAD では 172-194 行。
  同根: pr-review の「HTTP に timeout が無い」（(d) の毎 tick の再試行が、応答しない基準では tick そのものを止める形で重なる）
- 処置: fixed D23

## R17. 時計が戻ると、戻った幅だけ「ここまで動いていた」の印が止まり、その間の停止期間が記録されない（進めば眠っていないのに `suspended` が立つ）

- 成果物: crates/collector-windows/src/runtime.rs:118-124（眠りの判定）/ :137-143（印の更新）/ :172-229（契機）/
  crates/collector-windows/src/marker.rs:63-70 / clock.rs:45-47 / heartbeat.rs:158-163
- 根拠: 一時テストで、1 秒刻みに 120 秒回したあと時計を 2 時間戻し、そのまま 30 分回してから落として 400 秒後に立て直した実測。
  ```
  巻き戻し前の印                 = 2026-09-13T00:02:00Z
  戻った後 30 分動いた時点の印   = 2026-09-13T00:02:00Z（いまは 2026-09-12 22:30:00）
  戻った後 30 分間の POST 回数   = 0
  起動後の powered-off 記録      = 0 件
  ```
  契機はすべて `now - last >= 間隔`（壁時計の差）で、差が負のあいだは**印の更新も送信も生存信号もずれの測定も止まる**。
  止まったまま電源が落ちると、起動時の `powered_off_span` は `last >= now` で `None` を返す（marker.rs:68）ので、
  **その停止期間は 1 件も残らない**。印は戻った幅（ここでは 2 時間）だけ古いまま凍る。
  ずれの測定も次の契機が「戻る前の時刻 + 1 時間」になるので、**時計が破れた直後に限って測らない**（扉 #5 の「破れたことを後から知る」の逆）。
  逆向きも実測した: 時計が 3 分進むだけで、眠っていないのに
  `suspended enter 00:00:10 → leave range_end 00:03:10` の 2 件が凍結される（`SUSPEND_GAP_SEC` は壁時計の差だけで判定）。
  併せて `SUSPEND_GAP_SEC = 2` にした複製でも `ok. 51 passed`（下から縛るテストが無く、tick が 2 秒詰まっただけで偽の眠りが立つ値でも通る）。
  `powered_off_span_is_one_record` は「印が未来なら作らない」を担保しているが、**印が未来になる経路（動いている最中の巻き戻り）を Runtime で観測していない。**
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 irreversible（loss: uncaptured）。欠陥の指摘で、選び方の対立ではない —— **直す側に失われるものが無く、直さない側だけが失う**（どちらの読みでも何かが失われる A の型ではない）。merge 前で実データは 1 件も無いので、直せば何も失われていない。人間に問う選択肢が立たないので technical として直した。**PR 本文の冒頭に、この訂正を一覧で出す**
- 当初の loss: uncaptured —— 印が止まっている間に電源が落ちた停止期間（FR-82）と、巻き戻り直後のずれの測定（扉 #5）
- 提案: 契機（印・送信・生存信号・ずれ・眠りの判定）の経過を `std::time::Instant`（単調時計）で測り、壁時計は記録の時刻にだけ使う。
  壁時計が `last` より前に戻ったことを検知したら、その場でずれを測って記録に残す。
  「時計を戻しても 1 分以内に印が進む」「壁時計だけが 3 分進んでも `suspended` が立たない」を Runtime 経由のテストにする。
- 処置: fixed 11.16

## R18. 印と数えを切り詰めてから上書きするので、書きかけで電源が落ちると次のログオンから起動しなくなる

- 成果物: crates/collector-windows/src/heartbeat.rs:116-130（`CounterStore`）/ marker.rs:37-55（`Marker`）/
  runtime.rs:74-75, 105-106, 131 / main.rs:55-57
- 根拠: 一時テストで、書きかけ（空）の状態ファイルを置いて立ち上げた実測。
  ```
  counters.json が空   → Runtime::new = Err("数えが読めない: EOF while parsing a value at line 1 column 0")
  last-seen.txt が空   → start       = Err("印が読めない: …/last-seen.txt: premature end of input")
  もう一度起動         → start       = Err（同じ。誰も印を書き直さない）
  ```
  どちらも `std::fs::write`（ファイルを切り詰めてから書く）で上書きしており、`counters.json` は **tick ごと（1 秒ごと）**（runtime.rs:131）、
  `last-seen.txt` は 60 秒ごとに書かれる。**同じ crate の `Outbox::rewrite` は一時ファイル + `rename` を使っている**（outbox.rs:390-394。
  「途中で落ちても書きかけの全件にならない」と自分で書いている）のに、この 2 つには当てていない。
  読む側は「壊れていても空として扱わない」（marker.rs:35-36。`broken_marker_is_an_error` が固定）で Err を返し、
  受ける `main.rs` は `?` で終わるだけなので、**書きかけが 1 回起きると、利用者が状態ファイルを消すまで毎回のログオンで即終了する**
  （R3 と同じく `start "" /min` の窓ごと消えるので見えない）。
  印は電源断のため（FR-82）にあるのに、**電源断が書き込みと重なると収集そのものが止まる**。
  「壊れた印を初回起動に化かさない」という判断（停止期間を 1 件失わない）が、それより大きい「以後の全部」を失う形になっている。
- kind: technical
- kind 訂正（2026-09-13 処置のとき）: 当初 irreversible（loss: uncaptured）。欠陥の指摘で、選び方の対立ではない —— **直す側に失われるものが無く、直さない側だけが失う**（どちらの読みでも何かが失われる A の型ではない）。merge 前で実データは 1 件も無いので、直せば何も失われていない。人間に問う選択肢が立たないので technical として直した。**PR 本文の冒頭に、この訂正を一覧で出す**
- 当初の loss: uncaptured —— 起動できなかった期間の前景・離席・停止期間のすべて
- 提案: `CounterStore::save` と `Marker::touch` を一時ファイル + `rename` に揃える。
  読めない印・数えは退避して起動を続け、`powered-off` に「前回の終わりが読めなかった」印を載せる（初回起動には化かさない）。
  「空の `last-seen.txt` / `counters.json` があっても Runtime が回り始める」テストを 1 本置く。
- 処置: fixed 11.13

## R19. 生存信号とずれの測定の間隔は部品単体でしか縛られておらず、Runtime の `mark` を消して毎秒出しても全緑

- 成果物: crates/collector-windows/src/runtime.rs:172-213（`maybe_measure_skew` / `maybe_beat`）/
  heartbeat.rs:229（`heartbeat_interval_is_expected_gap`）/ clock.rs:118（`clock_skew_is_measured`）/ runtime.rs:437（`ticks_send_records_beats_and_skew`）
- 根拠: 複製で実測。
  ```
  self.beat_schedule.mark(now); を削る                  -> ok. 51 passed
  self.skew_schedule.mark(now); を削る                  -> ok. 51 passed
  if !self.beat_schedule.due(now) を if false && … にする -> ok. 51 passed
  ```
  `mark` を 2 つとも削った複製で `Runtime` を 1 秒刻みに 10 分回すと、**送られた生存信号 401 件・clock-skew 401 件**
  （正しくはどちらも 1 件）。Scenario「想定間隔ごとに生存信号が届く」「1 時間ごとにずれの測定記録が残る」の印の先は
  `Schedule` / `SkewSchedule` 単体で、**Runtime が契機を守ることは観測していない**
  （`ticks_send_records_beats_and_skew` は `/heartbeat` と `clock-skew` が「送られた」かだけを見る）。
  smoke の == 39 は手書きの本文を 1 件送るだけ（R5）。
  毎秒になると clock-skew が 1 日 86,400 件になり、NFR-5 のウィンドウの枠（1 日 5,990〜15,200 件）を単独で超える。
  生存信号の数えも毎秒 `take` されるので `attempts=1` の区間が並び、区間ごとの取得率が意味を失う。
- kind: technical
- 提案: Runtime を注入時計で 1 日（1 秒刻みか 10 秒刻み）回し、送られた生存信号が 4 件・clock-skew が 24 件になることを見るテストを 1 本置く
  （`heartbeat_interval_is_expected_gap` / `clock_skew_is_measured` の 1 日ぶんの数えを Runtime の層へ上げる）。
- 処置: fixed 11.14

## R20. `Outbox::open` が読み取り失敗を飲んで空として開き、次の書き直しで溜まっていた全件を消す

- 成果物: crates/collector-windows/src/outbox.rs:33（`if let Ok(text) = read_to_string`）/ :97-108（`rewrite`）
- 根拠: pr-review-toolkit（silent-failure-hunter の 3 / code-reviewer の A-6）。`NotFound` 以外（共有違反・権限・不正な UTF-8）も
  空として `Ok` を返し、最初の `remove` → `rewrite` が `pending`（新しい分だけ）で置き換える。
  再現: 不正な UTF-8 を置いた置き場で `Outbox::open` が `Ok` を返した（直す前）→ `unreadable_outbox_is_an_error_not_empty` で固定
- kind: technical
- 処置: fixed 11.17

## R21. 応答の件数と送った件数の不一致を `zip` が黙って切り、ずれた位置で断られた 1 件を取り除きうる

- 成果物: crates/collector-windows/src/sender.rs:143（`batch.iter().zip(results.iter())`）
- 根拠: silent-failure-hunter の 8。契約は「送った順の配列・位置で対応づける」だが長さを検査していなかった。
  `mismatched_reply_length_removes_nothing` で「2 件送って 1 件の応答 → 何も取り除かない」を固定
- kind: technical
- 処置: fixed 11.18

## R22. `ureq` の既定で 400 / 401 の本文が捨てられ、HTTP に timeout が無い

- 成果物: crates/collector-windows/src/sender.rs:187-206 / clock.rs:88
- 根拠: code-reviewer の A-1 / A-2、silent-failure-hunter の 9 / 12。`ureq-3.4.1/src/config.rs:884` の
  `http_status_as_error: true` で 400 の本文が落ち、全件拒否のバッチを永久に送り直す。`Timeouts::default()` は全部 `None`
  で、応答しない相手に見回りごと止まる（2 分を超えると眠りに化ける）。
  `http_transport_sends_bearer_and_reads_400_body`（本物の TCP で 400 を返す）と `http_transport_has_timeouts` で固定
- kind: technical
- 処置: fixed 11.4

## R23. 離席のまま画面が自動でロックされると、ロックした時刻がどこにも残らない

- 成果物: crates/collector-windows/src/engine.rs:212-230（`(Some(Idle), Some(Locked))` が `_ => {}` に落ちる）
- 根拠: code-reviewer の A-7。Windows の既定は「無操作 → 画面オフ → ロック」で、離席が先に立つのが日常の順序。
  `idle_then_lock_records_the_lock` で固定
- kind: technical
- 処置: fixed 11.19

## R24. 経過時間が読めない状態が報告されず、離席が閉じず、変換できない値が 0 秒に化ける

- 成果物: crates/collector-windows/src/engine.rs:193 / runtime.rs:246-259（`capability_of`）/ platform.rs:150
- 根拠: silent-failure-hunter の 4 / 5 / 10、pr-test-analyzer の G3。`blocker` は 2 つだけで `obs.idle` を見ていなかった。
  `IdleRead::Unavailable` で `away` を見る前に return していた。`Duration::from_std(d).unwrap_or_else(zero)`。
  `unreadable_idle_closes_the_span_eventually` / `unreadable_idle_is_not_zero` / `capability_follows_the_url_path` で固定
- kind: technical
- 処置: fixed 11.20

## R25. URL が読めなかったことが記録に残らず、読み取りの揺れが「URL の変化」の偽の記録になる

- 成果物: crates/collector-windows/src/engine.rs:61-66（`UrlRead::value()` が `Unavailable` と `NotBrowser` を畳む）
- 根拠: code-reviewer の B-1、pr-test-analyzer の I1。`Read("x") → Unavailable → Read("x")` で滞留を無視して 2 件出ていた。
  **形の凍結前に決める必要がある**指摘なので merge 前に `url_unavailable` を足した（列を持つ既定）。
  `url_flapping_is_not_a_change` / `payload_shape_is_pinned` で固定
- kind: technical
- 処置: fixed 11.21

## R26. 生存信号の `capturable` / `blockers` が信号を出す瞬間の 1 観測だけから決まる

- 成果物: crates/collector-windows/src/runtime.rs:129（毎 tick 上書き）/ :196-213
- 根拠: code-reviewer の B-3、silent-failure-hunter の 6、pr-test-analyzer の I2。6 時間 UI Automation が死んでいても最後の 1 秒が
  読めれば満点になる（NFR-13 の訂正 (2) が名指しした型）。UI Automation を開けなかった失敗も黙っていた。
  `blockers_are_sticky_within_the_interval` で固定
- kind: technical
- 処置: fixed 11.22

## R27. `ASHIATO_STATE_DIR` だけが既定（`.`）を持ち、起動のしかたで置き場が変わる

- 成果物: crates/collector-windows/src/config.rs:60
- 根拠: silent-failure-hunter の 11。同じ関数の doc が「既定で埋めると送り先を間違えたまま動く」と書いているのに 1 つだけ外れていた。
  `config_requires_every_var_including_state_dir` で固定
- kind: technical
- 処置: fixed 11.23

## R28. `exclusions.json` の書き間違い（知らない欄・`rules` の欠落・空の `value`）が「除外なし」に化ける

- 成果物: crates/collector-windows/src/exclusion.rs:47 / :59（`#[serde(default)]`・未知の欄の黙認）
- 根拠: silent-failure-hunter の 14、pr-test-analyzer の G4（空の `title-contains` のガードに試験が無かった）。
  `broken_registration_is_an_error_not_empty`（7 通りの書き間違い）/ `rules_hit_by_path_name_and_title` で固定
- kind: technical
- 処置: fixed 11.24

## R29. プロセス名が解決できない前景が空の名前で進み、プロセス名で指した除外をすり抜ける

- 成果物: crates/collector-windows/src/platform.rs:153-160（`unwrap_or_default()`）
- 根拠: silent-failure-hunter の 15。空の名前ではロック画面の判定も外れる。`process_name_is_none_not_empty` で解釈を固定し、
  前景は「読めなかった」に倒す（`blockers` に `foreground`）
- kind: technical
- 処置: fixed 11.25

## R30. 壊れた行の退避が前回の退避を上書きで消し、件数も出ない。断られた分の覚えが単調に増える

- 成果物: crates/collector-windows/src/outbox.rs:42-50 / sender.rs:149
- 根拠: silent-failure-hunter の 13 / 16。`broken_line_is_quarantined_not_dropped`（2 回目の退避が 1 回目を消さない）で固定
- kind: technical
- 処置: fixed 11.26

## R31. 未送信の追記と書き直しを同期していない（電源断で最後の追記・書き直しの中身が飛ぶ）

- 成果物: crates/collector-windows/src/outbox.rs:69 / :106
- 根拠: silent-failure-hunter の 19。`marker.rs` 自身が電源断を想定している。追記に `sync_data`、書き直しを `fsutil::atomic_write`（同期してから置き換え）へ
- kind: technical
- 処置: fixed 11.27

## R32. 二重に起動すると、2 つが同じ未送信を書き直し合って相手の追記を消す

- 成果物: crates/collector-windows/src/main.rs（単一起動の検査が無い）
- 根拠: silent-failure-hunter の 20。自動起動と手での起動が重なる経路がある
- kind: technical
- 処置: fixed D24

## R33. `Runtime::stop` に呼び手が無い（終了時に除外の数えと滞留待ちの題名が落ちる）

- 成果物: crates/collector-windows/src/runtime.rs:153 / main.rs:59-62
- 根拠: code-reviewer の B-4、pr-test-analyzer の I4。`grep '\.stop('` で 0 件。終了の合図（Ctrl-C・窓を閉じる）で `stop` を呼び、
  合図の無い落ち方は置き場の状態（D21）で次の起動が閉じる
- kind: technical
- 処置: fixed 11.1

## R34. CI の windows job が `cargo check` なので、作業場の lint が `cfg(windows)` の中身に 1 度も当たらない

- 成果物: .github/workflows/ci.yml（`collector-windows` job）
- 根拠: code-reviewer の B-5、pr-test-analyzer §4。ubuntu の clippy は `platform.rs` をコンパイルしない
- kind: technical
- 処置: fixed 11.29

## R35. `Engine` / `Foreground` / `Runtime` の `Debug` に題名・URL・除外した対象の見分けが出る

- 成果物: crates/collector-windows/src/engine.rs（`#[derive(Debug)]`）/ runtime.rs
- 根拠: pr-test-analyzer §4（R11 と同じ型を本文の側で）。`tracing` の `?field` 1 つで漏れる。
  `debug_does_not_leak_private_content` で固定
- kind: technical
- 処置: fixed 11.9

## R36. `Browsers` の試験がプロセス共有の環境変数を書き換える（並列の試験と競合する）

- 成果物: crates/collector-windows/src/browsers.rs（`env_adds_to_the_defaults`）
- 根拠: pr-test-analyzer §4。`with_extra` を切り、環境変数を触らない形にした
- kind: technical
- 処置: fixed 11.30

## R37. 除外の「件数」が入場を数えるのかが定まっておらず、2 本の試験で数え方が違う

- 成果物: crates/collector-windows/src/engine.rs（`exclusion` は 4 回の変化を起こして件数を見ず、`excluded_count_is_kept` は 3 を固定）/ design.md D18
- 根拠: pr-test-analyzer §4。design D18 に「対象を前景にしたこと自体を 1 回」と書き、`exclusion` でも 4 を固定した
- kind: technical
- 処置: fixed D18

## R38. `powered-off` / `idle` / `excluded` も `core.event` の行なので、PC を開かなかった日に「① 記録あり」が立ちうる

- 成果物: crates/server/src/coverage.rs（「① 記録あり」を `core.event` の行の有無で引く）/ docs/collector-contract.md §C-02
- 根拠: pr-test-analyzer §4。翌朝の起動で書かれる `powered-off`（`event_time` は前夜）が前夜の日に落ちる。
  数え方の正典は `collection-coverage`（ST02。凍結中）で、達成日数が動くので本人に問う型
- kind: defer
- 処置: followup ST02

## R39. 断られた分だけの当たり直し・1 回の件数の切り・閾値ちょうどの境界に試験が無い

- 成果物: crates/collector-windows/src/sender.rs:93-98 / :17 / engine.rs（`MIN_DWELL_SEC` / `IDLE_THRESHOLD_SEC`）
- 根拠: pr-test-analyzer の I5 / I6 / I9（`>=` を `>` にしても、`.take(MAX_BATCH)` を消しても緑だった）
- kind: technical
- 処置: fixed 11.31

## R40. 滞留を満たした題名がアプリの切り替えで捨てられ、眠っていた間が滞留に数えられる

- 成果物: crates/collector-windows/src/engine.rs:246 / :287 / :313-328（`report_suspend` が `pending` を触らない）
- 根拠: code-reviewer §読みにくさ。見回りの前に切り替わると 6 秒前景にあった題名が落ちていた。
  `dwelled_title_survives_app_switch` / `suspend_does_not_count_as_dwell` で固定
- kind: technical
- 処置: fixed 11.32

## R41. 基準時刻を取れない間、毎秒 `/healthz` を叩いて毎秒ログを出す

- 成果物: crates/collector-windows/src/runtime.rs:172-194（失敗時に `mark` しない）
- 根拠: pr-test-analyzer の I8、R16 の (d)。`SkewSchedule::failed` で 1 分後に測り直す。`skew_failure_backs_off` で固定
- kind: technical
- 処置: fixed D17

## R42. `/healthz` の応答に `date` ヘッダがあることを誰も見ていない

- 成果物: tools/smoke.sh / crates/collector-windows/src/clock.rs（`HttpDateClock`）
- 根拠: pr-test-analyzer の I8。無くなるとずれを 1 件も測れず、扉 #5 の「破れたことを後から知る」が黙って消える
- kind: technical
- 処置: fixed 11.33

## R43. `zone_has_both_id_and_offset` が実行環境のタイムゾーン設定に依存する

- 成果物: crates/collector-windows/src/config.rs（`zone_has_both_id_and_offset`）
- 根拠: pr-test-analyzer §4
- kind: technical
- 処置: rejected: **読み取りの本物の経路を通すための試験**で、依存は意図どおり。WSL（`Asia/Tokyo`）で通ることを実測した。ubuntu の runner（`Etc/UTC`）では PR の CI で確かめる（落ちたらこの処置を撤回して直す）。偽物に差し替えると `iana_time_zone` が読めない環境（FR-20 を満たせない環境）に気づく手段が無くなる

## R44. `idle_records_carry_elapsed_for_rethreshold` の最後の assert が直前の固定値の同義反復

- 成果物: crates/collector-windows/src/engine.rs（`assert!(span_ms < 600_000)`）
- 根拠: pr-test-analyzer §4
- kind: technical
- 処置: fixed 11.11

## R45. `powered_off_is_not_stopped` は語彙の不一致だけを見る代理で、「並んだときに判別できる」を見ていない

- 成果物: crates/collector-windows/src/contract.rs（`powered_off_is_not_stopped`）
- 根拠: pr-test-analyzer §4
- kind: technical
- 処置: rejected: 2 つは**別の表に置く**（`powered-off` は `core.event`、意図的な停止は `core.coverage_span.kind = stopped`。design D10）ので、同じ日に並べても行の置き場で判別でき、残るのは「語彙が重ならない」ことだけ —— それを試験が固定している。日の状態を引いたときにどう読むかは `collection-coverage` の担当で、R38 として ST02 へ申し送った


---

## 手ごとの結果

- **手 1（固定値の独立な再計算）: 不一致なし。**
  `payload_shape_is_pinned`（contract.rs）が直書きしている 5 本の期待文字列を
  Python の `json.dumps(OrderedDict, ensure_ascii=False, separators=(',',':'))` で組み直し、
  時刻の算術（+30 分 / +12 時間、ミリ秒・`Z` 終わり）も `datetime` で別に計算した →
  **5 本すべて一致**（`OK foreground / OK idle / OK powered-off / OK excluded / OK clock-skew`）。
  `EXPECTED_GAP_SEC = 21_600` は全移行を当てた本物の DB から独立に確認
  （`SELECT expected_gap_sec … WHERE logical_source='c02-window'` → `21600`）。
  `external_id_kind` も `none` を実測。**実装の出力を写した値ではない。**
- **手 2（ガードをわざと壊す）: 12 本のうち 11 本は赤くなった。1 本が素通り（R5）。**
  | 壊したもの | 結果 |
  |---|---|
  | 除外の判定を外す（`if false && …`） | FAILED 3 本 |
  | `payload` の項目名を `appName` に | FAILED 3 本 |
  | `marker` の初回起動の分岐（`last_seen?`）を外す | FAILED 1 本 |
  | 収集側が `sensitivity: 3` を付けて送る | FAILED 1 本 |
  | URL に `https://` を補う | FAILED 2 本 |
  | 除外の `excluded_count` を載せない | FAILED 2 本 |
  | 時刻の刻みを秒にする | FAILED 2 本 |
  | 時計が戻ったときの守り（`last >= now`）を外す | FAILED 1 本 |
  | 壊れた除外の登録を空に倒す | FAILED 1 本 |
  | 壊れた印を「初回起動」に化かす | FAILED 1 本 |
  | `accepted` を見ずに全部取り除く | FAILED 2 本 |
  | **`authorization: Bearer` を送らない** | **ok. 51 passed（R5）** |
  併せて `check-boundaries.sh` を検査の外側から確かめた ——
  `crates/collector-windows/Cargo.toml` に `ashiato-server` を足した複製で rc=1
  （`NG 収集側がサーバの内部に依存している`）。**空振りしていない。**
  `check-licenses.sh` は `cargo metadata --all-features` を使うので
  `cfg(windows)` の依存も見ている（`uiautomation: Apache-2.0` /
  `active-win-pos-rs`・`user-idle-time`・`ureq`: `MIT OR Apache-2.0` を実測。264 件）。
- **手 3（Scenario と test の突合）: 30/30 に印があり、うち 8 件で主張の階層とテストの階層が違う。**
  内訳は R2（除外の件数。`Engine` 単体では真、`Runtime` 経由では偽）/
  R4（離席の出入り。1 プロセス内でだけ真）/ R9（5 本。実機の層にしか無い主張を偽の観測で担保）/
  R13（`suspended` の経過時間）/ R14（URL の変化）。
  「既定の感度で格納される」は `tools/smoke.sh` == 38 だけが印を持つが、
  本物の DB で `sensitivity=1` を確かめているので階層は合っている
  （`crates/server/src/registry_tests.rs` でも同じ列を見ている）。
- **手 4（本人の決定 9 件が test で固定されているか）: 7 件は固定、2 件は未固定。**
  | deep | 決めたこと | 固定されているか |
  |---|---|---|
  | Q1 | PC の停止期間を 1 件残す | ○（`powered_off_span_is_one_record`。初回起動・時計が戻った場合も落ちる）。ただし R8 / R16 |
  | Q2 | NFR-13 の分母 | — ST02 へ defer。`docs/handoff/ST02.md:52-88` に実在（`fix/nfr13-c02-denominator`）。**穴に落ちていない** |
  | Q3 | 感度は既定のまま（外部 AI 可） | ○（`sensitivity_uses_collection_default` / `registry_tests` / smoke == 38。`sensitivity: 3` を付けると赤） |
  | Q4 | URL は補正しない | ○（`url_is_not_normalized`。`https://` を補うと赤）。ただし R14 |
  | Q5 | 除外の本文は送らない | ○（`exclusion` が直列化結果に本文が出ないことを見る。判定を外すと赤）。件数は R1 / R2 |
  | Q6 | アプリと URL は間引かない | ○（`app_switch_ignores_min_dwell` / `url_change_ignores_min_dwell`） |
  | Q6 | **題名の最小滞留 5 秒** | **×（0〜6 秒が全緑。R7）** |
  | Q7 | 離席の出入りを 1 件として残す | ○（`idle_transitions`）。**閾値 5 分は 60〜340 秒が全緑**（B なので可逆。R8 に併記） |
  | Q8 | 想定間隔 6 時間のまま | ○（`heartbeat_interval_is_expected_gap` が `EXPECTED_GAP_SEC` をリテラルで縛る。60 / 43200 で赤） |
- **手 5（tasks の `[x]` と実体）: 33 件の検証方法を全部走らせた。1 件が数字違い（R12）、1 件が gate に引っかかる（R10）。**
  `cargo test <名前>` 20 本はすべて rc=0 で 1 本以上が実際に走る
  （`foreground` → 3 本 / `min_dwell` → 3 本 / `exclusion` → 4 本 / `powered_off_span` → 2 本、
  残りは 1 本ずつ）。**テスト名が挙がっているのにテストが無いものは 0 件。**
  `grep` の検証も実測（2.1 `c02-window` → 1 / 6.4 `除外` → 9 / 9.2 `自動起動` → 1）。
  10.x の 7 本もすべて rc=0。
  なお `c02_window_external_id_kind_is_not_record` は DB が落ちていると rc=101 で落ちる
  （`smoke.sh` が末尾で `docker compose down -v` するため。手順としては tasks の
  「DB を使う検査は `docker compose up -d db` が前提」どおり）。
- **手 6（隙間）: 6 件。** R1（除外の件数が 5 分ぶん消える）/ R3（自動起動が起動しない）/
  R4（離席が閉じない）/ R6（ロック中の数え）/ R14（空文字の URL）/ R16（無言終了と
  `powered-off` の化け）。**`Outbox` はディスクに落ちているので、記録そのものは
  ST01 の型（メモリだけで 5 分ぶん消える）を踏んでいない** ——
  `outbox_survives_restart` / `outbox_survives_outage` / `counters_survive_restart` /
  `broken_line_is_quarantined_not_dropped` が実際に固定している。
  消えるのは **`Engine` が抱える 3 つの状態（`pending` / `away` / `excluded`）**だけで、
  そこだけが `state_dir` に落ちていない。
- **再検証（2026-09-13）での追加（手ごとのやり残し）**:
  - 手 1: `payload_shape_is_pinned` の 5 本を Python（`json.dumps` + `datetime`）で組み直し、**5 本とも一致**を再確認
  - 手 2: 前回の 12 本のうち 6 本を当て直した（除外の判定を外す → FAILED 3 / 時計の守り `last >= now` を外す → FAILED 1 /
    `accepted` を見ない → FAILED 2 / URL に `https://` を補う → FAILED 3 / 時刻の刻みを秒にする → FAILED 2 / Bearer を送らない → ok 51）。
    `check-boundaries.sh` は `ashiato-server` 依存を足した複製で rc=1。**前回の表と一致**
  - 手 3: 契機の配線を追加で突いた。生存信号・ずれの `mark` / `due` を外しても全緑（**R19**）
  - 手 4: deep の外の定数も振った。`SKEW_INTERVAL_SEC=7200` / `EXPECTED_GAP_SEC=43200` / `SEND_INTERVAL_SEC=86400` / `SUSPEND_GAP_SEC=100000` はそれぞれ FAILED 1。
    `SUSPEND_GAP_SEC=2` と `MAX_BATCH=1000000` は `ok. 51 passed`（前者は R17 に併記）
  - 手 5: tasks に挙がった `cargo test <名前>` 21 本を `--workspace` で全部走らせ、すべて rc=0・1 本以上が実際に走る
    （`foreground` 3 / `min_dwell` 3 / `exclusion` 4 / `powered_off_span` 2 / 他 1）。grep の 3 本も 1 / 9 / 1。**前回と一致**
  - 手 6: 前回が挙げた型（プロセスの再起動・権限・応答が読めない）に加え、**時計が戻る**（R17）と**状態ファイルの書きかけ**（R18）を実測した。
    どちらも ST07 の中の状態（印・数え・契機）なので defer ではない
