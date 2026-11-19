# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: 計測テストの台本に、指定したときだけ 1 クラスを 1 段で走らせる口を足し、AGENTS.md に絞り方を書くだけ。指定が無いときの 8 段と既存の検証コマンドは変えず、製品（収集アプリ・サーバ・画面）の振る舞いは変わらない
- 触れる予定のパス: tools/android-emulator.sh / tools/test-android-emulator.sh / AGENTS.md / docs/changes/CH19.md
