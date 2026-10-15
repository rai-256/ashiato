-- 戻し手順。記録が 1 件でも残っていれば登録を残す。記録を消す down は書かない（design D8）。
-- 出来事・稼働記録・破棄の報告・台帳など、`core.source` を参照する表のどれかに行があれば外部キーが拒むので、
-- その拒みを「残す」として扱う（参照する表を 1 つずつ数え上げて、後から足された表を取りこぼさない）。
DO $do$
BEGIN
  DELETE FROM core.source WHERE logical_source = 'c03-myactivity-youtube';
EXCEPTION WHEN foreign_key_violation THEN
  RAISE NOTICE 'c03-myactivity-youtube の記録が残っているので、登録は残す（design D8）';
END
$do$;
