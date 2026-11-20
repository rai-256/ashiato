# CH25 settle report

## R1 / R5

`check-ch25.py ci` が、`cache: gradle` の `cache-dependency-path` を D1 の 3 glob と完全一致で要求し、chain の cache key に同じ 3 glob を含む `hashFiles(...)` と `restore-keys` 不在を要求するようにした。

- `python3 openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py ci` → rc=0, `OK: ci`
- ci の build-stamp から `collector-android/**/*.lockfile` を一時的に外す → rc=1, `build-stamp が gradle の部品の取り置きを持たない`
- chain に `restore-keys: gradle-` を一時的に加える → rc=1, `chain が gradle の部品の取り置きを持たない`

## R2

rejected。Gradle build cache は task input から導く key で出力を復元するため、異なる source の APK に前 run の出力を混入させるという根拠は再現できない。同一 inputs の `FROM-CACHE` は正規の再利用である。

## R3

rejected。D1 は共有 key の最初の job だけが保存することと job 固有 dependency が残らないことを明記して受容しており、merge 後の復元失敗または 4.1(2) の観測を反転条件にしている。今回の根拠はその既知の代償を再記述するだけで、反転条件の実測ではない。

## R4

rejected。Task 2 に Android test を毎 run 実行する要件はなく、同一 task inputs の `FROM-CACHE` は Gradle の正規挙動である。提示された将来の非決定性だけでは、現行 workflow の欠陥を再現できない。
