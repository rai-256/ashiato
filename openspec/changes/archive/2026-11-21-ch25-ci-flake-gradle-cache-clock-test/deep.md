# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: CI の gradle の部品をキャッシュから取る形にする（依存の版・ロックの検査は変えない）のと、clock.rs の #[cfg(test)] の偽のサーバが返す瞬間の時刻にずらしを足すだけ。製品のコード（HttpDateClock::now）・API・送信の契約・移行には触れず、製品の振る舞いは変わらない。nightly / release も tools/release/build.sh apk で gradle を使うので含める
- 触れる予定のパス: .github/workflows/ci.yml / .github/workflows/nightly.yml / .github/workflows/release.yml / crates/collector-windows/src/clock.rs / docs/changes/CH25.md
