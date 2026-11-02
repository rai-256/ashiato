# ST44 収集アプリと画面の資格情報が読める範囲を締める（上流）

change: `openspec/changes/st44-collector-credential-scope/`。満たす要件は PERM-10。`requires: [ST28]`。扉 #13。

いまの API の合言葉（`API_TOKEN`）は、携帯の APK と PC の台本が同じ値を持ち、その値でサーバの全部の口が通る。これを、送る 5 つの口と、通知と送信に要る 4 つの読み
（送信契約の版・原本を受け取り済みか・気分の通知の判断・途絶の判定。どれも記録の中身を返さない）の**許可の一覧**だけに締める。一覧の外は 403 で断り、読み出しの記録に 1 行残す。
全部を読めるのは画面のログインだけになる。画面のログインの印の鍵は `API_TOKEN` からサーバだけが持つ `WEB_SESSION_KEY` に替える（収集アプリの値と DB の写しから画面の合言葉を総当たりできなくする）。
配った収集アプリは変えず、入れ直しも要らない。道具（縦串・e2e・録画・偽データ・確認の手順書）は、画面の合言葉でログインして読む形に書き直す。

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。本人の答えが推奨と違えば `spec --answers` が書き直す。

## 深掘りで本人が決めること（A。いまは推奨で書いた）

- **Q1**（loss: exported）: いまの API の合言葉そのものを送るだけに締める（推奨。design D1）。代替は収集アプリ用の合言葉を新しく作って `API_TOKEN` を全部読めるまま残す
- **Q2**（loss: uncaptured）: 配り方は共有の 1 本のまま（**本人の選択。推奨「端末ごとに発行し ST56 が作る」と違う**。design D2）。端末ごとの発行も「どの資格情報で届いたか」の欄も作らない。どの端末の資格情報で届いたかはずっと持たない。要件は FR-128 ★ 2026-11-02
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
- spec レビュー（`review/spec.md`）: 14 件 —— fixed 11（D9 の 401 を先に返す形・偽データの道具の書き直し・D10 の Scenario・導出元の食い違いなど）・escalated 1（Q2 の要件の★の担い手。Q2 の答えで PERM-10 の★は不要に）・rejected 2（ST56・ST37 への申し送り。Q2 で共有の 1 本を選んだので中身が無い）

---

# 下流（実装）の結果

## 未決（A。`deep.md` に積んだもの）

なし。

## merge 後に本人が行うこと（review/code.md R15）

1. **新しいサーバに入れ替える前に**、本人の `.env` に `WEB_SESSION_KEY` を 1 行足す —— `tools/env-init.sh`（無い行だけを乱数で足す。ほかの行は変えない）か、`openssl rand -hex 24` の値を手で。
   16 文字以上で、`API_TOKEN` とも `WEB_PASSWORD` とも違う値。足さないと、サーバは `kind=web_session_key reason=missing` を出して起動せず、`tools/dev.sh`・`tools/stack.sh` も理由を出して止まる（D8（仮）・D16（仮））。
   その間、収集アプリは送れない分を溜めて待つ（携帯 90 日・2 GB）ので記録は失わない
2. 入れ替えた最初の起動で、全部のブラウザのログインが 1 回切れる（印の鍵が替わるため）。画面の合言葉を入れ直せば戻る
3. 手で打つ読みの `curl` は、API の合言葉では 403 になる。`tools/web-login.sh http://<サーバ>` が出す `cookie: …` の見出しを付けて読む（`docs/network.md` §4・§6）

## 仮決め（B）と Ruling

- 上流の仮: D8・D10・D11・D12（上の節のまま）
- **D16（仮）（finish で追加）**: 日々の起動の入口 `tools/dev.sh`・`tools/stack.sh` は `WEB_SESSION_KEY` に既定値を持たず、無ければ止まる（使い捨ての DB を作る `smoke.sh`・`check-panic-log.sh` は既定値のまま）。
  既定値のままだと、鍵の無い `.env` で repo に載った鍵を使って起動し、D7 で塞いだ「DB の写しから画面の合言葉を確かめられる」状態が戻る（code_verify R17・/code-review R19）。
  反転条件: ハーネスの検証の経路が鍵の無い `.env` で `stack.sh` を呼んで止まる事例が出たら、`stack.sh` が起動の前に `env-init.sh` を呼ぶ形へ移る。
  code_verify は R17 を `premise` としたが、崩れたのは本人の答えでなく AI の仮決め D8 の中の食い違い（「無ければ起動しない」と「渡す側は台本の既定値」）なので、finish が `conflict` に直して仮で閉じた。仮の台帳（`provisional.jsonl`）に載せたので、merge の前に別の席が確かめる
- ledger の Ruling（全 Task）:
  - Task 7: 複数行の curl の空振りは finish では直さず park し、change で直す（続きの行をつないでから curl を取り、資格情報があるのに口が読めない curl は rc=1） — SDD の final の fix は 1 回だけで、今ある 4 か所は許可の口なので今の木に誤りは無い — 間違っていたときの費用: 後の Story が smoke.d に複数行の curl で API の合言葉の読みを足すと CI を通り、縦串の実行時に 403 で初めて分かる（記録は失わない。直すのは検査器の数行）
  - ほかの Task の ledger に Ruling は無い

## Task

8 Task・13 項目を全部埋めた（`- [ ]` 0）。残した Task は無い。

## 通した検証（`evidence.jsonl`。全部 PASS）

- 1.1 `cargo test -p ashiato-server st44_scope_`（15 本）/ 1.2・8.1 `cargo test -p ashiato-server`（全体。1916 passed）
- 2.1 `st44_routes_`（3）・`every_route_refuses_satellite` / 3.1 `st50_`（42）・`matches!(caller, Caller::ApiToken)` が無い
- 4.1 `--test server_startup`・`st44_api_token_change_keeps_login`・`st44_session_key_change_revokes`・`st44_secret_tag_not_from_api_token`・`web_session`（27）/ 4.2 `tools/check-env-init-session-key.sh`
- 5.1 `tools/web-login.sh --self-test` / 5.2 `tools/smoke.sh`・`check-shared-appends.sh`・`bash -n`
- 6.1 `npm run test:e2e`（243 passed）・`npm run typecheck` / 6.2・7.1 `tools/check-api-token-reads.sh --self-test` と各台本
- 7.2 `tools/check-openapi.sh` / 8.1 `check_scenarios.py`・`check-immutable.sh`・`check-panic-log.sh`
- finish の fix（04703a4e）の後: clippy `-D warnings`・`cargo test` の `st44_`・`access_log`・`web_session`・`server_startup`（39）・`tools/smoke.sh`・e2e 3 本（59 passed）・typecheck・verify-run 4.2/5.1/6.2/7.1/7.2 PASS。
  全体の `cargo test` と全 e2e は fix の後に走らせていない（CI が持つ）

## コードレビュー（`review/code.md`）

- final review: Critical 0 / Important 2 / Minor 13 → R1〜R14 fixed、R15 は上の「merge 後に本人が行うこと」
- code_verify（手 1〜7）: R16（検査が bash の台本で空振り）・R17（dev.sh・stack.sh が公開の鍵で起動）・R18（鍵の長さの境界を試していない）
- security review: 新しい指摘なし（API の合言葉は全口で許可の一覧を通ることを 264 求めの変種で確かめた）
- /code-review: R19〜R28 の 10 件
- finish: R16〜R19・R21〜R28 fixed（R17・R19 は D16（仮））、R20 rejected（録画の `.env` は毎回鍵つきで書き直され、既存の Scenario が行を足すことを禁じる）。scoped re-review で全部 ADDRESSED

## 未処置の申し送り

- `tools/check-api-token-reads.sh` は `\` で続く複数行の curl と `BASE="http://$BIND"; curl … "$BASE/<口>"` の形を黙って飛ばす（finish の scoped re-review の Important。上の Task 7 の Ruling）。`hx change` で直す
- `docs/handoff/ST44.md` の 2 件（`GET /feelings/reminder`・途絶の判定の口を収集アプリの資格情報で読める）は許可の一覧に入れ、試験を置いた（処置済み）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
