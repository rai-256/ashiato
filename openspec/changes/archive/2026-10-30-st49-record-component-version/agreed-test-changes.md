# 合意したテストの書き換え（期待は同じ・書き方だけ）

- Scenario: 初回の起動で写真と動画と写真の位置の権限を求める — 期待は変えていない。PermissionDeniedInstrumentedTest の `denyMediaDialog` は、前と同じく「写真と動画の権限の画面（permissioncontroller の拒否の釦）が DIALOG_TIMEOUT_MS 内に出なければ、同じ文言で assertTrue が落とす」。変えたのは、見つけてから押すまでにダイアログが描き直されて掴んだ要素が古くなった（StaleObjectException。CI run 40850941469）ときに探し直して押す（3 回まで）書き方だけで、`assertTrue(…)` は共通の `findAndClick` へ移した。古くなった要素は押す前に位置を読むところで落ちるので、2 度押しにはならない（commit adcdbb1a）
