spec-digest: 71671785758ff8ac6e414c3488e38ec988ca73dcc97e9da246041ffe02e92fb0
- Q8: 写った
- Q9: 写った
答え以外の変更: あり（`repair` の一時の起こし方を、Q8 の選択肢の detail の `postgres --single` でなく `pg_ctl start` と一時の `hba_file`（sspi で上がった利用者だけ）・127.0.0.1・一時の番号にした。それに伴い、DB の Requirement の「どの接続にも合言葉の認証」に `repair` の間だけの一時の入口の例外を 1 文と、Scenario「repair の後に合言葉なしの入口と一時の番号が残らない」を足した）
答え以外の変更の決定: D28

<!-- 以下は機械が読まない。確かめの中身 -->

## 確かめた範囲

基準は HEAD（9356514。答えを写す前の版）。`git diff HEAD -- openspec/changes/st55-server-installer` の design.md・proposal.md・provisional.jsonl・specs/distribution/spec.md・tasks.md を全部読んだ（review/spec.md R22〜R24 の処置を入れた後の版）。openspec validate --strict は valid、distribution の Scenario は 80 本で tasks の Global Constraints（ADDED 80・計 87）と合う。

### Q8（この Story で出口を作る `ashiato-setup.exe repair`）—— detail の項目ごと
- 止まったときの案内に `repair` を示す: spec 走らせ直しの Requirement の IF 文・Scenario「DB はあるのに秘密が無いと、何も書き換えずに止まる」の THEN・design D10・tasks 6.1
- 管理者の確認を 1 回: spec 新 Requirement の 1 文目・design D27（D22 と同じ）・D22 の口の一覧
- サービスを止め → 3 つの役割の合言葉を作り直し → 新しい秘密を設定に書いてから → サービスを開始: spec 新 Requirement の WHILE 文・design D27 (a)〜(e)
- 記録の行は書き換えない: spec 新 Requirement・D27
- `API_TOKEN` が作り直しになるので繋ぎ直しの手順を示す: spec 新 Requirement の WHEN 文・Scenario「repair の後は前の API の合言葉では送れず、繋ぎ直しの案内が出る」・D27・tasks 11.1
- Windows の CI で「設定を消す → 止まる → repair → 記録の件数が同じでログインできる」: Scenario「設定を退けて止まった後に repair すると、記録を残したままログインできる」・tasks 6.2
- 旧い本文（「出口は作らない」・反転条件）は design D10 から消え、Risks・Open Questions・proposal・tasks の Global Constraints も写っている

### Q9（`logs\db` に DB のサービスの書きを足し、起動の数行がイベントログに出るのは受け入れる）—— detail の項目ごと
- D3 の ACL に `logs\db` への `NT SERVICE\ashiato-db:(OI)(CI)M`: design D3・spec の ACL の Requirement・tasks 3.1
- D17 は D3 に乗ると明記: design D17「権限は D3 に乗る」
- 文言「送った文の字面と秘密はイベントログに出さない（起動の最初の数行と `pg_ctl` の知らせは出る）」: design D17 の見出し・spec のログの Requirement
- Windows の CI で DB の起動・`logs\db` にファイル・イベントログに文と秘密が無い: Scenario「DB のサービスは logs の db に書け、DB のログのファイルができる」（足した）・「送った文と秘密はイベントログに出ない」（改名）・tasks 4.1。旧い名前は残っていない
- R24 の処置（ログの権限の文を「状態の置き場の権限の Requirement に従わせる（logs\db は DB のサービス、logs\server はサーバのサービスが書く）」に言い換え）も Q9 の ACL の帰結で、写しに数える

### 答え以外の変更の分け方
- **答え以外とした（D28（仮）に依る）**: Q8 の detail と context は起こし方を「単独の利用者の形（`postgres --single`）」と名指していた。D28 はそれを替えた。R22 の処置（DB の Requirement の例外 1 文と Scenario 1 本）は、この一時の入口が 127.0.0.1 で待ち受ける形だから要るもので（`--single` なら待ち受けない）、D28 の節にも「spec の DB の Requirement はこの一時の入口を repair の間だけの例外とし…Scenario で見る」と中身として書かれている。よって D28 に依る変更に含める
- **答えの写しとした**: D27 の「止まった状態でなければ rc≠0」・`server.json.repair` を経てから置く段取りと途中で落ちたときの Scenario・(e) で画面の合言葉を決め直す・前の `API_TOKEN` が 401・D21 / D22 と ACL の Requirement の口の一覧に `repair` を足したもの（理由は前の版と同じ: detail が言っていないことを、選んだ側を仕様にするのに埋めたもの）
- **R23 の処置（D27 (b) の本人を置き場の根の ACL から読む・読みを持つ SID が 1 つでなければ止まる・(e) の (9) は本人のプロファイルで判定する・Requirement「repair で前のインストールの本人を変えない」・Scenario「別の管理者が repair しても、本人は変わらない」）は答えの写しとした**: Q8 の detail が言ったことを変えておらず、合意した既存の Requirement（spec の ACL の Requirement「別の管理者の資格情報で上がっても、その管理者を本人にしない」「走らせ直しで、前のインストールの本人を変えない」）を新しい口でも守らせる帰結で、合意した振る舞いを何も変えない。D27 は仮でないが、名指しに入れない（答え以外の変更ではないため）
