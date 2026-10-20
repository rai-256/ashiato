## Context

動機は `proposal.md` の Why。**この版は深掘り第 1 回の推奨の答えで書いた**（`deep.md` は答え待ち）。いまのコードの形（深掘りの手順 5 と `review/deep.md`）:

- 登録簿 `core.source` は `expected_gap_sec`（FR-35 の想定間隔）・`alert_after_days`（ST14）・`retired_on` などを持つが、**取り方と取得の間隔を持たない**。静的な行は 23 本、ほかに `worker::ensure_myactivity_source` が製品ごとに `c03-myactivity-u*` を足す
- 取り込み器の見回りの間隔は環境変数 `ASHIATO_ARCHIVE_SCAN_SEC`（既定 120 秒。`archive/config.rs`）。`tools/smoke.sh` が `1` を設定している。試験は `ArchiveConfig { scan_sec, .. }` を直接組む
- 走査（`worker::spawn_inspecting`）は `tokio::time::sleep(scan_sec)` で寝るだけで、前回の時刻を持たない
- 申し送り（`docs/handoff/ST13.md`）: R25 = 記録 → 台帳の `read` 行 → `archive_ledger_source` が別々の自動コミット / R34 = `scan.rs` が 2 つの置き場を `?` で直列に読み、専用のフォルダを作る処理が無い /
  M7 = `consecutive_failures` を 0 に戻す経路が無い（R89 で印付けの失敗もこの数に入る）/ M8 = `open.rs` が `.json` の I/O 失敗と zip を開けない I/O 失敗を `unsupported_format` / `broken_zip` と呼ぶ
- 携帯の収集アプリは休止を防ぐ手を持たない（`AndroidCapability.kt` は剥がれた後を報告するだけ）。生存信号は `raw` に収集側の原文（JSON）を持ち、S-01 は素通しで残す
- S-7 設定（`web/src/SettingsView.tsx`）は「全記録の書き出し」の節だけ（ST33）

## Goals / Non-Goals

**Goals**: 登録簿が取り方と間隔を持ち、サーバが回すものはそれに従う。後のソースが登録簿の 1 行と取る手を足すだけで乗る定期取得の枠。NFR-12 を崩している 5 つ（R25・R34・M7・M8・休止）を直す。S-7 で全ソースの取り方と間隔を読める。

**Non-Goals**:
- 定期取得するソースを足すこと（Q1 推奨: いま無い）。Google の API・ドライブ経由の取得
- 間隔を画面から変えること（Q6 推奨）・形の確認の印を画面から置くこと（Q5 推奨）
- 申し送りの R32・R35・M2〜M6・M9・M10（D16（仮））
- 端末と PC の収集の間隔を登録簿から収集側へ配ること（D9（仮））
- 定期取得のソースの生存信号（D6（仮））

## 決定の一覧

| D | 何を | 出所 | 仮か |
|---|---|---|---|
| D1 | 登録簿に取り方・契機・取得の間隔の 3 列を足し、DB の制約で守る | Q1（推奨）と Q1 の「聞かずに決める既定」（想定間隔と別の列・長い間隔は断る） | — |
| D2 | 取り方と契機を指定しない行は、名前の前置きから既定を入れる | 走っている ST09 の移行と `ensure_myactivity_source` が列を知らずに行を足すため | 仮 |
| D3 | 既存の 23 行の取り方・契機・間隔 | Q1（推奨）/ 収集側の定数 | — |
| D4 | 見回りの間隔は走査の周ごとに登録簿から読み、環境変数は起動を断る | Q1（推奨） | — |
| D5 | 定期取得の枠: 起動の直後に 1 回、戻らない時計、前回の時刻を持たない | Q1 の「聞かずに決める既定」 | — |
| D6 | 定期取得のソースの生存信号は枠に持たせない | AI が詰めた | 仮 |
| D7 | R25: 台帳の行と論理ソースごとの行を 1 トランザクション。記録は 1 件ずつのまま | Q2（推奨）/ ST12 D4（本人の決定 Q6） | — |
| D8 | R34 / M7 / M8 の直し方 | Q2（推奨） | — |
| D9 | 端末と PC の間隔は見せるための写し | AI が詰めた | 仮 |
| D10 | `GET /sources` と S-7 の「ソース」節の形 | Q3（推奨の形）/ Q6（推奨） | 仮 |
| D11 | 間隔の書き方（秒・分・時間） | AI が詰めた | 仮 |
| D12 | 休止の除外の状態の読み方・頼み方・載せ方 | Q4（推奨） | 仮 |
| D13 | 形の確認の印は CLI のまま | Q5（推奨） | 仮 |
| D14 | 間隔は画面から変えない | Q6（推奨） | 仮 |
| D15 | 定期取得するソースはいま無い。FR-15 の ★ は record の段 | Q1（推奨） | — |
| D16 | 申し送りのうち直すのは R25・R34・M7・M8 だけ | Q2（推奨） | 仮 |

## D1. 登録簿に取り方・契機・取得の間隔の 3 列を足す（Q1 推奨・Q1 の既定）

`core.source` に 3 列を足す移行 1 本（`YYYYMMDDHHMM_source_acquisition.sql` と `.down.sql`、`migrations/ORDER` の末尾へ 1 行）:

| 列 | 型 | 値 |
|---|---|---|
| `acquisition` | `text NOT NULL` | `device` / `pc` / `archive` / `server` / `scheduled`（画面の 端末 / PC / 書庫 / サーバ / 定期取得） |
| `fetch_trigger` | `text NOT NULL` | `interval` / `on_change` / `authored` / `derived` / `once` / `manual` |
| `fetch_interval_sec` | `integer NULL` | 契機が `interval` のときだけ値を持つ |

制約（CHECK。名前付きで、移行の guard は `pg_constraint` を `conrelid` で引く —— M9 の型を新しく作らない）:
- `(fetch_trigger = 'interval') = (fetch_interval_sec IS NOT NULL)`
- `fetch_interval_sec BETWEEN 1 AND expected_gap_sec`（想定間隔より長い取得の間隔は断る。Q1 の既定）
- `logical_source <> 's01-archive-inbox' OR fetch_interval_sec <= 300`（D4）
- `acquisition` と `fetch_trigger` の値の集合

**なぜ別の列か**（Q1 の既定）: 想定間隔は FR-35 の途絶の判定の閾値で、過去の格子もこれで引く。同じ列にすると、見回りを速めただけで過去の途絶の判定が変わる。
**なぜ DB の制約か**: 「断る」を画面の API に置くと、psql や移行から入った値が素通りする。変える口はいま登録簿（psql と移行）だけなので、断るのは DB の役目。

本人が変えた値を戻さない: 埋める `UPDATE` は `WHERE acquisition IS NULL`（列を足した直後だけ当たる）。移行を当て直しても値を上書きしない。

## D2（仮）. 取り方と契機を指定しない行は、名前の前置きから既定を入れる

`BEFORE INSERT` の trigger が、`acquisition` が NULL の行に前置きから取り方を入れ（`c01-` → `device` / `c02-` → `pc` / `c03-` → `archive` / `s01-` → `server`。それ以外は例外で断る）、
`fetch_trigger` が NULL なら取り方ごとの既定（`device` / `pc` → `on_change`、`archive` → `manual`、`server` → `derived`）を入れる。`scheduled` は前置きから入れない。

**なぜ**: 走っている ST09 の移行（`c01-screenshot` / `c01-image` / `c01-video`）と `ensure_myactivity_source`・`testdb::source` は、3 列を知らずに `INSERT` する。
NOT NULL だけにすると、ST09 が後から merge したときにその移行が落ちる（merge の順序に依存させない）。ST09 の 3 本は「変わったとき」で合っている（保存の知らせで取る）。

**反転条件**: 前置きの規則に当たらない論理ソースが出てきたら（`c04-` など）、その Story が規則を足す。既定が実態と違う行が出たら（間隔で取るのに `on_change`）、その行を足した Story が移行で値を指定する。

## D3. 既存の行の値（Q1 推奨）

| 論理ソース | 取り方 | 契機 | 取得の間隔 |
|---|---|---|---|
| `c01-location` | device | interval | 60 |
| `c01-app-usage` | device | interval | 1800 |
| `c01-app-usage-rollup` | device | once | — |
| `c01-photo` | device | on_change | — |
| `c01-clock` | device | interval | 3600 |
| `c02-window` | pc | on_change | — |
| `c02-browser-history` | pc | interval | Windows の収集アプリが履歴を読む間隔（下流が `crates/collector-windows/src/` の定数を読んで写す。想定間隔 86400 以下） |
| `c03-*`（静的な 13 本・`c03-myactivity-*`） | archive | manual | — |
| `s01-archive-inbox` | server | interval | 120 |
| `s01-stay` | server | derived | — |
| `s01-attribute` / `s01-feeling` / `s01-place` | server | authored | — |

端末と PC の値は収集側のコードの定数の写し（D9（仮））。下流は、写した値が収集側の定数と一致することを試験で確かめる（Rust の定数は `cargo test` で、Kotlin の定数は値を grep する台本で）。

## D4. 見回りの間隔は走査の周ごとに登録簿から読む（Q1 推奨）

`spawn_inspecting` の `sleep` の直前に `SELECT fetch_interval_sec FROM core.source WHERE logical_source = 's01-archive-inbox'` を読む（1 周 1 回。読めなければ前の値で寝て `warn` を出す）。
`ArchiveConfig::scan_sec` は消す。試験は登録簿の値を `UPDATE` して縮める（`archive_flow_tests.rs` の 1 秒）。`tools/smoke.sh` の `export ASHIATO_ARCHIVE_SCAN_SEC=1` は、S-01 の起動後に登録簿を `UPDATE` する行に置き換える
（共有の台本の**途中の行の書き換え**で、末尾への書き足しではない）。

`config::from_values` は `ASHIATO_ARCHIVE_SCAN_SEC` があれば `bail!`（「見回りの間隔は登録簿の s01-archive-inbox の fetch_interval_sec で設定する」）。黙って無視すると、本人が値を変えたつもりで効かない。

**300 秒の上限**: 読み始めは「大きさが 2 回の走査で変わらない」とき（ST12）なので、置いてから読み始めるまで最大で間隔の 2 倍。10 分以内（ST12 の Requirement）を守るには 300 秒以下。

## D5. 定期取得の枠（Q1 の既定）

新しいモジュール `crates/server/src/scheduled.rs`:

- `trait Fetcher { fn logical_source(&self) -> &str; async fn fetch(&self, pool, user_id) -> anyhow::Result<()> }` と、本番の取る手の一覧 `fetchers() -> Vec<Box<dyn Fetcher>>`（**いまは空**）
- 起動時に登録簿の `acquisition = 'scheduled'` の行を読み、取る手のある論理ソースごとに tokio の task を 1 つ起こす。取る手の無い行は `warn!(kind = "scheduled_no_fetcher", logical_source)` で飛ばす
- 各 task: すぐ 1 回 `fetch` → 終わったら登録簿から間隔を読み直し → `tokio::time::sleep(間隔)` → 繰り返す。**task が 1 つなので同じソースは重ならない**。失敗は `warn!(kind = "scheduled_fetch_failed", logical_source)` で、次の契機へ進む
- 壁時計（`Utc::now` / `SystemTime`）を読まない。`tokio::time` は戻らない時計（`Instant`）で測る。前回の時刻を DB に持たない
- `spawn` は `WorkerHandle` と同じく落とすと止まる手を返し、`lib.rs` の起動で取り込み器の隣に握る（利用者が未設定なら起こさない。取り込み器と同じ `ASHIATO_ARCHIVE_USER_ID`）

**なぜ前回の時刻を持たないか**（Q1 の既定）: 持つと「前回が未来にある」（時計が戻った）・「止まっていた間に契機を何回過ぎたか」を決めることになる（ST08 Q7 ほかで繰り返し出た型）。
持たなければ、起動の直後の 1 回が追いつきの 1 回になる。取得元が「前回から」の新着を返す形なら、取る手が自分の取得済みの印を持つ（後の Story が決める）。

**試験**: 本番の一覧は空なので、試験は取る手を差し込める `spawn_with(fetchers, …)` を使い、`tokio::time::pause` / `advance` で回数を数える。
「壁時計が戻っても」は、(a) 枠のモジュールが壁時計を読まないことを `grep` で見る台本と、(b) 試験の中で壁時計を読む取る手が 1 時間戻った値を見ても回数が変わらないこと、の 2 つで確かめる。

## D6（仮）. 定期取得のソースの生存信号は枠に持たせない

FR-78 の生存信号（取得できる状態か・試行と成功）は、取る手ごとに中身が違う（資格情報が切れた・取得元が止まった）。枠は `fetch` の成否しか知らない。
**反転条件**: 最初の取る手を足す Story が、枠の成否で生存信号を作れば足りると判断したら、枠に持たせる。

## D7. R25: 台帳の行と論理ソースごとの行を 1 トランザクション（Q2 推奨）

`worker.rs` の読み手で、読み終えた後の台帳の `read`（と `unreadable` の項目を持つ行）の `INSERT` と、その書庫の `archive_ledger_source` の `INSERT` を 1 つの `sqlx::Transaction` にまとめる。
**記録の格納（`PgSink` を通る `store_requests`）はまとめない** —— ST12 の D4（本人の決定 Q6: 1 件 1 トランザクションの格納の関門）の境界を変えない。

途中で落ちたとき: 台帳に `read` の行が無いので、次の走査で読み手が台帳を引いて（ST12 の R24）読み直し、記録は重複判定（同じソースの中の内容の鍵）で増えない。最終日は論理ソースごとの行から導くので出る。

**試験の失敗の差し込み**: 試験の DB にだけ、特定の `sha256` の `archive_ledger_source` への `INSERT` で例外を投げる trigger を作る（`testdb.rs` の補助）。本番のコードに試験用の分岐を置かない。

## D8. R34 / M7 / M8 の直し方（Q2 推奨）

- **R34**: `scan_once_with_hasher` は 2 つの置き場を別々に `list_dir` し、読めない置き場は `blockers` に積んで、読めた置き場の候補を返す（戻り値を `(候補, 読めない置き場)` にする）。
  生存信号の成功回数の定義（2 つの置き場をどちらも読めた走査）は変えない。`spawn_inspecting` の起動時に、専用のフォルダが無ければ `create_dir_all`（失敗したら `blockers` に出るだけで止めない）。ダウンロードのフォルダは作らない（本人の OS のフォルダ）
- **M7**: 読み手が読み終えた（台帳に `read` を書けた）とき、その `path` の `archive_sighting` の `consecutive_failures = 0, retry_after = NULL` にする。
  `scan.rs` が同じ `path` の `sha256` が前回と違うと見たら、同じ 2 列を戻してから候補にする
- **M8**: `open.rs` の `.json` の `std::fs::read` の失敗と、`.zip` の `File::open` の失敗を新しい種別 `file_unreadable` にする（zip として解けない `ZipArchive::new` の失敗は `broken_zip` のまま）。
  `web/src/messages.ts` に `file_unreadable` の文言（「ファイルを読み出せませんでした。置き直すか、ファイルの権限を確かめてください」）を足す。台帳の `reason_kind` の CHECK に値があれば移行で足す

## D9（仮）. 端末と PC の間隔は見せるための写し

収集側（C-01 / C-02）は自分の定数で動き、登録簿の値を読まない。登録簿の値は S-7 に出すための写し。
**反転条件**: 収集の間隔を本人が変えたくなったとき（電池・量）。そのときは登録簿を正本にして収集側へ配る口を作る（別の Story）。

## D10（仮）. `GET /sources` と S-7 の「ソース」節

読み出しの口 `GET /sources`（ほかの読み出しと同じ認証。`lib.rs` の `authorize`）。OpenAPI に載せる（`src/bin/openapi.rs`）。応答:

```json
{"sources": [{"logical_source": "c03-youtube-watch", "display_name": "YouTube の視聴履歴",
  "acquisition": "archive", "fetch_trigger": "manual", "fetch_interval_sec": null,
  "latest_at": null, "final_day": "2026-08-28", "retired_on": null,
  "alert_after_days": 75, "last_archive_on": "2026-08-28", "alert_days_left": 22}],
 "device_notice": {"unused_app_restrictions": "not_exempt"}}
```

- `latest_at`: 書庫以外の論理ソースの `max(event_time)`（利用者で絞る。削除済みを含めない）。書庫は `final_day`（ST12 の取り込み済みの最終日）
- `last_archive_on` / `alert_days_left`: `collection-coverage` の「書庫のソースは最後の書庫から登録簿の日数で途切れとする」と**同じ関数**で出す（`outage_store.rs` の最後の書庫の導き方を共有する。2 つ目の計算を書かない）。日は `Asia/Tokyo`。退役した行は `null`
- `device_notice`: 利用者の `c01-location` の最新の生存信号の `raw` の `unused_app_restrictions`（無ければ `null`）
- 並びはサーバが決める（取り方の順 → `display_name`）。画面は受け取った順に、取り方ごとの見出しの下に並べる

画面（`web/src/SettingsView.tsx` に「ソース」節、`web/src/sources.ts` に型と取得、文言は `messages.ts`）: Q3 の推奨の形（`proto.html` の初期表示）。
1024 px は 4 列（見出し・契機・最終・注記）、360 px は 2 段（見出し / 契機 · 最終 / 注記）。読むだけ（入力・ボタンを置かない）。書き出しの節の前に置く。

**反転条件**: Q3 で本人が別の形を選んだら、その形に。ST15 のソースごとの停止が乗るときは、行に操作が足される（ST15 が決める）。
**Risks**: `max(event_time)` を論理ソースごとに引くのは、索引（`event_by_user_source_*`）の有無で重さが変わる。下流は `EXPLAIN` で索引が使われることを確かめ、使われなければ索引を足す移行を同じ移行に入れる。

## D11（仮）. 間隔の書き方

`fetch_interval_sec` が 3600 で割り切れれば「N 時間ごと」、60 で割り切れれば「N 分ごと」、それ以外は「N 秒ごと」。契機の文言は 変わったとき / 本人が書いたとき / 位置から導く（`derived`）/ 始めたときに 1 回 / 手作業・2 か月ごと（書庫）。
**反転条件**: 本人が GO 資料や確認バッチで読みにくいと言ったとき。

## D12（仮）. 休止の除外（Q4 推奨）

- 読む: `androidx.core` の `PackageManagerCompat.getUnusedAppRestrictionsStatus`（`core-ktx 1.17.0` が持つ）。結果を 4 つに写す:
  `DISABLED` → `exempt` / `API_30_BACKPORT`・`API_30`・`API_31` → `not_exempt` / `FEATURE_NOT_AVAILABLE` → `unsupported` / `ERROR` と例外 → `unknown`
- 頼む: `MainActivity` の初回の流れ（権限の要求の後）で `not_exempt` なら、説明のダイアログを出し、応じたら `IntentCompat.createManageUnusedAppRestrictionsIntent` を開く。
  自動で出したことを `SharedPreferences` に 1 つの印で覚え、2 回目からは自動で出さない。`not_exempt` の間は `MainActivity` の画面に 1 行と「設定を開く」のボタン（24 dp 以上）
- 載せる: `c01-location` の生存信号の原文（`Heartbeat.kt` の `fields`）に `"unused_app_restrictions": "<4 つのどれか>"`。`capturable` と `blockers` は変えない（取得できる状態の条項は ST09 が書き換えている。欄を足すと `/heartbeat` の形も変わる）
- ST09 も `MainActivity` の初回の流れに写真の権限を足す。下流は ST09 の gate 済みの SHA の `MainActivity` に継ぎ足す（`story_deps.py` は ST09 を requires に持たないので取り込まない。衝突は rebase で解く）

**反転条件**: 剥がれる仕組みが前提（`premises.md`。確かさ 2）と違うと分かったら（前景サービスで「使った」に数えるなど）、頼むのをやめて表示だけに。状態を画面のほかの場所（稼働状況）にも出したくなったら、生存信号の欄に上げる。
**物理**: 「3 か月開かなくても剥がれない」はエミュレータで早送りできない（`adb shell cmd app_hibernation` は休止を手で入れるだけ）ので、確認待ちに置く。

## D13（仮）. 形の確認の印は CLI のまま（Q5 推奨）

`tools/archive-shape.sh` のまま。2 か月ごとの手作業の回に、確認待ちが出ていれば置く。`docs/archive-inbox.md` の 2 か月ごとの手順に入れる。
**反転条件**: 確認待ちが 2 か月の回ごとに出るようになったら、画面の口を作る（別の change）。

## D14（仮）. 間隔は画面から変えない（Q6 推奨）

S-7 は読むだけ。変えるのは登録簿（psql）。
**反転条件**: 定期取得するソースが 2 本以上になり、本人が間隔を変えたくなったとき。画面の資格情報で登録簿を書く口と権限を足す。

## D15. 定期取得するソースはいま無い（Q1 推奨）

枠は本番で 0 本で動く。FR-15 に「いま定期取得するソースは無い。枠と登録簿の間隔だけを置く」と ★ を足すのは、答えを受けた record の段（要件の改訂は record が行う）。
Q1 で別の選択肢（ドライブ経由・Google 以外のサービス）が選ばれたら、第 2 回で実物を確かめてから取る手を足す。

## D16（仮）. 申し送りのうち直すのは R25・R34・M7・M8 だけ（Q2 推奨）

R32・M2〜M6 は実物か規模を見てから、R35 は全 Story の前提、M9・M10 は移行の細部。`docs/handoff/ST13.md` に残す（消さない）。
**反転条件**: 本人が Q2 で「移行の細部も」を選んだら M6・M9・M10 を足す。

## Risks / Trade-offs

- [ST09 と `device-collection` / `MainActivity` を同時に触る] → ST09 の MODIFIED の Requirement には触らず、別の ADDED の Requirement と生存信号の原文で済ませた。INDEX に割当を足したので、ST09 が走っている間は ST13 の下流が `衝突待ち` になりうる
- [D2 の trigger が ST09 の行に誤った既定を入れる] → ST09 の 3 本は「変わったとき」で合う。合わない行は足した Story が値を指定する
- [環境変数を断ると、古い起動の設定（サービスの定義）で S-01 が起動しない] → 断る文言に置き場（登録簿の列）を書く。`docs/archive-inbox.md` と `tools/smoke.sh` を同じ PR で直す
- [`GET /sources` の `max(event_time)` が重い] → D10 の Risks のとおり `EXPLAIN` で確かめる
- [Q1 / Q4 の前提は確かさ 2 で止まっている] → どちらも推奨の側は前提が外れても失わない（`premises.md` の末尾）

## Migration Plan

移行 1 本（列 3 つ・制約・trigger・既存の行を埋める `UPDATE`）。down は trigger・制約・列を落とす。
展開: S-01 を止める → 環境変数 `ASHIATO_ARCHIVE_SCAN_SEC` を外す（値を変えていたなら、起動後に登録簿へ）→ 起動（移行が当たる）。携帯の収集アプリは更新して 1 回開く（除外の案内が出る）。
