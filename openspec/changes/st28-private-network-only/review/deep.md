# ST28 深掘りの独立レビュー（deep-questions.json / deep.md）

schema の手順 1〜5 を、問いの一覧を見る前に自分でやり直してから、Q1〜Q5 と C1〜C3 に突き合わせた。**問いの JSON と deep.md は触っていない。**
網のホスト名・網のアドレス・私設 IP はこのファイルに書いていない（実行結果は伏せ字にして読んだ）。

確かめた範囲: `docs/stories/ST28.md`・`docs/briefs/ST28.html`（`story_brief.py ST28` で再生成。R103 が出る）・`docs/stories/INDEX.md`、
`docs/requirements.md`（§1.4 / PERM-3 / PERM-7 / PERM-10 / NFR-7 / NFR-12 / NFR-15 / §5 技術的制約 / 扉 15 / 扉 23 / 扉ではないもの / EXT-K / EXT-L）、
`docs/production-prep.md`（A-1 / A-2 / B の自動セキュリティレビュー / C の既定値の棚卸し）、`docs/ui-direction.md`、`docs/screens.md`、
`openspec/specs/record-envelope/spec.md`、`openspec/changes/archive/2026-09-10-st01-location-ingest/design.md` D26、
`openspec/changes/archive/2026-09-14-st03-idempotent-ingest/review/code.md` R103、
`crates/server/src/lib.rs`（`authorize` / `run` の `BIND`・route）、`crates/collector-windows/src/{config,clock}.rs`、
`web/vite.config.ts`、`web/src`（外部 URL・端末内の保存）、`tools/{stack,dev,verify-prep,check-private}.sh`、`docker-compose.yml`、`.env.example`、
`.github/workflows/ci.yml`、`collector-android/app/build.gradle.kts`・`collector-android/README.md`、
並走中の change（`st06` / `st08` / `st12` / `st22`）の deep・design・tasks と `feat/*` ブランチの `main...` との差分。

**実行した確認**（どれも読むだけ。設定は変えていない）:

- `ss -ltnp`（アドレスは伏せ字）: DB は `127.0.0.1:55432` だけ。網のアドレス上の 18787 / 8787 / 5173 / 5180 / 5199 は tailscaled の serve が持つ口で、サーバ・画面自身は網のアドレスで待ち受けていない
- `tailscale serve status`: ashiato 用の 5 つの口はすべて **http**（tailnet only）で `127.0.0.1` へ proxy。https は操作盤（2024）の 1 つだけ
- `tailscale serve status --json`: キーは `TCP` / `Web` だけで `AllowFunnel` は無い（deep.md の C2 の前提と一致）
- `tailscale status --json`（名前は出さずに数えた）: **自分を含めて 3 ノード**（この WSL・Windows 側・Android）、利用者 1、共有で入ってきたもの 0、タグ付き 0
- `grep` で `web/src` `web/index.html` `crates/server/src` `collector-android/app/src/main` に外部への URL が 0 件、`web/src` に `localStorage` / `indexedDB` / service worker が 0 件、サーバと画面に `Cache-Control` が 0 件
- 追跡ファイルにこの機械の短い名前が 0 件（名前は出力せずに数えた）

手順ごとの結果:

- 手順 1（要件どうしの衝突）: R1（NFR-15 と PERM-3 / FR-60 / 扉 15）/ R2（Q1 の選択肢と PERM-10）/ R11（§1.4・§5・Story の完了判定が事業者名を残す）
- 手順 2（扉の幅）: 扉 23 そのものに ST28 の文脈での新しい幅は見つからなかった（確かめた範囲: 扉 23 の文面・ST30 / ST31 の関与要件・NFR-9。ST28 が持つのは「バックアップ以外は出ない」側だけ、という deep.md の読みと一致）。
  扉ではないが幅のある文面 —— PERM-7「明示的に許可した私設網の**内側**」が、自分の端末の上の他のアプリまで含むか（R3）
- 手順 3（新たに立つ一方通行）: R3・R4（exported）/ R5（公開の証明書ログ = exported）/ R9（誰が読んだかを記録しない = uncaptured）
- 手順 4（日常に影響する選択）: Q1 / Q2 / Q5 が拾っているものに加えて、端末を落としたときの手順（R4）。常時通知・電池・容量には新しい判断は無い（収集アプリの送信の仕組みは変わらない）
- 手順 5（既存コードが要件を満たしていない箇所）: R4（収集アプリの APK が全読みの合言葉を持つ）/ R6（`record-envelope` の正典が既に要求している門を、superuser の共有パスワードが外せる）/ R10（外部資源を読まないことの機械の守りが無い）

C のうち**指摘しなかったもの**: C1（`lib.rs:1815` は `BIND` を検査せずに bind する。`tools/stack.sh:83` の案内とも一致し、前提は正しい）。

---

## R1. NFR-15「拠点外に出るのは暗号化されたバックアップのみ」が、PERM-3 と扉 15 が既に許した「外部 AI に出す」と衝突している。一覧に無い
- 分類: 抜け
- 成果物: openspec/changes/st28-private-network-only/deep-questions.json / deep.md（手順 1）
- 根拠: docs/requirements.md:843（NFR-15）/ docs/requirements.md:606-617（PERM-3「外部 AI に出してよい」が既定。ST07 は題名と URL、ST16 は滞在を外部 AI で答えられる側に置いた）/ docs/requirements.md:537-539（FR-59 / FR-60。MCP 経由で外部 AI に返す）/ docs/requirements.md:999-1011（扉 15。「締める前に外部 AI へ出た分は戻らない」を本人が受け入れている）。deep.md の手順 1 は「PERM-7 と NFR-15 の間に直接の衝突は無い」と Story 内だけを見ている
- kind: conflict
- 提案: ST28 は NFR-15 を spec に書く Story なので、「拠点外に出る」が**保管**（写しが外に残る）を指すのか、**送出**（AI への問い合わせを含む）を指すのかを本人に問う。文面どおり書くと ST27 が spec と矛盾し、緩く読むなら NFR-15 の訂正（★）が要る。Q3 の「画面に映すのは『存在』に当たらない」という読みも同じ問いの一部にする（R7）。
- 処置: escalated — Q3 として問う（NFR-15 の読み方。B / conflict、推奨: 本システムが写しを拠点外に置くこと）。deep.md の手順 1 を直した

## R2. Q1 の選択肢 3（と、同じ PC の経路については選択肢 2）は PERM-10 を満たさない。context がそれを書いていない
- 分類: 分類違い（要件で幅が既に狭まっている）
- 成果物: openspec/changes/st28-private-network-only/deep-questions.json（Q1 の context・選択肢 2 / 3）
- 根拠: docs/requirements.md:635-642（PERM-10「すべての API 要求に、呼び出し元を識別できる資格情報を要求する」。追加の理由は「同じ PC の中から呼ぶものを何も止めない」）/ web/vite.config.ts:9,17-23（proxy が `authorization: Bearer <API_TOKEN>` を付ける。画面から来た要求は、呼び出し元が何も出さずに合言葉付きになる）/ docs/production-prep.md:179（B の自動セキュリティレビューで「loopback に閉じているだけでは足りない」として合言葉を全経路に要求した）
- kind: conflict
- 提案: context に「選択肢 3 は PERM-10 を満たさない（いまの proxy が既に満たしていない）。選ぶなら PERM-10 の訂正が要る」と書き、選択肢 2 にも「同じ PC の経路は PERM-10 を満たさないまま」を足す。要件どおりに読むなら、Q1 の「確かめるか」は決まっていて、残る幅は「どう確かめるか」だけになる。
- 処置: escalated — Q1 の context と選択肢 2 / 3 に「PERM-10 を満たさない（選ぶなら PERM-10 の訂正が要る）」を書いた

## R3. Q1 の選択肢 2（Tailscale の本人 ID）の不可逆の記述が狭い。本人の端末の上の他のアプリ・Windows 側のプロセスは、偽らずに本人の ID を付けて届く
- 分類: 不可逆の記述が誤り / 前提が誤り
- 成果物: openspec/changes/st28-private-network-only/deep-questions.json（Q1 の context「端末 2 台」・選択肢 2 の `irreversible`）
- 根拠: `tailscale status --json` を数えた結果（自分を含めて 3 ノード: この WSL・Windows 側・Android。どれも同じ利用者でタグ無し）。`tailscale serve` は網の内側から来た要求に、その端末の持ち主の利用者のヘッダを付ける —— Android の Tailscale は端末全体の VPN なので、**その電話に入っている他のアプリ**が serve の口を叩いても本人の ID が付く。Windows 側のノードで動く任意のプロセスも同じ。選択肢 2 の記述は「同じ PC の別プロセスが 127.0.0.1 へヘッダを偽って送る」経路しか挙げていない
- kind: irreversible
- loss: exported
- 提案: 選択肢 2 の `irreversible` に「網に入れた本人の端末の上の他のアプリ・プロセスが読んだ記録は戻らない」を足し、選択肢 3 も同様に書き直す。context の「端末 2 台」を実際のノード数（同じ PC が 2 ノードとして入っている）に直す。PERM-7 の「内側」がどこまでを含むかの幅として扱う。
- 処置: escalated — Q1 の選択肢 2 / 3 の irreversible に「本人の端末の上の他のアプリ・Windows 側のプロセス」を足し、context を「ノード 3 つ」に直した

## R4. 収集アプリの APK が、全記録を読める API の合言葉そのものを持っている。電話を落とすと Q1 / Q5 に関係なく全記録が読める。一覧に無い
- 分類: 抜け（Q5 の why の前提も崩れる）
- 成果物: openspec/changes/st28-private-network-only/deep-questions.json（Q1 の context・Q5 の why / context・選択肢「期限なし」の detail）
- 根拠: collector-android/README.md:40（`ashiato.apiToken=<.env の API_TOKEN と同じ値>`）/ collector-android/app/build.gradle.kts:22-23（BuildConfig に埋め込む）/ crates/server/src/lib.rs:241-247（合言葉は 1 本で、`/events` も同じ合言葉で全件を返す）/ tools/verify-prep.sh:36（同じ APK を `dist/verify-<tag>/` に置く）。落とした電話は網のノードのまま残るので、PERM-7 の網の制限もこの経路を止めない。Q5 の why「短いほど端末を落としたときに安全」は画面のログインにしか当てはまらない
- kind: irreversible
- loss: exported
- 提案: 問いを 1 つ足す（A / exported）—— 「収集アプリの合言葉で読めるものを、この Story で送信だけに絞るか / ST29（PERM-10 の配り方）まで全読みのまま置くか」。どちらでも、電話を落としたときの手順（網からノードを外す・合言葉を変えて APK と C-02 を入れ直す）を Q5 の context に書く。Q5 の why は「画面について」と範囲を明記する。
- 処置: escalated — Q4 を足した（A / exported。収集側の合言葉を送る経路だけに絞るか）。電話を落としたときの手順を Q4 の context に、Q6 の why に「画面について」を書いた

## R5. Q2 の推奨（`tailscale serve` の HTTPS）の選択肢に「公開の証明書ログに機械名と網の名前が載る」が書かれていない。配布先の既定になる
- 分類: 不可逆の記述が無い / 分類違い（daily）
- 成果物: openspec/changes/st28-private-network-only/deep-questions.json（Q2 の選択肢 1 / kind）
- 根拠: deep.md の手順 3 は「HTTPS の証明書は機械名を公開の証明書ログに載せるが、この網では既に載っている」と書くが、問いの選択肢にはこの一文が無い。`tailscale serve status` で ashiato 用の口はいま全部 http で、HTTPS はこの Story で ashiato の既定になる。docs/requirements.md:628-632（PERM-7 は「配布した全利用者に波及する」ことを理由に事業者名を外した —— 配布先の利用者が推奨どおりに有効にすると、その人の機械名と網の名前が消せない形で公開される）。Q2 の日常への影響は APK の入れ直し 1 回で、daily（常時・毎日）の型に合わない
- kind: irreversible
- loss: exported
- 提案: 選択肢 1 に `irreversible`「配布先で HTTPS を有効にすると、機械名と網の名前が公開の証明書ログに載り消せない（この網では 2026-09-26 に既に載っている）」を足す。本人の網では新たに出るものが無いので問い自体は B のままでよいが、kind は daily でなく「配布の既定の選択」として書き、配布の手順書に同じ注意を載せる D 番号を残す。
- 処置: escalated — Q2 を A（loss: exported）へ上げ、推奨の選択肢に証明書ログの irreversible を書いた。http の選択肢を足した

## R6. Q4 の推奨は「同じ PC の中を塞ぐ」と書くが、superuser のパスワードが公開のリポジトリにある固定値なので、役割を分けても同じ PC からは外せる。しかも門を外から守ることは正典が既に要求している
- 分類: 前提が誤り
- 成果物: openspec/changes/st28-private-network-only/deep-questions.json（Q4 の context・選択肢 1 / 3 の detail）
- 根拠: docker-compose.yml:5-6（`POSTGRES_USER: ashiato` / `POSTGRES_PASSWORD: ashiato` の直書き。`.env` から読んでいない）/ .env.example:2 / .github/workflows/ci.yml:66-67（同じ値）。役割を分けても、同じ PC のプロセスは `ashiato` / `ashiato` で superuser として 127.0.0.1:55432 に入り、`session_replication_role` も `DISABLE TRIGGER` も打てる。openspec/specs/record-envelope/spec.md:341（「これらの制限を、取り込み口の外から加えられた操作にも適用する」）—— 選択肢 3「やらない」は正典の要件を満たさないまま残すことになるが、detail は「Risks に残す」としか書いていない
- kind: premise
- 提案: 選択肢 1 の detail に「superuser のパスワードを `.env` の秘密に移すことまで含めて初めて同じ PC の経路が塞がる」を足す（または選択肢として分ける）。選択肢 3 の detail に「record-envelope の正典（spec.md:341）を満たさないまま置く。正典の訂正か followup が要る」を書く。
- 処置: fixed deep-questions.json — Q5 の context に公開の固定パスワードと正典の要求を書き、選択肢 1 を「.env の秘密に移すところまで」、選択肢 3 に正典の訂正か followup が要ることを書いた

## R7. Q3 は問うている部分（キャッシュを残すか）が「既定は厳しい側」で決まる C で、衝突の本体（画面に映すのは NFR-15 の『存在』に当たらない）は AI が why の中で決めている
- 分類: 分類違い / 不要
- 成果物: openspec/changes/st28-private-network-only/deep-questions.json（Q3）
- 根拠: deep-questions.json:61（why が「**画面に映すのは NFR-15 の『存在』に当たらない**と読む前提で」と読みを確定させている）。推奨の no-store は厳しい側で、費用は応答ヘッダ 1 つ（`grep` でサーバ・画面に `Cache-Control` 0 件、`web/src` に端末内の保存 0 件）。選択肢 2 の `irreversible`「後から no-store にしても端末から消えない」は、`Clear-Site-Data` 応答か本人の操作で消せるので誤り（それでも失われるのは「その間に電話を落とした」場合だけで、B と矛盾する記述になっている）
- kind: conflict
- 提案: no-store を C4 として D 番号に残し、Q3 は削る。NFR-15 の読み（画面に映す・端末に積む・外部 AI に出す）は R1 の問いに寄せて本人に返す。
- 処置: escalated — 当初の Q3（端末に残すか）を削り C4（no-store）へ。NFR-15 の読みは R1 の Q3 に寄せて本人に返す

## R8. Q4 の推奨は並走中の change と重なる。ST12 / ST22 の移行が表・ビュー・関数を足しているので、アプリ用の役割を作ると権限の付与が要る。kind も daily ではない
- 分類: 前提が誤り（費用の見積もり）/ 分類違い
- 成果物: openspec/changes/st28-private-network-only/deep-questions.json（Q4 の kind・選択肢 1 の「2〜3 Task」）
- 根拠: `git diff main...feat/st12-archive-ingestion -- migrations/`（`core.archive_ledger` ほか 5 表）/ `git diff main...feat/st22-record-deletion -- migrations/`（`core.deletion_ledger`・関数 2・`core.stay_erased` ビュー）/ crates/server/src/stay_tests.rs:1137・attributes_tests.rs:212（試験が `CREATE DATABASE` を打つ = 非 superuser の役割では CREATEDB が要る）/ openspec/specs/record-envelope/spec.md:341（門は `record-envelope` の capability。ST28 の `data-sensitivity` ではない。docs/stories/INDEX.md:68,77）
- kind: technical
- 提案: 選択肢 1 の detail に「並走中の ST12 / ST22 の表にも付与が要る（`ALTER DEFAULT PRIVILEGES` で機械的に揃えるか、後から付与の移行を足す）」「触るのは record-envelope の門」を書く。kind は daily ではなく範囲の問い（open）に直す。
- 処置: fixed deep-questions.json — Q5 の kind を open に直し、ST12 / ST22 の表への付与・CREATE DATABASE の権限・record-envelope の門・3〜4 Task を書いた

## R9. 誰がいつ読んだか（経路・資格情報の種類）を記録しない。後から「落とした電話の合言葉が使われたか」を確かめられない。一覧にも C にも無い
- 分類: 抜け
- 成果物: openspec/changes/st28-private-network-only/deep.md（C1〜C3）
- 根拠: crates/server/src/lib.rs:241-260（`authorize` は失敗だけを `warn` に出す。成功した読み出しは何も残さない）/ docs/production-prep.md:49（ログは件数・ソース名・所要時間・エラーの種別だけ）。Q1 / R4 が扱う exported は「見られたかどうか」が分からないと、事後に範囲を決められない。記録しなかった期間の読み出しは後から足せない
- kind: irreversible
- loss: uncaptured
- 提案: 「扉を開けたままにする既定（台帳は追記のみ / 列を持つ）」として C に足す —— 時刻・経路（loopback / 網）・資格情報の種類（画面のログイン / 収集の合言葉）・route だけを追記のみで残し、中身と識別子の文字列は残さない（A-2 と揃える）。本人に返すなら B（保持期間）で足りる。
- 処置: escalated — 問いにはしない（持つ側は何も失わず費用が小さい =「台帳は追記のみ」の既定）。deep.md の C5 として記録し、HTML の冒頭で C の一覧として本人に見せ、異論は C 番号で受ける

## R10. NFR-15 を画面の側で機械的に守る C が無い。画面が外部の資源（地図のタイル・字体・CDN）を読むと、座標や記録が網の外へ出る
- 分類: 抜け（C）
- 成果物: openspec/changes/st28-private-network-only/deep.md（C1〜C3）
- 根拠: `grep` で `web/src` と `web/index.html` に外部 URL 0 件（いまは出ていない）/ docs/ui-direction.md:243（画面に地図のピンがある。タイルを外から読めば表示範囲の座標が出る）/ docs/requirements.md:843（NFR-15）/ docs/requirements.md:49（地図アプリは衛星だが、本体の画面にも地図の表現がある）
- kind: technical
- 提案: C4 として「画面の応答に `Content-Security-Policy: default-src 'self'`（接続先も自分だけ）を付け、e2e で外部への要求が 0 件であることを測る」を足す。いまは出ていないので失われるものは無く、問いにはしない。
- 処置: fixed deep.md — C6（CSP を self に限り、e2e で外部への要求 0 件を測る）

## R11. §1.4・§5 の技術的制約・Story の題と完了の判定が「Tailscale」の名前を残し、2026-09-08 に事業者名を外した PERM-7 と食い違う。手順 1 の記録に無い
- 分類: 抜け（記録）
- 成果物: openspec/changes/st28-private-network-only/deep.md（手順 1・C2）
- 根拠: docs/requirements.md:55（§1.4「外部からは Tailscale 網内でのみ届く」）/ docs/requirements.md:867（§5「外部からの到達は Tailscale 網内に限る（PERM-7）」）/ docs/stories/ST28.md:43-44（完了の判定が「Tailscale 網内の端末から」）/ docs/requirements.md:628-632（PERM-7 の訂正）/ deep.md の C2（`tailscale funnel` を直接見る検査 = 事業者に依る）
- kind: technical
- 提案: 訂正の日付が新しい PERM-7 に揃える、と deep.md の手順 1 に C として書く（spec は「利用者が明示的に許可した私設網」で書き、Tailscale は design の既定の手段に置く）。C2 は「選んだ手段が Tailscale のときの検査」と条件を付ける。§5 の行の訂正を proposal に含める。
- 処置: fixed deep.md — C7（spec に事業者名を書かない。§1.4・§5・ST28 の題と完了の判定は record で PERM-7 に揃える）。C2 に「手段が Tailscale のとき」の条件を付けた

## R12. C3 は `tools/check-private.sh` の既存の規則に頼るが、その規則は網の短い名前（MagicDNS の `http://<名前>:<port>`）を見ない
- 分類: 抜け（C の穴）
- 成果物: openspec/changes/st28-private-network-only/deep.md（C3）
- 根拠: tools/check-private.sh:20-28（見るのは `*.ts.net` と IP の形だけ）/ `tailscale serve status` は ashiato の口を短い名前の URL でも案内する。いま追跡ファイルに短い名前は 0 件（確かめた）
- kind: technical
- 提案: C3 に「pre-commit では手元の `tailscale status --json` の自分の名前も禁止語に加える（名前はリポジトリに書かず、その場で読む）」を足すか、C3 の範囲が `*.ts.net` と IP だけであることを明記する。
- 処置: fixed deep.md — C3 に手元の機械の短い名前をその場で読んで禁止語に加えることを足した
