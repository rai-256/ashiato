-- 衛星の登録簿と資格情報、読み出しの記録の欄（ST29 / design D7 / D9）
-- 前進のみ。戻し手順は migrations/202610241200_satellite_credentials.down.sql に置く。
-- migrate() は起動のたびに全版を当てるため、すべて当て直せる形にする。役割の作成は入れない。
--
-- 資格情報の値は持たない（値全体の SHA-256 だけ。D6）。行は消さない（取り消し・外すは時刻を付けるだけ。D1 / D7）。

CREATE TABLE IF NOT EXISTS core.satellite (
  id            uuid        PRIMARY KEY,
  identifier    text        NOT NULL UNIQUE CHECK (identifier ~ '^[a-z0-9-]+(\.[a-z0-9-]+)+$' AND length(identifier) <= 64),
  name          text        NOT NULL CHECK (name = btrim(name) AND length(name) BETWEEN 1 AND 40),
  builtin       boolean     NOT NULL DEFAULT false,
  registered_at timestamptz NOT NULL,
  removed_at    timestamptz,
  CHECK (NOT (builtin AND removed_at IS NOT NULL))
);
-- 名前の重なりは、外していない衛星どうしでだけ見る（D16）。保存した名前（前後の空白を除いた後）で比べる。
CREATE UNIQUE INDEX IF NOT EXISTS satellite_live_name ON core.satellite (name) WHERE removed_at IS NULL;

CREATE TABLE IF NOT EXISTS core.satellite_credential (
  id           uuid        PRIMARY KEY,
  satellite_id uuid        NOT NULL REFERENCES core.satellite(id),
  label        text        NOT NULL DEFAULT '' CHECK (length(label) <= 40),
  token_sha256 bytea       NOT NULL UNIQUE,
  issued_at    timestamptz NOT NULL,
  last_used_at timestamptz,
  revoked_at   timestamptz
);

-- 最初の 2 つ。固定の uuid で、当て直しても増えない。資格情報は入れない（利用者が発行するまで 0 行）。
INSERT INTO core.satellite (id, identifier, name, builtin, registered_at) VALUES
  ('5a7e1117-0000-4000-8000-000000000001', 'ashiato.ai-external', '外部 AI',     true, now()),
  ('5a7e1117-0000-4000-8000-000000000002', 'ashiato.ai-local',    'ローカル AI', true, now())
ON CONFLICT (identifier) DO NOTHING;

-- 行を消させない錠（UPDATE は許す: removed_at / revoked_at / last_used_at / name を書く）。
-- **この移行専用の関数** —— 他の版の関数を使うと、その関数を落とす戻し手順が依存で当たらなくなる。
CREATE OR REPLACE FUNCTION core.reject_satellite_delete() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '衛星の登録簿と資格情報（%）の行は消せない（ST29 / design D7）', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS satellite_no_delete ON core.satellite;
CREATE TRIGGER satellite_no_delete
  BEFORE DELETE ON core.satellite
  FOR EACH ROW EXECUTE FUNCTION core.reject_satellite_delete();
DROP TRIGGER IF EXISTS satellite_no_truncate ON core.satellite;
CREATE TRIGGER satellite_no_truncate
  BEFORE TRUNCATE ON core.satellite
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_satellite_delete();

DROP TRIGGER IF EXISTS satellite_credential_no_delete ON core.satellite_credential;
CREATE TRIGGER satellite_credential_no_delete
  BEFORE DELETE ON core.satellite_credential
  FOR EACH ROW EXECUTE FUNCTION core.reject_satellite_delete();
DROP TRIGGER IF EXISTS satellite_credential_no_truncate ON core.satellite_credential;
CREATE TRIGGER satellite_credential_no_truncate
  BEFORE TRUNCATE ON core.satellite_credential
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_satellite_delete();

-- 読み出しの記録（D9）。衛星と資格情報は番号（uuid）で書く。外部キーは付けない
-- （追記のみの表が登録簿の行に縛られると、登録簿の戻し手順が当たらなくなる。登録簿の行は消さないので指す先は残る）。
-- 制約の名前は 202609290900_access_control が付けた既定の名前（`\d core.access_log` で確認済み）。
ALTER TABLE core.access_log ADD COLUMN IF NOT EXISTS satellite_id uuid;
ALTER TABLE core.access_log ADD COLUMN IF NOT EXISTS satellite_credential_id uuid;
ALTER TABLE core.access_log DROP CONSTRAINT IF EXISTS access_log_credential_check;
ALTER TABLE core.access_log ADD CONSTRAINT access_log_credential_check
  CHECK (credential IN ('web_session', 'api_token', 'satellite', 'none'));
ALTER TABLE core.access_log DROP CONSTRAINT IF EXISTS access_log_outcome_check;
ALTER TABLE core.access_log ADD CONSTRAINT access_log_outcome_check
  CHECK (outcome IN ('ok', 'unauthorized', 'forbidden', 'login_ok', 'login_failed', 'login_throttled', 'logout'));
ALTER TABLE core.access_log DROP CONSTRAINT IF EXISTS access_log_satellite_ids;
ALTER TABLE core.access_log ADD CONSTRAINT access_log_satellite_ids
  CHECK ((credential = 'satellite') = (satellite_id IS NOT NULL AND satellite_credential_id IS NOT NULL));
