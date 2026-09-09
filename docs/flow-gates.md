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
| コード | `pr-review-toolkit` の 3 agent + `code-verify` agent | `/story` の PR 前 | `review/code.md` |
| PR | `scripts/merge_gate.sh` | 人間が merge する前 | draft 状態 + PR コメント |
| archive | `scripts/archive.sh` | 下流の merge 後 | `openspec/specs/`（正典） |

agent は `.claude/agents/`。いずれも **`Edit` を持たない**（指摘を出すだけで直さない）。

## 指摘の形と処置

`openspec/changes/<change>/review/<段階>.md`:

```
## R1. <主張を 1 行で>
- 成果物: <path>
- 根拠: <file:line> / <コマンドと結果>
- kind: technical | conflict | irreversible | daily | premise | defer
- 処置: fixed D16 | fixed 9.3 | rejected: <理由> | escalated | deferred ST04
```

| kind | 意味 | 処置 |
|---|---|---|
| `technical` | 技術判断で閉じる | 直して D番号かタスクを指す。反証できれば `rejected:` と理由 |
| `conflict` / `irreversible` / `daily` | `/story` の 3 分類と同じ | **人間へ。** `deep.md` に R番号つきで追記し `ask_wizard` の問いに。PR は draft |
| `premise` | 既決の前提が崩れた（例: 列の型が `jsonb` で「そのまま残す」が成立しない） | 同上。直せそうでも人間へ —— 前提を直すと本人の答えが変わりうる |
| `defer` | 他 Story の担当 | その Story が実際にそれを持っているか確かめて `deferred ST<NN>` |

`scripts/review_triage.py . <change>` が、処置の無い指摘・指す先の不在・人間に返すべきものの `fixed`・
「要件へ戻すもの」の戻し漏れ（`requirements.md` に `★ 日付` の印が無い）を FAIL にする。

## 検査の一覧

| スクリプト | 見るもの | 走る場所 |
|---|---|---|
| `check_chain.py` | 要件 → Story の鎖（8 観点。対象外は INDEX の「Story の対象外」）。`stories.json` からの再生成と一致するか | ローカル、`merge_gate`、CI（`HARNESS2_TOKEN` があるとき） |
| `check_scenarios.py` | 全 `#### Scenario:` に test の印（`Scenario: <名前>`）があるか。無いものは「人間の確認待ち」に無ければ FAIL | ローカル、`merge_gate`、`archive` |
| `review_triage.py` | 上の処置 | ローカル、`merge_gate`、`archive` |
| `merge_gate.sh` | head を main に追従 → その head の CI → tasks の残り → 検査 3 本 → deep の未回答。落ちれば draft に戻す | PR の最後 |
| `archive.sh` | 人間の確認待ち以外が全部 `[x]`、Scenario の担保、指摘の処置 → `openspec archive` → PR | 下流の merge 後 |

`scripts/` は harness2 への symlink で CI の runner には無い。CI の `chain` job は
`secrets.HARNESS2_TOKEN`（harness2 の Contents: read）があるときだけ動き、無ければ warning で素通りする。
**ローカルの `merge_gate.sh` が同じ検査を必ず走らせる**ので、関門はそちら。

## 規則（1 行ずつ）

- `closes #N` は残タスク 0 のときだけ。それ以外は `refs #N`。issue が残タスクの入口
- 人間は **draft でない PR だけ**を merge する。draft は「gate が通っていない」の印
- `ST<NN>.md` は生成物。判断は `docs/stories/stories.json`、逐語は `requirements.md`。直接編集しない
- deep の答えで要件が変わるなら `requirements.md` に `★ 日付` の印を入れて改訂し、Story を再生成する
- deep の問い JSON は `openspec/changes/<change>/deep-questions*.json` に commit する（問いが残らないとレビューできない）
- test には `// Scenario: <名前>` の印。実機でしか確かめられないものは `tasks.md` の「人間の確認待ち」に `Scenario: <名前>` で挙げる
- main が赤になったら `fix/ci-<slug>` を切って同じ gate を通す

## 現在地（2026-09-09）

- ST01: issue #2 を残 11 件の入口として再開。`check_scenarios` は 30 Scenario / 印 0 で FAIL（印を付けるのは ST01 の残作業。HTTP 層の結合テストを `crates/server/tests/` へ移すのと一緒にやる）
- ST02: `review_triage` が FR-33 / NFR-13 の戻し漏れで FAIL（上流の残作業。deep-questions JSON は `/tmp` に書き捨てたため残っていない。次の回から commit する）
