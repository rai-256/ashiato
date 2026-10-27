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

-- 版の記録の行を消す（tools/check-migrations.sh の、これから足す版の規則）。
-- 登録簿の行を残したときも消す —— 残すのは参照を守るためで、版は戻っている（もう一度当てても ON CONFLICT で二重にならない）。
DELETE FROM migration_ledger.applied WHERE name = '202610251100_health_permission_source';
