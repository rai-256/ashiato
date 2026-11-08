# CH12 の前提

外のサービスの書き出しに依る決定は無い。依るのは、ADR の本文が述べるこの repo の事実と、GitHub の公開範囲・harness2 の台本の有無。
どれも手元で読んで確かめた（2026-11-08）。ADR の本文は依頼の逐語で、この change は書き換えない（食い違いは PR 本文で挙げる。Q1）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| この repo は公開（PUBLIC）で、ADR の全文は既に origin に出ている | 外に出る（`exported`）ことを問うか | `gh repo view --json visibility`・`git branch -r --contains ff9a78dd7` | PUBLIC。依頼の逐語を持つ `docs/changes/CH12.md` の commit は `origin/docs/ch12-upstream` にある。docs/adr/ に置いても新しく外に出るものは無い | 実物 | 5 |
| 完了の判定が使う `scripts/adr_backlog.py` と harness2 の ADR 24 件は、harness2 の main に無く、push もされず PR も無い（手元の worktree `harness2-adr` の枝 `docs/adr-backfill` にだけある） | 完了の判定をどう確かめるか・README と ADR からの参照の行き先（Q2） | `ls`・`git log --all -- scripts/adr_backlog.py`・`git branch -r --contains ae1a0b77f`（空）・`git ls-remote origin 'refs/heads/docs/adr*'`（空）・`gh pr list -R rai-256/harness2 --search adr`（0 件）（deep-review） | 依頼の「別の PR で置いた」は今は成り立たない。harness2 は PRIVATE | 実物 | 5 |
| adr_backlog.py は cwd のパスからメモリの置き場を決め、無ければ rc=0 で終わる | 検証が空の合格にならないか（Q2） | deep-review がこの worktree で走らせた | 「メモリが無い: …-ashiato2-up-ch12/memory」rc=0。`--memory …-ashiato2/memory` で rc=1・残り 19 件（ハーネスのメモリ） | 実物 | 4 |
| 出所に挙げたメモリの 4 ファイルは実在する | 「出所にメモリのファイル名がある」の意味 | `ls ~/.claude/projects/-home-yosis-dev-ashiato2/memory/` | foundation-for-satellites-premise.md・lint-purity-ui-refresh-plan.md・prefer-collision-free-allocation-over-checks.md・worktree-port-allocation-followup.md の 4 つとも有る | 実物 | 5 |
| 出所に挙げた repo の資料（`tools/ports.sh`・`docs/audit/2026-10-11-pure-crate-plan.md`・`Cargo.toml`・`web/eslint.config.js`）は実在する | 出所のリンク切れ | `ls`・`grep` | 全部有る | 実物 | 5 |
| `tools/ports.sh` は ADR の決定どおり（main 55432・`-stNN` 55500+NN・`-verify` 55499・ほかは台帳 `ashiato-ports`・尽きたら非 0）。`testdb.rs` はこの台本を呼ぶ | worktree のポートの ADR の「影響」 | `tools/ports.sh:6-20,52-58`・`crates/server/src/testdb.rs:65-84` | ADR のとおり。2026-10-21 の `-ch01`/`-ch02` の注記も `ports.sh:14` にある | 実物（コード） | 2 |
| `Cargo.toml` の pedantic は deny、外した文書の催促は 4 種（doc_markdown・missing_errors_doc・missing_panics_doc・must_use_candidate）、「まだ直していないだけ」の表がある | lint の ADR | `Cargo.toml:36-46` | ADR のとおり。ただし 1.99 の注記は Cargo.toml の中では「2026-10-11 実測」、ADR は「2026-10-12」。足した commit 51132b81e の日付は 2026-10-12 | 実物（コード） | 2 |
| Kotlin の detekt（formatting = ktlint）は baseline 付きで入っている | lint の ADR | `collector-android/app/build.gradle.kts:403-406`・`collector-android/config/detekt-baseline.xml` | 入っている | 実物（コード） | 2 |
| 分けた純粋な crate は main にまだ無い | 純粋 crate の ADR の「影響」 | `ls crates` | build-stamp・collector-windows・installer・server・winsvc。分けた crate は無い | 実物 | 5 |
| 衛星の ADR の「影響」（要件の書き直しは未着手・接続は未決）は、記録の時点の話で、その後に進んでいる | Q1 の食い違い | `docs/requirements.md:15-21,804-805` | 要件の目的は 2026-10-11 に ★ で書き直し済み。接続は FR-59 の ★ 2026-10-11 で「いったん MCP」。ADR は「メモリの記録の時点で」と断っているので矛盾ではないが、その後が書かれていない | 実物 | 5 |
| 表面の ADR の後、表面は 2026-10-12 に作り直された（骨格は変えていない） | Q1 の食い違い | `docs/ui-direction.md:119-122` | 「★ 2026-10-12 表面を作り直した（骨格は変えない。本人の決定）」。ADR に矛盾はしないが、その後が書かれていない | 実物 | 5 |

| ADR の事実のうち、ほかに食い違うもの（deep-review が見つけた） | Q1 の一覧 | deep-review が読んで・走らせた | `crates/winsvc/Cargo.toml` に `[lints] workspace = true` が無い。`crates/installer/src/commands.rs:107-117` が開発用ポートの範囲を写しで持つ。`tools/ports.sh` は `GIT_DIR=/nonexistent` で 55432・rc=0。衛星の監査は `docs/audit/2026-10-09-satellite-foundation.md` で済み。ST08・ST12・ST21 は archive 済みで `Cargo.toml:43-80` の表は残る | 実物 | 4 |

## 聞かずに決めた既定（C。spec の段が design の D に写す）

- C1. ADR 6 ファイルは依頼の逐語のまま置く（1 字も変えない）。逐語との一致を検証で機械が見る。比べる元は `docs/changes/CH12.md` の `## 依頼（逐語）` の引用（`> ` を外したもの）の「=== ファイル」の塊で、塊の終わりは次の「=== ファイル」か引用の終わり（上半分は深掘りの後に書き直されるので使わない）
- C2. 振る舞いを変えない（コード・台本・CI に触らない）。足すのは `docs/adr/` の 6 ファイルと `CLAUDE.md` の表の 1 行だけ
  - ★ 2026-11-08 spec の段で改めた: 検証用の台本 `tools/test-adr-docs.sh` を 1 本足す（spec の Scenario の印はコードの拡張子のファイルからしか拾われない）。CI には載せない（Q3）。既存の台本・コード・CI には触らない（design D2）
  - ★ 2026-11-08 第 1 回の答え（Q3・Q4。推奨と違う）で改めた: 台本の名前は `tools/check-adr.sh` にし、CI の chain job に 1 行で載せる（Q3）。`AGENTS.md` にも 1 行の指示を足す（Q4）。既存の台本・コードと、ci.yml のほかの段には触らない（design D2・D7）
- C3. 外に出ることは問わない。全文は既に公開の origin にある（上の表の 1 行目）
- C4. 検証の出力（evidence に写る）と PR 本文には、adr_backlog の出力をそのまま出さず、4 つのメモリ名が残りに有るか無いかだけを出す（公開されていないメモリの名前と説明を公開の repo に出さない）
