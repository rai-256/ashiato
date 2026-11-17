#!/usr/bin/env bash
# 判定が落ちたら観測を必ずログに出す部品（CH16）と、出さずに NG で終わる道を落とす検査。
#
#   . tools/release/check-ng-observations.sh      # source: 観測を出す部品を定義する（設計 D1〜D4）
#   tools/release/check-ng-observations.sh <台本>… # 直に走らせる: 台本を静的に見る（設計 D5）
#   tools/release/check-ng-observations.sh --self-test
#
# source されたときは関数を定義するだけで、呼び手のシェルの設定（set -e など）には触れない。
# 台本が渡す名前（読むのは呼ぶ時。source 時には読まない）:
#   NG_STATE_DIR      状態の置き場（サーバのログは logs/server の最新）
#   NG_SETTINGS_FILES 設定のファイル（改行区切り。場所に空白があってよい。キーの名前だけ出す）
#   NG_HEALTHZ_URL    サーバの healthz
#   NG_OBSERVED_DIR   命令の記録（observed.log）と、判定のために書く出力のファイルの置き場
#   NG_OBSERVE_TIMEOUT 観測を取る命令 1 つの時間の上限（秒。既定 30）
#   ng_secret_values  台本が定義する関数。伏せる値を 1 行ずつ出す。設定が「無い」ときは字面の値だけで rc=0、
#                     「あるのに読めない」ときだけ rc≠0。設定の秘密は部品の ng_settings_secrets で読む（読み手を台本に写さない）

# ---------------------------------------------------------------------------
# 部品（source されたとき使う）
# ---------------------------------------------------------------------------

# 伏せる値の置き場を NG_SF に置く。NG_OBSERVED_DIR があればそこ（権限を絞る）。無ければ呼び出しの一時の置き場 $1
ng_secret_file() {
  if [[ -n "${NG_OBSERVED_DIR:-}" && -d "${NG_OBSERVED_DIR:-}" ]]; then
    NG_SF="$NG_OBSERVED_DIR/.ng-secrets"
  else
    NG_SF="$1/.ng-secrets"
  fi
}

# ファイル $1 の中身を変数 $2 へ（fork しない）。無ければ空
ng_slurp() {
  local -n ng_slurp_to="$2"
  ng_slurp_to=''
  [[ ! -f "$1" ]] || IFS= read -r -d '' ng_slurp_to <"$1" || true
}

# 伏せる値を集めて NG_SF に書き足す（一度集めた値は消さない）。rc≠0 は「伏せる値を読めない」。
# 命令の記録のたびに呼ばれるので、集めた値が前の回と同じなら書き足さない（fork を減らす）
ng_collect_secrets() {
  local vals='' rc=0 sf="${NG_SF:-}" last
  [[ -n "$sf" ]] || return 1
  if declare -F ng_secret_values >/dev/null 2>&1; then
    vals="$(ng_secret_values 2>/dev/null)" || rc=$?
  else
    rc=1
  fi
  if [[ -f "$sf" ]]; then
    ng_slurp "$sf.vals" last
    [[ ! -f "$sf.vals" || "$last" != "$vals" ]] || return "$rc"
  fi
  (
    umask 077
    {
      [[ ! -f "$sf" ]] || cat "$sf"
      printf '%s\n' "$vals" | tr -d '\r'
    } | awk 'length($0) > 0 && !seen[$0]++' >"$sf.new" 2>/dev/null && mv -f "$sf.new" "$sf" &&
      printf '%s' "$vals" >"$sf.vals"
  ) 2>/dev/null || rc=1
  return "$rc"
}

# インストーラが設定に書く秘密のキー（観測で伏せる）。_URL で終わるキーは接続の合言葉を伏せる
NG_SERVER_SECRET_KEYS=(API_TOKEN WEB_SESSION_KEY WEB_PASSWORD_VERIFIER DATABASE_URL)
NG_OWNER_SECRET_KEYS=(DATABASE_OWNER_URL)

ng_read_secrets_py() {
  local py=python
  command -v python >/dev/null 2>&1 || py=python3
  "$py" -c '
import json, sys, urllib.parse
d = json.load(open(sys.argv[1], encoding="utf-8"))
for k in sys.argv[2:]:
    v = str(d.get(k, "") or "")
    if k.endswith("_URL"):
        v = urllib.parse.unquote(urllib.parse.urlparse(v).password or "")
    print(v)' "$@" | tr -d '\r'
  return "${PIPESTATUS[0]}"
}

# 設定のファイル $1 の、キー（$2 以降）の秘密を 1 行ずつ出す。読めない・形が読めないときは rc≠0。
# 命令の記録のたびに呼ばれるので、NG_OBSERVED_DIR があれば、ファイルの中身とキーが前と同じ間は前の答えを使う
# （Python を起こさない。中身で比べるので、同じ時刻に書き換えても古い答えを使わない。比べるのは bash の read で fork しない）
ng_read_secrets() {
  local c want src val
  if [[ -z "${NG_OBSERVED_DIR:-}" || ! -d "${NG_OBSERVED_DIR:-}" || ! -r "$1" ]]; then
    ng_read_secrets_py "$@"
    return
  fi
  c="$NG_OBSERVED_DIR/.ng-read-${2:-none}"
  ng_slurp "$1" want
  want="$*"$'\n'"$want"
  ng_slurp "$c.src" src
  if [[ ! -f "$c.val" || ! -f "$c.src" || "$src" != "$want" ]]; then
    rm -f "$c.src"
    ( umask 077; ng_read_secrets_py "$@" >"$c.val" ) || { rm -f "$c.val"; return 1; }
    ( umask 077; printf '%s' "$want" >"$c.src" ) || true
  fi
  ng_slurp "$c.val" val
  printf '%s' "$val"
}

# インストーラの 2 つの設定（$1 = server.json・$2 = owner.json）の秘密を 1 行ずつ出す。無いファイルは飛ばす（rc=0）。
# あるのに読めない・形が読めないときだけ rc≠0
ng_settings_secrets() {
  [[ ! -e "$1" ]] || ng_read_secrets "$1" "${NG_SERVER_SECRET_KEYS[@]}" || return 1
  [[ ! -e "$2" ]] || ng_read_secrets "$2" "${NG_OWNER_SECRET_KEYS[@]}" || return 1
  return 0
}

# 標準入力を、伏せる値（NG_SF）と見出しの形（Authorization:・ashiato_session=）を *** にして標準出力へ
ng_redact() {
  local q="'"
  if [[ -f "${NG_SF:-}" ]]; then
    NG_SF="$NG_SF" awk '
      BEGIN { n = 0; sf = ENVIRON["NG_SF"]; while ((getline v < sf) > 0) if (length(v) > 0) vals[++n] = v; close(sf) }
      # 先に全部の値の出現（重なりも）の位置に印を付け、印の続く所をまとめて *** にする
      # （値が別の値の部分文字列でも、前後が重なっても、残りを素で出さない）
      { line = $0; L = length(line); split("", m)
        for (i = 1; i <= n; i++) {
          v = vals[i]; lv = length(v); s = 1
          while (s <= L && (p = index(substr(line, s), v)) > 0) { q = s + p - 1; for (k = q; k < q + lv; k++) m[k] = 1; s = q + 1 }
        }
        out = ""; k = 1
        while (k <= L) {
          if (k in m) { out = out "***"; while (k <= L && (k in m)) k++ }
          else { j = k; while (j <= L && !(j in m)) j++; out = out substr(line, k, j - k); k = j }
        }
        print out }'
  else
    cat
  fi | sed -E \
    -e 's/([Aa][Uu][Tt][Hh][Oo][Rr][Ii][Zz][Aa][Tt][Ii][Oo][Nn]:[[:space:]]*)((Bearer|Basic|Token|Digest)[[:space:]]+)?[^[:space:]"'"$q"']+/\1\2***/g' \
    -e 's/(ashiato_session=)[^[:space:];"'"$q"']+/\1***/g'
}

# 観測を取る命令を 1 つ、時間の上限つきで走らせ、出力を（CR を除いて）出す。落ちたら 1 行書く
ng_try() {
  local rc=0 t="${NG_TMP:-}"
  echo "-- $*"
  if command -v timeout >/dev/null 2>&1; then
    timeout -k 1 "${NG_OBSERVE_TIMEOUT:-30}" "$@" >"$t/try" 2>&1 </dev/null || rc=$?
  else
    # 包んだ関数（sc.exe など）でなく実体を呼ぶ（塊を出す最中に記録を足して、判定の記録を押し出さない）
    command "$@" >"$t/try" 2>&1 </dev/null || rc=$?
  fi
  tr -d '\r' <"$t/try"
  (( rc == 0 )) || echo "（取れない: $1 rc=$rc）"
  return 0
}

ng_part_services() {
  local svc
  for svc in ashiato-db ashiato-server ashiato-updater; do
    ng_try sc.exe queryex "$svc"
    ng_try sc.exe qc "$svc"
  done
}

ng_part_settings() {
  local f keys
  while IFS= read -r f; do
    [[ -n "$f" ]] || continue
    if [[ ! -e "$f" ]]; then echo "設定 $f: まだ無い"; continue; fi
    if [[ ! -r "$f" ]]; then echo "（取れない: 設定の読み取り $f）"; continue; fi
    keys="$(grep -oE '"[^"]+"[[:space:]]*:' "$f" 2>/dev/null | sed -E 's/^"([^"]+)".*/\1/' | tr '\n' ' ')"
    echo "設定のキー $f: $keys"
  done <<<"${NG_SETTINGS_FILES:-}"
}

ng_part_files() {
  local d="${NG_OBSERVED_DIR:-}" f
  [[ -d "$d" ]] || return 0
  while IFS= read -r f; do
    [[ -n "$f" ]] || continue
    echo "-- $f（末尾 40 行）"
    tail -n 40 "$d/$f" | tr -d '\r'
  done < <(cd "$d" && ls -t out ./*.out trace ./*.window ./redirect.*.txt 2>/dev/null | sed 's|^\./||' | awk '!s[$0]++' | head -n 2)
}

ng_part_serverlog() {
  local latest
  latest="$(ls -t "${NG_STATE_DIR:-}/logs/server"/* 2>/dev/null | head -n 1)"
  [[ -n "$latest" ]] || { echo '-- サーバのログ: 無い'; return 0; }
  echo "-- サーバのログ $latest（末尾 40 行）"
  tail -n 40 "$latest" | tr -d '\r'
}

# 命令の記録（observed.log.1 と observed.log）の直近 12 件（1 件 40 行まで）。記録の後の「@@x <n>」は直前の記録の回数
ng_part_records() {
  local d="${NG_OBSERVED_DIR:-}" pend=1
  local log="$d/observed.log"
  [[ -s "$log" ]] || return 0
  [[ ! -f "$d/.observed.dup" ]] || pend="$(<"$d/.observed.dup")"
  echo '-- 命令の記録（直近 12 件）'
  {
    [[ ! -f "$log.1" ]] || cat "$log.1"
    cat "$log"
    if [[ "$pend" =~ ^[0-9]+$ ]] && (( pend > 1 )); then printf '@@x %s\n' "$pend"; fi
  } | tr -d '\r' | awk -v max=12 -v per=40 '
    /^@@@ / { n++; head[n] = substr($0, 5); cnt[n] = 0; om[n] = 0; dup[n] = 1; next }
    n > 0 && /^@@x [0-9]+$/ { dup[n] = substr($0, 5) + 0; next }
    n > 0 { if (cnt[n] < per) { cnt[n]++; body[n, cnt[n]] = $0 } else om[n]++ }
    END {
      s = n - max + 1; if (s < 1) s = 1
      for (i = s; i <= n; i++) {
        h = head[i]
        if (match(h, / rc=[0-9]+$/)) h = substr(h, 1, RSTART - 1) " (rc=" substr(h, RSTART + 4) ")"
        print "[命令] " h
        for (k = 1; k <= cnt[i]; k++) print body[i, k]
        if (om[i] > 0) print "（以降 " om[i] " 行省略）"
        if (dup[i] > 1) print "（同じ記録 ×" dup[i] "）"
      }
    }'
}

ng_part_health() {
  ng_try curl -s -o /dev/null -w 'healthz HTTP %{http_code}\n' --max-time 10 "$NG_HEALTHZ_URL"
}

# 観測の塊を標準出力へ。ng_report が部分殻で呼ぶ。$2 は判定が見たファイル（末尾 80 行を、伏せる値を読めるときだけ出す）
ng_emit() {
  local obs="${1:-}" tf="${2:-}" ok=1
  set +e +u +o pipefail
  NG_TMP="$(mktemp -d "${TMPDIR:-/tmp}/ng-obs.XXXXXX")" || NG_TMP=''
  if [[ -z "$NG_TMP" ]]; then
    printf '== 観測（ここから）\n（取れない: mktemp rc=1）\n== 観測（ここまで）\n'
    return 0
  fi
  ng_secret_file "$NG_TMP"
  ng_collect_secrets || ok=0
  {
    # 判定が見た値も 80 行まで（netstat の全出力などで塊を読めなくしない）
    if [[ -n "$obs" ]]; then
      printf '%s\n' "$obs" | awk 'NR <= 80 { print; next } { om++ } END { if (om > 0) print "（以降 " om " 行省略）" }'
    fi
    if [[ -n "${NG_STATE_DIR:-}" ]]; then ng_part_services; fi
    if [[ -n "${NG_HEALTHZ_URL:-}" ]]; then ng_part_health; fi
    if [[ -n "${NG_SETTINGS_FILES:-}" ]]; then ng_part_settings; fi
    if (( ok )); then
      if [[ -n "$tf" && -f "$tf" ]]; then echo "-- $tf（末尾 80 行）"; tail -n 80 "$tf" | tr -d '\r'; fi
      ng_part_records
      ng_part_files
      if [[ -n "${NG_STATE_DIR:-}" ]]; then ng_part_serverlog; fi
    else
      echo '（伏せる値を読めないので出さない）'
    fi
  } >"$NG_TMP/block" 2>&1
  echo '== 観測（ここから）'
  ng_redact <"$NG_TMP/block"
  echo '== 観測（ここまで）'
  rm -rf "$NG_TMP"
  return 0
}

# NG の行と観測の塊を fd（既定は標準エラー）へ出して戻る。exit しない
#   ng_report <理由> [<判定が見た値>] [<fd>] [<判定が見たファイル>]
# 判定が見た値は伏せる値を読めなくても出す（値を選んで渡すのは判定）。ファイルの中身は値に入れず第 4 引数で渡す
# （伏せる値を読めないときは部品が出さない）
ng_report() {
  local reason="${1:-}" obs="${2:-}" fd="${3:-2}" tf="${4:-}"
  {
    printf 'NG: %s\n' "$reason"
    ( ng_emit "$obs" "$tf" ) || true
  } >&"$fd" || true
  return 0
}
die() { ng_report "$*"; exit 1; }
die_seen() { ng_report "${1:-}" "${2:-}"; exit 1; }

# 観測の命令を記録して呼ぶ口。$1 は記録に書く名前、$2 は呼ぶ実体（名前でも System32 の下の場所でもよい）
# 標準出力・標準エラー・終了コードは直に呼んだときと同じ（命令はパイプに書かない）
ng_observed_exec() {
  local name="$1" exe="$2" t rc=0
  shift 2
  if [[ -z "${NG_OBSERVED_DIR:-}" || ! -d "${NG_OBSERVED_DIR:-}" || -n "${NG_REC_BUSY:-}" ]]; then
    command "$exe" "$@"
    return
  fi
  t="$(mktemp -d "$NG_OBSERVED_DIR/.cap.XXXXXX" 2>/dev/null)" || { command "$exe" "$@"; return; }
  command "$exe" "$@" >"$t/o" 2>"$t/e" || rc=$?
  ( NG_REC_BUSY=1; ng_record "$t" "$name $*" "$rc" ) 2>/dev/null || true
  # 書き先のパイプが閉じていても失敗にしない。cat は外部命令なので SIGPIPE はこの関数を殺さない
  cat "$t/o" || true
  cat "$t/e" >&2 || true
  # 一時の置き場を消せなくても（Defender などがつかむ）呼び手の set -e を止めない
  rm -rf "$t" || true
  return "$rc"
}
ng_observed() { local n="$1"; shift; ng_observed_exec "$n" "$n" "$@"; }

# 記録を 1 件書く。書くときにも伏せる。直前と同じなら書き足さず、回数だけ .observed.dup に数える（記録を読み直さない・
# 書き直さない）。記録は NG_RECORDS_PER_FILE 件ごとに observed.log.1 へ送る（残るのは多くてその 2 倍。出すのは直近 12 件）
NG_RECORDS_PER_FILE=24
ng_record() {
  local t="$1" cmdline="$2" rc="$3" d="$NG_OBSERVED_DIR"
  local log="$d/observed.log" last="$d/.observed.last" dup="$d/.observed.dup" num="$d/.observed.n" n=0 c=1
  set +e +u +o pipefail
  ng_secret_file "$t"
  ng_collect_secrets || true
  {
    printf '@@@ %s rc=%s\n' "$(printf '%s' "$cmdline" | tr '\n' ' ')" "$rc"
    # 出力の行が @ で始まるなら空白を 1 つ前に置く（記録の区切り @@@・@@x と取り違えない）
    head -n 200 "$t/o" | sed 's/^@/ @/'
    if [[ -s "$t/e" ]]; then echo '@@ stderr'; head -n 200 "$t/e" | sed 's/^@/ @/'; fi
  } 2>/dev/null | ng_redact >"$t/rec"
  if [[ -f "$last" && -s "$log" && "$(<"$last")" == "$(<"$t/rec")" ]]; then
    [[ ! -f "$dup" ]] || c="$(<"$dup")"
    [[ "$c" =~ ^[0-9]+$ ]] || c=1
    printf '%s' "$((c + 1))" >"$dup"
    return 0
  fi
  (
    umask 077
    if [[ -f "$dup" ]]; then
      c="$(<"$dup")"
      if [[ "$c" =~ ^[0-9]+$ ]] && (( c > 1 )); then printf '@@x %s\n' "$c" >>"$log"; fi
      rm -f "$dup"
    fi
    [[ ! -f "$num" ]] || n="$(<"$num")"
    [[ "$n" =~ ^[0-9]+$ ]] || n=0
    if (( n >= NG_RECORDS_PER_FILE )); then mv -f "$log" "$log.1"; n=0; fi
    cat "$t/rec" >>"$log"
    cp "$t/rec" "$last"
    printf '%s' "$((n + 1))" >"$num"
  )
  return 0
}

sc.exe() { ng_observed sc.exe "$@"; }
icacls.exe() { ng_observed icacls.exe "$@"; }
netstat.exe() { ng_observed netstat.exe "$@"; }
netsh.exe() { ng_observed netsh.exe "$@"; }
reg.exe() { ng_observed reg.exe "$@"; }
curl() { ng_observed curl "$@"; }

# ---------------------------------------------------------------------------
# 検査（直に走らせたとき）
# ---------------------------------------------------------------------------

# 台本を 1 つ静的に見る。外れた行を出力して、1 つでもあれば rc=1
ng_check_file() {
  local f="$1"
  [[ -r "$f" ]] || { echo "NG: 読めない台本: $f" >&2; return 1; }
  awk -v file="$f" '
    function bad(rule, name, text) { printf "%s:%d: 決まり %d（%s）: %s\n", file, NR, rule, name, text; found = 1 }
    # 引用の中身を空白にする（ヒアドキュメントの開きを探すため）。<< の直後の引用（区切りの語）は残す
    function unquoted(s,   out, i, ch, q) {
      out = ""; q = ""
      for (i = 1; i <= length(s); i++) {
        ch = substr(s, i, 1)
        if (q != "") { if (ch == q) q = ""; else if (ch == "\\" && q == "\"") i++; continue }
        if ((ch == "\"" || ch == "\047") && out !~ /<<-?[[:space:]]*$/) { q = ch; out = out " "; continue }
        out = out ch
      }
      return out
    }
    BEGIN {
      N[1] = "NG の行は観測を出す口だけが出す"
      N[2] = "0 でない exit は観測を出す口と使い方を示す口の中だけ"
      N[3] = "System32 の観測の命令は記録する口の中だけで呼ぶ"
      N[4] = "観測を出す部品を読み込み、口を定義し直さない"
      N[5] = "理由の印には理由を書く"
      found = 0; sourced = 0; heredoc = ""; dash = 0; fn = ""
    }
    {
      line = $0
      if (heredoc != "") {
        t = line; if (dash) sub(/^\t+/, "", t)
        if (t == heredoc) heredoc = ""
        next
      }
      code = line; sub(/^[[:space:]]+/, "", code)
      if (code ~ /^#/) next
      marked = 0
      if (match(line, /#[[:space:]]*ng-observations:/)) {
        reason = substr(line, RSTART + RLENGTH); gsub(/^[[:space:]]+|[[:space:]]+$/, "", reason)
        marked = 1
        if (reason == "") bad(5, N[5], line)
      }
      c = code; sub(/[[:space:]]#([[:space:]].*)?$/, "", c)

      # 関数の中か（先頭の列の name() { … } から先頭の列の } まで。1 行で閉じるものはその行だけ）
      cur = fn
      if (c ~ /^(function[[:space:]]+)?[A-Za-z_][A-Za-z0-9_.-]*[[:space:]]*\(\)[[:space:]]*\{/ || c ~ /^function[[:space:]]+[A-Za-z_][A-Za-z0-9_.-]*[[:space:]]*\{/) {
        nm = c; sub(/^function[[:space:]]+/, "", nm); sub(/[[:space:](\{].*$/, "", nm)
        cur = nm
        o = gsub(/\{/, "{", c); cl = gsub(/\}/, "}", c)
        fn = (o > cl) ? nm : ""
      } else if (line ~ /^\}/) {
        fn = ""
      }

      if (!marked) {
        if (c ~ /(^|[;&|({[:space:]])(echo|printf)[[:space:]]/ && c ~ /NG:/) bad(1, N[1], line)
        # 字面の 1〜9 と、変数・$? の終了コード（"$rc"・$?）。引用した字面（"1" など）も。exit 0 と引数の無い exit は見ない
        if (c ~ /(^|[;&|({[:space:]])exit[[:space:]]+["\047]?(\$|[1-9])/ && cur != "usage") bad(2, N[2], line)
        if (c ~ /[Ss]ystem32[\\\/]+(sc|icacls|netstat|netsh|reg|curl)(\.exe)?([^A-Za-z0-9_]|$)/ && c !~ /ng_observed/) bad(3, N[3], line)
      }
      if (c ~ /(^|[;&|({[:space:]])(source|\.)[[:space:]]+[^#]*check-ng-observations\.sh/) sourced = 1
      if (c ~ /^(function[[:space:]]+)?(die|ng_report|die_seen)[[:space:]]*\(\)/ || c ~ /^function[[:space:]]+(die|ng_report|die_seen)([[:space:]{]|$)/) bad(4, N[4], line)

      # ヒアドキュメントの開き。この行の判定は済んでいる。コメント（c で除いた）・引用の中・算術の << は開きでない
      hc = unquoted(c); gsub(/\$?\(\([^)]*\)\)/, " ", hc)
      if (match(hc, /(^|[^<])<<-?[[:space:]]*["\047]?[A-Za-z_][A-Za-z0-9_]*/)) {
        h = substr(hc, RSTART, RLENGTH); sub(/^[^<]*<</, "", h)
        dash = (h ~ /^-/); sub(/^-/, "", h); gsub(/^[[:space:]]+|["\047]/, "", h)
        heredoc = h
      }
    }
    END {
      if (!sourced) { printf "%s:1: 決まり 4（%s）: check-ng-observations.sh を読み込む行が無い\n", file, N[4]; found = 1 }
      exit found
    }' "$f"
}

ng_check() {
  local f rc=0
  [[ $# -gt 0 ]] || { echo '使い方: check-ng-observations.sh <台本>… | --self-test' >&2; return 2; }
  for f in "$@"; do ng_check_file "$f" || rc=1; done
  return "$rc"
}

# ---------------------------------------------------------------------------
# 自己試験
# ---------------------------------------------------------------------------

ng_self_test() {
  local part d bin nobin t rc se so i
  part="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
  d="$(mktemp -d "${TMPDIR:-/tmp}/ch16-ngobs.XXXXXX")"
  trap "rm -rf '$d'" EXIT
  bin="$d/bin"; mkdir -p "$bin"
  fail() { echo "NG: $*" >&2; exit 1; }

  # ---- 検査（決まり 1〜4・印）----
  local src='. "$root/tools/release/check-ng-observations.sh"'
  mkcopy() { printf '%s\n' '#!/usr/bin/env bash' "$src" "$@" >"$d/copy.sh"; }
  runcheck() { rc=0; "$part" "$d/copy.sh" >"$d/chk.out" 2>&1 || rc=$?; }

  mkcopy 'f() { :; }'
  runcheck
  [[ $rc == 0 ]] || fail "外れの無い写しが落ちた: $(cat "$d/chk.out")"

  # Scenario: 観測の口の外で NG の行を出す台本を、検査が落とす
  mkcopy 'f() { echo "NG: x" >&2; }'
  runcheck
  [[ $rc == 1 ]] && grep -q 'copy.sh:3: 決まり 1' "$d/chk.out" && ! grep -Eq '決まり [234]' "$d/chk.out" ||
    fail "決まり 1 だけに外れる写しを落とさない: rc=$rc $(cat "$d/chk.out")"

  # Scenario: 観測の口の外で 0 でない exit をする台本を、検査が落とす
  mkcopy 'f() { exit 3; }'
  runcheck
  [[ $rc == 1 ]] && grep -q 'copy.sh:3: 決まり 2' "$d/chk.out" && ! grep -Eq '決まり [134]' "$d/chk.out" ||
    fail "決まり 2 だけに外れる写しを落とさない: rc=$rc $(cat "$d/chk.out")"
  # 変数・$? の終了コードの exit も決まり 2 に外れる
  for t in 'f() { local rc=3; exit "$rc"; }' 'f || exit $?' 'f() { exit $rc; }' 'f() { exit "1"; }' "f() { exit '2'; }"; do
    mkcopy "$t"
    runcheck
    [[ $rc == 1 ]] && grep -q 'copy.sh:3: 決まり 2' "$d/chk.out" || fail "変数の exit を落とさない: $t"
  done
  mkcopy 'f() { exit 0; }' 'g() { exit; }'
  runcheck
  [[ $rc == 0 ]] || fail "exit 0 と引数の無い exit を落とした: $(cat "$d/chk.out")"
  mkcopy 'usage() { echo "使い方" >&2; exit 2; }'
  runcheck
  [[ $rc == 0 ]] || fail "usage の中の exit 2 を通さない: $(cat "$d/chk.out")"

  # Scenario: 記録しない口で観測の命令を呼ぶ台本を、検査が落とす
  mkcopy 'f() { "$windir/System32/sc.exe" query x; }'
  runcheck
  [[ $rc == 1 ]] && grep -q 'copy.sh:3: 決まり 3' "$d/chk.out" && ! grep -Eq '決まり [124]' "$d/chk.out" ||
    fail "決まり 3 だけに外れる写しを落とさない: rc=$rc $(cat "$d/chk.out")"
  mkcopy 'windows_sc() { ng_observed_exec sc.exe "$windir/System32/sc.exe" "$@"; }'
  runcheck
  [[ $rc == 0 ]] || fail "記録する口の中の System32 の呼び出しを通さない: $(cat "$d/chk.out")"
  mkcopy 'windows_netsh() { ng_observed_exec netsh.exe "$windir/System32/netsh.exe" "$@"; }'
  runcheck
  [[ $rc == 0 ]] || fail "記録を通す netsh を落とした: $(cat "$d/chk.out")"
  mkcopy 'f() { "$windir/System32/netsh.exe" advfirewall; }'
  runcheck
  [[ $rc == 1 ]] && grep -q 'copy.sh:3: 決まり 3' "$d/chk.out" || fail "netsh の直呼びを落とさない: rc=$rc"
  mkcopy 'f() { "$windir\System32\sc.exe" query x; }'
  runcheck
  [[ $rc == 1 ]] && grep -q 'copy.sh:3: 決まり 3' "$d/chk.out" || fail "System32\\ の形の直呼びを落とさない: rc=$rc"

  # Scenario: 観測を出す部品を読まない・口を定義し直す台本を、検査が落とす
  printf '%s\n' '#!/usr/bin/env bash' 'f() { :; }' >"$d/copy.sh"
  runcheck
  [[ $rc == 1 ]] && grep -q '決まり 4' "$d/chk.out" && ! grep -Eq '決まり [123]' "$d/chk.out" ||
    fail "部品を読まない写しを落とさない: rc=$rc $(cat "$d/chk.out")"
  mkcopy 'die() { echo "x" >&2; }'
  runcheck
  [[ $rc == 1 ]] && grep -q 'copy.sh:3: 決まり 4' "$d/chk.out" && ! grep -Eq '決まり [123]' "$d/chk.out" ||
    fail "口を定義し直す写しを落とさない: rc=$rc $(cat "$d/chk.out")"

  # Scenario: 理由の印を付けた行は除き、理由の無い印は外れにする
  mkcopy 'exit 2 # ng-observations: 使い方の誤り'
  runcheck
  [[ $rc == 0 ]] || fail "理由つきの印の行を除かない: $(cat "$d/chk.out")"
  mkcopy 'exit 2 # ng-observations:'
  runcheck
  [[ $rc == 1 ]] || fail "理由の無い印を外れにしない"

  # ヒアドキュメントの中は見ない
  mkcopy 'cat <<'"'"'EOF'"'"'' 'echo "NG: x"' 'exit 3' 'EOF'
  runcheck
  [[ $rc == 0 ]] || fail "ヒアドキュメントの中を見た: $(cat "$d/chk.out")"
  mkcopy 'cat <<-"EOF" # 区切りの後のコメント' 'exit 3' '	EOF' 'cat <<<"NG: 文字列"'
  runcheck
  [[ $rc == 0 ]] || fail "引用の区切り・タブを除く区切りのヒアドキュメントの中を見た: $(cat "$d/chk.out")"
  # コメント・引用の中・算術の << はヒアドキュメントの開きでない（後ろの行を検査から外さない）
  for t in 'sc_query x # 出力は <<EOF の形' 'echo "a <<EOF b"' 'mask=$((1<<n))' 'echo '"'"'<<EOF'"'"''; do
    mkcopy "$t" 'f() { exit 3; }'
    runcheck
    [[ $rc == 1 ]] && grep -q 'copy.sh:4: 決まり 2' "$d/chk.out" || fail "ヒアドキュメントでない << の後ろの行を見ない: $t"
  done

  # ---- 部品 ----
  # 偽の命令は置き場の bin に置く。run_case が PATH の先に足して、子の bash で台本を走らせる
  fake() { printf '#!/usr/bin/env bash\n%s\n' "$2" >"$bin/$1"; chmod +x "$bin/$1"; }
  local work="$d/work" state="$d/state" settings="$d/state/settings.json"
  mkdir -p "$work" "$state/logs/server"
  printf 'server-log-line-1\nserver-log-line-2\n' >"$state/logs/server/server.log"
  printf '{"api_secret":"SECRET-API-1","db_password":"SECRET-DB-1"}\n' >"$settings"
  run_case() { # run_case <body> : 子の bash で body を走らせ、rc・so・se を取る
    printf '%s\n' 'set -euo pipefail' ". \"$part\"" "$1" >"$d/case.sh"
    rc=0
    env PATH="${CASE_PATH:-$bin:$PATH}" NG_STATE_DIR="$state" NG_SETTINGS_FILES="$settings" NG_HEALTHZ_URL='http://127.0.0.1:1/healthz' \
      NG_OBSERVED_DIR="$work" "${CASE_ENV[@]}" bash "$d/case.sh" >"$d/so" 2>"$d/se" || rc=$?
  }
  CASE_ENV=(X_UNUSED=1)
  local sv='ng_secret_values() { if [[ -f "'"$settings"'" ]]; then grep -oE ":\"[^\"]+\"" "'"$settings"'" | tr -d ":\""; fi; echo PW-SCREEN-1; }'
  reset_work() { rm -rf "$work"; mkdir -p "$work"; }

  # Scenario: 観測の命令を記録しても、判定が見る出力は変わらない
  reset_work
  fake sc.exe 'printf "out1\r\nout2\r\n"; printf "err1\r\n" >&2; exit 3'
  run_case 'rc1=0; sc.exe query >"'"$d"'/w.o" 2>"'"$d"'/w.e" || rc1=$?
rc2=0; command sc.exe query >"'"$d"'/d.o" 2>"'"$d"'/d.e" || rc2=$?
[[ $rc1 == 3 && $rc2 == 3 ]]'
  [[ $rc == 0 ]] || fail "包んだ呼び出しの終了コードが命令のものでない: $(cat "$d/se")"
  cmp -s "$d/w.o" "$d/d.o" || fail "包んだ呼び出しの標準出力が直と違う"
  cmp -s "$d/w.e" "$d/d.e" || fail "包んだ呼び出しの標準エラーが直と違う"
  [[ -s "$work/observed.log" ]] || fail "命令の記録ができない"

  # Scenario: 記録する命令の出力をパイプが途中で閉じても、終了コードは命令のものになる
  reset_work
  fake sc.exe 'echo FIRSTLINE; head -c 200000 /dev/zero | tr "\0" x; echo; exit 0'
  run_case 'rc1=0; sc.exe query | grep -q "^FIRSTLINE$" || rc1=$?
[[ $rc1 == 0 ]]'
  [[ $rc == 0 ]] || fail "パイプが閉じたとき終了コードが命令のものでない: rc=$rc $(cat "$d/se")"
  [[ ! -s "$d/se" ]] || fail "パイプが閉じたとき標準エラーに出る: $(cat "$d/se")"

  # Scenario: 観測を取る命令が無い・落ちても、NG の行と終了コードは変わらず、取れなかったことを書く
  reset_work
  nobin="$d/nobin"; mkdir -p "$nobin"
  for t in bash env grep sed awk tail head cat mktemp timeout tr date rm ls mv cp sort mkdir cmp; do
    command -v "$t" >/dev/null || fail "自己試験に要る道具が無い: $t"
    ln -sf "$(command -v "$t")" "$nobin/$t"
  done
  CASE_PATH="$nobin"
  run_case "$sv"'
die "理由そのもの"'
  unset CASE_PATH
  [[ $rc == 1 ]] || fail "命令が無いとき終了コードが 1 でない: $rc"
  [[ "$(head -n 1 "$d/se")" == 'NG: 理由そのもの' ]] || fail "命令が無いとき最初の行が NG でない: $(head -n 1 "$d/se")"
  grep -q '（取れない: sc.exe rc=' "$d/se" || fail "sc.exe が無いことを書かない"
  grep -q '（取れない: curl rc=' "$d/se" || fail "curl が無いことを書かない"
  fake sc.exe 'exit 5'
  run_case "$sv"'
die "理由そのもの"'
  [[ $rc == 1 && "$(head -n 1 "$d/se")" == 'NG: 理由そのもの' ]] || fail "sc.exe が落ちたとき NG の行か終了コードが変わる: rc=$rc"
  grep -q '（取れない: sc.exe rc=5）' "$d/se" || fail "sc.exe が落ちたことを書かない"
  [[ "$(grep -c '^== 観測' "$d/se")" == 2 ]] || fail "観測の塊の見出しが 2 行でない"
  [[ ! -s "$d/so" ]] || fail "標準出力に観測が出る"

  # Scenario: 観測を取る命令が戻らなくても、時間の上限で打ち切って NG で終わる
  fake sc.exe 'exec sleep 600'
  CASE_ENV=(NG_OBSERVE_TIMEOUT=1)
  SECONDS=0
  run_case "$sv"'
die "戻らない"'
  CASE_ENV=(X_UNUSED=1)
  (( SECONDS < 30 )) || fail "時間の上限で打ち切らない: ${SECONDS} 秒"
  [[ $rc == 1 ]] && grep -q '（取れない: sc.exe rc=' "$d/se" || fail "戻らない命令で NG の行か取れなかった行が無い: rc=$rc"

  # Scenario: 設定に無い秘密も、見出しの形で伏せる
  reset_work
  fake sc.exe 'echo "Authorization: Bearer HDR-OUT-SECRET"; echo "Cookie: ashiato_session=COOKIE-OUT-SECRET"'
  run_case "$sv"'
curl -H "Authorization: Bearer HDR-ARG-SECRET" -b "ashiato_session=COOKIE-ARG-SECRET" http://127.0.0.1:1/x || true # gitleaks:allow
sc.exe query
die_seen "理由" "Authorization: Bearer HDR-SEEN-SECRET"'
  [[ $rc == 1 ]] || fail "見出しの形の試験が rc=1 で終わらない: $rc"
  for t in HDR-OUT-SECRET COOKIE-OUT-SECRET HDR-ARG-SECRET COOKIE-ARG-SECRET HDR-SEEN-SECRET; do
    ! grep -q -- "$t" "$d/se" "$work/observed.log" || fail "見出しの形の秘密が残る: $t"
  done
  grep -q 'Authorization: Bearer \*\*\*' "$d/se" || fail "Authorization の値を *** にしない"

  # Scenario: 一度伏せた値は、設定が変わった後も伏せ続ける
  reset_work
  printf '{"api_secret":"SECRET-API-1","db_password":"SECRET-DB-1"}\n' >"$settings"
  fake sc.exe 'echo "got SECRET-API-1 and SECRET-DB-1"'
  fake icacls.exe 'echo "new SECRET-API-2"'
  run_case "$sv"'
sc.exe query
printf "{\"api_secret\":\"SECRET-API-2\"}\n" >"'"$settings"'"
icacls.exe x
echo "tail SECRET-API-1" >"'"$work"'/out"
die "設定が変わった後"'
  [[ $rc == 1 ]] || fail "設定を変える試験が rc=1 で終わらない: $rc"
  ! grep -q 'SECRET-API-1\|SECRET-DB-1' "$d/se" "$work/observed.log" || fail "前の値が標準エラーか記録に残る"
  grep -q 'got \*\*\* and \*\*\*' "$d/se" || fail "命令の記録が出ない: $(cat "$d/se")"
  grep -q 'api_secret' "$d/se" || fail "設定のキーの名前が出ない"
  # 変えた後の値も集め直して伏せる（前の回と同じ値なら集め直しを飛ばすが、変われば飛ばさない）
  ! grep -q 'SECRET-API-2' "$d/se" "$work/observed.log" && grep -q '^new \*\*\*$' "$d/se" || fail "変えた後の設定の値が出る"

  # Scenario: 待ちのループの同じ記録は 1 件にまとまり、判定が見た出力を押し出さない
  reset_work
  printf '{"api_secret":"SECRET-API-1"}\n' >"$settings"
  fake sc.exe 'echo "SAME-OUT"'
  fake icacls.exe 'echo "OTHER-OUT"'
  run_case "$sv"'
for i in $(seq 30); do sc.exe query; done
icacls.exe x
die "待った後"'
  [[ $rc == 1 ]] && grep -q 'SAME-OUT' "$d/se" && grep -q 'OTHER-OUT' "$d/se" && grep -q '（同じ記録 ×30）' "$d/se" ||
    fail "同じ記録を 1 件にまとめない: rc=$rc $(cat "$d/se")"
  [[ "$(grep -c '^\[命令\] sc.exe query' "$d/se")" == 1 ]] || fail "同じ記録が複数件出る"

  # Scenario: 設定のファイルがまだ無くても、命令の記録は出す
  reset_work
  rm -f "$settings"
  fake icacls.exe 'echo "REC-OUT"'
  run_case "$sv"'
icacls.exe x
die "設定が無い"'
  [[ $rc == 1 ]] && grep -q 'REC-OUT' "$d/se" && ! grep -q '（伏せる値を読めないので出さない）' "$d/se" ||
    fail "設定が無いとき命令の記録を出さない: rc=$rc $(cat "$d/se")"

  # Scenario: 伏せる値を読めなければ、命令の記録とファイルの末尾を出さない
  reset_work
  printf '{"api_secret":"SECRET-API-1"}\n' >"$settings"
  fake icacls.exe 'echo "REC-OUT"'
  run_case 'ng_secret_values() { return 1; }
icacls.exe x
echo "FILE-TAIL-LINE" >"'"$work"'/out"
die "読めない"'
  [[ $rc == 1 && "$(head -n 1 "$d/se")" == 'NG: 読めない' ]] || fail "伏せる値を読めないとき NG の行か終了コードが変わる: rc=$rc"
  grep -q '（伏せる値を読めないので出さない）' "$d/se" || fail "出さない理由の行が無い"
  ! grep -q 'REC-OUT\|FILE-TAIL-LINE\|server-log-line' "$d/se" || fail "読めないのに記録かファイルの末尾を出す"
  run_case 'icacls.exe x
die "定義が無い"'
  grep -q '（伏せる値を読めないので出さない）' "$d/se" && ! grep -q 'REC-OUT' "$d/se" || fail "ng_secret_values が無いとき記録を出す"

  # 判定が渡したファイルの末尾は、伏せる値を読めるときだけ出す（値に入れて渡すと読めなくても出てしまう）
  reset_work
  printf '{"api_secret":"SECRET-API-1"}\n' >"$settings"
  printf 'TAIL-FILE-LINE SECRET-API-1\n' >"$d/tail.in"
  run_case "$sv"'
ng_report "ファイルつき" "" 2 "'"$d"'/tail.in"; exit 1'
  grep -q '^TAIL-FILE-LINE \*\*\*$' "$d/se" || fail "判定が渡したファイルの末尾を伏せて出さない: $(cat "$d/se")"
  run_case 'ng_secret_values() { return 1; }
ng_report "ファイルつき" "" 2 "'"$d"'/tail.in"; exit 1'
  ! grep -q 'TAIL-FILE-LINE' "$d/se" && grep -q '（伏せる値を読めないので出さない）' "$d/se" ||
    fail "伏せる値を読めないのに判定が渡したファイルの末尾を出す"

  # 観測の量の上限: 記録は直近 12 件・1 件 40 行・出力のファイルは 2 つ・サーバのログは末尾 40 行。命令 1 つの時間の既定は 30 秒
  reset_work
  fake sc.exe 'if [[ "$2" == long ]]; then seq 41 | sed "s/^/LONG-/"; else echo "REC-$2"; fi'
  for t in a b c; do echo "FILE-$t" >"$work/$t.out"; done
  seq 41 | sed 's/^/SLOG-/' >"$state/logs/server/zz-latest.log"
  run_case "$sv"'
for i in $(seq 13); do sc.exe query "n$i"; done
sc.exe query long; sc.exe query long
die "上限"'
  rm -f "$state/logs/server/zz-latest.log"
  [[ $rc == 1 ]] || fail "上限の試験が rc=1 で終わらない: $rc"
  ! grep -qx 'REC-n2' "$d/se" && grep -qx 'REC-n3' "$d/se" || fail "命令の記録が直近 12 件でない: $(cat "$d/se")"
  grep -qx 'LONG-40' "$d/se" && ! grep -qx 'LONG-41' "$d/se" && grep -q '（以降 1 行省略）' "$d/se" || fail "1 件の記録が 40 行で切れない"
  grep -q '（同じ記録 ×2）' "$d/se" || fail "最後の記録の回数が出ない"
  [[ "$(grep -c '^-- [a-z]*\.out（末尾 40 行）$' "$d/se")" == 2 ]] || fail "出力のファイルが 2 つでない"
  ! grep -qx 'SLOG-1' "$d/se" && grep -qx 'SLOG-2' "$d/se" || fail "サーバのログが末尾 40 行でない"
  awk '/^ng_try\(\)/,/^}/' "$part" | grep -q 'NG_OBSERVE_TIMEOUT:-30}' || fail "観測の命令の時間の既定が 30 秒でない"

  # 記録は 24 件ごとに送り、残るのは多くて 48 件（待ちのループが長くても記録が育ち続けない）
  reset_work
  fake sc.exe 'echo "REC-$2"'
  run_case 'for i in $(seq 60); do sc.exe query "m$i"; done'
  [[ -f "$work/observed.log.1" ]] || fail "記録を送らない"
  (( $(cat "$work/observed.log.1" "$work/observed.log" | grep -c '^@@@ ') <= 48 )) || fail "記録が 48 件を超えて残る"
  run_case "$sv"'
die "送った後"'
  grep -qx 'REC-m60' "$d/se" && grep -qx 'REC-m49' "$d/se" && ! grep -qx 'REC-m48' "$d/se" || fail "送った後に直近 12 件が出ない"

  # 設定の秘密の読み手は、中身が同じ間は Python を起こし直さない。中身が変われば（同じ秒・同じ長さでも）読み直す
  reset_work
  local py3; py3="$(command -v python3)"
  printf '#!/usr/bin/env bash\necho x >>"%s/pycount"\nexec "%s" "$@"\n' "$d" "$py3" >"$bin/python"; chmod +x "$bin/python"
  rm -f "$d/pycount"
  printf '{"API_TOKEN":"tok-a-1"}\n' >"$settings"
  run_case 'a="$(ng_read_secrets "'"$settings"'" API_TOKEN)"; b="$(ng_read_secrets "'"$settings"'" API_TOKEN)"
printf "{\"API_TOKEN\":\"tok-b-1\"}\n" >"'"$settings"'"
c="$(ng_read_secrets "'"$settings"'" API_TOKEN)"
[[ $a == tok-a-1 && $b == tok-a-1 && $c == tok-b-1 ]]'
  rm -f "$bin/python"
  [[ $rc == 0 ]] || fail "設定の秘密の読み手が中身の変化に追いつかない: $(cat "$d/se")"
  [[ "$(wc -l <"$d/pycount")" == 2 ]] || fail "中身が同じなのに Python を起こし直す: $(wc -l <"$d/pycount") 回"

  # 設定のファイルの場所に空白があっても 1 つの場所として読む（改行区切り）
  reset_work
  mkdir -p "$d/state dir"
  printf '{"api_secret":"SECRET-API-1"}\n' >"$d/state dir/settings.json"
  CASE_ENV=(NG_SETTINGS_FILES="$d/state dir/settings.json"$'\n'"$d/state dir/none.json")
  run_case "$sv"'
die "空白"'
  CASE_ENV=(X_UNUSED=1)
  grep -qF "設定のキー $d/state dir/settings.json: api_secret" "$d/se" && grep -qF "設定 $d/state dir/none.json: まだ無い" "$d/se" ||
    fail "空白を含む設定の場所を 1 つとして読まない: $(cat "$d/se")"

  # 包んだ命令の一時の置き場を消せなくても（偽の rm が落ちる）、set -e の台本は包んだ命令の後の文へ進む
  reset_work
  fake sc.exe 'echo OUT'
  fake rm 'exit 1'
  run_case 'sc.exe query >/dev/null; echo survived'
  /bin/rm -f "$bin/rm"
  [[ $rc == 0 ]] && grep -qx survived "$d/so" || fail "一時の置き場を消せないと台本が止まる: rc=$rc $(cat "$d/se")"

  # 伏せる値が別の伏せる値の部分文字列でも、長い方を全部伏せる
  reset_work
  fake sc.exe 'echo "val=abcdef-LONGSECRET"'
  run_case 'ng_secret_values() { printf "%s\n" abc abcdef-LONGSECRET; }
sc.exe query
die "部分文字列"'
  ! grep -q 'LONGSECRET' "$d/se" "$work/observed.log" && grep -qx 'val=\*\*\*' "$d/se" ||
    fail "部分文字列の組で長い方の残りが出る: $(grep 'val=' "$d/se")"
  reset_work
  fake sc.exe 'echo "val=abcXdef"'
  run_case 'ng_secret_values() { printf "%s\n" abcX Xdef; }
sc.exe query
die "前後が重なる"'
  ! grep -q 'val=.*def' "$d/se" "$work/observed.log" && grep -qx 'val=\*\*\*' "$d/se" ||
    fail "前後が重なる組で残りが出る: $(grep 'val=' "$d/se")"

  # timeout が無いときも、塊を出す最中の命令（sc.exe など）は記録に足さない（判定の記録を押し出さない）
  reset_work
  mkdir -p "$d/nto"
  for t in bash env grep sed awk tail head cat mktemp tr date rm ls mv cp sort mkdir cmp seq; do
    command -v "$t" >/dev/null || fail "自己試験に要る道具が無い: $t"
    ln -sf "$(command -v "$t")" "$d/nto/$t"
  done
  fake sc.exe 'echo "REC-$2"'
  CASE_PATH="$bin:$d/nto"
  run_case "$sv"'
sc.exe query only
die "timeout が無い"'
  unset CASE_PATH
  [[ $rc == 1 ]] && grep -q '^-- sc.exe queryex ashiato-db$' "$d/se" || fail "timeout が無いとき塊を出さない: rc=$rc $(cat "$d/se")"
  [[ "$(grep -c '^@@@ ' "$work/observed.log")" == 1 ]] || fail "timeout が無いとき塊を出す命令が記録に足される: $(grep '^@@@ ' "$work/observed.log")"

  # 命令の出力にある @@@ で始まる行を、記録の区切りと取り違えない
  reset_work
  fake sc.exe 'echo "@@@ fake header"; echo "@@x 9"; echo "after"'
  run_case "$sv"'
sc.exe query
die "区切りの形"'
  ! grep -q '^\[命令\] fake header' "$d/se" && grep -qx ' @@@ fake header' "$d/se" && grep -qx 'after' "$d/se" &&
    ! grep -q '（同じ記録 ×9）' "$d/se" || fail "出力の @@@ の行を記録の区切りと取り違える: $(cat "$d/se")"

  # 判定が見た値も 80 行で切り、切ったことを示す
  reset_work
  run_case "$sv"'
die_seen "長い値" "$(seq 85 | sed "s/^/SEEN-/")"'
  grep -qx 'SEEN-80' "$d/se" && ! grep -qx 'SEEN-81' "$d/se" && grep -q '（以降 5 行省略）' "$d/se" ||
    fail "判定が見た値が 80 行で切れない"

  echo 'check-ng-observations: self-test OK'
}

# ---------------------------------------------------------------------------
# 入口。source されたときは何もしない
# ---------------------------------------------------------------------------
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  set -euo pipefail
  case "${1:-}" in
    --self-test) ng_self_test ;;
    *) ng_check "$@" ;;
  esac
fi
