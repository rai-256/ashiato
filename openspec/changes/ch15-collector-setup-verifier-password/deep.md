# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: bug（仕様がもう言っている）
- 申告の根拠: 正典の Scenario（data-sensitivity）が、照らすための値（verifier）で起動したサーバで満たされていない不具合。collector_setup_post の照合を token_matches(&given, &app.login.password) からログインと同じ WebLogin::accepts に替えるだけで、仕様・API の型・移行・送信契約に触れない。試験は verifier で起動したサーバで web_session_tests.rs に置き、直す前に落ちることを記録する。lib.rs は試験で verifier の形の App を組む口が要るときだけ触る
- 触れる予定のパス: crates/server/src/web_session.rs / crates/server/src/web_session_tests.rs / crates/server/src/lib.rs / docs/changes/CH15.md / openspec/changes/ch15-collector-setup-verifier-password/**
- 名指しの Scenario: 合言葉を入れ直すと、設定の文字列とコピーのボタンが出る
