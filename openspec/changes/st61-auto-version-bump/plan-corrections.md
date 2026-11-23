# plan の訂正（凍結後の検証コマンドだけ）

`scripts/plan_fix.py` が書く。受け入れ条件は変えず、成立しないことが証跡で実証された検証コマンドだけを、人間の承認か機械の証跡による承認のうえで直した記録。

## 2026-11-23 8.1

- 旧: `bash -c '! grep -nF "release/" tools/release/version.sh'`
- 新: `tools/release/check-no-release-branch.sh`
- 理由: version.sh は tools/release/ のパスを 48 か所で正当に持つので grep -F release/ は原理的に通らない。消すのは修正の枝の分岐（release/<数字>・origin 上の release/）で、それだけを見る台本に置き換える
- 実証: evidence.jsonl の 2026-11-23T03:37:24+00:00（FAIL、rc=1、ログ .superpowers/sdd/logs/20261123-123724-evidence-8.1-1-e89f85-vdx2a00fg.log、HEAD c8ef1f3）
- 承認: machine: evidence.jsonl 2026-11-23T03:37:33+00:00 PASS

## 2026-11-23 2.2

- 旧: `bash -c 'ASHIATO_BUILD_VERSION=0.4.0-beta.2 scripts/quiet-run apk -- ./collector-android/gradlew -p collector-android :app:assembleRelease && tools/release/check-artifacts.sh apk-version collector-android/app/build/outputs/apk/release/*.apk'`
- 新: `tools/release/check-artifacts.sh apk-build 0.4.0-beta.2`
- 理由: apk-version は比べる版番号を渡さないと APK 自身の versionName でしか比べず、版を無視した APK や置き場に残った別の組み立ての APK でも rc=0 だった（code-verify R4）。apk-version に --expect-version を必須にしたので旧コマンドは成立しない。新しい入口は置き場を空にしてから渡した版番号で組み、組んだ APK を渡した版番号と比べる
- 実証: evidence.jsonl の 2026-11-23T13:44:58+00:00（FAIL、rc=2、ログ .superpowers/sdd/logs/20261123-224458-evidence-2.2-2-a22244-vwjgo4tu5.log、HEAD 07ddea0）
- 承認: machine: evidence.jsonl 2026-11-23T13:46:06+00:00 PASS
