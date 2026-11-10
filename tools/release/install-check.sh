#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <配布 zip> | --self-test | --list | --probe-console | --check-coverage <change>" >&2; exit 2; }

scenario_names=(
  '配る zip からのインストールで、2 つのサービスが自動で起きる形で入る'
  'インストールは Docker も WSL も使わない'
  'サービスは利用者でなくサービス専用のアカウントで動く'
  'DB のサービスが異常に終わると起こし直される'
  '設定のファイルが無いと、理由を書いて起動しない'
  'DB の起動が遅れても、サーバのサービスは待ってから起きる'
  'DB が 120 秒開かなければ、理由を書いて終わる'
  'サービス専用のアカウントで登録できなければ、共有のアカウントへ倒さず止まる'
  'サービスを止めて起こし直すと、サーバは DB の後に起きて答える'
  'サーバのサービスが異常に終わると起こし直される'
  'サービスの登録簿に秘密が無い'
  '状態の置き場はほかのアカウントから読めない'
  '表の所有者の合言葉は、サーバのサービスが読めないファイルにだけある'
  '本人は状態の置き場を読めるが書けない'
  '別の利用者は状態の置き場を読めない'
  'DB の置き場にはサーバのアカウントが書けない'
  '上がった方が別の管理者でも、本人は上がる前に控えた利用者になる'
  'インストーラは状態の置き場を選ぶ口を持たない'
  'インストールが終わると、インストールの間の書きの権限が残らない'
  '配る DB は OS に依らない並べ方で作られ、符号の順に並ぶ'
  '配る DB は合言葉なしの接続を通さない'
  '配る DB は手元の 55400 番だけで待ち受ける'
  '配る DB はログに文と値を書かない'
  'DB のサービスは logs の db に書け、DB のログのファイルができる'
  '送った文と秘密はイベントログに出ない'
  'ログのファイルはほかのアカウントから読めない'
  'ログは DB が 7 日分、サーバが 30 日分残る'
  'DB を作るときの一時の合言葉のファイルが残らない'
  '配る DB と開発の DB は同じメジャー版'
  'インストールごとに秘密が違う'
  'インストーラの出力とログに秘密が出ない'
  '作った秘密でサーバの起動の検査が通る'
  '配る形に移行を当ててサーバが起きる'
  '実行時の役割は管理者でも表の所有者でもない'
  '移行の前の退避は状態の置き場に、同梱の pg_dump で取られる'
  '移行が失敗すると、サーバのサービスを開始せずに止まる'
  'サービスの起動では移行を当てない'
  'インストールの間、ネットワークに出ない'
  '走らせ直しても秘密と DB は変わらない'
  'DB を作る途中で落ちても、走らせ直しで DB ができる'
  'DB はあるのに秘密が無いと、何も書き換えずに止まる'
  '合言葉を決める前に閉じたら、走らせ直しで合言葉の段からやり直す'
  '別の利用者が走らせ直しても、本人は変わらない'
  '設定を退けて止まった後に repair すると、記録を残したままログインできる'
  'repair の後は前の API の合言葉では送れず、繋ぎ直しの案内が出る'
  '止まった状態でなければ repair は何も変えずに終わる'
  'repair の後に合言葉なしの入口と一時の番号が残らない'
  '別の管理者が repair しても、本人は変わらない'
  '英字でない名前の管理者でも repair でき、決めた合言葉でログインできる'
  'repair の途中で落ちても、もう一度 repair できる'
  '決めた合言葉で、そのまま画面にログインできる'
  '短い・既定の雛形・一致しない合言葉は受け付けない'
  '端末でないときも、断りより前に合言葉の条件を 1 回だけ示す'
  '短い・雛形で断るときは、理由に条件を添える'
  'コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める'
  '窓で打った合言葉で決め直せる'
  '窓で打った合言葉は、窓に出ない'
  '標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る'
  'API の合言葉やログインの印の鍵と同じ値は受け付けない'
  '合言葉を決め終えると、既定のブラウザで画面を開く'
  '合言葉の元の値は、どこにも残らない'
  'インストールした形の設定は、画面の合言葉の元の値を持たない'
  '決め直した合言葉でログインでき、前の合言葉ではできない'
  '決め直すと、それまでのログインは切れる'
  '決め直しても、ほかの秘密と記録は変わらない'
  '管理者に上がらなければ決め直せない'
  'ダウンロードに置いた Takeout が、インストールした形で取り込まれる'
  '取り込み待ちは本人のプロファイルの直下にでき、サーバが読み書きできる'
  '取り込み待ちを消しても、サーバが作り直す'
  'ダウンロードにはサーバのアカウントの読みだけが足される'
  'インストールした形の書き出しは、本人が読めて、ほかのアカウントは読めない'
  '番号が塞がっていると、登録せずに止まる'
  '消した後に入れ直すと、前の記録の上で続く'
  '消すとダウンロードと取り込み待ちに足した権限が外れる'
  '起動を拒んだ理由が、サーバのログのファイルに残る'
  'インストールすると「アプリと機能」に載る'
  '消すとサービスとプログラムと登録が消える'
  '消しても記録の置き場は残り、その場所を示す'
  '最後に Tailscale の転送先を示し、Tailscale の設定は変えない'
  '空きが見積もりより少なければ知らせ、インストールは続ける'
)

# Windows の通しで見ない Scenario（Linux の試験・配る組み立ての検査が担保する）。免除はこの名前の一覧だけ
# （`.rs` に印があるかで免除すると、両方にある Scenario を --list から消しても通った。review R73）
covered_elsewhere=(
  'Linux の配布物は PostgreSQL を持たない'
  'Windows の配布物にインストーラと PostgreSQL が入り、管理画面は入らない'
  'インストーラとサーバの版番号はそろう'
  'インストールした形では、指定が無ければ状態の置き場の export に書く'
  'インストールした形では、設定が無ければ状態の置き場の media に置く'
  'インストールした形でも、照らすための値と合う合言葉でだけログインできる'
  'インストールした形で照らすための値が無いと起動しない'
  'インストールした形の設定に画面の合言葉の元の値があると起動しない'
  'サービスの停止を受けると、書き途中の書き出しを残さずに終わる'
  '上がっていない状態から開くと、管理者に上がる起こし直しは 1 回だけ'
  '上がる前の段は、開いた利用者を本人として控える'
  '同梱する PostgreSQL の指紋が違えば組まない'
  '配る番号は開発の台本の割り当てと被らない'
  '配る組み立ては試験の差し込みを持たない'
  '開発の台本で起こすサーバは、今までどおり環境変数を読む'
)

list_scenarios() {
  printf '%s\n' "${scenario_names[@]}"
}

check_coverage() {
  local change="$1"
  [[ -d "$change/specs" ]] || die "spec の置き場が無い: $change/specs"
  python3 - "$change" "$0" "${covered_elsewhere[@]}" <<'PY'
import re
import subprocess
import sys
from pathlib import Path

change = Path(sys.argv[1])
script = Path(sys.argv[2])
listed = set(subprocess.check_output([str(script), "--list"], text=True).splitlines())
scenarios = []
for path in change.glob("specs/**/spec.md"):
    scenarios.extend(re.findall(r"^#### Scenario: (.+)$", path.read_text(encoding="utf-8"), re.M))
canonical = set()
for path in (change.parent.parent / "specs").glob("**/spec.md"):
    canonical.update(re.findall(r"^#### Scenario: (.+)$", path.read_text(encoding="utf-8"), re.M))
scenarios = [name for name in scenarios if name not in canonical]

elsewhere = set(sys.argv[3:])
tracked = subprocess.check_output(["git", "ls-files"], text=True).splitlines()
marker_paths = {
    Path("tools/release/pgsql.sh"),
    Path("tools/release/check-artifacts.sh"),
    Path("tools/check-ports.sh"),
    Path("tools/check-pg-major.sh"),
}
marker_files = [Path(p) for p in tracked if p.endswith(".rs") or Path(p) in marker_paths]
marked = set()
for path in marker_files:
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            comment = re.match(r"^\s*//+\s*Scenario:\s*(.+?)\s*$", line)
            shell = re.match(r"^\s*echo\s+['\"]== Scenario:\s*(.+?)['\"]\s*$", line)
            if comment:
                marked.add(comment.group(1))
            elif shell:
                marked.add(shell.group(1))
    except (OSError, UnicodeDecodeError):
        pass
failed = False
for name in scenarios:
    if name not in listed and name not in elsewhere:
        print(f"NG: --list に無い Scenario: {name}", file=sys.stderr)
        failed = True
for name in sorted(elsewhere):
    if name not in scenarios:
        print(f"NG: covered_elsewhere の名前が spec に無い: {name}", file=sys.stderr)
        failed = True
    elif name not in marked:
        print(f"NG: covered_elsewhere の Scenario に試験の印が無い: {name}", file=sys.stderr)
        failed = True
    if name in listed:
        print(f"NG: covered_elsewhere と --list の両方にある: {name}", file=sys.stderr)
        failed = True
if failed:
    raise SystemExit(1)
print(f"OK: {len(scenarios)} Scenario の coverage")
PY
}

# ============================================================================
# 確かめの部品。Windows の命令の出力を受け取って判定するだけの関数（self-test が合う出力と
# 1 つずつ外れた出力で試す）。実地の確かめ（下の check_release_zip / check_test_zip）はこれを使う。
# ============================================================================

# icacls の出力（$1）を「主体<TAB>権限」の行にする。$2 は icacls に渡した場所（1 行目の頭から外す）。
acl_lines() {
  local output="$1" path="$2"
  ACL_PATH="$path" awk '{
    path = ENVIRON["ACL_PATH"]
    line = $0; sub(/\r$/, "", line)
    if (index(line, path) == 1) line = substr(line, length(path) + 1)
    sub(/^[ \t]+/, "", line)
    i = index(line, ":(")
    if (i == 0) next
    print substr(line, 1, i - 1) "\t" substr(line, i + 1)
  }' <<<"$output"
}

# 主体（大文字と小文字を区別しない）の権限を出す。無ければ空。
# サービスの仮想アカウントの SID（`sc.exe showsid` と同じ計算。インストーラの commands::service_sid）。
service_sid() {
  PY -c 'import hashlib,struct,sys; d=hashlib.sha1(sys.argv[1].upper().encode("utf-16-le")).digest(); print("S-1-5-80-"+"-".join(str(x) for x in struct.unpack("<5I", d)))' "$1"
}

acl_perm() {
  local lines="$1" principal="$2"
  ACL_PRINCIPAL="$principal" awk -F '\t' 'tolower($1) == tolower(ENVIRON["ACL_PRINCIPAL"]) { print $2 }' <<<"$lines"
}

# ACL の主体が、与えた主体だけであることを見る（受け継いだ ACE も主体として数える）。
assert_acl_only() {
  local lines="$1" what="$2"; shift 2
  [[ -n "$lines" ]] || die "$what の ACL を読めない"
  local principal allowed ok
  while IFS=$'\t' read -r principal _; do
    ok=0
    for allowed in "$@"; do
      [[ "${principal,,}" == "${allowed,,}" ]] && ok=1
    done
    [[ "$ok" == 1 ]] || die "$what の ACL に余計な主体がある: $principal"
  done <<<"$lines"
}

# 受け継いだ ACE（(I)）が無い＝継承を切っている。
assert_acl_not_inherited() {
  local lines="$1" what="$2"
  ! grep -Fq '(I)' <<<"$lines" || die "$what が上の置き場の ACL を受け継いでいる"
}

# 権限が読みだけ（RX）で、書き（M・F・W）を持たない。
assert_read_only_perm() {
  local perm="$1" what="$2"
  [[ -n "$perm" ]] || die "$what の項目が無い"
  grep -Fq '(RX)' <<<"$perm" || die "$what が読みでない: $perm"
  ! grep -Eq '\((M|F|W|D)\)|\(M,|,W|WD|AD' <<<"$perm" || die "$what が書きを持つ: $perm"
}

assert_modify_perm() {
  local perm="$1" what="$2"
  grep -Eq '\((M|F)\)' <<<"$perm" || die "$what が読み書きを持たない: ${perm:-（項目が無い）}"
}

# sc.exe qc の出力: 自動で起き、サービスの仮想アカウントで動く。
assert_service_config() {
  local qc="$1" service="$2"
  grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*2' <<<"$qc" || die "$service が自動で起きる形でない"
  grep -Eiq "SERVICE_START_NAME[[:space:]]*:[[:space:]]*NT SERVICE\\\\$service[[:space:]]*\$" <<<"${qc//$'\r'/}" ||
    die "$service がサービス専用のアカウントで動かない"
}

# sc.exe qfailure の出力: 異常に終わると起こし直す。
assert_restart_on_failure() {
  local qfailure="$1" service="$2"
  grep -Eiq 'RESTART' <<<"$qfailure" || die "$service の回復の設定に起こし直しが無い"
}

assert_db_watch_acl() {
  local sddl="$1" server_sid="$2"
  local dacl="${sddl%%S:*}"
  grep -Fq "(A;;LCRP;;;$server_sid)" <<<"$dacl" || die 'DB のサービスにサーバの状態読み・開始の権限が無い'
  ! grep -Eq "\(A;;[^;]*(WP|DC)[^;]*;;;${server_sid//\/\\}\)" <<<"$dacl" ||
    die 'DB のサービスにサーバの停止・設定変更権限がある'
}

# sc.exe query の出力の状態の番号（4 = RUNNING・1 = STOPPED）。
service_state_of() {
  awk -F ':' 'toupper($1) ~ /^[ \t]*STATE[ \t]*$/ { split($2, f, " "); print f[1]; exit }' <<<"${1//$'\r'/}"
}

# netstat -ano の出力: 番号が 127.0.0.1 だけで待ち受けている。
assert_loopback_listener_only() {
  local netstat="$1" port="$2"
  grep -Eq "^[[:space:]]*TCP[[:space:]]+127\\.0\\.0\\.1:$port[[:space:]].*:0[[:space:]]" <<<"$netstat" ||
    die "127.0.0.1:$port で待ち受けていない"
  ! grep -Eq "^[[:space:]]*TCP[[:space:]]+(0\\.0\\.0\\.0|\\[::\\]):$port[[:space:]]" <<<"$netstat" ||
    die "$port が手元の外のアドレスで待ち受けている"
}

# pg_hba.conf の有効な行に trust が無い。
assert_no_trust() {
  ! grep -Ev '^[[:space:]]*(#|$)' <<<"$1" | grep -Ewq 'trust' || die 'pg_hba.conf の有効な行に trust がある'
}

# 文の中に秘密（$2 以降）の値が 1 つも無い。値は出さず、項目の番号だけを示す。
assert_no_secret_in() {
  local text="$1" what="$2"; shift 2
  local value index=0
  for value in "$@"; do
    index=$((index + 1))
    [[ -n "$value" ]] || continue
    ! grep -Fq -- "$value" <<<"$text" || die "$what に秘密がある（$index 番目の値）"
  done
}

# 合言葉の条件の一覧が、出力に 1 回だけ、最初の断りより前にあり、「表示されません」（端末の言い方）を持たない。
# 一覧とは 4 つの条件（16 文字以上・change-me で始めない・API の合言葉・ログインの印の鍵）を全部含む行
assert_requirements_line() {
  local file="$1" what="$2" text hits first_refusal
  text="$(tr -d '\r' <"$file")"
  hits="$(grep -nF -e '16 文字以上' <<<"$text" | grep -F 'change-me で始めない' | grep -F 'API の合言葉' |
    grep -F 'ログインの印の鍵' || true)"
  [[ -n "$hits" && "$(wc -l <<<"$hits")" == 1 ]] || die "$what の出力に合言葉の条件の行がちょうど 1 つでない"
  first_refusal="$(grep -nF '受け付けない:' <<<"$text" | head -n 1 | cut -d: -f1 || true)"
  [[ -z "$first_refusal" || "${hits%%:*}" -lt "$first_refusal" ]] || die "$what の出力で、合言葉の条件が最初の断りより後にある"
  ! grep -Fq '表示されません' <<<"$text" || die "$what の出力（端末でない）に「表示されません」がある"
}

# 疑似コンソールの窓で命令を走らせ、窓の文字（VT を除く）を $WORK/<名前>.window に残す。戻り値は窓の子の終了コード。
# 窓の台本は上限を超えると子を止めて rc≠0 にする（install_with と同じ上限。password の口は起こし直しで最大 180 秒待つ）
window_run() {
  local name="$1" command="$2" input="${3:-}" rc=0 args=()
  [[ -z "$input" ]] || args=(-InputFile "$(cygpath -w "$input")")
  powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$(cygpath -w "$root/tools/release/conpty-run.ps1")" \
    -Command "$command" ${args[@]+"${args[@]}"} -Out "$(cygpath -w "$WORK/$name.vt")" \
    -TimeoutSec "${ASHIATO_INSTALL_CHECK_SETUP_TIMEOUT:-600}" >"$WORK/$name.ps.out" 2>&1 || rc=$?
  PY -c 'import re,sys; vt=re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)"); t=open(sys.argv[1],"rb").read().decode("utf-8",errors="replace"); sys.stdout.write(vt.sub("",t).replace("\r",""))' \
    "$(pypath "$WORK/$name.vt")" >"$WORK/$name.window" || {
    # 窓の台本の出力（hr=・err= など）を手がかりに出す。台本は入力の行を出さないので合言葉は載らない
    cat "$WORK/$name.ps.out" >&2 2>/dev/null || true
    die "窓の文字を読めない: $name（窓の台本 rc=$rc）"
  }
  return "$rc"
}

# 窓の文字（$1）が、最初の問いより前に合言葉の条件（表示されません まで）を持ち、2 つの問いを持つ。
# 窓の幅で折り返されても読めるよう、改行を除いた続きで見る
assert_window_prompt() {
  PY - "$(pypath "$1")" <<'PY' || die '窓に合言葉の問いと条件が読める形で出ていない'
import sys
t = open(sys.argv[1], encoding="utf-8").read().replace("\n", "")
first = t.find("画面の合言葉: ")
if first < 0 or "画面の合言葉（もう一度）: " not in t:
    sys.exit("窓に2つの問いが無い")
head = t[:first]
start = head.find("画面の合言葉を決めます")
if start < 0:
    sys.exit("最初の問いより前に条件の文が無い")
cond = head[start:]
for want in ("16 文字以上", "change-me で始めない", "API の合言葉", "ログインの印の鍵", "表示されません"):
    if want not in cond:
        sys.exit("条件の文に無い: " + want)
PY
}

# 応答の見出しからログインの印を読む。
session_cookie_from_headers() {
  sed -n 's/^[Ss]et-[Cc]ookie:[[:space:]]*ashiato_session=\([^;]*\).*/\1/p' <<<"${1//$'\r'/}" | head -n 1
}

# 状態の置き場に一時の合言葉のファイル（名前に pwfile を持つ）が無い（-o は括弧でくくる。review R25）。
assert_no_pwfile() {
  local dir="$1" found
  found="$(find "$dir" \( -name '*pwfile*' -o -name '.pwfile' -o -name '.initdb-staging-*' \) -print 2>/dev/null || true)"
  [[ -z "$found" ]] || die "一時の合言葉のファイルが残っている: $found"
}

# ============================================================================
# >>> 実地の確かめ（本物の Windows で、zip を展開して本物のインストーラを走らせる）
# ============================================================================

STATE='C:/ProgramData/ashiato'
PROGRAM='C:/Program Files/ashiato'
SETTINGS="$STATE/settings/server.json"
# 表の所有者の合言葉は管理者と SYSTEM だけが読めるファイルに分ける（design D4。deep.md Q11）。runner は上がった管理者
OWNER_SETTINGS="$STATE/settings/owner.json"
URL='http://127.0.0.1:18700'
PW1='install-check-password-0123'
PW2='install-check-reset-0123456'
OTHER_USER='ashiato-ci-other'
ADMIN_B='ashiato-ci-admin'
ADMIN_NONASCII='ashiato-ci-é'
WORK=''
FIREWALL_RULE_COUNT=0
PORT_LISTENER_PID=''
OUTBOUND_REMOTE_IPV4='0.0.0.0-126.255.255.255,128.0.0.0-255.255.255.255'
# netsh の remoteip は ::/0 を「One or more of the address prefixes is invalid.」で拒む（CI run 41956556582）。
# 範囲で書き、IPv4 の 127.0.0.0/8 と同じく IPv6 の loopback（::1）も残す。
OUTBOUND_REMOTE_IPV6='::2-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff'

win() { printf '%s' "${1//\//\\}"; }
PY() { if command -v python >/dev/null 2>&1; then python "$@"; else python3 "$@"; fi; }
# Python に渡すファイルの場所。common_setup が Git Bash の場所の読み替えを止める（MSYS_NO_PATHCONV）ので、
# Windows の Python に `/tmp/…` をそのまま渡すと開けない（CI run 42834307188）。Git Bash では `C:/…` に直して渡す
pypath() { if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi; }
setting() { PY -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8")).get(sys.argv[2], ""))' "$SETTINGS" "$1"; }
owner_setting() { PY -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8")).get(sys.argv[2], ""))' "$OWNER_SETTINGS" "$1"; }
url_password() { PY -c 'import sys, urllib.parse; print(urllib.parse.unquote(urllib.parse.urlparse(sys.argv[1]).password or ""))' "$1"; }
start_port_listener() {
  local port="$1" python_bin
  if command -v python >/dev/null 2>&1; then python_bin=python; else python_bin=python3; fi
  "$python_bin" -c 'import socket,sys,time; s=socket.socket(); s.bind(("127.0.0.1",int(sys.argv[1]))); s.listen(); time.sleep(120)' "$port" >/dev/null 2>&1 &
  PORT_LISTENER_PID=$!
}
secret_values() {
  setting API_TOKEN; setting WEB_SESSION_KEY; setting WEB_PASSWORD_VERIFIER
  url_password "$(setting DATABASE_URL)"; url_password "$(owner_setting DATABASE_OWNER_URL)"
}
# Git Bash 自身にも GNU の whoami.exe があり、PATH から呼ぶと Windows の /groups などを
# 引数として受け取れない。Windows 本体の実行ファイルを必ず使う。
windows_whoami() {
  local windows_dir
  windows_dir="$(cygpath -u "${WINDIR:-${SystemRoot:-C:\\Windows}}")"
  "$windows_dir/System32/whoami.exe" "$@"
}
# Git Bash の引数変換を通すと `/add` などを `net.exe` の switch として渡せないことがある。
# Windows 本体の実行ファイルを直接選び、引数変換だけを抑止する。cmd.exe を挟むと、空白を含む
# 実行パスの引用符がコマンド名の一部として解釈される。
windows_net() {
  local windows_dir
  windows_dir="$(cygpath -u "${WINDIR:-${SystemRoot:-C:\\Windows}}")"
  MSYS2_ARG_CONV_EXCL='*' "$windows_dir/System32/net.exe" "$@"
}
# Firewall の PowerShell cmdlet は GitHub の Windows runner で error 87 になったため、Windows 本体の
# netsh.exe を直接使う。Git Bash の引数変換を止め、空白を含む program= の値を 1 引数のまま渡す。
windows_netsh() {
  local windows_dir
  windows_dir="$(cygpath -u "${WINDIR:-${SystemRoot:-C:\\Windows}}")"
  MSYS2_ARG_CONV_EXCL='*' "$windows_dir/System32/netsh.exe" "$@"
}
block_outbound_program() {
  local rule="$1" program="$2" remote_ip="$3"
  windows_netsh advfirewall firewall add rule "name=$rule" dir=out action=block "program=$program" "remoteip=$remote_ip" enable=yes
}
outbound_block_remote_ips() {
  # loopback（127.0.0.0/8 と ::1）はインストール中の DB・サーバ間通信に必要なので、それ以外の IPv4 と IPv6 だけを遮断する。
  printf '%s\n' "$OUTBOUND_REMOTE_IPV4" "$OUTBOUND_REMOTE_IPV6"
}
clear_outbound_block_rules() {
  local count="$1" rule rc=0
  for ((rule = 1; rule <= count; rule++)); do
    windows_netsh advfirewall firewall delete rule "name=ashiato-install-check-$rule" >/dev/null || rc=1
  done
  [[ "$rc" == 0 ]] && FIREWALL_RULE_COUNT=0
  return "$rc"
}
runner_sid() { windows_whoami /user /fo csv /nh | tr -d '\r' | grep -Eo 'S-1-[0-9-]+' | head -n 1; }
runner_account() { windows_whoami | tr -d '\r'; }
user_sid() { PY -c 'import subprocess,sys; print(subprocess.check_output(["powershell.exe","-NoProfile","-Command",f"(New-Object System.Security.Principal.NTAccount(\"{sys.argv[1]}\")).Translate([System.Security.Principal.SecurityIdentifier]).Value"], text=True).strip())' "$1"; }
icacls_of() { icacls.exe "$(win "$1")" 2>/dev/null | tr -d '\r'; }
# 置き場の中から `icacls .` を読む（1 行目の頭が `.` になり、日本語を含む場所の表示の符号に依らない）
acl_of() { acl_lines "$(cd "$1" && icacls.exe . 2>/dev/null | tr -d '\r')" .; }
sc_query() { sc.exe query "$1" 2>/dev/null | tr -d '\r'; }
state_code() { service_state_of "$(sc_query "$1")"; }
psql_bin() { printf '%s' "$PROGRAM/pgsql/bin/psql.exe"; }
app_sql() {
  PGPASSWORD="$(url_password "$(setting DATABASE_URL)")" PGPASSFILE=NUL \
    "$(psql_bin)" -X -A -t -q -w -h 127.0.0.1 -p 55400 -U ashiato_app -d ashiato -c "$1" | tr -d '\r'
}
http_code() { curl -s -o /dev/null -w '%{http_code}' "$@" || true; }
login() {
  local headers
  headers="$(curl -s -D - -o /dev/null -H 'Content-Type: application/json' \
    --data "{\"password\":\"$1\"}" "$URL/session" || true)"
  session_cookie_from_headers "$headers"
}

wait_state() {
  local service="$1" code="$2" seconds="$3" deadline=$((SECONDS + $3))
  while [[ "$(state_code "$service")" != "$code" ]]; do
    if [[ "$SECONDS" -ge "$deadline" ]]; then
      service_diagnostics "$service" >&2 || true
      die "$service が $seconds 秒で状態 $code にならない"
    fi
    sleep 1
  done
}

# サービスが期待の状態にならないときの様子（run 42175126266〜42191736601 の DB の回復は、SCM が起こし直さなかったのか・
# PostgreSQL が回復中か・起きた後に落ちたのかをログから区別できなかった）。秘密は出さない（状態・プロセス・DB のログ・イベント）
service_diagnostics() {
  local service="$1"
  echo "---- 診断: $service"
  sc.exe queryex "$service" 2>&1 | tr -d '\r'
  sc.exe qfailure "$service" 2>&1 | tr -d '\r'
  echo '---- プロセス（postgres / pg_ctl / ashiato）'
  tasklist.exe /V /FO CSV /NH 2>/dev/null | tr -d '\r' | grep -Ei 'postgres|pg_ctl|ashiato' || true
  if [[ -n "${STATE:-}" ]]; then
    echo '---- postmaster.pid'
    head -n 4 "$STATE/db/17/postmaster.pid" 2>&1 || true
    local log
    for log in $(ls -t "$STATE/logs/db" 2>/dev/null | head -n 2); do
      echo "---- DB のログの末尾: $log"
      tail -n 40 "$STATE/logs/db/$log" 2>&1 || true
    done
  fi
  echo '---- サービス制御のイベント（直近）'
  powershell.exe -NoProfile -NonInteractive -Command \
    "Get-WinEvent -FilterHashtable @{LogName='System'; ProviderName='Service Control Manager'; StartTime=(Get-Date).AddMinutes(-10)} -ErrorAction SilentlyContinue | Select-Object -First 15 | ForEach-Object { '{0:HH:mm:ss} {1} {2}' -f \$_.TimeCreated, \$_.Id, (\$_.Message -replace '\\s+', ' ') }" 2>&1 | tr -d '\r' || true
  powershell.exe -NoProfile -NonInteractive -Command \
    "Get-WinEvent -FilterHashtable @{LogName='Application'; StartTime=(Get-Date).AddMinutes(-10)} -ErrorAction SilentlyContinue | Where-Object { \$_.ProviderName -match 'PostgreSQL|ashiato' } | Select-Object -First 15 | ForEach-Object { '{0:HH:mm:ss} {1} {2}' -f \$_.TimeCreated, \$_.ProviderName, (\$_.Message -replace '\\s+', ' ') }" 2>&1 | tr -d '\r' || true
  echo '---- 診断ここまで'
}

wait_healthz() {
  local seconds="$1" deadline=$((SECONDS + $1))
  while [[ "$(http_code "$URL/healthz")" != 200 ]]; do
    [[ "$SECONDS" -lt "$deadline" ]] || die "サーバが $seconds 秒で答えない"
    sleep 1
  done
}

wait_database() {
  local seconds="$1" deadline=$((SECONDS + $1)) value
  while :; do
    value="$(app_sql 'SELECT 1' 2>/dev/null || true)"
    [[ "$value" == 1 ]] && return 0
    [[ "$SECONDS" -lt "$deadline" ]] || die "DB が $seconds 秒で接続を受けない"
    sleep 1
  done
}

# サーバのログに、見張りが DB のサービスを起こし直した印（db_service_restarted）が何行あるか（D35・review R62）。
db_restart_count() {
  cat "$STATE"/logs/server/* 2>/dev/null | grep -Fc 'db_service_restarted' || true
}

# 見張りの起こし直しの印が、$1 行より増えるのを $2 秒まで待つ。増えなければ診断を出して落ちる。
wait_db_restarted_by_server() {
  local before="$1" seconds="$2" deadline=$((SECONDS + $2))
  while [[ "$(db_restart_count)" -le "$before" ]]; do
    if [[ "$SECONDS" -ge "$deadline" ]]; then
      service_diagnostics ashiato-db >&2 || true
      tail -n 40 "$(ls -t "$STATE"/logs/server/* 2>/dev/null | head -n 1)" >&2 2>/dev/null || true
      die "サーバの見張りが $seconds 秒で DB のサービスを起こし直さない（サーバのログに db_service_restarted が無い）"
    fi
    sleep 1
  done
}

# 止める・開始する（既に止まっている・回復で起こし直し中でも落ちない）。
stop_service() { sc.exe stop "$1" >/dev/null 2>&1 || true; wait_state "$1" 1 60; }
start_service() { sc.exe start "$1" >/dev/null 2>&1 || true; }

# PID のプロセスが $2 秒以内に居なくなるか
pid_gone() {
  local pid="$1" limit="$2" i
  for ((i = 0; i < limit; i++)); do
    tasklist.exe /FI "PID eq $pid" /FO CSV /NH 2>/dev/null | tr -d '\r' | grep -q "\"$pid\"" || return 0
    sleep 1
  done
  return 1
}
# postmaster.pid の先頭行は、DB の主プロセスの PID。
postmaster_pid_from() {
  local pid
  pid="$(sed -n '1{s/\r$//;p;}' <<<"$1")"
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  printf '%s\n' "$pid"
}

service_pid() {
  sc.exe queryex "$1" | tr -d '\r' | awk -F ':' 'toupper($1) ~ /PID/ { gsub(/ /, "", $2); print $2; exit }'
}

# 本人（runner）が settings に書く間だけ書きを足す（インストーラと同じ形。D21）。
with_settings_write() {
  icacls.exe "$(win "$STATE/settings")" /grant "*$(runner_sid):(OI)(CI)M" /L >/dev/null
  local rc=0
  "$@" || rc=$?
  icacls.exe "$(win "$STATE/settings")" /reset /L >/dev/null
  return "$rc"
}

set_setting() {
  PY - "$SETTINGS" "$1" "$2" <<'PY'
import json, sys
path, key, value = sys.argv[1:4]
with open(path, encoding="utf-8") as f:
    settings = json.load(f)
if value == "<delete>":
    settings.pop(key, None)
else:
    settings[key] = value
with open(path, "w", encoding="utf-8") as f:
    json.dump(settings, f, indent=2)
PY
}

# ほかの利用者（管理者でない）として PowerShell の式を走らせ、その終了コードを返す。
run_as_other() {
  powershell.exe -NoProfile -NonInteractive -Command "
    \$c = New-Object System.Management.Automation.PSCredential('$OTHER_USER', (ConvertTo-SecureString '$OTHER_PASSWORD' -AsPlainText -Force))
    \$p = Start-Process powershell.exe -Credential \$c -WorkingDirectory 'C:\\Users\\Public' -Wait -PassThru -WindowStyle Hidden -ArgumentList '-NoProfile','-NonInteractive','-Command',\"$1\"
    exit \$p.ExitCode" >/dev/null 2>&1
}

# 管理者 $1（合言葉 $2）として、上がった札でコマンドの行 $3 を走らせ、その終了コードを返す（タスクの最上位の特権）。
# タスクとその .cmd は呼び出しごとに作って消す 1 つの名前（どの管理者で走らせても同じ。一度に 1 つしか走らせない）
run_as_admin() {
  local user="$1" password="$2" cmdline="$3" name='ashiato-install-check-admin' script='C:/Users/Public/ashiato-install-check-admin.cmd'
  # タスクの命令の行は長さに上限があるので、走らせる行は .cmd のファイルに置く
  printf '@echo off\r\n%s\r\n' "$cmdline" >"$script"
  schtasks.exe /Create /TN "$name" /TR "$(win "$script")" /RU "$user" /RP "$password" \
    /RL HIGHEST /SC ONCE /ST 23:59 /F >/dev/null
  schtasks.exe /Run /TN "$name" >/dev/null
  local deadline=$((SECONDS + 900)) status
  sleep 3
  while :; do
    status="$(schtasks.exe /Query /TN "$name" /FO LIST /V | tr -d '\r')"
    grep -Eq 'Status:[[:space:]]+Running' <<<"$status" || break
    [[ "$SECONDS" -lt "$deadline" ]] || die "管理者 $user の実行が終わらない"
    sleep 2
  done
  local result
  result="$(sed -n 's/^Last Result:[[:space:]]*//p' <<<"$status" | head -n 1)"
  schtasks.exe /Delete /TN "$name" /F >/dev/null
  [[ "$result" == 0 ]]
}

# 別の管理者 B として走らせる。
run_as_admin_b() { run_as_admin "$ADMIN_B" "$ADMIN_B_PASSWORD" "$1"; }

# net.exe は 14 文字を超える合言葉に「続けますか (Y/N)」と聞き返し、入力の無い CI ではそこで作成が
# 失敗する。合言葉は 14 文字に収める（"Ci-" + 8 + "-a1"）。既にいる利用者は合言葉だけ付け直し、
# それも通らなければ作成の出力ごと止まる（捨てると後の「見つからない」しか残らない）
ensure_user() {
  local out
  out="$(windows_net user "$1" "$2" /add 2>&1)" && return 0
  windows_net user "$1" "$2" >/dev/null 2>&1 || die "利用者 $1 を作れない: $(tr -d '\r' <<<"$out")"
}

make_users() {
  OTHER_PASSWORD="Ci-$(PY -c 'import secrets; print(secrets.token_urlsafe(6))')-a1"
  ADMIN_B_PASSWORD="Ci-$(PY -c 'import secrets; print(secrets.token_urlsafe(6))')-b2"
  ensure_user "$OTHER_USER" "$OTHER_PASSWORD"
  ensure_user "$ADMIN_B" "$ADMIN_B_PASSWORD"
  windows_net localgroup Administrators "$ADMIN_B" /add >/dev/null 2>&1 || true
  ADMIN_NONASCII_PASSWORD="Ci-$(PY -c 'import secrets; print(secrets.token_urlsafe(6))')-c3"
  ensure_user "$ADMIN_NONASCII" "$ADMIN_NONASCII_PASSWORD"
  windows_net localgroup Administrators "$ADMIN_NONASCII" /add >/dev/null 2>&1 || true
}

remove_users() {
  windows_net user "$OTHER_USER" /delete >/dev/null 2>&1 || true
  windows_net user "$ADMIN_B" /delete >/dev/null 2>&1 || true
  windows_net user "$ADMIN_NONASCII" /delete >/dev/null 2>&1 || true
}

# 前の回のインストールを消し、まっさらな機械にする（状態の置き場も消す）。
clean_machine() {
  local service
  for service in ashiato-server ashiato-db; do
    if sc.exe query "$service" >/dev/null 2>&1; then
      sc.exe stop "$service" >/dev/null 2>&1 || true
      local until=$((SECONDS + 120))
      while [[ "$(state_code "$service")" != 1 && "$SECONDS" -lt "$until" ]]; do sleep 1; done
      sc.exe delete "$service" >/dev/null 2>&1 || true
    fi
  done
  taskkill.exe /F /IM postgres.exe >/dev/null 2>&1 || true
  reg.exe delete 'HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato' /f >/dev/null 2>&1 || true
  rm -rf "$PROGRAM"
  if [[ -e "$STATE" ]]; then
    icacls.exe "$(win "$STATE")" /setowner '*S-1-5-32-544' /T /C /L >/dev/null 2>&1 || true
    icacls.exe "$(win "$STATE")" /grant '*S-1-5-32-544:(OI)(CI)F' /T /C /L >/dev/null 2>&1 || true
    rm -rf "$STATE"
  fi
  [[ ! -e "$STATE" ]] || die '前の回の状態の置き場を消せない'
  rm -rf "$PROFILE_U/ashiato"
  icacls.exe "$(win "$DOWNLOADS_U")" /remove:g 'NT SERVICE\ashiato-server' /L >/dev/null 2>&1 || true
}

install_with() {
  # $1 = 標準入力に渡す 2 行の合言葉（空なら標準入力を閉じる）。残りはインストーラの引数
  # インストーラが戻らないと job の上限（6 時間）まで止まり、どこで止まったかも残らない（run 42163474147 の
  # repair が 1 時間戻らなかった）。上限を付け、超えたら理由を書いて失敗にする（呼び手が出力を見せる）
  local password="$1" rc=0; shift
  local limit="${ASHIATO_INSTALL_CHECK_SETUP_TIMEOUT:-600}"
  if [[ -n "$password" ]]; then
    printf '%s\n%s\n' "$password" "$password" | timeout -k 10 "$limit" "$SETUP" "$@" || rc=$?
  else
    timeout -k 10 "$limit" "$SETUP" "$@" </dev/null || rc=$?
  fi
  if [[ "$rc" == 124 || "$rc" == 137 ]]; then
    # 呼び手は出力を $out へ向け、「失敗するはず」の Scenario では失敗を合格と読む。止まったことを紛れさせないよう、
    # 呼び手の向け先でない元の標準エラー（fd 9）へ書き、その場で落とす
    {
      echo "NG: インストーラ（$*）が ${limit} 秒で戻らない"
      # 呼び手の多くは出力を $out へ向ける（bash の動的な見え方で呼び手の local が見える）
      [[ -n "${out:-}" && -f "${out:-}" ]] && tail -n 80 "$out"
      tasklist.exe /FO CSV /NH 2>/dev/null | tr -d '\r' | grep -Ei 'ashiato|postgres|pg_ctl|psql|takeown|icacls|powershell|sc\.exe' || true
    } >&9
    exit 1
  fi
  return "$rc"
}

common_setup() {
  local archive="$1"
  [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]] || die '実地の確かめは Windows（Git Bash）でだけ走る'
  # Git Bash の引数を Windows の場所に読み替えさせない（`/grant` などが場所に化ける）。
  # Python の出力は UTF-8 にする（取り込み待ちの日本語の場所を、パイプの先でも崩さない）
  export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' PYTHONUTF8=1 PYTHONIOENCODING=utf-8
  # 元の標準エラー。install_with が、呼び手が出力を向けた先とは別に止まった理由を書く
  exec 9>&2
  windows_whoami /groups | grep -Fq 'S-1-16-12288' || die 'P17: 上がった札（High Mandatory Level）で動いていない'
  [[ -f "$archive" ]] || die "配布 zip が無い: $archive"
  WORK="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-install.XXXXXX")"
  unzip -q "$archive" -d "$WORK/unpacked"
  SETUP="$WORK/unpacked/ashiato-setup.exe"
  [[ -f "$SETUP" && -f "$WORK/unpacked/ashiato-server.exe" && -f "$WORK/unpacked/pgsql/bin/postgres.exe" ]] ||
    die 'zip にインストーラ・サーバ・PostgreSQL が揃っていない'
  PROFILE_U="$(cygpath -m "$USERPROFILE")"
  # インストーラと同じく、登録簿の User Shell Folders からダウンロードの実の場所を読む
  local downloads
  downloads="$(reg.exe query 'HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders' \
    /v '{374DE290-123F-4565-9164-39C4925E467B}' 2>/dev/null | tr -d '\r' |
    sed -n 's/.*REG_[A-Z_]*[[:space:]]\{1,\}//p' | head -n 1)"
  downloads="${downloads//%USERPROFILE%/$USERPROFILE}"
  [[ -n "$downloads" ]] || downloads="$USERPROFILE\\Downloads"
  DOWNLOADS_U="$(cygpath -m "$downloads")"
  mkdir -p "$DOWNLOADS_U"
  INBOX_U="$PROFILE_U/ashiato/取り込み待ち"
  powershell.exe -NoProfile -NonInteractive -Command 'Set-Culture ja-JP' || die '地域の設定を ja-JP にできない'
  trap common_teardown EXIT
  make_users
}

common_teardown() {
  local exit_code=$?
  if (( FIREWALL_RULE_COUNT > 0 )); then
    clear_outbound_block_rules "$FIREWALL_RULE_COUNT" || {
      echo 'NG: 外向き遮断の規則を外せない' >&2
      [[ "$exit_code" != 0 ]] || exit_code=1
    }
  fi
  remove_users
  [[ -z "$WORK" ]] || rm -rf "$WORK"
  return "$exit_code"
}

# ---------------------------------------------------------------------------
# 配る zip（差し込みにも命令の記録にも依らない Scenario。D24）
# ---------------------------------------------------------------------------
check_release_zip() {
  common_setup "$1"
  local out="$WORK/out" listener

  echo '=== 配る zip'
  clean_machine
  echo '== Scenario: 番号が塞がっていると、登録せずに止まる'
  start_port_listener 55400
  listener="$PORT_LISTENER_PID"
  sleep 2
  if install_with "$PW1" install >"$out" 2>&1; then kill "$listener"; die '塞がった番号でインストールが進んだ'; fi
  kill "$listener" 2>/dev/null || true; wait "$listener" 2>/dev/null || true
  grep -Fq '55400' "$out" || die '塞がった番号と持ち主の案内が無い'
  ! sc.exe query ashiato-db >/dev/null 2>&1 || die '番号の検査の前に DB のサービスを登録した'
  [[ ! -e "$STATE" ]] || die '番号の検査の前に状態の置き場を作った'

  echo '== Scenario: インストーラは状態の置き場を選ぶ口を持たない'
  if "$SETUP" install --state-dir 'C:\ashiato-install-check-other' >"$out" 2>&1; then die '状態の置き場を選ぶ引数を受け付けた'; fi
  grep -Fq '不明な引数' "$out" || die '知らない引数の理由が出ない'
  [[ ! -e 'C:/ashiato-install-check-other' && ! -e "$STATE" ]] || die '知らない引数で置き場を作った'

  echo '== Scenario: インストールの間、ネットワークに出ない'
  local program remote_ip rule=0
  # 一時フォルダは runner で 8.3 の短い名前（C:\Users\RUNNER~1\…）になり、netsh が
  # 「The application name could not be resolved.」で断る。setup は長い名前で渡す
  for program in "$(cygpath -wl "$SETUP")" "$(win "$PROGRAM")\\ashiato-server.exe" "$(win "$PROGRAM")\\ashiato-setup.exe" \
      "$(win "$PROGRAM")\\pgsql\\bin\\postgres.exe" "$(win "$PROGRAM")\\pgsql\\bin\\psql.exe" \
      "$(win "$PROGRAM")\\pgsql\\bin\\pg_dump.exe" "$(win "$PROGRAM")\\pgsql\\bin\\initdb.exe" "$(win "$PROGRAM")\\pgsql\\bin\\pg_ctl.exe"; do
    while IFS= read -r remote_ip; do
      rule=$((rule + 1))
      # 数えるのは足せた規則だけ。足せなかった規則を消しにいくと、その失敗が本当の理由を覆う
      block_outbound_program "ashiato-install-check-$rule" "$program" "$remote_ip" || {
        clear_outbound_block_rules "$((rule - 1))" || die '外向き遮断の規則を外せない'
        die "外向きを止める規則を足せない: $program"
      }
      FIREWALL_RULE_COUNT=$rule
    done < <(outbound_block_remote_ips)
  done
  install_with "$PW1" install >"$WORK/install.out" 2>&1 || {
    clear_outbound_block_rules "$rule" || die '外向き遮断の規則を外せない'
    cat "$WORK/install.out" >&2
    die '外向きを止めた状態でインストールできない'
  }
  clear_outbound_block_rules "$rule" || die '外向き遮断の規則を外せない'

  echo '== Scenario: 配る zip からのインストールで、2 つのサービスが自動で起きる形で入る'
  local service
  for service in ashiato-db ashiato-server; do
    sc.exe qc "$service" | tr -d '\r' | grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*2' || die "$service が自動で起きる形でない"
    [[ "$(state_code "$service")" == 4 ]] || die "$service が動いていない"
  done
  wait_healthz 30

  echo '== Scenario: インストールは Docker も WSL も使わない'
  for service in ashiato-db ashiato-server; do
    ! sc.exe qc "$service" | grep -Eiq 'docker|wsl' || die "$service が Docker か WSL を通して動く"
  done
  ! unzip -l "$1" | grep -Eiq 'docker|wsl' || die '配布 zip に Docker か WSL の部品がある'
  ! grep -Eiq 'docker|wsl' "$WORK/install.out" || die 'インストーラの出力に Docker か WSL がある'

  echo '== Scenario: サービスは利用者でなくサービス専用のアカウントで動く'
  for service in ashiato-db ashiato-server; do
    assert_service_config "$(sc.exe qc "$service" | tr -d '\r')" "$service"
  done

  echo '== Scenario: DB のサービスが異常に終わると起こし直される'
  assert_restart_on_failure "$(sc.exe qfailure ashiato-db | tr -d '\r')" ashiato-db
  local postmaster_pid restarted_before
  postmaster_pid="$(postmaster_pid_from "$(<"$STATE/db/17/postmaster.pid")")" ||
    die 'DB の postmaster.pid から主プロセス番号を読めない'
  restarted_before="$(db_restart_count)"
  # pg_ctl のサービスではなく、postmaster.pid の先頭にある PostgreSQL の主だけを終える。
  taskkill.exe /F /PID "$postmaster_pid" >/dev/null || die 'PostgreSQL の主を異常に終わらせられない'
  pid_gone "$postmaster_pid" 10 || die 'DB の PostgreSQL の主を異常に終わらせられない'
  # 起こし直したのがサーバの見張り（D35）であることを、サーバのログの kind=db_service_restarted で確かめる。
  # 状態 4 だけを見ると、pg_ctl が主の死にまだ気づいていない間にも通ってしまう（review R62）。
  # 見張りは 5 秒ごとに読むので、pg_ctl が止まるまでの間を足しても 60 秒あれば足りる
  wait_db_restarted_by_server "$restarted_before" 60
  # PostgreSQL は強制終了後に WAL を回復してから受け付ける。CI の遅い runner でも
  # 回復そのものを失敗と取り違えないよう、十分長く待つ。
  wait_state ashiato-db 4 60
  wait_database 60
  assert_db_watch_acl "$(sc.exe sdshow ashiato-db | tr -d '\r')" "$(service_sid ashiato-server)"

  echo '== Scenario: サーバのサービスが異常に終わると起こし直される'
  assert_restart_on_failure "$(sc.exe qfailure ashiato-server | tr -d '\r')" ashiato-server
  before_pid="$(service_pid ashiato-server)"
  taskkill.exe /F /PID "$before_pid" >/dev/null || die 'サーバのサービスを異常に終わらせられない'
  sleep 3
  wait_state ashiato-server 4 90
  [[ "$(service_pid ashiato-server)" != "$before_pid" ]] || die 'サーバのサービスが起こし直されていない'
  wait_healthz 150

  echo '== Scenario: サービスを止めて起こし直すと、サーバは DB の後に起きて答える'
  sc.exe qc ashiato-server | tr -d '\r' | sed -n '/DEPENDENCIES/,/SERVICE_START_NAME/p' | grep -Fq 'ashiato-db' ||
    die 'サーバのサービスが DB に依らない'
  stop_service ashiato-server
  stop_service ashiato-db
  sc.exe start ashiato-server >/dev/null || die 'サーバのサービスを開始できない'
  wait_state ashiato-db 4 60
  wait_state ashiato-server 4 60
  wait_healthz 150

  echo '== Scenario: サービスの登録簿に秘密が無い'
  local -a secrets
  mapfile -t secrets < <(secret_values | tr -d '\r')
  [[ "${#secrets[@]}" == 5 && -n "${secrets[0]}" ]] || die '設定から秘密を読めない'
  assert_no_secret_in "$(reg.exe query 'HKLM\SYSTEM\CurrentControlSet\Services\ashiato-db' /s | tr -d '\r')" 'DB のサービスの登録簿' "${secrets[@]}"
  assert_no_secret_in "$(reg.exe query 'HKLM\SYSTEM\CurrentControlSet\Services\ashiato-server' /s | tr -d '\r')" 'サーバのサービスの登録簿' "${secrets[@]}"

  local me lines
  me="$(runner_account)"
  echo '== Scenario: 状態の置き場はほかのアカウントから読めない'
  lines="$(acl_of "$STATE")"
  assert_acl_only "$lines" '状態の置き場' 'NT SERVICE\ashiato-server' "$me"
  assert_acl_not_inherited "$lines" '状態の置き場'
  for dir in settings logs/server backups/pre-migrate; do
    assert_acl_only "$(acl_of "$STATE/$dir")" "$dir" 'NT SERVICE\ashiato-server' "$me"
  done

  echo '== Scenario: 本人は状態の置き場を読めるが書けない'
  assert_read_only_perm "$(acl_perm "$lines" "$me")" '本人'
  [[ -s "$SETTINGS" ]] && head -c 1 "$SETTINGS" >/dev/null || die '本人が設定を読めない'

  echo '== Scenario: 別の利用者は状態の置き場を読めない'
  if run_as_other "Get-Content -LiteralPath '$(win "$SETTINGS")' -ErrorAction Stop | Out-Null; exit 0"; then
    die '別の利用者が設定を読めた'
  fi
  run_as_other "exit 0" || die '別の利用者として命令を走らせられない（確かめの前提が崩れた）'

  echo '== Scenario: 表の所有者の合言葉は、サーバのサービスが読めないファイルにだけある'
  [[ -z "$(setting DATABASE_OWNER_URL)" ]] || die 'サーバのサービスが読む設定に表の所有者の合言葉がある'
  [[ "$(owner_setting DATABASE_OWNER_URL)" == postgres://ashiato_owner:* ]] || die '所有者の設定に表の所有者の接続が無い'
  lines="$(acl_lines "$(cd "$STATE/settings" && icacls.exe owner.json 2>/dev/null | tr -d '\r')" owner.json)"
  assert_acl_only "$lines" '所有者の設定' 'BUILTIN\Administrators' 'NT AUTHORITY\SYSTEM'
  assert_acl_not_inherited "$lines" '所有者の設定'
  if run_as_other "Get-Content -LiteralPath '$(win "$OWNER_SETTINGS")' -ErrorAction Stop | Out-Null; exit 0"; then
    die '別の利用者が所有者の設定を読めた'
  fi
  ! compgen -G "$STATE/settings/.owner-staging-*" >/dev/null || die '所有者の設定の書きかけの置き場が残った'

  echo '== Scenario: DB の置き場にはサーバのアカウントが書けない'
  lines="$(acl_of "$STATE/db")"
  assert_acl_only "$lines" 'DB の置き場' 'NT SERVICE\ashiato-db' "$me"
  assert_acl_not_inherited "$lines" 'DB の置き場'
  assert_modify_perm "$(acl_perm "$lines" 'NT SERVICE\ashiato-db')" 'DB のサービス'
  ! icacls_of "$STATE/db/17" | grep -Fiq 'ashiato-server' || die 'DB の置き場にサーバのアカウントの項目がある'

  echo '== Scenario: インストールが終わると、インストールの間の書きの権限が残らない'
  for dir in "" /db /logs/db /settings /backups/pre-migrate; do
    lines="$(acl_of "$STATE$dir")"
    assert_read_only_perm "$(acl_perm "$lines" "$me" | head -n 1)" "本人（$STATE$dir）"
  done
  for dir in settings backups/pre-migrate; do
    ! acl_of "$STATE/$dir" | grep -v '(I)' | grep -q . || die "$dir に一時の書きの項目が残った"
  done

  echo '== Scenario: 配る DB は OS に依らない並べ方で作られ、符号の順に並ぶ'
  [[ "$(app_sql "select datlocprovider::text || '|' || datlocale || '|' || pg_encoding_to_char(encoding) from pg_database where datname = 'ashiato'")" == 'b|C.UTF-8|UTF8' ]] ||
    die 'DB の並べ方が builtin の C.UTF-8・UTF8 でない'
  [[ "$(app_sql "select 'a' < 'B'")" == f ]] || die 'DB が符号の順に並ばない'
  [[ "$(app_sql 'show data_checksums')" == on ]] || die 'DB のチェックサムが無い'
  [[ "$(app_sql "select string_agg(rolname, ',' order by rolname) from pg_roles where rolsuper")" == ashiato ]] ||
    die 'DB の管理者の役割が ashiato の 1 つだけでない'

  echo '== Scenario: 配る DB は合言葉なしの接続を通さない'
  local role
  for role in ashiato ashiato_app ashiato_owner; do
    if PGPASSWORD='' PGPASSFILE=NUL "$(psql_bin)" -X -w -h 127.0.0.1 -p 55400 -U "$role" -d postgres -c 'select 1' >/dev/null 2>&1; then
      die "合言葉なしで $role として入れた"
    fi
  done
  assert_no_trust "$(tr -d '\r' <"$STATE/db/17/pg_hba.conf")"

  echo '== Scenario: 配る DB は手元の 55400 番だけで待ち受ける'
  [[ "$(app_sql 'show listen_addresses')" == 127.0.0.1 && "$(app_sql 'show port')" == 55400 ]] ||
    die 'DB の待ち受けの設定が 127.0.0.1:55400 でない'
  assert_loopback_listener_only "$(netstat.exe -ano -p tcp | tr -d '\r')" 55400
  ! netstat.exe -ano -p tcpv6 | tr -d '\r' | grep -Eq '\]:55400[[:space:]].*LISTEN' || die 'DB が IPv6 で待ち受けている'

  echo '== Scenario: 配る DB はログに文と値を書かない'
  local marker="ashiato-install-check-marker-$RANDOM$RANDOM"
  [[ "$(app_sql 'show log_statement')|$(app_sql 'show log_min_duration_statement')|$(app_sql 'show log_error_verbosity')|$(app_sql 'show log_min_error_statement')" == 'none|-1|terse|panic' ]] ||
    die 'DB のログの 4 つの設定が違う'
  app_sql "select '$marker'" >/dev/null
  app_sql "select 1 / 0 where '$marker' = '$marker'" >/dev/null 2>&1 || true
  sleep 2
  ! grep -RFq "$marker" "$STATE/logs/db" || die 'DB のログに送った文がある'

  echo '== Scenario: DB のサービスは logs の db に書け、DB のログのファイルができる'
  # log_directory は pg_read_all_settings の役割でないと読めないので、DB の設定のファイルで見る
  [[ "$(app_sql 'show logging_collector')" == on ]] || die 'DB がログをファイルに集めていない'
  grep -Fxq "log_directory='$STATE/logs/db'" <(tr -d '\r' <"$STATE/db/17/postgresql.conf") ||
    die 'DB のログの置き場が状態の置き場の logs/db でない'
  ls "$STATE/logs/db" | grep -Eq '^postgresql-(Sun|Mon|Tue|Wed|Thu|Fri|Sat)\.log$' || die 'DB のログのファイルが無い'
  assert_modify_perm "$(acl_perm "$(acl_of "$STATE/logs/db")" 'NT SERVICE\ashiato-db')" 'logs/db の DB のサービス'

  echo '== Scenario: 送った文と秘密はイベントログに出ない'
  local events
  events="$(wevtutil.exe qe Application "/q:*[System[Provider[@Name='PostgreSQL']]]" /f:text 2>/dev/null | tr -d '\r' || true)"
  ! grep -Fq "$marker" <<<"$events" || die 'イベントログに送った文がある'
  assert_no_secret_in "$events" 'イベントログ' "${secrets[@]}"

  echo '== Scenario: ログのファイルはほかのアカウントから読めない'
  assert_acl_only "$(acl_of "$STATE/logs/db")" 'logs/db' 'NT SERVICE\ashiato-db' "$me"
  assert_acl_only "$(acl_of "$STATE/logs/server")" 'logs/server' 'NT SERVICE\ashiato-server' "$me"
  if run_as_other "Get-ChildItem -LiteralPath '$(win "$STATE")\\logs\\db' -ErrorAction Stop | Out-Null; exit 0"; then
    die '別の利用者が DB のログを読めた'
  fi

  echo '== Scenario: ログは DB が 7 日分、サーバが 30 日分残る'
  [[ "$(app_sql 'show log_rotation_age')|$(app_sql 'show log_truncate_on_rotation')" == '1d|on' ]] &&
    grep -Fxq "log_filename='postgresql-%a.log'" <(tr -d '\r' <"$STATE/db/17/postgresql.conf") ||
    die 'DB のログが曜日の 7 つのファイルで回らない'
  ls "$STATE/logs/server" | grep -Eq '^server-[0-9]{4}-[0-9]{2}-[0-9]{2}\.log$' || die 'サーバの日付のログのファイルが無い'

  echo '== Scenario: DB を作るときの一時の合言葉のファイルが残らない'
  assert_no_pwfile "$STATE"

  echo '== Scenario: 配る DB と開発の DB は同じメジャー版'
  "$PROGRAM/pgsql/bin/postgres.exe" --version | grep -Eq 'PostgreSQL\) 17(\.|$)' || die '同梱の PostgreSQL が 17 でない'
  [[ "$(app_sql 'show server_version_num')" == 17* ]] || die '動いている DB が 17 でない'

  echo '== Scenario: インストーラの出力とログに秘密が出ない'
  assert_no_secret_in "$(cat "$WORK/install.out")" 'インストーラの出力' "${secrets[@]}"
  assert_no_secret_in "$(cat "$STATE"/logs/server/* "$STATE"/logs/db/* 2>/dev/null)" 'ログ' "${secrets[@]}"

  echo '== Scenario: 作った秘密でサーバの起動の検査が通る'
  # API の合言葉は収集アプリの口だけに通る（ST44 D4。/sources は 403）。収集アプリが最初に打つ GET /contract で見る
  local contract_code
  wait_healthz 30
  contract_code="$(http_code -H "Authorization: Bearer $(setting API_TOKEN)" "$URL/contract")"
  [[ "$contract_code" == 200 ]] ||
    die "作った API の合言葉で読めない (HTTP $contract_code)"

  echo '== Scenario: 配る形に移行を当ててサーバが起きる'
  [[ "$(app_sql "select count(*) > 0 from information_schema.tables where table_schema = 'core'")" == t ]] ||
    die '移行が当たっていない'
  wait_healthz 30

  echo '== Scenario: 実行時の役割は管理者でも表の所有者でもない'
  [[ "$(setting DATABASE_URL)" == postgres://ashiato_app:* ]] || die '実行時の接続が ashiato_app でない'
  [[ "$(app_sql "select rolsuper || '|' || rolcreaterole || '|' || rolcreatedb from pg_roles where rolname = current_user")" == 'false|false|false' ]] ||
    die '実行時の役割が管理者の権限を持つ'
  [[ "$(app_sql 'select count(*) from pg_tables where tableowner = current_user')" == 0 ]] || die '実行時の役割が表の所有者である'

  echo '== Scenario: 移行の前の退避は状態の置き場に、同梱の pg_dump で取られる'
  [[ "$(setting MIGRATE_BACKUP_DIR)" == "$STATE/backups/pre-migrate" && "$(setting PG_DUMP)" == "$PROGRAM/pgsql/bin/pg_dump.exe" ]] ||
    die '退避の置き場か pg_dump の場所が違う'
  [[ -f "$PROGRAM/pgsql/bin/pg_dump.exe" ]] || die '同梱の pg_dump が無い'
  local dumps
  dumps="$(find "$STATE/backups/pre-migrate" -name 'pre-migrate-*.dump' -size +0 | wc -l)"
  [[ "$dumps" -ge 1 ]] || die '移行の前の退避が無い'

  echo '== Scenario: サービスの起動では移行を当てない'
  stop_service ashiato-server
  start_service ashiato-server; wait_healthz 150
  [[ "$(find "$STATE/backups/pre-migrate" -name 'pre-migrate-*.dump' | wc -l)" == "$dumps" ]] ||
    die 'サービスの起動で移行の前の退避が取られた（移行を当てた）'

  echo '== Scenario: 取り込み待ちは本人のプロファイルの直下にでき、サーバが読み書きできる'
  [[ -d "$INBOX_U" ]] || die '取り込み待ちが本人のプロファイルの直下に無い'
  [[ "$(setting ASHIATO_INBOX_DIR)" == "$INBOX_U" ]] || die '設定の取り込み待ちの場所が違う'
  grep -Eiq 'Documents|Desktop|OneDrive' <<<"$INBOX_U" && die '取り込み待ちが同期されるフォルダの下にある'
  assert_modify_perm "$(acl_perm "$(acl_of "$INBOX_U")" 'NT SERVICE\ashiato-server')" '取り込み待ちのサーバ'

  echo '== Scenario: ダウンロードにはサーバのアカウントの読みだけが足される'
  assert_read_only_perm "$(acl_perm "$(acl_of "$DOWNLOADS_U")" 'NT SERVICE\ashiato-server')" 'ダウンロードのサーバ'

  echo '== Scenario: 取り込み待ちを消しても、サーバが作り直す'
  rm -rf "$INBOX_U"
  stop_service ashiato-server
  start_service ashiato-server; wait_healthz 150
  local deadline=$((SECONDS + 120))
  until [[ -d "$INBOX_U" ]]; do [[ "$SECONDS" -lt "$deadline" ]] || die 'サーバが取り込み待ちを作り直さない'; sleep 2; done
  assert_modify_perm "$(acl_perm "$(acl_of "$INBOX_U")" 'NT SERVICE\ashiato-server')" '作り直した取り込み待ちのサーバ'

  echo '== Scenario: インストールした形の書き出しは、本人が読めて、ほかのアカウントは読めない'
  # 書き出しを 1 回走らせ、サービスが既定の置き場（状態の置き場の export）に zip を作ることを見る。
  # 置き場が無いときに状態の置き場へ倒すと、書き出しの置き場を確かめないまま通る（review R36・R43）
  # 書き出しは画面の口（ST44 D4 で API の合言葉は 403）。本人のログインの印で始める
  local export="$STATE/export" export_code export_deadline export_cookie
  export_cookie="$(login "$PW1")"
  [[ -n "$export_cookie" ]] || die '書き出しの前にログインできない'
  export_code="$(curl -s -o /dev/null -w '%{http_code}' -X POST \
    -H "Cookie: ashiato_session=$export_cookie" -H 'Content-Type: application/json' \
    --data '{"password":"install-check-export-0123"}' "$URL/export" || true)"
  [[ "$export_code" == 202 ]] || die "書き出しを始められない (HTTP $export_code)"
  export_deadline=$((SECONDS + 180))
  until compgen -G "$export/ashiato-export-*.zip" >/dev/null; do
    [[ "$SECONDS" -lt "$export_deadline" ]] || die "書き出しの zip が $(win "$export") にできない（サービスが既定の置き場を使っていない）"
    sleep 2
  done
  ls "$export" >/dev/null || die '本人が書き出しの置き場を読めない'
  assert_acl_only "$(acl_of "$export")" '書き出しの置き場' 'NT SERVICE\ashiato-server' "$me"
  if run_as_other "Get-ChildItem -LiteralPath '$(win "$export")' -ErrorAction Stop | Out-Null; exit 0"; then
    die '別の利用者が書き出しの置き場を読めた'
  fi

  echo '== Scenario: 決めた合言葉で、そのまま画面にログインできる'
  local cookie_old
  cookie_old="$(login "$PW1")"
  [[ -n "$cookie_old" ]] || die 'インストールで決めた合言葉でログインできない'
  [[ "$(http_code -H "Cookie: ashiato_session=$cookie_old" "$URL/session")" == 200 ]] || die 'ログインの印が通らない'

  echo '== Scenario: 短い・既定の雛形・一致しない合言葉は受け付けない'
  local before_other
  before_other="$(PY -c 'import json,sys; s=json.load(open(sys.argv[1],encoding="utf-8")); s.pop("WEB_PASSWORD_VERIFIER"); print(json.dumps(s, sort_keys=True))' "$SETTINGS")"
  local records
  records="$(app_sql 'select count(*) from core.source')"
  printf '%s\n' short short change-me-password-0123 change-me-password-0123 "$PW2" other-password-0123456 "$PW2" "$PW2" |
    "$SETUP" password >"$out" 2>&1 || { cat "$out" >&2; die '入れ直しの後の合言葉で決め直せない'; }
  grep -Fq '受け付けない: 短い' "$out" && grep -Fq '受け付けない: 雛形' "$out" && grep -Fq '受け付けない: 一致しない' "$out" ||
    die '短い・雛形・一致しない合言葉を理由つきで断らない'

  echo '== Scenario: 端末でないときも、断りより前に合言葉の条件を 1 回だけ示す'
  # インストールの最後・password の口・repair の口（repair の節で同じ関数を呼ぶ）の 3 つで同じ形
  assert_requirements_line "$WORK/install.out" 'インストールの最後'
  assert_requirements_line "$out" 'password の口'

  echo '== Scenario: 短い・雛形で断るときは、理由に条件を添える'
  grep -Fq '受け付けない: 短い（16 文字以上）' "$out" && grep -Fq '受け付けない: 雛形（change-me で始めない）' "$out" ||
    die '短い・雛形の断りに条件が添えられていない'

  echo '== Scenario: 決め直した合言葉でログインでき、前の合言葉ではできない'
  wait_healthz 150
  [[ -n "$(login "$PW2")" ]] || die '決め直した合言葉でログインできない'
  [[ -z "$(login "$PW1")" ]] || die '前の合言葉でログインできた'

  echo '== Scenario: 決め直すと、それまでのログインは切れる'
  [[ "$(http_code -H "Cookie: ashiato_session=$cookie_old" "$URL/session")" == 401 ]] || die '決め直す前のログインの印が通る'

  echo '== Scenario: 決め直しても、ほかの秘密と記録は変わらない'
  [[ "$(PY -c 'import json,sys; s=json.load(open(sys.argv[1],encoding="utf-8")); s.pop("WEB_PASSWORD_VERIFIER"); print(json.dumps(s, sort_keys=True))' "$SETTINGS")" == "$before_other" ]] ||
    die '決め直しでほかの設定が変わった'
  [[ "$(app_sql 'select count(*) from core.source')" == "$records" ]] || die '決め直しで記録が変わった'

  # 窓の節（design D6）。窓の試験が化けを見分けられることを先に確かめる（陰性の対照）。
  # CI の install-windows は、インストールの前に独立の step（--probe-console。tasks 3.3）でも同じ対照を走らせる。
  # 正本はこちら（窓の節の結果を読む前に必ず通る。CI の外で台本だけを走らせても落ちない）。CI の step は
  # インストールに時間をかける前に早く落とすためで、二重に走る分はインストールより短い（review/code.md R25）。
  # console_probe は trap を張り替えるので、子の shell で走らせる
  (console_probe) || die '窓の試験が化けを見分けられない'
  local api session win_in="$WORK/pw2.in" win_in_short="$WORK/pw2-short.in" win_setup win_rc=0
  api="$(setting API_TOKEN)"; session="$(setting WEB_SESSION_KEY)"
  printf '%s\n%s\n' "$PW2" "$PW2" >"$win_in"
  # 向け替えの窓では、先に短い合言葉を打って断りの理由も窓に出ることを見る（review/code.md R5）
  printf '%s\n%s\n%s\n%s\n' short short "$PW2" "$PW2" >"$win_in_short"
  win_setup="$(cygpath -w "$SETUP")"

  echo '== Scenario: コード頁が UTF-8 でない窓でも、合言葉の問いと条件が読める'
  window_run window1 "chcp 932 >nul & chcp & \"$win_setup\" password" "$win_in" || win_rc=$?
  local codepage
  codepage="$(sed -n 's/.*:[[:space:]]*\([0-9]\{3,5\}\)[[:space:]]*$/\1/p' "$WORK/window1.window" | head -n 1)"
  [[ -n "$codepage" && "$codepage" != 65001 ]] || { cat "$WORK/window1.window" >&2; die "窓のコード頁が読めないか UTF-8 のまま（chcp: ${codepage:-読めない}）"; }
  assert_window_prompt "$WORK/window1.window"

  echo '== Scenario: 窓で打った合言葉で決め直せる'
  [[ "$win_rc" == 0 ]] || { cat "$WORK/window1.window" "$WORK/window1.ps.out" >&2; die "窓での決め直しが失敗した（rc=$win_rc）"; }
  wait_healthz 150
  [[ -n "$(login "$PW2")" ]] || die '窓で決めた合言葉でログインできない'

  echo '== Scenario: 窓で打った合言葉は、窓に出ない'
  ! grep -Fq -- "$PW2" "$WORK/window1.vt" "$WORK/window1.window" || die '窓の文字に打った合言葉がある'
  assert_no_secret_in "$(cat "$WORK/window1.window")" '窓の文字' "$api" "$session"

  echo '== Scenario: 標準出力と標準エラーを向け替えても、合言葉の問いは窓に出る'
  win_rc=0
  window_run window2 "chcp 932 >nul & \"$win_setup\" password >\"$(cygpath -w "$WORK/redirect.out.txt")\" 2>\"$(cygpath -w "$WORK/redirect.err.txt")\"" "$win_in_short" || win_rc=$?
  [[ "$win_rc" == 0 ]] || { cat "$WORK/window2.window" "$WORK/window2.ps.out" >&2; die "向け替えた窓での決め直しが失敗した（rc=$win_rc）"; }
  grep -Fq '画面の合言葉: ' "$WORK/window2.window" || die '向け替えると問いが窓に出ない'
  grep -Fq '16 文字以上' "$WORK/window2.window" || die '向け替えると条件が窓に出ない'
  # 窓の幅で折り返されても読めるよう、改行を除いた続きで見る
  tr -d '\n' <"$WORK/window2.window" | grep -Fq '受け付けない: 短い（16 文字以上）' || die '向け替えると断りの理由が窓に出ない'
  ! grep -Fq '画面の合言葉: ' "$WORK/redirect.out.txt" "$WORK/redirect.err.txt" || die '向け替えた先に問いが書かれた'
  assert_no_secret_in "$(cat "$WORK/window2.window" "$WORK/redirect.out.txt" "$WORK/redirect.err.txt")" '向け替えの窓と出力' "$api" "$session" "$PW2"
  wait_healthz 150
  [[ -n "$(login "$PW2")" ]] || die '向け替えた窓で決めた合言葉でログインできない'

  echo '== Scenario: API の合言葉やログインの印の鍵と同じ値は受け付けない'
  printf '%s\n' "$api" "$api" "$session" "$session" "$PW1" "$PW1" | "$SETUP" password >"$out" 2>&1 ||
    die 'API の合言葉と同じ値を断った後に決め直せない'
  grep -Fq '受け付けない: API の合言葉と同じ' "$out" && grep -Fq '受け付けない: ログインの印の鍵と同じ' "$out" ||
    die 'API の合言葉・ログインの印の鍵と同じ値を理由つきで断らない'
  assert_no_secret_in "$(cat "$out")" '決め直しの出力' "$api" "$session"
  wait_healthz 150

  echo '== Scenario: 合言葉の元の値は、どこにも残らない'
  ! grep -RFaq -e "$PW1" -e "$PW2" "$STATE" 2>/dev/null || die '状態の置き場に合言葉の元の値がある'
  ! reg.exe query 'HKLM\SYSTEM\CurrentControlSet\Services\ashiato-server' /s | grep -Fq -e "$PW1" -e "$PW2" || die '登録簿に合言葉の元の値がある'
  ! grep -Fq -e "$PW1" -e "$PW2" "$WORK/install.out" "$out" || die 'インストーラの出力に合言葉の元の値がある'

  echo '== Scenario: インストールした形の設定は、画面の合言葉の元の値を持たない'
  [[ "$(setting WEB_PASSWORD_VERIFIER)" == '$argon2id$'* ]] || die '設定に照らすための値が無い'
  [[ -z "$(setting WEB_PASSWORD)" ]] || die '設定に WEB_PASSWORD がある'
  ! grep -Fq -e "$PW1" -e "$PW2" "$SETTINGS" || die '設定に合言葉の元の値がある'

  echo '== Scenario: 起動を拒んだ理由が、サーバのログのファイルに残る'
  stop_service ashiato-server
  cp "$SETTINGS" "$WORK/settings.saved"
  with_settings_write set_setting WEB_SESSION_KEY 123456789012345
  sc.exe start ashiato-server >/dev/null 2>&1 || true
  wait_state ashiato-server 1 120
  grep -RFq 'kind=web_session_key' "$STATE/logs/server" || die '起動を拒んだ理由がサーバのログに無い'
  with_settings_write cp "$WORK/settings.saved" "$SETTINGS"

  echo '== Scenario: 設定のファイルが無いと、理由を書いて起動しない'
  with_settings_write mv "$SETTINGS" "$WORK/settings.moved"
  sc.exe start ashiato-server >/dev/null 2>&1 || true
  wait_state ashiato-server 1 120
  grep -RFq 'settings_missing' "$STATE/logs/server" || die '設定が無い理由がサーバのログに無い'
  with_settings_write cp "$WORK/settings.saved" "$SETTINGS"
  start_service ashiato-server; wait_healthz 150

  echo '== Scenario: 走らせ直しても秘密と DB は変わらない'
  local settings_sum db_oid
  local owner_sum
  settings_sum="$(sha256sum <"$SETTINGS")"
  owner_sum="$(sha256sum <"$OWNER_SETTINGS")"
  db_oid="$(app_sql "select oid from pg_database where datname = 'ashiato'")"
  install_with '' install >"$out" 2>&1 || { cat "$out" >&2; die '走らせ直しが失敗'; }
  [[ "$(sha256sum <"$SETTINGS")" == "$settings_sum" ]] || die '走らせ直しで設定が変わった'
  [[ "$(sha256sum <"$OWNER_SETTINGS")" == "$owner_sum" ]] || die '走らせ直しで所有者の設定が変わった'
  [[ "$(app_sql "select oid from pg_database where datname = 'ashiato'")" == "$db_oid" ]] || die '走らせ直しで DB が作り直された'
  [[ "$(app_sql 'select count(*) from core.source')" == "$records" ]] || die '走らせ直しで記録が変わった'

  echo '== Scenario: 合言葉を決める前に閉じたら、走らせ直しで合言葉の段からやり直す'
  stop_service ashiato-server
  with_settings_write set_setting WEB_PASSWORD_VERIFIER ''
  if install_with '' install >"$out" 2>&1; then die '標準入力を閉じたのにインストールが終わった'; fi
  grep -Fq '合言葉の入力が無い' "$out" || die '合言葉の段で止まっていない'
  install_with "$PW1" install >"$out" 2>&1 || { cat "$out" >&2; die '合言葉の段からのやり直しが失敗'; }
  [[ "$(app_sql "select oid from pg_database where datname = 'ashiato'")" == "$db_oid" ]] || die 'やり直しで DB が作り直された'
  [[ -n "$(login "$PW1")" ]] || die 'やり直しで決めた合言葉でログインできない'

  echo '== Scenario: 別の利用者が走らせ直しても、本人は変わらない'
  local owner_sid
  owner_sid="$(runner_sid)"
  printf '%s\n%s\n' "$PW1" "$PW1" >'C:/Users/Public/ashiato-install-check.in'
  cp -r "$WORK/unpacked" 'C:/Users/Public/ashiato-install-check-zip'
  run_as_admin_b "\"C:\\Users\\Public\\ashiato-install-check-zip\\ashiato-setup.exe\" install --profile \"$(win "$PROFILE_U")\" --downloads \"$(win "$DOWNLOADS_U")\" < C:\\Users\\Public\\ashiato-install-check.in > C:\\Users\\Public\\ashiato-install-check.out 2>&1" ||
    { cat 'C:/Users/Public/ashiato-install-check.out' >&2; die '別の管理者の走らせ直しが失敗'; }
  [[ "$(setting ASHIATO_OWNER_SID)" == "$owner_sid" ]] || die '走らせ直しで設定の本人が変わった'
  assert_acl_only "$(acl_of "$STATE")" '走らせ直しの後の状態の置き場' 'NT SERVICE\ashiato-server' "$me"

  echo '== Scenario: DB はあるのに秘密が無いと、何も書き換えずに止まる'
  local pg_version_sum listing
  pg_version_sum="$(sha256sum <"$STATE/db/17/PG_VERSION")"
  with_settings_write mv "$SETTINGS" "$WORK/settings.away"
  listing="$(ls -la "$STATE/settings")"
  if install_with "$PW1" install >"$out" 2>&1; then die '秘密の無い DB でインストールが進んだ'; fi
  grep -Fq 'repair' "$out" && grep -Fq 'ProgramData' "$out" || die '食い違いと置き場と repair の案内が無い'
  [[ ! -e "$SETTINGS" && "$(ls -la "$STATE/settings")" == "$listing" ]] || die '止まるときに設定を書いた'
  [[ "$(sha256sum <"$STATE/db/17/PG_VERSION")" == "$pg_version_sum" ]] || die '止まるときに DB を書き換えた'

  echo '== Scenario: 設定を退けて止まった後に repair すると、記録を残したままログインできる'
  local old_token
  old_token="$(PY -c 'import json,sys; print(json.load(open(sys.argv[1],encoding="utf-8"))["API_TOKEN"])' "$(cygpath -m "$WORK/settings.away")")"
  install_with "$PW1" repair >"$out" 2>&1 || { cat "$out" >&2; die 'repair が失敗'; }
  assert_requirements_line "$out" 'repair の口'
  [[ -f "$SETTINGS" ]] || die 'repair の後に設定が無い'
  wait_healthz 150
  [[ -n "$(login "$PW1")" ]] || die 'repair の後にログインできない'
  [[ "$(app_sql "select oid from pg_database where datname = 'ashiato'")" == "$db_oid" ]] || die 'repair で DB が作り直された'
  [[ "$(app_sql 'select count(*) from core.source')" == "$records" ]] || die 'repair で記録が変わった'

  echo '== Scenario: repair の後は前の API の合言葉では送れず、繋ぎ直しの案内が出る'
  [[ "$(http_code -H "Authorization: Bearer $old_token" "$URL/contract")" == 401 ]] || die '前の API の合言葉が通る'
  [[ "$(http_code -H "Authorization: Bearer $(setting API_TOKEN)" "$URL/contract")" == 200 ]] || die '新しい API の合言葉が通らない'
  grep -Fq '繋ぎ直し' "$out" && grep -Fq 'docs/install.md' "$out" || die '繋ぎ直しの案内が無い'
  assert_no_secret_in "$(cat "$out")" 'repair の出力' "$(setting API_TOKEN)" "$old_token"

  echo '== Scenario: repair の後に合言葉なしの入口と一時の番号が残らない'
  [[ ! -e "$STATE/settings/repair.pg_hba.conf" && ! -e "$STATE/settings/repair.pg_ident.conf" ]] || die '一時の接続の規則が残った'
  ! powershell.exe -NoProfile -NonInteractive -Command "Get-CimInstance Win32_Process -Filter \"Name='postgres.exe'\" | Select-Object -ExpandProperty CommandLine" |
    grep -Fiq 'hba_file' || die 'DB が一時の接続の規則で動いている'
  assert_no_trust "$(tr -d '\r' <"$STATE/db/17/pg_hba.conf")"
  ! grep -Ev '^[[:space:]]*(#|$)' "$STATE/db/17/pg_hba.conf" | grep -Ewq 'sspi' || die 'pg_hba.conf に sspi の入口がある'
  local pid ports
  ports="$(for pid in $(tasklist.exe /FI 'IMAGENAME eq postgres.exe' /FO CSV /NH | tr -d '\r' | cut -d, -f2 | tr -d '"'); do
    netstat.exe -ano -p tcp | tr -d '\r' | awk -v pid="$pid" '$NF == pid && $4 ~ /LISTEN/ { print $2 }'; done | sort -u)"
  [[ "$ports" == '127.0.0.1:55400' ]] || die "postgres が 55400 のほかに待ち受けている: $ports"

  echo '== Scenario: 止まった状態でなければ repair は何も変えずに終わる'
  settings_sum="$(sha256sum <"$SETTINGS")"
  if install_with "$PW1" repair >"$out" 2>&1; then die '止まった状態でない repair が進んだ'; fi
  grep -Fq '走らせ直す' "$out" && grep -Fq 'password' "$out" || die 'repair が要らない理由の案内が無い'
  [[ "$(sha256sum <"$SETTINGS")" == "$settings_sum" ]] || die '要らない repair で設定が変わった'

  echo '== Scenario: 別の管理者が repair しても、本人は変わらない'
  with_settings_write mv "$SETTINGS" "$WORK/settings.away2"
  run_as_admin_b "\"C:\\Users\\Public\\ashiato-install-check-zip\\ashiato-setup.exe\" repair --profile \"$(win "$PROFILE_U")\" --downloads \"$(win "$DOWNLOADS_U")\" < C:\\Users\\Public\\ashiato-install-check.in > C:\\Users\\Public\\ashiato-install-check.out 2>&1" ||
    { cat 'C:/Users/Public/ashiato-install-check.out' >&2; die '別の管理者の repair が失敗'; }
  [[ "$(setting ASHIATO_OWNER_SID)" == "$owner_sid" ]] || die '別の管理者の repair で本人が変わった'
  assert_acl_only "$(acl_of "$STATE")" 'repair の後の状態の置き場' 'NT SERVICE\ashiato-server' "$me"
  wait_healthz 150

  echo '== Scenario: 英字でない名前の管理者でも repair でき、決めた合言葉でログインできる'
  with_settings_write mv "$SETTINGS" "$WORK/settings.away3"
  if ! run_as_admin "$ADMIN_NONASCII" "$ADMIN_NONASCII_PASSWORD" "\"C:\\Users\\Public\\ashiato-install-check-zip\\ashiato-setup.exe\" repair --profile \"$(win "$PROFILE_U")\" --downloads \"$(win "$DOWNLOADS_U")\" < C:\\Users\\Public\\ashiato-install-check.in > C:\\Users\\Public\\ashiato-install-check.out 2>&1"; then
    cat 'C:/Users/Public/ashiato-install-check.out' >&2
    if [[ -f "$STATE/settings/repair.pg_ident.conf" ]]; then xxd "$STATE/settings/repair.pg_ident.conf" >&2; fi
    if [[ -f "$STATE/settings/repair-db.log" ]]; then tail -n 20 "$STATE/settings/repair-db.log" >&2; fi
    die '英字でない名前の管理者の repair が失敗'
  fi
  wait_healthz 150
  [[ -n "$(login "$PW1")" ]] || die '英字でない名前の管理者の repair の後にログインできない'
  [[ "$(app_sql "select oid from pg_database where datname = 'ashiato'")" == "$db_oid" ]] || die '英字でない名前の管理者の repair で DB が作り直された'
  [[ "$(app_sql 'select count(*) from core.source')" == "$records" ]] || die '英字でない名前の管理者の repair で記録が変わった'

  echo '== Scenario: インストールすると「アプリと機能」に載る'
  local key='HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\ashiato' version
  version="$(basename "$1" | sed -E 's/^ashiato-server-(.*)-windows-x86_64\.zip$/\1/')"
  reg.exe query "$key" /v DisplayName | tr -d '\r' | grep -Eq 'DisplayName[[:space:]]+REG_SZ[[:space:]]+ashiato$' || die '表示名が無い'
  reg.exe query "$key" /v DisplayVersion | tr -d '\r' | grep -Fq "$version" || die "版番号が $version でない"
  reg.exe query "$key" /v UninstallString | tr -d '\r' | grep -Fq '"C:\Program Files\ashiato\ashiato-setup.exe" uninstall' ||
    die 'UninstallString がプログラムの置き場のインストーラを指さない'
  [[ -f "$PROGRAM/ashiato-setup.exe" ]] || die 'UninstallString の指すインストーラが無い'

  echo '== Scenario: インストールごとに秘密が違う'
  local first_secrets
  first_secrets="$(secret_values | head -n 2; url_password "$(setting DATABASE_URL)"; url_password "$(owner_setting DATABASE_OWNER_URL)")"
  "$PROGRAM/ashiato-setup.exe" uninstall >"$out" 2>&1 || { cat "$out" >&2; die '入れ直しの前の消す口が失敗'; }
  sleep 5
  clean_machine
  install_with "$PW1" install >"$out" 2>&1 || { cat "$out" >&2; die '入れ直しが失敗'; }
  local value
  while IFS= read -r value; do
    [[ -n "$value" ]] || continue
    ! { secret_values | head -n 2; url_password "$(setting DATABASE_URL)"; url_password "$(owner_setting DATABASE_OWNER_URL)"; } | grep -Fxq -- "$value" ||
      die '入れ直しで同じ秘密が作られた'
  done <<<"$first_secrets"

  echo '== Scenario: 消すとサービスとプログラムと登録が消える'
  printf 'keep\n' >"$INBOX_U/install-check-keep.txt"
  "$PROGRAM/ashiato-setup.exe" uninstall >"$out" 2>&1 || { cat "$out" >&2; die '消す口が失敗'; }
  deadline=$((SECONDS + 60))
  while [[ -e "$PROGRAM" ]]; do [[ "$SECONDS" -lt "$deadline" ]] || die 'プログラムの置き場が消えない'; sleep 2; done
  ! sc.exe query ashiato-db >/dev/null 2>&1 || die 'DB のサービスが残った'
  ! sc.exe query ashiato-server >/dev/null 2>&1 || die 'サーバのサービスが残った'
  ! reg.exe query "$key" >/dev/null 2>&1 || die '「アプリと機能」の登録が残った'

  echo '== Scenario: 消しても記録の置き場は残り、その場所を示す'
  [[ -f "$STATE/db/17/PG_VERSION" && -f "$SETTINGS" ]] || die '状態の置き場が消えた'
  [[ -f "$INBOX_U/install-check-keep.txt" ]] || die '取り込み待ちの中身が消えた'
  grep -Fq 'ProgramData' "$out" && grep -Fq 'docs/install.md' "$out" || die '残した置き場と手順の案内が無い'

  echo '== Scenario: 消すとダウンロードと取り込み待ちに足した権限が外れる'
  # サービスを消した後は、残った ACE が名前でなく SID で表示される。名前と SID の両方で見る（review R31）
  local server_sid
  server_sid="$(service_sid ashiato-server)"
  [[ -z "$(acl_perm "$(acl_of "$DOWNLOADS_U")" 'NT SERVICE\ashiato-server')" ]] || die 'ダウンロードにサーバの項目が残った'
  [[ -z "$(acl_perm "$(acl_of "$INBOX_U")" 'NT SERVICE\ashiato-server' | grep -v '(I)')" ]] || die '取り込み待ちにサーバの項目が残った'
  ! icacls.exe "$(win "$DOWNLOADS_U")" | grep -Fq "$server_sid" || die 'ダウンロードにサーバの SID の項目が残った'
  ! icacls.exe "$(win "$INBOX_U")" | grep -v '(I)' | grep -Fq "$server_sid" || die '取り込み待ちにサーバの SID の項目が残った'

  echo '== Scenario: 消した後に入れ直すと、前の記録の上で続く'
  settings_sum="$(sha256sum <"$SETTINGS")"
  install_with '' install >"$out" 2>&1 || { cat "$out" >&2; die '消した後の入れ直しが失敗'; }
  wait_healthz 150
  [[ "$(sha256sum <"$SETTINGS")" == "$settings_sum" ]] || die '入れ直しで設定が変わった'
  [[ -n "$(login "$PW1")" ]] || die '入れ直しの後に前の合言葉でログインできない'

  echo '== Scenario: 上がった方が別の管理者でも、本人は上がる前に控えた利用者になる'
  local other_sid
  other_sid="$(user_sid "$OTHER_USER")"
  "$PROGRAM/ashiato-setup.exe" uninstall >"$out" 2>&1 || true
  sleep 5
  clean_machine
  install_with "$PW1" install --owner-sid "$other_sid" --profile "$(win "$PROFILE_U")" --downloads "$(win "$DOWNLOADS_U")" >"$out" 2>&1 ||
    { cat "$out" >&2; die '本人を渡したインストールが失敗'; }
  lines="$(acl_lines "$(icacls.exe "$(win "$STATE")" | tr -d '\r')" "$(win "$STATE")")"
  assert_read_only_perm "$(acl_perm "$lines" "$(hostname)\\$OTHER_USER")" '控えた本人'
  [[ -z "$(acl_perm "$lines" "$me")" ]] || die '上がった管理者が状態の置き場の ACL にある'
  run_as_other "Get-Content -LiteralPath '$(win "$SETTINGS")' -ErrorAction Stop | Out-Null; exit 0" || die '控えた本人が設定を読めない'
  if run_as_other "Set-Content -LiteralPath '$(win "$STATE")\\install-check-write' -Value x -NoNewline -ErrorAction Stop; exit 0"; then
    die '本人が状態の置き場に書けた'
  fi
  if run_as_other "Add-Content -LiteralPath '$(win "$SETTINGS")' -Value x -NoNewline -ErrorAction Stop; exit 0"; then
    die '本人が設定のファイルに書けた'
  fi

  "$PROGRAM/ashiato-setup.exe" uninstall >/dev/null 2>&1 || true
  sleep 5
  clean_machine
  rm -rf 'C:/Users/Public/ashiato-install-check-zip' 'C:/Users/Public/ashiato-install-check.in' \
    'C:/Users/Public/ashiato-install-check.out' 'C:/Users/Public/ashiato-install-check-admin.cmd'
  echo 'OK: 配る zip の実地の確かめ'
}

# ---------------------------------------------------------------------------
# 試験の zip（差し込み ASHIATO_SETUP_TEST_*・ASHIATO_SERVICE_TEST_* か命令の記録に依る Scenario。D24）
# ---------------------------------------------------------------------------
check_test_zip() {
  common_setup "$1"
  local out="$WORK/out" trace="$WORK/trace"

  echo '=== 試験の zip'
  clean_machine
  echo '== Scenario: サービス専用のアカウントで登録できなければ、共有のアカウントへ倒さず止まる'
  if ASHIATO_SETUP_TEST_REGISTER_FAIL=1 install_with "$PW1" install >"$out" 2>&1; then die '登録の失敗でインストールが進んだ'; fi
  grep -Fq '登録に失敗' "$out" || die '登録の失敗の理由が出ない'
  ! sc.exe query ashiato-db >/dev/null 2>&1 || die '登録の失敗の後に DB のサービスがある'
  ! sc.exe query ashiato-server >/dev/null 2>&1 || die '登録の失敗の後にサーバのサービスがある'
  ! reg.exe query 'HKLM\SYSTEM\CurrentControlSet\Services' /s /f LocalSystem /d 2>/dev/null | grep -Fiq 'ashiato' || die '共有のアカウントでサービスを作った'

  echo '== Scenario: DB を作る途中で落ちても、走らせ直しで DB ができる'
  clean_machine
  if ASHIATO_SETUP_TEST_STOP_AT=initdb install_with "$PW1" install >"$out" 2>&1; then die 'initdb の段の停止でインストールが進んだ'; fi
  [[ ! -f "$STATE/db/17/PG_VERSION" ]] || die 'initdb の段で止めたのに DB がある'
  ASHIATO_SETUP_TEST_TRACE=1 install_with "$PW1" install >"$trace" 2>&1 || { cat "$trace" >&2; die 'initdb の段からの走らせ直しが失敗'; }
  [[ -f "$STATE/db/17/PG_VERSION" ]] || die '走らせ直しで DB ができていない'
  wait_healthz 150

  echo '== Scenario: 合言葉を決め終えると、既定のブラウザで画面を開く'
  local server_line browser_line
  server_line="$(grep -n '^result=ok sc.exe start ashiato-server' "$trace" | tail -n 1 | cut -d: -f1)"
  browser_line="$(grep -n '^result=ok Start-Process <web-url>' "$trace" | tail -n 1 | cut -d: -f1)"
  [[ -n "$server_line" && -n "$browser_line" && "$browser_line" -gt "$server_line" ]] ||
    die 'サーバの開始の後に既定のブラウザで画面を開いていない'

  echo '== Scenario: 最後に Tailscale の転送先を示し、Tailscale の設定は変えない'
  grep -Fq 'tailscale serve --bg --https=443 http://127.0.0.1:18700' "$trace" || die 'Tailscale の転送先の案内が無い'
  ! grep -Eiq '^result=ok .*tailscale' "$trace" || die 'Tailscale の命令を走らせた'
  ! grep -Eiq '^result=ok .*(docker|wsl)' "$trace" || die '命令の記録に Docker か WSL がある'

  echo '== Scenario: 空きが見積もりより少なければ知らせ、インストールは続ける'
  ASHIATO_SETUP_TEST_FREE_GB=10 install_with '' install >"$out" 2>&1 || { cat "$out" >&2; die '空きが少ないとインストールが止まった'; }
  grep -Fq '空きは 10 GB で、見積もりは 40 GB' "$out" || die '空きと見積もりの案内が無い'

  echo '== Scenario: 管理者に上がらなければ決め直せない'
  local settings_sum
  settings_sum="$(sha256sum <"$SETTINGS")"
  if printf '%s\n%s\n' "$PW2" "$PW2" | ASHIATO_SETUP_TEST_ELEVATION=deny "$SETUP" password >"$out" 2>&1; then
    die '管理者の確認を断っても決め直せた'
  fi
  [[ "$(sha256sum <"$SETTINGS")" == "$settings_sum" ]] || die '管理者の確認を断ったのに設定が変わった'

  echo '== Scenario: DB の起動が遅れても、サーバのサービスは待ってから起きる'
  stop_service ashiato-server
  cp "$SETTINGS" "$WORK/settings.saved"
  with_settings_write set_setting ASHIATO_SERVICE_TEST_DB_DELAY_SEC 20
  local started=$SECONDS
  start_service ashiato-server
  wait_healthz 150
  [[ $((SECONDS - started)) -ge 15 ]] || die 'DB を待たずにサーバが起きた'
  ! grep -RFq 'database_unavailable' "$STATE/logs/server" || die 'DB を待つ間に諦めた'

  echo '== Scenario: DB が 120 秒開かなければ、理由を書いて終わる'
  stop_service ashiato-server
  with_settings_write set_setting ASHIATO_SERVICE_TEST_DB_DELAY_SEC never
  start_service ashiato-server
  wait_state ashiato-server 1 200
  grep -RFq 'kind=database_unavailable' "$STATE/logs/server" || die 'DB が開かない理由がサーバのログに無い'
  with_settings_write cp "$WORK/settings.saved" "$SETTINGS"
  start_service ashiato-server; wait_healthz 150

  if [[ "${ASHIATO_INSTALL_CHECK_TEST_TAKEOUT:-0}" == 1 ]]; then
    echo '== Scenario: ダウンロードに置いた Takeout が、インストールした形で取り込まれる'
    local archive_name='takeout-20261103T000000Z-001.zip'
    create_takeout_archive "$WORK/$archive_name"
    cp "$WORK/$archive_name" "$DOWNLOADS_U/$archive_name"
    [[ "$(app_sql "UPDATE core.source SET fetch_interval_sec = 300 WHERE logical_source = 's01-archive-inbox' RETURNING 1")" == 1 ]] ||
      die '取り込み器の見回りの間隔を短くできない（登録簿に s01-archive-inbox が 1 行ない）'
    stop_service ashiato-server
    start_service ashiato-server; wait_healthz 150
    local deadline=$((SECONDS + 600))
    until [[ "$(app_sql "select count(*) from core.archive_ledger where file_name = '$archive_name'")" -ge 1 ]]; do
      [[ "$SECONDS" -lt "$deadline" ]] || die 'ダウンロードの Takeout が 10 分で取り込まれない'
      sleep 5
    done
    ! grep -RFq 'archive_disabled' "$STATE/logs/server" || die '取り込み器が止まっている'
    rm -f "$DOWNLOADS_U/$archive_name"
  fi

  echo '== Scenario: repair の途中で落ちても、もう一度 repair できる'
  with_settings_write mv "$SETTINGS" "$WORK/settings.away"
  if ASHIATO_SETUP_TEST_STOP_AT=repair-roles install_with "$PW1" repair >"$out" 2>&1; then die 'repair の役割の段の停止で進んだ'; fi
  install_with "$PW1" repair >"$out" 2>&1 || { cat "$out" >&2; die '途中で落ちた後の repair が失敗'; }
  [[ ! -e "$STATE/settings/repair.pg_hba.conf" && ! -e "$STATE/settings/server.json.repair" ]] && ! compgen -G "$STATE/settings/.owner-staging-*" >/dev/null ||
    die 'repair の一時のファイルが残った'
  wait_healthz 150
  [[ -n "$(login "$PW1")" ]] || die 'もう一度の repair の後にログインできない'

  echo '== Scenario: 移行が失敗すると、サーバのサービスを開始せずに止まる'
  clean_machine
  if ASHIATO_SETUP_TEST_MIGRATE_FAIL=1 install_with "$PW1" install >"$out" 2>&1; then die '移行の失敗でインストールが進んだ'; fi
  grep -Fq '移行に失敗した' "$out" || die '移行の失敗の理由が出ない'
  [[ "$(state_code ashiato-server)" != 4 ]] || die '移行の失敗の後にサーバのサービスが動いている'
  [[ "$(http_code "$URL/healthz")" != 200 ]] || die '移行の失敗の後にサーバが答える'

  clean_machine
  echo 'OK: 試験の zip の実地の確かめ'
}

create_takeout_archive() {
  local archive="$1" fixture="$root/tools/fixtures/takeout/shape.txt"
  if command -v zip >/dev/null 2>&1; then
    (cd "$(dirname "$fixture")" && zip -q -j "$archive" "$(basename "$fixture")")
  elif command -v powershell.exe >/dev/null 2>&1; then
    local ps_archive="$archive" ps_fixture="$fixture"
    if command -v cygpath >/dev/null 2>&1; then
      ps_archive="$(cygpath -w "$archive")"
      ps_fixture="$(cygpath -w "$fixture")"
    fi
    powershell.exe -NoProfile -NonInteractive -Command \
      "Compress-Archive -LiteralPath '$ps_fixture' -DestinationPath '$ps_archive' -Force"
  elif command -v pwsh >/dev/null 2>&1; then
    pwsh -NoProfile -NonInteractive -Command \
      "Compress-Archive -LiteralPath '$fixture' -DestinationPath '$archive' -Force"
  else
    die 'Takeout の fixture を zip にする道具が無い（zip / PowerShell）'
  fi
}

check_zip() {
  local archive="$1"
  if [[ "$archive" == *package-windows-test* ]]; then
    check_test_zip "$archive"
  else
    check_release_zip "$archive"
  fi
}

# ============================================================================
# <<< 実地の確かめ
# ============================================================================

# 窓の試験が化けを見分けられるかを先に確かめる（陰性の対照。conpty-run.ps1 -Probe）
console_probe() {
  local ps script work
  # Python に日本語を出させる（windows-latest の Git Bash は cp1252 で落ちる）
  export PYTHONUTF8=1 PYTHONIOENCODING=utf-8
  if command -v wslpath >/dev/null 2>&1; then
    ps=/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe
    [[ -x "$ps" ]] || { echo 'Windows の PowerShell が無い' >&2; exit 2; }
    script="$(wslpath -w "$root/tools/release/conpty-run.ps1")"
  elif command -v powershell.exe >/dev/null 2>&1; then
    ps=powershell.exe
    script="$(cygpath -w "$root/tools/release/conpty-run.ps1")"
  else
    echo 'Windows の PowerShell が無い' >&2; exit 2
  fi
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-console.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  local outw="$work/probe"
  command -v wslpath >/dev/null 2>&1 && outw="$(wslpath -w "$work")\\probe" || outw="$(cygpath -w "$work")\\probe"
  local ps_out ps_rc=0
  ps_out="$("$ps" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$script" -Probe -Out "$outw" -TimeoutSec 90 2>&1)" || ps_rc=$?
  if [[ $ps_rc != 0 ]]; then
    printf '%s\n' "$ps_out" >&2
    die "窓の試験が化けを見分けられない: 台本が rc=$ps_rc で終わった（90=CreatePseudoConsole 失敗・91=CreateProcess 失敗は上の hr=/err=。それ以外は時間切れか台本の書き方）"
  fi
  PY - "$(pypath "$work/probe.raw.txt")" "$(pypath "$work/probe.console.txt")" <<'PY'
import re, sys
vt = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)")
def read(p):
    try:
        return vt.sub("", open(p, "rb").read().decode("utf-8", errors="replace"))
    except OSError:
        sys.exit("窓の試験が化けを見分けられない: 出力のファイルが無い " + p)
raw, con = read(sys.argv[1]), read(sys.argv[2])
want = "画面の合言葉"
if "CONSOLE-OK" not in raw or "CONSOLE-OK" not in con:
    sys.exit("窓の試験が化けを見分けられない: 子が窓につながっていない")
if want not in con:
    sys.exit("窓の試験が化けを見分けられない: .console.txt に画面の合言葉が無い")
if want in raw:
    sys.exit("窓の試験が化けを見分けられない: .raw.txt が化けていない")
m = re.search(r"(\d{3,5})\s*$", next((l for l in raw.splitlines() if re.search(r":\s*\d{3,5}\s*$", l)), ""))
if not m:
    sys.exit("窓の試験が化けを見分けられない: chcp の返したコード頁が読めない")
if m.group(1) == "65001":
    sys.exit("窓の試験が化けを見分けられない: 窓が UTF-8 のまま")
mangled = (want + ": ").encode("utf-8").decode("cp" + m.group(1), errors="replace")
run = ""
for ch in mangled:
    run = run + ch if ch != "�" else ""
    if len(run) >= 3:
        break
else:
    run = ""
if len(run) < 3 or run[:3] not in raw:
    sys.exit("窓の試験が化けを見分けられない: .raw.txt に化けた並びが無い")
print("OK: 窓の試験は化けを見分けられる（コード頁 " + m.group(1) + "）")
PY
}

# Linux で走る自己試験。本物の Windows は無いので、(1) 確かめの部品が合う出力で通り 1 つずつ外れた出力で
# 落ちること、(2) --list の全部の Scenario が実地の確かめ（CI が走らせる経路）に 1 回ずつ印を持ち、
# 自己試験の側には印が無いこと、(3) Windows でないと実地の確かめが走らないこと、を見る。
self_test() {
  local work
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-install-self-test.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  expect_fail() { if ( "$@" ) >/dev/null 2>&1; then die "外れた出力で通った: $*"; fi; }

  local -a blocked_remote_ips
  mapfile -t blocked_remote_ips < <(outbound_block_remote_ips)
  [[ "${blocked_remote_ips[*]}" == '0.0.0.0-126.255.255.255,128.0.0.0-255.255.255.255 ::2-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff' ]] ||
    die '外向き遮断が loopback（127.0.0.0/8 と ::1）だけを残していない'

  local listener
  start_port_listener 55400
  listener="$PORT_LISTENER_PID"
  kill "$listener"
  wait "$listener" 2>/dev/null || true
  ! kill -0 "$listener" 2>/dev/null || die '番号を塞ぐ試験の Python が止まらない'
  PY -c 'import socket,sys; s=socket.socket(); s.bind(("127.0.0.1",int(sys.argv[1])))' 55400 ||
    die '番号を塞ぐ試験の Python が番号を手放さない'

  # (1) 確かめの部品
  # Git Bash の PATH にある同名コマンドではなく、Windows 本体の実行ファイルを選ぶ。
  # Windows を持たない自己試験でも、WINDIR の System32 を選び、引数をそのまま渡すことを固定する。
  local fake_windows whoami_output
  fake_windows="$work/windows"
  mkdir -p "$fake_windows/System32"
  cat >"$fake_windows/System32/whoami.exe" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@"
EOF
  chmod +x "$fake_windows/System32/whoami.exe"
  whoami_output="$(
    cygpath() { [[ "$1" == -u ]] && printf '%s\n' "$2"; }
    WINDIR="$fake_windows" windows_whoami /groups
  )"
  [[ "$whoami_output" == /groups ]] || die 'Windows 本体の whoami.exe を選べない'
  cat >"$fake_windows/System32/net.exe" <<'EOF'
#!/usr/bin/env bash
printf '%s ' "$@"
EOF
  chmod +x "$fake_windows/System32/net.exe"
  whoami_output="$(
    cygpath() { [[ "$1" == -u ]] && printf '%s\n' "$2"; }
    WINDIR="$fake_windows" windows_net user test /add
  )"
  [[ "$whoami_output" == 'user test /add ' ]] || die 'Windows 本体の net.exe を選べない'
  cat >"$fake_windows/System32/netsh.exe" <<'EOF'
#!/usr/bin/env bash
printf '<%s>\n' "$@"
if [[ -n "${NETSH_LOG:-}" ]]; then printf '<%s>\n' "$@" >>"$NETSH_LOG"; fi
if [[ "${NETSH_FAIL_RULE:-}" == "${!#}" ]]; then exit 1; fi
EOF
  chmod +x "$fake_windows/System32/netsh.exe"
  whoami_output="$(
    cygpath() { [[ "$1" == -u ]] && printf '%s\n' "$2"; }
    WINDIR="$fake_windows" block_outbound_program ashiato-install-check-1 'C:\Program Files\ashiato\ashiato-server.exe' '0.0.0.0-126.255.255.255,128.0.0.0-255.255.255.255'
  )"
  [[ "$whoami_output" == $'<advfirewall>\n<firewall>\n<add>\n<rule>\n<name=ashiato-install-check-1>\n<dir=out>\n<action=block>\n<program=C:\\Program Files\\ashiato\\ashiato-server.exe>\n<remoteip=0.0.0.0-126.255.255.255,128.0.0.0-255.255.255.255>\n<enable=yes>' ]] ||
    die '外向き遮断の netsh.exe の引数が違う'
  : >"$work/netsh.log"
  (
    cygpath() { [[ "$1" == -u ]] && printf '%s\n' "$2"; }
    WINDIR="$fake_windows" NETSH_LOG="$work/netsh.log" clear_outbound_block_rules 2
  ) >/dev/null
  whoami_output="$(<"$work/netsh.log")"
  [[ "$whoami_output" == $'<advfirewall>\n<firewall>\n<delete>\n<rule>\n<name=ashiato-install-check-1>\n<advfirewall>\n<firewall>\n<delete>\n<rule>\n<name=ashiato-install-check-2>' ]] ||
    die '外向き遮断の規則を残さず外せない'
  if (
    cygpath() { [[ "$1" == -u ]] && printf '%s\n' "$2"; }
    WINDIR="$fake_windows" NETSH_FAIL_RULE='name=ashiato-install-check-2' clear_outbound_block_rules 2
  ) >/dev/null 2>&1; then
    die '外向き遮断の規則を外せないのに成功にした'
  fi

  [[ "$(service_sid TrustedInstaller)" == 'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464' ]] ||
    die 'サービスの SID の計算が sc.exe showsid と違う'
  local acl ok_lines
  acl=$'C:\\ProgramData\\ashiato NT SERVICE\\ashiato-server:(OI)(CI)(M)\n                       PC\\runner:(OI)(CI)(RX)\n\nSuccessfully processed 1 files; Failed processing 0 files'
  ok_lines="$(acl_lines "$acl" 'C:\ProgramData\ashiato')"
  [[ "$(acl_perm "$ok_lines" 'nt service\ASHIATO-SERVER')" == '(OI)(CI)(M)' ]] || die 'ACL の主体を読めない'
  assert_acl_only "$ok_lines" t 'NT SERVICE\ashiato-server' 'pc\runner'
  assert_acl_not_inherited "$ok_lines" t
  assert_read_only_perm "$(acl_perm "$ok_lines" 'PC\runner')" t
  expect_fail assert_acl_only "$ok_lines" t 'NT SERVICE\ashiato-server'
  expect_fail assert_acl_only "$(acl_lines "$acl"$'\n BUILTIN\\Users:(OI)(CI)(RX)' 'C:\ProgramData\ashiato')" t 'NT SERVICE\ashiato-server' 'PC\runner'
  expect_fail assert_acl_not_inherited "$(acl_lines $'C:\\x PC\\runner:(I)(OI)(CI)(RX)' 'C:\x')" t
  expect_fail assert_read_only_perm '(OI)(CI)(M)' t
  expect_fail assert_read_only_perm '' t
  assert_modify_perm '(OI)(CI)(M)' t
  assert_modify_perm '(I)(M)' t
  expect_fail assert_modify_perm '(OI)(CI)(RX)' t

  local qc=$'SERVICE_NAME: ashiato-db\r\n        START_TYPE         : 2   AUTO_START\r\n        SERVICE_START_NAME : NT SERVICE\\ashiato-db\r\n'
  assert_service_config "$qc" ashiato-db
  expect_fail assert_service_config "${qc/2   AUTO_START/3   DEMAND_START}" ashiato-db
  expect_fail assert_service_config "${qc/NT SERVICE\\ashiato-db/LocalSystem}" ashiato-db
  assert_restart_on_failure $'FAILURE_ACTIONS              : RESTART -- Delay = 10000 milliseconds.' ashiato-db
  expect_fail assert_restart_on_failure 'FAILURE_ACTIONS              :' ashiato-db
  [[ "$(service_state_of $'        STATE              : 4  RUNNING\r\n')" == 4 ]] || die '状態の番号を読めない'
  [[ "$(service_state_of $'        STATE              : 1  STOPPED')" == 1 ]] || die '止まった状態の番号を読めない'
  [[ "$(postmaster_pid_from $'4242\r\nC:/ProgramData/ashiato/db/17\r\n')" == 4242 ]] ||
    die 'postmaster.pid の親プロセス番号を読めない'
  expect_fail postmaster_pid_from $'not-a-pid\r\nC:/ProgramData/ashiato/db/17\r\n'
  assert_db_watch_acl '(D:(A;;LCRP;;;S-1-5-80-server))' S-1-5-80-server
  expect_fail assert_db_watch_acl '(D:(A;;LCRP;;;S-1-5-80-server)(A;;WP;;;S-1-5-80-server))' S-1-5-80-server
  expect_fail assert_db_watch_acl '(D:(A;;LCRPWP;;;S-1-5-80-server))' S-1-5-80-server
  expect_fail assert_db_watch_acl '(D:(A;;LCRP;;;S-1-5-80-other))S:(A;;WP;;;S-1-5-80-server)' S-1-5-80-server
  local netstat=$'  TCP    127.0.0.1:55400        0.0.0.0:0              LISTENING       4242'
  assert_loopback_listener_only "$netstat" 55400
  expect_fail assert_loopback_listener_only "$netstat"$'\n  TCP    0.0.0.0:55400          0.0.0.0:0              LISTENING       4242' 55400
  expect_fail assert_loopback_listener_only '  TCP    127.0.0.1:5432  0.0.0.0:0  LISTENING  1' 55400

  assert_no_trust $'# trust はコメントだけ\nhost all all 127.0.0.1/32 scram-sha-256'
  expect_fail assert_no_trust $'host all all 127.0.0.1/32 trust'

  assert_no_secret_in 'kind=web_session_key reason=too_short' t 'secret-value-0123456789'
  expect_fail assert_no_secret_in 'leaked secret-value-0123456789' t 'secret-value-0123456789'

  # 合言葉の条件の行（管の出力）
  local req='画面の合言葉を決めます。16 文字以上・change-me で始めない・自動で作った API の合言葉やログインの印の鍵と同じにしない'
  printf '%s\n' "$req" '受け付けない: 短い（16 文字以上）' >"$work/req.good"
  assert_requirements_line "$work/req.good" t
  printf '%s\n' '受け付けない: 短い' "$req" >"$work/req.late"
  expect_fail assert_requirements_line "$work/req.late" t
  printf '%s\n' "$req" "$req" >"$work/req.twice"
  expect_fail assert_requirements_line "$work/req.twice" t
  printf '%s\n' "$req。入れた文字は表示されません" >"$work/req.tty"
  expect_fail assert_requirements_line "$work/req.tty" t
  printf '%s\n' '16 文字以上だけ' >"$work/req.none"
  expect_fail assert_requirements_line "$work/req.none" t

  [[ "$(session_cookie_from_headers $'HTTP/1.1 204 No Content\r\nset-cookie: ashiato_session=abc_DEF-1; HttpOnly; Secure\r\n')" == abc_DEF-1 ]] ||
    die 'ログインの印を見出しから読めない'
  [[ -z "$(session_cookie_from_headers $'HTTP/1.1 401 Unauthorized\r\n')" ]] || die '401 で印を読んだ'

  mkdir -p "$work/state/settings"
  assert_no_pwfile "$work/state"
  touch "$work/state/settings/.initdb.pwfile"
  expect_fail assert_no_pwfile "$work/state"
  rm -f "$work/state/settings/.initdb.pwfile"; touch "$work/state/.pwfile"
  expect_fail assert_no_pwfile "$work/state"
  rm -f "$work/state/.pwfile"; mkdir "$work/state/settings/.initdb-staging-1-2"
  expect_fail assert_no_pwfile "$work/state"
  rmdir "$work/state/settings/.initdb-staging-1-2"

  # GitHub Actions の Windows runner には zip が無い。PowerShell の経路でも、
  # Takeout の fixture を実際に読める zip として作れることを確かめる。
  local fallback_bin fallback_archive
  fallback_bin="$work/no-zip-bin"
  fallback_archive="$work/takeout.zip"
  mkdir -p "$fallback_bin"
  cat >"$fallback_bin/powershell.exe" <<'EOF'
#!/bin/bash
set -euo pipefail
command="${!#}"
[[ "$command" =~ -LiteralPath\ \'([^\']+)\'.*-DestinationPath\ \'([^\']+)\' ]] || exit 2
fixture="${BASH_REMATCH[1]}"
archive="${BASH_REMATCH[2]}"
/usr/bin/python3 - "$fixture" "$archive" <<'PY'
import sys
import zipfile
from pathlib import Path

fixture, archive = map(Path, sys.argv[1:])
with zipfile.ZipFile(archive, "w") as output:
    output.write(fixture, fixture.name)
PY
EOF
  chmod +x "$fallback_bin/powershell.exe"
  PATH="$fallback_bin" create_takeout_archive "$fallback_archive"
  [[ "$(/usr/bin/unzip -Z1 "$fallback_archive")" == 'shape.txt' ]] ||
    die 'zip が無い Windows で Takeout の fixture を zip にできない'

  # 管理者トークンでは所有者の書込拒否を確かめられない。実地の確かめが、初回の所有者にした
  # 非管理者の利用者として書込を試す経路を持つことを固定する。
  local owner_runner='run_as_other'
  grep -Fq "$owner_runner \"Set-Content -LiteralPath" "$0" ||
    die '本人の書込拒否を非管理者の所有者として確かめる経路が無い'

  # PostgreSQL 18 の datlocprovider は "char" なので、文字列との連結では text に明示する。
  # そうしないと Windows の実地検査で演算子を選べず、並べ方を確かめる前に止まる。
  local release_check
  release_check="$(awk '/^check_release_zip\(\)/,/^self_test\(\)/' "$0")"
  grep -Fq "datlocprovider::text || '|'" <<<"$release_check" ||
    die 'DB の並べ方の照合が datlocprovider を text に明示していない'

  # (2) Scenario の印の置き場（CI の comm -23 が構造的に欠けないこと。D24）
  local script="$0" body marks name count
  body="$(awk '/^# >>> 実地の確かめ/{on=1} /^# <<< 実地の確かめ/{on=0} on' "$script")"
  local tag='Scenario'
  marks="$(sed -n "s/^[[:space:]]*echo '== ${tag}: \(.*\)'\$/\1/p" <<<"$body")"
  while IFS= read -r name; do
    count="$(grep -Fxc -- "$name" <<<"$marks" || true)"
    [[ "$count" == 1 ]] || die "Scenario の印が実地の確かめに 1 つでない（$count）: $name"
  done < <(list_scenarios)
  while IFS= read -r name; do
    list_scenarios | grep -Fxq -- "$name" || die "--list に無い Scenario の印: $name"
  done <<<"$marks"
  ! awk '/^self_test\(\)/{on=1} on' "$script" | grep -Eq "^[[:space:]]*echo ['\"]== Scenario:" ||
    die '自己試験の側に Scenario の印がある（印は実地の確かめにだけ置く）'

  # (3) Windows でないと実地の確かめは走らない
  touch "$work/package-windows-test.zip"
  if (check_zip "$work/package-windows-test.zip") >/dev/null 2>&1; then die 'Windows でないのに実地の確かめが走った'; fi

  rm -rf "$work"
  trap - EXIT
  echo 'OK: installer の確かめの部品と Scenario の印の置き場を確認した'
}

[[ "${1:-}" == --self-test ]] && { self_test; exit 0; }
[[ "${1:-}" == --list && $# == 1 ]] && { list_scenarios; exit 0; }
[[ "${1:-}" == --probe-console && $# == 1 ]] && { console_probe; exit 0; }
[[ "${1:-}" == --check-coverage && $# == 2 ]] && { check_coverage "$2"; exit 0; }
[[ $# == 1 ]] || usage
check_zip "$1"
