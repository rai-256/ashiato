#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <run-id> <版番号>|--channel nightly <run-id> <版番号>|--self-test" >&2; exit 2; }

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

draft_nightly() (
  local run_id="$1" version="$2" work apk signed b64
  [[ "$run_id" =~ ^[0-9]+$ ]] || die "run-id が不正: $run_id"
  "$root/tools/release/version.sh" check "$version" >/dev/null
  local run_info head_repo repo conclusion workflow
  run_info="$(gh api "repos/{owner}/{repo}/actions/runs/$run_id" \
    --jq '[(.head_repository.full_name // "-"), .repository.full_name, (.conclusion // "-"), .name] | @tsv')" || die "run $run_id を読めない"
  IFS=$'\t' read -r head_repo repo conclusion workflow <<<"$run_info"
  [[ -n "$repo" && "$head_repo" == "$repo" ]] || die "run $run_id はこの repo の枝の組み立てでない（$head_repo）"
  [[ "$workflow" == nightly ]] || die "run $run_id は nightly の run でない（$workflow）"
  [[ "$conclusion" == success ]] || die "run $run_id は成功していない（$conclusion）"
  [[ -n "${ASHIATO_SIGNING_KEYSTORE_B64:-}" && -n "${ASHIATO_SIGNING_STORE_PASSWORD:-}" &&
    -n "${ASHIATO_SIGNING_KEY_ALIAS:-}" && -n "${ASHIATO_SIGNING_KEY_PASSWORD:-}" &&
    -n "${ASHIATO_EXPECT_DEV_CERT_SHA256:-}" ]] || die '開発版の鍵が無い'
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-nightly-draft.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  gh run download "$run_id" --name nightly-linux --dir "$work"
  gh run download "$run_id" --name nightly-windows --dir "$work"
  apk="$work/ashiato-collector-$version.apk"
  [[ -f "$apk" ]] || die "開発版の APK が無い: $(basename "$apk")"
  b64="$ASHIATO_SIGNING_KEYSTORE_B64"
  signed="$work/ashiato-collector-dev-$version.apk"
  ASHIATO_EXPECT_DEV_CERT_SHA256="$ASHIATO_EXPECT_DEV_CERT_SHA256" \
    ASHIATO_SIGNING_KEYSTORE_B64="$b64" "$root/tools/release/sign-apk.sh" "$apk" dev "$signed"
  rm -f "$apk"
  ASHIATO_EXPECT_CERT_SHA256="$ASHIATO_EXPECT_DEV_CERT_SHA256" \
    "$root/tools/release/check-artifacts.sh" "$work" --channel nightly --expect-version "$version"
  gh release create "nightly-draft-$run_id" "$work"/* --draft --prerelease \
    --title "nightly draft $version" --notes-file "$root/tools/release/nightly-notes.md"
)

nightly_self_test() {
  local work="$(mktemp -d "${TMPDIR:-/tmp}/release-nightly-draft.XXXXXX")" version password
  password="$(od -An -tx1 -N16 /dev/urandom | tr -d " \n")"  # 使い捨ての署名鍵の合言葉。字面で置かない（check-db-secret.sh）
  trap "rm -rf '$work'" RETURN
  local fake="$work/bin" calls="$work/calls"; mkdir -p "$fake"
  version="$($root/tools/release/version.sh source)-dev.20261102"
  printf '%s\n' "$password" | "$root/tools/release/make-key.sh" dev "$work/dev.jks" >/dev/null
  local b64 cert
  b64="$(base64 -w0 "$work/dev.jks")"
  cert="$(keytool -J-Duser.language=en -J-Duser.country=US -list -v -keystore "$work/dev.jks" -storepass "$password" 2>/dev/null | sed -n 's/^\s*SHA256: //p' | tr -d ':' | tr 'A-F' 'a-f' | head -n1)"
  GRADLE_OPTS="${GRADLE_OPTS:-} -Dkotlin.compiler.execution.strategy=in-process" ASHIATO_BUILD_VERSION="$version" ASHIATO_APK_UNSIGNED=1 \
    "$root/tools/release/build.sh" apk "$work/source" >/dev/null 2>&1
  printf '%s+0000000000000000000000000000000000000000\n' "$version" >"$work/stamp"
  # 配布物の検査（check-artifacts.sh）は、どの配布物にも許諾の 3 つのファイルを求める（ST53）
  cp "$root/LICENSE" "$work/LICENSE"
  cat >"$work/THIRD-PARTY-NOTICES.txt" <<'NOTICES'
== rust-std 1
License: MIT OR Apache-2.0

== anyhow 1.0.0
License: MIT
MIT License of anyhow

== react 18.0.0
License: MIT
MIT License of react
NOTICES
  printf 'Commit: %s\n' 0000000000000000000000000000000000000000 >"$work/SOURCE.txt"
  local files=(stamp LICENSE THIRD-PARTY-NOTICES.txt SOURCE.txt)
  tar -czf "$work/server-linux.tar.gz" -C "$work" "${files[@]}"
  (cd "$work" && zip -q server-windows.zip "${files[@]}" && zip -q collector-windows.zip "${files[@]}")
  cat >"$fake/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"${DRAFT_NIGHTLY_CALLS:?}"
if [[ "$1" == run && "$2" == download ]]; then
  out="${@: -1}"
  mkdir -p "$out"
  cp "$DRAFT_NIGHTLY_SOURCE/ashiato-collector-"*.apk "$out/"
  if [[ "$5" == nightly-linux ]]; then cp "$DRAFT_NIGHTLY_SOURCE/ashiato-server-$DRAFT_NIGHTLY_VERSION-linux-x86_64.tar.gz" "$out/"; else
    cp "$DRAFT_NIGHTLY_SOURCE/ashiato-server-$DRAFT_NIGHTLY_VERSION-windows-x86_64.zip" "$out/"
    cp "$DRAFT_NIGHTLY_SOURCE/ashiato-collector-windows-$DRAFT_NIGHTLY_VERSION-x86_64.zip" "$out/"
  fi
  exit 0
fi
if [[ "$1" == api ]]; then
  printf '%s\n' "${DRAFT_NIGHTLY_RUN:-o/ashiato	o/ashiato	success	nightly}"
  exit 0
fi
[[ "$1" == release && "$2" == create ]] || exit 20
grep -q -- '--draft' <<<"$*"
grep -q -- '--prerelease' <<<"$*"
grep -q -- '--notes-file' <<<"$*"
[[ "$*" != *"git"* ]]
EOF
  chmod +x "$fake/gh"
  # 偽の upload は、nightly-linux と nightly-windows の両方に同じ作業置き場を渡す。
  mkdir -p "$work/source"
  cp "$work/server-linux.tar.gz" "$work/source/ashiato-server-$version-linux-x86_64.tar.gz"
  cp "$work/server-windows.zip" "$work/source/ashiato-server-$version-windows-x86_64.zip"
  cp "$work/collector-windows.zip" "$work/source/ashiato-collector-windows-$version-x86_64.zip"
  # gh の偽台本は、APK と3つの配布物を run download の結果として再現する。
  echo "== Scenario: 開発版の確かめの組み立ては、開発版の鍵で署名した下書きになり、印を作らない"
  DRAFT_NIGHTLY_CALLS="$calls" DRAFT_NIGHTLY_SOURCE="$work/source" DRAFT_NIGHTLY_VERSION="$version" DRAFT_NIGHTLY_RUN=$'o/ashiato\to/ashiato\tsuccess\tnightly' PATH="$fake:$PATH" \
    ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
    ASHIATO_EXPECT_DEV_CERT_SHA256="$cert" "$0" --channel nightly 77 "$version" >/dev/null
  grep -Fq 'release create nightly-draft-77' "$calls" || die '下書きの要求が無い'
  grep -Fq -- '--notes-file' "$calls" || die '下書きの説明文指定が無い'
  ! grep -Eq 'git|release upload|release edit' "$calls" || die '印または公開の要求が出た'
  local invalid_run
  for invalid_run in $'fork/ashiato\to/ashiato\tsuccess\tnightly' $'o/ashiato\to/ashiato\tsuccess\tci' $'o/ashiato\to/ashiato\tfailure\tnightly'; do
    : >"$calls"
    if DRAFT_NIGHTLY_CALLS="$calls" DRAFT_NIGHTLY_SOURCE="$work/source" DRAFT_NIGHTLY_VERSION="$version" DRAFT_NIGHTLY_RUN="$invalid_run" PATH="$fake:$PATH" \
        ASHIATO_SIGNING_KEYSTORE_B64="$b64" ASHIATO_SIGNING_STORE_PASSWORD="$password" ASHIATO_SIGNING_KEY_ALIAS=ashiato-dev ASHIATO_SIGNING_KEY_PASSWORD="$password" \
        ASHIATO_EXPECT_DEV_CERT_SHA256="$cert" "$0" --channel nightly 77 "$version" >/dev/null 2>&1; then
      die "不正な nightly run を受け入れた: $invalid_run"
    fi
    ! grep -Fq 'release create' "$calls" || die "不正な nightly run で下書きを作った: $invalid_run"
  done
}

self_test() {
  local work fake_bin calls
  work="$(mktemp -d "${TMPDIR:-/tmp}/release-draft.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  fake_bin="$work/bin"
  calls="$work/gh-calls"
  mkdir -p "$fake_bin"
  export DRAFT_SELF_TEST_ROOT="$root"

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
  cp "$DRAFT_SELF_TEST_ROOT/LICENSE" "$out/LICENSE"
  cat >"$out/THIRD-PARTY-NOTICES.txt" <<'NOTICES'
== rust-std 1
License: MIT OR Apache-2.0

== anyhow 1.0.0
License: MIT
MIT License of anyhow

== react 18.0.0
License: MIT
MIT License of react

== com.google.android.gms:play-services-location 1.0.0
License: Apache-2.0
Apache License of play-services-location
NOTICES
  printf 'Commit: %s\n' 0000000000000000000000000000000000000000 >"$out/SOURCE.txt"
  tar -czf "$out/ashiato-server-$v-linux-x86_64.tar.gz" -C "$out" stamp LICENSE THIRD-PARTY-NOTICES.txt SOURCE.txt
  (cd "$out" && zip -q "ashiato-server-$v-windows-x86_64.zip" stamp LICENSE THIRD-PARTY-NOTICES.txt SOURCE.txt)
  (cd "$out" && zip -q "ashiato-collector-windows-$v-x86_64.zip" stamp LICENSE THIRD-PARTY-NOTICES.txt SOURCE.txt)
  if [[ "${DRAFT_SELF_TEST_MISSING_APK:-0}" != 1 ]]; then
    cp "$out/stamp" "$out/classes.dex"
    mkdir -p "$out/assets/licenses"
    cp "$out/LICENSE" "$out/assets/licenses/LICENSE"
    cp "$out/THIRD-PARTY-NOTICES.txt" "$out/assets/licenses/THIRD-PARTY-NOTICES.txt"
    # APK 側は gms の中の第三者の見出しも必須。
    printf '\n(gms の中の第三者)\n' >>"$out/assets/licenses/THIRD-PARTY-NOTICES.txt"
    cp "$out/SOURCE.txt" "$out/assets/licenses/SOURCE.txt"
    (cd "$out" && zip -q "ashiato-collector-$v.apk" classes.dex assets/licenses/LICENSE assets/licenses/THIRD-PARTY-NOTICES.txt assets/licenses/SOURCE.txt)
  fi
  rm -rf "$out/stamp" "$out/classes.dex" "$out/LICENSE" "$out/THIRD-PARTY-NOTICES.txt" "$out/SOURCE.txt" "$out/assets"
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
  nightly_self_test
}

if [[ "${1:-}" == --self-test ]]; then self_test; exit 0; fi
if [[ "${1:-}" == --channel && "${2:-}" == nightly ]]; then
  [[ $# == 4 ]] || usage
  draft_nightly "$3" "$4"
else
  [[ $# == 2 ]] || usage
  draft_release "$1" "$2"
fi
