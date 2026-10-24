# CH04 試験用 DB に溜まるデータで試験が遅くなるのを止める（上流）

change: `openspec/changes/ch04-test-db-growth/`。新しい capability `test-database`（ADDED 6・Scenario 22 本）と、`data-durability` への ADDED 1（Scenario 3 本）。
依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH04.md`。**`requires: [CH02]`**（書き出しの直しは CH02 が作り替えた `excluded.rs` の上に積む）。

> **第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた**（2026-10-20 の流れ）。下の Q1〜Q4 は**本人の答えではなく推奨**で、`deep.md` は `（答え待ち・推奨: …）` のまま。
> 答えが推奨と違えば `spec --answers` が仕様に写し直す。全問が B（仮でよい）。
>
> **Q2 は spec の段で書き直した（本人に渡す前）。** questions の段の Q2 は、CH02（同じ `excluded.rs` の「次の記録の引き」の二乗を窓関数で直した change。下流は済み）を見落としていた。
> CH02 の後に残るのは「候補の行 × 消した時間」の入れ子で、さらに spec-review R1 の実測で、時間の比（2.5 倍）は直す前の式でも通り判定にならないと分かったので、判定を「消した時間を読んだ行の数」に替えた（`premises.md` P14、`review/spec.md` R1・R12）。

## 深掘り（第 1 回。推奨で書いた）

| Q | 推奨（仕様はこれで書いた） | どこに効くか |
|---|---|---|
| Q1 試験のデータの隔離と片付け | 回ごとの database ＋ 書き出しの試験だけ試験ごとの database。片付けは次の回の始まりに `DROP DATABASE`（トリガは撃たない） | test-database の Requirement 1〜4、design D1〜D6、tasks 1〜3・5・6 |
| Q2 書き出しの問い合わせの残りの遅さ | この change で N × M の入れ子を直す。判定は「N と M を一緒に 4 倍にして消した時間を読む量が 8 倍未満（直す前は 8 倍以上）」。時間は 120 万行まで測って残し、合否にしない。件数の数え方（ST33 D13）は替えない | data-durability の ADDED、design D9〜D11・D14、tasks 8・9 |
| Q3 再発を機械で止める | 残りを見る台本 `tools/check-test-db-growth.sh`（2 回走らせて前の回が残らない。CI の rust job）＋ 試験の接続に `statement_timeout`（既定 60 秒） | test-database の Requirement 5・6、design D7・D8、tasks 4・7 |
| Q4 今ある `ashiato` のデータと volume | この change では何もしない | design Non-Goals |

聞かずに決めたもの（`premises.md` の C1〜C7）: 試験の期待値・印・名前は変えない / 除く記録は 1 件も変えない（突き合わせの試験）/ 除く範囲を利用者で絞らない / トリガは外さず黙らせない /
本物の PostgreSQL と port は触らない / 走っているかを接続の数で判定しない（助言ロック。D1）/ `app_pool()` と `*_tmp_*` も同じ片付けに載せる（D3・D5）。

## 仮決め（B）と反転条件（design の（仮））

- **D1（仮）** 回の id `ashiato_run_<12 桁>` を頭に持つ 4 種の database。走っている印は `postgres` database で持つ session の助言ロック（専用スレッドの runtime）。反転条件: 専用の接続が切れる例が出たら、その回の試験を落とす形に
- **D2（仮）** 片付けと作成は `pg_advisory_lock(4820252)` で 1 本ずつ、試験ごとの写しは過程の中で 1 本ずつ。反転条件: 写す 1 本が 1 秒を超えたら `STRATEGY FILE_COPY`、それでも超えたら Q1 を本人に返す
- **D4（仮）** 書き出しの試験は `testdb::isolated()`（Drop で消す。Drop は panic しない）。反転条件: 書き出し以外にも同じ回の干渉で遅い群があれば広げる
- **D5（仮）** 使い捨ては `testdb::scratch()`。`--lint` が足場の外の `CREATE DATABASE` と書き出しの試験の `testdb::pool()` を落とす（`db-ok: <理由>` で通す）。反転条件: 正当な例が増えたら関数の頭で選ぶ形に
- **D7（仮）** `statement_timeout` の既定 60 秒、`ASHIATO_TEST_STATEMENT_TIMEOUT`（`0` で上限なし）。反転条件: まっさらな回で正当な試験が落ちたら、その最長の 2 倍に
- **D8（仮）** 台本は一覧を 3 回撮って比べる。CI は `ch04_` の絞り込みで 2 回。反転条件: CI で psql が読めなければ容器の外から読む
- **D9（仮）** 重なりは、整った消した時間を `range_agg` の multirange にして `&&`（幅の範囲は CASE の中でだけ作る）、整っていないものは今の `EXISTS`。反転条件: 読んだ行の数が 8 倍の線を越えたら直す・直せなければ Q2 を本人に返す
- **D12（仮）** `docs/changes/CH04.md` の `capability` は空のまま。反転条件: 同じ Requirement の名前や `test-database` を作る上流が入ったら書く
- **D13（仮）** CH03 の `test-hygiene` と別の capability `test-database`。反転条件: 両方の archive の後にまとめたくなれば移す change を立てる
- **D14（仮）** 時間は `tools/export-scaling.sh` で D9 の前と後を測って残す（CI では走らせない）。反転条件: 後が前より遅ければ D9 を直す

仮でない決定: D3（`pool()`・`app_pool()`・`url()` の形は変えず繋ぎ先だけ替える）・D6（片付けの規則）・D10（移行は足さない）・D11（最後の条件だけを戻した判定と突き合わせる）・D15（文書）。

## capability

`test-database` を新しく立てた（CH03 の `test-hygiene` は未 archive で、同じ新しい capability に 2 つの change が Purpose を積むのを避けた。D13）。
`data-durability` には別の名前の Requirement を ADDED で足し、CH02 の delta を MODIFIED しない（CH02 の spec の本文を写さず、gate・archive の順に依らない）。capability の前倒しは無い。

## tasks

10 Task・30 項目。Scenario 25 本は全部どこかの Task が名指しする。画面は持たないので e2e と人間の確認待ちは無い。製品で変わるのは書き出しの除く範囲の判定の速さだけ（中身は変わらない）。

## レビュー

- `review/deep.md`（questions の段の deep-review）: 10 件。処置済み（R7 の処置に、spec の段で Q2 を書き直したことを追記）
- `review/spec.md`（spec-review）: 17 件。fixed 15（うち仮 3: R8 → D4・R10 → D5・R11 → D14）/ escalated 2（R1・R12。どちらも Q2 の前提と判定で、本人に渡す前の第 1 回の Q2 を書き直して本人の答えに委ねた）/ rejected 0。
  書き直した Q2 は spec-review を通したので、deep-review にはもう一度通していない
- 走っている変更との重なり: `testdb.rs`（st09・st13。関数を足すだけで形は衝突しない）・`excluded.rs`（CH02 の上に積む。st09・st41 とは行が重ならない）・書き出しの試験のファイル（CH03・st09・st13・st23・st29・st41）・`ci.yml`（CH03・st09）。
  差し戻しはしない。後から merge する側が rebase で追従し、CH04 の後に入る書き出しの試験の `testdb::pool()` は `--lint` が止める

🤖 Generated with [Claude Code](https://claude.com/claude-code)
