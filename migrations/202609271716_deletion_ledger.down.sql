-- 先に戻す操作を済ませること。台帳を落とすと、滞在を消した・戻した操作の履歴が失われる。
DROP VIEW IF EXISTS core.stay_erased;
DROP TRIGGER IF EXISTS deletion_ledger_no_truncate ON core.deletion_ledger;
DROP TRIGGER IF EXISTS deletion_ledger_append_only ON core.deletion_ledger;
DROP TABLE IF EXISTS core.deletion_ledger;
DROP FUNCTION IF EXISTS core.reject_deletion_ledger_change();
DROP FUNCTION IF EXISTS core.try_timestamptz(text);
