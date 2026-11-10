spec-digest: 28915c92cf103a4fb677d47778ace5604d43fa3beb6153d0fcdbe3f220d160c2
- Q2: 写った
答え以外の変更: あり（D8（仮）の子の出力の読み方（UTF-8 として正しければそのまま・そうでなければ窓のコード頁・読めない並びは MB_ERR_INVALID_CHARS と IsDBCSLeadByteEx で 1 文字ずつ U+FFFD にして止まらない）と、D9（仮）の確かめ方（package-windows の単体の試験を --lib なしで数える・出力の末尾を組む関数を lib へ移し窓のコード頁を選ぶ試験を console.rs に置く・読めない並びの試験・install-check の英字でない名前の管理者の repair と、失敗時に出すもの・cp932 のバイト列の作り方）。これに従う ADDED の SHALL と Scenario の具体・tasks の Task 5・6・Global Constraints・Task 4.2 の検証の形・premises.md の 4 行・Risks の 1 行・proposal と CH14.md の完了の判定の行。Task 5・6 を差し込んだことによる番号の付け直し（Task 5→7・6→8・7.1→9.1）と、D7（仮）の反転条件・CH14.md の確認バッチの行の 7.1→9.1 の参照の直し）
答え以外の変更の決定: D7 D8 D9
