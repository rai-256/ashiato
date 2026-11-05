// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

/**
 * 画面（V-01）の組み立ての印（ST49 / design D5・D6）。`<版番号>+<commit 40 桁|unknown>[.dirty][.debug]`。
 * `vite.config.ts` と `vitest.config.ts` が読み、`define: { __ASHIATO_WEB_VERSION__ }` に埋める。
 * Rust の `crates/build-stamp` と同じ規則を、vite の道具の中で書き直したもの（形は同じ正規表現で見る）。
 */
declare const process: { env: Record<string, string | undefined> };

/** この部品のソースの道（リポジトリの根から。cwd に依らない pathspec）。 */
const SOURCE_PATHS = [":/web", ":/Cargo.toml"];

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
  /** 根の Cargo.toml の内容。未指定時はリポジトリのファイルを読む。 */
  cargoToml?: string;
  /** `vite build`（production）だけ `.debug` を付けない */
  production: boolean;
  git?: Git;
  env?: Record<string, string | undefined>;
}

/** 根の Cargo.toml の `[workspace.package]` の版（画面の側の唯一の読み方。e2e もこれを使う。/code-review R40） */
export function sourceVersion(cargoToml: string): string {
  const match = /^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m.exec(cargoToml);
  if (!match?.[1]) throw new Error("Cargo.toml の [workspace.package] version が無い");
  return match[1];
}

function versionShape(value: string): { base: string } | undefined {
  const match = /^(\d+\.\d+\.\d+)(?:-beta\.([1-9]|[1-8]\d|9[0-8])|-dev\.\d{8})?$/.exec(value);
  return match ? { base: match[1] } : undefined;
}

function resolvedVersion(source: string, env: Record<string, string | undefined>): string {
  const sourceParsed = versionShape(source);
  if (!sourceParsed) throw new Error(`Cargo.toml の版番号が不正: ${source}`);
  const given = env.ASHIATO_BUILD_VERSION?.trim();
  if (!given) return source;
  const givenParsed = versionShape(given);
  if (!givenParsed) throw new Error(`ASHIATO_BUILD_VERSION の形が不正: ${given}`);
  if (givenParsed.base !== sourceParsed.base) throw new Error(`版番号の出所が一致しない: 出所 ${source}、渡した版番号 ${given}`);
  return given;
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
    return SOURCE_PATHS.some((path) => git(["status", "--porcelain", "--", path]).trim() !== "");
  } catch {
    return false;
  }
}

export function webBuildStamp({ cargoToml, production, git = execGit, env = process.env }: StampInput): string {
  const source = cargoToml ?? readFileSync(new URL("../Cargo.toml", import.meta.url), "utf8");
  const version = resolvedVersion(sourceVersion(source), env);
  const commit = commitOf(git, env);
  return `${version}+${commit}${isDirty(git) ? ".dirty" : ""}${production ? "" : ".debug"}`;
}
