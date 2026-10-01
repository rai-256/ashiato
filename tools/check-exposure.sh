#!/usr/bin/env bash
# 本システムの口が網の外に開いていないことを状態で確かめる（ST28 / design D11）。
#
#   tools/check-exposure.sh              # 実機を見る（ss と、あれば tailscale）
#   tools/check-exposure.sh --listen-only # (a) だけ（tools/stack.sh が起動の後に使う）
#   tools/check-exposure.sh --self-test  # fixture で 10 個を撃つ
#
# (a) `ss -ltnH` の待ち受けのうち、BIND の port・WEB_PORT・DEV_WEB_PORT（既定 5173）・DB の 55432 が loopback。
# (b) `tailscale serve status --json`（あれば）: 網の外への公開（AllowFunnel）が無い・本システムの口を
#     平文で網へ出していない・開発用の画面の口を網へ出していない。**読めなければ落ちる**。
#     `tailscale` が無ければ (b) は飛ばしたと出す（網の手段が別のもの）。
# 入力の差し替え: EXPOSURE_SS_OUTPUT / EXPOSURE_SERVE_JSON にファイルを渡す。
# **自動で直さない** —— 網の手段の設定は本人の網のもの。
set -euo pipefail
cd "$(dirname "$0")/.."

fx=tools/fixtures/exposure
run_check() {
  local bind="${BIND:-127.0.0.1:18787}"
  local ports="${bind##*:} ${WEB_PORT:-5180} ${DEV_WEB_PORT:-5173} 55432"
  local ng=0

  local skipped=0
  # 網の手段の設定（(a) が tailscaled の口を見分けるのにも使う）
  local json=""
  if [ -n "${EXPOSURE_SERVE_JSON:-}" ]; then
    json="$(cat "$EXPOSURE_SERVE_JSON")" || { echo "  NG 網の手段の設定を読めなかった"; ng=1; }
  elif command -v tailscale >/dev/null 2>&1; then
    json="$(tailscale serve status --json 2>/dev/null)" || { echo "  NG 網の手段の設定を読めなかった（tailscale serve status --json が失敗）"; ng=1; json=""; }
  else
    skipped=1
  fi
  # 網の手段はあるのに設定が空（rc=0 でも）なら、読めなかったのと同じ扱い（final review R4）
  if [ "$skipped" = 0 ] && [ "$ng" = 0 ] && [ -z "${json//[[:space:]]/}" ]; then
    echo "  NG 網の手段の設定を読めなかった（空だった）"; ng=1
  fi
  local https_ports=""
  if [ -n "$json" ]; then
    https_ports="$(printf '%s' "$json" | python3 -c '
import json, sys
try:
    print(" ".join(k for k, v in (json.load(sys.stdin).get("TCP") or {}).items() if (v or {}).get("HTTPS")))
except Exception:
    pass')" || https_ports=""
  fi

  # (a)
  local ss
  if [ -n "${EXPOSURE_SS_OUTPUT:-}" ]; then ss="$(cat "$EXPOSURE_SS_OUTPUT")"; else ss="$(ss -ltnH)"; fi
  if ! printf '%s\n' "$ss" | PORTS="$ports" HTTPS_PORTS="$https_ports" python3 -c '
import ipaddress, os, sys
ports = set(os.environ["PORTS"].split())
https = set(os.environ["HTTPS_PORTS"].split())
def is_tailnet(host):
    try:
        ip = ipaddress.ip_address(host)
    except ValueError:
        return False
    if ip.version == 4:  # 共有アドレス空間（tailnet の v4）は private でも global でもない
        return not ip.is_private and not ip.is_global
    return ip.packed[:6] == bytes.fromhex("fd7a115ca1e0")  # tailnet の v6
bad = 0
for line in sys.stdin:
    f = line.split()
    if len(f) < 4:
        continue
    addr, _, port = f[3].rpartition(":")
    if port not in ports:
        continue
    host = addr.split("%")[0].strip("[]")
    if host.startswith("127.") or host == "::1":
        continue
    # 網の手段（tailscale serve）が暗号化して出している口は、網の側の待ち受けとして許す
    if port in https and is_tailnet(host):
        continue
    print("  NG loopback 以外で待ち受けている口 port=%s address=%s" % (port, addr))
    bad = 1
sys.exit(bad)'; then ng=1; fi

  [ "${LISTEN_ONLY:-0}" = 1 ] && return $ng
  [ "$skipped" = 1 ] && { echo "  -- tailscale が無いので (b) は飛ばした（網の手段が別のもの）"; return $ng; }
  if [ -n "$json" ]; then
    if ! printf '%s' "$json" | OURS="${bind##*:} ${WEB_PORT:-5180}" DEV="${DEV_WEB_PORT:-5173}" python3 -c '
import json, os, sys
try:
    d = json.load(sys.stdin)
    assert isinstance(d, dict)
except Exception:
    print("  NG 網の手段の設定を読めなかった（JSON として読めない）")
    sys.exit(1)
ours = set(os.environ["OURS"].split())
dev = os.environ["DEV"]
bad = 0
scopes = [d] + [v for v in (d.get("Foreground") or {}).values() if isinstance(v, dict)]
for s in scopes:
    if any(v is True for v in (s.get("AllowFunnel") or {}).values()):
        print("  NG 網の外への公開（AllowFunnel）が有効")
        bad = 1
    tcp = s.get("TCP") or {}
    for key, web in (s.get("Web") or {}).items():
        lport = key.rpartition(":")[2]
        for h in ((web or {}).get("Handlers") or {}).values():
            target = str((h or {}).get("Proxy", "")).rstrip("/").rpartition(":")[2]
            if target == dev:
                print("  NG 開発用の画面の口を網へ出している port=%s" % lport)
                bad = 1
            elif target in ours and not (tcp.get(lport) or {}).get("HTTPS"):
                print("  NG 本システムの口を平文で網へ出している port=%s" % lport)
                bad = 1
    for lport, t in tcp.items():
        fwd = str((t or {}).get("TCPForward", "")).rpartition(":")[2]
        if fwd == dev:
            print("  NG 開発用の画面の口を網へ出している port=%s" % lport)
            bad = 1
        elif fwd in ours and not (t or {}).get("TerminateTLS"):
            print("  NG 本システムの口を平文で網へ出している port=%s" % lport)
            bad = 1
sys.exit(bad)'; then ng=1; fi
  fi
  return $ng
}

expect() { # <期待 pass|fail> <Scenario 名> <ss fixture> <serve fixture> [出力に含まれるべき語]
  local want="$1" name="$2" out rc=0
  out="$(EXPOSURE_SS_OUTPUT="$fx/$3" EXPOSURE_SERVE_JSON="$fx/$4" BIND=127.0.0.1:18787 WEB_PORT=5180 DEV_WEB_PORT=5173 \
    run_check 2>&1)" || rc=$?
  if [ "$want" = fail ]; then
    [ "$rc" -ne 0 ] || { echo "error: 落ちるはずが通った: $name" >&2; exit 1; }
    printf '%s' "$out" | grep -q -- "$5" || { echo "error: 落ちた理由に '$5' が無い: $name" >&2; printf '%s\n' "$out" >&2; exit 1; }
  else
    [ "$rc" -eq 0 ] || { echo "error: 通るはずが落ちた: $name" >&2; printf '%s\n' "$out" >&2; exit 1; }
  fi
}

if [ "${1:-}" = "--self-test" ]; then
  # 印（Scenario の echo）は 1 Scenario に 1 つ、THEN を確かめる撃ち方の直後にだけ置く（review R10 / final review R11）
  expect fail '網の外への公開が有効だと検査が落ちる' ss-ok.txt serve-funnel.json 'AllowFunnel'
  echo "Scenario: 網の外への公開が有効だと検査が落ちる"
  expect fail 'loopback 以外で待ち受ける口があると検査が落ちる' ss-bad-bind.txt serve-ok.json 'port=18787 address=0.0.0.0'
  echo "Scenario: loopback 以外で待ち受ける口があると検査が落ちる"
  expect fail '網へ平文で出している口があると検査が落ちる' ss-ok.txt serve-plain.json '平文'
  echo "Scenario: 網へ平文で出している口があると検査が落ちる"
  expect fail '開発用の画面を網へ出していると検査が落ちる' ss-ok.txt serve-dev.json '開発用の画面'
  echo "Scenario: 開発用の画面を網へ出していると検査が落ちる"
  expect fail '網の手段の設定を読めないと検査が落ちる' ss-ok.txt serve-unreadable.json '読めなかった'
  echo "Scenario: 網の手段の設定を読めないと検査が落ちる"
  # 同じ Scenario の別の形（印は上に 1 つだけ）: 空の設定（rc=0 で空を返した）も読めなかったと扱う
  expect fail '網の手段の設定が空だと検査が落ちる' ss-ok.txt serve-empty.json '読めなかった'
  expect pass 'loopback と暗号化された網の口だけなら検査は通る' ss-tailnet-https.txt serve-tailnet-https.json
  echo "Scenario: loopback と暗号化された網の口だけなら検査は通る"
  # 以下は上の Scenario の別の形（印は上に 1 つだけ）
  expect pass '網の口が無く loopback だけなら検査は通る' ss-ok.txt serve-ok.json
  expect fail 'tailnet の v6 でも HTTPS で出していない口は落ちる' ss-tailnet-https.txt serve-ok.json 'port=18787 address=.fd7a'
  expect fail 'LAN のアドレスで待ち受ける口は落ちる' ss-tailnet-lan.txt serve-tailnet-https.json 'address=203.0.113.5'
  echo "OK check-exposure の自己検査（10 個）"
  exit 0
fi

[ "${1:-}" = "--listen-only" ] && LISTEN_ONLY=1
if run_check; then echo "OK 網の外に開いている口は無い"; else
  echo "error: 網の外に開いている口がある。自動では直さない（網の手段の設定は本人のもの）" >&2; exit 1
fi
