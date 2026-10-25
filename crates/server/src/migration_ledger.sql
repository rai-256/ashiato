-- 版の記録（ST48 / design D1）。`migrations/ORDER` の版にしない —— 版の記録を読む前に要り、自分自身を記録できない。
-- 何度流しても同じ。置き場は `core` の外（`grants.sql` の既定の権限が届かない）。
CREATE SCHEMA IF NOT EXISTS migration_ledger;
CREATE TABLE IF NOT EXISTS migration_ledger.form (
  only_row boolean PRIMARY KEY DEFAULT true CHECK (only_row),
  form     integer NOT NULL               -- 記録の形の版。いまは 1
);
CREATE TABLE IF NOT EXISTS migration_ledger.applied (
  name           text PRIMARY KEY,         -- 版の名前（ORDER の 1 行）
  sha256         text NOT NULL,            -- 前進側の .sql の中身の sha256（16 進の小文字）
  applied_at     timestamptz NOT NULL DEFAULT now(),
  build_version  text NOT NULL,            -- 当てた組み立ての版番号（CARGO_PKG_VERSION）
  build_commit   text NOT NULL,            -- 当てた組み立ての commit（D8）
  before_ledger  boolean NOT NULL DEFAULT false   -- 版の記録より前から当たっていた（D3）
);
INSERT INTO migration_ledger.form (form) VALUES (1) ON CONFLICT DO NOTHING;
-- アプリの役割には読むことだけ。`grants.sql` でなく、ここに置く
-- （`grants.sql` は記録の schema の無い DB にも psql で当たる。`tools/check-immutable.sh`）。
GRANT USAGE ON SCHEMA migration_ledger TO ashiato_app;
GRANT SELECT ON ALL TABLES IN SCHEMA migration_ledger TO ashiato_app;
