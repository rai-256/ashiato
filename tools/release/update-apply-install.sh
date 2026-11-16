#!/usr/bin/env bash
set -euo pipefail
shopt -s extglob

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
die() { echo "NG: $*" >&2; exit 1; }
usage() { echo "使い方: $0 <正式旧> <正式新> [<開発旧> <開発新> [<互換旧> <互換新>]] | --self-test | --list" >&2; exit 2; }

# Scenario: 新しい版のインストーラで走らせると、移行を当てて新しい版のサーバが起きる
# Scenario: 更新の移行の間、サーバは待ち受けない
# Scenario: 更新しても、秘密と設定と画面の合言葉は変わらない
# Scenario: 同じ版で走らせ直すと、更新の手順に入らない
# Scenario: サーバのアカウントは、更新の途中の印を書けない
# Scenario: 開発版のインストーラは、正式版のインストールを更新しない
# Scenario: 更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる
# Scenario: 戻した後、更新の間に溜まった未送信が入る
# Scenario: 戻した後の「アプリと機能」の版とプログラムの置き場は前の版
# Scenario: 戻しは DB の管理者の役割を使わない
# Scenario: 戻した後、切っていない更新係は動いている
# Scenario: 書きかけの退避は戻さない
# Scenario: 正式版のインストーラは、開発版のインストールを更新しない
# Scenario: 待ち受けを開いた後に止まった更新は、走らせ直しても DB を戻さない
# Scenario: 待ち受けを開かずに戻すと、更新の履歴に当てた版の名前を持つ戻したの 1 行が足される
# Scenario: DB に新しい版の知らない版があると、戻さずに前の版のまま起きる
# Scenario: 系統が違う DB には、戻さずに前の版のまま起きる
# Scenario: 退避の前に落ちた回は、前の回の退避を戻さない
# Scenario: 新しい版のサーバが待ち受けを開かなければ、戻して前の版で起きる
# Scenario: 待ち受けを開いた後に新しい版が落ちても、DB は戻さない
# Scenario: 更新の途中で止まると、サービスは自分で起きない
# Scenario: 移行の後に止まった更新は、走らせ直すと戻してからやり直し、新しい版で起きる
# Scenario: 移行の後に止まった更新は、前の版のインストーラで走らせ直すと前の版で起きる
# Scenario: 戻しが落ちると、サーバを止めたまま、退避の場所とやり直しの口を示す
# Scenario: 戻しをやり直す口で、前の版で起きる
# Scenario: 印が無ければ、戻しの口は何も替えない
# Scenario: 移行が落ちて戻すと、窓に戻したことと版と理由が出る
# Scenario: 移行の前に止まると、窓に入らなかったことと種別が出る
# Scenario: 更新が通ると、更新の記録に通った行が足される
# Scenario: 更新が通ると、移行の前の退避は直前の 1 つだけになる
# Scenario: 戻した回は退避を消さない
# Scenario: 更新が通ると、前の回に残した落ちた DB を消す
# Scenario: 大きい版の違う PostgreSQL を持つ配布物では、何も替えずに止まる
# Scenario: サーバの切った更新の確認は、更新でも戻しでも切ったまま
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

settings_fingerprint() {
  local settings='C:/ProgramData/ashiato/settings'
  [[ -d "$settings" ]] || { printf '%s\n' absent; return; }
  find "$settings" -maxdepth 1 -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}'
}

completed_backup_count() {
  local backups='C:/ProgramData/ashiato/backups/pre-migrate'
  [[ -d "$backups" ]] || { printf '0\n'; return; }
  find "$backups" -maxdepth 1 -type f -name '*.dump' | wc -l | tr -d ' '
}

# 書き込みに当たる権利（icacls の単純な権利と細かい権利）。括弧の中のどの位置にあっても拾う
# （`(RX,DE)` も。review/code.md R63・Task 6 F12）
write_rights='[(,](F|M|W|D|WD|AD|DE|DC|WDAC|WO|WA|WEA|GA|GW)[,)]'

assert_update_acl() {
  local acl="$1" line principal permissions admins=0 system=0 server_read=0 db_read=0
  while IFS= read -r line; do
    line="${line%$'\r'}"
    [[ "$line" == *:* ]] || continue
    principal="${line%%:*}"; permissions="${line#*:}"
    principal="${principal##+([[:space:]])}"; principal="${principal%%+([[:space:]])}"
    if [[ "$principal" =~ (^|\\)Administrators$ ]]; then
      grep -Eq "$write_rights" <<<"$permissions" || die 'Administrators に書き権限が無い'
      admins=1
    elif [[ "$principal" =~ (^|\\)(SYSTEM|NT[[:space:]]+AUTHORITY\\SYSTEM)$ ]]; then
      grep -Eq "$write_rights" <<<"$permissions" || die 'SYSTEM に書き権限が無い'
      system=1
    elif [[ "$principal" =~ NT[[:space:]]+SERVICE\\ashiato-server ]]; then
      ! grep -Eq "$write_rights" <<<"$permissions" || die 'サーバのサービスに更新の印の書き権限がある'
      grep -Eq '\(R(X)?([,)]|$)' <<<"$permissions" || die 'サーバのサービスに更新の印の読み権限が無い'
      server_read=1
    elif [[ "$principal" =~ NT[[:space:]]+SERVICE\\ashiato-db ]]; then
      ! grep -Eq "$write_rights" <<<"$permissions" || die 'DB のサービスに更新の印の書き権限がある'
      grep -Eq '\(R(X)?([,)]|$)' <<<"$permissions" || die 'DB のサービスに更新の印の読み権限が無い'
      db_read=1
    elif grep -Eq "$write_rights" <<<"$permissions"; then
      die "Administrators と SYSTEM 以外に更新の印の書き権限がある: $principal"
    fi
  done <<<"$acl"
  (( admins && system && server_read && db_read )) || die '更新の印の ACL が必要な主体を満たさない'
}

assert_output_contains() {
  local output="$1" phrase="$2"
  grep -Fq -- "$phrase" <<<"$output" || die "出力に必要な文が無い: $phrase"
}

append_install_output() {
  local output="$1" trace="$2"
  cat "$output" >>"$trace"
}

windows_sc() { MSYS2_ARG_CONV_EXCL='*' sc.exe "$@" 2>/dev/null | tr -d '\r'; }
windows_icacls() { MSYS2_ARG_CONV_EXCL='*' icacls.exe "$@" 2>/dev/null | tr -d '\r'; }

install_zip() {
  local archive="$1" destination="$2" output="$3" action="${4:-install}"; shift 4
  unzip -q "$archive" -d "$destination"
  [[ -x "$destination/ashiato-setup.exe" || -f "$destination/ashiato-setup.exe" ]] || die 'zip にインストーラが無い'
  printf '%s\n%s\n' 'st58-test-password-012345' 'st58-test-password-012345' |
    env "$@" "$destination/ashiato-setup.exe" "$action" >"$output" 2>&1
}

# インストーラの環境に置く差し込み（1 行に 1 つ）。`STOP_AT` は誤りを返して自動の戻しを通る。
# 「途中で止まった印が残る」形は `CRASH_AT`（戻さずにその場で終わる）、「段で待つ間を外から見る」形は `PAUSE_AT`
# で作る（review/code.md R36・R58）。移行の失敗 `MIGRATE_FAIL` はインストーラが起こす `migrate` に届く。
scenario_hook() {
  case "$1" in
    '更新の移行の間、サーバは待ち受けない') printf '%s\n' 'ASHIATO_SETUP_TEST_PAUSE_AT=migrating' ;;
    '更新の途中で止まると、サービスは自分で起きない') printf '%s\n' 'ASHIATO_SETUP_TEST_CRASH_AT=after-migrate' ;;
    '更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる'|\
    '戻した後、更新の間に溜まった未送信が入る'|\
    '戻した後の「アプリと機能」の版とプログラムの置き場は前の版'|\
    '戻した後、切っていない更新係は動いている'|\
    '移行が落ちて戻すと、窓に戻したことと版と理由が出る'|\
    '戻しは DB の管理者の役割を使わない'|\
    '戻した回は退避を消さない') printf '%s\n' 'ASHIATO_SERVER_TEST_MIGRATE_FAIL=sqlstate:40001' ;;
    '書きかけの退避は戻さない') printf '%s\n' 'ASHIATO_SERVER_TEST_MIGRATE_FAIL=during-backup' ;;
    '退避の前に落ちた回は、前の回の退避を戻さない') printf '%s\n' 'ASHIATO_SERVER_TEST_MIGRATE_FAIL=before-backup' ;;
    '戻しが落ちると、サーバを止めたまま、退避の場所とやり直しの口を示す')
      printf '%s\n' 'ASHIATO_SETUP_TEST_STOP_AT=restoring' 'ASHIATO_SERVER_TEST_MIGRATE_FAIL=sqlstate:40001' ;;
    '大きい版の違う PostgreSQL を持つ配布物では、何も替えずに止まる') printf '%s\n' 'ASHIATO_SETUP_TEST_PG_MAJOR=999' ;;
    *) : ;;
  esac
}

# サーバのサービスの環境に置く差し込み。サーバは SCM が起こすので、インストーラの環境は届かない。
# 値は版番号で、その版のサーバだけが従う（戻した前の版は従わない。review/code.md R58 (d)）。
scenario_service_env() {
  local name="$1" new_version="$2"
  case "$name" in
    '新しい版のサーバが待ち受けを開かなければ、戻して前の版で起きる'|\
    '待ち受けを開かずに戻すと、更新の履歴に当てた版の名前を持つ戻したの 1 行が足される')
      printf '%s\n' "ASHIATO_SERVER_TEST_NO_LISTEN=$new_version" ;;
    '待ち受けを開いた後に新しい版が落ちても、DB は戻さない') printf '%s\n' "ASHIATO_SERVER_TEST_EXIT_AFTER_LISTEN=$new_version" ;;
    *) : ;;
  esac
}

service_environment_key='HKLM\SYSTEM\CurrentControlSet\Services\ashiato-server'
set_service_env() {
  MSYS2_ARG_CONV_EXCL='*' reg.exe add "$service_environment_key" /v Environment /t REG_MULTI_SZ /d "$1" /f >/dev/null ||
    die "サーバのサービスの環境に差し込みを置けない: $1"
}
clear_service_env() {
  MSYS2_ARG_CONV_EXCL='*' reg.exe delete "$service_environment_key" /v Environment /f >/dev/null 2>&1 || true
}

# 前の版を入れた後・新しい版を走らせる前に、表の所有者の接続で DB の版の記録を細工する（F13）。
# 知らない版: この組み立ての知らない移行の行を足す。系統違い: 最初の行の版を開発版の系統にする（系統は最初の行の版で決まる）。
scenario_db_prep() {
  case "$1" in
    'DB に新しい版の知らない版があると、戻さずに前の版のまま起きる'|'移行の前に止まると、窓に入らなかったことと種別が出る')
      printf '%s\n' "INSERT INTO migration_ledger.applied (name, sha256, build_version, build_commit) VALUES ('999912312359_st58_from_the_future', 'abc', '9.9.9', 'st58')" ;;
    '系統が違う DB には、戻さずに前の版のまま起きる')
      printf '%s\n' "UPDATE migration_ledger.applied SET build_version = build_version || '-dev.20261115' WHERE build_version NOT LIKE '%-dev.%'" ;;
    *) : ;;
  esac
}

# 細工を戻す。DB は Scenario をまたいで同じものを使うので、残すと後の Scenario が全部 db_ahead になる。
scenario_db_cleanup() {
  case "$1" in
    'DB に新しい版の知らない版があると、戻さずに前の版のまま起きる'|'移行の前に止まると、窓に入らなかったことと種別が出る')
      printf '%s\n' "DELETE FROM migration_ledger.applied WHERE name = '999912312359_st58_from_the_future'" ;;
    '系統が違う DB には、戻さずに前の版のまま起きる')
      printf '%s\n' "UPDATE migration_ledger.applied SET build_version = replace(build_version, '-dev.20261115', '')" ;;
    *) : ;;
  esac
}

json_value() {
  local file="$1" key="$2" python_bin
  if command -v python >/dev/null 2>&1; then python_bin=python; else python_bin=python3; fi
  "$python_bin" -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8")).get(sys.argv[2], ""))' "$file" "$key"
}

owner_sql() {
  local url
  url="$(json_value 'C:/ProgramData/ashiato/settings/owner.json' DATABASE_OWNER_URL | tr -d '\r')"
  [[ "$url" == postgres://* ]] || die '所有者の設定に表の所有者の接続が無い'
  PGPASSFILE=NUL MSYS2_ARG_CONV_EXCL='*' 'C:/Program Files/ashiato/pgsql/bin/psql.exe' -X -A -t -q -w -v ON_ERROR_STOP=1 -d "$url" -c "$1" | tr -d '\r'
}

scenario_action() {
  case "$1" in
    '印が無ければ、戻しの口は何も替えない') printf '%s\n' rollback ;;
    *) printf '%s\n' install ;;
  esac
}

# 落ちた DB（`ashiato_failed_*`）の増え方。前の Scenario の戻しが残した分を数えないように、走らせる前との差で見る。
failed_db_delta() {
  local trace="$1" before after
  before="$(sed -n 's/^failed-databases-before=//p' "$trace" | tail -n 1)"
  after="$(sed -n 's/^failed-databases=//p' "$trace" | tail -n 1)"
  [[ "$before" =~ ^[0-9]+$ && "$after" =~ ^[0-9]+$ ]] || die '落ちた DB の件数を読めない'
  printf '%s\n' "$(( after - before ))"
}

assert_scenario_result() {
  local name="$1" output="$2" trace="$3" rc="$4" old_version="$5" new_version="$6"
  case "$name" in
    '新しい版のインストーラで走らせると、移行を当てて新しい版のサーバが起きる')
      (( rc == 0 )) || die '通常更新が失敗した'
      grep -Eq "healthz=200|/healthz.*200" "$trace" || die '更新後の healthz 200 が無い'
      grep -Fq "version=$new_version" "$trace" || die '更新後の版が新しい版でない'
      ! grep -Eq 'update/state\.json|stage=(started|migrating|listening)' "$trace" || die '更新の印が残っている'
      ;;
    '更新の移行の間、サーバは待ち受けない')
      # 見るのは移行の段で待っている間の状態（pause- の行）。待ちが解けた後は更新が通る
      grep -Fq 'pause-stage=migrating' "$trace" || die '移行の段で待っている間に見ていない'
      ! grep -Eq '^pause-healthz=200$' "$trace" || die '移行の段で待っている間に healthz が答えた'
      grep -A3 '^pause-server-state:$' "$trace" | grep -Eq 'STATE[[:space:]]*:[[:space:]]*1[[:space:]]' || die '移行の段で待っている間にサーバが実行中でないと確認できない'
      grep -A8 '^pause-server-config:$' "$trace" | grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*4' || die '移行の段で待っている間に開始の種類が無効でない'
      ;;
    '更新しても、秘密と設定と画面の合言葉は変わらない')
      (( rc == 0 )) || die '秘密保持の更新が失敗した'
      local before after
      before="$(sed -n 's/^settings-before=//p' "$trace" | tail -n 1)"; after="$(sed -n 's/^settings-after=//p' "$trace" | tail -n 1)"
      [[ "$before" != absent && -n "$before" && "$before" == "$after" ]] || die '設定の指紋が更新前後で一致しない'
      ;;
    '同じ版で走らせ直すと、更新の手順に入らない')
      (( rc == 0 )) || die '同じ版の再実行が失敗した'
      ! grep -Eq 'migrate|service_stop.*ashiato-server|previous' "$trace" || die '同じ版なのに更新手順へ入った'
      ;;
    'サーバのアカウントは、更新の途中の印を書けない')
      assert_update_acl "$(windows_icacls 'C:\\ProgramData\\ashiato\\update')"
      ;;
    '開発版のインストーラは、正式版のインストールを更新しない')
      (( rc == 0 )) || die '別チャンネルのインストーラが失敗した'
      grep -Fq "installed-version=$old_version" "$trace" || die '開発版のインストーラが正式版の版を替えた'
      ;;
    '正式版のインストーラは、開発版のインストールを更新しない')
      (( rc == 0 )) || die '別チャンネルのインストーラが失敗した'
      grep -Fq "installed-version-dev=$old_version" "$trace" || die '正式版のインストーラが開発版の版を替えた'
      ;;
    '更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる'|'新しい版のサーバが待ち受けを開かなければ、戻して前の版で起きる')
      (( rc != 0 )) || die '戻しを伴う失敗が成功扱いになった'
      grep -Fq "installed-version=$old_version" "$trace" || die '戻し後の版が旧版でない'
      grep -Fq 'update-stage=absent' "$trace" || die '戻し後に更新の印が残っている'
      grep -Eq '^healthz=200$' "$trace" || die '戻した前の版が答えない'
      (( $(failed_db_delta "$trace") >= 1 )) || die '戻しで落ちた DB を退けていない'
      ;;
    '戻した後、更新の間に溜まった未送信が入る')
      grep -Fq 'update-stage=absent' "$trace" || die '戻しの完了を確認できない'
      grep -Eq '^history-after=[1-9][0-9]*$' "$trace" || die '戻し後の履歴を読めない' ;;
    '戻した後の「アプリと機能」の版とプログラムの置き場は前の版')
      (( rc != 0 )) || die '移行の失敗が成功扱いになった'
      grep -Fq "installed-version=$old_version" "$trace" || die '戻し後のプログラム版を確認できない' ;;
    '戻しは DB の管理者の役割を使わない')
      grep -Eq '^role=ashiato_owner result=(ok|pending) .*update-history record-rollback' "$trace" || die '戻しの record-rollback が表の所有者の接続を使っていない'
      ;;
    '戻した後、切っていない更新係は動いている')
      (( rc != 0 )) || die '移行の失敗が成功扱いになった'
      grep -A4 '^updater-service:$' "$trace" | grep -Eq 'STATE[[:space:]]*:[[:space:]]*4' || die '戻し後の更新係が実行中でない'
      grep -A8 '^updater-config:$' "$trace" | grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*2' || die '戻し後の更新係の開始の種類が自動でない' ;;
    '書きかけの退避は戻さない') (( rc != 0 )) && [[ "$(failed_db_delta "$trace")" == 0 ]] || die '書きかけ退避を戻した' ;;
    '戻した回は退避を消さない')
      (( rc != 0 )) || die '戻した回が成功扱いになった'
      grep -Fq 'update-stage=absent' "$trace" || die '戻しの完了を確認できない'
      grep -Eq '^backups-after=[1-9][0-9]*$' "$trace" || die '戻した回の退避を残した確認が無い'
      ;;
    '更新が通ると、移行の前の退避は直前の 1 つだけになる')
      (( rc == 0 )) || die '更新が成功しなかった'
      grep -Fq 'backups-after=1' "$trace" || die '直前の退避 1 つの確認が無い'
      ;;
    '更新が通ると、前の回に残した落ちた DB を消す')
      (( rc == 0 )) || die '更新が成功しなかった'
      grep -Fq 'failed-databases=0' "$trace" || die '前回の落ちた DB の掃除を確認できない'
      ;;
    '更新の途中で止まると、サービスは自分で起きない')
      (( rc != 0 )) || die '途中停止が成功扱いになった'
      grep -Fq 'update-stage=present' "$trace" || die '途中で止まった印が残っていない'
      grep -Fq 'sc-start-refused=1' "$trace" || die '止まった後の sc start が断られていない'
      grep -A8 '^server-config:$' "$trace" | grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*4' || die '途中停止後の開始の種類が無効でない'
      ;;
    '移行の後に止まった更新は、走らせ直すと戻してからやり直し、新しい版で起きる'|'移行の後に止まった更新は、前の版のインストーラで走らせ直すと前の版で起きる')
      (( rc == 0 )) || die '移行後停止の再実行が成功しなかった'
      grep -Fq 'after-migrate' "$trace" || die '移行後停止の差し込みを確認できない'
      if [[ "$name" == '移行の後に止まった更新は、走らせ直すと戻してからやり直し、新しい版で起きる' ]]; then
        grep -Fq "version=$new_version" "$trace" || die '再実行後の版が新しい版でない'
      else
        grep -Fq "version=$old_version" "$trace" || die '前の版での再実行後の版が旧版でない'
      fi
      ;;
    '待ち受けを開いた後に新しい版が落ちても、DB は戻さない')
      (( rc == 0 )) || die '待ち受けの後に落ちた回が失敗になった'
      grep -Fq "installed-version=$new_version" "$trace" || die '待ち受け後に新しい版を保っていない'
      [[ "$(failed_db_delta "$trace")" == 0 ]] || die '待ち受けの後に DB を戻した'
      ;;
    '待ち受けを開いた後に止まった更新は、走らせ直しても DB を戻さない')
      (( rc == 0 )) || die '待ち受けの後に止まった更新の走らせ直しが失敗した'
      grep -Fq 'after-listen' "$trace" || die '待ち受けの後に止める差し込みを確認できない'
      grep -Fq "installed-version=$new_version" "$trace" || die '走らせ直した後の版が新しい版でない'
      grep -Fq 'update-stage=absent' "$trace" || die '走らせ直した後に更新の印が残っている'
      grep -Fq 'failed-databases=0' "$trace" || die '待ち受けの後に止まった更新で DB を戻した'
      ;;
    '戻しが落ちると、サーバを止めたまま、退避の場所とやり直しの口を示す')
      (( rc != 0 )) || die '戻し失敗が成功扱いになった'
      grep -Fq 'ashiato-setup.exe rollback' "$output" || die '戻しのやり直しの口が出ていない'
      grep -Fq 'この回の退避:' "$output" || die 'この回の退避の場所が出ていない'
      ! grep -A3 '^server-state:$' "$trace" | grep -Eq 'STATE[[:space:]]*:[[:space:]]*4' || die '戻しが落ちた後にサーバが実行中'
      grep -A8 '^server-config:$' "$trace" | grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*4' || die '戻しが落ちた後の開始の種類が無効でない'
      ;;
    '戻しをやり直す口で、前の版で起きる')
      (( rc == 0 )) || die '戻しのやり直しが失敗した'
      grep -Fq "installed-version=$old_version" "$trace" || die 'やり直し後の版が旧版でない'
      grep -Fq 'update-stage=absent' "$trace" || die 'やり直し後に更新の印が残っている'
      ;;
    '印が無ければ、戻しの口は何も替えない')
      (( rc != 0 )) || die '印無しの戻しが成功扱いになった'
      grep -Fq '戻すものはありません' "$output" || die '印無しの戻し確認が無い'
      ;;
    '移行が落ちて戻すと、窓に戻したことと版と理由が出る') grep -Fq '戻しました' "$output" || die '戻しの理由の表示が無い' ;;
    '移行の前に止まると、窓に入らなかったことと種別が出る')
      grep -Fq '入りませんでした' "$output" || die '入らなかった理由の表示が無い'
      grep -Fq 'DB に新しい版の知らない版がある' "$output" || die 'db_ahead の短い説明が出ていない'
      grep -Eq '^outcome-last=.*"result":"not_applied".*"kind":"db_ahead"|^outcome-last=.*"kind":"db_ahead".*"result":"not_applied"' "$trace" ||
        die '更新の記録の最後の行が not_applied・db_ahead でない'
      ;;
    '待ち受けを開かずに戻すと、更新の履歴に当てた版の名前を持つ戻したの 1 行が足される')
      grep -Eq '^history-after=[1-9][0-9]*$' "$trace" || die '戻した履歴を読めない'
      ;;
    'DB に新しい版の知らない版があると、戻さずに前の版のまま起きる'|'系統が違う DB には、戻さずに前の版のまま起きる')
      grep -Fq 'db-prep=ok' "$trace" || die 'DB の版の記録の下準備をしていない'
      (( rc != 0 )) || die '未知の DB 版を成功扱いにした'
      [[ "$(failed_db_delta "$trace")" == 0 ]] || die '未知の DB 版を戻した'
      grep -Fq "installed-version=$old_version" "$trace" || die '未知の DB 版で旧版を起こした確認が無い'
      ;;
    '退避の前に落ちた回は、前の回の退避を戻さない') (( rc != 0 )) && [[ "$(failed_db_delta "$trace")" == 0 ]] || die '退避前の失敗で戻しを実行した' ;;
    '更新が通ると、更新の記録に通った行が足される') (( rc == 0 )) && grep -Eq '^history-after=[1-9][0-9]*$' "$trace" || die '更新の記録を読めない' ;;
    '大きい版の違う PostgreSQL を持つ配布物では、何も替えずに止まる') (( rc != 0 )) && grep -Eq 'PG_MAJOR|PostgreSQL|not_applied' "$trace" || die 'PostgreSQL 大版違いの停止を確認できない' ;;
    'サーバの切った更新の確認は、更新でも戻しでも切ったまま')
      grep -Fq 'update-check-enabled=false' "$trace" || die '更新確認の無効状態を確認できない'
      grep -A8 '^updater-config:$' "$trace" | grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*4' || die '切った更新係の開始の種類が無効でない' ;;
    *) die "Scenario の判定が未定義: $name" ;;
  esac
}

server_version_at() {
  local raw
  raw="$(MSYS2_ARG_CONV_EXCL='*' "$1" --version 2>/dev/null || true)"
  grep -oE '[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?' <<<"$raw" | head -n 1 || true
}

# 移行の段で待っているインストーラの外から、待ち受けとサービスの状態を見る（PAUSE_AT）。
observe_paused_update() {
  local trace="$1" i
  for (( i = 0; i < 240; i++ )); do
    [[ -e 'C:/ProgramData/ashiato/update/paused' ]] && break
    sleep 1
  done
  [[ -e 'C:/ProgramData/ashiato/update/paused' ]] || die '移行の段で待つ差し込みに届かない'
  {
    printf 'pause-stage=%s\n' "$(tr -d '\r\n' <'C:/ProgramData/ashiato/update/paused')"
    printf 'pause-healthz=%s\n' "$(curl.exe --silent --max-time 5 -o NUL -w '%{http_code}' http://127.0.0.1:18700/healthz 2>/dev/null || true)"
    printf 'pause-server-state:\n%s\n' "$(windows_sc query ashiato-server || true)"
    printf 'pause-server-config:\n%s\n' "$(windows_sc qc ashiato-server || true)"
  } >>"$trace"
  : >'C:/ProgramData/ashiato/update/resume'
}

run_scenario() {
  local name="$1" old_zip="$2" new_zip="$3" root_work="$4" old_version="$5" new_version="$6" work output trace rc hook action final_zip final_destination retry_output retry_rc settings_before backups_before history_before failed_before service_env
  work="$root_work/${#name}-$RANDOM"; mkdir -p "$work/old" "$work/new"
  output="$work/install.out"
  clear_service_env
  install_zip "$old_zip" "$work/old" "$output" install || die "前の版の初回インストールに失敗: $name"
  trace="$work/trace"
  settings_before="$(settings_fingerprint)"
  backups_before="$(completed_backup_count)"
  history_before="$(owner_sql 'SELECT count(*) FROM core.update_history' 2>/dev/null || printf unavailable)"
  failed_before="$(owner_sql "SELECT count(*) FROM pg_database WHERE datname LIKE 'ashiato_failed_%'" 2>/dev/null || printf unavailable)"
  printf 'failed-databases-before=%s\n' "$failed_before" >>"$trace"
  retry_output=''
  final_zip="$new_zip"
  final_destination="$work/new"
  hook=(); while IFS= read -r pair; do [[ -z "$pair" ]] || hook+=("$pair"); done < <(scenario_hook "$name")
  local prep_sql cleanup_sql
  prep_sql="$(scenario_db_prep "$name")"
  cleanup_sql="$(scenario_db_cleanup "$name")"
  if [[ -n "$prep_sql" ]]; then
    owner_sql "$prep_sql" || die "DB の版の記録の下準備に失敗: $name"
    echo 'db-prep=ok' >>"$trace"
  fi
  if [[ "$name" == 'サーバの切った更新の確認は、更新でも戻しでも切ったまま' ]]; then
    "$work/old/ashiato-setup.exe" update-check off >"$work/update-check-off.out" 2>&1 || die '更新確認を切れない'
  fi
  service_env="$(scenario_service_env "$name" "$new_version")"
  [[ -z "$service_env" ]] || set_service_env "$service_env"
  action="$(scenario_action "$name")"
  # 途中で止まる（戻さずに終わる）更新は、同じ PC の残った印を使って走らせ直す。
  local crash_stage=''
  case "$name" in
    '移行の後に止まった更新は、走らせ直すと戻してからやり直し、新しい版で起きる'|\
    '移行の後に止まった更新は、前の版のインストーラで走らせ直すと前の版で起きる') crash_stage=after-migrate ;;
    '待ち受けを開いた後に止まった更新は、走らせ直しても DB を戻さない') crash_stage=after-listen ;;
  esac
  if [[ -n "$crash_stage" ]]; then
    retry_output="$work/retry.out"
    set +e
    install_zip "$new_zip" "$work/new" "$retry_output" install ASHIATO_SETUP_TEST_TRACE=1 "ASHIATO_SETUP_TEST_CRASH_AT=$crash_stage"
    retry_rc=$?
    set -e
    (( retry_rc != 0 )) || die "途中で止まる初回の実行が成功した: $name"
    [[ -e 'C:/ProgramData/ashiato/update/state.json' ]] || die "途中で止まった印が残っていない: $name"
    [[ "$name" == '移行の後に止まった更新は、前の版のインストーラで走らせ直すと前の版で起きる' ]] && final_zip="$old_zip"
    final_destination="$work/retry"
    mkdir -p "$final_destination"
    hook=()
  elif [[ "$name" == '戻しをやり直す口で、前の版で起きる' ]]; then
    set +e
    install_zip "$new_zip" "$work/new" "$output" install ASHIATO_SETUP_TEST_TRACE=1 ASHIATO_SETUP_TEST_STOP_AT=restoring ASHIATO_SERVER_TEST_MIGRATE_FAIL=sqlstate:40001
    set -e
    [[ -e 'C:/ProgramData/ashiato/update/state.json' ]] || die '戻しが落ちた印が残っていない'
    final_destination="$work/new"
    action=rollback
    hook=()
  fi
  set +e
  if [[ "$name" == '更新の移行の間、サーバは待ち受けない' ]]; then
    install_zip "$final_zip" "$final_destination" "$output" "$action" ASHIATO_SETUP_TEST_TRACE=1 "${hook[@]}" &
    local installer_pid=$!
    observe_paused_update "$trace"
    wait "$installer_pid"
    rc=$?
  elif [[ "$action" == rollback && -x "$final_destination/ashiato-setup.exe" ]]; then
    # 口 `rollback` は展開済みのインストーラで走らせる（zip を同じ場所へ展開し直さない）
    env ASHIATO_SETUP_TEST_TRACE=1 "$final_destination/ashiato-setup.exe" rollback >"$output" 2>&1
    rc=$?
  else
    install_zip "$final_zip" "$final_destination" "$output" "$action" ASHIATO_SETUP_TEST_TRACE=1 "${hook[@]}"
    rc=$?
  fi
  set -e
  clear_service_env
  if [[ "$name" == '更新の途中で止まると、サービスは自分で起きない' ]]; then
    if MSYS2_ARG_CONV_EXCL='*' sc.exe start ashiato-server >/dev/null 2>&1; then
      echo 'sc-start-refused=0' >>"$trace"
    else
      echo 'sc-start-refused=1' >>"$trace"
    fi
  fi
  # 判定用の値は installer の exit code や固定文ではなく、更新後の Windows の実状態から採る。
  {
    local health version settings_after history_after failed_databases update_check_enabled
    health="$(curl.exe --silent --show-error --max-time 5 -o NUL -w '%{http_code}' http://127.0.0.1:18700/healthz 2>/dev/null || true)"
    printf 'healthz=%s\n' "$health"
    version="$(server_version_at 'C:/Program Files/ashiato/ashiato-server.exe')"
    [[ -n "$version" ]] || die 'ashiato-server --version の形式が不正'
    printf 'installed-version=%s\n' "$version"
    [[ ! -e 'C:/Program Files/ashiato-dev/ashiato-server.exe' ]] ||
      printf 'installed-version-dev=%s\n' "$(server_version_at 'C:/Program Files/ashiato-dev/ashiato-server.exe')"
    printf 'settings-before=%s\nsettings-after=%s\n' "$settings_before" "$(settings_fingerprint)"
    printf 'backups-before=%s\nbackups-after=%s\n' "$backups_before" "$(completed_backup_count)"
    history_after="$(owner_sql 'SELECT count(*) FROM core.update_history' 2>/dev/null || printf unavailable)"
    printf 'history-before=%s\nhistory-after=%s\n' "$history_before" "$history_after"
    failed_databases="$(owner_sql "SELECT count(*) FROM pg_database WHERE datname LIKE 'ashiato_failed_%'" 2>/dev/null || printf unavailable)"
    printf 'failed-databases=%s\n' "$failed_databases"
    update_check_enabled="$(json_value 'C:/ProgramData/ashiato-updater/settings/check.json' enabled 2>/dev/null || printf unavailable)"
    printf 'update-check-enabled=%s\n' "${update_check_enabled,,}"
    [[ ! -e 'C:/ProgramData/ashiato/update/outcomes.jsonl' ]] ||
      printf 'outcome-last=%s\n' "$(tail -n 1 'C:/ProgramData/ashiato/update/outcomes.jsonl' | tr -d '\r')"
    printf 'server-state:\n%s\n' "$(windows_sc query ashiato-server || true)"
    printf 'server-config:\n%s\n' "$(windows_sc qc ashiato-server || true)"
    printf 'updater-service:\n%s\n' "$(windows_sc query ashiato-updater || true)"
    printf 'updater-config:\n%s\n' "$(windows_sc qc ashiato-updater || true)"
    windows_icacls 'C:\\ProgramData\\ashiato\\update' || true
    if [[ -e 'C:/ProgramData/ashiato/update/state.json' ]]; then echo 'update-stage=present'; else echo 'update-stage=absent'; fi
    [[ -d 'C:/Program Files/ashiato/previous' ]] && echo 'previous=present'
  } >>"$trace"
  if [[ -n "$cleanup_sql" ]]; then
    owner_sql "$cleanup_sql" || die "DB の版の記録の細工を戻せない: $name"
  fi
  [[ -z "$retry_output" ]] || append_install_output "$retry_output" "$trace"
  append_install_output "$output" "$trace"
  assert_scenario_result "$name" "$output" "$trace" "$rc" "$old_version" "$new_version"
  echo "== Scenario: $name"
}

run_windows_check() {
  local old_zip="$1" new_zip="$2" dev_old_zip="${3:-$1}" dev_new_zip="${4:-$2}"
  local compat_old_zip="${5:-$1}" compat_new_zip="${6:-$2}" work old new output
  [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]] || die '実地の確かめは Windows（Git Bash）でだけ走る'
  work="$(mktemp -d "${TMPDIR:-/tmp}/ashiato-update-apply.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  local i
  for i in "${!scenario_names[@]}"; do
    local name="${scenario_names[$i]}" scenario_old="$old_zip" scenario_new="$new_zip" old_version=0.4.0 new_version=0.5.0
    if [[ "$name" == '同じ版で走らせ直すと、更新の手順に入らない' ]]; then
      scenario_new="$old_zip"; new_version="$old_version"
    elif [[ "$name" == '開発版のインストーラは、正式版のインストールを更新しない' ]]; then
      scenario_new="$dev_new_zip"; new_version=0.5.0-dev.20261116
    elif [[ "$name" == '正式版のインストーラは、開発版のインストールを更新しない' ]]; then
      scenario_old="$dev_old_zip"; old_version=0.4.0-dev.20261115
    elif [[ "$name" == *'知らない版'* || "$name" == *'系統が違う DB'* || "$name" == '移行の前に止まると、窓に入らなかったことと種別が出る' ]]; then
      scenario_old="$compat_old_zip"; scenario_new="$compat_new_zip"; old_version=0.4.1; new_version=0.6.0
    fi
    run_scenario "$name" "$scenario_old" "$scenario_new" "$work" "$old_version" "$new_version"
  done
  local qc acl
  qc="$(windows_sc qc ashiato-server)"
  grep -Eq 'START_TYPE[[:space:]]*:[[:space:]]*2' <<<"$qc" || die '更新後のサーバが自動起動でない'
  acl="$(windows_icacls 'C:\ProgramData\ashiato\update')"
  assert_update_acl "$acl"
}

self_test() {
  local work fake_path output calls evidence_trace evidence_output
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
if [[ -n "${ASHIATO_SERVER_TEST_MIGRATE_FAIL:-}" ]]; then
  printf '更新 0.5.0 は移行が落ちたので、0.4.0 に戻しました（202611152200_x・制約に反した）\n'
  exit 1
fi
if [[ -n "${ST58_FAKE_DB_AHEAD:-}" ]]; then
  printf '更新 0.5.0 は入りませんでした（DB に新しい版の知らない版がある）。0.4.0 のまま動いています\n'
  exit 1
fi
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
  local window_output="$work/window-install.out" window_trace="$work/window-trace"
  # 移行が落ちる窓: インストーラの環境の差し込みで起こす
  [[ "$(scenario_hook '移行が落ちて戻すと、窓に戻したことと版と理由が出る')" == 'ASHIATO_SERVER_TEST_MIGRATE_FAIL=sqlstate:40001' ]] ||
    die '移行が落ちる窓の Scenario に移行の失敗の差し込みが無い'
  set +e
  env ASHIATO_SERVER_TEST_MIGRATE_FAIL=sqlstate:40001 "$fake_path/ashiato-setup.exe" install >"$window_output"
  set -e
  assert_scenario_result '移行が落ちて戻すと、窓に戻したことと版と理由が出る' "$window_output" "$work/empty-trace" 1 0.4.0 0.5.0
  # 入らなかった窓: spec の GIVEN は「DB に知らない版」。差し込みではなく DB の細工で起こし、細工は後で戻す
  [[ -z "$(scenario_hook '移行の前に止まると、窓に入らなかったことと種別が出る')" ]] ||
    die '入らなかった窓の Scenario がインストーラの差し込みで止めている（spec は db_ahead）'
  grep -Fq '999912312359_st58_from_the_future' <<<"$(scenario_db_prep '移行の前に止まると、窓に入らなかったことと種別が出る')" ||
    die '入らなかった窓の Scenario が DB に知らない版を足していない'
  grep -Fq 'DELETE' <<<"$(scenario_db_cleanup '移行の前に止まると、窓に入らなかったことと種別が出る')" ||
    die '入らなかった窓の Scenario が細工を戻していない'
  set +e
  env ST58_FAKE_DB_AHEAD=1 "$fake_path/ashiato-setup.exe" install >"$window_output"
  set -e
  printf '%s\n' 'outcome-last={"at":"t","result":"not_applied","to_version":"0.5.0","from_version":"0.4.0","reason":{"kind":"db_ahead"}}' >"$window_trace"
  assert_scenario_result '移行の前に止まると、窓に入らなかったことと種別が出る' "$window_output" "$window_trace" 1 0.4.0 0.5.0
  printf '%s\n' 'outcome-last={"at":"t","result":"rolled_back","reason":{"kind":"aborted"}}' >"$window_trace"
  expect_fail assert_scenario_result '移行の前に止まると、窓に入らなかったことと種別が出る' "$window_output" "$window_trace" 1 0.4.0 0.5.0
  local wrong_window_output="$work/wrong-window-install.out"
  printf '%s\n' '更新 0.5.0 は移行が落ちたので、0.4.0 に戻しました（中断）' >"$wrong_window_output"
  expect_fail assert_scenario_result '移行の前に止まると、窓に入らなかったことと種別が出る' "$wrong_window_output" "$work/empty-trace" 1 0.4.0 0.5.0
  # サーバへの差し込みはサービスの環境で、新しい版だけに効く版番号を値にする（R58 (d)）
  [[ "$(scenario_service_env '新しい版のサーバが待ち受けを開かなければ、戻して前の版で起きる' 0.5.0)" == 'ASHIATO_SERVER_TEST_NO_LISTEN=0.5.0' ]] ||
    die '待ち受けを開かない差し込みがサービスの環境で新しい版に絞られていない'
  [[ -z "$(scenario_hook '新しい版のサーバが待ち受けを開かなければ、戻して前の版で起きる')" ]] ||
    die 'サーバへの差し込みをインストーラの環境に置いている'
  # 途中で止まる形は戻しを通らない差し込み、戻しの失敗は移行の失敗と組にする（R58 (a)(b)）
  [[ "$(scenario_hook '更新の途中で止まると、サービスは自分で起きない')" == 'ASHIATO_SETUP_TEST_CRASH_AT=after-migrate' ]] ||
    die '途中で止まる Scenario が戻しを通る差し込みを使っている'
  grep -Fq 'ASHIATO_SERVER_TEST_MIGRATE_FAIL=' <<<"$(scenario_hook '戻しが落ちると、サーバを止めたまま、退避の場所とやり直しの口を示す')" ||
    die '戻しが落ちる Scenario が移行を落としていない（戻しに届かない）'
  local paused_trace="$work/paused-trace"
  printf '%s\n' 'pause-stage=migrating' 'pause-healthz=000' 'pause-server-state:' 'STATE              : 1  STOPPED' 'pause-server-config:' 'START_TYPE         : 4   DISABLED' >"$paused_trace"
  assert_scenario_result '更新の移行の間、サーバは待ち受けない' "$window_output" "$paused_trace" 0 0.4.0 0.5.0
  sed -i 's/^pause-healthz=000$/pause-healthz=200/' "$paused_trace"
  expect_fail assert_scenario_result '更新の移行の間、サーバは待ち受けない' "$window_output" "$paused_trace" 0 0.4.0 0.5.0
  expect_fail assert_update_acl $'Administrators:(F)\nNT AUTHORITY\\SYSTEM:(F)\nNT SERVICE\\ashiato-server:(WD)\nNT SERVICE\\ashiato-db:(R)'
  local right
  for right in DE WDAC WO GW '(RX,DE)'; do
    [[ "$right" == '('* ]] || right="($right)"
    expect_fail assert_update_acl $'Administrators:(F)\nNT AUTHORITY\\SYSTEM:(F)\nNT SERVICE\\ashiato-server:(RX)\nNT SERVICE\\ashiato-db:(R)\nBUILTIN\\Users:'"$right"
  done
  evidence_trace="$work/evidence-trace"
  evidence_output="$work/install.out"
  printf '%s\n' "$output" >"$evidence_output"
  # Exercise the Scenario verdict function itself with both an observed-state
  # success and a deliberately corrupted observation.  This keeps the
  # self-test from merely testing helper functions while CI owns Windows.
  cat >"$evidence_trace" <<'EOF'
installed-version=0.4.0
healthz=200
update-stage=absent
failed-databases-before=0
failed-databases=0
settings-before=abc
settings-after=abc
backups-before=1
backups-after=1
history-before=1
history-after=2
role=ashiato_owner result=ok ashiato-server.exe update-history record-rollback
update-check-enabled=false
updater-service:
STATE : 4 RUNNING
updater-config:
START_TYPE : 4 DISABLED
EOF
  assert_scenario_result '書きかけの退避は戻さない' "$evidence_output" "$evidence_trace" 1 0.4.0 0.5.0 || die '書きかけ退避の判定が通らない'
  sed -i 's/^failed-databases=0$/failed-databases=1/' "$evidence_trace"
  assert_scenario_result '更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる' "$evidence_output" "$evidence_trace" 1 0.4.0 0.5.0
  sed -i 's/^failed-databases=1$/failed-databases=0/' "$evidence_trace"
  expect_fail assert_scenario_result '更新の移行が落ちると、退避から DB を戻し、前の版のサーバが起きる' "$evidence_output" "$evidence_trace" 1 0.4.0 0.5.0
  assert_scenario_result '戻しは DB の管理者の役割を使わない' "$evidence_output" "$evidence_trace" 1 0.4.0 0.5.0 || die '戻し役割の判定が通らない'
  assert_scenario_result 'サーバの切った更新の確認は、更新でも戻しでも切ったまま' "$evidence_output" "$evidence_trace" 1 0.4.0 0.5.0 || die '更新確認の判定が通らない'
  [[ "$(scenario_hook '戻した回は退避を消さない')" == 'ASHIATO_SERVER_TEST_MIGRATE_FAIL=sqlstate:40001' ]] ||
    die '戻した回の退避を残す Scenario に移行失敗の差し込みが無い'
  local prep_name
  for prep_name in 'DB に新しい版の知らない版があると、戻さずに前の版のまま起きる' '系統が違う DB には、戻さずに前の版のまま起きる'; do
    [[ -n "$(scenario_db_prep "$prep_name")" ]] || die "DB の細工が要る Scenario に下準備が無い: $prep_name"
  done
  grep -Fq 'migration_ledger.applied' <<<"$(scenario_db_prep 'DB に新しい版の知らない版があると、戻さずに前の版のまま起きる')" ||
    die '知らない版の下準備が版の記録を触っていない'
  grep -Fq -- "-dev." <<<"$(scenario_db_prep '系統が違う DB には、戻さずに前の版のまま起きる')" ||
    die '系統違いの下準備が開発版の系統にしていない'
  [[ -z "$(scenario_db_prep '新しい版のインストーラで走らせると、移行を当てて新しい版のサーバが起きる')" ]] ||
    die '通常更新の Scenario が DB を細工している'
  [[ "$(scenario_action '印が無ければ、戻しの口は何も替えない')" == rollback ]] ||
    die '印無しの戻し Scenario が rollback 操作を使っていない'
  echo 'OK: update-apply-install の自己試験'
}

[[ "${1:-}" == --list && $# == 1 ]] && { list_scenarios; exit 0; }
[[ "${1:-}" == --self-test && $# == 1 ]] && { self_test; exit 0; }
[[ $# == 2 || $# == 4 || $# == 6 ]] || usage
run_windows_check "$@"
