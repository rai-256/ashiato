-- 戻し手順（202610241200_satellite_credentials）。**不可逆** ——
-- 衛星の登録簿と資格情報の行（誰がいつ登録し、どの資格情報をいつ発行・取り消したか）が失われる。取り直す手段は無い。
-- 読み出しの記録の追加の欄（衛星の番号・資格情報の番号）も失われる。
--
-- 注意: 衛星の行（credential = 'satellite'）や結果 'forbidden' の行が読み出しの記録にあると、
-- 元の CHECK を付けられず、この戻し手順は ADD CONSTRAINT で落ちる。読み出しの記録は書き換えも削除もできないので、
-- 行があるときは戻せない（衛星が 1 度でも使われた DB は、戻さずに前へ進める）。
ALTER TABLE core.access_log DROP CONSTRAINT IF EXISTS access_log_satellite_ids;
ALTER TABLE core.access_log DROP CONSTRAINT IF EXISTS access_log_outcome_check;
ALTER TABLE core.access_log ADD CONSTRAINT access_log_outcome_check
  CHECK (outcome IN ('ok', 'unauthorized', 'login_ok', 'login_failed', 'login_throttled', 'logout'));
ALTER TABLE core.access_log DROP CONSTRAINT IF EXISTS access_log_credential_check;
ALTER TABLE core.access_log ADD CONSTRAINT access_log_credential_check
  CHECK (credential IN ('web_session', 'api_token', 'none'));
ALTER TABLE core.access_log DROP COLUMN IF EXISTS satellite_credential_id;
ALTER TABLE core.access_log DROP COLUMN IF EXISTS satellite_id;

DROP TRIGGER IF EXISTS satellite_credential_no_truncate ON core.satellite_credential;
DROP TRIGGER IF EXISTS satellite_credential_no_delete ON core.satellite_credential;
DROP TRIGGER IF EXISTS satellite_no_truncate ON core.satellite;
DROP TRIGGER IF EXISTS satellite_no_delete ON core.satellite;
DROP TABLE IF EXISTS core.satellite_credential;
DROP TABLE IF EXISTS core.satellite;
DROP FUNCTION IF EXISTS core.reject_satellite_delete();
