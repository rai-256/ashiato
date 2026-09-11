# 確認バッチの記録

Story ごとには動作確認をしない。gate を通った下流の PR をまとめて `verify/<tag>` ブランチにし、
ビルド済みの成果物と確認手順書を作ってから、人間が **1 回だけ**確かめる（2026-09-12）。

| ファイル | 何か | 誰が書くか |
|---|---|---|
| `<tag>.json` | 手順書の問い（ask_wizard の入力）。Story の完了の判定と tasks の「人間の確認待ち」から機械的に | `scripts/verify_checklist.py` |
| `<tag>.md` | 答えの記録。本人の貼り戻しをそのまま写す | `scripts/verify_record.py` |

流れ: `scripts/verify_batch.sh` → `docs/briefs/verify-<tag>.html` を渡す → 貼り戻し →
`scripts/verify_record.py <tag>` → `gh pr ready` → 人間が merge → `scripts/archive.sh ST<NN>`。

成果物は `dist/verify-<tag>/`（`tools/verify-prep.sh` が作る。gitignore）。
`run.sh` が DB → サーバ（release）→ 画面（build 済み）→ 偽データ を 1 コマンドで起動する。
