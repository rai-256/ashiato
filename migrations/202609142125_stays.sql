-- 滞在の基準と吸収の台帳、滞在用の論理ソース（ST16 / FR-31 / FR-76。design D10 / D2）
-- 前進のみ。戻し手順は migrations/202609142125_stays.down.sql に置く。
--
-- **滞在そのものは `core.event` に入れる**（design D1）。この版が足すのは、
-- 滞在の行に載せきれない 2 つの事実だけ —— 「どの基準で作ったか」の版と、「どの滞在に吸収されたか」。
-- どちらも**追記のみ**（「台帳は追記のみ」の既定。deep.md C4）。
--
-- **当て直せる形にする**（`migrate()` は起動のたびに全版を当てる）。

-- ------------------------------------------------------------------ 判定の基準（利用者ごと・版）
--
-- **いまの基準は、その利用者の `id` が最大の行**（design D10）。行が 1 本も無い利用者は
-- コードの既定（100 m / 10 分 / 10 分 / `{c01-location}`）で、最初に作り直したときに最初の版として書く ——
-- 移行は利用者の一覧を知らないので、ここでは行を入れない。
--
-- 範囲は spec「範囲外の基準は断られる」と同じ値。アプリが先に 400 で弾くので、
-- この `CHECK` に当たるのはアプリを通らない書き込みだけ。
CREATE TABLE IF NOT EXISTS core.stay_criteria (
  id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id     uuid NOT NULL,                                   -- FR-29。基準は利用者ごと（R12）
  radius_m    integer NOT NULL CHECK (radius_m BETWEEN 1 AND 10000),
  min_minutes integer NOT NULL CHECK (min_minutes BETWEEN 1 AND 1440),
  gap_minutes integer NOT NULL CHECK (gap_minutes BETWEEN 1 AND 1440),
  sources     text[]  NOT NULL CHECK (cardinality(sources) > 0),
  created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS stay_criteria_by_user ON core.stay_criteria (user_id, id);

-- ------------------------------------------------------------------ 吸収の台帳
--
-- 作り直しでどの新しい滞在にも割り当てられなかった滞在は、行を消さずに
-- `deleted_by = 'rebuild:absorbed'` の印を付け、ここに「どこへ吸収されたか」を 1 行書く（design D3）。
-- **吸収先は無いことがある**（時間の重なる新しい滞在が 1 件も無い）ので `NULL` を許す。
-- `event_id` には FK を張る —— 吸収された滞在の行が消えると、この台帳が指す先を失う。
CREATE TABLE IF NOT EXISTS core.stay_absorbed (
  id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  event_id      uuid NOT NULL REFERENCES core.event(id),
  user_id       uuid NOT NULL,
  into_event_id uuid REFERENCES core.event(id),
  criteria_id   bigint NOT NULL REFERENCES core.stay_criteria(id),
  at            timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS stay_absorbed_by_event ON core.stay_absorbed (event_id);

-- ------------------------------------------------------------------ 台帳 2 つの錠
--
-- ST03 の `erasure_ledger` と同じ形。**開ける必要のある操作が 1 つも無い**。
-- `TRUNCATE` は行トリガで撃たれないので、文トリガを別に置く。
-- **ST03 の `core.reject_truncate()` を使わない** —— 使うと、その関数を落とす
-- `202609120944_gates.down.sql` が依存で当たらなくなる（`tools/check-immutable.sh` が実測で落ちた）。
CREATE OR REPLACE FUNCTION core.reject_stay_ledger_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '滞在の台帳（%）は追記のみ。書き換えも削除もできない（FR-31 / design D10）', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS stay_criteria_append_only ON core.stay_criteria;
CREATE TRIGGER stay_criteria_append_only
  BEFORE UPDATE OR DELETE ON core.stay_criteria
  FOR EACH ROW EXECUTE FUNCTION core.reject_stay_ledger_change();

DROP TRIGGER IF EXISTS stay_criteria_no_truncate ON core.stay_criteria;
CREATE TRIGGER stay_criteria_no_truncate
  BEFORE TRUNCATE ON core.stay_criteria
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_stay_ledger_change();

DROP TRIGGER IF EXISTS stay_absorbed_append_only ON core.stay_absorbed;
CREATE TRIGGER stay_absorbed_append_only
  BEFORE UPDATE OR DELETE ON core.stay_absorbed
  FOR EACH ROW EXECUTE FUNCTION core.reject_stay_ledger_change();

DROP TRIGGER IF EXISTS stay_absorbed_no_truncate ON core.stay_absorbed;
CREATE TRIGGER stay_absorbed_no_truncate
  BEFORE TRUNCATE ON core.stay_absorbed
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_stay_ledger_change();

-- ------------------------------------------------------------------ 滞在用の論理ソース
--
-- **`external_id_kind = 'record'`**（design D2 / D3）。滞在は識別子を `external_id` にも置くので、
-- 一意索引 `event_dedup_hash`（`WHERE external_id IS NULL`）に当たらない（deep-review R1 の実測を避ける）。
-- `expected_gap_sec` は稼働状況に使われない（`coverage::must_sources()` は NFR-13 の 5 ソースだけ）が、
-- 列が `NOT NULL` なので 1 日を入れる。
-- **取り込みの口では断らない**（design D2（仮））—— 断ると `record-envelope` の要件を変える。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
VALUES ('s01-stay', '滞在', 86400, 'record')
ON CONFLICT (logical_source) DO NOTHING;
