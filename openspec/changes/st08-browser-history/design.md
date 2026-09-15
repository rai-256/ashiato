# ST08 設計 — PC のブラウザ履歴を集める

## Context

`crates/collector-windows` は ST07 でウィンドウの収集（`c02-window`）を持ち、未送信の保持・送信・生存信号・
除外・停止の印がすべて揃っている。**ST08 はその上に 2 本目の論理ソース `c02-browser-history` を載せる。**
登録簿には移行 `202609111111_coverage_rebuild.sql` で既に行があり（`expected_gap_sec = 86400`）、
`external_id_kind` は `'none'`（ST07 design D2 の実測）。

深掘りで本人が決めた 3 件は `deep.md`。**ここで決めるのは、その決定を実装に落とすときの技術判断だけ**で、
観測可能な振る舞いは `specs/desktop-collection/spec.md` に置いてある（`design.md` に置くと
`openspec archive` で正典から落ちるため）。

事実（Chromium のソース。2026-09-15 に `chromium.googlesource.com` の main で確認。レビューが再現したものを含む）:

| 事実 | 出所 |
|---|---|
| 90 日を過ぎた訪問は手元の DB から削除される | `history_backend.h` `kExpireDaysThreshold = 90` |
| `visits.id` は `INTEGER PRIMARY KEY AUTOINCREMENT`。ただし全期間の削除で表が DROP され、番号が 1 から振り直される | `visit_database.cc` / `history_backend.cc` `DeleteAllHistory` → `RecreateAllTablesButURL`（review/deep.md R2） |
| ページの題名は `urls.title`（URL ごとに最新の 1 つ）、滞在時間は `visits.visit_duration`（タブを離れた後に書き込まれる） | `url_database.cc` / `history_backend.cc` `UpdateVisitDuration`（R1） |
| 同期で入った他端末の訪問は `originator_cache_guid` / `originator_visit_id` を持つ。同期を切ると一括で消える | `AddSyncedVisit` / `DeleteAllForeignVisitsAndResetIsKnownToSync`（R3） |
| 動いている間、履歴 DB は `locking_mode=EXCLUSIVE` で開かれる | `history_database.cc`（R8） |

## Goals / Non-Goals

**Goals**
- 見つかったブラウザの全プロファイルの訪問を、訪問 1 件ごとに、読み直しても増えない形で残す
- 「取っていないと後から作れない」もの（90 日で消える訪問・消えた事実・書き換わる前の題名）を day one で残す
- ST07 のウィンドウの記録の形を 1 文字も変えない

**Non-Goals**
- **ブラウザで消した訪問を ashiato でも消したことにする**（FR-50 / ST22。`docs/handoff/ST22.md`）
- **同期している複数の PC から届いた同じ訪問を 1 件に畳んで読む形**（読む側の Story。材料は D9 で残す）
- **NFR-13 の C-02 の分母の数え方**（`collection-coverage`。`docs/handoff/ST02.md` の st07-active-window R10）
- **未送信の上限**（ST04）
- ダウンロード履歴・検索語の表（deep.md C-7）

## Decisions

### D1. 読み手は 2 種類 —— Chromium 系と Firefox。置き場は既知の場所を探す

| ブラウザ | 置き場（`%LOCALAPPDATA%` / `%APPDATA%`） | プロファイル | 表 |
|---|---|---|---|
| Chrome | `Google\Chrome\User Data\<dir>\History` | `Default` / `Profile N` / … | `visits` + `urls` |
| Edge | `Microsoft\Edge\User Data\<dir>\History` | 同上 | 同上 |
| Brave | `BraveSoftware\Brave-Browser\User Data\<dir>\History` | 同上 | 同上 |
| Vivaldi | `Vivaldi\User Data\<dir>\History` | 同上 | 同上 |
| Opera | `%APPDATA%\Opera Software\Opera Stable\History`（と `…\_side_profiles\<dir>\History`） | 無いときは `Default` と読む | 同上 |
| Firefox | `%APPDATA%\Mozilla\Firefox\Profiles\<dir>\places.sqlite` **と** `profiles.ini` の `Path=`（相対 / 絶対）の和 | `<dir>` | `moz_historyvisits` + `moz_places` |

**`User Data` の直下を 1 段だけ見て、`History` を持つディレクトリをプロファイルとする**（`Local State` の
`profile.info_cache` に頼らない —— 表示名の一覧に無いディレクトリも取りこぼさない。扉を開けたままにする側）。
`Guest Profile` / `System Profile` も `History` があれば読む。
**Firefox は `profiles.ini` の一覧と `Profiles\` の走査の和を取る**（プロファイルは既定の外に置ける。review/spec.md R10）。

**表示名との対応**（spec「プロファイルの表示名との対応が残る」）: Chromium 系は `User Data\Local State` の
`profile.info_cache.<dir>.name`、Firefox は `profiles.ini` の `Name=`。ブラウザ 1 つにつき、**初回と、対応が帳面に残した前回のものから変わったときだけ** `profiles` 記録を 1 件
（D4。変わらない対応を毎日送ると、出来事の時刻が違うので毎日版が積む）。読めなければ対応の無いディレクトリは表示名を省く（ディレクトリだけは載る）。

置き場の組み立て（どのディレクトリを見るか）は **Linux で確かめられる純粋な関数**にする（`history/locate.rs`）。
反転条件: 実機で上の表に無い置き場（Opera GX・Chrome のベータ版など）を使っていると分かったら、表に 1 行足す（記録の形は変えない）。

### D2. 履歴 DB は**写しを取って**読む。読み手は `rusqlite`（`bundled`）

動いているブラウザは履歴 DB を排他で開く。**`History` と、あれば `History-journal` / `History-wal`
（Firefox は `places.sqlite` と `places.sqlite-wal`）を置き場の一時ディレクトリへ写し、写しを読み取り専用で開く**。
SQLite の排他ロックは 1 GB の位置のバイト範囲にしか掛からないので、それより小さい DB の写しは読める。
写しは読み終えたら消す（**私的な内容を置き場に残さない**）。

`rusqlite` は MIT、`bundled` の SQLite は public domain（`tools/check-licenses.sh` の許可に入る）。
**`cfg(windows)` の外に置く** —— 読み取りと見分けを Linux の単体で、本物の SQLite の DB を作って確かめるため。
`bundled` は C をコンパイルするので、ubuntu の windows 向け型検査（ST07 D20）に mingw の C コンパイラが要る（tasks 1.2）。

**反転条件**: 実機でブラウザが動いている間の写しが失敗する（共有違反）と分かったら、ボリュームシャドウコピー、
またはブラウザが止まっている時刻まで取得を遅らせる（取得の形だけで、記録の形は変えない）。

### D3.（仮）取得の契機と「前回以降」 —— 仮なのは試し直しの間隔だけ

- **契機**: 起動時に**前回の取得の成功**から 24 時間以上経っていれば取得し、動作中は前回の成功から 24 時間ごと。
  「前回の成功」は置き場に書く（`browser-history/last_success.json`。**単調時計ではなく壁時計**で持つ ——
  起動をまたぐ値なので）。**失敗した取得は成功を進めない**ので、次の見回りで 1 分後に試し直す
- **前回以降**: **毎回、DB にある訪問を全部読み**、帳面（D10）と比べて**まだ送っていない訪問と、送った後で内容が変わった訪問だけ**を積む。
  訪問時刻で「前回より新しい」を切らない —— 同期の訪問は発生元の古い時刻で後から入る（review R10）
- **成功とみなすのは、未送信に積み終えて帳面を書いた後**（spec の保持の Requirement）。積む前に落ちたら、次の取得が同じ訪問を再び積む（D6 で畳まれる）
- **読みは見回りとは別のスレッド**で行い、積むのは見回りの側（ST07 の `Runtime::push`）。Firefox が何年分も持っていると
  最初の読みは数十秒かかりうるので、見回りを止めると眠りの判定（ST07 D19。2 分）に化ける

- **見回りを止めない**ことを固定する（review/spec.md R15）。読みが長くても `c02-window` に `suspended` が入らない

**仮なのは失敗した取得を試し直す間隔（1 分）だけ**。24 時間は FR-13 の値で、仮ではない（R14）。
**反転条件**: 1 分ごとの試し直しがブラウザの描画を重くすると分かったら延ばす（記録の形は変えない）。
1 回の読みが実測で数分を超える / 未送信が大きくなりすぎると分かっても、**「古い訪問を読まない」には倒さない** ——
spec の「過去の履歴をすべて取り込む」「古い時刻の訪問も取り込む」を破るので、**本人に返す**（R14）。

### D4. 1 件に載せる項目と、原文の直列化

**ST07 D1 と同じく、`raw` は収集側が組んだ JSON を文字列のまま送り、`payload` は同じ中身から作る。**
型は `VisitPayload`（`history/contract.rs`）。`WindowPayload` とは別の型にする（ST07 の形を固定したテストに触れないため。D13）。

| 項目 | 種類 | 中身 |
|---|---|---|
| `kind` | 全部 | `visit` / `vanished` / `excluded` / `profiles` |
| `at` | 全部 | 出来事の時刻（D5）。`event_time` と同じ値 |
| `browser` / `family` | 全部 | `chrome` / `edge` / `brave` / `vivaldi` / `opera` / `firefox`、`chromium` / `firefox` |
| `profile_dir` | 全部 | ディレクトリ名（`Default` / `Profile 1`）。**表示名は載せない**（下の注） |
| `visit_id` / `visit_time_raw` | `visit` | DB の値そのもの（Chromium は 1601 年起点のマイクロ秒、Firefox は 1970 年起点のマイクロ秒） |
| `url` / `title` | `visit` | DB の文字列そのまま。補正しない。題名が NULL なら欄を省く |
| `transition` / `transition_core` | `visit` | 整数そのままと、下位 8 ビットの名前（`link` / `typed` / `reload` …）。Firefox は `visit_type` |
| `from_visit` / `opener_visit` | `visit` | DB の値（0 は欄を省く） |
| `visit_duration_us` | `visit` | Chromium のみ |
| `originator_cache_guid` / `originator_visit_id` | `visit` | 他端末の訪問だけ（D9） |
| `is_known_to_sync` | `visit` | Chromium のみ |
| `tz_basis` | 全部 | `collected-at`（D5） |
| `vanished` | `vanished` | 消えた訪問の識別子と手がかりの一覧（D10） |
| `excluded_count` | `excluded` | 除外した訪問の数（D11） |
| `profiles` | `profiles` | ディレクトリ名 → 表示名の対応（D1。表示名が読めないディレクトリは名前を省く） |

> **プロファイルの表示名を訪問ごとに載せないのは、改名 1 回でそのプロファイルの 90 日ぶんの全訪問が「内容が変わった」になり、
> 版の行が一斉に積むため**（D6）。ディレクトリ名は改名で変わらない。**表示名は対応が変わったときに 1 件の `profiles` 記録で持つ** ——
> Q1 の選択肢で「記録にプロファイル名を載せる」と本人に見せており、改名やプロファイルの削除の後では対応が取れなくなる（review/spec.md R3）。

直列化の形は `visit_payload_shape_is_pinned` で 1 文字単位に固定する（**形が変わると同じ訪問が「内容が変わった」として版を積む**）。

### D5. 出来事の時刻はマイクロ秒、タイムゾーンは取得時の PC のもの + 印

- `event_time` は **`SecondsFormat::Micros`** で書く。**ST07 の `contract::rfc3339`（ミリ秒）を流用しない**（review R5）。
  取り込み口の `event_time` は `timestamp with time zone`（マイクロ秒）で、`content_hash` も `timestamp_micros` を混ぜる
- 起点の換算（Chromium: 1601-01-01、Firefox: 1970-01-01）は純粋な関数にして境界を単体で固定する
- `tz_id` / `tz_offset_min` は**その内容を読んだ取得のときの** `Zone::current()`。`payload.tz_basis = "collected-at"`（review R6）。
  取得時の分の差は、訪問時刻の分の差と夏時間で食い違いうるが、瞬間（UTC）は失われない
- **更新で版が積むときは、行のゾーンは新しい版を読んだ取得のもの**になる（取り込み口は更新で `tz_*` も動かす。
  前の版のゾーンは前の版に残る。review/spec.md R18）。最初に送ったゾーンを帳面に持って送り続ける形は採らない ——
  版ごとに「その内容を読んだとき」が揃っているほうが、読む側の規則が 1 つで済む
- `vanished` / `excluded` / `profiles` の出来事の時刻は取得した時刻。やり直しで同じ識別子が別の時刻で届くと、取り込み口は 1 行のまま版を 1 つ積む（行は増えない）

### D6. 識別子の作り方

**第 2 回 Q5 で本人が「組全体を 1 つのハッシュにする」を選んだ**（2026-09-15）。初版の `v1:<family>:<browser>:<profile_dir>:<visit_id>:<visit_time_raw>:<sha256(url) の先頭 32 桁>` は採らない。

```
visit    : v1:visit:<sha256(family \x1f browser \x1f profile_dir \x1f visit_id \x1f visit_time_raw \x1f url) の 16 進 64 桁>
vanished : v1:vanished:<sha256(browser \x1f profile_dir \x1f 消えた訪問の識別子を並べて整列したもの) の 16 進 64 桁>
excluded : v1:excluded:<sha256(browser \x1f profile_dir \x1f 今回新しく除外した訪問の識別子を整列したもの) の 16 進 64 桁>
profiles : v1:profiles:<sha256(browser \x1f 対応表を整列したもの) の 16 進 64 桁>
```

- **番号だけにしない**（R2）。番号・訪問時刻・URL の組なら、表の作り直しで番号が重なっても別の識別子になる
- **組全体を 1 つのハッシュにする**（review/spec.md R2 / 第 2 回 Q5 の推奨）。識別子は本文を物理的に消しても
  書き換えられずに残る（`gates.sql` が `external_id` の書き換えを拒む）ので、URL だけのハッシュだと辞書で引ける。
  区切りは本文に現れない `\x1f`（組の境目のずれで別の組が同じ値になるのを防ぐ）
- **`external_id` の一意索引（`event_dedup_ext`）は B-tree で、数 KB の URL を入れると索引の行の上限（約 2.7 KB）を超えて
  挿入が 500 で落ちる**。ハッシュなら長さは一定
- **`vanished` / `excluded` / `profiles` は中身から決まる形にする**（review/spec.md R5）。取得時刻を入れると、
  積んだ後・帳面を書く前に落ちて取得をやり直したとき、同じ事実が別の識別子で 2 件入る。
  同じ中身なら同じ識別子になり、取り込み口で畳まれる。`profiles` は対応が変わらない限り同じ識別子で、変わったら新しい行になる
- **`device_id` は入れない** —— 設定の打ち直しで `device_id` が変わると、読み直した 90 日ぶんが全部別の識別子になって二重に入る
- 形は `visit_external_id_is_pinned` で固定する。`v1:` は形を変えたときに読む側が区別するための接頭辞

`source_updated_at` の扱いは D15（仮）。

### D7. 登録簿を「記録ごと」に変える移行は、**このソースの記録が 0 件のときだけ**当てる

```sql
UPDATE core.source SET external_id_kind = 'record'
 WHERE logical_source = 'c02-browser-history'
   AND external_id_kind <> 'record'
   AND NOT EXISTS (SELECT 1 FROM core.event WHERE logical_source = 'c02-browser-history');
```

`migrate()` は起動のたびに全版を当て直す（`crates/server/src/lib.rs`）。**条件を付けないと、本人が後から変えた値を再起動のたびに戻す**
（ST03 の `202609120940_source_columns.sql` が警告している型）。記録が 1 件でも入った後に鍵の宣言を変えると
既存の行と対応が切れる（Q3 の不可逆そのもの）ので、**入った後は移行が触らない**形にする。
名前は作成時刻（`YYYYMMDDHHMM_browser_history_record_id.sql`）。down は `'none'` へ戻す同じ条件の文。

### D8. 更新は ST03 の経路にそのまま乗る —— 取り込み口は変えない

取り込み口は `external_id_kind = 'record'` のソースで、同じ識別子・違う内容の到着を**前の版を `core.event_version` へ移してから更新**する
（`apply_external_update`）。削除済みの行は書き換えない、古い `source_updated_at` の到着は捨てる、も既にある。
**ST08 はサーバのコードを 1 行も変えない**（移行 1 本と `MIGRATIONS` の配列だけ）。
担保として、サーバの結合テストに「`c02-browser-history` で題名だけ違う到着が 1 行のまま版を 1 つ積む」を置く（tasks 2.2）。

### D9. 同期で入った他端末の訪問

**第 2 回 Q4 で本人が「PC 側の番号で作る」を選んだ**（2026-09-15。spec-review R1: 第 1 回 Q3 の context は「発生元の番号で見分ける」と見せていたので、逆の形を本人に返した）。

`originator_cache_guid` が空でない訪問は `originator_cache_guid` / `originator_visit_id` を載せる。**識別子は D6 と同じ形**
（PC の DB の番号で作る）—— 発生元の番号で作ると、同期している PC が 2 台あるとき 2 台が同じ識別子で交互に「更新」を送り、
版が毎日積む。**記録の端末は読んだ PC**（FR-24）。2 台から届いた同じ訪問は、訪問時刻（同期で保たれる）と URL のハッシュで
読む側が 1 件に畳める（Non-Goals）。

### D10. 帳面と「消えた」の見つけ方

- **帳面**はプロファイルごとに置き場へ持つ（`browser-history/<browser>/<profile_dir>.ledger`）。
  中身は識別子ごとの「送った内容のハッシュ・訪問時刻・他端末か・除外したか」と、前回読んだ最大の訪問番号。
  **URL と題名は帳面に書かない**（ハッシュだけ）
- 取得のたびに、帳面にあって今回の読みに無い識別子（除外したものを除く）を**消えた**とし、手がかりを付けて `vanished` に載せ、帳面から外す:
  - `age_days` —— 訪問時刻から取得時刻までの日数（**全ブラウザで事実として載せる**。「90 日を過ぎたか」の真偽にしない ——
    Firefox は日数で消さないので、真偽にするとブラウザごとに意味が変わる。review/spec.md R7）
  - `foreign` —— 他端末の訪問だった
  - `table_recreated` —— 今回の最大の訪問番号が前回より小さい（Chromium は `sqlite_sequence` の値でも見る）
  - `profile_gone` —— プロファイルのディレクトリそのものが無くなった
  - **経路を名指しする値（`deleted_by_user` など）は置かない**
- **1 件の `vanished` に載せるのは 1,000 件まで**（仮）。まとまりは識別子を整列した順に切るので、やり直しても同じまとまりになる（D6）。全期間の削除では数万件が一度に消えるので、取り込み口の 1 件の大きさに収める
- **読めなかったプロファイルでは消えたと判定しない**（読めないのと消えたのを混ぜない。D12 の `blockers` に載る）
- 帳面は一時ファイル + 置き換えで書く（ST07 の `fsutil::atomic_write`）。**壊れていたら退避して空から始める** ——
  失うのは「消えた」の比べの 1 回ぶんで、読み直した訪問は D6 で畳まれる

**反転条件**（仮の値）: 1,000 件が取り込み口の本文の上限に当たると分かったら減らす。Firefox の帳面が実測で大きすぎる（数十 MB）と
分かったら、90 日より前の識別子を帳面から外す（その範囲の「消えた」は見なくなる、と deep.md に追記して本人に返す）。

### D11. 除外の写像と、登録の形を 2 つ足す

ST07 の `exclusion.rs` の `Rule` に 2 つ足す（**`deny_unknown_fields` のまま**。書き間違いは起動時に止まる）:

| 登録 | ウィンドウの記録 | ブラウザ履歴 |
|---|---|---|
| `exe-path` / `process-name`（既存） | 前景のプロセス | **そのブラウザの全プロファイル**（実行ファイル名で `chrome.exe` → Chrome など D1 の表に写す） |
| `title-contains`（既存） | 窓の題名 | ページの題名 |
| `url-contains`（**足す**） | アドレスバーの URL | 訪問の URL |
| `browser-profile`（**足す**。`browser` + `profile`） | 当てない（窓からプロファイルは分からない） | そのプロファイル |

- 除外は**送る前**に判定し（ST07 第 2 回 Q9）、当たった訪問は帳面に「除外した」と書く。**帳面に除外済みとある訪問は数え直さない**
- `excluded` は取得 1 回・プロファイル 1 つにつき 1 件（その回に新しく除外した数が 0 なら書かない）。識別子は中身から決まる（D6）
- **`url-contains` がウィンドウに当たったときは、その前景の変化を丸ごと除外する**（アプリ名・窓の題名も落とし、件数に 1 回数える）。
  ブラウザの窓の題名はページの題名そのものなので、URL だけ伏せると本文が残る（review/spec.md R9）
- **登録を後から足した**とき: 既に送った訪問はそのまま（消さない）。以後その訪問の内容が変わっても送らない。「消えた」の比べからも外す
- **登録を後から外した**とき: 除外済みの訪問がまだ DB にあれば、次の取得で送る（捨てるより入れる）

### D12. 生存信号は 2 本。履歴の側の取得可否

- `c02-browser-history` 用に**別の `Schedule`（86400 秒）と別の数え**（`counters-browser-history.json`）を持つ。
  起動直後に 1 回出す（ST07 と同じ）。未送信の置き場（`heartbeat.jsonl`）は共有する（要求が `logical_source` を持つ）
- 試行と成功は**プロファイル 1 つの読み 1 回**を 1 試行と数える
- `blockers`: `history-unreadable:<browser>:<profile_dir>`（写し・開く・読むのいずれかが失敗）/ `history-none-found`。
  **区間の間に一度でも欠けたものを残す**（ST07 の R26 と同じ和）
- 生存信号の `raw` にはブラウザ名とディレクトリ名だけ（表示名・URL は載せない）
- **区間に読みが 1 回も無いとき**（起動直後の信号・24 時間経たない起動）は、信号を出す前に**見つかった対象の写しを取って開けるか**
  （`SELECT 1` まで）を確かめ、その結果を取得可否にする。行は読まない（review/spec.md R4）。確かめも 1 試行と数える

### D13. ST07 の形は変えない

- `WindowPayload` / `contract::rfc3339` / `LOGICAL_SOURCE` / `payload_shape_is_pinned` / `examples/sample_body` は触らない
- `IngestRequest` に `source_updated_at: Option<String>` を**`skip_serializing_if = "Option::is_none"`** で足す。
  ウィンドウの記録は `None` なので送る本文は 1 文字も変わらない（`window_request_body_is_unchanged` で固定）
- 履歴の要求は `IngestRequest::of_visit`（別の組み立て）で作る

### D14. Windows の実行時テストに履歴を足す

ST07 D25 の `runtime_windows.rs` と同じ流儀。**`windows-latest` には Edge と Chrome と Firefox が入っている**ので、
テストが専用のプロファイル（`--user-data-dir` / `-profile` で一時ディレクトリ）でページを開き、
**ブラウザを開いたまま**本物の読み手に読ませる。Linux の単体は `rusqlite` で Chromium / Firefox の表の形を作って
見分け・消えた・除外を確かめる（本物のブラウザの挙動は実行時テスト、規則は単体）。

**Chrome と Firefox の実行時テストは飛ばさない**（review/spec.md R10）。runner に無ければ job で入れる。
6 つのブラウザの置き場の探し方は単体（D1 の表を全行）が持つ。
**反転条件**: runner の Firefox / Chrome が一時プロファイルに履歴を書くまで待てない（不安定）と分かったら、
待ち方を直す。**飛ばす形には倒さない** —— 本人が Q1 で選んだ 6 つのうち読み手が 2 種類あり、Firefox の読み手を
本物のブラウザで 1 度も確かめないことになるため、倒すなら本人に返す。

### D15.（仮）`source_updated_at` には、その内容を読んだ時刻を載せる

契約（`docs/collector-contract.md`）と正典 `record-envelope` はこの欄を「外部サービス側の更新時刻（**または版**）」と定めている。
**履歴 DB は更新時刻を持たない**が、読んだ時刻は同じ訪問について単調に進む**版**として働く。載せないと、未送信の再送で
**古い題名の到着が新しい題名を書き戻す**（取り込み口は更新時刻の無い到着を「届いた順」で当てる）。
観測できる振る舞い（書き戻さない）は spec の識別子の Requirement に置いた（review/spec.md R6）。

**反転条件**: `record-envelope` の側で「版」の読みが外部の値に限られると決まったら、読んだ時刻ではなく
帳面に持つ単調な版番号に替える（外から見える振る舞いは同じ）。

## Risks / Trade-offs

- **[同期の印の切り替わりで版が一斉に積む]** → 同期を入れると `is_known_to_sync` が 90 日ぶん変わり、全訪問が 1 版ずつ積む。
  1 回きりで、失うものは無い（容量だけ）。反転条件: 実測で問題なら `is_known_to_sync` を原文から外す（**形が変わるので本人に返す**）
- **[Firefox の初回が大きい]** → 何年分も持っていると、初回の未送信が数十万件になる。上限は置かない（ST04）。D3 の反転条件
- **[写しの途中で DB が書き換わる]** → 写しが壊れていたら開けずに「読めなかった」になり、次の見回りで試し直す（D3）。黙って 0 件にしない
- **[URL にトークンが入る]** → ST07 と同じ。本人が Q1（全プロファイル）と ST07 の Q3 / Q4 の代償を読んだうえで選んだ。守りは除外（D11）と PERM-9
- **[プロファイルの置き場が表に無い]** → 「見つかったもの」に入らず、黙って取られない。D1 の反転条件。
  `history-none-found` だけは生存信号に出る

## Migration Plan

- 移行 1 本（D7）。**このソースの記録が 0 件のときだけ**当たり、2 回目以降の起動では何もしない
- **merge の順序は書かない** —— gate が main に rebase するので、後から merge する側が追従する

## Open Questions

無し。深掘り 1 巡・3 問で本人が決めたことと、C（聞かない）に落とした deep.md の C-1〜C-8 で閉じている。
