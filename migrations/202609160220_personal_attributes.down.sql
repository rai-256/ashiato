-- 戻し手順（202609160220_personal_attributes）。**一部だけ戻る。不可逆な部分がある** ——
--
-- 1. 錠の関数とトリガは落とす。**主張が書き換えられる状態へ戻る**（FR-44 が
--    実装者の善意でしか守られない状態。この版を当てる前と同じ）。
-- 2. **主張の行（`core.event` の `s01-attribute`）は消さない。** 消す手段を持たせない
--    のがこの版の目的なので、戻しでそれを迂回しない。
-- 3. **主張の行か種類の行が 1 つでも残っていれば、登録簿の行も種類の 2 表も残す**（design D12 /
--    spec-review R20 / review/code.md R7）。主張の原文は種類を**識別子で**指すので、
--    種類の表を落とすと「その識別子が何という名前だったか」が戻しで永久に失われる。
--    外部キーで当たるのに任せると戻しが途中で止まるので、条件を明示して落とす。
--
--    **種類の側も見る。** 「主張 0 件」だけを条件にしていたときは、**本人が足した種類
--    （副業・同居など）が戻しで消えた** —— まだ主張を書いていない種類は、本人が名前を
--    決めたという事実そのものが成果物で、台帳は追記のみで作り直せない（実測。review/code.md R7）。
--
-- 2 表と登録簿の行が落ちるのは、**主張も種類も 1 つも無いとき**だけ（＝当てたが使わなかった場合）。

DROP TRIGGER IF EXISTS event_claim_no_delete ON core.event;
DROP TRIGGER IF EXISTS event_claim_requires_ledger ON core.event;
DROP TRIGGER IF EXISTS event_claim_immutable ON core.event;
DROP FUNCTION IF EXISTS core.reject_claim_delete();
DROP FUNCTION IF EXISTS core.require_claim_erasure_ledger();
DROP FUNCTION IF EXISTS core.reject_claim_rewrite();

DO $do$
BEGIN
  IF EXISTS (SELECT 1 FROM core.event WHERE logical_source = 's01-attribute')
     OR EXISTS (SELECT 1 FROM core.attribute_kind) THEN
    RAISE NOTICE '主張か種類が残っているので、種類の 2 表と登録簿の行は残す（design D12）';
    RETURN;
  END IF;

  -- 台帳の錠は自分で外してから落とす（DELETE / TRUNCATE を拒むトリガが付いている）
  DROP TRIGGER IF EXISTS attribute_kind_name_no_truncate ON core.attribute_kind_name;
  DROP TRIGGER IF EXISTS attribute_kind_name_append_only ON core.attribute_kind_name;
  DROP TRIGGER IF EXISTS attribute_kind_no_truncate ON core.attribute_kind;
  DROP TRIGGER IF EXISTS attribute_kind_append_only ON core.attribute_kind;
  DROP TABLE IF EXISTS core.attribute_kind_name;
  DROP TABLE IF EXISTS core.attribute_kind;
  DROP FUNCTION IF EXISTS core.reject_attribute_kind_change();
  DELETE FROM core.source WHERE logical_source = 's01-attribute';
END;
$do$;
