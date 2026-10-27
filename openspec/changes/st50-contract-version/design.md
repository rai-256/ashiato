# ST50 design — 収集アプリとサーバの版が食い違っても記録が失われない

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。Q1〜Q9 の推奨は仮でない D として書き（Q1 → D1、Q2 → D2、Q3 → D3、Q4 → D4、Q5 → D5、Q6 → D6、Q7 → D7、Q8 → D8、Q9 → D9）、
> 本人の答えが推奨と違えば `spec --answers` がその D と Scenario だけを書き直す。
> 深掘りの「聞かない既定」C1〜C9（premises.md）は D に写した: C1 → D10、C2 → D11、C3 → D7・D8、C4 → D12、C5 → D13（仮）、C6 → D14、C7 → D15、C8 → D16、C9 → D17。
> 仮の決定 D13・D18・D19・D22・D23 は `provisional.jsonl` に載せた（D13 は深掘りの独立検査 review/deep.md R10、D18・D19 は spec の段で下流が決めそうな判断を先に出したもの、D22・D23 は spec の独立検査 review/spec.md R7・R8）。

## Context

動機は proposal.md の Why。いまの形（前提の出所は `premises.md`）:

- S-01 の送る口は `POST /ingest`・`/heartbeat`・`/drops`・`/media/missing` と `PUT` / `HEAD /media/originals/{sha256}`（`crates/server/src/lib.rs` の `router`）。
  どの要求の型も `deny_unknown_fields` を持たず、知らない欄を読み飛ばす（P1。`lib.rs:1126-1131` の 1 件ごとの解釈は失敗を `malformed` にする）。`router` に `fallback` は無く、知らない道は 404（P2）
- 資格情報は `authorize` が `Caller::ApiToken`（収集アプリ）・`Caller::WebSession`（画面）・`Caller::Satellite`（衛星。送る口では 403）に分ける（`web_session.rs:43`）
- 携帯の `Sender` は 200 / 400 以外を一時的な失敗として 1 件も取り除かず（P3）、`PERMANENT_ERRORS` の 6 種だけを捨てる（P4）。原本の送り手は 409 を `record_missing` と読み、ほかは試し直す（P9）
- PC の `Sender` は `/ingest` と `/heartbeat` だけを送り、読めない応答では何も取り除かず、断られた 1 件を捨てずに「飛ばす」印だけを付ける（P5。`sender.rs:131`・`:152`）。PC の未送信に上限は無い（P6）
- 封筒の `schema_version` は 2 本とも 1 の決め打ち（P8）。まだ配った印 `v*` は 0 個（P7）。`tools/check-openapi.sh` は S-01 の型の欄が携帯にあるかの片方向だけを見て、PC は対象外（review/deep.md R12）
- 通知: 携帯は途絶の通知（`OutageNotifier`。チャンネル `outage`）と常駐の通知・上限の 7 日前の知らせ（`Retention.kt`）を持つ。PC は `toast.rs` の 1 通（`Tag = "outage"`）だけ

## Goals / Non-Goals

**Goals**: 収集アプリと S-01 の送信契約の版が食い違っても、収集アプリの記録・生存信号・破棄の報告・原本が S-01 に黙って欠けて入らない。食い違いは収集アプリの手元に残り、
本人に「サーバを更新してください」と届く。版の上げ忘れは CI が止める。今配っている収集アプリとの組み合わせ（記録が入ること）は変えない。

**Non-Goals**:
- 収集アプリの自動の更新・S-01 の自動の更新（ST53〜ST58）。S-01 が収集アプリへ「古い」と伝える逆向きの検査（S-01 は自分以下の版を全部受け付けるので要らない）
- 画面（V-01）と衛星の契約の版（D16）。衛星の契約は別に作る（`docs/collector-contract.md` の冒頭）
- 契約の版で記録を選び出す口（画面・API の絞り込み）。版の意味ごとの読み分けの実装（版 1 しか無い間は要らない）
- 生存信号の版（ST51 が足す欄。足すなら ST51 が契約の版を上げる）

## Decisions

### D1. 記録は作った時の契約の版を持つ（溜めの 1 行ごとに焼き込み、記録の表に残す）

- 収集アプリ 2 本は、行を未送信に積む 1 か所で、その時の送信契約の版を行に焼き込む（記録・生存信号・破棄の報告・原本の欠けは 1 件の欄 `contract_version`、原本の送り待ちは送り待ちの台帳 `media/originals.jsonl` の 1 行の欄）。
  送るときは積んだ値のまま送り、今の版で上書きしない（深掘り Q1 の推奨）
- S-01 は記録の 1 件の `contract_version` を `core.event.contract_version`（`integer`・空を許す）に書く。前の版の表 `core.event_version` にも同じ列を持たせて写す。
  生存信号・破棄の報告・原本の欠けの `contract_version` は検査（D4）にだけ使い、格納しない（Q1 は記録だけを問うた。版を残す要求が出たら列を足す）
- 収集アプリの資格情報で届かなかった記録（画面・S-01 の中の書庫・滞在）は空（収集アプリの送信契約に依らない。版 1 と書くと書き出しで誤読される）
- 呼び手を下へ渡す: 今の `ingest` は `authorize` の戻り値（`Caller`）を捨て、`ingest_one` / `ingest_request` は呼び手を知らない（`lib.rs:1830`・`:1116`・`:1141`）。
  `ingest_one` / `ingest_request` に「収集アプリの 1 件か」の印（`Caller::ApiToken` なら真）を渡す引数を足し、書庫の取り込み（`store_with` → `ingest_request`）と滞在・画面は偽を渡す。
  ST23 も同じ `ingest_request` に `Settle` の引数を足す（ST23 design D7・D8）—— 後から main に入る側が rebase で両方の引数を合わせる（review/spec.md R4）
- 代替: 送った時の要求の版を残す（更新の前に作った記録に新しい版が付く）・残さない（ST49 の印の commit が `unknown` の記録で戻らない）—— 深掘り Q1 の選択肢

### D2. 知らない欄は 1 件ごとの `unknown_field` で断る（`deny_unknown_fields` を使わない）

- 1 件の JSON の最上位の鍵の集まりを、その要求の型が知る欄の一覧（型から作る。手で並べない）と比べ、一覧に無い鍵が 1 つでもあれば `unknown_field`。型の解釈（`serde_json::from_value`）より**前**に見る
- `#[serde(deny_unknown_fields)]` を足すだけの実装は、知らない欄を解釈の失敗＝`malformed` にし、携帯の収集アプリが恒久として記録ごと捨てる（review/deep.md R4・R11）。使わない
- 判定は S-01 自身が知る欄で行い、要求が名乗る版の欄で行わない（review/deep.md R9）。`payload` などの中は見ない（封筒の最上位だけが契約。FR-130 の検査（D7）も最上位を見る）
- 掛けるのは `Caller::ApiToken` の要求だけ。画面のログインの要求は今どおり読み飛ばす（D16）
- 応答の状態符号は今の規則のまま（1 件以上を受け付ければ 200、1 件も受け付けなければ 400。本文は 1 件ごとの結果）。`unknown_field` を `IngestError` の列挙と `docs/collector-contract.md` の表に足す

### D3. 止めるのは収集アプリの資格情報の送る口すべて

- `POST /ingest`・`/heartbeat`・`/drops`・`/media/missing`、`PUT` / `HEAD /media/originals/{sha256}`。見出しの検査（D4）はこの 6 つの入口で、資格情報の判定の直後・本文の解釈の前に行う
- 読み出しの口（`GET /outages` など。収集アプリが途切れを問う）は止めない —— 何も格納せず、欄の脱落が起きない
- 止めている間は生存信号も届かないので、S-01 の稼働状況には途絶として出る（記録が届いていないことと一致する。途絶の知らせも同時に出うる。深掘り Q3 の選択肢 1 の detail）
- 代替: 取り込みだけを止める（生存信号・破棄の報告・原本の欠けに足した欄が古い S-01 で失われる）—— 深掘り Q3

### D4. 確かめる口 `GET /contract`、見出し `Ashiato-Contract-Version`、断りは 426

- `GET /contract` → `200 {"contract_version": <S-01 の版>}`（意味: その版以下を全部受け付ける。C4）。資格情報は D15。読み出しの記録には今の規則のまま残る（D23（仮））
- 要求の版は見出し `Ashiato-Contract-Version: <正の整数>`。本文に載せない —— 原本の `PUT` は本文がバイト列で、`HEAD` は本文を持たない。見出しなら 6 つの口に同じ形で載る
- 見出しが S-01 の版より大きい・正の整数として読めない → `426`・本文 `{"error": "contract_unsupported", "contract_version": <S-01 の版>}`（`HEAD` は本文を持てないので 426 だけ）。何も格納しない（1 件ごとの結果も返さない）
- 1 件の `contract_version`（D1）が S-01 の版より大きい・正の整数でない → その 1 件だけ `contract_unsupported`（見出しは版以下でも 1 件が嘘をつく場合の守り。収集アプリは見出しを載せた行の最大にする（D9）ので、正常なら起きない）
- 426 を選ぶ理由: 409 は携帯の原本の送り手が `record_missing` と読み 7 日で欠けにする（P9。review/deep.md R6）。426 はどの口でも未使用で、今の携帯（P3）・PC（P5）・原本の送り手（P9）のどれも捨てない
- **最初の配った印の後は、道・答えの形・見出しの名前・426 と本文の鍵を変えない**（配った収集アプリが従い続ける。review/deep.md R7）。spec の Requirement に書き、版 1 の口の形の例（`crates/server/tests/fixtures/contract/v1/wire.json`。D8）に S-01 が従うことを試験で固定し、例の書き換えを CI が落とす（D7 の 4）
- S-01 の版は `crates/server` の定数 `CONTRACT_VERSION` を既定にし、`App` の持ち物にする（試験が版 2 の S-01 を作れるように。本番は定数のまま。設定や環境変数で変えられる口は作らない）
- 代替: 問い合わせだけで確かめる（確かめてから送るまでの間に S-01 が入れ替わると、知らない欄は D2 にだけ守られる）—— 深掘り Q4

### D5. PC の通知は止まった時に 1 回、止まっている間は 1 日 1 回（別の札）

- `toast.rs` に札 `Tag = "contract"`（`Group = "ashiato"`）の通知を足す。途切れの札 `outage` と分ける（同じ札だと途切れの通知を置き換える）。出し直しは同じ札で前を置き換える
- 文言: 「サーバを更新してください（未送信 <n> 日分を PC に残しています）」。`<n>` は契約の版で手元に残している行の最も古いものを積んでからの日数（切り捨て）
- 出す時: 契約の版で止まった状態（D9）に入った契機で 1 回。止まったまま前に出した時から 24 時間経った契機で 1 回。状態を抜けたら札の通知を消す（`ToastNotificationManager.History.Remove`）
- 前に出した時刻は PC の手元の小さなファイルに持つ（再起動しても、前に出してから 24 時間経つまで出し直さない。spec の Scenario）。途切れの通知（ST14）の印と同じ置き場
- 判定と文言は OS に依らない関数にし（`#[cfg(test)]` で ubuntu の試験）、OS の口（`Toaster`）は今の `outage_notice::Toaster` の形に札を足す
- 代替: 止まった時に 1 回だけ・送る契機ごと —— 深掘り Q5（B）

### D6. 携帯は画面に出し、通知でも止まった時に 1 回・1 日 1 回

- 画面: `MainActivity` に 1 行「サーバを更新してください（未送信 <n> 日分を携帯に残しています）」を足す。止まった状態の間だけ出す
- 通知: 新しいチャンネル `contract`（「サーバの更新」）と通知 id。出す時・出し直し・消す時は D5 と同じ規則。前に出した時刻は `SharedPreferences` に持つ
- 常駐の通知の文言は変えない（未送信の日数は今どおり出る）。上限の 7 日前の知らせ（`Retention.kt`）は、止まった状態のときに文言へ「サーバを更新してください」を添える（鳴らし方は変えない）
- 止まった状態は `LocationService` の送る契機が持ち、画面は同じ状態を読む（プロセスの中の 1 つの値と、画面のために `SharedPreferences` へ写した値）
- 代替: 画面にだけ出す（要件の逐語）—— 深掘り Q6（B）

### D7. 版の上げ忘れの検査 `tools/check-contract-version.sh`

- **封筒の形の写し** `docs/contract/envelope.json`（D21 の形）を、版と一緒に持つ。作り直す道具は `tools/check-contract-version.sh --write`
- 検査（`tools/check-contract-version.sh`。CI の `check-*` の並びに足す）:
  1. 写しの S-01 の部分を `docs/openapi.json` から作り直し、commit された写しと違えば落とす（`jq` だけで走る。`docs/openapi.json` とコードのずれは `check-openapi.sh` が見る）。
     収集アプリの部分のずれは、それぞれの単体試験（`St50EnvelopeShapeTest`・`st50_envelope_shape`）が `android` / `collector-windows` の job で落とす。`--write` は 3 つ全部を作り直す（手元で gradle と cargo を走らせる）
  2. S-01・携帯・PC の版の定数が写しの `version` と違えば、どれが違うかを出して落とす（D12）
  3. 最後の配った印（`git tag --list 'v[0-9]*' --sort=-creatordate` の先頭。ベータ版の印を含む。D8）の写し（`git show <印>:docs/contract/envelope.json`）と比べ、
     欄の追加・削除・型・必須の変化があるのに `version` が印の版より大きくなければ落とす。印が 0 個なら比べない（`tools/check-released-migrations.sh` と同じ「配った」の定義。C3）。
     印はあるのに写しを取り出せなければ落とす（取り寄せの不足）
  4. 配った印ごとに、その印の版にある封筒の例（`crates/server/tests/fixtures/contract/`。D8）が今の木で変わっている・消えていれば落とす（`git diff --quiet <印> HEAD -- <例>`）
  5. `docs/collector-contract.md` の「## 送信契約の版の履歴」の表に今の `version` の行（版・最初の印・何を変えたか・版を上げた理由）が無ければ落とす
- 比べるのは S-01 の型（`docs/openapi.json` の `IngestRequest` / `HeartbeatRequest` / `DropReportRequest` / `MediaMissingRequest`）と、収集アプリ 2 本が**実際に直列化する**欄の両方（review/deep.md R12）。
  収集アプリの側だけで足された欄（古い S-01 に黙って捨てられる当の欄）は S-01 の型を見ても出ない
- 収集アプリの欄は試験が実際の直列化から書き出す: 携帯は `St50EnvelopeShapeTest`（JVM の単体試験。全部の欄を埋めた 1 件と、任意の欄を空にした 1 件を直列化して鍵と JSON の型を出す）、
  PC は `contract::tests::st50_envelope_shape`（同じ形）。どちらも環境変数 `ASHIATO_ENVELOPE_OUT` があればそこへ書き、無ければ写しの該当部分と比べて落ちる
- **CI の置き場は rust の job の `check-released-migrations.sh` の隣**（`.github/workflows/ci.yml:91-92`。その job は印を取り寄せている（`:83-90`）。別の job に置くと浅い checkout で印が 0 個に見え、黙って通る）
- `--self-test` を持つ（`$TMPDIR` の使い捨ての git の木で、印あり・印なし・欄の追加・型の変化・版の食い違い・写しのずれ・履歴の行の欠けを作り、落ちる / 通るを確かめる）
- 意味だけの変化は機械では見えない。版の履歴の「版を上げた理由」を書く規則にとどめる（深掘り Q7）
- 代替: S-01 の型の欄の追加だけを見る（型の変化・欄の削除・収集アプリだけの欄を通す）—— 深掘り Q7

### D8. 「配った」はタグ `v*`（ベータ版を含む）。版 1 は最初の `v*` で閉じる

- 配った印は `git tag --list 'v[0-9]*'`。`-beta.N` の印にも当たる。毎晩上書きされる開発版（FR-105）は印を持たないので入らない
- 最初の印までに封筒に足した欄（ST41 の終わりの時刻・ST49 の `client_version`・ST50 の `contract_version`）は全部版 1。最初の印の後に欄を足す change は版を 2 に上げる
- 配った版の封筒の例を S-01 の試験の材料に置き（`crates/server/tests/fixtures/contract/v1/` に記録・生存信号・破棄の報告・原本の欠けの今の形の例と、口の形の例 `wire.json`（`/contract` の道と答えの鍵・見出しの名前・426 と本文の鍵））、受け付けることを試験で固定する。
  版を上げたら `v<N>/` を足し、前の版の例は消さず書き換えない（FR-124。D10）
- 代替: 正式版だけ（ベータ版の間に足した欄が版で守られず、ベータ版の契約を外した S-01 に溜めた記録が断られ続ける）—— 深掘り Q8

### D9. 記録ごとに止める（S-01 が受け付ける版の行は送り、新しい版の行だけを残す）

- 送る契機ごとに、まず `GET /contract`（D18（仮））。答えの版 `A` を得たら、各送り手（記録・生存信号・破棄の報告・原本の欠け・原本）は**版 ≤ A の行だけ**から今の規則（溜めた順・まとめ送りの上限）で 1 回分を組み、
  見出しに**その回に載せた行の版の最大**を入れる（版ごとにまとめ送りを分けない —— S-01 は A 以下を全部受け付けるので、混ぜても同じ結果になる）
- 版 > A の行は送らない。**捨てない・飛ばす印も付けない**（飛ばす印は「断られた 1 件」に付けるもので、送っていない行には付けない）
- **送らずに残す**のは、版 > A の行（いつも）と、答えが受け付ける版として読めない契機の全部の行（D11）
- **S-01 の更新を待っている状態**（D5・D6 の知らせを出す間）: 次のどれかの間 ——（a）版 > A の行が未送信にある、（b）問い合わせの口が 404 を返した、（c）直前の送信が 426 だった、（d）直前の送信で 1 件以上が `unknown_field` か `contract_unsupported` だった。
  （b）〜（d）は次の契機の確かめで A が得られ、（a）に当たる行が無く、送信が 426 / `unknown_field` / `contract_unsupported` を返さなければ抜ける
- **200 で読めない本文**（公衆の Wi-Fi の入口の画面が 200 で HTML を返す。`IngestRequest.kt:76` が名指す事故）は送らずに残すが、更新を待っている状態にしない ——
  S-01 を更新しても直らないので、促すと誤った案内になる（review/spec.md R7）
- 確かめは 1 つの契機で 1 回。5 分の契機の中で続けて送る間（device-collection の「間隔を待たずに続けて送る」）は同じ答えを使う
- 版を持たない行（ST50 より前に積んだ行）は版 1（D10）
- 代替: 溜め全体を止める（古い S-01 でも受け付けられた記録まで携帯の上限で捨てられる）—— 深掘り Q9

### D10. 版を名乗らないものは版 1、配った版は外さない

- 見出しの無い要求・`contract_version` の無い 1 件・版を持たない溜めの行は版 1（C1）。ST50 より前の収集アプリはこの形で送り続け、新しい S-01 に受け付けられる
- S-01 は自分の版以下を全部受け付ける。版の数を減らす（古い版を外す）口を作らない（FR-124）
- 移行で、ST50 より前に収集アプリから入った記録（論理ソース `c01-*` / `c02-*`。登録簿の名前の頭で C-01 / C-02 が決まっている）とその前の版を 1 で埋める。ほかの記録は空のまま
- ST49 の `collector_version` が ST49 より前の記録を「不明」にする（ST49 D12）のと違うのは、ST50 より前の記録の契約の版は FR-130 の★で 1 と決まっていて推定ではないため（まだ配った印が無い。P7）

### D11. 答えとして読めないものは「受け付けない」、届かないものは一時的な失敗

- 受け付けない（その契機は何も送らない）: 404・`200` で本文が JSON でない・`contract_version` が無い・正の整数でない。このうち更新を促すのは 404 だけ（D9。200 の読めない本文は公衆の Wi-Fi の入口の画面でも起きる）
- 一時的な失敗（今どおり。止まった状態にしない・知らせない）: 到達できない・時間切れ・`401`・`403`・`5xx`・そのほかの状態符号
- 理由: 古い S-01 が 404 以外を返しても（P2 は確かさ 3）、読めない答えはどれも送らない側に倒れる（FR-125「口を持たない S-01 は受け付けないものとして扱う」）。到達できない・資格情報は更新が原因ではないので、更新を促すと誤った案内になる

### D12. 契約の版は整数 1 つ。値は 3 か所に置き、写しと CI で突き合わせる

- S-01: `crates/server/src/contract_version.rs` の `pub const CONTRACT_VERSION: u32`。PC: `crates/collector-windows/src/contract.rs` の `pub const CONTRACT_VERSION: u32`。携帯: `IngestRequest.kt` の `const val CONTRACT_VERSION`
- 3 つの値は `docs/contract/envelope.json` の `version` と CI で突き合わせる（D7 の 2。片方だけ直す事故を防ぐ。`docs/collector-contract.md` の冒頭と同じ理由）。今の値は 1

### D13（仮）. PC の未送信には上限を足さない（FR-125 の上限は C-01 に掛かる）

- PC は契約の版で止まっている間も行を溜め続け、捨てない（NFR-7 の★「上限は C-01 だけ」。premises P6）。FR-125 の★「手元に残した記録には FR-8 / FR-9 の上限を掛ける」は FR-8 の主語（C-01）に掛かると読む
- 失わない側: 捨てなければ何も失われない（ウィンドウは 1 日 1.3〜2.7 MB。desktop-collection の「到達できない間の記録を保持する」の注記）
- 反転条件: PC の溜めが PC の容量を圧迫する事例が出たら、または本人が FR-125 の★を「C-02 にも上限を掛ける」と読むと答えたら、C-02 に上限を足す change を立てる
- 要件へ戻すもの: FR-125 の★に「上限は FR-8 の主語（C-01）に掛かる。C-02 の未送信には置かない（NFR-7）」を足す（`deep.md` の「要件へ戻すもの」。record の段で直す）

### D14. 受け付けるようになったら今の送り方のまま送る

- S-01 を更新して A が上がれば、次の契機から版 ≤ A の行が今の規則で送られる。特別な一括送りの口は作らない（C6）
- 止まった状態を抜けた契機で、PC の `Sender` と携帯の `Sender` の「飛ばす」印を捨てる（受け付けるようになった 1 件が、`unknown_field` / `contract_unsupported` で付いた印のせいで先頭で待たされないように）

### D15. 確かめる口は API の合言葉で守る

- `GET /contract` は `Caller::ApiToken` だけを通す。合言葉が無い・一致しなければ 401（record-envelope の「すべての API 要求は資格情報を要求する」）。画面のログインと衛星は 403（spec の Scenario）
- 版は私的データではないが、合言葉の無い相手に S-01 の版を明かさない（C7）
- 読み出しの記録の扱いは D23（仮）

### D16. 画面と衛星は対象外

- 画面（V-01）は S-01 が配る（FR-104 の★）ので常に同じ版。衛星の契約は別に作る（`docs/collector-contract.md` の冒頭）。どちらの要求にも見出しの検査（D4）・知らない欄の検査（D2）・記録の版（D1）を掛けない（C8）

### D17. 新しい断りの種別は捨てない側

- `unknown_field`・`contract_unsupported` は携帯の `PERMANENT_ERRORS` に入れない（C9。P4）。PC は断られた 1 件を種別に依らず捨てない（P5）
- 携帯の `SenderTest` の「恒久の 6 種」の試験はそのまま通る（6 種を変えない）

### D18（仮）. 確かめは送る契機ごとに 1 回。答えを覚えない

- 5 分の送る契機の頭で 1 回 `GET /contract` を引き、その契機の送信（続けて送る分を含む）に使う。答えを端末やファイルに覚えない（再起動しても次の契機で確かめ直す）
- 失わない側: 毎回確かめれば、S-01 の入れ替えを最大 1 契機（5 分）で拾う。覚えると古い答えで送り、D4 の 426 にだけ守られる
- 費用: 5 分に 1 回の小さな GET（本文は数十バイト）。携帯の電池は送信と同じ契機に乗るので増えない。読み出しの記録には今の規則のまま 1 回 1 行が残る（携帯と PC で 1 日 576 行。D23（仮））
- 反転条件: 問い合わせの数が S-01 のログや携帯の電池で目に見える量になったら、答えを 1 時間覚えて 426 のときだけ確かめ直す形に変える

### D19（仮）. 外部サービス由来の更新では、契約の版も新しい中身の 1 件の版に動く

- `apply_external_update` で中身を入れ替えるとき、`contract_version` も新しい 1 件の値にし、前の値は前の版の表に写す（`schema_version` と同じ扱い。ST49 D1（仮）と同じ向き）
- 失わない側: 前の値は前の版の表に残るので、どちらの読み方にも後から戻せる
- 費用: 前の版の INSERT と本表の UPDATE の名指しの列に 1 つ足すだけ
- 反転条件: 契約の版を「最初に届いた時の版」として読む要求が出たら、本表を最初の値に戻し、新しい値を前の版から引く

### D20. 移行と、門・錠

- 新しい移行 `migrations/<作成時刻>_contract_version.sql`（と `.down.sql`、`migrations/ORDER` の末尾へ 1 行）。名前の時刻は `202610250000` 以後なので ST48 の規則（前進側に取引の文を持たない・`.down.sql` が `migration_ledger.applied` の行を消す）に従う
- 中身: `core.event` と `core.event_version` に `contract_version integer`（空を許す・`CHECK (contract_version IS NULL OR contract_version >= 1)`）。`core.event_live`（`SELECT *` の作り直し）・`core.event_version_live`（列を名指し）を作り直す
- **埋め戻し（D10）は錠を外して行い、同じ移行の中で張り直す**（review/spec.md R3）: 前の版の錠 `event_version_immutable`（`BEFORE UPDATE`。今の本体は本文の消去でない UPDATE を全部止める。
  `202610231200_event_end_time.sql:186-191`）と、本表の門 `event_requires_version`（`DEFERRABLE INITIALLY DEFERRED`。検査は COMMIT の時に、その時点の本体で走る。`gates.sql:150-153`）は、
  埋め戻しの UPDATE を通さない。手順: `ALTER TABLE core.event_version DISABLE TRIGGER event_version_immutable` と `ALTER TABLE core.event DISABLE TRIGGER event_requires_version` →
  2 つの表の `c01-*` / `c02-*` の行を `UPDATE … SET contract_version = 1` → 2 つの `ENABLE TRIGGER` → 錠と門の本体の置き換え。移行は 1 つのまとまりで当たる（`migration_ledger.rs:448-461`）ので、
  途中で落ちれば外した錠も戻る。張り直したことは `tools/check-immutable.d/` の節がわざと UPDATE を投げて確かめる（Task 4.3）
- 錠: 前の版の錠 `core.reject_version_rewrite` と、収集した記録の門 `core.require_version_or_ledger`（前の版を同じまとまりに積むときだけ通す）に `contract_version` を足す。**移行を当てる時点の本体を写して列を足すだけ**にし、ほかの条件を変えない
  （ST49 も同じ関数に版の列を足す。後から main に入る側が rebase で前の側の本体を写す。ST41 の移行 `202610231200_event_end_time.sql` が関数の本体の md5 を確かめてから置き換える形に倣う）
- 書き出し: `EVENT_COLUMNS` / `EVENT_VERSION_COLUMNS` に `contract_version` を `schema_version` の次に足す（説明は data-durability の Requirement の文言。ST49 と同じ位置に足して衝突の幅を減らす）
- 錠の確かめは新しい `tools/check-immutable.d/<作成時刻>_st50-contract-version.sh`（`202610231200_st41-event-end-time.sh` の頭の形で schema を作り直してから撃つ）

### D21. 封筒の形の写しの形

```json
{"version": 1,
 "s01":     {"IngestRequest": {"id": {"type": "string", "required": true}, …}, "HeartbeatRequest": {…}, "DropReportRequest": {…}, "MediaMissingRequest": {…}},
 "android": {"ingest": {…}, "heartbeat": {…}, "drops": {…}, "media_missing": {…}},
 "windows": {"ingest": {…}, "heartbeat": {…}}}
```

- 型は JSON の型（`string` / `integer` / `number` / `boolean` / `object` / `array` / `null` の組み合わせ）。S-01 の側は `docs/openapi.json` の `properties` と `required` から、収集アプリの側は D7 の試験の直列化から作る
  （収集アプリの「必須」は、任意の欄を空にした 1 件でも出る欄）
- 鍵は名前の順に並べる（差分を読みやすく、作り直しが決まった形になるように）

### D22（仮）. 200 で読めない答えでは送らずに残すが、更新は促さない

- 送らずに残す条件（D11）と、「サーバを更新してください」を出す条件（D9 の「S-01 の更新を待っている」）を分ける。200 で読めない本文（公衆の Wi-Fi の入口の画面が 200 で HTML を返す。
  `IngestRequest.kt:76` が名指す事故）は、送らずに残すが知らせない（review/spec.md R7）
- 失わない側: 行は送らずに残すので、何も失われない。変わるのは知らせを出すかだけ（後から変えれば次の契機から効く）
- 費用: 判定の分岐 1 つ。404 を返す ST50 より前の S-01 の下では今どおり促す
- 反転条件: 200 の読めない答えが続くのに知らせが無く、本人が気付かないまま携帯の上限で記録が捨てられる事例が出たら、「200 の読めない答えが 24 時間続いたら促す」に変える

### D23（仮）. 確かめる口の問い合わせは、今の規則のまま読み出しの記録に残す

- 合言葉が認められた `GET /contract` は、取り込みの口（`access_log.rs` の `is_ingest`。記録・生存信号・破棄の報告）に入れず、今の規則のまま 1 回 1 行を読み出しの記録に残す
  （原本の欠け・原本の受け口も今は残っている）。data-sensitivity の Requirement は変えない
- 失わない側: 残せば「いつ誰が確かめたか」が後から読める。残さないと、その期間の行は後から足せない
- 費用: 携帯と PC の 2 台で 1 日 576 行（5 分ごと × 2）。行は時刻・経路・資格情報の種類・口の名前・結果だけで、中身を持たない（review/spec.md R8）
- 反転条件: 読み出しの記録を人が読むときに、この行が「誰がいつ読んだか」を読みにくくする事例が出たら、合言葉が認められた問い合わせを取り込みと同じく残さない形に変える（data-sensitivity の Requirement を MODIFIED）

## Risks / Trade-offs

- [新しい収集アプリが ST50 より前の S-01 に当たると、版 1 の記録も含めて何も送らない] → FR-125 の逐語（口を持たない S-01 は受け付けないものとして扱う）。入れ替えは **S-01 を先に**する（Migration Plan）。
  止まっている間は「サーバを更新してください」が出て、携帯は 90 日・2 GB を超えた分を捨てる（FR-8 / FR-9。破棄として報告）
- [止めている間、S-01 の稼働状況は途絶と出て、途絶の知らせと更新の知らせが同時に出うる] → 記録が届いていないのは事実なので途絶は正しい。2 つは別の札・別のチャンネルで出す（D5・D6）
- [S-01 を古い版へ戻すと、新しい版で作った行は手元に残り続ける] → 戻した S-01 は自分以下しか受け付けない（D4）。新しい版の行は S-01 を戻し直すまで残り、携帯は上限で捨てる。戻した組み合わせで更新を促す文言は正しい（S-01 が古い）
- [収集アプリを古い版へ戻すと、溜めに残った新しい版の行（ST49 D3（仮）で欄ごと保たれた行）が来る] → 行の版は積んだ時の版のまま（D1）なので、S-01 が受け付ける版なら送られ、知らない欄も無い（D2 は S-01 の知る欄で見る）
- [3 つの版の値を片方だけ上げる] → CI が落とす（D7・D12）
- [意味だけを変えて版を上げ忘れる] → 機械では止まらない。版の履歴の「理由」の行を検査が求めるだけ（深掘り Q7）
- [`GET /contract` の 404 を返す別のもの（逆向きの代理・別のサーバ）につながっている] → 送らずに残す側に倒れる（D11）。更新の知らせが出るので、本人が気付ける

## Migration Plan

1. ST50 の S-01 を先に入れる（`GET /contract` が版 1 を答える。見出しの無い今の収集アプリはそのまま受け付けられる）
2. 収集アプリ 2 本を ST50 の版へ入れ替える。溜めに残っていた版を持たない行は版 1 として送られる
3. 戻すとき: 収集アプリを先に戻し、S-01 を後に戻す（逆にすると、ST50 の収集アプリが ST50 より前の S-01 に当たって止まる。止まっても行は残り、S-01 を戻し直せば入る）。
   移行の `.down.sql` は列と錠の足し分を外し、版の記録の行を消す

この順は `docs/collector-contract.md` の版の節に書く。

## Open Questions

（無し。仮の決定 D13・D18・D19・D22・D23 は反転条件つきで台帳にある）
