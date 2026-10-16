# 整合の確認（2026-10-12・FR-13 の追記（ST08 の深掘り Q7〜Q10））

- 基準: `90952ed325ed6ba063a79e46132e6e66e60ce5b0`..`c2e159ffa7b91a78cd1cfadb88a0c46f3d25a898`

対象: `docs/requirements.md` の FR-13 に足した 2 つの ★ 2026-10-05（Q7 時計が戻ったらすぐ取得する / Q8 URL の行が無い訪問は URL と題名を空にして入れ「消えた」にしない /
Q9 大量の取得でも前景の観測を止めない / Q10 送る準備まで済まなければ「取得できない」にし、理由は種別だけ）。文書とコードは head（`c2e159ff`）の内容で読んだ。

機械の入力:
- `req_impact.py --base 90952ed3`: 改訂は FR-13 の 1 件、扉 0。名指しは `docs/stories/INDEX.md:31`・`stories.json:197`・`docs/ui-direction.md:83`・
  `openspec/changes/st08-browser-history/`（deep.md・design.md:110・proposal.md:22,86・spec.md 8 か所・review 2 本）・
  `openspec/changes/st25-day-timeline/`（spec.md:365 ほか）・`openspec/specs/collection-coverage/spec.md:537`・`openspec/specs/desktop-collection/spec.md:41`
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった: FR-13」で、この監査を求めている当のもの
- `check_scenarios.py`: OK（ST08 の delta は 77 本。Q9 / Q10 の Scenario には `runtime.rs:2822` / `:2925` に印がある）

走っている change の印: **st08-browser-history（この枝。改訂を起こした当の change）**、st25-day-timeline（上流 merge 済み・凍結）、st12 / st21（FR-13 と重ならない）。

## C1. ST08 の spec が Q8 を持たず、「訪問の記録は URL を載せる」「取った訪問が無ければ消えた」と逆を定めている
- 重大度: 高
- 文書: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md:98
- 要件: FR-13
- 根拠: 要件 `docs/requirements.md:215`「訪問の行があってその URL の行が無い訪問は、URL と題名を空にした訪問として記録し、「消えた」とは記録しない。」。spec:98-100「THE SYSTEM SHALL その記録に、URL・ページの題名・訪問時刻・…を載せる。」、spec:151「その識別子を、ブラウザ・プロファイル・履歴 DB の訪問の番号・訪問時刻・URL の組から作り」、spec:263-264「WHEN 前回までに送った訪問が、次の取得のときにブラウザの履歴に無い THE SYSTEM SHALL 「消えた」事実を記録として残す。」。URL の行を失った訪問は識別子が変わる（design D6:181-183「URL の行を失った訪問は前と別の識別子になるが、「消えた」とはしない」）ので、spec の文言どおりなら前の識別子は「消えた」になる。Q8 の振る舞いは design D2:71-73 / D6:181 / D10:235 と `docs/collector-contract.md:411`、コード（`history/read.rs:126,166` の `LEFT JOIN`、`fetch.rs:442` のテスト）にだけあり、spec には Requirement も Scenario も無い（`grep "URL の行" spec.md` 0 件）。design.md:10-12 自身が「観測可能な振る舞いは spec に置いてある（design.md に置くと `openspec archive` で正典から落ちるため）」と書いている。走っている change だが、archive するとこの spec が正典になり、Q8 は正典から落ちて逆の SHALL が残る。正典を読んで「直す」実装者は URL の無い訪問を捨てるか「消えた」にする（loss: uncaptured。捨てた訪問は後から取れない）
- 直すところ: その文書（ST08 の spec に Q8 の Requirement と Scenario を足し、spec:98 / :151 / :263 の SHALL を URL の行が無い訪問について書き分ける）
- 機械: 安く捕まえられる（deep.md の「効く先」に `spec` と書かれていない A の答えでも、要件へ戻した ★ の文が spec の SHALL / Scenario のどこにも対応しなければ warn にする検査。ただし対応づけは語の一致では取れないので、少なくとも「A の答えの数 > spec の導出元に出る Q 番号の数」を見る程度）
- 行き先: fix（ST08 の spec に Q8 の SHALL と Scenario 2 本を足し、識別子・消えたの SHALL を書き分けた。印は runtime.rs の 2 本。2026-10-12、このブランチ）

## C2. ST08 の spec が取得の契機を 2 つに限っており、Q7（時計が戻ったらすぐ取得する）を持たない
- 重大度: 高
- 文書: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md:9
- 要件: FR-13
- 根拠: 要件 `docs/requirements.md:214`「前回の取得の成功が今より先にあるとき（PC の時計が戻った）は、24 時間を待たずにすぐ取得する。」。spec:9-10「THE SYSTEM SHALL 取得契機を、起動したときに前回の取得の成功から 24 時間以上経っている場合と、動作中に前回の取得の成功から 24 時間経った場合とする。」で契機を言い切っており、時計が戻った場合が無い。Scenario spec:69-72「WHEN 前回の取得の成功から 24 時間経たないうちに収集を起動し直す / THEN その起動では取得が行われない」は、前回の成功が先にある（経過が負）ときも文字どおりには当てはまり、要件と逆になる。コードは要件どおり（`history/fetch.rs:234-240` の `last > now ||`、テスト `history_schedule_fetches_at_once_when_last_success_is_ahead`）。deep.md:316 の「効く先: design D3、tasks 5.4」に spec が無い。archive すると正典に Q7 が無く、契機を spec どおりに作り直すと戻った幅だけ取得が止まる（loss: uncaptured。deep.md Q7「戻った幅が 90 日を超えるとその間の訪問は後から取れない」）
- 直すところ: その文書（ST08 の spec の契機の SHALL に時計が戻った場合を足し、Scenario を 1 本足す）
- 機械: 安く捕まえられる（C1 と同じ検査）
- 行き先: fix（ST08 の spec の契機に時計が戻った場合を足し、Scenario を 1 本足した。印は fetch.rs の `history_schedule_fetches_at_once_when_last_success_is_ahead`。2026-10-12、このブランチ）

## C3. 凍結中の ST25 が「ブラウザ履歴の訪問には必ず `url` と `title` がある」を前提に畳み方と要約を決めている
- 重大度: 中
- 文書: openspec/changes/st25-day-timeline/design.md:16
- 要件: FR-13, FR-56
- 根拠: ST25 design:16「どちらにも `url`・`title`・`browser`・`at` はある」、:80「この表が読むのは両方にある `browser` / `url` / `title` だけ」、D4 :102「`c02-browser-history` の訪問だけを `(event_time, url)` で畳み」。spec:81「2 台以上の PC から届いた同じ訪問（訪問の時刻と URL が同じもの）を 1 件として返す」、spec:187-191 はブラウザの要約を「訪問の多い URL のドメインの上位 3 つ」で出し、spec:286 は中身を「題名とドメイン」で出す。改訂後の FR-13（:215）で `url` と `title` の無い訪問が入る（`docs/collector-contract.md:411`「URL の行が見つからない訪問は `url` と `title` を省く」）。ST25 は (1) `url` が無い訪問どうしを `(event_time, url)` で畳むか（SQL の NULL は `=` では一致せず `GROUP BY` / `DISTINCT` では 1 つにまとまる）、(2) 件数に数えてドメインの上位から外すか、(3) 題名もドメインも無い行に何を出すか、を決めていない。加えて `docs/handoff/ST08.md` の R66（URL 付きで送った訪問が後で URL の行を失うと、URL 無しの 2 件目が入る）の 2 件は `(event_time, url)` では畳まれず、二重に数えられる。表示は計算し直せる（失われるものは無い）ので差し戻しはしない
- 直すところ: 凍結した Story への申し送り（ST25。下流が開始時に読む `docs/handoff/ST25.md` に、URL の無い訪問の畳み方・数え方・行の出し方と R66 の二重を書く）
- 機械: できない（ST25 の下流の実装ができた後なら、URL の無い訪問を含む fixture の e2e で捕まる）
- 行き先: handoff ST25

## C4. ST08 の tasks.md が Q7〜Q10 を受けておらず、Scenario の本数も古い
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/tasks.md:20
- 要件: FR-13
- 根拠: tasks:20「この change の delta は 75 本」、:195「75 本すべてに印」。spec の `#### Scenario:` は 77 本（Q9 の「訪問の多い取得でも前景の観測は止まらない」と Q10 の「送る準備を終えられない取得は取得できないとして報告される」が増えた）で、どの Task もこの 2 本を名指ししていない。deep.md:316 は Q7 の効く先を「tasks 5.4」、:326 は Q8 の効く先を「tasks 4.3 / 6.1」と書くが、tasks:101-105（5.4）・:79-83（4.3）・:117-122（6.1）の本文と検証は変わっていない。tasks:3 の読む順は「本人が決めた 3 件 + 第 2 回の 2 件」で第 3〜5 回（Q6〜Q10）を挙げない
- 直すところ: その文書（tasks.md の本数・該当 Task の本文と検証・読む順）
- 機械: 捕まえられた（本数は `check_scenarios.py` の数と tasks の「N 本」を比べれば安く捕まえられる。Task が名指ししない Scenario は `task_context.py` の側で warn にできる）
- 行き先: leave: tasks.md は issue ができた時点で凍結（CLAUDE.md「走っている Story へは差し戻さない」）。Scenario の印は check_scenarios.py が 81 本すべてを機械で見ており、Q7〜Q10 の本文と検証は spec・design・コードの側で持つ

## C5. ST08 の proposal と design の要約が、深掘り 1〜2 巡・FR-13 の訂正 1 回のままになっている
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/proposal.md:86
- 要件: FR-13
- 根拠: proposal:86「**FR-13** を訂正（★ 2026-09-15）—— 対象の範囲・訪問ごとの識別子と版・消えた事実」で ★ 2026-10-05 の 2 回の追記が無い。proposal:17「深掘りは 1 巡・3 問」、:37「2 巡・5 問」の表は Q1〜Q5 だけ。design:10「深掘りで本人が決めた 3 件は `deep.md`」、design:327「Open Questions: 無し。深掘り 1 巡・3 問で本人が決めたことと…」。deep.md:3-6 は第 5 回・10 問まで進んでいる
- 直すところ: その文書（proposal の「要件の改訂」と「深掘りで本人が決めたこと」に Q6〜Q10、design の Context と Open Questions）
- 機械: 安く捕まえられる（requirements.md の項目の最新の ★ の日付が、その項目を `satisfies` に持つ走っている change の proposal に出てこなければ warn）
- 行き先: docs（proposal に第 3〜5 回の表と FR-13 の ★ 2026-10-05、design の Context と Open Questions を直した。2026-10-12、このブランチ）

## C6. 要件は URL の無い訪問を「URL と題名を空にした」と言い、design・契約・コードは欄を「省く」
- 重大度: 低
- 文書: docs/requirements.md:215
- 要件: FR-13
- 根拠: 要件「URL と題名を空にした訪問として記録し」。design D2:72「URL と題名を省いた `visit` として送る」、`docs/collector-contract.md:411`「`url` と `title` を省く」、`history/contract.rs:52-56`（`url: Option<String>` に `skip_serializing_if = "Option::is_none"`）。読む側（ST25 など）は「空文字の `url`」と「`url` の欄が無い」を別に扱いうる（C3）
- 直すところ: requirements（「URL と題名の無い訪問として」に寄せる）
- 機械: できない
- 行き先: requirements（FR-13 を「URL と題名の無い訪問」に直し、★ 2026-10-12 にこの監査を書いた）

## K1. 区間に読みが無い生存信号は DB を開けるかだけで「取得できる」とし、送る準備の失敗を見ない —— 毎晩切る PC では Q10 の守りが出ない
- 重大度: 高
- 文書: docs/requirements.md:218
- 要件: FR-13, FR-78, NFR-13
- 根拠: 要件「履歴の「取得できる状態」（FR-78）は、履歴の DB が読めたかだけでなく、送る準備（除外の登録を読む・手元に書く）まで済んだかで決める。」。一方 ST08 spec:453-454「IF 前回の生存信号からの間にブラウザ履歴を 1 回も読んでいない THEN … 見つかった対象の履歴 DB を開けるかを確かめ、その結果で取得できる状態かを決める。」で、コードの確かめ（`history/collect.rs:112-120` の `probe` = `read::probe_readable`、`runtime.rs:666-671` で `beat.record(&profiles, true)`）は送る準備を見ない。起動直後の信号（`runtime.rs:162` で区間の `blockers` は起動ごとに空から始まる。D12「起動直後に 1 回出す」）は確かめで出るので、同じ起動の取得が確かめより先に終わらない限り `capturable=true` になる。**実測**（head のコピーに一時テストを足して `cargo test -p ashiato-collector-windows --lib`）: 除外の登録を壊し、読みを確かめより遅らせる（`FakeReader::gated`）と、起動直後の履歴の信号は `capturable=true blockers=[] attempts=1 successes=1`。読みが一瞬で終わる `FakeReader::new` では `capturable=false blockers=["history-not-queued"]` で、競争で決まる。本物の読みは全行を読み、確かめは `SELECT 1` までなので、確かめが先に終わるのが普通。次の信号は 24 時間後なので、spec:20 自身が挙げる「PC を毎晩切る運用」では毎日の信号が起動直後の 1 本だけになり、除外の登録が壊れて 1 件も送れない間も毎日「取得できる」が残る。稼働状況は日ごとに `bool_or(capturable)`（`crates/server/src/coverage.rs:640`）なので、24 時間以上動かした日でも起動した日は「取得できる」に数えられる。Q10 のテスト `history_heartbeat_reports_not_queued_at_runtime` は `runtime.rs:2953` の `for beat in &beats[1..]` で起動直後の信号を見ていない。生存信号は書き換えられない（FR-78）ので、偽の「取得できる」は後から直せず、その間に 90 日を過ぎた訪問は取れない（要件:220-221 の理由そのもの）
- 直すところ: その文書（ST08 spec:453-454 の確かめを、送る準備の直近の結果も見る形にする）とコード（直近の取得の送る準備の失敗を起動をまたいで持ち、確かめの結果と合わせる）
- 機械: 安く捕まえられる（`history_heartbeat_reports_not_queued_at_runtime` を `beats[0..]` にし、読みを `gated` で遅らせる形を足せば落ちる）
- 行き先: fix（直近の取得が送る準備を終えられなかったかを印のファイルで起動をまたいで持ち、確かめの結果と合わせる。spec の確かめの SHALL と Scenario 1 本、テスト `history_heartbeat_at_startup_reports_previous_not_queued`。2026-10-12、このブランチ）

## K2. 時計が戻った後の題名・滞在時間の更新は、取り込み口で古い到着として捨てられ、帳面が送ったと記録するので二度と送られない
- 重大度: 高
- 文書: openspec/changes/st08-browser-history/design.md:304
- 要件: FR-13, FR-22
- 根拠: 要件 `docs/requirements.md:208-209`「同じ訪問の題名や滞在時間が後から書き換わっていれば、新しい行を作らず更新し、前の版を残す」と :214（時計が戻ったらすぐ取得する）。design D15:304-305「時計が戻ると読んだ時刻も戻るので…題名・滞在時間の更新が古い到着として捨てられうる（次に時計が前の到着を追い越した取得で当たる）。」。コード: `source_updated_at` は読んだ時刻（`history/contract.rs:330`）、取り込み口は `incoming < known` で `SkippedStale`（`crates/server/src/lib.rs:1214-1216`）、収集側の帳面は積んだ時点で内容のハッシュを持ち（`history/fetch.rs:280` `mark_queued`）、`select_new_or_changed`（`fetch.rs:263-272`）はハッシュが同じ訪問を積まない。したがって「時計が追い越した取得で当たる」のは、追い越した後でその訪問の内容がもう一度変わったときだけで、捨てられた版そのものは再送されない。時計が先に進んでいた間に送った訪問の、戻った後の最終の題名・滞在時間は取り込み口に入らず、Chromium は 90 日で消すので、先に進んでいた幅が 90 日を超えれば後から取れない（loss: uncaptured）。Q7 の推奨の理由は「失うものが無い」（deep.md:314）で、この損失は Q7 の問いでも D15 の（仮）の反転条件でも本人に見せていない
- 直すところ: その文書（design D15 の記述を事実に合わせ、反転条件「帳面に持つ単調な版番号に替える」を採るかを本人に返す）
- 機械: 安く捕まえられる（サーバの結合テストで、同じ識別子に先の `source_updated_at` の到着の後、戻った `source_updated_at` で違う題名を送り、題名が更新されないことを固定すれば、D15 の文と食い違うことが見える）
- 行き先: ask（ST08 deep.md 第 6 回 Q11。design D15 の記述は事実に合わせて訂正した）

## K3. 送信契約は PC 側の `blockers` を 3 つと言い、履歴の 3 種（Q10 の `history-not-queued` を含む）が無い
- 重大度: 中
- 文書: docs/collector-contract.md:393
- 要件: FR-13, FR-78
- 根拠: 契約「PC 側の `blockers` は **`foreground`（前景が読めない）/ `uiautomation`（URL の経路が応答しない）/ `idle`（最後の入力からの経過時間が読めない）** の 3 つ（design D4）。」。コード `crates/collector-windows/src/heartbeat.rs:26-35` の `history_blocker` は `history-none-found` / `history-not-queued` / `history-unreadable:<browser>:<profile_dir>`。同じファイルの :15 の注釈も「**PC 側は 3 つ。**」のまま。契約の C-02 の履歴の節（:401-420）は `payload` の形だけで、生存信号の語彙を書いていない。改訂後の FR-13 の「理由には種別だけを載せる」が契約のどこにも無い
- 直すところ: その文書（契約の生存信号の節に履歴の 3 種と「種別だけ。URL・パスは載せない」を足す）とコード（heartbeat.rs:15 の注釈）
- 機械: 安く捕まえられる（`heartbeat.rs` の `pub const … = "…"` の値が契約に 1 回以上出ることを grep で見る）
- 行き先: docs（送信契約の生存信号の節に履歴の 3 種と「種別だけ」を足し、heartbeat.rs の注釈を直した。2026-10-12、このブランチ）

## K4. 読みのスレッドが落ちた取得は、生存信号に数えられない
- 重大度: 低
- 文書: docs/requirements.md:219
- 要件: FR-13
- 根拠: 要件「済まなかった取得は「取得できない」とし」。`history/collect.rs:353-358` は読み（と積む分の組み立て）のスレッドの `join` が失敗すると `self.schedule.failed(wall); return Some(Err(e));` で戻り、`self.report` を置かない。`runtime.rs:662-665` は `take_report` が無ければ数えないので、その取得は試行にも「取得できない」にも入らず、区間に他の読みが無ければ確かめ（K1）が「取得できる」を出す。`FsHistoryReader::read`（`collect.rs:91-110`）は `Err` を返さないので、ここに来るのは panic だけ（`unwrap_used = "deny"` で経路は狭い）。積む分の組み立ての `Err` は `fetched.plan` を通って `finish` に入り、`history-not-queued` になる（こちらは要件どおり）
- 直すところ: コード（`join` の失敗も `finish` に通し、`queued=false` として報告する）
- 機械: 安く捕まえられる（`read` で panic する読み手を差し込み、次の信号が `capturable=false` であることを見るテスト）
- 行き先: fix（読みのスレッドが落ちた取得も finish に通して「取得できない」に数える。テスト `history_heartbeat_counts_panicked_read_as_not_queued`。2026-10-12、このブランチ）

## 未確認

- **大量の未送信がある間の送信が見回りを止める長さ。** 送信は見回りのスレッドで行い（`runtime.rs:725-738` の `maybe_send` → `send`）、送れた分を取り除くたびに `Outbox::remove` が残り全件を直列化して一時ファイルに書き、同期して置き換える（`outbox.rs:128-146`）。15 万件を積んだ後は 1 回の送信（200 件）ごとに 15 万件ぶんを書き直すことになる。何秒止まるかは測っていない。要件:217 の「送る準備のために前景の観測を止めない」は送信そのものを言っていないが、理由（:219-220「止まっていた間の前景は後から取れず…「眠っていた」が残る」）は同じに当たる
- ST08 の Q7・Q8 の振る舞いを Windows の実行時テスト（`tests/runtime_windows.rs`）が持っているか。tasks 10.1 / 10.2 は未完で、手元の Windows では走らせていない
- ST25 の下流が URL の無い訪問を SQL でどう扱うか（C3）。ST25 の下流のコードはまだ無い

## 触らなくてよいと判断したもの

- **FR-78 の本文「取得できる状態か（権限・センサ・接続）」** と `openspec/specs/collection-coverage/spec.md:47`・`device-collection/spec.md:234` の同じ括弧: 例示であって語彙を閉じていない。deep.md:406 が「FR-78 は変えない」と理由つきで決めている。サーバ（`crates/server/src/heartbeat.rs:59`）も `blockers` の値を限らない
- **NFR-13 の数え方への効き**: Q10 で、送る準備が失敗した日の履歴の信号が「取得できない」になり、C-02 の分子から外れる。NFR-13 の「壊れていなかった日数」を測る意図（collection-coverage spec:542）と同じ向きで、判定式は変わらない
- `docs/stories/stories.json:197`（ST08 の価値・完了の判定・壊してはいけないもの）: 「前日に見たページが翌日の取得で入っている」「2 回続けて取得しても行が増えない」は改訂後も成り立つ。Q9 は「ST07 のウィンドウ記録」を壊さない側の決定で、breaks の文言と同じ向き
- `docs/ui-direction.md:83` の「ブラウザ履歴 数十〜数百（FR-13）」: deep.md:158 の「1 日数百〜千件規模」とずれるが、改訂（取得 1 回の件数）とは別の、1 日の密度の目安。千件にしても生の記録は 1 日 4,120 件 → 約 5,000 件で、「どの単位を 1 行にするかで 3 桁変わる」の判断は変わらない
- `openspec/specs/desktop-collection/spec.md:41`・`openspec/specs/collection-coverage/spec.md:537`: FR-13 を「訪問であって前景ではない」「利用駆動」の例として引くだけで、改訂した 4 点に触れない
- st25 の `deep-questions.json:17`・`review/deep.md`・`review/spec.md`: 経緯の記録で、FR-13 を名指しするのは当時の文言の引用
- st12-archive-ingestion・st21-place-registry: FR-13 も `c02-browser-history` も扱わない（grep の当たりは NFR-13 だけ）
- 語の揺れ: 改訂は概念の名前を変えていない（「送る準備」「帳面」「前景の観測」は ST08 の spec・design と同じ語）
- `crates/collector-windows/README.md:60-100`: 除外の登録の書き間違いは「空に倒さずエラーで止まる」と言い、Q10 の後もそのとおり（履歴は送らず `history-not-queued`）
