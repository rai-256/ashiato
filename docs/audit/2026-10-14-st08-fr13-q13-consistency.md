# 整合の確認（2026-10-14・FR-13 の ★ 2026-10-14 訂正（ST08 の深掘り第 8 回 Q13。例外を「送った版の手がかりがすべて失われた後」へ広げる））

- 基準: `cf7ff534be910f6ef9663bc78d2cc5fc07548c73`..`113dde5b0121ecdd2fe7238e9f23a173f728d03c`

（確認は `cbe957f0` の木で読んだ。C8 / C9 の行き先 requirements の直し（FR-13 の語を「前に送った版の記録」「失われた（無い・読めない）」に揃え、生存信号に出すものを読めない最大値に絞る）を `113dde5b` で入れたので、基準の head をそこへ進めた）

対象: `docs/requirements.md:232-236` に足した「**この例外は、保存場所が残っていても、前に送った版の手がかり（送った版の最大値と帳面）がすべて読めなくなった後に広げる。
手がかりが読めなかったことは、履歴の取得の状態（FR-78）に種別で出す。**」（★ 2026-10-14 訂正。ST08 の深掘り Q13）。
本人の決定は `openspec/changes/st08-browser-history/deep.md` の第 8 回（`deep-answers-8.txt`）。出所は `review/code.md` の R90 と `docs/audit/2026-10-14-st08-fr13-q12-consistency.md` の K1。
deep.md 第 8 回は「作りの変更（読めない `sent_version.json` を上書きせず退避し、読めなかったことを生存信号の `blockers` に種別で載せる。design D16 の『閉じないもの』を本人の決定（Q12 / Q13）へ書き換える。
spec の識別子の Requirement の SHALL の括弧を広げる）はこの段では書かない（下流の fix で）」とする。下の指摘は、その fix がまだ入っていない head（`cbe957f0`）の木で読んだ。

機械の入力:
- `req_impact.py --base cf7ff534…`: 改訂は FR-13 の 1 件、扉 0。名指しは `docs/collector-contract.md:401`・`docs/handoff/ST25.md:21`・`docs/stories/INDEX.md:31`・`stories.json:197`・
  `docs/ui-direction.md:87`・`openspec/changes/st08-browser-history/`（deep.md・deep-questions*.json・design.md:110,336・proposal.md:22,108-113・spec.md 8 か所・review 3 本）・
  `openspec/changes/st14-outage-alert/`（deep.md:31・spec.md:1074 ほか）・`openspec/specs/browsing-views/spec.md:593`・`openspec/specs/collection-coverage/spec.md:537`・`openspec/specs/desktop-collection/spec.md:41`
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-13」で、この確認を求めている当のもの
- `check_scenarios.py . st08-browser-history`: OK。`openspec validate st08-browser-history --strict`: valid。`review_triage.py . st08-browser-history`: OK
- `grep -n 'Q13'` は spec.md・proposal.md・`docs/collector-contract.md`・`docs/handoff/ST08.md` で 0。design.md は :338・:365 の「Q13 で本人に返している」の 2 か所だけ

走っている change の印: **st08-browser-history（この枝。改訂を起こした当の change。tasks.md は凍結）** —— Q13 は loss: uncaptured の問いで、答えが求める作り（退避と種別）はまだコードに無い（K1）。
凍結した tasks.md に Q13 の Task は無く、deep.md・R90 の処置はどちらも「答えの記録の後の fix で入れる」とする。
st14-outage-alert（FR-13 は「Chrome の履歴は 90 日」で引くだけ。ただし FR-35 は「取得できる状態」の信号だけを生きていると数えるので、C7 の決め方で数えが変わる）、
st17-daily-feeling・st33-data-export・st47-youtube-myactivity-split（FR-13 を名指ししない）。走っている change への差し戻しは書かない。

## C1. ST08 の spec の識別子の Requirement が、例外を「保存場所ごと消えた・入れ直した後」に限ったまま、保存場所が残っている場合も前に送った版より前にしないと言い切る
- 重大度: 中
- 文書: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md:169
- 要件: FR-13
- 根拠: 要件 :232「**この例外は、保存場所が残っていても、前に送った版の手がかり（送った版の最大値と帳面）がすべて読めなくなった後に広げる。**」。
  spec :169-171「THE SYSTEM SHALL その更新時刻を、その訪問について前に送った版より前にしない（…ただし、収集アプリの保存場所（`state_dir`）ごと消えた・入れ直した後は、前に送った版を持たないので除く）」。
  導出元 :176「第 6 回 Q11 … / 第 7 回 Q12（保存場所ごと消えた後は除く）」に第 8 回 Q13 が無く、理由の段落 :186-187 も「保存場所ごと消えた・入れ直した後に…取りこぼしてよい（本人が第 7 回 Q12 で選んだ…）」で止まる。
  保存場所が残ったまま `sent_version.json` と帳面が両方読めないとき、コードは床を持たない（`crates/collector-windows/src/history/collect.rs:270-275`・:646-650）ので、この SHALL はその場合に満たされない。
  archive するとこの spec が `desktop-collection` の正典になり、正典だけが Q13 の場合も守ると言う
- 直すところ: その文書（SHALL の括弧を「送った版の手がかり（送った版の最大値と帳面）がすべて失われた後（保存場所ごと消えた・入れ直した場合を含む）は除く」に広げ、導出元と理由の段落に第 8 回 Q13 を足す）
- 機械: 安く捕まえられる（要件の最新の ★ が名指す深掘りの Q 番号が、その Story の spec の導出元に出てこなければ warn。前回の C1 と同じ型）
- 行き先: fix（ST08 自身の下流の回し直し。spec の SHALL の括弧・導出元・理由の段落に Q13 を写す。このブランチ）

## C2. ST08 の spec の生存信号の Requirement に、送った版の手がかりが読めなかったことを出す SHALL も Scenario も無い
- 重大度: 中
- 文書: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md:484
- 要件: FR-13, FR-78
- 根拠: 要件 :233「**手がかりが読めなかったことは、履歴の取得の状態（FR-78）に種別で出す。**」。spec の「PC 側の収集は想定間隔ごとに生存信号を送る」:484-495 が履歴について挙げるのは
  「1 つでも履歴を読めなかったとき、またはブラウザの履歴が 1 つも見つからないとき」(:489-490) と「送る準備…を終えられなかった取得があったとき」(:491-492) の 2 つで、
  送った版の最大値・帳面が読めなかったことは無い。Scenario（:547-576）にも無いので、`check_scenarios.py` は Q13 の守りのテストの印を求めない
- 直すところ: その文書（SHALL を 1 文と Scenario を 1 本足す —— 例「送った版の最大値が読めなかった取得の後、その区間の生存信号の満たされていないものに読めなかった種別が挙がり、パスを含まない」。取得できない状態にするかは C7 の答えに合わせる）
- 機械: 安く捕まえられる（足した Scenario は `check_scenarios.py` がテストの印を求める）
- 行き先: fix（ST08 自身の下流の回し直し。生存信号の Requirement に SHALL と Scenario を足す。このブランチ）

## C3. design D16 が Q13 を「本人に返している」のまま持ち、:328 は読めない最大値を黙って無いものとして扱うと書く。D12 の blockers の語彙にも種別が無い
- 重大度: 中
- 文書: openspec/changes/st08-browser-history/design.md:328
- 要件: FR-13, FR-78
- 根拠: design :328「書けなければ送る準備の失敗にする…。**読めなければ無いものとして、帳面の前の版と読んだ時刻で決める**」。:337-338「置き場が残っていても `sent_version.json` が読めず帳面も退避されたときに同じく捨てられうる場合（review/code.md R90）は、
  例外に含めるかを深掘り第 8 回 Q13 で本人に返している」、Open Questions :362「深掘り（第 1〜7 回・Q1〜Q12）」・:365「深掘り第 8 回 Q13 で本人に返している」、Context :10「第 2〜7 回の Q4〜Q12」。
  D12 :268-272 の `blockers` は `history-unreadable:…` / `history-none-found` / `history-not-queued` の 3 つ。
  deep.md 第 8 回は回答済みで「読めない `sent_version.json` を上書きせず退避し、読めなかったことを生存信号の `blockers` に種別で載せる」「design D16 の『閉じないもの』を本人の決定（Q12 / Q13）へ書き換える」。
  実装者が design だけを読むと :328 のとおり黙って `None` にし、低い版で上書きする今の作り（K1）を正しいと読む
- 直すところ: その文書（D16 :328 を「読めなければ上書きせず退避し、種別を生存信号に載せる（第 8 回 Q13）」に、「閉じないもの」を Q12 / Q13 の本人の決定に、D12 に種別を足し、Context と Open Questions の範囲を第 8 回 Q13 まで延ばす）
- 機械: 安く捕まえられる（deep.md の回答済みの Q の「効く先」に design D<n> があるのに、design のその Q の言及が「返している」「未回答」の語と並んでいれば warn）
- 行き先: fix（ST08 自身の下流の回し直し。D16 を本人の決定（Q12 / Q13）へ、D12 に種別を足す。このブランチ）

## C4. 送信契約が、履歴の `blockers` の語彙に手がかりが読めなかった種別を持たず、`source_updated_at` の例外も保存場所ごと消えた後に限る
- 重大度: 中
- 文書: docs/collector-contract.md:398
- 要件: FR-13, FR-78, FR-22
- 根拠: 契約 :398-400「ブラウザ履歴のソース（`c02-browser-history`）の生存信号の `blockers` は…**`history-none-found` … / `history-unreadable:<browser>:<profile_dir>` … / `history-not-queued` …**」。
  :427-428「**例外**: 収集アプリの保存場所（`state_dir`）ごと消えた・入れ直した後は前に送った版を持たないので、前に届いた版より前になりうる（…ST08 deep.md 第 7 回 Q12）」。
  要件 :232-233 は保存場所が残っていても手がかりがすべて読めなくなった後を例外に含め、読めなかったことを種別で出す。契約は C-02 の外の読み手（取り込み口・稼働状況の画面・ST14・ST25）も読む文書で、
  語彙が閉じた列挙として書かれているので、新しい種別が来たときの読み方が契約から取れない
- 直すところ: その文書（:398-400 の列挙に手がかりが読めなかった種別を足し、:427 の例外を「送った版の手がかりがすべて失われた後（保存場所ごと消えた・入れ直した場合を含む）。ST08 deep.md 第 8 回 Q13」に広げる）
- 機械: 安く捕まえられる（`heartbeat.rs` の `history_blocker` の定数の文字列が、契約の `c02-browser-history` の段に全部出てくるかを見る）
- 行き先: fix（送信契約の blockers の語彙と例外の範囲を直す。このブランチ）

## C5. ST08 の proposal が第 8 回 Q13 と FR-13 の ★ 2026-10-14 訂正（Q13）を持たない
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/proposal.md:65
- 要件: FR-13
- 根拠: proposal :65-69 は「whole-branch review の R88 から、第 7 回（2026-10-14。1 問…）」の表（Q12）で止まり、:113「**FR-13** を訂正（★ 2026-10-14）—— 上の『取りこぼさない』から、収集アプリの保存場所ごと消えた・入れ直した後を除く（第 7 回 Q12）」に Q13 の広げた範囲が無い。
  deep.md 第 8 回は「回答済み」で、「要件へ戻すもの」に FR-13 の訂正（Q13）を挙げる。PR 本文と issue はこの proposal を読む
- 直すところ: その文書（第 8 回（Q13）の表と、「要件の改訂」に FR-13 の ★ 2026-10-14 訂正（Q13）を足す）
- 機械: 安く捕まえられる（deep.md の「第 n 回の要件へ戻すもの」の出所の Q 番号が proposal に無ければ warn。前回の C4 と同じ型）
- 行き先: fix（ST08 自身の下流の回し直しで proposal も直す。このブランチ）

## C6. 申し送り ST08 の R87 が、spec の SHALL を Q12 の括弧だけで 1 文にまとめるよう指示している
- 重大度: 低
- 文書: docs/handoff/ST08.md:45
- 要件: FR-13
- 根拠: handoff :45「`specs/desktop-collection/spec.md:169-171` と導出の段落を『読んだ時刻を基に、ただし前に送った版より前にしない（収集アプリの保存場所ごと消えた・入れ直した後は除く —— 第 7 回 Q12）』の 1 文にまとめる。例外の括弧は落とさない」。
  merge の後に `fix/` でこの指示どおりに書くと、C1 で広げた括弧が Q12 の範囲へ狭まる（R87 の文は Q13 の前の 2026-10-14 に書かれた）
- 直すところ: その文書（R87 の括弧を「送った版の手がかりがすべて失われた後は除く —— 第 7 回 Q12 / 第 8 回 Q13」に替えるか、「C1 で直した SHALL の括弧のまま」にする）
- 機械: できない（申し送りの指示文と要件の例外の範囲の突き合わせは語では取れない）
- 行き先: fix（C1 で広げた SHALL に合わせて handoff の R87 の文も直す。このブランチ）

## C7. 要件は読めなかったことを「取得の状態に種別で出す」とだけ言い、「取得できない」にするかを決めていない。今の作りでは種別を載せた信号は必ず「取得できない」になり、稼働状況と途絶の数えが変わる
- 重大度: 中
- 文書: docs/requirements.md:233
- 要件: FR-13, FR-78, FR-35, NFR-13
- 根拠: 要件 :233「手がかりが読めなかったことは、履歴の取得の状態（FR-78）に種別で出す。」。同じ FR-13 の Q10 は :219-220「済まなかった取得は『取得できない』とし、理由には種別だけを載せる」と状態まで言う。
  spec :484「取得できない状態のとき、何が満たされていないかを生存信号に載せる」は満たされていないもの（`blockers`）を「取得できない」と組にし、
  コードは `Capability::from_blockers`（`crates/collector-windows/src/heartbeat.rs:93-96`）で `capturable: blockers.is_empty()` —— 種別を 1 つ載せればその信号は `capturable=false` になる
  （取り込み口は `capturable=true` と `blockers` の組を拒まない。拒むのは `!capturable` で `blockers` が空のときだけ。`crates/server/src/heartbeat.rs:59`）。
  `capturable=false` の履歴の信号は、稼働状況で日ごとの `bool_or(capturable)`（`crates/server/src/coverage.rs:640`）を通り、履歴の信号は 24 時間に 1 本なので、その日は記録が届いていても「取得できない」の日になる。
  FR-35 :413「生存信号は『取得できる状態』（FR-78）のものだけを『生きている』と数える」、NFR-13 :1022「取得できる状態の生存信号（FR-78）があった日」も同じ値を読む。
  手がかりが読めなかった後も、履歴そのものは読めて送れている（床が無いだけ）ので、「取得できない」と読ませるかどうかは要件の語からは決まらない
- 直すところ: requirements（「取得できない」にするのか、「取得できる」のまま種別だけを載せるのかを FR-13 に書く。後者なら spec :484 と `from_blockers` の組を崩すことになる）
- 機械: できない（どちらの状態にするかは決めの問題）
- 行き先: fix（「取得できない」にするかは計算し直せば戻る（B）。下流の fix が design に D（仮）と反転条件で書き、spec と `from_blockers` をそれに合わせる。このブランチ）

## C8. 「手がかりが読めなかったこと」が、帳面が読めなかったことも含むのか、すべて読めなくなったときだけなのかが要件で決まらず、deep.md の作り（`sent_version.json` だけ）とずれる
- 重大度: 中
- 文書: docs/requirements.md:233
- 要件: FR-13, FR-78
- 根拠: 要件 :232「前に送った版の手がかり（送った版の最大値と帳面）がすべて読めなくなった後」、:233「手がかりが読めなかったことは…種別で出す」—— :233 の「手がかり」は :232 で最大値と帳面の両方を指す語で、
  「すべて」が付かない。deep.md 第 8 回の作りは「読めない `sent_version.json` を上書きせず退避し、読めなかったことを生存信号の `blockers` に種別で載せる」で、帳面を挙げない。
  コードで帳面が壊れて退避されたことは、件数のログ（`history/collect.rs:325` の `history_ledger_quarantined`）に出るだけで生存信号には出ない（`runtime.rs:171-191` の `HistoryBeat::record` は帳面の退避を受け取らない）。
  要件の語どおりなら帳面の退避も生存信号に出す必要があり、deep.md どおりなら要件の語が広すぎる
- 直すところ: requirements（:233 を「送った版の最大値が読めなかったことは」か「送った版の最大値か帳面が読めなかったことは」か「手がかりがすべて読めなくなったことは」のどれかに絞る）
- 機械: できない（語の範囲の取り違えは語の一致では取れない）
- 行き先: requirements（`113dde5b` で直した。FR-13 は「送った版の最大値のファイルがあるのに読めなかったこと」を出すと絞った —— deep.md 第 8 回の作りどおり）

## C9. 「手がかり」が FR-13 の中で 2 つの別のものを指す。表題と本文で「失われた」と「読めなくなった」が揺れる
- 重大度: 低
- 文書: docs/requirements.md:232
- 要件: FR-13
- 根拠: 要件 :232「前に送った版の手がかり（送った版の最大値と帳面）」。同じ FR-13 の「消えた」事実について、ST08 の文書は「手がかり」を消えた訪問の経路の事実の名に使う ——
  spec :293「消えた訪問の識別子と、経路の手がかりとなる事実」、design :137「消えた訪問の識別子と手がかりの一覧（D10）」・:222、tasks.md :117「手がかり（`age_days` / `foreign` / `table_recreated` / `profile_gone`）」、proposal :27「経路の手がかりを載せ」。
  また要件の本文 :232 は「すべて読めなくなった後」、★ の表題 :234 と deep.md の要件へ戻すものは「すべて失われた後」で、読めない（ファイルはある）と失われた（無い）が同じ条件の名として並ぶ
- 直すところ: requirements（送った版の側を「送った版の記録」など別の語にし、条件の名を 1 つに揃える）
- 機械: 安く捕まえられる（要件の新しい語が、同じ要件を導出元に持つ spec で別の文脈の語として既にあれば warn。語の一覧が要る）
- 行き先: requirements（`113dde5b` で直した。送った版の側を「前に送った版の記録」にし、条件の名を「失われた（無い・読めない）」に揃えた）

## K1. 要件は手がかりが読めなかったことを取得の状態に出すと言うが、コードは読めない `sent_version.json` を数えず、前に送った版より低い値で上書きする
- 重大度: 高
- 文書: docs/requirements.md:233
- 要件: FR-13, FR-78
- 根拠: 要件 :233「手がかりが読めなかったことは、履歴の取得の状態（FR-78）に種別で出す。」。コード `crates/collector-windows/src/history/collect.rs:270-275` の `read_sent_version` は
  `std::fs::read(...).ok()?` と `serde_json::from_slice::<SentVersion>(&bytes).ok()` で、無いことと読めないことを区別せず `None` を返し、どこにも数えない。
  `version`（:646-665）は `floor.is_none_or(|f| f < version)` で、読めなかった最大値の上に今回の版を `atomic_write` で書く —— 帳面も退避されていると今回の版は読んだ時刻で、
  R90 の実測では前に送った版より 400 日古い値で上書きされた（壊れていたファイルの中身は二度と読めなくなる）。
  `heartbeat.rs:27-35` の `history_blocker` は `NONE_FOUND` / `NOT_QUEUED` / `unreadable` だけで、`runtime.rs:171-191` の `HistoryBeat::record` も手がかりの読みの結果を受け取らない。
  deep.md 第 8 回と R90 の処置は「答えの記録の後の fix で入れる」とし、head（`cbe957f0`）ではまだ入っていない
- 直すところ: コード（読めない `sent_version.json` を上書きせず退避し、無い（初回・保存場所ごと消えた）と読めないを分け、読めないときだけ種別を履歴の生存信号に載せる。C7・C8 の答えに合わせる）
- 機械: 安く捕まえられる（Runtime の層のテスト —— `sent_version.json` を `{broken` にして取得し、退避されていること・生存信号の `blockers` に種別が挙がること・ファイルが無いだけの初回では挙がらないことを見る。R90 の一時テストと同じ形）
- 行き先: fix（読めない `sent_version.json` を上書きせず退避し、無いと読めないを分け、読めないときだけ種別を履歴の生存信号に載せる。C7 の D（仮）に合わせる。このブランチ）

## 未確認

- 帳面が「すべて」読めないかの判定の範囲は確かめていない。`version` の床（`collect.rs:647-649`）は、その取得で開いた帳面（`stores`。:559-621 で見つかったプロファイルと消えたプロファイルの分）だけを見る。
  その取得で開かれない帳面に前に送った版が残っている場合に要件の「すべて」に当たるかは、コードの経路を全部は追っていない
- 退避した `sent_version.json` の置き場の名前（帳面の `*.broken.ledger` と揃えるか）と、退避が失敗したときの扱いは、まだ作りが無いので確かめようがない
- 稼働状況の画面が履歴の `blockers` の文字列をどう出すか（`web/src` に `history-` の語は無い）は見ていない。新しい種別が画面でどう読めるかは未確認

## 触らなくてよいと判断したもの

- `openspec/changes/st08-browser-history/tasks.md`: 凍結（CLAUDE.md「走っている Story へは差し戻さない」）。Q13 の作りは deep.md と R90 の処置が「答えの記録の後の fix」とし、このブランチ自身の fix で入る
- `docs/stories/ST08.md` と `stories.json` の ST08: 要件の逐語は FR-13 の冒頭で切ってあり、★ 2026-10-14 の行は入らない。表題・価値・完了の判定（「前日に見たページが翌日の取得で D-01 に入っている」「2 回続けて取得しても行が増えない」）は Q13 の前提に依らない。`check_chain.py` の再生成の一致も通る
- spec の Scenario :230「帳面を作り直した後も、時計が戻った後の更新は前に送った版より新しい版で届く」: WHEN は帳面だけが壊れた場合で、送った版の最大値は残る。Q13 の例外（すべて読めない）と食い違わない
- 要件 :230-231「保存場所が消えるときは未送信の記録も一緒に消える」: Q12 の場合の理由で、Q13 の場合（保存場所が残る）には当てていない。:235-236 の Q13 の理由はこれを使っていない
- `review/code.md` の R90「処置: escalated — deep.md 第 8 回 Q13」: Q13 は回答済みになり、escalated の行き先として正しい。`review_triage.py` も OK
- 前回の確認 `docs/audit/2026-10-14-st08-fr13-q12-consistency.md` の K1（行き先: fix）: この改訂の出所。最新の監査はこの確認になるので、前回の行き先は検査に読まれない
- `docs/handoff/ST25.md:21`・`openspec/specs/browsing-views/spec.md:593`・`openspec/specs/collection-coverage/spec.md:537`・`openspec/specs/desktop-collection/spec.md:41`・
  `openspec/changes/st14-outage-alert/`・`docs/ui-direction.md:87`・`docs/stories/INDEX.md:31`: FR-13 を「URL と題名の無い訪問」「90 日」「利用が主語」「件数」で引くだけで、時計が戻った後の版の扱いに依らない
  （ST14 の FR-35 の数えへの影響は C7 に書いた）
- `docs/collector-contract.md:393-396` の「PC 側の `blockers` は…3 つ」: ウィンドウのソースの語彙で、履歴は :398 で別の語彙と書き分けてある（2026-10-12 の整合の確認 K3 で直った）
