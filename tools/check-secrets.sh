#!/usr/bin/env bash
# 秘密（API キー・トークン・秘密鍵・認証ヘッダなど）が**リポジトリに入っていない**ことを gitleaks で確かめる。
#
#   tools/check-secrets.sh --staged         # staged だけ見る（.githooks/pre-commit → tools/pre-commit.sh）
#   tools/check-secrets.sh --range A..B     # その範囲の commit で足された行だけ見る（CI の chain job。PR の commit）
#   tools/check-secrets.sh                  # 履歴全体（2164 commit で約 2 秒。2026-10-17 実測）
#   tools/check-secrets.sh --self-test      # 偽の鍵を足した staged で落ち、値が出力に出ないこと
#
# check-private.sh（網の名前・私設 IP）と check-db-secret.sh（DB の合言葉の字面）は ashiato2 固有の語を見る。
# こちらは**汎用の形**（クラウドの鍵・GitHub の token・秘密鍵・`password=…` など、gitleaks の既定の規則）を見る。
#
# 誤検知の逃がし方（値は書かない。出力も --redact で伏せる）:
# - 今から足す行: 行末に `gitleaks:allow` のコメント
# - 過去の commit に既にあるもの: `.gitleaksignore` に fingerprint（`<commit>:<file>:<rule>:<line>`）を 1 行
#
# gitleaks は版と sha256 を固定して ~/.cache に置く（初回だけ取りに行く）。**取れなければ落ちる**（黙って飛ばさない）。
# 費用: gitleaks は MIT の単体の実行ファイルで、ライセンスも外部サービスも要らない。
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=8.30.1
SHA256=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb   # gitleaks_8.30.1_linux_x64.tar.gz

gitleaks_bin() {
  if [ -n "${GITLEAKS_BIN:-}" ]; then echo "$GITLEAKS_BIN"; return; fi
  local dir="${XDG_CACHE_HOME:-$HOME/.cache}/ashiato2/gitleaks-$VERSION"
  if [ ! -x "$dir/gitleaks" ]; then
    local tgz="gitleaks_${VERSION}_linux_x64.tar.gz" tmp
    tmp="$(mktemp -d)"
    curl -fsSL -o "$tmp/$tgz" "https://github.com/gitleaks/gitleaks/releases/download/v$VERSION/$tgz" \
      || { echo "error: gitleaks $VERSION を取れなかった（網が無い？ GITLEAKS_BIN で場所を渡せる）" >&2; exit 1; }
    echo "$SHA256  $tmp/$tgz" | sha256sum -c --quiet - \
      || { echo "error: gitleaks の sha256 が合わない" >&2; exit 1; }
    mkdir -p "$dir"
    tar -xzf "$tmp/$tgz" -C "$dir" gitleaks
    rm -rf "$tmp"
  fi
  echo "$dir/gitleaks"
}

GL="$(gitleaks_bin)"
common=(--no-banner --redact --exit-code 1 --log-level warn)

case "${1:-}" in
  --staged)
    "$GL" git --staged "${common[@]}" . ;;
  --range)
    "$GL" git --log-opts="${2:?--range に A..B を渡す}" "${common[@]}" . ;;
  --self-test)
    # 使い捨ての repo に偽の鍵（gitleaks の規則が拾う形・本物ではない）を staged して、落ちることと値が出ないことを見る
    tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
    fake="ghp_$(printf 'Ab1Cd2Ef3Gh4Ij5Kl6Mn7Op8Qr9St0Uv1Wx2')"
    git -C "$tmp" init -q
    printf 'token = "%s"\n' "$fake" > "$tmp/leak.txt"
    git -C "$tmp" add leak.txt
    if out="$(cd "$tmp" && "$GL" git --staged "${common[@]}" --log-level info . 2>&1)"; then
      echo "self-test 失敗: 偽の鍵を staged しても落ちなかった" >&2; exit 1
    fi
    if grep -qF "$fake" <<<"$out"; then
      echo "self-test 失敗: 出力に値が出た" >&2; exit 1
    fi
    echo "self-test 通過（偽の鍵で落ち、値は伏せられた）" ;;
  "")
    "$GL" git "${common[@]}" . ;;
  *)
    echo "error: 知らない引数: $1" >&2; exit 2 ;;
esac
