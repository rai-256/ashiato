-- 戻し手順（202609142125_stays）。**一部だけ戻る。不可逆な部分がある** ——
--
-- 1. 基準の台帳と吸収の台帳を落とす。**「どの基準で作ったか」の版の一覧と、
--    「どの滞在に吸収されたか」の記録が失われる**（滞在の行の `raw` に基準の値は残る）。
-- 2. **`core.event` の `s01-stay` の行と履歴の行は消せない**（`core.event_version` の門が行の削除を拒み、
--    版を持つ滞在は FK で消せない）。代わりに読み出しから外す印を付ける。
-- 3. 登録簿の `s01-stay` の行は、`core.event` の FK が指しているので残す。
UPDATE core.event
   SET deleted_at = coalesce(deleted_at, now()),
       deleted_by = 'rebuild:rolled-back'
 WHERE logical_source = 's01-stay' AND origin = 'derived' AND deleted_at IS NULL;

DROP TRIGGER IF EXISTS stay_absorbed_no_truncate ON core.stay_absorbed;
DROP TRIGGER IF EXISTS stay_absorbed_append_only ON core.stay_absorbed;
DROP TRIGGER IF EXISTS stay_criteria_no_truncate ON core.stay_criteria;
DROP TRIGGER IF EXISTS stay_criteria_append_only ON core.stay_criteria;
DROP TABLE IF EXISTS core.stay_absorbed;
DROP TABLE IF EXISTS core.stay_criteria;
DROP FUNCTION IF EXISTS core.reject_stay_ledger_change();
