# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-04 9.2

- 旧: `bash -c 'scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests "*ContractGate*"'`
- 新: `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests *St50*`
- 理由: ST50 の携帯の試験は ContractGate の名を持たず（St50ContractNoticeTest・St50SenderTest・St50EnvelopeShapeTest）、*ContractGate* は main の時点から 1 本も当たらず No tests found で落ちる。受け入れ条件「ST50 の試験が通る」をそのまま確かめるため、ST50 の試験の名前で絞る（41 本 PASS）
- 実証: evidence.jsonl の 2026-11-04T09:20:43+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261104-182043-evidence-9.2-1-7bca0a-vnk1i75dk.log、HEAD 8690a27）
- 承認: machine: evidence.jsonl 2026-11-04T09:21:00+00:00 PASS
