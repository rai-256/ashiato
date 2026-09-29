-- 詳細の件数集計（ST22 / design D8）が利用するライブ行の検索経路。
-- event_live の削除済み除外を索引の述語にも反映し、(user_id, event_time) の範囲検索を先頭から受ける。
CREATE INDEX IF NOT EXISTS event_by_user_time_live
  ON core.event (user_id, event_time)
  WHERE deleted_at IS NULL;
