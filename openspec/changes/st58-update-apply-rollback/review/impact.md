# impact-review — st58-update-apply-rollback

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 medium, IMP-05 medium, IMP-06 low

## R1 — 更新の履歴の「前の版」を読む処理が rolled_back を数えず、戻した後の次の更新の from_version を誤りうる（依存の見落とし）
- 対象: impact.dependencies（T3 の読む側が無い）/ impact.targets[T3]
- 種類: 依存の見落とし
- 指摘: crates/server/src/update_history.rs:127-136 の `previous_version` は `WHERE outcome IN ('applied','nothing_to_apply') ORDER BY seq DESC LIMIT 1` で「今の版」を決め、次の更新の `from_version` に使う。design D26（design.md:190）は、戻した回に `applied`/`nothing_to_apply` を「当てた新しい版の名前」で書き直してから `rolled_back` を足す。`rolled_back` はこの IN に入らないので、戻した後の最後の通った行は落ちた新しい版を指し、次に通る更新の「前の版」が実際に動いていた版と食い違う。D5（T3）の dependencies は「前の版の DB が rolled_back を拒む」「書き出しを読む側」だけで、サーバ自身のこの読みを挙げていない。tasks/spec にもこの読みの直しは見当たらない（grep で previous_version / from_version の記述なし）。版の記録は凍結した表で後から直せない（memory の one_way_door と同じ性質）。
- 直し方の案: dependencies に `previous_version` を読む側として足し、rolled_back の行の to_version を「今の版」とみなす（または IN に足して seq で並べる）かを design で決め、Scenario に戻した後の更新の from_version を足す。
- 処置: fixed: dependencies に D13（update_history.rs の previous_version）と E14 を足し、IMP-07（medium）として findings・scope.residual・acceptance・overview に出した。直し方の案は IMP-07 の options

## R2 — IMP-04 の「止まる」は unknowns と食い違う言い切り（言い切り）
- 対象: impact.overview / impact.scenarios[S1].result / impact.findings[IMP-04].unknowns
- 種類: 言い切り
- 指摘: overview は「直した版が出るまでサーバは止まる」、S1 は「0.5.0 のまま落ち続ける」と断定するが、IMP-04 の unknowns は「新しい移行の無い版なら前の版のインストーラで走らせ直すと前の版で起きうるか（…確かめていない）」と書く。design.md:157 も、段が `listening` なら「どちらの版のインストーラも 8 の残りを進めて終える」と読め、前の版のインストーラを走らせ直す道が実際に何をするかは未確認。止まる・戻れないを確かめずに断定している一方、IMP-04 の options 2 番目（プログラムだけ戻す口）の要否もここで決まる。
- 直し方の案: overview・S1 に「走らせ直しで前の版が起きるかは未確認」と限定を足すか、Windows の通しで確かめてから断定する。
- 処置: fixed: overview と S1 の言い切りを外し、前の版のインストーラで走らせ直したときは確かめていないと書いた。IMP-04 の unknowns に印が残っている場合を足した

## R3 — D9（PC の同時起動）の basis と evidence が、IMP-05 の guess と不整合（根拠）
- 対象: impact.dependencies[D9] / impact.findings[IMP-05]
- 種類: 根拠
- 指摘: D9 は `holds: no`・`basis: observed` だが、E10（autostart.rs:114 の `current_exe`）が示すのは「自動起動の台本は動いている実行ファイルを指す」ことだけで、「入れ替え後も本人が置いた場所の実行ファイルが前の版のまま残る」は design D12（仮）の設計の予定にすぎない（まだコードに無い）。E11 も premises の記述。実物で観測できた事実ではないので、IMP-05 が `guess` なのに D9 を `observed`・`holds: no` と置くのは強すぎる。
- 直し方の案: D9 を `basis: derived` か `holds: unknown` にそろえる。
- 処置: fixed: D9 を basis derived・refs D12 にし、assumes に「design D12（仮）の予定」と書いた

## R4 — mixed の can「前の版の収集アプリの未送信は新しい版のサーバに届く」を支える ref が無い（根拠）
- 対象: impact.situations[mixed].can
- 種類: 根拠
- 指摘: refs はインストーラの Requirement と D1 だけで、サーバが前の版の収集アプリの送信契約を受け付けることを示す Scenario/実物が挙がっていない。spec の近い Scenario は PC の入れ替え前後の未送信（distribution/spec.md:128, 134）で、サーバの更新後に旧版の収集アプリの送信が届く場面を扱っていない。仕様の写しではなく、確かめていない「できる」。
- 直し方の案: 該当 Scenario を refs に足すか、can から外して未確認と書く。
- 処置: fixed: situations[mixed].can を確かめていないこととして書き直し、basis を unconfirmed にした

## R5 — R2・R7・R10・R11・R12・R13 の legit「見つけていない」の探し方が示されていない（言い切り）
- 対象: impact.rules[R2,R7,R10,R11,R12,R13].legit
- 種類: 言い切り
- 指摘: R2 の「正当な利用を止める場面を見つけていない」は限定付きで書かれているが、R2 は「この回の取り終えた退避が無い回」を戻さない規則で、MIGRATE_BACKUP=off（migrate_backup.rs:28）がシステムの環境に付いたインストールや退避が取れなかった回の扱いは確かめた形跡が無い（design D26 は backup_failed を移行が DB に触れない前提で外している）。確かめた範囲を書かないと「見つけていない」が検索の結果か、確かめなかっただけか分からない。
- 直し方の案: R2 の legit に、確かめた場面（退避が無効・退避の失敗・終了コード 2〜4）を書く。
- 処置: fixed: R2 の legit に確かめた場面（終了コード 2・3・4・退避の前に落ちた回・退避に失敗した回）と確かめていない場面（退避を切ったインストール）を書いた。R7・R10〜R13 は当てはめた範囲（Scenario と利用場面の 6 つ）を添えた

## R6 — IMP-02 の消した記録が残る期間は、既にある毎日のバックアップとの差が書かれていない（重大度）
- 対象: impact.findings[IMP-02] / impact.memory[0]
- 種類: 重大度
- 指摘: memory[0] の `why` は「消したつもりの記録が同じ PC に残る期間がある」と書くが、毎日のバックアップ（30 日分。memory[2] 自身が言及）も削除を巻き戻さない写しである。IMP-02 は追加で増える分（落ちた DB と退避の数か月）に絞れており、medium は妥当だが、unknowns の「落ちた DB を書き出しやバックアップが読むか」は書き出し（export.rs の対象表）を見れば機械的に確かめられる。確かめられることを unknown のまま残している。
- 直し方の案: export.rs とバックアップの対象を見て unknowns を閉じるか、「確かめていない」と明記する。
- 処置: fixed: 書き出しはサーバが繋ぐ記録の DB だけを読むことを IMP-02 に書き、unknowns から外した。確かめる途中で、毎日のバックアップ（ST30）がまだ作られていないと分かった。仕様の「30 日分は別に残る」との食い違いとして open に出し、keep.1・memory・R4・S3 を直した

