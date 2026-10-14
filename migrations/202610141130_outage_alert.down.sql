-- 戻し手順。台帳は判定の材料ではないので、落としても判定は戻る。3 列を落とす（印と日数は失われるが移行で再び入る）。
DROP TABLE IF EXISTS core.outage_ledger;
DROP FUNCTION IF EXISTS core.reject_outage_ledger_change();
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_started_on_from_not_self;
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_started_on_from_fk;
ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_alert_after_days_positive;
ALTER TABLE core.source DROP COLUMN IF EXISTS started_on_from;
ALTER TABLE core.source DROP COLUMN IF EXISTS alert_after_days;
ALTER TABLE core.source DROP COLUMN IF EXISTS outage_target;
