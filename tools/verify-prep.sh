#!/usr/bin/env bash
# 確認バッチの成果物を作る（harness2 の scripts/verify_batch.sh が呼ぶ。単独でも使える）。
#
#   ./tools/verify-prep.sh <tag>
#     → dist/verify-<tag>/ashiato-server   サーバ（release）
#     → dist/verify-<tag>/web/             画面（vite build）
#     → dist/verify-<tag>/app-debug.apk    Android（JDK / SDK が ~/.local/opt にあるとき）
#     → dist/verify-<tag>/run.sh           DB → サーバ → 画面 → 偽データ を 1 コマンドで起動
#     → dist/verify-<tag>/manifest.md      起動手順（確認手順書の冒頭に載る）
#
# **確認しないといけないタイミングで、できる準備は全部ここでやる。** 実機が繋がっていれば APK も入れる。
# 人間がやるのは run.sh を叩いて、手順書のとおりに見ることだけ。
set -euo pipefail
cd "$(dirname "$0")/.."
tag="${1:?使い方: $0 <tag>}"
out="dist/verify-$tag"
mkdir -p "$out"
note() { echo "  $*"; }

echo "== サーバ（release）"
cargo build -q --release -p ashiato-server --bin ashiato-server
cp target/release/ashiato-server "$out/ashiato-server"

echo "== 画面（build）"
# npm ci は毎回。node_modules があっても dev 依存が欠けていることがある（実測 2026-09-12: vitest と
# @testing-library/react が無く tsc -b が落ちた。CI は毎回 npm ci なので気付かない）
(cd web && npm ci --silent >/dev/null && npm run build --silent >/dev/null)
rm -rf "$out/web" && cp -r web/dist "$out/web"

echo "== Android（APK）"
apk=""; device=""; android_note="ビルドしていない（~/.local/opt/jdk21 が無い。tools/android-env.sh を見る）"
if [ -f tools/android-env.sh ] && [ -d "$HOME/.local/opt/jdk21" ]; then
  # shellcheck disable=SC1091
  . ./tools/android-env.sh
  if (cd collector-android && ./gradlew -q :app:assembleDebug >/dev/null 2>&1); then
    cp collector-android/app/build/outputs/apk/debug/app-debug.apk "$out/app-debug.apk"
    apk="$out/app-debug.apk"
    android_note="\`$apk\`"
    if command -v adb >/dev/null; then
      device="$(adb devices 2>/dev/null | awk 'NR>1 && $2=="device"{print $1}' | head -1 || true)"
      if [ -n "$device" ] && adb -s "$device" install -r "$apk" >/dev/null 2>&1; then
        android_note="\`$apk\` を端末 $device に入れた（adb install -r）"
      else
        android_note="\`$apk\`。端末が繋がっていないので手で入れる: \`adb install -r $apk\`"
      fi
    fi
  else
    android_note="gradle が落ちた。\`cd collector-android && ./gradlew :app:assembleDebug\` で確かめる"
  fi
fi
note "$android_note"

# ---- run.sh（ビルド済みのものを起動する。ソースからは何も作らない）
cat > "$out/run.sh" <<EOF
#!/usr/bin/env bash
# 確認バッチ $tag の起動。DB → サーバ（release）→ 画面（build 済み）→ 偽データ。Ctrl-C で全部止まる。
#   SEED=max ./dist/verify-$tag/run.sh     # 偽データの量: normal（既定）/ max / empty
set -euo pipefail
cd "\$(dirname "\$0")/../.."
if [ -f .env ]; then set -a; . ./.env; set +a; fi
export DATABASE_URL="\${DATABASE_URL:-postgres://ashiato:ashiato@127.0.0.1:55432/ashiato}"
export BIND="\${BIND:-127.0.0.1:18787}"
export API_TOKEN="\${API_TOKEN:-dev-token-0123456789abcdef}"
export WEB_PORT="\${WEB_PORT:-5180}"     # 開発用 vite（5173）と衝突しない番号。--strictPort で黙って逃げない
# port が使用中なら 30 秒待たずにここで止める（実測: 5173 を別プロジェクトの vite が使っていて、
# preview が隣の番号に逃げ、確認者は別の画面を見ていた）
# 同じ host:port か、全インタフェース（0.0.0.0 / [::] / *）で塞がれているときだけ「使用中」。
# 番号だけで見ない —— 本番のサーバが Tailscale の IP:18787 で動いている横で 127.0.0.1:18787 は使える（実測）
busy() { local l port="\${1##*:}"; l="\$(ss -ltn 2>/dev/null | awk '{print \$4}')"
  printf '%s\\n' "\$l" | grep -qxF "\$1" || printf '%s\\n' "\$l" | grep -qE "^(0\\.0\\.0\\.0|\\[::\\]|\\*):\$port\$"; }
busy "\$BIND" && { echo "error: \$BIND は使用中（\$(ss -ltnp 2>/dev/null | grep -F "\$BIND " | grep -oE 'users:\\(.*' | head -1)）。BIND を変えるか、そのサーバを止める"; exit 1; }
busy "127.0.0.1:\$WEB_PORT" && { echo "error: port \$WEB_PORT は使用中。WEB_PORT=<別の番号> で叩き直す"; exit 1; }
echo "== DB"; docker compose up -d --wait db >/dev/null
trap 'kill 0' EXIT
echo "== サーバ \$BIND"; ./dist/verify-$tag/ashiato-server &
for _ in \$(seq 1 30); do curl -sf "http://\$BIND/healthz" >/dev/null && break; sleep 1; done
curl -sf "http://\$BIND/healthz" >/dev/null || { echo "サーバが起動しない（BIND=\$BIND）"; exit 1; }
echo "== 偽データ（\${SEED:-normal}）"; ./tools/seed.sh "\${SEED:-normal}" >/dev/null || echo "warn: seed が落ちた（続ける）"
echo "== 画面 http://127.0.0.1:\$WEB_PORT"
(cd web && npx vite preview --host 127.0.0.1 --port "\$WEB_PORT" --strictPort --outDir ../dist/verify-$tag/web >/dev/null 2>&1) &
echo
echo "画面: http://127.0.0.1:\$WEB_PORT    API: http://\$BIND    （端末から届くには BIND を LAN / Tailscale の IP にする）"
wait
EOF
chmod +x "$out/run.sh"

# ---- manifest.md（確認手順書の冒頭に載る。段落ごとに 1 つの note になる）
cat > "$out/manifest.md" <<EOF
起動: \`./dist/verify-$tag/run.sh\` —— DB → サーバ（release、ビルド済み）→ 画面（build 済み）→ 偽データ（SEED=normal|max|empty）。Ctrl-C で全部止まる。

画面 \`http://127.0.0.1:5180\`（\`WEB_PORT\` で変えられる）/ API は \`.env\` の BIND（既定 \`127.0.0.1:18787\`）。port が使用中なら run.sh がその場で止まる。どの画面がどの URL かは \`docs/screens.md\`。

スマホ・端末から届かせるなら \`http://yoshi.tail4360f4.ts.net:<port>\` ——  **IP ではなくホスト名**（\`tailscale serve\` はホスト名で振り分けるので、\`100.85.27.45\` 宛は tailscale 自身が 404 を返す。実測 2026-09-16: 収集アプリが \`error=server_404\` を出し続けた）。

確認に使う 1 行（手順書の問いが「手順書の…を叩く」と書いているもの）——
作り直し: \`curl -sS -H "authorization: Bearer \$API_TOKEN" -H 'content-type: application/json' -X POST http://127.0.0.1:18787/stays/rebuild -d '{"radius_m":30}'\`（戻すときは \`100\`。範囲外の値は 400 で、基準も滞在も変わらない）。
位置が変わっていないこと: \`docker exec ashiato2-db-1 psql -U ashiato -d ashiato -tAc "select count(*), md5(string_agg(content_hash, ',' order by content_hash)) from core.event where logical_source='c01-location'"\`（作り直しの前後で同じ値）。

Android: $android_note。接続先は \`~/.gradle/gradle.properties\` の \`ashiato.baseUrl\`（\`collector-android/README.md\`）。

成果物: \`$out/\`（ashiato-server / web / $( [ -n "$apk" ] && echo app-debug.apk || echo "APK なし" ) / run.sh）。ソースは verify ブランチの head。
EOF

echo "== 出来た: $out/"
ls -1 "$out"
