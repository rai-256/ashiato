# 機械が承認した検証コマンドの訂正を、壊して確かめた記録（code_verify 手 7）

`python3 scripts/plan_fix.py pending openspec/changes/st23-record-erasure` → 1 件（1.1）。

## 1.1 2026-10-23T23:42:23+00:00
- 壊したもの: 1.1 が守る派生の門の「解析済みを残して原文だけ空にする書き換えは拒む」分岐を外した —— `migrations/202610240500_record_purge.sql:135-137`（`IF NOT is_erasure THEN RAISE EXCEPTION '滞在の本文の消去は、原文と解析済みを一緒に空にする形だけが通る…'; END IF;`）を削除。作業ツリーの複製（`/tmp/st23cv/mut`。HEAD 9c6f2459 の作業ツリーを rsync した独立の git、別の compose project・`ASHIATO_DB_PORT=55791`）で当てた。作業ツリーでは走らせていない —— `tools/check-immutable.sh` は終了時に `docker compose down -v` を撃つので、作業ツリーで走らせると `ashiato2-st23-db-1` の volume が消える。複製は当てた後に `cp` で元に戻し、作業ツリーの `git status --porcelain` は壊す前と後で同じ（どちらもこのファイルと `review/code.md` 以外に出ない）
- 実行: tools/check-st23-derived-erasure.sh
- 結果: 落ちた

補足（rc と出力の要点）: rc=1。`check-immutable.sh` の ST23 の節が `  NG 解析済みを残して原文だけ空にする書き換えが通った` を出し、末尾が `書き換え禁止 NG`。
同じ複製で `erasure_span_no_truncate` のトリガを作らない形に壊しても rc=1（`NG 消去の操作の台帳を消せた・切り詰められた: TRUNCATE core.erasure_span;`）。
壊さない複製では rc=0（`OK 節の OK 行 36 本`）。
ただし、**節の中身を空にした**複製でも rc=0（`OK 節の OK 行 25 本`）—— 訂正の入口は門を壊せば落ちるが、節が何も確かめなくなったことは捕まえない。`review/code.md` の R17 に書いた（手 2 の指摘。この訂正の結果は「落ちた」のまま）。
