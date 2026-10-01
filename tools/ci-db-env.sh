#!/usr/bin/env bash
# CI の job の中で DB の合言葉を乱数から作り、$GITHUB_ENV へ渡す（ST28 / design D19）。
# **ワークフローにも配布物にも字面の合言葉を置かない。** job が終われば消える。
set -euo pipefail
: "${GITHUB_ENV:?GitHub Actions の中で走らせる}"
for name in POSTGRES_PASSWORD OWNER_DB_PASSWORD APP_DB_PASSWORD WEB_PASSWORD; do
  value="$(openssl rand -hex 24)"
  echo "::add-mask::$value"
  echo "$name=$value" >> "$GITHUB_ENV"
  declare "$name=$value"
done
{
  echo "DATABASE_URL=postgres://ashiato_app:${APP_DB_PASSWORD}@127.0.0.1:55432/ashiato"
  echo "DATABASE_OWNER_URL=postgres://ashiato_owner:${OWNER_DB_PASSWORD}@127.0.0.1:55432/ashiato"
} >> "$GITHUB_ENV"
