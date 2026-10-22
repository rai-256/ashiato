-- 取り方・契機・取得の間隔を登録簿から外す。列の値は失われる（既定と D3 の表で再び入る）。不可逆
DROP TRIGGER IF EXISTS source_acquisition_fill ON core.source;
DROP FUNCTION IF EXISTS core.source_acquisition_fill();
DROP FUNCTION IF EXISTS core.source_fetch_trigger_default(text);
DROP FUNCTION IF EXISTS core.source_acquisition_default(text);
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_archive_inbox_interval_check;
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_fetch_interval_within_gap_check;
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_fetch_interval_iff_interval_check;
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_fetch_values_check;
ALTER TABLE core.source DROP COLUMN IF EXISTS fetch_interval_sec;
ALTER TABLE core.source DROP COLUMN IF EXISTS fetch_trigger;
ALTER TABLE core.source DROP COLUMN IF EXISTS acquisition;
