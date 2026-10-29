-- 戻し手順（202610251000_record_component_version）。**不可逆** ——
-- 版の列を落とすので、この版の後に入った記録・報告・台帳の行の「作った部品の版」は失われる。
-- 逆順: 門と前の版の錠を ST41 の本体に戻す → 列を指すビューを落とす → 列 → ビューをいまの形に戻す → 版の記録の行を消す。
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

-- 列を落とす前に、列を指すビューを落とす（event_folded は event_live を指す）
DROP VIEW IF EXISTS core.event_folded;
DROP VIEW IF EXISTS core.event_version_live;
DROP VIEW IF EXISTS core.event_live;

ALTER TABLE core.attribute_kind_name DROP COLUMN IF EXISTS server_version;
ALTER TABLE core.stay_criteria       DROP COLUMN IF EXISTS server_version;
ALTER TABLE core.feeling_reminder    DROP COLUMN IF EXISTS server_version;
ALTER TABLE core.deletion_ledger     DROP COLUMN IF EXISTS server_version;
ALTER TABLE core.media_missing       DROP COLUMN IF EXISTS server_version;
ALTER TABLE core.media_missing       DROP COLUMN IF EXISTS collector_version;
ALTER TABLE core.drop_report         DROP COLUMN IF EXISTS server_version;
ALTER TABLE core.drop_report         DROP COLUMN IF EXISTS collector_version;
ALTER TABLE core.event_version       DROP COLUMN IF EXISTS server_version;
ALTER TABLE core.event_version       DROP COLUMN IF EXISTS web_version;
ALTER TABLE core.event_version       DROP COLUMN IF EXISTS collector_version;
ALTER TABLE core.event               DROP COLUMN IF EXISTS server_version;
ALTER TABLE core.event               DROP COLUMN IF EXISTS web_version;
ALTER TABLE core.event               DROP COLUMN IF EXISTS collector_version;

CREATE VIEW core.event_live AS SELECT * FROM core.event WHERE deleted_at IS NULL;

CREATE VIEW core.event_version_live AS
  SELECT v.id, v.event_id, v.user_id, v.logical_source, v.version_no,
         v.event_time, v.content_hash, v.raw, v.payload,
         v.source_updated_at, v.external_ref, v.superseded_at,
         v.tz_offset_min, v.tz_id, v.schema_version, v.unit_system, v.crs,
         e.sensitivity,
         e.origin,
         e.external_id,
         v.end_time, v.end_tz_offset_min, v.end_tz_id
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

-- 版の記録の表は適用する側（migrate）が作る。表が無い DB（試験の素の DB）では何もしない。
DO $$
BEGIN
    IF to_regclass('migration_ledger.applied') IS NOT NULL THEN
        DELETE FROM migration_ledger.applied WHERE name = '202610251000_record_component_version';
    END IF;
END
$$;
