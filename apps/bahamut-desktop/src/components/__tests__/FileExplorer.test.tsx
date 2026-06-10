import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { FileExplorer } from "../FileExplorer";
import { createProjectFile, deleteProjectPath } from "../../lib/api";
import type { FileTreeResponse } from "../../lib/types";

vi.mock("../../lib/api", () => ({
  createProjectFile: vi.fn(),
  createProjectFolder: vi.fn(),
  renameProjectPath: vi.fn(),
  deleteProjectPath: vi.fn(),
}));

const tree: FileTreeResponse = {
  root: "C:\\proj",
  truncated: false,
  nodes: [
    {
      name: "src",
      path: "C:\\proj\\src",
      is_dir: true,
      children: [{ name: "main.rs", path: "C:\\proj\\src\\main.rs", is_dir: false }],
    },
    { name: "README.md", path: "C:\\proj\\README.md", is_dir: false },
  ],
};

function setup(selectedPath: string | null = null) {
  const handlers = {
    onSelect: vi.fn(),
    onOpenFile: vi.fn(),
    onTreeChanged: vi.fn().mockResolvedValue(undefined),
    onPathRemoved: vi.fn(),
    onPathRenamed: vi.fn(),
    onStatus: vi.fn(),
  };
  render(
    <FileExplorer
      tree={tree}
      openFilePaths={[]}
      dirtyFilePaths={[]}
      selectedPath={selectedPath}
      {...handlers}
    />,
  );
  return handlers;
}

describe("FileExplorer", () => {
  beforeEach(() => {
    vi.mocked(createProjectFile).mockReset();
    vi.mocked(deleteProjectPath).mockReset();
  });

  it("creates a file in the project root through the inline form", async () => {
    vi.mocked(createProjectFile).mockResolvedValue({ path: "C:\\proj\\new.ts" });
    const { onStatus, onTreeChanged } = setup();

    fireEvent.click(screen.getByRole("button", { name: "+ File" }));
    fireEvent.change(screen.getByPlaceholderText("name"), { target: { value: "new.ts" } });
    fireEvent.click(screen.getByRole("button", { name: "OK" }));

    expect(await screen.findByRole("button", { name: "+ File" })).toBeInTheDocument();
    expect(createProjectFile).toHaveBeenCalledWith("C:\\proj\\new.ts");
    expect(onStatus).toHaveBeenCalledWith(expect.stringContaining("Created"));
    expect(onTreeChanged).toHaveBeenCalled();
  });

  it("rejects invalid names client-side before any backend call", async () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: "+ File" }));
    fireEvent.change(screen.getByPlaceholderText("name"), { target: { value: "bad/name" } });
    fireEvent.click(screen.getByRole("button", { name: "OK" }));

    expect(await screen.findByText(/characters that are not allowed/)).toBeInTheDocument();
    expect(createProjectFile).not.toHaveBeenCalled();
  });

  it("surfaces backend rejections as visible errors", async () => {
    vi.mocked(createProjectFile).mockRejectedValue(
      "Access denied: path is outside the project workspace",
    );
    setup();
    fireEvent.click(screen.getByRole("button", { name: "+ File" }));
    fireEvent.change(screen.getByPlaceholderText("name"), { target: { value: "x.ts" } });
    fireEvent.click(screen.getByRole("button", { name: "OK" }));

    expect(await screen.findByText(/Access denied/)).toBeInTheDocument();
  });

  it("requires explicit confirmation before delete", async () => {
    vi.mocked(deleteProjectPath).mockResolvedValue({
      path: "C:\\proj\\README.md",
      trash_path: "C:\\trash\\1-README.md",
      snapshot_id: 7,
    });
    const { onPathRemoved } = setup("C:\\proj\\README.md");

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(deleteProjectPath).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Move to trash" }));
    expect(await screen.findByRole("button", { name: "Delete" })).toBeInTheDocument();
    expect(deleteProjectPath).toHaveBeenCalledWith("C:\\proj\\README.md");
    expect(onPathRemoved).toHaveBeenCalledWith("C:\\proj\\README.md");
  });

  it("cancelling the delete dialog performs no operation", () => {
    setup("C:\\proj\\README.md");
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(deleteProjectPath).not.toHaveBeenCalled();
  });
});
