-- 写しからの読み直しで格納か印付けに続けて失敗した回数（ST12 / final review 第 3 回 R82 / design D22-a）。
-- 前進のみ。戻し手順は 202610051730_archive_reread_failure.down.sql に置く。
--
-- **書庫のハッシュで数える。** 確認待ちを読み直す経路（`ingest_confirmed_pending`）と、解析器の版を上げて
-- 読み直す経路（`reparse_older_versions`）は、置き場のファイルを持たない（書庫は「取り込み済み」へ移っている）。
-- 置き場のパスで数える `core.archive_sighting` には行が無いか、走査が消すので、ここでは数えられない。
-- 数えなかったときは、格納か印付けが落ち続けると走査のたびに写しを全件読み直し、台帳にも画面にも何も出なかった。
-- 書き換えてよい観測値（台帳ではない）。読み直しに成功したら行を消す。
CREATE TABLE IF NOT EXISTS core.archive_reread_failure (
  user_id uuid NOT NULL,
  sha256 text NOT NULL,
  consecutive_failures integer NOT NULL DEFAULT 0 CHECK (consecutive_failures >= 0),
  retry_after timestamptz,
  PRIMARY KEY (user_id, sha256)
);
