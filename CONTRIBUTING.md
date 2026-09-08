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
cargo test --workspace
(cd web && npx tsc -b && npm run lint && npm run build)
./tools/smoke.sh
```

CI が走らせるのと同じコマンドです（`.github/workflows/ci.yml`）。
