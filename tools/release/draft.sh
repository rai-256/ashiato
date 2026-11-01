#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <run-id> [--prerelease]|--self-test" >&2; exit 2; }

# PR の package は ASHIATO_BUILD_VERSION を渡さず出所の版で組む（D14）ので、版番号は出所の版から取る
# （final review R5）。先行公開の種類は --prerelease で選ぶ。
draft_release() (
  local run_id="$1" prerelease="$2" version work tag name
  [[ "$run_id" =~ ^[0-9]+$ ]] || die "run-id が不正: $run_id"
  version="$("$root/tools/release/version.sh" source)"
  "$root/tools/release/version.sh" check "$version" >/dev/null

  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-draft.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  tag="v$version"

  gh run download "$run_id" --name release-package --dir "$work"
  "$root/tools/release/check-artifacts.sh" "$work" --allow-debug-key --expect-version "$version"

  # 載せるのは check-artifacts が見た 4 つの配布物だけ（置き場の他のファイルは載せない。final review R22）
  local -a files=()
  for name in \
    "ashiato-server-${version}-linux-x86_64.tar.gz" \
    "ashiato-server-${version}-windows-x86_64.zip" \
    "ashiato-collector-windows-${version}-x86_64.zip" \
    "ashiato-collector-${version}.apk"; do
    files+=("$work/$name")
  done
  # 本文を渡す（TTY から走らせても gh が本文を対話で聞かない。final review R22）
  local -a args=(release create "$tag" "${files[@]}" --title "$tag" --draft --target main
    --notes "ST52 の確かめの下書き（PR の組み立て run $run_id の配布物）。公開しない")
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
  v="${DRAFT_SELF_TEST_VERSION:?}"
  mkdir -p "$out"
  stamp="$v+0000000000000000000000000000000000000000"
  printf '%s\n' "$stamp" >"$out/stamp"
  tar -czf "$out/ashiato-server-$v-linux-x86_64.tar.gz" -C "$out" stamp
  (cd "$out" && zip -q "ashiato-server-$v-windows-x86_64.zip" stamp)
  (cd "$out" && zip -q "ashiato-collector-windows-$v-x86_64.zip" stamp)
  if [[ "${DRAFT_SELF_TEST_MISSING_APK:-0}" != 1 ]]; then
    cp "$out/stamp" "$out/classes.dex"
    (cd "$out" && zip -q "ashiato-collector-$v.apk" classes.dex)
  fi
  rm -f "$out/stamp" "$out/classes.dex"
  # 検査していないファイルが置き場にあっても、下書きには載らないこと（R22）
  printf 'x\n' >"$out/extra.txt"
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

  # 版番号は生きた出所の版から組む（出所の版を上げても壊れない。final review R10）
  local version
  version="$("$root/tools/release/version.sh" source)"
  export DRAFT_SELF_TEST_VERSION="$version"

  echo "== Scenario: 確かめの組み立ては、印を作らずに下書きのリリースになる"
  DRAFT_SELF_TEST_CALLS="$calls" PATH="$fake_bin:$PATH" "$0" 42 --prerelease
  [[ "$(wc -l <"$calls")" == 1 ]] || die '下書きの要求が 1 つでない'
  grep -Fq "release create v$version " "$calls" || die '下書きの release create が無い'
  grep -Fq -- '--draft' "$calls" || die 'draft 指定が無い'
  grep -Fq -- '--prerelease' "$calls" || die 'prerelease 指定が無い'
  grep -Fq -- '--target main' "$calls" || die 'main target 指定が無い'
  grep -Fq -- '--notes' "$calls" || die '本文の指定が無い'
  ! grep -Eq 'tag|publish' "$calls" || die '印または公開の要求が出た'
  [[ "$(tr ' ' '\n' <"$calls" | grep -cE '\.(tar\.gz|zip|apk)$')" == 4 ]] || die '載せる配布物が 4 つでない'
  for name in "ashiato-server-$version-linux-x86_64.tar.gz" "ashiato-server-$version-windows-x86_64.zip" \
    "ashiato-collector-windows-$version-x86_64.zip" "ashiato-collector-$version.apk"; do
    grep -Fq "/$name" "$calls" || die "下書きに $name が無い"
  done
  ! grep -Fq 'extra.txt' "$calls" || die '検査していないファイルを載せた'
  : >"$calls"
  DRAFT_SELF_TEST_CALLS="$calls" PATH="$fake_bin:$PATH" "$0" 42 >/dev/null
  ! grep -Fq -- '--prerelease' "$calls" || die '--prerelease を付けないのに先行公開にした'

  echo "== Scenario: 確かめの組み立ても、配布物が欠ければ下書きを作らない"
  : >"$calls"
  if DRAFT_SELF_TEST_CALLS="$calls" DRAFT_SELF_TEST_MISSING_APK=1 PATH="$fake_bin:$PATH" \
      "$0" 42 --prerelease >/dev/null 2>&1; then
    die '配布物が欠けたのに成功した'
  fi
  [[ ! -s "$calls" ]] || die '配布物が欠けたのに配布元へ要求を出した'
  echo "OK: 下書きの先行公開と配布物欠落時の停止を確認した"
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
case "$#:${2:-}" in
  1:) draft_release "$1" 0 ;;
  2:--prerelease) draft_release "$1" 1 ;;
  *) usage ;;
esac
