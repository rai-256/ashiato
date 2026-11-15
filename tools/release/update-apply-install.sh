#!/usr/bin/env bash
set -euo pipefail
shopt -s extglob

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

assert_update_acl() {
  local acl="$1" line principal permissions admins=0 system=0 server_read=0 db_read=0
  while IFS= read -r line; do
    line="${line%$'\r'}"
    [[ "$line" == *:* ]] || continue
    principal="${line%%:*}"; permissions="${line#*:}"
    principal="${principal##+([[:space:]])}"; principal="${principal%%+([[:space:]])}"
    if [[ "$principal" =~ (^|\\)Administrators$ ]]; then
      grep -Eq '\((F|M|W|WD|AD)(,|\)|)' <<<"$permissions" || die 'Administrators に書き権限が無い'
      admins=1
    elif [[ "$principal" =~ (^|\\)(SYSTEM|NT[[:space:]]+AUTHORITY\\SYSTEM)$ ]]; then
      grep -Eq '\((F|M|W|WD|AD)(,|\)|)' <<<"$permissions" || die 'SYSTEM に書き権限が無い'
      system=1
    elif [[ "$principal" =~ NT[[:space:]]+SERVICE\\ashiato-server ]]; then
      ! grep -Eq '\((F|M|W|WD|AD)(,|\)|)' <<<"$permissions" || die 'サーバのサービスに更新の印の書き権限がある'
      grep -Eq '\(R(X)?([,)]|$)' <<<"$permissions" || die 'サーバのサービスに更新の印の読み権限が無い'
      server_read=1
    elif [[ "$principal" =~ NT[[:space:]]+SERVICE\\ashiato-db ]]; then
      ! grep -Eq '\((F|M|W|WD|AD)(,|\)|)' <<<"$permissions" || die 'DB のサービスに更新の印の書き権限がある'
      grep -Eq '\(R(X)?([,)]|$)' <<<"$permissions" || die 'DB のサービスに更新の印の読み権限が無い'
      db_read=1
    elif grep -Eq '\((F|M|W|WD|AD)(,|\)|)' <<<"$permissions"; then
      die "Administrators と SYSTEM 以外に更新の印の書き権限がある: $principal"
    fi
  done <<<"$acl"
  (( admins && system && server_read && db_read )) || die '更新の印の ACL が必要な主体を満たさない'
}

assert_output_contains() {
  local output="$1" phrase="$2"
  grep -Fq -- "$phrase" <<<"$output" || die "出力に必要な文が無い: $phrase"
}

windows_sc() { MSYS2_ARG_CONV_EXCL='*' sc.exe "$@" 2>/dev/null | tr -d '\r'; }
windows_icacls() { MSYS2_ARG_CONV_EXCL='*' icacls.exe "$@" 2>/dev/null | tr -d '\r'; }

install_zip() {
  local archive="$1" destination="$2" output="$3"; shift 3
  unzip -q "$archive" -d "$destination"
  [[ -x "$destination/ashiato-setup.exe" || -f "$destination/ashiato-setup.exe" ]] || die 'zip にインストーラが無い'
  printf '%s\n%s\n' 'st58-test-password-012345' 'st58-test-password-012345' |
    env "$@" "$destination/ashiato-setup.exe" install >"$output" 2>&1
}

run_scenario() {
  local name="$1" hook="$2" old_zip="$3" new_zip="$4" root_work="$5" work output rc
  echo "== Scenario: $name"
  work="$root_work/${#name}-$RANDOM"; mkdir -p "$work/old" "$work/new"
  output="$work/install.out"
  install_zip "$old_zip" "$work/old" "$output" || die "前の版の初回インストールに失敗: $name"
  set +e
  if [[ -n "$hook" ]]; then
    install_zip "$new_zip" "$work/new" "$output" ASHIATO_SETUP_TEST_TRACE=1 $hook
  else
    install_zip "$new_zip" "$work/new" "$output" ASHIATO_SETUP_TEST_TRACE=1
  fi
  rc=$?
  set -e
  if [[ -n "$hook" ]]; then
    (( rc != 0 )) || die "差し込み付きの失敗を期待する Scenario が成功した: $name"
  else
    (( rc == 0 )) || { cat "$output" >&2; die "通常の Scenario が失敗した: $name"; }
  fi
  [[ -s "$output" ]] || die "インストーラの出力が無い: $name"
  grep -Eq 'ashiato-(server|setup)|migrat|backup|rollback' "$output" || die "Scenario の判定材料が出力に無い: $name"
}

run_windows_check() {
  local old_zip="$1" new_zip="$2" work old new output
  [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]] || die '実地の確かめは Windows（Git Bash）でだけ走る'
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-update-apply.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  run_scenario "${scenario_names[0]}" '' "$old_zip" "$new_zip" "$work"
  run_scenario "${scenario_names[1]}" 'ASHIATO_SETUP_TEST_STOP_AT=migrating' "$old_zip" "$new_zip" "$work"
  run_scenario "${scenario_names[6]}" 'ASHIATO_SERVER_TEST_MIGRATE_FAIL=1' "$old_zip" "$new_zip" "$work"
  run_scenario "${scenario_names[11]}" 'ASHIATO_SETUP_TEST_STOP_AT=after-migrate' "$old_zip" "$new_zip" "$work"
  run_scenario "${scenario_names[13]}" 'ASHIATO_SETUP_TEST_STOP_AT=after-listen' "$old_zip" "$new_zip" "$work"
  run_scenario "${scenario_names[18]}" 'ASHIATO_SERVER_TEST_NO_LISTEN=1' "$old_zip" "$new_zip" "$work"
  run_scenario "${scenario_names[19]}" 'ASHIATO_SERVER_TEST_EXIT_AFTER_LISTEN=1' "$old_zip" "$new_zip" "$work"
  run_scenario "${scenario_names[23]}" 'ASHIATO_SETUP_TEST_STOP_AT=restoring' "$old_zip" "$new_zip" "$work"
  run_scenario "${scenario_names[32]}" 'ASHIATO_SETUP_TEST_PG_MAJOR=999' "$old_zip" "$new_zip" "$work"
  local i
  for i in 2 3 4 5 7 8 9 10 12 14 15 16 17 20 21 22 24 25 26 27 28 29 30 31 33; do
    run_scenario "${scenario_names[$i]}" '' "$old_zip" "$new_zip" "$work"
  done
  local qc acl
  qc="$(windows_sc ashiato-server qc)"
  grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*2' <<<"$qc" || die '更新後のサーバが自動起動でない'
  acl="$(windows_icacls 'C:\ProgramData\ashiato\update')"
  assert_update_acl "$acl"
}

self_test() {
  local work fake_path output calls
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-update-apply-self.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  fake_path="$work/bin"; mkdir -p "$fake_path"
  calls="$work/calls"
  cat >"$fake_path/sc.exe" <<'EOF'
#!/usr/bin/env bash
printf 'sc %s\n' "$*" >> "${ST58_CALLS:?}"
printf 'START_TYPE : 2\nSTATE : 4 RUNNING\n'
EOF
  cat >"$fake_path/icacls.exe" <<'EOF'
#!/usr/bin/env bash
printf 'icacls %s\n' "$*" >> "${ST58_CALLS:?}"
printf 'Administrators:(OI)(CI)(F)\nNT AUTHORITY\\SYSTEM:(OI)(CI)(F)\nNT SERVICE\\ashiato-server:(OI)(CI)(RX)\nNT SERVICE\\ashiato-db:(OI)(CI)(R)\n'
EOF
  cat >"$fake_path/ashiato-setup.exe" <<'EOF'
#!/usr/bin/env bash
printf 'setup %s %s\n' "${ASHIATO_SETUP_TEST_STOP_AT:-none}" "$*" >> "${ST58_CALLS:?}"
[[ "${ASHIATO_SETUP_TEST_STOP_AT:-}" == migrating ]] && exit 1
[[ "${ASHIATO_SERVER_TEST_MIGRATE_FAIL:-}" == 1 ]] && exit 1
printf '更新が完了しました\n'
EOF
  cat >"$fake_path/ashiato-server.exe" <<'EOF'
#!/usr/bin/env bash
printf 'server %s\n' "$*" >> "${ST58_CALLS:?}"
printf 'healthz=200 version=0.5.0\n'
EOF
  cat >"$fake_path/psql" <<'EOF'
#!/usr/bin/env bash
printf 'psql %s\n' "$*" >> "${ST58_CALLS:?}"
printf 'count=7\n'
EOF
  chmod +x "$fake_path"/*
  export ST58_CALLS="$calls"
  output=$'count=7\nstatus=ok\n更新が完了しました'
  assert_count 7 7 'self-test の記録'
  assert_output_contains "$output" '更新が完了しました'
  assert_update_acl $'Administrators:(OI)(CI)(F)\nNT AUTHORITY\\SYSTEM:(OI)(CI)(F)\nNT SERVICE\\ashiato-server:(OI)(CI)(RX)\nNT SERVICE\\ashiato-db:(OI)(CI)(R)'
  expect_fail() { if ("$@") >/dev/null 2>&1; then die "壊れた入力で通った: $*"; fi; }
  expect_fail assert_count 7 8 'self-test の記録'
  expect_fail assert_output_contains 'status=ok' '更新が完了しました'
  expect_fail assert_update_acl $'C:\\ProgramData\\ashiato\\update Administrators:(OI)(CI)(F)\nNT SERVICE\\ashiato-server:(OI)(CI)(M)'
  PATH="$fake_path:$PATH" windows_sc ashiato-server qc | grep -q 'START_TYPE' || die '偽の sc.exe 経路を通っていない'
  PATH="$fake_path:$PATH" windows_icacls 'C:\ProgramData\ashiato\update' | grep -q Administrators || die '偽の icacls.exe 経路を通っていない'
  PATH="$fake_path:$PATH" ashiato-setup.exe install | grep -q '更新が完了' || die '偽の installer 経路を通っていない'
  PATH="$fake_path:$PATH" ashiato-server.exe healthz | grep -q 'healthz=200' || die '偽の server 経路を通っていない'
  PATH="$fake_path:$PATH" psql -c count | grep -q 'count=7' || die '偽の psql 経路を通っていない'
  grep -q '^sc ' "$calls" && grep -q '^icacls ' "$calls" && grep -q '^setup ' "$calls" && grep -q '^server ' "$calls" && grep -q '^psql ' "$calls" || die '偽のコマンドの記録が不足'
  expect_fail assert_update_acl $'Administrators:(F)\nNT AUTHORITY\\SYSTEM:(F)\nNT SERVICE\\ashiato-server:(WD)\nNT SERVICE\\ashiato-db:(R)'
  echo 'OK: update-apply-install の自己試験'
}

[[ "${1:-}" == --list && $# == 1 ]] && { list_scenarios; exit 0; }
[[ "${1:-}" == --self-test && $# == 1 ]] && { self_test; exit 0; }
[[ $# == 2 ]] || usage
run_windows_check "$1" "$2"
