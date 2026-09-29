## Why

**画面に届いた人は、合言葉なしで全記録を読める。** 画面を配る vite の proxy が API の合言葉（`API_TOKEN`）を付けているので、
網のノードの上のどのアプリも・同じ PC のどのプロセスも、画面の URL を叩けば PERM-10 を素通りする。
加えて、網の外に開かないことを守るのは「いまの設定がたまたまそうなっている」ことだけで、機械の側に受け皿が無い ——
`BIND` に何を書いてもサーバは起動し（`tools/stack.sh` は「LAN の IP にする」と案内している）、`tailscale funnel` が有効かは誰も見ていない。
さらに **アプリが DB の superuser で接続している**ので、記録の門（トリガ）は `SET session_replication_role='replica'` の 1 文で外れる
（ST03 のレビュー R103、`処置: deferred ST28`）。その superuser の合言葉は `docker-compose.yml` に公開の固定値で書かれている。

いま作る理由は、本人が外出先から自分のデータを見たいから（Story の価値）。外に向けて開く前に、**開いてよい範囲と、開いた先で誰が読めるか**を
機械で固定しておかないと、見られた記録は戻らない（loss: exported）。

## What Changes

深掘りは **1 巡・6 問**（A 3 / B 3）。本人がすべて選択肢から選んだ（未回答 0・推奨のまま 0）。**推奨と違う側を選んだ問いが 2 つ**（Q4 / Q6）。
独立レビュー（`deep-review`）は 12 件 —— 問いを 2 つ足し、1 つを A に上げ、前提と文面を 5 件直し、聞かないで決める既定を 4 件足した・直した。
聞かずに決めた既定 C1〜C7 に本人の異論は無かった。

- **画面は合言葉のログインを求める**（Q1）。ログインしていないブラウザには、画面の経路から 1 件も記録が返らない。
  画面を配る側は API の合言葉を付けなくなる。ログインの印はブラウザのスクリプトから読めない
- **ログインは期限で切れない。ログアウトするまで覚えている**（Q6。推奨の 30 日を覆した）。
  端末を落としたときの締め出しは、**画面の合言葉を変えるとすべての端末のログインが無効になる**ことで行う
- **収集側の合言葉（`API_TOKEN`）が読める範囲は変えない**（Q4。推奨を覆し、ST29 まで全読みのまま置く）。
  ST29 へ `docs/handoff/ST29.md` で申し送った
- **網の内側から届く口は暗号化された接続だけ**（Q2）。既定の手段は `tailscale serve` の HTTPS。サーバと画面は loopback で待ち受ける。
  収集アプリは loopback 以外へ平文で送らない（ST01 D26 の平文の例外を外す）
- **サーバと画面は loopback でしか待ち受けない**（Q2 / C1）。loopback 以外のアドレスでは起動しない。
  deep の C1 は「明示した網なら許す」だったが、Q2 の答え（127.0.0.1 から動かない・網の口は暗号化だけ）と食い違うので厳しい側に揃えた（design D7（仮）。レビュー R1）
- **網の外に開いていないことを機械で見る**（C2）。網の外への公開が有効か、loopback 以外で待ち受ける口・平文で網へ出している口・網へ出している開発用の画面があれば検査が落ちる
- **アプリは DB の管理者でも表の所有者でもない役割で接続する**（Q5）。その接続からは記録の門を外せない。
  移行は所有者の役割で別に当てる。DB の管理者の合言葉を `docker-compose.yml` の固定値から `.env` の秘密へ移す
- **誰がいつ読んだかを追記のみの台帳に残す**（C5）。中身と識別子の文字列は残さない
- **応答はブラウザに記録を残させない**（C4）。**画面は外部の資源を読まない**（C6）
- **本システムは記録の写しを拠点外に置かない**（NFR-15 / Q3）。サーバが外部の宛先へ送る部品を持たないことを機械で見て、DB の接続先が loopback でなければ起動しない

**BREAKING**:
- **画面は、ログインしないと記録を 1 件も出さない**（いままで画面の URL に届けば全部見えた）
- **サーバは DB の管理者・表の所有者の接続では起動しない。** 起動の前に移行を所有者の役割で当てる手順が要る
  （`tools/stack.sh` / `tools/dev.sh` / `tools/smoke.sh` / 確認バッチの `run.sh` / CI を揃える）
- **収集アプリの接続先は `https://` になる。** 平文の接続先で作った APK は網越しに送れない（入れ直しが要る）
- **サーバは loopback 以外では起動しない。DB の接続先が loopback でなくても起動しない**
- `API_TOKEN` による取り込み口・読み出し口の振る舞いは変えない（既存の道具・試験・収集側はそのまま動く）

## 深掘りで本人が決めたこと（2026-09-29。1 巡・6 問）

全文は `deep.md`。**下流はこれを勝手に変えない。**

| # | 決めたこと | 段 |
|---|---|---|
| Q1 | 画面は**合言葉でログイン**し、端末ごとに覚えさせる | A（exported） |
| Q2 | 網の外への出し方は **`tailscale serve` の HTTPS**。サーバと画面は 127.0.0.1 から動かない。配布先の機械名が公開の証明書ログに載ることを受け入れる | A（exported） |
| Q3 | NFR-15 の「拠点外に出る」は**本システムが記録の写しを拠点外に置くこと**と読む | B |
| Q4 | 収集側の合言葉は **ST29 まで全読みのまま置く**（**推奨を覆した**） | A（exported） |
| Q5 | DB の役割分離を**この Story でやる**（superuser の合言葉を `.env` の秘密に移すところまで） | B |
| Q6 | 画面のログインは**期限なし（ログアウトするまで）**（**推奨の 30 日を覆した**） | B |

## Capabilities

### New Capabilities

- `data-sensitivity`: 誰がどの経路で記録に届くか（網の内側に限る・画面のログイン・DB の役割・読み出しの台帳・拠点外に置かない）。
  INDEX の割当は ST24 / ST28 / ST29 で、**ST28 が最初に作る**（layer 1。ST24 は layer 4）。感度（PERM-2〜6）とプラグイン権限（PERM-8）は ST24 / ST29 が足す

### Modified Capabilities

（なし）

- `record-envelope` の「すべての API 要求は資格情報を要求する」「取り込み口の外から加えられた操作にも適用する」は**文面を変えない**。
  この Story は画面の経路とアプリの DB 接続でそれを**成り立たせ直す**側で、振る舞いの契約は `data-sensitivity` に置く
  （DB の役割はアクセス制御の性質で、記録の骨格の契約ではない。record-envelope を開くと並走中の Story と capability が重なる）
- capability の前倒しは無い（`data-sensitivity` は INDEX の割当どおり）

## Impact

- **サーバ**（`crates/server/src/`）: 資格情報の確かめ方（`authorize`）にログインの印を足す / ログインの口（`/session`）/ 起動時の検査
  （待ち受けのアドレス・接続した役割・画面の合言葉）/ 読み出しの台帳 / 応答ヘッダ / 移行を別の起動に分ける（`migrate`）/ OpenAPI
- **移行**: 1 本（`YYYYMMDDHHMM_access_control.sql`。ログインの印の表・読み出しの台帳）と、移行の後に毎回当てる付与の段
- **DB の足場**: `docker-compose.yml`（管理者の合言葉を `.env` から）/ 役割を作る台本 / `.env.example` / CI の `services` と環境変数
- **画面**（`web/`）: `vite.config.ts`（proxy が合言葉を付けない・CSP・`no-store`）/ ログインとログアウト / e2e（ログインの足場を全 e2e の既定にする）
- **収集側**: Android の network security config（loopback 以外の平文を拒む）と build の検査 / C-02 の接続先の検査
- **道具と文書**: `tools/stack.sh` / `tools/dev.sh` / `tools/smoke.sh` / `tools/verify-prep.sh` / `tools/check-immutable.sh` /
  網の外に開いていないことの検査 / 拠点外への口の検査 / `.githooks/pre-commit`（手元の機械の短い名前）/ 網の手順書 / `docs/screens.md`
- **並走中の Story との重なり**（`openspec/changes/` の st06 / st08 / st12 / st22）。**どれにも差し戻さない**:
  - capability は重ならない（st06 = device-collection / st08 = desktop-collection / st12 = external-ingestion・collection-coverage / st22 = record-deletion・browsing-views）
  - **同じファイルは触る**: `crates/server/src/lib.rs`（`MIGRATIONS`・route・`run()`）、`crates/server/src/testdb.rs`（st12 がプールの持ち方を変えている）、
    `docs/openapi.json`、`web/e2e/` の既存の e2e、`tools/stack.sh`、`tools/smoke.sh`（st06 / st08 / st12）、`tools/check-immutable.sh`（st12 / st22）、
    `docker-compose.yml`（st12 が `max_connections` を足す）、`.github/workflows/ci.yml`（st06 / st08）。
    後から merge する側が main に rebase して追従する（ST28 が後なら Task 10 で揃える）
  - **st12 / st22 が足す表**: 付与は移行の配列の後に毎回当てる段で `core` の全表に掛け、既定の権限も置くので、**あちらの移行を書き換えずに**
    アプリの役割から届く（design D4）。**st12 / st22 が足す route**: 既定で画面のログインの印を受ける（design D7）ので、あちらのハンドラは変えない
  - **st12 / st22 が足す e2e**: ログインは Playwright の既定の足場（storageState）で済ませる（design D10）ので、既定の `page` を使う e2e は書き換えずに通る
- **要件**: PERM-10 に ★ 2026-09-29 補足（画面の経路は画面の合言葉のログインで満たす。design D21（仮）。レビュー R19）
- **ST29 への申し送り**: `docs/handoff/ST29.md`（Q4。収集側の合言葉が読める範囲を ST29 で絞る）
- **依存**: 足す crate は HMAC / 乱数 / cookie の読み書き / 応答ヘッダ（design D6）。ライセンスは `tools/check-licenses.sh` が見る
- **本人の機械での移行**（merge の後。design D20（仮））: DB の退避 → `.env` の秘密 → 役割 → 移行 → `tailscale serve` の https 化 → APK の入れ直し
