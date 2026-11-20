# CH21 の前提（外の形式・振る舞い）

調べた日: 2026-11-20。失われるもの（`loss`）のある決定は Q2（Sandbox の網）。Q2 が依る前提（Sandbox から外・私設網・手元の網に
届くか）は、この PC で Sandbox を起こして確かめた（確かさ 5）。「網はつなぐが私設網と手元の網を塞ぐ」案の、塞げるかどうかは
確かめていない（確かさ 1）—— その案は「塞げなければ網を切る側に倒す」を含めて推奨にしている。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| 席の台本 `scripts/tsukau.py run` の差し込み口は `tools/tsukau/tsukau.json` と `env_hook up/down`、公開してよいのは `public.json`（止まり方・件数・題）だけ | 差し込み口の形・手順書に写すもの | harness2 の `scripts/tsukau.py` 冒頭と README「使ってみる席」を読んだ | そのとおり。rc 2 は引数・設定の誤り、rc 3 は crashed / env_failed / interrupted | 公式文書（`.harness2/scripts/tsukau.py:1-80`） | 2 |
| `tools/verify-prep.sh` は `<tag>` だけを受け取り、束ねた Story の一覧を受け取らない。動くのは確認バッチの作業ツリー（`verify/<tag>`。origin/main に各 PR の head を merge した木） | 範囲（その回で足した機能）の求め方 | `.harness2/scripts/verify_batch.sh` の 186〜196 行と 117〜160 行を読んだ | 引数は `<tag>` だけ。`--story` で足した（すでに main の）Story は木の差分に出ない | 実物（`.harness2/scripts/verify_batch.sh:189`） | 2 |
| `manifest.md` は確認手順書の冒頭と verify PR の本文の両方に載る | 件数・題を手順書と PR 本文に写す口 | `verify_batch.sh` の 215 行（`--manifest`）と 241 行（本文に `cat`） | 両方に載る。段落ごとに手順書の note になる | 実物（`.harness2/scripts/verify_batch.sh:241`） | 2 |
| Windows の `wsb` CLI で、窓を開かずに Sandbox を起こし（`wsb start -c <設定>`）、中でコマンドを走らせられる（`wsb exec`） | PC の相手で、席が Sandbox の中を操作する道 | 2026-11-20 にこの PC で `wsb start`（網あり・書ける共有フォルダ 1 つ）→ `wsb exec -r System` で PowerShell の台本を走らせ、結果を共有フォルダに書かせた | 走る。**`wsb exec` は終了コードしか返さない**（出力は共有フォルダのファイルで受け取る）。`-r System` は窓を開かなくても走る | 実物 | 5 |
| 画面の写しを撮る・利用者として操作するには、Sandbox の窓を開く（`wsb connect`）必要がある | Q4（Sandbox の窓） | 同じ Sandbox で、窓を開く前と後に `wsb exec -r ExistingLogin` と画面の写しを試した | 窓を開く前: `ExistingLogin` は「ログオン セッションが存在しない」で走らず、`System` では画面の写しが「handle is invalid」で撮れない。`wsb connect` で窓を開いた後: `ExistingLogin` で走り、画面の写し（2032×1111）が撮れた。窓は本人のデスクトップに出る | 実物 | 5 |
| 網をつないだ Sandbox から、外・私設網（tailscale）・手元の網に届く | Q2（Sandbox の網）の代償 | 同じ Sandbox の中から TCP でつないだ | `api.github.com:443` に届き HTTPS は 200。**私設網の WSL（`<私設網の IP>:22`）とこの Windows（`<私設網の IP>:445`）にも届いた**。手元の網の `<手元の網の宛先>:80` は届かなかった（その宛先に何も無いだけかもしれない） | 実物 | 5 |
| Sandbox の中のファイアウォールで、私設網と手元の網への宛先だけを塞げる | Q2 の推奨の案 | 確かめていない | 不明。塞げなければ網を切る側に倒す | 知識 | 1 |
| 録画は毎回まっさらな Windows Sandbox を起こし、網を切って（`<Networking>Disable`）中のデスクトップを撮る。始める前に別の Sandbox が動いていれば撮らずに止まり、自分のシナリオの後と片付けでは動いている Sandbox を全部止める | 録画との取り合い | `.harness2/scripts/record-run` の 263・265・281〜290・317・348・571〜573 行 | そのとおり。席の片付けが Sandbox を閉じ損ねると同じバッチの PC の録画が撮れない。手で呼んだ席の Sandbox は録画の片付けに止められる | 実物（`.harness2/scripts/record-run:317`） | 2 |
| Windows の配る zip（サーバ・PC の収集アプリ）は CI の job（`package-windows`）が組むが、確認バッチの準備はその CI より前に走る。手元（WSL から Windows 側の cargo）で組んだ実績は収集アプリの exe だけ（録画） | Q3（配る zip の出所） | `.github/workflows/ci.yml:541`（`package.sh windows`）・`.harness2/scripts/verify_batch.sh` の並び（準備 → push → CI）・`tools/record-env.sh` | そのとおり。**手元で zip 全体を組めるか・何分かかるかは確かめていない** | 実物（ファイル）／組めるかは未確認 | 2 |
| 0.1.0 の更新係の panic（#329）は、本人が自分の PC に配る形を入れて見つけた。外への HTTPS の握手で初めて起き、配る組み立ての CI は外への接続を塞いでいて素通りした。更新係は 1 時間ごとに確かめる | Q2（Sandbox の網）の得 | PR #329 の本文と `crates/updater/src/lib.rs:305` を読んだ | そのとおり（`api.github.com` との握手で panic） | 実物（GitHub の PR #329） | 2 |
| 確認バッチの候補は 1 日に 1〜5 回作り直される | Q6（作り直しのたびに回すか）の費用 | `gh pr list` で `verify/` の PR の作られた日を数えた（10-30〜11-19） | 1 日 1〜5 本（平均 2 本ほど） | 実物 | 4 |
