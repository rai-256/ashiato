# 書庫を置く手順

書庫の読み手は、利用者と2つの置き場を環境変数で指定して起動する。

```text
ASHIATO_ARCHIVE_USER_ID=<利用者 UUID>
ASHIATO_INBOX_DIR=<専用の置き場>
ASHIATO_DOWNLOADS_DIR=<ダウンロードのフォルダ>
ASHIATO_ARCHIVE_COPY_DIR=<写しの置き場>
ASHIATO_ARCHIVE_KEEP_COPIES=true
```

Google Takeout は予約エクスポートで作り、形式は **JSON** を選ぶ。届いた
`takeout-*.zip` はダウンロードのフォルダに残したまま読まれる。端末から書き出した
`Timeline.json` は専用の置き場へ置く。PCへ運ぶときは Tailscale のファイル送信または
USB など、網の外へ出ない手段を使う。

最初の Takeout は「形の確認待ち」として表示される。値を出さない
`tools/archive-shape.sh` の一覧を確認し、問題なければ
`tools/archive-shape.sh --confirm <形のハッシュ>` で印を置く。新しい製品、初めての
中身、または書き出しの言語を変えた場合は、再び確認待ちになる。

専用の置き場で読み終えた書庫は `取り込み済み` に移る。システムは本人の書庫を削除しない。
