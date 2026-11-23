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

2026-11-23 追記: ST61（`st61-auto-version-bump`、上流の枝 `origin/docs/st61-upstream`）の上流が走り始めて、`spec_overlap.py candidates` の候補が 1 件になった（名指し・相手 MODIFIED・capability distribution）。
- st61-auto-version-bump / 同梱の PostgreSQL の大きい版が違う配布物では、何も替えずに止まる: 独立（ST61 の MODIFIED は Requirement の本文 2 文を変えず、★ の注記を足して Scenario の WHEN の版 `0.4.0` を `0.0.2` に替えるだけ（st61 の specs/distribution/spec.md:868-881 と正典 openspec/specs/distribution/spec.md:3979-3992 の diff）。CH26 が依るのは本文の 2 文目「小さい版は置き場と一緒に入れ替え・戻す」（design.md:140）と、CI の古い側・新しい側の 2 つの zip が在ることだけで、版の値 `0.4.0`/`0.5.0` は design.md:78 の括弧の中にあるだけで specs/tasks の Scenario・検証は持たない（`grep -n "0\.4\.0\|0\.5\.0" specs tasks.md` が 0 件）ので、どちらの順で入っても意味が変わらない。）

## 第 2 回（合意の違和感の直し、2026-11-22）

第 1 回の合意で本人が R2 を A → C に変えた違和感の直し（`git diff 2d410fee0 91d7d8b17`）を、書いた者の意図を知らない席から見た。成果物には触れていない。通常の路へ昇格済み（`lane-verdict.json` の `promoted`）なので、軽い路の約束（予定のパス・ADDED のみ）では見ていない。

### 機械の検査（最初に写す）

| コマンド | rc | 結果 |
|---|---|---|
| `openspec validate ch26-nightly-vulns-cleanup --strict` | 0 | valid |
| `python3 scripts/check_chain.py .` | 0 | chain: OK（この change は要件から来ないので観点 8 は無い） |
| `python3 scripts/check_scenarios.py . ch26-nightly-vulns-cleanup` | 1 | 担保の無い Scenario 19 件。全部この change のもの（dependency-vulnerability-scan 4・distribution 15）で、実装の前なので想定どおり。ただし 1 件は tasks のとおりに印を置いても拾われない → R17 |
| `python3 scripts/review_triage.py . ch26-nightly-vulns-cleanup` | 0 | 指摘 15 件 / 仮決め 4 件 / 要件へ戻すもの 0 件（第 1 回の分） |
| `python3 scripts/spec_overlap.py candidates . ch26-nightly-vulns-cleanup` | 0 | `[]`（候補 0 件） |
| `python3 scripts/evidence.py lint openspec/changes/ch26-nightly-vulns-cleanup` | 0 | `[OK] 検証コマンド` |

実物で確かめたこと（指摘にしないもの）:
- `printf 'tools/release/pgsql.lock\n' | tools/release/changes.sh code` は rc=0 を返す。`windows` の判定（`ci.yml:59` の `^tools/release/`）も真なので、lock だけを変えた PR でも `package-windows` と `install-windows` は走る
- 鍵の step の前に `decide` の step を置き、`export WINDOWS_BUILD_KEY_TOOLS=…|ci.yml=$blob|prev-pgsql=…` にした ci.yml の写しで、`tools/release/windows-build-key.sh --check-workflow` は rc=0 を返す。D9 の形は今の検査のまま足せる
- `crates/installer/build.rs:32-41` は lock の大きい版だけを読み、組み立ての印の入力（`INPUTS`）は lock を含まない。`build.sh:141` `:145` は作業木の lock から許諾の文の `version:` と DLL の一覧を書き、`pgsql.sh:5` も同じファイルを読む。古い側を組む間だけ作業木の lock を置き換えれば、zip の中身・許諾の文・大きい版の読み先はそろう（P19）。`pgsql.sh fetch` は版ごとの名前で置く（`:24`）
- `THIRD-PARTY-NOTICES.txt` は zip の根に置かれる（`build.sh:114`・`check-artifacts.sh:207`）

### 観点 1: deep の決定が正典に写っているか

`deep.md:25` の補足（違和感）を項目ごとに当てた:

| 補足の項目 | 写った先 | 判定 |
|---|---|---|
| R2 を A → C に変える | design D5（`design.md:65-83`。D6 の前まで）・proposal・CH26.md:23・`deep.md:19` | 写った。A の名残（「本人の PC で見る」「CI は変えない」）は specs・tasks に残っていない |
| lock を変える PR だけ、test-old を変更前の lock で組む | specs の新しい Requirement（`specs/distribution/spec.md:50-57`）・D7（仮）・tasks 4.2 | 写った。ただし「lock を変える PR」の判定と、base が取れないときの扱いに穴がある → R16・R19 |
| 既存の `update-apply-install.sh` で 17.6 → 17.12 の更新と戻しを Windows の runner で通す | Scenario「lock を変えた PR では…」・CH26.md の完了の判定 8 | 写った。ただし更新した後・戻した後に動いている PostgreSQL の版は、どこも見ていない → R22 |
| docs/release.md の手順を CI で機械が行う | D8（仮）・tasks 4.3 | 写った |
| lock を変えない PR の組み立てと試験は今のまま | Requirement の本文・D7（仮）・CH26.md の壊してはいけないもの 4 | 本文には写った。Scenario は `decide` の出力だけを見る（R23）。Windows の checkout で常に `swap` になりうる（R26） |
| tasks 5.1 を削る | tasks.md（5.1 は check_scenarios に置き換わった）・「人間の確認待ち」の節も無い（check_scenarios は名無しの確認待ち 0） | 写った |
| 本人の PC の移行は ST64 | design の Non-Goals・proposal の「変えないもの」 | 写った。ST64 は origin/main の `docs/stories/INDEX.md:88` にある。この枝の木にはまだ無い（R27） |

本人の決定に数値は無い。版の組 `17.6-1 → 17.12-1` は Scenario と完了の判定 8 にある。

## R16. base の lock が取れないと `same` に倒れ、版の確かめも同じ `same` から期待値を作るので、lock を変えた PR が 17.12 → 17.12 のまま緑になる
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（D7（仮）・D8（仮））・tasks.md 4.2・specs/distribution/spec.md・docs/changes/CH26.md（完了の判定 8）
- 根拠:
  - D7（`design.md:95`）は「base の lock が取れない run は入れ替えない。取れないときは warning を出す（lock を変える PR で取れないなら、下の D8 の確かめが落とす）」と書く
  - D8（`design.md:108`）と tasks 4.2（`tasks.md:85`）の版の確かめの期待値は「`swap` なら前の lock の版、でなければ今の lock の版」で、`decide` の step の出力から作る。tasks 4.2（`tasks.md:82`）は取れないときに「warning を出して `same`」を出す。すると古い側の期待値は今の lock の版（17.12-1）になり、17.12-1 の zip で一致して通る。D7 の「D8 の確かめが落とす」は成り立たない
  - 同じ run の鍵は `prev-pgsql=none` になるので、後の run も `same` で組んだ組み立てを引きうる（D9）
  - 完了の判定 8（`CH26.md:48`）の「`古い側 17.6-1 → 新しい側 17.12-1` を出して通り」は CI の出力の 1 行で、merge gate は CI が緑かどうかしか見ない。この 1 行を機械で見る Task も Scenario も無い
  - 「base が取れないとき」の扱いは design と tasks にだけあり、specs の Requirement にも Scenario にも無い（観点 3: 観測できる振る舞いが design にだけある）
- kind: technical
- 提案: 「この PR が lock を変えたか」を `decide` と別の材料で決める。`changes` の job（`fetch-depth: 0`・`ci.yml:32`）が、PR の run でだけ、PR 自身の差分（merge-base から head の 3 点の `git diff`。今の `files` は 2 点の `ci.yml:43` なので別に取る。R19）で `tools/release/pgsql.lock` を見て出力し（`pgsql=true`。base が引けないときは今の `all` と同じく true に倒す）、`package-windows` は PR の run で `pgsql=true` なのに `swap` でなければ落とす。base の lock を取れない PR の run も、`pgsql=true` なら落とす（main への push は D7 のとおり入れ替えないので、この確かめを掛けない）。この 2 つの振る舞い（「lock を変えた PR で前の lock を取れなければ落ちる」「lock を変えた PR で `swap` でなければ落ちる」）を Requirement と Scenario に書く。こうすれば CI が緑であることが `17.6-1 → 17.12-1` の組を含意し、完了の判定 8 を merge gate が見られる
- 処置: fixed D7 仮 —— 変えたかは `changes` の job が PR 自身の差分（分岐点から head）で決め（`pgsql`）、変えた PR で前の lock が取れない・`decide` が `swap` でない・大きい版が違うなら落とす。版の確かめの期待値は判定でなく前の lock と今の lock から取る（`check-zips`）。Requirement の本文と Scenario に書き、完了の判定 8 を「CI が緑なら版の組を含む」に直した（specs/distribution・tasks 4.2・CH26.md）

## R17. ci.yml に置く Scenario の印は `check_scenarios.py` に拾われないので、tasks 5.1 が 19 件中 18 件で止まる
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/tasks.md（4.2・5.1）
- 根拠: tasks 4.2（`tasks.md:85`）は、版を確かめる ci.yml の step に「`# Scenario: lock を変えた PR では、前の lock の版の zip から今の lock の版の zip へ更新し、戻しも通る` の印を置く」と書く。`scripts/check_scenarios.py:60` の `TEST_EXT = {".rs", ".kt", ".sh", ".py", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"}` に `.yml` は無く（`:187` で拡張子が外れたファイルを飛ばす）、今の `ci.yml` にある `Scenario:` の字（`:622` `:633` `:645`）も印として数えられていない。tasks 5.1 の検証 `python3 scripts/check_scenarios.py . ch26-nightly-vulns-cleanup` rc=0 は、この 1 件で落ちる
- kind: technical
- 提案: 版の確かめを `pgsql-prev-lock.sh` のサブコマンド（例: `check-zips <判定> <前の lock> <今の lock> <古い側の zip> <新しい側の zip>`）にし、`echo "== Scenario: …"` の印はその台本に置く。ci.yml はそれを呼ぶだけにする（D8 の「ci.yml は呼ぶだけ」とも合う）。`--self-test` で、組の食い違い（古い側が今の版・新しい側が前の版）が落ちることも見られる
- 処置: fixed 4.1 —— 版の確かめを `pgsql-prev-lock.sh check-zips` にし、Scenario の印は台本の `--self-test` に置く。ci.yml には印を置かない（design D8・tasks 4.1・4.2）

## R18. 使い回しのときも走る版の確かめの step は、今の `--check-workflow` を通る置き場が save の後しか無い。D8 と tasks 4.2 は置き場を書かない
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（D8（仮））・tasks.md 4.2
- 根拠: 今の ci.yml の写しの `package-windows` に、`if:` の無い step（`pgsql-prev-lock.sh zip-version …`）を 3 か所に足して `tools/release/windows-build-key.sh --check-workflow <写し>` を走らせた:
  - 最初の upload の前: rc=1「restore と最後の upload-artifact の間の step 10 が if: で飛ばされない（引いた置き場に書き得る）」（`windows-build-key.sh` の `s_asis`）
  - 最後の upload の後・save の前: rc=1「最後の upload-artifact と save の間に、作業木を見る step のほかの step がある」（`s_save`）
  - save の後: rc=0

  tasks 4.2 は「組んだ後（使い回したときも走る。`if:` に `cache-hit` を書かない）」とだけ書き、置き場を書かない。組んだ直後に置くのが自然な読み方だが、そこでは `--check-workflow` が落ちる。Global Constraints（`tasks.md:11`）の「検査を同じ PR で直してよい」に流れると R25 の矛盾に当たる。
  save の後に置くと、保存してから確かめるので、版の食い違った組み立てがその鍵で保存され、同じ鍵の後の run は引いて同じ所で落ち続ける（鍵が変わるまで直らない）
- kind: technical
- 提案: D8 と tasks 4.2 に置き場を書く。案: (1) 組み立ての step の中（`cache-hit != 'true'`）で、`old` を組んだ直後に確かめて落とす（保存させない）。(2) 使い回しのときのために、save の後にも同じ確かめを置く。この 2 か所を `check-workflow` が見る形に入れる。`install-windows`（zip を落とした後）に置く案もある
- 処置: fixed D8 仮 —— 置き場を (1) 試験の zip を組む step の中（保存の前）と (2) save の後の `cache-hit` に依らない step の 2 か所に決め、`check-workflow` がその 2 つを見る。`windows-build-key.sh` は変えない（tasks 4.2）

## R19. 「lock を変える PR」を base の先端の lock と head の lock の違いで決めると、main より古い枝では今のままにならない
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（D7（仮））・specs/distribution/spec.md・tasks.md 4.2
- 根拠:
  - `package-windows` は PR の head を checkout し（`ci.yml:474` `ref: ${{ github.event.pull_request.head.sha || github.sha }}`）、走る ci.yml は merge の commit のもの（鍵の step の注記 `ci.yml:479-481`）
  - (a) CH26 が merge された後、head が CH26 より前の PR では、merge の commit の ci.yml が head の木に無い `tools/release/pgsql-prev-lock.sh` を呼ぶ。tasks 4.2 は「`decide` が rc≠0 なら落とす」なので、その PR は main を取り込むまで `package-windows` が落ちる
  - (b) `decide` は base の先端の lock と head の lock を比べる（D7）。後で別の PR が lock を上げた後、lock を変えていない古い枝では中身が違うので `swap` になり、古い側を新しい lock（base）、新しい側を古い lock（head）で組んで、小さい版を下げる向きで更新を走らせる。Requirement の本文（`spec.md:54`）の「PR が lock を変えないか、PR でない run のとき、試験の zip を今のまま全部 PR の lock で組む」と食い違い、この場合の Scenario も無い。D7 の反転条件（`design.md:99`）は「base の先端と merge-base の lock が違う PR が出て」を挙げているが、出たときに何が起きるか（逆向きの `swap`）は書いていない
  - `changes` の job の差分（`ci.yml:43` の `git diff --name-only "$BASE" "$HEAD"`）も 2 点の比べなので、同じ枝で同じずれが出る
- kind: technical
- 提案: 「PR が lock を変えたか」は PR 自身の差分（merge-base から head。3 点の `git diff`）で決め、前の lock を base の先端から取るのは、変えたときだけにする（R16 の `pgsql=true` をこの 3 点の差分で取れば、同じ材料で足りる）。台本が head の木に無いときは、lock を変えていない PR なら今のまま組む（または merge の commit から台本を取り出して使う）。古い枝の扱いを Requirement に 1 行で書き、Scenario を足す
- 処置: fixed D7 仮 —— 「変えたか」を分岐点からの 3 点の差分で決め、前の lock は分岐点の lock。`pgsql == false` と PR でない run は台本を呼ばない（古い枝・CH26 より前の head も今のまま）。Scenario「base の側だけが lock を変えた古い枝は、変えたと判定しない」を足した

## R20. 鍵の入力を足すのに、正典の鍵の Requirement を書き換えず ADDED で足している。正典の列挙が偽になる
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/distribution/spec.md・proposal.md
- 根拠: 正典 `openspec/specs/distribution/spec.md:3341`（Requirement「PR の CI は、組み立ての入力が同じ Windows の配布物を使い回し、インストールの試験は全部走らせる」）は「組み立ての鍵を、…追跡している全部のファイルの中身と、走っている CI の定義と、組み立てに使う道具の版（Rust・Node・runner の image）から作る」と鍵の入力を列挙する。CH26 の新しい Requirement（`spec.md:56`）は「使い回しの鍵に、古い側を組んだ lock を入れる」を足すが、これは木の外から来る 4 つ目の入力で（D9 `design.md:119`）、archive の後は正典の 2 つの Requirement が鍵の入力について違うことを言う。proposal（`proposal.md:41`）は「`## MODIFIED` を書かない。正典の Requirement は変えずに足すだけで済む」と書くが、昇格した後は MODIFIED を書ける
- kind: technical
- 提案: 鍵の入力の一文は、正典の Requirement を `## MODIFIED` で書き換えて足す（「…道具の版と、更新の試験の古い側を組んだ lock（PR の base から取ったとき）から作る」）。CH26 の新しい Requirement からは鍵の一文を外す。proposal の Capabilities の注記もそろえる
- 処置: fixed specs/distribution/spec.md —— 正典の Requirement「PR の CI は、組み立ての入力が同じ Windows の配布物を使い回し、…」を `## MODIFIED` で全文写し、鍵の入力に前の lock を足し、Scenario「鍵を出す手順は、古い側を組んだ前の lock の指紋を鍵に入れる」を足した。新しい Requirement から鍵の一文を外し、proposal の Capabilities をそろえた（design D9）

## R21. specs の Scenario が CI の実装の名前（環境変数・step の出力の字・キャッシュの条件）を持つ
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/distribution/spec.md
- 根拠: Scenario「CI の定義は、…」（`spec.md:79-81`）の THEN は `WINDOWS_BUILD_KEY_TOOLS`・`prev-pgsql=`・`if:` に `cache-hit` を持たない、を言い、「CI の定義の形が崩れると…」（`:83-85`）も同じ。`decide` の 3 つの Scenario は台本の出力の字 `swap` / `same` を言う。正典の `openspec/specs/` には `WINDOWS_BUILD_KEY_TOOLS`・`cache-hit`・`check-workflow` の字は 1 つも無い（`grep -rn` で 0 件）。CH17 の鍵の Requirement（正典 `:3337-3346`）も、同じことを実装の名前を出さずに言う（「鍵が完全に一致したときだけ使う」）
- kind: technical
- 提案: Scenario は観測できる振る舞いで書く（例:「前の lock が違えば鍵が違う」「使い回した run でも版の確かめが走る」「lock を変えていない PR の組み立ては、前の lock を読まない」）。`WINDOWS_BUILD_KEY_TOOLS`・`prev-pgsql=`・`cache-hit` の形は design D8・D9 と `check-workflow` の自己試験に置く
- 処置: fixed specs/distribution/spec.md —— Scenario から `WINDOWS_BUILD_KEY_TOOLS`・`prev-pgsql=`・`cache-hit`・`swap`/`same` を外し、観測できる振る舞い（判定する・同じ中身と判定する・保存の前と使い回した run の両方で確かめる・前の lock の指紋を鍵に入れる）で書いた。実装の名前は design D8・D9 と tasks 4.2 に置いた

## R22. 更新した後・戻した後に動いている PostgreSQL の版を、どこも見ていない。CI が緑でも、小さい版が入れ替わったことは示されない
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/distribution/spec.md・design.md（D5）
- 根拠:
  - 正典 `openspec/specs/distribution/spec.md:3966` は「大きい版が同じで小さい版が違う PostgreSQL は、プログラムの置き場と一緒に入れ替え、戻すときは一緒に前の版に戻す」と言うが、この一文を見る Scenario は無い（同じ Requirement の Scenario は `:3971` の大きい版違いの 1 つだけ。`:3977` からは別の Requirement）。P14 のとおり、今の CI はこれを走らせたことが無い
  - `tools/release/update-apply-install.sh` の判定が戻した後に見るのは、アプリの版（`:409` `installed-version=$old_version`）だけ。`postgres.exe` / `pg_ctl.exe` の版を読む所は無い（`--version` を読むのは `:506` の `ashiato-server` だけ）
  - CH26 の Scenario「lock を変えた PR では…」（`spec.md:75-77`）が見るのは、zip の許諾の文の版（組んだときの lock の写し）と `update-apply-install.sh` が通ることだけ。インストーラが pgsql の置き場を入れ替えなくても、戻しで前の版に戻さなくても、緑になる
  - proposal（`proposal.md:31`）は `update-apply-install.sh` を「変えないもの」に入れている
- kind: technical
- 提案: どちらかにする。(1) 更新の後に `pgsql\bin\postgres.exe -V`（か `SELECT version()`）が新しい側の版、戻した後に前の側の版であることを 1 つの Scenario（できれば 2 つ）にし、`install-windows` の `update-apply-install.sh` の後の step か台本の判定で見る（台本を変えるなら proposal の「変えないもの」から外す）。(2) 見ないなら、D5 の Risks に「CI は小さい版の違う zip の間の更新と戻しが通ることを見るが、入れ替わった PostgreSQL の版は見ない」と書き、確認バッチの仮の一覧で本人が見られるようにする
- 処置: fixed D10 仮 —— 見ない（`update-apply-install.sh` に版の観測を足すのは本人の補足「既存の update-apply-install.sh で通す」と「変えないもの」を越える）。費用と反転条件を design D10（仮）と Risks に書き、台帳に載せて確認バッチの仮の一覧で本人が見る（完了の判定 6）

## R23. Requirement の本文だけが言い、Scenario が言っていないことがある。崩した形の写しも、主張の一部しか崩さない
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/distribution/spec.md・tasks.md 4.2
- 根拠:
  - 本文（`spec.md:54`）の「PR が lock を変えないか、PR でない run のとき、試験の zip を今のまま全部 PR の lock で組む」を見る Scenario が無い（`decide` の `same` の Scenario は台本の出力を見るだけで、ci.yml が PR でない run で `decide` を飛ばすかは見ない）
  - 本文（`spec.md:56`）の「使い回しの鍵に、古い側を組んだ lock を入れる（前の lock が変われば組み直す）」は、`check-workflow` が `prev-pgsql=` の字があることを見るだけ。値が `swap` のときの前の lock の sha256 で、`same` のときに `none` かは見ない（`=none` を固定で書いても通る）
  - 「CI の定義の形が崩れると…」（`spec.md:83-85`）の 4 つの写しは、正の Scenario（`:79-81`）が言う 4 つの形のうち「`decide` の step が鍵の step より前」を崩さず、版の確かめの step を消す写しも無い。この 2 つは崩しても `check-workflow` が落ちることを誰も見ない
- kind: technical
- 提案: PR でない run の扱いは R19 の Scenario に合わせて 1 つ足す。鍵の値は `check-workflow` で式の形（`swap` の出力と前の lock の sha256 から作ること）を見るか、Scenario を「前の lock が違う 2 つの run は鍵が違う」にして台本の自己試験で見る。崩した写しに「`decide` を鍵の後へ動かす」「版の確かめの step を消す」の 2 つを足す
- 処置: fixed specs/distribution/spec.md —— 本文の「lock を変えない・PR でない run は今のまま」を Scenario「base の側だけが lock を変えた古い枝は…」と CI の定義の Scenario（前の lock を取る手順は変えたと判定された run でだけ走る）にし、鍵の値は Scenario「鍵を出す手順は、…前の lock の指紋を鍵に入れる」で見る。崩した写しは 7 通り（鍵の後へ動かす・判定に依らず走らせる・古い側のほかも置き換える・戻さない・保存の前の確かめを消す・使い回しのときの確かめを引けなかったときだけにする・鍵に渡さない）にした

## R24. 1 つの Scenario が複数の主張を束ねている（第 1 回 R13 と同じ型の、新しい 3 件）
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/specs/distribution/spec.md
- 根拠:
  - 「組んだ zip の同梱の版を、許諾の文から読む」（`spec.md:71-73`）: THEN が「`17.6-1` を出す」と「節が無い zip では rc≠0」の 2 つ
  - 「lock を変えた PR では…」（`:75-77`）: THEN が「`package-windows` が版の組を確かめて出す」と「`install-windows` の `update-apply-install.sh` が全部の Scenario を通す」の 2 つ。別の job で、R17 のとおり印も片方にしか置けない
  - 「CI の定義は、…」（`:79-81`）: THEN が 4 つの形を束ねる（片方だけ崩れても、R23 のとおり崩した写しが無い形は緑のまま）
- kind: technical
- 提案: それぞれ分ける。「lock を変えた PR では…」は、`package-windows` の版の組の Scenario（印は R17 の台本）と、`install-windows` の更新と戻しの Scenario（`update-apply-install.sh` の既存の印が担保する旨を書くか、R22 の版の観測の Scenario にする）に分ける
- 処置: fixed specs/distribution/spec.md —— zip-version を「読む」と「無ければ落ちる」の 2 つに、版の組を「通る」と「落ちる」の 2 つに、CI の定義を「前の lock と置き換え」「版の確かめの 2 か所」「鍵」の 3 つに分けた。「lock を変えた PR では…更新し、戻しも通る」は Scenario から外し、Requirement の本文で今の `update-apply-install.sh` の Scenario 群が担保すると書き、緑が版の組を含むことは `check-zips` の Scenario と完了の判定 8 で見る

## R25. 触ってよいパスが成果物の間で食い違う。tasks 2.1 には軽い路の止め方が残っている
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/tasks.md（Global Constraints・2.1・5.4）・design.md（D9（仮））・proposal.md・docs/changes/CH26.md（完了の判定 5）
- 根拠:
  - Global Constraints（`tasks.md:11`）と D9（`design.md:119`）は「`windows-build-key.sh --check-workflow` が落ちて避けられないときだけ、その検査を同じ PR で直してよい」と書く。一方 proposal（`proposal.md:31`）は `windows-build-key.sh` を「変えないもの」に入れ、5.4 の検証の正規表現（`tasks.md:107`）と完了の判定 5（`CH26.md:45`）の許すパスにも `tools/release/windows-build-key.sh` は無い。直すと 5.4 が落ちる。R18 のとおり、版の確かめの step の置き場しだいで、この道に入りうる
  - tasks 2.1（`tasks.md:49`）は「`tools/release/build.sh` の変更が要る —— 予定のパスの外なので手を止める（軽い路から外れる。ledger に書いて昇格させる）」と書く。もう昇格しているので「昇格させる」は効かない。手を止めた後に何をするか（A で返すのか、許すパスを広げるのか）が決まらない
- kind: technical
- 提案: `windows-build-key.sh` を直す道を残すなら、5.4 の正規表現・完了の判定 5・proposal の「変えないもの」に条件つきで足す。残さないなら Global Constraints と D9 からその一文を消し、R18 の置き場を tasks に書いて検査を変えずに済むことを示す。2.1 の止め方は「`build.sh` の変更が要れば手を止め、理由を ledger に書いて A で返す」など、昇格した後の形に書き直す
- 処置: fixed tasks.md —— `windows-build-key.sh` を直してよいとする一文を Global Constraints と D9 から消し（D8 の 2 か所の置き場で今の検査のまま通る）、組み立ての台本に触る理由が出たら A で返すにした。2.1 の止め方も「ledger に書いて A で返す」に直した

## R26. `decide` と `zip-version` は Windows の Git Bash で走るのに、その環境の制約が書かれていない（python・改行）
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（D8（仮））・tasks.md 4.1
- 根拠:
  - `decide` と `zip-version` は `package-windows`（`runs-on: windows-latest`）の step で呼ばれる。同じ置き場の `windows-build-key.sh:11` は「Windows の Git Bash でも動くよう、sha256sum と git と bash の組み込みだけを使う」と制約を書いている。Windows で走る台本は `python` を先に探し、無ければ `python3` を使う（`update-apply-install.sh:312` `:320`・`install-check.sh:381` `:390`。`install-windows` の `ci.yml:664` も `python` で呼ぶ）。D8 は `check-workflow` が python3 と yaml で読むとだけ書き、`decide` / `zip-version` が何で書かれるかを決めていない。Linux の `chain` の自己試験だけが通り、Windows の runner で初めて落ちうる
  - `decide` は「中身が同じなら `same`」（`tasks.md:76`）で、`git show` で取った base の lock（LF）と作業木の `tools/release/pgsql.lock` を比べる。`build.sh:46` は「Windows の Git checkout は core.autocrlf で作業木を CRLF にし得る」と書き、`.gitattributes` は `pgsql.lock` の改行を決めていない（`git check-attr -a tools/release/pgsql.lock` は何も出さない）。作業木が CRLF なら、lock を変えていない PR でも中身が違って `swap` になる。今の runner では `pgsql.sh fetch` の版の正規表現（`pgsql.sh:20`）が通っているので lock は CR なしで読めており、今起きている不具合ではない。checkout の設定や手元の Windows で崩れないための、台本の頑健さの条件
- kind: technical
- 提案: D8 と tasks 4.1 に「`decide`・`zip-version` は bash・git・unzip・sha256sum だけで書く（python を使わない）」と書く。`decide` の「中身が同じ」は、作業木のファイルでなく blob の id（`git rev-parse <base>:tools/release/pgsql.lock` と `HEAD:tools/release/pgsql.lock`）か、改行を落とした値どうしで比べる。自己試験に、CRLF の写しが `same` になることを置く
- 処置: fixed D8 仮 —— `changed`・`decide`・`zip-version`・`check-zips` は bash・git・unzip・sha256sum・awk/sed だけで書く（Global Constraints・tasks 4.1）。`decide` は行末の CR を落として比べ、Scenario「改行だけが違う 2 つの lock は、同じ中身と判定する」を足した

## R27. design と proposal の「走っている CH22・CH24・CH25」は、origin/main では archive 済み。P21 の行番号はずれる
- 成果物: openspec/changes/ch26-nightly-vulns-cleanup/design.md（Risks）・proposal.md（Impact）・premises.md（P21）
- 根拠: origin/main（`56fd1c5a8`）に `openspec/changes/archive/2026-11-21-ch22-soak-install-online-ci`・`2026-11-21-ch24-zip-upgrade`・`2026-11-21-ch25-ci-flake-gradle-cache-clock-test` がある。この枝の merge-base（`6642ee755`）からの `git diff HEAD origin/main -- .github/workflows/ci.yml` は +50/-5 行で、CH22 が `install-windows` に `timeout-minutes: 100` と置く段（`soak-install.sh`）を足し、CH25 が `setup-java` に gradle のキャッシュ、`chain` の頭にキャッシュの step を足した。`package-windows` の step（鍵・restore・組み立て・upload・save）は変わっていないが、`package-windows` より後の `ci.yml` の行は 29 行後ろへずれ（hunk の見出し `@@ -596,6 +625,7 @@`）、P21 の `ci.yml:479-517` は合わない。`Cargo.lock`（372 行）と `osv-scanner.toml`（-28 行）も CH24 で変わっている（rustix は main でも 1.1.5 のまま）。`design.md:130`・`proposal.md:49-51` は「走っている」「後から merge する側が rebase で合わせる」と書く。あわせて origin/main の `docs/release.md:7` に、§1・§6・§8 は置き換え中だという ★ の注記が入った。tasks 4.3 が書き換える「### PostgreSQL の小さい版を上げる PR」は §8 の中にある（origin/main の `docs/release.md:138`）
- kind: technical
- 提案: Risks と Impact を「CH22・CH24・CH25 は merge 済み。実装は main の上で始める（ci.yml・Cargo.lock・osv-scanner.toml は main の形が元）」に直し、P21 の行番号を main で取り直す。tasks 4.3 は、§8 の ★ の注記を残したまま小節だけを書き換えると書き添える
- 処置: fixed proposal.md —— CH22・CH24・CH25 を「main に merge 済み。実装は main の上で始める」に直し（proposal の Impact・design の Context と Risks・tasks の Global Constraints）、P21 の行番号を origin/main で取り直した。tasks 4.3 に §8 の ★ の注記を残して小節だけを書き換えると書いた

### 観点 2: Scenario が検証可能か

R16（「lock を変えた PR」が base を取れないと空で通る）・R22（更新と戻しの後の PostgreSQL の版を観測しない）・R23（本文だけが言うこと・崩した写しの抜け）・R24（束ね）。`decide` の 3 つの Scenario と `zip-version` は、使い捨ての lock と zip で WHEN と THEN を観測できる。

### 観点 3: 置き場の誤り

R16 の後段（base を取れないときの振る舞いが design D7 と tasks にだけある）・R20（鍵の入力は正典の Requirement の書き換え）・R21（Scenario に実装の名前）。D7（仮）の「入れ替えるのは test-old だけ」「PR でない run は入れ替えない」は Requirement の本文に出ている。proposal の Capabilities（`dependency-vulnerability-scan`・`distribution`）と `specs/` のディレクトリ、CH26.md の front matter の `capability` は一致する。

### 観点 4: tasks が検証を持つか

R17（5.1 が tasks のとおりでは落ちる）・R18（4.2 の版の確かめの置き場が無く、自然な置き場では 4.2 の `--check-workflow` が落ちる）・R25（5.4 と Global Constraints の食い違い、2.1 の止め方）。4.1 の 2 つ目の検証は前の lock を merge-base から取り、D7 は base の先端から取る。手元の確かめとしては足りるが、R19 の決め方を直すならそろえる。依存の順（2.1 で lock を替える → 4.1 の `decide` が `swap`）は正しい。完了の判定 8 は Windows の runner でしか走らず、Task の手元の検証を持たない。R16 の直しで、CI が緑であることが判定 8 を含意する形にできる。

### 観点 5: change の文書との一致

`docs/changes/CH26.md` の完了の判定は 8 つが埋まっていて、6（`:46`）と 8（`:48`）は違和感の直しに合う。壊してはいけないもの 3・4（`:35-36`）は R2 = C に書き直されている。食い違いは R25（判定 5 の許すパス）と R16（判定 8 を機械で見ない）。依頼の逐語の範囲を CI の変更で越えるが、deep の R2 = C（`deep.md:19` `:25`）が決めているので指摘しない。

### 観点 6: 答えが写ったか

該当なし（`spec --answers` の呼び出しではない。`answers.md` は書かない）。違和感の補足の写りは観点 1 の表で見た。名指しの補足のほかの変更は、`premises.md` の P19〜P21（D7〜D9 の根拠）・`provisional.jsonl` の D5 の resolved と D7〜D9 の 3 行・proposal の昇格の注記・CH26.md の判定 4 の参照（4.1 → 5.2）で、どれも補足の直しかその根拠として説明がつく。

### 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）。CH26 の specs は ADDED だけで、走っている change（origin/main の `openspec/changes/` は `st59-change-password-from-web` だけ）と同じ Requirement を持たず、名指しもしない。
呼び出し元が挙げた CH22・CH25 は origin/main で archive 済み（2026-11-21）で、走っていない（R27）。ファイルの重なりとしては、CH22 は `install-windows`、CH25 は `setup-java` と `chain` の頭を変え、CH26 が触る `package-windows`（鍵の step・組み立て・版の確かめ）と `chain` の `windows-build-key.sh` の隣の 2 行とは同じ行を取り合わない。R20 で正典の鍵の Requirement を MODIFIED にするなら、その Requirement を書き換える走っている change が無いことを `spec_overlap.py candidates` でもう一度見る。
