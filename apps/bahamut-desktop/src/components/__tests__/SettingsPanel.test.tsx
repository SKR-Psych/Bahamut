import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { SettingsPanel } from "../SettingsPanel";
import { resetAppSettings, updateAppSettings } from "../../lib/api";
import type { AppSettings } from "../../lib/types";

vi.mock("../../lib/api", () => ({
  updateAppSettings: vi.fn(),
  resetAppSettings: vi.fn(),
}));

const settings: AppSettings = {
  max_file_size_bytes: 2 * 1024 * 1024,
  max_search_file_size_bytes: 1024 * 1024,
  ui_prefs: {
    glassmorphism: true,
    solid_mode: false,
    confirm_tab_close: true,
    theme: "dark",
  },
  ai: {
    local_ai_enabled: false,
    provider: "ollama",
    active_model: null,
    context_limit: 256 * 1024,
    per_file_attachment_limit: 64 * 1024,
    history_persistence: true,
    ollama_endpoint: "http://127.0.0.1:11434",
    request_timeout_ms: 120000,
    temperature: 0.2,
    max_output_tokens: 2048,
  },
};

describe("SettingsPanel", () => {
  beforeEach(() => {
    vi.mocked(updateAppSettings).mockReset();
    vi.mocked(resetAppSettings).mockReset();
  });

  it("renders the persisted values", () => {
    render(<SettingsPanel settings={settings} onSettingsChanged={vi.fn()} />);
    expect(screen.getByLabelText(/Maximum editable file size/)).toHaveValue("2");
    expect(screen.getByLabelText(/Maximum searched file size/)).toHaveValue("1");
    expect(screen.getByLabelText(/Glassmorphism effects/)).toBeChecked();
    expect(screen.getByLabelText(/Accessibility solid mode/)).not.toBeChecked();
  });

  it("rejects invalid numeric input without calling the backend", async () => {
    render(<SettingsPanel settings={settings} onSettingsChanged={vi.fn()} />);
    fireEvent.change(screen.getByLabelText(/Maximum editable file size/), {
      target: { value: "not-a-number" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));
    expect(await screen.findByText(/must be a number between/)).toBeInTheDocument();
    expect(updateAppSettings).not.toHaveBeenCalled();
  });

  it("rejects out-of-range sizes", async () => {
    render(<SettingsPanel settings={settings} onSettingsChanged={vi.fn()} />);
    fireEvent.change(screen.getByLabelText(/Maximum editable file size/), {
      target: { value: "999" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));
    expect(await screen.findByText(/must be a number between/)).toBeInTheDocument();
    expect(updateAppSettings).not.toHaveBeenCalled();
  });

  it("persists valid changes and reports them to the parent", async () => {
    const persisted = { ...settings, max_file_size_bytes: 4 * 1024 * 1024 };
    vi.mocked(updateAppSettings).mockResolvedValue(persisted);
    const onSettingsChanged = vi.fn();
    render(<SettingsPanel settings={settings} onSettingsChanged={onSettingsChanged} />);

    fireEvent.change(screen.getByLabelText(/Maximum editable file size/), {
      target: { value: "4" },
    });
    fireEvent.click(screen.getByLabelText(/Accessibility solid mode/));
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(updateAppSettings).toHaveBeenCalledWith({
      max_file_size_bytes: 4 * 1024 * 1024,
      max_search_file_size_bytes: 1024 * 1024,
      ui_prefs: { ...settings.ui_prefs, solid_mode: true },
      ai: settings.ai,
    });
    expect(onSettingsChanged).toHaveBeenCalledWith(persisted);
  });

  it("resets to defaults via the backend", async () => {
    vi.mocked(resetAppSettings).mockResolvedValue(settings);
    const onSettingsChanged = vi.fn();
    render(<SettingsPanel settings={settings} onSettingsChanged={onSettingsChanged} />);

    fireEvent.click(screen.getByRole("button", { name: "Reset to defaults" }));
    expect(await screen.findByText("Settings reset to defaults")).toBeInTheDocument();
    expect(resetAppSettings).toHaveBeenCalled();
    expect(onSettingsChanged).toHaveBeenCalledWith(settings);
  });
});
