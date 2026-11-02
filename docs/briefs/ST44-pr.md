# ST44 収集アプリと画面の資格情報が読める範囲を締める（上流）

change: `openspec/changes/st44-collector-credential-scope/`。満たす要件は PERM-10。`requires: [ST28]`。扉 #13。

いまの API の合言葉（`API_TOKEN`）は、携帯の APK と PC の台本が同じ値を持ち、その値でサーバの全部の口が通る。これを、送る 5 つの口と、通知と送信に要る 4 つの読み
（送信契約の版・原本を受け取り済みか・気分の通知の判断・途絶の判定。どれも記録の中身を返さない）の**許可の一覧**だけに締める。一覧の外は 403 で断り、読み出しの記録に 1 行残す。
全部を読めるのは画面のログインだけになる。画面のログインの印の鍵は `API_TOKEN` からサーバだけが持つ `WEB_SESSION_KEY` に替える（収集アプリの値と DB の写しから画面の合言葉を総当たりできなくする）。
配った収集アプリは変えず、入れ直しも要らない。道具（縦串・e2e・録画・偽データ・確認の手順書）は、画面の合言葉でログインして読む形に書き直す。

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。本人の答えが推奨と違えば `spec --answers` が書き直す。

## 深掘りで本人が決めること（A。いまは推奨で書いた）

- **Q1**（loss: exported）: いまの API の合言葉そのものを送るだけに締める（推奨。design D1）。代替は収集アプリ用の合言葉を新しく作って `API_TOKEN` を全部読めるまま残す
- **Q2**（loss: uncaptured）: 配り方は共有の 1 本のまま締め、端末ごとの発行と「どの資格情報で届いたか」の欄は ST56 が作る（推奨。design D2）。推奨のまま答えが来たら、record の段が PERM-10 の★を足す
- **Q3**（B）: 画面のログインで通る範囲は変えない（推奨。design D3）

## 仮決め（B。`provisional.jsonl`）

- **D8（仮）** 印の鍵は環境変数 `WEB_SESSION_KEY` で渡し、無い・16 文字未満・API の合言葉か画面の合言葉と同じなら起動しない。反転条件: 自動の更新が `.env` に行を足せず起動しない事例が出たら、サーバが鍵を作って DB の外のファイルに残す
- **D10（仮）** 衛星の形でなく API の合言葉とも一致しない `Bearer` と有効なログインの印の組は、今のまま印で通す（ST29 の申し送りを閉じる）
- **D11（仮）** API の合言葉とログインの印を両方付けた求めは API の合言葉として扱う（読みは 403）。読む道具は印だけを付ける
- **D12（仮）** `GET /session` は API の合言葉では 403

深掘りの AI の既定（聞かずに決めた）は D4（許可の一覧 9 つ・1 つの関数を middleware と `session_get` から引く）・D5（403 と読み出しの記録。移行は無し）・D6（呼び出し元の種類は `_` の無い `match`）・
D7（印の鍵をサーバだけの値に）・D9（資格情報を求めない口は 3 つ。資格情報の無い求めは middleware が引数より先に 401）・D13（道具の書き直し）・D14（落とした電話の手順）・D15（API の説明に 403）。

## capability

- 変える: `data-sensitivity`（ADDED 3・MODIFIED 2・REMOVED 1。「画面から記録を読むには合言葉のログインが要る」は Scenario「API の合言葉での読み書きは変わらない」を落とすため、名前を替えて置き直した）・
  `record-envelope`（MODIFIED 1。例外の 3 つ）・`subjective-log`（MODIFIED 1）・`satellite-platform`（MODIFIED 1。導出元だけ）
- INDEX.md の表で `record-envelope`・`subjective-log`・`satellite-platform` に ST44 を足した（ST44 の決定を写すだけで、後続の振る舞いの前倒しではない）。
  `device-collection`・`desktop-collection` は割り当てに残すが、推奨では収集アプリの振る舞いが変わらず delta を持たない

## tasks

8 Task（13 項目）。新しく印を置く Scenario は 23 本。移行は無い。走っている ST52 とは同じ `router` に触るが独立（ST52 は router の前で `/api` を剥がし、許可の一覧は剥がした後の道で引く）。

## レビュー

- 深掘りの独立検査（`review/deep.md`）: 10 件 —— escalated 6（Q1・Q2 に写した）・fixed 3（D10 仮・proposal・D5）・rejected 1
- spec レビュー（`review/spec.md`）: 14 件 —— fixed 11（D9 の 401 を先に返す形・偽データの道具の書き直し・D10 の Scenario・導出元の食い違いなど）・escalated 1（Q2 の要件の★の担い手）・deferred 2（ST56・ST37）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
