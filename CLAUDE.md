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

## 借り物の優先順位（superpowers との決着）

外部プラグインを 12 本入れている。一覧・出所・落とし穴は **`~/dev/harness2/CATALOG.md`** が単一情報源。

`superpowers` プラグインは毎セッション「1% でも該当しそうな skill は必ず先に呼べ」という規範を
注入するが、**本プロジェクトでは以下が優先する**（`using-superpowers` 自身が
「CLAUDE.md > skill」と定めているので、これが正しい決着の付け方）。

| 領域 | 採るもの | 採らないもの |
|---|---|---|
| 要件の掘り方 | `grilling`（harness2 にベンダリング） | `superpowers:brainstorming` |
| 計画の器 | OpenSpec の change（proposal / specs / tasks） | `superpowers:writing-plans` / `executing-plans` |
| skill の書き方 | `skill-creator` | `superpowers:writing-skills` |
| 並列・worktree | 当面**使わない**（旧ハーネスで肥大した領域） | `dispatching-parallel-agents` / `using-git-worktrees` |

superpowers から実際に使うのは **`test-driven-development` と `verification-before-completion` の 2 本**。

その他の使い分け:

- 画面の**方向を決める**のは `/ui-direction`、**コードに落とす**のは `frontend-design` skill
- PR レビューは `pr-review-toolkit`（agent 6 本）を本体とする
- `security-guidance` は既定のまま。**ターン終了ごとと commit ごとに LLM を呼ぶ**ので、
  コードを書き始める前にコスト設定を決める（CATALOG.md の表を見る）
