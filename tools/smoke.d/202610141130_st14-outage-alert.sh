# smoke.sh の Story の節（tools/smoke.d/ の決まりは README.md）。本体の末尾から移した（2026-10-15）
# ================================================================ 収集の途切れ（ST14 / tasks 11.1）
echo "== 45. GET /outages は起動直後 warming_up、待ちを 0 にした起動では ready（design D5 / D6）"
out=$(curl -fsS "${AUTH[@]}" "http://$BIND/outages")
printf '%s' "$out" | jq -e '.status == "warming_up" and (has("outages") | not)' >/dev/null \
  || { echo "起動直後が warming_up でない: $out"; exit 1; }
kill "$SRV"; wait "$SRV" 2>/dev/null || true
# 待ち時間の値（既定 30 分）は変えない。この起動だけが環境変数で 0 にする
OUTAGE_WARMUP_SEC=0 env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL ./target/debug/ashiato-server & SRV=$!
for _ in $(seq 1 60); do curl -sf "http://$BIND/healthz" >/dev/null && break; sleep 1; done
out=$(curl -fsS "${AUTH[@]}" "http://$BIND/outages?device_id=smoke-dev")
printf '%s' "$out" | jq -e '.status == "ready" and (.outages | type == "array")' >/dev/null \
  || { echo "待ちを 0 にした起動が ready でない: $out"; exit 1; }
echo "OK outages warming_up -> ready"
