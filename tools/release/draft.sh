#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <run-id> <版番号>|--self-test" >&2; exit 2; }

draft_release() (
  local run_id="$1" version="$2" work tag prerelease=0
  [[ "$run_id" =~ ^[0-9]+$ ]] || die "run-id が不正: $run_id"
  "$root/tools/release/version.sh" check "$version" >/dev/null

  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-draft.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  tag="v$version"
  [[ "$version" == *-beta.* ]] && prerelease=1

  gh run download "$run_id" --name release-package --dir "$work"
  "$root/tools/release/check-artifacts.sh" "$work" --allow-debug-key --expect-version "$version"

  local -a args=(release create "$tag" "$work"/* --title "$tag" --draft --target main)
  [[ "$prerelease" == 1 ]] && args+=(--prerelease)
  gh "${args[@]}"
)

self_test() {
  local work fake_bin calls
  work="$(mktemp -d "${TMPDIR:-/tmp}/release-draft.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  fake_bin="$work/bin"
  calls="$work/gh-calls"
  mkdir -p "$fake_bin"

  cat >"$fake_bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
calls="${DRAFT_SELF_TEST_CALLS:?}"
if [[ "${1:-}" == run && "${2:-}" == download ]]; then
  [[ "${3:-}" == 42 && "${4:-}" == --name && "${5:-}" == release-package && "${6:-}" == --dir ]] || exit 21
  out="${7:?}"
  mkdir -p "$out"
  stamp='0.1.0-beta.1+0000000000000000000000000000000000000000'
  printf '%s\n' "$stamp" >"$out/stamp"
  tar -czf "$out/ashiato-server-0.1.0-beta.1-linux-x86_64.tar.gz" -C "$out" stamp
  (cd "$out" && zip -q "ashiato-server-0.1.0-beta.1-windows-x86_64.zip" stamp)
  (cd "$out" && zip -q "ashiato-collector-windows-0.1.0-beta.1-x86_64.zip" stamp)
  if [[ "${DRAFT_SELF_TEST_MISSING_APK:-0}" != 1 ]]; then
    cp "$out/stamp" "$out/classes.dex"
    (cd "$out" && zip -q "ashiato-collector-0.1.0-beta.1.apk" classes.dex)
  fi
  rm -f "$out/stamp" "$out/classes.dex"
  exit 0
fi
if [[ "${1:-}" == release && "${2:-}" == create ]]; then
  printf '%s\n' "$*" >>"$calls"
  exit 0
fi
exit 22
EOF
  chmod +x "$fake_bin/gh"

  cat >"$fake_bin/apkanalyzer" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cat <<'FIELDS'
.field public static final BASE_URL:Ljava/lang/String; = ""
.field public static final API_TOKEN:Ljava/lang/String; = ""
.field public static final USER_ID:Ljava/lang/String; = ""
FIELDS
EOF
  chmod +x "$fake_bin/apkanalyzer"

  echo "== Scenario: 確かめの組み立ては、印を作らずに下書きのリリースになる"
  DRAFT_SELF_TEST_CALLS="$calls" PATH="$fake_bin:$PATH" "$0" 42 0.1.0-beta.1
  [[ "$(wc -l <"$calls")" == 1 ]] || die '下書きの要求が 1 つでない'
  grep -Fq 'release create v0.1.0-beta.1' "$calls" || die '下書きの release create が無い'
  grep -Fq -- '--draft' "$calls" || die 'draft 指定が無い'
  grep -Fq -- '--prerelease' "$calls" || die 'prerelease 指定が無い'
  grep -Fq -- '--target main' "$calls" || die 'main target 指定が無い'
  ! grep -Eq 'tag|publish' "$calls" || die '印または公開の要求が出た'

  echo "== Scenario: 確かめの組み立ても、配布物が欠ければ下書きを作らない"
  : >"$calls"
  if DRAFT_SELF_TEST_CALLS="$calls" DRAFT_SELF_TEST_MISSING_APK=1 PATH="$fake_bin:$PATH" \
      "$0" 42 0.1.0-beta.1 >/dev/null 2>&1; then
    die '配布物が欠けたのに成功した'
  fi
  [[ ! -s "$calls" ]] || die '配布物が欠けたのに配布元へ要求を出した'
  echo "OK: 下書きの先行公開と配布物欠落時の停止を確認した"
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
[[ $# == 2 ]] || usage
draft_release "$1" "$2"
