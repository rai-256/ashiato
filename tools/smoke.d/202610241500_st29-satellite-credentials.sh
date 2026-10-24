# ST29: 衛星の登録 → 資格情報の発行 → その値は読み書きの口で 403 → 取り消すと 401。tools/smoke.sh の末尾が `.` で読む
# サーバのログを取るため、同じ DB に向けた 2 台目を別ポートで起こす（1 台目は標準出力に出す）。

echo "== ST29. ログを取る 2 台目のサーバを起動"
SAT_BIND="127.0.0.1:$((ASHIATO_HTTP_PORT + 1))"
SAT_LOG=$(mktemp)
BIND="$SAT_BIND" env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL ./target/debug/ashiato-server >"$SAT_LOG" 2>&1 & SAT_SRV=$!
for _ in $(seq 1 60); do curl -sf "http://$SAT_BIND/healthz" >/dev/null && break; sleep 1; done
curl -sf "http://$SAT_BIND/healthz" >/dev/null || { echo "ST29 の 2 台目が起動しない"; kill "$SAT_SRV" 2>/dev/null || true; exit 1; }

# 画面のログイン。cookie は Secure 付きで http では jar が送らないので、値を取って自分で付ける
echo "== ST29. Scenario: 画面のログインで衛星を登録し、資格情報を発行する"
sat_cookie=$(curl -s -D - -o /dev/null -H 'content-type: application/json' \
  -d "{\"password\":\"$WEB_PASSWORD\"}" "http://$SAT_BIND/session" \
  | tr -d '\r' | sed -n 's/^set-cookie: \(ashiato_session=[^;]*\).*/\1/Ip' | head -1)
[ -n "$sat_cookie" ] || { echo "画面のログインの cookie が取れない"; exit 1; }
SAT_ID_NAME="dev.t$(od -An -N6 -tx1 /dev/urandom | tr -d ' \n').smoke"
sat_reg=$(curl -s -H "cookie: $sat_cookie" -H 'content-type: application/json' \
  -d "{\"name\":\"縦串の衛星\",\"identifier\":\"$SAT_ID_NAME\"}" "http://$SAT_BIND/satellites")
sat_id=$(printf '%s' "$sat_reg" | jq -r '.id // empty')
[ -n "$sat_id" ] || { echo "衛星を登録できない: $sat_reg"; exit 1; }
sat_issue=$(curl -s -H "cookie: $sat_cookie" -H 'content-type: application/json' \
  -d '{"label":"縦串"}' "http://$SAT_BIND/satellites/$sat_id/credentials")
SAT_VALUE=$(printf '%s' "$sat_issue" | jq -r '.value // empty')
sat_cid=$(printf '%s' "$sat_issue" | jq -r '.id // empty')
[[ "$SAT_VALUE" =~ ^ashsat_[a-z0-9.-]+_[A-Za-z0-9_-]{43}$ ]] || { echo "資格情報の形が合わない（値は出さない）"; exit 1; }
[ -n "$sat_cid" ] || { echo "資格情報の番号が無い"; exit 1; }
echo "OK 登録と発行"

echo "== ST29. Scenario: 衛星の資格情報は読み書きの口で 403"
sat_get=$(curl -s -o /dev/null -w '%{http_code}' -H "authorization: Bearer $SAT_VALUE" "http://$SAT_BIND/events")
[ "$sat_get" = "403" ] || { echo "衛星の資格情報の GET /events が $sat_get"; exit 1; }
sat_post=$(curl -s -o /dev/null -w '%{http_code}' -H "authorization: Bearer $SAT_VALUE" \
  -H 'content-type: application/json' -d '[]' "http://$SAT_BIND/ingest")
[ "$sat_post" = "403" ] || { echo "衛星の資格情報の POST /ingest が $sat_post"; exit 1; }
echo "OK 403"

echo "== ST29. Scenario: 取り消した資格情報は次の求めから使えない"
sat_rev=$(curl -s -o /dev/null -w '%{http_code}' -X DELETE -H "cookie: $sat_cookie" \
  "http://$SAT_BIND/satellites/$sat_id/credentials/$sat_cid")
case "$sat_rev" in 2??) : ;; *) echo "資格情報を取り消せない: $sat_rev"; exit 1 ;; esac
sat_after=$(curl -s -o /dev/null -w '%{http_code}' -H "authorization: Bearer $SAT_VALUE" "http://$SAT_BIND/events")
[ "$sat_after" = "401" ] || { echo "取り消した資格情報の GET /events が $sat_after"; exit 1; }
echo "OK 401"

echo "== ST29. Scenario: 資格情報の値はサーバのログに出ない"
kill "$SAT_SRV" 2>/dev/null || true; wait "$SAT_SRV" 2>/dev/null || true
if grep -Eq 'ashsat_[a-z0-9.-]+_[A-Za-z0-9_-]{43}' "$SAT_LOG"; then echo "ログに資格情報の形がある"; exit 1; fi
if grep -qF -- "$SAT_VALUE" "$SAT_LOG"; then echo "ログに発行した値そのものがある"; exit 1; fi
[ -s "$SAT_LOG" ] || { echo "サーバのログが空（grep が空振りしている）"; exit 1; }
rm -f "$SAT_LOG"
echo "OK ログに値が無い"
