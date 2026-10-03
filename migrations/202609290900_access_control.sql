-- 画面のログインの印と、読み出しの記録（ST28 / design D3 / D8 / D9）
-- 前進のみ。戻し手順は migrations/202609290900_access_control.down.sql に置く。
--
-- **当て直せる形にする**（`migrate()` は毎回全版を当てる）。役割の作成は入れない（D4 の `tools/db-roles.sh`）。

-- ログインの印（D3）。印そのものは持たず SHA-256 だけ。`secret_tag` は画面の合言葉の世代
-- （HMAC-SHA256。鍵は API_TOKEN）—— 合言葉を変えると全行が有効の条件から外れる。行は消さない。
CREATE TABLE IF NOT EXISTS core.web_session (
  token_sha256 bytea       PRIMARY KEY,
  secret_tag   bytea       NOT NULL,
  issued_at    timestamptz NOT NULL,
  revoked_at   timestamptz
);

-- 読み出しの記録（D8）。**追記のみ**。route は型（`/stays`）だけで、クエリ文字列と path の値は入れない。
CREATE TABLE IF NOT EXISTS core.access_log (
  at         timestamptz NOT NULL DEFAULT now(),
  via        text        NOT NULL CHECK (via IN ('direct', 'forwarded')),
  credential text        NOT NULL CHECK (credential IN ('web_session', 'api_token', 'none')),
  route      text        NOT NULL,
  method     text        NOT NULL,
  outcome    text        NOT NULL
               CHECK (outcome IN ('ok', 'unauthorized', 'login_ok', 'login_failed', 'login_throttled', 'logout')),
  status     smallint    NOT NULL
);

-- 追記のみの錠。既存の台帳と同じ形（行トリガ + TRUNCATE の文トリガ）。
-- **他の版の関数を使わない** —— 使うと、その関数を落とす戻し手順が依存で当たらなくなる。
CREATE OR REPLACE FUNCTION core.reject_access_log_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '読み出しの記録（%）は書き換えも削除もできない（ST28 / design D8）', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS access_log_immutable ON core.access_log;
CREATE TRIGGER access_log_immutable
  BEFORE UPDATE OR DELETE ON core.access_log
  FOR EACH ROW EXECUTE FUNCTION core.reject_access_log_change();

DROP TRIGGER IF EXISTS access_log_no_truncate ON core.access_log;
CREATE TRIGGER access_log_no_truncate
  BEFORE TRUNCATE ON core.access_log
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_access_log_change();
