import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { SearchPanel } from "../SearchPanel";
import { searchProject } from "../../lib/api";
import type { SearchResponse } from "../../lib/types";

vi.mock("../../lib/api", () => ({
  searchProject: vi.fn(),
  cancelProjectSearch: vi.fn().mockResolvedValue(undefined),
}));

const sampleResponse: SearchResponse = {
  files: [
    {
      path: "C:\\proj\\src\\main.rs",
      name: "main.rs",
      matches: [
        { line: 2, column: 5, preview: "    println!(\"needle\");" },
        { line: 9, column: 1, preview: "needle again" },
      ],
    },
    {
      path: "C:\\proj\\README.md",
      name: "README.md",
      matches: [{ line: 1, column: 1, preview: "Needle in docs" }],
    },
  ],
  total_matches: 3,
  files_scanned: 12,
  truncated: false,
  timed_out: false,
  cancelled: false,
};

describe("SearchPanel", () => {
  beforeEach(() => {
    vi.mocked(searchProject).mockReset();
  });

  it("renders grouped results with line numbers after a search", async () => {
    vi.mocked(searchProject).mockResolvedValue(sampleResponse);
    render(<SearchPanel onOpenResult={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Search query"), { target: { value: "needle" } });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));

    expect(await screen.findByText("main.rs")).toBeInTheDocument();
    expect(screen.getByText("README.md")).toBeInTheDocument();
    expect(screen.getByText(/3 matches in 2 files/)).toBeInTheDocument();
    expect(screen.getByText("needle again")).toBeInTheDocument();
  });

  it("opens a file at the matched line when a result is clicked", async () => {
    vi.mocked(searchProject).mockResolvedValue(sampleResponse);
    const onOpenResult = vi.fn();
    render(<SearchPanel onOpenResult={onOpenResult} />);

    fireEvent.change(screen.getByLabelText("Search query"), { target: { value: "needle" } });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));

    fireEvent.click(await screen.findByText("needle again"));
    expect(onOpenResult).toHaveBeenCalledWith("C:\\proj\\src\\main.rs", "main.rs", 9);
  });

  it("passes the option toggles to the backend", async () => {
    vi.mocked(searchProject).mockResolvedValue(sampleResponse);
    render(<SearchPanel onOpenResult={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Search query"), { target: { value: "ne.dle" } });
    fireEvent.click(screen.getByLabelText("Case sensitive"));
    fireEvent.click(screen.getByLabelText("Regular expression"));
    fireEvent.click(screen.getByRole("button", { name: "Search" }));

    await screen.findByText("main.rs");
    expect(searchProject).toHaveBeenCalledWith({
      query: "ne.dle",
      case_sensitive: true,
      whole_word: false,
      regex: true,
    });
  });

  it("shows the error state when the backend rejects", async () => {
    vi.mocked(searchProject).mockRejectedValue("Invalid search pattern: unclosed group");
    render(<SearchPanel onOpenResult={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Search query"), { target: { value: "[bad" } });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));

    expect(await screen.findByText(/Invalid search pattern/)).toBeInTheDocument();
  });

  it("flags truncated results", async () => {
    vi.mocked(searchProject).mockResolvedValue({ ...sampleResponse, truncated: true });
    render(<SearchPanel onOpenResult={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Search query"), { target: { value: "needle" } });
    fireEvent.click(screen.getByRole("button", { name: "Search" }));

    expect(await screen.findByText(/results truncated/)).toBeInTheDocument();
  });
});
