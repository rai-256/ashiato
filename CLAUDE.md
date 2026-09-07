# ashiato2

行動履歴（足跡）を記録・可視化するライフログ系プロジェクト。
**harness2（つぎはぎハーネス）で上流から作り直す試走**。

## 応答言語

ユーザーへの応答は **常に日本語**。例外はコード・識別子・パス・コマンド・ツールの生出力のみ。

## 構成

| パス | 何か | 書き込み |
|---|---|---|
| `reference/` | 旧 ashiato からの参考資料 | **しない**（読むだけ） |
| `docs/` | フローが作る成果物 | する |
| `openspec/` | Story ごとの設計と正典 | する（`openspec` CLI が管理） |
| `src/` `tests/` | 実装 | する |
| `.claude/skills` → `~/dev/harness2/skills` | skill（symlink） | harness2 側で直す |
| `scripts` → `~/dev/harness2/scripts` | 検査スクリプト（symlink） | 同上 |

**旧ハーネス（`.harness/`）は持ち込んでいない。** hook もレビューキューも無い。

## フロー

```
/requirements    →  docs/requirements.md
/stories         →  docs/stories/ST*.md + INDEX.md
【製造準備・1 度だけ】
/ui-direction    →  docs/ui-direction.md + preview.html
/production-prep →  docs/production-prep.md + 実物
【Story ごと】
  openspec new change st<NN>-<slug>
  context → deep(grilling) → proposal → specs → tasks → apply → archive
```

検査: `python3 scripts/check_chain.py`

## skill の起動

`.claude/skills` は harness2 への symlink なので `/requirements` `/stories`
`/ui-direction` `/production-prep` として起動できる。
