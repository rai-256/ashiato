---
status: plan
date: 2026-10-11
---

> 本人の決定（2026-10-11）: 「純粋関数で」を散文でなく機械で止める。**計算を I/O の依存を持たない crate に分ける**（検査スクリプト案は不採用）。
> 実施は走っている ST12・ST21 の merge 後に `fix/` 1 本（collector 側は ST08 の merge 後に 2 本目）。lint の残り（`Cargo.toml` / `web/eslint.config.js` の「まだ直していないだけ」の表を空にする）も同じ時に。
> 下は Plan agent が origin/main と走っているブランチを読んで立てた計画。**行番号は調べた時点のもの。着手時に §8 の手順で取り直す。** 「0. 先に決めること」は提案で、本人の決定ではない。

# 計画: 計算を I/O の無い crate へ分ける（`fix/` 1 本。ST12・ST21 の merge 後）

調べた版: origin/main `4f170499`（＝ ashiato2-ratchet `fecc1327` から lint の commit を除いたもの。crates/web/src に差は無い）、ST12 `f7d6ed0b`、ST21 `8fe9c702`、ST08 `508b63e3`。行番号はすべてこの時点のもの。lib.rs は ST12 で +414 行・ST21 で +269 行になるので、**着手するときに merge 後の main で表を取り直すこと**（手順は §8）。

---

## 0. 先に決めること（提案）

| 論点 | 提案 | 理由 |
|---|---|---|
| crate の名前 | `crates/pure`（package `ashiato-pure`）。`core` は避ける | この repo では `core` は PostgreSQL のスキーマの名前（`core.source` など。coverage.rs:327 の SQL ほか多数）。Rust 組み込みの `core` crate とも紛らわしい。名前を見れば規則が分かる |
| tracing | **純粋とみなさない。core では禁止** | ログは出力の経路で、一度出たら消せない（tools/check-log-private.sh:7-8）。core の中の warn は「値として返し、server がログに出す」形に変える。そうすると警告が出ることを純粋なテストで確かめられるようになる（今はログを捕まえないと確かめられない） |
| utoipa（`ToSchema` の derive） | **許可する**（`default-features=false, features=["macros","chrono","uuid"]`） | 実行時の I/O が無い。`axum_extras` は utoipa-gen（proc-macro）にしか効かない（~/.cargo/registry/.../utoipa-5.5.0/Cargo.toml の features）。移す型に `#[schema(...)]` は 1 つも無い（attributes/heartbeat/ingest/drops/coverage を grep して 0 件）。代わりに `cfg_attr(feature="openapi")` にする案もあるが、derive を書き換える量が増える割に得るものが無い |
| anyhow | 許可する | エラーの値でしかない。ST12 の純粋な関数が `anyhow::Result` を返す（archive/slice.rs:13,80 など） |
| 乱数（`Uuid::new_v4`） | 現在時刻と同じ扱いで禁止。引数で渡す | OS の entropy を読む。ST12 の worker.rs:903 が要求の id を `new_v4()` で作っている |
| collector-windows | **同じ crate に入れない**。ST08 の merge 後に別 crate・別 fix にする | tools/check-boundaries.sh:12-15 で「収集側と server の契約は docs/collector-contract.md と HTTP だけ」と決めてある。今は両者で共有している計算も無い（§3） |

---

## 1. `crates/server/src` の分類（origin/main）

判定の基準: sqlx / tokio / axum / std::fs / std::env / std::net / std::process / `Utc::now`・`Instant::now`・`SystemTime::now` / `Uuid::new_v4` / tracing のどれかを使えば「I/O」とした。

| モジュール | 行数 | 分類 | 根拠 |
|---|---|---|---|
| stay.rs | 505 | **純粋** | use は chrono だけ（:6）。`detect` :124、`clusters` :85、`distance_m` :71、`raw` :189 |
| attributes.rs | 1259 | **純粋** | chrono / serde / unicode-normalization だけ（:11-13）。`parse_claim` :147。`view` :379 は `today` を引数で受ける |
| ingest.rs | 519 | **純粋** | serde / unicode だけ（:4-5）。`validate` :130、`content_hash` :170、`to_nfc` :200 |
| heartbeat.rs | 155 | **純粋** | serde だけ（:7）。`validate` :55、`content_hash` :73 |
| stay_store.rs | 1238 | 混在 | sqlx :8。純粋な部分は §2 |
| attributes_store.rs | 503 | 混在 | sqlx :9。tracing :270,:325,:346 |
| coverage.rs | 1181 | 混在 | sqlx :240-1080（全部 async）、tracing :285,:400-460,:521 |
| drops.rs | 570 | 混在 | axum :10、sqlx :252-326、tracing :242,:248 |
| lib.rs | 2457 | 混在（ほぼ I/O） | env :244,:2281-2332、`Utc::now` :249,:334,:1736,:2057、`Instant::now` :1380,:1621、tokio :337,:457 |
| web_session.rs | 492 | 混在 | axum :9、sqlx :199、`Instant::now` :121,:130、tokio sleep :333 |
| net_guard.rs | 158 | 混在 | `tokio::net::lookup_host` :49。純粋なのは `bind_allowed` :34 と `db_host_is_local` :57 |
| migrate_backup.rs | 300 | 混在 | env :27-32、fs :133-162。純粋なのは `dump_target` :49 と `percent_decode` :67 |
| access_log.rs | 145 | I/O | axum middleware :110、sqlx :62（`via_of` :19 は HeaderMap を受ける） |
| deletion.rs | 420 | I/O | sqlx :5、関数が全部 async |
| main.rs / bin/openapi.rs | 11 / – | I/O（入口） | tokio::main :4、env :6 |
| testdb.rs と `*_tests.rs` | – | テスト用の I/O | §5 |

既に分かれている対: stay.rs / stay_store.rs、attributes.rs / attributes_store.rs、ingest.rs・heartbeat.rs / lib.rs の `ingest_one` :711・`heartbeat_one` :1844。

ついでに見つけた誤り: docs/testing.md:171 は coverage.rs を「純粋関数」と書いているが、実際は sqlx と混ざっている。この fix で記述を直す（cargo-mutants の対象は `-p ashiato-pure` になる）。

## 2. 混在モジュールから切り出せる関数と、そのための変更

**coverage.rs**（純粋に切り出せるのは約 600 / 1181 行）
- そのまま移せるもの: 定数 :14-30、`Subject` :35、`must_sources` :43、`DayState`・`band` :62-91、`Band` :96、`DayCell` / `DroppedRange` / `Interval` / `SourceCoverage` / `SourceRow` :104-180、`DayFacts` :194（private なので pub にする）、`RawSource` :298、`CHAIN_MAX_DEPTH` :314、`Chain` :487、`decide` :940、`SourceAchievement` / `Achievement` :1033-1078。
- `resolve_chain` :373 は tracing::warn! を 6 か所で呼んでいる（:400, :411, :433, :443, :452, :460）。変更: `(Chain, Vec<ChainNote>)` を返し、server の `sources` :504 がログに出す。`kind` の名前は今のまま残すので、check-log-private の ALLOWED は変わらない。
- `achievement` :1080: DB を読む `facts` :1113 が数える処理の途中に挟まっている。変更: (a) 純粋な `counting_window(started_on, today) -> Option<(from, last)>`（:1103-1111 の計算）を作る、(b) server はソースごとに `facts` を読む、(c) 残り :1114-1176 を純粋な `count_achievement(today, targets, rows, facts_per_row) -> Achievement` にする。`today` は既に引数で受けている（:1074-1076 の doc）。
- `dropped_by_day` の後半 :803-833（時間ごとの件数を区間に当てる部分）→ 純粋な `assign_hours(sections, hours)` にする。`of_source` の DayCell の組み立て :999-1030 も純粋な関数にできる。
- server に残すもの: `drop_islands_cte` :575 と各 SQL。日境界は SQL の `AT TIME ZONE` で切る、と design D1 で決めてある。

**stay_store.rs**（約 280 行）
- `day_bounds` :27、`jst_date` :33 → `pure::day` へ。
- `Mark` と `Mark::of` :246-268、`Existing` :271、`span_of` :295、`overlap` :462、`Assignment` :468、`assign` :483、`touches_erased` :541、`ErasedSpan` :936、`subtract_spans` :942 → `pure::stay_assign` へ。`pub(crate)` は `pub` に広げる。
- `failure_kind` :430 と `sqlx_variant` :446 は sqlx の型を見るので server に残す。

**attributes_store.rs**（約 120 行）
- `initial_kind_id` :66（`Uuid::new_v5` なので v5 の feature が要る）、`claim_from_payload` :392 は移す。
- `stored_claim_of` :336 は `#[derive(sqlx::FromRow)] ClaimRow` :286 を受けているので、FromRow の無い同じ形の struct を core に置き、server で詰め替える。tracing :346 は返り値にする。

**drops.rs**（約 390 行。テストを含む）
- `DropHour` :29、`DropReportRequest` :37、`Invalid` :66、`validate` :85、`validate_hourly` :125、`content_hash` :161、`RANGELESS_REASONS` :25、inline のテスト :389-570 は移す。
- `drop_one` :233 と `drops_post` :356 は残す。

**lib.rs**（移せるのは約 60-120 行。残りは I/O）
- `default_sensitivity` :591（`DEFAULT_SENSITIVITY` :582 も一緒に）、`place_identifiers` :690（`IngestRequest` を受けるので ingest の隣に置く）、`Update` と `was_skipped` / `reason` :1130-1166、`criteria_out_of_range` :1581。最後のものは `RebuildRequest` という DTO を受けているので、`Option<i32>` を 3 つ受ける形に変える。
- `today_jst` :2048 は server に残して `pure::day::jst_date(Utc::now())` にする。`App::now` :329 と `Utc::now` の他の呼び出し（:249, :1736）も server に残す。ここが時刻を引く「端」になる。
- `ingest_one` :711-1096 と `heartbeat_one` :1844 は、DB の読みと判断が交互に並んでいる。切り出すには作りを変える必要があるので **この fix の対象外**にする。

**任意（計算ではなく検査なので、今回は動かさなくてよい）**: web_session の `check_web_password` :77、`secret_tag` :91（hmac）、`recent` / `throttled_at` :109/:124（`Instant` を引数で受けている）、`session_max_age_days` :141。net_guard の `bind_allowed` :34、`db_host_is_local` :57。migrate_backup の `dump_target` :49。lib の `token_matches` :417（sha2 + subtle）。

**重複の整理**: JST の日付を引く式が 4 か所にある。stay_store.rs:33、lib.rs:2048、ST21 places.rs:466 `jst_midnight`、places.rs:1089 `jst_day`。これを `pure::day` の 1 か所にまとめる。lib.rs:2054-2056 の doc が「式は 1 本だけ」と言っているが、ST21 がそれを破っている。

## 3. collector-windows

| 分類 | モジュール（行数）と根拠 |
|---|---|
| 純粋 | engine.rs（1166。use は chrono / contract / exclusion だけ :17-20）、contract.rs（584。serde だけ :10）、clock_record.rs（293）、telemetry.rs（74）、winrules.rs（96。`Path::file_name` を使うが fs は触らない :20） |
| 混在 | exclusion.rs（`hits` :48,:94 は純粋。`load` :78-79 が fs）、heartbeat.rs（`Capability` / `Counters` / `Schedule` / `signal` :31-236 は純粋。`CounterStore` :119-160 が fs）、clock.rs（`SkewSchedule` :76-170、`parse_http_date` :174、`host_of` :179 は純粋。`HttpDateClock` :188 が ureq、`Utc::now` :225）、time_sync.rs（`raw_text` :236、`value_of` :249、parse_* は純粋。`Command` :14、env :139 が I/O）、config.rs（`from_lookup` :104 は純粋。`Local::now` :27、env :95）、browsers.rs（`with_extra` は純粋。env :42）、marker.rs（fs :38-83）、runtime.rs（`Source` / `Transport` は注入済みだが、`Utc::now` / `Instant::now` :348-349 と fs :395 を持つ） |
| I/O | platform.rs（uiautomation :27-29、cfg(windows)）、outbox.rs、sender.rs（ureq :187）、fsutil.rs、autostart.rs、clock_worker.rs（thread :71）、main.rs |

**server と共有する計算は無い**。収集側は content_hash を計算せず、server だけが持つ（ingest.rs:170、heartbeat.rs:73）。あるのは**通信の型の写し**だけ: collector の contract.rs:301 `IngestRequest` / :353 `HeartbeatRequest` と、server の ingest.rs:15 / heartbeat.rs:11。これを Rust の型として共有すると check-boundaries.sh:12-15 の境界（契約は文書と HTTP だけ）を崩す。Android の収集は Kotlin なので、契約は言語に依らない形のままにしておく。**共有しない**。

提案: 収集側には別の crate（例 `crates/pure-c02`）を作り、engine / contract / clock_record / telemetry / winrules と、混在モジュールの純粋な部分（約 4,100 行。ST08 の history/contract.rs 580 行・fetch.rs 598 行を含む）を移す。**ST08 が collector の runtime.rs（+1562）・exclusion・contract・heartbeat を触っているので、ST08 の merge 後に 2 本目の fix で行う。** clock_record は `clock_worker::ClockReading` と `time_sync::TimeSyncReading` / `raw_text` に依存しているので（clock_record.rs:8-13）、その型も一緒に移す。check-boundaries.sh には「collector-windows は ashiato-pure（server 側の計算）に依存しない」を足す。

## 4. 走っているブランチが足すもの

**ST12（archive/）**
| ファイル | 分類 | 根拠 / 必要な変更 |
|---|---|---|
| chrome.rs 9、legacy.rs 73、myactivity.rs 22、timeline.rs 50、timezone.rs 57、youtube.rs 44、slice.rs 270 | 純粋 | bytes / &str を受けて値を返す。slice の `stream_array_items<R: Read>` :13 は reader を受けるだけなので許す |
| mod.rs 80 | 純粋 | 定数だけ（:18-48） |
| classify.rs 197 | 純粋 | ただし `open::ArchiveFile`（open.rs:9。データだけの struct）を core に移す必要がある |
| open.rs 93 | I/O | fs :48,:65、zip :69 |
| config.rs 91 | 混在 | env :90。`from_values` は純粋 |
| scan.rs 209 | I/O | sqlx :65-140、`Utc::now` :101、fs :152,:206 |
| worker.rs 2299 | 混在 | 純粋に切り出せるのは約 900 行: `requires_shape_confirmation` :7、`file_name_of` :83、`unreadable_summary` :344、`archive_created_at` :357（既に `discovered_at` を受けている）、`FileRequests` :381、`requests_for_file(_reporting)` :392,:855、項目を読む関数群 :408-850、`build_request` :881、`shape_for_file` :1133、`path_shape` :1201、`hash_shape` :1402。**変更: `build_request` の `Uuid::new_v4()` :903 を id を作る関数の引数（`&mut impl FnMut() -> Uuid`）にする**。id を内容から決める形には変えない（重複の判定が変わってしまう）。:1344 の `new_v4` は `store_heartbeat` に渡す部分（I/O 側）なので残す |
| archive_tests.rs（sync 32 / async 29）、archive_flow_tests.rs（6 / 63） | – | sync のうち fs を触らないものを core へ |

**ST21**
- places.rs 1283 行のうち約 950 行が純粋: `parse_place_record` :118、`parse_coord` :197、`coord_marker` :310、型 :38-100,:367-406,:433-460,:611-722、`versions` :504、`coord_windows` :594、`view` / `view_with_stays` / `place_out` :729-960、`StayPoint` / `MatchPlace` / `assign` :964-1012、`summarize` :1018、`candidates` :1098、`candidates_view` :1158。I/O は :263-430 と :1214-1283（sqlx、tracing :1236）。
- 依存: `attributes::ValidFrom::from_json` / `is_well_formed`。ST21 がこれを `pub(crate)` で足しているので `pub` にする。`jst_midnight` / `jst_day` は `pure::day` へまとめる。places_tests.rs の sync 29 本も対象。

**ST08**（collector だけ）: history/contract.rs 580 と fetch.rs 598 は純粋（`detect_vanished` は now を受ける）。ledger.rs と locate.rs は混在（fs :73 / :66,:156）、read.rs は I/O（rusqlite :121）、collect.rs は混在。§3 の 2 本目の fix で扱う。

## 5. 移す順番・Cargo.toml・テストの置き場

**順番**（葉から。1 段 = 1 commit で、毎回テストを緑にする）
0. 足場: `crates/pure/{Cargo.toml, clippy.toml, src/lib.rs}`、workspace の members に追加、`tools/check-pure.sh`、CI。空の lib に違反をわざと植えて、落ちることを確かめる。
1. `day`（jst_date / day_bounds / DAY_TZ）
2. `ingest`（全部）
3. `heartbeat`（全部）
4. `stay`（全部）
5. `attributes`（全部。ST21 の差分を含む）
6. `drops` の検証部分
7. `stay_assign`（4 に依存）
8. attributes_store の純粋な部分（5 に依存）
9. coverage の純粋な部分
10. lib.rs の小さい関数
11. places（1・5 に依存）
12. archive（2 に依存）

**変更を小さくする工夫**: server の lib.rs で `pub use ashiato_pure::{ingest, heartbeat, stay, attributes};` と再 export すれば、`crate::stay::…` という呼び出しはそのまま動く。worker が使う `crate::IngestRequest` も同じ。

**`crates/pure/Cargo.toml` で許す依存（直接の依存の許可リスト）**
```toml
[lints]
workspace = true
[dependencies]
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
sha2.workspace = true
unicode-normalization.workspace = true
chrono = { version = "0.4", default-features = false, features = ["std", "serde"] }   # clock/now を外す
uuid   = { version = "1", default-features = false, features = ["std", "v5", "serde"] } # v4（乱数）を外す
utoipa = { version = "5.5.0", default-features = false, features = ["macros", "chrono", "uuid"] }
# subtle は token_matches を移すときだけ
[features]
fixture = []   # stay::fixture を server のテストから使うため
```
- 時刻の止め方: chrono 0.4.45 では `Utc::now` が `feature = "now"` の裏にある（chrono-0.4.45/src/offset/utc.rs:22。`clock = [..., "now"]`）。now を入れなければ **core を単独でビルドしたときに `Utc::now` が存在しない**。
- 落とし穴: workspace の chrono :18 と uuid :17 は default features を有効にしたうえで v4 を持っている。`workspace = true` で継承すると、member 側では default を外せない。workspace の定義を `default-features=false, features=["std","serde"]` に変え、server と collector の側で `features=["clock"]` / `["v4","v5"]` を足す。
- もう 1 つの落とし穴: `cargo clippy --workspace` は feature を統一するので、server が有効にした clock が core にも効き、`Utc::now` がコンパイルできてしまう。そのため 2 段にする。(a) CI で `cargo clippy -p ashiato-pure --all-targets -- -D warnings` を**単独で**走らせる、(b) clippy の `disallowed_methods` を workspace でのビルドでも効かせる（§6）。

**テストの置き場**
- inline の `#[cfg(test)]` は module と一緒に移る: stay 17、attributes 22、ingest 19、heartbeat 4、drops 12。
- 別ファイルから core へ移すもの: coverage/tests/states.rs:677 `decide_follows_the_order_of_d7`、achievement.rs:520 `must_sources_are_the_five_of_nfr13`、:544 `seven_states_fold_into_three_bands`、api_tests.rs:771 `heartbeat_hash_is_pinned`、ST12・ST21 の sync テストのうち純粋なもの。
- `#[tokio::test]` は全部 server に残す（DB を使う）。
- `stay::fixture`（stay.rs:217。`#[cfg(test)] pub(crate)`）は stay_tests.rs:273 から使われている。crate をまたぐと cfg(test) は見えないので、`#[cfg(any(test, feature="fixture"))] pub mod fixture` にし、server の dev-dependencies で `features=["fixture"]` を有効にする。
- core のテストにある `Uuid::new_v4()`（ingest.rs:243、heartbeat.rs:94,149、drops.rs:402,565、attributes.rs:477 ほか）は `Uuid::from_u128(n)` に変える。disallowed_methods はテストにも効くうえ、決まった値の方がテストとしても良い。
- 移す前と後で `cargo test --workspace -- --list` の本数が同じであることを確かめる（ci.yml の windows runtime job と同じ考え方）。

## 6. 機械で止める検査

Cargo.toml に依存が無いだけでは止まらない。**std は常に使える**（std::fs、std::env、std::process、std::net、std::thread、`SystemTime::now`、`Instant::now`）。そこで 3 層にする。

1. **clippy.toml は crate ごとに置ける**。clippy は `CARGO_MANIFEST_DIR` から親のディレクトリへ向かって最初に見つけた clippy.toml を使う。今は root に clippy.toml が無いので、`crates/pure/clippy.toml` は pure にだけ効く（実装のときに違反を植えて確かめる）。
   - `disallowed-methods`: `chrono::Utc::now`, `chrono::Local::now`, `std::time::SystemTime::now`, `std::time::Instant::now`, `uuid::Uuid::new_v4`, `std::fs::*` の主な関数（read, read_to_string, write, create_dir_all, remove_file, rename, read_dir, metadata）, `std::env::{var, var_os, vars, args, current_dir, temp_dir}`, `std::process::exit`, `std::thread::{spawn, sleep}`, `std::path::Path::{exists, is_file, is_dir, metadata, canonicalize, read_dir, try_exists}`
   - `disallowed-types`: `std::fs::File`, `std::process::Command`, `std::net::{TcpStream, TcpListener, UdpSocket}`
   - `disallowed-macros`: `tracing::{trace, debug, info, warn, error}`, `std::println`, `std::eprintln`
   - `crates/pure/src/lib.rs` に `#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]` を書く。**forbid なので、中で `#[allow]` しても通らない**。workspace の `[lints] workspace = true` は他の lint の指定と同じ表に並べられないので、crate の属性で書く。
2. **`tools/check-pure.sh`（`--self-test` 付き。check-no-time-server.sh と同じ形）**
   - (a) `cargo metadata` で取った pure の直接の依存が許可リストに収まっている。
   - (b) `cargo tree -p ashiato-pure -e normal,build,dev --prefix none --format '{p}'` で出る依存の全体に、禁止リストのものが 1 つも無い: sqlx*, tokio*, axum*, hyper*, reqwest, ureq, mio, socket2, rusqlite, zip, tracing*, getrandom, rand*, **iana-time-zone**（chrono の clock が入ったことの印になる）, winapi。
   - (c) clippy.toml と forbid の行がある。
   - (d) build.rs が無い。
   - (e) 念のための grep（`std::(fs|env|process|net|thread)`、`Utc::now`、`new_v4`）。
   - CI の rust job（ci.yml:73-85）と tools/pre-commit.sh に足す。
3. **cargo-deny は勧めない**。repo に deny.toml が無いうえ、bans は依存のグラフ全体に掛かる規則で、「pure の下にだけ禁止」とは書けない。`wrappers` は直接の親しか縛らない。tokio の正当な親（sqlx-core、hyper、axum、tower…）を全部書く必要があり、壊れやすい。既存の流儀（tools/check-*.sh を CI の 1 手にする）に合わせる。

このほか: check-log-private.sh は `crates/*/src` を見るので pure にも自動で効く。check-openapi.sh:6-8 で **docs/openapi.json の差が 0** であることが、型を移しても API が変わらなかった証拠になる。

## 7. 見積もり

**Rust（server。ST12・ST21 の merge 後）**
- 移す行（ほぼそのままの移動）:
  - 丸ごと移す 4 つ: 約 2,470 行
  - coverage 約 600、stay_store 約 280、attributes_store 約 120、drops 約 390、lib 約 100
  - places 約 950
  - archive の小さいファイル約 800 と worker 約 900
  - 別ファイルにある sync テスト約 1,200（推定）
  - **合計 約 7,800 行**
- 書き換える行: 約 300-500 行（可視性の変更、再 export、`resolve_chain` の警告を返り値に、`achievement` の分割、`ClaimRow` の詰め替え、id の注入、fixture の feature、テストの uuid）。
- 触るファイル: 約 50。
  - 新規: pure の約 22 ファイル（Cargo.toml、clippy.toml、lib、day、ingest、heartbeat、stay、stay_assign、attributes、drops、coverage、places、archive/*）
  - server: lib.rs、coverage.rs、stay_store.rs、attributes_store.rs、drops.rs、places.rs、archive/{mod, open, worker, classify ほか}、deletion.rs（import）、coverage/tests/{states, achievement}.rs、api_tests.rs、stay_tests.rs、archive_tests.rs、places_tests.rs、Cargo.toml
  - その他: root の Cargo.toml / Cargo.lock、ci.yml、tools/check-pure.sh、tools/pre-commit.sh、tools/check-boundaries.sh、CLAUDE.md:17 の表、AGENTS.md、docs/testing.md:171
- CI の負担: crates/ を変えると Windows の job も起動する（ci.yml:51。無料枠を 2 倍で消費する）。

**Rust（collector。2 本目の fix）**: 約 4,100 行、約 20 ファイル。

**Web（同じ fix に含める）**
- 計算のファイル: coverage.ts、stays.ts、attributes.ts、tokens.ts、contrast.ts、messages.ts、ST12 の archives.ts（`now: Date` を引数で受ける :76,:90）、ST21 の places.ts。
- 今ある不純な部分:
  - attributes.ts の `newNonce` :118（`crypto.getRandomValues` :120）、`readIngestResponse` :227 と `readKindResponse` :266（async で `Response.json()` を読む）
  - ST21 places.ts の `fetch` :353,:363、`crypto.randomUUID` :404,:409
  - stays.ts と coverage.ts の `new Date(...)` は引数付きなので純粋
- 変更:
  - attributes.ts → `attributesApi.ts` を分ける（純粋な `outcomeOf(status, body)` を残し、async の薄い wrapper を Api 側に置く）。
  - places.ts → `placesApi.ts` を分け、`buildRegistration` は id を作る関数を引数で受ける。
  - 呼び出し側を直す: MasterView.tsx:11-13,153,221,432,441、__tests__/attributes.test.ts:14-15、ST21 の PlaceForms など。
- eslint（eslint.config.js）:
  - `files: ["src/**/*.ts"]`（画面は .tsx なので、**.ts を既定で対象にする**）、`ignores: ["src/**/*Api.ts", "src/**/*.d.ts", "src/__tests__/**"]`。
  - `no-restricted-globals`: fetch, window, document, localStorage, sessionStorage, navigator, location, crypto, performance, setTimeout, setInterval, XMLHttpRequest, WebSocket
  - `no-restricted-properties`: Date.now, Math.random, crypto.randomUUID
  - `no-restricted-imports`: react, react-dom, `./*Api`, `./session`
  - `no-restricted-syntax`: `NewExpression[callee.name='Date'][arguments.length=0]`
  - **落とし穴**: flat config では、後のブロックが同じ規則を設定すると前の設定は上書きされ、合わさらない。既存の `no-restricted-syntax`（画面の文言の禁止 :52-68）が計算のファイルで消えてしまう。selector を定数の配列に出し、両方のブロックで展開する。messages.ts は文言の禁止の対象外（:50）なので、Date の selector だけを掛ける別ブロックにする。
- 見積もり: 約 10 ファイル、約 200 行。

## 8. 着手するときの手順と危険

- merge 後の main で表を取り直す（`grep -nE 'sqlx|tokio|std::fs|std::env|Utc::now|Instant::now|new_v4|tracing::' crates/server/src/**/*.rs`、`#[test]` と `#[tokio::test]` の本数）。ST12・ST21 の後に別の Story が lib.rs を触れば、行番号はまたずれる。
- `pub(crate)` を `pub` に広げる（stay_store の `assign` / `span_of` / `Existing`、attributes の `from_json` など）。pure は workspace の中でしか使わないので害は小さい。
- 各段で確かめること:
  - `cargo test --workspace`
  - `cargo clippy -p ashiato-pure --all-targets -- -D warnings`（単独）
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `./tools/check-openapi.sh`（差が 0）
  - `./tools/check-pure.sh --self-test`
  - pure に `chrono::Utc::now()` を植える → 単独ビルドでコンパイルエラー、workspace の clippy で disallowed が出ることを 1 度だけ確かめる
- ST08 が走ったままなら、collector の側は 2 本目に回す（同じファイルを触るので衝突する）。
