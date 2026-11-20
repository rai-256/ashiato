#!/usr/bin/env python3
"""CH25 の確かめ（change の中の台本。製品にも tools/ にも置かない）。

使い方:
  python3 openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py ci     # gradle の job が部品の取り置きを持ち、ほかは base のまま
  python3 openspec/changes/ch25-ci-flake-gradle-cache-clock-test/check-ch25.py clock  # 製品のコードは base のまま、Scenario の幅は弱めていない

base は `git merge-base origin/main HEAD`。落ちたら理由を出して rc=1。
"""
import copy
import re
import subprocess
import sys

import yaml

WORKFLOWS = {
    ".github/workflows/ci.yml": {"build-stamp", "android", "android-instrumented", "package-linux", "chain"},
    ".github/workflows/nightly.yml": {"package-linux"},
    ".github/workflows/release.yml": {"package-linux"},
}
# gradle を（直に・台本を通して）呼ぶ印。setup-java を持つ job も gradle の job とみなす
GRADLE_RUN = re.compile(
    r"gradlew\b|check-gradle-locks\.sh|build\.sh apk|check-build-stamp\.sh android|check-licenses\.sh android|package\.sh linux|draft\.sh --self-test"
)
SHA = re.compile(r"@[0-9a-f]{40}( |$)")
CLOCK = "crates/collector-windows/src/clock.rs"
RANGES = {
    "clock_reference_pc_ahead_gives_positive_difference": "(300_000..301_000 + i64::try_from(width).unwrap()).contains(&skew)",
    "clock_reference_pc_behind_gives_negative_difference": "(-300_000 - i64::try_from(width).unwrap()..-299_000 + i64::try_from(width).unwrap())",
}

fails = []


def git(*args):
    return subprocess.run(["git", *args], capture_output=True, text=True, check=True).stdout


def base():
    return git("merge-base", "origin/main", "HEAD").strip()


def uses_gradle(job):
    for s in job.get("steps", []):
        if "actions/setup-java" in str(s.get("uses", "")):
            return True
        if GRADLE_RUN.search(str(s.get("run", ""))):
            return True
        if "android-emulator-runner" in str(s.get("uses", "")):
            return True
    return False


def has_cache(job):
    """setup-java の cache: gradle か、gradle を呼ぶ最初の段より前の setup-gradle。"""
    for s in job.get("steps", []):
        u = str(s.get("uses", ""))
        if "actions/setup-java" in u and (s.get("with") or {}).get("cache") == "gradle":
            return True
        if "gradle/actions/setup-gradle" in u:
            return True
        if is_gradle_cache_step(s):
            return True
        if GRADLE_RUN.search(str(s.get("run", ""))) or "android-emulator-runner" in u:
            return False
    return False


def is_gradle_cache_step(s):
    """chain の job（setup-java を持たない）の取り置き: actions/cache で ~/.gradle を持つ段。"""
    u = str(s.get("uses", ""))
    return u.startswith("actions/cache@") and ".gradle/caches" in str((s.get("with") or {}).get("path", ""))


def without_cache(job):
    """取り置きの指定を外した形（base と比べるため）。"""
    j = copy.deepcopy(job)
    steps = []
    for s in j.get("steps", []):
        if "gradle/actions/setup-gradle" in str(s.get("uses", "")) or is_gradle_cache_step(s):
            continue
        if "actions/setup-java" in str(s.get("uses", "")) and isinstance(s.get("with"), dict):
            s["with"].pop("cache", None)
            s["with"].pop("cache-dependency-path", None)
        steps.append(s)
    j["steps"] = steps
    return j


def check_ci():
    b = base()
    for path, expected in WORKFLOWS.items():
        text = open(path, encoding="utf-8").read()
        now = yaml.safe_load(text)
        old = yaml.safe_load(git("show", f"{b}:{path}"))
        jobs = now.get("jobs", {})
        found = {name for name, job in jobs.items() if uses_gradle(job)}
        if found != expected:
            fails.append(f"{path}: gradle を使う job が想定と違う（見つけた {sorted(found)} / 想定 {sorted(expected)}）")
        for name in sorted(found):
            if not has_cache(jobs[name]):
                fails.append(f"{path}: {name} が gradle の部品の取り置きを持たない")
        if set(jobs) != set(old.get("jobs", {})):
            fails.append(f"{path}: job の集合が base から変わった")
        for name, job in jobs.items():
            o = old.get("jobs", {}).get(name)
            if o is None:
                continue
            if without_cache(job) != without_cache(o):
                fails.append(f"{path}: {name} が取り置きの指定の外で base から変わった")
        top_now = {k: v for k, v in now.items() if k != "jobs"}
        top_old = {k: v for k, v in old.items() if k != "jobs"}
        if top_now != top_old:
            fails.append(f"{path}: jobs の外（on・permissions など）が base から変わった")
        for line in text.splitlines():
            m = re.match(r"\s*(- )?uses:\s*(\S+)(.*)$", line)
            if m and not m.group(2).startswith("./") and not SHA.search(m.group(2) + " "):
                fails.append(f"{path}: uses が commit の SHA で固定されていない: {line.strip()}")
    if subprocess.run(["git", "diff", "--quiet", b, "--", "collector-android"]).returncode != 0:
        fails.append("collector-android/ が base から変わった（依存の版・ロックは変えない）")


def fn_body(src, name):
    i = src.find(f"fn {name}(")
    if i < 0:
        return None
    j = src.find("\n    }\n", i)
    return src[i:j] if j > 0 else None


def check_clock():
    b = base()
    now = open(CLOCK, encoding="utf-8").read()
    old = git("show", f"{b}:{CLOCK}")
    mark = "#[cfg(test)]\nmod tests {"
    if mark not in now or now.split(mark)[0] != old.split(mark)[0]:
        fails.append(f"{CLOCK}: 試験の外（製品のコード）が base から変わった")
    for name, rng in RANGES.items():
        body = fn_body(now, name)
        if body is None:
            fails.append(f"{CLOCK}: {name} が無い")
            continue
        if rng not in body:
            fails.append(f"{CLOCK}: {name} の幅が Scenario の字面（{rng}）でない")
        if "skew_against(Duration::minutes(" not in body:
            fails.append(f"{CLOCK}: {name} がずらし（Duration）で skew_against を呼んでいない")
        sc = "Scenario: PC の時計が" + ("進んでいると差が正で残る" if "ahead" in name else "遅れていると差が負で残る")
        if f"/// {sc}\n    #[test]\n    fn {name}(" not in now:
            fails.append(f"{CLOCK}: {name} に「{sc}」の印が無い")
    check_clock_guard(now)


def check_clock_guard(now):
    body = fn_body(now, "clock_reference_setup_time_is_not_in_the_difference")
    if body is None:
        fails.append(f"{CLOCK}: 用意の時間を長くする試験 clock_reference_setup_time_is_not_in_the_difference が無い")
    else:
        helpers = [fn_body(now, n) or "" for n in re.findall(r"fn (skew_against\w*)\(", now)]
        slept = "std::thread::sleep" in body or any("std::thread::sleep" in h for h in helpers)
        if not slept or not re.search(r"1_?500", body):
            fails.append(f"{CLOCK}: clock_reference_setup_time_is_not_in_the_difference が 1,500 ms 眠っていない")
        for rng in RANGES.values():
            if rng not in body:
                fails.append(f"{CLOCK}: clock_reference_setup_time_is_not_in_the_difference に幅の字面 {rng} が無い")
    # 差と幅の式（skew_against か、そこから呼ぶ skew_against_* の中）
    bodies = [fn_body(now, n) or "" for n in re.findall(r"fn (skew_against\w*)\(", now)]
    for expr in ("(r.wall_after - r.time).num_milliseconds()", "r.uptime_after_ms - r.uptime_before_ms"):
        if not any(expr in b for b in bodies):
            fails.append(f"{CLOCK}: skew_against の差・幅の式 {expr} が字面のまま無い")


def main():
    what = sys.argv[1:] or ["ci", "clock"]
    for w in what:
        {"ci": check_ci, "clock": check_clock}[w]()
    for f in fails:
        print("FAIL:", f)
    if fails:
        return 1
    print("OK:", " ".join(what))
    return 0


if __name__ == "__main__":
    sys.exit(main())
