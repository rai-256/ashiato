#!/usr/bin/env bash
# 手元の網の名前・私設 IP が**リポジトリに入っていない**ことを確かめる。
#
#   tools/check-private.sh            # 追跡ファイル全部を見る（CI の chain job）
#   tools/check-private.sh --staged   # staged だけ見る（.githooks/pre-commit）
#
# **`.gitignore` では守れない。** 守れるのは `.env` のような「置かないファイル」だけで、
# 実際に漏れていたのは `docs/` と `openspec/` の**追跡済みの文書**だった
# （実測 2026-09-19: 公開準備の commit が 5 ファイル中 3 つしか直せておらず、
# `docs/verify/20260916-1442.json` と ST02 の review に網のホスト名と IP が残っていた）。
# 人が grep し忘れる形で残るので、機械に見させる。
set -euo pipefail
cd "$(dirname "$0")/.."

staged=0
[ "${1:-}" = "--staged" ] && staged=1

# 見つけたら止めるもの。**実値の形**だけを書き、置き換え後の雛形は下の許容で逃がす
patterns=(
  # tailnet のホスト名（`xxxx.ts.net`）
  '[A-Za-z0-9_-]+\.ts\.net'
  # Tailscale の CGNAT 100.64.0.0/10
  '\b100\.(6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.[0-9]{1,3}\.[0-9]{1,3}\b'
  # RFC1918（家の LAN）
  '\b192\.168\.[0-9]{1,3}\.[0-9]{1,3}\b'
  '\b10\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\b'
  '\b172\.(1[6-9]|2[0-9]|3[01])\.[0-9]{1,3}\.[0-9]{1,3}\b'
)
# 雛形・説明のための書き方は通す（**実値でないと分かる形だけ**）
allow='example\.ts\.net|tailXXXXXX\.ts\.net|<[^>]*>\.ts\.net|\.ts\.net` *$'

# この検査自身はパターンを持っているので見ない
self='tools/check-private.sh'

# **意図して残すファイル**は `tools/check-private.allow` に 1 行 1 パス（`#` はコメント）。
# 当時そう書いたという事実そのものである記録（確認バッチ・独立レビュー）を後から
# 書き換えない、という判断を採るならここに置く。**空なら全部を見る。**
allowfile='tools/check-private.allow'
skip_path() {
  [ -f "$allowfile" ] || return 1
  grep -vE '^\s*(#|$)' "$allowfile" 2>/dev/null | grep -qxF "$1"
}

if [ "$staged" -eq 1 ]; then
  mapfile -d '' -t files < <(git diff --cached --name-only -z --diff-filter=ACMR)
else
  mapfile -d '' -t files < <(git ls-files -z)
fi

ng=0
for f in "${files[@]}"; do
  [ -n "$f" ] || continue
  [ "$f" = "$self" ] && continue
  skip_path "$f" && continue
  [ -f "$f" ] || continue
  grep -Iq . "$f" 2>/dev/null || continue     # バイナリは見ない
  for p in "${patterns[@]}"; do
    hits="$(grep -nEI "$p" "$f" 2>/dev/null | grep -vE "$allow" || true)"
    [ -z "$hits" ] && continue
    while IFS= read -r line; do
      [ -n "$line" ] || continue
      echo "  NG $f:$line"
      ng=1
    done <<< "$hits"
  done
done

if [ "$ng" -ne 0 ]; then
  cat >&2 <<'MSG'

error: 手元の網の名前か私設 IP がリポジトリに入ろうとしています。

  このリポジトリは公開する前提です。実値は `.env`（.gitignore 済み）にだけ置き、
  文書やコードには `<手元の網のホスト名>` のような伏せ字を書きます。
  画面の許可ホストは `.env` の ALLOWED_HOSTS から読みます（web/vite.config.ts）。

  どうしても通す必要があるなら: HARNESS_ALLOW_PRIVATE_NAMES=1 git commit ...
MSG
  exit 1
fi

echo "OK 網の名前・私設 IP は入っていない（${#files[@]} ファイル）"
