# worktree ごとの待ち受けのポート（source して使う）。
# 並行して走る Story の worktree どうしで、DB とサーバのポートを取り合わない
# （実測 2026-09-30: ST22 の DB が 55432 を持ったまま、ST05 の smoke が DB を起動できなかった）。
#
# 規則:
#   - main の checkout（worktree でない木。CI もここ）→ DB 55432 / サーバ 18787（スマホから見る tailscale serve の先）。
#   - 名前が `<main の名前>-st<NN>`（下流の Story の worktree）→ DB は 55500+NN、サーバは 19000+2×NN（+1 は panic の確かめ用と、
#     縦串の 2 つ目のサーバ（tools/smoke.d/202610212100_st13-archive-hands-off.sh）。この 2 つは同時に走らない）。
#   - 名前が `-verify` で終わる（確認バッチの作業ツリー。harness2 の verify_batch.sh が作る）→ DB は 55499、サーバは 18787。
#     確認の間だけ立てるサーバは、スマホから届く 18787 のまま。DB は main の DB と被らないように分ける。
#   - それ以外の worktree（change の `-ch<NN>`、上流の `-up-st<NN>`、録画の `-rec-st<NN>`、道具の作業など）→
#     git の共通ディレクトリの台帳（`ashiato-ports`。番号 n とパスの対）で空いている n を機械が割り当てて記録し、
#     DB は 55600+n、サーバは 19200+2×n。名前のハッシュは衝突しうるので使わない。消えた worktree の番号は使い回す。
#     実測 2026-10-21: `-ch01` と `-ch02` が 55432 に落ち、5 日残っていた確認バッチの DB と取り合って両方止まった。
# ASHIATO_DB_PORT / ASHIATO_HTTP_PORT で上書きできる。
# **規則はここだけ。** Rust のテスト（crates/server/src/testdb.rs など）もこの台本を呼んで port を得る。
# 割り当てられなければ（台帳が尽きた・書けない）非 0 を返す。55432 には落とさない（落とすと main の DB と取り合う）。
_ports_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# 台帳（$1）から、この worktree の番号を引く。無ければ空いている番号を割り当てて記録する。
_ports_slot() {
  (
    flock 9
    local n path mine= keep=() used=()
    if [[ -f $1 ]]; then
      while IFS=$'\t' read -r n path; do
        [[ -d $path ]] || continue   # 消えた worktree の行は捨てる（番号を空ける）
        keep+=("$n"$'\t'"$path")
        used[n]=1
        [[ $path == "$_ports_root" ]] && mine=$n
      done <"$1"
    fi
    if [[ -z $mine ]]; then
      for ((n = 0; n < 100; n++)); do [[ -n ${used[n]:-} ]] || { mine=$n; break; }; done
      [[ -n $mine ]] || { echo "ports.sh: 台帳 $1 の番号が尽きた（100 個）" >&2; exit 1; }
      keep+=("$mine"$'\t'"$_ports_root")
      printf '%s\n' "${keep[@]}" >"$1.tmp" && mv "$1.tmp" "$1"
    fi
    echo "$mine"
  ) 9>"$1.lock"
}

_ports_dirs="$(git -C "$_ports_root" rev-parse --path-format=absolute --git-dir --git-common-dir 2>/dev/null || true)"
_ports_gitdir=${_ports_dirs%%$'\n'*}
_ports_common=${_ports_dirs##*$'\n'}
_ports_main=${_ports_common%/.git}
_ports_main=${_ports_main##*/}
if [[ -z $_ports_dirs || $_ports_gitdir == "$_ports_common" ]]; then
  : "${ASHIATO_DB_PORT:=55432}"
  : "${ASHIATO_HTTP_PORT:=18787}"
elif [[ ${_ports_root##*/} =~ ^"$_ports_main"-st0*([0-9]+)$ ]]; then
  _ports_n=${BASH_REMATCH[1]}
  : "${ASHIATO_DB_PORT:=$((55500 + _ports_n))}"
  : "${ASHIATO_HTTP_PORT:=$((19000 + 2 * _ports_n))}"
elif [[ ${_ports_root##*/} == *-verify ]]; then
  : "${ASHIATO_DB_PORT:=55499}"
  : "${ASHIATO_HTTP_PORT:=18787}"
elif [[ -z ${ASHIATO_DB_PORT:-} || -z ${ASHIATO_HTTP_PORT:-} ]]; then
  _ports_n=$(_ports_slot "$_ports_common/ashiato-ports") || return 1
  : "${ASHIATO_DB_PORT:=$((55600 + _ports_n))}"
  : "${ASHIATO_HTTP_PORT:=$((19200 + 2 * _ports_n))}"
fi
export ASHIATO_DB_PORT ASHIATO_HTTP_PORT

# `.env` の DB の URL（合言葉は `.env` だけが持つ。ST28 / design D19）の port を、この worktree の DB の port に差し替える。
# 合言葉を台本に書かないまま、worktree ごとの DB に繋ぐ（2026-10-05。ST05 と ST28 の統合）
ashiato_db_url() { printf '%s' "$1" | sed -E "s#^(postgres(ql)?://[^@/]*@[^:/]+):[0-9]+/#\1:${ASHIATO_DB_PORT}/#"; }
unset _ports_root _ports_n _ports_dirs _ports_gitdir _ports_common _ports_main
unset -f _ports_slot
