# 貢献について

## ライセンス

本体は **AGPL-3.0-only** です（`LICENSE`）。

## CLA（貢献者ライセンス同意）

**貢献は CLA の下でのみ受け付けます。** 全文は [`docs/CLA.md`](docs/CLA.md)。要点は次の 1 点です。

> 提供いただいたコードの著作権はあなたに残ります。そのうえで、このプロジェクトの管理者に対し、
> **提供分を AGPL-3.0 以外の条件でもライセンスできる許諾**を与えていただきます。

### なぜ必要か

AGPL-3.0 だけで受け入れると、その部分は AGPL に固定されます。すると
**ソースを開けられない相手に商用ライセンスを売る道（デュアルライセンス）が閉じます**。
これは開発費を賄うための経路なので、閉じると開発が続きません。

**この同意は遡って取れません。** 1 件でも CLA 無しに取り込むと、その部分について道が閉じます。

### 手順

Pull Request の本文に次の 1 行を入れてください。

```
I agree to the CLA in CONTRIBUTING.md.
```

DCO（`Signed-off-by`）だけでは足りません。DCO は「自分が書いた」ことの表明で、
**別条件でライセンスする許諾を含まない**ためです。

## 送る前に

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
docker compose up -d --wait db && cargo test --workspace          # テストは本物の PostgreSQL を使う
./tools/check-migrations.sh && ./tools/check-boundaries.sh && ./tools/check-openapi.sh
(cd web && npm ci && ../tools/check-licenses.sh && npx tsc -b && npm run lint && npm run test && npm run build)
(cd collector-android && ./gradlew :app:assembleDebug :app:testDebugUnitTest)
cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings
./tools/smoke.sh && ./tools/check-panic-log.sh && ./tools/check-immutable.sh
```

CI が走らせるのと同じコマンドです（`.github/workflows/ci.yml`）。OS を触る部分は Windows の上で
`cargo test -p ashiato-collector-windows`、Android は `./tools/android-emulator.sh` で（CI でも走ります）。
テスト全体の書き方は `docs/testing.md`。
