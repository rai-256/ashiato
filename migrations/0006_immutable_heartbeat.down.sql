-- 0006 の戻し手順。**保護を外すだけ**なので行は失われない。
-- ただし外した後は、生存信号が psql からも第三者製プラグイン（PERM-8）からも書き換えられる。
DROP TRIGGER IF EXISTS heartbeat_immutable ON core.heartbeat;
DROP FUNCTION IF EXISTS core.reject_heartbeat_rewrite();
