# CH21 の前提（外の形式・振る舞い）

調べた日: 2026-11-20（第 3 版。第 2 版の調べのうち今も効くものは残し、見つけた後の処置の経路を足した）。
失われるもの（`loss`）のある決定は Q4（公開の repo に出すもの）と Q8（Sandbox の網）。Q4 が依る前提のうち、repo が公開・手順書の問いと
答えが commit される・違和感の issue が公開の repo に立つ、は実物で確かめた（確かさ 5）。席の台本が題を公開してよいと決めていること・確認バッチの DB の中身は
読んで確かめただけ（確かさ 2）で、推奨はどちらにも寄りかからない側（題は伏せる検査を通す・席は専用の DB で立てる）に置いた。Q8 が依る前提は第 2 版で確かめたもの
（網をつなぐと私設網に届く。確かさ 5）。網を切った Sandbox で配る形が通るかは確かめていない（確かさ 1）—— 推奨の「網を切る」は
何も外へ出さない側なので、確かさ 1 の行に失うものは載らない。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| 確認バッチの `fix` は、人間が手順書で「通らなかった」と答えた後にだけ走る（記録 → fix → 手順書へ戻る）。人間が答える前に AI の結果から fix へ入る道は無い | Q1（処置をどこに置くか）・Q2（直しをいつ・どこに積むか） | `.harness2/graph/harness_graph/verify.py` の 325〜350 行（`record` の `failed` が「通らなかった」の行だけ）と 403〜423 行の辺 | そのとおり。fix へ入るのは `record` からだけ | 実物（`.harness2/graph/harness_graph/verify.py:338`） | 2 |
| `verify/<tag>` は毎回 origin/main から作り直す使い捨てで、そこに積んだ直しは次の作り直しで消える。だから統合の CI の直し（ci_fix）は、直しを原因の Story のブランチへ載せる | Q2（直しをいつ・どこに積むか） | `.harness2/scripts/verify_ci_route.py` の冒頭（12〜13 行） | そのとおり。載せる先は trailer `Story: ST<NN>`（change は `CH<NN>`）で決まる | 実物（`.harness2/scripts/verify_ci_route.py:12`） | 2 |
| 人間が答える前の候補（draft の verify PR）は、新しいバッチに置き換えられることがある（Story が確認を待ちはじめると自動で作り直す）。人間が「確認を始める」で固定した後は置き換えられない | Q2 | `.harness2/graph/harness_graph/verify.py` の冒頭（30〜32 行）と 305 行 | そのとおり | 実物（`.harness2/graph/harness_graph/verify.py:305`） | 2 |
| 手順書の問いは harness2 の `verify_checklist.py` が組む。プロジェクトが足せるのは `manifest.md` の段落（冒頭の note）だけで、答えられる問いは足せない | Q1（人間に回すものを問いにするには harness2 の変更が要る） | `.harness2/scripts/verify_checklist.py` の 550〜590 行（引数は tag・Story・`--manifest`・`--carried`） | そのとおり | 実物（`.harness2/scripts/verify_checklist.py:583`） | 2 |
| ashiato2 の repo は公開。手順書の問い（`docs/verify/<tag>.json`）と答え（`docs/verify/<tag>-answers-<n>.txt`）は commit されて main に入る。渡す HTML（`docs/briefs/`）と成果物（`dist/`）は git が無視する | Q4（公開の repo に出すもの） | `gh repo view` の visibility・`git log -- docs/verify/20261118-0905.json`・`.gitignore` の 34・37 行 | 公開（PUBLIC）。`docs/verify/20261118-0905.json` は main にある | 実物 | 5 |
| 確認バッチの「違和感」の問いが通らなかったものは、台本（`verify_iwakan.py`）がラベル「違和感」の issue を公開の repo に立てる | Q4・Q1（前からある違和感を受け皿の依頼にする口） | `.harness2/scripts/verify_iwakan.py` の冒頭と、`gh issue list -l 違和感` | 立っている（#60「ST12 違和感（確認バッチ 20261006-0754）: 通らなかった」ほか） | 実物 | 5 |
| 席の台本は、公開してよいものを `public.json`（止まり方・件数・題）だけと決めている。候補の本文・画像・ログは repo の外の実行のフォルダに置く | Q4 | `.harness2/scripts/tsukau.py` の冒頭（実行のフォルダの表） | そのとおり。題は 1 行 80 字まで | 公式文書（`.harness2/scripts/tsukau.py:40`） | 2 |
| 確認バッチの DB は、立てたとき偽データ（`SEED=normal|max|empty`）を入れるが、確認の間に本人の携帯が送った記録も入り、確認の後も残る。作業場所には main の checkout の本物の `.env`（合言葉）を写す | Q4（題や根拠に出うる値）・Q8（席が使う DB と合言葉の既定） | `tools/verify-prep.sh` の 23〜33・131・147 行 | そのとおり。席が確認バッチの DB と作業場所をそのまま使うと、本物の記録と合言葉に触れうる | 実物（`tools/verify-prep.sh:147`） | 2 |
| 席（`/tsukau seat`）は案内だけを見て利用者として触り、コード・試験・仕様は読まない。直さない | Q3（本物かを決める席が何を読むか） | `.harness2/README.md` の「使ってみる席」の表（586 行） | そのとおり | 公式文書（`.harness2/README.md:586`） | 2 |
| 席の台本 `scripts/tsukau.py run` の差し込み口は `tools/tsukau/tsukau.json` と `env_hook up/down`、公開してよいのは `public.json`（止まり方・件数・題）だけ | 差し込み口の形・手順書に写すもの | harness2 の `scripts/tsukau.py` 冒頭と README「使ってみる席」を読んだ | そのとおり。rc 2 は引数・設定の誤り、rc 3 は crashed / env_failed / interrupted | 公式文書（`.harness2/scripts/tsukau.py:1-80`） | 2 |
| `tools/verify-prep.sh` は `<tag>` だけを受け取り、束ねた Story の一覧を受け取らない。動くのは確認バッチの作業ツリー（`verify/<tag>`。origin/main に各 PR の head を merge した木） | 範囲（その回で足した機能）の求め方 | `.harness2/scripts/verify_batch.sh` の 186〜196 行と 117〜160 行を読んだ | 引数は `<tag>` だけ。`--story` で足した（すでに main の）Story は木の差分に出ない | 実物（`.harness2/scripts/verify_batch.sh:189`） | 2 |
| `manifest.md` は確認手順書の冒頭と verify PR の本文の両方に載る | 件数・題を手順書と PR 本文に写す口 | `verify_batch.sh` の 215 行（`--manifest`）と 241 行（本文に `cat`） | 両方に載る。段落ごとに手順書の note になる | 実物（`.harness2/scripts/verify_batch.sh:241`） | 2 |
| Windows の `wsb` CLI で、窓を開かずに Sandbox を起こし（`wsb start -c <設定>`）、中でコマンドを走らせられる（`wsb exec`） | PC の相手で、席が Sandbox の中を操作する道 | 2026-11-20 にこの PC で `wsb start`（網あり・書ける共有フォルダ 1 つ）→ `wsb exec -r System` で PowerShell の台本を走らせ、結果を共有フォルダに書かせた | 走る。**`wsb exec` は終了コードしか返さない**（出力は共有フォルダのファイルで受け取る）。`-r System` は窓を開かなくても走る | 実物 | 5 |
| 画面の写しを撮る・利用者として操作するには、Sandbox の窓を開く（`wsb connect`）必要がある | Q9（Sandbox の窓） | 同じ Sandbox で、窓を開く前と後に `wsb exec -r ExistingLogin` と画面の写しを試した | 窓を開く前: `ExistingLogin` は「ログオン セッションが存在しない」で走らず、`System` では画面の写しが「handle is invalid」で撮れない。`wsb connect` で窓を開いた後: `ExistingLogin` で走り、画面の写し（2032×1111）が撮れた。窓は本人のデスクトップに出る | 実物 | 5 |
| 網をつないだ Sandbox から、外・私設網（tailscale）・手元の網に届く | Q8（Sandbox の網）の代償 | 同じ Sandbox の中から TCP でつないだ | `api.github.com:443` に届き HTTPS は 200。**私設網の WSL（`<私設網の IP>:22`）とこの Windows（`<私設網の IP>:445`）にも届いた**。手元の網の `<手元の網の宛先>:80` は届かなかった（その宛先に何も無いだけかもしれない） | 実物 | 5 |
| Sandbox の中のファイアウォールで、私設網と手元の網への宛先だけを塞げる | Q8 の推奨の案 | 確かめていない | 不明。塞げなければ網を切る側に倒す | 知識 | 1 |
| 録画は毎回まっさらな Windows Sandbox を起こし、網を切って（`<Networking>Disable`）中のデスクトップを撮る。始める前に別の Sandbox が動いていれば撮らずに止まり、自分のシナリオの後と片付けでは動いている Sandbox を全部止める | 録画との取り合い | `.harness2/scripts/record-run` の 263・265・281〜290・317・348・571〜573 行 | そのとおり。席の片付けが Sandbox を閉じ損ねると同じバッチの PC の録画が撮れない。手で呼んだ席の Sandbox は録画の片付けに止められる | 実物（`.harness2/scripts/record-run:317`） | 2 |
| Windows の配る zip（サーバ・PC の収集アプリ）は CI の job（`package-windows`）が組むが、確認バッチの準備はその CI より前に走る。手元（WSL から Windows 側の cargo）で組んだ実績は収集アプリの exe だけ（録画） | Q7（配る zip の出所） | `.github/workflows/ci.yml:541`（`package.sh windows`）・`.harness2/scripts/verify_batch.sh` の並び（準備 → push → CI）・`tools/record-env.sh` | そのとおり。**手元で zip 全体を組めるか・何分かかるかは確かめていない** | 実物（ファイル）／組めるかは未確認 | 2 |
| この PC の Windows 側に、配る zip を組む台本が前提にする道具がある（Git Bash の `bash.exe`・Visual Studio の `vswhere.exe`・VC の再頒布物の `Microsoft.VC143.CRT`） | Q7（手元で組む） | 2026-11-20（第 2 版の spec の段）に `/mnt/c/Program Files/Git/bin/bash.exe`・`/mnt/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe`・`…/2022/BuildTools/VC/Redist/MSVC/14.44.35112/x64/Microsoft.VC143.CRT` を `ls` で見た | 3 つとも有る。**組み立てを走らせてはいない**（何分かかるか・PostgreSQL の取得が通るかは下流で測る） | 実物（ファイルの有無） | 2 |
| 網を切った Windows Sandbox で、配る形のインストーラ・サーバ・DB が入り、中のサーバと PC の収集アプリが loopback で通じる | Q8 の推奨（網を切る） | 確かめていない（録画は網を切った Sandbox に PC の収集アプリだけを入れ、サーバは外に置いた） | 不明。下流の最初の PC の Task で確かめる。通らなければ A として人間に返す | 知識 | 1 |
| 0.1.0 の更新係の panic（#329）は、本人が自分の PC に配る形を入れて見つけた。外への HTTPS の握手で初めて起き、配る組み立ての CI は外への接続を塞いでいて素通りした。更新係は 1 時間ごとに確かめる | Q8（Sandbox の網）の得 | PR #329 の本文と `crates/updater/src/lib.rs:305` を読んだ | そのとおり（`api.github.com` との握手で panic） | 実物（GitHub の PR #329） | 2 |
| 確認バッチの候補は 1 日に 1〜5 回作り直される | Q11（作り直しのたびに回すか）の費用 | `gh pr list` で `verify/` の PR の作られた日を数えた（10-30〜11-19） | 1 日 1〜5 本（平均 2 本ほど） | 実物 | 4 |
