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
# 起動の実体は tools/stack.sh。**人間が見るものと e2e が見るものを同じ起動にする**ため、
# ここでは「どのビルド済みを使うか」だけを渡す（2026-09-18）。
cat > "$out/run.sh" <<EOF
#!/usr/bin/env bash
# 確認バッチ $tag の起動。DB → サーバ（release）→ 偽データ → 画面（build 済み）。Ctrl-C で全部止まる。
#   SEED=max ./dist/verify-$tag/run.sh     # 偽データの量: normal（既定）/ max / empty
set -euo pipefail
cd "\$(dirname "\$0")/../.."
export SERVER_BIN="dist/verify-$tag/ashiato-server"
export WEB_DIST="dist/verify-$tag/web"
exec ./tools/stack.sh up
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
