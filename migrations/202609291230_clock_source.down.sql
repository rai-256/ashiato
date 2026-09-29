-- 記録が 1 件でも残っていれば登録簿の行は残す（記録が行を参照する）。
DO $do$
BEGIN
  IF EXISTS (SELECT 1 FROM core.event WHERE logical_source = 'c01-clock') THEN
    RAISE NOTICE 'c01-clock の記録が残っているので、登録簿の行は残す（design D1）';
    RETURN;
  END IF;
  DELETE FROM core.source WHERE logical_source = 'c01-clock';
END;
$do$;
