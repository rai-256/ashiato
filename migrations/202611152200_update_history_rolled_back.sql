-- ST58: 戻した更新を追記専用の履歴へ残す。
-- 不可逆 —— この版を戻すと rolled_back を受け付ける制約が失われる（行は失わない）。
ALTER TABLE core.update_history DROP CONSTRAINT IF EXISTS update_history_outcome_check;
ALTER TABLE core.update_history ADD CONSTRAINT update_history_outcome_check
  CHECK (outcome IN ('attempted', 'applied', 'nothing_to_apply', 'failed', 'backup_failed', 'rolled_back'));
