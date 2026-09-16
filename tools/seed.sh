#!/usr/bin/env bash
# 開発用の偽データ生成器（製造準備 A-4）。**全 Story がこれを使う。**
# Story ごとに fixture を作らせると、各自バラバラのデータで動作確認することになる。
#
#   ./tools/seed.sh [normal|max|empty]
#     normal  滞在 9 件ぶんに相当する 1 日（UI の方向が前提にしている通常の量）
#     max     要件上限の 15 件（FR-76 は 1 日 5〜15 件。1 画面に収まらない側の確認用）
#     empty   ソースは登録するがデータを入れない（欠損の意味を確かめる）
#
# normal / max は、2026-09-07（Asia/Tokyo）の端末の位置（`c01-location`）を 60 秒ごとに入れる（ST16 / tasks 7.1）。
# 既定の基準（100 m / 10 分）で滞在が 9 件 / 15 件になり、最後の滞在の後に 30 分の記録の欠けが 1 つある。
# 送るたびにサーバが滞在を作り直す（design D5）ので、入れ終われば `GET /stays?date=2026-09-07` に並ぶ。
set -euo pipefail
cd "$(dirname "$0")/.."
MODE="${1:-normal}"
BIND="${BIND:-127.0.0.1:18787}"
API_TOKEN="${API_TOKEN:-dev-token-0123456789abcdef}"
AUTH=(-H "authorization: Bearer $API_TOKEN" -H 'content-type: application/json')

case "$MODE" in
  normal) N=9  ;;
  max)    N=15 ;;
  empty)  N=0  ;;
  *) echo "使い方: $0 [normal|max|empty]"; exit 2 ;;
esac

docker compose exec -T db psql -q -U ashiato -d ashiato -c \
  "INSERT INTO core.source (logical_source, display_name, expected_gap_sec, external_id_kind)
   VALUES ('seed-location','偽データ（位置）',21600,'none') ON CONFLICT DO NOTHING;"

# **原文は text で送る**（design D16）。JSON の値ではなく文字列にくるむ。
# 「収集した」記録には device_id が要る（design D19）。

for i in $(seq 1 "$N"); do
  hh=$(printf '%02d' $(( 7 + i )))
  curl -sf "${AUTH[@]}" -X POST "http://$BIND/ingest" -d "{
    \"id\":\"$(printf '%08d-0000-4000-8000-000000000000' "$i")\",
    \"user_id\":\"00000000-0000-0000-0000-000000000000\",
    \"logical_source\":\"seed-location\",\"external_id\":null,\"device_id\":\"seed\",
    \"origin\":\"collected\",\"event_time\":\"2026-09-07T${hh}:00:00Z\",
    \"tz_offset_min\":540,\"tz_id\":\"Asia/Tokyo\",\"schema_version\":1,
    \"raw\":\"{\\\"lat\\\":35.68,\\\"lon\\\":139.76,\\\"acc_m\\\":$(( 5 + i ))}\",
    \"payload\":{\"lat\":35.68,\"lon\":139.76,\"acc_m\":$(( 5 + i ))}}" >/dev/null
done
echo "$MODE: $N 件を入れた"

# ------------------------------------------------------------------ 位置の 1 日（ST16）
#
# **前に入れた別の並びの位置を読み出しから外し、この並びの位置を戻す**（R36）。
# 位置の記録は書き換えられない（FR-30）ので、削除の印だけを付け外しする。並びは端末識別子（`seed-<並び>`）で分ける。
# 外さないと、normal の後に max を入れた DB で 2 つの並びが重なり、滞在が崩れる。
docker compose exec -T db psql -q -U ashiato -d ashiato -c \
  "UPDATE core.event
      SET deleted_at = CASE WHEN device_id = 'seed-$MODE' THEN NULL ELSE coalesce(deleted_at, now()) END,
          deleted_by = CASE WHEN device_id = 'seed-$MODE' THEN NULL ELSE 'seed' END
    WHERE logical_source = 'c01-location' AND device_id LIKE 'seed-%'
      AND user_id = '00000000-0000-0000-0000-000000000000';" >/dev/null
[ "$N" -eq 0 ] && exit 0

# **組み立ては python に任せる**（1,440 点の揺れと移動の補間を bash で書くと読めない）。
# 揺れは種を固定した乱数なので、**何度入れても同じ本文**になり、再送として畳まれる（FR-22）。
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
python3 - "$N" "$MODE" > "$work/location.jsonl" <<'PY'
import json, math, random, sys, uuid
from datetime import datetime, timedelta, timezone

stays = int(sys.argv[1])
mode = sys.argv[2]
rnd = random.Random(f"20260907/{mode}")   # 並びごとに違う揺れ（別の並びと同じ本文にならない）
JST = timezone(timedelta(hours=9))
day0 = datetime(2026, 9, 7, tzinfo=JST)
LAT0, LON0 = 35.6812, 139.7671
M_PER_DEG = 111_320.0

def place(k):
    # 地点どうしは 2 km ずつ離す（互いに 500 m 以上。移動の 1 分ごとの点も半径 100 m に入らない）
    return (2000.0 * k, 0.0)

def at(north, east):
    return (LAT0 + north / M_PER_DEG, LON0 + east / (M_PER_DEG * math.cos(math.radians(LAT0))))

MOVE, GAP = 8, 30
last_end = 23 * 60                    # 最後の滞在は 23:00 に終わり、23:00〜23:30 が記録の欠け
span = last_end - MOVE * (stays - 1)
length = span // stays
points = []                           # (分, 北, 東, 揺らすか)
t = 0
for k in range(stays):
    end = last_end if k == stays - 1 else t + length
    n, e = place(k)
    points += [(m, n, e, True) for m in range(t, end + 1)]
    if k < stays - 1:
        (n1, e1), (n2, e2) = place(k), place(k + 1)
        for i in range(1, MOVE + 1):
            f = i / (MOVE + 1)
            points.append((end + i, n1 + (n2 - n1) * f, e1 + (e2 - e1) * f, False))
        t = end + MOVE + 1
# 欠けの後は、どこにもとどまらずに歩き続ける（滞在の件数を変えない）
n, e = place(stays)
for m in range(last_end + GAP + 1, 24 * 60):
    points.append((m, n + 300.0 * (m - last_end - GAP), e, False))

for m, n, e, jitter in points:
    dn, de = (rnd.uniform(-15, 15), rnd.uniform(-15, 15)) if jitter else (0.0, 0.0)
    lat, lon = at(n + dn, e + de)
    acc = rnd.randint(8, 45)
    when = (day0 + timedelta(minutes=m)).astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    body = {"lat": round(lat, 7), "lon": round(lon, 7), "acc_m": acc}
    raw = json.dumps(body, separators=(",", ":"))
    print(json.dumps({
        # **識別子に並びの名前を混ぜる**（R36）。時刻だけから作ると、normal の後に max を同じ DB へ入れたとき
        # 同じ識別子に別の本文が乗り、`id_reused` で全件断られる
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"ashiato-seed-location/{mode}/{when}")),
        "user_id": "00000000-0000-0000-0000-000000000000",
        "logical_source": "c01-location", "external_id": None, "device_id": f"seed-{mode}",
        "origin": "collected", "event_time": when, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1, "raw": raw, "payload": body,
    }, ensure_ascii=False))
PY

# 端末と同じく 200 件ずつ送る（`collector-android` の MAX_BATCH）
total=$(wc -l < "$work/location.jsonl")
split -l 200 "$work/location.jsonl" "$work/part."
accepted=0
for part in "$work"/part.*; do
  # **受け入れた件数を数える**（R51）。1 件でも受け入れれば 200 が返るので、状態符号だけでは部分的な失敗が見えない
  got=$(jq -s -c . "$part" | curl -sS -f "${AUTH[@]}" -X POST "http://$BIND/ingest" --data-binary @- \
        | jq '[.[] | select(.accepted)] | length')
  accepted=$((accepted + got))
done
[ "$accepted" -eq "$total" ] || { echo "$MODE: 位置 $total 件のうち $accepted 件しか受け入れられなかった"; exit 1; }
echo "$MODE: 2026-09-07 の位置を $total 件入れた（滞在 $N 件になる並び）"

# ------------------------------------------------------------------ 破棄の報告（ST04 / tasks 10.4）
#
# 確認バッチの画面で**印と文字が見える材料**。端末が上限で捨てたときに送るのと同じ形で `/drops` へ送る。
# - 2026-09-07: 10:00〜13:00（JST）の 180 件だけを捨てた日 —— 記録は残っているので「記録あり」に**右下の三角の印**、
#   週を選ぶと「うち 180 件を破棄（10:00〜13:00）」
# - 2026-09-05: **2 本に割れた報告**（00:00〜12:00 と 12:00〜24:00）で丸ごと覆う日 —— つないで判定するので「破棄された期間」、件数 1,440
# 原文は欄を組んだ文字列そのもので、何度入れても同じ本文 → 再送として畳まれる（冪等）。
python3 - > "$work/drops.json" <<'PY'
import json
from datetime import datetime, timedelta, timezone

def report(rid, start, hours, per_hour=60):
    end = start + timedelta(hours=hours)
    z = lambda t: t.strftime("%Y-%m-%dT%H:%M:%SZ")
    fields = {
        "id": rid, "user_id": "00000000-0000-0000-0000-000000000000",
        "logical_source": "c01-location", "device_id": "seed", "reason": "age",
        "created_at": "2026-12-01T00:00:00Z", "range_start": z(start), "range_end": z(end),
        "count": per_hour * hours,
        "hourly": [{"hour": z(start + timedelta(hours=h)), "count": per_hour} for h in range(hours)],
    }
    return dict(fields, raw=json.dumps(fields, separators=(",", ":")))

utc = timezone.utc
print(json.dumps([
    # 2026-09-07 10:00〜13:00 JST = 01:00〜04:00 UTC
    report("04040404-5eed-4000-8000-000000000907", datetime(2026, 9, 7, 1, tzinfo=utc), 3),
    # 2026-09-05 00:00〜12:00 JST と 12:00〜24:00 JST（端が接する 2 本）
    report("04040404-5eed-4000-8000-00000905a000", datetime(2026, 9, 4, 15, tzinfo=utc), 12),
    report("04040404-5eed-4000-8000-00000905b000", datetime(2026, 9, 5, 3, tzinfo=utc), 12),
]))
PY
# **収集開始日を破棄の日より前にする。** 本番では破棄より前に届いた記録か生存信号で開始日が前にあるが、
# 偽データの位置は 09-07 からしか無い。そのままだと 09-05 は ST02 の判定順で「導入前」（破棄より先に見る）になる。
# **生存信号ではなく記録を 1 件**置く —— 生存信号は登録簿に行ができた日より前だと開始日に効かない（第 9 回 Q32）
curl -sS -f "${AUTH[@]}" -X POST "http://$BIND/ingest" -d '{"id":"04040404-5eed-4000-8000-0000000e0901",
  "user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c01-location","external_id":null,
  "device_id":"seed","origin":"collected","event_time":"2026-09-01T03:00:00Z","tz_offset_min":540,
  "tz_id":"Asia/Tokyo","schema_version":1,"raw":"{\"lat\":35.6812,\"lon\":139.7671,\"acc_m\":20}",
  "payload":{"lat":35.6812,"lon":139.7671,"acc_m":20}}' >/dev/null
got=$(curl -sS -f "${AUTH[@]}" -X POST "http://$BIND/drops" --data-binary @"$work/drops.json" | jq '[.[] | select(.accepted)] | length')
[ "$got" -eq 3 ] || { echo "$MODE: 破棄の報告 3 件のうち $got 件しか受け入れられなかった"; exit 1; }
echo "$MODE: 破棄の報告を 3 件入れた（2026-09-07 の一部 180 件 / 2026-09-05 を 2 本で丸ごと 1,440 件）"
