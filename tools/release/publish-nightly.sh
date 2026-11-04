#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <置き場> <版番号> | --self-test" >&2; exit 2; }

publish_nightly() (
  local dir="$1" version="$2" temp key signed apk asset keep
  [[ -d "$dir" ]] || die "配布物の置き場が無い: $dir"
  "$root/tools/release/version.sh" check "$version" >/dev/null
  temp="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/ashiato-nightly-key.$$"
  key="$temp/dev.jks"
  mkdir -p "$temp"
  cleanup() { rm -rf "$temp"; }
  trap cleanup EXIT
  [[ -n "${ASHIATO_SIGNING_KEYSTORE_B64:-}" && -n "${ASHIATO_SIGNING_STORE_PASSWORD:-}" &&
    -n "${ASHIATO_SIGNING_KEY_ALIAS:-}" && -n "${ASHIATO_SIGNING_KEY_PASSWORD:-}" &&
    -n "${ASHIATO_EXPECT_DEV_CERT_SHA256:-}" ]] || die '開発版の鍵が無い'
  printf '%s' "$ASHIATO_SIGNING_KEYSTORE_B64" | base64 --decode >"$key" || die '開発版の鍵を復元できない'
  apk="$dir/ashiato-collector-$version.apk"
  [[ -f "$apk" ]] || die "開発版の APK が無い: $(basename "$apk")"
  signed="$dir/ashiato-collector-dev-$version.apk"
  ASHIATO_EXPECT_DEV_CERT_SHA256="$ASHIATO_EXPECT_DEV_CERT_SHA256" \
    ASHIATO_SIGNING_KEYSTORE_B64="$(base64 -w0 "$key")" \
    "$root/tools/release/sign-apk.sh" "$apk" dev "$signed"
  rm -f "$apk"
  ASHIATO_EXPECT_CERT_SHA256="$ASHIATO_EXPECT_CERT_SHA256" \
    "$root/tools/release/check-artifacts.sh" "$dir" --channel nightly --expect-version "$version"
  if ! gh release view nightly >/dev/null 2>&1; then
    gh release create nightly --prerelease --title nightly --target "$GITHUB_SHA" \
      --notes-file "$root/tools/release/nightly-notes.md"
  fi
  gh release upload nightly "$dir"/* --clobber
  keep="$(find "$dir" -maxdepth 1 -type f -printf '%f\n')"
  gh release view nightly --json assets --jq '.assets[].name' | while IFS= read -r asset; do
    if [[ -n "$asset" ]] && ! grep -Fxq -- "$asset" <<<"$keep"; then
      gh release delete-asset nightly "$asset" --yes
    fi
  done
  gh release edit nightly --notes-file "$root/tools/release/nightly-notes.md"
  local first_line
  first_line="$(gh release view nightly --json body --jq '.body' | head -n1)"
  [[ "$first_line" == この開発版はサポートしません。* ]] || die '開発版の説明文の先頭が正しくない'
  git push --force origin "${GITHUB_SHA:?}:refs/tags/nightly"
)

self_test() {
  local work="$(mktemp -d "${TMPDIR:-/tmp}/publish-nightly.XXXXXX")" version calls fakebin password=self-test-password
  trap "rm -rf '$work'" EXIT
  fakebin="$work/bin"; calls="$work/calls"; mkdir -p "$fakebin"
  version="$($root/tools/release/version.sh source)-dev.20261102"
  cat >"$fakebin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${NIGHTLY_CALLS:?}"
if [[ "$1 $2 $3" == "release view nightly" && $# == 3 ]]; then exit 1; fi
if [[ "$1 $2 $3 $4" == "release view nightly --json" ]]; then
  if [[ "${*: -1}" == ".body" ]]; then
    [[ "${NIGHTLY_BAD_NOTES:-0}" == 1 ]] && printf '%s\n' '古い説明文' || printf '%s\n' 'この開発版はサポートしません。'
  fi
  exit 0
fi
exit 0
EOF
  chmod +x "$fakebin/gh"
  cat >"$fakebin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${NIGHTLY_CALLS:?}"
EOF
  chmod +x "$fakebin/git"
  echo "== Scenario: 開発版の鍵の指紋の期待値が無ければ、開発版の APK に署名しない"
  if env -u ASHIATO_EXPECT_DEV_CERT_SHA256 PATH="$fakebin:$PATH" GITHUB_SHA=deadbeef "$0" "$work/missing" "$version" >/dev/null 2>&1; then die '期待の指紋なしで公開した'; fi
  echo "== Scenario: 開発版の鍵が置き場に無ければ、開発版は前の版のまま残る"
  mkdir -p "$work/missing"
  if env -u ASHIATO_SIGNING_KEYSTORE_B64 PATH="$fakebin:$PATH" GITHUB_SHA=deadbeef "$0" "$work/missing" "$version" 2>"$work/missing.err"; then die '鍵無しで公開した'; fi
  grep -Fq '開発版の鍵が無い' "$work/missing.err"
  [[ ! -s "$calls" ]] || die '鍵無しで配布元へ要求した'
  echo "== Scenario: 開発版を公開するたびに、説明文は決まった文で書き直される"
  "$root/tools/release/make-key.sh" dev "$work/dev.jks" <<<"$password" >/dev/null
  local b64 cert
  b64="$(base64 -w0 "$work/dev.jks")"
  cert="$(keytool -J-Duser.language=en -J-Duser.country=US -list -v -keystore "$work/dev.jks" -storepass "$password" 2>/dev/null | sed -n 's/^\s*SHA256: //p' | tr -d ':' | tr 'A-F' 'a-f' | head -n1)"
  local unsigned="$work/ashiato-collector-$version.apk"
  GRADLE_OPTS="${GRADLE_OPTS:-} -Dkotlin.compiler.execution.strategy=in-process" ASHIATO_BUILD_VERSION="$version" ASHIATO_APK_UNSIGNED=1 \
    "$root/tools/release/build.sh" apk "$work/out" >/dev/null 2>&1
  mv "$work/out/ashiato-collector-$version.apk" "$unsigned"
  for f in "$work/ashiato-server-$version-linux-x86_64.tar.gz" "$work/ashiato-server-$version-windows-x86_64.zip" "$work/ashiato-collector-windows-$version-x86_64.zip"; do
    printf '%s+0000000000000000000000000000000000000000\n' "$version" >"$work/stamp"
    case "$f" in *.tar.gz) tar -czf "$f" -C "$work" stamp;; *) (cd "$work" && zip -q "$f" stamp);; esac
  done
  : >"$calls"
  NIGHTLY_CALLS="$calls" PATH="$fakebin:$PATH" GITHUB_SHA=deadbeef RUNNER_TEMP="$work/temp" \
    ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
    ASHIATO_EXPECT_DEV_CERT_SHA256="$cert" ASHIATO_EXPECT_CERT_SHA256="$cert" "$0" "$work" "$version" >/dev/null
  grep -Fq 'release edit nightly --notes-file' "$calls" || die '説明文の書き直しが無い'
  grep -Fq 'push --force origin deadbeef:refs/tags/nightly' "$calls"
  echo "== Scenario: 説明文の先頭にサポートしない旨が無ければ、公開の手順は落ちる"
  local bad="$work/bad"; mkdir -p "$bad"
  cp "$work/ashiato-collector-dev-$version.apk" "$bad/ashiato-collector-$version.apk"
  cp "$work/ashiato-server-$version-linux-x86_64.tar.gz" "$bad/"
  cp "$work/ashiato-server-$version-windows-x86_64.zip" "$bad/"
  cp "$work/ashiato-collector-windows-$version-x86_64.zip" "$bad/"
  : >"$calls"
  if NIGHTLY_BAD_NOTES=1 NIGHTLY_CALLS="$calls" PATH="$fakebin:$PATH" GITHUB_SHA=deadbeef RUNNER_TEMP="$work/temp2" \
      ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
      ASHIATO_EXPECT_DEV_CERT_SHA256="$cert" ASHIATO_EXPECT_CERT_SHA256="$cert" "$0" "$bad" "$version" >/dev/null 2>&1; then
    die '不正な説明文を受け入れた'
  fi
  ! grep -Fq 'push --force origin' "$calls" || die '不正な説明文で印を動かした'
  echo "OK: 開発版の鍵、配布物、説明文、印の更新順を確認した"
}

[[ "${1:-}" == --self-test ]] && { self_test; exit 0; }
[[ $# == 2 ]] || usage
publish_nightly "$1" "$2"
