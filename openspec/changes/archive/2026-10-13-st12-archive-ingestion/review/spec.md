# ST12 spec レビュー（独立）

対象: `openspec/changes/st12-archive-ingestion/` の proposal.md / specs/external-ingestion/spec.md / design.md / tasks.md、`docs/stories/ST12.md`（再生成後）。
突き合わせの正本: deep.md（Q1〜Q8、C1〜C18、要件へ戻すもの）、`docs/requirements.md`（FR-14 / FR-16 / FR-17 / FR-55 / NFR-12 / §5 EXT-M）、
`docs/stories/INDEX.md`、`openspec/specs/`（正典。特に `collection-coverage`）、proto.html。**成果物は触っていない。**

実施日: 2026-09-15。ブランチ `docs/st12-upstream`（HEAD 753f2bc）。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate st12-archive-ingestion --strict` | `Change 'st12-archive-ingestion' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。観点 8 の再生成との一致を含む） |
| `python3 scripts/check_scenarios.py . st12-archive-ingestion` | `scenarios: FAIL (担保なし 58 件)`（rc=1）。58 件はすべてこの change が足した `external-ingestion` の Scenario で、上流の段階ではテストが無いので想定どおり。tasks 12.2 が回収する |
| `python3 scripts/review_triage.py . st12-archive-ingestion` | `triage: OK`（rc=0。このファイルを書く前の状態。review/deep.md の 11 件） |

手で確かめたこと:

- **collection-coverage の正典との文言上の矛盾**: 「Must の 5 ソースがひとスクロール以内」（正典 spec.md:623 / :728-731）、「退役したソースは Must の後ろで畳む」（:624 / :733-737）、
  格子の形（縦長・3 段・週の選択・直近 4 週。:607-622）、8 状態の判定順（:372-382）について、external-ingestion の画面の Requirement（spec.md:372-381）は**どれも否定していない**。
  並びは「Must → 書庫 → 退役」で、正典の「退役は Must の後ろ」を満たす。ただし置き場と試験の側に食い違いがある（R1 / R2）
- **生存信号の形**: `crates/server/src/heartbeat.rs:12-31` の `HeartbeatRequest`（`capturable` / `blockers` / `attempts` / `successes` / `emitted_at`）と `validate()`（:52-63）に照らし、
  D10 の信号は受け入れの判定（理由の無い「取れない」を断る・回数を持つ・成功 ≤ 試行）を満たす形になっている。食い違いは時刻と回数の意味の側（R8）
- **tasks が名指しする道具と名前**: `tools/{check-migrations,check-immutable,check-openapi,check-boundaries,check-licenses,smoke,seed}.sh`、crate `ashiato-server`、`--bin openapi`（`crates/server/src/bin/openapi.rs`）、
  `api_tests` / `dedup_tests` / `registry_tests`（`lib.rs:18,22,27`）、`heartbeat_*` の試験（`api_tests.rs:273-451`）、`ingest_one`（`lib.rs:365`）、`heartbeat_one`（`lib.rs:1196`）、`coverage_get`（`lib.rs:1353`）、
  `must_sources()`（`coverage.rs:43`）、`retiredLast`（`web/src/coverage.ts`、`App.tsx:5,122`）、`todayInTz`（`App.tsx`）、`TEXT.muted`（`tokens.ts:41`）、
  `web/src/__tests__/one-scroll.test.tsx` / `target-size.test.tsx`、`seed.sh` の `127.0.0.1:18787` は**すべて実在**した。食い違いは R1 / R15
- **要件へ戻すもの**: FR-14 / FR-16 / FR-17 / FR-55 / NFR-12 に ★ 2026-09-15 がある（`docs/requirements.md:178` `:187` `:194` `:464` `:607`）。§5 に EXT-M の行がある（:839。表の行なので ★ ではなく確認日 2026-09-15）
- **proposal の Capabilities と specs/**: `external-ingestion` の 1 つで一致。INDEX の表（`INDEX.md:71`）とも一致。前倒しの有無は R2 / R16

---

## R1. tasks の終了条件が既存の試験 2 本と両立せず、12.1 の `cargo test --workspace` が緑にならない
- 成果物: openspec/changes/st12-archive-ingestion/tasks.md（1.1 / 2.1 / 9.2 / 12.1）/ design.md（D12 / D14）
- 根拠: (1) tasks.md:34 と design.md:264 は移行を「`MIGRATIONS` 配列の末尾に足す」。`crates/server/src/stay_tests.rs:42-46` は `crate::MIGRATIONS.last()` の名前が `_stays` で終わることを assert している（「滞在の移行が MIGRATIONS の末尾に無い」）ので、足した時点で `stays_migration_applies_twice` が落ちる。
  (2) tasks.md:125-127（9.2）は `coverage_get` に登録簿の `c03-*` を足し、「既存の `CT coverage` がすべて通る」を条件にする。`api_tests.rs:720-744`（`coverage_endpoint_returns_five_sources`。正典の Scenario「ソースごとに格子が分かれる」の印）は `/coverage` の名前の並びを**リテラルの 5 本と `assert_eq!`** で比べている。`testdb` は全移行を当てる（`testdb.rs:39`）ので登録簿に 10 本の `c03-*` が入り、この試験は落ちる。
  一方 tasks.md:44（2.1）は `git diff --exit-code origin/main -- crates/server/src/api_tests.rs` rc=0 を求めていて、試験を直す道を塞いでいる。`cargo test coverage` は `api_tests::coverage_endpoint_returns_five_sources` にも当たる
- kind: technical
- 提案: 1.1 に「`stays_migration_applies_twice` の末尾の assert を『`s01-stay` が 1 行』だけに直す（または末尾の名前を新しい移行に合わせる）」を足す。9.2 に「`coverage_endpoint_returns_five_sources` を『先頭の 5 本がリテラルの順、その後ろは `c03-` だけ』に直す」を足し、2.1 の diff の条件から `api_tests.rs` を外すか、9.2 より後の条件にしない形に分ける
- 処置: fixed 1.1 — `stays_migration_applies_twice` の主張を「`_stays` の移行が配列にあり、当て直しても `s01-stay` が 1 行」に直すと 1.1 に書き、9.2 に `coverage_endpoint_returns_five_sources` の主張の直し方を書いた。2.1 の `git diff --exit-code` の対象から `api_tests.rs` を外した（design の Risks にも 1 行）

## R2. 画面の振る舞いを `external-ingestion` に置いたのは、INDEX の ST04 の前例と盤面の並列判定の前提に反する
- 成果物: openspec/changes/st12-archive-ingestion/design.md（D13）/ proposal.md（Capabilities / 重なりの表）/ specs/external-ingestion/spec.md（稼働状況の画面の Requirement）
- 根拠: design.md:251-253 は「`collection-coverage` に MODIFIED を書くと、下流が走っている ST04 と capability が重なる（衝突待ちの型）」を理由に置き場を選んでいる。
  `INDEX.md` の 2026-09-14 の訂正は、**ST04 の完了の判定が「ST02 の画面に出る」ことを理由に ST04 を `collection-coverage` に割り当て**、代償（ST14 / ST15 が衝突待ち）を書いている。ST12 の完了の判定「画面に日付が出る」「読めなかった書庫が画面の『直近に置いた書庫』に出る」（ST12.md:65,67）は同じ型なのに、INDEX に理由の記載が無い。
  `python3 scripts/board.py` は ST12 を `external-ingestion` だけで見ていて ST04 と重ならないと判定するが、proposal.md:81 自身が `lib.rs`（route と `MIGRATIONS`）・`App.tsx`・`CoverageGrid.tsx`・`docs/openapi.json` を両方が触ると書いている（R1 の 2 本の試験もその実例）。
  さらに、正典の画面の Requirement（collection-coverage spec.md:604-742）は書庫のソースの格子を知らないまま残るので、ST14 / ST15 がその Requirement を MODIFIED で書き換えるとき「Must → 書庫 → 退役」の並びとひとスクロールの制約（external-ingestion spec.md:373,379）が見えない。
  文言としての矛盾は無い（冒頭「手で確かめたこと」）
- kind: conflict
- 提案: どちらかに決めて INDEX に書く —— (a) ST12 を `collection-coverage` にも割り当て、画面の 3 条項（並び・見出しの注記・群の頭の箱）を MODIFIED で置き、衝突待ちの代償を書く。(b) いまの置き場のまま、INDEX に「ST12 は FR-55 の画面を `external-ingestion` に置いた。`collection-coverage` の画面の Requirement を書き換える Story は external-ingestion spec の並びの条項も見る」と理由と申し送りを書く
- 処置: fixed D13 仮 — いまの置き場（`external-ingestion`）のまま、ST04 の前例と扱いを違えた理由・代償・反転条件（ST14 / ST15 が画面の Requirement を MODIFIED で書き換えるとき移す）を design D13（仮）に書き、`docs/stories/INDEX.md` に 2026-09-15 の訂正（理由と、画面の Requirement を書き換える Story への申し送り）を足した。PR 本文に列挙する

## R3. 観測できる振る舞いが design にだけあり、archive で正典から落ちる
- 成果物: openspec/changes/st12-archive-ingestion/design.md（D1 / D7 / D9 / D11 / D12）
- 根拠: 次は応答・画面・台帳・ファイルとして外から観測できるが、spec に SHALL も Scenario も無い。
  (1) `GET /archives/status` の新設と応答の形（design.md:230-233）。proposal.md:31,69 は「稼働状況の応答に増える」と書いている。
  (2) 「直近に置いた書庫」は `already_read` を含む `finished_at` の最新行で、件数は論理ソースの合計（design.md:233）—— 同じ書庫を置き直すと箱は「入った 0 · 既にあった 0 · 読めなかった 0」になる（D8 は中身を読まないので `archive_ledger_source` の行が無い）。proto の「同じ書庫をもう一度置いた」の場面は「既にあった 51,204」を出していた（proto.html:198 の `again`）。
  (3) 最終日に**削除済みの行も含める**（design.md:218）。
  (4) 取り込み済みに同じ名前があれば `<元の名前> (2)`、移動に失敗しても台帳は残る（design.md:200-201）。
  (5) `ASHIATO_ARCHIVE_USER_ID` が無ければ取り込み器が起きない、`KEEP_COPIES` の綴り違いで起動が止まる（design.md:55-56）。
  (6) 「取り込み中」を画面に出さない（design.md:175）
- kind: technical
- 提案: (1)(2)(3) は spec の最終日と画面の Requirement に SHALL と Scenario を足す（(2) は「置き直した書庫のとき箱に何が出るか」を決めて Scenario にする）。(4)(5)(6) は本人の手触りに効くので、spec に 1 行ずつ置くか、D 番号に（仮）と反転条件を付ける
- 処置: fixed specs/external-ingestion/spec.md — (2) 既に読んだ書庫を置き直したときの箱（Scenario「既に読んだ書庫を置き直すとそれが箱に出る」）/ (3) 最終日に削除済みの記録を含める（Scenario「最終日の記録を消しても最終日は戻らない」）/ (4) 同じ名前は上書きせず番号（Scenario「取り込み済みに同じ名前があっても上書きしない」）を spec に置いた。(1) `/archives/status` の形は design D12 に置いたまま（画面の Scenario が観測点）。(5) 起動の振る舞いは design D1 の（仮）と反転条件。(6) 読んでいる間の表示は第 2 回 Q9 で本人に問うた（R11 と同じ問い）

## R4. HTML のマイアクティビティの数え方が spec と design で違う
- 成果物: openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md / design.md（D3）
- 根拠: spec.md:74 は「HTML の形のファイルは読めなかったものとして台帳に残す」と無条件。design.md:101 は「**同じ製品の JSON が同じ書庫にあれば、HTML は読まなかったに数える**」、さらに YouTube の `.html` も読めなかったに含める（spec は マイアクティビティ だけ）。
  Scenario（spec.md:112-115）は JSON が無い書庫しか見ないので、どちらの実装でも通る。台帳の「読めなかった」は画面の箱に出る（spec.md:376）ので、数え方の違いは本人に見える
- kind: technical
- 提案: spec に D3 の 2 条件（同じ製品の JSON があるとき / YouTube の HTML）を SHALL で書き、「JSON と HTML が同居する書庫では HTML は読まなかったに数える」の Scenario を足す（あるいは D3 を spec に合わせる）
- 処置: fixed specs/external-ingestion/spec.md — HTML は同じ製品の JSON が無いとき読めなかった・あるとき読まなかった、を SHALL にし、YouTube の HTML も含めた。Scenario「JSON と同居する HTML は読まなかったに数える」を足した

## R5. 同じ名前で置き直した書庫は D8 の条件では台帳に行が立たず、取り込み済みへも移らない。Scenario は「別の名前で」を選んでいて落ちない
- 成果物: openspec/changes/st12-archive-ingestion/design.md（D8 / D9）/ specs/external-ingestion/spec.md
- 根拠: spec.md:262 は「覚えている書庫と同じ中身のファイルを置き直す」とき名前を問わず `already_read` を 1 行残す。design.md:186-187 は「`archive_sighting` に無い `(inbox, file_name)` で現れたときだけ」行を足す。`archive_sighting` は書き換えてよいキャッシュで消す規則が無い（design.md:183-184）ので、取り込み済みへ移した後に**同じ名前**で専用のフォルダへ置き直すと行が立たない。
  D9 の移動は台帳の INSERT の後（design.md:200）なので、ファイルは専用のフォルダに残り続け、画面の箱も変わらない。完了の判定 2「同じ書庫をもう一度置いても行が増えない」（ST12.md:64）の本人の操作はたいてい同じ名前での置き直しだが、Scenario（spec.md:273-276）と tasks 11.3（tasks.md:152）はどちらも「別の名前で」を使う
- kind: technical
- 提案: D8 の条件を「前回の走査の一覧に無かった `(inbox, file_name)`」など、取り込み済みへ移した後の同名を拾う形に直す。spec に「同じ名前で置き直しても台帳に 1 行・取り込み済みへ移る」の Scenario を足す
- 処置: fixed D8 — sighting の行を一覧から消えた走査で消す形にし、同じ名前の置き直しを新しいファイルとして見るようにした。spec に Scenario「同じ名前で置き直しても台帳に 1 行残り取り込み済みへ移る」「ダウンロードのフォルダに残り続ける書庫は台帳を増やさない」を足し、tasks 3.2 に割り当てた

## R6. 「置かれてから 10 分以内に読み始める」は D1 の既定で守れず、試験も既定の値を見ない
- 成果物: openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md / design.md（D1）/ tasks.md（3.3）
- 根拠: spec.md:14 は 10 分以内。design.md:59 は 5 分おきの走査で「安定の確認 2 回ぶんで満たす」とするが、走査の直後に置くと 1 回目の走査（約 5 分後）で初めて見つけ、2 回目（約 10 分後）で安定と判定し、そこからハッシュを取るので最悪で 10 分を超える。
  design.md:61 は「1 つの書庫を読み終えるまで次の書庫に進まない」「移行前のロケーション履歴は数十分〜1 時間」なので、その間に置いた書庫は 1 時間近く読み始められない（spec の SHALL に反する）。
  tasks.md:60（3.3）は間隔 1 秒・10 秒以内で見るので、既定の 300 秒でも 10 分を守るかは誰も確かめない。Scenario（spec.md:25,30,35,40）の「10 分待つ」も既定の間隔に依存する
- kind: technical
- 提案: spec の数値を「走査の間隔の 2 倍 + 読み中の書庫の終わり」と矛盾しない言い方に直すか、D1 を「読み中も走査は続け、安定の確認は並行して進める」に変える。既定の値で 10 分の関係を確かめる単体の試験（時計を差し替える）を 3.2 に足す
- 処置: fixed D1 — 走査と読み手を分け（読んでいる間も走査は続く）、走査の間隔の既定を 120 秒にした（最悪 4 分余りで読み始める）。spec の SHALL を「読んでいる書庫が無いとき 10 分以内」と「読んでいる間は見つけた順に続けて読む」に分け、Scenario「既定の間隔でも置かれてから 10 分以内に読み始める」「読んでいる間に置いた書庫は読み終えた後に読まれる」を足した。tasks 3.2 の試験は既定の 120 秒のまま時計を差し替える

## R7. 格納の失敗で読み直す規則に上限が無く、失敗し続けても台帳にも画面にも出ない
- 成果物: openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md / design.md（D4 / D7）
- 根拠: spec.md:197 は「サーバの失敗で落ちたとき、その書庫を読み終えたものとして覚えない（次の走査で読み直す）」。design.md:121 は Err で書庫を中断し台帳に書かない、design.md:175 は読み終えてから 1 回で INSERT し「取り込み中」を出さない。
  同じ書庫で毎回 Err になる（1 件の値が DB の制約に当たるのに `Rejected` ではなく Err に分類される、容量が尽きている等）と、5 分おきに大きな書庫を読み直し続け、台帳に行が無いので画面の箱は「まだ書庫が置かれていません」か前の書庫のまま。
  FR-55 の理由「置いたのに入っていないことに気づけない（書庫は約 7 日で失効する）」（requirements.md:464-465）がそのまま起きる
- kind: technical
- 提案: 「同じ書庫が N 回続けて中断したら『読めなかった（格納の失敗）』として台帳に 1 行残し、次の走査からは読み直さない（解析器の版が上がるか置き直したら読む）」のような上限と、その Scenario を足す。N は D 番号に（仮）で置く
- 処置: fixed D7 — 同じ書庫が続けて 3 回中断したら台帳に `store_failed` を 1 行・以後 1 時間に 1 回読み直す、を design D4 / D7（回数と間隔は仮）と spec の SHALL に置き、Scenario「格納に続けて失敗した書庫は台帳と画面に出る」を足した（tasks 6.2 / 10.3）

## R8. 取り込み器の生存信号は collection-coverage の受け入れの形は満たすが、送る日・回数・置き場の対応が Scenario と合わない
- 成果物: openspec/changes/st12-archive-ingestion/design.md（D10）/ specs/external-ingestion/spec.md（生存信号の Requirement）
- 根拠: 形は満たす（`heartbeat.rs:52-63` の検査に通る。冒頭）。食い違いは 4 点。
  (1) design.md:209 は「`Asia/Tokyo` の日が変わって**最初の走査**で」残すので、`emitted_at` は翌日に属し、中身の回数は前日の走査のもの。spec.md:334-335 の Scenario「取り込み器が 1 日動き → **その日の**取得できる状態の生存信号」と日がずれ、夜に PC を切る運用ではその日の信号が無い。
  (2) 回数は `tokio::spawn` の中の値（design.md:48,208）で、サーバの再起動で消える。再起動の後の最初の走査を「日が変わった」と見るかが決まっておらず、同じ日に 2 件か 0 件になる。
  (3) spec.md:325 の「走査の回数と成功した走査の回数」を確かめる Scenario が無い（2 本とも `capturable` しか見ない）。正典が生存信号に回数を要求した理由（collection-coverage spec.md:49,68-72）が、このソースでは検証されない。
  (4) design.md:208,210 は**両方の**置き場が読めたときだけ成功・取得可とするので、ダウンロードのフォルダが読めないだけで、専用のフォルダからしか入らないタイムラインのソース（spec.md:12-13）まで「取れない」になる。専用のフォルダは作らない（design.md:60）ので、導入直後は全ソースが毎日「取れない」
- kind: technical
- 提案: (1) 信号の `emitted_at` と「その日」の対応を spec に書く（例: 前日ぶんを翌日の最初の走査で送るなら Scenario をそう書く）。(2) 回数を `archive_sighting` か別の表に持つか、再起動の後の扱いを D10 に書く。(3) 回数の Scenario を 1 本足す。(4) 置き場とソースの対応（タイムライン系は専用のフォルダだけ）で `capturable` を決めるかを D10 に（仮）で書く
- 処置: fixed D10 仮 — (1) その日の最初の走査で送る（`emitted_at` はその日）(2) 同じ日に既にあれば送らない（再起動しても 1 件）、回数は再起動で 0 から（仮と反転条件）(3) Scenario「生存信号は走査の回数と読めた走査の回数を持つ」を足した (4) 信号を取り込み器の論理ソース 1 本にしたので、置き場とソースの対応の問題は消えた（R9 の処置）

## R9. 1 日 1 回の生存信号が、書庫のソースで「途絶」と FR-35 の通知を立たなくし、Q7 の 60 日を効かなくする
- 成果物: openspec/changes/st12-archive-ingestion/deep.md（C17 / 後続へ送るもの ST14）/ specs/external-ingestion/spec.md / docs/requirements.md（FR-35）
- 根拠: 正典の判定順（collection-coverage spec.md:379,381-382）では、取得できる状態の生存信号がある日は「動いていた・記録なし」で、「途絶」は想定間隔を超えて記録も信号も無いときだけ。FR-35（requirements.md:294-296）の通知も「最後の記録**または最後の生存信号**」から数える。
  取り込み器が毎日信号を送る（spec.md:323）ので、本人が書き出しを 1 年忘れても、書庫のソースは格子で「途絶」にならず、FR-35 の 180 日（60 日 × 3）の通知も鳴らない。登録簿の 60 日（Q7 の答えの効く先。deep.md:130）は判定にも通知にも効かなくなる。
  deep.md:214-215 は「最後に置いた書庫から数えるほうが近い。数え方は ST14 が決める」と送っているが、FR-35 の本文は ST14 の裁量ではなく要件で決まっており、ST12 が C17 で原因を作った衝突が要件に書かれていない
- kind: conflict
- 提案: FR-35 に ★ で「書庫のソースは取り込み器の生存信号では経過を止めない（最後に置いた書庫 / 最後の記録から数える）」の方向を足すか、衝突として `docs/handoff/` か INDEX の申し送りに要件の番号つきで書く。spec の画面の Requirement の理由の文（spec.md:386「60 日を過ぎたことを知らせるのは ST14」）に、格子では途絶が出ないことを 1 行足す
- 処置: fixed D10 仮 — 生存信号を書庫の各ソースではなく取り込み器そのものの論理ソース `s01-archive-inbox`（想定間隔 1 日）1 本に送る形に変えた（deep.md の C19。C17 から変えたことを記録し、本人には第 2 回 Q9 の context で見せた）。書庫のソースは記録だけで判定されるので、書き出しを忘れると 60 日で「途絶」になり FR-35 の通知の対象になる。spec に Scenario「書庫のソースには生存信号が残らない」「書き出しを忘れると書庫のソースは途絶になる」を足し、要件の FR-35 は変えない（書庫のソースに信号が無ければ本文どおりに働く）。ST14 への申し送りを deep.md の後続に直した

## R10. Q2 の補足の読み取りで、spec が本人の「消す」と逆の文（既に作った写しは消さない）を SHALL にしている
- 成果物: openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md / design.md（D9）/ deep.md（Q2 の補足の読み取り）
- 根拠: 本人の補足は逐語で「残すか消すかを設定できるようにする、デフォルトは残す」（deep.md:55）。deep.md:56-58 はこれを「写しを作るか作らないか」と読み、spec.md:220 は「設定が『残さない』である THE SYSTEM SHALL 写しを作らない（**既に作った写しは消さない**）」とした。
  この読みでは、設定を切り替えてもシステムは何も「消さ」ない。選んだ選択肢の文で「消す」の主語は本人（「消すのは本人」）なので、補足の「消す」は (a) 写しを消す、(b) 取り込み済みへ移した書庫をシステムが消す、のどちらにも読める。deep.md:59 は (b) だけを「読み違いなら」として挙げ、(a)（切り替えたら既存の写しも消える）を挙げていない。
  写しには本文（検索語・URL・座標）が残り、FR-51 の物理削除が届かない（deep.md:210-211）ので、本人が (a) のつもりで設定を切り替えると、残っていると思っていない本文が残る
- kind: conflict
- 提案: PR 本文の冒頭に「補足の『消す』を、写しを作らない（既存は残す）と読んだ。既存の写しも消す / 書庫を消す のつもりなら merge の前に止める」を (a) (b) 両方の読みを並べて書く。spec の SHALL は読みが確定するまで（仮）と対応づけ、D9 に反転条件を書く
- 処置: escalated — 第 2 回 Q11（B / open。推奨: 写しを作らない・既存は残す）として本人に問うた。読みが 3 つ（写しを作らない / 切り替えたら既存の写しも消す / 取り込み済みの書庫をシステムが消す）。deep.md の第 2 回 Q11 に R10 を記録し、design D9 の読み取りを（仮）にして反転条件を書いた。tasks 7.2 は答えが入るまで着手しない

## R11. Q5 の proto の読み取りの前提が、proto が実際に描いたものと違う
- 成果物: openspec/changes/st12-archive-ingestion/deep.md（Q5 の読み取り）/ design.md（D7 / D12）/ specs/external-ingestion/spec.md（画面の Requirement）
- 根拠: (1) 箱の位置 —— deep.md:106-108 は「格子 × 区画の頭」で proto は読めなかった書庫のときだけ箱を描いたとし、箱を「Must の 5 本の直後」に置いた。proto.html:343-347 は、その箱を `page.insertBefore(b, page.children[2])` で**達成の直後・Must の 5 本の前**に差し込んでいた（`children[0]` = 見出し、`[1]` = 達成、`[2]` = Must の最初の格子）。proto が描いた唯一の位置と、読み取りが選んだ位置が逆で、本人はどちらの箱も見ていない（見た場面は「初めて置いた」。deep.md:101）。
  (2) 段 —— 本人が選んだ選択肢の説明は「格子は記録のある日だけ埋まり、他は『それ以外』の段」（deep.md:95、proto.html:163）で、proto のコメントも「書庫のソースは生存信号を持たないので」（proto.html:334）。C17 の日次の信号と正典の判定順では、導入後の記録の無い日はほぼすべて「動いていた・記録なし」の段になり（R9）、本人が見た絵と逆の段で埋まる。deep.md:110-111 はこれを記録しているが、本人に返していない。
  (3) 取り込み中 —— design.md:175 は「本人は Q5 で取り込み中の表示を選んでいない」を根拠に出さないとしたが、proto は選択肢に関わらず、読み中のソースの見出しに「取り込み中」を出していた（proto.html:324）。選べる形で見せていないので、選ばなかったことの根拠にならない
- kind: premise
- 提案: 3 点を PR 本文の冒頭に本人への確認として並べる（箱は Must の前か後か / 格子のほとんどが「動いていた・記録なし」で埋まってよいか / 数十分かかる読み中に何も出なくてよいか）。proto の `importGrids` を直して、読めた書庫の箱と信号のある日の段を描いた絵を添える
- 処置: escalated — 第 2 回 Q9（A / visual）として本人に問うた。第 1 回の proto の不具合（読めた書庫の箱を描かず、読めなかった書庫の箱を Must の前に差し込んでいた）と格子の段の描き方（ST02 の判定順どおり）を直した `proto-r2.html` を作り、箱の位置（Must の前 / 後ろ）と読んでいる間の表示（出さない / 出す）を選ばせる。deep.md の第 2 回 Q9 に R11 を記録。tasks 10.3 は答えが入るまで着手しない

## R12. マイアクティビティの論理ソース名は凍結されるのに、ダウンロードのフォルダを自動で読むので、`archive-shape.sh` の確認より先に本物の書庫が入りうる
- 成果物: openspec/changes/st12-archive-ingestion/design.md（D2 / D1 / D15）/ tasks.md（H.1 / 11.5）
- 根拠: design.md:83-85 は `c03-myactivity-<products[0] を畳んだもの>` を名前にし、「名前は凍結されるので、**最初の本物の書庫を入れる前に** `tools/archive-shape.sh` で確かめる」とする。論理ソースは収集した行で書き換えを拒まれる（`migrations/202609120944_gates.sql:55-58`）。
  ところが取り込み器は `ASHIATO_ARCHIVE_USER_ID` を置いた時点で起き（design.md:56）、既定でブラウザのダウンロードのフォルダ（design.md:53）の `takeout-*.zip` を読む（spec.md:11）。本人のダウンロードのフォルダに**過去にダウンロードした Takeout の書庫が残っていれば、最初の走査で読まれる**。確認の順序は手順書（tasks.md:156-157）と人間の確認待ち（tasks.md:173-175）の文にしかなく、機械が止めない。
  `products` が言語で訳される・欠けると分かったとき（design.md:85 の反転条件）には、既に入った行の論理ソースを直せない
- kind: irreversible
- loss: rewrite-all
- 提案: 本人に返す —— 「マイアクティビティだけ、形の確認が済むまで格納しない（写しと台帳には残し、確認の印を置いてから読み直す）」か「確認の前に入ってもよい（名前が外れたら全行の書き直しを受け入れる）」かを問う。前者なら D8 の読み直しの経路で戻せるので、取り込み器の設定に確認の印を 1 つ足すだけで済む
- 処置: escalated — 第 2 回 Q10（A / rewrite-all。推奨: 形の確認の印を置くまでマイアクティビティだけ格納しない）として本人に問うた。選択肢に「確認を待たずに入れる」「1 本の論理ソースにまとめる」を置いた。deep.md の第 2 回 Q10 に R12 を記録。tasks 5.5 は答えが入るまで着手しない

## R13. C12「列を持つ」を D11 が「読むときに導く」に変え、proposal の Impact とも食い違う
- 成果物: openspec/changes/st12-archive-ingestion/design.md（D11）/ proposal.md（Impact / What Changes）
- 根拠: deep.md:174（C12）は「最終日は、いちばん新しい出来事の時刻と、それを運んだ書庫の作られた時刻の両方を保持する（**列を持つ**）」。design.md:218-219（D11（仮））は表を持たず `core.event` の `max(event_time)` を引き、書庫の作られた時刻はその行の `payload->>'archive_sha256'` から引く —— 内容の鍵で重複を弾くので、その行が持つのは**最初に運んだ**書庫で、最新の出来事を含む最近の書庫ではない（spec.md:345「その最終日の記録を運んだ書庫」の読みが 2 つに割れる）。D11 は C12 からの変更を書いていない。
  proposal.md:66 は移行に「ソースごとの取り込み済み最終日」を含め、proposal.md:69 は「稼働状況の応答に…最終日・直近の書庫を足す（`coverage_get`）」、proposal.md:81 は `coverage.rs` を触るとする。design は表を持たず（D11）、`/archives/status` を新設し（D12）、`coverage.rs` を触らない（design.md:288）
- kind: technical
- 提案: D11 の出所に「C12 の『列を持つ』を読むときに導く形に変えた理由（作り直せる）」を書き、spec.md:345 の「運んだ書庫」を「最初に運んだ書庫」か「最新の出来事を含む最新の書庫」かに決めて Scenario を足す。proposal の Impact を design に合わせて直す
- 処置: fixed D11 — 最終日を `core.event` から引く形をやめ、台帳の論理ソースごとの行に `max_event_at` を持たせて導く形にした（C12 の「列を持つ」をこの列で満たす）。「運んだ書庫」を「最終日の出来事を運んだ書庫のうち、いちばん新しく作られた書庫」と spec に定め、Scenario「最終日と一緒に運んだ書庫の作られた時刻が残る」を足した。proposal の Impact を design に合わせて直した（`coverage.rs` は触らない・`/archives/status` を新設）

## R14. Scenario が 2 つの主張を束ねているものと、Requirement の本文だけが言っていることがある
- 成果物: openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md
- 根拠: THEN と AND で 2 つを主張する Scenario が 8 本ある —— 「書き込み途中のファイルは読まれない」（:45-47。読まれない / 名前が変わったら読まれる）、「書庫の位置は携帯端末の位置に入らない」（:87-89）、「対象でないファイルは読まれず数だけ残る」（:108-110）、「UTC しか持たない時刻は UTC と印で残る」（:141-143）、
  「壊れた 1 件があっても残りは格納される」（:204-206）、「読んだ製品のファイルの写しが残る」（:238-240）、「残さない設定では写しを作らない」（:244-246）、「最終日はいちばん新しい出来事の日」（:356-358）、「書庫のソースの格子は 360 px に収まる」（:425-427）。
  本文にあって Scenario に無いもの: 削除済みの内容を**別の新しい書庫**からも入れない（:157。Scenario :183-186 は同じ書庫だけ）/ 設定を切り替えても既に作った写しは消さない（:220）/ 台帳の行に利用者識別子（:291）/ 取り込み器の生存信号に端末からと同じ保護（冪等・書き換え禁止。:326）/ 地域を位置や居住地から推定しない（:122）/ 「N 日前」の今日を `Asia/Tokyo` で数える（:374。Scenario :388-391 は日の境目を見ない）
- kind: technical
- 提案: 束ねた 8 本は、片方だけ満たす実装で緑になるものから分ける（特に :45-47 と :87-89 と :356-358）。本文だけのもの 6 つに Scenario を 1 本ずつ足す（:157 は「消した記録と同じ内容を含む、中身の違う別の書庫を置く」）
- 処置: fixed specs/external-ingestion/spec.md — 2 つの主張を束ねた 9 本を分けた（「書き込み途中」「携帯端末の位置」「対象でないファイル」「UTC」「壊れた 1 件」「写し」2 本「最終日」「360 px」）。本文だけの 6 つに Scenario を足した（「消した記録は別の書庫からも戻らない」「残さない設定に切り替えても既にある写しは残る」「台帳の行は利用者ごとに分かれる」「起動し直しても同じ日の生存信号は 1 件」「地域は位置から推定されない」「何日前は日本時間の今日から数える」）。Scenario は 58 → 87 本

## R15. tasks の終了条件のうち、数えているものが目的とずれているものがある
- 成果物: openspec/changes/st12-archive-ingestion/tasks.md（10.4 / 12.4 / 12.5 / 6.2）
- 根拠: (1) tasks.md:166（12.4）の `grep -cE "D(1|2|3|5|6|7|9|10|11)（仮）"` は**一致した行の数**を数えるので、同じ D 番号を 9 行書いても 9 以上になり、9 つの仮決めを列挙したことを確かめない。
  (2) tasks.md:167（12.5）の「あればその各項目に PR 本文で触れている」はコマンドも終了コードも持たない。
  (3) tasks.md:141（10.4）は「書庫のソース 12 本」、design.md:245 は「11 本足した応答」、design.md:263 の固定の登録は 10 本で、数が 3 つある。
  (4) tasks.md:99（6.2）は「`store_one` を差し替えた試験用の格納で 5 件目に Err」を条件にするが、design.md:108-121（D4）の `store_one(pool, IngestRequest)` は差し替えの継ぎ目を持たない
- kind: technical
- 提案: (1) は D 番号ごとに `grep -q` を 9 回回す形にする。(2) は `docs/handoff/ST12.md` の見出しの数と PR 本文の一致数を比べる形にする。(3) は 1 つの数（固定 10 本 + マイアクティビティ n 本）に揃える。(4) は D4 に格納を trait か関数の引数で受ける形を書く
- 処置: fixed 12.4 — (1) D 番号ごとに `grep -q` を回す形にした (2) 12.5 を handoff の R 番号と PR 本文の突き合わせのコマンドにした (3) 書庫のソースの数を「固定の 10 本 + 取り込み器 1 本、画面の試験はマイアクティビティ 2 本を足した 12 本」に揃えた (4) design D4 に `RecordSink` の trait と試験用の `FailingSink` を書き、tasks 2.1 / 6.2 に割り当てた

## R16. satisfies に無い要件（NFR-12 / NFR-3）を導出元にした Requirement があり、INDEX に理由が無い
- 成果物: openspec/changes/st12-archive-ingestion/specs/external-ingestion/spec.md / docs/stories/INDEX.md
- 根拠: ST12 の satisfies は FR-14 / FR-16 / FR-17 / FR-55（ST12.md:4、INDEX.md:34）。spec.md:19 と :439 は NFR-12、:383 は NFR-3 を導出元にしている。どちらも ST13 の satisfies（INDEX.md:35）。NFR-12 は ST12 の深掘りが ★ で改訂した（requirements.md:607）。
  INDEX には ST03 / ST16 / ST04 の「satisfies に無い条項を満たす」表があるが、ST12 の分は無い。FR-78（生存信号）・FR-35 / FR-61（登録簿の 60 日）も同じく ST02 / ST01 の本体の要件に条項を足している
- kind: technical
- 提案: INDEX に ST12 の訂正を 1 つ足し、「NFR-12（Google 系の手作業にタイムラインの書き出しと運搬を含む。★ Q7 / Q8）・NFR-3（書庫のソースは最終日で見せる）・FR-78（取り込み器の生存信号）を ST12 が満たす。本体の Story は変えない」を表で書く
- 処置: fixed D13 — docs/stories/INDEX.md の 2026-09-15 の訂正に、ST12 が満たす satisfies の外の条項（NFR-12 / NFR-3 / FR-78 / FR-35・FR-61）を表で足した（ST12 の change 直下のファイルではないので、処置の指す先は INDEX。design D13 からも参照）

---

観点 1（deep の決定が正典に写っているか）: Q1（6 つ。spec.md:66-68, 91-94）/ Q2（写し・移動・既定は残す。spec.md:215-256）/ Q3（2 つの置き場。spec.md:11-13）/ Q4（いちばん新しい出来事の日。spec.md:344-368）/ Q5（格子・「まで（N 日前）」・群の頭の 1 件・まだ無い。spec.md:372-432）/ Q6（内容の鍵だけ。spec.md:152-191）/ Q7（60 日。spec.md:436-450）/ Q8（運ぶ仕組みを作らない。design D13・tasks 11.5）は、それぞれ Requirement と Scenario がある。数値（10 分・60 日・540 分・1,280 px・360 px・24 px）も Scenario にある。当初案を覆したもの（C3 → Q6、C1 の粒度、C5、Q1 の「過去分を 1 回」）は覆した後だけが残っている。要件へ戻すもの 6 件は入っている。ずれは R10（Q2 の補足）/ R11（Q5 の proto）/ R13（C12）/ R9（C17 と Q7）/ R12（D2 の凍結と Q3 の自動読み取り）に書いた。
観点 2: R6 / R8 / R14 に書いた。
観点 3: R3 / R4 に書いた。spec に実装の名前（関数名・crate 名・列名）は無い（`Timeline.json` 等は取得元のファイル名）。proposal と specs/ と INDEX の capability 表は一致。置き場の判断は R2、proposal と design の食い違いは R13。
観点 4: 依存順（移行 → 関門の切り出し → 走査 → 開く → 解析器 → 重複 → 台帳・写し → 生存信号 → API → 画面）は、鍵に効く論理ソースの名前（1.1 / 5.5）が格納（5.7 / 6.1）より前にあり問題なし。人間の確認待ちは H.1（本物の Google の書き出し）と「違和感」1 問で、2026-09-14 の決定に沿っている。指摘は R1 / R15、確認の順序が機械で止まらない点は R12。
観点 5: check_chain の観点 8（再生成との一致）は OK。ST12.md の「価値」「壊してはいけないもの」「完了の判定」5 行は deep の決定（写しを残す・消さない・内容の鍵・読めなかった書庫を文字で出す）と矛盾しない。完了の判定 2 の本人の操作と Scenario の置き直し方のずれは R5。satisfies の外は R16。


## 処置のまとめ（呼び出し元が付けた）

16 件すべてに処置を付けた。

| 処置 | 件数 | 指摘 |
|---|---|---|
| 人間へ（第 2 回の深掘り Q9 / Q10 / Q11） | 3 | R11（premise → Q9 visual）/ R12（irreversible / rewrite-all → Q10）/ R10（conflict → Q11 B） |
| 仮決めとして design に置いた（PR 本文に列挙） | 3 | R2（D13 仮 + INDEX の訂正）/ R8・R9（D10 仮。生存信号を取り込み器の論理ソースへ = C19） |
| 成果物を直した | 10 | R1 / R3 / R4 / R5 / R6 / R7 / R13 / R14 / R15 / R16 |
