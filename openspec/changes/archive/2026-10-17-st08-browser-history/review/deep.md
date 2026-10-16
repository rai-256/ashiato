# ST08 深掘りの独立レビュー（deep-questions.json）

**やり方**: `openspec/schemas/ashiato/schema.yaml` の `deep` の手順 1〜5 を、問いの一覧と
`deep.md` を**開かずに**先にやり直した。材料は `docs/stories/ST08.md` / `docs/requirements.md` /
`crates/collector-windows/` / `crates/server/src/{ingest,lib,coverage}.rs` / `migrations/` / 正典
`openspec/specs/desktop-collection/spec.md`、開発用 DB（`docker exec ashiato2-db-1 psql`）、
Chromium の一次ソース（`chromium.googlesource.com` の main。2026-09-15 取得）。
終わってから `deep-questions.json`（2 問 / A 2・B 0）と `deep.md` の C-1〜C-7 を突き合わせた。
問いの JSON は編集していない。

指摘の見出しの `[ ]` は種別（抜け / 不要 / 分類違い / premise / technical）。

## 手順ごとの結果

- **手順 1（要件どうしの衝突）** —— satisfies は FR-13 だけ。周辺と突き合わせた結果、
  **FR-18（原文をそのまま）× FR-22（同一なら更新）× 完了の判定「2 回続けて取得しても行が増えない」**が
  履歴 DB の書き換わる列（題名・滞在時間・from_visit・同期の印）の上で衝突する（R1）。
  **FR-20（UTC ずれとタイムゾーン識別子）× FR-13** は、履歴 DB に訪問時のゾーンが無いので値の出どころが無い（R6）。
  FR-79 × NFR-13（過去 90 日ぶんで収集開始日が遡る）は ST02 第 9 回 Q32 で決着済みで、`deep.md` の判断に同意する。
  FR-12 × FR-13（URL の二重）の読みも同意する。ただし**複数の PC が同期している場合の二重**は別に残る（R7）
- **手順 2（扉の幅）** —— doors は空。周辺の扉のうち **扉 #13（provenance）の幅**が一覧に無い:
  同期で PC の履歴 DB に入った他端末の訪問の `device_id` を何と読むか（R7）。
  扉 #7 は R1、扉 #15 は Q1 の選択肢の代償に書かれている
- **手順 3（新たに立つ一方通行）** —— Q1（どのブラウザ・プロファイル）・C-1（初回の遡り）・C-2（訪問ごと）・
  C-5（同期の訪問に印）は受けている。**差分は R1（鍵の入力）・R5（時刻の刻み）・R6（ゾーン）・
  R8（生存信号の「取得できる状態」）・R9（除外の写像）。**
  C-4 と Q2 が拠っている「`visits.id` は再利用されない」は**事実と違う**（R2。実行で再現した）
- **手順 4（日常に影響する選択）** —— 通知（72 時間）・負荷（24 時間に 1 回の写し）・容量は `deep.md` の記述を
  確かめて同意する。ST08 は面を持たない（`docs/ui-direction.md:25`）ので**画面の構造を文字で問う問いは無い —— 該当なし**。
  新たに毎日目に入るもの・手作業は見つからなかった（確かめた範囲: README の常駐・自動起動・除外登録の手順、FR-35 の想定間隔）
- **手順 5（既存コードが要件を満たしていない箇所）** —— FR-13 は未実装（`grep -rn "browser-history" crates/collector-windows` が 0 件）で、
  「動いているが満たしていない」ではない。**満たせなくなる形で既に固まっているもの**が 2 つ:
  取り込み口の鍵は必ず `raw` を混ぜ（R1）、C-02 の時刻の書き方はミリ秒で切る（R5）。
  登録簿の `c02-browser-history` は `external_id_kind = none` / `expected_gap_sec = 86400` / 0 行（psql で確認）

---

## R1. [抜け] 同じ訪問を読み直したときの鍵（C-4）は design ではなく A の問い —— いまの取り込み口に「原文に依存しない鍵」は無い

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: `crates/server/src/ingest.rs:170-181` —— `content_hash` は `logical_source` + `event_time` + **`raw` のバイト列**で、
  `external_id_kind = none` のソースはこれだけで畳む（`crates/server/src/lib.rs:280-292` / `:432-447`）。
  開発 DB の `core.source` は `c02-browser-history | 86400 | none`（`migrations/202609120940_source_columns.sql:37` が
  列を足した回に既存行を `none` へ倒した）。**`deep.md` C-4 の「原文の文字列に依存しない鍵」は、
  登録簿を `record` に変えて `external_id` を送る以外に作れない。**
  一方 C-3 は「費用ゼロで取れるものは入れる」で、その中に**後から書き換わる列**がある:
  `urls.title`（`url_database.cc:156` `UPDATE urls SET title=?,visit_count=?…`）、
  `visits.visit_duration`（`history_backend.cc:910` `UpdateVisitDuration` —— タブを閉じた後で入る）、
  `visits.from_visit`（`visit_database.cc:441`）、`is_known_to_sync`（同 `:570`）。
  これらを `raw` に入れて `none` のままだと、重ねて読むたびに別の行になり、
  完了の判定「2 回続けて取得しても行が増えない」（`docs/stories/ST08.md`）が落ちる。
  入れなければ FR-18「取得元から受け取った原文をそのまま」の幅を狭めることになる。
  `record` にすれば FR-22 の「更新して前の版を履歴に残す」に乗るが、何を外部識別子にするかは R2 の事実で縛られる。
  **いまは 0 行なので登録簿も鍵の入力も変えられるが、1 行でも入った後に変えると、既存行と新しい行の鍵が合わず二重に入る**
  （ST03 Q15 / FR-29 の注記「ハッシュに混ぜると凍結を外して全行を計算し直す移行が要る」と同じ型）
- kind: irreversible
- loss: rewrite-all
- 提案: C-4 を問いに上げる（Q3 など）。選択肢ごとに「`raw` に入る列」「登録簿の `external_id_kind`」「重ねて読んだときの挙動
  （増える / 更新して版が残る / 書き換わる列は捨てる）」と、それぞれで失われるものを書く。`deep.md` の「重なりの幅は design の仮で足りる」は、
  この鍵が重ねて読んでも動かないことが前提なので、答えが出るまで仮にできない

- 処置: escalated — Q3 として新設（`irreversible` / `loss: rewrite-all`）。選択肢は「識別子を持たせ更新して版を残す（推奨）/ 書き換わらない値だけで見分ける / 別の行として増やす」。deep.md の C-4 を問いへ移した
## R2. [premise] 「`visits.id` は再利用されない」は事実と違う —— 閲覧データを全期間削除すると表が作り直され、番号が 1 から振り直される

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: Q2 の context「`visits.id` は `INTEGER PRIMARY KEY AUTOINCREMENT` で**再利用されません**」と、`deep.md` C-4「`visits.id` は再利用されない（AUTOINCREMENT）」。
  Chromium の `history_backend.cc:3651` —— `ExpireHistoryBetween` は開始・終了が無く URL の指定も無いとき（＝全期間）
  `DeleteAllHistory` に入り、`ClearAllMainHistory`（`:4027`）が `RecreateAllTablesButURL` を呼ぶ。
  これは `visit_database.cc:263-267` の `DROP TABLE visits` → 作り直し。
  SQLite では表を DROP すると AUTOINCREMENT の続き番号も消える —— **実行で確かめた**:
  `python3 -c` で `visits` に 5 行 → 1 行削除 → 追加は `id=6`（再利用なし）/ `DROP TABLE` → 作り直し → 追加は **`id=1`**。
  さらに、壊れた DB は `history_backend.cc:3778-3790`（`DatabaseErrorCallback` → `KillHistoryDatabase`）で消されて作り直される。
  **影響**: (a) 番号を鍵に使うと、全期間削除の後の新しい訪問が過去の訪問と同じ鍵になり、`duplicate`（＝受理）として黙って落ちる
  （`record` にしていれば、古い訪問の行を別の訪問で「更新」する）。(b) Q2 の検知（前回見えた番号が今回無い＝消した）は、
  全期間削除の直後に同じ番号の新しい訪問が現れると「消えていない」と読む
- kind: premise
- loss: discarded
- 提案: Q2 の context と `deep.md` C-4 の根拠を直し、番号だけでは同じ訪問を指せないこと（表の作り直し・DB の作り直し）を書く。
  R1 の問いの選択肢の側に「番号 + 訪問時刻（マイクロ秒）+ URL」など、作り直しで衝突しない入力を並べる

- 処置: escalated — `loss: discarded` なので人間へ返す。Q2 / Q3 の context に事実として置き、見分けの組（ブラウザ・プロファイル・番号・訪問時刻・URL）を Q3 の選択の前提として本人に見せる。Q2 の context を直し、番号が全期間の削除・DB の破損で振り直されること、比べる組は番号・訪問時刻（マイクロ秒）・URL であることを書いた。Q3 の context にも同じ事実を置き、deep.md の C-4 の根拠（「再利用されない」）を消した
## R3. [premise] Q2: Chrome は同期を切ると他端末の訪問を一括で消す —— 「ブラウザで消した」の検知が本人の操作以外を拾う

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: Q2 の context は消える経路を「本人の削除」と「90 日の期限切れ（訪問の時刻で区別できる）」の 2 つとしている。
  Chromium の `history_backend.cc:1961-1990` `DeleteAllForeignVisitsAndResetIsKnownToSync` —— 同期を切ると、
  その時点の最大番号までの**他端末由来の訪問を全部**バッチ（`:190` 1 回 100 件）で消す。これは本人がブラウザで履歴を消した操作ではない。
  `deep.md` C-5 は同期の訪問を「印を付けて入れる」と決めているので、**同期を切った日に、スマホで見た 90 日ぶんの訪問が
  「ブラウザで消した」として記録される**。選択肢 2（ashiato でも消したことにする）を採ると、ST22 の後でそれが削除に化ける
- kind: premise
- 提案: Q2 の context に第 3 の経路（同期の停止）を足し、選択肢 1・2 の説明に「他端末由来の訪問（`originator_cache_guid` が空でない）が
  一括で消えた場合をどう読むか」を書く。読み分けは印（C-5）があれば後からできるので、`loss` は付けない

- 処置: fixed deep-questions.json — Q2 の context に消える経路を 4 つ並べ（本人 / 期限切れ / 同期を切った / 表の作り直し）、手がかりを記録に載せて判定はしないと書いた
## R4. [分類違い] Q2 は「消えた事実を残すか」（取るか取らないか）と「ashiato でも消すか」（ST22 の振る舞い）が 1 問に混ざっている

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: Q2 の選択肢 2 の detail が自分で「**ST22 までの間は選択肢 1 と同じ動きになる**」と書き、選択肢 1 の detail も
  「消したことにするかどうかは、ST22 ができた後で決め直せる（事実が残っているので）」と書いている。
  つまり ST08 の中で選択肢 1 と 2 の違いは 0 で、`loss: uncaptured` が当たるのは「選択肢 3（事実を残さない）か否か」だけ。
  「ashiato でも消すか」は FR-50 の担当 Story（`docs/stories/INDEX.md:44` ST22）の振る舞いで、ST08 では入力が残っていれば戻る。
  ST22 はまだ `openspec/changes/` に無く、`docs/handoff/ST22.md` が既にある
- kind: defer
- 提案: Q2 を「消えたことに気づいて事実を残すか」の 1 点に絞る（取らない側に `loss: uncaptured` が付くので A のまま）。
  「ブラウザで消したものを ashiato でも消したことにするか」は `docs/handoff/ST22.md` に送り、ST22 の深掘りで問う

- 処置: deferred ST22 — Q2 を「消えた事実を残すか」の 1 点に絞った。「ブラウザで消したものを ashiato でも消したことにするか」は `docs/handoff/ST22.md` に申し送った（ST22 は tasks.md が無い）
## R5. [抜け] 訪問の時刻の刻み（マイクロ秒）が、既存の書き方でミリ秒に切られる —— `event_time` は鍵の入力

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: 履歴 DB の `visits.visit_time` はマイクロ秒の整数（`visit_database.cc` の CREATE TABLE `visit_time INTEGER NOT NULL`、
  Chromium の `base::Time` の内部値）。取り込み口の鍵は `event_time.timestamp_micros()` を混ぜ（`crates/server/src/ingest.rs:178`）、
  `core.event.event_time` は `timestamp with time zone`（マイクロ秒。psql で確認）。一方 C-02 の時刻の書き方は
  `crates/collector-windows/src/contract.rs:202-203` の `to_rfc3339_opts(SecondsFormat::Millis, true)` で、`IngestRequest::of` が
  必ずこれを通す（同 `:253`）。ST07 の `payload_shape_is_pinned` がこの形を 1 文字単位で固定している。
  そのまま流用すると列の値はミリ秒に切られ、**後から刻みを直すと `event_time` も鍵も変わり、凍結済みの全行と対応が切れる**。
  C-3 は「訪問時刻（DB の値そのものも）」を `raw` に入れるので値そのものは失われないが、列と鍵は戻らない
- kind: technical
- loss: rewrite-all
- 提案: `deep.md` の C に足す（既定: 刻みを落とさない = マイクロ秒で `event_time` を送る。細かい粒度で持つ）。
  design で ST07 の `rfc3339` を流用しないことと、刻みを固定するテストを置くことを明記する

- 処置: escalated — Q3 の context に「訪問時刻はマイクロ秒で持つ（ウィンドウのミリ秒の書き方を流用しない）」を置き、鍵の組の一部として本人に見せる。deep.md に R5 として記録
## R6. [抜け] FR-20 のタイムゾーンに、訪問したときの値の出どころが無い —— 過去 90 日ぶんを取得時のゾーンで凍結する

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: FR-20（`docs/requirements.md:172-173`）は「出来事が起きた時刻に、UTC からのずれとタイムゾーン識別子の両方」を求める。
  履歴 DB の `visit_time` は UTC の整数でゾーンを持たない。C-02 のゾーンは**起動時の PC の設定**
  （`crates/collector-windows/src/config.rs:27` `chrono::Local::now().offset()`）で、ST07 は出来事と同時に作るので正しかったが、
  ST08 は最大 24 時間後（C-1 の初回は最大 90 日後）に作るので、旅行先で見た訪問に帰宅後のゾーンが付く。
  `tz_offset_min` / `tz_id` は「収集した」記録の列で凍結される（FR-30、移行 `202609082001_immutable_collected`）ので、
  後から位置の記録などで正しいゾーンが分かっても書き直せない
- kind: conflict
- loss: rewrite-all
- 提案: 問いを立てるか、C に「扉を開けたままにする既定」として足す（例: 取得時のゾーンであることを `raw` / `payload` に印として残す）。
  どちらにしても `deep.md` の手順 1 に FR-20 × FR-13 を挙げる

- 処置: escalated — Q3 の context に「訪問時のゾーンは DB に無い → 取得時の PC のゾーンを付け、取得時のゾーンだという印を載せる」を置いた。deep.md に R6 として記録
## R7. [抜け] 同期で入った他端末の訪問の `device_id`（扉 #13 の幅）と、同期している PC が 2 台あるときの二重

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: FR-24（`docs/requirements.md:202-203`）「どのソース・**どの端末**…が生成したか」。`deep.md` C-5 は同期の訪問を
  `originator_cache_guid` の印付きで入れると決めたが、**エンベロープの `device_id` 列に何を入れるか**（収集した PC か、訪問が起きた端末か）は書いていない。
  `device_id` も凍結される列。さらに `visit_database.cc` の CREATE TABLE の注記どおり、他端末由来の行は
  `originator_cache_guid` / `originator_visit_id` を持ち、**自端末の行は空文字と 0** を持つ。PC を 2 台（`docs/requirements.md` §5 は
  C-02 と S-01 の配置を定めず、台数も縛らない）で同期していると、同じ訪問が A では自端末の行、B では他端末の行として読まれ、
  `raw` が違うので `event_dedup_hash`（`migrations/202609120942_dedup_indexes.sql` の `(user_id, logical_source, content_hash)`）で畳まれない
  —— `docs/stories/ST08.md` の「URL が二重に入らないこと」の、`deep.md` の読みとは別の二重
- kind: conflict
- loss: rewrite-all
- 提案: R1 の問いの選択肢に「他端末由来の訪問をどの鍵で同じとみなすか（`originator_cache_guid` + `originator_visit_id` は大域で一意）」を含め、
  `device_id` の読み（収集した端末に固定する、など）を `deep.md` の C-5 に明記する。複数 PC を想定しないなら、その前提を C に書く

- 処置: escalated — Q3 の context に「記録の端末は収集した PC、発生元（`originator_cache_guid` + 発生元での番号）は印として載せ、2 台の PC からの同じ訪問は読む側で畳める」を置いた。deep.md に R7 として記録
## R8. [抜け] 2 本目のソースの生存信号で「取得できる状態」を何と判定するか —— Chrome が動いている間、履歴 DB は排他で開けない

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: FR-78（`docs/requirements.md:272-298`）は取得可否を載せよと言い「載せなかった期間の取得率は後から作れない」、
  NFR-13 は C-02 の 2 ソースで「取得できる状態だった日」を分子にする。いまの生存信号は 1 ソースに固定
  （`crates/collector-windows/src/heartbeat.rs:14` / `:226` が `LOGICAL_SOURCE = "c02-window"` を使う）で、
  正典の「取得できる状態」の定義は前景と URL の読み取り経路だけ（`openspec/specs/desktop-collection/spec.md` の生存信号の Requirement）。
  Chromium は履歴 DB を開いた後 `PRAGMA locking_mode=EXCLUSIVE` にする（`history_database.cc:405-407`）ので、Chrome が動いている間は直接読めない。
  `deep.md` 手順 4 は「写しを読む」と書くが、写しに失敗したとき・Q1 で選んだブラウザやプロファイルの一部だけが読めないとき・
  対象のブラウザが入っていないときに `capturable` をどう出すかは、C にも問いにも無い。
  定義を誤った期間の信号は凍結される（移行 `202609111112_immutable_heartbeat`）ので、その日の分子は戻らない
- kind: technical
- loss: uncaptured
- 提案: `deep.md` の C に足す（既定は厳しい側: 選んだ対象の 1 つでも読めなければ取得できないとし、`blockers` に対象のブラウザ・プロファイルを載せる）。
  spec に「写しに失敗した」「対象が見つからない」の Scenario を置く

- 処置: escalated — Q1 の context に「選んだ対象の 1 つでも読めなければ取得できない。読めなかったブラウザ・プロファイルを信号に載せる」を置いた（範囲の答えが対象を決めるので Q1 に置く）。deep.md に R8 として記録
## R9. [technical] 除外を履歴に効かせる（C-6）の写像が無い —— いまの規則はプロセスとウィンドウ題名にしか当たらず、「プロファイルを除く」は登録できない

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: `crates/collector-windows/src/exclusion.rs:21-56` —— `Rule` は `exe-path` / `process-name` / `title-contains` の 3 つで、
  `deny_unknown_fields`、当たり方は前景のプロセスとウィンドウ題名（`hits(&Foreground)`）。履歴の行にはプロセスもウィンドウ題名も無い
  （あるのは `urls.title` = そのページの**最新の**題名）。Q1 の context「取ったうえで『このプロファイルは除く』を登録できるようにします」と
  選択肢 1 の detail「除外に登録できる」は、**いまの登録の形では書けない**（知らない `match` は起動時にエラーで止まる。README の「書き間違えた登録は…エラーで止まる」）。
  `deep.md` C-6 は「効かせる」だけで、`process-name: msedge.exe` が Edge の履歴 DB 全体を指すのか、`title-contains` が `urls.title` に当たるのかが決まっていない。
  写像を誤ると、ウィンドウで除外した対象の URL が履歴の側から既定の感度（PERM-3）で入り、凍結される
- kind: technical
- loss: exported
- 提案: C-6 に規則ごとの写像（プロセスの規則 → そのブラウザの全プロファイル、題名の規則 → `urls.title`、新しい `match` を足すならその名前）を書き、
  spec の Scenario に落とす。Q1 の context と選択肢 1 の「除外に登録できる」は、登録の形を足すことが前提だと書く

- 処置: escalated — Q1 の context に「ブラウザ（とプロファイル）・URL を指す登録の形を足す。プロセスの登録はそのブラウザの履歴全体に効かせる」を置き、選択肢 1 の「除外に登録できる」に登録の形を足す前提を書いた。deep.md の C-6 に写像を書いた
## R10. [technical] 「24 時間間隔」と「前回取得以降」を何で測るか —— 24 時間続けて動かない PC と、遅れて届く同期の訪問

- 成果物: openspec/changes/st08-browser-history/deep-questions.json
- 根拠: `deep.md` は「取得の時刻・重なりの幅は design の（仮）で足りる」とする。いまの生存信号の契機は**メモリ上の前回時刻**で、
  起動直後に 1 回出す（`crates/collector-windows/src/heartbeat.rs:162-196`）。同じ形で「起動直後に出す」を持たずに作ると、
  毎晩電源を切る PC では 24 時間の契機に一度も届かない。また「以降」を訪問時刻で測ると、同期で後から入る他端末の訪問
  （`history_backend.cc:1760` `AddSyncedVisit`。訪問時刻は発生元のもの）は前回の位置より古い時刻で入るので読まれない。
  どちらも 90 日以内に規則を直せば読み直せるので `loss` は付けないが、直すまでに 90 日を過ぎた分は取れない
- kind: technical
- 提案: design の D（仮）に「起動時と、前回の**成功**から 24 時間」「取得した分を未送信に積んでから前回の位置を進める」
  「以降を訪問時刻でなく読み直し（R1 の鍵が動かない前提）で測る」を置き、反転条件を書く。問いには上げない
- 処置: fixed deep.md — C-8 に置いた（起動時と前回の成功から 24 時間 / 積んでから位置を進める / 重なりを持って読み直す）。値は design の（仮）で持つ
