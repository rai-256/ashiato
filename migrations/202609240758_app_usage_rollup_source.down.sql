-- 戻し手順（202609240758_app_usage_rollup_source）。
--
-- **その論理ソースの記録が 1 件でも残っていれば、登録簿の行は残す**
-- （`s01-attribute` の戻し手順と同じ形）—— `core.event.logical_source` は
-- この行を外部キーで指しているので、消せば戻しが途中で止まるか、
-- 外部キーごと壊せば**その記録が何のソースだったかが永久に失われる**。
-- 集計は取得元が 2 年ぶんしか持たず、消えた分はもう取り直せない。
--
-- 行が落ちるのは**1 件も入っていないとき**だけ（＝当てたが使わなかった場合）。

DO $do$
BEGIN
  IF EXISTS (SELECT 1 FROM core.event WHERE logical_source = 'c01-app-usage-rollup') THEN
    RAISE NOTICE '集計の記録が残っているので、登録簿の行は残す';
    RETURN;
  END IF;
  DELETE FROM core.source WHERE logical_source = 'c01-app-usage-rollup';
END;
$do$;
