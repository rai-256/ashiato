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
