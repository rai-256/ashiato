DROP TABLE IF EXISTS core.archive_pending_shape;
DROP TABLE IF EXISTS core.archive_scan_counter;
DROP TABLE IF EXISTS core.archive_sighting;
DROP TABLE IF EXISTS core.archive_shape_confirmation;
DROP TABLE IF EXISTS core.archive_file;
DROP TABLE IF EXISTS core.archive_ledger_source;
DROP TABLE IF EXISTS core.archive_ledger;
DELETE FROM core.source WHERE logical_source LIKE 'c03-%' OR logical_source = 's01-archive-inbox';
DROP FUNCTION IF EXISTS core.reject_archive_ledger_change();
