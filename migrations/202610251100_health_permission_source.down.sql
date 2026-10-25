-- 登録簿の行を参照するものが 1 件でも残っていれば、登録簿の行は残す（外部キーで落ちないように）。
-- 記録（core.event）だけでなく、破棄の報告（core.drop_report）と、稼働の数え
-- （core.coverage / core.coverage_span / core.heartbeat）も見る（`c01-clock` の戻しと同じ）。
DO $do$
BEGIN
  IF EXISTS (SELECT 1 FROM core.event WHERE logical_source = 'c01-health-permission')
     OR EXISTS (SELECT 1 FROM core.drop_report WHERE logical_source = 'c01-health-permission')
     OR EXISTS (SELECT 1 FROM core.coverage WHERE logical_source = 'c01-health-permission')
     OR EXISTS (SELECT 1 FROM core.coverage_span WHERE logical_source = 'c01-health-permission')
     OR EXISTS (SELECT 1 FROM core.heartbeat WHERE logical_source = 'c01-health-permission')
  THEN
    RAISE NOTICE 'c01-health-permission を参照する行が残っているので、登録簿の行は残す（design D1）';
    RETURN;
  END IF;
  DELETE FROM core.source WHERE logical_source = 'c01-health-permission';
END;
$do$;
