#!/usr/bin/env bash
# Rust・Node の依存の許諾が AGPL-3.0 と両立するかを機械で確かめる。
set -euo pipefail
# 自己試験は自分を呼び直す。cd の後でも相対の $0 で壊れないよう、先に絶対パスを取る
self="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
cd "$(dirname "$0")/.."

SELF_LICENSE='AGPL-3.0-only WITH AdditionRef-ashiato-play-services'
ALLOW="${ALLOW_LICENSES:-MIT,MIT-0,Apache-2.0,BSD-2-Clause,BSD-3-Clause,ISC,Unicode-3.0,Unicode-DFS-2016,Zlib,CC0-1.0,MPL-2.0,0BSD,Apache-2.0 WITH LLVM-exception,CDLA-Permissive-2.0,BlueOak-1.0.0,CC-BY-4.0,Python-2.0}"

check_section() {
  local section=$1 input=$2
  python3 - "$ALLOW" "$SELF_LICENSE" "$section" "$input" <<'PY'
import json
import os
import re
import sys

allow = {item.strip() for item in sys.argv[1].split(",")}
self_license = sys.argv[2]
section = sys.argv[3]
input_path = sys.argv[4]

def allowed(expr):
    """SPDX の AND/OR 式を評価する（WITH は 1 つの原子として扱う）。"""
    tokens = re.findall(r"\(|\)|[A-Za-z0-9.\-+]+(?:\s+WITH\s+[A-Za-z0-9.\-]+)?|AND|OR", expr)
    position = 0
    def parse_or():
        nonlocal position
        value = parse_and()
        while position < len(tokens) and tokens[position] == "OR":
            position += 1
            value = parse_and() or value
        return value
    def parse_and():
        nonlocal position
        value = parse_atom()
        while position < len(tokens) and tokens[position] == "AND":
            position += 1
            value = parse_atom() and value
        return value
    def parse_atom():
        nonlocal position
        if position < len(tokens) and tokens[position] == "(":
            position += 1
            value = parse_or()
            if position < len(tokens) and tokens[position] == ")":
                position += 1
            return value
        token = tokens[position] if position < len(tokens) else ""
        position += 1
        return token in allow
    return bool(tokens) and parse_or()

def check_package(license_value, own):
    license_value = str(license_value or "").strip()
    if own and license_value == self_license:
        return True
    return bool(license_value) and allowed(license_value.replace("/", " OR "))

bad = []
count = 0
if section == "rust":
    with open(input_path, encoding="utf-8") as stream:
        metadata = json.load(stream)
    members = set(metadata.get("workspace_members", []))
    for package in metadata.get("packages", []):
        count += 1
        if not check_package(package.get("license"), package.get("id") in members):
            bad.append((package.get("name", ""), package.get("license") or "(許諾の記載が無い)"))
elif section == "node":
    root = input_path
    parent_package = os.path.join(os.path.dirname(root), "package.json")
    packages = []
    root_name = None
    if os.path.isfile(parent_package):
        with open(parent_package, encoding="utf-8") as stream:
            packages.append(json.load(stream))
        # 自分の部品の特例は、親の package.json の名前にだけ及ぼす（無ければ誰にも及ぼさない）
        root_name = packages[0].get("name")
    if not os.path.isdir(root):
        print(f"  NG node_modules が無い: {root}（この検査は空振りしている）")
        sys.exit(1)
    for directory, _, filenames in os.walk(root):
        if "package.json" not in filenames:
            continue
        try:
            with open(os.path.join(directory, "package.json"), encoding="utf-8") as stream:
                packages.append(json.load(stream))
        except (OSError, ValueError):
            continue
    for package in packages:
        if "name" not in package or "version" not in package:
            continue
        count += 1
        license_value = package.get("license", "")
        if isinstance(license_value, dict):
            license_value = license_value.get("type", "")
        if not check_package(license_value, package["name"] == root_name):
            bad.append((package["name"], license_value or "(許諾の記載が無い)"))
elif section == "android":
    with open(os.environ.get("ASHIATO_LICENSES_POM_NAMES", "tools/licenses/pom-license-names.tsv"), encoding="utf-8") as stream:
        names = dict(line.rstrip("\n").split("\t", 1) for line in stream if line.strip())
    addition = os.environ.get("ASHIATO_LICENSES_ADDITION", "LICENSES/AdditionRef-ashiato-play-services.txt")
    listed = set()
    if os.path.isfile(addition):
        in_list = False
        with open(addition, encoding="utf-8") as stream:
            for line in stream:
                line = line.rstrip("\n")
                if line == "Listed libraries:":
                    in_list = True
                elif in_list and line.startswith("- "):
                    listed.add(line[2:])
                elif in_list and not line:
                    in_list = False
    components = {}
    with open(input_path, encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, 1):
            fields = line.rstrip("\n").split("\t")
            if len(fields) != 4:
                print(f"  NG TSV の列数が 4 でない: {input_path}:{line_number}")
                sys.exit(1)
            coordinate, name, url, source = fields
            components.setdefault(coordinate, []).append((name, url, source))
    count = len(components)
    if not count:
        print("  NG 対象が無い（Android の依存の POM が空）")
        sys.exit(1)
    proprietary = []
    for coordinate, licenses in sorted(components.items()):
        package = ":".join(coordinate.split(":")[:2])
        translated = []
        for name, url, source in licenses:
            if name == "-":
                bad.append((coordinate, "(許諾の記載が無い)"))
                continue
            if name not in names:
                bad.append((coordinate, name))
                continue
            spdx = names[name]
            translated.append(spdx)
            if spdx == "PROPRIETARY":
                if package not in listed:
                    bad.append((coordinate, name))
                else:
                    proprietary.append(coordinate)
            elif spdx not in allow:
                bad.append((coordinate, name))
    for coordinate, name in bad:
        print(f"  NG {coordinate}: {name}")
    for coordinate in sorted(set(proprietary)):
        print(f"  通した PROPRIETARY: {coordinate}")
    print(f"  {count} 件を確認 / 不許可 {len(bad)} 件")
    sys.exit(1 if bad else 0)
else:
    raise SystemExit(f"未知の節: {section}")

for name, license_value in bad:
    print(f"  NG {name}: {license_value}")
print(f"  {count} 件を確認 / 不許可 {len(bad)} 件")
sys.exit(1 if bad else 0)
PY
}

run_rust() {
  local metadata=$1
  if [[ -z $metadata ]]; then
    metadata=$(mktemp "$work_tmp/meta.XXXXXX.json")
    cargo metadata --format-version 1 --all-features >"$metadata"
  fi
  echo "== Rust の依存"
  check_section rust "$metadata"
}

run_node() {
  echo "== Node の依存"
  check_section node "${ASHIATO_LICENSES_NODE_ROOT:-web/node_modules}"
}

run_android() {
  echo "== Android の依存"
  echo "== Scenario: 今の携帯の依存は、許諾の検査を通る"
  echo "== Scenario: 今の携帯の依存で、オープンソースでないとして通すのは名指しの 4 つだけ"
  local list="${ASHIATO_LICENSES_POM_LIST:-}"
  if [[ -z "$list" ]]; then
    local gradlew="${ASHIATO_GRADLEW:-./collector-android/gradlew}"
    if [[ "$gradlew" == "false" ]]; then
      echo "  NG 対象が無い（gradle が依存を解決できない）"
      return 1
    fi
    list="$work_tmp/android-poms.tsv"
    local -a command=("$gradlew" -p collector-android -q :app:ashiatoDependencyPoms "-Pashiato.pomOut=$list")
    if [[ -n "${ASHIATO_LICENSES_EXTRA_REPO:-}" ]]; then
      command+=("-Pashiato.extraRepo=$ASHIATO_LICENSES_EXTRA_REPO")
    fi
    local gradle_output
    if ! gradle_output=$("${command[@]}" 2>&1); then
      echo "$gradle_output"
      grep -En 'POM|Could not find' <<<"$gradle_output" || true
      echo "  NG 対象が無い（gradle の依存または POM を解決できない）"
      return 1
    fi
  fi
  [[ -s "$list" ]] || { echo "  NG 対象が無い（Android の依存の POM が空）"; return 1; }
  check_section android "$list"
}

self_test() {
  local test_root cargo_meta node_root
  test_root=$(mktemp -d)
  trap 'rm -rf "$test_root"' RETURN
  cargo_meta="$test_root/meta.json"
  node_root="$test_root/web/node_modules"
  mkdir -p "$node_root/allowed" "$test_root/web"
  cat >"$test_root/web/package.json" <<'JSON'
{"name":"ashiato-web","version":"0.0.0","license":"AGPL-3.0-only WITH AdditionRef-ashiato-play-services"}
JSON
  cat >"$node_root/allowed/package.json" <<'JSON'
{"name":"allowed-package","version":"1.0.0","license":"MIT"}
JSON
  echo "== Scenario: 他人の部品の AGPL の許諾は、許諾の検査が落とす"
  cat >"$cargo_meta" <<'JSON'
{"packages":[{"id":"path+fake#evil-agpl","name":"evil-agpl","license":"AGPL-3.0-only"}],"workspace_members":[]}
JSON
  local output
  if output=$(ASHIATO_LICENSES_CARGO_META="$cargo_meta" "$self" rust 2>&1); then
    echo "  self-test NG: 他人の AGPL を通した" >&2; return 1
  fi
  grep -q "evil-agpl" <<<"$output" || return 1
  echo "== Scenario: 自分の部品は、決めた許諾の式で許諾の検査を通る"
  cat >"$cargo_meta" <<'JSON'
{"packages":[{"id":"path+fake#ashiato-server","name":"ashiato-server","license":"AGPL-3.0-only WITH AdditionRef-ashiato-play-services"}],"workspace_members":["path+fake#ashiato-server"]}
JSON
  ASHIATO_LICENSES_CARGO_META="$cargo_meta" ASHIATO_LICENSES_NODE_ROOT="$node_root" "$self" rust node >/dev/null
  echo "  self-test: 自分の部品の名前で、決めた式でない許諾を持つものが落ちる"
  cat >"$cargo_meta" <<'JSON'
{"packages":[{"id":"path+fake#ashiato-server","name":"ashiato-server","license":"AGPL-3.0-only"}],"workspace_members":["path+fake#ashiato-server"]}
JSON
  if output=$(ASHIATO_LICENSES_CARGO_META="$cargo_meta" "$self" rust 2>&1); then
    echo "  self-test NG: 自分の部品の不正な式を通した" >&2; return 1
  fi
  grep -q "ashiato-server" <<<"$output" || return 1
  echo "== Scenario: 許可の一覧は、AGPL-3.0-only を外したほかは前と同じ"
  [[ $ALLOW == "MIT,MIT-0,Apache-2.0,BSD-2-Clause,BSD-3-Clause,ISC,Unicode-3.0,Unicode-DFS-2016,Zlib,CC0-1.0,MPL-2.0,0BSD,Apache-2.0 WITH LLVM-exception,CDLA-Permissive-2.0,BlueOak-1.0.0,CC-BY-4.0,Python-2.0" ]] || return 1
  [[ $ALLOW != *AGPL-3.0-only* ]] || return 1
  echo "== Scenario: node_modules が無ければ、画面の節は通さずに落ちる"
  if output=$(ASHIATO_LICENSES_NODE_ROOT="$test_root/missing-node-modules" "$self" node 2>&1); then
    echo "  self-test NG: 無い node_modules を通した" >&2; return 1
  fi
  grep -q "node_modules が無い" <<<"$output" || return 1

  local android_list android_addition
  android_list="$test_root/android.tsv"
  android_addition="$test_root/Addition.txt"
  cat >"$android_addition" <<'TXT'
Listed libraries:
- com.google.android.gms:play-services-tasks
TXT
  cat >"$android_list" <<'TSV'
com.google.android.gms:play-services-tasks:1.0.0	Android Software Development Kit License	https://example.invalid	self
com.example.allowed:library:1.0.0	The Apache Software License, Version 2.0	https://example.invalid	self
TSV
  echo "== Scenario: 名指しの外の Google のライブラリを足すと、許諾の検査が落ちる"
  printf '%s\n' 'com.google.android.gms:play-services-ads:1.0.0	Android Software Development Kit License	https://example.invalid	self' >>"$android_list"
  if output=$(ASHIATO_LICENSES_POM_LIST="$android_list" ASHIATO_LICENSES_ADDITION="$android_addition" "$self" android 2>&1); then
    echo "  self-test NG: 名指しの外の Google の部品を通した" >&2; return 1
  fi
  grep -q 'com.google.android.gms:play-services-ads' <<<"$output" || return 1
  grep -q 'Android Software Development Kit License' <<<"$output" || return 1
  sed -i '/play-services-ads/d' "$android_list"

  echo "== Scenario: 対応表に無い許諾の名前の部品があれば、許諾の検査が落ちる"
  printf '%s\n' 'com.example.unknown:library:1.0.0	Unknown License Name	https://example.invalid	self' >>"$android_list"
  if output=$(ASHIATO_LICENSES_POM_LIST="$android_list" ASHIATO_LICENSES_ADDITION="$android_addition" "$self" android 2>&1); then
    echo "  self-test NG: 対応表に無い許諾を通した" >&2; return 1
  fi
  grep -q 'com.example.unknown:library:1.0.0' <<<"$output" || return 1
  grep -q 'Unknown License Name' <<<"$output" || return 1
  sed -i '/com.example.unknown/d' "$android_list"

  echo "== Scenario: 許諾の名前を 2 つ持つ部品は、片方でも許可の外なら許諾の検査が落ちる"
  printf '%s\n%s\n' 'com.example.dual:library:1.0.0	The Apache Software License, Version 2.0	https://example.invalid	self' 'com.example.dual:library:1.0.0	Android Software Development Kit License	https://example.invalid	self' >>"$android_list"
  if output=$(ASHIATO_LICENSES_POM_LIST="$android_list" ASHIATO_LICENSES_ADDITION="$android_addition" "$self" android 2>&1); then
    echo "  self-test NG: 不許可な二重許諾を通した" >&2; return 1
  fi
  grep -q 'com.example.dual:library:1.0.0' <<<"$output" || return 1
  grep -q 'Android Software Development Kit License' <<<"$output" || return 1
  sed -i '/com.example.dual/d' "$android_list"

  echo "== Scenario: 許諾の名前を 2 つ持つ部品は、両方が許可されていれば通る"
  printf '%s\n%s\n' 'com.example.dual:library:1.0.0	The Apache Software License, Version 2.0	https://example.invalid	self' 'com.example.dual:library:1.0.0	The MIT License	https://example.invalid	self' >>"$android_list"
  ASHIATO_LICENSES_POM_LIST="$android_list" ASHIATO_LICENSES_ADDITION="$android_addition" "$self" android >/dev/null
  sed -i '/com.example.dual/d' "$android_list"

  echo "== Scenario: 名指しの一覧を正本から消すと、その部品で許諾の検査が落ちる"
  : >"$android_addition"
  if output=$(ASHIATO_LICENSES_POM_LIST="$android_list" ASHIATO_LICENSES_ADDITION="$android_addition" "$self" android 2>&1); then
    echo "  self-test NG: 名指しを消した部品を通した" >&2; return 1
  fi
  grep -q 'com.google.android.gms:play-services-tasks' <<<"$output" || return 1
  cp "LICENSES/AdditionRef-ashiato-play-services.txt" "$android_addition"

  echo "== Scenario: POM が許諾を持たない部品は、親の POM の許諾で判定する"
  local fake_repo="$test_root/maven-repo" copied_android copied_cargo parent_list parent_output
  copied_android="$test_root/copied-collector-android"
  copied_cargo="$test_root/Cargo.toml"
  cp -a collector-android "$copied_android"
  cp Cargo.toml "$copied_cargo"
  # この自己試験は偽の依存を後から足すため、固定の検査は check-gradle-locks.sh に任せる。
  sed -i '/^dependencyLocking {$/,/^}$/d' "$copied_android/build.gradle.kts" "$copied_android/app/build.gradle.kts"
  mkdir -p "$fake_repo/com/example/parent/1.0.0" "$fake_repo/com/example/child/1.0.0"
  : >"$fake_repo/com/example/child/1.0.0/child-1.0.0.jar"
  cat >"$fake_repo/com/example/parent/1.0.0/parent-1.0.0.pom" <<'POM'
<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion><groupId>com.example</groupId><artifactId>parent</artifactId><version>1.0.0</version><licenses><license><name>The Apache Software License, Version 2.0</name><url>https://www.apache.org/licenses/LICENSE-2.0.txt</url></license></licenses></project>
POM
  cat >"$fake_repo/com/example/child/1.0.0/child-1.0.0.pom" <<'POM'
<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion><parent><groupId>com.example</groupId><artifactId>parent</artifactId><version>1.0.0</version></parent><groupId>com.example</groupId><artifactId>child</artifactId><version>1.0.0</version></project>
POM
  cat >"$fake_repo/com/example/child/1.0.0/child-1.0.0.module" <<'MODULE'
{"formatVersion":"1.1","component":{"group":"com.example","module":"child","version":"1.0.0"},"createdBy":{"gradle":{"version":"8.13"}},"variants":[{"name":"runtime","attributes":{"org.gradle.category":"library","org.gradle.dependency.bundling":"external","org.gradle.libraryelements":"jar","org.gradle.usage":"java-runtime"},"files":[{"name":"child-1.0.0.jar","url":"child-1.0.0.jar"}]}]}
MODULE
  sed -i 's#url = uri(extraRepo)#url = uri(extraRepo); metadataSources { gradleMetadata(); mavenPom(); artifact() }#' "$copied_android/settings.gradle.kts"
  sed -i '/implementation("androidx.core:core-ktx:1.17.0")/a\    implementation("com.example:child:1.0.0")' "$copied_android/app/build.gradle.kts"
  parent_list="$test_root/parent.tsv"
  parent_output=$("$copied_android/gradlew" -p "$copied_android" --no-configuration-cache --rerun-tasks -q :app:ashiatoDependencyPoms \
    "-Pashiato.pomOut=$parent_list" "-Pashiato.extraRepo=$fake_repo" 2>&1) || {
    echo "$parent_output" >&2
    return 1
  }
  grep -q $'^com.example:child:1.0.0\tThe Apache Software License, Version 2.0\t.*\tparent$' "$parent_list" || return 1
  ASHIATO_LICENSES_POM_LIST="$parent_list" ASHIATO_LICENSES_ADDITION="$android_addition" "$self" android >/dev/null

  echo "== Scenario: POM を取り寄せられない部品があれば、許諾の検査が落ちる"
  local missing_version="9.9.$RANDOM"
  local missing_coordinate="com.example:missing:$missing_version"
  mkdir -p "$fake_repo/com/example/missing/$missing_version"
  : >"$fake_repo/com/example/missing/$missing_version/missing-$missing_version.jar"
  : >"$fake_repo/com/example/missing/$missing_version/missing-$missing_version.aar"
  cat >"$fake_repo/com/example/missing/maven-metadata.xml" <<META
<metadata><groupId>com.example</groupId><artifactId>missing</artifactId><versioning><latest>$missing_version</latest><release>$missing_version</release><versions><version>$missing_version</version></versions></versioning></metadata>
META
  cat >"$fake_repo/com/example/missing/$missing_version/missing-$missing_version.module" <<MODULE
{"formatVersion":"1.1","component":{"group":"com.example","module":"missing","version":"$missing_version","attributes":{"org.gradle.status":"release"}},"createdBy":{"gradle":{"version":"8.13"}},"variants":[{"name":"runtime","attributes":{"org.gradle.category":"library","org.gradle.dependency.bundling":"external","org.gradle.libraryelements":"jar","org.gradle.usage":"java-runtime"},"files":[{"name":"missing-$missing_version.jar","url":"missing-$missing_version.jar"}]},{"name":"android-runtime","attributes":{"org.gradle.category":"library","org.gradle.dependency.bundling":"external","org.gradle.libraryelements":"aar","org.gradle.usage":"java-runtime"},"files":[{"name":"missing-$missing_version.aar","url":"missing-$missing_version.aar"}]}]}
MODULE
  sed -i "/implementation(\"com.example:child:1.0.0\")/a\\    releaseImplementation(\"$missing_coordinate\")" "$copied_android/app/build.gradle.kts"
  grep -q "$missing_coordinate" "$copied_android/app/build.gradle.kts" || return 1
  printf '%s\n' 'tasks.register("resolveFakeReleaseRuntime") { doLast { configurations.getByName("releaseRuntimeClasspath").resolve() } }' >>"$copied_android/app/build.gradle.kts"
  local resolve_output
  resolve_output=$("$copied_android/gradlew" -p "$copied_android" --no-configuration-cache --refresh-dependencies -q :app:resolveFakeReleaseRuntime \
    --write-locks \
    "-Pashiato.extraRepo=$fake_repo" 2>&1) || { printf '%s\n' "$resolve_output" >&2; return 1; }
  if output=$("$copied_android/gradlew" -p "$copied_android" --offline --no-configuration-cache --rerun-tasks -q :app:ashiatoDependencyPoms \
    "-Pashiato.pomOut=$test_root/missing.tsv" "-Pashiato.extraRepo=$fake_repo" 2>&1); then
    echo "  self-test NG: POM の取得失敗を通した" >&2
    return 1
  fi
  grep -q "$missing_coordinate" <<<"$output" || return 1
  grep -Eq "POM を取れない|Could not find missing-${missing_version}\.pom" <<<"$output" || return 1
  ! grep -q "Could not find $missing_coordinate" <<<"$output" || return 1

  echo "== Scenario: 許諾の検査の自己試験は、固定を入れた後も通る"

  echo "== Scenario: 検査する対象が無い節は、通さずに落ちる"
  if output=$(ASHIATO_GRADLEW=false ASHIATO_LICENSES_POM_LIST= "$self" android 2>&1); then
    echo "  self-test NG: 対象の無い Android 節を通した" >&2; return 1
  fi
  grep -q '対象が無い' <<<"$output" || return 1
  echo "self-test OK"
}

if [[ ${1:-} == --self-test ]]; then
  self_test
  exit 0
fi

work_tmp=${TMPDIR:-}
if [[ -z $work_tmp ]]; then
  work_tmp=$(mktemp -d)
else
  work_tmp=$(mktemp -d "$work_tmp/check-licenses.XXXXXX")
fi
trap 'rm -rf "$work_tmp"' EXIT

sections=()
if (($# == 0)); then sections=(rust node android); else sections=("$@"); fi
fail=0
for section in "${sections[@]}"; do
  case $section in
    rust) run_rust "${ASHIATO_LICENSES_CARGO_META:-}" || fail=1 ;;
    node) run_node || fail=1 ;;
    android) run_android || fail=1 ;;
    *) echo "使い方: $0 [rust|node|android]… | --self-test" >&2; exit 2 ;;
  esac
done
if ((fail == 0)); then echo "ライセンス OK"; else echo "ライセンス NG" >&2; exit 1; fi
