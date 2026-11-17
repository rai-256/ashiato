-- 不可逆 —— rolled_back の行がある DB は、履歴を失わないため戻せない。
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM core.update_history WHERE outcome = 'rolled_back') THEN
    RAISE EXCEPTION 'rolled_back の履歴があるため移行を戻せない';
  END IF;
END
$$;
ALTER TABLE core.update_history DROP CONSTRAINT IF EXISTS update_history_outcome_check;
ALTER TABLE core.update_history ADD CONSTRAINT update_history_outcome_check
  CHECK (outcome IN ('attempted', 'applied', 'nothing_to_apply', 'failed', 'backup_failed'));

DO $$
BEGIN
    IF to_regclass('migration_ledger.applied') IS NOT NULL THEN
        DELETE FROM migration_ledger.applied WHERE name = '202611152200_update_history_rolled_back';
    END IF;
END
$$;
