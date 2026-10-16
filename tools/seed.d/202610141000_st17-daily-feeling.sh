# seed.sh の Story の節（tools/seed.d/ の決まりは README.md）。本体の末尾から移した（2026-10-15）
# ------------------------------------------------------------------ 主観（ST17）
#
# 確認バッチと e2e が見る「その日どう感じたか」。固定の 30 日（2026-09-04〜10-03。proto と同じ）のうち:
#   - 09-04・09-05 は最初の記録より前（使い始める前）、残る 28 日のうち 9 日が未記入、19 日に記録がある
#   - 09-07 は 3 回書き直した日、09-12 は前（当日の夜）と後（翌朝）の両方を書いた日
#   - 滞在に付いた気分: 09-15 の滞在（普通の滞在・書き直し 1 回。この節が足す位置から作る）/ 09-10 の夜から翌朝へ続く滞在 /
#     吸収された滞在（09-12。あとから届いた位置で 2 つの滞在が 1 つになった）/ 本人が消した滞在（09-14）
# 滞在は、**この節が足す位置の記録**（device `seed-feeling`。ほかの並びの日と被らない 09-10〜09-14）から
# サーバが作る。吸収は「欠けのある 2 つの滞在 → 欠けを埋める位置が後から届く」の順に送って起こす。
#
# **何度流しても同じ結果にする**: 識別子・乱数・記入の日時はすべて定数（滞在の識別子はサーバが一度決めたものを読む）。
# 原文が 1 バイトも変わらないので、2 回目は内容の鍵で畳まれる（FR-22）。主観の行は DB の錠で消せないので、
# 増えていないことを**この節自身が数え**、違えば非 0 で終える。
USER_ID="00000000-0000-0000-0000-000000000000"
python3 - "$work" <<'PY'
import json, math, random, sys, uuid
from datetime import datetime, timedelta, timezone

work = sys.argv[1]
USER = "00000000-0000-0000-0000-000000000000"
JST = timezone(timedelta(hours=9))
LAT0, LON0 = 35.6812, 139.7671
M_PER_DEG = 111_320.0

def coord(north, east=0.0):
    return (LAT0 + north / M_PER_DEG, LON0 + east / (M_PER_DEG * math.cos(math.radians(LAT0))))

def point(when, north, east, jitter, rnd):
    dn, de = (rnd.uniform(-15, 15), rnd.uniform(-15, 15)) if jitter else (0.0, 0.0)
    lat, lon = coord(north + dn, east + de)
    t = when.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    body = {"lat": round(lat, 7), "lon": round(lon, 7), "acc_m": rnd.randint(8, 45)}
    raw = json.dumps(body, separators=(",", ":"))
    return {
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"ashiato-seed-feeling-location/{t}")),
        "user_id": USER, "logical_source": "c01-location", "external_id": None, "device_id": "seed-feeling",
        "origin": "collected", "event_time": t, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1, "raw": raw, "payload": body,
    }

def stay_points(start, end, north, rnd):
    """start〜end を 1 分ごとに（揺らして）とどまる。そのあと 10 分、北へ 300 m/分で歩き去る"""
    out, m = [], 0
    t = start
    while t <= end:
        out.append(point(t, north, 0.0, True, rnd)); t += timedelta(minutes=1)
    for i in range(1, 11):
        out.append(point(end + timedelta(minutes=i), north + 300.0 * i, 0.0, False, rnd))
    return out

def jst(d, hh, mm):
    return datetime.fromisoformat(f"{d}T{hh:02d}:{mm:02d}:00+09:00")

rnd = random.Random("20261014/feeling-locations")
a, b = [], []
# 09-10 19:00 〜 09-11 06:30: 前の日から続く滞在
a += stay_points(jst("2026-09-10", 19, 0), jst("2026-09-11", 6, 30), 60000.0, rnd)
# 09-12: 10:00〜10:40 と 11:00〜11:40 の同じ場所の 2 つの滞在（10:41〜10:59 は記録の欠け）→ b で欠けを埋めて 1 つにする
a += stay_points(jst("2026-09-12", 10, 0), jst("2026-09-12", 10, 40), 64000.0, rnd)[:-10]
a += stay_points(jst("2026-09-12", 11, 0), jst("2026-09-12", 11, 40), 64000.0, rnd)
b += [point(jst("2026-09-12", 10, 41) + timedelta(minutes=i), 64000.0, 0.0, True, rnd) for i in range(19)]
# 09-15: 気分を付ける普通の滞在 2 つ（並びの位置の記録は使わない。max → normal の往復で滞在が作り直されても原文が変わらない）
a += stay_points(jst("2026-09-15", 10, 0), jst("2026-09-15", 10, 40), 66000.0, rnd)
a += stay_points(jst("2026-09-15", 15, 0), jst("2026-09-15", 15, 40), 67000.0, rnd)
# 09-14 14:00〜14:40: 本人があとで消す滞在
a += stay_points(jst("2026-09-14", 14, 0), jst("2026-09-14", 14, 40), 68000.0, rnd)
for name, rows in (("a", a), ("b", b)):
    with open(f"{work}/feel-loc-{name}.jsonl", "w") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
PY
send_lines "$work/feel-loc-a.jsonl" >/dev/null
send_lines "$work/feel-loc-b.jsonl" >/dev/null   # a のあと（欠けを埋める位置が後から届く）

# サーバが作った滞在の識別子と区間を読む（消した・吸収された滞在も含む）。`日付 → [{id, start, end, mark}]`
stay_rows() {  # $1: 日（Asia/Tokyo）
  docker compose exec -T db psql -qtAX -U ashiato -d ashiato -c "
    SELECT coalesce(json_agg(r ORDER BY r.start), '[]'::json) FROM (
      SELECT e.id, to_char(e.event_time AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS start,
             e.payload->>'end' AS \"end\", e.deleted_by AS mark,
             EXISTS (SELECT 1 FROM core.stay_absorbed a WHERE a.event_id = e.id) AS absorbed
        FROM core.event e
       WHERE e.user_id = '$USER_ID' AND e.logical_source = 's01-stay'
         AND (e.event_time AT TIME ZONE 'Asia/Tokyo')::date = DATE '$1') r;"
}
stays_0915=$(stay_rows 2026-09-15); stays_0910=$(stay_rows 2026-09-10)
stays_0912=$(stay_rows 2026-09-12); stays_0914=$(stay_rows 2026-09-14)
printf '%s' "$stays_0915" | jq -e 'length == 2' >/dev/null || { echo "normal: 2026-09-15 の滞在が 2 つでない: $stays_0915"; exit 1; }
printf '%s' "$stays_0910" | jq -e 'length == 1 and .[0].end > "2026-09-10T15:00:00Z"' >/dev/null \
  || { echo "normal: 09-10 の夜から翌朝へ続く滞在が 1 つになっていない: $stays_0910"; exit 1; }
printf '%s' "$stays_0912" | jq -e '[.[] | select(.absorbed)] | length >= 1' >/dev/null \
  || { echo "normal: 09-12 の滞在が吸収されていない（位置の欠けが埋まって 1 つになるはず）: $stays_0912"; exit 1; }
printf '%s' "$stays_0914" | jq -e 'length == 1' >/dev/null || { echo "normal: 09-14 の滞在が 1 つでない: $stays_0914"; exit 1; }

python3 - "$work" "$stays_0915" "$stays_0910" "$stays_0912" "$stays_0914" > "$work/feelings.jsonl" <<'PY'
import json, sys, uuid
from datetime import datetime, timedelta, timezone

work = sys.argv[1]
s0915, s0910, s0912, s0914 = (json.loads(x) for x in sys.argv[2:6])
USER = "00000000-0000-0000-0000-000000000000"
NS = uuid.UUID("17171717-0000-4000-8000-000000000017")
JST = timezone(timedelta(hours=9))
z = lambda t: t.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

def nonce(seed):
    # 固定の乱数（種から決める。本物の画面は crypto.getRandomValues）。22 文字以上の base64url の文字だけ
    return uuid.uuid5(NS, f"nonce/{seed}").hex[:24]

out = []
def feeling(seed, target, event_time, written_at, valence, energy=None, note=None):
    fid = str(uuid.uuid5(NS, f"feeling/{seed}"))
    scales = [{"id": "valence", "version": 1, "value": valence}]
    if energy is not None:
        scales.append({"id": "energy", "version": 1, "value": energy})
    raw = json.dumps({"feeling": fid, "nonce": nonce(seed), "target": target, "written_at": written_at,
                      "written_tz": "Asia/Tokyo", "scales": scales, "note": note},
                     ensure_ascii=False, separators=(",", ":"))
    out.append({"id": fid, "user_id": USER, "logical_source": "s01-feeling", "external_id": None, "device_id": None,
                "origin": "authored", "event_time": event_time, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
                "schema_version": 1, "raw": raw, "payload": {}})

def day(date, written, valence, energy=None, note=None, tag=""):
    midnight = datetime.fromisoformat(f"{date}T00:00:00+09:00")
    feeling(f"day/{date}/{written}{tag}", {"kind": "day", "date": date}, z(midnight), written, valence, energy, note)

def stay(row, seed, written, valence, energy=None, note=None):
    target = {"kind": "stay", "stay": row["id"], "start": row["start"], "end": row["end"]}
    feeling(f"stay/{seed}", target, row["start"], written, valence, energy, note)

# --- 日（19 日。09-07 は 3 回書き直し、09-12 は前と後の両方）
day("2026-09-06", "2026-09-06T21:10:00+09:00", 1)
day("2026-09-07", "2026-09-07T21:30:00+09:00", -1)
day("2026-09-07", "2026-09-07T21:45:00+09:00", 0, note="少し持ち直した")
day("2026-09-07", "2026-09-07T22:10:00+09:00", 1, 0)
day("2026-09-10", "2026-09-10T21:00:00+09:00", 2, 1)
day("2026-09-11", "2026-09-11T21:20:00+09:00", 0)
day("2026-09-12", "2026-09-12T21:30:00+09:00", 1, note="よく歩いた日")                       # 前
day("2026-09-12", "2026-09-13T08:10:00+09:00", 2, 1, "朝になって思い返すと、いい日だった")   # 後
day("2026-09-14", "2026-09-14T21:40:00+09:00", -1)
day("2026-09-15", "2026-09-15T21:05:00+09:00", 0)
day("2026-09-17", "2026-09-17T20:55:00+09:00", -2, -2, "ずっと眠かった")
day("2026-09-18", "2026-09-18T21:15:00+09:00", 1)
day("2026-09-20", "2026-09-20T21:30:00+09:00", 2)
day("2026-09-21", "2026-09-21T21:00:00+09:00", 0)
day("2026-09-22", "2026-09-22T22:00:00+09:00", -1)
day("2026-09-24", "2026-09-24T21:25:00+09:00", 1, 2)
day("2026-09-25", "2026-09-25T21:10:00+09:00", 0)
day("2026-09-26", "2026-09-26T21:45:00+09:00", 1)
day("2026-09-28", "2026-09-28T21:00:00+09:00", 2)
day("2026-09-29", "2026-09-29T21:30:00+09:00", -1)
day("2026-09-30", "2026-09-30T21:15:00+09:00", 1)
day("2026-10-03", "2026-10-03T21:20:00+09:00", 0)

# --- 滞在（09-15 の 1 つ目は書き直し 1 回）
stay(s0915[0], "0915-1a", "2026-09-15T12:30:00+09:00", 1)
stay(s0915[0], "0915-1b", "2026-09-16T07:50:00+09:00", 2, note="あとから思うと楽しかった")
stay(s0915[1], "0915-2", "2026-09-15T21:50:00+09:00", -1, -1)
stay(s0910[0], "0910-carry", "2026-09-11T07:30:00+09:00", 1)                  # 前の日から続く滞在
absorbed = next(r for r in s0912 if r["absorbed"])
stay(absorbed, "0912-absorbed", "2026-09-12T12:00:00+09:00", 0, note="ここで一休み")  # 吸収された滞在
stay(s0914[0], "0914-deleted", "2026-09-14T15:00:00+09:00", -1)                # のちに本人が消す滞在

for item in out:
    print(json.dumps(item, ensure_ascii=False))
PY
feel_total=$(wc -l < "$work/feelings.jsonl")
rm -f "$work"/feelings.jsonl.part.*; split -l 200 "$work/feelings.jsonl" "$work/feelings.jsonl.part."
got=0
for part in "$work"/feelings.jsonl.part.*; do
  n=$(jq -s -c . "$part" | curl -sS -f "${AUTH[@]}" -X POST "http://$BIND/ingest" --data-binary @- \
      | jq '[.[] | select(.accepted)] | length') || { echo "normal: 主観の送信に失敗した"; exit 1; }
  got=$((got + n))
done
[ "$got" -eq "$feel_total" ] || { echo "normal: 主観 $feel_total 件のうち $got 件しか受け入れられなかった"; exit 1; }

# 本人が消した滞在: まだ消していなければ消す（消した後の 2 回目は何もしない。位置の記録は書き換えない）
deleted_stay=$(printf '%s' "$stays_0914" | jq -r '.[0] | select(.mark == null) | .id')
if [ -n "$deleted_stay" ]; then
  curl -sS -f "${AUTH[@]}" -X POST "http://$BIND/stays/erase" -d "{\"stay_id\":\"$deleted_stay\"}" >/dev/null \
    || { echo "normal: 09-14 の滞在を消せない"; exit 1; }
fi

# **2 回流しても主観が増えない** —— seed が入れた識別子（uuid5。原文ごとに固定）の行が、入れた件数とちょうど同じ。
# 窓（09-04〜10-03）の中の全部の行を数えない: 主観の行は錠で消せないので、確認バッチで人間が窓の中に 1 件書いた DB では
# 二度と通らなくなる（R3）。seed の識別子の集合に絞れば、他の記録があっても畳まれたかだけを見られる。
seed_ids=$(jq -r '.id' "$work/feelings.jsonl" | paste -sd, -)
ours=$(docker compose exec -T db psql -qtAX -U ashiato -d ashiato -c \
  "SELECT count(*) FROM core.event WHERE user_id = '$USER_ID' AND logical_source = 's01-feeling'
      AND id = ANY(string_to_array('$seed_ids', ',')::uuid[]);" | tr -d '[:space:]')
[ "$ours" -eq "$feel_total" ] \
  || { echo "normal: 主観が畳まれていない（seed の識別子の行が $ours 行。入れたのは $feel_total 件）"; exit 1; }
others=$(docker compose exec -T db psql -qtAX -U ashiato -d ashiato -c \
  "SELECT count(*) FROM core.event WHERE user_id = '$USER_ID' AND logical_source = 's01-feeling'
      AND event_time >= '2026-09-03T15:00:00Z' AND event_time < '2026-10-03T15:00:00Z'
      AND NOT (id = ANY(string_to_array('$seed_ids', ',')::uuid[]));" | tr -d '[:space:]')

# 読み戻して形を確かめる: 30 日 / 使い始める前 2 / 未記入 9 / 記録あり 19、書き直し・前と後・吸収・消した滞在。
# 窓の中に seed 以外の記録（確認バッチで人間が書いたもの）があれば、日の状態の件数はそれで動くので数えず、
# seed が入れた記録だけで決まる形（下限）を確かめる。
fb=$(curl -sf "${AUTH[@]}" "http://$BIND/feelings?from=2026-09-04&to=2026-10-03") \
  || { echo "normal: 主観を読み出せない"; exit 1; }
if [ "$others" -eq 0 ]; then
  shape='(.days | length) == 30 and .first_day == "2026-09-06"
    and ([.days[] | select(.status == "before_start")] | length) == 2
    and ([.days[] | select(.status == "missing")] | length) == 9
    and ([.days[] | select(.status == "recorded")] | length) == 19
    and (.days[] | select(.date == "2026-09-07") | (.day.records | length) == 3)
    and (.days[] | select(.date == "2026-09-12") | .day.latest.before != null and .day.latest.after != null)
    and (.days[] | select(.date == "2026-09-12") | [.stays[].records[] | select(.target_stay != null)] | length) == 1
    and (.days[] | select(.date == "2026-09-14") | .hidden_stay_records == 1 and (.stays | length) == 0)
    and (.days[] | select(.date == "2026-09-15") | .stays_with_records) == 2
    and (.days[] | select(.date == "2026-09-15") | [.day.latest.before.scales[] | select(.id == "valence") | .value]) == [0]'
  said="未記入 9 / 使い始める前 2 / 記録あり 19"
else
  # 人間が書いた記録は件数を増やすだけ（主観は消せない・seed の記録は残る）ので、seed の分を下限として見る
  shape='(.days | length) == 30
    and ([.days[] | select(.status == "recorded")] | length) >= 19
    and (.days[] | select(.date == "2026-09-07") | (.day.records | length) >= 3)
    and (.days[] | select(.date == "2026-09-12") | .day.latest.before != null and .day.latest.after != null)
    and (.days[] | select(.date == "2026-09-12") | [.stays[].records[] | select(.target_stay != null)] | length) >= 1
    and (.days[] | select(.date == "2026-09-14") | .hidden_stay_records >= 1)
    and (.days[] | select(.date == "2026-09-15") | .stays_with_records) >= 2
    and (.days[] | select(.date == "2026-09-15") | .day.latest.before != null)'
  said="seed 以外の $others 件があるので、日の状態の件数は数えず seed の記録の下限だけ"
  echo "normal: 窓の中に seed 以外の主観が $others 件ある —— 日の状態の件数（未記入 9 など）は確かめず、seed の記録で決まる形だけを確かめる"
fi
printf '%s' "$fb" | jq -e "$shape" >/dev/null \
  || { echo "normal: 読み戻した主観が想定と合わない: $(printf '%s' "$fb" | jq -c '[.days[] | {date, status}]')"; exit 1; }
echo "normal: 主観を $feel_total 件入れ、畳まれて増えていないことと読み戻した形（$said）を確かめた"
