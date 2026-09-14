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

検査: `python3 scripts/check_chain.py`（要件 → Story の鎖と、`stories.json` からの再生成との一致）

**レビューと関門は `docs/flow-gates.md`。** 成果物ごとに独立レビュー（`deep-review` / `spec-review` /
`code-verify` + `pr-review-toolkit`）→ 指摘 1 件ごとに `処置:`（`review_triage.py` が処置の無い指摘を止める）
→ `scripts/merge_gate.sh`（その head の CI・tasks・検査・未回答。落ちれば draft に戻す）→ 人間が merge
→ `scripts/archive.sh`。**人間は draft でない PR だけを merge する。**

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
上流（worktree ../ashiato2-up-st<NN>）   下流（worktree ../ashiato2-st<NN>）
─────────────────────────            ─────────────────────────
docs/st<NN>-upstream を切る
openspec: proposal → specs
        → design → tasks
PR + issue（issue_body.py。merge を待たない）
merge_gate → CI 緑 → merge
                            ────→  git worktree add ../st<NN> feat/st<NN>-<slug>
次の Story の上流へ                    別セッションで実装
（proposal まで。specs は                openspec apply
 前の Story が merge されるまで待つ）     /commit-push-pr
                                       PR まで出す ★ merge が停止点
```

### 起動はどちらも 1 コマンド

```bash
scripts/upstream.sh ST02       # 上流: worktree（../ashiato2-up-st02）に docs/st02-upstream を用意して起動
scripts/story.sh ST01          # 下流: worktree を用意して、その中で起動
```

`/story-upstream` は **ブリーフ →（画面があれば proto）→ deep → proposal → specs → design → tasks → PR + issue**。
`/story` は **issue と deep.md と handoff を読む → tasks を順に → PR**。どちらも停止点は merge。
**issue は PR と同時に作る**（`scripts/issue_body.py` が本文を機械的に出し、`merge_gate.sh` が貼り直す）。
merge の後に作る規則だと、上流のセッションは PR で止まるので作る係がいなくなる
（実測: ST02 は merge から issue まで 10 時間空いた）。**merge_gate が OK のとき「次の 1 手」を印字する**
（上流なら `scripts/story.sh` と盤面が出す次の `scripts/upstream.sh`、下流なら確認バッチと `scripts/archive.sh`）。

### 何を並列で始めてよいかは盤面が決める

```bash
python3 scripts/board.py                          # 状態ごとの一覧と「いま同時に始められる上流」
python3 scripts/board.py --html docs/briefs/board.html   # スマホで見るなら
```

並列にしてよいのは **2 つとも**満たすもの同士: (1) `requires` が全部 archive 済み（specs を最後まで書ける）、
(2) capability（`docs/stories/INDEX.md` の表）が走っている Story と重ならない。番号順ではない。
同じ capability を 2 本が同時に触ると差し戻しが起きる（実測: ST02 と ST03 が登録簿を共有し、12 時間で 5 往復）。
`衝突待ち` と出た Story は始めない。

### 確認は Story ごとにしない。バッチで 1 回

Story の下流が終わっても**人間に動作確認を求めない**。gate を通った PR をまとめて `/verify`（確認バッチ）にかける:

```bash
scripts/verify_batch.sh          # verify/<tag> を切り、ready な feat/st* の PR を全部 merge
                                 # → tools/verify-prep.sh（server の release / web の build / APK / run.sh / manifest）
                                 # → 手順書 docs/briefs/verify-<tag>.html（完了の判定と「人間の確認待ち」から機械的に）
                                 # → draft の PR
./dist/verify-<tag>/run.sh       # 人間: DB → サーバ → 画面 → 偽データ を 1 コマンドで起動して、手順書のとおりに見る
python3 scripts/verify_record.py <tag> answers.txt   # 貼り戻しを tasks.md と docs/verify/<tag>.md に記録 → gh pr ready
```

停止点は verify の PR の merge。merge 後に Story ごとに `scripts/archive.sh ST<NN>`（Story の PR を閉じ、worktree を片付ける）。
**準備（ビルド・起動・手順書・実機への APK）は AI が済ませる。** 人間がやるのは run.sh を叩いて見ることだけ。

**人間の確認は正しさのテストではない**（2026-09-14）。正しさは単体・結合・実行時テストが持つ ——
Windows の OS を触る部分は `windows-latest` の実行時テスト（テストが自分で窓を作って本物の前景を読ませる）、
Android は エミュレータの計測テスト（実機を繋いでも同じものが走る。実機の確認は最小にする）。
「人間の確認待ち」に残せるのは機械が再現できない物理的な操作（ロック・スリープ・電池・本物の GPS・時間そのもの）だけで、
手順書はそれに加えて Story ごとに 1 問「触ってみて違和感は無かったか」を聞く。

深掘りの前に **Story ブリーフ**を出す。人間が「この Story は何か」を知らないまま
一方通行の判断を求められる状態を避けるため。

```bash
python3 scripts/story_brief.py ST02 --open   # docs/briefs/ST02.html
```

`docs/stories/ST<NN>.md` と `INDEX.md` から**機械的に引くだけ**で、要約も推測も足さない
（扉の判断の土台になる文書なので、生成側の解釈が混ざると事実として読まれる）。
`docs/briefs/` は生成物なので commit しない。

### 深掘りの問いも HTML で渡す

**1 問ずつ会話で聞かない。** 論点が揃ったら 1 枚にまとめる。

```bash
python3 scripts/ask_wizard.py --example > /tmp/q.json   # 入力の形
python3 scripts/ask_wizard.py /tmp/q.json -o docs/briefs/ST<NN>-deep.html
```

`SendUserFile`（`display: "render"`）で渡す → タップで選ぶ → 「回答をコピー」→
**その文字列をセッションに貼り戻す**。戻りの形は固定で、未回答も分かる。

```
=== ST02 の深掘り の回答 ===
Q1 [稼働記録の日付境界] -> 端末のタイムゾーンで区切る
  補足: 深夜をまたぐ行動が多い
Q2 [感度の既定] -> (未回答)
```

- **一覧にする理由**: 1 問ずつだと前の問いの文脈を抱えたまま次を読むことになり、
  長い深掘りほど答えが雑になる
- **サーバを立てない理由**: PC のセッションをスマホから見ることが多く、`localhost` は届かない
- 各問いに `kind`（`irreversible` / `conflict` / `daily` / `premise` / `visual` / `open`）を付ける。
  **分類できない問いは、たいてい人間に聞く必要が無い**。
  `premise` は「**既に決めたことの根拠が事実と違っていた**」（実測: ST02 第 8 回 Q30、ST03 第 2 回の 7 件）
- **問いは 2 段。判定の言葉は「後から答えを変えたら何が失われるか」**（2026-09-12。ST02 / ST03 の振り返り）
  - **A 止める** —— `loss` を持つ問い（`uncaptured` 取っていないデータ / `discarded` 捨てた・拒んだ /
    `exported` 外に出た / `rewrite-all` 凍結した全行の書き直し）と `premise` / `visual`。未回答なら merge_gate が止める
  - **B 仮でよい** —— 計算し直せば戻る（判定式・閾値・順序・表示・導出の規則）。推奨を既定にし、
    未回答なら推奨を採ったと読む。下流では AI が仮で決め、`design.md` に `D<n>（仮）` と反転条件を書き、
    PR 本文に列挙する。人間は merge のときに 1 回で見る
  - **C 聞かない** —— 片方の選択肢が扉を開けたままにし、費用が小さいもの。「扉を開けたままにする既定」
    （細かい粒度で持つ / 列を持つ / 鍵でなく索引 / 台帳は追記のみ / 捨てるより印を付けて入れる /
    既定は厳しい側）を当てて D 番号に残すだけ
  - `kind: irreversible` は **`loss` が必須**。名付けられない不可逆は不可逆ではない
    （実測: ST02 の「不可逆」14 問のうち、失われるものがあったのは 4 問）
- **画面の構造は文字で問わない。** 画面を持つ Story（ブリーフの「面」）は問いより先に
  `docs/briefs/ST<NN>-proto.html` を `playground` skill で作り、問いは `kind: visual` + `proto` で
  その HTML を埋め込む（実測: ST02 は格子だけで 8 問・6 回を使い、2 問は絵があれば要らなかった）

**上流は先行 Story が merge されるまで `deep` と `proposal` で止まる。** 機械的に
書けない（先行が archive されるまで capability が `openspec/specs/` に無いので
`MODIFIED` を書けない）うえ、実装が spec の穴を開けるので書いても古くなる。

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

**worktree にする理由**: 同じディレクトリで 2 セッションが git を触ると壊れる。上流も Story ごとの worktree に分けるので、
盤面が「同時に始められる」と出した上流どうしを実際に並べられ、main の作業ツリーは main のまま残る
（2026-09-15。以前は上流が main の作業ツリーでブランチを切り替えていて、コンソールが 2 本目の上流を拒んだ）。

**停止点は 1 つだけ**: **merge**。そこまでは人間を待たずに走り切り、PR を出す。
CI が落ちたら自分で直す。それ以外は推奨 default を採って進み、決めたことを記録する。
止まらないのではなく、**1 か所だけで止まる**。止まる印は **draft**（`merge_gate.sh` が付け外しする）。
`closes #N` は残タスク 0 のときだけ。それ以外は `refs #N`（issue が残タスクの入口）。

例外が 1 つ。**`deep.md` に人間へ返す項目が積まれたときは `--draft` で PR を出す**
（本文の冒頭に未決を列挙する）。未決でも**作業は捨てない** —— 手元に抱えたまま止まると、
次のセッションが状況を復元できない。

**走っている Story へは差し戻さない**（2026-09-12）。issue ができた時点でその Story の `tasks.md` は凍結。
他の Story が見つけた事は、見つけた側の change で直すか、先行の merge 後に `fix/` で拾う
（`docs/handoff/ST<NN>.md` に書き、`処置: followup ST<NN>`）。例外は失われるもの（A）だけで、
それは tasks ではなく先行 Story の deep の問いとして立てる。下流が handoff を読むのは開始時と PR 前の 2 回。
`review_triage.py` は凍結された Story への `deferred` を FAIL にする。
実測: ST03 の上流が ST02 の下流に 5 件を差し戻し、12 時間で 5 往復した。

**移行の名前は作成時刻**（`YYYYMMDDHHMM_<slug>.sql`。連番にしない —— 並走する Story が番号を取り合う）。
`tools/check-migrations.sh` が形を見る。適用の順は `crates/server/src/lib.rs` の `MIGRATIONS` 配列。
