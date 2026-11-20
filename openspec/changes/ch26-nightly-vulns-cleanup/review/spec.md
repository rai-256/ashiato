# CH26 spec review（軽い路 `reason: same`）

2026-11-21。書いた者の意図を知らない席から、成果物どうしと実物との整合だけを見た。成果物には触れていない。

## 機械の検査（最初に写す）

| コマンド | rc | 結果 |
|---|---|---|
| `openspec validate ch26-nightly-vulns-cleanup --strict` | 0 | valid |
| `python3 scripts/check_chain.py .` | 0 | chain: OK（要件 174 / Story 60。この change は要件から来ないので観点 8 は無い） |
| `python3 scripts/check_scenarios.py . ch26-nightly-vulns-cleanup` | 1 | 担保の無い Scenario 7 件。全部この change のもの（dependency-vulnerability-scan 3・distribution 4）。印は実装の段（tasks 1.3・2.2・3.1）で置く予定で、3.2 が rc=0 を見る。上流の段では想定どおり |
| `python3 scripts/review_triage.py . ch26-nightly-vulns-cleanup` | 0 | 指摘 0 / 仮決め 0 |
| `python3 scripts/spec_overlap.py candidates . ch26-nightly-vulns-cleanup` | 0 | `[]`（候補 0 件） |

実物で確かめたこと（指摘にしないもの）: P3（`cargo tree -i rustix@1.1.5 -e normal --target all` は何も出さない）、P6 の行番号（`read.rs:121` `:164` `:231` `:253`。ほかに `:283` の `probe_readable` も写しを読み取り専用で開く）、P12（`crates/installer/build.rs:32-41`）、P14（`.github/workflows/ci.yml:544`）、`tools/release/windows-build-key.sh:436,496` が `pgsql.lock` を鍵に含む、`tools/release/changes.sh code` は `pgsql.lock` だけの差分でも rc=0（`package-windows` が走る）。
`cargo update -p rustix@1.1.5 --precise 1.1.6 --dry-run` は `Updating rustix v1.1.5 -> v1.1.6` だけを出す。`pgsql.sh fetch|slim|--self-test`・`check-vulns.sh --base|nightly`・`check-pg-minor.sh` の `PG_VERSIONS_URL`・`ashiato-server migrate`（`crates/server/src/main.rs:43`）は実在し、tasks の形で呼べる。
外の事実のうち、P1 の ID と別名・P2（rustix introduced 1.0.0 / fixed 1.1.6）・P4 の fixed 0.39.0・P5 の引用文・P8（chrono は fixed なし）は `https://api.osv.dev/v1/vulns/<ID>` で一致した（勧告の本文も「`rusqlite` 0.41 declares Rust 1.89」と書く）。P10（EDB・versions.json）と P15（Docker Hub）はこの席では確かめていない。P4 のうち rusqlite 0.41 → libsqlite3-sys `^0.39.0`・`rust_version = 1.89.0` は手元の crates.io の索引の写し（`~/.cargo/registry/index/.../.cache/ru/sq/rusqlite`）で一致した。

## 観点 1: deep の決定が正典に写っているか

軽い路なので deep.md は申告の写しだけ。申告（`lane.json` の `why`）の「minor の更新と戻しの確かめは docs/release.md §PostgreSQL の小さい版を上げる PR の手順で行う」が design D5 で覆されている → R2。
申告の「chrono・libsqlite3-sys・rustix は Cargo.toml / Cargo.lock の版を上げ」は、2 件を例外に回した。依頼の「上げられないものは例外の表に」の内なので指摘しない。ただし理由の事実の誤り → R4。

## R1. CH26.md に「壊してはいけないもの」と「完了の判定」が 2 つずつあり、ブリーフは後ろの空の雛形を拾う
- 成果物: docs/changes/CH26.md
- 根拠: `docs/changes/CH26.md:31` `:39`（埋めた節）と `:68` `:72`（`change_new.py` の雛形の「（上流が…書く）」が残っている）。`python3 -c 'import story_brief as b; dict(b.parse_sections(t))'` は `完了の判定` = `'（上流が spec の段で書く。確認バッチの問いと archive の判定になる）'`・`壊してはいけないもの` = `'（上流が深掘りの後に書く。…）'` を返す（`scripts/story_brief.py:298` が `dict(...)` で後ろの節を残す）。`stories_json.section` / `verify_checklist.section` は前の節を拾うので、読む道具によって中身が食い違う。tasks 4.2 の「「完了の判定」に書き足す」も、どちらの節か決まらない（検証の grep はファイルのどこにあっても通る）
- kind: technical
- 提案: 末尾の 2 つの雛形の節（`:68-74`）を消し、各見出しを 1 つにする。4.2 の検証は節を切り出して見る形にする
- 処置: fixed 4.2 —— docs/changes/CH26.md の末尾の雛形の 2 節を消して見出しを 1 つに。4.2 の検証は「## 完了の判定」の節を切り出し、節が 1 つであることも見る

## R2. D5 は依頼の「既に入っている PC で確かめる」と `docs/release.md` の手順を、Linux の置き場の入れ替えで代えている。インストーラの戻しの経路はそれでは覆えない
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（D5）・proposal.md・specs/distribution/spec.md・provisional.jsonl（D5）
- 根拠:
  - 依頼の逐語 `docs/changes/CH26.md:64`「既に入っている PC で minor の更新が既存の DB をそのまま使えることを確かめる」。`proposal.md:3` は依頼の「どこまで」を本人の決定として扱うと書いている
  - `docs/release.md:138-140`「変更前の小さい版を同梱した zip から更新と戻しの両方を通します。移行が失敗した場合に前の版へ戻ること…結果を PR に残してから変更を取り込みます」。軽い路の申告（`lane.json` の `why`・`deep.md:7`）は「この手順で行う」と言うが、`proposal.md:27` は手順を変えないと言いながら、この PR では手順を走らせない
  - D5 の「CI を変えるのは予定のパスの外」は、機械で走らせる道を CI の変更だけに絞っている。`tools/release/update-apply-install.sh:23` は `<正式旧> <正式新>` の zip を引数で取るので、base の lock（17.6-1）で組んだ zip と 17.12-1 の zip を Windows で 1 回走らせれば、repo のファイルに触れずに手順を通せる（どこで走らせるかは確かめていない）
  - Linux の入れ替えは、インストーラの戻しの経路を通らない。移行の前の退避は、置いた新しいプログラムの `pgsql\bin\pg_dump.exe` で取る（サーバの設定の `PG_DUMP` = `{program}/pgsql/bin/pg_dump.exe`。`crates/installer/src/secrets.rs:57`）。戻し（`crates/installer/src/main.rs:1672` の `rollback`）は、先にプログラムの置き場を前の版へ戻し（`:1747` の `restore_program_files`）、その後に `restore_database`（`:1762`）が `{program}\pgsql\bin\pg_restore.exe`（`:2115`・`commands.rs:855`）で退避を戻す。17.12 → 17.6 の戻しでは、17.12 の pg_dump の退避を 17.6 の pg_restore が読む。release.md が求める「移行が失敗した場合に前の版へ戻ること」はこの経路で、`minor-swap` のどの段も走らせない（`minor-swap` は同じ置き場を別の版で起こすだけで、退避と戻しを通らない）
  - 5.1(1) の代わりは条件つき（入っていなければ何も確かめない）で、R3 のとおり入っていても空振りしうる
- kind: premise
- 提案: まず、CI に触れずに base の 17.6-1 の zip から 17.12-1 の zip へ `update-apply-install.sh` を走らせる道（Windows の手元・使い捨ての Windows）を tasks に足せるか確かめ、足せればその Task で閉じる。足せないなら、本人の決定として扱う依頼の「どこまで」を狭めることになるので、premise として人間へ返す（「Linux の入れ替えと本人の PC の確かめで代えてよいか」を本人に問う。軽い路では escalated）
- 処置: escalated —— deep.md の R2・deep-questions-r1.json。あわせて D5 と specs に「17.12 の退避を 17.6 の pg_restore で戻す」Scenario、tasks 3.1 を足した

## R3. 5.1(1) の本人の PC での更新は、版が同じなら更新の手順に入らず、確かめが空振りする
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/tasks.md（5.1）
- 根拠: PR の配布物の版は `Cargo.toml` の出所の版（`tools/release/build.sh:15-21` の `resolve_version`。今は `0.1.1`。この change は版を上げない）。正典の `openspec/specs/distribution/spec.md:3667-3670`「入っている版と同じ版のインストーラを走らせる → 前の版の退け先は作られず…」。本人の配布版が同じ `0.1.1` なら更新の手順に入らない。そのときも「記録が見える」と答えられるが、17.12 への入れ替えは確かめていない。`git ls-remote --tags origin 'v*'` は空で、本人の PC にどの版が入っているかは成果物のどこにも書かれていない。
  (1) が確かめたいこと（小さい版の入れ替えで DB をそのまま使えること）は、作った記録で機械が再現できる（R2）。`realdata` が正当なのは、「本人の記録が更新の後も見える」ことだけ
- kind: technical
- 提案: (1) は、入っている版と配布物の版を問いに書かせ、同じなら「更新の手順に入らなかった」と答える選択を足すか、機械の確かめ（R2）が通った後の違和感の確認だけに絞る
- 処置: fixed 5.1 —— 入っている版と配布物の版を書かせ、同じなら「更新の手順に入らなかった」と答える選択を足した

## R4. D2 の「rusqlite 0.41 は Rust 1.89 を求める」は障壁にならない。反転条件の 3 つ目は既に成り立っている
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（D2）・premises.md（P4）・proposal.md・tasks.md（5.1(2)）・provisional.jsonl（D2 の cost）・docs/changes/CH26.md:21
- 根拠: repo に `rust-toolchain*` も `rust-version` も無い（`ls rust-toolchain*` は無し・`grep rust-version Cargo.toml crates/*/Cargo.toml` は 0 件）。CI は `dtolnay/rust-toolchain@… # stable`（`.github/workflows/ci.yml:112` ほか）、手元は `rustc 1.99.0`。索引の写しでは rusqlite 0.41.0 の `rust_version` は `1.89.0`。だから D2 の反転条件「リポジトリの Rust が 1.89 以上になり rusqlite 0.41 へ上げる change を立てられる。どれかに当たれば `届かない` をやめて上げる」（`design.md:33`）は書いた時点で既に成り立ち、D2 は自分の反転条件に当たっている。
  「上げると本体のコードの試験と API の追従が要る」（`design.md:23`）には premises の行が無い（0.32 → 0.41 で `read.rs` の使う API が変わるかは確かめていない）。上げられない理由として事実なのは、`rusqlite = "0.32"`（`crates/collector-windows/Cargo.toml:26`）の互換の範囲の外であることだけ
- kind: technical
- 提案: 上げない理由を「互換の範囲の外（0.32 → 0.41。`Cargo.toml` の範囲を替える）」にし、Rust 1.89 を理由・cost・反転条件・5.1(2) から外す。API の追従を理由に残すなら、0.32 → 0.41 の変更点を premises に足す
- 処置: fixed D2 仮 —— 上げない理由を「rusqlite = "0.32" の互換の範囲の外」だけにし、Rust 1.89 を理由・反転条件・proposal・P4・台帳の cost から外した

## R5. D2 の理由「開くのは本人のブラウザが書いた履歴の写しだけ」は、Firefox の絶対パスの profile では保証されない
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（D2）・premises.md（P6・P7）
- 根拠: `crates/collector-windows/src/history/locate.rs:208` は `IsRelative=` を読み、`IsRelative=0` の `Path=` を絶対パスとして辿る（試験 `locate.rs:359-379` は profile の置き場の外の `places.sqlite` を見つける）。P7 の「利用者のプロファイルの下を書けるのは本人と同じ利用者の権限の処理」は、絶対パスの置き場（共有のフォルダ・別のドライブ）では成り立たない場合がある。P7 は確かさ 1（知識）のまま、D2 はその上に乗っている（`design.md:79` も自分で認めている）。tasks 1.2 の確かめは `Connection::open*` の呼び所を読むだけで、開くファイルがどこから来るか（locate）は見ない
- kind: technical
- 提案: 理由と反転条件に「profile の置き場が本人以外に書ける場所（`IsRelative=0` で利用者の置き場の外）にある場合」を入れるか、tasks 1.2 の確かめに locate の絶対パスの扱いを足す。5.1(2) で本人に見せる理由も同じにそろえる
- 処置: fixed D2 仮 —— 理由と反転条件に Firefox の IsRelative=0 の profile を入れ、P16 を足し、tasks 1.2 に locate.rs の確かめを足した

## R6. 正典は「届かない」を「製品に届かない理由」と定めるが、D2 も D3 も製品に届く勧告をこの形で書く
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（D2・D3）
- 根拠: `openspec/specs/dependency-vulnerability-scan/spec.md:71`「理由は `届かない: ` か `上げる予定: ` で始める（前者は製品に届かない理由を短く、後者は上げる見込み）」。今の表の `届かない` は全部「組み立ての道具専用」「試験専用」（`osv-scanner.toml`）。D2 は「形は当たる」（`design.md:25`）と認めたうえで、攻撃する側が得るものが無いことを理由にする。D3 の chrono は 5 つの製品の crate が直に使い（P9）、理由「保守終了の知らせで、直す版も既知の欠陥も無い」は届かない理由ではない。検査（`届かない: ` の接頭辞だけを見る）は通るが、正典の文言とは食い違う
- kind: conflict
- 提案: D2・D3 に、正典の 2 つの型のどちらにも素直に当たらないこと、そのうえで `届かない` を選んだ理由を書き（仮の理由に足す）、確認バッチの (2) で本人に見せる。型を足すなら検査の台本と正典の変更で、この change の範囲（軽い路）の外
- 処置: fixed D2 仮 —— 正典の「製品に届かない」の型との違いと、それでも 届かない: を選ぶ理由を D2・D3 に書き、仮の決定として確認バッチで見せる

## R7. 「全体の検査と毎晩の検査は、この 3 件の勧告を出さない」は、osv.dev に届かないときに空で通る
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/dependency-vulnerability-scan/spec.md・tasks.md（1.3）・docs/changes/CH26.md（完了の判定 1）
- 根拠: tasks 1.3「出力と材料のファイルに 3 つの ID の文字が無いことを見る（rc は見ない）」。`tools/check-vulns.sh:17` によれば、網に失敗すると rc=2 と `error: 検査できなかった（…）` を出す。nightly は rc=2 なら材料を書かない（`:12`）。このとき出力にも材料にも ID は無く、Scenario は通る。一方 `CH26.md:41` の完了の判定 1 は「rc=0 で」と言うが、rc=0 を見るのは tasks 1.2 の全体の検査だけで、`nightly` を rc で見る検証はどこにも無い（依頼の「tools/check-vulns.sh nightly … が通り」）
- kind: technical
- 提案: Scenario の THEN に「どちらも rc≠2（検査できた）」を足す（rc=1 は許す、の意図は保てる）。`nightly <使い捨ての置き場>` の rc=0 を見る検証を 1.2 か 1.3 に足し、完了の判定 1 と合わせる
- 処置: fixed 1.2 —— nightly の rc=0 と材料が空の検証を足した。specs の THEN に rc が 2 でないこと、CH26.md 完了の判定 1 も合わせた

## R8. 時点の事実の Requirement が ADDED で正典に入る。期限が切れるか 17.13 が出ると偽になり、一度きりの台本なので緑のまま残る
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/dependency-vulnerability-scan/spec.md・specs/distribution/spec.md
- 根拠: 「`ignoreUntil` は台本を走らせた日より後で、その日から 90 日以内」（dependency-vulnerability-scan の Scenario 2）は、期限の日を過ぎると偽になる。design D3（`design.md:43`）はそのとき検査が落ちることを意図しているので、正典と設計の意図が食い違う。「`version` は `17.12-1`」も、次の小さい版を上げる change が来れば偽になる。印は CI に組まない台本（design D6・`design.md:75`）が持ち、archive の後も `check_scenarios.py` の担保として数えられ続けるので、偽になっても何も落ちない。さらに、軽い路は `## MODIFIED` / `## REMOVED` を書けないので、次の 17.13 の change は軽い路でこの Requirement を直せない。
  加えて distribution の 2 つ目の Requirement の本文（`specs/distribution/spec.md:22-23`）は、確かめ方（Linux で走らせる・`pg_dump` の行の除き方・確認バッチに回す）を Requirement に書いている。これは design / tasks に置く中身
- kind: technical
- 提案: 正典に残す Requirement を、時点に依らない形にする（例: 「同梱の PostgreSQL の大きい版は 17 で、lock の version と url と sha256 がそろう」「夜間の検査が挙げた勧告は、上げるか正典の形の例外の行で片付く」）。版と ID の事実は tasks と CH26.md の完了の判定に置く。どうしても残すなら、期限と版の時点を Requirement の文に書く。確かめ方の段落は design D5 へ移す
- 処置: fixed D5 —— Requirement を時点に依らない形にし、版・ID・期限の事実は CH26.md の完了の判定へ。確かめ方の段落は design D5 へ移した

## R9. Requirement の本文と「壊してはいけないもの」が言っていて、Scenario も検証も言っていないことがある
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/distribution/spec.md・specs/dependency-vulnerability-scan/spec.md・tasks.md
- 根拠（軽い路の `same` は、今の振る舞いが残ることを Scenario で固定する）:
  - `dlls` を「実行ファイルから辿れる DLL にそろえ」（`specs/distribution/spec.md:6`）: Scenario は無い。tasks 2.1 の検証（`fetch` → `slim`）は一覧の DLL が zip に在ることしか見ない。足りない DLL があっても通る（Windows で起動して初めて分かる）。調べ直しの結果を機械で照らす終了条件が無い
  - `sha256` は「その zip の値」（`:5`）: Scenario は 64 桁の 16 進であることだけを見る（`:12`）。照合は tasks 2.1 の `fetch` だけ
  - 「実行ファイルの一覧・DB の置き場の名前（`db\17`）は変えない」（`:7`）・壊してはいけないもの 2「許諾の文の置き場は変わらない」: Scenario は無い
  - 壊してはいけないもの 4「例外の表のほかの行を変えない」: Scenario も検証も無い。tasks 4.3 はファイルの名前しか見ない（CH24 は tasks 3.1 で base の行との一致を tomllib で見ている）
  - 壊してはいけないもの 5「rusqlite 0.32・libsqlite3-sys 0.30 は今のまま」・Requirement の「`Cargo.toml` の依存の範囲を変えない」（dependency-vulnerability-scan `:8`）: Scenario は無い。4.3 が `Cargo.toml` の差分を拾うのは間接的で、`Cargo.lock` の他の版の動きは R10 のとおり拾えない
- kind: technical
- 提案: `check-ch26.sh` に Scenario を足す（lock の `executables`・`license`・`license_destination` が base と同じ／`osv-scanner.toml` の base の行が全部同じ中身で残る／`Cargo.lock` の rustix 以外の `(name, version)` の組が base と同じ）。`dlls` は 17.12-1 の zip で `objdump -p` を辿った集まりと lock の一覧を比べる確かめを台本に置く
- 処置: fixed 2.1 —— dlls を objdump で辿った集まりと比べる pg-dlls、1.1 に Cargo.lock の組の比べ、1.3 に例外の表の既にある行が残ること、2.2 に lock の executables/license/大きい版が base と同じことの Scenario を足した

## R10. tasks 1.1 の検証は、言っていることを測っていない
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/tasks.md（1.1）
- 根拠:
  - 1 本目 `cargo tree -i rustix@1.1.5 >/dev/null 2>&1 && … || exit 0`: `cargo tree` がどんな理由で落ちても（索引に届かない・lock が壊れている）rc=0 になる
  - 2 本目は `^[-+]name = ` の行だけを見る。版だけが変わった項目は `name =` の行が差分の文脈行になり、引っかからない。実測: `git diff origin/main origin/feat/ch24-zip-upgrade -- Cargo.lock | grep -E '^[-+](name|version) = '` は `-version = "0.8.4"` / `+version = "0.9.4"` のように `name` の行を伴わない版の変化を出す。本文の「差分が rustix の 1 項目だけ」も、壊してはいけないもの 1 の「依存の木のほかの所は動かない」も、この検証では落ちない
- kind: technical
- 提案: 1 本目は `Cargo.lock` を読んで `rustix 1.1.5` が無いことを直に見る（R9 の Scenario と同じ読み方）。2 本目は base と HEAD の `Cargo.lock` を tomllib で読み、`(name, version)` の組の差が `rustix 1.1.5 → 1.1.6` だけであることを見る
- 処置: fixed 1.1 —— tomllib で base と今の (name, version) の組を比べる check-ch26.sh lock に替えた

## R11. tasks 2.2・3.1 の書き方の言い過ぎと抜け
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/tasks.md（2.2・3.1）
- 根拠:
  - 2.2「`PG_VERSIONS_URL=file://<写し>` で…（網に出ない）」: `tools/check-pg-minor.sh` は、最新の minor との比べ（`:103`）より前に、`:102` の `edb_status` で EDB へ `curl --head` を打つ（`PG_EDB_URL_BASE` が空なら lock の URL から作る）。落ちても `unknown` で rc に響かないが、網には出る。網に出さないなら `PG_EDB_URL_BASE=file://…` も要る（自己試験 `:161` はそうしている）
  - 3.1「`tools/stack.sh` と同じ環境の渡し方」: `stack.sh` は移行の前に `./tools/db-roles.sh` で役割を作り（`tools/stack.sh:101`）、退避を `MIGRATE_BACKUP=off` か `PG_DUMP`（容器の中の pg_dump）で決めている（`:105-106`。手元の pg_dump 16 は 17 の DB で落ちると注記がある）。`ashiato-server migrate` は既定で退避を取る（`crates/server/src/lib.rs` の `run_migrate`）。tasks はこの 2 つに触れていない
- kind: technical
- 提案: 2.2 は `PG_EDB_URL_BASE` も写しへ向けるか、「網に出ない」を消す。3.1 に「役割を `tools/db-roles.sh` と同じに作る」「`MIGRATE_BACKUP=off`（使い捨て）」を書く
- 処置: fixed 2.2 —— PG_EDB_URL_BASE も写しに向ける。3.1 に tools/db-roles.sh と MIGRATE_BACKUP=off を書いた

## R12. `minor-swap` の置き場は、インストーラと同じ initdb の形で作らないと、確かめているものが配る物の DB と違う
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/tasks.md（3.1）・design.md（D5）
- 根拠: インストーラは `--locale-provider=builtin --builtin-locale=C.UTF-8 --encoding=UTF8 --data-checksums` で initdb する（`crates/installer/src/db.rs:12-26`）。`postgres:*` の像の既定の initdb は libc の locale で、data checksums の指定も無い。像の OS の版が 17.6 と 17.12 の間で違えば、libc の照合順序の版の食い違い（配る物には起きない）が出うる。tasks 3.1 は initdb の形を決めていない
- kind: technical
- 提案: 3.1 に `POSTGRES_INITDB_ARGS` でインストーラと同じ引数を渡すことを書く（引数は `db.rs` の `initdb_args` から写したと台本の注記に書く）
- 処置: fixed 3.1 —— POSTGRES_INITDB_ARGS にインストーラと同じ引数（P18）

## R13. 1 つの Scenario が複数の主張を束ねている
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/distribution/spec.md・specs/dependency-vulnerability-scan/spec.md
- 根拠: 「lock の版と URL は 17.12-1 を指し、大きい版は 17」（`specs/distribution/spec.md:12`）は版・URL・大きい版・sha256 の形の 4 つ。「17.6 で移行を当てた置き場を、17.12 が同じ中身で使う」（`:28`）は起きる・中身が同じ・移行をもう一度当てても足すものが無い、の 3 つ（最後のものは「同じ中身で使う」とは別の主張で、題に無い）。「libsqlite3-sys と chrono の勧告は…」（dependency-vulnerability-scan `:17`）は 2 行の存在・接頭辞・期限の幅
- kind: technical
- 提案: 少なくとも「移行をもう一度当てても足す移行が無い」は別の Scenario に分ける。他も、題が言う主張だけを THEN に残す
- 処置: fixed 3.1 —— 「移行をもう一度当てても足す移行は無い」を分け、lock の Scenario を url・変わらないもの・dlls に、例外の表の Scenario を既にある行・足した行に分けた

## R14. #334〜#337 を閉じる者がいない
- 成果物: docs/changes/CH26.md（完了の判定 7）・tasks.md（Global Constraints）
- 根拠: `CH26.md:47`「merge の後に #334〜#337 を閉じる（夜間の検査が緑で走った後）」。tasks は「本物の `gh` で issue を閉じない」（`tasks.md:14`）と言い、閉じる Task も人間の確認待ちの項も無い。夜間の検査は閉じない（`tools/check-vulns-issues.py:166`「開いた issue は勧告が消えても閉じない」）。依頼（`CH26.md:65`）は「#334〜#337 を閉じる」を終わりの条件に入れている
- kind: technical
- 提案: 閉じるのがだれか（archive の段・本人）と、そのときに見るもの（merge の後の夜間の検査の run が緑）を、完了の判定 7 か人間の確認待ちに書く
- 処置: fixed tasks.md —— Global Constraints に「#334〜#337 は merge の後に本人が閉じる」、docs/changes/CH26.md の完了の判定 7 に、夜間の検査が緑なのを本人が見て閉じる確認バッチの問いを書いた

## R15. 5.1(2) は物理ではなく判断の問いで、仮の台帳と重なっている
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/tasks.md（5.1）
- 根拠: 5.1 の `> 物理: realdata（本人の PC に入っている配布版と、その DB の記録は、機械から見えない）`（`tasks.md:79`）は (1) の説明しか持たないが、同じ項に (2)「例外の表の 2 行の理由で受け入れてよいか」（`tasks.md:77-78`）が入っている。(2) は機械が再現できない物理ではなく、仮の決定（D2・D3）の受け入れの判断。D2・D3 は `provisional.jsonl:1-2` に載っており、仮は確認バッチで人間が見る（`CLAUDE.md` の「進め方の要点」）。`docs/testing.md:124-126` は、人間の確認待ちに置けるのは物理と名付けられるものだけとしている
- kind: technical
- 提案: (2) を人間の確認待ちから外し、仮の台帳（`provisional.jsonl` の D2・D3）の問いと理由を R4・R5・R6 の直しにそろえる。台帳から出る問いで足りないものがあれば、台帳の行の `question` / `keeps` に書く
- 処置: fixed 5.1 —— (2) を人間の確認待ちから外した。D2・D3 は仮の台帳から確認バッチの一覧に出る

## 軽い路の約束（通ったもの）

- specs は `## ADDED Requirements` だけで、`## MODIFIED` / `## REMOVED` / `## RENAMED` を持たない（2 ファイル）
- tasks の Global Constraints（`tasks.md:9-10`）と 4.3 の許すパスは `lane.json` の予定のパスの内。`docs/briefs/` `.superpowers/` とこの change の中は `scripts/lanes.py:136-137` が数えない
- design の（仮）は D2・D3・D5 の 3 つで、`provisional.jsonl` の 3 行と一致する
- A の扱い: R2 は人間へ返す側（premise）

## 観点 2: Scenario が検証可能か

R7（網に届かないと空で通る）・R8（日付と時点に依る）・R9（本文だけが言っている）・R13（束ね）。「同梱の版の遅れの検査が通る」は版の一覧の写しを使い、最新と同じ minor なら日付を見ずに rc=0 で返る（`tools/check-pg-minor.sh:103-105`）ので、日付に依って崩れない。

## 観点 3: 置き場の誤り

R8 の後段（確かめ方が Requirement の本文にある）。design の D 番号のうち観測できる振る舞いを決めているのは D5 の確かめの範囲だけで、これは spec 側にも Requirement として出ている。proposal の Capabilities（ADDED を足す 2 つ）と `specs/` のディレクトリ、`CH26.md` の front matter `capability` は一致する。

## 観点 4: tasks が検証を持つか

R10・R11・R9（2.1 の `dlls` の調べ直しに機械の終了条件が無い）・R1（4.2）・R3・R15（5.1）。順序（1.2 → 1.3 の nightly、2.1 → 2.2、3.1 → 3.2 → 4.x）は依存の順になっている。2.1 の「`VCRUNTIME140_1.dll` を取り込むなら手を止めて A で返す」は、予定のパスの外に出るための止め方としては筋が通っているが、人間が失うものは無く A の型には当たらない（昇格で足りる）。

## 観点 5: change の文書との一致

R1（最優先。ブリーフでは完了の判定と壊してはいけないものが空に見える）・R7（完了の判定 1 の rc）・R14（完了の判定 7）・R9（壊してはいけないもの 2・4・5）。依頼の「どこまで」は、互換の範囲・例外の規則・PR 本文の一覧（tasks 1.2 の `Ruling:`）・PostgreSQL の版と検査値（lock 以外に 17.6-1 と検査値を持つのは自己試験の写しだけ。`git grep -nE '17\.6-1|d378882a' -- ':!openspec/changes'` で確かめた）・CH24 との重なり（proposal の Impact）は写っている。勧告が当たる使い方の確かめは R5・R6、既存の DB の確かめは R2 が欠けを指す。

## 観点 6: 答えが写ったか

該当なし（`spec --answers` の呼び出しではない。軽い路で深掘りの答えが無い）。

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）。CH24（`ch24-zip-upgrade`）は `Cargo.lock` と `osv-scanner.toml` を触るが、ファイルの重なりで Requirement の重なりは無い。CH24 の `feat` の `Cargo.lock` は rustix 1.1.5 / tempfile 3.27.0 のままで、zip の 7 行は表の途中（`osv-scanner.toml:171-262` の範囲）、CH26 の 2 行は末尾。どちらの順でも git の 3-way で衝突しない見込み。
