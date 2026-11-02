#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <run-id> <版番号>|--self-test" >&2; exit 2; }

# 下書きの印の名前は渡した版番号（v<版番号>）で、-beta. を含めば先行公開にする（D16）。
# 配布物は PR の package の組み立てで、PR の package は ASHIATO_BUILD_VERSION を渡さず出所の版で組む（D14）。
# だから配布物の検査は出所の版と比べる（final review R5）。渡す版番号の MAJOR.MINOR.PATCH は出所の版と同じでなければならない。
draft_release() (
  local run_id="$1" version="$2" src work tag name prerelease=0
  [[ "$run_id" =~ ^[0-9]+$ ]] || die "run-id が不正: $run_id"
  "$root/tools/release/version.sh" check "$version" >/dev/null
  src="$("$root/tools/release/version.sh" source)"

  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-draft.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  tag="v$version"
  [[ "$version" == *-beta.* ]] && prerelease=1

  # run の出所を確かめる（公開の repo では fork の PR も ci を走らせ release-package を上げる。fork のコードで組んだものを
  # この repo の下書きに載せない。security-review R31）。この repo の枝から走り、成功した ci の run だけを受ける
  local run_info head_repo repo conclusion workflow
  run_info="$(gh api "repos/{owner}/{repo}/actions/runs/$run_id" \
    --jq '[(.head_repository.full_name // "-"), .repository.full_name, (.conclusion // "-"), .name] | @tsv')" || die "run $run_id を読めない"
  IFS=$'\t' read -r head_repo repo conclusion workflow <<<"$run_info"
  [[ -n "$repo" && "$head_repo" == "$repo" ]] || die "run $run_id はこの repo の枝の組み立てでない（$head_repo）"
  [[ "$workflow" == ci ]] || die "run $run_id は ci の run でない（$workflow）"
  [[ "$conclusion" == success ]] || die "run $run_id は成功していない（$conclusion）"
  gh run download "$run_id" --name release-package --dir "$work"
  "$root/tools/release/check-artifacts.sh" "$work" --allow-debug-key --expect-version "$src"

  # 載せるのは check-artifacts が見た 4 つの配布物だけ（置き場の他のファイルは載せない。final review R22）
  local -a files=()
  for name in \
    "ashiato-server-${src}-linux-x86_64.tar.gz" \
    "ashiato-server-${src}-windows-x86_64.zip" \
    "ashiato-collector-windows-${src}-x86_64.zip" \
    "ashiato-collector-${src}.apk"; do
    files+=("$work/$name")
  done
  # 本文を渡す（TTY から走らせても gh が本文を対話で聞かない。final review R22）
  local -a args=(release create "$tag" "${files[@]}" --title "$tag" --draft --target main
    --notes "ST52 の確かめの下書き。配布物は PR の package（出所の版 $src）の組み立て（run $run_id）。公開しない")
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
if [[ "${1:-}" == api ]]; then
  [[ "${2:-}" == "repos/{owner}/{repo}/actions/runs/42" ]] || exit 23
  printf '%s\n' "${DRAFT_SELF_TEST_RUN:-o/ashiato	o/ashiato	success	ci}"
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

  # 版番号は生きた出所の版から組む（出所の版を上げても壊れない。final review R10）。
  # 偽の gh の配布物は PR の package と同じく出所の版の名前で作る（R5）
  local version
  version="$("$root/tools/release/version.sh" source)"
  export DRAFT_SELF_TEST_VERSION="$version"

  echo "== Scenario: 確かめの組み立ては、印を作らずに下書きのリリースになる"
  DRAFT_SELF_TEST_CALLS="$calls" PATH="$fake_bin:$PATH" "$0" 42 "${version}-beta.1"
  [[ "$(wc -l <"$calls")" == 1 ]] || die '下書きの要求が 1 つでない'
  grep -Fq "release create v${version}-beta.1 " "$calls" || die '下書きの release create が無い'
  grep -Fq -- '--draft' "$calls" || die 'draft 指定が無い'
  grep -Fq -- '--prerelease' "$calls" || die 'prerelease 指定が無い'
  grep -Fq -- '--target main' "$calls" || die 'main target 指定が無い'
  grep -Fq -- '--notes' "$calls" || die '本文の指定が無い'
  grep -Fq "出所の版 $version" "$calls" || die '本文に配布物の出所の版が無い'
  ! grep -Eq 'tag|publish' "$calls" || die '印または公開の要求が出た'
  [[ "$(tr ' ' '\n' <"$calls" | grep -cE '\.(tar\.gz|zip|apk)$')" == 4 ]] || die '載せる配布物が 4 つでない'
  for name in "ashiato-server-$version-linux-x86_64.tar.gz" "ashiato-server-$version-windows-x86_64.zip" \
    "ashiato-collector-windows-$version-x86_64.zip" "ashiato-collector-$version.apk"; do
    grep -Fq "/$name" "$calls" || die "下書きに $name が無い"
  done
  ! grep -Fq 'extra.txt' "$calls" || die '検査していないファイルを載せた'
  : >"$calls"
  # 正式版の形の版番号では先行公開にしない
  DRAFT_SELF_TEST_CALLS="$calls" PATH="$fake_bin:$PATH" "$0" 42 "$version" >/dev/null
  grep -Fq "release create v$version " "$calls" || die '正式版の形の下書きの release create が無い'
  ! grep -Fq -- '--prerelease' "$calls" || die '正式版の形の版番号で先行公開にした'

  echo "== Scenario: 確かめの組み立ても、配布物が欠ければ下書きを作らない"
  : >"$calls"
  if DRAFT_SELF_TEST_CALLS="$calls" DRAFT_SELF_TEST_MISSING_APK=1 PATH="$fake_bin:$PATH" \
      "$0" 42 "${version}-beta.1" >/dev/null 2>&1; then
    die '配布物が欠けたのに成功した'
  fi
  [[ ! -s "$calls" ]] || die '配布物が欠けたのに配布元へ要求を出した'

  # fork の PR・ci 以外・成功していない run の配布物は載せない（security-review R31）
  local run out
  for run in $'fork/ashiato\to/ashiato\tsuccess\tci' $'o/ashiato\to/ashiato\tsuccess\tnightly' $'o/ashiato\to/ashiato\tfailure\tci'; do
    : >"$calls"
    if out="$(DRAFT_SELF_TEST_CALLS="$calls" DRAFT_SELF_TEST_RUN="$run" PATH="$fake_bin:$PATH" "$0" 42 "${version}-beta.1" 2>&1)"; then
      die "出所の確かめを通らない run で成功した: $run"
    fi
    grep -Fq 'run 42 は' <<<"$out" || die "出所の確かめ以外の理由で落ちた: $out"
    [[ ! -s "$calls" ]] || die "出所の確かめを通らない run で下書きを作った: $run"
  done
  echo "OK: 下書きの先行公開と配布物欠落時の停止を確認した"
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
[[ $# == 2 ]] || usage
draft_release "$1" "$2"
