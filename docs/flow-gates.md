# レビューと関門 —— 成果物ごとの独立レビューと、指摘の処置

2026-09-09 に決めた。ST01 の下流と ST02 の上流を流して分かったことが根拠。

## なぜ要ったか（実測）

| 起きたこと | 何が無かったか |
|---|---|
| spec の「バイト単位で一致」が **false** なのに全テスト緑（`raw` が `jsonb`） | Scenario と test を結ぶ機構。テストが主張より低い階層を見ていた |
| 34/35・全緑の自己申告に対し、独立検証で 6 件（記録が無言で消える経路、テスト 0 本の `[x]` など） | レビューの器。`/story` は「当面は手で」と落としていた |
| 「draft に戻した」と書いたが draft になっておらず、翌日 33/44 で merge | merge の可否を機械が持つ状態 |
| PR で CI 緑 → 翌日 merge → main が赤（smoke が日付依存） | merge 時点の head で CI を見る工程 |
| `closes #2` で issue が閉じ、残 11 件の入口が消えた | 残タスクの入口を守る規則 |
| ST02 の deep が FR-33 / NFR-13 の改訂を挙げたが要件も Story も変わらず | deep から要件・Story への戻し |
| PERM-7 を改訂したのに ST28 の逐語が古いまま。`make_story.py` の入力が commit されていなかった | 判断の単一情報源（`stories.json`）と再生成の一致検査 |
| **tasks 本文が「取り込み口まで通す」「Runtime の層で固定する」と書いているのに、`runtime.rs` は 1 行も変わらず smoke は手書き JSON を POST するだけで `[x]`（ST08・36 件）** | **Task 単位の独立 spec compliance review。**要求は書いてあった —— 局所の検証（`cargo test <名前>` rc=0）だけを満たした実装を、その場で誰も brief と突き合わせなかった |

テストの書き方そのものは `docs/testing.md`（1 枚）。

**原則は 1 つ。** 成果物を作った文脈を持たない者が固定の観点で見て、根拠つきの指摘だけを出す。
直すのは作った側で、指摘 1 件ごとに処置を必ず記録する。処置の無い指摘があれば機械が止める。
`deep` を schema の artifact にしたのと同じ理由 —— **文書に名前があるだけで機械の受け皿が無い工程は消える。**

## レビューの置き場

| 成果物 | レビュアー | いつ | 出力 |
|---|---|---|---|
| requirements.md | `scripts/check_chain.py` | `/requirements` の最後、CI | stdout |
| stories | `check_chain.py`（仮の段階はこれだけ。判断の実質は deep 後の `spec-review`） | `/stories` の最後、CI | stdout |
| ui-direction | `ui-review` agent | `/ui-direction` C-4 | 会話 |
| production-prep | `security-guidance`（commit 時。実測で 2 件のバグを出した） | commit | 会話 |
| **deep の問い** | `deep-review` agent。schema の手順 1〜5 を独立にやり直す | 人間に HTML を渡す前 | `review/deep.md` |
| proposal / specs / design / tasks / 再生成後の Story | `spec-review` agent | 上流の PR 前 | `review/spec.md` |
| **コード（Task ごと）** | `superpowers:subagent-driven-development` の **task reviewer**（`task-reviewer-prompt.md`）。fix があれば **re-reviewer**（`re-review-prompt.md`） | Task が終わるたび。**controller が `[x]` を付ける前** | `.superpowers/sdd/st<nn>-task-<N>/task-<N>-findings.md`（reviewer の返答の逐語。`F<k>` の番号だけを足す）＋ ledger の round の行 |
| **コード（ブランチ全体）** | `superpowers:requesting-code-review` の `code-reviewer.md`（最上位モデル）＋ `code-verify` agent | 全 Task 完了後、PR の前 | `review/code.md` |
| PR | `scripts/merge_gate.sh`（グラフの `gate` node） | 人間が merge する前 | draft 状態 + PR コメント。落ちればグラフが `fix`（fresh な agent）→ `publish` → `gate` を回し、3 回で人間 |
| archive | `scripts/archive.sh` | 下流の merge 後 | `openspec/specs/`（正典） |

**fix round は round ごとに fresh な fixer**（2026-09-26）。前の round の会話を持ち越さないので、
指摘は会話ではなく上の写しが正本になる。未解決の一覧は `python3 scripts/fix_round.py <写し>` が逐語の記録から
導き（controller が数え直さない）、グラフの `sdd_task` が node の後に同じ台本で「写しが読めるか・未解決が
残ったまま complete になっていないか」を見る。park するなら ledger に ruling つきで残す（SDD の breaker）。

> なぜ（実測 2026-09-26、ST06 Task 7 の replay）: resume だと fix round 2 の開始時の文脈が 169k で、
> 150k を超えた 37 呼び出しだけで 6.03M。中身は初回実装の tool 履歴・test と build の出力・探索で、
> 数件の指摘を直すのに要るものではなかった。

agent は `.claude/agents/`。いずれも **`Edit` を持たない**（指摘を出すだけで直さない）。
SDD の 3 つの prompt は Superpowers のものを**そのまま**使う（`~/.claude/plugins/cache/*/superpowers/*/skills/`）。
ハーネスは prompt を書き直さない —— 書き直した瞬間、upstream の更新が届かなくなる。

## 指摘の形と処置

`openspec/changes/<change>/review/<段階>.md`:

```
## R1. <主張を 1 行で>
- 成果物: <path>
- 根拠: <file:line> / <コマンドと結果>
- kind: technical | conflict | irreversible | daily | premise | defer
- loss: uncaptured | discarded | exported | rewrite-all   （後から変えると失われるものがあるときだけ）
- 処置: fixed D16 | fixed D16 仮 | fixed 9.3 | rejected: <理由> | escalated | deferred ST04 | followup ST02
```

判定の言葉は **「後から答えを変えたら何が失われるか」**（2026-09-12。ST02 / ST03 の振り返りで、
「不可逆」と分類した 29 問のうち失われるものがあったのは 12 問だった）。

| kind | 意味 | 処置 |
|---|---|---|
| `technical` | 技術判断で閉じる | 直して D番号かタスクを指す。反証できれば `rejected:` と理由 |
| `loss` がある（**A 止める**） | 後から変えると失われる: `uncaptured` 取っていないデータ / `discarded` 捨てた・拒んだ / `exported` 外に出た / `rewrite-all` 凍結した全行の書き直し | **人間へ。** `escalated`。`deep.md` に R番号と `loss` つきで追記し `ask_wizard` の問いに。PR は draft |
| `irreversible` | **`loss` が必須。** 名付けられないなら不可逆ではない | `loss` を付けて人間へ。無ければ kind を `conflict` / `daily` に直して下の行へ |
| `conflict` / `daily`（**B 仮でよい**） | 計算し直せば戻る（判定式・閾値・順序・表示・導出）。日常に影響する選択もここ | **仮で閉じる。** `fixed D<n> 仮`。design の D<n> に（仮）と反転条件を書き、PR 本文に列挙する。人間は merge のときに見る。**印の無い `fixed` は FAIL**（要件の矛盾を黙って解くのが当初の事故） |
| `premise` | 既決の前提が崩れた（例: 列の型が `jsonb` で「そのまま残す」が成立しない） | 人間へ。直せそうでも —— 前提を直すと本人の答えが変わりうる。例外は deep 段階（問いの `context` を直して問い直す） |
| `defer` | 他 Story の担当 | その Story に **`tasks.md` がまだ無い**なら `deferred ST<NN>`。**ある**（issue 済み・凍結）なら `followup ST<NN>` にして `docs/handoff/ST<NN>.md` へ。**走っている Story へ差し戻さない** |

### レビューの席は 4 つ（増やさない）

1. **task reviewer** —— 1 Task の diff と brief だけ。spec 準拠と品質の 2 つの verdict
2. **re-reviewer** —— fix ラウンドの diff だけ。各指摘を ADDRESSED / NOT ADDRESSED
3. **final reviewer** —— ブランチ全体。plan alignment / 品質 / 設計 / test / production readiness
4. **code-verify** —— **申告と実態のずれ。** 固定値を独立に再計算し、ガードをわざと壊し、
   `[x]` の検証コマンドを実際に叩く。diff を読むだけの reviewer には出せない指摘を出す

**preflight はレビューの席ではない。** `scripts/prepare.sh downstream`（グラフの `prepare` node）が起動前に見るのは
**SDD を始められる最低条件**だけ —— `tasks.md` が機械的に読めるか（`openspec validate --strict`）、
Task brief が切り出せる形か、`tasks.md` が名指しする道具が実在するか、未処置の指摘が残っていないか。
**「要求が実装されたか」を preflight で見ない**（それは task reviewer の席で、
grep とファイル名の当て推量では接続点の名前が違うプロジェクトで空振りする）。

> **`pr-review-toolkit` の 3 agent は下流の既定から外した**（2026-09-23）。
> `code-reviewer.md` と席が重複する（plan alignment・error handling・test が本物の振る舞いを見るか）。
> PR そのもののレビューが要るときは `/pr-review-toolkit:review-pr` を人間が別に呼ぶ。

**context の隔離が席の前提。** reviewer に渡すのは brief / implementer の report file /
review package のパスと Global Constraints だけで、**implementer の推論や会話履歴は 1 行も渡さない**
（SDD の "Reviewer inputs"）。同じ文脈を共有した目は、同じ誤解を通す。

### `[x]` は controller が付ける

実装した subagent は `tasks.md` を編集しない。task reviewer が通してから controller が付ける。

> 実測 2026-09-22（ST08）: 実装者自身が付けていたので、**存在しないテスト名**
> （`cargo test window_request_body_is_unchanged` は 0 本で rc=0）の行まで `[x]` になった。採点者と受験者が同じだった。
> （訂正 2026-09-25: 以前ここに挙げていた「`check_scenarios.py . st08-browser-history`（通るほう）を走らせた」は、
> Story の範囲として**正しい検査**だった。下の「関門の 3 段」を見る。）
>
> **これは要求の欠落ではない。** Task 5 は「`tools/smoke.sh` に『サーバを止めて取得 → 起動 →
> 送信 → psql で件数』の手順を足して rc=0」「**Runtime の層で固定する**」と書き、Task 8 は
> 「注入した時計で **Runtime を 2 日回して確かめる**」と書いていた。それでもブランチ全体で
> `runtime.rs` / `engine.rs` / `main.rs` は 1 行も変わらなかった。**書いてある要求を、
> Task の時点で誰も diff と突き合わせなかった**のが穴で、そこが task reviewer の席である。

`scripts/review_triage.py . <change>` が、処置の無い指摘・指す先の不在・人間に返すべきものの `fixed`・
印の無い B の `fixed`・凍結された Story への `deferred`・
「要件へ戻すもの」の戻し漏れ（`requirements.md` に `★ 日付` の印が無い）を FAIL にする。

### 検証の証跡 —— `[x]` は controller の申告ではなく、ハーネスの記録で決まる（2026-09-25）

`tasks.md` の項目の検証コマンドは `scripts/verify-run <項目>` で走らせる。ハーネスが**本文に書かれたコマンドをそのまま**
走らせ、`openspec/changes/<change>/evidence.jsonl` に 1 行ずつ残す（コマンド・作業ツリーの tree SHA・HEAD・change・Story・
時刻・executor・環境・rc・`PASS` / `FAIL` / `BLOCKED_INFRA`・ログ）。status は機械が決める ——
`BLOCKED_INFRA` は「要る環境が無い」（`/dev/kvm` を開けない・端末が無い・出力が環境の欠落を示す）。
過去の測定値は `scripts/evidence.py invalidate` で `STALE`（参考のみ）にでき、完了の代わりには使えない。

> 実測 2026-09-25（ST06 Task 7.1）: sandbox の中からエミュレータが起動できず検証は rc=2 だったのに、controller が
> 前日に以前の実装で測った値を証跡に採用する Ruling を書いて `[x]` にした。取り直すと 510,004,080 bytes（以前は 446,653,440）。

### 凍結後に検証コマンドそのものが成立しないとき —— `scripts/plan_fix.py`（2026-09-25）

証跡の gate は tasks.md に**書かれたコマンド**しか認めない。書かれたコマンドが、環境は揃っているのに原理的に通らない
（plan の欠陥）なら、別名で別のコマンドの証跡を認めるのではなく、**検証コマンドそのものを正式に直す**:

1. controller は tasks.md を直さない。`scripts/verify-run <項目> --command '<旧>'` で、**いまのコード・環境の揃った状態の `FAIL`**
   を実証として残す（`BLOCKED_INFRA` は環境の欠落で、実証にならない）
2. premise の A として `deep.md` に積み、成立しない理由と提案する正式な入口を添えて人間に返す
3. 人間が承認したら `scripts/plan_fix.py <change> <項目> --old … --new … --reason … --approved …`。実証・入口の実在・承認を
   機械で確かめ、検証コマンドの 1 か所**だけ**を直す（受け入れ条件は変えない）。旧コマンドと理由は項目の下の注記と
   `<change>/plan-corrections.md` に残る

> 実測 2026-09-25（ST06 8.2）: 裸の `./gradlew :app:connectedDebugAndroidTest` は、2 段と `-Pashiato.baseUrl` を前提にした
> テストで 12 本中 7 本が落ちた。正式な入口は `tools/android-emulator.sh`（2 段・baseUrl つき）。

### 関門の 3 段 —— 何を見るかを混ぜない（2026-09-25）

| 段 | 見るもの | どこで |
|---|---|---|
| **Task gate** | その Task の `[x]` の項目の検証コマンドすべてに、**いまのコード**に対する `PASS` の証跡があるか（`evidence.py check --task N`）。無い・古い・`FAIL`・`BLOCKED_INFRA` なら Task は完了しない | グラフの `sdd_task` の後 |
| **Story gate** | 正典（archive 済み）+ **自分の change** の Scenario（`check_scenarios.py . <change>`）・処置・未回答・tasks の残り・`[x]` の最新の証跡が `PASS` でないもの（`evidence.py check --story`）。走っている他の Story の change では落ちない | `merge_gate.sh`、`archive.sh` |
| **Integration gate** | 統合した木で、正典 + **束ねた全 change** の Scenario（`check_scenarios.py . <束ねた change…>`）。Story どうしの食い違いはここで初めて見える。全 change の監査は `check_scenarios.py . --all` | `verify_batch.sh`（確認バッチの統合ブランチ） |

`check_scenarios.py .` を change 名なしで呼ぶと、グラフが渡す `HX_CHANGE`（実行中の Story の change）を対象にする。
`HX_CHANGE` も `--all` も無ければ、推測せずに rc=2（ブランチ名から当てない）。

## 検査の一覧

| スクリプト | 見るもの | 走る場所 |
|---|---|---|
| `check_chain.py` | 要件 → Story の鎖（8 観点。対象外は INDEX の「Story の対象外」）。`stories.json` からの再生成と一致するか | ローカル、`merge_gate`、CI（`HARNESS2_TOKEN` があるとき） |
| `check_scenarios.py` | 正典 + 対象の change の `#### Scenario:` に test の印（`Scenario: <名前>`）があるか。無いものは「人間の確認待ち」に無ければ FAIL。対象は change 名 / `HX_CHANGE` / `--all`（上の 3 段） | ローカル、`merge_gate`、`archive`、`verify_batch`（統合） |
| `evidence.py` / `verify-run` | 検証の証跡の記録（run）と判定（check --task / --story）、無効化（invalidate） | implementer と controller（run）、グラフの `sdd_task`（Task gate）、`merge_gate`（Story gate） |
| `review_triage.py` | 上の処置 | ローカル、`merge_gate`、`archive` |
| `merge_gate.sh` | head を main に追従 → その head の CI → tasks の残り → 検査 3 本 → deep の未回答。落ちれば draft に戻し、通れば ready。rc と `[FAIL]` の行でグラフが分岐する | グラフの `gate` |
| `issue_body.py` | 下流へ渡す issue の本文を deep / tasks / design / Story から機械的に出す。**上流の PR と同時に作る**（merge を待たない） | `publish.sh upstream`、`prepare.sh downstream` |
| `story_facts.py` | Story 1 本の事実（archive 済み / 上流が main に / 下流が main に / 確認済み / 着手できるか）を JSON で | グラフの `observe` / `admission` / 待ちの node |
| `archive.sh` | main に入っていること（Story の PR か、それを含む verify の PR が merge 済み）、人間の確認待ち以外が全部 `[x]`、Scenario の担保、指摘の処置 → `openspec archive` → PR → Story の PR を閉じ、下流の worktree を片付ける | 下流の merge 後 |
| `board.py` | 盤面と着手の規則 `admit()`。requires が全部 archive 済みで、capability が走っている他の Story と重ならないか | グラフの `admission`（`story_facts.py` 経由） |
| `verify_batch.sh` | 確認バッチ。ready な `feat/st*` の PR を `verify/<tag>` に merge → `tools/verify-prep.sh`（成果物）→ `verify_checklist.py`（手順書）→ draft の PR | verify グラフの `batch` |
| `verify_record.py` | 手順書の貼り戻しを tasks.md（`[x]` と印）と `docs/verify/<tag>.md` に記録する。通らなかったものを列挙して rc=1 | verify グラフの `record` |
| `tools/verify-prep.sh`（ashiato2） | server の release / web の build / APK（実機があれば adb で入れる）/ `run.sh` / `manifest.md` を `dist/verify-<tag>/` に | `verify_batch` |
| `cargo test -p ashiato-collector-windows --test runtime_windows`（ashiato2、Windows の上で） | 前景・入力・アドレスバーを本物の OS から読ませて記録を数える実行時テスト。テストが自分で窓を作る | CI の `collector-windows-runtime`（windows-latest）、手元の Windows |
| `tools/android-emulator.sh`（ashiato2） | エミュレータを立てて `src/androidTest` の計測テスト（前景サービス・権限の入口・HTTP を本物の framework で）。実機を繋いでも同じ gradle タスク | CI の `android-instrumented`（ubuntu + KVM）、手元 |
| `check-migrations.sh`（ashiato2） | 前進側の破壊的変更・戻し手順の欠落・**名前が作成時刻 `YYYYMMDDHHMM_<slug>.sql` でない**もの | ローカル、CI |

`scripts/` は harness2 への symlink で CI の runner には無い。CI の `chain` job は
`secrets.HARNESS2_TOKEN`（harness2 の Contents: read）があるときだけ動き、無ければ warning で素通りする。
**ローカルの `merge_gate.sh` が同じ検査を必ず走らせる**ので、関門はそちら。

## 規則（1 行ずつ）

- `closes #N` は残タスク 0 のときだけ。それ以外は `refs #N`。issue が残タスクの入口
- 人間は **draft でない PR だけ**を merge する。draft は「gate が通っていない」の印
- `ST<NN>.md` は生成物。判断は `docs/stories/stories.json`、逐語は `requirements.md`。直接編集しない
- deep の答えで要件が変わるなら `requirements.md` に `★ 日付` の印を入れて改訂し、Story を再生成する
- deep の問い JSON は `openspec/changes/<change>/deep-questions*.json` に commit する（問いが残らないとレビューできない）
- test には `// Scenario: <名前>` の印。**実機の OS を触る部分も機械で確かめる**（2026-09-14。ST07 の 11 件を 1 件にした）——
  Windows は `windows-latest` の実行時テスト（`crates/collector-windows/tests/runtime_windows.rs`）、Android はエミュレータの計測テスト
  （実機を繋いでも同じものが走る）。「人間の確認待ち」に置けるのは機械が再現できない物理的な操作（ロック・スリープ・電池・本物の GPS・時間そのもの）だけ
- **人間の確認は正しさのテストではない。** 確認バッチの手順書は、完了の判定と物理的な操作に加えて、Story ごとに 1 問「触ってみて違和感は無かったか」を聞く
  （`verify_checklist.py`）。答えは他と同じ形（通った = 違和感なし）
- main が赤になったら `fix/ci-<slug>` を切って同じ gate を通す
- issue は**上流の PR と同時に**作る（`scripts/issue_body.py`）。merge を待たない —— 待つと作る係がいなくなる（実測: ST02 で 10 時間の空き）
- **走っている Story（`tasks.md` がある）へは差し戻さない。** `followup ST<NN>` にして `docs/handoff/ST<NN>.md` へ。下流が読むのは開始時と PR 前の 2 回
- 止まるのは **A（`loss` がある）** だけ。B は `fixed D<n> 仮` で進み、PR 本文に列挙して merge のときに人間が見る
- **画面の構造は `kind: visual` + `proto`**（playground）で触って決める。文字の選択肢で問わない
- 移行の名前は作成時刻。連番にしない（並走する Story が番号を取り合う）
- merge の順序を design に書かない。gate が main に rebase するので、後から merge する側が追従する
- **並列はグラフの `admission` が決める**（規則は `board.py` の `admit()`）。requires が全部 archive 済み ∧ capability が走っている Story と重ならない
- **Story ごとに動作確認を求めない。** gate を通った PR は verify グラフ（`scripts/hx verify`）でまとめ、成果物と手順書を AI が用意してから人間が 1 回で確かめる。停止点は verify の PR の merge

## 現在地（2026-09-09）

- ST01: issue #2 を残 11 件の入口として再開。`check_scenarios` は 30 Scenario / 印 0 で FAIL（印を付けるのは ST01 の残作業。HTTP 層の結合テストを `crates/server/tests/` へ移すのと一緒にやる）
- ST02: `review_triage` が FR-33 / NFR-13 の戻し漏れで FAIL（上流の残作業。deep-questions JSON は `/tmp` に書き捨てたため残っていない。次の回から commit する）
