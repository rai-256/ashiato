# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: 試験を走らせる作業箱の設定（tools/workbox.json）に、箱の中でサーバを組み立てるのに要る圧縮ライブラリ（liblzma-dev・libbz2-dev）を足し、箱 1 つの CPU の上限（cpus: 6）を置くだけ。製品（サーバ・インストーラ・収集アプリ・画面）のコード・API や送信の契約・移行・依存の版には触れず、試験の中身と判定も変えない。作業箱はまだ有効になっておらず（HARNESS_WORKBOX=tests は merge の後に人間が足す）、いまの試験の走り方も変わらない。何を・どこまでは依頼が名指ししてあり、決めることが無い
- 触れる予定のパス: tools/workbox.json / docs/changes/CH18.md / openspec/changes/ch18-workbox-libs-cpus/**
