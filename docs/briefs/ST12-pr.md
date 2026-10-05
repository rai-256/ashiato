# ST12 書庫を置くだけで過去のデータが入る

## 人間へ返す未決（A）—— **なし**（第 4 回 Q13 は 2026-10-05 に答えが入った）

- **第 4 回 Q13（loss: exported）—— 本人が ST22 で消した場面の位置が、書庫から別の論理ソースで入ってくる**（code-verify 第 3 回 R66 / ST22 からの申し送り **st22-record-deletion R6**）。
  本人は推奨の側を選んだ: **消した時間帯に入る書庫の位置は削除済みの印を付けて入れ、後から滞在を消したときもその時間帯の書庫の位置に印を付ける。戻せば一緒に戻る**（`deep-answers-1.txt`。FR-50 に書庫の位置を含めた）。
  spec に Requirement「本人が滞在を消した時間帯の書庫の位置は、削除済みの印を付けて入る」（Scenario 5 本）、design **D22**（重なりの判定は C: 区間の端が触れるだけでも印を付ける。滞在の判定の入力には書庫の位置を足さない）、
  tasks に **Task 15（14.1〜14.3）** を足した。いまの `[x]` は動かしていない。**Task 15 はこの本文を書いた時点で未実装**（グラフの Task ループが回し、final review と code-verify をやり直す）

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
| **D21（仮）** | `tools/archive-shape.sh` は `psql` が無ければ開発用コンテナの `psql` へ回す（R68）。tasks 11.4 の検証は `${BIND}` に叩く（R70） | 本人の DB が開発用コンテナでなくなったとき / port を台帳で割り当てるようになったとき |

### Ruling（全 Task の ledger から。`.superpowers/sdd/st12-task-final/progress.md`）

- re-review Minor 1 — Ruling: `reparse_older_versions` で写しが全部未確認・見分け不能なら `read` 行を書かず、120 秒ごとに写しを読み直すのを park する — 止まりはせず負荷だけで、そのような書庫は形の印が置かれれば抜ける — 誤りなら走査ごとの CPU と I/O が残る（直すなら「読み直したが対象なし」の印を版ごとに残す）
- re-review Minor 2 — Ruling: 残さない設定で、同じ中身の写しを共有する書庫が別の書庫より先に読み直されると、その写しが消えずに残るのを park する — 残る側（捨てない側）に倒れる取りこぼしで、本人のデータは失われない — 誤りなら残さない設定でも写しが残る（直すなら `made_copy` を写しの側で持つ）
- re-review Minor 3 — Ruling: Timeline の 1 セグメントが `visit` と `activity` の両方を持つと 2 件が同じ原文を共有するのを park する — 実物は通常どちらか一方で、原文は両方の記録に正しく含まれる — 誤りなら原文の重複分だけ容量が増える
- code-verify 第 3 回の re-review Minor 1 — Ruling: 取り込み器を起こさない（利用者が未設定）ときでも、置き場の環境変数に相対パスを入れればサーバ全体が起動しないのを park する — 明示した設定の誤りを黙って通さない側（KEEP_COPIES の綴り違いと同じ）で、既定のままなら起きない — 誤りなら書庫を使わない人が設定を消すまで起動できない
- code-verify 第 3 回の re-review Minor 2 — Ruling: 800 px の Scenario の印が `test.fail` の試験にも付き、`check_scenarios.py` が「担保あり」と数えるのを park する — 未達であることは試験の中・design D20（仮）・この本文の冒頭で示し、緑に変われば `test.fail` が落ちる — 誤りなら印の数だけを見る人が未達に気づかない（直すなら `check_scenarios.py` が `test.fail` の印を別に数える。harness2 側）
- code-verify 第 3 回の re-review Minor 3 — Ruling: `archive-shape.sh` の docker への回り道は `-q` 付きで `INSERT 0 1` を出さず、手元の `psql` と出力が少し違うのを park する — 印の有無は一覧を出し直せば分かり、smoke は行数で見ている — 誤りなら本人が印を置けたかを出力だけでは読めない

## Task

- **45/49 が `[x]`**（Task 15 の 3 項目を足した後）。残りは **13.1（human）** —— 本物の Takeout の書庫と端末から書き出した `Timeline.json` を置いて見る。合成では Google の実物の形の揺れを再現できない
- **Task 15（14.1〜14.3。第 4 回 Q13 / design D22）を足した** —— この本文を書いた時点では未実装。グラフの Task ループが実装・review し、final review と code-verify をやり直す

## 独立レビュー（指摘 119 件すべてに処置。`review_triage.py` rc=0）

final review（R47〜R63）と code-verify 3 回。このターンは code-verify 第 3 回の 8 件（R64〜R71）を処置し、scoped re-review が全件 ADDRESSED（R66 は正しく escalated）・新しい Critical / Important なしと判定した。

- **R64** 本物のブラウザでは箱の外寸が 178 px で、溢れたとき「ほか N 件」と読めなかった書庫の行が箱の外に切られて見えなかった（jsdom は折り返しも `box-sizing` も知らないので全緑だった）→ `border-box` と 1 行化
- **R65** 実寸を主張する画面の Scenario 18 本が jsdom の宣言値だけ → `web/e2e/archive-layout.spec.ts`（上の節）
- **R67** 置き場と写しの既定が作業ディレクトリからの相対で、別の場所から起動し直すと印を置いた後の読み直しが永久に止まった
- **R68** `archive-shape.sh` はこの機械では `psql: command not found` で止まり、smoke だけが差し替えて緑だった

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

## 通した検証（code-verify 第 3 回の処置のターン。Task 15 を足す前の HEAD で）

Task 15 を足した後は、新しい Scenario 5 本の印がまだ無いので `check_scenarios.py` は Task 15 の実装まで落ちる。`openspec validate --strict` と `check_chain.py` は足した後も rc=0。

`evidence.jsonl` はこの change に無い（`verify-run` の記録を持たない時期の Story）ので、手で走らせた結果を書く。

| コマンド | rc |
|---|---|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0（server 508 passed） |
| `cd web && npx vitest run` | 0（195 passed） |
| `cd web && npm run lint && npm run build` | 0 |
| `cd web && npx playwright test` | 0（28 passed。うち 1 本は D20 の expected-fail） |
| `tools/smoke.sh` | 0（9b は `psql` の無いこの機械で道具の回り道を通った） |
| `tools/check-immutable.sh` / `check-migrations.sh` / `check-openapi.sh` / `check-boundaries.sh` / `check-licenses.sh` / `check-private.sh` | 0 |
| `python3 scripts/check_scenarios.py . st12-archive-ingestion` | 0 |
| `python3 scripts/check_chain.py .` / `openspec validate st12-archive-ingestion --strict` | 0 |
| `python3 tools/st12_delta_diff.py` | 0 |
| 11.4（`tools/seed.sh normal` → `curl … "http://${BIND:-127.0.0.1:18787}/archives/status" \| jq -e '.latest_archive.outcome=="read"'`） | 0 |
| `python3 scripts/review_triage.py . st12-archive-ingestion` | 0 |

変異で確かめた: マイアクティビティの名前のハッシュの入力を変える / 登録の 60 日を 30 日にする → それぞれ新しい試験が落ちる。

既知の不安定（ST12 の外）: `drops_tests::drops_api_idempotent` が 4 回に 1 回 `40P01 deadlock detected` で落ちる（code-verify 第 3 回の実測）。
