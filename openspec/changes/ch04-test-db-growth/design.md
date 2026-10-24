# CH04 design — 試験用 DB に溜まるデータで試験が遅くなるのを止める

> 第 1 回の深掘りの推奨の答え（Q1〜Q4）と、聞かずに決めたもの（C1〜C7。`premises.md` の末尾）で先に書いた。
> 本人の答えが推奨と違えば、その Q を `refs` に持つ D を書き直す（各 D の反転条件）。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-10-24 の repo と各 worktree の DB）:

- `testdb::pool()` は `DATABASE_OWNER_URL` の database（`ashiato`）にそのまま繋ぎ、初回だけ移行を当てる（`testdb.rs:77-104`）。試験の隔離は利用者・ソース・日（`unique_day()`）で取っていて、行は消さない（P1）
- 消せない表は行の DELETE と表の TRUNCATE をトリガが断る（P2）。`DROP DATABASE` はトリガを撃たない（P3）。所有者の役割は `CREATEDB` を持つ（P4。CI も同じ）
- 1 回の `cargo test` で 2.3〜4.4 万行が入る（P12）。書き出しの除く範囲は利用者を問わず全部の行を判定する（P7・C3）ので、書き出しの試験（116 本）は回の中でも互いのデータを判定し合う
- 使い捨ての database を作る試験が 5 か所あり、`testdb::url()` の末尾の database 名を書き換えて `*_tmp_*` を作る。DROP は返り値の future を待ったときだけ走り、panic すると残る（P13）
- プールは試験ごとに作って捨てる（`testdb.rs:74-76`）。`#[tokio::test]` は試験ごとに runtime を作るので、`static` に置いた接続は作った runtime と一緒に死ぬ
- 書き出しの除く範囲の判定は、CH02 の後、次の記録を窓関数で一度に読み、最後に候補の行ごとに `erased_spans`（MATERIALIZED。索引なし）を `EXISTS` で舐める（P14）。
  CH02 の実測で 120 万行・消した滞在 1 件の件数の数えが 25.5 秒。その大半は窓の並べ替えと緯度経度の判定で、CH04 の直しでは減らない。
  CH04 が取るのは、M が年数とともに増えたときに乗る候補の行 N × 消した時間 M の項（今の量では最悪の形でも 120 万行・M = 100 で +7.3 秒。P14）

## Goals / Non-Goals

**Goals:**
- 試験が入れたデータで、後の試験（同じ回の他の書き出しの試験も、次の回も）が遅くならない（`test-database` の 1〜4 つ目の Requirement）
- 1 本の問い合わせが何十分も走り続けない（5 つ目）
- 前の回の database が残らないことを、台本と CI が確かめる（6 つ目）
- 書き出しの除く範囲の判定が、年数（N と M を一緒に）の 2 乗で伸びない（`data-durability` の ADDED）。直したかは消した時間を読んだ行の数で見る（時間は測って残すだけ。Q2）

**Non-Goals:**
- いま各 worktree の `ashiato` に積もっているデータと、使われていない Docker の volume（Q4）。この change の後の試験は `ashiato` に記録を積まないので、遅さには効かなくなる。容量は smoke か `down -v` を走らせた worktree から戻る
- 件数の数え方の替え（ST33 design D13。120 万行で 3 秒を超えたまま。数字を渡すだけ。Q2）
- 除く範囲を利用者で絞ること（C3）、除く記録を変えること（C2）
- 消せない表のトリガを外す・無効にする・`session_replication_role` を変えること（C4）
- `tests/server_startup.rs`・縦串・開発用の起動の繋ぎ先（`ashiato` のまま。C7）、worktree ごとの port（C5）
- 試験の過程の外から殺された回の、殺された時点で走っていた問い合わせの即時の停止（D7 の上限と、次の回の片付けの `WITH (FORCE)` に任せる）

## Decisions

### D1（仮）. 名前と印 —— 回の id を頭に持つ 4 種の database

回の id は `ashiato_run_<乱数 12 桁の 16 進>`（過程ごとに 1 つ）。database の名前は全部この id で始める:

| 種 | 名前 | 中身 | いつ消えるか |
|---|---|---|---|
| テンプレート | `<id>_tpl` | 移行と権限（`crate::MIGRATIONS`・`crate::GRANTS`）を当てたもの。当てた後は誰も繋がない | 次の回の始まり |
| 回の database | `<id>` | テンプレートを写したもの。`pool()`・`app_pool()`・`url()` の繋ぎ先 | 次の回の始まり |
| 試験ごと | `<id>_x_<乱数 12 桁>` | テンプレートを写したもの。書き出しの試験が使う（D4） | その試験の終わり（Drop）。殺されたら次の回の始まり |
| 使い捨て | `<id>_tmp_<乱数 12 桁>` | 空（移行を当てない）。移行の当て方を見る試験が使う（D5） | 同上 |

- 名前は 63 字に収まる（最長 41 字）
- 回が走っていることの印は、**回の間ずっと持つ session の助言ロック**（`pg_advisory_lock(4820253, hashtext('<id>'))`）。`postgres` database に繋いだ専用の 1 本の接続が持つ（C6）。
  助言ロックは database ごとに分かれるので、持つ側も確かめる側も `postgres` database で取る
- 専用の接続は、`std::thread` で立てた自分の runtime（`current_thread`）の上に置き、過程が終わるまで手放さない。`#[tokio::test]` の runtime に置くと、その試験の終わりに接続ごと死ぬ（Context）。
  過程が SIGKILL で殺されると、カーネルが TCP を閉じ、PostgreSQL が backend を終わらせてロックが外れる
- 退けた案: 名前に pid を入れて生死を `/proc` で見る —— pid は使い回され、CI の容器と手元で見え方が違う。接続の数で見る —— 走っている回でも 0 本になる間がある（C6・review/deep.md R3）

反転条件: 専用の接続が試験の途中で切れる（PostgreSQL の再起動など）と、その回は走っていないと読まれる。切れたことを検知したら、その回の試験を落とす形に替える（いまは扱わない。DB の再起動の間の試験はどれにせよ落ちる）。

### D2（仮）. 作る順と直列化 —— 片付けと作成は 1 本ずつ、試験ごとの写しは回の中で 1 本ずつ

`pool()` の初回（`tokio::sync::OnceCell`。いまの `MIGRATED` を置き換える）が次を行い、回の database の URL を覚える（プールは覚えない。runtime を越えて持てない）:

1. `postgres` database に所有者で繋ぎ、`pg_advisory_lock(4820252)`（片付けと作成の直列化。いまの移行の直列化 `4820251` とは別の鍵）を取る
2. 専用の接続（D1）で自分の回の助言ロックを取る
3. 片付け（D6）
4. `CREATE DATABASE <id>_tpl` → それに繋いで移行と権限を 1 つのトランザクションで当てる → 接続を閉じる
5. `CREATE DATABASE <id> TEMPLATE <id>_tpl`
6. 1 のロックを外す

- 試験ごとの database は `CREATE DATABASE <id>_x_… TEMPLATE <id>_tpl` を、過程の中の `tokio::sync::Mutex` で 1 本ずつ打つ（同じテンプレートからの同時の写しを避ける。P16）
- テンプレートに移行の後で繋ぐ接続は無い（`url()` も `pool()` も回の database を指す）
- 写す時間は下流が tasks 2.3 で測って下の「計測の記録」に書く。反転条件: 1 本あたり 1 秒を超えるなら `STRATEGY FILE_COPY` を試し、それでも超えるなら Q1 の推奨（書き出しの試験だけ試験ごと）の重みを本人に返す

### D3. `pool()`・`app_pool()`・`url()` の形は変えず、繋ぎ先だけを回の database にする

- 3 つとも引数・戻り値の型を変えない。st09・st13 が `testdb.rs` に足した関数（`source_media`・`archive_scan_row`・`fail_ledger_source_insert`）は `pool()` を受け取るだけなので、rebase の後もそのまま動く
- `app_pool()` はアプリの役割（`DATABASE_URL` の利用者と合言葉）で回の database に繋ぐ。新しい database にもアプリの役割は `CONNECT` を持つはず（P15。確かさ 2）。`app_role_tests` が回の database で通ることを tasks 2.4 で見る
- `url()` は回の database の URL を返す。同期の関数なので、回がまだ作られていなければ panic で「先に `pool()` を呼ぶ」と示す（いまの呼び出しは全部 `pool()` の後。下流が `rg 'testdb::url\(\)'` で確かめる）
- `unique_day()` は残す。隔離の理由の注記だけを「回の database の中で書き出しの試験以外が共有する」に書き直す（C1。試験は変えない）

### D4（仮）. 書き出しの試験は `testdb::isolated()` の試験ごとの database を持つ

- `testdb::isolated().await` が `TestDb` を返す。`TestDb::pool().await` はその database に繋いだプール（呼ぶたびに同じ database）、`TestDb::url()` はその URL、`TestDb::name()` は名前
- `TestDb` の `Drop` は、`std::thread` で立てた自分の runtime で `postgres` database に繋ぎ、`DROP DATABASE <名前> WITH (FORCE)` を打って、そのスレッドを待つ。
  試験が通っても panic しても（unwind の途中でも）Drop は走る。`WITH (FORCE)` は試験のプールの残りの接続を切る
- **Drop は panic しない**（いつでも。巻き戻しの途中で Drop が panic すると過程が abort し、同じ実行ファイルの全部の試験が結果を出さずに止まる）。繋げない・DROP が落ちた・スレッドが panic した、のどれでも、
  名前と理由を `eprintln!` で出して戻る。残りは次の回の片付け（D6）が消す（review/spec.md R15）
- 当てるのは `export_*tests.rs` の全部（`export_tests.rs`・`export_file_tests.rs`・`export_job_tests.rs`・`export_ledger_tests.rs`・`export_excluded_tests.rs` と、CH02 の `export_window_tests.rs`）。
  試験の中で `testdb::pool()` を何度も呼ぶ所は、全部同じ `TestDb` から取る（取り違えると、別の database を見て件数が合わず落ちる —— 落ちる側なので見逃さない）
- `World::new()`（`export_excluded_tests.rs:27`）は `TestDb` を持つ欄を足し、`World` が試験の最後まで生きる形にする
- `EXPORT_SERIAL` は残す（書き出しの置き場の直列化は database と別の理由。C1）
- 退けた案: 試験の中で `testdb::pool()` が「いまの試験の database」を返す（task-local）—— 背景の取り込み器のように別の task で `pool()` を呼ぶ所で黙って回の database に落ちる

反転条件: 書き出し以外の試験にも、同じ回の他の試験の記録で遅くなるものが見つかれば（tasks 7.2 の全体の時間で目立つ群）、その群にも `isolated()` を当てる。写す時間が書き出しの試験の数で効きすぎれば（D2 の反転条件）、書き出しの試験だけを回の database に戻すかを本人に返す（Q1）。

### D5（仮）. 使い捨ての database は `testdb::scratch()` が作る

- いまの 5 か所（`clock_tests.rs`・`feelings_tests.rs`・`stay_tests.rs`・`attributes_tests.rs`・`places_tests.rs`）の自前の `fresh_db()` の中身を、`testdb::scratch().await`（空の `<id>_tmp_…` を作り、`TestDb` と同じ Drop を持つ値を返す）に替える。
  `fresh_db()` の名前と戻り値の形（プールと、待つと消える future）は残す（呼び出し側と、`fresh_db` を使う CH02 の `export_window_tests.rs` を書き換えない）。future は Drop と二重に消しても落ちない（`DROP DATABASE IF EXISTS`）
- **database を作る書き方を `testdb.rs` の外に置かせない**: `tools/check-test-db-growth.sh --lint` が `crates/server/src` と `crates/server/tests` の `CREATE DATABASE` を挙げて落とし、文言に `testdb::isolated()` と `testdb::scratch()` を示す。
  `testdb.rs` と、行に `db-ok: <理由>` の印がある行（同じ行か 1 行上。CH03 の `tmp-ok:` と同じ規則）は通す。片付けの試験（D6）が持ち主のいない回の database を作るのは、この印で通す
- 同じ `--lint` が、書き出しの試験のファイル（`crates/server/src/export*_tests.rs`）の `testdb::pool()` も落とし、文言に `testdb::isolated()` を示す（同じ `db-ok:` で通す）。
  走っている st29 は `export_file_tests.rs` に `testdb::pool()` を 3 か所足している。CH04 の後に入る側は、CI の `--lint` が rebase の後に止める（review/spec.md R9）
- 退けた案: 名前の規則を各所に書かせる —— 規則が 5 か所に散り、走っている Story が足した新しい所は印を持たないまま残る

反転条件: `--lint` が書き出しの試験のファイルの中の正当な `testdb::pool()`（回の database を見ること自体を確かめる試験など）を落とす例が `db-ok:` で足りないほど増えたら、対象をファイル名でなく試験の関数の頭（`export_`）で選ぶ形に替える。

### D6. 片付けの規則 —— 走っていない回のものと、印の無い `_tmp_` の残りだけを消す

`pg_database` の名前を読み、次を `DROP DATABASE … WITH (FORCE)` で消す:

- 名前が `ashiato_run_<12 桁>` で始まり、その回の助言ロック（D1）を `pg_try_advisory_lock` で取れた（= 持ち主がいない）もの。取れたロックはすぐ外す。取れなければ触らない
- 名前に `_tmp_` を含み、`ashiato_run_` で始まらないもの（CH04 の前のコードが残した `st16_tmp_…` など。P13）

`ashiato`・`postgres`・`template0`・`template1`・それ以外の名前（CH02 の計測の `ch02_volume_…` や smoke の `restored` など）は消さない。
`WITH (FORCE)` は、殺された回の backend が走らせたままの問い合わせ（P10 の 23 分）を切るため。消せない表のトリガは database ごと消えるので撃たない（P3・C4）

反転条件: 印の無い `_tmp_` を消したことで、同じ worktree で並んで走っている CH04 の前のコード（rebase していない古い build）の試験が落ちる例が出たら、`_tmp_` は作られてから 1 時間より古いものだけを消す
（`pg_database` は作った時刻を持たないので、`pg_stat_file` で database の置き場の時刻を読む形になる）。

### D7（仮）. `statement_timeout` の既定は 60 秒。環境変数で替える

- 試験の足場の全部の接続（`pool()`・`app_pool()`・`TestDb`・`scratch()`）に、繋ぐときの `options` で `statement_timeout` を置く。移行を当てる 1 本の `raw_sql` にも効く
- 既定は 60 秒（仮）。`ASHIATO_TEST_STATEMENT_TIMEOUT`（PostgreSQL の時間の書き方。`0` で上限なし）で替える。`tools/export-scaling.sh` は `0` を渡す
- 環境変数の値から上限を決める部分は、値を引数で受ける純粋な関数（`statement_timeout_from(Option<&str>)`）にし、試験は環境変数を書き換えずにそれを確かめる（並走する試験に効かせない）。
  上限を引数で受けて接続を作る内側の関数も足場に置き、`pg_sleep` の試験はそれに 1 秒を渡す（review/spec.md R6）
- 値の根拠: CH02 の実測で、2 万行（量の試験の既定）の書き出しの全体が数秒、10 万行で 12.7 秒。試験の 1 本の文で 60 秒を超えるものは、いまの共有の DB の積もり以外に見当たらない
- 上限で落ちた試験は、PostgreSQL のエラー（`canceling statement due to statement timeout`）を unwrap して panic し、`cargo test` がその試験の名前を出す

反転条件: tasks 7.2 のまっさらな回で 1 本でも上限で落ちる正当な試験があれば、その試験の最長の文の時間の 2 倍に既定を上げ、ここに書く。

### D8（仮）. `tools/check-test-db-growth.sh` —— 一覧を 3 回撮って比べる

- 使い方: `tools/check-test-db-growth.sh [cargo test の引数…]`（引数が無ければ `-p ashiato-server`）/ `--lint`（D5）/ `--self-test`
- 手順: 一覧 S0 → 試験 1 回目 → 一覧 S1 → 試験 2 回目 → 一覧 S2。一覧は `pg_database` の名前（`tools/db.sh exec -T db psql` で `postgres` database を読む。`TEST_DB_LIST_CMD` で差し替えられる）。
  試験の命令は `TEST_DB_CMD` で差し替えられる（既定は `cargo test <引数>`）。先に `cargo test --no-run` で組む
- 判定（順に。最初に外れたもので落ちる）: 試験の rc（0 でなければその rc）→ S1 にあって S0 に無い名前のうち S2 に残るもの（前の回の残り）→ S2 にあって S1 に無い名前のうち、
  `_x_` か `_tmp_` を含むもの、または `ashiato_run_` の回の id が 2 つ以上あるもの（最後の回の余計な残り）→ S0 に `ashiato` があって S2 に無い。落ちるときは名前を挙げる
- 同じ worktree で別の `cargo test` と並べて走らせない（並んだ回の database が S1 と S2 の差に入る）。冒頭のコメントに書く
- CI の rust job: `cargo test` の行の後に、`--self-test`・`--lint`・`tools/check-test-db-growth.sh -p ashiato-server ch04_` の 3 行。絞り込みの `ch04_` は、この change が足す試験
  （回の database・試験ごと・使い捨て・片付けを使う）だけを 2 回走らせる。全部を 2 回走らせると CI の時間が倍になる
- 前例は `tools/check-log-private.sh`（`--self-test`・CI）と CH03 の `tools/tmp-leak-check.sh`（試験の rc → 本数 → 残り の順）

反転条件: CI で一覧を読む psql の利用者・database の名前が手元と違って読めないなら、`DATABASE_OWNER_URL` の database を `postgres` に替えた URL で、容器の外の psql か `cargo run` の小さな道具で読む。

### D9（仮）. 重なりの判定は、消した時間を 1 つの範囲の集まりにして 1 回で引く（Q2 の推奨）

CH02 の後の `overlap_sql` の最後の条件（`EXISTS (SELECT 1 FROM erased_spans s WHERE s.start_at <= w.width_end AND s.end_at >= w.event_time)`）を、次の 3 つの和に替える:

1. 整った消した時間（`start_at` と `end_at` がどちらも NULL でなく、`start_at <= end_at`）を `range_agg(tstzrange(start_at, end_at, '[]'))` で 1 つの `tstzmultirange` にした CTE を 1 度だけ作り、
   整った幅（`w.event_time <= w.width_end`）の行は `その multirange && tstzrange(w.event_time, w.width_end, '[]')` で判定する（multirange と範囲の重なりは二分探索。M 件を行ごとに舐めない）。
   **幅の側の範囲は `CASE WHEN w.event_time <= w.width_end THEN … && tstzrange(…) END` の中でだけ作る**（`tstzrange` は始まりが終わりより後だとエラーになり、AND は評価の順を約束しない。順を固定するのは CASE だけ。review/spec.md R3）。
   消した時間の側は CTE の WHERE で集約の前に絞るので、そのままでよい
2. 整っていない消した時間（`start_at > end_at`）は別の小さな CTE に分け、いまの `EXISTS` の式のまま全部の候補の行に当てる（本物では消した滞在の版の `end` が始まりより前のときだけで、件数は小さい）
3. 整っていない幅（`w.width_end < w.event_time`。写しの時点が記録より前など）の行は、いまの `EXISTS` の式のまま全部の消した時間に当てる

- 閉じた区間（`'[]'`）にするのは、いまの `<=` / `>=` が端を含むため。端どうしが触れるだけでも重なる（spec の突き合わせの族）
- `start_at` か `end_at` が NULL の消した時間は、今の view からは来ない（`core.stay_erased` も前の版の側も `coalesce` で埋める。review/spec.md R5）。来ても、いまの式では比較が NULL になってどの行とも重ならないので、1 も 2 もそれを含めない（`range_agg` に NULL の端を渡すと無限の端になり、除き過ぎる）
- 使い捨ての PostgreSQL 17 で、NULL の端・逆向きの区間・端が触れるだけのものを含む乱数の族を 6 回比べ、CH02 の `EXISTS` と差が 0 件だった（review/spec.md R3）
- 2 と 3 の組み合わせ（整っていない幅 × 整っていない消した時間）は 3 が全部の消した時間に当てるので拾う
- 退けた案: `erased_spans` に GiST 索引 —— CTE に索引は張れず、一時表は写しの `READ ONLY` で作れない（CH02 D5 の表）。消した時間を時刻で並べて二分探索を SQL で書く —— 重なり合う消した時間で誤る
- 結果が変わらないことは D11 の突き合わせが固定する

反転条件: tasks 8.3 の読んだ行の数が 8 倍の線を越える（`range_agg` の組み立てや 3 の行が多いなど）なら、計画を読み、伸びている節をここに書いて直す。直せないなら Q2 の判定を本人に返す。

### D10. 移行は足さない

D9 は SQL の断片の中だけで閉じ、表にも索引にも触らない。CH02 の D1 が退けた索引（120 万行で 14 分）にも戻らない。`migrations/ORDER` に行を足さない。

### D11. 変わらないことは、最後の条件だけを足す前の形に戻した判定と突き合わせる（C2）

- 比べる相手は、今の `excluded_sql(…).ctes()` の文字列から、D9 の最後の重なりの条件（`excluded.rs` の 1 つの定数に置く）を 1 か所だけ CH02 の `WHERE EXISTS (SELECT 1 FROM erased_spans s WHERE s.start_at <= w.width_end AND s.end_at >= w.event_time)` に置き換えたもの。
  置き換えた数が記録の側と前の版の側で 1 つずつ（計 2）であることを assert する。CH02 の `excluded_sql_strict` と同じ形（review/spec.md R4）
- 丸ごと写さない: `overlap_sql`・`end_key_sql`・`ExcludedSql` の欄は私的で、別のファイルからは見えない。走っている st41 が (a) の読み方と `erased_spans` を変えるので、写しは黙って古い形のまま残る。
  置き換える形なら、st41・st09 が後から入っても、比べる 2 つは最後の条件だけが違う
- 突き合わせの試験は `crates/server/src/export_overlap_tests.rs`（新しいファイル）に置き、`testdb::isolated()` の試験ごとの database（他の試験の消した滞在が混ざらない）で、同じトランザクション・同じ写しの時点の式で両方を走らせ、`excluded_event` と `excluded_version` の id の集まりが一致することを見る
- 族（spec の Scenario の列挙）: 消した時間の始まり・終わりに幅の端がちょうど触れる / 中に入る / 外れる・終わりが始まりより前の消した時間（前の版の `end` を始まりより前にする）・
  内容に `end` を持たない消した滞在（終わりは始まりの 1 µs 後）と `end` を持たない前の版（終わりは前の版の時刻）・
  幅の終わりが記録の時刻より前の記録（写しの時点を記録より前の固定の時刻で渡す）・重なり合う消した時間と隣り合う消した時間・消した滞在の前の版の広い時間・前の版の行・消した滞在 0 件・1 µs の差。族は外さない
- 新しい試験の名前の頭は `ch04_`（D8 の CI の絞り込みに入る）

### D12（仮）. `docs/changes/CH04.md` の `capability` は空のままにする

`data-durability` を書くと、走っている ST09・ST29・ST41・CH02 と衝突待ちになる。CH04 が足すのは別の名前の Requirement（ADDED）で、どれの MODIFIED とも重ならない（proposal の Impact）。
`test-database` は新しく、他に触る change は無い。失うものは無い（重なれば gate の rebase で追従する）。
反転条件: CH04 が走っている間に、別の Story / change が `data-durability` の同じ名前の Requirement を足すか、`test-database` を作る上流に入るなら、`capability` を書いて admission に止めさせる。

### D13（仮）. 試験の DB の約束は、CH03 の `test-hygiene` と別の capability `test-database` に置く

CH03 の `test-hygiene` は CH03 の delta にしかなく、正典に無い。同じ新しい capability に 2 つの change が `## Purpose` を持つ ADDED を積むと、archive の後の側の Purpose の扱いが決まっていない。
CH04 を CH03 に依らせると、CH03 の gate を待つ理由のない作業が待つ。一時フォルダと試験の DB は、壊れ方（ファイルが残る / 行が積もって遅くなる）も直し方も別。
反転条件: 両方が archive された後に「試験の後始末」として 1 つにまとめたくなったら、`test-database` の Requirement を `test-hygiene` へ移す change を立てる（OpenSpec は capability の改名を許さないので、移す）。

### D14（仮）. 時間は `tools/export-scaling.sh` が既存の量の試験で測って残す。合否にしない（Q2 の推奨）

- 直したかの合否は、消した時間を読んだ行の数で見る（spec の 1 つ目の Scenario。tasks 8.3）。時間は、今の量では入れ子の費用が線形の部分に埋もれ（最悪の形でも 120 万行・M = 100 で +7.3 秒。P14）、
  直す前の式でも隣り合う組の比が 2.5 倍を下回るので、判定にならない（review/spec.md R1）
- `export_volume_one_run_reports_time_and_memory`（`export_file_tests.rs:1449`）に、消した滞在の数 `ASHIATO_EXPORT_VOLUME_ERASED`（CH02 が `0` / `1` で足した変数を、件数に広げる。既定 `1` のまま）を読ませる。
  M ≥ 2 のときは、記録を M 日に等分して置き（1 日 N / M 行。日をまたいで密度を同じにする = 年数を伸ばすのと同じ形）、各日の 01:00〜02:00 を消した滞在にする。
  M ≤ 1 の置き方と assert は変えない（C1。既定の 2 万行の試験の振る舞いは同じ）
- 台本は (N, M) = (300,000, 25)・(600,000, 50)・(1,200,000, 100) の 3 回を `ASHIATO_TEST_STATEMENT_TIMEOUT=0` で走らせ、各回の `seconds` と `summary_seconds` と、隣り合う組の比を標準出力に出す。
  下流は D9 を当てる前（Task 8.4 の前）と後の両方で走らせ、下の「計測の記録」に写す（前と後の差が、入れ子を取った分）
- 試験は書き出しの試験なので、D4 で試験ごとの database で走る（CH02 D8 の手作りの使い捨ての database は要らない）
- `--self-test` は偽の時間（`EXPORT_SCALING_CMD` で差し替え）で、値と比を出すこと・試験が落ちればその rc で落ちることを見る
- CI では走らせない（120 万行で数分。CH02 の実測で書き出し 146.5 秒）。下流の記録と確認バッチの材料にする

反転条件: D9 の後の値が前より遅くなれば（multirange の組み立てが重いなど）、計画を読んで D9 を直す。

### D15. 文書

- `docs/testing.md` に 1 節: 試験の DB は回ごとに作られ、次の回の始まりに片付く / 書き出しの試験と使い捨ては `testdb::isolated()`・`testdb::scratch()` / `CREATE DATABASE` を書かない / `ASHIATO_TEST_STATEMENT_TIMEOUT` / 台本の使い方
- `docs/flow-gates.md` の検査の表に `tools/check-test-db-growth.sh` を 1 行
- `testdb.rs` の冒頭の注記に、回の database と片付けの規則（D1・D6）を書く

## Risks / Trade-offs

- [書き出しの試験の数だけ database を写す時間が増える] → tasks 2.3 で 1 本の時間を測る。D2 の反転条件
- [`app_pool()` が新しい database に繋げない（P15 の確かさ 2）] → tasks 2.4 で `app_role_tests` を回の database で通す。落ちたら回の database の作成の後に `GRANT CONNECT` を足す（D3 に書き足す）
- [片付けが走っている回の database を消す] → 助言ロックで判定し（D1）、spec の Scenario「走っている回の database は、接続が 0 本でも消えない」が固定する
- [印の無い `_tmp_` の残りを消すと、同じ worktree の古い build の試験を落としうる] → D6 の反転条件
- [`statement_timeout` で正当な試験が落ちる] → tasks 7.2 のまっさらな回の全体で確かめる。D7 の反転条件
- [片付けの試験が並んで走り、ほかの試験の片付けが作りかけの持ち主のいない database を消す] → 片付けの試験は TEMPLATE を使わない空の database を材料にし、1 つの `tokio::sync::Mutex` で並ばせない（tasks 3.2。review/spec.md R16）
- [D9 の 3 つの和で、いまの `EXISTS` と読みが 1 件でも食い違う] → D11 の族で突き合わせる。食い違いは除く記録が変わる（C2 に反する）ので、見つかれば D9 を直す（族を減らさない）
- [CH02 の gate の後に CH02 の SQL が変わる] → `requires: [CH02]` で下流の入口が CH02 の gate を通った SHA を取り込み、D11 はその SHA の SQL を写す。CH02 の spec が変われば merge gate が「依存の spec:」で落とす
- [各 worktree の `ashiato` は積もったまま（Q4）] → 遅さには効かない（試験が読まない）。容量は残る

## Migration Plan

移行は無い（D10）。戻すときは `testdb.rs`・試験の手伝い・`excluded.rs`・台本・CI の行を前の commit に戻すだけ。残った `ashiato_run_*` の database は、戻した後は誰も消さないので、`tools/db.sh down -v` で volume ごと捨てる。

## 計測の記録（下流が書く）

| 何 | 値 |
|---|---|
| テンプレートを写す 1 本の時間（tasks 2.3） | 72〜197 ms（16 本同時に呼び、1 本ずつ写した。1 秒未満。D2 の反転条件に触れない。2026-10-24） |
| まっさらな回の `cargo test -p ashiato-server` の全体の時間（tasks 7.2） | 113.02 秒（lib の 1216 本。2026-10-24） |
| 同じ worktree で 2 回目の全体の時間（tasks 7.2。1 回目と同じ桁なら積もっていない） | 113.99 秒（1 回目と同じ桁。前の回の database は残らなかった） |

| (N, M) | D9 の前 seconds（証跡なし） | D9 の前 summary_seconds | D9 の後 seconds（別の回・証跡なし） | D9 の後 summary_seconds | D9 の後 seconds（証跡 9.2。head e27a521a） | D9 の後 summary_seconds（証跡 9.2） |
|---|---|---|---|---|---|---|
| (300,000, 25) | 55.7 | 10.5 | 44.2 | 8.9 | 75.3 | 15.6 |
| (600,000, 50) | 100.5 | 714.1 | 86.6 | 18.1 | 103.1 | 537.2 |
| (1,200,000, 100) | 331.0 | 2643.1 | 193.7 | 2007.4 | 210.6 | 2176.2 |

出どころ（2026-10-24 測定。`tools/export-scaling.sh`）:
- 「前」は `excluded.rs` だけを 7700aa81 の親の版に戻した同じ木での 1 回。evidence.jsonl に項目として残っていない（この表だけが記録）。
- 「後（別の回）」は D9 の木での 1 回で、証跡の無い試走。
- 「後（証跡 9.2）」だけが `scripts/verify-run 9.2` の公式の記録（evidence.jsonl の 9.2・head e27a521a・3382 秒）。

読み方: 同じ D9 の木の 2 回の間で seconds が 44.2 → 75.3、86.6 → 103.1 と動く。回ごとの揺れが前後の差より大きいので、「前と後のどちらが速いか」はこの測定では言えない（1 回ずつの比較は無効。D9 が速くなったとも遅くなったとも主張しない）。
D14 の反転条件は、証跡のある後の 1 回だけで判定する。隣り合う組の seconds の比は 1.37・2.04 で、どちらも 2.5 倍以内なので触れない（別の回の後は 1.96・2.24、前は 1.80・3.29）。
前後の優劣を言うには同じ条件で複数回測って中央値を取る必要があるが、この change の目的（件数を増やしても seconds が線形に近い）には要らないので測り直していない。

注意: `summary_seconds` は前も後も (600,000, 50) か (1,200,000, 100) で数百〜数千秒に跳ねる（後: 8.9 → 18.1 → 2007.4）。D9 の差ではなく件数の数え（`read_summary`）の側の計画の変わり目に見える。この change の範囲外なので原因は調べていない。
