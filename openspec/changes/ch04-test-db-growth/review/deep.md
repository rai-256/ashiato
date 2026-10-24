# CH04 deep-questions の検査（deep-review）

検査の対象: `openspec/changes/ch04-test-db-growth/deep-questions.json`（Q1〜Q4、全部 open / B）と `premises.md`（P1〜P11・C1〜C5）。
手順 1〜8 は問いの一覧を見る前に、依頼の逐語（`docs/changes/CH04.md`）とコード・各 worktree の DB（読むだけの問い合わせ）から先にやり直した。
試験は走らせていない（機械の負荷が 16 コアに 36 のさなかで、`cargo test` の全体を足すと並走する Story を遅くする）。代わりに既存の DB の行を時刻で分けて、1 回ぶんの量を見積もった。

## 手順ごとの記録

- 手順 1（衝突）: 依頼 1 の「同じ回の他の試験も」と、ST33 で人間が決めた「書き出しの除く範囲は利用者を問わない」（Q7、正典 data-durability:209,231）がぶつかる。利用者で分けても、同じ回の書き出しの試験はその回の全部の行を読む → R1
- 手順 2（扉の幅）: 「触らない」の 3 つ（期待値・Scenario の印 / 本物の PostgreSQL / port の割り当て）は C1・C5 で受けている。テンプレートから database を作る案でも本物の PostgreSQL のままなので、幅は残っていない。Q1 案 2 の書き方の誤りは R2
- 手順 3（新たに立つ一方通行）: 該当なし（確かめた範囲: 試験の DB の行は誰の記録でもない。`DROP DATABASE` はトリガの定義を変えない。`excluded.rs` の式を直すなら C2 が結果の突き合わせで除く記録を固定し、C3 が利用者で絞る向きを止めている。C3 の根拠は ST33 の Q7（A・exported）で、`openspec/changes/archive/2026-10-17-st33-data-export/deep.md:248-252`）。全問 B の分類は妥当
- 手順 4（日常）: agent の待ち時間（Q3）。Docker の容量は、P5 の誤りで Q4 の「作り直せば消える」が成り立たない → R8
- 手順 5（既存コード）: Q1 推奨で壊れる箇所と、片付けの対象から漏れる箇所 → R3・R4。製品の問い合わせの前提の誤り → R6
- 手順 6（過去の取りこぼし）: 「検証コマンドが成立しない」（11 件）と「DB を共有する連鎖の検証」（ST33 Q10）が Q3 推奨 (a) に当たる → R5。「見積もり・根拠を実物で測っていない」が Q1・Q2 に当たる → R1・R6。「初回・大量の場合」は R1 で扱う
- 手順 7（族の網羅）: 「試験が残すもの」の族の兄弟は 4 つある。回ごとの database（Q1）・試験の途中で落ちて残る `*_tmp_*` の database（R4）・同じ worktree で並んで走る回（R3）・Docker の匿名 volume（R8）。Q1 が拾っているのは最初の 1 つだけ
- 手順 8（前提の確かさ）: `loss` のある決定が無いので、「確かさ 4 未満の前提に loss が載る」には当たらない。ただ確かさ 2 の P5・P6・P11 は結果が誤り（R8・R9・R10）、確かさ 1 の P9 も誤り（R6）

---

## R1. 依頼 1 の「同じ回の他の試験も」が、問いでも C でも扱われていない。Q1 推奨が断つのは回をまたいで積もる分だけ
- 成果物: openspec/changes/ch04-test-db-growth/deep-questions.json
- 根拠: docs/changes/CH04.md:22（「後の試験（同じ回の他の試験も、次の回も）が遅くならないようにする」）/ Q1 の context の「1 回ぶんのデータしか積もらない」は量を書いていない / st23 の DB で `core.event` を `ingest_time` の 5 分ごとに数えた結果、一度に入った塊は 2.3 万〜4.4 万行・利用者 310〜380 人（00:05 は 43,772 行・379 人、00:25 は 23,110 行・327 人、02:25 は 23,062 行・310 人）。verify の 4.3 万行（CH04.md:17）と同じ桁 / 1 回で大量に入れる試験: `deletion_tests.rs:265`（20,001 行）・`stay_tests.rs:1231,1277`（各 20,160 行）・`export_file_tests.rs:1449-1470`（`c02-window` を 2 万行。注記に「座標を持つ行は共有の DB に残り、以後のすべての書き出しが判定し直す」）/ 書き出しは利用者を問わず全部の行を読む（`openspec/specs/data-durability/spec.md:209,231`、`export/dump.rs:243-309`）/ 書き出しの試験は 116 本（`export_*tests.rs` の `#[tokio::test]` 22+43+36+3+12）
- kind: conflict
- 提案: Q1 の context に「1 回ぶんの量（数万行）の DB で、`export_` の試験の最長が何秒か」を測った数字を置く。測らないなら「同じ回の中の干渉をどこまで許すか」を Q1 の選択肢の軸に足す（例: 書き出しの試験 116 本だけはテンプレートから試験ごとの database を作る）。それも無いまま推奨を採るなら、「同じ回は対象外」と C に書き、依頼 1 をどこまで満たすかを人間に見せる
- 処置: escalated —— 同じ回の干渉をどこまで断つかを Q1（B）の選択肢の軸にして本人に見せる。Q1 の context に 1 回ぶんの量（P12）を置き、推奨を「回ごとの database ＋ 書き出しの試験だけ試験ごとの database（テンプレートから写す）」に替えた。「回ごとだけ」は同じ回の干渉が残ると書いた別の選択肢にした。premises.md に P12 を足した

## R2. Q1 案 2（試験ごとの database）の detail が、版の数と作り方の両方を誤っていて、案が不当に不利に見える
- 成果物: openspec/changes/ch04-test-db-growth/deep-questions.json
- 根拠: `grep -vc '^#\|^$' migrations/ORDER` = 29（detail は「移行（57 版）」。57 は ORDER の行数で、注記の行を含む）/ 試験ごとに移行を当て直す必要は無い。移行を当てた database を 1 つ作れば、`CREATE DATABASE … TEMPLATE <それ>` で写せる（テンプレートは写す間だれも繋がっていない必要がある）/ 「大幅に遅くなる見込み」は測っていない（premises.md に行が無い）
- kind: premise
- 提案: detail を「移行を当てたテンプレートから写す」形に直し、1 本あたりにかかる時間を測って書く。R1 の「同じ回」を満たすのはこの案（か、書き出しの試験だけに絞った案）なので、比べられる形で並べる
- 処置: fixed deep-questions.json —— 版の数を 29 に直し、試験ごとの案をテンプレートから写す形に書き直した。「大幅に遅くなる」は消し、1 本あたりの時間は spec の段で測ると書いた

## R3. Q1 推奨の「誰も繋がっていない `ashiato_test_*` を消す」では、走っている別の回の database を消さないことを保証できない
- 成果物: openspec/changes/ch04-test-db-growth/deep-questions.json
- 根拠: `crates/server/src/testdb.rs:74-77`（プールは試験ごとに作って捨てる）。1 つの回の中でも、DB を使う試験の合間・終わり際・`cargo test <名前>` で DB を使わない試験だけが走っている間は、繋がっている接続が 0 本になる。その間に同じ worktree の別の回（agent が背景で走らせた絞り込みの試験など）が始まると、`WITH (FORCE)` 無しの `DROP DATABASE` でも通ってしまい、走っている回の残りの試験が「database が無い」で落ちる。逆に、殺された回のバックエンドは問い合わせが終わるまで繋がったまま残り、その database は消えない（P10 の 23 分走っていた 5 本は、pkill で殺された回の残りでありうる）
- kind: premise
- 提案: Q1 の推奨の detail と context から「走っている別の回の database は消さない」を事実のように書くのをやめる。生きているかを接続の数以外で判定すること（回の間ずっと持つ session の錠、名前に pid を入れて生死を見る、など。どれにするかは spec でよい）を C として足す
- 処置: fixed premises.md —— C6 を足した（走っているかを接続の数で判定しない。回の間ずっと持つ印で判定し、どれにするかは spec で決める）。Q1 の推奨の detail から「走っている別の回は消さない」を事実のように書くのをやめ、context は C6 を指す

## R4. Q1 推奨で振る舞いが変わる箇所と、片付けの対象から漏れるものが context に無い
- 成果物: openspec/changes/ch04-test-db-growth/deep-questions.json
- 根拠: (1) `testdb.rs:108-110` の `app_pool()` は `DATABASE_URL` の database（`ashiato`）に繋ぐ。`pool()` だけを使い捨ての database に替えると、役割の試験（`app_role_tests.rs:38,73,157`）は移行の当たっていない `ashiato` を叩く。smoke の `down -v` の後なら表が無くて落ちる / (2) `clock_tests.rs:40-56`・`feelings_tests.rs:16-31`・`stay_tests.rs:1322-1336`・`attributes_tests.rs:209-224`・`places_tests.rs:14-29` は `testdb::url()` の末尾の `/<db>` を書き換えて `*_tmp_*` を作る。DROP は返り値の future を待ったときだけ走るので、試験が途中で panic すると残る。実物: st23 のコンテナに `st16_tmp_4782f0f4c6bb4b4d8ee6a99c612b26bc` が残っている（`docker exec ashiato2-st23-db-1 psql -U ashiato -d postgres -tAc "select datname from pg_database"`）。Q1 推奨が消すのは `ashiato_test_*` だけ / (3) `crates/server/tests/server_startup.rs:8-14,165-176,211-214` は `testdb` を通らずに環境の URL で `ashiato` へ `migrate` を当て、サーバを起こす。この変更の後も `ashiato` を使い続ける
- kind: technical
- 提案: Q1 の context に (1)〜(3) を並べ、推奨の detail に「`app_pool` と `*_tmp_*` を作る 5 か所も使い捨ての database に向ける。始まりの片付けは `*_tmp_*` も対象にする」を足すか、C に置く。(3) を `ashiato` のまま残すかは、R5 の検査の作り方と一緒に決める
- 処置: fixed premises.md —— P13 と C7 を足した（`app_pool()` と `*_tmp_*` の 5 か所も同じ片付けに載せ、始まりの片付けの対象にする。`server_startup.rs` は `ashiato` のまま）。Q1 の推奨の detail に `*_tmp_*` を足した

## R5. Q3 推奨 (a) の検査が、Q1 推奨と CI のどちらと組み合わせても成り立たない
- 成果物: openspec/changes/ch04-test-db-growth/deep-questions.json
- 根拠: (1) Q1 の context の「Rust の試験には全体の終わりに必ず走る後片付けが無い。片付けは次の回の始まりに」に従うと、`cargo test` の直後にはその回の database が必ず 1 つ残る。(a) の「使い捨ての database が残っていない」はいつ走らせても落ちる / (2) 「`ashiato` の行数が前後で変わらない」: CI では `testdb` が `ashiato` に移行を当てなくなるので、`cargo test` の前の `ashiato` には表が無い（数える問い合わせ自体が落ちる）。`cargo test --workspace` の中で `server_startup.rs:165-176` が `ashiato` に `migrate` を当てると、`core.source` に既定の行が入る（9 つの移行が `INSERT INTO core.source`: `grep -ln "INSERT INTO core.source" migrations/*.sql`）。だから前と後で必ず変わる / 過去の取りこぼしの「検証コマンドが成立しない」（11 件）・ST33 Q10「DB を共有する連鎖の検証を凍結前に順に走らせていない」と同じ型
- kind: technical
- 提案: (a) の判定を Q1 推奨の作りと合う形に書き直す（例:「前の回の database が残っていない・残っているのはこの回の 1 つだけ」「`ashiato` は数えない、または `server_startup` の移行の後を基準にする」）。spec の段で CI と手元の両方で空走させることを tasks の条件にする
- 処置: fixed deep-questions.json —— Q3 の推奨 (a) を「2 回続けて走らせた後に、最後の回の分だけが残り、前の回と `*_tmp_*` が残っていない」に替え、`ashiato` の行数は使わないと context に書いた

## R6. P9 の「未測定」「製品は利用者 1 人なので次の記録の引きは素直なはず」は誤り。測った数字が既にあり、引きは利用者 1 人でも行ごとに O(N)
- 成果物: openspec/changes/ch04-test-db-growth/premises.md（P8・P9）と deep-questions.json（Q2 の context）
- 根拠: (1) ST33 で 35 万行（`c02-window`・座標つき 1%・消した滞在 1 件、使い捨ての database）を測ってある。書き出しは 42.8〜43.0 秒、`GET /export/summary` は 1 回 6.5 秒で、D13 の反転条件（3 秒）を超えている（`openspec/changes/archive/2026-10-17-st33-data-export/design.md:380-381`・`review/code.md:39`）。座標つきの行は 1 行約 8ms（同 `design.md:164`）。M=1 でこの時間なので、主な費用は N×M の入れ子ではなく、次の記録の引きにある / (2) 次の記録の引きの計画を st23 の DB で `EXPLAIN` した結果: `Bitmap Index Scan on event_by_source_time`（`event_time >= x` の緩い条件）→ `Bitmap Heap Scan` → `Sort Key: event_time, id` → `Limit`。`id` が索引に無いので並べ替えが要り、`LIMIT 1` でも同じソースの後ろの行を全部読む。利用者が 1 人でも、候補の 1 行ごとに O(N) かかる / (3) 量を変えて測る試験は既にある: `export_file_tests.rs:1449` の `export_volume_one_run_reports_time_and_memory`（`ASHIATO_EXPORT_VOLUME_ROWS`）
- kind: premise
- 提案: P9 を「ST33 で 35 万行を測った（確かさ 4）。N だけで二乗に伸びる経路がある（EXPLAIN、確かさ 3）」に直し、Q2 の context に (1)(2) を書く。Q2 推奨の「測る台本」は (3) の試験を広げる形にする（消した滞在の数 M を変えられるようにする）と書く
- 処置: fixed premises.md —— P9 を ST33 の実測（確かさ 4）と EXPLAIN（確かさ 3）に書き直し、Q2 の context に測った数字と次の記録の引きの経路を書いた。Q2 の推奨を「この change で直す」に替え、既存の量の試験を広げる形にした

## R7. Q2 推奨の判定（N か M を倍にして 3 倍を超えたら二乗）では、Q2 の context が恐れている N×M の伸びを捕まえられない
- 成果物: openspec/changes/ch04-test-db-growth/deep-questions.json
- 根拠: Q2 の context は「製品では候補が約 120 万行、消した時間は年とともに増えるので、両方が育つと二乗に近く伸びる」と書く。一方 N×M の入れ子は、N だけ・M だけを倍にしても時間は 2 倍にしかならず、推奨の判定では「素直」と出て直さない。N と M はどちらも使った年数に比例して増える（`design.md:158` の「消した滞在の数は数十件の桁、記録は約 120 万行」）
- kind: technical
- 提案: 判定を「N と M を一緒に倍にする（年数を倍にするのと同じ）」か、「何年分の量で何秒を超えたら直す」という量と時間の閾値に替える。D13 の反転条件（3 秒）は 35 万行で既に超えている（R6）ので、閾値を新しく決めるなら、その判定とどう繋がるかも書く
- 処置: fixed deep-questions.json —— Q2 の判定を「N と M を一緒に倍にして 2.5 倍以内」と「120 万行・M=100 で件数の数えが 3 秒（ST33 の反転条件）以内」に替えた

## R8. P5「DB のコンテナは volume を持たない」は誤り。Q4 の「作り直せば空になる」「容量は作り直したときに消える」は作り直し方によって成り立たない
- 成果物: openspec/changes/ch04-test-db-growth/premises.md（P5）と deep-questions.json（Q4 の context と案 1 の detail）
- 根拠: `docker inspect ashiato2-st23-db-1 --format '{{json .Mounts}}'` → `/var/lib/postgresql/data` に匿名 volume が付いている（`docker image inspect postgres:17-alpine` の `Config.Volumes` が宣言している。compose に `volumes:` が無くても付く）。`docker compose up --force-recreate` は匿名 volume を新しいコンテナへ引き継ぐので中身が残り、空になるのは `down -v` か `up -V` のときだけ。`down` に `-v` を付けなかった回の volume が積もっている: `docker volume ls -qf dangling=true | wc -l` = 62
- kind: premise
- 提案: P5 を直す。Q4 の context と案 3 の detail の「作り直す」を `docker compose down -v`（か `up -V`）と書く。案 1 の「容量は作り直したときに消える」を、積もっている匿名 volume の扱いと合わせて書き直す
- 処置: fixed premises.md —— P5 を匿名 volume に直した（確かさ 5）。Q4 の context と選択肢を `down -v` と使われていない volume 62 個の扱いに書き直した

## R9. P6「試験のデータも smoke の database に積もる」は誤り。smoke は始まりと終わりにコンテナを volume ごと捨てる
- 成果物: openspec/changes/ch04-test-db-growth/premises.md（P6）と deep-questions.json（Q4 の context）
- 根拠: `tools/smoke.sh:36`（始まりに `docker compose down -v`）・`tools/smoke.sh:29`（trap で終わりにも `down -v`）。smoke は試験のデータを見ずに消す。同じ worktree で走っている `cargo test` も、Q1 推奨の使い捨ての database も一緒に消える
- kind: premise
- 提案: P6 を「smoke は同じコンテナを使うが、始まりと終わりに volume ごと捨てる」に直す。Q4 の「縦串は引き続き `ashiato` を使う」の後に、smoke を走らせた worktree では積もった分が既に消えていることを足す
- 処置: fixed premises.md —— P6 を「smoke は始まりと終わりに volume ごと捨てる」に直し、Q4 の context に足した

## R10. P11 の「service の DB」は誤り。CI の DB は compose で立てている（結論の「毎回まっさら」は正しい）
- 成果物: openspec/changes/ch04-test-db-growth/premises.md（P11）
- 根拠: `.github/workflows/ci.yml:62-63`（「DB は `services` でなく compose で立てる」）・`ci.yml:67-69`（`ci-db-env.sh` → `docker compose up -d --wait db` → `db-roles.sh`）。CI でも所有者は `db-roles.sh` で `CREATEDB` を持つので、Q1 推奨は CI でも動く
- kind: premise
- 提案: P11 の「確かめ方」と「結果」を compose に直す。`db-roles.sh` が CI でも走るので P4 は CI にも当てはまる、と書き足す
- 処置: fixed premises.md —— P11 を compose に直し、P4 が CI にも当てはまることを書き足した
