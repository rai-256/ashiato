# ST44 深掘りの問いの独立レビュー（deep-review）

対象: `openspec/changes/st44-collector-credential-scope/deep-questions.json`（第 1 回。Q1〜Q3）・`premises.md`。
手順 1〜8 は問いの一覧を読む前にやり、そのあとで一覧と突き合わせた。問いの JSON は触っていない。

## 手順ごとの確かめ

- **手順 1（要件どうしの衝突）**: PERM-10 本文と★ 4 つ（requirements.md:1171-1191）、扉 #13 の関与要件 FR-24 / FR-92（:390）、FR-128（:1072）、
  正典 `data-sensitivity`（:119・:124・:208「API の合言葉での読み書きは変わらない」）・`record-envelope`（:302-305。例外の無い「すべての API 要求」）・
  `subjective-log`:599・`satellite-platform`:274・`collection-coverage`:1485-1496 を読んだ。
  衝突は 2 組。(a) PERM-10★ 2026-10-11（:1185-1186）は、資格情報の種類を「本体の収集アプリ・画面・衛星」に限り、収集アプリの書き手は「本体」・端末は送り手の申告のまま、と既に決めている。
  Q1 案 2 の「本人の合言葉」はこの 3 種の外の 4 つ目になり、Q2 の「どの資格情報で届いたか」は★が受け入れた範囲より細かい → R3・R7。
  (b) 正典は ST44 が書き直す節を、Q1 の既定に挙がった 2 つより多く持っている → R9
- **手順 2（扉の幅）**: 扉 #13「書き手はサーバが資格情報から決める」。資格情報が共有の 1 本だと、「書き手」は種類（本体）までしか決まらない。
  端末ごとにしても、記録の行に資格情報の識別子を持たせる担い手がいなければ、書き手は同じ粒度のまま → R6
- **手順 3（新たに立つ一方通行）**:
  (a) `Caller` に新しい種類を足すと、`contract_version.rs:100` / `:226` の `matches!(caller, Caller::ApiToken)` は黙って偽になる。契約の門（FR-130）が外れ、知らない欄が黙って捨てられる（discarded）→ R1。
  (b) `API_TOKEN` は `core.web_session.secret_tag` の HMAC の鍵（web_session.rs:97-105・migrations/202609290900_access_control.sql:6-7）。収集アプリに鍵が残る → R2。
  (c) 読み出しの記録（追記のみ）の `credential` は CHECK で 4 値（migrations/202610241200_satellite_credentials.sql:75-76）。収集アプリの資格情報の識別子を書く欄が無い → R6。
  (d) 合言葉を替えた後、収集アプリは 401 の間も溜め続けるが、上限を越えると捨てる → R5
- **手順 4（日常に影響する選択）**: 毎日目に入るものは無い。電話を落としたときの締め出し（共有の 1 本なら全部を入れ直す）と、`API_TOKEN` を替えると全部のブラウザのログインが切れること（docs/network.md:43）が、たまに起きる手作業になる。
  前者は Q2 にある。後者は R2 で扱う。案 1 では、本人が確認の手順書の `curl`（tools/verify-prep.sh:140）をログインで打ち直すことになる。これは R3 の費用に含めた
- **手順 5（既存コードとの突き合わせ）**: 全部の口のハンドラが `authorize` を通り、資格情報なしで通るのは `/healthz`（lib.rs:4211）とログイン・ログアウト（web_session.rs:280・:386）だけだった。
  見た口は events / outages_get / feelings_reminder_get / coverage_get / stays_rebuild / export_post（いずれも本体の 6 行目前後で `authorize`）。
  `authorize_quiet` は Bearer を印より先に見る（lib.rs:683-688）。収集アプリが呼ぶ口の一覧（premises.md）は、`LocationService.kt:241-258,937-962`・`OriginalAndroid.kt:79`・`FeelingReminderSource.kt:48`・`sender.rs:120,130,441`・`runtime.rs:815`・`clock.rs:207` と合っていた。
  PERM-10 本文の例外の 3 つと、正典の「すべての API 要求」が食い違っている点は Q1 の既定にある。「動いているが要件を満たしていない」は、この食い違いのほかに見つからなかった
- **手順 6（過去の取りこぼし）**:
  - 「送らずに残すので失わない」を根拠にするなら、残した行の上限を挙げる → 該当（R5）
  - 申し送りの全項目を突き合わせる → `docs/handoff/ST44.md` の C1 2 件と、`docs/handoff/ST29.md` の st28 Q4・st17 R5 は Q1 / Q2 が受けている。
    受けていないのは、ST29 の design.md:254 の「厳しくするなら ST44 が収集アプリの合言葉と一緒に決める」（R8）と、audit 2026-10-25 C3（docs/audit/2026-10-25-consistency.md:51）の「配り方の問いに『1 つの APK を全員に配る』前提を足す」（R7）
  - 書き直せない記録（追記のみの台帳）の形 → 該当（読み出しの記録の `credential`。R6）
  - 外に出る文言 → 該当なし（403 を足す API の説明は、ほかの決定に依らない）
  - 時計 → 該当なし（ST44 は時刻で状態を動かさない。`last_used_at` を持つのは端末ごとの発行で、ST56 の範囲）
  - 初回・大量、欠けた入力、外部の DB → 該当なし（ST44 は記録の形を変えない）
- **手順 7（族の網羅）**: Q1 の族「全部を読める合言葉が外にある経路」に、「全部を読む合言葉（画面の合言葉）を手元で当てられる材料が外にある経路」が無い（`API_TOKEN` は secret_tag の鍵）→ R2。
  「落とした電話のブラウザに残ったログイン」は ST28 Q6（期限なし）で決まっていて、族の「画面のログイン」に含まれる。Q2 には族が無い。
  兄弟の「資格情報の切り替えの間に溜まる分」は R5 で、「Caller の種類を見る判定」は R1 で挙げた
- **手順 8（前提の確かさ）**: premises.md は「外の形式に依る決定なし」と書いている。
  しかし Q1 案 1 の費用（道具が画面のログインで読む）は、`Secure` の cookie を `http://127.0.0.1` へ付けるかどうか、という curl と Playwright の振る舞いに依る → R4

Q3 は B として成り立っている（推奨あり・失うものなし。画面のログインはいまも `/ingest` に「収集した」を書けるので、案 2 で守れる範囲は小さいが、そのことは問いの質を変えない）。指摘は無い。

## R1. 収集アプリの資格情報に新しい `Caller` を足すと、契約の門（FR-130）が黙って外れて新しい欄が捨てられる。この一方通行がどの問いにも無い
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: crates/server/src/contract_version.rs:100（`if !matches!(caller, Caller::ApiToken) { return Ok(()) }`）/ crates/server/src/contract_version.rs:226-229（`strict = matches!(caller, Caller::ApiToken)`。偽なら知る欄の一覧が空になり、知らない欄を見ない）/ crates/server/src/contract_version.rs:59-63（`/contract` は `ApiToken` だけ）/ crates/server/src/lib.rs:1252-1257（`Writer::of` は `_` が無いのでコンパイルで捕まるが、`matches!` は捕まらない）/ docs/requirements.md:1080-1082（FR-130: 知らない欄を黙って捨てると、新しい欄だけが失われる）
- kind: irreversible
- loss: discarded
- 提案: Q1 案 2（収集アプリ用の合言葉を新しく作る）と Q2 の「端末ごと」（ST56 で新しい種類になる）の `irreversible` に、「`Caller::ApiToken` だけを見る契約の門と `/contract` を新しい種類へ移し忘れると、収集アプリの新しい欄が 200 で受け付けられたまま捨てられる」と書く。
  どちらの案でも効く既定として、「`Caller` の種類を見る判定は `matches!` でなく `_` の無い `match` にする（種類が増えたらコンパイルが落ちる）」を Q1 の AI の既定に足し、ST56 への申し送りにも書く
- 処置: escalated —— Q1 の context の AI の既定に「呼び出し元の種類を見る判定は `_` の無い `match`」を足した（どの案でも効く。design D6）。ST56 への申し送りは review/spec.md R10

## R2. Q1 案 1 の「全部を読む合言葉はサーバの外に 1 つも残らない」は誤り。`API_TOKEN` は画面の合言葉の世代の印の HMAC の鍵で、収集アプリに残る
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: crates/server/src/web_session.rs:97-105（`secret_tag(api_token, web_password)`。鍵は `API_TOKEN`。注記「DB の写しだけでは合言葉の候補を確かめられない」）/ migrations/202609290900_access_control.sql:6-10（`core.web_session.secret_tag`）/ docs/network.md:43（`API_TOKEN` を替えると全部の端末のログインが切れる）/ Q1 案 1 の `irreversible`
- kind: irreversible
- loss: exported
- 提案: 案 1 では、送るだけに締めた `API_TOKEN` が、携帯の APK と PC の台本に残り続ける。DB の写し（`web_session` の行）と落とした電話の値がそろうと、画面の合言葉を手元で総当たりでき、全部を読める。
  案 1 の `irreversible` をこの事実に直す。あわせて、AI の既定に「secret_tag の鍵をサーバだけが持つ値に替える（替えた時点で全部のログインが 1 回切れるだけで、記録は失わない）」を足すか、残る道として context に書く。
  案 2 で `API_TOKEN` を替えると、全部のブラウザのログインも切れる。これも案 2 の detail に足す
- 処置: escalated —— Q1 の context に `secret_tag` の鍵の事実を書き、AI の既定に「印の鍵をサーバだけが持つ値に替える」を足した（design D7・D8（仮））。案 1 の irreversible をそれを前提にした文に、案 2 の detail に「全部のブラウザのログインも切れる」を足した

## R3. Q1 は失わない側（案 1）が Story の範囲内で要件どおりなので、A でなく B（仮）にできる。案 2 は要件にない 4 つ目の資格情報を足す
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: docs/requirements.md:1185-1186（PERM-10★: 資格情報は本体の収集アプリ・画面・衛星。「本人の合言葉」という種類は無い）/ crates/server/src/web_session.rs:41-48（`Caller` は 3 種）/ 案 1 の費用を数えた: `grep -c 'AUTH\[@\]' tools/smoke.sh tools/smoke.d/*.sh` が合計 48・`grep -l "BEARER\|Bearer" web/e2e/*.ts` が 8 ファイル・`grep -rln API_TOKEN tools/recording` が 5 ファイル・tools/verify-prep.sh:140・tools/st25-seed-check.sh:16
- kind: conflict
- 提案: 失う側（exported）は案 2 にしか無い。案 1 の費用は道具の書き直しで、範囲を越えず要件も変えない。
  費用が許せるなら、案 1 を既定にした仮（台帳 D<n>）に落とし、問いから外す。高すぎると判断して A のまま残すなら、上の件数を案 1 の detail に書き、案 2 の detail に「PERM-10★ の 3 種に 4 つ目を足す（要件の本文を変える）」と書く
- 処置: rejected: A のまま残す。案 1 の費用（道具の書き直し。`tools/smoke.sh:25` の `AUTH` を使う 30 か所と smoke.d・e2e 8 ファイル・録画 5 ファイル）は R4 の前提で増減し、本人が台本から全部を読む手段を失うことは本人の使い方に効く。案 2 は `docs/requirements.md:1185-1186` の資格情報の 3 種に 4 つ目を足す。件数を案 1 の detail に、「要件の資格情報に 4 つ目を足す」を案 2 の detail と context に書いた

## R4. 案 1 の「道具は画面の合言葉でログインして読む」は、`Secure` の cookie を http の道具が付けるかに依る。premises.md は「外の形式に依る決定なし」としている
- 成果物: openspec/changes/st44-collector-credential-scope/premises.md
- 根拠: crates/server/src/web_session.rs:196（`HttpOnly; Secure; SameSite=Strict`）/ web/playwright.config.ts:31（`http://127.0.0.1`）/ web/e2e/feel.spec.ts:16（「ログインの印は `secure` で、画面の外の要求には付かない」。e2e が Bearer を使っている理由）/ crates/server/src/lib.rs:683-688（Bearer を印より先に見る。道具が両方を付けると、収集アプリの扱いになって読めない）/ 手元で確かめた: curl 8.18.0 は `Secure` の cookie を `http://127.0.0.1` と `http://localhost` に付けた（小さな http サーバで `Set-Cookie: s=abc; Secure` → 次の求めに `cookie=s=abc`）
- kind: premise
- 提案: premises.md に、curl（CI の版を含む）と Playwright の `request`（`page.request` と `request.newContext` の storageState）が印を付けるかを、走らせた結果つきで書く。確かさは走らせたかどうかで付ける。
  e2e の注記と食い違うなら、案 1 の detail の「道具は…ログインして読む形に直す」に費用を足す。読む求めに Bearer を付けない形（書く求めと読む求めで見出しを分ける）にすることも、案 1 の既定に書く
- 処置: fixed: premises.md に表で書いた（curl は確かさ 4、Playwright は注記で 2、見出しで自分で付ければ読めるは 3）。どれも費用だけに効き、loss のある決定は載せていない。案 1 の detail に「印を見出しで付けて読む・読む求めに Bearer を付けない」を書いた

## R5. 合言葉を替える場面（Q1 案 2 の入れ直し・Q2 の共有の 1 本での締め出し）では、401 の間に溜まった分が上限を越えると捨てられる。どちらも書いていない
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt:345-348（401 は取り除かずログだけ。通知は出さない）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/Outbox.kt:160（90 日で先頭から捨てる）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/AgeClock.kt:41（2 GB の上限）/ crates/collector-windows/src/sender.rs:254（401・403 はログの種別だけ）/ LocationService.kt:254・sender.rs:441（途絶の問い合わせも同じ合言葉なので、全部の端末が 401 なら、途絶の通知もどこからも出ない）/ docs/stories/ST56.md:41（設定し直しを促すのは ST56）
- kind: irreversible
- loss: discarded
- 提案: Q1 案 2 の `irreversible` と、Q2 の「共有の 1 本のまま」「ST56 まで共有」の detail に、次を書く。
  「サーバの値を替えてから入れ直すまでの間、携帯は 90 日・2 GB まで溜め、それを越えた分は捨てる。端末は 401 を本人に知らせない。気付く手がかりは画面の格子の途絶だけ」。
  案 2 を選んだ場合の既定（切り替えの間、旧い値でも送るだけは通す、など）を、AI の既定か仮として足す
- 処置: escalated —— Q1 案 2 の irreversible と、Q2 の context（共有の 1 本）・推奨と「共有の 1 本のまま」の irreversible に、入れ直すまでの間に 90 日・2 GB を越えた分を捨てること・端末が知らせないこと・途絶の通知も出ないことを書いた

## R6. Q2 の「端末ごとに発行した後の記録は、どの資格情報で届いたかを持てる」は、選択肢のどれでも担い手がいない。記録の行と読み出しの記録のどちらにも、識別子を書く欄が無い
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: Q2 の `why`・推奨の `irreversible` / docs/stories/ST37.md:4（書き手の列は ST37。layer 3 で、ST56〈layer 5〉より先に形が決まる）/ docs/stories/ST56.md:4（ST56 は FR-128 だけ）/ crates/server/src/web_session.rs:52-57（`credential` は種類の 3 値だけ）/ migrations/202610241200_satellite_credentials.sql:68・75-76（衛星には `satellite_credential_id` があるが、収集アプリの資格情報の識別子の欄は無い）
- kind: irreversible
- loss: uncaptured
- 提案: Q2 の各案に、識別子を書く場所と作る Story を書く。書く場所は 2 つ: 記録の行（`core.event`。凍結されるので、届いた時に書くしかない）と、読み出しの記録（追記のみ）の行。
  作る Story は、書き手の列を作る ST37 への申し送りか、ST56 の範囲に含めるか。どちらも無いと、推奨を選んでも「偽の記録を後から見分ける」は成り立たない（できるのは取り消しだけ）
- 処置: escalated —— Q2 の why に「届いた時に書くしかない」、context に欄がいまは無いこと、各案に欄を作る Story（推奨は ST56、ST37 へ置き場を申し送る）を書いた（design D2）

## R7. Q2 の context が、PERM-10★ の「端末は送り手の申告のまま」と、監査 C3 の「1 つの APK を全員に配る」前提を引いていない
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: docs/requirements.md:1186（「本体の収集アプリの資格情報なら書き手は『本体』…端末は送り手の申告のまま」。2026-10-11 に受け入れた）/ docs/audit/2026-10-25-consistency.md:51（「凍結した Story への申し送り（ST44。配り方の問いに『1 つの APK を全員に配る』前提を足す）」）/ docs/requirements.md:964（FR-104）
- kind: conflict
- 提案: Q2 の context に 2 行足す。
  (1) 要件は、書き手を「本体」までとし、端末を申告のままとすることを既に受け入れている。端末ごと（資格情報ごと）に残す案は、FR-24 / 扉 #13 の粒度を上げる（要件の本文を変える）ので、選んだら要件の★を足す。
  (2) APK は 1 つで全員に配る（FR-104）ので、資格情報は組み立てでなく入れた後に入る（ST56）。ST56 までは開発の組み立て（`-P`）だけが値を持つ
- 処置: escalated —— Q2 の context に PERM-10★ の「端末は申告のまま」（選んだら要件に★）と、FR-104 の 1 つの APK・ST56 までは `-P` の組み立てだけが値を持つことを書いた

## R8. ST29 が ST44 に残した「`ashsat_` で始まらない違う Bearer と有効な印の組は、印で通る」が一覧に無い
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: openspec/changes/archive/2026-10-27-st29-satellite-credentials/design.md:235（「`ashsat_` で始まらない `Bearer` の扱いは変えない（ST44 の範囲）」）/ 同 :254（「厳しくするなら ST44 が収集アプリの合言葉と一緒に決める」）/ crates/server/src/lib.rs:683-688
- kind: conflict
- 提案: 失うものは無い（印は有効なので、読めるのは本人の画面と同じ範囲）。AI の既定として Q1 の context に 1 行書く。いまのまま印で通すか、ST29 の D19 に揃えて 401 にするかを決めて、申し送りを閉じる
- 処置: fixed D10 仮 —— Q1 の context の AI の既定に「衛星の形でない違う Bearer と有効な印の組は、いまのまま印で通す」を足し、design の D10（仮）と反転条件・provisional.jsonl に写した

## R9. Q1 の既定に挙がった正典の書き直しが 2 つだけ。ST44 を名指しして「変えない」と書いた節がほかにもある。案 2 では書き直さない節もある
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: openspec/specs/subjective-log/spec.md:599（「ST29 までは…収集アプリの資格情報でも主観の中身は読める」）/ openspec/specs/satellite-platform/spec.md:274（「API の合言葉で通るいまある口の範囲は変えない（ST44）」）/ openspec/specs/data-sensitivity/spec.md:119・:124（SHALL と導出元の両方）/ openspec/specs/collection-coverage/spec.md:1496（例外に残す、と書いてある。整合している）
- kind: technical
- 提案: Q1 の既定の「正典の書き直し」を、上の 4 か所の一覧に直す（collection-coverage は確かめ済みとして挙げる）。
  案 2 では `API_TOKEN` の振る舞いが変わらないので、data-sensitivity:119 と Scenario「API の合言葉での読み書きは変わらない」は残る。「書き直す」は案 1 のときだけ、と書き分ける
- 処置: fixed proposal.md —— Q1 の context の正典の書き直しを 4 か所に直し、collection-coverage は整合、案 2 では data-sensitivity の 2 つが残ると書き分けた。spec の段で proposal の Capabilities と specs に写した

## R10. Q1 の既定の理由「401 だと収集アプリが『合言葉がずれた』と区別できない」は、PC の収集アプリには当てはまらない（401 と 403 を同じ種別でログに出す）
- 成果物: openspec/changes/st44-collector-credential-scope/deep-questions.json
- 根拠: crates/collector-windows/src/sender.rs:254（`401 | 403 => "send_unauthorized"`）/ collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt:345-361（携帯は 401 を `unauthorized`、403 を `server_403` に分ける）
- kind: premise
- 提案: 403 にする既定は変えない。理由を「サーバの読み出しの記録と携帯のログで分かれる。PC の収集アプリは同じ種別に出す」に直す。PC の側で分けるかは、ST44 の範囲の外として 1 行書く
- 処置: fixed D5 —— 403 の既定の理由を「読み出しの記録と携帯のログで分かれる。PC は同じ種別（分けるのは範囲の外）」に直した
