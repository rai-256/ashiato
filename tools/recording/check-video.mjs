// SPDX-License-Identifier: AGPL-3.0-only
// 録画が「再生できるか」だけを機械で確かめる（見やすさは判定しない）。
//
//   node check-video.mjs <playwright のモジュールの置き場> <動画のパス>
//     → 標準出力に JSON 1 行: {exists, bytes, duration_s, width, height, played_s, playable, error}
//
// Chromium の <video> に読ませて、長さと縦横が取れ、再生して 1 秒以上進めば playable=true。
// rc は常に 0（判定は JSON の側。呼び出し元が記録する）。
import { existsSync, mkdtempSync, rmSync, statSync, writeFileSync, copyFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

const [modDir, video] = process.argv.slice(2);
const out = { exists: false, bytes: 0, duration_s: null, width: null, height: null, played_s: null, playable: false, error: null };

if (!video || !existsSync(video)) {
  out.error = "動画が無い";
  console.log(JSON.stringify(out));
  process.exit(0);
}
out.exists = true;
out.bytes = statSync(video).size;

const dir = mkdtempSync(join(tmpdir(), "rec-check-"));
let browser;
try {
  const { chromium } = await import(join(modDir, "playwright", "index.mjs"));
  const name = basename(video);
  copyFileSync(video, join(dir, name));
  writeFileSync(join(dir, "v.html"), `<video src="${encodeURIComponent(name)}" preload="auto" muted></video>`);
  browser = await chromium.launch();
  const page = await browser.newPage();
  await page.goto(`file://${dir}/v.html`);
  const r = await page.evaluate(async () => {
    const v = document.querySelector("video");
    const once = (ev) => new Promise((res, rej) => {
      v.addEventListener(ev, res, { once: true });
      v.addEventListener("error", () => rej(new Error(`video error ${v.error?.code}`)), { once: true });
    });
    if (v.readyState < 1) await once("loadedmetadata");
    // webm は duration が Infinity のことがあるので、末尾まで送って確定させる
    if (!isFinite(v.duration)) { v.currentTime = 1e9; await once("seeked"); }
    const duration = v.duration;
    v.currentTime = 0;
    await v.play();
    await new Promise((res) => setTimeout(res, 1500));
    v.pause();
    return { duration, width: v.videoWidth, height: v.videoHeight, played: v.currentTime };
  });
  out.duration_s = Math.round(r.duration * 100) / 100;
  out.width = r.width;
  out.height = r.height;
  out.played_s = Math.round(r.played * 100) / 100;
  out.playable = r.duration > 0 && r.width > 0 && r.played >= 1.0;
} catch (e) {
  out.error = String(e?.message ?? e).slice(0, 300);
} finally {
  await browser?.close().catch(() => undefined);
  rmSync(dir, { recursive: true, force: true });
}
console.log(JSON.stringify(out));
