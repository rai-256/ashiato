# ST41 の前提 —— 外の形式・振る舞い

**外のサービスの書き出しや端末の形式に依る決定は無い**（幅のある出来事を書く衛星はまだ 1 つも無く、送り手の形はこの Story が決める側）。
依るのは DB（PostgreSQL 17。`docker ps` の `postgres:17-alpine`）の範囲型の振る舞いだけで、問いを立てる前に本物で確かめた（2026-10-21）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| 終わりの列を足さずに範囲を作る**式の索引**（GiST、`tstzrange(event_time, coalesce(end_time, event_time), '[]')`）で、期間の重なり（`&&`）が索引で引ける | 期間に重なる記録を引く（完了の判定 2）。NFR-24 は索引の追加を変更に数えないので、範囲の列を別に持たなくてよい | 使い捨ての `postgres:17-alpine` に 20 万行を入れ、`EXPLAIN` を見る | `Bitmap Index Scan on e_range`（式の索引が当たる）。数も合う（75 件） | 実物（PostgreSQL 17） | 4 |
| 終わりが始まりより前の値で範囲を作ると、PostgreSQL は**エラーで落とす** | Q1（終わりが始まりより前の記録をどう扱うか）。素の式で索引にすると、その 1 行の INSERT が落ちる | 同上で `select tstzrange('2026-01-02','2026-01-01')` | `ERROR: range lower bound must be less than or equal to range upper bound` | 実物（PostgreSQL 17） | 4 |
| 始まりに寄せた式 `tstzrange(event_time, greatest(event_time, coalesce(end_time, event_time)), '[]')` なら、逆順の終わりを持つ行も INSERT でき、索引も当たる | Q1 の「送られたまま入れる」側が成り立つか | deep-review が使い捨ての PostgreSQL 17 に 52 万行で実行（review/deep.md R2） | 逆順の 1 行が `INSERT 0 1`、`Bitmap Index Scan` が当たる | 実物（PostgreSQL 17） | 4 |
| 本物の量（約 120 万行。CH02 の proposal の Why）で上の式の GiST 索引を作ると約 30 秒かかり、その間 `core.event` への書き込みが待つ（起動時の移行の中。`CONCURRENTLY` はトランザクションの中で使えない）。`IF NOT EXISTS` なので待つのは最初の起動の 1 回だけ | 移行の形（起動 1 回の取り込みの待ち。収集アプリの送信の仕組みは失敗を持ち続けて再送する） | 使い捨ての `postgres:17-alpine` に 120 万行（原文 300 字）を入れて `CREATE INDEX` を `\timing` で測る（2026-10-21） | 29.3 秒。終わりのある行だけの部分索引（`WHERE end_time IS NOT NULL`）なら、いまは 0 行で一瞬（点の記録の側は、`GET /events` が利用者で絞らず `event_time` を先頭に持つ索引も無いので、`source` なしの期間では全走査になる。期間なしの今の挙動からの退行ではなく、索引か利用者の絞り込みは ST45 の読み出しで決める。review/code.md R3）—— design の候補 | 実物（PostgreSQL 17） | 4 |
| 半開区間 `[)` で始まりと終わりが同じだと**空の範囲**になり、何にも重ならない。閉区間 `[]` なら点として重なる | Q2（境界の含み方）・終わりの無い記録（点）を重なりに入れるか | 同上で `tstzrange(t,t,'[)') && …` と `tstzrange(t,t,'[]') && …` | `[)` は `f`、`[]` は `t` | 実物（PostgreSQL 17） | 4 |
| 足したばかりの 3 列（すべて NULL）への CHECK を `NOT VALID` で足すと全行を読まず一瞬で終わり、以後の INSERT には効く。`NOT VALID` なしでも 120 万行で 1 秒かからない | 移行の CHECK（design D15）。起動時の移行の中で `core.event` の書き込みを待たせる時間 | 使い捨ての `postgres:17-alpine` に 120 万行（原文 300 字）を入れ、3 列を足してから `ADD CONSTRAINT … NOT VALID` と素の `ADD CONSTRAINT` を `\timing` で測り、揃わない行の INSERT を打つ（2026-10-22。spec レビュー R14） | `NOT VALID` 1.4 ms / 1.2 ms、素の検査つき 357.7 ms。揃わない行は `violates check constraint` で落ちる | 実物（PostgreSQL 17） | 4 |
