# CH09 design — 実時間で「〜秒以内」を求める試験を混み具合で落ちない形にする

> 第 1 回の深掘りの推奨の答え（Q0〜Q10）と、聞かずに決めたもの（D12・D13。印「C」）で先に書いた。
> 本人の答えが推奨と違えば、その Q を持つ D を書き直す（各 D の反転条件）。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-04 の repo）:

- 1 の試験（`crates/collector-windows/src/runtime.rs:3612` `history_large_fetch_keeps_observing_foreground`）は、偽の履歴の読み手を**止めずに**（`FakeReader::new`）
  30,000 件を読ませ、見回り 1 回ごとの実時間の最大（`slowest`）を `POLL_INTERVAL_SEC`（1 秒）と比べる。見回りの側の仕事は 1 回 2,000 件（`QUEUE_PER_TICK`）を積むだけで
  （`history/collect.rs:401-480`）、落ちた 1.34 秒はその手間が混み具合で延びたものと読める。止める仕組み（`FakeReader::gated`、`:2355`）は既にあり、
  `history_slow_read_does_not_disturb_window`（`:3549`）が使っている
- 2 の試験（`crates/server/src/archive_scan_interval_tests.rs:227`）は、走査の数が増えたのを見た直後に止めた時計を 1,001ms 進める（`:255-259`）。取り込み器は数を足した後に
  DB の往復をもう 1 回してから眠りに入り、眠りの起点はそのときの時計で決まる（`archive/worker.rs:1544-1571`・`:2226-2249`）。往復が `settle(25)` より長いと、その歩で走査が
  止まったままになる。上限を延ばしても直らない（`review/deep.md` R1 の模型）
- 同じ形の候補は Rust に十数か所、画面の単体試験に暗黙の 1 秒の待ちが 317 か所（23 ファイル）、携帯の単体試験に `await(5, SECONDS)` が数か所（`premises.md` 4 行目）
- 手元に `stress` / `stress-ng` は無い（16 コア）

## Goals / Non-Goals

**Goals:**
- 1 の試験が実時間を計らずに「見回りが履歴の取得を待たない」を確かめ、待つ実装なら確実に落ちる（spec `test-timing` の 1 つ目の Requirement）
- 2 の試験と同じ形の待ちが、混み具合では落ちず、見ている性質は弱めない（2 つ目。正典の 2 本の Scenario の書き直し）
- 負荷の下で続けて通ること・壊すと落ちることを、書いたまま走る台本で確かめる（3・4 つ目）
- 製品のコードを変えず、既存の試験を減らさない（5 つ目）

**Non-Goals:**
- 製品のコード（見回り・取り込み器）の振る舞いの変更（依頼の「範囲の外」。Q0・Q7）
- 並走の数を減らすなどハーネス側の対策（依頼の「範囲の外」）
- 画面の e2e（playwright）・携帯の計測テスト（`app/src/androidTest`）・縦串の台本（`tools/smoke.sh`）・Windows でだけ組まれる `crates/collector-windows/tests/runtime_windows.rs` を直すこと（Q2。一覧に「探した／探していない」を書くだけ）
- 速さそのものを確かめる性能の試験を足すこと（Q1 の 4 つめの選択肢は採らない）

## Decisions

### D1（仮）. 1 の試験は、止めた読み ＋ 見張り ＋ 1 回に積む件数で確かめる（Q1）

- 読み手は `FakeReader::gated` にする。1 回目の読み（帳面を作る取得）は試験がすぐ放す。2 回目（30,000 件）の読みは止めたまま、24 時間後へ飛んだ後に
  見回りを 5 回以上回し、その間に前景を `editor` → `browser` へ切り替える。読みを放す前に、切り替えた前景がウィンドウのソースに積まれていることを見る
- **見張り**: 別スレッドが `recv_timeout(30 秒)` で「見回りが返った」の合図を待つ。来なければ、放す側の `Sender` の写しで読みを放し、印（`AtomicBool`）を立てる。
  見回りは読みが終われば返るので、試験は見回りの直後に印を見て `panic!("見張りが読みを放した: 見回りが履歴の取得を待った")` で落ちる。
  待つ実装なら、止めた読みの最初の見回りで必ずここに来る（混み具合に依らない。30 秒は見回りの普段の所要時間より 3 桁長い）
- 読みを放した後は、見回りの前後で積まれた履歴の記録（`c02-browser-history`）の差を取り、その最大が `QUEUE_PER_TICK` 以下であることを見る。
  `queueing_ticks >= (N - 10) / QUEUE_PER_TICK`・積みこぼし（`N` 件ちょうど）・積み込みの間に眠り（`suspended`）が入らないこと・積んでいる途中の前景の変化が
  積み込みの後にならないこと、の assert は残す（依頼の 1）
- `slowest` と `POLL_INTERVAL_SEC` の比較は消す。試験の名前・`// Scenario: 訪問の多い取得でも前景の観測は止まらない` の印は残し、spec `test-timing` の
  「履歴の読みを止めている間も…」「読みを放した後、見回り 1 回で積む…」の印を足す
- 捕まえない退行（受け入れたもの）: 帳面の書き込み（`Stage::Saving`）を待つ退行。止めた読みでは捕まらない。Q0 の推奨（口を足さない）と Q1 の推奨のとおり
- 反転条件: Q1 が「名前付きパイプで書き込みも止める」なら、`.tmp` の場所に FIFO を置く別の試験を足す（Linux 専用・その取得は失敗で終わるので積みこぼしの assert とは分ける）。
  「差し込み口で止める」なら Q0 が「足してよい」のときだけ `history/collect.rs` に `#[cfg(test)]` の口。「性能の試験として残す」なら `#[ignore]` の試験に時間の確かめを移す

### D2（仮）. 2 の試験は、届かなければ同じ歩を進め直し、時計の経過を 60 秒未満で見る（Q8）

- 歩ごとに `advance(1001ms)` → `settle(25)` → 走査の数が `before + step` に届くのを**最大 1 秒（実時間）**待つ。届かなければ同じ歩でもう一度 `advance(1001ms)` する。
  進め直しは時計の経過を 1 秒ずつ使うだけで、確かめを弱めない（間隔 1 秒なら 10 回は 10〜20 秒で入る）
- 打ち切り: 時計の経過が 60 秒に達するか、試験全体の実時間が 30 秒（D4）を超えたら落ちる。文言は
  `走査が {n} 回に届かない: 最後に見た数 {last}・時計の経過 {virtual:?}・待った時間 {real:?}`（spec「取り込み器が変えた間隔を読まないように壊すと…」が `待った時間` を見る）
- 終わりの assert は「変えた後の 10 回が時計の経過 60 秒未満」（正典 `external-ingestion` の Scenario を MODIFIED で書き直す）。古い間隔のまま眠る退行は 10 回に 1,200 秒かかる
- `settle(500)`（:250。取り込み器が眠りに入るのを待つ）は残す。観測する口が無く（Q0）、進め直しがあるので、眠りに入る前に変えても落ちない（D5）
- 同じ型の `scheduled_tests.rs`（刻み 100ms・200 回の待ち 5 か所）は、待ちの上限だけを D4 にし、刻みと進め方は変えない（落ちた記録が無い。一覧に「同じ競り合いの型・刻みで避けている」と書く）
- 反転条件: Q8 が「刻みを細かく」なら `scheduled_tests.rs` と同じ 100ms 刻みにし、正典の Scenario は「10 秒の間に 5 回以上」のまま（MODIFIED を外す）。
  「眠りに入ったことを観測」なら Q0 が「足してよい」ときだけ取り込み器に `#[cfg(test)]` の印を足し、「10 秒ちょうど」を残す（MODIFIED を外す）

### D3（仮）. 洗い出しの範囲は、関門で走る 3 か所（Q2）

- 探して直す: `crates/` の Rust（`cargo test --workspace` で組まれるもの）・`web/src/__tests__`・`collector-android/app/src/test`
- 探して一覧に載せるだけ: `crates/collector-windows/tests/runtime_windows.rs`（`#![cfg(windows)]`。関門でも CI でも走らない）・`crates/collector-windows/examples/browser_history_smoke.rs`（例。試験でない）
- 探さない（一覧に「探していない」と書く）: `web/e2e`・`collector-android/app/src/androidTest`・`tools/*.sh`
- 反転条件: Q2 が「Windows の試験も直す」なら `runtime_windows.rs` を直し、Windows の clone で 1 回走らせる手順を tasks に足す（人間の確認待ちでなく、`~/dev/ashiato` の
  メモどおり WSL から Windows 側の cargo）。「e2e なども探して載せる」なら探す範囲を足す

### D4（仮）. 条件を待つ上限は、実時間の期限で一律 30 秒（Q5）

- 回数（`for _ in 0..200` × 25ms）でなく、`Instant`（Rust）・`System.nanoTime`（Kotlin）の期限で切る。間隔はいまのまま
- 落ちたときの文言に、待った時間と最後に見た値を出す
- Rust の手伝い: サーバは `testdb.rs` に `wait_until`（期限 30 秒・間隔を引数）を置いてよい。PC の収集アプリは `runtime.rs` などの `mod tests` の中に置く。手伝いを作るかは下流が呼び出しの数で決める
- **除く**: 試験の中の時刻を兼ねるループ（`runtime.rs` の `tick_until_read`・`tick_until_read_again`。1 回ごとに `t(sec)` を進める）は回数のまま。期限に変えると時刻が
  最大 6,000 秒前後まで進み、1 時間で効く規則をまたぐ。時刻を固定する `tick_until_history_idle` は期限に変えてよい
- 反転条件: Q5 が「いまの上限の 6 倍」なら、期限でなく回数を 6 倍にする

### D5（仮）. 条件を待たない固定の眠りは、観測できる条件があれば条件の待ちにする（Q4）

- 観測できる条件がある眠り（相手の結果が DB や戻り値で見えるもの）は、その条件の待ち（D4）に変える
- 観測できない眠り（取り込み器が眠りに入ったか、timer を処理したか）は残し、一覧に「固定の眠り・観測の口が無い（Q0）・余裕」を書く。既知のもの:
  `archive_scan_interval_tests.rs` の `settle(500)`（:250）・`settle(25)`（:258）・`advance_secs(600, 25)`（:207。初期値 120 秒の試験。600 秒で 4〜6 回の幅がある）
- 反転条件: Q4 が「一律に延ばす」なら固定の眠りを延ばす（試験は遅くなる）。「触らない」なら待ちの上限だけを直す（2 の試験は D2 だけ）

### D6（仮）. 打ち切りの試験は相手の長さと上限を一緒に延ばし、計算の形の試験は直さない（Q3）

- 打ち切り: `testdb_tests.rs:371`（statement timeout 1 秒）は `pg_sleep(5)` → `pg_sleep(60)`、「5 秒未満」→「30 秒未満」（正典 `test-database` の Scenario を MODIFIED）。
  `time_sync.rs:650`（打ち切り 200ms）は `sleep 30` → `sleep 60`、「5 秒未満」→「30 秒未満」。打ち切りが効けば普段の所要時間は変わらない
- 計算の形: `stay_tests.rs:2389`（作り直し 1 日が 3 秒未満なら「隣をたどっていない」）は直さず、一覧に「計算の形・数で見る口が無い（Q0）」を書く。
  製品のコードに触れずに数で見られるもの（たとえば作り直しが読む行の数が DB から見える）が見つかったときだけ数に変える
- 反転条件: Q3 が「どちらも直さない」なら 2 か所とも一覧だけ（正典 `test-database` の MODIFIED を外す）。「上限を延ばす」なら `stay_tests.rs` も 30 秒にする

### D7（仮）. 負荷の台本 `tools/load-repeat.sh`（Q6）

- 形: `tools/load-repeat.sh [--times N] [--load host|db|none] [--expect-fail] -- <命令…>`・`--self-test`。既定は `--times 10 --load host`
- 負荷: `host` は `nproc` 本の bash の busy loop（`while :; do :; done &`）。`db` は DB のコンテナの中で同じ busy loop（`tools/db.sh` の `exec` から）。
  `trap` で EXIT・INT・TERM のどれでも止める。道具は足さない
- 出力: 各回の rc と所要時間、最後に `pass <k>/<N>`。rc は spec のとおり（全部通る 0・1 回でも落ちる 1。`--expect-fail` は逆）
- 証跡: change の `load-proof.sh before` は**直す前の木**（Task 1 の時点）で 1 と 2 の試験に `--expect-fail` を当て、結果を `load-before.txt` に
  `history: yes|no` / `scan: yes|no`（落ちる回が出たか）で残す。`host` で再現しなければ `db` で試し、それでも再現しなければ `no` のまま残す（直したことの証が弱いと書く）。
  `load-proof.sh after` は直した後に 2 本を 10 回ずつ通す
- 反転条件: Q6 が「直した後の 10 回だけ」なら `before` を落とす。「1 回きりの `bash -c`」なら台本を作らず tasks の検証に書く

### D8（仮）. 製品の不具合の疑いは直さない（Q7）

- 洗い出しの途中で、製品が本当に待っている・遅すぎると分かったら、その試験は直さず、一覧に「製品の不具合の疑い」と根拠を書き、PR 本文に「別の change に出す」と書く。
  `docs/handoff/` には書かない（宛先の Story が無い。人間が `hx change` で登録する）
- 反転条件: Q7 が「小さければ直す」なら、その直しを tasks に足し、D10 の検査の除外にそのファイルと行を書く

### D9（仮）. 画面の単体試験の暗黙の上限は、設定で一括して上げる（Q9）

- `web/src/__tests__/setup.ts` で `configure({ asyncUtilTimeout: 10_000 })`（`@testing-library/react` から）。`web/vitest.config.ts` の `test` に `testTimeout: 30_000`
- 確かめ: 画面の単体試験を 1 本足し（`web/src/__tests__/ch09-timeouts.test.ts`）、`getConfig().asyncUtilTimeout` と vitest の設定の `testTimeout` を読む
- 試験の中に `{ timeout: N }` を書いた箇所は、10 秒より短ければ一覧に載せて同じ形で延ばす
- 反転条件: Q9 が「待ちも 30 秒」なら 30,000 / 60,000。「触らない」なら設定を変えず一覧だけ（spec の Scenario を外す）

### D10. 製品のコードの差分が 0 行であることは、検査で見る（Q0）

- change の中の `check-product-unchanged.sh`: 分岐点（`git merge-base origin/main HEAD`）から HEAD までの差分で、`crates/**/*.rs` のうち試験のファイル
  （`*_tests.rs`・`testdb.rs`・`tests/`）でないもの、`collector-android/app/src/main`、`web/src`（`__tests__` を除く）の変わった行を拾う。
  Rust のファイルは、その版で最初の `#[cfg(test)]` の行（その次に `mod tests` が来るもの）より後ろの行だけを試験のモジュールとみなす。
  外で変わった行があればファイルと行を挙げて rc=1。`--self-test` で、外を変えた木・中だけを変えた木の 2 つを作って見る
- `web/vitest.config.ts` は設定（D9）で、`web/src` の外なので対象にならない
- 反転条件: Q0 が「`#[cfg(test)]` の口なら足してよい」なら、`#[cfg(test)]` の付いた項目の中の行も試験のものとして数える

### D11（仮）. 壊すと落ちることは、使い捨ての作業木に差分を当てる台本で見る（Q10）

- 形: `tools/break-check.sh --patch <差分> --expect <文言> [--timeout 秒] -- <命令…>`・`--self-test`。`git worktree add --detach <一時の場所> HEAD` に
  `git apply` し、その中で命令を走らせ、`git worktree remove --force` で消す（`trap` で必ず）。元の木の `target/` を `CARGO_TARGET_DIR` で共有してよい（組み直しの時間を減らす）
- 壊す差分は change の中に置く:
  - `break-wait-on-read.patch`: `crates/collector-windows/src/history/collect.rs` の取得の段（`Stage::Reading`、`:401-480`）で、読みのスレッドの終わりを待つ（`is_finished` を見て返る所を
    `join` で待つ形に）。期待する文言は `見張りが読みを放した`（D1）
  - `break-ignore-interval.patch`: `crates/server/src/archive/worker.rs` の `wait_for_next_scan`（`:2226-2249`）で、登録簿の間隔を眠りの前に 1 回だけ読み、眠っている間は読み直さない。
    期待する文言は `待った時間`（D2）
- 差分は凍結の前に一度空走させる（当たること・期待する文言で落ちること。`review/deep.md` R7）
- 反転条件: Q10 が「手で壊して貼る」なら台本を作らず、証跡に出力を貼り、`git diff --exit-code` で戻したことを見る

### D12（C）. 「起きない」を短い窓で見る試験・経過の下限で見る試験は、直さず分類して載せる

- `UnavailableLogGateTest.kt:72`（200ms のうちに終わらない、を `assertFalse`）・`web_session_tests.rs:273`（経過 ≥ 300ms）。混むと見逃す側へ弱まるだけで、落ちはしない
- 一覧の分類に「起きないを短い窓で見る」「下限で見る」を足す

### D13（C）. 既存の試験の名前・印・本数を減らさない

- change の中の `check-test-names.sh`（CH03 の同じ名の台本を写し、画面の単体試験の `it(` / `test(` の名前を足す）。分岐点と比べて、消えた名前・印が無く、本数が減っていないこと
- 1 の試験・2 の試験の名前は変えない（正典の Scenario の印がその試験にある）

### D14. 洗い出しの一覧の置き場と形

- `openspec/changes/ch09-load-tolerant-timing-tests/inventory.md` に表で書く。列: 場所（`path:line` と試験の名前）・型（次の 8 つ）・方針（直した形 / 直さない）・理由
  - 型: `待たない`（1 の形）/ `条件の待ち`（D4）/ `固定の眠り`（D5）/ `打ち切り`（D6）/ `計算の形`（D6）/ `起きない・下限`（D12）/ `暗黙の上限`（D9）/ `速さが要件`（依頼の 3）
  - 末尾に「探した範囲・探していない範囲」（D3）と「製品の不具合の疑い」（D8。無ければ「無し」）
- change の中の `check-inventory.sh`: 候補を機械で拾い、候補を持つファイルが全部 `inventory.md` に出ることを見る（rc=1 で抜けたファイルを挙げる）。拾う形:
  Rust は `elapsed()`・`Instant::now()`・`recv_timeout(`・`for _ in 0..<数>` の後 5 行以内に `sleep(`/`settle(` があるもの。
  TypeScript は `waitFor`/`findBy`（ファイルの単位で 1 行にまとめてよい）・`timeout:`・`setTimeout(`。Kotlin は `await(<数>, TimeUnit`・`Thread.sleep(`・`withTimeout`・`nanoTime`・`currentTimeMillis`
- PR 本文（下流の `/story finish`）は `inventory.md` の表を写す（依頼の 3）

### 既知の候補（上流が読んだもの。下流は `check-inventory.sh` の候補で完成させる）

| 場所 | 型 | 方針（推奨の答えで） |
|---|---|---|
| `crates/collector-windows/src/runtime.rs:3612` `history_large_fetch_keeps_observing_foreground` | 待たない | D1 |
| `crates/collector-windows/src/runtime.rs:1936` `clock_worker_slow_read_does_not_hold_the_patrol`（「30 秒未満」） | 待たない | 既に止めた読み手（`SlowReference`）。経過の比較を D1 の見張りの形に替える |
| `crates/collector-windows/src/runtime.rs:2499`・`:3462` `tick_until_read`・`tick_until_read_again` | 条件の待ち（時刻を兼ねる） | 回数のまま（D4 の除く） |
| `crates/collector-windows/src/runtime.rs:2512` `tick_until_history_idle`・`:2470`・`:2545` | 条件の待ち | D4 |
| `crates/collector-windows/src/history/collect.rs:868`・`:969`（「10 秒未満」で読みの終わりを待つ） | 条件の待ち | D4 |
| `crates/collector-windows/src/time_sync.rs:650` `time_sync_timeout_kills_the_child` | 打ち切り | D6 |
| `crates/server/src/archive_scan_interval_tests.rs:157` `wait_for_scans`・`:227` | 条件の待ち（＋競り合い） | D2・D4 |
| `crates/server/src/archive_scan_interval_tests.rs:207`・`:250`・`:258` | 固定の眠り | D5（残す） |
| `crates/server/src/scheduled_tests.rs:103`・`:289`・`:389`・`:436`・`:514` | 条件の待ち | D4（刻みは変えない。D2） |
| `crates/server/src/testdb_tests.rs:371` `ch04_statement_timeout_cancels_a_long_query` | 打ち切り | D6 |
| `crates/server/src/stay_tests.rs:2389` | 計算の形 | 直さない（D6） |
| `crates/server/src/outage_api_tests.rs:419`（ロックで待つまでの 200 回） | 条件の待ち | D4 |
| `crates/server/src/web_session_tests.rs:273` | 起きない・下限 | 直さない（D12） |
| `collector-android/app/src/test/.../UnavailableLogGateTest.kt:34`・`:36`・`:58`・`:62`・`:69`・`:75` | 条件の待ち | D4（30 秒） |
| `collector-android/app/src/test/.../UnavailableLogGateTest.kt:72` | 起きない・下限 | 直さない（D12） |
| `web/src/__tests__`（`waitFor`/`findBy` 317 か所・23 ファイル） | 暗黙の上限 | D9 |
| `crates/collector-windows/tests/runtime_windows.rs` | （Windows だけ） | 載せるだけ（D3） |

## Risks / Trade-offs

- [Saving を待つ退行は捕まらない] → D1 で受け入れた。Q1 の別の選択肢で捕まえられる（反転条件）
- [2 の試験は「1 秒以内に効く」を粗くしか見ない] → 古い間隔の退行は捕まる（D2）。正典の Scenario の文を確かめられる形に書き直し、嘘の印を残さない
- [期限 30 秒で、本当に壊れた試験は落ちるまでが延びる] → 待ち 1 か所ごとに最大 30 秒（画面は 10 秒）。下流の fix の往復が延びうる（Q5・Q9 の why）
- [`load-proof.sh before` で再現しない] → `db` の負荷でも試し、それでも出なければ `no` と書く。直した後の 10 回は、そのときは「直したことの証」としては弱い（D7）
- [`break-check.sh` の作業木の組み立てが遅い] → `CARGO_TARGET_DIR` を元の木と共有する。共有中に元の木で cargo を走らせない（tasks の順で避ける）
- [走っている st53・st55 と同じファイル] → proposal の Impact。1 行目（SPDX）と別の箇所で、後から merge する側が rebase で追う
