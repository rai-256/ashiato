-- 0004 の戻し。0002 の関数定義に戻す（分類の移動と来歴の書き換えを再び許す）。
-- **戻すと、原文の不変が 3 手で迂回できる状態に戻る**（review R2）。
BEGIN;
CREATE OR REPLACE FUNCTION core.reject_collected_rewrite() RETURNS trigger AS $fn$
BEGIN
  IF OLD.origin = 'collected' AND (
       NEW.raw        IS DISTINCT FROM OLD.raw
    OR NEW.payload    IS DISTINCT FROM OLD.payload
    OR NEW.event_time IS DISTINCT FROM OLD.event_time
  ) THEN
    RAISE EXCEPTION '収集した記録の原文・解析済み・出来事の時刻は書き換えられない（FR-30）';
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;
COMMIT;
