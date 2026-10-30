-- 戻し手順（202610300900_update_history）。**不可逆** —— 更新の履歴の行と、生存信号の版の列の値は失われる。
-- 逆順: ビュー → 生存信号の版の列 → 門 → 表 → 版の記録の自分の行。
DROP VIEW IF EXISTS core.collector_version_change;
ALTER TABLE core.heartbeat DROP COLUMN IF EXISTS collector_version;
DROP TRIGGER IF EXISTS update_history_no_truncate ON core.update_history;
DROP TRIGGER IF EXISTS update_history_immutable ON core.update_history;
DROP TABLE IF EXISTS core.update_history;
DROP FUNCTION IF EXISTS core.reject_update_history_rewrite();

-- 版の記録の表は適用する側（migrate）が作る。表が無い DB（試験の素の DB）では何もしない。
DO $$
BEGIN
    IF to_regclass('migration_ledger.applied') IS NOT NULL THEN
        DELETE FROM migration_ledger.applied WHERE name = '202610300900_update_history';
    END IF;
END
$$;
