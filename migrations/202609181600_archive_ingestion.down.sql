DROP TABLE IF EXISTS core.archive_pending_shape;
DROP TABLE IF EXISTS core.archive_scan_counter;
DROP TABLE IF EXISTS core.archive_sighting;
DROP TABLE IF EXISTS core.archive_shape_confirmation;
DROP TABLE IF EXISTS core.archive_file;
DROP TABLE IF EXISTS core.archive_ledger_source;
DROP TABLE IF EXISTS core.archive_ledger;
-- **記録が無いときだけ消す**（design D14）。無条件に消すと、記録が 1 件でも
-- 入っていれば core.event の外部キーで戻し自体が落ちる。
DELETE FROM core.source s
 WHERE (s.logical_source LIKE 'c03-%' OR s.logical_source = 's01-archive-inbox')
   AND NOT EXISTS (SELECT 1 FROM core.event e WHERE e.logical_source = s.logical_source);
DROP FUNCTION IF EXISTS core.reject_archive_ledger_change();
