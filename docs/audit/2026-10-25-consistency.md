# 整合の確認（2026-10-25・配布と更新の要件を足した改訂）

- 基準: `81b353d429e96cafd26c35e95d5dc2be75f20710`..`597664509c1e72ca010f8dfc98fe5a62fc2a3055`

対象は `docs/requirements.md` の §3.1「#### 配布と更新」（FR-104〜FR-127）・NFR-25 / NFR-26・§5「配布の方針（2026-10-25 決定）」・範囲外の ★ 補足・EXT-O〜EXT-R・扉 32〜35・採番の記録。
文書とコードは head（`59766450`）の内容で読んだ。

機械の入力:
- `req_impact.py --base 81b353d4`: 改訂 26 件（FR-104〜FR-127・NFR-25・NFR-26）・扉 4 項（32〜35）。すべて新規で、要件の外で名指ししている箇所は 0 件
- `check_chain.py`: 未回収 26 件（FR-104〜FR-127・NFR-25・NFR-26。/stories の前）・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった」（この監査が受けるもの）

名指しが 0 件なので、以下はすべて ID を名指ししない意味の衝突として読んだもの。

走っている change（archive 以外）: `ch03-test-tmp-cleanup`・`ch04-test-db-growth`・`st23-record-erasure`・`st29-satellite-credentials`・`st41-event-end-time`。印は C16 にまとめた。

## C1. 更新の確認を S-01 が配布元へ問い合わせる（FR-112）が、正典の spec と CI の検査は、サーバが HTTP の送り手の部品を持つことを禁じている
- 重大度: 高
- 文書: docs/requirements.md:975
- 要件: FR-112, FR-116, NFR-15
- 根拠: FR-112「利用者のチャンネルに新しい版があるかを、**S-01**・C-01・C-02 のそれぞれで 1 日 1 回、配布元に確かめる」、FR-116（:983）「記録は 1 行も外へ出ない（NFR-15 は記録の写しの話で、この問い合わせは当たらない）」。
  一方、正典 `openspec/specs/data-sensitivity/spec.md:411`「THE SYSTEM SHALL サーバに、外部の宛先へ記録を送る部品（**HTTP の送り手**・メールの送り手・外部の保管庫の手）を持たせない」と Scenario（:419-422）「サーバを組み立てる部品の一覧を、外部の宛先へ送る部品の一覧（**HTTP クライアント**…）と突き合わせる → 一致する部品は 0 件」。
  検査は `tools/check-offsite.sh:14` `exact=(reqwest ureq isahc surf attohttpc curl lettre object_store opendal)` で、CI が走らせる（`.github/workflows/ci.yml:79-80`）。
  FR-116 は NFR-15 の解釈として当たらないと書くが、正典は「記録を送るか」でなく「部品を持つか」で固定しているので、S-01 に HTTP クライアントを足した時点で CI が落ちる。spec の導出元（:416）は「暗号化されたバックアップの送り手を足すときは、この Requirement を MODIFIED にしてその部品だけを開ける」とだけ書き、更新の確認の経路を持たない
- 直すところ: 新しい Story（data-sensitivity の当該 Requirement を MODIFIED にする前提を Story に書く）/ requirements（FR-112 か FR-116 に、正典の「部品を持たない」をどう開けるかを書く）
- 機械: 捕まえられた（`tools/check-offsite.sh` が CI で落とす）。ただし落ちるのは Story の実装の時で、上流の計画の時点では見えない
- 行き先: requirements（FR-112 を「S-01 の更新係（D-01 に繋がらない別のプロセス）」に直した。正典の「サーバに HTTP の送り手を持たせない」は開けない）

## C2. S-01 をコンテナの実行環境の上で配る（§5・FR-108）が、正典は S-01 の待ち受けと DB の接続先を loopback に限り、コンテナの中の loopback は外から届かない
- 重大度: 高
- 文書: docs/requirements.md:1444
- 要件: FR-108, FR-104, PERM-7
- 根拠: §5「**S-01 はまずコンテナの実行環境（既定は Docker Desktop）の上で配る**（FR-108）」。
  正典 `openspec/specs/data-sensitivity/spec.md:12-13`「THE SYSTEM SHALL サーバと画面を loopback のアドレスでだけ待ち受けさせる。WHEN … loopback でないアドレスが与えられる THE SYSTEM SHALL 起動を拒否し」、:424-427「DB の接続先を loopback でない宛先にしてサーバ、または移行を起動する → どちらも DB に接続せずに終わり」。
  コードは `crates/server/src/net_guard.rs:31` `DEFAULT_BIND: &str = "127.0.0.1:18787"`、:57-72 `db_host_is_local`（loopback・`localhost`・unix socket だけ真）、`crates/server/src/lib.rs:258-260` で移行も拒む。
  いまの S-01 はコンテナでなく手元で動く Linux のバイナリで（`tools/stack.sh:60-63` が `cargo build --release` して直に起動）、コンテナにあるのは DB だけ（`docker-compose.yml` は `db` の 1 サービス）。Dockerfile はリポジトリに無い（`git ls-files`）。
  S-01 をコンテナに入れると、コンテナの中の 127.0.0.1 で待ち受ける口は port の公開で届かず、別のコンテナの DB はホスト名（loopback でない）になるので、正典の守りを外すか、待ち受けの形（ホストの網を共有する等）を決める必要がある。§5 も FR-108 もこれに触れていない
- 直すところ: requirements（§5 / FR-108 に「何をコンテナに入れるか」と、loopback の守りを保つことを書く）/ 新しい Story
- 機械: 捕まえられた（`net_guard` が起動を拒む。`tools/check-exposure.sh`）。ただし実装の時まで見えない
- 行き先: requirements（FR-108・§5 に「コンテナに入れるのは DB だけ・S-01 は Windows で直に動かす」を書いた。Windows 向けの組み立ては [NEEDS CLARIFICATION]）

## C3. 配布元に置いた 1 つの APK を全員が入れる（FR-104 / FR-114）が、いまの C-01 は接続先と資格情報を組み立ての時に APK へ焼き込み、実行時に設定する口が無い。要件にも設定の口が無い
- 重大度: 高
- 文書: docs/requirements.md:953
- 要件: FR-104, FR-114, FR-111, PERM-10
- 根拠: FR-104「S-01・C-01・C-02 を、同じ版番号を付けた 1 つのリリースとして配布元に置く」、FR-114「新しい版を落とし、入れる操作（Android のインストール画面…）を開く」。
  `collector-android/app/build.gradle.kts:28-39` が `BASE_URL` / `API_TOKEN` / `USER_ID` / `WEB_URL` を `buildConfigField` で焼き込み、`collector-android/app/src/main/kotlin/dev/ashiato/collector/Config.kt:5-6`「`~/.gradle/gradle.properties` か `-P` から `BuildConfig` に入る」、:18-20 実行時は `BuildConfig` だけを読む（差し替えは試験だけ）。
  `docs/network.md:28`「`ashiato.baseUrl` を … にして APK を入れ直す」、:35「`.env` の `API_TOKEN` を変えて、APK と C-02 を入れ直す」、`docs/handoff/ST29.md:4`「収集アプリの APK（BuildConfig に埋め込み）」。
  配布元の APK は利用者ごとの接続先・合言葉・利用者識別子を持てないので、配った APK は `Config.isComplete` が偽のまま送信を始めない。改訂は画面の合言葉（FR-111）と DB の合言葉（FR-110）の設定は書いたが、収集アプリの接続先と資格情報の設定（入れた後に端末で入れる・S-01 と組にする）を書いていない。
  C-02 も同じ形で、接続先と合言葉は環境変数（`crates/collector-windows/src/config.rs:62-66` `ASHIATO_BASE_URL` / `ASHIATO_API_TOKEN` / `ASHIATO_STATE_DIR`）を自動起動の `.cmd` に平文で書いて渡す（`crates/collector-windows/src/autostart.rs:20-34`）。Windows のインストーラでの設定の口が要件に無い。
  ST44 の完了の判定「収集アプリの資格情報では、送ることだけができて記録は読めない（**配り方は深掘りで決める**。handoff/ST29.md の Q4）」がこの配り方の宛先になりうる
- 直すところ: requirements（収集アプリの接続先と資格情報を入れた後に設定する要件）/ 凍結した Story への申し送り（ST44。配り方の問いに「1 つの APK を全員に配る」前提を足す）
- 機械: できない
- 行き先: requirements（FR-128 を足した。設定のしかたと資格情報の配り方は ST29 / ST44 の深掘りへ）

## C4. 古い S-01 が新しい D-01 を書き換えるのを止めるのは「起動せず」（FR-117）だけで、起動の前に走る `ashiato-server migrate` が古い版の門を当て直す
- 重大度: 高
- 文書: docs/requirements.md:984
- 要件: FR-117, FR-118, FR-105, FR-30
- 根拠: FR-117「IF S-01 が、自分の知る版より新しい移行の当たった D-01 に繋がる THEN THE SYSTEM SHALL **起動せず**、どちらの版かを示す。ベータ版から正式版へ移った・古い版に戻したときに、新しい DB を古い S-01 が書き換えるのを止める」。
  起動の台本は移行を先に当ててからサーバを起こす（`tools/stack.sh:85` `"$server" migrate` → :89 サーバ）。`crates/server/src/lib.rs:186-197` `migrate()` は `MIGRATIONS` の全版を毎回当て直し、最後に `GRANTS` を当てる。
  収集した記録の門の関数 `core.reject_collected_rewrite` は 3 つの版で `CREATE OR REPLACE` される（`migrations/202609082001_immutable_collected.sql`・`202609100000_immutable_origin.sql`・`202609120944_gates.sql`）。
  古い版の `migrate` が新しい D-01 に当たると、古い版が知る最後の定義で門が上書きされ、その後に足された門（新しい版の `CREATE OR REPLACE`）が古い形に戻る。FR-117 の拒否はその後のサーバの起動で初めて効く。
  さらに、FR-117 / FR-118 を持たない版（いまの版を含む）を配ってしまうと、その版へ戻したときには拒否そのものが無い。要件は「FR-117 / FR-118 を最初に配る版に入れる」ことも書いていない（§3.1 の冒頭「配る相手の前に、本人の環境をこの経路に切り替える」の順序だけ）
- 直すところ: requirements（FR-117 を移行の当て直しにも効かせる。FR-117 / FR-118 を最初に配る版の条件にする）
- 機械: 安く捕まえられる（古い `MIGRATIONS` の前方部分を新しい DB に当てたら拒まれる、を `cargo test` の試験で）
- 行き先: requirements（FR-129 を足した。FR-117 / FR-118 / FR-129 を最初に配る版に入れることを節の冒頭に書いた）

## C5. 送信契約の版（FR-124 / FR-125）が、FR-26 の「スキーマ版」と同じ欄に重ねられ、欄を足しても 1 のまま上げてこなかった。いまの S-01 は版を確かめず知らない欄を黙って捨てるので、FR-125 の「受け付けない」が起きず、新しい欄が失われる
- 重大度: 高
- 文書: docs/requirements.md:1000
- 要件: FR-124, FR-125, FR-26, FR-122, FR-10
- 根拠: FR-124「正式版として出した送信契約の版（`schema_version`。`docs/collector-contract.md`）の記録を、すべて受け付け続ける。…契約の版はめったに上がらない（2026-10-25 時点で 1）」、FR-125「IF C-01 / C-02 の送信契約の版を S-01 が受け付けない THEN … 記録を送らずに手元に残す。収集アプリは送る前に、S-01 が受け付ける契約の版を確かめる」。
  FR-26（:388）「すべての記録に、それが**書かれた時点のスキーマ版**を持たせる」、`docs/collector-contract.md:34` `schema_version | integer | FR-26`。契約に欄を足しても版は上げていない: `docs/collector-contract.md:268`「（位置の記録に足した 4 欄）`schema_version` は 1 のまま」。走っている `st41-event-end-time` も封筒に終わりの時刻の欄を足すが、`schema_version` に触れていない。
  サーバの側は `crates/server/src/ingest.rs:15-26` `IngestRequest` に `#[serde(deny_unknown_fields)]` が無く（同じファイルの他の型 `lib.rs:1833` 等には有る）、`schema_version` は値を確かめずにそのまま格納する（`lib.rs:1377` `.bind(req.schema_version)`）。
  このため、FR-122 の版の欄や終わりの時刻を持つ新しい収集アプリが古い S-01 に送ると、断られずに 200 で受け付けられ、新しい欄だけが黙って捨てられる（uncaptured）。FR-125 の「受け付ける契約の版を確かめる」口もいまの S-01 には無く、その口が無い S-01 に当たったときの収集アプリの振る舞いが要件に無い。
  また、S-01 が版を断る形を 400 にすると、収集アプリは FR-10（:201）「恒久的に断られた記録（400）は未送信から取り除いて捨て」と契約の状態符号（`docs/collector-contract.md:122`）に従って捨てる。FR-125 は断りの形を決めていない
- 直すところ: requirements（契約の版を FR-26 のスキーマ版と分けるか、欄を足したら上げる規則を書く。版を問う口の無い S-01 と、断りの状態符号の扱いを書く）/ docs/collector-contract.md
- 機械: 安く捕まえられる（`tools/check-openapi.sh` の差分で封筒の欄が増えたのに版が上がっていなければ落とす）
- 行き先: requirements（FR-124 で契約の版をスキーマ版と分け、FR-130 で欄を足したら上げる規則と機械の検査を足した。FR-125 に捨てない応答と、口を持たない S-01 の扱いを書いた）

## C6. FR-125 は手元に残した記録を「FR-10 の未送信と同じに扱う（消さない）」と書くが、未送信は FR-8 / FR-9 で 90 日・2 GB を超えると捨てられる
- 重大度: 中
- 文書: docs/requirements.md:1004
- 要件: FR-125, FR-8, FR-9, FR-10, FR-126
- 根拠: FR-125「手元に残した記録は FR-10 の未送信と同じに扱う（**消さない**）」。
  FR-8（:181-186）「端末に積んでから 90 日かつ 2 GB を上限として保持する。…**上限は送れない理由を問わずにかける**」、FR-9（:187-188）「IF 端末内の保持が 90 日または 2 GB を超える THEN … 最も古い記録から破棄し」。
  「未送信と同じに扱う」なら 90 日で捨て、「消さない」なら FR-8 の上限の例外になって端末の空きを食う。S-01 の更新は手動（FR-114）なので、更新されないまま 90 日を越える形は起きうる
- 直すところ: requirements（どちらかに決め、FR-8 の★と揃える）
- 機械: できない
- 行き先: requirements（FR-125 に FR-8 / FR-9 の上限を掛けると書いた。「消さない」を外した）

## C7. FR-122 は「送った収集アプリの版」を持たせるが、扉 32 は「書いた」部品の版で、未送信を更新後に送ると古い版で作った記録に新しい版が付く。FR-121 の版の変化の検知も行き来する
- 重大度: 中
- 文書: docs/requirements.md:994
- 要件: FR-122, FR-121, FR-10
- 根拠: FR-122「C-01 / C-02 が送る記録 1 件ごとに、**送った**収集アプリの版を持たせる。…配った後に『この版の収集だけ取り違えていた』と分かったとき、版が無いとどの記録がそれかを後から知れない」。
  扉 32（:1672-1674）「記録ごとに、**書いた**本体の部品の版を持つか … C-01 / C-02 の記録は収集アプリの版」。
  未送信は端末に最長 90 日残り（FR-8）、更新の後に送られる。送った時の版を付けると、取り違えた版が作った記録に直した版が付き、FR-122 の目的が成り立たない。
  作った時の版を付けると、FR-121（:993）「C-01 / C-02 の版の変化は、次に届いた記録の版（FR-122）が前と違うことで S-01 が知る」は、更新の後に溜まった古い版の記録が届くたびに版が戻ったように見え、追記のみの更新の履歴に偽の変化が積まれる
- 直すところ: requirements（FR-122 を「記録を作った時の版」に直し、FR-121 の検知を記録の版でなく別の申告にする）
- 機械: できない
- 行き先: requirements（FR-122 を「作った時の版」に、FR-121 の検知を生存信号の申告にした。扉 32 も直した）

## C8. 扉 32 / FR-122 / FR-123 は C-01 / C-02・C-03・派生の記録だけに版を持たせ、画面（V-01）から本人が書く記録はどちらにも入らない
- 重大度: 中
- 文書: docs/requirements.md:1673
- 要件: FR-122, FR-123, FR-25
- 根拠: 扉 32「持つ —— C-01 / C-02 の記録は収集アプリの版、C-03 と派生の記録は S-01 の版。衛星の記録は対象にしない」「持たなかった期間の記録は、どの版で書かれたかを後から知れない」。
  FR-25 の 3 分類のうち「本人が書いた」記録（主観・個人属性の主張・場所・人物。V-01 から S-01 が書く）は、収集アプリでも C-03 でも派生でもなく、衛星でもない。対象外と決めた記述も無い。
  扉の失われ方（後から知れない）はこの記録にもそのまま当たる
- 直すところ: requirements（扉 32 と FR-123 に「本人が書いた」記録を入れるか、外す理由を書く）
- 機械: できない
- 行き先: requirements（FR-123 と扉 32 に V-01 から本人が書く記録を足した）

## C9. FR-109 が拠点の外の Windows のフォルダに置くのは D-01 と D-02 だけで、S-01 が手元に書くほかの状態（移行の前の退避・書き出し・拠点内のバックアップ・DB の合言葉）は、コンテナの入れ替えで消えうる。FR-119 の戻し先もその 1 つ
- 重大度: 高
- 文書: docs/requirements.md:969
- 要件: FR-109, FR-119, FR-110, FR-70, NFR-16
- 根拠: FR-109「D-01 と D-02 の実体を、コンテナの実行環境の外の Windows のフォルダに置く」。
  FR-119（:988）「移行の前の退避（…`ashiato-server migrate` が当てる前に毎回取る）から D-01 を戻し」。退避の置き場の既定は作業ディレクトリからの相対 `backups/pre-migrate`（`crates/server/src/migrate_backup.rs:6`・:29-30）。
  書き出しの置き場の既定は「サーバを動かしている利用者のローカルのアプリのデータのフォルダの下」（`openspec/specs/data-durability/spec.md:26`、`crates/server/src/export.rs:142`）。FR-70 の拠点内 30 日のバックアップ、FR-110 の DB の合言葉の置き場も FR-109 の対象に無い。
  S-01 をコンテナで動かす（§5）と、これらの既定はコンテナの中を指し、更新でコンテナを入れ替えると消える。移行の前の退避が消えると、FR-119 が戻す元が更新の最中に無くなる
- 直すところ: requirements（FR-109 の対象を「S-01 が持ち続ける状態のすべて」に広げるか、列挙する）
- 機械: できない
- 行き先: requirements（FR-109 の対象を S-01 が持ち続ける状態のすべてに広げた）

## C10. 扉 34 は署名鍵を「拠点外のバックアップ」に置くと決めたが、拠点外のバックアップ（FR-66〜FR-70・ST30・ST31）は D-01 と D-02 だけを運び、製造準備の鍵の保管の欄も扉 24 だけを持つ
- 重大度: 中
- 文書: docs/requirements.md:1682
- 要件: FR-66, FR-67, FR-68, FR-70, 扉 24
- 根拠: 扉 34「署名鍵は暗号化して手元の USB と**拠点外のバックアップ**の 2 か所、鍵の合言葉は紙 1 部」「鍵を失うと、それまでに配った APK を上書き更新できない」。
  FR-66 / FR-67（:935-936）「D-01 と D-02 の増分 / 完全バックアップを取得する」、FR-68「暗号化した写しを拠点外の保管先へ送る」。ST30 の完了の判定・ST31 の完了の判定も D-01 / D-02 の写しだけ（`docs/stories/stories.json` ST30 / ST31）。
  署名鍵は開発者の手元の鍵で、本システムのバックアップ（利用者の D-01 / D-02）の範囲に入る経路が無い。
  `docs/production-prep.md:72` A-3「鍵・資格情報の保管 | Windows の資格情報ストアに置き、復旧用の写しを紙で 1 部物理保管する（扉 #24）」は扉 34 の鍵を持たない
- 直すところ: requirements（扉 34 の「拠点外のバックアップ」が何を指すかを書く）/ docs/production-prep.md（A-3 に署名鍵の行）
- 機械: できない
- 行き先: docs（`docs/production-prep.md` A-3 に署名鍵の行を足し、扉 34 に「拠点外のバックアップ」が何を指すかを書いた）

## C11. FR-113 / FR-115 / FR-126 は C-02 の「画面」と「設定」を前提にするが、C-02 には画面が無く、設定は環境変数だけ
- 重大度: 中
- 文書: docs/requirements.md:977
- 要件: FR-113, FR-115, FR-126
- 根拠: FR-113「V-01 と、その部品（C-01 / C-02）の**画面**に『更新あり』を出す」、FR-115「部品ごとに**設定で**無効にできる」、FR-126「収集アプリの**画面**に『サーバを更新してください』と出す」。
  C-02 の表に出る部分はトーストの通知だけ（`crates/collector-windows/src/toast.rs:2`「途切れの通知を Windows のトーストとして出す」）で、窓・通知領域のアイコンを作るコードは無い（`crates/collector-windows/src` を `tray|Shell_NotifyIcon|CreateWindowEx` で探して 0 件）。設定は環境変数（`config.rs:62-66`）
- 直すところ: requirements（C-02 では通知で出す・設定は何で持つかを書く）/ 新しい Story
- 機械: できない
- 行き先: requirements（FR-113 / FR-126 を C-02 は通知で出すに、FR-115 に設定の置き場を書いた）

## C12. FR-104 のリリースは S-01・C-01・C-02 だけで V-01 を持たず、V-01 はいま S-01 と別のプロセス（`vite preview`）で配っている
- 重大度: 中
- 文書: docs/requirements.md:953
- 要件: FR-104, FR-113, FR-121
- 根拠: FR-104「S-01・C-01・C-02 を、同じ版番号を付けた 1 つのリリースとして配布元に置く」、FR-121「S-01・C-01・C-02 のいずれかの版が変わる」、FR-113「V-01 … に『更新あり』を出す」。
  `tools/stack.sh:116` `(cd web && npx vite preview --host 127.0.0.1 --port "$WEB_PORT" …)`、`docs/network.md:7`「画面と API の 2 本。サーバは `BIND`…、画面は `WEB_PORT`（既定 `5180`）」。
  画面は S-01 のバイナリに入っておらず、配るには Node の実行環境か、S-01 が画面を配る形への変更が要る。どちらも要件に無く、V-01 の版がどの部品の版に従うかも書いていない
- 直すところ: requirements（FR-104 に V-01 を入れるか、「V-01 は S-01 に含む」と書く）
- 機械: できない
- 行き先: requirements（FR-104 に「S-01（V-01 を含む）」と、V-01 を S-01 が配ることを書いた）

## C13. 開発版を正式版と並べて入れられる（FR-106）のは C-01 だけで、C-02 は同じ PC で開発版と正式版が自動起動の項目と通知の差出人を取り合う
- 重大度: 低
- 文書: docs/requirements.md:960
- 要件: FR-106, FR-105
- 根拠: FR-106「C-01 を正式版と別のアプリ識別子で配る。正式版と並べて入れられ、開発版が正式版の端末内の記録を上書きしない」。
  C-02 は自動起動の項目の名前が固定（`crates/collector-windows/src/autostart.rs:16` `ENTRY_NAME: &str = "ashiato-collector.cmd"`）、通知の差出人も固定（`toast.rs:16` `AUMID: &str = "dev.ashiato.collector"`）。C-02 の開発版の扱い（並べるか・入れないか）は要件に無い
- 直すところ: requirements
- 機械: できない
- 行き先: requirements（FR-106 に C-02 の開発版の別の識別子を足した）

## C14. FR-104 は「事業者の名前を要件に縛らない —— PERM-7 と同じ理由」と書くが、FR-108 は要件の本文に Docker Desktop を書き、実装上の既定の置き場（製造準備）に配布元と実行環境の既定が無い
- 重大度: 低
- 文書: docs/requirements.md:966
- 要件: FR-104, FR-108, PERM-7
- 根拠: FR-104「配布元の既定は GitHub Releases（実装上の既定。事業者の名前を要件に縛らない —— PERM-7 と同じ理由）」。FR-108「実行環境の既定は Docker Desktop（EXT-O。個人の利用は無料）」。
  PERM-7 の★（:1085-1088）「既定の手段は引き続き Tailscale とし、実装上の既定として `docs/production-prep.md` に置く」。`docs/production-prep.md` に GitHub Releases・Docker Desktop の記述は無い（`grep -n -i 'docker desktop\|github releases'` で 0 件）
- 直すところ: docs/production-prep.md（実装上の既定として配布元と実行環境を置く）
- 機械: できない
- 行き先: docs（`docs/production-prep.md` A-2 に配布元と実行環境の既定を置き、FR-104 / FR-108 の本文から事業者の名前を外した）

## C15. 稼働記録の扉 14 は「データが無い」の理由を列挙するが、改訂が足した「更新で止めた」（FR-121）が入っていない
- 重大度: 低
- 文書: docs/requirements.md:1571
- 要件: FR-121, 扉 14, 扉 33
- 根拠: 扉 14「動きが無かったのか / 収集が壊れていたのか / 自分で止めたのか / バッファから破棄されたのか / そのソースが退役したのかを、後から区別する唯一の手段」。
  FR-121 の★（:992）「更新で止めた時間は稼働状況の格子に途絶として出る。理由が無いと『更新で止めた』のか『壊れていた』のかを後から区別できない」。扉 33 は別に立てたが、扉 14 の列挙と関与要件に FR-121 が無く、「唯一の手段」の言い方とも合わない
- 直すところ: requirements（扉 14 に補足を足す）
- 機械: できない
- 行き先: requirements（扉 14 に補足と関与要件 FR-121 を足した）

## C16. 走っている change への印（差し戻しではない）
- 重大度: 低
- 文書: openspec/changes/st41-event-end-time/proposal.md:15
- 要件: FR-124, FR-125, FR-118
- 根拠: `st41-event-end-time` は封筒に終わりの時刻の欄を足すが、`schema_version` に触れていない（`proposal.md` / `design.md` / `tasks.md` を `schema_version` で探して 0 件）—— C5 の「欄を足しても版を上げない」の新しい 1 件。配る前なので、いまの時点で失われるものは無い（配った後に古い S-01 へ送ると、終わりの時刻が黙って捨てられる形になる）。
  `st29-satellite-credentials/design.md:15`「`migrate()` は起動のたびに全版を当て直す（各版は当て直せる形で書く）」、`st23-record-erasure`・`st41-event-end-time`・`ch04-test-db-growth`（試験の DB のテンプレートに全版を当てる）も同じ前提で移行を書く。FR-118 が当てた版を記録する形に変えても、当て直せる形で書いた版は壊れない。失われるものは無い。
  `ch03-test-tmp-cleanup` は配布と更新に触れない
- 直すところ: leave
- 機械: できない
- 行き先: leave: 配る前なので失われるものは無い。ST41 の欄の足し方は FR-130 の★で「最初の正式版までは版 1 に含める」とした

## K1. PC と端末の時刻サーバの検査が、時刻と関係の無い子プロセスの起動まで止めるので、C-02 から Windows のインストーラを開く（FR-114）と CI が落ちる
- 重大度: 中
- 文書: docs/requirements.md:978
- 要件: FR-114
- 根拠: FR-114「新しい版を落とし、入れる操作（Android のインストール画面・**Windows のインストーラ**・S-01 の更新の手順）を開く」。
  `tools/check-no-time-server.sh:6-10`「**PC の子プロセスは `w32tm` と `wevtutil` の照会だけ**（`Command::new` の相手は … 2 つの形だけを許す）」、:22 の型に `ProcessBuilder|getRuntime\(\)\.exec`、:46 `$0 ~ /Command::new\(/ && $0 !~ /Command::new\((w32tm|wevtutil)_program\(\)\)/` を当たりとする。見る場所は `crates/collector-windows/src` と `collector-android/app/src/main`（:109-111）、CI で走る（`.github/workflows/ci.yml:84-85`）。
  正典の Scenario（`openspec/specs/desktop-collection/spec.md:565-568`）が求めるのは「時刻を問い合わせる通信を起こす経路が 1 つも無い」だけで、子プロセスの全面禁止は検査の側の取り決め
- 直すところ: コード（検査の許可の形にインストーラの起動を足すか、FR-114 の Story が検査を直す前提を持つ）
- 機械: 捕まえられた（同じ検査が落とす）
- 行き先: requirements（FR-114 の★に、その Story が `tools/check-no-time-server.sh` の許可の形を直す前提を書いた）

## K2. FR-109 の★は「名前付きボリューム」が消えうると書くが、いまの DB はボリュームを宣言しておらず、匿名のボリュームに置かれている
- 重大度: 中
- 文書: docs/requirements.md:970
- 要件: FR-109
- 根拠: FR-109 の★「名前付きボリュームは実行環境を入れ直す・消すと一緒に消えることがあり」。
  `docker-compose.yml` の `db` は `image: postgres:17-alpine` で `volumes:` を持たない（ファイル全体に `volumes` の語が無い）。データの置き場はイメージが宣言する匿名のボリュームになる。
  FR-109 は配布の後の形を決める要件だが、★の前提（いまは名前付き）が実態と違うので、移し替えの Story がいまの本人の D-01 の場所を読み違える
- 直すところ: requirements（★の「名前付きボリューム」を実態に合わせる）
- 機械: 安く捕まえられる（`docker-compose.yml` の `db` に `volumes` が無ければ落とす検査）
- 行き先: requirements（FR-109 の★を「匿名のボリューム」に直した。動いている DB は worktree と試験のものだけで、本番の D-01 はこの compose に無いことを 2026-10-25 に確かめた）

## K3. FR-104 は 3 つの部品の版をそろえるが、版はいま 3 か所に別々に手で書かれている
- 重大度: 低
- 文書: docs/requirements.md:953
- 要件: FR-104, NFR-25
- 根拠: FR-104「S-01・C-01・C-02 を、同じ版番号を付けた 1 つのリリースとして」。
  `Cargo.toml:5-6` `[workspace.package] version = "0.1.0"`（S-01 と C-02 が引く）、`collector-android/app/build.gradle.kts:26-27` `versionCode = 1` / `versionName = "0.1.0"`、`web/package.json:4` `"version": "0.1.0"`。
  `docs/production-prep.md:47`「バージョンの単一情報源 | **build 設定から機械取得する。手書きの定数を作らない**」。値はいまそろっているが、3 か所をそろえる仕組みは無い
- 直すところ: コード（版の単一の出所を Story で作る）
- 機械: 安く捕まえられる（3 か所の版を突き合わせる検査）
- 行き先: stories（FR-104 の★に版の出所を 1 つにすることを書いた。配布の Story の範囲に入れる）

## K4. 改訂後の FR-127 / 扉 35 が足す「追加の許可」は、README・CONTRIBUTING・ソースの SPDX の「AGPL-3.0-only」と、依存の検査の許可一覧に入っていない
- 重大度: 低
- 文書: CONTRIBUTING.md:5
- 要件: FR-127, NFR-26, 扉 35
- 根拠: §5「本体の AGPL-3.0 に GPLv3 第 7 条の追加の許可を書き足す」、FR-127「本体の許諾（AGPL-3.0 と追加の許可。§5）…を同梱する」。
  `CONTRIBUTING.md:5`「本体は **AGPL-3.0-only** です（`LICENSE`）」、`README.md:45`「AGPL-3.0-only（`LICENSE`）」、ソースの先頭 `// SPDX-License-Identifier: AGPL-3.0-only`（例 `crates/server/src/ingest.rs:1`）。`LICENSE` は FSF の全文だけで追加の許可の節が無い（:1-2 以降、第 7 条に基づく本プロジェクト固有の文が無い）。
  いまは許可をまだ書き足していないので文書は実態どおりだが、許可を足す Story がこれらを一緒に直す前提を持たないと、許可の範囲が配布物の表示と食い違う
- 直すところ: 新しい Story（FR-127 の Story の範囲に README・CONTRIBUTING・SPDX・`tools/check-licenses.sh` の許可一覧を入れる）
- 機械: 安く捕まえられる（`LICENSE` に許可の節があるのに SPDX が `WITH` を持たなければ落とす）
- 行き先: stories（FR-127 の★に、追加の許可の Story が直すファイルを書いた）

## 未確認

- **FR-109 の実現性**: Docker Desktop（WSL2 の裏）で PostgreSQL のデータを Windows のフォルダ（bind mount）に置くと、初期化で権限の変更や fsync が通らないことがあるとされる。この環境で試していない
- **K2 の匿名のボリュームの振る舞い**: `docker compose down` → `up` で新しい空のボリュームに付け替わり、本人の D-01 が見えなくなるか。実データの DB に触るので走らせていない。本人の D-01 がこの compose の DB に入っているかも確かめていない
- **S-01 を Windows 向けに組み立てられるか**（C2 の別の道）。試していない
- **CLA（`docs/CLA.md` §2-2「提供分を AGPL-3.0 以外の条件でもライセンスすること」）が GPLv3 第 7 条の追加の許可を付ける許諾を含むか**。§5 は含むと読む。法的な読みは確かめていない。著作者が 1 人であること（§5）は `git log` の作成者が本人の 2 つの名義だけで、食い違いは無い
- **ベータ版から正式版への移り（FR-105）で Android の `versionCode` が単調に増えるか**（`-beta.N` → 正式版）。番号の付け方が決まっていないので確かめられない
- **EXT-O〜EXT-R の引用**を出典で取り直していない
- 正典の Scenario のテストの印の全件の突き合わせは走らせていない（この改訂は正典の Scenario を変えていない）

## 触らなくてよいと判断したもの（理由つき）

- **PERM-7 / §1.4 のローカルファーストと更新の確認の外向きの通信**: PERM-7 と §1.4 が限るのは「外部からの接続」（入る向き）で、配布元への問い合わせ（出る向き）は当たらない。NFR-15 は★（2026-09-29）で「記録の写しを拠点外に置くこと」に読みが固定され、FR-116 がそれに沿う。衝突するのは正典の部品の禁止（C1）だけ
- **C-01 の通信の設定**: 生成される network security config は平文を loopback にだけ許し（`collector-android/app/build.gradle.kts:111-118`）、HTTPS の宛先は限らないので、配布元への HTTPS の問い合わせは止まらない
- **画面が外部の資源を読まない正典（`data-sensitivity/spec.md:268-288`。`default-src 'self'`）**: FR-112 は確かめる側を S-01・C-01・C-02 にしており、V-01 は S-01 の結果を出すだけなので当たらない
- **扉 24（バックアップの暗号鍵）と扉 34（署名鍵）**: 別の鍵で、保管の決定どうしは食い違わない（範囲の穴は C10）
- **FR-118 の★「いまの `ashiato-server migrate` は全部の移行を毎回当て直し、当てた版を数えていない」**: `crates/server/src/lib.rs:186-197` と `migrate_backup.rs:3` のとおり
- **FR-119 の「`ashiato-server migrate` が当てる前に毎回取る」**: `lib.rs:261-275` のとおり（使い捨ての DB の `MIGRATE_BACKUP=off` を除く）
- **NFR-26 の★「いまの `tools/check-licenses.sh` は Rust と Node だけを見て、Android は対象外（design D15 で『公開前に決める』）」**: `tools/check-licenses.sh:136-142` のとおり。`play-services-location` は `build.gradle.kts:168` にある
- **扉 34 の「いまの C-01 は debug の鍵で入っている」**: `build.gradle.kts` に `signingConfigs` も release の設定も無い。アプリ識別子 `dev.ashiato.collector`（:23）も扉 34 と一致する
- **FR-110 の「固定値を持たない（ST28 / design D19 と同じ）」**: `docker-compose.yml` の `POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?…}` と注記のとおり。`.env` の生成は `tools/env-init.sh` が乱数で行う
- **FR-122 の「いまの記録は…アプリの版を持っていない」**: 封筒（`docs/collector-contract.md:23-40`・`ingest.rs:15-26`）にも収集アプリ 2 つにも版の欄は無い
- **範囲外の★「範囲外のままなのは課金・販路・複数利用者・サポート体制」**: `docs/production-prep.md` の「やらないと決めたもの」と同じ 4 つ
- **FR-89 / FR-95（衛星向けの API の版）と FR-124（送信契約の版）**: 対象が別（衛星と本体の収集アプリ）で、FR-123 の★もそう書いている
