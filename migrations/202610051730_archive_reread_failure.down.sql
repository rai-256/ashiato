-- 202610051730_archive_reread_failure.sql の戻し。読み直しの失敗の回数（書き換えてよい観測値）を落とす。
-- 不可逆: 続けて失敗した回数と 1 時間待つ時刻が失われる（台帳の `store_failed` の行は残る）。
-- 戻した後は、印付けが落ち続ける書庫を走査のたびに写しから読み直す。
DROP TABLE IF EXISTS core.archive_reread_failure;
