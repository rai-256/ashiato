# CH21 の前提（問いより先に確かめたもの）

確かさ: 1 言っただけ / 2 出所を示した / 3 辿った / 4 走らせた / 5 実物で再現した。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| このリポジトリ（issue の置き場）は公開されている | Q7（違和感の候補を issue にするか。issue は誰でも読める） | `gh repo view --json visibility` | `PUBLIC` | 実物 | 5 |
| 手元の Windows で Windows Sandbox を起こせる | 席の置き場（依頼の「どこまで」） | `C:\Windows\System32\WindowsSandbox.exe` の有無・PC の収集アプリの録画（ST07/ST08、2026-10-15 実地で passed） | 有り。録画で起動・撮影・片付けまで通った | 実物 | 4 |
| 手元の Windows に Git の bash と cargo がある | Q2（配る zip を手元で組めるか） | `/mnt/c/Program Files/Git/bin/bash.exe`・`/mnt/c/Users/yosis/.cargo/bin/cargo.exe` の有無 | どちらも有り。`tools/release/package.sh windows` を手元で通したことはまだ無い | 実物（有無だけ） | 2 |
| CI は PR ごとに配る Windows の zip を artifact に上げる | Q2（CI の組み立てを使う案） | `.github/workflows/ci.yml:458-548`（`package-windows` job → `package-windows-release`） | 有り。ただしコードが変わったときだけ走る（`needs.changes.outputs.code`）・Actions の予算に依る | 公式の定義（repo の file:line） | 2 |
| 確認バッチは `tools/verify-prep.sh` を録画より先に呼び、その `manifest.md` が手順書の冒頭に載る。落ちても続ける | 差し込み口・失敗の扱い | `.harness2/scripts/verify_batch.sh:180-208` | そのとおり（落ちたら `warn:` を出して成果物なしで続ける） | repo の file:line | 2 |
| 確認バッチの録画も同じ Sandbox を使う | 席の順番（Sandbox の取り合い） | `tools/recording/README.md`（PC のシナリオはまっさらな Sandbox で撮る） | 使う。Sandbox は同時に 1 つしか起こせない（知識） | repo の文書 + 知識 | 2 |
| 手元の側（WSL）の Claude から、Sandbox の中のブラウザ（Edge）を遠隔で操れる（デバッグの口を Sandbox の網の口へ転送する） | Q3（Claude をどう動かすか） | 未実施（下流の最初の Task で試す） | 未確認 | 知識 | 1 |
| 配る zip のインストーラ（サービスの登録・昇格を含む）が Sandbox の中で最後まで通る | この change の土台・Q2 | 未実施（ST55 の P16/P19 は CI に委ねたまま。Sandbox で通ったのは利用者の権限で動く PC の収集アプリの録画だけ）。下流の最初の Task で試す | 未確認 | 知識 | 1 |
| 11-19 の違和感（更新の確認がエラーを出し続ける）がどの版・どの条件で起き、Sandbox で再現するか | Q13 | 未実施 | 未確認 | 本人の報告（依頼） | 1 |
| 確認バッチの手順書（`docs/verify/<tag>.json`）は commit されて公開の origin へ push され、manifest は PR の本文にも写る | Q7 | `.harness2/scripts/verify_checklist.py:583-586`・`verify_batch.sh:209・214・234`（deep-review R2）と、main に commit 済みの手順書に manifest の段落があるか（`git ls-files docs/verify` → `grep run.sh`） | そのとおり。commit 済みの手順書に manifest の段落が載っている | 実物 | 5 |
| 手順書に出る候補の題に、本人の記録・私的な値が入らない | Q7（題 1 行は公開の手順書に出る） | Claude に渡す材料が目的の一覧・配る zip・合成の書き出しだけであること（spec の「材料の置き場と Claude の作業場所には…何も無い」が機械で縛る）を辿り、題は 1 行 60 文字・`tools/check-private.sh` の型を伏せる（spec の「題は 1 行 60 文字までで…」）。spec-review R2 | 材料に本人のものは無い（作りで）。Claude の言い回しそのものは機械では縛れない | 設計（この change の spec） | 3（自己検査が通れば 4） |

失われるもの（`loss`）を持つ決定は Q7・Q8 で、依る前提（公開のリポジトリは確かさ 5、手順書が公開の repo に出ることも確かさ 5）。
確かさ 1〜2 の前提に依る問い（Q2・Q3・Q13）は、どれを選んでも何も失わない（作り直せば戻る）ので B に置いた。
