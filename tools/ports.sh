# worktree ごとの待ち受けのポート（source して使う）。
# 並行して走る Story の worktree どうしで、DB とサーバのポートを取り合わない
# （実測 2026-09-30: ST22 の DB が 55432 を持ったまま、ST05 の smoke が DB を起動できなかった）。
#
# 規則: worktree のディレクトリ名が `-st<NN>` で終わる → DB は 55500+NN、サーバは 19000+2×NN（+1 は panic の確かめ用）。
#       `-verify` で終わる（確認バッチの作業ツリー。harness2 の verify_batch.sh が作る）→ DB は 55499、サーバは 18787。
#       それ以外（main）は従来どおり DB 55432 / サーバ 18787（スマホから見る tailscale serve の先）。
#       確認バッチは 2026-10-14 から main の checkout と別の作業ツリーで組む。compose project が別になり、DB の番号が
#       main の DB と被るので分ける（サーバは確認の間だけ立てるので、スマホから届く 18787 のまま）。
# ASHIATO_DB_PORT / ASHIATO_HTTP_PORT で上書きできる。
# **同じ規則が crates/server/src/testdb.rs にある**（Rust のテストの既定の接続先）。変えるなら両方を変える。
_ports_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ ${_ports_root##*/} =~ -st0*([0-9]+)$ ]]; then
  _ports_n=${BASH_REMATCH[1]}
  : "${ASHIATO_DB_PORT:=$((55500 + _ports_n))}"
  : "${ASHIATO_HTTP_PORT:=$((19000 + 2 * _ports_n))}"
elif [[ ${_ports_root##*/} == *-verify ]]; then
  : "${ASHIATO_DB_PORT:=55499}"
  : "${ASHIATO_HTTP_PORT:=18787}"
else
  : "${ASHIATO_DB_PORT:=55432}"
  : "${ASHIATO_HTTP_PORT:=18787}"
fi
export ASHIATO_DB_PORT ASHIATO_HTTP_PORT

# `.env` の DB の URL（合言葉は `.env` だけが持つ。ST28 / design D19）の port を、この worktree の DB の port に差し替える。
# 合言葉を台本に書かないまま、worktree ごとの DB に繋ぐ（2026-10-05。ST05 と ST28 の統合）
ashiato_db_url() { printf '%s' "$1" | sed -E "s#^(postgres(ql)?://[^@/]*@[^:/]+):[0-9]+/#\1:${ASHIATO_DB_PORT}/#"; }
unset _ports_root _ports_n
