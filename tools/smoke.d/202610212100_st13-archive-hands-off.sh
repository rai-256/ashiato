# ST13: Google 系の手作業は「書き出して置く」だけ、の縦串。tools/smoke.sh の末尾が `.` で読む
# 9b で印を置いた YouTube の形（日本語）を、起動し直さず・登録簿も触らずに、2 冊目で使い回す。

youtube_events() {
  psql -c "SELECT count(*) FROM core.event WHERE user_id = '$1'::uuid AND logical_source = 'c03-youtube-watch' AND deleted_at IS NULL;"
}

# Scenario: Takeout をダウンロードのフォルダに落とすだけで入る
# Scenario: 同じ形の 2 回目の書庫は印を求めずに入る
echo "== Scenario: Takeout をダウンロードのフォルダに落とすだけで入る"
echo "== Scenario: 同じ形の 2 回目の書庫は印を求めずに入る"
HANDS_FIXTURE="$ARCHIVE_ROOT/fixture-hands-off"
mkdir -p "$HANDS_FIXTURE/Takeout/YouTube"
printf '%s' '[{"time":"2026-11-12T03:00:00Z","title":"二冊目 を視聴しました","titleUrl":"https://www.youtube.com/watch?v=handsoff","products":["YouTube"]}]' \
  > "$HANDS_FIXTURE/Takeout/YouTube/watch-history.json"
hands_before=$(youtube_events "$ARCHIVE_USER")
pending_before=$(psql -c "SELECT count(*) FROM core.archive_pending_shape WHERE user_id = '$ARCHIVE_USER'::uuid;")
marks_before=$(psql -c "SELECT count(*) FROM core.archive_shape_confirmation WHERE user_id = '$ARCHIVE_USER'::uuid;")
# 2 か月後に作られた同じ製品・同じ言語の書庫を、ダウンロードのフォルダへ落とす。ほかには何もしない
(cd "$HANDS_FIXTURE" && zip -q -r "$ASHIATO_DOWNLOADS_DIR/takeout-20261113T041200Z-001.zip" Takeout)
for _ in $(seq 1 30); do
  [ "$(youtube_events "$ARCHIVE_USER")" -gt "$hands_before" ] && break
  sleep 1
done
[ "$(youtube_events "$ARCHIVE_USER")" -gt "$hands_before" ] \
  || { echo "ダウンロードに落とした 2 冊目が入らない: $hands_before"; exit 1; }
pending_after=$(psql -c "SELECT count(*) FROM core.archive_pending_shape WHERE user_id = '$ARCHIVE_USER'::uuid;")
marks_after=$(psql -c "SELECT count(*) FROM core.archive_shape_confirmation WHERE user_id = '$ARCHIVE_USER'::uuid;")
[ "$pending_before" = "$pending_after" ] || { echo "同じ形の 2 冊目が確認待ちを増やした: $pending_before -> $pending_after"; exit 1; }
[ "$marks_before" = "$marks_after" ] || { echo "印が増えている（手で置いていない）: $marks_before -> $marks_after"; exit 1; }
echo "   → 印を置かずに 2 冊目が入った"

# Scenario: 専用のフォルダを作らずに端末のタイムラインが入る
echo "== Scenario: 専用のフォルダを作らずに端末のタイムラインが入る"
# 専用のフォルダが無い状態で、もう 1 つの S-01 を起動する（別の利用者・別のフォルダ。DB は同じ）
HANDS_USER="00000000-0000-0000-0000-0000000000a7"
HANDS_ROOT="$ARCHIVE_ROOT/hands-off"
# port は tools/ports.sh の規則でこの worktree が持つ 2 つ目（サーバ +1）。ほかの worktree の port とは被らない
# （+100 は worktree st(NN+50) のサーバと被っていた。final review R13）。+1 を使う tools/check-panic-log.sh とは
# 同じ worktree で同時に走らない —— どちらも始めと終わりに同じ DB を docker compose down -v で落とす。
HANDS_BIND="127.0.0.1:$((ASHIATO_HTTP_PORT + 1))"
mkdir -p "$HANDS_ROOT/downloads" "$HANDS_ROOT/copies"
[ ! -e "$HANDS_ROOT/inbox" ] || { echo "専用のフォルダが先にある"; exit 1; }
HANDS_SRV=$(env -u POSTGRES_PASSWORD -u OWNER_DB_PASSWORD -u DATABASE_OWNER_URL \
  ASHIATO_ARCHIVE_USER_ID="$HANDS_USER" ASHIATO_INBOX_DIR="$HANDS_ROOT/inbox" \
  ASHIATO_DOWNLOADS_DIR="$HANDS_ROOT/downloads" ASHIATO_ARCHIVE_COPY_DIR="$HANDS_ROOT/copies" \
  BIND="$HANDS_BIND" bash -c './target/debug/ashiato-server >/dev/null 2>&1 & echo $!')
# 途中で落ちても 2 つ目のサーバを残さない。tools/smoke.sh の後始末（cleanup）も続けて呼ぶ（final review R13）
# HANDS_SRV が空のときに kill しない（kill 0 はプロセスグループ全体を止める。tools/smoke.sh と同じ）
hands_cleanup() { if [ -n "${HANDS_SRV:-}" ]; then kill "$HANDS_SRV" 2>/dev/null || true; fi; cleanup; }
trap hands_cleanup EXIT
for _ in $(seq 1 60); do curl -sf "http://$HANDS_BIND/healthz" >/dev/null && break; sleep 1; done
curl -sf "http://$HANDS_BIND/healthz" >/dev/null || { kill "$HANDS_SRV" 2>/dev/null || true; echo "2 つ目の S-01 が起動しない"; exit 1; }
[ -d "$HANDS_ROOT/inbox" ] || { kill "$HANDS_SRV" 2>/dev/null || true; echo "起動しても専用のフォルダが作られない"; exit 1; }
# 本人はフォルダを作らず、作られたフォルダへ端末から書き出した Timeline.json を運ぶ
printf '%s' '{"semanticSegments":[{"startTime":"2026-09-12T03:00:00Z","endTime":"2026-09-12T04:00:00Z","visit":{"topCandidate":{"placeLocation":{"latLng":"35.0116°, 135.7681°"}}}}]}' \
  > "$HANDS_ROOT/inbox/Timeline.json"
for _ in $(seq 1 30); do
  tl=$(psql -c "SELECT count(*) FROM core.event WHERE user_id = '$HANDS_USER'::uuid AND logical_source = 'c03-timeline-visit' AND deleted_at IS NULL;")
  [ "$tl" -ge 1 ] && break
  sleep 1
done
kill "$HANDS_SRV" 2>/dev/null || true
HANDS_SRV=""
trap cleanup EXIT
[ "${tl:-0}" -ge 1 ] || { echo "端末のタイムラインが記録として入らない"; exit 1; }
echo "   → フォルダを作らずにタイムラインの訪問が $tl 件入った"
