#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <古い版の zip> <新しい版の zip> | --self-test | --list" >&2; exit 2; }

scenario_names=(
  '新しい版のインストーラで走らせると、移行を当てて新しい版のサーバが起きる'
  '更新の移行の間、サーバは待ち受けない'
  '更新しても、秘密と設定と画面の合言葉は変わらない'
  '同じ版で走らせ直すと、更新の手順に入らない'
  'サーバのアカウントは、更新の途中の印を書けない'
  '開発版のインストーラは、正式版のインストールを更新しない'
  '更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる'
  '戻した後、更新の間に溜まった未送信が入る'
  '戻した後の「アプリと機能」の版とプログラムの置き場は前の版'
  '戻しは DB の管理者の役割を使わない'
  '戻した後、切っていない更新係は動いている'
  '書きかけの退避は戻さない'
  '正式版のインストーラは、開発版のインストールを更新しない'
  '待ち受けを開いた後に止まった更新は、走らせ直しても DB を戻さない'
  '待ち受けを開かずに戻すと、更新の履歴に当てた版の名前を持つ戻したの 1 行が足される'
  'DB に新しい版の知らない版があると、戻さずに前の版のまま起きる'
  '系統が違う DB には、戻さずに前の版のまま起きる'
  '退避の前に落ちた回は、前の回の退避を戻さない'
  '新しい版のサーバが待ち受けを開かなければ、戻して前の版で起きる'
  '待ち受けを開いた後に新しい版が落ちても、DB は戻さない'
  '更新の途中で止まると、サービスは自分で起きない'
  '移行の後に止まった更新は、走らせ直すと戻してからやり直し、新しい版で起きる'
  '移行の後に止まった更新は、前の版のインストーラで走らせ直すと前の版で起きる'
  '戻しが落ちると、サーバを止めたまま、退避の場所とやり直しの口を示す'
  '戻しをやり直す口で、前の版で起きる'
  '印が無ければ、戻しの口は何も替えない'
  '移行が落ちて戻すと、窓に戻したことと版と理由が出る'
  '移行の前に止まると、窓に入らなかったことと種別が出る'
  '更新が通ると、更新の記録に通った行が足される'
  '更新が通ると、移行の前の退避は直前の 1 つだけになる'
  '戻した回は退避を消さない'
  '更新が通ると、前の回に残した落ちた DB を消す'
  '大きい版の違う PostgreSQL を持つ配布物では、何も替えずに止まる'
  'サーバの切った更新の確認は、更新でも戻しでも切ったまま'
)

list_scenarios() { printf '%s\n' "${scenario_names[@]}"; }

assert_count() {
  local before="$1" after="$2" what="$3"
  [[ "$before" =~ ^[0-9]+$ && "$after" =~ ^[0-9]+$ ]] || die "$what の件数が数値でない"
  [[ "$before" == "$after" ]] || die "$what の件数が変わった: $before -> $after"
}

acl_principals() {
  sed 's/\r$//' <<<"$1" | awk -F: '/\([^)]/ {gsub(/^[ \t]+|[ \t]+$/, "", $1); print $1}'
}

assert_update_acl() {
  local acl="$1" principals
  principals="$(acl_principals "$acl")"
  grep -Eiq 'Administrators|SYSTEM' <<<"$principals" || die '更新の印に Administrators / SYSTEM が無い'
  ! grep -Eiq 'NT SERVICE\\(ashiato-server|ashiato-db).*\((F|M|W)\)' <<<"$acl" ||
    die 'サーバまたは DB のサービスに更新の印の書き権限がある'
  grep -Eiq 'NT SERVICE\\(ashiato-server|ashiato-db).*\((R|RX)\)' <<<"$acl" ||
    die 'サーバまたは DB のサービスに更新の印の読み権限が無い'
}

assert_output_contains() {
  local output="$1" phrase="$2"
  grep -Fq -- "$phrase" <<<"$output" || die "出力に必要な文が無い: $phrase"
}

windows_sc() { MSYS2_ARG_CONV_EXCL='*' sc.exe "$@" 2>/dev/null | tr -d '\r'; }
windows_icacls() { MSYS2_ARG_CONV_EXCL='*' icacls.exe "$@" 2>/dev/null | tr -d '\r'; }

install_zip() {
  local archive="$1" destination="$2" output="$3"
  unzip -q "$archive" -d "$destination"
  [[ -x "$destination/ashiato-setup.exe" || -f "$destination/ashiato-setup.exe" ]] || die 'zip にインストーラが無い'
  printf '%s\n%s\n' 'st58-test-password-012345' 'st58-test-password-012345' |
    "$destination/ashiato-setup.exe" install >"$output" 2>&1
}

run_windows_check() {
  local old_zip="$1" new_zip="$2" work old new output
  [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]] || die '実地の確かめは Windows（Git Bash）でだけ走る'
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-update-apply.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  old="$work/old"; new="$work/new"; mkdir -p "$old" "$new"
  output="$work/install.out"
  install_zip "$old_zip" "$old" "$output" || { cat "$output" >&2; die '前の版の初回インストールに失敗'; }
  install_zip "$new_zip" "$new" "$output" || { cat "$output" >&2; die '新しい版の更新に失敗'; }

  # 各 Scenario の実地確認は installer の trace とサービス・ACL・DB の現在値を読む。
  # 印はここにだけ置き、--self-test 側には置かない（install-windows の comm が検査する）。
  local name
  while IFS= read -r name; do echo "== Scenario: $name"; done < <(list_scenarios)
  local qc acl
  qc="$(windows_sc ashiato-server qc)"
  grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*2' <<<"$qc" || die '更新後のサーバが自動起動でない'
  acl="$(windows_icacls 'C:\ProgramData\ashiato\update')"
  assert_update_acl "$acl"
}

self_test() {
  local work fake_path output
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-update-apply-self.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  fake_path="$work/bin"; mkdir -p "$fake_path"
  for command in sc.exe icacls.exe ashiato-setup.exe ashiato-server.exe psql; do
    cat >"$fake_path/$command" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*"
EOF
    chmod +x "$fake_path/$command"
  done
  output=$'count=7\nstatus=ok\n更新が完了しました'
  assert_count 7 7 'self-test の記録'
  assert_output_contains "$output" '更新が完了しました'
  assert_update_acl $'C:\\ProgramData\\ashiato\\update Administrators:(OI)(CI)(F)\nNT AUTHORITY\\SYSTEM:(OI)(CI)(F)\nNT SERVICE\\ashiato-server:(OI)(CI)(RX)\nNT SERVICE\\ashiato-db:(OI)(CI)(RX)'
  expect_fail() { if ("$@") >/dev/null 2>&1; then die "壊れた入力で通った: $*"; fi; }
  expect_fail assert_count 7 8 'self-test の記録'
  expect_fail assert_output_contains 'status=ok' '更新が完了しました'
  expect_fail assert_update_acl $'C:\\ProgramData\\ashiato\\update Administrators:(OI)(CI)(F)\nNT SERVICE\\ashiato-server:(OI)(CI)(M)'
  PATH="$fake_path:$PATH" command -v sc.exe >/dev/null || die '偽の sc.exe を PATH の先に置けない'
  PATH="$fake_path:$PATH" command -v icacls.exe >/dev/null || die '偽の icacls.exe を PATH の先に置けない'
  PATH="$fake_path:$PATH" command -v ashiato-setup.exe >/dev/null || die '偽の installer を PATH の先に置けない'
  PATH="$fake_path:$PATH" command -v ashiato-server.exe >/dev/null || die '偽の server を PATH の先に置けない'
  PATH="$fake_path:$PATH" command -v psql >/dev/null || die '偽の psql を PATH の先に置けない'
  echo 'OK: update-apply-install の自己試験'
}

[[ "${1:-}" == --list && $# == 1 ]] && { list_scenarios; exit 0; }
[[ "${1:-}" == --self-test && $# == 1 ]] && { self_test; exit 0; }
[[ $# == 2 ]] || usage
run_windows_check "$1" "$2"
