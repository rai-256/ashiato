-- 戻し手順（202610181200_media_capture）。**一部だけ戻る。不可逆な部分がある** ——
--
-- 1. **3 つの表のどれかに行が 1 つでもあれば、3 つとも落とさずに残す**（錠の関数とトリガも）。
--    原本の受け取りと欠けは、端末から消えたら二度と作れない（ST19 の移行の前例）。
-- 2. 生存信号の `network` は、非 NULL の行が 1 つでもあれば列を残す（送った時点の接続は作り直せない）。
-- 3. 4 つのソースの `external_id_kind`（record のまま）と、当て直した `collection_started_on` は戻らない
--    （記録の識別子との対応が切れる・前の値を持っていない）。
-- 4. 足した 3 つの登録簿の行は、参照（記録・生存信号・残した表）が無いときだけ消す（あれば残す）。
-- 5. `core.source.started_on_basis` は落とす（記録から引き直せば元の値に戻る）。
--
-- 表と欄が落ちるのは、行が 1 つも無いときだけ。

DO $do$
DECLARE
  has_rows boolean := false;
  has_network boolean := false;
  src text;
BEGIN
  -- 2 回目の戻しでは表がもう無い。表が在るときだけ中を数える
  IF to_regclass('core.media_original') IS NOT NULL THEN
    EXECUTE 'SELECT EXISTS (SELECT 1 FROM core.media_original)' INTO has_rows;
  END IF;
  IF NOT has_rows AND to_regclass('core.media_missing') IS NOT NULL THEN
    EXECUTE 'SELECT EXISTS (SELECT 1 FROM core.media_missing)' INTO has_rows;
  END IF;
  IF NOT has_rows AND to_regclass('core.text_extraction') IS NOT NULL THEN
    EXECUTE 'SELECT EXISTS (SELECT 1 FROM core.text_extraction)' INTO has_rows;
  END IF;

  IF has_rows THEN
    RAISE NOTICE '原本の受け取り・欠け・文字抽出の行が残っているので、3 つの表は残す（ST09 / design D15）';
  ELSE
    DROP TRIGGER IF EXISTS text_extraction_no_truncate ON core.text_extraction;
    DROP TRIGGER IF EXISTS text_extraction_append_only ON core.text_extraction;
    DROP TRIGGER IF EXISTS media_missing_no_truncate ON core.media_missing;
    DROP TRIGGER IF EXISTS media_missing_append_only ON core.media_missing;
    DROP TRIGGER IF EXISTS media_original_no_truncate ON core.media_original;
    DROP TRIGGER IF EXISTS media_original_append_only ON core.media_original;
    DROP TABLE IF EXISTS core.text_extraction;
    DROP TABLE IF EXISTS core.media_missing;
    DROP TABLE IF EXISTS core.media_original;
    DROP FUNCTION IF EXISTS core.reject_media_change();
  END IF;

  IF EXISTS (SELECT 1 FROM information_schema.columns
              WHERE table_schema = 'core' AND table_name = 'heartbeat' AND column_name = 'network') THEN
    EXECUTE 'SELECT EXISTS (SELECT 1 FROM core.heartbeat WHERE network IS NOT NULL)' INTO has_network;
    IF has_network THEN
      RAISE NOTICE '接続の有無の入った生存信号が残っているので、network 列は残す';
    ELSE
      ALTER TABLE core.heartbeat DROP COLUMN network;
    END IF;
  END IF;

  ALTER TABLE core.source DROP CONSTRAINT IF EXISTS source_started_on_basis_known;
  ALTER TABLE core.source DROP COLUMN IF EXISTS started_on_basis;

  -- 足した 3 つのソースの行。参照が残っていれば残す
  FOREACH src IN ARRAY ARRAY['c01-screenshot', 'c01-image', 'c01-video'] LOOP
    BEGIN
      DELETE FROM core.source WHERE logical_source = src;
    EXCEPTION WHEN foreign_key_violation THEN
      RAISE NOTICE '% は参照が残っているので登録簿の行を残す', src;
    END;
  END LOOP;
END
$do$;
