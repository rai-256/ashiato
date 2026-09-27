-- 戻し手順（202609240758_app_usage_rollup_source）。
--
-- **その論理ソースへの参照が 1 件でも残っていれば、登録簿の行は残す**。
-- event / heartbeat / coverage / coverage_span / drop_report / source.succeeds は
-- この行を外部キーで指しているので、消せば戻しが途中で止まるか、
-- 外部キーごと壊せば**その記録が何のソースだったかが永久に失われる**。
-- 集計は取得元が 2 年ぶんしか持たず、消えた分はもう取り直せない。
--
-- 行が落ちるのは**1 件も入っていないとき**だけ（＝当てたが使わなかった場合）。

DO $do$
BEGIN
  IF EXISTS (SELECT 1 FROM core.event WHERE logical_source = 'c01-app-usage-rollup')
    OR EXISTS (SELECT 1 FROM core.heartbeat WHERE logical_source = 'c01-app-usage-rollup')
    OR EXISTS (SELECT 1 FROM core.coverage WHERE logical_source = 'c01-app-usage-rollup')
    OR EXISTS (SELECT 1 FROM core.coverage_span WHERE logical_source = 'c01-app-usage-rollup')
    OR EXISTS (SELECT 1 FROM core.drop_report WHERE logical_source = 'c01-app-usage-rollup')
    OR EXISTS (SELECT 1 FROM core.source WHERE succeeds = 'c01-app-usage-rollup') THEN
    RAISE NOTICE '集計のソースへの参照が残っているので、登録簿の行は残す';
    RETURN;
  END IF;
  BEGIN
    DELETE FROM core.source WHERE logical_source = 'c01-app-usage-rollup';
  EXCEPTION WHEN foreign_key_violation THEN
    -- 並行して増えた参照や、後続の版が足した参照にも登録簿を残す。
    RAISE NOTICE '集計のソースへの参照があるので、登録簿の行は残す';
  END;
END;
$do$;
