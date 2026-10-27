#!/usr/bin/env bash
# 作業ツリーが消えた docker のコンテナとネットワークを片付ける（CH05 Q7）。
#
#   tools/docker-tidy.sh                    片付ける（外したものを 1 つごとに標準エラーへ 1 行）
#   tools/docker-tidy.sh --dry-run          外すものを同じ形で出すだけで、何も外さない
#   tools/docker-tidy.sh --self-test        偽の docker と、main を乱数の名前に替えた本物の docker 1 件で確かめる
#   tools/docker-tidy.sh --verify-env-check 検証の準備（tools/verify-env.sh）への組み込みを確かめる（tasks 3.2）
#
# 外す条件（design D4）:
#   コンテナ: compose の project と working_dir の印を持つ / project が `<main の名前>-` で始まる / working_dir の親のフォルダが main と同じ /
#             working_dir がファイルシステムに無く、`git worktree list` にも無い / working_dir が main でも `-verify` で終わる木でもない。
#             動いていても、条件を満たせば止めて外す。`docker rm -f <id>` のみ（`-v` を付けない）。
#   ネットワーク: compose の project の印を持つ / project が `<main の名前>-` で始まる / 繋がったコンテナ（止まっているものも。
#             `docker ps -aq --filter network=`）とその project の印のコンテナが、この回で外すもののほかに無い /
#             生きている作業ツリーの名前（小文字）と project が同じでなく、`-verify` でない。
# **volume は消さない**（名前の無い volume はコンテナを外した後も残る）。`down` も volume を消す命令も打たない。
# 並んだ片付けで、もう無いもの（No such container / No such network）は外せたものとして扱う。docker に問えない・外せないときは理由を出して rc=1。
#
# 差し替え（--self-test と --verify-env-check が使う。tools/verify-env.sh から呼ばれても環境からそのまま届く）:
#   TEST_DOCKER_TIDY_DOCKER     docker を呼ぶ命令（既定 docker）
#   TEST_DOCKER_TIDY_MAIN       main の checkout の path（既定 git worktree list の最初の行）
#   TEST_DOCKER_TIDY_WORKTREES  生きている作業ツリーの path の一覧（1 行 1 path のファイル。既定 git worktree list）
#
# 検証の準備（tools/verify-env.sh）が作り直す（`--plan` が reset）ときだけ、DB を消す前に呼ぶ（design D5）。
# なぜ要るか: 消えた作業ツリーの DB のコンテナとネットワークが残り、docker のネットワークの割り当てを使い切る（CH05 Q7）。
set -uo pipefail
cd "$(dirname "$0")/.."

# dq <変数名> <docker の引数…>: 成れば標準出力だけを、落ちれば標準エラーを変数へ入れ、docker の rc を返す
# （docker の警告（標準エラー）を一覧の行として読まない。review/code.md R24）。tidy の中から呼ぶ（docker の命令は tidy の局所）
dq() {
  local _v=$1 _e _o _rc=0
  shift
  _e=$(mktemp) || return 1
  _o=$("${docker[@]}" "$@" 2>"$_e") || _rc=$?
  if ((_rc)); then printf -v "$_v" %s "$(<"$_e")"; else printf -v "$_v" %s "$_o"; fi
  rm -f "$_e"
  return $_rc
}

tidy() {
  local dry=$1
  local -a docker
  read -ra docker <<<"${TEST_DOCKER_TIDY_DOCKER:-docker}"

  local -a wt=()
  local main
  if [[ -n ${TEST_DOCKER_TIDY_WORKTREES:-} ]]; then
    mapfile -t wt <"$TEST_DOCKER_TIDY_WORKTREES"
  else
    local l
    while IFS= read -r l; do [[ $l == "worktree "* ]] && wt+=("${l#worktree }"); done \
      < <(git worktree list --porcelain 2>/dev/null)
  fi
  main=${TEST_DOCKER_TIDY_MAIN:-${wt[0]:-}}
  if [[ -z $main ]]; then
    echo "docker-tidy: main の checkout が分からない（git worktree list が空）" >&2
    return 1
  fi
  local main_name=${main##*/} main_parent=${main%/*}
  main_name=${main_name,,}

  # 生きている作業ツリーの名前（compose の project の名前の決め方: basename を小文字にする）
  local -A live_name=()
  local p
  for p in "${wt[@]}" "$main"; do [[ -n $p ]] && live_name[${p##*/}]=1; done
  local -A live_lc=()
  for p in "${!live_name[@]}"; do live_lc[${p,,}]=1; done

  local out
  if ! dq out ps -a --filter label=com.docker.compose.project \
    --format '{{.ID}}|{{.Names}}|{{.Label "com.docker.compose.project"}}|{{.Label "com.docker.compose.project.working_dir"}}'; then
    echo "docker-tidy: docker ps -a に問えない: $out" >&2
    return 1
  fi

  local -A c_proj=() c_has=()
  local -a rm_id=() rm_name=() rm_wd=()
  local id name proj wd listed w
  while IFS='|' read -r id name proj wd; do
    [[ -n $id ]] || continue
    c_proj[$id]=$proj
    [[ -n $proj && -n $wd ]] || continue
    [[ ${proj,,} == "$main_name"-* ]] || continue
    [[ ${wd%/*} == "$main_parent" ]] || continue
    [[ $wd == "$main" || $wd == *-verify ]] && continue
    [[ -e $wd ]] && continue
    listed=0
    for w in "${wt[@]}"; do [[ $w == "$wd" ]] && listed=1; done
    ((listed)) && continue
    rm_id+=("$id"); rm_name+=("$name"); rm_wd+=("$wd")
  done <<<"$out"
  local -A will_rm=()
  for id in "${rm_id[@]}"; do will_rm[$id]=1; done

  local nout
  if ! dq nout network ls --filter label=com.docker.compose.project \
    --format '{{.ID}}|{{.Name}}|{{.Label "com.docker.compose.project"}}'; then
    echo "docker-tidy: docker network ls に問えない: $nout" >&2
    return 1
  fi
  local -a rm_net=()
  local nid nname nproj att keep k
  while IFS='|' read -r nid nname nproj; do
    [[ -n $nname && -n $nproj ]] || continue
    [[ ${nproj,,} == "$main_name"-* && ${nproj,,} != *-verify ]] || continue
    [[ -z ${live_lc[${nproj,,}]:-} ]] || continue
    if ! dq att ps -aq --filter "network=$nname"; then
      echo "docker-tidy: docker ps --filter network=$nname に問えない: $att" >&2
      return 1
    fi
    keep=0
    for k in $att; do [[ -n ${will_rm[$k]:-} ]] || keep=1; done
    for k in "${!c_proj[@]}"; do
      [[ ${c_proj[$k]} == "$nproj" && -z ${will_rm[$k]:-} ]] && keep=1
    done
    ((keep)) || rm_net+=("$nname")
  done <<<"$nout"

  local i rc=0 res
  for i in "${!rm_id[@]}"; do
    if [[ $dry == 1 ]]; then
      echo "docker-tidy: 外す: コンテナ ${rm_name[i]}（作業ツリー ${rm_wd[i]} が無い）" >&2
    elif res=$("${docker[@]}" rm -f "${rm_id[i]}" 2>&1) || [[ $res == *"No such container"* ]]; then
      echo "docker-tidy: コンテナ ${rm_name[i]} を外した（作業ツリー ${rm_wd[i]} が無い）" >&2
    else
      echo "docker-tidy: コンテナ ${rm_name[i]} を外せなかった: $res" >&2
      rc=1
    fi
  done
  for nname in "${rm_net[@]}"; do
    if [[ $dry == 1 ]]; then
      echo "docker-tidy: 外す: ネットワーク $nname" >&2
    elif res=$("${docker[@]}" network rm "$nname" 2>&1) || [[ $res == *"No such network"* ]]; then
      echo "docker-tidy: ネットワーク $nname を外した" >&2
    else
      echo "docker-tidy: ネットワーク $nname を外せなかった: $res" >&2
      rc=1
    fi
  done
  return $rc
}

# ---- --self-test ---------------------------------------------------------------------------
probe_label=dev.ashiato.ch05-tidy-probe=1
# この回だけの印（並走するほかの木の self-test の探りに触らないよう、後始末も確かめもこの値に絞る）。探りを作る口が init_probe で決める
probe_run_label=
# 探りの控え（EXIT の trap が使うので大域）: コンテナ id・ネットワーク id・名前の無い volume・一時のフォルダ
probe_c= probe_n= real_vol= probe_tmp=

fail=0
check() { # check <条件の説明> <rc: 0 が成立>
  if [[ $2 != 0 ]]; then echo "NG: $1" >&2; fail=1; else echo "ok: $1"; fi
}
has() { grep -qF -- "$2" "$1"; } # has <file> <str>

# 偽の docker。状態は $FAKE_DIR の ps / nets / attach（|区切り）。引数は calls に 1 行ずつ書き留める
write_fake_docker() {
  cat >"$1" <<'FAKE'
#!/usr/bin/env bash
d=$FAKE_DIR
echo "$*" >>"$d/calls"
if [[ -n ${FAKE_FAIL_ALL:-} ]]; then echo "fake: daemon に繋げない" >&2; exit 1; fi
# 本物の CLI も rc=0 のまま標準エラーへ警告を出す（設定のファイルが読めないなど）。一覧の行として読まれてはならない
[[ -z ${FAKE_WARN:-} ]] || echo "WARNING: Error loading config file: $d/config.json" >&2
case "$1" in
  ps)
    if [[ $2 == -a ]]; then awk -F'|' '$3 != ""' "$d/ps"
    else # ps -aq --filter network=NAME
      n=${4#network=}
      awk -F'|' -v n="$n" '$1 == n {print $2}' "$d/attach" | while read -r c; do grep -q "^$c|" "$d/ps" && echo "$c"; done
    fi ;;
  network)
    if [[ $2 == ls ]]; then awk -F'|' '$3 != ""' "$d/nets"
    elif [[ $2 == rm ]]; then
      if [[ -n ${FAKE_NOSUCH:-} ]]; then echo "Error response from daemon: network $3 not found: No such network: $3" >&2; exit 1; fi
      if [[ -n ${FAKE_FAIL_RM:-} ]]; then echo "Error response from daemon: permission denied" >&2; exit 1; fi
      grep -v "^[^|]*|$3|" "$d/nets" >"$d/nets.n"; mv "$d/nets.n" "$d/nets"
    fi ;;
  rm)
    if [[ -n ${FAKE_NOSUCH:-} ]]; then echo "Error response from daemon: No such container: $3" >&2; exit 1; fi
    if [[ -n ${FAKE_FAIL_RM:-} ]]; then echo "Error response from daemon: permission denied" >&2; exit 1; fi
    grep -v "^$3|" "$d/ps" >"$d/ps.n"; mv "$d/ps.n" "$d/ps" ;;
esac
exit 0
FAKE
  chmod +x "$1"
}

self_test() {
  local tmp
  tmp=$(mktemp -d) || return 1
  probe_tmp=$tmp
  local P=$tmp/parent main=$tmp/parent/mainrepo
  mkdir -p "$main" "$P/mainrepo-st10" "$P/mainrepo-st12"
  write_fake_docker "$tmp/fakedocker"

  # 状態を作り直す。ps: id|名前|project|working_dir  nets: id|名前|project  attach: ネットワーク名|コンテナ id
  # 偽の docker は、どの命令でも rc=0 のまま標準エラーへ警告を出す（FAKE_WARN）
  # 生きている作業ツリー: main・st10（path あり）・st11（path なし。prune の前）
  setup() {
    export FAKE_DIR=$tmp/fake
    rm -rf "$FAKE_DIR"; mkdir -p "$FAKE_DIR"
    : >"$FAKE_DIR/ps"; : >"$FAKE_DIR/nets"; : >"$FAKE_DIR/attach"; : >"$FAKE_DIR/calls"
    printf '%s\n' "$main" "$P/mainrepo-st10" "$P/mainrepo-st11" >"$tmp/wts"
    export TEST_DOCKER_TIDY_DOCKER=$tmp/fakedocker TEST_DOCKER_TIDY_MAIN=$main TEST_DOCKER_TIDY_WORKTREES=$tmp/wts
    unset FAKE_FAIL_ALL FAKE_NOSUCH FAKE_FAIL_RM
    export FAKE_WARN=1
  }
  run() { tidy "$@" >"$tmp/out" 2>"$tmp/err"; echo $? >"$tmp/rc"; }
  rc_is() { [[ $(<"$tmp/rc") == "$1" ]]; }
  ps_has() { grep -q "^$1|" "$FAKE_DIR/ps"; }
  net_has() { grep -q "^[^|]*|$1|" "$FAKE_DIR/nets"; }
  add_c() { echo "$1|$2|$3|$4" >>"$FAKE_DIR/ps"; }
  add_n() { echo "$1|$2|$3" >>"$FAKE_DIR/nets"; }
  att() { echo "$1|$2" >>"$FAKE_DIR/attach"; }

  echo "Scenario: 作業ツリーが消えたコンテナは、動いていても外される"
  setup
  add_c c1 mainrepo-st97-db-1 mainrepo-st97 "$P/mainrepo-st97"
  add_n n1 mainrepo-st97_default mainrepo-st97; att mainrepo-st97_default c1
  run 0
  rc_is 0 && ! ps_has c1 && ! net_has mainrepo-st97_default
  check "消えたコンテナとネットワークを外し rc=0（docker の警告があっても）" $?
  has "$tmp/err" "mainrepo-st97-db-1" && has "$tmp/err" "$P/mainrepo-st97" && has "$tmp/err" "mainrepo-st97_default"
  check "標準エラーに名前と path とネットワークが出る" $?

  echo "Scenario: 作業ツリーが残っていれば、止まっているコンテナも外さない"
  setup
  add_c a mainrepo-st10-db-1 mainrepo-st10 "$P/mainrepo-st10"       # 一覧にあり path もある
  add_c b mainrepo-st12-db-1 mainrepo-st12 "$P/mainrepo-st12"       # path はあるが一覧に無い
  add_c c mainrepo-st11-db-1 mainrepo-st11 "$P/mainrepo-st11"       # 一覧にあるが path が無い
  add_n n mainrepo-st10_default mainrepo-st10; att mainrepo-st10_default a
  run 0
  rc_is 0 && ps_has a && ps_has b && ps_has c && net_has mainrepo-st10_default
  check "3 つのコンテナとネットワークが残り rc=0" $?

  echo "Scenario: main の checkout と確認バッチのコンテナは、path が無くても外さない"
  setup
  local gm=$P/ghostmain
  : >"$tmp/wts"; export TEST_DOCKER_TIDY_MAIN=$gm
  add_c m ghost-main-1 ghostmain-x "$gm"
  add_c v ghost-verify-1 ghostmain-verify "$gm-verify"
  run 0
  rc_is 0 && ps_has m && ps_has v
  check "main と -verify のコンテナが残り rc=0" $?

  echo "Scenario: 印の無いコンテナと、別の repo の compose のコンテナには触らない"
  setup
  add_c u plain-1 "" ""                                               # 印なし
  add_c s sentinel-1 ch06-sentinel-ab12 "$P/ch06-sentinel-ab12"        # 別の repo の compose
  add_c o other-1 mainrepo-st60 "/elsewhere/mainrepo-st60"            # 親のフォルダが違う
  add_c r other-2 otherrepo-st61 "$P/otherrepo-st61"                  # <main>- で始まらない
  run 0
  rc_is 0 && ps_has u && ps_has s && ps_has o && ps_has r
  check "どのコンテナも残り rc=0" $?

  echo "Scenario: コンテナの無いネットワークは、作業ツリーが残っていなければ外す"
  setup
  add_c k1 keep-1 mainrepo-st12 "$P/mainrepo-st12"
  add_c k2 keep-2 mainrepo-st10 "$P/mainrepo-st10"
  add_n n1 mainrepo-st98_default mainrepo-st98                         # 外す
  add_n n2 mainrepo-st11_default mainrepo-st11                         # 生きている作業ツリー（その project のコンテナは無い）
  add_n n3 mainrepo-verify_default mainrepo-verify                     # 確認バッチ
  add_n n4 mainrepo-st20_default mainrepo-st20; att mainrepo-st20_default k1   # 外さないコンテナが繋がった
  add_n n5 mainrepo-st21_default mainrepo-st21; att mainrepo-st21_default k2   # 止まった外さないコンテナが繋がった
  add_n n6 mainrepo-st12x_default mainrepo-st12                        # 繋がりは無いが project の印のコンテナが残る
  add_n n7 mainrepo-plain_default ""                                   # 印なし
  add_n n8 other-st98_default other-st98                               # <main>- で始まらない
  run 0
  rc_is 0 && ! net_has mainrepo-st98_default && net_has mainrepo-st11_default && net_has mainrepo-verify_default \
    && net_has mainrepo-st20_default && net_has mainrepo-st21_default && net_has mainrepo-st12x_default \
    && net_has mainrepo-plain_default && net_has other-st98_default
  check "前の 1 つが無く、後の 7 つが残り rc=0" $?

  echo "Scenario: --dry-run は外すものを出すだけで、何も外さない"
  setup
  add_c c1 mainrepo-st97-db-1 mainrepo-st97 "$P/mainrepo-st97"
  add_n n1 mainrepo-st97_default mainrepo-st97; att mainrepo-st97_default c1
  add_n n2 mainrepo-st98_default mainrepo-st98
  run 1
  rc_is 0 && has "$tmp/err" mainrepo-st97-db-1 && has "$tmp/err" mainrepo-st97_default && has "$tmp/err" mainrepo-st98_default \
    && ps_has c1 && net_has mainrepo-st97_default && net_has mainrepo-st98_default
  check "名前が出て rc=0、何も外れない" $?

  echo "Scenario: 外すものが無ければ、片付けは何も出さない"
  setup
  add_c k keep-1 mainrepo-st10 "$P/mainrepo-st10"
  run 0
  rc_is 0 && [[ ! -s $tmp/out && ! -s $tmp/err ]]
  check "何も出ず rc=0" $?

  echo "Scenario: もう無いものを外そうとしたら、外せたものとして扱う"
  setup
  add_c c1 mainrepo-st97-db-1 mainrepo-st97 "$P/mainrepo-st97"
  add_n n1 mainrepo-st97_default mainrepo-st97; att mainrepo-st97_default c1
  FAKE_NOSUCH=1 run 0
  rc_is 0
  check "No such で rc=0" $?

  echo "Scenario: 片付けが docker に問えなければ、片付けは rc≠0 で落ちる"
  setup
  TEST_DOCKER_TIDY_DOCKER=false run 0
  ! rc_is 0 && [[ -s $tmp/err ]]
  check "問えなければ rc≠0 で理由が出る" $?

  echo "Scenario: 外す命令が失敗すれば、片付けは rc≠0 で落ちる"
  setup
  add_c c1 mainrepo-st97-db-1 mainrepo-st97 "$P/mainrepo-st97"
  FAKE_FAIL_RM=1 run 0
  ! rc_is 0 && has "$tmp/err" mainrepo-st97-db-1 && has "$tmp/err" "permission denied"
  check "外せなかった名前と理由が出て rc≠0" $?

  echo "Scenario: 片付けが docker に打つ命令に、volume を消すものは無い"
  setup
  add_c c1 mainrepo-st97-db-1 mainrepo-st97 "$P/mainrepo-st97"
  add_n n1 mainrepo-st97_default mainrepo-st97; att mainrepo-st97_default c1
  run 0
  local bad=0 line
  [[ -s $FAKE_DIR/calls ]] || bad=1
  while IFS= read -r line; do
    case $line in
      "ps "* | "inspect "* | "network ls"* | "network inspect "* | "network rm "*) ;;
      "rm -f "*) [[ $line =~ ^rm\ -f\ [^\ -][^\ ]*$ ]] || bad=1 ;;
      *) bad=1 ;;
    esac
  done <"$FAKE_DIR/calls"
  check "打った命令は ps・inspect・network ls/inspect/rm・rm -f <id> だけ" $bad

  real_docker_case "$tmp" || fail=1

  echo "-- 検証の準備は、DB を消す前・塞いでいる network を確かめる前に片付けを呼ぶ（spec「片付けはその判定より前に終わり」。review/code.md R18）"
  verify_env_order
  check "tools/verify-env.sh の段の並びが 片付け → reset_db → clear_blocking_network" $?

  echo "Scenario: CI の rust job は、片付けの self-test を走らせる"
  ci_rust_runs | grep -qxE ' *- run: \./tools/docker-tidy\.sh --self-test *'
  check "ci.yml の rust job に --self-test の - run: 行がある" $?
  ci_rust_runs | grep -qxE ' *- run: \./tools/docker-tidy\.sh --verify-env-check *'
  check "ci.yml の rust job に --verify-env-check の - run: 行がある" $?
  return $fail
}

# tools/verify-env.sh の段（字下げの無い行）で、片付けの行が `reset_db` の行と `clear_blocking_network` の行より前にある。
# 順が逆だと、消えた作業ツリーの DB が subnet を塞ぐ木で clear_blocking_network が rc=1 になる（CH05 が防ぐ止まり方。design D5）
verify_env_order() {
  awk '/^[^[:space:]#]/ && /tools\/docker-tidy\.sh/ && !t {t=NR}
       /^[^[:space:]#]/ && /(^|[|&] *)reset_db *$/ && !r {r=NR}
       /^clear_blocking_network *$/ && !c {c=NR}
       END {exit !(t && r && c && t < r && t < c)}' tools/verify-env.sh
}

# ci.yml の rust job の `- run:` 行だけを出す（コメントや別の job の行を数えない）
ci_rust_runs() {
  awk '/^  rust:/{f=1; next} /^  [A-Za-z0-9_-]+:/{f=0} f && /^ *- run:/' .github/workflows/ci.yml
}

# 本物の docker で 1 件。main を乱数の名前に替えるので、この機械の本物の残り（ashiato2-*）には当たらない
# 後始末は本体を呼んだ後の明示の呼び出しと、途中で止まったときの EXIT の trap（init_probe が張る）の 2 つ
real_docker_case() {
  local tmp=$1 rc=0
  real_docker_body "$tmp" || rc=1
  cleanup_probes
  return $rc
}

real_docker_body() {
  local tmp=$1 r d main proj wd rc=0
  r=$(openssl rand -hex 4) || return 1
  d=$(mktemp -d "$tmp/real.XXXXXX") || return 1
  main=$d/ch05tidy$r; proj=ch05tidy$r-st97; wd=$d/$proj
  mkdir -p "$main"; echo "$main" >"$d/wts"
  make_probe "$proj" "$wd" || return 1

  TEST_DOCKER_TIDY_MAIN=$main TEST_DOCKER_TIDY_WORKTREES=$d/wts TEST_DOCKER_TIDY_DOCKER=docker tidy 0 2>"$d/err"
  rc=$?
  echo "Scenario: 作業ツリーが消えたコンテナは、動いていても外される"
  check "本物の docker: 片付けが rc=0" "$rc"
  probe_gone
  check "本物の docker: 動いていたコンテナ（控えた id）とネットワーク（控えた id）が無い" $?
  echo "Scenario: 外したコンテナの名前の無い volume は残る"
  docker volume inspect "$real_vol" >/dev/null 2>&1
  check "本物の docker: 控えた id の volume が残る" $?
}

# 探りを作る口の初め: この回だけの印を決め、途中で止まっても探りを残さないよう EXIT の trap を張る
init_probe() {
  probe_run_label=dev.ashiato.ch05-tidy-run=$(openssl rand -hex 6) || return 1
  trap 'cleanup_probes; [[ -z ${probe_tmp:-} ]] || rm -rf "$probe_tmp"' EXIT
  trap 'exit 130' INT TERM HUP
}

# make_probe <project> <working_dir>: 探りのネットワークとコンテナを作り、id と名前の無い volume を控える（rc≠0 なら理由を出す）
make_probe() {
  local proj=$1 wd=$2 out
  if ! out=$(docker network create --label "$probe_label" --label "$probe_run_label" \
    --label "com.docker.compose.project=$proj" "${proj}_default" 2>&1); then
    echo "docker-tidy: 本物の docker でネットワークを作れない（割り当てが尽きた？）: $out" >&2
    return 1
  fi
  probe_n=$out
  if ! out=$(docker run -d --label "$probe_label" --label "$probe_run_label" --label "com.docker.compose.project=$proj" \
    --label "com.docker.compose.project.working_dir=$wd" --network "${proj}_default" \
    postgres:17-alpine sleep 600 2>&1); then
    echo "docker-tidy: 本物の docker でコンテナを起こせない: $out" >&2
    return 1
  fi
  probe_c=$out
  real_vol=$(docker inspect -f '{{range .Mounts}}{{if eq .Type "volume"}}{{.Name}}{{end}}{{end}}' "$probe_c")
  [[ -n $real_vol ]] || { echo "docker-tidy: 名前の無い volume が付かなかった" >&2; return 1; }
}

# 控えたコンテナとネットワークが（id で見て）もう無い
probe_gone() {
  [[ -z $(docker ps -aq --no-trunc --filter "id=$probe_c") ]] && ! docker network inspect "$probe_n" >/dev/null 2>&1
}

# 探りの後始末。消すのはこの回の印（probe_run_label）のついたものと控えた id だけ（ほかの木の探りや本物の compose には触らない）。
# `rm -fv` の `-v` が消すのは自前の探りのコンテナの名前の無い volume だけ。片付けの本体（tidy）からは呼ばない
cleanup_probes() {
  local ids
  [[ -n $probe_run_label ]] || return 0
  ids=$(docker ps -aq --filter "label=$probe_run_label" 2>/dev/null) && [[ -z $ids ]] || docker rm -fv $ids >/dev/null 2>&1
  ids=$(docker network ls -q --filter "label=$probe_run_label" 2>/dev/null) && [[ -z $ids ]] || docker network rm $ids >/dev/null 2>&1
  [[ -z $real_vol ]] || docker volume rm "$real_vol" >/dev/null 2>&1
  probe_c= probe_n= real_vol=
  return 0
}

# ---- --verify-env-check -----------------------------------------------------------------------
# この worktree の DB を作り直すので、`--plan` が reset のときだけ走る（design D5）。
# main は乱数の名前に替える —— この機械の本物の残り（ashiato2-*）には当たらない
verify_env_check() {
  local tmp plan r main proj wd rc=0 n m
  plan=$(HARNESS_VERIFY=1 tools/verify-env.sh --plan 2>&1)
  if [[ $plan != reset ]]; then
    echo "docker-tidy: --verify-env-check は、この木の DB を作り直す（--plan が reset）ときだけ走る: $plan" >&2
    return 2
  fi
  init_probe || return 1
  tmp=$(mktemp -d) || return 1
  probe_tmp=$tmp
  r=$(openssl rand -hex 4) || return 1
  main=$tmp/ch05tidy$r; proj=ch05tidy$r-st97; wd=$tmp/$proj
  mkdir -p "$main"; echo "$main" >"$tmp/wts"
  export TEST_DOCKER_TIDY_MAIN=$main TEST_DOCKER_TIDY_WORKTREES=$tmp/wts

  # 引数を書き留めて本物の docker に渡す命令
  cat >"$tmp/recdocker" <<REC
#!/usr/bin/env bash
echo "\$*" >>"$tmp/calls"
exec docker "\$@"
REC
  chmod +x "$tmp/recdocker"
  : >"$tmp/calls"
  calls() { wc -l <"$tmp/calls"; }

  echo "Scenario: 印の無い検証の準備と --plan は、片付けを呼ばない"
  TEST_DOCKER_TIDY_DOCKER=$tmp/recdocker HARNESS_VERIFY=1 tools/verify-env.sh >/dev/null 2>&1
  [[ $(calls) -ge 1 ]]
  check "作り直す準備は片付けを呼ぶ（書き留めた命令が 1 個以上。対照）" $?
  : >"$tmp/calls"
  TEST_DOCKER_TIDY_DOCKER=$tmp/recdocker env -u HARNESS_VERIFY tools/verify-env.sh >/dev/null 2>&1
  check "印の無い準備が rc=0" $?
  TEST_DOCKER_TIDY_DOCKER=$tmp/recdocker HARNESS_VERIFY=1 tools/verify-env.sh --plan >/dev/null 2>&1
  check "--plan が rc=0" $?
  [[ $(calls) == 0 ]]
  check "印の無い準備と --plan は片付けを呼ばない（新しく書き留めた命令が 0 個）" $?

  echo "Scenario: 片付けが落ちても、検証の準備は DB を作り直して続く"
  TEST_DOCKER_TIDY_DOCKER=false HARNESS_VERIFY=1 tools/verify-env.sh >"$tmp/out" 2>"$tmp/err"
  check "片付けが落ちても準備が rc=0" $?
  n=$(grep -c '片付けに失敗した' "$tmp/err")
  m=$(sed -nE 's/.*片付けに失敗した.*rc=([0-9]+).*/\1/p' "$tmp/err")
  [[ $n == 1 && $m =~ ^[0-9]+$ ]] && ((m >= 1))
  check "標準エラーに 片付けに失敗した と片付けの rc（1 以上）の 1 行だけ" $?
  [[ $(grep -cF '切った接続:' "$tmp/err") == 1 ]]
  check "標準エラーに 切った接続: の 1 行" $?
  [[ -n $(docker compose ps -q --status running db) ]]
  check "この worktree の DB のコンテナが立っている" $?

  echo "Scenario: 消えた作業ツリーのコンテナは、印つきの準備で外れ、その volume は残る"
  if ! make_probe "$proj" "$wd"; then
    rc=1
  else
    HARNESS_VERIFY=1 tools/verify-env.sh >"$tmp/out" 2>"$tmp/err"
    check "印つきの準備が rc=0" $?
    [[ -z $(docker ps -aq --no-trunc --filter "id=$probe_c") ]]
    check "消えた作業ツリーのコンテナ（控えた id）が無い" $?
    docker volume inspect "$real_vol" >/dev/null 2>&1
    check "控えた id の volume が残る" $?
    [[ $(grep -cF '切った接続:' "$tmp/err") == 1 ]]
    check "標準エラーに 切った接続: の 1 行" $?
  fi
  cleanup_probes
  unset TEST_DOCKER_TIDY_MAIN TEST_DOCKER_TIDY_WORKTREES
  [[ $rc == 0 && $fail == 0 ]]
}

case "${1:-}" in
  "") tidy 0 ;;
  --dry-run) tidy 1 ;;
  --self-test) init_probe && self_test ;;
  --verify-env-check) verify_env_check ;;
  *) echo "使い方: tools/docker-tidy.sh [--dry-run|--self-test|--verify-env-check]" >&2; exit 2 ;;
esac
