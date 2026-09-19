// Scenario: 直近に置いた書庫の箱は Must の前にある
// Scenario: 直近に置いた書庫の結果が箱に出る
// Scenario: 書庫が 1 つも置かれていないことが出る
import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { LatestArchive } from "../LatestArchive";
it("書庫の結果と空状態を文字で出す", () => {
  const { rerender } = render(<LatestArchive status={{ sources: [], latest_archive: { file_name: "takeout.zip", inserted: 2, duplicate: 1, unreadable: 0 } }} />);
  expect(screen.getByTestId("latest-archive").textContent).toContain("takeout.zip");
  rerender(<LatestArchive status={null} />);
  expect(screen.getByText("まだ書庫が置かれていません")).toBeTruthy();
});
