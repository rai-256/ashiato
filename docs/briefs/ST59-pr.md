# ST59 — 合言葉を画面から変えられる（上流: 深掘り第 1 回の推奨で書いた仕様）

change: `openspec/changes/st59-change-password-from-web/`（FR-133）

## 深掘りで本人が決めたこと

第 1 回の 3 問は、どれも「仮でよい」（後から変えても失うものが無い）問い。答えの前に**推奨の答えで**仕様を書いた。答えが推奨と違えば `spec --answers` が写し直す。

- Q1 合言葉を変えたら、ほかの端末のログインは全部切り、変えた画面だけ新しい合言葉でログインしたままにする（推奨）
- Q2 設定の画面に「画面の合言葉を変える」「収集アプリの値を変える」の 2 つの節を別々に置き、「携帯を落としたときは両方を変える」と書く（推奨）
- Q3 設定のファイルを指して起こしたサーバでだけ変えられる。`.env` の開発の形では断る（推奨）

聞かない既定（`review/deep.md` C1〜C7）: 読み出しの記録に口ごとに 1 行（D9）・設定のファイルに書けた後にその場で差し替え（D1）・1 つの鍵だけ置き換えてディスクへ流してから付け替え（D2）・値の置き場は `server.json` 1 つ（D3）・インストーラと同じ 5 つの検査（D4）・新しい収集アプリの値はサーバが乱数で作る（D6）・古い値は変えた瞬間から断る（D7。FR-128）。

## 仮決め（design.md の（仮）。GO 資料で見て合意する）

- D8（仮） 「設定のファイルを指して起こしたサーバ」の判定と、Linux の試験用の台本 `tools/settings-server.sh`（ST60 が Linux の口を作ったら替える）
- D9（仮） 読み出しの記録の順と語（変えた行は書き戻しの前に書く・打ち消しの行・移行を足さない）。台帳 `provisional.jsonl` に載せた（`review/spec.md#R2`）
- D10（仮） 口と本文の形（`POST /web-password`・`POST /collector-token`）
- D11（仮） 画面の節の並びと文言

## capability

`data-sensitivity` だけ（INDEX.md の割り当てどおり）。ADDED 2 本・MODIFIED 2 本（ログインの無効化を「変わった後の起動で」から「画面から変えたときはその場で」へ・読み出しの記録に残すものに変える求めを足す）。前倒しは無い。
正典 `distribution` の「秘密を画面に出さない」と、共有の値を本人の画面に出す口（ST56 から）の食い違いは、ここでは直さず `docs/handoff/ST60.md` へ申し送る（Task 8.2）。

## tasks

9 Task・14 項目（人間の確認待ちは無い。画面は e2e、Windows のサービスとインストーラは CI の `install-windows`）。ST59 が印を置く Scenario は 33 本。

## レビュー

- 深掘りの独立レビュー（`review/deep.md`）: 13 件（escalated 4・fixed 8・rejected 1）
- spec の独立レビュー（`review/spec.md`）: 13 件。すべて fixed（うち R2 は失う側のある指摘で、P の条件を満たすので D9 を失わない側に直して仮の台帳に載せた）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
