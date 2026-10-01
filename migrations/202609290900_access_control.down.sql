-- 戻し手順（202609290900_access_control）。**不可逆** ——
-- 読み出しの記録（誰がいつ読もうとしたか）が失われる。取り直す手段は無い。
-- ログインの印は失われても、画面でもう 1 度ログインすれば済む。
DROP TRIGGER IF EXISTS access_log_no_truncate ON core.access_log;
DROP TRIGGER IF EXISTS access_log_immutable ON core.access_log;
DROP TABLE IF EXISTS core.access_log;
DROP TABLE IF EXISTS core.web_session;
DROP FUNCTION IF EXISTS core.reject_access_log_change();
