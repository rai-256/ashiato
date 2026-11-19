# CH23 の前提（外の形式・振る舞い）

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| `gh label list --search <語> --json name` は、当てはまるラベルが無いと**空の文字**（0 バイト）を返し rc=0 | 偽の `gh` の `label list` の返し方・`tools/check-vulns-issues.py` の読み方（D1） | 2026-11-20 に gh 2.104.0 で `gh label list -R rai-256/ashiato --search zzznonexist --json name \| od -c` | 0 バイト・rc=0 | 実物 | 5 |
| `gh issue list --json …` は、当てはまる issue が無いと `[]` を返す | 偽の `gh` の `issue list` は今のまま（`[]`）でよい | 同じ日に `gh issue list -R rai-256/ashiato --state open --label zzznonexist --json number,title,body \| od -c` | `[]\n` | 実物 | 5 |
| 毎晩の検査は 2026-11-10 から赤で、11-10〜11-18 の schedule の 9 回は `issues` の job も failure。11-19 の手動の起動（ラベルを手で作った後）は `issues` が success で、#334〜#337 が立った | 直す対象が「知らせの段」であること・赤の色そのものは知らせにならないこと（D4） | `gh run list --workflow vulns-nightly.yml` と `gh run view --json jobs`・`gh issue list --label 脆弱性 --state all` | 記載のとおり（11-19 の run も `vulns`・`pg-minor` は failure） | 実物 | 5 |
| 予定した workflow の知らせは、workflow のファイルの cron を最後に変えた人に送られる | 9 晩の赤がどこに届いていたか（`vulns-nightly.yml` を最後に変えた commit はリポジトリの持ち主 本人。2026-11-09） | 2026-11-20 に github/docs の原稿（`content/actions/reference/workflows-and-actions/events-that-trigger-workflows.md:1052`）を `gh api … -H 'Accept: application/vnd.github.raw'` で読んだ。逐語: 「Notifications for scheduled workflows are sent to the user who last modified the cron syntax in the workflow file.」（公開の頁は https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows） | 本人に送られていた見込み。届いたか・埋もれたか・知らせの設定（失敗だけ・メールか web か）は確かめていない（確認バッチで聞く） | 公式文書 | 2 |
| `if:` に状態の関数（`success()`・`failure()`・`always()`・`cancelled()`）が無いと、`success()` が暗に足される（前の段が落ちた後は走らない） | 知らせの段の `if:` に `failure()` が要ること（Scenario「知らせの段は、材料を issue にする段が落ちたときだけ走る」） | https://docs.github.com/en/actions/reference/workflows-and-actions/expressions の逐語: 「A default status check of `success()` is applied unless you include one of these functions.」 | 記載のとおり | 公式文書 | 2 |
| Actions（`github-actions[bot]`）が立てた issue の知らせは、リポジトリを watch している本人に届く | 知らせの経路を issue にしてよいか（D2） | 公式文書では見つからなかった（watch の既定・bot が立てた issue のどちらも書いていない）。確認バッチで本人に #334〜#337 の知らせが届いたかを聞く | 確かめていない | 知識 | 1 |

`loss` のある決定はこの change に無い（どれも台本の書き直しで戻る）。知らせが本人に届くか（確かさ 2 と 1 の行）には決定を載せず、確認バッチの問い（`docs/changes/CH23.md` の完了の判定 5）で確かめる。
