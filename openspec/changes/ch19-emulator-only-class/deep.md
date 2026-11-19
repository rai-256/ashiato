# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: 計測テストの台本に、指定したときだけ 1 クラスを 1 段で走らせる口を足し、AGENTS.md に絞り方を書くだけ。指定が無いときの 8 段と既存の検証コマンドは変えず、製品（収集アプリ・サーバ・画面）の振る舞いは変わらない
- 触れる予定のパス: tools/android-emulator.sh / tools/test-android-emulator.sh / AGENTS.md / docs/changes/CH19.md

## 第 1 回（2026-11-19 起票）—— spec のレビュー（review/spec.md）が出した 1 件

軽い路で spec を書いたが、spec-review が「依頼の前提（gradle の `class=` を台本から渡すだけ）が、依頼が挙げた例のクラスそのもので成り立たない」と判定した（R11。premise）。
本人の決定（議論で決めたこと）の「何を: 指定したクラスだけを 1 段で走らせる」を、そのクラスでは読み替える必要があるので、本人に返す。答えで通常の路へ移る。問いは `deep-questions-r1.json`。

### Q1. ONLY に St56BackupTest を渡したとき、何を走らせるか（R11）

本人の決定は「指定したクラス（例: `ONLY=dev.ashiato.collector.St56BackupTest`）だけを 1 段で走らせる」。だが `St56BackupTest` は gradle の 1 回の計測テストでは通らない
（`prepareBackup` と `verifyRestored` の間に、テストの外でデータを消してバックアップから戻す往復が要る。全体の実行でも gradle の段から外し、`am instrument` と `tools/android-backup-roundtrip.sh` で走らせている）。
同じ族: 入れ替えの試験（`tools/android-st58-replace.sh`）と採寸の試験（`tools/usage-volume.sh`）も gradle の 1 段では前提が無く必ず落ちる。
推奨: St56 は全体と同じ準備・往復・確かめの手順だけを走らせる（design D2（仮））。入れ替えと採寸は起こさずに専用の台本の名前を出して止める（D4（仮））。

**答え（本人の選択。推奨と同じ選択肢を本人が選んだ・補足なし）**: 全体と同じ準備・往復・確かめの手順だけを走らせる

- 決まったこと: `ONLY=dev.ashiato.collector.St56BackupTest` のときは gradle の計測テストを呼ばず、全体の 2〜3 段目と同じ命令
  （installDebug → bmgr enable → am instrument …#prepareBackup → `tools/android-backup-roundtrip.sh` → am instrument …#verifyRestored と `OK (1 test)` の照合）を同じ順に走らせる。
  議論で決めた「指定したクラスだけを 1 段で走らせる」は、このクラスに限りこの手順として読む（決定の「何を」の読み替え。範囲「台本の絞る口と AGENTS.md だけ」は変えない）
- 選択肢の detail にあった「入れ替え・採寸の試験は起こさずに止める」も同じ答えに含まれる（D4（仮）の向きを本人が選んだ）
- 仕様への写し: design D2（仮）・D4（仮）は本人の答えで決まった。spec の段で（仮）を外し、出所を Q1 にする
