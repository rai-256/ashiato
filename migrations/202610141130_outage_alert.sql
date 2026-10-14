-- 途切れの通知（ST14 / design D2 / D7）。移行は 1 本。当て直しても壊れない・本人が変えた値を戻さない。

-- 判定の対象か（C1。既定は「対象」）／通知までの日数（Q9。NULL なら想定間隔の 3 倍）／収集開始日を借りる先（Q8）
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS outage_target boolean NOT NULL DEFAULT true;
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS alert_after_days integer NULL;
ALTER TABLE core.source ADD COLUMN IF NOT EXISTS started_on_from text NULL;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'source_alert_after_days_positive') THEN
    ALTER TABLE core.source ADD CONSTRAINT source_alert_after_days_positive CHECK (alert_after_days > 0);
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'source_started_on_from_fk') THEN
    ALTER TABLE core.source ADD CONSTRAINT source_started_on_from_fk
      FOREIGN KEY (started_on_from) REFERENCES core.source (logical_source);
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'source_started_on_from_not_self') THEN
    ALTER TABLE core.source ADD CONSTRAINT source_started_on_from_not_self
      CHECK (started_on_from IS DISTINCT FROM logical_source);
  END IF;
END
$$;

-- 判定の対象外の印。無い名前は何もしない（後から入っても出どころで外れる。D3）
UPDATE core.source SET outage_target = false
 WHERE logical_source IN ('c01-clock', 'c01-app-usage-rollup', 's01-stay', 's01-attribute', 's01-feeling', 's01-place')
   AND outage_target;

-- 書庫は最後の書庫から 75 日。本人が変えた値は戻さない
UPDATE core.source SET alert_after_days = 75
 WHERE logical_source LIKE 'c03-%' AND alert_after_days IS NULL;

-- 集計の開始日はアプリ利用から借りる。汚れた値を直す（記録は触らない）
UPDATE core.source SET started_on_from = 'c01-app-usage'
 WHERE logical_source = 'c01-app-usage-rollup'
   AND EXISTS (SELECT 1 FROM core.source WHERE logical_source = 'c01-app-usage')
   AND started_on_from IS DISTINCT FROM 'c01-app-usage';
UPDATE core.source SET collection_started_on =
         (SELECT collection_started_on FROM core.source WHERE logical_source = 'c01-app-usage')
 WHERE logical_source = 'c01-app-usage-rollup'
   AND (SELECT collection_started_on FROM core.source WHERE logical_source = 'c01-app-usage') IS NOT NULL
   AND collection_started_on IS DISTINCT FROM
         (SELECT collection_started_on FROM core.source WHERE logical_source = 'c01-app-usage');

-- 途切れと戻りと届けたことの台帳（D7。追記のみ。記録の本文・座標・原文の列は持たない）
CREATE TABLE IF NOT EXISTS core.outage_ledger (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id uuid NOT NULL,
  logical_source text NOT NULL REFERENCES core.source (logical_source),
  kind text NOT NULL CHECK (kind IN ('opened', 'closed', 'delivered')),
  episode_id bigint NULL REFERENCES core.outage_ledger (id),
  device_id text NULL,
  last_alive_at timestamptz NULL,
  judged_at timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT outage_ledger_shape CHECK (
    (kind = 'opened' AND episode_id IS NULL AND device_id IS NULL)
    OR (kind = 'closed' AND episode_id IS NOT NULL AND device_id IS NULL AND last_alive_at IS NULL)
    OR (kind = 'delivered' AND episode_id IS NOT NULL AND device_id IS NOT NULL AND last_alive_at IS NULL)
  ),
  CONSTRAINT outage_ledger_opened_has_last_alive CHECK (kind <> 'opened' OR last_alive_at IS NOT NULL)
);
-- 途切れの始まりは最後に生きていた時刻を必ず持つ（D7）。形の CHECK とは別の名前で持ち、この CHECK より前に表を作った DB にも足す。
-- 先に書いた行は検めない（NOT VALID）—— 台帳は追記のみで直せず、新しい行はこの CHECK が縛る
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint
                  WHERE conname = 'outage_ledger_opened_has_last_alive'
                    AND conrelid = 'core.outage_ledger'::regclass) THEN
    ALTER TABLE core.outage_ledger ADD CONSTRAINT outage_ledger_opened_has_last_alive
      CHECK (kind <> 'opened' OR last_alive_at IS NOT NULL) NOT VALID;
  END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS outage_ledger_closed_once
  ON core.outage_ledger (episode_id) WHERE kind = 'closed';
CREATE UNIQUE INDEX IF NOT EXISTS outage_ledger_delivered_once
  ON core.outage_ledger (episode_id, device_id) WHERE kind = 'delivered';
-- 拒否の関数は書庫の台帳と同じ形で、この台帳専用に持つ（書庫の移行の戻しが共有の関数を落とすので、共有すると戻せなくなる）
CREATE OR REPLACE FUNCTION core.reject_outage_ledger_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '途切れの台帳（%）は追記のみ', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS outage_append_only ON core.outage_ledger;
CREATE TRIGGER outage_append_only BEFORE UPDATE OR DELETE ON core.outage_ledger
  FOR EACH ROW EXECUTE FUNCTION core.reject_outage_ledger_change();
DROP TRIGGER IF EXISTS outage_no_truncate ON core.outage_ledger;
CREATE TRIGGER outage_no_truncate BEFORE TRUNCATE ON core.outage_ledger
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_outage_ledger_change();
