// SPDX-License-Identifier: AGPL-3.0-only
import { execFileSync } from "node:child_process";

/**
 * 画面（V-01）の組み立ての印（ST49 / design D5・D6）。`<版番号>+<commit 40 桁|unknown>[.dirty][.debug]`。
 * `vite.config.ts` と `vitest.config.ts` が読み、`define: { __ASHIATO_WEB_VERSION__ }` に埋める。
 * Rust の `crates/build-stamp` と同じ規則を、vite の道具の中で書き直したもの（形は同じ正規表現で見る）。
 */
declare const process: { env: Record<string, string | undefined> };

/** この部品のソースの道（リポジトリの根から。cwd に依らない pathspec）。 */
const SOURCE_PATH = ":/web";

/** git を呼ぶ関数。引数を渡して標準出力を返し、失敗したら投げる。試験は偽のものを渡す。 */
export type Git = (args: string[]) => string;

export const execGit: Git = (args) =>
  execFileSync("git", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
    // `git status` が index を書き換えて次の組み立てを呼ばないように（D5）
    env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" },
  });

export interface StampInput {
  /** `web/package.json` の `version` */
  version: string;
  /** `vite build`（production）だけ `.debug` を付けない */
  production: boolean;
  git?: Git;
  env?: Record<string, string | undefined>;
}

/** commit の出所は環境変数 `ASHIATO_BUILD_COMMIT`（空でなければ）→ `git rev-parse HEAD` → `unknown`。 */
function commitOf(git: Git, env: Record<string, string | undefined>): string {
  const fromEnv = env.ASHIATO_BUILD_COMMIT?.trim();
  if (fromEnv) return fromEnv;
  try {
    const head = git(["rev-parse", "HEAD"]).trim();
    return /^[0-9a-f]{40}$/.test(head) ? head : "unknown";
  } catch {
    return "unknown";
  }
}

function isDirty(git: Git): boolean {
  try {
    return git(["status", "--porcelain", "--", SOURCE_PATH]).trim() !== "";
  } catch {
    return false;
  }
}

export function webBuildStamp({ version, production, git = execGit, env = process.env }: StampInput): string {
  const commit = commitOf(git, env);
  return `${version}+${commit}${isDirty(git) ? ".dirty" : ""}${production ? "" : ".debug"}`;
}
