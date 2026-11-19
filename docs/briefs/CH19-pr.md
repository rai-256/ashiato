# CH19 エミュレータの計測テストを 1 クラスだけ走らせる口を足す

change: `openspec/changes/ch19-emulator-only-class/`（capability `emulator-startup`・ADDED の Scenario 13 本）。依頼と完了の判定は `docs/changes/CH19.md`（受け皿の依頼 #328 から）。
`tools/android-emulator.sh` に `ONLY=dev.ashiato.collector.<クラス>` の口を足す。直している途中はそのクラスだけを走らせ（全体の 17 分を待たない）、通ったら最後に全体を `scripts/verify-run` で走らせる。`ONLY` が無いか空なら、全体の 8 段は命令も行も変えない。

軽い路（`same`）で入ったが、仕様のレビュー（R11。premise）が「依頼の例の `St56BackupTest` は gradle の 1 段では走らない」と見つけたので通常の路へ昇格し、本人に問うた。

## 深掘りで本人が決めたこと（A）

- **Q1（第 1 回・R11 から）ONLY に St56BackupTest を渡したとき何を走らせるか** —— 答え: 全体と同じ準備・往復・確かめの手順だけを走らせる（推奨と同じ・補足なし）。
  gradle の計測テストは呼ばず、全体の 2〜3 段目と同じ命令（installDebug → bmgr enable → prepareBackup → `tools/android-backup-roundtrip.sh` → verifyRestored と `OK (1 test)` の照合）を同じ順に走らせる（design D2）。
  入れ替え（`St58PackageReplacedInstrumentedTest`）と採寸（`UsageVolumeInstrumentedTest`）の試験は、エミュレータを起こさずに専用の台本の名前を出して rc=1 で止める（design D4）
- 議論で決めたこと（何を・なぜ・どこまで）は依頼の逐語のまま（`docs/changes/CH19.md`）。聞き直していない

## 仮決め（B）

- D3（仮）ONLY の段の前に前の実行の結果を消し、段の後に渡したクラスの結果が XML に無ければ 0 本として rc=1。反転条件: gradle が 0 本で確実に rc≠0 になると実物で確かめられたら
- D5（仮）ONLY の値は完全修飾のクラス名 1 つだけ（メソッド・複数は rc=1）。反転条件: 絞った検証（harness2 #271）がメソッド単位か複数を要ると分かったら
- 仮の台帳（`provisional.jsonl`）に載せたものは無い

## capability

前倒しなし。依頼の `device-collection` を、台本の振る舞いの正典の `emulator-startup`（CH10 が作った）に直した（`docs/changes/CH19.md` の「行き先」）。
正典の「6 段」の Requirement は実物（8 段）より古い。ADDED は 8 を決め打たず台本から数える形にし、MODIFIED での直しは別の change へ持ち越す（proposal の Impact・R12）。

## tasks

4 Task・9 項目（偽の SDK の命令の列の固定 → ONLY の口 → バックアップの試験の手順と AGENTS.md → 本物のエミュレータと触ったパスの確かめ）。
既存の Story / change の `tasks.md` の検証コマンドは書き換えない。

## レビュー

- `review/spec.md`: 12 件。fixed 10（D2・D4・1.1×3・2.2×3・spec×2 … R1〜R10）・fixed proposal.md 1（R12。持ち越し）・escalated 1（R11 → Q1 で本人が答えた）
- `review/answers.md`: Q1 は写った・答え以外の変更なし（D2・D4 の（仮）を外し出所を Q1 にしただけ）
