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
  openspec/changes/st<NN>-<slug>/ を作る
  deep → proposal → specs → design → tasks → apply → archive
```

**`deep`（深掘り）は人間に問う工程で、AI が代わりに決めてはいけない。**
`openspec/schemas/ashiato/schema.yaml` で `proposal` の前提にしてあるので、
深掘りを飛ばして proposal を書くことはできない（`openspec validate` が落ちる）。

> なぜ器を作ったか: **一度飛ばされたから。** OpenSpec 既定の `spec-driven` スキーマには
> `deep` に対応する artifact が無く、スキーマに沿って書くと**黙って消えた**。
> 実装者が 8 件の技術判断を独断で決め、うち 4 件は本人が決めるべきものだった
> （うち 1 件は要件どうしの矛盾を独断で解いたもので、不可逆）。
> 文書に名前があるだけで機械の側に受け皿が無い工程は消える。`deep` の中身は
> `openspec/changes/*/deep.md`、問い方の規範は `grilling` skill。

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

それまでの代わりが **`.githooks/pre-commit`** —— main / develop での commit を拒否する。
clone 後に 1 度だけ有効化する:

```bash
git config core.hooksPath .githooks
```

例外は `HARNESS_ALLOW_PROTECTED_COMMIT=1 git commit ...`。

> hookify では代用できない。hookify の条件はコマンド文字列（`command` / `file_path` /
> `new_text` …）にしか正規表現を当てられず、**「いまどのブランチにいるか」を見る手段が無い**
> （`core/config_loader.py`）。git hook なら 5 行で済み、Claude 以外の経路にも効く。

## Story ごとの進め方（上流 → 下流）

**上流と下流を分けて、下流が走っている間に次の Story の上流を進める。**
分割点は `openspec/changes/<change>/tasks.md` —— これが上流の最後の成果物であり、
下流の唯一の入力になる。

```
上流（main の作業ツリー）              下流（別の worktree）
─────────────────────────            ─────────────────────────
docs/st<NN>-upstream を切る
openspec: proposal → specs
        → design → tasks
PR → CI 緑 → merge
gh issue create（tasks.md を本文に）
                            ────→  git worktree add ../st<NN> feat/st<NN>-<slug>
次の Story の上流へ                    別セッションで実装
（proposal まで。specs は                openspec apply
 前の Story が merge されるまで待つ）     /commit-push-pr
                                       PR まで出す ★ merge が停止点
```

### 下流の起動は 1 コマンド

```bash
scripts/story.sh ST01          # worktree を用意して、その中で claude を起動する
```

worktree が無ければ `feat/<change名>` で作り、あれば main に追従させてから入る。
最後に `claude --permission-mode auto "/story ST01"` を exec するので、
**新しいセッションが auto mode で始まる**。

- **新しいセッション**である必要がある —— プラグインはセッション開始時に読み込まれる
- **auto mode** を明示する必要がある —— 既定は manual で、放っておくと毎アクション確認になる
  （`--permission-mode` は `claude --help` に出ないが実在する。2.1.226 で実測。
  取りうる値は `acceptEdits` / `auto` / `bypassPermissions` / `manual` / `dontAsk` / `plan`）

モードを変えたいときは `STORY_PERMISSION_MODE=manual scripts/story.sh ST01`。

すでに worktree の中にいるなら、セッション内で `/story ST01` だけでよい。
`/story` は **場所の確認 → issue と deep.md と tasks.md を読む → 規律を敷く →
tasks を順に進める → PR まで出す** をやる。工程を発明はしない
（進め方の実体は Story ごとの成果物が持っている）。

**なぜ specs を待つか**: 実装は spec の穴を開ける（実測: 1 Story あたり 4 件、
うち 1 件は実装が黙って決めた設計判断）。ST01 は土台なので、ここが動くと
後続の spec が古くなる。proposal（何を・なぜ）は実装詳細に依存しないので先に書ける。

**worktree にする理由**: 同じディレクトリで 2 セッションが git を触ると壊れる。

**停止点は 1 つだけ**: **merge**。そこまでは人間を待たずに走り切り、PR を出す。
CI が落ちたら自分で直す。それ以外は推奨 default を採って進み、決めたことを記録する。
止まらないのではなく、**1 か所だけで止まる**。

例外が 1 つ。**`deep.md` に人間へ返す項目が積まれたときは `--draft` で PR を出す**
（本文の冒頭に未決を列挙する）。未決でも**作業は捨てない** —— 手元に抱えたまま止まると、
次のセッションが状況を復元できない。
