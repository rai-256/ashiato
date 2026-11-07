#!/usr/bin/env bash
# St56BackupTest のバックアップ採取・データ消去・復元を、ローカルと CI で同じ条件で行う。
set -euo pipefail

PACKAGE=dev.ashiato.collector

for _ in $(seq 1 30); do
  adb shell bmgr transport com.android.localtransport/.LocalTransport | grep -q 'Selected transport' && break
  sleep 2
done
backup_out=$(adb shell bmgr backupnow --non-incremental --monitor @pm@ "$PACKAGE" | tr -d '\r')
echo "$backup_out"
grep -qx "Package $PACKAGE with result: Success" <<<"$backup_out" \
  || { echo "error: St56BackupTest のバックアップが取れない" >&2; exit 1; }

adb shell run-as "$PACKAGE" mkdir -p files shared_prefs databases cache code_cache no_backup
for dir in files shared_prefs databases cache code_cache no_backup; do
  adb shell run-as "$PACKAGE" find "$dir" -mindepth 1 -delete
done

restore_out=$(adb shell bmgr restore 1 "$PACKAGE" --monitor | tr -d '\r')
echo "$restore_out"
grep -qx 'restoreFinished: 0' <<<"$restore_out" \
  || { echo "error: St56BackupTest の復元が終わらない" >&2; exit 1; }
