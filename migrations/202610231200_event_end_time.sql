-- ST41: 幅のある出来事の終わりの時刻（design D1 / D5 / D10 / D11 / D15）。
-- 2 回当てても落ちない形（`migrate()` は起動のたびに全版を当てる）。

-- ------------------------------------------------------------------ 1. 3 列と CHECK（D1）
ALTER TABLE core.event         ADD COLUMN IF NOT EXISTS end_time          timestamptz;
ALTER TABLE core.event         ADD COLUMN IF NOT EXISTS end_tz_offset_min integer;
ALTER TABLE core.event         ADD COLUMN IF NOT EXISTS end_tz_id         text;
ALTER TABLE core.event_version ADD COLUMN IF NOT EXISTS end_time          timestamptz;
ALTER TABLE core.event_version ADD COLUMN IF NOT EXISTS end_tz_offset_min integer;
ALTER TABLE core.event_version ADD COLUMN IF NOT EXISTS end_tz_id         text;

-- **NOT VALID で足す。** 既存の行は足したばかりの 3 列がすべて NULL で条件を満たすことが自明なので、
-- 全行を読む検査は要らない（以後の INSERT / UPDATE には効く）。VALIDATE は移行の中では打たない（D15 / spec レビュー R14）。
DO $mig$
DECLARE
  t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['event', 'event_version'] LOOP
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = ('core.' || t)::regclass AND conname = t || '_end_together') THEN
      EXECUTE format(
        'ALTER TABLE core.%I ADD CONSTRAINT %I CHECK (
           (end_time IS NULL) = (end_tz_offset_min IS NULL) AND (end_time IS NULL) = (end_tz_id IS NULL)
         ) NOT VALID', t, t || '_end_together');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = ('core.' || t)::regclass AND conname = t || '_end_values') THEN
      EXECUTE format(
        'ALTER TABLE core.%I ADD CONSTRAINT %I CHECK (
           end_tz_offset_min BETWEEN -1439 AND 1439 AND end_tz_id <> ''''
         ) NOT VALID', t, t || '_end_values');
    END IF;
  END LOOP;
END
$mig$;

-- ------------------------------------------------------------------ 2. 読み出しの 2 つの形
-- `SELECT *` の展開を作り直す（末尾に列が増えるだけなので置き換えられる）。
CREATE OR REPLACE VIEW core.event_live AS
  SELECT * FROM core.event WHERE deleted_at IS NULL;

CREATE OR REPLACE VIEW core.event_version_live AS
  SELECT v.id, v.event_id, v.user_id, v.logical_source, v.version_no,
         v.event_time, v.content_hash, v.raw, v.payload,
         v.source_updated_at, v.external_ref, v.superseded_at,
         v.tz_offset_min, v.tz_id, v.schema_version, v.unit_system, v.crs,
         e.sensitivity,          -- **親の値**（Q21。履歴は自分の感度を持たない）
         e.origin,
         e.external_id,
         v.end_time, v.end_tz_offset_min, v.end_tz_id
    FROM core.event_version v
    JOIN core.event e ON e.id = v.event_id
   WHERE e.deleted_at IS NULL;

-- ------------------------------------------------------------------ 3. 消した滞在の時間（D11）
-- 終わりを列→内容の `end`→始まり+1µs の順に読む。滞在は終わりを列に書かない（D13）ので、いまの振る舞いは変わらない。
CREATE OR REPLACE VIEW core.stay_erased AS
  SELECT id,
         user_id,
         event_time AS start_at,
         coalesce(end_time, core.try_timestamptz(payload->>'end'), event_time + interval '1 microsecond') AS end_at,
         deleted_at,
         deleted_by
    FROM core.event
   WHERE logical_source = 's01-stay'
     AND origin = 'derived'
     AND deleted_at IS NOT NULL;

-- ------------------------------------------------------------------ 4. 門と錠（D5）。いまの本体を写して終わりを足すだけ
CREATE OR REPLACE FUNCTION core.require_version_or_ledger() RETURNS trigger AS $fn$
DECLARE
  content_changed boolean;
  is_erasure      boolean;
BEGIN
  IF OLD.origin <> 'collected' THEN
    RETURN NULL;   -- 「本人が書いた」「派生させた」記録は対象外（FR-30 の範囲）
  END IF;

  content_changed :=
       NEW.raw          IS DISTINCT FROM OLD.raw
    OR NEW.payload      IS DISTINCT FROM OLD.payload
    OR NEW.event_time   IS DISTINCT FROM OLD.event_time
    OR NEW.end_time     IS DISTINCT FROM OLD.end_time
    OR NEW.content_hash IS DISTINCT FROM OLD.content_hash;

  IF NOT content_changed THEN
    RETURN NULL;   -- 論理削除・感度・更新時刻だけの書き換えは素通し（FR-50 / PERM-2）
  END IF;

  -- 消去の形だけを台帳の側へ回す（R51）。出来事の時刻・終わりの時刻・冪等キーは消去でも動かない
  -- （動かせるなら消去の顔で改竄できる）。payload も空でなければ消去ではない（R95）。
  is_erasure :=
        NEW.raw = '' AND OLD.raw <> ''
    AND NEW.payload = '{}'::jsonb
    AND NEW.event_time   = OLD.event_time
    AND NEW.end_time     IS NOT DISTINCT FROM OLD.end_time
    AND NEW.content_hash = OLD.content_hash;

  IF is_erasure THEN
    IF NOT EXISTS (
      SELECT 1 FROM core.erasure_ledger l
       WHERE l.event_id = NEW.id AND l.scope = 'event'
         AND l.txid = pg_current_xact_id()
    ) THEN
      RAISE EXCEPTION '本文の消去は、同じまとまりに消去の台帳の行があるときだけ通る（FR-51 / 深掘り Q23）';
    END IF;
    -- 親を消すなら履歴も同じまとまりで消す（R40 / tasks 7.4c）
    IF EXISTS (
      SELECT 1 FROM core.event_version v WHERE v.event_id = NEW.id AND v.raw <> ''
    ) THEN
      RAISE EXCEPTION '消去は親とその記録のすべての履歴を同じまとまりで行う（深掘り Q23 / R40）';
    END IF;
    RETURN NULL;
  END IF;

  -- 履歴行の中身が「更新前の版」であることまで見る（R94）。終わりだけ違う履歴の 1 行で通させない。
  IF NOT EXISTS (
    SELECT 1 FROM core.event_version v
     WHERE v.event_id = NEW.id AND v.txid = pg_current_xact_id()
       AND v.raw          = OLD.raw
       AND v.payload      = OLD.payload
       AND v.event_time   = OLD.event_time
       AND v.end_time     IS NOT DISTINCT FROM OLD.end_time
       AND v.content_hash = OLD.content_hash
  ) THEN
    RAISE EXCEPTION '収集した記録の書き換えは、同じまとまりに**更新前の版**の履歴があるときだけ通る（FR-30 / 深掘り Q10）';
  END IF;
  RETURN NULL;
END;
$fn$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION core.reject_version_rewrite() RETURNS trigger AS $fn$
BEGIN
  IF  NEW.id                IS DISTINCT FROM OLD.id
   OR NEW.event_id          IS DISTINCT FROM OLD.event_id
   OR NEW.user_id           IS DISTINCT FROM OLD.user_id
   OR NEW.logical_source    IS DISTINCT FROM OLD.logical_source
   OR NEW.version_no        IS DISTINCT FROM OLD.version_no
   OR NEW.event_time        IS DISTINCT FROM OLD.event_time
   OR NEW.end_time          IS DISTINCT FROM OLD.end_time
   OR NEW.end_tz_offset_min IS DISTINCT FROM OLD.end_tz_offset_min
   OR NEW.end_tz_id         IS DISTINCT FROM OLD.end_tz_id
   OR NEW.content_hash      IS DISTINCT FROM OLD.content_hash
   OR NEW.source_updated_at IS DISTINCT FROM OLD.source_updated_at
   OR NEW.external_ref      IS DISTINCT FROM OLD.external_ref
   OR NEW.tz_offset_min     IS DISTINCT FROM OLD.tz_offset_min
   OR NEW.tz_id             IS DISTINCT FROM OLD.tz_id
   OR NEW.schema_version    IS DISTINCT FROM OLD.schema_version
   OR NEW.unit_system       IS DISTINCT FROM OLD.unit_system
   OR NEW.crs               IS DISTINCT FROM OLD.crs
   OR NEW.superseded_at     IS DISTINCT FROM OLD.superseded_at
   OR NEW.txid              IS DISTINCT FROM OLD.txid
  THEN
    RAISE EXCEPTION '履歴は追記のみ。本文の消去以外の書き換えはできない（深掘り Q17）';
  END IF;
  -- 既に消去された版には、もう通す操作が無い（R95 の履歴側）
  IF OLD.raw = '' THEN
    RAISE EXCEPTION '既に消去された履歴の版は書き換えられない（深掘り Q17 / FR-51）';
  END IF;
  -- 残るのは raw / payload。両方を空へ落とす向きだけを通す
  IF NEW.raw <> '' OR NEW.payload <> '{}'::jsonb THEN
    RAISE EXCEPTION '履歴の書き換えは本文の消去（原文と解析済みを空にする）だけが通る（深掘り Q17 / FR-18）';
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;

-- ------------------------------------------------------------------ 5. 期間の部分索引（D10）
-- 部分索引にするのは、いまは終わりを持つ行が 0 件で作成が一瞬で終わるため（全行の式の索引は書き込みを待たせる）。
CREATE INDEX IF NOT EXISTS event_end_range ON core.event USING gist
  (tstzrange(event_time, greatest(event_time, end_time), '[]'))
  WHERE end_time IS NOT NULL;
