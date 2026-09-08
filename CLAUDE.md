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
| 並列 | 当面**使わない**（旧ハーネスで肥大した領域） | `dispatching-parallel-agents` |

superpowers から実際に使うのは **5 本**:

| skill | いつ |
|---|---|
| `test-driven-development` | 実装 |
| `verification-before-completion` | 実装の完了判定 |
| `requesting-code-review` | PR を出す前 |
| `receiving-code-review` | レビュー指摘を受けたとき。**鵜呑みにも空返事にもしない**規範 |
| `finishing-a-development-branch` | 実装が終わって **main へどう統合するか**を決めるとき |

`using-git-worktrees` は Story 並列をやるなら要る。**いまはやらないので保留**（不採用ではない）。

その他の使い分け:

- 画面の**方向を決める**のは `/ui-direction`、**コードに落とす**のは `frontend-design` skill
- PR レビューは `pr-review-toolkit`（agent 6 本）を本体とする
- `security-guidance` は既定のまま。**ターン終了ごとと commit ごとに LLM を呼ぶ**ので、
  コードを書き始める前にコスト設定を決める（CATALOG.md の表を見る）


## git 運用

**Story の作業は `main` で直接やらない。** `feat/st<NN>-<slug>` を切って PR にする。
理由は CI —— `.github/workflows/ci.yml` は `pull_request` トリガを持っているが、
PR が無いと `push: [main]` しか発火せず、**落ちたときには main がもう汚れている**。
PR にして初めて CI が門番になる。

| やること | 使うもの |
|---|---|
| ブランチを切って commit → push → PR | `/commit-push-pr`（main にいれば自動でブランチを切る） |
| 単発の commit | `/commit`（直近のコミットメッセージの文体に合わせる） |
| merge 後の後片付け | `/clean_gone`（remote で消えたブランチと worktree を掃除） |
| main へどう統合するか迷ったら | `superpowers:finishing-a-development-branch` |

コミットメッセージは `feat:` / `fix:` / `docs:` / `chore:` + **日本語 1 行**。
本文は「なぜ」を書く。`/commit` が直近を読んで踏襲する。

**ブランチ保護は掛けられない。** private + 無料プランでは GitHub API が 403 を返す（実測）。
AGPL-3.0 で公開したら無料で使えるようになるので、そこで機械強制へ切り替える。
それまでは規約と CI で運用する。
