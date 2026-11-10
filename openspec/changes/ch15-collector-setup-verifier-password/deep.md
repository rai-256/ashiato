# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: bug（仕様がもう言っている）
- 申告の根拠: 正典の Scenario（data-sensitivity）が、照らすための値（verifier）で起動したサーバで満たされていない不具合。collector_setup_post の照合を token_matches(&given, &app.login.password) からログインと同じ WebLogin::accepts に替えるだけで、仕様・API の型・移行・送信契約に触れない。試験は verifier で起動したサーバで web_session_tests.rs に置き、直す前に落ちることを記録する。lib.rs は試験で verifier の形の App を組む口が要るときだけ触る
- 触れる予定のパス: crates/server/src/web_session.rs / crates/server/src/web_session_tests.rs / crates/server/src/lib.rs / docs/changes/CH15.md / openspec/changes/ch15-collector-setup-verifier-password/**
- 名指しの Scenario: 合言葉を入れ直すと、設定の文字列とコピーのボタンが出る

## 下流で積んだ A（軽い路から通常の路へ昇格。2026-11-11）

昇格の理由（`lane-verdict.json` の `diff`）: Scenario「合言葉を入れ直すと、設定の文字列とコピーのボタンが出る」の項目（1.1）に、本番のコードが base のままの木で落ちた証跡が無い。
調べたこと: 直す前の FAIL は記録されている（2026-11-10T13:05:32Z、head `edf97de6a`・木 `db0e9a532`、`ch15_verifier_collector_setup_returns_token` が `left: 403 / right: 200`）。その後ブランチが #289（`eb243793d`）の上に載せ替えられ、`lanes.py red_green` が FAIL の木を今の base と比べた。間に main へ入った CH13 の archive・確認バッチのファイル（`tools/lanes.json` のどの分類にも当たらない `*.json` `*.txt` `*.py` `*.yaml`）が本番のコードとして数えられた。本番のコード（`web_session.rs`・`lib.rs`）は、その木でも base と同じ。試験のコードは載せ替えた後の `738fa6dbb` と同じ。
コードは変えていない。問いの HTML: `docs/briefs/CH15-deep-r1.html`（`deep-questions-r1.json`）

### Q-L1 [配った形のサーバで、資格情報を返す口が正しい合言葉を受け付けるようになる変化を受け入れるか] (未回答)

- **分類**: A（premise・loss: exported）
- **変化**: `crates/server/src/web_session.rs` の `collector_setup_post` の照合を `token_matches(&given, &app.login.password)` から、ログインの口と同じ `check_given_password`（`LOGIN_CHECK` の印の中、blocking の側で `WebLogin::accepts`）に替える。インストーラで入れたサーバ（照らす形）で正しい合言葉なら 200 で資格情報が返り、画面に設定の文字列とコピーのボタンが出る（今は必ず 403「合言葉が違います」）。合言葉そのものの形（開発の形）の振る舞いは変わらない
- **選択肢**: (1) 受け入れる。今ある直す前の FAIL の記録を根拠と認め、コードはこのまま（推奨）/ (2) 受け入れる。今の base の上で本番のコードを戻した木で 1.1 の FAIL を取り直してから進める / (3) 受け入れない
- **族**: 照らす形で `login.password` が空のまま残り、合言葉そのものと比べる所。ログインの口・`secret_tag`・起動時の WEB_SESSION_KEY の検査は ST55 で決めた。ほかに比べる所は無い
- **上流の漏れ**: premises —— 記録の後に base が動く（rebase）ことを前提に入れていなかった。`red_green` は記録の時点の base でなく今の base と比べる

### Q-L2 [求めが取り消されても照合を走り切り、記録の行を残す振る舞いを、ログインの口にも及ぼしてよいか] (未回答)

- **分類**: A（premise・loss: uncaptured）
- **変化**: `web_session.rs` の 2 つの口（`POST /session`・`POST /collector-setup`）の「照合 → 読み出しの記録の行 → 答え」を `run_to_completion`（`tokio::spawn`）で走らせる（review/code.md R1）。接続が切れても照合は走り切り、失敗なら数え、行が 1 行残る。これまでは取り消された求めで行が残らず、印を待つ間に取り消されると失敗にも数えなかった。CH15 の「壊してはいけないもの」4（ログインの口）に触れる
- **選択肢**: (1) 受け入れる（2 つの口とも。推奨）/ (2) 資格情報の口だけ。ログインの口は base の扱いに戻す / (3) 2 つの口とも base の扱いに戻す（R1 は持ち越し）
- **族**: 2 つの口の照合を 1 つの道にしたことで依頼の外で変わる振る舞い。断られている間は印を待たない（R4・R5）・空の合言葉は Argon2id を回さない・503 の道を外す（R9）・2 つの口が同じ印で並ぶ、は design D1 で決めた（応答と数えは base と同じ）。試験の App を `testdb::verifier_app` に寄せた（R8）のは振る舞いを変えない
- **上流の漏れ**: spec —— 軽い路の design D1 は差し替えだけを決め、取り消しのときの行と数えを族として見ていなかった。deep と spec-review を通らないので拾う席が無かった
