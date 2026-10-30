-- アプリの役割（ashiato_app）への付与（ST28 / design D4）。
-- `migrate()` が MIGRATIONS を当てた後に**毎回**当てる。配列の外に置くのは、並走中の change が
-- 配列の末尾へ移行を足しても、その移行の表へこの付与が必ず届くようにするため。
-- 何度当てても同じ結果になる。**TRUNCATE / REFERENCES / TRIGGER は付けない。**
GRANT USAGE ON SCHEMA core TO ashiato_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA core TO ashiato_app;
REVOKE TRUNCATE, REFERENCES, TRIGGER ON ALL TABLES IN SCHEMA core FROM ashiato_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA core TO ashiato_app;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA core TO ashiato_app;

ALTER DEFAULT PRIVILEGES FOR ROLE ashiato_owner IN SCHEMA core
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ashiato_app;
ALTER DEFAULT PRIVILEGES FOR ROLE ashiato_owner IN SCHEMA core
  GRANT USAGE, SELECT ON SEQUENCES TO ashiato_app;
ALTER DEFAULT PRIVILEGES FOR ROLE ashiato_owner IN SCHEMA core
  GRANT EXECUTE ON FUNCTIONS TO ashiato_app;

-- 更新の履歴は読むことと足すことだけ（ST51 / design D10）。全部の表への付与の後ろで UPDATE・DELETE を外す。
-- 書き換えの門は所有者の接続でも断る。実行時の役割は起動の行を書く（D2）ので INSERT は残す。
REVOKE UPDATE, DELETE ON core.update_history FROM ashiato_app;
