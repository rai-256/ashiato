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

# ------------------------------------------------------------------ 個人属性の主張（ST19）
#
# 確認バッチの画面（S-6 マスタ管理）の材料。proto の「導入直後」と同じ量 ——
# **種類 5・主張 21 件**（訂正で取り消した 1・予定 1・「いつから」が分からない 1 を含む）。
#
# **normal だけに入れる**（max / empty は位置の件数を変える並びで、属性には効かない）。
#
# **何度当てても同じ結果にする**:
#   - 種類は、いまの名前で既にあれば足さない（台帳は追記のみなので、足すと名前が増え続ける）
#   - 主張は固定の識別子・固定の乱数・固定の主張した日時で送る。原文が 1 バイトも変わらないので、
#     2 回目は内容の鍵で畳まれて `duplicate` になる（FR-22 / 深掘り C2）
[ "$MODE" = "normal" ] || exit 0

# ------------------------------------------------------------------ 書庫（ST12）
# 確認バッチで書庫の見出しと直近結果を表示する最小の材料。
curl -sf "${AUTH[@]}" -X POST "http://$BIND/ingest" -d '{"id":"12121212-0000-4000-8000-000000000001","user_id":"00000000-0000-0000-0000-000000000000","logical_source":"c03-youtube-watch","external_id":null,"device_id":"s01-c03","origin":"collected","event_time":"2026-09-16T03:00:00Z","tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,"raw":"{\"watch\":\"seed\"}","payload":{"archive_sha256":"seed-archive"}}' >/dev/null
docker compose exec -T db psql -q -U ashiato -d ashiato -c "
WITH ledger AS (
  INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome, created_at)
  VALUES ('00000000-0000-0000-0000-000000000000', repeat('1', 64), 'seed', 'read', '2026-09-19T00:00:00Z')
  ON CONFLICT DO NOTHING RETURNING id
)
INSERT INTO core.archive_ledger_source (ledger_id, logical_source, inserted_count, max_event_at)
SELECT id, 'c03-youtube-watch', 1, '2026-09-16T03:00:00Z' FROM ledger ON CONFLICT DO NOTHING;"
echo "normal: 書庫の視聴履歴と読めた台帳を入れた"

attrs_get() { curl -sf "${AUTH[@]}" "http://$BIND/attributes"; }

# **まず読み出して住所と職業を置く**（design D7 の初期化）。
# 先に「住所」を足そうとすると、初期化が先に置いた住所と名前が重なって 400 になる
attrs=$(attrs_get) || { echo "normal: 個人属性を読み出せない"; exit 1; }
for name in 副業 同居 生年月日; do
  if ! printf '%s' "$attrs" | jq -e --arg n "$name" '[.kinds[].name] | index($n)' >/dev/null; then
    curl -sf "${AUTH[@]}" -X POST "http://$BIND/attributes/kinds" -d "{\"name\":\"$name\"}" >/dev/null \
      || { echo "normal: 種類「$name」を足せない"; exit 1; }
  fi
done
attrs=$(attrs_get)
kinds=$(printf '%s' "$attrs" | jq -c '[.kinds[] | {(.name): .id}] | add')

# **種類の識別子は足したときに決まる**（住所と職業は利用者から導いた v5、他は v4）ので、
# 原文は毎回いまの識別子で組む。識別子は一度できれば変わらないので、2 回目も同じ原文になる。
python3 - "$kinds" > "$work/claims.jsonl" <<'PY'
import json, sys, uuid

kinds = json.loads(sys.argv[1])
USER = "00000000-0000-0000-0000-000000000000"
NS = uuid.UUID("19191919-0000-4000-8000-000000000019")

def nonce(seed):
    # **固定の乱数**（種から決める）。本物の画面は crypto.getRandomValues で毎回引くが、
    # 偽データは「何度当てても同じ結果」が要るので種から決める。長さは 22 文字以上（design D4）
    return uuid.uuid5(NS, f"nonce/{seed}").hex[:24]

out = []
def claim(kind, seed, value, precision, date, asserted, note=None, supersedes=None):
    cid = str(uuid.uuid5(NS, f"claim/{seed}"))
    raw = json.dumps({
        "claim": cid, "nonce": nonce(seed), "kind": kinds[kind], "value": value,
        "valid_from": {"precision": precision, "date": date},
        "supersedes": supersedes, "note": note,
    }, ensure_ascii=False, separators=(",", ":"))
    out.append({
        "id": cid, "user_id": USER, "logical_source": "s01-attribute",
        "external_id": None, "device_id": None, "origin": "authored",
        "event_time": asserted, "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
        "schema_version": 1, "raw": raw, "payload": {},
    })
    return cid

# --- 住所 7 件（「いつから」が分からない 1 / 訂正 1 組 / 予定 1）
claim("住所", "addr-1", "北海道 札幌市", "unknown", None, "2026-01-10T01:00:00Z",
      note="子どものころ。何年からかは思い出せない")
claim("住所", "addr-2", "東京都 中野区", "year", "2013", "2026-01-10T01:05:00Z", note="上京した年")
wrong = claim("住所", "addr-3", "東京都 目黒区", "month", "2019-04", "2026-01-10T01:10:00Z")
claim("住所", "addr-4", "東京都 目黒区", "month", "2019-10", "2026-02-02T02:00:00Z",
      note="4 月ではなく 10 月だった", supersedes=wrong)
claim("住所", "addr-5", "東京都 世田谷区", "day", "2023-03-18", "2026-03-18T03:00:00Z",
      note="転職に合わせて引っ越した")
claim("住所", "addr-6", "神奈川県 川崎市", "month", "2026-06", "2026-06-05T04:00:00Z")
# **予定**（今日より後の「いつから」。深掘り C6）。日付は固定 ——
# 毎回変えると原文が変わり、当て直しで畳まれずに主張が増える。
# **2027-04-01 を過ぎると「予定」ではなくなる**ので、そのときはここを先へ動かす
claim("住所", "addr-7", "千葉県 船橋市", "day", "2027-04-01", "2026-09-10T05:00:00Z",
      note="契約済み。引っ越しはこれから")

# --- 職業 6 件
claim("職業", "job-1", "学生", "year", "2009", "2026-01-10T01:20:00Z")
claim("職業", "job-2", "会社員（受託開発）", "month", "2013-04", "2026-01-10T01:25:00Z")
claim("職業", "job-3", "会社員（自社開発）", "month", "2017-04", "2026-01-10T01:30:00Z")
claim("職業", "job-4", "会社員（基盤）", "day", "2023-03-01", "2026-03-18T03:05:00Z")
claim("職業", "job-5", "会社員（基盤・主任）", "month", "2025-04", "2026-04-01T01:00:00Z")
claim("職業", "job-6", "会社員（基盤・主任）", "month", "2025-04", "2026-09-01T01:00:00Z",
      note="変わっていないことを確かめた")   # **同じ値をもう一度書く**（畳まれない。深掘り C2）

# --- 副業 4 件（終わり方に「なし」を使う。深掘り C10）
claim("副業", "side-1", "受託のフロントエンド", "year", "2015", "2026-01-10T01:35:00Z")
claim("副業", "side-2", "技術記事の執筆", "month", "2018-07", "2026-01-10T01:40:00Z")
claim("副業", "side-3", "技術書の共著", "month", "2021-09", "2026-01-10T01:45:00Z")
claim("副業", "side-4", None, "month", "2024-03", "2026-03-20T01:00:00Z",
      note="本業が忙しくなったのでやめた")

# --- 同居 3 件
claim("同居", "live-1", "ひとり", "year", "2013", "2026-01-10T01:50:00Z")
claim("同居", "live-2", "配偶者", "day", "2020-11-22", "2026-01-10T01:55:00Z")
claim("同居", "live-3", "配偶者・子 1 人", "day", "2024-08-09", "2026-08-09T01:00:00Z")

# --- 生年月日 1 件
claim("生年月日", "birth-1", "1991-02-14", "day", "1991-02-14", "2026-01-10T02:00:00Z")

assert len(out) == 21, f"主張が {len(out)} 件（21 件のはず）"
for item in out:
    print(json.dumps(item, ensure_ascii=False))
PY

# **訂正は取り消し先より後に送る**（取り消し先がその時点で DB に無いと `invalid_supersedes`）。
# `/ingest` は 1 件ずつ別のまとまりで確定するので、同じまとまり送りの中で順に効く
claims_total=$(wc -l < "$work/claims.jsonl")
got=$(jq -s -c . "$work/claims.jsonl" | curl -sS "${AUTH[@]}" -X POST "http://$BIND/ingest" --data-binary @- \
      | jq '[.[] | select(.accepted)] | length')
[ "$got" -eq "$claims_total" ] \
  || { echo "normal: 主張 $claims_total 件のうち $got 件しか受け入れられなかった"; exit 1; }
# **入れた数と読めた数が一致することを断言する**（review/code.md R13）。
# 取り込みの受理数しか数えていなかったときは、**21 件が全部読み出しから消えても
# 「21 件入れた」と印字して exit 0 した** —— そのあと確認バッチの手順書を持った人間が、
# 空のカードが並ぶ画面を見ることになる。**この事故の型に対する門はここ 1 行だけ。**
after=$(attrs_get) || { echo "normal: 入れた後に読み出せない"; exit 1; }
printf '%s' "$after" | jq -e '
  ([.kinds[].claims[]] | length) + ([.kinds[].superseded[]] | length) == 21
  and (.kinds | length) == 5' >/dev/null \
  || { echo "normal: 入れた 21 件と読めた件数が合わない: $(printf '%s' "$after" \
        | jq -c '[.kinds[] | {name, claims:(.claims|length), sup:(.superseded|length)}]')"; exit 1; }
# **導出の分岐が実際に通っていること**（訂正・予定・「なし」・「分からない」）——
# 縦串は精度 `month`・訂正なしの 3 件しか流さないので、ここが唯一それらを読む場所
printf '%s' "$after" | jq -e '
  (.kinds[] | select(.name == "住所") | (.superseded | length) == 1 and (.upcoming | length) == 1
     and ([.claims[] | select(.valid_from.precision == "unknown")] | length) == 1)
  and (.kinds[] | select(.name == "副業") | .current.value == null)' >/dev/null \
  || { echo "normal: 訂正・予定・分からない・「なし」のどれかが読み出しに出ていない"; exit 1; }
echo "normal: 個人属性の主張を $claims_total 件入れ、同じ数を読み戻した（種類 $(printf '%s' "$after" | jq '.kinds | length') 件）"
