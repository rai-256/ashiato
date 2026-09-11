// SPDX-License-Identifier: AGPL-3.0-only
/**
 * 5 本の達成日数と分母、合否、確定か暫定か（tasks 8.6 / NFR-13 /
 * 深掘り 第 4 回 Q9, 第 5 回 Q18, 第 7 回 Q27）。
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AchievementPanel } from "../AchievementPanel";
import { achievement, FIVE } from "./fixtures";

describe("達成の表示", () => {
  // Scenario: 5 本の達成日数と合否が出る
  it("5 ソースそれぞれの達成日数と分母が数値で出る", () => {
    render(<AchievementPanel data={achievement()} />);
    const expected = [360, 355, 352, 351, 340];
    FIVE.forEach(([id], i) => {
      expect(screen.getByTestId(`achieved-${id}`).textContent).toBe(String(expected[i]));
      expect(screen.getByTestId(`denominator-${id}`).textContent).toBe("365");
    });
  });

  it("5 本すべてが分母の 95 % 以上かどうかが出る", () => {
    // 線は 346.75。340 が届かないので未達
    render(<AchievementPanel data={achievement()} />);
    expect(screen.getByTestId("verdict").getAttribute("data-verdict")).toBe("false");
    expect(screen.getByTestId("verdict").textContent).toContain(
      "分母の 95 % に届かないソースがある",
    );
    // 落ちたソースが分かる
    const failing = screen
      .getByTestId("achievement")
      .querySelector('[data-source="c02-browser-history"]');
    expect(failing?.getAttribute("data-met")).toBe("false");
  });

  // Scenario: 全部の窓が閉じた日に確定する
  it("確定か暫定かが出る", () => {
    render(<AchievementPanel data={achievement()} />);
    expect(screen.getByTestId("confirmation").textContent).toBe("確定");
  });

  // Scenario: 暫定のときは確定までの残り日数が出る
  it("暫定のときは確定までの残り日数が出る", () => {
    render(
      <AchievementPanel data={achievement({ confirmed: false, days_until_confirmed: 65 })} />,
    );
    expect(screen.getByTestId("confirmation").textContent).toBe("暫定");
    expect(screen.getByTestId("until-confirmed").textContent).toContain("65");
  });

  // Scenario: 始まっていないソースがあると残り日数の代わりに理由が出る
  it("始まっていないソースがあると、残り日数の代わりに理由が出る", () => {
    render(
      <AchievementPanel
        data={achievement({
          confirmed: false,
          days_until_confirmed: null,
          confirms_on: null,
          not_started: ["c02-window", "c02-browser-history"],
        })}
      />,
    );
    const line = screen.getByTestId("until-confirmed").textContent ?? "";
    expect(line).not.toContain("あと");
    expect(line).toContain("収集を開始していない");
    expect(line).toContain("c02-window");
  });

  it("確定しているときは残り日数を出さない", () => {
    render(<AchievementPanel data={achievement()} />);
    expect(screen.queryByTestId("until-confirmed")).toBeNull();
  });
});
