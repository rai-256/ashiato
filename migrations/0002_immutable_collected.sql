-- 0002 「収集した」記録の書き換えを DB で拒む（FR-30 / design D3）
-- 前進のみ。戻し手順は migrations/0002_immutable_collected.down.sql に置く。
--
-- **アプリ層のチェックでは足りない。** 同じ PC で動く第三者製プラグイン（PERM-8）や、
-- psql を直に叩く運用が素通りする。原文を書き換えられると、解析の誤りに後から
-- 気付いても元に戻せない（扉 #7）。

CREATE OR REPLACE FUNCTION core.reject_collected_rewrite() RETURNS trigger AS $fn$
BEGIN
  -- 論理削除（deleted_at / deleted_by）は通す。FR-50 が認めている。
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

DROP TRIGGER IF EXISTS event_immutable_collected ON core.event;
CREATE TRIGGER event_immutable_collected
  BEFORE UPDATE ON core.event
  FOR EACH ROW EXECUTE FUNCTION core.reject_collected_rewrite();
