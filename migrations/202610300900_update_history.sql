-- ST51: 更新の履歴（design D4（仮）・D5（仮）・D6・D10・D11）。
-- 2 回当てても落ちない形。取引は適用する側が持つ。
-- 表は追記のみ（書き換え・削除・切り詰めは所有者でも断る）。形は specs/distribution の表で固定する。

-- ------------------------------------------------------------------ 1. 更新の履歴（1 回の版の変化につき 1 行）
CREATE TABLE IF NOT EXISTS core.update_history (
  seq              bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,   -- 書いた順（時計は戻りうるので時刻で決めない）
  at               timestamptz NOT NULL DEFAULT now(),
  component        text NOT NULL CHECK (component = 'S-01'),
  via              text NOT NULL CHECK (via IN ('migrate', 'startup')),
  from_version     text NULL,
  to_version       text NOT NULL,
  outcome          text NOT NULL CHECK (outcome IN ('attempted', 'applied', 'nothing_to_apply', 'failed', 'backup_failed')),
  applied_versions text[] NOT NULL DEFAULT '{}',
  failed_version   text NULL,
  sqlstate         text NULL
);

-- ------------------------------------------------------------------ 2. 書き換えの門（所有者でも断る。core.reject_heartbeat_rewrite と同じ形）
CREATE OR REPLACE FUNCTION core.reject_update_history_rewrite() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '更新の履歴は書き換えられない（追記のみ）';
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS update_history_immutable ON core.update_history;
CREATE TRIGGER update_history_immutable
  BEFORE UPDATE OR DELETE ON core.update_history
  FOR EACH ROW EXECUTE FUNCTION core.reject_update_history_rewrite();

DROP TRIGGER IF EXISTS update_history_no_truncate ON core.update_history;
CREATE TRIGGER update_history_no_truncate
  BEFORE TRUNCATE ON core.update_history
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_update_history_rewrite();

-- ------------------------------------------------------------------ 3. 生存信号の版の列（D4・D6。既存の行は埋めない）
ALTER TABLE core.heartbeat ADD COLUMN IF NOT EXISTS collector_version text NULL;

-- ------------------------------------------------------------------ 4. 収集アプリの版の変化（D5・D6。導くビュー）
-- 版は列が空でなければ列から、空なら原文の `client_version` から読む。原文が JSON でなければ「版を持たない」
-- （`pg_input_is_valid` で確かめてから読むので、1 行の不正な原文でビューが落ちない）。
-- 端末が空の信号は、利用者・部品ごとの 1 つの並びにする。並べ方は発信の時刻・受信の時刻・id の順。
CREATE OR REPLACE VIEW core.collector_version_change AS
WITH sig AS (
  SELECT h.user_id,
         NULLIF(h.device_id, '')                          AS device_id,
         CASE WHEN h.logical_source LIKE 'c01-%' THEN 'C-01' ELSE 'C-02' END AS component,
         COALESCE(
           NULLIF(h.collector_version, ''),
           CASE WHEN pg_input_is_valid(h.raw, 'jsonb')
                THEN NULLIF((h.raw::jsonb) ->> 'client_version', '')
           END
         )                                                AS version,
         h.emitted_at, h.received_at, h.logical_source, h.id
    FROM core.heartbeat h
   WHERE h.logical_source LIKE 'c01-%' OR h.logical_source LIKE 'c02-%'
), ordered AS (
  SELECT s.*,
         lag(s.version) OVER (
           PARTITION BY s.user_id, COALESCE(s.device_id, ''), s.component
           ORDER BY s.emitted_at, s.received_at, s.id
         ) AS from_version
    FROM sig s
   WHERE s.version IS NOT NULL
)
SELECT user_id, device_id, component,
       from_version, version AS to_version,
       emitted_at, received_at, logical_source
  FROM ordered
 WHERE from_version IS NOT NULL AND from_version <> version;
