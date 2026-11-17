# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: Windows のインストールの試験の台本（install-check.sh・update-check-install.sh）の判定が NG のときに観測をログへ出すことと、出さない判定を落とす検査（自己試験か tools/release の検査の台本と、それを CI で走らせる 1 手）だけを変える。製品（サーバ・インストーラ・収集アプリ・画面）と振る舞いの契約には触れず、判定が OK / NG を出す条件も変えない。何を・なぜ・どこまでは受け皿 #307 の議論で本人が決めてある
- 触れる予定のパス: tools/release/install-check.sh / tools/release/update-check-install.sh / tools/release/check-*.sh / .github/workflows/ci.yml / docs/testing.md
