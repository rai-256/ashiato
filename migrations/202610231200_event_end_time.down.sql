-- 戻し手順（202610231200_event_end_time）。**不可逆** ——
-- 終わりの 3 列を落とすので、この版の後に入った記録の終わりの時刻・ずれ・タイムゾーンは失われる。
-- 逆順: 索引 → 関数をいまの版に戻す → ビューをいまの形に戻す → CHECK → 列。
DROP INDEX IF EXISTS core.event_end_range;

CREATE OR REPLACE FUNCTION core.require_version_or_ledger() RETURNS trigger AS $fn$
DECLARE
  content_changed boolean;
  is_erasure      boolean;
BEGIN
  IF OLD.origin <> 'collected' THEN
    RETURN NULL;
  END IF;

  content_changed :=
       NEW.raw          IS DISTINCT FROM OLD.raw
    OR NEW.payload      IS DISTINCT FROM OLD.payload
    OR NEW.event_time   IS DISTINCT FROM OLD.event_time
    OR NEW.content_hash IS DISTINCT FROM OLD.content_hash;

  IF NOT content_changed THEN
    RETURN NULL;
  END IF;

  is_erasure :=
        NEW.raw = '' AND OLD.raw <> ''
    AND NEW.payload = '{}'::jsonb
    AND NEW.event_time   = OLD.event_time
    AND NEW.content_hash = OLD.content_hash;

  IF is_erasure THEN
    IF NOT EXISTS (
      SELECT 1 FROM core.erasure_ledger l
       WHERE l.event_id = NEW.id AND l.scope = 'event'
         AND l.txid = pg_current_xact_id()
    ) THEN
      RAISE EXCEPTION '本文の消去は、同じまとまりに消去の台帳の行があるときだけ通る（FR-51 / 深掘り Q23）';
    END IF;
    IF EXISTS (
      SELECT 1 FROM core.event_version v WHERE v.event_id = NEW.id AND v.raw <> ''
    ) THEN
      RAISE EXCEPTION '消去は親とその記録のすべての履歴を同じまとまりで行う（深掘り Q23 / R40）';
    END IF;
    RETURN NULL;
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM core.event_version v
     WHERE v.event_id = NEW.id AND v.txid = pg_current_xact_id()
       AND v.raw          = OLD.raw
       AND v.payload      = OLD.payload
       AND v.event_time   = OLD.event_time
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
  IF OLD.raw = '' THEN
    RAISE EXCEPTION '既に消去された履歴の版は書き換えられない（深掘り Q17 / FR-51）';
  END IF;
  IF NEW.raw <> '' OR NEW.payload <> '{}'::jsonb THEN
    RAISE EXCEPTION '履歴の書き換えは本文の消去（原文と解析済みを空にする）だけが通る（深掘り Q17 / FR-18）';
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;

-- 列を落とす前に、列を指すビューを落とす
DROP VIEW IF EXISTS core.stay_erased;
DROP VIEW IF EXISTS core.event_folded;
DROP VIEW IF EXISTS core.event_version_live;
DROP VIEW IF EXISTS core.event_live;

ALTER TABLE core.event_version DROP CONSTRAINT IF EXISTS event_version_end_values;
ALTER TABLE core.event_version DROP CONSTRAINT IF EXISTS event_version_end_together;
ALTER TABLE core.event         DROP CONSTRAINT IF EXISTS event_end_values;
ALTER TABLE core.event         DROP CONSTRAINT IF EXISTS event_end_together;
ALTER TABLE core.event_version DROP COLUMN IF EXISTS end_tz_id;
ALTER TABLE core.event_version DROP COLUMN IF EXISTS end_tz_offset_min;
ALTER TABLE core.event_version DROP COLUMN IF EXISTS end_time;
ALTER TABLE core.event         DROP COLUMN IF EXISTS end_tz_id;
ALTER TABLE core.event         DROP COLUMN IF EXISTS end_tz_offset_min;
ALTER TABLE core.event         DROP COLUMN IF EXISTS end_time;

CREATE VIEW core.event_live AS SELECT * FROM core.event WHERE deleted_at IS NULL;

CREATE VIEW core.event_version_live AS
  SELECT v.id, v.event_id, v.user_id, v.logical_source, v.version_no,
         v.event_time, v.content_hash, v.raw, v.payload,
         v.source_updated_at, v.external_ref, v.superseded_at,
         v.tz_offset_min, v.tz_id, v.schema_version, v.unit_system, v.crs,
         e.sensitivity,
         e.origin,
         e.external_id
    FROM core.event_version v
    JOIN core.event e ON e.id = v.event_id
   WHERE e.deleted_at IS NULL;

CREATE VIEW core.event_folded AS
  SELECT user_id,
         logical_source,
         content_hash,
         count(*)                                            AS folded_rows,
         min(event_time)                                     AS event_time,
         (array_agg(id           ORDER BY ingest_time, id))[1] AS id,
         (array_agg(raw          ORDER BY ingest_time, id))[1] AS raw,
         (array_agg(payload      ORDER BY ingest_time, id))[1] AS payload,
         (array_agg(origin       ORDER BY ingest_time, id))[1] AS origin,
         max(sensitivity)                                    AS sensitivity,
         array_agg(external_id ORDER BY ingest_time, id)
           FILTER (WHERE external_id IS NOT NULL)            AS external_ids
    FROM core.event_live
   GROUP BY user_id, logical_source, content_hash;

CREATE VIEW core.stay_erased AS
  SELECT id,
         user_id,
         event_time AS start_at,
         coalesce(core.try_timestamptz(payload->>'end'), event_time + interval '1 microsecond') AS end_at,
         deleted_at,
         deleted_by
    FROM core.event
   WHERE logical_source = 's01-stay'
     AND origin = 'derived'
     AND deleted_at IS NOT NULL;
