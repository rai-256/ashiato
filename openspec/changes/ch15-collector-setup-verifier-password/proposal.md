# CH15 — 配った形のサーバで、収集アプリの設定が正しい合言葉を断る

> 軽い路（`lane.json` の `reason: bug`）。深掘りと合意は省いてある。正典の Scenario の振る舞いは変えず、コードだけを正典に合わせる。

## Why

2026-11-09、本人がインストーラで入れたサーバで、画面にはログインできるのに、「収集アプリの設定」で同じ合言葉を入れると「合言葉が違います」と断られた。
正典の Scenario「合言葉を入れ直すと、設定の文字列とコピーのボタンが出る」（`openspec/specs/data-sensitivity/spec.md`）が、配った形で満たされていない。
原因は、資格情報を返す口（`POST /collector-setup`、`crates/server/src/web_session.rs` の `collector_setup_post`）が、本文の合言葉を `token_matches(&given, &app.login.password)` で比べていること。
配った形のサーバは、合言葉そのものを持たず照らすための値（verifier）で起動する（`WebLogin::new_with_verifier`）。このとき `password` は空なので、口はどの合言葉も 403 で断る。
開発の形（合言葉そのもので起動する `WebLogin::new`）では `password` に合言葉が入っているので、既存の試験（`st56_tests.rs`）も e2e も通り、見逃した。

## What Changes

- `collector_setup_post` の合言葉の照合を、ログインの口（`session_post`）と同じ照合に替える（`WebLogin::accepts`）。照らすための値の形では Argon2id で照らし、合言葉そのものの形では今と同じに比べる
- 照らすための値の照合は CPU を数十ミリ秒使うので、ログインの口と同じ通し方にする（照合を 1 つずつ通す印・blocking の側での照合・失敗の数え。design D1）。正典の「ログインの口と同じ失敗の遅れと回数の制限を当て、失敗の数えを共有する」をそのまま守る。2 つの口が共に使う照合の関数を 1 つ置き、**ログインの口（`session_post`）の照合もその関数へ移す**（ログインの口のコードに触れるが、振る舞いは変えない。確かめは tasks 1.2）。依頼と `lane.json` の「`accepts` に替えるだけ」より一歩広いが、予定のパス（`web_session.rs`）の中に収まる
- 照らすための値で起動したサーバに対する試験を足す（`crates/server/src/web_session_tests.rs`。直す前に落ちることを記録する）

正典（`openspec/specs/`）は変えない（`.openspec.yaml` の `skip_specs: true`。振る舞いは正典がもう言っている）。

## Capabilities

### New Capabilities

なし

### Modified Capabilities

なし（`data-sensitivity` の Requirement「画面は、ログインして画面の合言葉を入れ直したときだけ、収集アプリの設定の文字列を出す」を、配った形でも満たすようにコードを直すだけ）

## Impact

- 触れる予定のパス（`lane.json`）: `crates/server/src/web_session.rs`（本番のコード）・`crates/server/src/web_session_tests.rs`（試験）・`crates/server/src/lib.rs`（試験で照らす形の App を組む口が要るときだけ。今ある `App::with_login` で足りる見込み）・`docs/changes/CH15.md`・この change
- 触れないもの: API の型と契約（`docs/openapi.json`。応答の形・状態の番号は同じ）・移行・画面（`web/`）・収集アプリ・インストーラ・`st55_tests.rs` / `st56_tests.rs`（予定のパスの外。触れると通常の路へ昇格する）
- 走っている change との重なり: `ch13-workbox-config`（作業箱の設定）・`ch14-installer-password-prompt-mojibake`（インストーラの合言葉の問い）。どちらも `crates/server` に触れない
