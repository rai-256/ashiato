-- 個人属性の主張（ST19 / FR-44 / FR-45。design D2 / D7 / D12）
-- 前進のみ。戻し手順は migrations/202609160220_personal_attributes.down.sql に置く。
--
-- **主張そのものは `core.event` に入る**（design D1。`origin='authored'` /
-- `logical_source='s01-attribute'`）。この版が足すのは 3 つだけ ——
--   1. 種類の表と名前の台帳（主張を束ねる器。記録ではないので `core.event` に入れない）
--   2. 主張の行の錠（即時の錠・COMMIT 時の門・行の削除の拒否）
--   3. 登録簿の 1 行
--
-- **ST03 の関数（`core.reject_collected_rewrite` / `core.require_version_or_ledger` /
-- `core.reject_collected_delete`）は書き換えない**（design D2）—— 変えると ST03 の
-- `.down.sql` と `tools/check-immutable.sh` の前提が動く。ST16 が `core.reject_truncate()` を
-- 使わなかったのと同じ理由で、関数は主張のために新しく作る。
--
-- **当て直せる形にする**（`migrate()` は起動のたびに全版を当てる）。

-- ------------------------------------------------------------------ 種類と名前の台帳（D7）
--
-- 種類は記録ではない（本人の出来事ではなく、主張を束ねる器）。名前を変えても識別子は変わらない
-- （扉 #17 と同じ型）—— 名前で種類を指すと、名前を直した日に過去の主張が別の種類に割れる。
--
-- **いまの名前は `kind_id` ごとに `id` が最大の行**。台帳は追記のみなので、前の名前は残り続ける。
CREATE TABLE IF NOT EXISTS core.attribute_kind (
  id         uuid PRIMARY KEY,
  user_id    uuid NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

-- **`(id, user_id)` に一意を張る**（spec-review R16）。下の名前の台帳から複合の外部キーで指し、
-- **種類と名前の利用者が食い違う行を DB が作らせない**ようにするため。
CREATE UNIQUE INDEX IF NOT EXISTS attribute_kind_id_user ON core.attribute_kind (id, user_id);
CREATE INDEX IF NOT EXISTS attribute_kind_by_user ON core.attribute_kind (user_id, created_at, id);

CREATE TABLE IF NOT EXISTS core.attribute_kind_name (
  id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  kind_id    uuid NOT NULL,
  user_id    uuid NOT NULL,
  name       text NOT NULL CHECK (name <> ''),
  created_at timestamptz NOT NULL DEFAULT now(),
  FOREIGN KEY (kind_id, user_id) REFERENCES core.attribute_kind (id, user_id)
);

CREATE INDEX IF NOT EXISTS attribute_kind_name_by_kind ON core.attribute_kind_name (kind_id, id);
CREATE INDEX IF NOT EXISTS attribute_kind_name_by_user ON core.attribute_kind_name (user_id, id);

-- ------------------------------------------------------------------ 2 表の錠（追記のみ）
--
-- ST16 の `core.reject_stay_ledger_change()` と同じ形。**開ける必要のある操作が 1 つも無い。**
-- 名前を変えるのは行を足すことで、書き換えることではない。
-- `TRUNCATE` は行トリガで撃たれないので、文トリガを別に置く。
--
-- **ST03 の `core.reject_truncate()` を使わない** —— 使うと、その関数を落とす
-- `202609120944_gates.down.sql` が依存で当たらなくなる（ST16 が実測で踏んだ）。
CREATE OR REPLACE FUNCTION core.reject_attribute_kind_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '種類の台帳（%）は追記のみ。書き換えも削除もできない（FR-44 / 深掘り C7）', TG_TABLE_NAME;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS attribute_kind_append_only ON core.attribute_kind;
CREATE TRIGGER attribute_kind_append_only
  BEFORE UPDATE OR DELETE ON core.attribute_kind
  FOR EACH ROW EXECUTE FUNCTION core.reject_attribute_kind_change();

DROP TRIGGER IF EXISTS attribute_kind_no_truncate ON core.attribute_kind;
CREATE TRIGGER attribute_kind_no_truncate
  BEFORE TRUNCATE ON core.attribute_kind
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_attribute_kind_change();

DROP TRIGGER IF EXISTS attribute_kind_name_append_only ON core.attribute_kind_name;
CREATE TRIGGER attribute_kind_name_append_only
  BEFORE UPDATE OR DELETE ON core.attribute_kind_name
  FOR EACH ROW EXECUTE FUNCTION core.reject_attribute_kind_change();

DROP TRIGGER IF EXISTS attribute_kind_name_no_truncate ON core.attribute_kind_name;
CREATE TRIGGER attribute_kind_name_no_truncate
  BEFORE TRUNCATE ON core.attribute_kind_name
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_attribute_kind_change();

-- ------------------------------------------------------------------ 主張の即時の錠（D2 の 1）
--
-- **FR-44「書き換えず追記する」を DB の側で強制する**（深掘り C1 / Q1）。
-- アプリ層に置くと、同じ PC の psql やプラグインが素通りする（FR-30 が凍結を
-- アプリ層に置かなかった理由と同じ）。
--
-- 通すのは **削除の印（`deleted_at` / `deleted_by`）と感度**、そして下の門が見る
-- **台帳つきの消去**だけ（Q1 —— 主張も記録なので FR-50 / FR-51 が及ぶ）。
CREATE OR REPLACE FUNCTION core.reject_claim_rewrite() RETURNS trigger AS $fn$
BEGIN
  -- **主張へ付け替える向きを塞ぐ**（ST03 の R114 と同じ型）。
  -- 別の「本人が書いた」記録の論理ソースを主張へ移せると、錠の外で作った行を
  -- 主張として固定できる（＝主張の捏造）。
  IF NEW.logical_source = 's01-attribute'
     AND OLD.logical_source IS DISTINCT FROM 's01-attribute' THEN
    RAISE EXCEPTION '他の記録を個人属性の主張へ付け替えることはできない（FR-44 / 深掘り C1）';
  END IF;

  IF OLD.logical_source = 's01-attribute' THEN
    -- 分類そのものを動かせないようにする（動かせると錠の外へ 3 手で抜けられる）
    IF NEW.logical_source IS DISTINCT FROM OLD.logical_source THEN
      RAISE EXCEPTION '主張の論理ソースは書き換えられない（FR-44 / FR-22）';
    END IF;
    IF NEW.origin IS DISTINCT FROM OLD.origin THEN
      RAISE EXCEPTION '主張の由来は書き換えられない（FR-44 / FR-25）';
    END IF;
    IF NEW.id IS DISTINCT FROM OLD.id THEN
      RAISE EXCEPTION '主張の識別子は書き換えられない（FR-21 / FR-44）';
    END IF;
    -- **`user_id` も凍結する**（design D2）。ST03 は収集側の設定ミスを直す余地で
    -- 凍結しなかったが、**主張は取り消し先を同じ利用者の中で引く**ので、
    -- 動かすと取り消しが別の利用者を指す。
    IF NEW.user_id IS DISTINCT FROM OLD.user_id THEN
      RAISE EXCEPTION '主張の利用者は書き換えられない（FR-44 / 取り消し先が別の利用者を指す）';
    END IF;
    -- **2 つの時刻はどちらも来歴**（FR-45）。動かせると「後から直したことが分かる」が崩れる
    IF NEW.event_time IS DISTINCT FROM OLD.event_time THEN
      RAISE EXCEPTION '主張した日時は書き換えられない（FR-45 / 深掘り C4）';
    END IF;
    IF NEW.tz_offset_min IS DISTINCT FROM OLD.tz_offset_min
       OR NEW.tz_id IS DISTINCT FROM OLD.tz_id THEN
      RAISE EXCEPTION '主張した日時の地域は書き換えられない（FR-45）';
    END IF;
    IF NEW.ingest_time IS DISTINCT FROM OLD.ingest_time THEN
      RAISE EXCEPTION '主張が D-01 に入った時刻は書き換えられない（FR-19 / FR-44）';
    END IF;
    IF NEW.external_id IS DISTINCT FROM OLD.external_id
       OR NEW.external_ref IS DISTINCT FROM OLD.external_ref THEN
      RAISE EXCEPTION '主張の外部識別子は書き換えられない（FR-23 / FR-44）';
    END IF;
    IF NEW.device_id IS DISTINCT FROM OLD.device_id THEN
      RAISE EXCEPTION '主張の端末識別子は書き換えられない（FR-44）';
    END IF;
    IF NEW.schema_version IS DISTINCT FROM OLD.schema_version
       OR NEW.unit_system IS DISTINCT FROM OLD.unit_system
       OR NEW.crs IS DISTINCT FROM OLD.crs
       OR NEW.source_updated_at IS DISTINCT FROM OLD.source_updated_at THEN
      RAISE EXCEPTION '主張のエンベロープは書き換えられない（FR-44 / FR-28）';
    END IF;
    -- **`raw` / `payload` / `content_hash` はここでは見ない。**
    -- 値・「いつから」・種類・訂正先・補足はすべて原文と解析済みの中にあり、
    -- それらを守るのは下の門（消去の形と台帳を見る）—— FR-51 の消去を通すため、
    -- 即時の錠で閉じ切ることはできない。
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_claim_immutable ON core.event;
CREATE TRIGGER event_claim_immutable
  BEFORE UPDATE ON core.event
  FOR EACH ROW EXECUTE FUNCTION core.reject_claim_rewrite();

-- ------------------------------------------------------------------ 主張の門（COMMIT 時。D2 の 2）
--
-- 本文（原文・解析済み・内容の鍵）が動く書き換えは、**消去の形で、かつ同じまとまりに
-- その主張の台帳の行があるときだけ**通す。
--
-- **「その主張の」を見る**（spec-review R7）—— `event_id = NEW.id` まで照合しないと、
-- 台帳を 1 行書いたまとまりで**何件でも**消去できる。
-- **消去の形で絞る**（ST03 の R51 / R95 と同じ穴）—— 台帳があるだけで通すと、
-- 原文（唯一の復元元）を消しながらもっともらしい解析済みを植えられる。
--
-- 主張には**前の版の経路が無い**（書き換えないので履歴表を使わない）ので、
-- ST03 の「履歴を書けば通る」に当たる開口部は持たない。
CREATE OR REPLACE FUNCTION core.require_claim_erasure_ledger() RETURNS trigger AS $fn$
DECLARE
  content_changed boolean;
  is_erasure      boolean;
BEGIN
  IF OLD.logical_source <> 's01-attribute' THEN
    RETURN NULL;   -- 主張以外はこの門の対象外（ST03 の門が別に見る）
  END IF;

  content_changed :=
       NEW.raw          IS DISTINCT FROM OLD.raw
    OR NEW.payload      IS DISTINCT FROM OLD.payload
    OR NEW.content_hash IS DISTINCT FROM OLD.content_hash;

  IF NOT content_changed THEN
    RETURN NULL;   -- 削除の印・感度だけの書き換えは素通し（FR-50 / PERM-2 / Q1）
  END IF;

  -- **出来事の時刻と内容の鍵は消去でも動かない** —— 動かせるなら消去の顔で改竄できる。
  -- （`event_time` は即時の錠が既に凍結しているが、ここでも印として見る）
  -- **`payload` も空でなければ消去ではない**（ST03 の R95）。
  is_erasure :=
        NEW.raw = '' AND OLD.raw <> ''
    AND NEW.payload = '{}'::jsonb
    AND NEW.event_time   = OLD.event_time
    AND NEW.content_hash = OLD.content_hash;

  IF NOT is_erasure THEN
    RAISE EXCEPTION '主張は書き換えられない。通るのは本文の消去だけ（FR-44 / FR-51 / 深掘り Q1）';
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM core.erasure_ledger l
     WHERE l.event_id = NEW.id AND l.scope = 'event'
       AND l.txid = pg_current_xact_id()
  ) THEN
    RAISE EXCEPTION '主張の消去は、同じまとまりにその主張の消去の台帳の行があるときだけ通る（FR-51 / 深掘り Q1）';
  END IF;
  RETURN NULL;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_claim_requires_ledger ON core.event;
CREATE CONSTRAINT TRIGGER event_claim_requires_ledger
  AFTER UPDATE ON core.event
  DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_claim_erasure_ledger();

-- ------------------------------------------------------------------ 主張の行の削除（D2 の 3）
--
-- **表の切り詰めは ST03 の文トリガ（`event_no_truncate`）が既に拒む**ので、
-- ここで足すのは行の削除だけ。
CREATE OR REPLACE FUNCTION core.reject_claim_delete() RETURNS trigger AS $fn$
BEGIN
  IF OLD.logical_source = 's01-attribute' THEN
    RAISE EXCEPTION '主張は行ごと消せない（FR-44 / 深掘り C1）。消すなら削除の印か台帳つきの消去';
  END IF;
  RETURN OLD;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_claim_no_delete ON core.event;
CREATE TRIGGER event_claim_no_delete
  BEFORE DELETE ON core.event
  FOR EACH ROW EXECUTE FUNCTION core.reject_claim_delete();

-- ------------------------------------------------------------------ 主張の論理ソース（D1）
--
-- **`external_id_kind = 'none'`**（design D1）—— 粒度が「無し」のソースでは、
-- 送り主が外部識別子を付けても重複の判定にも更新の経路にも乗らない
-- （`crates/server/src/lib.rs` の `place_identifiers`）。主張は `external_id` を持たないので、
-- 一意索引 `event_dedup_hash`（`WHERE external_id IS NULL`）で内容の鍵によって畳まれる ——
-- **同じ原文の再送が 1 件に畳まれる**のはこの経路（深掘り C2）。
--
-- `expected_gap_sec` は稼働状況に使われない（`coverage::must_sources()` は NFR-13 の 5 ソースだけ）が、
-- 列が `NOT NULL` なので 1 日を入れる。
INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
VALUES ('s01-attribute', '個人属性', 86400, 'none')
ON CONFLICT (logical_source) DO NOTHING;
