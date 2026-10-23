-- 登録簿に取り方・契機・取得の間隔を足す（ST13 / design D1 / D2 / D3）。
--
-- 当て直しても値を戻さない: 埋める UPDATE は `WHERE acquisition IS NULL`（列を足した直後だけ当たる）。
-- 制約は pg_constraint を conrelid で引いて、無いときだけ足す。
-- 取得の間隔 `fetch_interval_sec` は想定間隔 `expected_gap_sec` と別の列（想定間隔は FR-35 の途絶の判定の閾値）。

ALTER TABLE core.source ADD COLUMN IF NOT EXISTS acquisition text;
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS fetch_trigger text;
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS fetch_interval_sec integer;

-- D2: 取り方を名前の前置きから決める。前置きの分からない名前は `server`（断らない）。
-- `scheduled` は前置きから入れない。
CREATE OR REPLACE FUNCTION core.source_acquisition_default(p_logical_source text)
RETURNS text LANGUAGE sql IMMUTABLE AS $$
  SELECT CASE
    WHEN p_logical_source LIKE 'c01-%' THEN 'device'
    WHEN p_logical_source LIKE 'c02-%' THEN 'pc'
    WHEN p_logical_source LIKE 'c03-%' THEN 'archive'
    ELSE 'server'
  END
$$;

-- 契機の既定は取り方ごと（device / pc → on_change、archive → manual、server → derived）。
CREATE OR REPLACE FUNCTION core.source_fetch_trigger_default(p_acquisition text)
RETURNS text LANGUAGE sql IMMUTABLE AS $$
  SELECT CASE p_acquisition
    WHEN 'device' THEN 'on_change'
    WHEN 'pc' THEN 'on_change'
    WHEN 'archive' THEN 'manual'
    ELSE 'derived'
  END
$$;

-- 取り方と契機を指定しない INSERT（走っている ST09 の移行・ensure_myactivity_source・試験と道具の台本）に既定を入れる。
CREATE OR REPLACE FUNCTION core.source_acquisition_fill() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.acquisition IS NULL THEN
    NEW.acquisition := core.source_acquisition_default(NEW.logical_source);
  END IF;
  IF NEW.fetch_trigger IS NULL THEN
    NEW.fetch_trigger := core.source_fetch_trigger_default(NEW.acquisition);
  END IF;
  RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS source_acquisition_fill ON core.source;
CREATE TRIGGER source_acquisition_fill
  BEFORE INSERT ON core.source
  FOR EACH ROW EXECUTE FUNCTION core.source_acquisition_fill();

-- D1: 制約（名前付き）
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint
                 WHERE conrelid = 'core.source'::regclass AND conname = 'source_fetch_values_check') THEN
    ALTER TABLE core.source ADD CONSTRAINT source_fetch_values_check CHECK (
      acquisition IN ('device', 'pc', 'archive', 'server', 'scheduled')
      AND fetch_trigger IN ('interval', 'on_change', 'authored', 'derived', 'once', 'manual'));
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_constraint
                 WHERE conrelid = 'core.source'::regclass AND conname = 'source_fetch_interval_iff_interval_check') THEN
    ALTER TABLE core.source ADD CONSTRAINT source_fetch_interval_iff_interval_check CHECK (
      (fetch_trigger = 'interval') = (fetch_interval_sec IS NOT NULL));
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_constraint
                 WHERE conrelid = 'core.source'::regclass AND conname = 'source_fetch_interval_within_gap_check') THEN
    ALTER TABLE core.source ADD CONSTRAINT source_fetch_interval_within_gap_check CHECK (
      fetch_interval_sec BETWEEN 1 AND expected_gap_sec);
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_constraint
                 WHERE conrelid = 'core.source'::regclass AND conname = 'source_archive_inbox_interval_check') THEN
    ALTER TABLE core.source ADD CONSTRAINT source_archive_inbox_interval_check CHECK (
      logical_source <> 's01-archive-inbox' OR fetch_interval_sec <= 300);
  END IF;
END
$$;

-- D3: 既存の行の値。収集側の定数の写し（D9。端末と PC は tools/check-source-intervals.sh が一致を見る）。
-- c02-browser-history の 86400 は crates/collector-windows/src/history/fetch.rs の HISTORY_INTERVAL（24 時間）。
UPDATE core.source s
   SET acquisition = v.acquisition, fetch_trigger = v.fetch_trigger, fetch_interval_sec = v.fetch_interval_sec
  FROM (VALUES
    ('c01-location',        'device', 'interval',  60),
    ('c01-app-usage',       'device', 'interval',  1800),
    ('c01-app-usage-rollup','device', 'once',      NULL),
    ('c01-photo',           'device', 'on_change', NULL),
    ('c01-clock',           'device', 'interval',  3600),
    ('c02-window',          'pc',     'on_change', NULL),
    ('c02-browser-history', 'pc',     'interval',  86400),
    ('s01-archive-inbox',   'server', 'interval',  120),
    ('s01-stay',            'server', 'derived',   NULL),
    ('s01-attribute',       'server', 'authored',  NULL),
    ('s01-feeling',         'server', 'authored',  NULL),
    ('s01-place',           'server', 'authored',  NULL)
  ) AS v(logical_source, acquisition, fetch_trigger, fetch_interval_sec)
 WHERE s.logical_source = v.logical_source AND s.acquisition IS NULL;

-- 表に無い行（c03-* と、ST09 が先に足した行など）は D2 の関数で埋める。
UPDATE core.source
   SET acquisition = core.source_acquisition_default(logical_source)
 WHERE acquisition IS NULL;
UPDATE core.source
   SET fetch_trigger = core.source_fetch_trigger_default(acquisition)
 WHERE fetch_trigger IS NULL;

ALTER TABLE core.source ALTER COLUMN acquisition SET NOT NULL;
ALTER TABLE core.source ALTER COLUMN fetch_trigger SET NOT NULL;
