-- 書き換えと消去の門（ST03 / 深掘り Q10 / Q17 / Q23。design D4 / D5 / D10）**BREAKING**
-- 前進のみ。戻し手順は migrations/202609120944_gates.down.sql に置く。
--
-- ============================================================================
-- **この版は 0002 / 0004 が凍結した 4 列を開ける。**
-- `tools/check-immutable.sh` はその 4 列の UPDATE が**拒まれること**を CI で検査していた。
-- 作り替えないと CI が落ち、**検査を消すと守りが黙って消える**（R38）。
-- 作り替えた台本は「履歴を書かない書き換えは拒まれる / 書けば通る」の 2 本になっている。
-- ============================================================================
--
-- 門の最終形（D4）:
--   **同じトランザクションに履歴行がある書き換え、または同じトランザクションに
--   台帳行がある消去だけを通す。**
-- 本表と履歴の**両方**に置く —— 片方だけだと、本表は消せるのに履歴が消せない
-- 「消せない DB」が残る（R41 / R52）。
--
-- 落ちるのは **COMMIT の瞬間**（`DEFERRABLE INITIALLY DEFERRED`）。だから
-- **取り込みは 1 件 1 トランザクション**でなければならない（D3）——
-- まとめ送りを 1 トランザクションにすると 1 件の失敗が全件を巻き戻す。

-- ------------------------------------------------------------------ 本表の即時の錠
--
-- 0004 から**開けるもの**: `raw` / `payload` / `event_time` / `content_hash`
--   （Q1 の更新。ただし下の遅延制約トリガが履歴か台帳を要求する）
-- 0004 から**据え置くもの**: `origin` / `ingest_time`
-- **足すもの**（D10 / R54）: `external_id` / `external_ref`
--   `external_id` が凍結一覧に無いと、Q6 の部分索引の下では
--   **識別子を 1 文書き換えるだけで同じ本文が 2 行入る**（実測。履歴も台帳も残らない）。
--   `source_updated_at` は凍結しない —— 更新のたびに動く列。
CREATE OR REPLACE FUNCTION core.reject_collected_rewrite() RETURNS trigger AS $fn$
BEGIN
  IF OLD.origin = 'collected' THEN
    -- 分類そのものを動かせないようにする（0004 の 3 手の迂回）
    IF NEW.origin IS DISTINCT FROM OLD.origin THEN
      RAISE EXCEPTION '収集した記録の由来は変えられない（FR-25 / FR-30）';
    END IF;
    IF NEW.ingest_time IS DISTINCT FROM OLD.ingest_time THEN
      RAISE EXCEPTION '収集した記録の格納の時刻は書き換えられない（FR-30）';
    END IF;
    -- **識別子は来歴**（D10）。動かすと同じ本文が 2 行に増える
    IF NEW.external_id IS DISTINCT FROM OLD.external_id THEN
      RAISE EXCEPTION '収集した記録の外部識別子は書き換えられない（FR-23 / FR-30）';
    END IF;
    IF NEW.external_ref IS DISTINCT FROM OLD.external_ref THEN
      RAISE EXCEPTION '収集した記録の対象の識別子は書き換えられない（FR-23 / FR-30）';
    END IF;
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;

-- ------------------------------------------------------------------ 本表の門（COMMIT 時）
CREATE OR REPLACE FUNCTION core.require_version_or_ledger() RETURNS trigger AS $fn$
DECLARE
  content_changed boolean;
  is_erasure      boolean;
BEGIN
  IF OLD.origin <> 'collected' THEN
    RETURN NULL;   -- 「本人が書いた」「派生させた」記録は対象外（FR-30 の範囲）
  END IF;

  content_changed :=
       NEW.raw          IS DISTINCT FROM OLD.raw
    OR NEW.payload      IS DISTINCT FROM OLD.payload
    OR NEW.event_time   IS DISTINCT FROM OLD.event_time
    OR NEW.content_hash IS DISTINCT FROM OLD.content_hash;

  IF NOT content_changed THEN
    RETURN NULL;   -- 論理削除・感度・更新時刻だけの書き換えは素通し（FR-50 / PERM-2）
  END IF;

  -- **消去の形だけを台帳の側へ回す**（R51）。台帳の行があるだけで通すと、
  -- 台帳を 1 行書くだけで改竄が通った（実測）。
  -- 原文が空であることを印にできるのは、`ingest.rs` が空の原文を受け口で断っているため
  -- （`raw_that_cannot_be_stored_is_rejected` が固定している）。
  -- **出来事の時刻と冪等キーは消去でも動かない** —— 動かせるなら消去の顔で改竄できる。
  is_erasure :=
        NEW.raw = '' AND OLD.raw <> ''
    AND NEW.event_time   = OLD.event_time
    AND NEW.content_hash = OLD.content_hash;

  IF is_erasure THEN
    IF NOT EXISTS (
      SELECT 1 FROM core.erasure_ledger l
       WHERE l.event_id = NEW.id AND l.scope = 'event'
         AND l.txid = pg_current_xact_id()
    ) THEN
      RAISE EXCEPTION '本文の消去は、同じまとまりに消去の台帳の行があるときだけ通る（FR-51 / 深掘り Q23）';
    END IF;
    -- **親を消すなら履歴も同じまとまりで消す**（R40 / tasks 7.4c）。
    -- 分けると 2 段目（履歴だけを残す操作）がそのまま開口部になる。
    IF EXISTS (
      SELECT 1 FROM core.event_version v WHERE v.event_id = NEW.id AND v.raw <> ''
    ) THEN
      RAISE EXCEPTION '消去は親とその記録のすべての履歴を同じまとまりで行う（深掘り Q23 / R40）';
    END IF;
    RETURN NULL;
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM core.event_version v
     WHERE v.event_id = NEW.id AND v.txid = pg_current_xact_id()
  ) THEN
    RAISE EXCEPTION '収集した記録の書き換えは、同じまとまりに前の版の履歴があるときだけ通る（FR-30 / 深掘り Q10）';
  END IF;
  RETURN NULL;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_requires_version ON core.event;
CREATE CONSTRAINT TRIGGER event_requires_version
  AFTER UPDATE ON core.event
  DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_version_or_ledger();

-- ------------------------------------------------------------------ 履歴の錠と門
--
-- Q17 の答え（追記のみ。通すのは削除の印・感度・本文の消去だけ）は、Q21 で
-- 感度と削除の列が消えた結果、**通す必要がある開口部が本文の消去 1 つだけ**になった。
-- 閉じ切ると FR-51「消去は履歴に残した前の版にも及ぶ」が満たせない（R65）。
CREATE OR REPLACE FUNCTION core.reject_version_rewrite() RETURNS trigger AS $fn$
BEGIN
  IF  NEW.id                IS DISTINCT FROM OLD.id
   OR NEW.event_id          IS DISTINCT FROM OLD.event_id
   OR NEW.user_id           IS DISTINCT FROM OLD.user_id
   OR NEW.logical_source    IS DISTINCT FROM OLD.logical_source
   OR NEW.version_no        IS DISTINCT FROM OLD.version_no
   OR NEW.event_time        IS DISTINCT FROM OLD.event_time
   OR NEW.content_hash      IS DISTINCT FROM OLD.content_hash
   OR NEW.source_updated_at IS DISTINCT FROM OLD.source_updated_at
   OR NEW.external_ref      IS DISTINCT FROM OLD.external_ref
   OR NEW.superseded_at     IS DISTINCT FROM OLD.superseded_at
   OR NEW.txid              IS DISTINCT FROM OLD.txid
  THEN
    RAISE EXCEPTION '履歴は追記のみ。本文の消去以外の書き換えはできない（深掘り Q17）';
  END IF;
  -- 残るのは raw / payload。**空へ落とす向きだけ**を通す
  IF NEW.raw <> '' THEN
    RAISE EXCEPTION '履歴の原文は本文の消去以外で書き換えられない（深掘り Q17 / FR-18）';
  END IF;
  RETURN NEW;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_version_immutable ON core.event_version;
CREATE TRIGGER event_version_immutable
  BEFORE UPDATE ON core.event_version
  FOR EACH ROW EXECUTE FUNCTION core.reject_version_rewrite();

CREATE OR REPLACE FUNCTION core.require_ledger_for_erasure() RETURNS trigger AS $fn$
BEGIN
  IF NEW.raw IS NOT DISTINCT FROM OLD.raw AND NEW.payload IS NOT DISTINCT FROM OLD.payload THEN
    RETURN NULL;
  END IF;
  IF NOT EXISTS (
    SELECT 1 FROM core.erasure_ledger l
     WHERE l.event_id = NEW.event_id AND l.txid = pg_current_xact_id()
  ) THEN
    RAISE EXCEPTION '履歴の本文の消去は、同じまとまりに消去の台帳の行があるときだけ通る（FR-51 / 深掘り Q23）';
  END IF;
  RETURN NULL;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS version_requires_ledger ON core.event_version;
CREATE CONSTRAINT TRIGGER version_requires_ledger
  AFTER UPDATE ON core.event_version
  DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_ledger_for_erasure();

-- ------------------------------------------------------------------ 台帳の錠
--
-- **開ける必要のある操作が 1 つも無い**（R50）。ここを追記のみにすると
-- Q10 → Q17 → Q23 の連鎖が止まる（同じ問いが 1 段外へ出ない）。
CREATE OR REPLACE FUNCTION core.reject_ledger_change() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '消去の台帳は追記のみ。書き換えも削除もできない（FR-51 / 深掘り Q23）';
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS erasure_ledger_append_only ON core.erasure_ledger;
CREATE TRIGGER erasure_ledger_append_only
  BEFORE UPDATE OR DELETE ON core.erasure_ledger
  FOR EACH ROW EXECUTE FUNCTION core.reject_ledger_change();

-- ------------------------------------------------------------------ 行の削除と表の切り詰め
--
-- **いまの 0002 / 0004 / 0006 は `UPDATE` しか見ていない**（D5。実測で
-- `DELETE FROM core.event` が 3 行消し、`TRUNCATE` も通った）。
-- 行トリガは `TRUNCATE` で撃たれないので、**文トリガを別に置く**。
CREATE OR REPLACE FUNCTION core.reject_collected_delete() RETURNS trigger AS $fn$
BEGIN
  IF OLD.origin = 'collected' THEN
    RAISE EXCEPTION '収集した記録は行ごと消せない（FR-30 / design D5）';
  END IF;
  RETURN OLD;
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_no_delete ON core.event;
CREATE TRIGGER event_no_delete
  BEFORE DELETE ON core.event
  FOR EACH ROW EXECUTE FUNCTION core.reject_collected_delete();

CREATE OR REPLACE FUNCTION core.reject_truncate() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '表の切り詰めはできない（FR-30 / FR-51 / design D5）';
END;
$fn$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION core.reject_version_delete() RETURNS trigger AS $fn$
BEGIN
  RAISE EXCEPTION '履歴の行は削除できない（深掘り Q17）';
END;
$fn$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS event_version_no_delete ON core.event_version;
CREATE TRIGGER event_version_no_delete
  BEFORE DELETE ON core.event_version
  FOR EACH ROW EXECUTE FUNCTION core.reject_version_delete();

DROP TRIGGER IF EXISTS event_no_truncate ON core.event;
CREATE TRIGGER event_no_truncate
  BEFORE TRUNCATE ON core.event
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_truncate();

DROP TRIGGER IF EXISTS event_version_no_truncate ON core.event_version;
CREATE TRIGGER event_version_no_truncate
  BEFORE TRUNCATE ON core.event_version
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_truncate();

DROP TRIGGER IF EXISTS erasure_ledger_no_truncate ON core.erasure_ledger;
CREATE TRIGGER erasure_ledger_no_truncate
  BEFORE TRUNCATE ON core.erasure_ledger
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_truncate();

-- 生存信号も同じ扱い（0006 は UPDATE と DELETE を見ているが、`TRUNCATE` は素通しだった）
DROP TRIGGER IF EXISTS heartbeat_no_truncate ON core.heartbeat;
CREATE TRIGGER heartbeat_no_truncate
  BEFORE TRUNCATE ON core.heartbeat
  FOR EACH STATEMENT EXECUTE FUNCTION core.reject_truncate();
