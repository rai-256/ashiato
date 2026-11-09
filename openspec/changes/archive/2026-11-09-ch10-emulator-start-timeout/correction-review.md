# 訂正の確かめ（code_verify 手 7）

## 5.1 2026-11-07T05:36:32+00:00
- 壊したもの: 5.1 が守る文書の 1 つ、docs/testing.md:177 の「手元: `./tools/android-emulator.sh`」の行の「同じ AVD」を「同一の AVD」に書き換えた（作業ツリーで一時的に。別に AGENTS.md の「error: /dev/kvm を開けない」を「error: kvm を開けない」にした場合も rc=1）。どちらも元に戻し、git status --porcelain と git diff が壊す前と同じことを cmp で確かめた
- 実行: ./openspec/changes/ch10-emulator-start-timeout/check-5.1.sh
- 結果: 落ちた
