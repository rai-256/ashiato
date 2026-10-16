# 整合の確認（2026-10-14・FR-13 の ★ 2026-10-14 訂正（ST08 の深掘り第 7 回 Q12。時計が戻った後の更新の例外））

- 基準: `eabd0cde92551d293a84b13d426e1b4f90fcb69f`..`b4d846fa9babe4f1174ebebb1139228d6727f2a6`

（改訂は確認の時点で commit されていない —— `docs/requirements.md` の FR-13 の ★ 2026-10-14 訂正と `deep.md` の第 7 回の答えは作業ツリーの差分で、
`git diff eabd0cde -- docs/requirements.md` で読んだ。head は改訂前の commit のままなので、改訂を commit したら head をその commit の 40 桁に差し替える。
差し替えないと `consistency_triage.py` は FR-13 を「この監査を名指ししない改訂」で止め続ける）

対象: `docs/requirements.md:228-231` に足した「**ただし、収集アプリの保存場所ごと消えた・入れ直した後で、しかも時計が前に届いた版より戻っているときは、その後の更新を取りこぼしてよい。**」
（★ 2026-10-14 訂正。ST08 の深掘り Q12）。本人の決定は `openspec/changes/st08-browser-history/deep.md` の第 7 回（`deep-answers-7.txt`）。作りは変えない（design D16 のまま）。
出所は前回の確認 `docs/audit/2026-10-14-st08-fr13-q11-recheck-consistency.md` の C1（行き先: ask）と `review/code.md` の R88。文書とコードは作業ツリーの内容で読んだ。

機械の入力:
- `req_impact.py --base eabd0cde…`: 改訂は FR-13 の 1 件、扉 0。名指しは `docs/collector-contract.md:401`・`docs/handoff/ST25.md:21`・`docs/stories/INDEX.md:31`・`stories.json:197`・
  `docs/ui-direction.md:87`・`openspec/changes/st08-browser-history/`（deep.md・deep-questions*.json・design.md:110・proposal.md:22,102-106・spec.md 8 か所・review 3 本）・
  `openspec/changes/st14-outage-alert/`（deep.md:31・spec.md:1074 ほか）・`openspec/specs/browsing-views/spec.md:593`・`openspec/specs/collection-coverage/spec.md:537`・`openspec/specs/desktop-collection/spec.md:41`
- `check_chain.py`: 未回収 0・warn 0。FAIL 1 件は「整合の確認の後に要件が変わった（この監査を名指ししない改訂）: FR-13」で、この確認を求めている当のもの
- `check_scenarios.py . st08-browser-history`: OK。`openspec validate st08-browser-history --strict`: valid。`review_triage.py . st08-browser-history`: OK（指摘 118 件）
- `grep -c 'Q12\|2026-10-14'` は design.md・proposal.md・spec.md・`docs/collector-contract.md`・`docs/handoff/ST08.md` のどれも 0

走っている change の印: **st08-browser-history（この枝。改訂を起こした当の change。tasks.md は凍結）**、st14-outage-alert（FR-13 は「Chrome の履歴は 90 日」と「利用が主語」で引くだけ。Q12 は効かない）、
st17-daily-feeling・st33-data-export・st47-youtube-myactivity-split（FR-13 を名指ししない）。失われるもの（A）を伴う差し戻しは無い。

## C1. ST08 の spec が「前に送った版より前にしない」を条件なしの SHALL で言い切り、Q12 の例外を持たない
- 重大度: 中
- 文書: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md:169
- 要件: FR-13
- 根拠: 要件 `docs/requirements.md:228`「**ただし、収集アプリの保存場所ごと消えた・入れ直した後で、しかも時計が前に届いた版より戻っているときは、その後の更新を取りこぼしてよい。**」。
  spec :169-170「THE SYSTEM SHALL その更新時刻を、その訪問について前に送った版より前にしない（PC の時計が戻った後に読んだ内容も、手元の帳面を作り直した後も、前に送った版より新しい版として届ける）」
  —— 条件が無い。導出元 :174-175「第 6 回 Q11（前に送った版より前にしない）。design D6 / D7 / D8 / D15 / D16（**仮**）」に第 7 回 Q12 が無く、:183-184 の理由の段落も Q11 で止まる。
  コードは保存場所が消えた後の「前に送った版」を持てない（`crates/collector-windows/src/history/collect.rs:270-275` `read_sent_version` は無ければ `None`、:646-647 は残った帳面の `sent_version` だけを床にする）ので、
  この SHALL はその場合に満たされない。archive するとこの spec が `desktop-collection` の正典になり、正典だけが条件なしの守りを言うことになる
- 直すところ: その文書（識別子の Requirement の SHALL か括弧に「収集アプリの保存場所（`state_dir`）ごと消えた・入れ直した後は除く」を足し、導出元に第 7 回 Q12 を足す）
- 機械: 安く捕まえられる（要件の最新の ★ が名指す深掘りの Q 番号が、その Story の spec の導出元に出てこなければ warn）
- 行き先: fix（ST08 自身の下流の回し直し。このブランチ）

## C2. design D16 が「閉じないもの」を（仮）の決めの中に置いたままで、Q12 で本人が受け入れた例外になったことを持たない
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/design.md:334
- 要件: FR-13
- 根拠: design :319「### D16.（仮）帳面とは別に、送った版の最大値を持ち、版をそれより前にしない」、:334-335「扉を開けたままにする既定（大きい側に倒す）として決めた（仮）。**閉じないもの**: 置き場（`state_dir`）ごと消えた・入れ直した後に
  時計が戻っていると、最大値も帳面も無いので D15 の前と同じく捨てられうる」、反転条件 :338-339「(2) 置き場ごと消えた後の取りこぼしが実測で起きる…なら、取り込み口から訪問ごとの版を引いて床にする（取り込み口の口が増えるので本人に返す）」。
  Context :10「深掘りで本人が決めたこと（第 1 回の 3 件、第 2〜6 回の Q4〜Q11）」、Open Questions :359-361「深掘り（第 1〜6 回・Q1〜Q11）…閉じないもの（置き場ごと消えた後）と反転条件を D16 に書いた」。
  deep.md 第 7 回「design D16 の『閉じないもの』は（仮）の扱いから本人の決定（Q12）へ書き換える（spec の段・下流の fix で…）」「反転条件 (2) は残す」。
  あわせて語: 要件と問いの HTML は「保存場所」、design・deep.md の本文は「置き場（`state_dir`）」で、どちらも同じものを指すことが D16 に書かれていない
- 直すところ: その文書（D16 の「閉じないもの」を第 7 回 Q12 の本人の決定として書き、Context と Open Questions の深掘りの範囲を第 7 回 Q12 まで延ばす。「保存場所」=「置き場（`state_dir`）」を 1 か所で結ぶ）
- 機械: 安く捕まえられる（deep.md の回答済みの Q の「効く先」に design D<n> があるのに、design にその Q 番号が出てこなければ warn。前回の C2 と同じ型）
- 行き先: fix（ST08 自身の下流の回し直し。このブランチ）

## C3. 送信契約が `source_updated_at` を「その訪問について前に送った版より前にしない」と条件なしで書き、保存場所が消えた後の例外を持たない
- 重大度: 中
- 文書: docs/collector-contract.md:425
- 要件: FR-13, FR-22
- 根拠: 契約 :425-426「履歴は**その内容を読んだ時刻。ただし、その訪問について前に送った版より前にしない（前の版 + 1 マイクロ秒）**を `source_updated_at` に置く
  （ST08 deep.md 第 6 回 Q11 / design D15。帳面を作り直した後も、収集アプリが持つ送った版の最大値より前にしない —— design D16（仮））」。
  要件 :228 は保存場所ごと消えた後は取りこぼしてよいとし、そのとき版は取り込み口が持つ版より前になりうる（取り込み口は `incoming < known` を `SkippedStale` にする。`crates/server/src/lib.rs:1528-1530`）。
  契約は C-02 の外の読み手（取り込み口・ST25 など）も読む文書で、条件なしの文を「同じ訪問の版は届いた順に単調」と読むと、保存場所が消えた後に古い版が届くことを想定しない
- 直すところ: その文書（`source_updated_at` の段に「収集アプリの保存場所（`state_dir`）ごと消えた・入れ直した後は、前に届いた版より前になりうる（ST08 deep.md 第 7 回 Q12）」を足す）
- 機械: できない（語の一致では例外の有無の真偽は取れない）
- 行き先: fix（ST08 自身の下流の回し直し。このブランチ）

## C4. ST08 の proposal が第 7 回 Q12 と FR-13 の ★ 2026-10-14 訂正を持たない
- 重大度: 低
- 文書: openspec/changes/st08-browser-history/proposal.md:59
- 要件: FR-13
- 根拠: proposal :59-63 は「第 6 回（2026-10-12。1 問…）」の表（Q11）で止まり、:100-106「### 要件の改訂」の FR-13 は ★ 2026-09-15・★ 2026-10-05 × 2・★ 2026-10-12 × 2 まで。
  :106「PC の時計が戻った後に書き換わった題名・滞在時間も、前に送った版より新しい版として届け、取りこぼさない（第 6 回 Q11）」に例外が無い。
  deep.md 第 7 回は「回答済み」で、「要件へ戻すもの」に FR-13 の訂正（★ 2026-10-14）を挙げる。PR 本文と issue はこの proposal を読む
- 直すところ: その文書（第 7 回（Q12）の表と、「要件の改訂」に FR-13 の ★ 2026-10-14 訂正を足す）
- 機械: 安く捕まえられる（deep.md の「要件へ戻すもの」の表にある ★ の日付が、proposal の「要件の改訂」に無ければ warn）
- 行き先: fix（ST08 自身の下流の回し直しで proposal も直す。このブランチ）

## C5. 申し送り ST08 の R87 が、spec の SHALL を例外の無い 1 文に書き直すよう指示している
- 重大度: 低
- 文書: docs/handoff/ST08.md:45
- 要件: FR-13
- 根拠: handoff :45「`specs/desktop-collection/spec.md:168-169` と導出の段落を『読んだ時刻を基に、ただし前に送った版より前にしない』の 1 文にまとめる」。
  merge の後に `fix/` でこの指示どおりに書くと、C1 の言い切りが残る（R87 は Q12 の前の 2026-10-14 の re-review で書かれた）。要件 :228 の例外を名指ししていない
- 直すところ: その文書（R87 の書き直しの文に Q12 の例外を含めるか、C1 を直した後なら「C1 で直した SHALL と 1 文にまとめる」に替える）
- 機械: できない（申し送りの指示文と要件の例外の突き合わせは語では取れない）
- 行き先: fix（C1 で直した SHALL に合わせて handoff の R87 の文も直す。このブランチ）

## K1. 保存場所が残っていても、`sent_version.json` が読めず帳面も退避されたときは同じく取りこぼすが、要件の例外はその場合を含まない
- 重大度: 低
- 文書: docs/requirements.md:228
- 要件: FR-13
- 根拠: 要件 :228 の例外は「収集アプリの保存場所ごと消えた・入れ直した後」に限る。コード: `history/collect.rs:270-275`「送った版の最大値を読む。無い・読めないなら `None`」
  （`serde_json::from_slice(...).ok()` で、読めないことは数えず黙る）、:646-650 は床を `read_sent_version` と開いた帳面の `sent_version` だけから取る。
  壊れた帳面は退避して空から始める（`history/ledger.rs` の `LedgerStore::open`。design D16 :321）。`sent_version.json` は `fsutil::atomic_write`（:659-665）で書くので書きかけでは壊れにくいが、
  ディスクの破損などで「`sent_version.json` が読めない」と「送った訪問の帳面がすべて退避された」が重なり、時計が取り込み口の版より戻っていると、保存場所が残っていても C1 と同じく `SkippedStale` で捨てられ続ける。
  design :328「読めなければ無いものとして、帳面の前の版と読んだ時刻で決める」は扱いを書くが、「閉じないもの」（:334-335）にこの場合が入っていない
- 直すところ: requirements か design（要件の例外を「送った版の手がかりがすべて失われたとき」に広げるか、design D16 の「閉じないもの」にこの場合を足して要件の例外の範囲に入るかを決める。読めないことを数える（テレメトリ）なら コード）
- 機械: 安く捕まえられる（Runtime の層のテストで、`sent_version.json` を `{broken` にし帳面も壊してから時計を戻し、戻った後の版が前に送った版より新しいか・読めないことが数えられるかを見る）
- 行き先: fix（読めないことを数え、D16 の「閉じないもの」に足す。Q12 の例外（送った版の手がかりが残らない）の範囲に入らないと判断したら ask）

## 未確認

- 「入れ直した」ときに保存場所が消えるかは確かめていない。`state_dir` は環境変数 `ASHIATO_STATE_DIR`（`crates/collector-windows/src/config.rs:111`）で決まり、
  入れ直しで同じ値を指すか・消すかはこのリポジトリのコードには無い（配布・インストーラの形が無い）。要件の「入れ直した」は保存場所が消える入れ直しだけを指すと読んだ
- 要件 ★ 2026-10-14 の「保存場所が消えるときは未送信の記録も一緒に消える」は、未送信 `outbox.jsonl`（`runtime.rs:222`）と送った版の最大値 `browser-history/sent_version.json`（`history/collect.rs:262`・:285）が
  同じ `state_dir` の下にあることまでを確かめた。`state_dir` の一部だけが消える場合は見ていない
- 「収集側は取り込み口が持つ版を知る経路が無い」は、取り込み口の router（`crates/server/src/lib.rs:2949-2973`）に訪問ごとの `source_updated_at` を返す口が無く、`SkippedStale` は応答では `accepted: true` の中に畳まれログ（`lib.rs:1329-1334`）に出るだけ、までを確かめた。`/events` の応答の中身は見ていない

## 触らなくてよいと判断したもの

- `openspec/changes/st08-browser-history/tasks.md`: 凍結（CLAUDE.md「走っている Story へは差し戻さない」）。Q12 は作りを変えないので、Task も検証コマンドも変わらない
- `docs/stories/ST08.md` と `stories.json` の ST08: 要件の逐語は FR-13 の冒頭 6 行で切ってあり（「…（続きは requirements.md）」）、★ 2026-10-14 の行は入らない。
  表題・価値・完了の判定（「前日に見たページが翌日の取得で D-01 に入っている」「2 回続けて取得しても行が増えない」）は Q12 の前提に依らない。`check_chain.py` の再生成の一致も通る
- spec の Scenario :222・:227（時計が戻った後・帳面を作り直した後）: どちらも保存場所が残る場合で、Q12 の例外と食い違わない。例外の側（取りこぼしてよい）は観測できる振る舞いを固定しないので Scenario は要らない
- コードの注釈（`history/contract.rs:309-311`・`history/collect.rs:203`・`history/fetch.rs:292-296`）の「前に送った版」: 手元が持つ前に送った版の意味で、`next_version` の振る舞いと合う
- `review/code.md` の R88「処置: escalated — deep.md 第 7 回 Q12」: Q12 は回答済みになり、escalated の行き先として正しい。`review_triage.py` も OK
- 前回の確認 `docs/audit/2026-10-14-st08-fr13-q11-recheck-consistency.md` の C1（行き先: ask）: Q12 の答えで閉じた。最新の監査はこの確認になるので、前回の行き先は検査に読まれない
- `docs/handoff/ST25.md:21`・`openspec/specs/browsing-views/spec.md:593`・`openspec/specs/collection-coverage/spec.md:537`・`openspec/specs/desktop-collection/spec.md:41`・
  `openspec/changes/st14-outage-alert/`・`docs/ui-direction.md:87`・`docs/stories/INDEX.md:31`: FR-13 を「URL と題名の無い訪問」「90 日」「利用が主語」「件数」で引くだけで、時計が戻った後の版の扱いに依らない
- `openspec/specs/record-envelope/spec.md` の「外部サービス側の更新時刻（または版）」: 保存場所が消えた後の版も「版」に当たる（前回と同じ）
