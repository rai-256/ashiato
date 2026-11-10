## CH15 — 配った形のサーバで、収集アプリの設定が正しい合言葉を断る（上流・軽い路）

軽い路（`lane.json` の `reason: bug`。名指しの Scenario「合言葉を入れ直すと、設定の文字列とコピーのボタンが出る」）。人間の深掘りと合意は省いた。正典（`openspec/specs/`）は変えない（`skip_specs: true`）。

### 何を直すか
- インストーラで入れたサーバは、照らすための値（verifier）で起動するので `WebLogin.password` が空。資格情報を返す口（`POST /collector-setup`）がこの値と比べているため、正しい合言葉でも 403 になる
- 照合をログインの口と同じ道に通す（design D1: `LOGIN_CHECK` の印・`spawn_blocking` の側の `WebLogin::accepts`・失敗の数えを共有）。2 つの口が共に使う関数を 1 つ置き、`session_post` もこれを使う。振る舞いは変えない

### 決めたこと
- 深掘りで本人が決めたこと: なし（軽い路）
- 仮決め: なし。D1 は正典の Requirement（同じ遅れ・回数の制限・数えの共有）に従う実装の決定
- capability の前倒し: なし

### tasks
- Task 1 つ（1.1 試験を先に書く → 直す前に FAIL を記録 → 直す → PASS。1.2 は今の振る舞いが残る確かめ）と、人間の確認待ち 1 つ（9.1。配った形で文字列が出る。物理: realdata）

### レビュー
- spec-review: 指摘 7 件、すべて technical、7 件とも fixed（検証の形 1.1・1.2、lib.rs に触れない、近道を止める grep、空の合言葉と 429 の順、`session_post` に触れることの明記、e2e の追加、9.1 の版）。A は無い

🤖 Generated with [Claude Code](https://claude.com/claude-code)
