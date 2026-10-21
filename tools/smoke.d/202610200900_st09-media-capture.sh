# ST09: 写真と動画の記録・写真の原本（D-01 / D-02）の縦串。tools/smoke.sh の末尾が `.` で読む
# 端末の側は計測テスト（MediaCaptureInstrumentedTest）が、本物の MediaStore と端末の中のテスト用の受け手で見る。
# ここは本物のサーバに、収集側が送る形の記録と原本の PUT を送り、D-01 の行と D-02 のファイルを見る。

MEDIA_FIXTURES=collector-android/app/src/test/resources/media
MEDIA_USER="00000000-0000-0000-0000-000000000000"
photo_file="$MEDIA_FIXTURES/camera-gps.jpg"
video_file="$MEDIA_FIXTURES/video-gps.mp4"
photo_sha=$(sha256sum "$photo_file" | cut -d' ' -f1)
video_sha=$(sha256sum "$video_file" | cut -d' ' -f1)

# 収集側の記録の形（design D4 の欄の最小）。外部識別子は sha256 そのもの（C17）
media_record() { # id source sha name
  local payload="{\"kind\":\"media\",\"sha256\":\"$3\",\"media_id\":7,\"volume\":\"external_primary\",\"display_name\":\"$4\",\"relative_path\":\"DCIM/Camera/\",\"time_basis\":\"taken\",\"tz_basis\":\"exif\"}"
  printf '[{"id":"%s","user_id":"%s","logical_source":"%s","external_id":"%s","device_id":"media-smoke","origin":"collected","event_time":"2026-09-08T02:00:00.000000Z","tz_offset_min":540,"tz_id":"Asia/Tokyo","schema_version":1,"raw":%s,"payload":%s}]' \
    "$1" "$MEDIA_USER" "$2" "$3" "$(rawstr "$payload")" "$payload"
}
media_put() { # sha file -> HTTP status
  curl -s "${AUTH[@]}" -o /tmp/smoke.media.body -w '%{http_code}' -X PUT \
    --data-binary "@$2" -H 'content-type: application/octet-stream' \
    "http://$BIND/media/originals/$1?user_id=$MEDIA_USER"
}

# Scenario: 写真を撮るとメタが D-01 に、原本が D-02 に入る
echo "== ST09. 写真の記録を送り、原本を PUT して、D-01 の行と D-02 のファイル（バイト一致）を見る"
code=$(post "$(media_record 09000001-0000-4000-8000-000000000001 c01-photo "$photo_sha" camera-gps.jpg)")
[ "$code" = "200" ] || { echo "写真の記録が $code"; exit 1; }
[ "$(jq -r '.[0].accepted' /tmp/smoke.body)" = "true" ] || { echo "写真の記録が受理されない: $(cat /tmp/smoke.body)"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-photo' AND payload->>'sha256'='$photo_sha';")" = "1" ] \
  || { echo "写真の記録が D-01 に 1 行入っていない"; exit 1; }
code=$(media_put "$photo_sha" "$photo_file")
[ "$code" = "201" ] || { echo "写真の原本の PUT が $code: $(cat /tmp/smoke.media.body)"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.media_original WHERE sha256='$photo_sha';")" = "1" ] \
  || { echo "受け取りの行が 1 つ無い"; exit 1; }
placed=$(find "$ASHIATO_MEDIA_DIR" -type f -not -path '*/.incoming/*')
[ "$(printf '%s\n' "$placed" | grep -c .)" = "1" ] || { echo "D-02 のファイルが 1 つでない: $placed"; exit 1; }
cmp "$photo_file" "$placed" || { echo "D-02 のファイルのバイトが端末のファイルと違う"; exit 1; }
[ "$(curl -s "${AUTH[@]}" -o /dev/null -w '%{http_code}' -I "http://$BIND/media/originals/$photo_sha?user_id=$MEDIA_USER")" = "200" ] \
  || { echo "受け取り済みの問い合わせ（HEAD）が 200 でない"; exit 1; }

# Scenario: 動画を撮るとメタだけが入り原本は送られない
echo "== ST09. 動画の記録を送り、原本の PUT は断られ、D-02 に何も増えない"
code=$(post "$(media_record 09000002-0000-4000-8000-000000000002 c01-video "$video_sha" video-gps.mp4)")
[ "$code" = "200" ] || { echo "動画の記録が $code"; exit 1; }
[ "$(jq -r '.[0].accepted' /tmp/smoke.body)" = "true" ] || { echo "動画の記録が受理されない: $(cat /tmp/smoke.body)"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.event WHERE logical_source='c01-video' AND payload->>'sha256'='$video_sha';")" = "1" ] \
  || { echo "動画の記録が D-01 に 1 行入っていない"; exit 1; }
code=$(media_put "$video_sha" "$video_file")
[ "$code" = "422" ] || { echo "動画の原本の PUT が 422 でない: $code"; exit 1; }
[ "$(jq -r '.error // empty' /tmp/smoke.media.body)" = "video_not_accepted" ] \
  || { echo "動画の断りの理由が video_not_accepted でない: $(cat /tmp/smoke.media.body)"; exit 1; }
[ "$(psql -c "SELECT count(*) FROM core.media_original WHERE sha256='$video_sha';")" = "0" ] \
  || { echo "動画の受け取りの行ができた"; exit 1; }
[ "$(find "$ASHIATO_MEDIA_DIR" -type f -not -path '*/.incoming/*' | wc -l)" = "1" ] \
  || { echo "動画のあとで D-02 のファイルが増えた"; exit 1; }
[ -z "$(find "$ASHIATO_MEDIA_DIR/.incoming" -type f 2>/dev/null)" ] || { echo ".incoming に書きかけが残った"; exit 1; }
