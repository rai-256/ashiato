DROP TABLE IF EXISTS core.archive_pending_shape;
DROP TABLE IF EXISTS core.archive_scan_counter;
DROP TABLE IF EXISTS core.archive_sighting;
DROP TABLE IF EXISTS core.archive_shape_confirmation;
DROP TABLE IF EXISTS core.archive_file;
DROP TABLE IF EXISTS core.archive_ledger_source;
DROP TABLE IF EXISTS core.archive_ledger;
-- **何かが参照している行は消さない**（design D14 の「記録が無いときだけ消す」）。
-- 無条件に消すと、記録や生存信号が 1 件でもあれば外部キーで戻し自体が落ちる。
-- 参照元は `core.source(logical_source)` を指す外部キーを持つ表すべて
-- （`information_schema` で数えた: event / heartbeat / coverage / coverage_span /
--  drop_report / source.succeeds）。
DELETE FROM core.source s
 WHERE (s.logical_source LIKE 'c03-%' OR s.logical_source = 's01-archive-inbox')
   AND NOT EXISTS (SELECT 1 FROM core.event        e WHERE e.logical_source = s.logical_source)
   AND NOT EXISTS (SELECT 1 FROM core.heartbeat    h WHERE h.logical_source = s.logical_source)
   AND NOT EXISTS (SELECT 1 FROM core.coverage     c WHERE c.logical_source = s.logical_source)
   AND NOT EXISTS (SELECT 1 FROM core.coverage_span p WHERE p.logical_source = s.logical_source)
   AND NOT EXISTS (SELECT 1 FROM core.drop_report  d WHERE d.logical_source = s.logical_source)
   AND NOT EXISTS (SELECT 1 FROM core.source       o WHERE o.succeeds = s.logical_source);
DROP FUNCTION IF EXISTS core.reject_archive_ledger_change();
