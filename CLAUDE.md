# ashiato2

行動履歴（足跡）を記録・可視化するライフログ系プロジェクト。
**harness2（つぎはぎハーネス）で上流から作り直す試走**。

## 応答言語

ユーザーへの応答は **常に日本語**。例外はコード・識別子・パス・コマンド・ツールの生出力のみ。

## 構成

| パス | 何か | 書き込み |
|---|---|---|
| `docs/` | フローが作る成果物 | する |
| `openspec/` | Story ごとの設計と正典 | する（`openspec` CLI が管理） |
| `src/` `tests/` | 実装 | する |
| `.claude/skills` → `~/dev/harness2/skills` | skill（symlink） | harness2 側で直す |
| `AGENTS.md` → `~/dev/harness2/codex/AGENTS.md` | Codex 下流の project 指示（symlink） | 同上 |
| `.agents/skills` → `~/dev/harness2/codex/skills` | Codex の skill（symlink） | 同上 |
| `scripts` → `~/dev/harness2/scripts` | 検査スクリプト（symlink） | 同上 |

**旧ハーネス（`.harness/`）は持ち込んでいない。** hook もレビューキューも無い。

**旧 ashiato からの参考資料（`reference/`）はこのリポジトリに無い。** 公開にあたって全履歴から消した
（本人の討議ログなので外に出さない）。文書に出てくる `旧 ashiato の …` という出所は、手元の
`~/dev/ashiato/` を指す。**出所の検算はそこでしかできない。**

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

**裏返しの規則: 機械が判定できることは、機械に決めさせる**（2026-09-18）。
同じ判定材料で結論まで出せるなら、「検査で止める」「規約に書く」「人間に確認してもらう」で代替しない。
実測: `story-codex.sh --here` が main から起動できる問題に「拒否する」を当てようとしたが、
`git branch --show-current` 1 本で**正しい場所へ回せた**（拒否は正しい手順を人間に覚え直させる）。
同じ型が確認バッチにも残っている —— 直近 2 回の 15 問のうち 6 問は画面の Scenario で、
本物のブラウザなら機械が判定できる（`docs/testing.md` §4 の「実寸は人間の確認待ち」）。

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
| **下流の実装の回し方** | **`superpowers:subagent-driven-development`（そのまま）** | 独自の実装オーケストレーション |
| skill の書き方 | `skill-creator` | `superpowers:writing-skills` |
| 並列 | **SDD の Task ループだけ**（implementer は 1 本ずつ。reviewer は SDD が決める） | `dispatching-parallel-agents` |

superpowers から実際に使うのは **6 本**:

| skill | いつ | 使い方 |
|---|---|---|
| **`subagent-driven-development`** | **下流の実装ぜんぶ** | `/story` が起動する。Task ループ・fix loop・ledger・review package・breaker は**全部これが持つ**。付属 prompt（`implementer-prompt.md` / `task-reviewer-prompt.md` / `re-review-prompt.md`）と付属スクリプト（`sdd-workspace` / `task-brief` / `review-package`）を**書き換えずに**使う |
| **`requesting-code-review`** | **全 Task 完了後の whole-branch review 1 回だけ** | SDD が `code-reviewer.md` を指すので、その呼び出し関係のまま。**Task ごとに重ねて呼ばない**（Task ごとは SDD の task reviewer） |
| `test-driven-development` | 実装（implementer の dispatch に入れる） | |
| `verification-before-completion` | 完了の申告の前 | |
| `receiving-code-review` | レビュー指摘を受けたとき。**鵜呑みにも空返事にもしない**規範 | |
| `finishing-a-development-branch` | **使わない。** この repo の統合は PR → `merge_gate.sh` → 人間の merge で決まっている | |

`using-git-worktrees` は Story 並列をやるなら要る。**いまはやらないので保留**（不採用ではない）。

その他の使い分け:

- 画面の**方向を決める**のは `/ui-direction`、**コードに落とす**のは `frontend-design` skill
- **下流のコードレビューは SDD の 3 席 + `code-verify` の 4 席**（`docs/flow-gates.md`）。
  `pr-review-toolkit` は下流の既定から外した（`code-reviewer.md` と重複する）——
  PR そのものを見たいときに人間が `/pr-review-toolkit:review-pr` を呼ぶ
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

## Story ごとの進め方（上流 → 下流）—— LangGraph の story グラフが回す

**流れの位置・待ち・失敗・再実行点は LangGraph が持つ**（2026-09-24。`~/dev/harness2/graph/`、図は `GRAPH.md`）。
thread 1 本 = Story 1 本。工程の中身は skill と SDD、外への作用は `scripts/` の台本、正本は artifact。

```
observe ─ admission ⟲ ─ upstream ─ wait_upstream_merge ⟲ ─ downstream ─ wait_verified ⟲ ─ archive
  upstream   = prepare → questions → ask(人間) → record → spec → publish → gate ⇄ fix
  downstream = prepare → sdd（superpowers:subagent-driven-development）→ [ask → record → sdd] → publish → gate ⇄ fix
```

分割点は `openspec/changes/<change>/tasks.md` —— 上流の最後の成果物であり、下流の唯一の入力。
上流は `../ashiato2-up-st<NN>` の `docs/st<NN>-upstream`、下流は `../ashiato2-st<NN>` の `feat/<change>`。

```bash
scripts/hx dev                  # LangGraph サーバ（langgraph dev）。Studio の URL が出る
scripts/hx start ST02           # thread を始める。入口は観測で決まる（上流から / 下流から / 確認待ちから）
scripts/hx status               # 位置・待ち（何を待っているか）・失敗（どの node で）
scripts/hx answer ST02 a.txt    # 深掘りの答え（ask_wizard の「回答をコピー」）で interrupt を解く
scripts/hx poke                 # 待ちの thread に条件を見直させる（merge した・先行が archive された）
scripts/hx retry ST02           # 落ちた node から再実行
```

- **工程間で会話を引き継がない。** agent の node（questions / record / spec / sdd / fix）は毎回 fresh な
  `claude -p`（`--permission-mode auto`。変えるなら `HARNESS_PERMISSION_MODE`）で、渡すのは skill と artifact のパスだけ。
  人間の答えは `deep-answers-<n>.txt` に落ちて、別のセッションが読む
- **分岐は artifact の観測で決める。** agent が「できた」と言っても、その stage の artifact（tasks.md など）が
  無ければ node が落ちる
- **issue は PR と同時に作る**（`publish.sh` が `issue_body.py` で）。merge の後に作る規則だと作る係がいなくなる
  （実測: ST02 は merge から issue まで 10 時間空いた）
- gate が落ちたら fresh な agent（`fix`）が gate の報告書を読んで直し、publish → gate をやり直す。
  3 回で通らなければ人間を待つ（kind=wait）

### 何を並列で始めてよいかは admission が決める

`admission` node が `board.py` の `admit()` で判定し、駄目なら理由つきで待つ（`hx poke` で見直す）。
並列にしてよいのは **2 つとも**満たすもの: (1) `requires` が全部 archive 済み（specs を最後まで書ける）、
(2) capability（`docs/stories/INDEX.md` の表）が走っている他の Story と重ならない。番号順ではない。
同じ capability を 2 本が同時に触ると差し戻しが起きる（実測: ST02 と ST03 が登録簿を共有し、12 時間で 5 往復）。
requires が「上流済み・archive 前」なら deep と proposal まで書いて待つ。
thread の無い Story も含めた一覧は `python3 scripts/board.py`。

### 確認は Story ごとにしない。バッチで 1 回（verify グラフ）

Story の下流が gate を通っても**人間に動作確認を求めない**。Story の thread は `wait_verified` で待つ。

```bash
scripts/hx verify                # batch: verify/<tag> に gate を通った feat/st* を merge → tools/verify-prep.sh
                                 #   → 手順書 → draft の PR。check で人間を待つ
./dist/verify-<tag>/run.sh       # 人間: DB → サーバ → 画面 → 偽データ を 1 コマンドで起動して、手順書のとおりに見る
scripts/hx answer verify-<時刻> a.txt   # record: tasks.md と docs/verify/<tag>.md に記録 → 通れば ready
```

停止点は verify の PR の merge。merge 後に `hx poke` で各 Story の thread が archive へ進む。
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

グラフの `ask` が HTML のパスを出して待つ → タップで選ぶ → 「回答をコピー」→
**その文字列で resume する**（`scripts/hx answer ST<NN> <file>` か Studio）。戻りの形は固定で、未回答も分かる。

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
    `exported` 外に出た / `rewrite-all` 凍結した全行の書き直し）と `premise` / `visual`。未回答ならグラフが同じ問いを聞き直す
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

**上流は先行 Story が archive されるまで `deep` と `proposal` で止まる。** 機械的に
書けない（先行が archive されるまで capability が `openspec/specs/` に無いので
`MODIFIED` を書けない）うえ、実装が spec の穴を開けるので書いても古くなる。

**`/story` は実装の回し方を持たない。** 回すのは `superpowers:subagent-driven-development`（SDD）——
**Task ごとに fresh な implementer** を出し、**Task ごとに独立の task reviewer**（`task-reviewer-prompt.md`）に
かけ、直しがあれば scoped re-review、全 Task 完了後に **whole-branch review**
（`requesting-code-review` の `code-reviewer.md` ＋ `code-verify`）を通す。
`/story` が持つのは **SDD に渡す 4 つの値**（PLAN_FILE = `tasks.md` / Global Constraints /
担当する Task / 完了の記録先）と、whole-branch review の処置（`review/code.md` → `review_triage.py`）だけ。
push・PR・gate はグラフの node。

| 誰が | 文脈 | 渡されるもの | 渡されないもの |
|---|---|---|---|
| controller（`/story`） | セッション全体 | plan・deep・handoff・ledger | Task の実装の中身 |
| implementer（Task ごとに新規） | **その Task だけ** | brief file・界面・global constraints・report file のパス | 前の Task の会話、plan 全文 |
| task reviewer（Task ごとに新規） | **その diff だけ** | brief file・report file・review package・global constraints | **implementer の推論と自己正当化** |
| final reviewer / code-verify | ブランチ全体 | review package・plan・ledger の parked / deferred | 同上 |

**`tasks.md` の `- [x]` は controller だけが付ける。** implementer は `tasks.md` を触らない ——
実測 2026-09-22（ST08）: 実装者自身が付けていたので、**存在しないテスト名**
（`cargo test window_request_body_is_unchanged` は 0 本で rc=0）や、tasks 本文と違う
（通るほうの）コマンドを走らせた行まで `[x]` になり、独立レビューが 36 件を出した。

**`tasks.md` の見出しは `## Task <N>: <名前>`**（SDD 付属の `task-brief` がこの形しか読まない）、
規律の節は `## Global Constraints`（reviewer へ逐語でコピーする節）。
Task の粒度は **checkbox 1 つではなく、見出し 1 つ**（1 Story あたり 8〜12）。

**なぜ specs を待つか**: 実装は spec の穴を開ける（実測: 1 Story あたり 4 件、
うち 1 件は実装が黙って決めた設計判断）。ST01 は土台なので、ここが動くと
後続の spec が古くなる。proposal（何を・なぜ）は実装詳細に依存しないので先に書ける。

**worktree にする理由**: 同じディレクトリで 2 セッションが git を触ると壊れる。上流も Story ごとの worktree に分けるので、
admission が通した Story どうしを実際に並べられ、main の作業ツリーは main のまま残る（`hx dev` は 8 run を並列に回す）。

**人間が止める場所は 3 種類だけ**: 深掘りの答え（A）、**merge**、確認バッチ。それ以外は推奨 default を採って進み、
決めたことを記録する。止まる印は thread の interrupt と、PR の **draft**（`merge_gate.sh` が付け外しする）。
`closes #N` は残タスク 0 のときだけ。それ以外は `refs #N`（issue が残タスクの入口）。

下流で **`deep.md` に人間へ返す項目（A）が積まれたら**、`/story` は問いの HTML を作って終え、グラフが `ask` で待つ。
答えを `record` が `deep.md` に書いた後、`/story` をもう一度呼ぶ（SDD の ledger から続きをやる）。
未決でも**作業は捨てない** —— 済んだ Task は commit されている。

**走っている Story へは差し戻さない**（2026-09-12）。issue ができた時点でその Story の `tasks.md` は凍結。
他の Story が見つけた事は、見つけた側の change で直すか、先行の merge 後に `fix/` で拾う
（`docs/handoff/ST<NN>.md` に書き、`処置: followup ST<NN>`）。例外は失われるもの（A）だけで、
それは tasks ではなく先行 Story の deep の問いとして立てる。下流が handoff を読むのは開始時と PR 前の 2 回。
`review_triage.py` は凍結された Story への `deferred` を FAIL にする。
実測: ST03 の上流が ST02 の下流に 5 件を差し戻し、12 時間で 5 往復した。

**移行の名前は作成時刻**（`YYYYMMDDHHMM_<slug>.sql`。連番にしない —— 並走する Story が番号を取り合う）。
`tools/check-migrations.sh` が形を見る。適用の順は `crates/server/src/lib.rs` の `MIGRATIONS` 配列。
