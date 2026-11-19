# CH19 — エミュレータの計測テストを 1 クラスだけ走らせる口を足す

> 軽い路（`lane.json` の `reason: same`）。深掘りと合意は省いてある。製品（収集アプリ・サーバ・画面）の振る舞いは変えず、
> 開発の道具の台本 `tools/android-emulator.sh` に、指定したときだけ使う絞る口を足す。依頼の「議論で決めたこと」（何を・なぜ・どこまで）は本人の決定として扱い、聞き直さない。

## Why

ST56 Task 5 で、計測テストの 1 クラス（`St56BackupTest`）を直すのに、全段（1 回 17 分。gradle 自体は 20 秒で落ちていた）を 27 回まわし、implement が 485 分かかった。
今の `tools/android-emulator.sh` には 1 クラスだけを走らせる口が無く、絞りたくても絞れない。
「落ちる → 直す → 直したところだけ走らせる → だめならまた直す、通れば全体を走らせる」の回し方ができるようにする。ハーネス側の「絞った検証」（harness2 #271）もこの口を使う。

## What Changes

- `tools/android-emulator.sh` に環境変数 `ONLY` を足す。完全修飾のクラス名を 1 つ渡すと、エミュレータを起こしてアプリを外し・権限を戻した後、gradle の計測テストを `-Pandroid.testInstrumentationRunnerArguments.class=<ONLY>` で 1 段だけ走らせる。
  走ったクラスの結果が XML に無ければ 0 本として止まる（design D3（仮））
- `ONLY=dev.ashiato.collector.St56BackupTest`（依頼の例そのもの）は、gradle の 1 段では走らない（準備と確かめの間にテストの外でバックアップを往復させる）ので、全体のときのバックアップの段と同じ手順だけを走らせる（design D2（仮））
- 入れ替えの試験（`tools/android-st58-replace.sh` が走らせる）と採寸の試験（`tools/usage-volume.sh` が走らせる）、形の違う値は、エミュレータを起こさずに rc=1 で止める（同じ台本の既存の設定の誤りと同じ rc）（design D4（仮）・D5（仮））
- `ONLY` が無いか空なら、今の全段（8 段）をそのまま走らせる。命令の列が変わらないことを偽の SDK の試験で固定する
- `AGENTS.md` の「長いコマンド」に、直している途中は絞って走らせ（エミュレータは `ONLY=`、`cargo test` は名前の絞り）、通ったら最後に全体を `scripts/verify-run` で走らせることを書く
- `docs/changes/CH19.md` の front matter の capability を `device-collection` から `emulator-startup` に直す（台本の振る舞いの正典は CH10 が作った `emulator-startup`。携帯の収集の振る舞いには触れない）

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `emulator-startup`: Requirement を 4 つ ADDED で足す（既存の Requirement と Scenario は変えない）

## Impact

- 触れる予定のパス（`lane.json`）: `tools/android-emulator.sh`・`tools/test-android-emulator.sh`・`AGENTS.md`・`docs/changes/CH19.md`（と、この change の中）
- 触れないもの: `tools/test-usage-volume.sh`（`android-emulator.sh` の行の形を grep する。変えずに通る形で足す）・`tools/emulator-start.sh`・`tools/android-backup-roundtrip.sh`・`tools/android-st58-replace.sh`・`tools/usage-volume.sh`・
  収集アプリのコードと計測テスト・CI・`docs/testing.md`・既存の change の `tasks.md` の検証コマンド
- 走っている change との重なり: 2026-11-19 時点で `openspec/changes/` に走っている change は CH19 だけ（ST58 は archive 済み）。`tools/android-emulator.sh` を触る走っている change は無い
- 正典の古さ（持ち越し。spec-review R12）: 正典 `emulator-startup` の Requirement「正常にエミュレータが立つときの計測テストの台本の振る舞いは変わらない」と Scenario は「6 段」「stage1〜stage6」と書くが、ST56・ST58 の後の実物は 8 段。軽い路は MODIFIED を書けないので、この change では直さない。
  ADDED の Requirement は「この change の時点で 8」と書き、正典の段の数の約束を「ONLY が無いか空のときの約束」と読むことを本文に置いた（R10）。正典の WHEN と段の数そのものは、別の change が MODIFIED で直す
- 後の利用: harness2 #271（絞った検証）がこの `ONLY` を使う。名前（`ONLY`）は依頼の決定
