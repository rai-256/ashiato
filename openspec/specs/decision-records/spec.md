# decision-records Specification

## Purpose
Story をまたぐ判断（製品の前提・コードの構成・道具の規則）を、commit されないメモリではなく repo の `docs/adr/` に 1 件 1 ファイルで残し、
索引から辿れ、どこから移したかを名指しできるようにする。開発の約束で、製品の振る舞いではない。

## Requirements

### Requirement: ADR は docs/adr/ に 1 件 1 ファイルで置き、README の一覧から辿れる

`docs/adr/` は README（`README.md`。ADR の書き方と一覧）と ADR のファイルを持つ SHALL。ADR のファイル名は `YYYYMMDDHHMM-<英小文字・数字・ハイフンの slug>.md` の形である SHALL。
README の「一覧（新しい順）」の項目は `- [<日付> <題>](<ファイル名>)` の形で、一覧が名指すファイルと `docs/adr/` の README 以外の `.md` は 1 対 1 で対応する SHALL。
各項目の日付と題は、名指す ADR の `- 日付:` の値と 1 行目の `# ` の見出しに一致し、項目はファイル名の降順（新しい順）に並ぶ SHALL。
各 ADR のファイル名の先頭の日付（`YYYYMMDD`）は、その ADR の `- 日付:` の値に一致する SHALL（README の「時刻は判断が決まった日」）。`- 状態: 置き換え済み（→ <後継のファイル名>）` の後継は `docs/adr/` にある ADR のファイルである SHALL。
各 ADR は、1 行目に `# <題>` を持ち、`- 日付: YYYY-MM-DD`・`- 状態: 採用` か `- 状態: 置き換え済み（→ <後継のファイル名>）`・`- 出所: …` の行をそれぞれ 1 つ持ち、
`## 背景`・`## 決定`・`## 退けた案`・`## 反転の条件`・`## 影響` の見出しをそれぞれ 1 つ、この順に持つ SHALL（README の「形」のとおり）。
`CLAUDE.md` の「文書とハーネス」の表は `docs/adr/` の行を持ち、何かの欄に「Story をまたぐ判断の記録」、書き込みの欄に「する」と書く SHALL。
`AGENTS.md` の「実装と計画の規約」の節は、Story をまたぐ判断を `docs/adr/` に書くよう指示する行を 1 つ持つ SHALL。

導出元: CH12

#### Scenario: README の一覧と docs/adr/ の ADR のファイルが 1 対 1 で対応する
- **WHEN** 検証の台本が README の一覧の項目と `docs/adr/` の README 以外の `.md` を比べる
- **THEN** 一覧にあって実ファイルが無いもの・実ファイルがあって一覧に無いものが、どちらも 0 件である

#### Scenario: 一覧の日付と題は ADR の日付と見出しに一致する
- **WHEN** 検証の台本が一覧の各項目を、名指す ADR の `- 日付:` の行と 1 行目の `# ` の見出しと比べる
- **THEN** すべての項目で、日付と題が一致する

#### Scenario: 一覧はファイル名の新しい順に並ぶ
- **WHEN** 検証の台本が一覧の項目の並びを、名指すファイル名と比べる
- **THEN** 項目はファイル名の降順に並ぶ

#### Scenario: ADR のファイル名は時刻と英小文字の slug の形を持つ
- **WHEN** 検証の台本が `docs/adr/` の README 以外の `.md` の名前を見る
- **THEN** どれも `^[0-9]{12}-[a-z0-9]+(-[a-z0-9]+)*\.md$` に当たる

#### Scenario: ADR のファイル名の日付は ADR の日付に一致する
- **WHEN** 検証の台本が `docs/adr/` の README 以外の `.md` の名前の先頭 8 桁と、その ADR の `- 日付:` の値を比べる
- **THEN** どの ADR でも、名前の `YYYYMMDD` を `YYYY-MM-DD` にしたものが `- 日付:` の値と一致する

#### Scenario: 置き換え済みの ADR が指す後継は docs/adr/ にある
- **WHEN** 検証の台本が `- 状態: 置き換え済み（→ X）` の行を持つ ADR を読む
- **THEN** どの X も `docs/adr/` にある README 以外のファイルの名前である

#### Scenario: CH12 で置いた README と 5 件の ADR がある
- **WHEN** 検証の台本が `docs/adr/` を見る
- **THEN** `README.md`・`202610030000-worktree-port-allocation.md`・`202610080000-foundation-for-satellites.md`・`202610110000-pure-crate-split.md`・`202610110001-ui-surface-only.md`・`202610110002-stricter-lint.md` がある

#### Scenario: CLAUDE.md の文書の表から docs/adr/ に辿れる
- **WHEN** 検証の台本が `CLAUDE.md` の「文書とハーネス」の表を読む
- **THEN** 1 列目が `` `docs/adr/` `` で、2 列目が「Story をまたぐ判断の記録」を含み、3 列目が「する」の行がちょうど 1 つある

#### Scenario: ADR は決まった項目と見出しをこの順に持つ
- **WHEN** 検証の台本が `docs/adr/` の README 以外の `.md` の行を読む
- **THEN** どの ADR も 1 行目が `# ` で始まり、`- 日付:`・`- 状態:`・`- 出所:` の行がそれぞれちょうど 1 つ、`## 背景`・`## 決定`・`## 退けた案`・`## 反転の条件`・`## 影響` の見出しがそれぞれちょうど 1 つ、この順にある

#### Scenario: AGENTS.md の規約から docs/adr/ に辿れる
- **WHEN** 検証の台本が `AGENTS.md` の「実装と計画の規約」の節を読む
- **THEN** `` `docs/adr/` `` と「Story をまたぐ判断」の両方を含む行がちょうど 1 つある

### Requirement: 各 ADR は出所を持ち、メモリから移した判断はメモリのファイル名を名指しする

各 ADR は `- 出所:` の行を持つ SHALL。メモリから移した判断の ADR は、その行にメモリのファイル名（`<名前>.md`）を書く SHALL。
CH12 で移した 4 つのメモリ（`worktree-port-allocation-followup.md`・`prefer-collision-free-allocation-over-checks.md`・`foundation-for-satellites-premise.md`・`lint-purity-ui-refresh-plan.md`）は、
それぞれ移した先の ADR の出所に名指しされる SHALL。

導出元: CH12

#### Scenario: 5 件の ADR の出所に、移したメモリのファイル名がある
- **WHEN** 検証の台本が 5 件の ADR の `- 出所:` の行を読む
- **THEN** ポートの ADR の行に `worktree-port-allocation-followup.md` と `prefer-collision-free-allocation-over-checks.md`、衛星の ADR の行に `foundation-for-satellites-premise.md`、純粋な crate・表面・lint の 3 件の ADR の行に `lint-purity-ui-refresh-plan.md` がある

#### Scenario: 出所の行が無い ADR は落ちる
- **WHEN** 検証の台本の自己検査が、使い捨ての木に写した ADR の 1 件から `- 出所:` の行を消して検査を走らせる
- **THEN** 検査は rc≠0 で終わり、その ADR のファイル名を出す

### Requirement: ADR の形は CI で PR ごとに確かめる

CI の chain job（出し分けの対象外で、どの PR でも走る段）は、ADR の形の検査（`tools/check-adr.sh`）の自己検査と本番を走らせる SHALL。
本番が見るのは、この capability の Requirement のうち、CH12 の受け入れの 1 回だけの判定（依頼の逐語との一致・harness2 の台本でのメモリの残り）を除いた全部である SHALL。
形から外れた ADR・一覧の書き忘れを含む PR は、この段で落ちる SHALL。

導出元: CH12

#### Scenario: CI の chain job が ADR の形の検査を走らせる
- **WHEN** 検証の台本が `.github/workflows/ci.yml` の chain job の段を読む
- **THEN** `- run: ./tools/check-adr.sh --self-test && ./tools/check-adr.sh` の段がちょうど 1 つある

#### Scenario: 決まった見出しの欠けた ADR は落ちる
- **WHEN** 検証の台本の自己検査が、使い捨ての木に写した ADR の 1 件から `## 決定` の見出しを消して検査を走らせる
- **THEN** 検査は rc≠0 で終わり、その ADR のファイル名を出す

### Requirement: ADR は網の名前・私設 IP を含まない

`docs/adr/` のファイルは、手元の網の名前・私設 IP を含まない SHALL。確かめは repo の既存の検査（`tools/check-private.sh`）に任せ、ADR の側で例外を足さない SHALL（`tools/check-private.allow` に `docs/adr/` のパスを足さない）。
既存の検査は追跡しているファイルしか見ないので、`docs/adr/` のファイルは追跡されている SHALL。

導出元: CH12

#### Scenario: docs/adr/ を含む追跡ファイルで、網の名前の検査が通る
- **WHEN** 検証の台本が `tools/check-private.sh` を走らせる
- **THEN** `git ls-files docs/adr` が README と 5 件の ADR を返し、`tools/check-private.sh` が rc=0 で終わり、`tools/check-private.allow` に `docs/adr/` で始まる行が無い
