-- 0007 登録簿の生涯（退役・引き継ぎ）と、収集開始日の引き直し
-- （ST02 / 深掘り 第 8 回 Q29・Q31、ST03 の差し戻し R56 / R64）。
-- 前進のみ。戻し手順は migrations/0007_source_lifecycle.down.sql に置く（**不可逆**）。

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

-- ------------------------------------------------------------------ 収集開始日の引き直し
--
-- **第 8 回 Q29 の本体はここ。** 本人の答えは「受けるが、収集開始日の計算から外す」で、
-- 閾値は**登録簿に行ができた日（`registered_at`）より前**。
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
        SELECT (e.event_time AT TIME ZONE 'Asia/Tokyo')::date AS day
          FROM core.event e
         WHERE e.logical_source = src.logical_source
           AND (e.event_time AT TIME ZONE 'Asia/Tokyo')::date
               >= (src.registered_at AT TIME ZONE 'Asia/Tokyo')::date
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
   AND s.collection_started_on IS DISTINCT FROM f.first_day;

-- **1 件も残らないソースは NULL に戻す**（第 5 回 Q22「1 件も届いていないソースは開始していない」）。
-- 閾値より前の信号しか無いソースは、ここで「まだ開始していない」へ戻る。
UPDATE core.source s
   SET collection_started_on = NULL
 WHERE s.collection_started_on IS NOT NULL
   AND NOT EXISTS (
     SELECT 1 FROM core.event e
      WHERE e.logical_source = s.logical_source
        AND (e.event_time AT TIME ZONE 'Asia/Tokyo')::date
            >= (s.registered_at AT TIME ZONE 'Asia/Tokyo')::date
   )
   AND NOT EXISTS (
     SELECT 1 FROM core.heartbeat h
      WHERE h.logical_source = s.logical_source
        AND (h.emitted_at AT TIME ZONE 'Asia/Tokyo')::date
            >= (s.registered_at AT TIME ZONE 'Asia/Tokyo')::date
   );
