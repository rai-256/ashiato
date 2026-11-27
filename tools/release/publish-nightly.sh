#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <置き場> <版番号> <印 nightly を動かす先の commit> | --self-test" >&2; exit 2; }

publish_nightly() (
  local dir="$1" version="$2" target="${3:-}" temp key signed apk asset keep
  [[ "$target" =~ ^[0-9a-f]{40}$ ]] || die "印 nightly を動かす先の commit（40 桁の hex）が無い: '$target'"
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
  # 戻した鍵のファイルで署名する（B64 は渡さない）。検査に渡す指紋も開発版の鍵のもの（draft.sh と同じ。final review R17）
  env -u ASHIATO_SIGNING_KEYSTORE_B64 ASHIATO_SIGNING_KEYSTORE_FILE="$key" \
    ASHIATO_EXPECT_DEV_CERT_SHA256="$ASHIATO_EXPECT_DEV_CERT_SHA256" \
    "$root/tools/release/sign-apk.sh" "$apk" dev "$signed"
  rm -f "$apk"
  ASHIATO_EXPECT_CERT_SHA256="$ASHIATO_EXPECT_DEV_CERT_SHA256" \
    "$root/tools/release/check-artifacts.sh" "$dir" --channel nightly --expect-version "$version"
  if ! gh release view nightly >/dev/null 2>&1; then
    gh release create nightly --prerelease --title nightly --target "$target" \
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
  git push --force origin "$target:refs/tags/nightly"
)

self_test() {
  local run_sha=dddddddddddddddddddddddddddddddddddddddd target_sha=cccccccccccccccccccccccccccccccccccccccc
  local work="$(mktemp -d "${TMPDIR:-/tmp}/publish-nightly.XXXXXX")" version calls fakebin password
  password="$(od -An -tx1 -N16 /dev/urandom | tr -d " \n")"  # 使い捨ての署名鍵の合言葉。字面で置かない（check-db-secret.sh）
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
  # 配布物の検査は本物の組み立て（Windows のインストーラなど）を要するので、ここでは受けた引数だけを見る台本に差し替える。
  # 本物の検査は check-artifacts.sh --self-test と CI の package の job が持つ
  local fake_root="$work/root" pn checks="$work/checks" f
  mkdir -p "$fake_root/tools"; ln -s "$root/Cargo.toml" "$fake_root/Cargo.toml"; ln -s "$root/LICENSE" "$fake_root/LICENSE"
  cp -rs "$root/tools/release" "$fake_root/tools/release"; rm -f "$fake_root/tools/release/check-artifacts.sh"
  printf '#!/usr/bin/env bash\nprintf "%%s cert=%%s\\n" "$*" "${ASHIATO_EXPECT_CERT_SHA256:-}" >>"${NIGHTLY_CHECKS:?}"\n' >"$fake_root/tools/release/check-artifacts.sh"; chmod +x "$fake_root/tools/release/check-artifacts.sh"
  pn="$fake_root/tools/release/publish-nightly.sh"; export NIGHTLY_CHECKS="$checks"
  echo "== Scenario: 開発版の鍵が置き場に無ければ、開発版は前の版のまま残る"
  mkdir -p "$work/missing"
  if env -u ASHIATO_SIGNING_KEYSTORE_B64 PATH="$fakebin:$PATH" GITHUB_SHA=run0000 "$pn" "$work/missing" "$version" "$run_sha" 2>"$work/missing.err"; then die '鍵無しで公開した'; fi
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
  local other_password=self-test-other-password other_cert dir
  "$root/tools/release/make-key.sh" dev "$work/other.jks" <<<"$other_password" >/dev/null
  other_cert="$(keytool -J-Duser.language=en -J-Duser.country=US -list -v -keystore "$work/other.jks" -storepass "$other_password" 2>/dev/null | sed -n 's/^\s*SHA256: //p' | tr -d ':' | tr 'A-F' 'a-f' | head -n1)"
  local fingerprint_missing="$work/fingerprint-missing" fingerprint_wrong="$work/fingerprint-wrong"
  mkdir -p "$fingerprint_missing" "$fingerprint_wrong"
  for dir in "$fingerprint_missing" "$fingerprint_wrong"; do
    cp "$unsigned" "$dir/ashiato-collector-$version.apk"
    cp "$work/ashiato-server-$version-linux-x86_64.tar.gz" "$dir/"
    cp "$work/ashiato-server-$version-windows-x86_64.zip" "$dir/"
    cp "$work/ashiato-collector-windows-$version-x86_64.zip" "$dir/"
  done
  echo "== Scenario: 開発版の鍵の指紋の期待値が無ければ、開発版の APK に署名しない"
  : >"$calls"
  if env -u ASHIATO_EXPECT_DEV_CERT_SHA256 NIGHTLY_CALLS="$calls" PATH="$fakebin:$PATH" GITHUB_SHA=run0000 RUNNER_TEMP="$work/temp-missing" \
      ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
      "$pn" "$fingerprint_missing" "$version" "$run_sha" >/dev/null 2>&1; then
    die '指紋なしで開発版 APK に署名した'
  fi
  [[ ! -e "$fingerprint_missing/ashiato-collector-dev-$version.apk" ]] || die '指紋なしで署名済み APK ができた'
  [[ ! -s "$calls" ]] || die '指紋なしで配布元へ要求した'
  : >"$calls"
  if NIGHTLY_CALLS="$calls" PATH="$fakebin:$PATH" GITHUB_SHA=run0000 RUNNER_TEMP="$work/temp-wrong" \
      ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
      ASHIATO_EXPECT_DEV_CERT_SHA256="$other_cert" "$pn" "$fingerprint_wrong" "$version" "$run_sha" >/dev/null 2>&1; then
    die '別鍵の指紋で開発版 APK に署名した'
  fi
  [[ ! -e "$fingerprint_wrong/ashiato-collector-dev-$version.apk" ]] || die '別鍵の指紋で署名済み APK ができた'
  [[ ! -s "$calls" ]] || die '別鍵の指紋で配布元へ要求した'
  : >"$calls"
  NIGHTLY_CALLS="$calls" PATH="$fakebin:$PATH" GITHUB_SHA=run0000 RUNNER_TEMP="$work/temp" \
    ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
    ASHIATO_EXPECT_DEV_CERT_SHA256="$cert" ASHIATO_EXPECT_CERT_SHA256="$other_cert" "$pn" "$work" "$version" "$run_sha" >/dev/null  # 正式版の名前の指紋は使わない（別鍵の値でも通る）
  grep -Fq 'release edit nightly --notes-file' "$calls" || die '説明文の書き直しが無い'
  grep -Fq -- "--channel nightly --expect-version $version" "$checks" || die '開発版の検査の引数が違う'
  grep -Fq "cert=$cert" "$checks" || die '開発版の鍵の指紋が検査へ渡らない'
  ! grep -Fq "cert=$other_cert" "$checks" || die '正式版の名前の指紋が検査へ渡った'
  grep -Fq "push --force origin $run_sha:refs/tags/nightly" "$calls"
  # 判定が返した commit を第 3 引数で渡すと、印 nightly はその commit へ動く（run の commit ではない）
  : >"$calls"
  local target_dir="$work/target"; mkdir -p "$target_dir"
  cp "$work/ashiato-collector-dev-$version.apk" "$target_dir/ashiato-collector-$version.apk"
  cp "$work/ashiato-server-$version-linux-x86_64.tar.gz" "$work/ashiato-server-$version-windows-x86_64.zip" "$work/ashiato-collector-windows-$version-x86_64.zip" "$target_dir/"
  NIGHTLY_CALLS="$calls" PATH="$fakebin:$PATH" GITHUB_SHA=run0000 RUNNER_TEMP="$work/temp-target" \
    ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
    ASHIATO_EXPECT_DEV_CERT_SHA256="$cert" "$pn" "$target_dir" "$version" "$target_sha" >/dev/null
  grep -Fq "push --force origin $target_sha:refs/tags/nightly" "$calls" || die '印 nightly が判定の commit へ動かない'
  ! grep -Fq "$run_sha" "$calls" || die '印 nightly の先に run の commit が混じった'
  # 先の commit が空・短い・hex でないなら、GITHUB_SHA（run の commit）へ戻らず落ちる
  for bad_sha in "" deadbeef not-a-commit; do
    : >"$calls"
    if NIGHTLY_CALLS="$calls" PATH="$fakebin:$PATH" GITHUB_SHA="$run_sha" RUNNER_TEMP="$work/temp-sha" \
        ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
        ASHIATO_EXPECT_DEV_CERT_SHA256="$cert" "$pn" "$target_dir" "$version" "$bad_sha" >/dev/null 2>&1; then
      die "先の commit が '$bad_sha' で公開した"
    fi
    [[ ! -s "$calls" ]] || die "先の commit が '$bad_sha' で配布元へ要求した"
  done
  echo "== Scenario: 説明文の先頭にサポートしない旨が無ければ、公開の手順は落ちる"
  local bad="$work/bad"; mkdir -p "$bad"
  cp "$work/ashiato-collector-dev-$version.apk" "$bad/ashiato-collector-$version.apk"
  cp "$work/ashiato-server-$version-linux-x86_64.tar.gz" "$bad/"
  cp "$work/ashiato-server-$version-windows-x86_64.zip" "$bad/"
  cp "$work/ashiato-collector-windows-$version-x86_64.zip" "$bad/"
  : >"$calls"
  if NIGHTLY_BAD_NOTES=1 NIGHTLY_CALLS="$calls" PATH="$fakebin:$PATH" GITHUB_SHA=run0000 RUNNER_TEMP="$work/temp2" \
      ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
      ASHIATO_EXPECT_DEV_CERT_SHA256="$cert" "$pn" "$bad" "$version" "$run_sha" >/dev/null 2>&1; then
    die '不正な説明文を受け入れた'
  fi
  ! grep -Fq 'push --force origin' "$calls" || die '不正な説明文で印を動かした'
  echo "OK: 開発版の鍵、配布物、説明文、印の更新順を確認した"
}

[[ "${1:-}" == --self-test ]] && { self_test; exit 0; }
[[ $# == 3 ]] || usage
publish_nightly "$@"
