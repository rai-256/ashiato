-- 先に書き出しの台帳を使う機能を止めること。台帳を落とすと、どの写しがどこに書かれたかの履歴が失われる。
DROP TRIGGER IF EXISTS export_ledger_no_truncate ON core.export_ledger;
DROP TRIGGER IF EXISTS export_ledger_append_only ON core.export_ledger;
DROP TABLE IF EXISTS core.export_ledger;
DROP FUNCTION IF EXISTS core.reject_export_ledger_change();
