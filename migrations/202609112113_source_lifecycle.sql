-- 0007 登録簿の生涯（退役・引き継ぎ）と、収集開始日の引き直し
-- （ST02 / 深掘り 第 8 回 Q29・Q31、ST03 の差し戻し R56 / R64）。
-- 前進のみ。戻し手順は migrations/202609112113_source_lifecycle.down.sql に置く（**不可逆**）。

-- ------------------------------------------------------------------ 退役と引き継ぎ
--
-- **この 2 列は ST02 が作り、ST02 が読む**（2026-09-11 の判断。
-- `openspec/changes/st03-idempotent-ingest/design.md` の D7）。
-- 読む側が稼働記録（状態・途絶・分母・画面）で、ST03 は 1 度も読まない。
-- ST03 が作るのは `external_id_kind` だけで、そちらは移行 0008 に入る。

-- **真偽値にしない**（ST03 の R56。実測）。「退役したか」を真偽値で持つと、
-- 退役した瞬間に**退役より前に起きた本物の途絶まで遡って消える** ——
-- 過去の日を引くときに「いま退役しているか」しか見られないため。
-- 日付なら「その日に退役していたか」を日ごとに問えて、`collection_started_on` と対になる。
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS retired_on date;

-- 引き継ぎ元（FR-61 / 第 8 回 Q31）。**古い名前を分母から外し、新しい名前が窓を引き継ぐ**。
-- 自己参照の FK なので、鎖は登録簿の中で閉じる。
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS succeeds text;
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
     WHERE constraint_schema = 'core' AND constraint_name = 'source_succeeds_fkey'
  ) THEN
    ALTER TABLE core.source
      ADD CONSTRAINT source_succeeds_fkey
      FOREIGN KEY (succeeds) REFERENCES core.source(logical_source);
  END IF;
END $$;

-- **1 つの引き継ぎ元を 2 本が引き継げない**（review/code-r2.md の H-4）。
-- 一意索引が無いと、鎖をたどる側が辞書順で片方を**黙って捨て**（枝分かれ）、
-- 2 つの Must が同じ後継に解決すると**同じソースが 5 本中 2 本として二重に数えられる**（合流）。
-- どちらも「どの名前を数えたか」からは読み取れない。入口で塞ぐほうが安い。
--
-- **当てられなくても起動を止めない。** `migrate()` は起動のたびに全版を当てるので、
-- 既に枝分かれのある登録簿では索引が作れず、**サーバが起動しなくなる** ——
-- 収集が止まって記録が落ちるのは、このプロジェクトでいちばん高い代償。
-- 作れなかったことは WARNING で残し、**たどる側も分岐を決定的に畳んで警告を出す**
-- （`coverage::resolve_chain`）。入口と出口の両方で受ける。
DO $$
BEGIN
  CREATE UNIQUE INDEX IF NOT EXISTS source_succeeds_unique
    ON core.source (succeeds) WHERE succeeds IS NOT NULL;
EXCEPTION WHEN unique_violation THEN
  RAISE WARNING '引き継ぎ元が枝分かれしている行があるので source_succeeds_unique を作れない。登録簿を直すこと';
END $$;

-- **自分自身を引き継ぎ元にできない。** 鎖をたどる側が無限に回る
-- （たどる側にも深さの上限を置いてあるが、入口で塞ぐほうが安い）。
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.table_constraints
     WHERE constraint_schema = 'core' AND constraint_name = 'source_succeeds_not_self'
  ) THEN
    ALTER TABLE core.source
      ADD CONSTRAINT source_succeeds_not_self CHECK (succeeds IS DISTINCT FROM logical_source);
  END IF;
END $$;

-- ------------------------------------------------------------------ 記録を日で引く索引
--
-- **稼働状況の出どころが `core.coverage` から `core.event` へ移った**（tasks 15.5 / R57）。
-- 行数が「日数 × ソース数」で頭打ちだった表から、**記録の数で伸びる表**へ移ったのに
-- 索引が無く、`EXPLAIN ANALYZE` は `Seq Scan on event` を出していた
-- （review/code-r2.md の R9 / I4）。位置は 60 秒間隔で年 50 万行を超え、
-- 画面 1 回で 10 回以上の全走査になる。**壊れて見えないまま遅くなる型。**
CREATE INDEX IF NOT EXISTS event_by_source_time
  ON core.event (logical_source, event_time);

-- REPAIR-BEGIN
-- ------------------------------------------------------------------ 収集開始日の引き直し
--
-- **この印の間だけを検査が抜き出して当てる**（`migration_repairs_polluted_started_on`）。
-- ファイル全体を当てると `ALTER TABLE core.source` と `CREATE INDEX ... ON core.event` が
-- 2 つの表に強い錠を掛け、**並んで走っている記録の挿入と deadlock する**（実測 40P01）。
-- 印で切り出せば、検査は本物の SQL の逐語を当てたまま、錠は登録簿だけで済む ——
-- 閾値をこのファイルから消せば検査が落ちる、という性質は保たれる。
--
-- **第 8 回 Q29 と第 9 回 Q32 の本体はここ。** 本人の答えは「受けるが、収集開始日の
-- 計算から外す」（Q29）で、閾値は**登録簿に行ができた日（`registered_at`）より前**。
-- そして **閾値が掛かるのは生存信号だけ**（Q32）——
-- 生存信号の `emitted_at` は端末の時計そのものだが、記録の `event_time` は
-- **出来事が起きた時刻**で、古いことに正当な理由がある（端末にある写真は撮影時刻が
-- 何年も前、ブラウザ履歴は導入時点で過去ぶんが取れる、Takeout 系は過去 1 年ぶんを
-- まとめて流し込む）。記録にも掛けていたときは、**登録簿に行を足してから過去ぶんを
-- 流し込む運用で、記録が何万件あっても全日が⑦「導入前」**になった（実測）。
--
-- 外す条件を受け口に足すだけでは足りない —— 収集開始日は `least()` でしか動かない
-- （前にしか動かない）ので、**一度 1999 年に落ちた行は正しい日を送り直しても戻らない**。
-- 端末の時計が 27 年戻った信号 1 件で、成功条件 1 が「確定・未達」で固まる（実測）。
-- **記録も信号も消さない**（本人の答えの前半）—— 消すのは収集開始日への寄与だけ。
UPDATE core.source s
   SET collection_started_on = f.first_day
  FROM (
    SELECT src.logical_source,
           min(d.day) AS first_day
      FROM core.source src
      JOIN LATERAL (
        -- **記録に閾値は掛からない**（第 9 回 Q32）
        SELECT (e.event_time AT TIME ZONE 'Asia/Tokyo')::date AS day
          FROM core.event e
         WHERE e.logical_source = src.logical_source
        UNION ALL
        SELECT (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date
          FROM core.heartbeat h
         WHERE h.logical_source = src.logical_source
           AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date
               >= (src.registered_at AT TIME ZONE 'Asia/Tokyo')::date
      ) d ON true
     GROUP BY src.logical_source
  ) f
 WHERE s.logical_source = f.logical_source
   AND (
     -- (a) まだ埋まっていない
     s.collection_started_on IS NULL
     -- (b) **前へ動かす**（第 7 回 Q26。後から古い記録が届いたとき）
     OR s.collection_started_on > f.first_day
     -- (c) **汚れているときだけ後ろへ動かす**（第 8 回 Q29 の修復）。
     --
     -- 「汚れている」を**狭く定める** —— いまの開始日が
     --   1. 閾値より前に発信された生存信号の日**ちょうど**にあり、かつ
     --   2. その日に記録が 1 件も無い
     -- とき。**その 2 つが揃うのは、その信号が開始日を書いたときだけ。**
     --
     -- 「登録より前」だけでは判定にならない（第 9 回 Q32）—— 過去ぶんを流し込んだ
     -- ソースの開始日は**正しく**登録より前になる。
     -- 「支える記録が無い」だけでも足りない —— **記録を破棄した後**の再起動でそれが真になり、
     -- 開始日が前へ動いて⑤「破棄された期間」が⑦「導入前」に化ける（C-3）。
     -- 信号の日ちょうどであることまで見れば、破棄では当たらない。
     OR (
       EXISTS (
         SELECT 1 FROM core.heartbeat h
          WHERE h.logical_source = s.logical_source
            AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date = s.collection_started_on
            AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date
                < (s.registered_at AT TIME ZONE 'Asia/Tokyo')::date
       )
       AND NOT EXISTS (
         SELECT 1 FROM core.event e
          WHERE e.logical_source = s.logical_source
            AND (e.event_time AT TIME ZONE 'Asia/Tokyo')::date = s.collection_started_on
       )
     )
   );

-- **汚れた値しか無いソースは NULL に戻す**（第 5 回 Q22「1 件も届いていないソースは開始していない」）。
--
-- **`collection_started_on` が汚れている行だけを対象にする**（review/code-r2.md の C-3）。
-- この移行は版管理表を持たない `migrate()` が**起動のたびに当て直す**ので、
-- 条件を付けないと「記録を破棄した後に再起動する」だけで正規の収集開始日が消える ——
-- ⑤「破棄された期間」が⑦「導入前」に化け、`coverage_span` の行だけが残る。
-- 書き込み経路（`touch_started_on`）も 0005 も「前にしか動かさない」ので、
-- **後ろへ動かしてよいのは汚れを直すときだけ**。
UPDATE core.source s
   SET collection_started_on = NULL
 WHERE s.collection_started_on IS NOT NULL
   -- 上と同じ「汚れている」の定め方（信号の日ちょうど・その日に記録が無い）
   AND EXISTS (
     SELECT 1 FROM core.heartbeat h
      WHERE h.logical_source = s.logical_source
        AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date = s.collection_started_on
        AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date
            < (s.registered_at AT TIME ZONE 'Asia/Tokyo')::date
   )
   -- **記録は 1 件でもあれば開始日を作る**（第 9 回 Q32。閾値は掛からない）
   AND NOT EXISTS (
     SELECT 1 FROM core.event e
      WHERE e.logical_source = s.logical_source
   )
   AND NOT EXISTS (
     SELECT 1 FROM core.heartbeat h
      WHERE h.logical_source = s.logical_source
        AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date
            >= (s.registered_at AT TIME ZONE 'Asia/Tokyo')::date
   );
-- REPAIR-END
