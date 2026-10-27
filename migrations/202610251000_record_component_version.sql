-- ST49: 記録ごとの部品の版（design D7 / D16 / D19 / D21（仮））。
-- 2 回当てても落ちない形（`migrate()` は起動のたびに全版を当てる）。取引は適用する側が持つ。
-- 値の形は検査しない（D8）ので CHECK は置かない。既存の行は埋めない（D12。NULL = 不明）。

-- ------------------------------------------------------------------ 1. 列（どれも text NULL・既定値なし）
ALTER TABLE core.event                ADD COLUMN IF NOT EXISTS collector_version text;
ALTER TABLE core.event                ADD COLUMN IF NOT EXISTS web_version       text;
ALTER TABLE core.event                ADD COLUMN IF NOT EXISTS server_version    text;
ALTER TABLE core.event_version        ADD COLUMN IF NOT EXISTS collector_version text;
ALTER TABLE core.event_version        ADD COLUMN IF NOT EXISTS web_version       text;
ALTER TABLE core.event_version        ADD COLUMN IF NOT EXISTS server_version    text;
ALTER TABLE core.drop_report          ADD COLUMN IF NOT EXISTS collector_version text;
ALTER TABLE core.drop_report          ADD COLUMN IF NOT EXISTS server_version    text;
ALTER TABLE core.media_missing        ADD COLUMN IF NOT EXISTS collector_version text;
ALTER TABLE core.media_missing        ADD COLUMN IF NOT EXISTS server_version    text;
ALTER TABLE core.deletion_ledger      ADD COLUMN IF NOT EXISTS server_version    text;
ALTER TABLE core.feeling_reminder     ADD COLUMN IF NOT EXISTS server_version    text;
ALTER TABLE core.stay_criteria        ADD COLUMN IF NOT EXISTS server_version    text;
ALTER TABLE core.attribute_kind_name  ADD COLUMN IF NOT EXISTS server_version    text;

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
         v.end_time, v.end_tz_offset_min, v.end_tz_id,
         v.collector_version, v.web_version, v.server_version
    FROM core.event_version v
    JOIN core.event e ON e.id = v.event_id
   WHERE e.deleted_at IS NULL;

-- ------------------------------------------------------------------ 3. 門と前の版の錠（D21（仮））。いまの本体を写して版の 3 列を足すだけ
-- **写した元が変わっていたら落とす**（ST41 の R21 と同じ型）。下の 2 関数は本体ごと置き換えるので、ORDER でこの版より前の版が
-- 関数を変えると、起動のたびにここの古い写しがそれを黙って戻す。当てる直前の本体が、写した元（ST41 の本体。この版の `.down.sql` が戻す本体も同じ）か
-- この版の本体（2 回目の当て）のどちらでもなければ止める。
-- 直し方: 前の版の本体を写し直して版を足し、下の md5 を取り直す。
DO $mig$
DECLARE
  f record;
  got text;
BEGIN
  FOR f IN SELECT * FROM (VALUES
      ('require_version_or_ledger', ARRAY['048663f9a11ebe059461fa00f5b0f86d'], 'e3651d3492ae1d0d7e9f3b3683a6a161'),
      ('reject_version_rewrite',    ARRAY['297643252bd76e0f28239a983924434e'], 'f2caef81d5e0f36aac6a78918fab7bec')
    ) AS t(name, copied_from, this_version)
  LOOP
    SELECT md5(prosrc) INTO got FROM pg_proc
     WHERE pronamespace = 'core'::regnamespace AND proname = f.name;
    IF NOT (got = ANY(f.copied_from)) AND got IS DISTINCT FROM f.this_version THEN
      RAISE EXCEPTION 'core.%() の本体が、202610251000_record_component_version が写した元と違う（md5 %）。前の版の変更を写し直す',
        f.name, got;
    END IF;
  END LOOP;
END
$mig$;

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
    OR NEW.content_hash IS DISTINCT FROM OLD.content_hash
    OR NEW.collector_version IS DISTINCT FROM OLD.collector_version
    OR NEW.web_version       IS DISTINCT FROM OLD.web_version
    OR NEW.server_version    IS DISTINCT FROM OLD.server_version;

  IF NOT content_changed THEN
    RETURN NULL;   -- 論理削除・感度・更新時刻だけの書き換えは素通し（FR-50 / PERM-2）
  END IF;

  -- 消去の形だけを台帳の側へ回す（R51）。出来事の時刻・終わりの時刻・冪等キー・作った部品の版は消去でも動かない
  -- （動かせるなら消去の顔で改竄できる）。payload も空でなければ消去ではない（R95）。
  is_erasure :=
        NEW.raw = '' AND OLD.raw <> ''
    AND NEW.payload = '{}'::jsonb
    AND NEW.event_time   = OLD.event_time
    AND NEW.end_time     IS NOT DISTINCT FROM OLD.end_time
    AND NEW.content_hash = OLD.content_hash
    AND NEW.collector_version IS NOT DISTINCT FROM OLD.collector_version
    AND NEW.web_version       IS NOT DISTINCT FROM OLD.web_version
    AND NEW.server_version    IS NOT DISTINCT FROM OLD.server_version;

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

  -- 履歴行の中身が「更新前の版」であることまで見る（R94）。終わり・部品の版だけ違う履歴の 1 行で通させない。
  -- 版は NULL がありうる（不明）ので `IS NOT DISTINCT FROM` で照合する（`=` を写すと版の無い記録の更新が全部落ちる）。
  IF NOT EXISTS (
    SELECT 1 FROM core.event_version v
     WHERE v.event_id = NEW.id AND v.txid = pg_current_xact_id()
       AND v.raw          = OLD.raw
       AND v.payload      = OLD.payload
       AND v.event_time   = OLD.event_time
       AND v.end_time     IS NOT DISTINCT FROM OLD.end_time
       AND v.content_hash = OLD.content_hash
       AND v.collector_version IS NOT DISTINCT FROM OLD.collector_version
       AND v.web_version       IS NOT DISTINCT FROM OLD.web_version
       AND v.server_version    IS NOT DISTINCT FROM OLD.server_version
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
   OR NEW.collector_version IS DISTINCT FROM OLD.collector_version
   OR NEW.web_version       IS DISTINCT FROM OLD.web_version
   OR NEW.server_version    IS DISTINCT FROM OLD.server_version
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

-- ------------------------------------------------------------------ 4. 版の記録
-- 版の記録（migration_ledger）は適用する側（migrate）が書く（ST48 / D10）。ここでは書かない。
