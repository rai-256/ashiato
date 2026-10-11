# ST12 書庫を置くだけで過去のデータが入る

## 人間へ返す未決（A）—— **なし**（第 6 回 Q15 は 2026-10-05 に答えが入った）

- **第 6 回 Q15（premise / loss: exported）—— YouTube の視聴・検索の履歴の項目が位置（`locationInfos`）を持っていたとき、消した場面の印の対象に入れるか**（code-verify 第 5 回 R92）。
  本人は推奨の側を選んだ: **位置を持つ Takeout の項目（YouTube の視聴・検索も含む）は、消した時間帯ならマイアクティビティと同じく削除済みの印を付けて入れる（後から消したときも同じ）**（`deep-answers-3.txt`。FR-50 に追記）。
  spec の同じ Requirement に YouTube の論理ソースを足し（Scenario 4 本）、design **D22-d**（**ソースの名前でなく項目が位置を持つかで決める** —— 位置の 7 本以外で書庫が項目を入れるソースすべて。本人が名指していない `c03-chrome-history` も C（厳しい側）で含める）、
  tasks に **Task 17（16.1〜16.3）** を足し、13.1 の見るものに YouTube の視聴・検索の形の `field_names` を足した。
  **Task 17 は実装済み**（final review 第 4 回・code-verify 第 6 回を通した）。足し忘れを止める見張り（R93）は、種類を derive で全部辿る（R97）。実物に欄があるかは 13.1 で確かめる（無ければ何も起きない）

- **第 5 回 Q14（premise / loss: exported）—— マイアクティビティの項目が位置（`locationInfos`）を持っていたとき、消した場面の印の対象に入れるか**（code-verify 第 4 回 R81）。
  本人は推奨の側を選んだ: **位置を持つマイアクティビティの項目は、消した時間帯なら書庫の位置と同じく削除済みの印を付けて入れる（後から消したときも同じ）**（`deep-answers-2.txt`。FR-50 にマイアクティビティの位置を含めた）。
  spec の Requirement「本人が滞在を消した時間帯の書庫の位置は、削除済みの印を付けて入る」に対象を足し（Scenario 4 本）、design **D22-b**（欄の有無だけで判定・payload と内容の鍵は変えない・中身は解析しない）、
  tasks に **Task 16（15.1〜15.3）** を足し、13.1 の見るものに `tools/archive-shape.sh` の `field_names` の `locationInfos` を足した。**Task 16 は実装済み**（final review 第 3 回・code-verify 第 5 回を通した）。
  判定は欄の値が空でなければ印を付ける側（**D22-c（仮）**。R91）。実物の書庫に欄があるかは 13.1 で確かめる（無ければ何も起きない）
- 第 4 回 Q13（消した場面の位置が書庫から別の論理ソースで入る。st22-record-deletion R6）は 2026-10-05 に「印を付けて入れる」の答えが入り、**Task 15（14.1〜14.3）/ design D22 として実装済み**。
  spec に Requirement「本人が滞在を消した時間帯の書庫の位置は、削除済みの印を付けて入る」（Scenario 5 本）。重なりの判定は端が触れるだけでも印を付ける。滞在の判定の入力には書庫の位置を足さない

## 実寸で成り立っていない Scenario（merge のときに見る）

- **「箱が上限の高さのとき 2 ソースが 800 px に収まる」は、本物の Chromium の 360×640 で 846 px**（design **D20（仮）**。code-verify R65）。
  箱が最も低いときでも 779 px で、箱と余白を除いた土台が約 703 px —— **ST02 第 8 回 Q30 の 640 px が、箱の無い画面で既に約 63 px 超えている**
  （達成の欄の実寸が約 253 px）。ST12 の箱が足す量は上限 160 px に収まっている（実測の外寸は上限まで埋めて 130 px）。
  `web/e2e/archive-layout.spec.ts` は「いまは成り立たない」を `test.fail` で固定している（直ったら緑に変わったところで落ちるので外し忘れない）。
  直す先は ST02 の達成の欄の高さか、予算の数そのもの。第 2 回 Q9 の読み取りのとおり、数の置き方が変わるだけで何も失われない
- 5 本目の 1,440 px（1,409 px）・箱の 160 px・押し下げ 160 px 以下・360 px 幅・週の帯 24 px は、本物のブラウザで成り立つことを測った

## 仮決め（B）と反転条件

| D | 何を仮で決めたか | 反転条件 |
|---|---|---|
| **D1（仮）** | 取り込み器は S-01 の中の背景の仕事。設定は環境変数。利用者が未設定なら起こさない。**置き場の既定は本人のホーム（`%USERPROFILE%` / `%LOCALAPPDATA%`、無ければ `$HOME`）から作り、相対パスは起動を止める**（R67） | 本人が起動の失敗を運用上の負担と感じたとき |
| **D2（仮）** | 論理ソースの名前と登録（移行前の 3 本は読み終えた日の翌日に退役。マイアクティビティの日本語名はハッシュ。名前と 60 日を試験で固定した。R69） | 退役した格子が一時的に「途絶」で見えるのが紛らわしいと分かったとき |
| **D3（仮）** | 中身は**形**で見分ける（訳されたフォルダ名に依存しない） | 実物の書庫で形が表と違うとき（解析器の版を上げ、覚えている書庫を読み直す） |
| **D5（仮）** | 原文は書庫のバイト列の切り出し。項目 1 件の上限は 64 MiB | 64 MiB を超える正当な項目が実物にあったとき |
| **D6（仮）** | 各解析器が作る記録の形（論理ソースへの割り振り） | 実物の書庫に、表に無い中身が出たとき |
| **D7（仮）** | 台帳の列（追記のみ・記録の本文を持たない） | 実物で台帳から辿れないことが出たとき |
| **D9（仮）** | 写しは既定で残す。確認待ちの書庫は設定に関わらず写す | 本人が確認待ちの写しは残したくないと言ったとき |
| **D10（仮）** | 生存信号は取り込み器の論理ソース 1 本にだけ | 書庫のソースの途絶が本人の実感と合わないとき |
| **D12（仮）** | 箱は Must の 5 本の前・**外寸** 160 px 以下（`border-box`）・各行は 1 行で「…」に省く（R64）・取り込み器の止まりは 3 日で出す | 優先の低い行が常に省かれ、本人が見落としたと分かったとき |
| **D17（仮）** | 書庫は 1 ファイルずつメモリに載せて読む。**tasks 4.4 の検証は関数の性質の証跡で、本番の経路はまだ通らない**（R71） | H.1 で本物を置いて読み込みが終わらない・常駐メモリが問題になるとき |
| **D18（仮）** | 印を置いた後の読み直しは台帳の行を増やさない | 本人が「印を置いた回」を台帳で数えたいと言ったとき |
| **D19（仮）** | 箱の「取り込み器」は直近の走査を見る | 「読めない」が点滅して邪魔になるとき |
| **D20（仮）** | 画面の実寸は本物のブラウザで測る。800 px は `test.fail` で未達を固定（上の節） | 予算の数を置き直したとき、または土台が 640 px に収まったとき |
| **D22-a（仮）** | 置き場の書庫の印付けが落ち続けたら格納の失敗として数え、3 回で台帳に `store_failed`（outcome は流用）・以後 1 時間に 1 回（R80）。**写しからの読み直し（確認待ち・版の読み直し）も書庫の `sha256` ごとに格納と印付けの失敗をまとめて数え、成功したら数を消す**（R82。移行 3 本目 `core.archive_reread_failure`）。部品ごとに試験で固定（R90） | 本人が箱の文言で格納の失敗と印付けの失敗を見分けたいと言ったとき |
| **D22-c（仮）** | マイアクティビティの `locationInfos` は中身の形を見ない。値が `null`・空の配列・空のオブジェクト・空文字でなければ位置を持つとみなす（R91。design の C「厳しい側」に実装を揃えた）。**D22-d の範囲（YouTube の視聴・検索・Chrome の履歴）も同じ判定**（R95） | 13.1 で実物の欄が位置でない値を持つ形だと分かり、位置を持たない項目まで印が付くとき（印は戻せる） |
| **D21（仮）** | `tools/archive-shape.sh` は `psql` が無ければ開発用コンテナの `psql` へ回す（R68）。tasks 11.4 の検証は `${BIND}` に叩く（R70） | 本人の DB が開発用コンテナでなくなったとき / port を台帳で割り当てるようになったとき |

### Ruling（全 Task の ledger から。`.superpowers/sdd/st12-task-final/progress.md`）

- re-review Minor 1 — Ruling: `reparse_older_versions` で写しが全部未確認・見分け不能なら `read` 行を書かず、120 秒ごとに写しを読み直すのを park する — 止まりはせず負荷だけで、そのような書庫は形の印が置かれれば抜ける — 誤りなら走査ごとの CPU と I/O が残る（直すなら「読み直したが対象なし」の印を版ごとに残す）
- re-review Minor 2 — Ruling: 残さない設定で、同じ中身の写しを共有する書庫が別の書庫より先に読み直されると、その写しが消えずに残るのを park する — 残る側（捨てない側）に倒れる取りこぼしで、本人のデータは失われない — 誤りなら残さない設定でも写しが残る（直すなら `made_copy` を写しの側で持つ）
- re-review Minor 3 — Ruling: Timeline の 1 セグメントが `visit` と `activity` の両方を持つと 2 件が同じ原文を共有するのを park する — 実物は通常どちらか一方で、原文は両方の記録に正しく含まれる — 誤りなら原文の重複分だけ容量が増える
- code-verify 第 3 回の re-review Minor 1 — Ruling: 取り込み器を起こさない（利用者が未設定）ときでも、置き場の環境変数に相対パスを入れればサーバ全体が起動しないのを park する — 明示した設定の誤りを黙って通さない側（KEEP_COPIES の綴り違いと同じ）で、既定のままなら起きない — 誤りなら書庫を使わない人が設定を消すまで起動できない
- code-verify 第 3 回の re-review Minor 2 — Ruling: 800 px の Scenario の印が `test.fail` の試験にも付き、`check_scenarios.py` が「担保あり」と数えるのを park する — 未達であることは試験の中・design D20（仮）・この本文の冒頭で示し、緑に変われば `test.fail` が落ちる — 誤りなら印の数だけを見る人が未達に気づかない（直すなら `check_scenarios.py` が `test.fail` の印を別に数える。harness2 側）
- code-verify 第 3 回の re-review Minor 3 — Ruling: `archive-shape.sh` の docker への回り道は `-q` 付きで `INSERT 0 1` を出さず、手元の `psql` と出力が少し違うのを park する — 印の有無は一覧を出し直せば分かり、smoke は行数で見ている — 誤りなら本人が印を置けたかを出力だけでは読めない
- final review 第 2 回の re-review Minor 1 — Ruling: `mark_archive_arrivals` をその書庫が入れた位置の時刻の範囲に絞るのは「範囲の外の行は、消すときの連鎖か前の書庫の印付けで処理済み」という前提に依るのを park する — 消す側と印付けは同じ助言ロックを取り、どちらかが必ず相手を見る（D22）ので前提は成り立つ — 誤りなら範囲の外の書庫の位置が生きたまま残る（直すなら作り直しの日付境界で全期間を見直す）
- final review 第 2 回の fixer の注意 — Ruling: R72 の失敗差し込みの試験は、落ちると試験の利用者だけに掛かる trigger を DB に残すのを park する — 条件に試験の利用者を入れてあり他の試験には掛からない — 誤りなら手元の DB に trigger が溜まる（直すなら試験の始めに同名の trigger を消す）
- code-verify 第 4 回の re-review Minor 1（**final review 第 3 回 R82 で解消**。下の 1 件目の Ruling と D22-a）— Ruling: 写しからの読み直し（`reread_archive`）で印付けが落ち続けたときに失敗を数えず、毎周 読み直すのを park する — その経路は置き場のファイルの行（`archive_sighting`）を持たず、格納の失敗も同じく数えていない。入力は手元の写しで、解析器の版を上げたときに限る（D22-a） — 誤りなら版を上げた後に印付けが落ち続けると、消した場面の位置が生きたまま毎周 読み直しが続き画面に出ない（直すなら版ごとに失敗を数える行を持ち、格納の失敗と一緒に数える）
- Ruling: R82 の数え方のために 3 本目の移行（`202610051730_archive_reread_failure`）を足し、tasks.md の前置き「移行は 1 本だけ足す（D14）」を越える — D14 に本人の決定は無く（——）、表は書き換えてよい観測値だけで追記のみの表に触らない — 誤りなら移行の本数の約束が破れる（直すなら既存の表に列を足す形へ畳む）
- Ruling: R85 で `"locationInfos"` の文字列の前置きを置き、欄名を `\u` でエスケープした原文は位置を持たないと判定する — Takeout はその形を書かない — 誤りならその項目は消した時間帯でも生きたまま入る（Q14 の loss: exported。直すなら前置きを外す）
- final review 第 4 回の re-review 範囲外 1 — Ruling: st12 の試験用 DB に `st12_fault_*_tg` の trigger が 4 本残るのを park する — 第 2 回 fixer の注意（R72）と同じもので、利用者ごとの条件付きなので他の試験に効かない — 誤りなら同じ DB を使う試験が不意に落ちる（DB を作り直せば消える）
- final review 第 4 回の re-review 範囲外 2 — Ruling: worktree の `.env` の DB の port（55432）が st12 の DB（55512）と違い、`app_pool()` と `server_startup` の試験が手元で落ちるのを park する — worktree の port の割り当ては ST05 の後の fix で台帳にする決定済みで、CI は自前の DB を使う — 誤りなら手元の `cargo test --workspace` が 5 本落ち続ける
- code-verify 第 6 回の fix の re-review 範囲外 1 — Ruling: st12 の試験用 DB に `core.event` が 689,485 行（DB 585MB）溜まり、`web_session_tests::web_session_endpoint_api_token_reads_and_writes` の `/events` が 64MB の上限を超えて手元で落ちるのを park する — 試験は ST12 で変えておらず、行は変異試験の繰り返しで溜まったもので、CI はまっさらな DB を使う — 誤りなら手元の `cargo test --workspace` がこの 1 本で落ち続ける（DB を作り直せば消える）

## Task

- **54/55 が `[x]`**（Task 1〜17 すべて）。残りは **13.1（human）** —— 本物の Takeout の書庫と端末から書き出した `Timeline.json` を置いて見る。合成では Google の実物の形の揺れを再現できない。
  13.1 では、マイアクティビティ・YouTube の視聴・検索の形の `field_names` に `locationInfos` があるかも見る
- **Task 16（15.1〜15.3。第 5 回 Q14 / design D22-b）は実装済み**
- **Task 17（16.1〜16.3。第 6 回 Q15 / design D22-d）は実装済み** —— YouTube の視聴・検索と Chrome の履歴の、位置を持つ項目にも格納の直後・消すときの連鎖・後着の印を付ける

## 独立レビュー（指摘 145 件すべてに処置。`review_triage.py` rc=0）

final review 4 回（R47〜R63 / R72〜R78 / R82〜R89 / R93〜R96）と code-verify 6 回（〜R92 / R97）。どの回も fix 1 回の後の scoped re-review で全件 ADDRESSED・新しい Critical / Important なし。

- **R97**（code-verify 第 6 回）R93 の見張りは種類を手で繋いだ連なりで辿っていて、足した種類を繋ぎ忘れると見張りが緑のまま分類から漏れた → `strum::EnumIter`（dev-dependency・MIT）で全種類を辿る。名前を変える変異で見張りが落ちることを確かめた（0baf123）
- **R93〜R96**（final review 第 4 回）分類の足し忘れを止める見張り（R93）・Chrome の履歴の印付けの試験（R94）・判定の名前を範囲に合わせる（`item_located_sql`。R95）。R96（作り直しの候補に Chrome の履歴が入る）は `EXPLAIN ANALYZE` で 5000 行 4.6 ms を測って rejected
code-verify 第 5 回（R90〜R92）も fix 1 回（5749da8）の後の scoped re-review で R90・R91 が ADDRESSED・新しい Critical / Important なし（R92 は escalated）。

- **R90** 読み直しの失敗を数える部品 5 つのうち 4 つと値 2 つ（3 回・1 時間）は、外しても全試験が緑だった → 版の読み直しを直に呼ぶ試験 2 本と、確認待ちの経路の試験の締め付け。変異 M4〜M8 を入れ直してすべて落ちることを確かめた
- **R91** design D22-b が「中身の形が違っても印は付く」と書きながら、実装は空でない配列だけを見ていた → 厳しい側に揃えた（D22-c（仮））
- **R92** YouTube の履歴の項目の位置 → escalated → 第 6 回 Q15 の答えで Task 17 / design D22-d（冒頭）
- **R82〜R88**（final review 第 3 回）写しからの読み直しでも失敗を数える（3 本目の移行）・欄名の前置き・Rust と SQL の判定の一致・重なる 2 つの消去の戻し ほか。R89 は `deferred ST13`

- **R72** 印付けを台帳の `read` の後に置いていたので、一時的に落ちると次の走査で読み直されず、消した場面の位置が生きたまま残った → 印付けを台帳の前へ
- **R73** 消すときの連鎖と印付けが、利用者の位置の全期間を助言ロックを握ったまま走査した → 基準・点のソースは索引の上下限で絞る
- **R79** 書庫の位置のソースの並びから件数の大半を占める 2 本を外しても全試験が緑だった → spec の 7 本のリテラルで固定し、経路の点・`Records.json` の点・後着の印の戻しを試験の材料に
- **R80** 印付けが落ち続けると、書庫を走査のたびに丸ごと読み直し続け、台帳にも画面にも出なかった → 格納の失敗として数える（D22-a（仮））
- **R81** 位置を持つマイアクティビティの項目が印の対象の外だった → escalated → 第 5 回 Q14 の答えで Task 16 / design D22-b（冒頭）
- それ以前: R64（箱の外寸 178 px）・R65（実寸の Scenario を本物のブラウザへ）・R67（置き場の既定が相対パス）・R68（`archive-shape.sh` の `psql`）

## 申し送り

- **ST30（バックアップ）**: 写し（`ASHIATO_ARCHIVE_COPY_DIR`）をバックアップの対象に入れる（design D9）。写しが消えると、解析器の版を上げても書庫を読み直せない
- **ST23（物理削除）**: 物理削除（FR-51）が写しに届かない（design D9）。`docs/handoff/ST23.md` に 1 件（残さない設定で読み直し後に写しを消す経路）
- **`docs/handoff/ST13.md`** —— 実データの規模・形が H.1 で分かってから、か全 Story が共有する前提に触るもの（R25 ほか `deferred ST13`）
- **ST02**: 800 px の土台の超過（D20（仮））。直すのは ST02 の達成の欄か予算の数

### `docs/handoff/ST12.md`（この Story への申し送り）の扱い

- **st22-record-deletion R3** —— 外部識別子で畳むソースで、前の版の内容が別の外部識別子で届くと生きた記録として入る。**ST12 は全ソースを `external_id_kind = 'none'`（第 1 回 Q6）にしたので起きない**
- **st22-record-deletion R4** —— 消していた間に届いた外部の更新がどこにも残らない。同じ理由で起きない
- **st22-record-deletion R6** —— 第 4 回 Q13 として人間に返し、**「印を付けて入れる」の答えで Task 15 / design D22 として受けた**（冒頭）
- **st25-day-timeline R3** —— タイムラインの論理ソースの名前が design（`c03-timeline-activity` / `-path`）と実装（`c03-timeline-move` / `-route`）で違う。**未処置**。宛先の指定どおり ST12 の merge 後の `fix/` で design を実装に揃える（ST25 は実装の名前で作っている）

## 通した検証

code-verify 第 6 回（b3f0d59、この worktree の試験用 DB 55512）が独立に走らせたもの:

| コマンド | rc |
|---|---|
| `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` | 0（server lib 539 passed / collector-windows 133 / server_startup 7、failed 0） |
| `tools/check-{migrations,boundaries,openapi,private,licenses}.sh` | 0 |
| `python3 scripts/check_scenarios.py . st12-archive-ingestion` | 0 |
| `openspec validate st12-archive-ingestion --strict` | 0 |
| `python3 scripts/check_chain.py .` | 0 |
| `python3 scripts/review_triage.py . st12-archive-ingestion` | 0 |

R97 の fix の後（0baf123）: fmt / clippy / 見張り 1 本・`tools/check-licenses.sh` は rc=0。`cargo test --workspace` は server lib 538 passed / 1 failed —— 落ちた `web_session_endpoint_api_token_reads_and_writes` は試験用 DB に溜まった 68.9 万行が原因（上の Ruling。ST12 の差分の外）。

`verify-run` の記録（`evidence.jsonl`）の最後の PASS:

| 項目 | コマンド | 結果 |
|---|---|---|
| 16.1 | `cargo test -p ashiato-server archive_erased_youtube_window`（`test result: ok. [1-9]` を確かめる。Task 17 の head `1920209`） | PASS |
| 16.2 | `cargo test -p ashiato-server archive_erased_youtube_cascade` / `archive_myactivity_location_rust_and_sql_agree`（同上） | PASS |
| 16.3 | `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` / `check_scenarios.py` | PASS |
| 15.1 | `cargo test -p ashiato-server archive_erased_myactivity_window`（Task 16 の head `e6c2de8`） | PASS |
| 15.2 | `cargo test -p ashiato-server archive_erased_myactivity_cascade`（同上） | PASS |
| 15.3 | `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` | PASS |
| 15.3 | `python3 scripts/check_scenarios.py . st12-archive-ingestion` | PASS |
| 14.1〜14.3 | 同じ形（Task 15 の head `48e6ab3`） | PASS |

変異で確かめた: `LOCATION_SOURCES` から `c03-timeline-route` を外す → 7 本落ちる / `c03-legacy-location` → 3 本 / 印付けの失敗を数えない → R80 の試験が落ちる / 端が触れる重なりを外す → 6 本 / 移行前の区間の 1 ms の余白を外す → 1 本 /
読み直しの格納の失敗を数えない・成功で数を消さない・版の読み直しの待ちを外す・閾値 4 回・待ち 8 秒 → それぞれ 1〜2 本（R90）。
web（vitest 195・lint）は code-verify 第 5 回で rc=0、playwright 28（うち 1 本は D20 の expected-fail）と `tools/check-*.sh` は第 3〜5 回で rc=0。その後、画面と API の形は触っていない。

既知の不安定（ST12 の外）: `drops_tests::drops_api_idempotent` が 4 回に 1 回 `40P01 deadlock detected` で落ちる（code-verify 第 3 回の実測）。
