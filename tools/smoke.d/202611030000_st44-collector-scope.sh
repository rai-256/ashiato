# ST44: 収集アプリの合言葉は送る口だけ、画面の印は読む口だけに使う。
echo "== ST44: 画面の合言葉で読む口の資格情報の範囲"
st44_api=(-H "authorization: Bearer $API_TOKEN")
st44_cookie=("${READ[@]}")
st44_code=$(curl -s "${st44_api[@]}" -o /dev/null -w '%{http_code}' "http://$BIND/events")
[ "$st44_code" = 403 ] || { echo "API の合言葉で /events が $st44_code"; exit 1; }
st44_code=$(curl -s "${st44_api[@]}" -o /dev/null -w '%{http_code}' "http://$BIND/outages")
[ "$st44_code" = 200 ] || { echo "API の合言葉で /outages が $st44_code"; exit 1; }
st44_code=$(curl -s "${st44_api[@]}" -o /dev/null -w '%{http_code}' "http://$BIND/feelings/reminder?date=2026-09-07")
[ "$st44_code" = 200 ] || { echo "API の合言葉で /feelings/reminder が $st44_code"; exit 1; }
st44_code=$(curl -s "${st44_cookie[@]}" -o /dev/null -w '%{http_code}' "http://$BIND/events")
[ "$st44_code" = 200 ] || { echo "印で /events が $st44_code"; exit 1; }
st44_code=$(curl -s -o /dev/null -w '%{http_code}' "http://$BIND/healthz")
[ "$st44_code" = 200 ] || { echo "資格情報なしで /healthz が $st44_code"; exit 1; }
