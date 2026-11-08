# CH12 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/ch12-memory-decisions-adr/deep-questions.json`（Q1〜Q4）・`premises.md`（C1〜C3）
入力: `docs/changes/CH12.md`（satisfies / requires / doors は空。change）・ADR の本文が述べる repo の事実・harness2 の `scripts/adr_backlog.py`（枝 `docs/adr-backfill` の ae1a0b77f）
確かめた日: 2026-11-08（worktree `ashiato2-up-ch12`、HEAD ff9a78dd7）

## 手順ごとの記録（一覧を見る前にやった分）

- **手順 1（要件どうしの衝突）**: satisfies は空。依頼の本文どうしで、「書き換えはしない（直さずに PR 本文で挙げる）」（CH12.md:17-18）と README の「決定が後で変わったら…置き換え済みにして新しい ADR を足す」は衝突しない。依頼と Q1 の 2 つ目の選択肢（ADR に追記する）は衝突する → R4
- **手順 2（扉の幅）**: 該当なし（change で doors 無し。確かめた範囲: `docs/changes/CH12.md` の front matter、`satisfies: []`）
- **手順 3（新たに立つ一方通行）**: ADR の本文は既に公開の origin にある（`gh repo view` → PUBLIC、`git ls-remote origin refs/heads/docs/ch12-upstream` → ff9a78dd7）。C3 は本文については正しい。ただし、検証の出力（evidence.jsonl の summary。commit される）と PR 本文を通って、**まだ公開されていないメモリの名前と説明**が外に出る経路がある → R5。列の型・保存形式・鍵は無い（文書だけ）
- **手順 4（日常に影響する選択）**: Q4（AGENTS.md の指示は agent の毎回の振る舞いを変える）だけ。一覧にある
- **手順 5（既存の repo が依頼を満たしていない箇所。grep と実行で確かめた）**:
  - `tools/check-private.sh`（追跡しているファイル全体）→ 「OK 網の名前・私設 IP は入っていない（2208 ファイル）」。ADR の本文と同じ文字列は既に CH12.md として木の中にある。tailscale の実際の名前（2 語）も CH12.md には 0 件（名前は出していない）
  - `tools/check-secrets.sh --range origin/main..HEAD` → rc=0
  - `docs/adr/` を足すと落ちる検査は無い: `check-spdx.sh` は SPDX の印を持つファイルしか見ない。`check-shared-appends.sh` が見るのは smoke/immutable/seed の 3 本だけ。`check-tmp-hygiene.sh` は crates が対象。harness2 の `req_impact.py:46-49` は `docs/**/*.md` を走査するが、ADR の本文には FR/扉の ID が無い。`init.py` は CLAUDE.md の `<!-- harness2:… -->` の印しか見ないので、表に行を足しても影響しない
  - 完了の判定の台本（adr_backlog.py）は、書いてあるまま走らせると成り立たない → R1
  - ADR の本文と repo の今の食い違いで premises / Q1 に無いもの → R3
- **手順 6（過去の取りこぼし）**: `upstream_misses.py report` の観点のうち、当てはまるのは 3 つ。「検証コマンドが成立しない／0 本でも通る」→ R1（当てはまり、一覧に無い）。「X を後のために残すなら、後でどう見つけるかと置き場の寿命」→ R4（Q1 の推奨は PR 本文に残して「後の ADR の手入れ」で直すと言うが、見つけ方が無い）。「答えの前に main で変わった★と突き合わせる」→ Q1 が一部やっている（R3 で残りを足す）。時計・保存場所・ACL・provisional の rests_on 系の観点は当てはまらない（文書だけで、台帳の行もまだ無い）
- **手順 7（族の網羅）**: Q1 の族「ADR が述べる事実と repo の今」は、兄弟 5 つが抜けている → R3。Q2 の族「まだ main に無い harness2 の物に依る」は、台本（Q2）のほかに README と ADR から harness2 の ADR への参照・Q4 の why の前提という兄弟がある → R2
- **手順 8（前提の確かさ）**: loss のある決定は一覧に無いので、確かさ 4 未満の前提に乗った loss は無い。ただし、確かさ 2（コードを読んだだけ）の行（`ports.sh`・`Cargo.toml`）を根拠に、Q1 の context が「ほかの事実は repo と一致した」と言い切っている。走らせると 2 か所が食い違った → R3。`tools/ports.sh` をこの worktree で走らせた結果は DB=55609 / HTTP=19218、rc=0（台帳の経路は ADR のとおり）。premises の 2 行目は「枝にある」とだけ書いていて、push されていないこと・PR が無いことを落としている → R2

## R1. 完了の判定の台本は、worktree で書いてあるまま走らせるとメモリを見つけられず rc=0 で通る（空の合格）
- 成果物: openspec/changes/ch12-memory-decisions-adr/deep-questions.json（Q2）・premises.md
- 根拠: harness2 ae1a0b77f:scripts/adr_backlog.py の `memory_dir_for(repo)`（cwd のパスを `-` でつないだメモリを見る）と `if not mem.is_dir(): … return 0`。この worktree で `python3 adr_backlog.py --adr docs/adr` → 「メモリが無い: /home/yosis/.claude/projects/-home-yosis-dev-ashiato2-up-ch12/memory」rc=0。`ls -d ~/.claude/projects/-home-yosis-dev-ashiato2*/memory` → 有るのは `-home-yosis-dev-ashiato2` だけ。`--memory …-ashiato2/memory --adr docs/changes` で走らせると rc=1・残り 19 件（すべてハーネスのメモリ。harness2 の ADR が main に入るまで rc は 1 のまま）。CI の runner には `~/.claude` が無い
- kind: technical
- 提案: Q2 の context と推奨の選択肢に、(a) `--memory ~/.claude/projects/-home-yosis-dev-ashiato2/memory` を必ず渡す、(b) rc ではなく「4 つのメモリ名が出力に無いこと」で判定する、(c) メモリの置き場が無いときは PASS でなく BLOCKED にする、を書く。この検証は手元でしか走らないことも context に書く

- 処置: fixed: Q2 の context と推奨の選択肢に、メモリの置き場の明示・名前での判定・置き場が無ければ通さない・手元でしか走らないことを書いた。premises に 1 行足した
## R2. 「harness2 の docs/adr/ に別の PR で置いた」は今は成り立たない（push されていない・PR が無い）。premises に無く、Q2 の選択肢 2 は待つ相手が無い
- 成果物: openspec/changes/ch12-memory-decisions-adr/premises.md・deep-questions.json（Q2・Q4）
- 根拠: docs/changes/CH12.md:15。`git -C ~/dev/harness2 fetch && git branch -r --contains ae1a0b77f` → 空。`git ls-remote origin 'refs/heads/docs/adr*'` → 空。`gh pr list -R rai-256/harness2 --state all --search adr` → `[]`。harness2 の ADR 24 件は、ローカルの worktree `/home/yosis/dev/harness2-adr`（docs/adr-backfill）にだけある。harness2 は PRIVATE（`gh repo view rai-256/harness2`）。ADR の本文は、harness2 の ADR を題名で 3 か所参照している（CH12.md の ports の影響・純粋 crate の影響・lint の影響）。README も「harness2 の docs/adr/ にある」と書く。Q4 の why は「adr_backlog の書いた会話のうちに ADR を促す」に頼っているが、これも同じ未 merge の commit にある
- kind: premise
- 提案: premises の 2 行目に「push されていない・PR は無い（確かめ方: ls-remote・gh pr list）」を足す。Q2 の選択肢 2 は「harness2 の ADR を push して PR にし、merge を待つ」と書き直す。族の兄弟として、README と ADR からの harness2 の参照が merge の時点で行き先を持たないことを、Q2 か新しい B の問い（推奨つき。例: 待たずに merge し、PR 本文に書く）で扱う。harness2 の 24 件がローカルにしか無いことは、この change の外の話として呼び出し元へ伝える

- 処置: fixed: premises の 2 行目を直し（push・PR 無し・PRIVATE）、Q2 の問いと選択肢 2 を「push して PR にし merge を待つ」に書き直した。参照の行き先が無いことは Q1 の context と Q2 の推奨（PR 本文に書く）で扱う。harness2 の 24 件がローカルにしか無いことは人間へ伝える
## R3. ADR の本文と repo の今の食い違いで、Q1 の 3 つに入っていないものが 5 つある。Q1 の「ほかの事実は repo と一致した」は誤り
- 成果物: openspec/changes/ch12-memory-decisions-adr/deep-questions.json（Q1 の context）・premises.md
- 根拠:
  1. lint の ADR「Rust: pedantic」（CH12.md:236）に対して、`crates/winsvc/Cargo.toml` には `[lints] workspace = true` が無い（ほかの 4 crate にはある。`grep -A1 '^\[lints\]' crates/*/Cargo.toml`）。winsvc の crate は pedantic も `unsafe_code = "forbid"` も受けていない（`crates/winsvc/src/lib.rs:3`「サーバ本体の lint 境界を越えない」、2026-11-06 の 19c44ef28）
  2. ports の ADR「規則はここだけに置く」（CH12.md:113）に対して、`crates/installer/src/commands.rs:107-117` が開発用の番号の範囲（55432・55499・55500..=55599・55600..=55700・19000..・19200..）を写しで持っている（`lib.rs:683` の試験）
  3. ports の ADR「割り当てられなければ非 0、55432 には落とさない」（CH12.md:115）に対して、`tools/ports.sh:46,51-53` は git が答えないと 55432 に落ちる。実測: この worktree で `GIT_DIR=/nonexistent bash -c '. tools/ports.sh && echo $ASHIATO_DB_PORT'` → 55432、rc=0
  4. 衛星の ADR の「進め方 1. 監査」は既に済んでいる（`docs/audit/2026-10-09-satellite-foundation.md` があり、`docs/requirements.md:17` などの★がそれを引く）。Q1 の (1) は要件の書き直しだけを挙げている
  5. 純粋 crate と lint の ADR の「ST12・ST21（・ST08）の merge 後に行う」の条件は満たされている（archive の `2026-10-13-st12-…`・`2026-10-13-st21-…`・`2026-10-17-st08-…`）。それでも `Cargo.toml:43-80` の「まだ直していないだけ」の表は残っている。純粋 crate の ADR は「まだ無い」と断っているが、lint の ADR は今の状態を書いていない
- kind: premise
- 提案: Q1 の context（と PR 本文に挙げる一覧）に 1〜5 を足し、「ほかの事実は一致した」を消す。premises の `ports.sh`・`Cargo.toml` の行は「読んだ」から「走らせた」に上げるか、確かさ 2 のまま食い違いを書く

- 処置: fixed: 5 つを Q1 の context の一覧と premises に足し、「ほかは一致」の文を消した
## R4. Q1 は依頼が既に決めている（不要）。問うべきなのは、食い違いの一覧を後で見つける置き場
- 成果物: openspec/changes/ch12-memory-decisions-adr/deep-questions.json（Q1）
- 根拠: docs/changes/CH12.md:17-18「書き換えはしない（事実と食い違う箇所を見つけたら、直さずに PR 本文で挙げる）」。Q1 の選択肢 2（ADR に追記する）は、この文に反する。Q1 の why は自分で「PR 本文は流れていく」と書き、推奨の detail は「追記は後の ADR の手入れで行える」と書くが、手入れのきっかけ（issue・change・台帳）がどこにも無い。upstream_misses の観点「X を後のために残すなら、後でどう見つけるかと置き場の寿命を context に書く」に当たる。adr_backlog の docstring は「判断の正本は ADR」と言うので、古い「影響」（例: 接続は未決）を読んだ agent が、今の状態を取り違えうる
- kind: defer
- 提案: Q1 は削り、C として「逐語のまま置き、R3 を含む食い違いを PR 本文に挙げる」とする。代わりに B の問い（推奨つき）を 1 つ置く:「食い違いの一覧を PR 本文のほかにどこへ残すか（issue／ADR の手入れの change を `hx change` で起こす／残さない）」

- 処置: fixed: Q1 を「食い違いの一覧を PR 本文のほかにどこへ残すか」に置き換えた（推奨: docs/changes/CH12.md の節）
## R5. C3「外に出ることは問わない」は ADR の本文だけを見ている。検証の出力と PR 本文を通って、公開されていないメモリの名前と説明が外に出る
- 成果物: openspec/changes/ch12-memory-decisions-adr/premises.md（C3）・deep-questions.json（Q2）
- 根拠: Q2 の推奨は adr_backlog.py を走らせ、その結果を PR 本文に書く。adr_backlog.py の出力は、残っているメモリ 1 件ごとに「ファイル名（type）: description」の 1 行を出す（実測 19 行と合計 1 行で計 20 行）。`.harness2/scripts/evidence.py:447` は出力の末尾 20 行を `summary` として evidence.jsonl に入れ、evidence.jsonl は commit される（例: `openspec/changes/archive/2026-11-07-ch09-load-tolerant-timing-tests/evidence.jsonl` は追跡されている）。repo は PUBLIC。今の 19 行には `check-private` の型（`.ts.net`・私設 IP）・ホームのパス・tailscale の語は 0 件。ただしメモリの中身は今後も変わる
- kind: irreversible
- loss: exported
- 提案: 失わない側は高くつかない。だから問いにはせず、C に 1 行足す:「完了の判定の検証は、4 つのメモリ名が残りに無いかどうかだけを出す（`--json` を絞り込む）。adr_backlog の一覧は evidence にも PR 本文にも載せない」。C3 の根拠の欄に、本文以外の出口（evidence・PR 本文）も見たことを書く

- 処置: fixed: premises に C4（出力は 4 つの名前の有無だけ）を足した
## R6. C1 の「逐語との一致を機械で見る」は、比べる元と最後の塊の終わりを決めていない。深掘りの後に CH12.md の上半分は書き直される
- 成果物: openspec/changes/ch12-memory-decisions-adr/premises.md（C1）
- 根拠: docs/changes/CH12.md:255「上流の深掘りの後に、何のために変えるかを 1〜3 文に整える」。整えると、`## 価値` の下にある素のままの写し（CH12.md:28-253）は消えうる。残るのは `## 依頼（逐語）` の引用（`> ` 付き。CH12.md:257 以降）だけになる。最後の塊（stricter-lint）は、次の `=== ファイル` を区切りにすると、その後ろの注記と引用の節まで取り込む。実測: 2 つの写しを、上は CH12.md:255 の注記の手前で切り、下は `> ` / `>` を外して比べた。6 つの塊はすべて一致した（README 1122 字ほか）
- kind: technical
- 提案: C1 に「比べる元は `## 依頼（逐語）` の引用で、行頭の `> ` と `>` を外す。塊は `=== ファイル: … ===` から、次の印か引用の終わりまで。末尾の改行は 1 つにそろえる」と書く
- 処置: fixed: C1 に比べる元（`## 依頼（逐語）` の引用）と塊の終わりを書いた

## 一覧の問いのうち、指摘しなかったもの

- Q3・Q4: どちらも loss が無い B で、推奨もある。依頼が挙げた置き方の外に出る選択肢だが、依頼は禁じていないので幅はある。kind も合っている
- 分類違い・不可逆の根拠が無いもの・文字で決めさせている画面の問い・既定の無い仮の問い: 該当なし（4 問とも `open`、loss 無し、`recommended` あり、画面の構造を問うものは無い）
