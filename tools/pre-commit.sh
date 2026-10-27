#!/usr/bin/env bash
# ashiato2 固有の commit 前の検査。harness2 の `.githooks/pre-commit`（main の保護）が最後に呼ぶ。
# 例外:   HARNESS_ALLOW_PRIVATE_NAMES=1 git commit ...
set -euo pipefail

# 手元の網の名前・私設 IP を**入る前に**止める。
# `.gitignore` では守れない —— 実際に漏れていたのは追跡済みの `docs/` と `openspec/` の文書で、
# 公開準備の commit も 6 か所中 3 か所しか直せていなかった（実測 2026-09-19）。
# CI（chain job）でも同じ検査を掛けるが、**Actions の予算が尽きている間はここだけが効く。**
if [[ "${HARNESS_ALLOW_PRIVATE_NAMES:-}" != "1" ]] && [ -x tools/check-private.sh ]; then
  tools/check-private.sh --staged >/dev/null || { tools/check-private.sh --staged || true; exit 1; }
fi

# ログに私的データを出せる書き方を**入る前に**止める（製造準備 A-2。一度出たログは消せない）。
# 追跡ファイル全体を見る（1 秒かからない）。
if [ -x tools/check-log-private.sh ]; then
  tools/check-log-private.sh >/dev/null || { tools/check-log-private.sh || true; exit 1; }
fi

# 試験の一時フォルダを手で作る書き方を**入る前に**止める（CH03。落ちた試験の一時フォルダは残り、/tmp を埋める）。
# 追跡ファイル全体を、commit される中身（index）で見る（1 秒かからない）。
if [ -x tools/check-tmp-hygiene.sh ]; then
  tools/check-tmp-hygiene.sh --staged >/dev/null || { tools/check-tmp-hygiene.sh --staged || true; exit 1; }
fi

# 秘密（API キー・token・秘密鍵など、gitleaks の既定の規則の形）を**入る前に**止める。staged だけ見る（1 秒かからない）。
# 誤検知は行末の `gitleaks:allow` か `.gitleaksignore`（tools/check-secrets.sh の冒頭）。
if [ -x tools/check-secrets.sh ]; then
  tools/check-secrets.sh --staged
fi
