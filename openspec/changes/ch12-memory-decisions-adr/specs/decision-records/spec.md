## Purpose

Story をまたぐ判断（製品の前提・コードの構成・道具の規則）を、commit されないメモリではなく repo の `docs/adr/` に 1 件 1 ファイルで残し、
索引から辿れ、どこから移したかを名指しできるようにする。開発の約束で、製品の振る舞いではない。

## ADDED Requirements

### Requirement: ADR は docs/adr/ に 1 件 1 ファイルで置き、README の一覧から辿れる

`docs/adr/` は README（`README.md`。ADR の書き方と一覧）と ADR のファイルを持つ SHALL。ADR のファイル名は `YYYYMMDDHHMM-<英小文字・数字・ハイフンの slug>.md` の形である SHALL。
README の「一覧（新しい順）」の項目は `- [<日付> <題>](<ファイル名>)` の形で、一覧が名指すファイルと `docs/adr/` の README 以外の `.md` は 1 対 1 で対応する SHALL。
各項目の日付と題は、名指す ADR の `- 日付:` の値と 1 行目の `# ` の見出しに一致し、項目はファイル名の降順（新しい順）に並ぶ SHALL。
`CLAUDE.md` の「文書とハーネス」の表は `docs/adr/` の行を持ち、何かの欄に「Story をまたぐ判断の記録」、書き込みの欄に「する」と書く SHALL。

導出元: CH12

#### Scenario: README の一覧と docs/adr/ の ADR のファイルが 1 対 1 で対応する
- **WHEN** 検証の台本が README の一覧の項目と `docs/adr/` の README 以外の `.md` を比べる
- **THEN** 一覧にあって実ファイルが無いもの・実ファイルがあって一覧に無いものが、どちらも 0 件である

#### Scenario: 一覧の日付と題は ADR の日付と見出しに一致し、新しい順に並ぶ
- **WHEN** 検証の台本が一覧の各項目を、名指す ADR の `- 日付:` の行と 1 行目の見出しと比べ、項目の並びをファイル名と比べる
- **THEN** 日付と題がすべて一致し、項目はファイル名の降順に並ぶ

#### Scenario: ADR のファイル名は時刻と英小文字の slug の形を持つ
- **WHEN** 検証の台本が `docs/adr/` の README 以外の `.md` の名前を見る
- **THEN** どれも `^[0-9]{12}-[a-z0-9]+(-[a-z0-9]+)*\.md$` に当たる

#### Scenario: CH12 で置いた README と 5 件の ADR がある
- **WHEN** 検証の台本が `docs/adr/` を見る
- **THEN** `README.md`・`202610030000-worktree-port-allocation.md`・`202610080000-foundation-for-satellites.md`・`202610110000-pure-crate-split.md`・`202610110001-ui-surface-only.md`・`202610110002-stricter-lint.md` がある

#### Scenario: CLAUDE.md の文書の表から docs/adr/ に辿れる
- **WHEN** 検証の台本が `CLAUDE.md` の「文書とハーネス」の表を読む
- **THEN** 1 列目が `` `docs/adr/` `` の行がちょうど 1 つあり、2 列目が「Story をまたぐ判断の記録」を含み、3 列目が「する」である

### Requirement: 各 ADR は出所を持ち、メモリから移した判断はメモリのファイル名を名指しする

各 ADR は `- 出所:` の行を持つ SHALL。メモリから移した判断の ADR は、その行にメモリのファイル名（`<名前>.md`）を書く SHALL。
CH12 で移した 4 つのメモリ（`worktree-port-allocation-followup.md`・`prefer-collision-free-allocation-over-checks.md`・`foundation-for-satellites-premise.md`・`lint-purity-ui-refresh-plan.md`）は、
それぞれ移した先の ADR の出所に名指しされる SHALL。メモリの残りを数える台本（harness2 の `adr_backlog.py`）にメモリの置き場を明示して走らせたとき、この 4 つは残りに数えられない SHALL。
検証は、公開されていないメモリの名前と説明を出力に出さず、4 つのメモリ名それぞれが残りに有るか無いかだけを出す SHALL。
数える台本かメモリの置き場が無いとき、検証は通ったことにせず、環境の欠落として止まる SHALL。

導出元: CH12

#### Scenario: 5 件の ADR の出所に、移したメモリのファイル名がある
- **WHEN** 検証の台本が 5 件の ADR の `- 出所:` の行を読む
- **THEN** ポートの ADR の行に `worktree-port-allocation-followup.md` と `prefer-collision-free-allocation-over-checks.md`、衛星の ADR の行に `foundation-for-satellites-premise.md`、純粋な crate・表面・lint の 3 件の ADR の行に `lint-purity-ui-refresh-plan.md` がある

#### Scenario: 出所の行が無い ADR は落ちる
- **WHEN** 検証の台本の自己検査が、使い捨ての木に写した ADR の 1 件から `- 出所:` の行を消して検査を走らせる
- **THEN** 検査は rc≠0 で終わり、その ADR のファイル名を出す

#### Scenario: メモリの残りを数える台本は、CH12 の 4 つのメモリを残りに数えない
- **WHEN** 検証の台本が `--backlog` で、メモリの置き場と `docs/adr/` を明示して数える台本を走らせる
- **THEN** 4 つのメモリ名のどれも残りの一覧に無く、台本は 4 つの名前ごとに「残りに無い」とだけ出して rc=0 で終わる

#### Scenario: 残りを数えた結果は、4 つのメモリ名の有無だけを出す
- **WHEN** 検証の台本が `--backlog` で走り終える
- **THEN** 標準出力と標準エラーに出るメモリの名前は 4 つのメモリ名だけで、メモリの説明（description）は出ない

#### Scenario: 数える台本かメモリの置き場が無ければ、通ったことにしない
- **WHEN** 検証の台本が `--backlog` で、数える台本が見つからないか、メモリの置き場のフォルダが無い
- **THEN** 台本は rc=127 で終わり、何が無いかを `error:` で始まる 1 行で出す（4 つのメモリ名の判定は出さない）

### Requirement: ADR は網の名前・私設 IP を含まない

`docs/adr/` のファイルは、手元の網の名前・私設 IP を含まない SHALL。確かめは repo の既存の検査（`tools/check-private.sh`）に任せ、ADR の側で例外を足さない SHALL（`tools/check-private.allow` に `docs/adr/` のパスを足さない）。

導出元: CH12

#### Scenario: docs/adr/ を含む追跡ファイルで、網の名前の検査が通る
- **WHEN** 検証の台本が `tools/check-private.sh` を走らせる
- **THEN** rc=0 で終わり、`tools/check-private.allow` に `docs/adr/` で始まる行が無い
