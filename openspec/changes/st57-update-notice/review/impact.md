# impact-review — st57-update-notice

seen: IMP-01 medium, IMP-02 medium, IMP-03 low, IMP-04 low, IMP-05 low

## R1 — 「ST55 のインストールに新しい zip を走らせ直すと更新係が足される」は、いまのインストーラでは「別の版が入っています」で止まる
- 対象: impact.situations[carryover].can（と tasks.md 11.2・design.md:199・targets T1 の searched）
- 種類: 新しい帰結（根拠: can が Scenario の写し）
- 指摘: `carryover.can` は Scenario「更新係の無いインストールを走らせ直すと、更新係が足される」の写しで、その場面で本当にできるかを確かめていない。この Scenario の GIVEN は「試験の組み立ての差し込みで更新係を消した形」＝同じ版の zip を走らせ直す形で、本人の実際の経路（ST55 で入れたものに新しい版の zip を重ねる）とは別物。`crates/installer/src/main.rs:773-797`（`copy_program_files`）は `commands::program_files()`（commands.rs:738。`ashiato-server.exe`・`ashiato-setup.exe`・`db-roles.sql`・`pgsql`）の既存ファイルが zip のものと内容違いなら `program_version_mismatch_message`（commands.rs:215「版を替える口はまだ無いので、何も替えずに止めます」）で止まる。新しい zip の `ashiato-setup.exe`・`ashiato-server.exe` は版の印が違うので必ず内容が違う。これを変える Task が tasks.md / design.md D7 に無い。つまり本人の PC の「走らせ直しで更新係が足される」は動かず、通るのは消して入れ直す道（状態の置き場は残る）だけ。tasks.md 11.2 はこの前提で書かれていて、実地で初めて落ちる。lost / dependencies にも無い。
- 直し方の案: dependencies に「ST55 のインストーラの版違いで止める検査（copy_program_files）」を足し、新しい版の上書きを今回の範囲に入れるか（Scenario を版違いの zip で固定）、「消して入れ直す」を手順にして carryover.can を書き直す。
- 処置: fixed: 依存 D8（copy_program_files・E10/E11）と IMP-06（medium）を足し、carryover の can / cannot を「消して入れ直す」に書き直し、open に仕様と移行の計画の食い違いを置いた（仕様は reading の段で直さない）

## R2 — IMP-02 の unknowns の 1 つ目は、コードで答えが出る。S1 の「走らせ直しても更新係は動き続けうる」は前の版では先に止まる
- 対象: impact.findings[IMP-02].unknowns / scenarios[S1].result
- 種類: 根拠
- 指摘: 「前の版のインストーラが Program Files を上書きするとき…」は未確認としているが、`main.rs:773-797`（R1 と同じ）で、前の版のインストーラは新しい版が入った置き場に対して `ashiato-setup.exe` 等が違うため「別の版が入っています」で止まり、何も上書きしない。S1 の action「前の版の zip を展開してインストーラを走らせ直し」は、更新係を足す前に止まる経路で、「走らせ直しの後も更新係が動き続けうる」は止まっただけの結果（更新係は元から動いたまま）。実際に起きる筋は「前の版のインストーラで uninstall → 登録が残る」だけ。また commands.rs:225-231 の `uninstall_commands` は本当に 2 つのサービスだけだが、その後プログラムの置き場を消す（同 215 行近辺のコメント）ので `ashiato-updater.exe` は消え、残るのは「実行ファイルの無いサービス登録」。S1 の result と not_when にこの区別が無い。重大度の medium は妥当。
- 直し方の案: S1 を「前の版で uninstall」だけに絞り、unknowns の 1 つ目を「止まる（main.rs:792）」で閉じ、残る登録が「実行ファイル無しで起動に失敗し続ける」ことを verify に書く。
- 処置: fixed: S1 を「前の版で消す」に絞り、走らせ直しは版違いで止まることを action に書いた。IMP-02 の unknowns の 1 つ目を閉じ、残るのが実行ファイルの無い登録であることを text と verify に書いた

## R3 — IMP-04 / D6 の前提「ST55 の録画の台本は配る zip を入れる」は、台本の一覧に見つからない
- 対象: impact.dependencies[D6]・evidence[E8]・findings[IMP-04]
- 種類: 根拠
- 指摘: D6 の who は「tools/recording/scenarios.d。ST55 の録画」とするが、`tools/recording/scenarios.d/` に st55 の台本は無く、`ashiato-setup`・install・zip を含む台本も無い（grep 0 件。st56 は PC の収集アプリの `setup`）。Windows の録画は `tools/recording/recording.json:115` で `"networking": "disable"`。E8 は ST57 自身の design.md の文で、台本の実物ではない（「実物を読んだ」根拠になっていない）。unknowns の「録画の環境が外への接続を持つか」は recording.json で既に答えが出ており、PC の収集アプリの録画なら外に出ない。basis observed / holds unknown は支えられず、IMP-04 は「該当する台本が見つからない」か、あれば guess にするのが正しい。
- 直し方の案: D6 を台本の実物（無ければ削除）で書き直し、IMP-04 を閉じるか、サーバの zip を入れる録画の有無を確かめて unknowns に残す。
- 処置: fixed: D6 を recording.json の networking: disable（E12）で書き直して holds: yes にし、E8 を外した。IMP-04 は dismissed（dismissed_by: E12）にした

## R4 — IMP-01 の「3 つ目の選択肢（受け入れる）」と S2 の「入れ替えの通知は出ない」は、保存先の食い違いを抱えたまま
- 対象: impact.scenarios[S2].result / findings[IMP-01].options
- 種類: 根拠
- 指摘: S2 は「入れ替えの通知は前に出したので出ない」と言うが、その印 `intro_shown` は design.md:182 では PC の `update-check.json` に持つ。一方、切った値は spec（specs/distribution/spec.md:483）で `settings.json` にだけ持つ。印が別ファイルにあるのは、IMP-01 の option 2（update-check.json に切った値を持つ）が既に半分できている形で、option 2 は新しいファイルを作る案ではなく、既存の `update-check.json` に値を足す案になる。`SettingsFile`（config.rs:415）のコード読みは正しい（既知でない欄は serde で捨て、config.rs:199-206 で書き直す）。ただ S2 の result はこの 2 ファイルの食い違い（切った値は消え、印は残る）が起きる条件だと明記しておらず、option 2 のコストを過大に見せている。
- 直し方の案: option 2 を「`update-check.json` に切った値を移す」と書き直し、S2 に 2 ファイルの状態を明記する。
- 処置: fixed: IMP-01 の option 2 を「update-check.json に移す」に書き直し、S2 の result に 2 つのファイルの状態（切った値は消え、intro_shown は残る）を書いた

## R5 — 重大度: 現状の medium は妥当。ただし R1 は medium 以上の IMP として加える必要がある
- 対象: impact.findings（全体）
- 種類: 重大度
- 指摘: R1 は、既存の本人（ST55 の利用者）に対して「この Story の目玉である更新係が、仕様の見える道（走らせ直し）では付かない」もので、本人は消して入れ直す以外に続けられないと気づく前に実地で落ちる。低くとも medium。IMP-03〜05 の low は、根拠の薄さ（IMP-05 は guess、IMP-04 は R3 のとおり）に見合っている。
- 直し方の案: R1 を IMP-06 として追加する。
- 処置: fixed: R1 を IMP-06（medium）として足した。ほかの重大度は変えない

