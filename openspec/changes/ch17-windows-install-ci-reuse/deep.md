# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: CI の package-windows / install-windows の job の回し方（台本だけの変更では前に作った Windows の配布物を使い回す・Rust のビルドにキャッシュを効かせる）を変えるだけで、製品のコード・配布物の中身・API や送信の契約・移行には触れない。走らせる確かめ（install-check / update-check-install の Scenario）も変えない。使い回しの可否を決めるビルドの入力の鍵は、本人が「作るときに決める」とした範囲で、誤っても CI の設定を直せば戻る（取っていないデータ・捨てたデータ・外に出たものは無い）ので、spec の段で D（仮）と反転条件に書き、spec-review に確かめさせる。鍵を出す判定は changes.sh の正本に足すか、tools/release/ に新しい台本を置く
- 触れる予定のパス: .github/workflows/ci.yml / tools/release/changes.sh / tools/release/windows-build-key*.sh / docs/testing.md / docs/adr/*.md
