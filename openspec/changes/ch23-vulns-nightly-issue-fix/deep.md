# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: bug（仕様がもう言っている）
- 申告の根拠: 正典の Scenario「落ちた夜に、依存ごとに 1 件の issue が立つ」は、ラベル「脆弱性」が無いとき label create の後に issue create を受けると言っているが、本物の gh はラベルが無いと空の文字を返し、tools/check-vulns-issues.py が読めずに落ちて issue を立てていなかった（偽の gh は [] を返すので自己試験は通っていた）。直すのは台本と自己試験の偽の gh で、赤に気づける形にするのは同じ毎晩の workflow と道具の中に閉じる（製品のコード・API・移行・収集アプリに触れない）
- 触れる予定のパス: tools/check-vulns-issues.py / tools/check-vulns.sh / tools/check-vulns-wiring.sh / tools/check-vulns* / .github/workflows/vulns-nightly.yml / docs/changes/CH23.md / docs/testing.md / docs/adr/*.md
- 名指しの Scenario: 落ちた夜に、依存ごとに 1 件の issue が立つ
