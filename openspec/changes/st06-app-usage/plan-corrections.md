# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認のうえで直した記録。

## 2026-09-26 8.2

- 旧: `./gradlew :app:connectedDebugAndroidTest`
- 新: `tools/android-emulator.sh`
- 理由: 計測テストは 2 段（権限を拒否したときのテストは pm reset-permissions の後に annotation 指定で走らせる）と -Pashiato.baseUrl（127.0.0.1 への平文 HTTP を許す）を前提にしていて、裸の gradle タスクでは原理的に通らない。プロジェクトの正式な入口は tools/android-emulator.sh（エミュレータの起動・2 段・baseUrl つき）。
- 実証: evidence.jsonl の 2026-09-25T15:00:51+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20260926-000051-evidence-8.2-1-9da249.log、HEAD 89a0e6f）
- 承認: 2026-09-25 本人の回答（ST06 8.2: 検証入口だけを正式な tools/android-emulator.sh に修正。受け入れ条件は変えない）
