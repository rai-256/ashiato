# CH04 の前提（2026-10-24 questions の段で確かめた）

この change の決定は外のサービスの形式には依らない。依るのは、手元の PostgreSQL のコンテナと、いまの試験の足場（`crates/server/src/testdb.rs`）・書き出しの除く範囲の SQL（`crates/server/src/export/excluded.rs`）の振る舞い。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | 試験は worktree ごとのコンテナの `ashiato` という 1 つの database に書き、消さない | Q1（隔離の手段） | `testdb.rs` を読む。`docker exec ashiato2-<id>-db-1 psql -c 'select count(*) …'` | `pool()` は `DATABASE_OWNER_URL` の database（`ashiato`）にそのまま繋ぐ。行数は st23 17.4 万・st41 18.3 万・verify 4.3 万・立てたばかりの st33 217。`event` の利用者は st23 で 1,461 人（試験 1 本ごとに `testdb::user()`） | 実物 | 5 |
| P2 | 消せない表（`core.event` `event_version` `heartbeat` `erasure_ledger` `stay_criteria` `stay_absorbed` `drop_report` など）は行トリガで DELETE を、文トリガで TRUNCATE を断る | Q1（試験の終わりに消す、ができるか） | `migrations/202609120944_gates.sql:233-291`・`202609142125_stays.sql:50-76`・`202609151546_drop_reports.sql:53-73` | 断る。試験の終わりに行を消すには、トリガを外すか `session_replication_role` で黙らせるしかない（依頼が禁じている） | 既存のテスト材料（migrations） | 2 |
| P3 | `DROP DATABASE` は行・文のトリガを撃たない（表ごとではなく database ごと消える） | Q1（使い捨ての database） | PostgreSQL の仕様（トリガは DML と TRUNCATE にだけ付く） | 撃たない。トリガの定義は触らない | 公式文書 https://www.postgresql.org/docs/17/sql-createtrigger.html | 2 |
| P4 | 試験の所有者の役割（`ashiato_owner`）は database を作れる | Q1 | `tools/db-roles.sh:50` | `ALTER ROLE ashiato_owner LOGIN CREATEDB NOSUPERUSER …` | 既存のテスト材料 | 2 |
| P5 | DB のコンテナは匿名 volume を持つ（compose に `volumes:` は無いが、image が宣言している） | Q4（今あるデータ） | `docker inspect ashiato2-st23-db-1 --format '{{json .Mounts}}'`・`docker volume ls -qf dangling=true`（review/deep.md R8） | `/var/lib/postgresql/data` に匿名 volume。`up --force-recreate` では中身が残り、空になるのは `down -v` か `up -V` のときだけ。使われていない volume が 62 個積もっている | 実物 | 5 |
| P6 | 縦串（`tools/smoke.sh`）は試験と同じコンテナを使い、始まりと終わりに volume ごと捨てる | Q4 | `tools/smoke.sh:29,36`（trap と始まりの `docker compose down -v`）（review/deep.md R9） | smoke を走らせた worktree では、積もった試験のデータはその時点で消える（同じ worktree で走っている `cargo test` も巻き込む） | 既存のテスト材料 | 2 |
| P7 | 書き出しの除く範囲（`excluded.rs`）は**利用者を問わない**。消した滞在は全利用者の記録から除く | Q2（直し方の範囲）・C3 | `excluded.rs` の `erased_spans`（`user_id` で絞らない）・`testdb::unique_day` の注記 | 問わない。試験では、他の試験が消した滞在（st23 で 534 件、2000-01-16〜2026-10-07）が全試験の書き出しにかかる | 既存のテスト材料 | 3 |
| P8 | `excluded.rs` の重なりの判定は、候補の行 N × 消した時間 M の入れ子になっている | Q2 | `overlap_sql` を読む: 候補は `event_time <= max(end_at)` の全行（st23 で 11.9 万行）、行ごとに `erased_spans`（MATERIALIZED。索引なし）を `EXISTS` で舐める。次の記録の引きは `event_by_source_time (logical_source, event_time)` で、`user_id`・`device_id` は索引の外で絞る | 入れ子。試験の DB では同じ名前のソース（`c01-location` 2.5 万行・1,461 人）で次の記録の引きが他の利用者の行を読み飛ばす | 既存のテスト材料 | 3 |
| P9 | 製品の量でも遅い。主な費用は「次の記録の引き」で（**CH02 が直した。P14**）、利用者 1 人でも候補の 1 行ごとに同じソースの後ろの行を全部読む | Q2 | ST33 の測定（35 万行・座標つき 1%・消した滞在 1 件で書き出し 42.8 秒、`GET /export/summary` 6.5 秒で D13 の反転条件 3 秒を超過、座標つきの行 1 行約 8ms。`openspec/changes/archive/2026-10-17-st33-data-export/design.md:164,380-381`）・st23 の DB で次の記録の引きを `EXPLAIN`（`event_by_source_time` を緩い条件で引いた後に `event_time, id` で全部を並べ替えてから `LIMIT 1`）（review/deep.md R6） | 遅い。N だけで二乗に伸びる経路がある。加えて N × M の入れ子（P8）も残る。量を変えて測る試験 `export_volume_one_run_reports_time_and_memory`（`export_file_tests.rs:1449`、`ASHIATO_EXPORT_VOLUME_ROWS`）が既にある | 既存のテスト材料（ST33 の実測）と EXPLAIN | 4（35 万行の時間）/ 3（二乗の経路） |
| P10 | いま st23 の DB で同じ形の問い合わせが 23 分以上走っている | 依頼の実測の裏付け | `pg_stat_activity`（2026-10-24 questions の段） | 5 本が 23 分（`WITH erased_spans AS MATERIALIZED …`） | 実物 | 5 |
| P12 | 1 回の `cargo test` で数万行が入り、書き出しの試験はその回の全部の行を判定する | Q1（同じ回の中の干渉） | st23 の `core.event` を `ingest_time` の 5 分ごとに数えた（review/deep.md R1）。2 万行級を入れる試験 `deletion_tests.rs:265`・`stay_tests.rs:1231,1277`・`export_file_tests.rs:1449`。書き出しの試験は 116 本 | 1 回で 2.3〜4.4 万行・利用者 300 人余り。書き出しは利用者を問わず全行を読む（P7）ので、同じ回の書き出しの試験どうしが互いのデータを判定し合う | 実物 | 5 |
| P13 | `testdb::url()` から `*_tmp_*` の database を作る試験が 5 か所あり、途中で panic すると残る。`app_pool()` と `tests/server_startup.rs` は `ashiato` を使う | Q1・Q3 | `clock_tests.rs:40-56`・`feelings_tests.rs:16-31`・`stay_tests.rs:1322-1336`・`attributes_tests.rs:209-224`・`places_tests.rs:14-29`・`testdb.rs:108-110`・`tests/server_startup.rs:8-14,165-176`（review/deep.md R4）。st23 に `st16_tmp_…` が残っている | 残る。`app_pool()` だけ `ashiato` に向いたままだと移行の当たっていない database を叩く | 実物 | 5 |
| P11 | CI は毎回まっさらな DB で走る | Q3（再発止めの置き場） | `.github/workflows/ci.yml:62-69`（`services` でなく compose で立て、`db-roles.sh` を走らせる）・`:74`（`cargo test --workspace`）（review/deep.md R10） | まっさら。積もるのは手元の worktree だけで、CI の時間では再発を見られない。`db-roles.sh` が CI でも走るので P4（CREATEDB）は CI にも当てはまる | 既存のテスト材料 | 2 |
| P14 | 「次の記録の引き」の二乗は CH02 が直した。直した後も、候補の行 × 消した時間の入れ子は残る | Q2（直す範囲）・design D9〜D11 | CH02 の下流のブランチ（`origin/feat/ch02-export-next-record-window`、PR 本文まで済み）の `design.md` の「計測の記録」と `excluded.rs` を読む（spec の段 2026-10-24） | CH02 の後、120 万行・消した滞在 1 件で書き出し 146.5 秒・件数の数え 25.5 秒、無しで 97.9 秒・0.9 秒。行数 10 倍で時間 7.3〜7.9 倍、12 倍で 11.3〜11.5 倍（ほぼ線形）。25.5 秒の大半は窓の並べ替えと緯度経度の判定で、入れ子（M = 1）ではない（review/spec.md R12）。入れ子の費用は、候補を全部の行にした最悪の形でも (30 万, 25)・(60 万, 50)・(120 万, 100) で +0.65・+2.0・+7.3 秒（review/spec.md R1。使い捨ての PG 17）。`overlap_sql` の最後の `WHERE EXISTS (SELECT 1 FROM erased_spans s WHERE s.start_at <= w.width_end AND s.end_at >= w.event_time)` は残り、候補の行ごとに `erased_spans`（MATERIALIZED。索引なし）を舐める。questions の段の P9 と Q2 はこれを見落としていた | 既存のテスト材料（CH02 の実測） | 4（時間）/ 3（入れ子が残ること） |
| P15 | 新しく作る database にも、アプリの役割（`ashiato_app`）は繋げる | design D3（`app_pool()` を回の database へ） | `tools/db-roles.sh` に `REVOKE CONNECT` が無い（PostgreSQL の既定で `PUBLIC` が新しい database に `CONNECT` を持つ）。database 単位の権限は TEMPLATE で写らない | 繋げるはず。下流が `app_role_tests` を回の database で通して確かめる（tasks 2.4） | 既存のテスト材料 | 2 |
| P16 | 同じテンプレートから `CREATE DATABASE … TEMPLATE` を同時に打つと落ちうる（テンプレートに他の接続があると断る） | design D2（試験ごとの database の作り方） | PostgreSQL の文書（`CREATE DATABASE` の「source database が他の session に使われていてはならない」）https://www.postgresql.org/docs/17/sql-createdatabase.html | テンプレートへは移行の後に誰も繋がない。同時の作成は回の中で 1 本ずつにする（D2）。外れたら下流の tasks 2.3 が落ちる | 公式文書 | 2 |

`loss` のある決定は無い（試験のデータは誰の記録でもない。製品の書き出しの中身は変えない —— C2）。P9 の「次の記録の引き」の二乗は CH02 が直した（P14。spec の段で見つけ、Q2 を本人に渡す前に書き直した）。Q2 は、CH02 の後に残る候補の行 × 消した時間の入れ子を「この change で直す」を推奨にする。

## 聞かないで決めるもの（C）

- **C1** 試験の期待値・Scenario の印・試験の名前は変えない（依頼の「触らない」）
- **C2** `excluded.rs` を直すなら、書き出しの中身（どの記録を除くか）は 1 行も変えない。直す前の式と直した後の式の結果が同じことを、同じ材料で両方を走らせて突き合わせる試験で固定する
- **C3** 除く範囲を利用者で絞る（`user_id` を足す）ことはしない。製品の振る舞い（FR-50 の除く範囲）を変えることになり、change の範囲を越える
- **C4** 消せない表のトリガは外さず、`session_replication_role` も使わない（依頼の逐語）
- **C5** 本物の PostgreSQL のまま。worktree ごとの port の割り当て（`tools/ports.sh`）は触らない
- **C6** 使い捨ての database が走っている回のものかを、接続の数で判定しない（プールは試験ごとに作って捨てるので、走っている回でも接続が 0 本になる間がある。review/deep.md R3）。回の間ずっと持つ印（session の助言ロックか、名前に入れた pid の生死）で判定する。どれにするかは spec で決める
- **C7** `app_pool()` と `*_tmp_*` を作る 5 か所も、回の database と同じ片付けに載せる（`*_tmp_*` は始まりの片付けの対象にする）。`tests/server_startup.rs` は今までどおり `ashiato` に移行を当てる（移行は何度当てても同じ結果で、記録を積まない）
