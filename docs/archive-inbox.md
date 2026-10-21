# 書庫を置く手順

書庫の読み手は、利用者と2つの置き場を環境変数で指定して起動する。

```text
ASHIATO_ARCHIVE_USER_ID=00000000-0000-0000-0000-000000000000
ASHIATO_INBOX_DIR=<専用の置き場>
ASHIATO_DOWNLOADS_DIR=<ダウンロードのフォルダ>
ASHIATO_ARCHIVE_COPY_DIR=<写しの置き場>
ASHIATO_ARCHIVE_KEEP_COPIES=true
```

置き場と写しの置き場は**絶対パス**で書く（相対パスなら起動を止める）。書かなければ
`%USERPROFILE%\Documents\ashiato\取り込み待ち` / `%USERPROFILE%\Downloads` /
`%LOCALAPPDATA%\ashiato\archive-copies`（Linux では `$HOME` の下の同じ並び）になる。
起動した場所に依らないので、サービスやタスクスケジューラから起動しても写しを見失わない。

置き場を見回る間隔は環境変数でなく**登録簿**（`core.source` の `s01-archive-inbox` の `fetch_interval_sec`、初期値 120 秒、上限 300 秒）で変える。
取り込み器は 1 秒ごとに登録簿を読むので、起動し直さずに 1 秒以内に効く。環境変数で指定して起動すると、設定先を示して起動を断る。

`ASHIATO_ARCHIVE_USER_ID` は **`.env` の `ASHIATO_USER_ID` と同じ値**にする（利用者は 1 名。
いまは `00000000-0000-0000-0000-000000000000`）。画面は利用者を名乗らずに読む（既定の利用者を見る）ので、
別の値を入れると読んだ書庫が画面にも稼働状況の格子にも出ない。

`tools/archive-shape.sh` にも、`DATABASE_URL` と一緒に同じ `ASHIATO_ARCHIVE_USER_ID` を環境で渡す。
手元に `psql` が無ければ、道具は開発用コンテナ（`docker compose` の `db`）の `psql` を使う（repo の根の compose を見る）。

Google Takeout は予約エクスポートで作り、形式は **JSON** を選ぶ。届いた
`takeout-*.zip` はダウンロードのフォルダに残したまま読まれる。端末から書き出した
`Timeline.json` は専用の置き場へ置く。PCへ運ぶときは Tailscale のファイル送信または
USB など、網の外へ出ない手段を使う。

最初の Takeout は「形の確認待ち」として表示される。値を出さない
`tools/archive-shape.sh` の一覧を確認し、問題なければ
`tools/archive-shape.sh --confirm <形のハッシュ>` で印を置く。新しい製品、初めての
中身、または書き出しの言語を変えた場合は、再び確認待ちになる。

YouTube の形の一覧には、値を出さずに次の 3 つが出る。

- `products`: ファイルが名乗る製品の名前（`YouTube`）。
- `languages`: 題名の決まり文句の表に当たった言語（`ja` / `en`）。**空（`[]`）なら印を置けない**。
  `--confirm` は印を置かずに断る。表にその言語を足して解析器の版を上げると、新しい言語の形に印を置ける
  （待つあいだ、ファイルは置き場に残り、写しも残る）。
- `destinations`: 行き先ごとの件数（`watch` / `search` / `other` / `ad` / `rating`）。`rating` が 0 なら、
  評価の決まり文句の表が外れている合図なので、印を置く前に `rating` の件数が想像と合うか見る。

専用の置き場で読み終えた書庫は `取り込み済み` に移る。システムは本人の書庫を削除しない。
