#!/usr/bin/env bash
# 「収集した」記録が書き換えられないことを、**わざと UPDATE を投げて**確かめる
# （製造準備 C の作法 / design D3 / FR-30）。
#
# アプリを通さず psql から直に叩く。**素通りする経路がここだから** ——
# 同じ PC の第三者製プラグインや手作業の psql を止められるのは DB の側だけ。
set -euo pipefail
cd "$(dirname "$0")/.."
cleanup() { docker compose down -v >/dev/null 2>&1 || true; }
trap cleanup EXIT

psql() { docker compose exec -T db psql -qtA -v ON_ERROR_STOP=1 -U ashiato -d ashiato "$@"; }

echo "== DB を起動してマイグレーションを当てる"
# **当てる版と順は `crates/server/src/lib.rs` の `MIGRATIONS` 配列から引く**
# （2026-09-12 / ST03）。手で並べていたときは ST02 の `202609112113_source_lifecycle` が
# **この検査にだけ入っておらず**、本番と違う schema を検査していた。
# 版を足してここへ足し忘れる、が構造として起きない形にする。
mapfile -t MIGS < <(grep -oE 'migrations/[0-9]{12}_[a-z_]+\.sql' crates/server/src/lib.rs \
                    | sed 's|migrations/||; s|\.sql$||')
[ "${#MIGS[@]}" -ge 12 ] || { echo "  NG lib.rs から版を引けない（${#MIGS[@]} 件）"; exit 1; }
docker compose up -d --wait db >/dev/null
for m in "${MIGS[@]}"; do
  psql < "migrations/$m.sql" >/dev/null || { echo "  NG $m が当たらない"; exit 1; }
done
echo "  OK ${#MIGS[@]} 版を当てた"

# **2 回当てても壊れないことを、ここで確かめる**（review R12）。
# run() は起動のたびに全版を当てるので、当て直しが安全でないと 2 回目の起動で落ちる。
# 0003 は「型が text なら何もしない」分岐を持っているが、その分岐を通る検査がどこにも無かった。
echo "== もう一度当てる（run() は起動のたびに全版を当てる）"
for m in "${MIGS[@]}"; do
  psql < "migrations/$m.sql" >/dev/null || { echo "  NG $m の 2 回目が落ちた"; exit 1; }
done
echo "  OK ${#MIGS[@]} 版とも当て直せる"

# **当て直しで `external_id_kind` が緩い側へ落ちないこと**（ST03 / 深掘り Q16）。
# 移行が「列を足した回だけ」で絞らず毎回 `'none'` を撃つ形だと、
# **後から登録した外部ソースが再起動のたびに識別子を要求しなくなる**（黙って守りが消える）。
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('ext-kind-check','当て直しの確認用',21600,'record') ON CONFLICT DO NOTHING;" >/dev/null
psql < migrations/202609120940_source_columns.sql >/dev/null
kind=$(psql -c "SELECT external_id_kind FROM core.source WHERE logical_source='ext-kind-check';")
[ "$kind" = "record" ] \
  || { echo "  NG 当て直しで external_id_kind が $kind へ落ちた（識別子の要求が黙って消える）"; exit 1; }
echo "  OK 当て直しても external_id_kind は落ちない"

# **0005 を当て直しても稼働記録が消えないこと**（review/spec.md の型の穴）。
# 0005 は古い形のときだけ表を作り直すが、その分岐が壊れると
# **起動のたびに稼働記録が全部消える**（毎回 0 件なので画面は「ずっと途絶」に見える）。
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('rebuild-check','当て直しの確認用',21600,'none') ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.coverage (user_id, logical_source, day, event_count)
         VALUES ('00000000-0000-0000-0000-000000000000','rebuild-check','2026-05-01',7)
         ON CONFLICT DO NOTHING;" >/dev/null
psql < migrations/202609111111_coverage_rebuild.sql >/dev/null
kept=$(psql -c "SELECT count(*) FROM core.coverage WHERE logical_source = 'rebuild-check';")
[ "$kept" = "1" ] || { echo "  NG 0005 の当て直しで稼働記録が消えた"; exit 1; }
echo "  OK 0005 を当て直しても稼働記録は消えない"

echo "== 「収集した」記録を 1 件置く"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('immutable-check','書き換え禁止の確認用',21600,'none') ON CONFLICT DO NOTHING;"  >/dev/null
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ('22222222-2222-4222-8222-222222222222',
                 '00000000-0000-0000-0000-000000000000','immutable-check','collected',
                 '2026-09-08T02:00:00Z',540,'Asia/Tokyo',1,'immutable-check-hash',
                 '{\"hello\":\"world\"}','{\"hello\":\"world\"}');" >/dev/null

# Scenario: 収集した記録は書き換えられない
# Scenario: 履歴を書かない書き換えは拒まれる
#   （**ST03 で落ちる場所が変わった。** 0002 / 0004 は BEFORE UPDATE で即座に拒んでいたが、
#    いまは遅延制約トリガが COMMIT の瞬間に「同じまとまりに履歴があるか」を見る。
#    psql -c の 1 文はそれ自体が 1 トランザクションなので、履歴なしの書き換えはここで落ちる）
fail=0
# 拒まれるべき 3 つ。**それぞれ別に確かめる** —— 1 つだけ効いていて他が素通しでも気付くように
for col in raw payload event_time; do
  case "$col" in
    event_time) val="'2000-01-01T00:00:00Z'" ;;
    *)          val="'{\"hello\":\"tampered\"}'" ;;
  esac
  if psql -c "UPDATE core.event SET $col = $val WHERE logical_source = 'immutable-check';" \
       >/dev/null 2>&1; then
    echo "  NG $col が書き換えられた（FR-30 違反）"; fail=1
  else
    echo "  OK $col の書き換えは拒まれた"
  fi
done

# 中身が本当に変わっていないこと（トリガが例外を投げても書けていた、を潰す）。
# **原文は text なので `raw->>` は引けない**（0003 / design D16）。丸ごと比べる。
got=$(psql -c "SELECT raw FROM core.event WHERE logical_source = 'immutable-check';")
[ "$got" = '{"hello":"world"}' ] || { echo "  NG 原文が変わっている: $got"; fail=1; }

# **表記だけ違う原文への書き換えも拒む。** 0003 で原文が text になって初めて手に入った保証で、
# jsonb の頃は `{"hello": "world"}`（空白違い）が「同値」と見なされて素通りしていた（review R20）。
if psql -c "UPDATE core.event SET raw = '{\"hello\": \"world\"}'
            WHERE logical_source = 'immutable-check';" >/dev/null 2>&1; then
  echo "  NG 表記だけ違う原文への書き換えが通った（0003 の効き目が消えている）"; fail=1
else
  echo "  OK 表記だけ違う原文への書き換えも拒まれた"
fi

# **由来を経由した迂回を塞げているか**（review R2 / design D21 / 0004）。
# 1 手ずつ投げているだけでは見えない経路 —— 'authored' へ移してから書き換え、'collected' へ戻す。
if psql -c "UPDATE core.event SET origin = 'authored'
            WHERE logical_source = 'immutable-check' AND origin = 'collected';" >/dev/null 2>&1; then
  echo "  NG 収集した記録の由来を動かせた（原文の不変が 3 手で迂回できる）"; fail=1
else
  echo "  OK 収集した記録の由来は動かせない"
fi
# 来歴（冪等キー・格納の時刻）も凍結されている
for col in content_hash ingest_time; do
  case "$col" in
    ingest_time) val="'2000-01-01T00:00:00Z'" ;;
    *)           val="'rewritten'" ;;
  esac
  if psql -c "UPDATE core.event SET $col = $val WHERE logical_source = 'immutable-check';" \
       >/dev/null 2>&1; then
    echo "  NG $col が書き換えられた（来歴が動く）"; fail=1
  else
    echo "  OK $col の書き換えは拒まれた"
  fi
done
# 最後に、3 手を通しで打っても原文が変わっていないことを見る
psql -c "UPDATE core.event SET origin='authored' WHERE logical_source='immutable-check';" \
  >/dev/null 2>&1 || true
psql -c "UPDATE core.event SET raw='{\"tampered\":1}' WHERE logical_source='immutable-check';" \
  >/dev/null 2>&1 || true
psql -c "UPDATE core.event SET origin='collected' WHERE logical_source='immutable-check';" \
  >/dev/null 2>&1 || true
got=$(psql -c "SELECT origin||' | '||raw FROM core.event WHERE logical_source='immutable-check';")
[ "$got" = 'collected | {"hello":"world"}' ] \
  || { echo "  NG 3 手の迂回で原文が変わった: $got"; fail=1; }
echo "  OK 3 手を通しても原文は変わらない"

# 論理削除は通らないといけない（tasks 3.2 / FR-50）
if psql -c "UPDATE core.event SET deleted_at = now(), deleted_by = 'check'
            WHERE logical_source = 'immutable-check';" >/dev/null 2>&1; then
  echo "  OK 論理削除は通る"
else
  echo "  NG 論理削除まで止めている（FR-50 を壊している）"; fail=1
fi

# 「本人が書いた」記録は書き換えてよい（禁止の範囲が広がっていないこと）。
# **確かめるのは ST19 の節**（主張の行がある DB で通ることまで見る。spec-review R14）。ここでは行を置くだけ。
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ('33333333-3333-4333-8333-333333333333',
                 '00000000-0000-0000-0000-000000000000','immutable-check','authored',
                 '2026-09-08T03:00:00Z',540,'Asia/Tokyo',1,'authored-hash','{}','{}');" >/dev/null

# --- 生存信号（FR-78 / 深掘り 第 4 回 Q13 / 0006）
#
# **生存信号は証拠である。** 記録が 0 件の日に「動いていなかった」のか「壊れていた」のかを
# 分ける唯一の材料で、書き換えられると扉 #14 の区別がそのまま嘘になる。
#
# **列ごとに投げる。** 0004 が実測で見つけた 3 手の迂回は、1 手ずつ投げているだけでは
# 見えなかった —— 分類を動かせる限り原文の不変は成り立たない。生存信号には分類列を
# 持たせていないが、**「持たせていない」ことも検査する**（後から足されうる）。
echo "== 生存信号を 1 件置く"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('hb-check','生存信号の確認用',21600,'none') ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.heartbeat
           (id, user_id, logical_source, device_id, emitted_at, capturable, blockers,
            attempts, successes, content_hash, raw)
         VALUES ('44444444-4444-4444-8444-444444444444',
                 '00000000-0000-0000-0000-000000000000','hb-check','hb-dev',
                 '2026-09-08T02:00:00Z', true, '{}', 360, 230, 'hb-check-hash',
                 '{\"alive\":true}');" >/dev/null

# Scenario: 格納された生存信号は書き換えられない
for col in raw content_hash received_at emitted_at capturable blockers attempts successes device_id user_id id; do
  case "$col" in
    received_at|emitted_at) val="'2000-01-01T00:00:00Z'" ;;
    capturable)             val="false" ;;
    blockers)               val="ARRAY['forged']" ;;
    attempts|successes)     val="0" ;;
    user_id)                val="'11111111-1111-4111-8111-111111111111'" ;;
    id)                     val="'55555555-5555-4555-8555-555555555555'" ;;
    *)                      val="'forged'" ;;
  esac
  if psql -c "UPDATE core.heartbeat SET $col = $val WHERE logical_source = 'hb-check';" \
       >/dev/null 2>&1; then
    echo "  NG 生存信号の $col が書き換えられた（FR-78 違反）"; fail=1
  else
    echo "  OK 生存信号の $col の書き換えは拒まれた"
  fi
done

# 中身が本当に変わっていない（トリガが例外を投げても書けていた、を潰す）
got=$(psql -c "SELECT raw FROM core.heartbeat WHERE logical_source = 'hb-check';")
[ "$got" = '{"alive":true}' ] || { echo "  NG 生存信号の原文が変わっている: $got"; fail=1; }

# **迂回路になる分類列を持たせていないこと**（0004 の 3 手の迂回と同じ型）。
# core.event は origin を動かしてから原文を書き換え、戻すことで迂回できた。
extra=$(psql -c "SELECT count(*) FROM information_schema.columns
                  WHERE table_schema='core' AND table_name='heartbeat'
                    AND column_name IN ('origin','kind','state','deleted_at','deleted_by');")
[ "$extra" = "0" ] || { echo "  NG 生存信号に分類列・論理削除列がある（迂回路になる）"; fail=1; }
echo "  OK 生存信号に迂回路になる列が無い"

# **削除も拒む**（ST02 の review/code.md の R22 / I10 / H-5）。
#
# 以前ここは `DELETE` が通ったら `ok …` と出し、拒まれたら何も出さず、
# **どちらの分岐でも `fail` を触っていなかった** —— 検査ではなく「検査に見える出力」だった。
# しかも印字していた主張（「アプリに DELETE の経路が無いことが担保」）を
# 裏付けるものがスクリプト内外に無かった。
#
# `UPDATE` だけを止めても **2 手で差し替えられる**:
#   DELETE FROM core.heartbeat WHERE id = '…';
#   INSERT INTO core.heartbeat (… capturable=false …);
# `content_hash` は `logical_source` + `emitted_at` + `raw` から決まるので、
# **同じ鍵のまま中身だけ入れ替えられる**。0004 が実測で見つけた 3 手の迂回と同じ型で、
# 0002 と 0006 が自分で書いた脅威（psql を直に叩く運用）がまさにこの 2 手を打てる。
if psql -c "DELETE FROM core.heartbeat WHERE logical_source = 'hb-check';" >/dev/null 2>&1; then
  echo "  NG 生存信号を削除できた（2 手で証拠を差し替えられる）"; fail=1
else
  echo "  OK 生存信号は削除できない"
fi
# 行が本当に残っていること（例外を投げても消えていた、を潰す）
left=$(psql -c "SELECT count(*) FROM core.heartbeat WHERE logical_source = 'hb-check';")
[ "$left" = "1" ] || { echo "  NG 生存信号が消えている（$left 行）"; fail=1; }


# ================================================================ ST03 の門
#
# **書き換えと消去の門を、取り込み口を通さずに確かめる**（深掘り Q10 / Q17 / Q23 / design D4）。
#
# Scenario: 取り込み口を通さない操作にも同じ制限が掛かる
#   （`cargo test` は取り込み口越しなので、**アプリ層だけの実装でも全部緑になる**。
#    DB の側を観測するのはこのスクリプトだけ —— Q10 の核心はここにある）
#
# **ST03 は 0002 / 0004 が凍結した 4 列を開ける。** 「拒まれること」だけを検査していた
# 台本をそのまま残すと CI が落ち、**消すと守りが黙って消える**（R38）。
# 「履歴を書かない書き換えは拒まれる / 書けば通る」の 2 本に作り替えてある。
echo "== ST03: 書き換えと消去の門"
GID='66666666-6666-4666-8666-666666666666'
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('gate-check','門の確認用',21600,'record') ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, external_id, external_ref, device_id, origin,
            event_time, tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ('$GID','00000000-0000-0000-0000-000000000000','gate-check','gate-ext','gate-ref',
                 'gate-dev','collected','2026-09-08T02:00:00Z',540,'Asia/Tokyo',1,
                 'gate-hash-1','{\"v\":1}','{\"v\":1}');" >/dev/null

# 履歴を 1 行書く SQL。**同じ文字列の中に書くと 1 トランザクションになる**。
#
# **更新前の版を明示して書く**（2026-09-12 / R94）。以前は
# `SELECT … FROM core.event` で写していたが、門が「履歴行が OLD と一致すること」まで
# 見るようになったので、**更新の後に撃つと NEW を写してしまい正しく拒まれる**。
# 順序に依らず通ることを見たいので、写す元をテーブルから外す。
# 引数: $1=version_no / $2=更新前の content_hash / $3=更新前の raw（＝payload にも使う）
#
# **`psql -c "…$(…)…"` の中で呼ばない。** 入れ子の引用で原文に `\` が混ざり、
# OLD と一致しなくなって門が正しく拒む（最初そう書いて 1 度踏んだ）。先に変数へ組む。
version_row_of() {
  printf "INSERT INTO core.event_version
     (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
   VALUES ('%s','00000000-0000-0000-0000-000000000000','gate-check',%s,
           '2026-09-08T02:00:00Z','%s','%s','%s');" "$GID" "$1" "$2" "$3" "$3"
}
VER1="$(version_row_of 1 gate-hash-1 '{"v":1}')"
VER2="$(version_row_of 2 gate-hash-2 '{"v":2}')"
ledger_row="INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
            VALUES ('$GID','00000000-0000-0000-0000-000000000000','gate-check','SCOPE','check');"

# Scenario: 履歴を書かない書き換えは拒まれる
if psql -c "UPDATE core.event SET raw='{\"tampered\":1}', content_hash='gate-hash-x'
            WHERE id='$GID';" >/dev/null 2>&1; then
  echo "  NG 履歴を書かない書き換えが通った（深掘り Q10 の門が効いていない）"; fail=1
else
  echo "  OK 履歴を書かない書き換えは拒まれた"
fi
got=$(psql -c "SELECT raw FROM core.event WHERE id='$GID';")
[ "$got" = '{"v":1}' ] || { echo "  NG 拒まれたのに原文が変わっている: $got"; fail=1; }

# Scenario: 履歴を書けば書き換えが通る
if psql -c "$VER1
            UPDATE core.event SET raw='{\"v\":2}', payload='{\"v\":2}', content_hash='gate-hash-2'
             WHERE id='$GID';" >/dev/null 2>&1; then
  echo "  OK 履歴を書けば書き換えが通る"
else
  echo "  NG 履歴を書いても書き換えが通らない（Q1 の更新が 1 行も動かない）"; fail=1
fi
got=$(psql -c "SELECT raw FROM core.event WHERE id='$GID';")
[ "$got" = '{"v":2}' ] || { echo "  NG 通ったのに内容が新しくなっていない: $got"; fail=1; }
kept=$(psql -c "SELECT raw FROM core.event_version WHERE event_id='$GID' AND version_no=1;")
[ "$kept" = '{"v":1}' ] || { echo "  NG 前の版の原文が履歴に残っていない: $kept"; fail=1; }

# **順序に依存しない**（更新の後に履歴を書いても通る）。制約トリガは COMMIT 時に見る
if psql -c "UPDATE core.event SET raw='{\"v\":3}', payload='{\"v\":3}', content_hash='gate-hash-3'
             WHERE id='$GID';
            $VER2" >/dev/null 2>&1; then
  echo "  OK 履歴を後から書いても通る（門は COMMIT の瞬間に見る）"
else
  echo "  NG 順序に依存している（1 件 1 トランザクションの前提が崩れる）"; fail=1
fi

# **同じまとまりの中で履歴を消しても落ちる**（履歴は削除できないので、行の削除として落ちる）
# Scenario: 履歴は消去以外の書き換えも行の削除もできない
for stmt in "UPDATE core.event_version SET version_no=99 WHERE event_id='$GID';" \
            "UPDATE core.event_version SET raw='{\"forged\":1}' WHERE event_id='$GID';" \
            "UPDATE core.event_version SET superseded_at='2000-01-01' WHERE event_id='$GID';" \
            "DELETE FROM core.event_version WHERE event_id='$GID';" \
            "TRUNCATE core.event_version;"; do
  if psql -c "$stmt" >/dev/null 2>&1; then
    echo "  NG 履歴が変えられた: $stmt"; fail=1
  fi
done
[ "$fail" -eq 0 ] && echo "  OK 履歴は消去以外の書き換えも行の削除も表の切り詰めもできない"
left=$(psql -c "SELECT count(*) FROM core.event_version WHERE event_id='$GID';")
[ "$left" = "2" ] || { echo "  NG 履歴が $left 行になっている（2 行のはず）"; fail=1; }

# Scenario: 台帳を書かない消去は拒まれる
if psql -c "UPDATE core.event SET raw='', payload='{}' WHERE id='$GID';" >/dev/null 2>&1; then
  echo "  NG 台帳を書かない消去が通った（唯一の開口部から原文が全部消える）"; fail=1
else
  echo "  OK 台帳を書かない消去は拒まれた"
fi
got=$(psql -c "SELECT raw FROM core.event WHERE id='$GID';")
[ "$got" = '{"v":3}' ] || { echo "  NG 拒まれたのに本文が消えている: $got"; fail=1; }

# Scenario: 台帳の行があっても、消去でない書き換えは通らない
#   （絞りが無いと、台帳を 1 行書くだけで改竄が通る —— 実測で確かめた経路）
if psql -c "${ledger_row/SCOPE/event}
            UPDATE core.event SET raw='{\"tampered\":2}', content_hash='gate-hash-x'
             WHERE id='$GID';" >/dev/null 2>&1; then
  echo "  NG 台帳を 1 行書くだけで改竄が通った（門が消去の形で絞れていない）"; fail=1
else
  echo "  OK 台帳の行があっても、消去でない書き換えは通らない"
fi

# Scenario: 履歴の本文は台帳が無ければ消せない
if psql -c "UPDATE core.event_version SET raw='', payload='{}' WHERE event_id='$GID';" \
     >/dev/null 2>&1; then
  echo "  NG 台帳なしで履歴の本文が消せた"; fail=1
else
  echo "  OK 履歴の本文は台帳が無ければ消せない"
fi

# Scenario: 履歴の本文は台帳があれば消せる
#   （**閉じ切ると FR-51「消去は履歴に残した前の版にも及ぶ」が満たせない** —— R65）
if psql -c "${ledger_row/SCOPE/version}
            UPDATE core.event_version SET raw='', payload='{}' WHERE event_id='$GID';" \
     >/dev/null 2>&1; then
  echo "  OK 履歴の本文は台帳があれば消せる"
else
  echo "  NG 履歴の本文が消せない（FR-51 が満たせない）"; fail=1
fi
got=$(psql -c "SELECT count(*) FROM core.event_version WHERE event_id='$GID' AND raw <> '';")
[ "$got" = "0" ] || { echo "  NG 通ったのに履歴の本文が残っている（$got 行）"; fail=1; }

# Scenario: 台帳は何をしても変えられない
#   （**開ける必要のある操作が 1 つも無い** —— ここで Q10 → Q17 → Q23 の連鎖が止まる）
for stmt in "UPDATE core.erasure_ledger SET reason='forged' WHERE event_id='$GID';" \
            "UPDATE core.erasure_ledger SET scope='version' WHERE event_id='$GID';" \
            "DELETE FROM core.erasure_ledger WHERE event_id='$GID';" \
            "TRUNCATE core.erasure_ledger;"; do
  if psql -c "$stmt" >/dev/null 2>&1; then
    echo "  NG 台帳が変えられた: $stmt"; fail=1
  fi
done
echo "  OK 台帳は書き換えも削除も切り詰めもできない"
led=$(psql -c "SELECT count(*) FROM core.erasure_ledger WHERE event_id='$GID';")
[ "$led" = "1" ] || { echo "  NG 台帳が $led 行になっている（1 行のはず）"; fail=1; }

# **凍結する列が増えている**（design D10 / R54）。
# `external_id` が開いていると、Q6 の部分索引の下では**識別子を 1 文書き換えるだけで
# 同じ本文が 2 行入る**（履歴も台帳も残らない）。
for col in external_id external_ref; do
  if psql -c "UPDATE core.event SET $col = 'forged' WHERE id='$GID';" >/dev/null 2>&1; then
    echo "  NG $col が書き換えられた（同じ本文が 2 行入る）"; fail=1
  else
    echo "  OK $col の書き換えは拒まれた"
  fi
done
# **`source_updated_at` は凍結しない**（更新のたびに動く列）。止めると Q20 が成り立たない
if psql -c "UPDATE core.event SET source_updated_at = now() WHERE id='$GID';" >/dev/null 2>&1; then
  echo "  OK source_updated_at は動かせる（凍結の範囲が広がっていない）"
else
  echo "  NG source_updated_at まで凍結している（Q20 の更新が当たらない）"; fail=1
fi

# **履歴は感度も削除の印も持たない**（深掘り Q21 / tasks 5.2）。
# 持たせると、親を締めても前の版が緩いまま残る —— 伝播の処理が無ければ書き忘れようがない
extra=$(psql -c "SELECT count(*) FROM information_schema.columns
                  WHERE table_schema='core' AND table_name='event_version'
                    AND column_name IN ('sensitivity','deleted_at','deleted_by');")
[ "$extra" = "0" ] || { echo "  NG 履歴が自分の感度・削除の印を持っている"; fail=1; }
# **履歴の原文は text**（R39。`jsonb` はキー順を変え、重複キーを落とす）
vtype=$(psql -c "SELECT data_type FROM information_schema.columns
                  WHERE table_schema='core' AND table_name='event_version' AND column_name='raw';")
[ "$vtype" = "text" ] || { echo "  NG 履歴の原文が $vtype（前の版はここにしか無い）"; fail=1; }
echo "  OK 履歴は感度も削除の印も持たず、原文は text"

# Scenario: 収集した記録は行ごと消せない
#   （**いまの 0002 / 0004 は UPDATE しか見ていなかった** —— 実測で DELETE が 3 行消した）
for stmt in "DELETE FROM core.event WHERE id='$GID';" \
            "TRUNCATE core.event CASCADE;" \
            "TRUNCATE core.heartbeat;"; do
  if psql -c "$stmt" >/dev/null 2>&1; then
    echo "  NG 記録が行ごと消せた: $stmt"; fail=1
  fi
done
echo "  OK 収集した記録は行ごとも表ごとも消せない"
left=$(psql -c "SELECT count(*) FROM core.event WHERE id='$GID';")
[ "$left" = "1" ] || { echo "  NG 記録が消えている"; fail=1; }

# --- 2026-09-12（実装レビュー R94 / R95 / R97）に足した 4 本。
# **どれも「門があること」ではなく「門が何を見ているか」を観測する** ——
# 足す前は、下の 4 つの改変がどれも緑のまま通った（実測）。
echo "== ST03: 門が「何を見ているか」（R94 / R95 / R97）"

# R94: **でっち上げの履歴では通らない。** 件数だけを見ていたときは、前の版と無関係な
# 履歴を 1 行書けば原文が消えた（本表 {"t":2} / 履歴 {"junk":1} で元の本文はどこにも無い）
if psql -c "INSERT INTO core.event_version
              (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload)
            VALUES ('$GID','00000000-0000-0000-0000-000000000000','gate-check',99,
                    '2000-01-01','junk','{\"junk\":1}','{}');
            UPDATE core.event SET raw='{\"tampered\":3}', content_hash='gate-hash-x'
             WHERE id='$GID';" >/dev/null 2>&1; then
  echo "  NG でっち上げの履歴で原文が書き換えられた（門が中身を見ていない）"; fail=1
else
  echo "  OK 更新前の版と中身の違う履歴では書き換えが通らない"
fi

# R95: **消去の顔で payload だけを差し替えられない。**
# `is_erasure` が payload を見ていなかったときは、台帳 1 行でこれが通った
if psql -c "${ledger_row/SCOPE/event}
            UPDATE core.event SET raw='', payload='{\"forged\":true}' WHERE id='$GID';" \
     >/dev/null 2>&1; then
  echo "  NG 消去の顔で解析済みを差し替えられた（原文は消えているので引き直せない）"; fail=1
else
  echo "  OK 消去は原文と解析済みの両方を空にする形だけが通る"
fi

# R97: **台帳を書いたうえでも、履歴の原文を空以外へは書き換えられない。**
# 台帳の無いまとまりで撃っていたときは、履歴の錠を丸ごと消しても緑のまま通った（空振り）
if psql -c "${ledger_row/SCOPE/version}
            UPDATE core.event_version SET raw='{\"forged\":1}' WHERE event_id='$GID';" \
     >/dev/null 2>&1; then
  echo "  NG 台帳があれば履歴の原文を偽の内容へ書き換えられた"; fail=1
else
  echo "  OK 台帳があっても、履歴の書き換えは本文の消去だけが通る"
fi
# 既に消去された版には、もう通す操作が無い（payload の差し替えも拒む）
if psql -c "${ledger_row/SCOPE/version}
            UPDATE core.event_version SET payload='{\"forged\":2}'
             WHERE event_id='$GID' AND raw='';" >/dev/null 2>&1; then
  echo "  NG 消去済みの履歴の解析済みを差し替えられた"; fail=1
else
  echo "  OK 消去済みの履歴の版は書き換えられない"
fi

# R114: **「収集した」へ由来を付け替えられない**（0004 が閉じたのは*出る*向きだけだった）。
# authored の行を collected にしつつ原文を書き換える 1 文は、履歴も台帳も残さずに通っていた
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ('77777777-7777-4777-8777-777777777777',
                 '00000000-0000-0000-0000-000000000000','gate-check','gate-dev','authored',
                 '2026-09-08T04:00:00Z',540,'Asia/Tokyo',1,'authored-gate','{\"a\":1}','{\"a\":1}');" \
  >/dev/null
if psql -c "UPDATE core.event SET origin='collected', raw='{\"forged\":1}'
             WHERE id='77777777-7777-4777-8777-777777777777';" >/dev/null 2>&1; then
  echo "  NG 「収集した」へ付け替えながら原文を書き換えられた（捏造が固定される）"; fail=1
else
  echo "  OK 「収集した」へ由来を付け替えることはできない"
fi

# R96: **logical_source は冪等キーの入力なので凍結する**（動かすと以後どの再送とも一致しない）
for col in logical_source id; do
  case "$col" in
    id) val="'88888888-8888-4888-8888-888888888888'" ;;
    *)  val="'gate-check'" ;;
  esac
  # 値を確実に変える
  [ "$col" = "logical_source" ] && val="'immutable-check'"
  if psql -c "UPDATE core.event SET $col = $val WHERE id='$GID';" >/dev/null 2>&1; then
    echo "  NG $col が書き換えられた（重複判定が黙って当たらない行ができる）"; fail=1
  else
    echo "  OK $col の書き換えは拒まれた"
  fi
done
# **user_id は開いたまま**（Q15 が「誤った値を後から 1 文で直せる」と決めている）
if psql -c "UPDATE core.event SET user_id='11111111-1111-4111-8111-111111111111'
             WHERE id='$GID';" >/dev/null 2>&1; then
  echo "  OK user_id は直せる（Q15 が鍵の中身に混ぜなかった理由そのもの）"
  psql -c "UPDATE core.event SET user_id='00000000-0000-0000-0000-000000000000' WHERE id='$GID';" >/dev/null
else
  echo "  NG user_id まで凍結している（Q15 の「後から直せる」が成り立たない）"; fail=1
fi

# **台帳の行と実際の消去を突き合わせる**（tasks 12.3）。
# **DB は件数を検算しない** —— 実測で台帳 1 行のまま 4 行消せた。
# ここで「本文が空なのに自分を名指しする台帳の行が無い」ものを数える。
orphan=$(psql -c "SELECT
   (SELECT count(*) FROM core.event e
     WHERE e.raw = '' AND NOT EXISTS (SELECT 1 FROM core.erasure_ledger l
                                       WHERE l.event_id = e.id AND l.scope = 'event'))
 + (SELECT count(*) FROM core.event_version v
     WHERE v.raw = '' AND NOT EXISTS (SELECT 1 FROM core.erasure_ledger l
                                       WHERE l.event_id = v.event_id));")
[ "$orphan" = "0" ] || { echo "  NG 台帳に載らない消去が $orphan 件ある"; fail=1; }
echo "  OK 消えた本文はすべて台帳に載っている"


# ================================================================ ST04 の破棄の報告
#
# **破棄の報告は「バッファから破棄されたのか」の唯一の証拠**（扉 #14 / FR-9）。
# 端末は送れた報告を持たないので、書き換えられると取り直す手段が無い。
# 生存信号と同じく**列ごとに**投げ、削除と表の切り詰めも拒まれることを見る。
echo "== ST04: 破棄の報告を 1 件置く"
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('drop-check','破棄の報告の確認用',21600,'none') ON CONFLICT DO NOTHING;" >/dev/null
DID='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
psql -c "INSERT INTO core.drop_report
           (id, user_id, logical_source, device_id, reason, range_start, range_end, count,
            created_at, content_hash, raw)
         VALUES ('$DID','00000000-0000-0000-0000-000000000000','drop-check','drop-dev','age',
                 '2026-09-01T01:00:00Z','2026-09-01T04:00:00Z',180,'2026-09-14T00:00:00Z',
                 'drop-check-hash','{\"reason\":\"age\"}');
         INSERT INTO core.drop_report_hour (report_id, hour, count)
         VALUES ('$DID','2026-09-01T01:00:00Z',60),('$DID','2026-09-01T02:00:00Z',60),
                ('$DID','2026-09-01T03:00:00Z',60);" >/dev/null

# Scenario: 格納された破棄の報告は書き換えられない
for col in id user_id logical_source device_id reason range_start range_end count created_at received_at content_hash raw; do
  case "$col" in
    id)                                  val="'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'" ;;
    user_id)                             val="'11111111-1111-4111-8111-111111111111'" ;;
    logical_source)                      val="'immutable-check'" ;;
    reason)                              val="'bytes'" ;;
    range_start)                         val="'2026-09-01T00:00:00Z'" ;;
    range_end|created_at|received_at)    val="'2026-09-02T00:00:00Z'" ;;
    count)                               val="1" ;;
    *)                                   val="'forged'" ;;
  esac
  if psql -c "UPDATE core.drop_report SET $col = $val WHERE id = '$DID';" >/dev/null 2>&1; then
    echo "  NG 破棄の報告の $col が書き換えられた（FR-9 違反）"; fail=1
  else
    echo "  OK 破棄の報告の $col の書き換えは拒まれた"
  fi
done
for col in report_id hour count; do
  case "$col" in
    report_id) val="'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'" ;;
    hour)      val="'2026-09-01T09:00:00Z'" ;;
    count)     val="1" ;;
  esac
  if psql -c "UPDATE core.drop_report_hour SET $col = $val WHERE report_id = '$DID';" >/dev/null 2>&1; then
    echo "  NG 時間ごとの件数の $col が書き換えられた（日の件数が嘘になる）"; fail=1
  else
    echo "  OK 時間ごとの件数の $col の書き換えは拒まれた"
  fi
done
# **削除と切り詰めも拒む**（UPDATE だけを止めると、消して入れ直す 2 手で差し替えられる。ST02 の R22）
for stmt in "DELETE FROM core.drop_report_hour WHERE report_id = '$DID';" \
            "DELETE FROM core.drop_report WHERE id = '$DID';" \
            "TRUNCATE core.drop_report_hour;" \
            "TRUNCATE core.drop_report CASCADE;"; do
  if psql -c "$stmt" >/dev/null 2>&1; then
    echo "  NG 破棄の報告が消せた: $stmt"; fail=1
  fi
done
left=$(psql -c "SELECT (SELECT count(*) FROM core.drop_report WHERE id = '$DID')
                    || '/' || (SELECT coalesce(sum(count), 0) FROM core.drop_report_hour WHERE report_id = '$DID');")
[ "$left" = "1/180" ] || { echo "  NG 破棄の報告が変わっている（$left。1/180 のはず）"; fail=1; }
echo "  OK 破棄の報告は行ごとも表ごとも消せない"
# **範囲も時間ごとの件数も持たない報告**も消せない（ST04 の review/code.md R4）。
# 時間ごとの件数を持つ行は外部キーと `drop_report_hour` の錠が先に止めるので、`drop_report` 自身の錠はこの形でしか観測できない
psql -c "INSERT INTO core.drop_report
           (id, user_id, logical_source, device_id, reason, count, created_at, content_hash, raw)
         VALUES ('dddddddd-dddd-4ddd-8ddd-dddddddddddd','00000000-0000-0000-0000-000000000000',
                 'drop-check','drop-dev','unreadable',2,'2026-09-14T00:00:00Z','drop-rangeless','{}');" >/dev/null
if psql -c "DELETE FROM core.drop_report WHERE id = 'dddddddd-dddd-4ddd-8ddd-dddddddddddd';" >/dev/null 2>&1; then
  echo "  NG 範囲を持たない破棄の報告を削除できた"; fail=1
else
  echo "  OK 範囲を持たない破棄の報告も削除できない"
fi
# 切り詰めの錠は、`CASCADE` だと `drop_report_hour` 側の錠でも止まって見分けられないので、錠そのものが在ることを見る
trg=$(psql -c "SELECT string_agg(tgname, ',' ORDER BY tgname) FROM pg_trigger
                WHERE NOT tgisinternal AND tgrelid IN ('core.drop_report'::regclass, 'core.drop_report_hour'::regclass);")
[ "$trg" = "drop_report_hour_immutable,drop_report_hour_no_truncate,drop_report_immutable,drop_report_no_truncate" ] \
  || { echo "  NG 破棄の報告の錠が揃っていない: $trg"; fail=1; }
for t in drop_report_immutable drop_report_hour_immutable; do
  ev=$(psql -c "SELECT (tgtype & 8 > 0) AND (tgtype & 16 > 0) FROM pg_trigger WHERE tgname = '$t';")
  [ "$ev" = "t" ] || { echo "  NG $t が削除と更新の両方を拒んでいない"; fail=1; }
done
echo "  OK 破棄の報告の 4 つの錠（更新・削除 / 切り詰め × 2 表）が在る"
# 1 件の破棄が空の範囲で入らないこと（R4。`coverage_span` はここで 500 を返していた）
if psql -c "INSERT INTO core.drop_report
              (id, user_id, logical_source, device_id, reason, range_start, range_end, count,
               created_at, content_hash, raw)
            VALUES ('cccccccc-cccc-4ccc-8ccc-cccccccccccc','00000000-0000-0000-0000-000000000000',
                    'drop-check','drop-dev','age','2026-09-01T01:00:00Z','2026-09-01T01:00:00Z',1,
                    now(),'drop-empty','{}');" >/dev/null 2>&1; then
  echo "  NG 空の範囲の破棄の報告が入った"; fail=1
else
  echo "  OK 空の範囲の破棄の報告は DB が拒む"
fi

# ================================================================ ST19: 個人属性の主張の錠
#
# **FR-44「書き換えず追記する」を DB の側で確かめる**（深掘り Q1 / C1。design D2）。
#
# Scenario: 取り込み口を通さない操作にも同じ制限が掛かる
#   （`cargo test` は取り込み口越しなので、**アプリ層だけの実装でも全部緑になる**。
#    主張を psql から直に殴れるのはここだけ —— 同じ PC の第三者製プラグインが打てるのもこの経路）
#
# 通すのは **削除の印・感度・その主張の台帳つきの消去**だけ（Q1 —— 主張も記録なので
# FR-50 / FR-51 が及ぶ）。`s01-attribute` の登録簿の行は移行が入れている。
echo "== ST19: 個人属性の主張の錠"
AU='00000000-0000-0000-0000-000000000000'
KIND='a0000000-0000-4000-8000-000000000001'
CLAIM_A='a1111111-1111-4111-8111-000000000001'   # 書き換えの拒否と、消去の偽物を全部ここへ当てる
CLAIM_B='a2222222-2222-4222-8222-000000000001'   # A を取り消す主張。削除の印・感度・取り消し先を見る
CLAIM_C='a3333333-3333-4333-8333-000000000001'   # 台帳つきの消去が**通る**ことだけを見る（唯一の開口部）

psql -c "INSERT INTO core.attribute_kind (id, user_id) VALUES ('$KIND','$AU')
         ON CONFLICT DO NOTHING;" >/dev/null
psql -c "INSERT INTO core.attribute_kind_name (kind_id, user_id, name)
         VALUES ('$KIND','$AU','住所');" >/dev/null

# 主張の原文。**`nonce` は原文にだけ入る**（design D4）—— `payload` へ写さない。
# ここは DB の錠を見る検査なので、原文と解析済みの組は取り込み口を通さず手で置く。
claim_row_of() {   # $1=id / $2=値 / $3=いつから / $4=取り消す主張（null 可）/ $5=乱数
  printf "INSERT INTO core.event
     (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
      schema_version, sensitivity, content_hash, raw, payload)
   VALUES ('%s','%s','s01-attribute','authored','2026-09-15T02:00:00Z',540,'Asia/Tokyo',1,2,
           'claim-hash-%s',
           '{\"claim\":\"%s\",\"nonce\":\"%s\",\"kind\":\"%s\",\"value\":\"%s\",\"valid_from\":{\"precision\":\"month\",\"date\":\"%s\"},\"supersedes\":%s,\"note\":null}',
           '{\"claim\":\"%s\",\"kind\":\"%s\",\"value\":\"%s\",\"valid_from\":{\"precision\":\"month\",\"date\":\"%s\"},\"supersedes\":%s,\"note\":null}');" \
    "$1" "$AU" "$1" "$1" "$5" "$KIND" "$2" "$3" "$4" "$1" "$KIND" "$2" "$3" "$4"
}
psql -c "$(claim_row_of "$CLAIM_A" '東京都 目黒区' '2019-10' 'null' 'Zm9vYmFyYmF6cXV4MTIzNDU2')" >/dev/null
psql -c "$(claim_row_of "$CLAIM_B" '東京都 世田谷区' '2023-03' "\"$CLAIM_A\"" 'YmFyYmF6cXV4Zm9vNjU0MzIx')" >/dev/null
psql -c "$(claim_row_of "$CLAIM_C" '大阪府 北区' '2013-04' 'null' 'cXV4Zm9vYmFyYmF6OTg3NjU0')" >/dev/null
echo "  OK 主張を 3 件置いた"

# --- 書き換えを拒む（値・「いつから」・取り消し先・主張した日時）
#
# **値・「いつから」・取り消し先はどれも原文と解析済みの中にある**ので、
# 当たる門は同じ。**それぞれ別に投げる** —— 1 つだけ効いていて他が素通しでも気付くように。
#
# Scenario: 主張の値を書き換える文は拒まれる
if psql -c "UPDATE core.event SET payload = jsonb_set(payload,'{value}','\"京都府\"')
             WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
  echo "  NG 主張の値が書き換えられた（FR-44 違反）"; fail=1
else
  echo "  OK 主張の値の書き換えは拒まれた"
fi
got=$(psql -c "SELECT payload->>'value' FROM core.event WHERE id='$CLAIM_A';")
[ "$got" = '東京都 目黒区' ] || { echo "  NG 拒まれたのに値が変わっている: $got"; fail=1; }

# **原文そのものの書き換えも拒む**（review/code.md R22）。
# 値を見るときに `payload` だけを試していたが、**`raw` は値の正典で、消去の唯一の復元元**。
# Scenario の WHEN「格納された主張の値を別の値にする更新」の最も直接的な読みはこちら。
if psql -c "UPDATE core.event SET raw = '{\"value\":\"京都府\"}' WHERE id='$CLAIM_A';" \
     >/dev/null 2>&1; then
  echo "  NG 主張の原文が書き換えられた（値の正典が動く）"; fail=1
else
  echo "  OK 主張の原文の書き換えは拒まれた"
fi
if psql -c "UPDATE core.event SET content_hash = 'forged' WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
  echo "  NG 主張の内容の鍵が書き換えられた"; fail=1
else
  echo "  OK 主張の内容の鍵の書き換えは拒まれた"
fi

# Scenario: 主張のいつからは書き換えられない
if psql -c "UPDATE core.event SET payload = jsonb_set(payload,'{valid_from,date}','\"2021-01\"')
             WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
  echo "  NG 主張の「いつから」が書き換えられた（FR-45 違反）"; fail=1
else
  echo "  OK 主張の「いつから」の書き換えは拒まれた"
fi

# Scenario: 主張の取り消し先は書き換えられない
#   （動かせると、どの主張を訂正したかが後から変えられ「いまの値」が黙って動く）
if psql -c "UPDATE core.event SET payload = jsonb_set(payload,'{supersedes}','\"$CLAIM_C\"')
             WHERE id='$CLAIM_B';" >/dev/null 2>&1; then
  echo "  NG 主張の取り消し先が書き換えられた（深掘り C5 違反）"; fail=1
else
  echo "  OK 主張の取り消し先の書き換えは拒まれた"
fi

# Scenario: 主張した日時は書き換えられない
#   （**FR-45 の 2 軸を分けた意味がここで消える** —— 後から直したことが分からなくなる）
if psql -c "UPDATE core.event SET event_time='2000-01-01T00:00:00Z' WHERE id='$CLAIM_A';" \
     >/dev/null 2>&1; then
  echo "  NG 主張した日時が書き換えられた（FR-45 / 深掘り C4 違反）"; fail=1
else
  echo "  OK 主張した日時の書き換えは拒まれた"
fi

# 原文が 1 バイトも動いていないこと（トリガが例外を投げても書けていた、を潰す）
got=$(psql -c "SELECT raw FROM core.event WHERE id='$CLAIM_A';")
case "$got" in
  *'"value":"東京都 目黒区"'*) ;;
  *) echo "  NG 主張の原文が変わっている: $got"; fail=1;;
esac

# Scenario: 主張の行は削除できない
if psql -c "DELETE FROM core.event WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
  echo "  NG 主張を行ごと消せた（FR-44 / 深掘り C1 違反）"; fail=1
else
  echo "  OK 主張は行ごと消せない"
fi
left=$(psql -c "SELECT count(*) FROM core.event WHERE id='$CLAIM_A';")
[ "$left" = "1" ] || { echo "  NG 主張が消えている"; fail=1; }

# Scenario: 他の記録を主張へ付け替えられない
#   （ST03 の R114 と同じ型。錠の外で作った行を主張として固定できると、主張を捏造できる）
if psql -c "UPDATE core.event SET logical_source='s01-attribute'
             WHERE id='33333333-3333-4333-8333-333333333333';" >/dev/null 2>&1; then
  echo "  NG 他の「本人が書いた」記録を主張へ付け替えられた（主張を捏造できる）"; fail=1
else
  echo "  OK 他の記録を主張へ付け替えられない"
fi

# --- 通さなければならない開口部（Q1。**閉じ切ると FR-50 / FR-51 が満たせない**）
#
# Scenario: 主張に削除の印を付けられる
if psql -c "UPDATE core.event SET deleted_at=now(), deleted_by='check' WHERE id='$CLAIM_B';" \
     >/dev/null 2>&1; then
  echo "  OK 主張に削除の印を付けられる"
else
  echo "  NG 主張の削除の印まで止めている（FR-50 / Q1 を壊している）"; fail=1
fi
psql -c "UPDATE core.event SET deleted_at=NULL, deleted_by=NULL WHERE id='$CLAIM_B';" >/dev/null

# Scenario: 主張の感度を変えられる
if psql -c "UPDATE core.event SET sensitivity=3 WHERE id='$CLAIM_B';" >/dev/null 2>&1; then
  echo "  OK 主張の感度を変えられる"
else
  echo "  NG 主張の感度まで止めている（PERM-2 / ST24 が当たらない）"; fail=1
fi

# --- 消去（FR-51）。**通るのは「その主張の台帳がある、消去の形」だけ**
claim_ledger() {   # $1=台帳に書く記録の id
  printf "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
          VALUES ('%s','%s','s01-attribute','event','check');" "$1" "$AU"
}

# Scenario: 台帳の無い主張の消去は拒まれる
if psql -c "UPDATE core.event SET raw='', payload='{}' WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
  echo "  NG 台帳を書かない消去が通った（唯一の開口部から本文が全部消える）"; fail=1
else
  echo "  OK 台帳の無い主張の消去は拒まれた"
fi
got=$(psql -c "SELECT raw <> '' FROM core.event WHERE id='$CLAIM_A';")
[ "$got" = "t" ] || { echo "  NG 拒まれたのに本文が消えている"; fail=1; }

# Scenario: 別の記録の台帳の行では主張の消去は通らない
#   （**`event_id = NEW.id` まで照合しないと、台帳 1 行で何件でも消去できる**。spec-review R7）
if psql -c "$(claim_ledger '77777777-7777-4777-8777-777777777777')
            UPDATE core.event SET raw='', payload='{}' WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
  echo "  NG 別の記録の台帳 1 行で主張が消去できた（門が「その主張の」を見ていない）"; fail=1
else
  echo "  OK 別の記録の台帳の行では主張の消去は通らない"
fi
got=$(psql -c "SELECT raw <> '' FROM core.event WHERE id='$CLAIM_A';")
[ "$got" = "t" ] || { echo "  NG 別の記録の台帳で本文が消えた"; fail=1; }

# Scenario: 台帳の行があっても消去の形でない書き換えは拒まれる
#   （ST03 の R95 と同じ穴。**原文（唯一の復元元）を消しながら、もっともらしい解析済みを植えられる**）
if psql -c "$(claim_ledger "$CLAIM_A")
            UPDATE core.event SET raw='', payload='{\"forged\":true}' WHERE id='$CLAIM_A';" \
     >/dev/null 2>&1; then
  echo "  NG 消去の顔で解析済みを差し替えられた（原文は消えているので引き直せない）"; fail=1
else
  echo "  OK 台帳の行があっても、消去の形でない書き換えは拒まれた"
fi

# Scenario: 台帳のある主張の消去は通る
#   （**閉じ切ると Q1 の「本文を消す」が満たせない** —— 本人が選んだのは消せる側）
if psql -c "$(claim_ledger "$CLAIM_C")
            UPDATE core.event SET raw='', payload='{}' WHERE id='$CLAIM_C';" >/dev/null 2>&1; then
  echo "  OK 台帳のある主張の消去は通る"
else
  echo "  NG 主張の本文が消せない（FR-51 / Q1 が満たせない）"; fail=1
fi
got=$(psql -c "SELECT raw = '' AND payload = '{}'::jsonb FROM core.event WHERE id='$CLAIM_C';")
[ "$got" = "t" ] || { echo "  NG 通ったのに本文が残っている"; fail=1; }
# **消去しても識別子と出来事の時刻と鍵は残る**（だから原文に乱数が要る。design D4 / C12）
got=$(psql -c "SELECT content_hash FROM core.event WHERE id='$CLAIM_C';")
[ "$got" = "claim-hash-$CLAIM_C" ] || { echo "  NG 消去で内容の鍵が動いた: $got"; fail=1; }

# Scenario: 主張以外の本人が書いた記録は従来どおり書き換えられる
#   （**禁止の範囲が主張の外へ広がっていないこと**。主張の行がある DB で見る —— spec-review R14。
#    段の `WHERE` を自分の行に絞る。絞らないと ST03 の `gate-check` の authored 行に当たって落ちる）
if psql -c "UPDATE core.event SET payload = '{\"edited\":true}'
            WHERE logical_source = 'immutable-check' AND origin = 'authored';" >/dev/null 2>&1; then
  echo "  OK 主張以外の本人が書いた記録は従来どおり書き換えられる"
else
  echo "  NG 主張以外まで止めている（ST17 の主観などが書き換えられなくなる）"; fail=1
fi

# --- 種類の 2 表（D7。**台帳は追記のみ** —— 名前を変えるのは行を足すこと）
#
# Scenario: 種類の名前の台帳は書き換えられない
if psql -c "UPDATE core.attribute_kind_name SET name='偽の名前' WHERE kind_id='$KIND';" \
     >/dev/null 2>&1; then
  echo "  NG 種類の名前が書き換えられた（前の名前が黙って消える）"; fail=1
else
  echo "  OK 種類の名前の台帳は書き換えられない"
fi
got=$(psql -c "SELECT name FROM core.attribute_kind_name WHERE kind_id='$KIND';")
[ "$got" = '住所' ] || { echo "  NG 拒まれたのに名前が変わっている: $got"; fail=1; }

# Scenario: 種類の台帳は削除も切り詰めもできない
for stmt in "DELETE FROM core.attribute_kind_name WHERE kind_id='$KIND';" \
            "TRUNCATE core.attribute_kind_name;" \
            "UPDATE core.attribute_kind SET user_id='11111111-1111-4111-8111-111111111111' WHERE id='$KIND';" \
            "DELETE FROM core.attribute_kind WHERE id='$KIND';" \
            "TRUNCATE core.attribute_kind CASCADE;"; do
  if psql -c "$stmt" >/dev/null 2>&1; then
    echo "  NG 種類の台帳が変えられた: $stmt"; fail=1
  fi
done
kinds=$(psql -c "SELECT count(*) FROM core.attribute_kind WHERE id='$KIND';")
names=$(psql -c "SELECT count(*) FROM core.attribute_kind_name WHERE kind_id='$KIND';")
[ "$kinds" = "1" ] && [ "$names" = "1" ] \
  && echo "  OK 種類の台帳は書き換えも削除も切り詰めもできない" \
  || { echo "  NG 種類 $kinds 行 / 名前 $names 行（どちらも 1 行のはず）"; fail=1; }


# --- **後から足した列が、黙って書き換えられる側に入らないこと**
#
# 即時の錠は列を 1 つずつ名指しで凍結する（design D2）。**名指しなので、`core.event` に
# 列が 1 本増えると、その列は何も言わずに書き換えられる側へ入る。**
# ここで「意図して開けている列」と「凍結している列」の合計が実際の列と一致することを見る ——
# 一致しなくなったら、増やした人が**どちら側かを決めるまで落ちる**（既定は厳しい側。扉 #15）。
open_cols="sensitivity deleted_at deleted_by"          # FR-50 / PERM-2 で開ける
gated_cols="raw payload content_hash"                  # 門が消去の形と台帳だけを通す
frozen_cols="id user_id logical_source external_id device_id origin event_time ingest_time
             tz_offset_min tz_id schema_version unit_system crs source_updated_at external_ref"
known=$(printf '%s\n' $open_cols $gated_cols $frozen_cols | sort)
actual=$(psql -c "SELECT column_name FROM information_schema.columns
                   WHERE table_schema='core' AND table_name='event';" | sort)
if [ "$known" != "$actual" ]; then
  echo "  NG core.event の列と、錠が知っている列がずれている（増えた列が黙って書き換えられる）"
  diff <(echo "$known") <(echo "$actual") | sed 's/^/     /'
  fail=1
else
  echo "  OK core.event の全 $(echo "$actual" | wc -l) 列が、開ける / 門で見る / 凍結する のどれかに入っている"
fi
# 凍結すると宣言した列が、本当に 1 つずつ拒まれること（宣言と実装のずれを見る）
for col in $frozen_cols; do
  case "$col" in
    event_time|ingest_time|source_updated_at) val="'2000-01-01T00:00:00Z'" ;;
    tz_offset_min|schema_version)             val="0" ;;
    id|user_id)                               val="'12121212-1212-4212-8212-121212121212'" ;;
    *)                                        val="'forged'" ;;
  esac
  if psql -c "UPDATE core.event SET $col = $val WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
    echo "  NG 主張の $col が書き換えられた（凍結すると宣言しているのに通る）"; fail=1
  fi
done
echo "  OK 凍結すると宣言した列はどれも書き換えられない"
# **門で見ると宣言した 3 列も、素の書き換えは拒まれること**（review/code.md R22）。
# 点呼が「知っている列」として通すだけだと、**門が将来その列を見なくなっても気付かない**
for col in $gated_cols; do
  case "$col" in
    payload) val="'{\"forged\":true}'" ;;
    *)       val="'forged'" ;;
  esac
  if psql -c "UPDATE core.event SET $col = $val WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
    echo "  NG 主張の $col が台帳なしで書き換えられた（門が見ていない）"; fail=1
  fi
done
echo "  OK 門で見ると宣言した列は、台帳なしの素の書き換えを拒む"

# ================================================================ ST12: 書庫の台帳
echo "== ST12: 書庫台帳は追記のみ"
psql -c "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome)
         VALUES ('00000000-0000-0000-0000-000000000000', repeat('e',64), 'immutable-check', 'read');
         INSERT INTO core.archive_ledger_source (ledger_id, logical_source)
         SELECT id, 'c03-youtube-watch' FROM core.archive_ledger WHERE sha256=repeat('e',64);
         INSERT INTO core.archive_file (sha256,user_id,inner_path,stored_path)
         VALUES (repeat('f',64),'00000000-0000-0000-0000-000000000000','watch.json','/tmp/watch.json');
         INSERT INTO core.archive_shape_confirmation (user_id,shape_hash,shape)
         VALUES ('00000000-0000-0000-0000-000000000000','shape-check','{}');" >/dev/null

archive_lock_check() {
  local table="$1" where="$2"
  local cols
  cols=$(psql -c "SELECT string_agg(format('%I = %I', column_name, column_name), ', ' ORDER BY ordinal_position)
                    FROM information_schema.columns WHERE table_schema='core' AND table_name='$table';")
  if psql -c "UPDATE core.$table SET $cols WHERE $where;" >/dev/null 2>&1; then
    echo "  NG $table の全列を書き換えられた"; fail=1
  fi
  if psql -c "DELETE FROM core.$table WHERE $where;" >/dev/null 2>&1; then
    echo "  NG $table の行を削除できた"; fail=1
  fi
  if psql -c "TRUNCATE core.$table CASCADE;" >/dev/null 2>&1; then
    echo "  NG $table を切り詰められた"; fail=1
  fi
}
archive_lock_check archive_ledger "sha256=repeat('e',64)"
archive_lock_check archive_ledger_source "logical_source='c03-youtube-watch'"
archive_lock_check archive_file "sha256=repeat('f',64)"
archive_lock_check archive_shape_confirmation "shape_hash='shape-check'"
[ "$fail" -eq 0 ] && echo "  OK 書庫台帳 4 表は全列の更新・削除・切り詰めを拒む"

# --- 戻し手順（D12）。**主張が残っていれば、種類の 2 表も登録簿の行も残す**
#
# 主張の原文は種類を**識別子で**指すので、表を落とすと「その識別子が何という名前だったか」が
# 戻しで永久に失われる（spec-review R20）。**錠が落ちるので、この節の最後に置く。**
before=$(psql -c "SELECT count(*) FROM core.attribute_kind_name;")
psql < migrations/202609160220_personal_attributes.down.sql >/dev/null 2>&1 \
  || { echo "  NG 202609160220_personal_attributes.down.sql が当たらない"; fail=1; }
after=$(psql -c "SELECT count(*) FROM core.attribute_kind_name;")
[ "$before" = "$after" ] \
  || { echo "  NG 主張が残っているのに名前の台帳が $before → $after 行になった"; fail=1; }
kept=$(psql -c "SELECT count(*) FROM core.source WHERE logical_source='s01-attribute';")
[ "$kept" = "1" ] || { echo "  NG 主張が残っているのに登録簿の行が消えた"; fail=1; }
# **この節が置いた 3 件だけを数える**（開発用 DB に偽データが残っていても効く検査にする）
claims=$(psql -c "SELECT count(*) FROM core.event
                   WHERE id IN ('$CLAIM_A','$CLAIM_B','$CLAIM_C');")
[ "$claims" = "3" ] || { echo "  NG 戻しでこの節の主張が $claims 件になった（3 件のはず）"; fail=1; }
echo "  OK 主張が残っていれば、戻しても種類の 2 表と登録簿の行と主張が残る"
# 当て直せる（前進のみの版を戻してから進める運用が成り立つ）
psql < migrations/202609160220_personal_attributes.sql >/dev/null 2>&1 \
  || { echo "  NG 戻した後に当て直せない"; fail=1; }
if psql -c "UPDATE core.event SET payload='{\"forged\":1}' WHERE id='$CLAIM_A';" >/dev/null 2>&1; then
  echo "  NG 戻して進めた後に主張の錠が消えている"; fail=1
else
  echo "  OK 戻して進めても主張の錠は効いている"
fi


# --- 戻し手順が当たること（R117）。
# `tools/check-migrations.sh` は **down.sql の存在と「不可逆」の記載だけ**を静的に見ており、
# **1 度も当てていない**。この change で 5 本増えるので、ここで逆順に当てて構文と依存を見る。
# **いちばん最後に置く** —— 表とビューが消えるので、上の検査はもう走れない。
echo "== 戻し手順を逆順に当てる（ST03 の 5 本。R117）"
ST03_UP=(202609120940_source_columns 202609120941_event_columns 202609120942_dedup_indexes
         202609120943_version_and_ledger 202609120944_gates)
# **まっさらな schema で当てる。** ここまでの検査が Q6 の「同じ本文・違う外部識別子」の行を
# 作っているので、`dedup_indexes` の戻しは**正しく**一意違反で落ちる（`.down.sql` が
# 「不可逆」と書いているのはまさにそれ）。ここで見たいのは**構文と依存の順**なので行を空にする。
psql -c "DROP SCHEMA core CASCADE;" >/dev/null
for m in "${MIGS[@]}"; do psql < "migrations/$m.sql" >/dev/null; done
down_fail=0
# **新しい版から戻す。** ST19 の主張の版がいちばん新しい（ここは schema を作り直した直後なので
# 主張は 0 件 —— D12 の「主張が残っていなければ 2 表と登録簿の行も落とす」側を通る）
psql < "migrations/202609160220_personal_attributes.down.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609160220_personal_attributes.down.sql が当たらない"; fail=1; down_fail=1; }
[ "$(psql -c "SELECT to_regclass('core.attribute_kind') IS NULL AND to_regclass('core.attribute_kind_name') IS NULL;")" = "t" ] \
  || { echo "  NG 主張が 0 件なのに種類の 2 表が残っている"; fail=1; down_fail=1; }
psql < "migrations/202609160220_personal_attributes.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609160220_personal_attributes.sql を戻した後に当て直せない"; fail=1; down_fail=1; }
psql < "migrations/202609160220_personal_attributes.down.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609160220_personal_attributes.down.sql を 2 回目に当てられない"; fail=1; down_fail=1; }

# **ST12 の書庫台帳の版をいちばん先に戻す**。戻して進め直せることまで見る
psql < "migrations/202609181600_archive_ingestion.down.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609181600_archive_ingestion.down.sql が当たらない"; fail=1; down_fail=1; }
[ "$(psql -c "SELECT to_regclass('core.archive_ledger') IS NULL;")" = "t" ] \
  || { echo "  NG 書庫台帳の戻しで表が消えていない"; fail=1; down_fail=1; }
psql < "migrations/202609181600_archive_ingestion.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609181600_archive_ingestion.sql を戻した後に当て直せない"; fail=1; down_fail=1; }
psql < "migrations/202609181600_archive_ingestion.down.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609181600_archive_ingestion.down.sql を 2 回目に当てられない"; fail=1; down_fail=1; }

# **ST04 の破棄の報告の版をいちばん先に戻す**（最後に足した版）。戻して進め直せることまで見る
psql < "migrations/202609151546_drop_reports.down.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609151546_drop_reports.down.sql が当たらない"; fail=1; down_fail=1; }
[ "$(psql -c "SELECT to_regclass('core.drop_report') IS NULL AND to_regclass('core.drop_report_hour') IS NULL;")" = "t" ] \
  || { echo "  NG 破棄の報告の戻しで表が消えていない"; fail=1; down_fail=1; }
psql < "migrations/202609151546_drop_reports.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609151546_drop_reports.sql を戻した後に当て直せない"; fail=1; down_fail=1; }
psql < "migrations/202609151546_drop_reports.down.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609151546_drop_reports.down.sql を 2 回目に当てられない"; fail=1; down_fail=1; }
# **ST16 の滞在の版を先に戻す**（ST16 の review/code.md R39）。滞在の台帳は `core.event` を指すので、
# ST03 の戻しより前に当てる。戻して進め直せることまで見る（前進のみの版を戻す運用が成り立つ）
psql < "migrations/202609142125_stays.down.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609142125_stays.down.sql が当たらない"; fail=1; down_fail=1; }
[ "$(psql -c "SELECT to_regclass('core.stay_criteria') IS NULL AND to_regclass('core.stay_absorbed') IS NULL;")" = "t" ] \
  || { echo "  NG 滞在の戻しで台帳が消えていない"; fail=1; down_fail=1; }
psql < "migrations/202609142125_stays.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609142125_stays.sql を戻した後に当て直せない"; fail=1; down_fail=1; }
psql < "migrations/202609142125_stays.down.sql" >/dev/null 2>&1 \
  || { echo "  NG 202609142125_stays.down.sql を 2 回目に当てられない"; fail=1; down_fail=1; }
for ((i=${#ST03_UP[@]}-1; i>=0; i--)); do
  m="${ST03_UP[$i]}"
  psql < "migrations/$m.down.sql" >/dev/null 2>&1 \
    || { echo "  NG $m.down.sql が当たらない"; fail=1; down_fail=1; }
done
[ "$down_fail" -eq 0 ] && echo "  OK ST19 の 1 本・ST04 の 1 本・ST16 の 1 本・ST03 の 5 本とも当たる"
# 当て直せること（前進のみの版を戻してから進める運用が成り立つ）
for m in "${ST03_UP[@]}"; do
  psql < "migrations/$m.sql" >/dev/null 2>&1 \
    || { echo "  NG $m.sql を当て直せない"; fail=1; }
done
# 門を確かめるための行を置く（schema を作り直したので登録簿も空）
psql -c "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
         VALUES ('gate-check','門の確認用',21600,'record') ON CONFLICT DO NOTHING;" >/dev/null
# 戻して進めた後も門が効いていること（**ここを見ないと「当たった」だけの検査になる**）
psql -c "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ('99999999-9999-4999-8999-999999999999',
                 '00000000-0000-0000-0000-000000000000','gate-check','gate-dev','collected',
                 '2026-09-08T06:00:00Z',540,'Asia/Tokyo',1,'after-down','{\"x\":1}','{\"x\":1}');" \
  >/dev/null 2>&1
if psql -c "UPDATE core.event SET raw='{\"tampered\":9}'
             WHERE id='99999999-9999-4999-8999-999999999999';" >/dev/null 2>&1; then
  echo "  NG 戻して進めた後に門が消えている"; fail=1
else
  echo "  OK 戻して進めても門は効いている"
fi

[ "$fail" -eq 0 ] && echo "書き換え禁止 OK" || { echo "書き換え禁止 NG"; exit 1; }
