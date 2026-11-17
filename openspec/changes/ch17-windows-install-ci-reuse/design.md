# CH17 design — Windows の配布物の使い回しと Rust のビルドのキャッシュ

## Context

動機は `proposal.md` の Why。いまの形（`.github/workflows/ci.yml`）:

- `package-windows`（`if: needs.changes.outputs.code == 'true'`、PR の head の commit を checkout）が、`ch14_` の試験 → `package.sh windows`（配る形）→
  `package.sh windows --test-build`（試験の形）を毎回ビルドし、`package-windows-release` / `package-windows-test` の成果物に上げる。キャッシュは PostgreSQL の zip だけ
- `install-windows`（`needs: [changes, package-windows]`、`if: windows == 'true'`）がその 2 つを受け、`install-check.sh` と `update-check-install.sh` を走らせ、
  走った Scenario を `--list` の全部と突き合わせる
- `package-artifacts` が Linux と Windows の配る形を集め、`check-artifacts.sh` を通して `release-package` に上げる（`tools/release/draft.sh` が下書きのリリースに使う）

守らなければならない機械の検査（触れない。本人の範囲は CI の Windows の 2 つの job だけ）:

| 検査 | 見ているもの |
|---|---|
| `tools/release/check-guards.sh workflows`（と `--self-test`） | `package-windows`・`package-artifacts` の job の `if: needs.changes.outputs.code == 'true'`、`package-windows` が `ref: ${{ github.event.pull_request.head.sha \|\| github.sha }}` で checkout すること、`ci.yml` が `changes.sh code` を使うこと |
| `tools/check-ci-ch14-step.sh` | `ch14_` の試験（下限 11 本つき）の step が、`package.sh windows` を含む最初の step より前にあること |
| `tools/check-vulns.sh --self-test` | `changes.sh` の `code` / `deps` の判定 |

## Goals / Non-Goals

- Goals: 試験の台本と文書だけが変わった PR の push で、Windows の配布物のビルド（約 33 分）を飛ばす。ビルドするときも Rust の依存を組み直さない
- Non-Goals: `install-windows` の手順を縮めること（17 分はそのまま）・手元の Windows を使うこと・他の job・製品のコード・配布物の作り方を変えること・
  Android など Windows の配布物に入らない部分だけの変更で使い回すこと（D1 の反転条件）

## Decisions

### D1（仮）. 組み立ての鍵は「名指しして除くもの以外の全部」から作る

鍵 = sha256（`git -C <repo の根> ls-tree -r --full-tree <checkout した commit>` の行のうち、除くパスを落としたもの ＋ 走っている run の CI の定義 ＋ 道具の版の行）。
`--full-tree` と repo の根で読む（cwd に依らない。review R8）。repo の根が引けなければ落ちる。
`-z` で読む（`core.quotePath` が日本語などの名前を引用符で包み、`docs/*`・`*.md` に当たらなくなるのを避ける。code review R9）。
除くパスの一覧は spec の名指しの 6 つちょうどで、自己試験がそれを固定する（足しすぎを止める。code review R4）。

- **除くもの（名指し）**: 試験の台本 `tools/release/install-check.sh`・`tools/release/update-check-install.sh`・`tools/release/conpty-run.ps1`（`install-check.sh` だけが読む）と、
  `changes.sh code` が「コードでない」とするもの（`docs/`・`openspec/` の下と `*.md`）。`build.sh`・`package.sh`・`notices.sh`・`pgsql.sh` はこの 3 つの台本を読まない
  （`grep` で確かめた。premise 確かさ 2）。配る組み立て（`cfg(test)` でないコード・`build.rs`・`build.sh` 系）は `*.md`・`docs/`・`openspec/` を読まない（同）。
  試験のコードは `docs/contracts/` などを `include_str!` で読む（`crates/update-check`・`crates/collector-windows` の `cfg(test)`。review R5）が、配る実行ファイルには入らず、D4 で飛ばす `ch14_` の試験は `docs/` を読まない
- **含むもの**: 上以外の追跡している全部。`.github/workflows/ci.yml`（`package-windows` の手順そのもの）・`tools/release/pgsql.lock`・`tools/licenses/`・`LICENSE`・
  `Cargo.toml`（版番号の出所。`package-artifacts` が head から取る期待値と、使い回した配布物の中身が同じであるための条件。spec の MODIFIED・整合の確認 K1）・`Cargo.lock`・`web/`・`migrations/`・`collector-android/` も入る
- **道具の版**: `rustc -V`・`node -v`・runner の `ImageOS` と `ImageVersion`（`dtolnay/rust-toolchain` の `stable` と `setup-node` の `24` は浮いている）。
  `ImageVersion` は git の木の外の入力 —— runner の Visual Studio から写す VC の実行時の DLL（`build.sh` の `bundle_vc_runtime`）と MSVC のリンカの版 —— も覆う（review R4）
- **走っている run の CI の定義**: pull_request の run は、head ではなく GitHub が作る merge の commit の `ci.yml` で走る（GitHub の一般の振る舞い。この repo では未確認・確かさ 1。review R11）。
  `package-windows` は head を checkout するので、鍵の step が `${{ github.sha }}` を浅く fetch し、その commit の `.github/workflows/ci.yml` の blob の id を道具の版の行に足す。fetch できなければ鍵を空にする（D2 でビルドする側）
- **git の blob の id から作る**: checkout の改行の変換（Windows の autocrlf）に依らない
- **台本**: `tools/release/windows-build-key.sh <commit>`（鍵を 1 行で出す）・`--self-test`（spec の鍵の 5 つの Scenario）・`--check-workflow [ci.yml]`（spec の CI の定義の 2 つの Scenario）。
  `changes.sh` には足さない（`deps` の判定に入っていて、触ると `vulns` の job が走る。`code` / `deps` の挙動は `check-vulns.sh --self-test` が固定している）

退けた案: 入力を数え上げる（`crates/` `Cargo.*` `web/` `migrations/` `tools/release/build.sh` …）。数え漏らした入力が変わっても使い回し、古い配布物で試験が通る。
名指しして除く形なら、知らない入力は「ビルドし直す」に倒れる。

**反転条件**: Android だけ・Linux の道具だけの変更でも Windows の配布物をビルドし直していて、1 周の短縮が足りないと分かったら、除くものに
`collector-android/` などを足す（足すときは、その下を Windows の組み立てが読まないことを確かめる）。ImageVersion で鍵が週ごとに変わり、
使い回しがほとんど効かないと分かったら、ImageVersion を外し、代わりに VC の再頒布の置き場の版（`VC/Redist/MSVC/<版>`）と MSVC の toolset の版を道具の版の行に入れる（review R4）。
試験でないコードが `docs/contract/` などを読むようになったら、その置き場を除くものから外す（review R5）。
除くものを足すときも、`Cargo.toml` と `LICENSE` は外さない（外すと `package-artifacts` の期待値と使い回した配布物が食い違い、使い回した run が落ちる）。

### D2（仮）. 保存物は actions/cache の完全一致で引き、PR の run でだけ使う

- 置き場: GitHub Actions のキャッシュ（`actions/cache/restore` と `actions/cache/save`。今の `actions/cache` と同じ repo・同じ pin の commit。依存は増えない）。
  中身は `package-windows` の job が `upload-artifact` で上げる置き場の**全部**（今は `package-windows`・`package-windows-test` の 2 つ）。上げる置き場を足したら、保存と引きの置き場にも足す
  （`--check-workflow` が「上げる置き場は全部、保存と引きの置き場に入る」を見る。走っている ST58 は試験の形を 6 つに分けるので、どちらが先に main に入っても後の側が合わせる。整合の確認 C4）。鍵は `windows-package-<D1 の鍵>`。`restore-keys` を書かない（前方一致で古い組み立てを引かない）
- 使うのは `pull_request` の run だけ。`main` への push は毎回ビルドし、保存する（PR の run は main の保存物を引けるので、main から枝を切った直後の台本だけの PR でも効く）
- 引けたら: `ch14_` の試験・`package.sh windows` の step を `if:` で飛ばし、引いた置き場を今と同じ名前（`package-windows-release` / `package-windows-test`）で、書き換えずに上げる。
  `install-windows` と `package-artifacts` は何も変えない
- 引けなかったら: 今と同じにビルドし、成功したときだけ `actions/cache/save` で保存する
- job の `if:` と checkout は変えない。step の並び（`ch14_` → 2 つの `package.sh windows`）も変えない（Context の表の検査）
- 鍵を出す step が落ちたら、鍵を空にして「引けなかった」と同じに扱う（既存の「比較できないときは全部走らせる」に合わせる）。台本の自己試験が落ちたら job を落とす。
  自己試験は組み立てるときだけ走らせる（引けたときは、同じ台本・同じ ci.yml の run が自己試験を通して保存している。code review R10）
- 走っている run の CI の定義は、run の commit が checkout した HEAD と同じ（main への push）なら HEAD から読み、fetch しない。PR の run だけ merge commit を浅く fetch し、
  token は `git -c` の引数でなく `GIT_CONFIG_*` の環境変数で渡す（code review R12）
- 鍵は commit の木から出すが、組み立ては作業木から組む。組み立ての後に追跡しているファイルが commit のままか（`git status --porcelain --untracked-files=no` が空か）を見て、
  変わっていれば警告を出して保存しない（code review R13）

退けた案: 前の run の成果物（artifact）を探して引く。`permissions: contents: read` を広げ（`actions: read`）、どの run を信じるかの判定が要る。キャッシュは
PR ごとの置き場に分かれ、fork の PR の保存物は他の PR や main から引けない。

**反転条件**: キャッシュの置き場（10 GB）が溢れて保存物がすぐ消え、使い回しが効かないと分かったら、Rust のキャッシュ（D5）を `~/.cargo` だけに絞る。
main の push の 33 分が問題になったら、main でも引く。

### D3. 使い回した run の `release-package` には、使い回した Windows の配布物が、組んだ commit の印のまま入る（本人の決定: 深掘り第 1 回 Q2）

本人は「要件を改め、使い回した配布物は組んだ commit を指すとする」を選んだ（`package-artifacts` は変えない）。振る舞いは spec の MODIFIED
（Requirement「配布物は、本体の許諾・依存の許諾の一覧・対応するソースの場所を持つ」と Scenario「使い回した Windows の配布物は、組んだ commit のソースの場所と印のまま上げる」）が持つ。
ここには、その振る舞いがどう成り立つかだけを書く。

- 使い回した run では、`release-package` の Windows の 2 つの zip（サーバ・収集アプリ）の `SOURCE.txt` と組み立ての印は、それを組んだ commit（同じ PR の前の push の head か main の commit）を指す
- `check-artifacts.sh` は配布物ごとに印と `SOURCE.txt` の commit の一致を見る（`check-artifacts.sh:164-167`）ので、どちらも組んだ commit で揃い、通る
- `package-artifacts` は期待する版番号と `LICENSE` を head から取る（`ci.yml` の `built/`）。`Cargo.toml` と `LICENSE` が鍵に入っているので、使い回すときはその 2 つが head と同じ（D1。整合の確認 K1）
- 外に出ない: `release-package` は CI の成果物で、`draft.sh` が作るのは公開しない下書き。本物のリリース（`release.yml`）と開発版（`nightly.yml`）は自分でビルドする
- `draft.sh` は変えない（本人の範囲）。下書きの説明文「配布物は PR の package の組み立て（run <N>）」は、使い回した run では Windows の 2 つについて別の run の組み立てになる。
  手で確かめる手順（`docs/release.md` §11.1 の「run の `head_sha` と配布物の組み立ての印の commit が同じ」）は、使い回した run では Windows の 2 つで食い違うので、
  その手順に「使い回した run では、Windows の 2 つは `SOURCE.txt` と印が同じ commit を指すことを見る」を足す（tasks 3.2。整合の確認 C3）
- 未確認（確かさ 1）: 使い回した配布物の `SOURCE.txt` が指す同じ PR の前の push の head を、枝を push し直した後に空の clone から `git fetch origin <commit>` で取れるか。
  取れなくても、PR の配布物は公開しないので外に出たものは無い。`docs/changes/CH17.md` の完了の判定 4 で確かめる（整合の確認 C2）

### D4（仮）. 使い回したときは `ch14_` の試験も飛ばす

`ch14_` の試験の入力（`crates/installer/`・`Cargo.lock`）は鍵に入っていて、鍵が同じなら前の run で同じ試験が通っている（保存は成功したときだけ）。

**反転条件**: `ch14_` の試験が runner の環境（コンソールのコードページなど）に依って揺れると分かったら、使い回したときも走らせる（D5 の Rust のキャッシュで数分）。

### D5（仮）. Rust のビルドのキャッシュ

ビルドするとき（D2 で引けなかったとき）だけ、`~/.cargo/registry`・`~/.cargo/git`・`target` を `actions/cache` で戻す。鍵は
`cargo-windows-<rustc -V>-<Cargo.lock の hash>`、`restore-keys` は `cargo-windows-<rustc -V>-`（Rust の版が同じなら前の依存のビルドを使う。cargo が指紋で組み直す）。

- 組み立ての印は古くならない: `build-stamp` は `.git/HEAD` と `ASHIATO_BUILD_COMMIT` を見張り（`crates/build-stamp/src/lib.rs` の `rerun_lines`）、
  checkout が HEAD を書き直すので印を組み直す。印が古くなっても `check-artifacts.sh` が印と `SOURCE.txt` の commit の食い違いで落とす
- `rustc -V` を鍵に入れ、Rust の版が変わったら前の `target` を使わない

**反転条件**: キャッシュの置き場が溢れる・戻すのに組み直すより時間がかかると分かったら、`target` を外して `~/.cargo` だけにする。

## Risks / Trade-offs

- [除くものの名指しが誤り、試験の台本が実は組み立てに入る] → 組み立てに入るのに鍵から外れると、古い配布物で試験が通る。除くのは 3 つの台本と文書だけにし、
  鍵の自己試験が「組み立ての台本（`build.sh`）が変われば鍵が変わる」を固定する。除くものを足すときは D1 の反転条件の確かめをする
- [使い回した配布物の印が前の commit] → D3。`release-package` から作った下書きの Windows の配布物は、前の commit の印を持つ
- [キャッシュの置き場の溢れ] → 古いものから消え、ビルドし直すだけ（遅くなるが、誤った配布物は使わない）
- [キャッシュの中身の改竄] → PR の保存物はその PR の置き場にあり、他の PR と main は引けない。main の保存物は main の push の run だけが書く

## Migration Plan

`ci.yml` を変えるだけ。戻すときは、この change の `ci.yml` の差分を戻す（保存物は 7 日で消える）。

## 確かめ（完了の判定は `docs/changes/CH17.md`）

手元では、鍵の台本の自己試験と CI の定義の検査、Context の表の 3 つの検査を走らせる。本物の Actions での短縮は、この change の PR の CI と、
同じ commit の再実行・merge の後に台本だけを変えた別の commit の PR・`crates/` を変えてビルドする run で確かめる（tasks の「人間の確認待ち」ではなく、`docs/changes/CH17.md` の完了の判定の 2〜5。CI の run を見れば分かる。review R10）。
