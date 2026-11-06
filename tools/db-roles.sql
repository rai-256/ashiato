-- 配布物と開発環境で共有する DB 役割の SQL。秘密は psql 変数へ環境変数から取り込む。
\getenv admin_pw ADMIN_PW
\getenv owner_pw OWNER_PW
\getenv app_pw APP_PW
SELECT format('ALTER ROLE %I PASSWORD %L', :'admin', :'admin_pw') \gexec
SELECT 'CREATE ROLE ashiato_owner LOGIN' WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ashiato_owner') \gexec
SELECT format('ALTER ROLE ashiato_owner LOGIN CREATEDB NOSUPERUSER NOCREATEROLE PASSWORD %L', :'owner_pw') \gexec
SELECT 'CREATE ROLE ashiato_app LOGIN' WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ashiato_app') \gexec
SELECT format('ALTER ROLE ashiato_app LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE PASSWORD %L', :'app_pw') \gexec
SELECT format('REVOKE %I FROM ashiato_app', r.rolname) FROM pg_auth_members m JOIN pg_roles r ON r.oid = m.roleid WHERE m.member = (SELECT oid FROM pg_roles WHERE rolname = 'ashiato_app') \gexec
SELECT format('ALTER DATABASE %I OWNER TO ashiato_owner', :'dbname') \gexec
DO $$
DECLARE r record;
BEGIN
  IF EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'core') THEN ALTER SCHEMA core OWNER TO ashiato_owner; END IF;
  FOR r IN SELECT c.relkind, format('%I.%I', n.nspname, c.relname) AS name FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'core' AND c.relkind IN ('r','p','v','m','f','S') AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.objid = c.oid AND d.deptype IN ('a','i','e')) LOOP
    EXECUTE format('ALTER %s %s OWNER TO ashiato_owner', CASE r.relkind WHEN 'S' THEN 'SEQUENCE' WHEN 'v' THEN 'VIEW' WHEN 'm' THEN 'MATERIALIZED VIEW' WHEN 'f' THEN 'FOREIGN TABLE' ELSE 'TABLE' END, r.name);
  END LOOP;
  FOR r IN SELECT p.prokind, p.oid::regprocedure AS name FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = 'core' AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.objid = p.oid AND d.deptype = 'e') LOOP
    EXECUTE format('ALTER %s %s OWNER TO ashiato_owner', CASE r.prokind WHEN 'p' THEN 'PROCEDURE' WHEN 'a' THEN 'AGGREGATE' ELSE 'FUNCTION' END, r.name);
  END LOOP;
  FOR r IN SELECT format('%I.%I', n.nspname, t.typname) AS name FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace WHERE n.nspname = 'core' AND t.typtype IN ('e','d','r') AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.objid = t.oid AND d.deptype = 'e') LOOP
    EXECUTE format('ALTER TYPE %s OWNER TO ashiato_owner', r.name);
  END LOOP;
END $$;
