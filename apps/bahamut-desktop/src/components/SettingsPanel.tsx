import { useEffect, useState } from "react";
import type { AppSettings } from "../lib/types";
import { resetAppSettings, updateAppSettings } from "../lib/api";

interface SettingsPanelProps {
  settings: AppSettings;
  /** Called with the persisted settings after a successful save/reset. */
  onSettingsChanged: (settings: AppSettings) => void;
}

const MIN_MIB = 0.001; // 1 KiB
const MAX_MIB = 50;

function bytesToMiB(bytes: number): string {
  return (bytes / (1024 * 1024)).toString();
}

function mibToBytes(mib: number): number {
  return Math.round(mib * 1024 * 1024);
}

export function SettingsPanel({ settings, onSettingsChanged }: SettingsPanelProps) {
  const [maxFileMiB, setMaxFileMiB] = useState(bytesToMiB(settings.max_file_size_bytes));
  const [maxSearchMiB, setMaxSearchMiB] = useState(
    bytesToMiB(settings.max_search_file_size_bytes),
  );
  const [prefs, setPrefs] = useState(settings.ui_prefs);
  const [error, setError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  useEffect(() => {
    setMaxFileMiB(bytesToMiB(settings.max_file_size_bytes));
    setMaxSearchMiB(bytesToMiB(settings.max_search_file_size_bytes));
    setPrefs(settings.ui_prefs);
  }, [settings]);

  const parseSize = (value: string, label: string): number | string => {
    const parsed = Number(value);
    if (!Number.isFinite(parsed) || parsed < MIN_MIB || parsed > MAX_MIB) {
      return `${label} must be a number between ${MIN_MIB} and ${MAX_MIB} MiB`;
    }
    return mibToBytes(parsed);
  };

  const save = async () => {
    setFeedback(null);
    const maxFile = parseSize(maxFileMiB, "Maximum editable file size");
    if (typeof maxFile === "string") {
      setError(maxFile);
      return;
    }
    const maxSearch = parseSize(maxSearchMiB, "Maximum searched file size");
    if (typeof maxSearch === "string") {
      setError(maxSearch);
      return;
    }
    setError(null);
    try {
      const persisted = await updateAppSettings({
        max_file_size_bytes: maxFile,
        max_search_file_size_bytes: maxSearch,
        ui_prefs: prefs,
      });
      onSettingsChanged(persisted);
      setFeedback("Settings saved");
    } catch (e) {
      setError(String(e));
    }
  };

  const reset = async () => {
    setFeedback(null);
    setError(null);
    try {
      const persisted = await resetAppSettings();
      onSettingsChanged(persisted);
      setFeedback("Settings reset to defaults");
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="settings-panel">
      <h3 className="panel-heading">Settings</h3>

      <fieldset className="settings-group">
        <legend>Limits</legend>
        <label className="settings-row" htmlFor="settings-max-file">
          <span>Maximum editable file size (MiB)</span>
          <input
            id="settings-max-file"
            className="input-field input-compact"
            inputMode="decimal"
            value={maxFileMiB}
            onChange={(e) => setMaxFileMiB(e.target.value)}
          />
        </label>
        <label className="settings-row" htmlFor="settings-max-search">
          <span>Maximum searched file size (MiB)</span>
          <input
            id="settings-max-search"
            className="input-field input-compact"
            inputMode="decimal"
            value={maxSearchMiB}
            onChange={(e) => setMaxSearchMiB(e.target.value)}
          />
        </label>
      </fieldset>

      <fieldset className="settings-group">
        <legend>Appearance</legend>
        <label className="settings-row settings-toggle">
          <input
            type="checkbox"
            checked={prefs.glassmorphism}
            onChange={(e) => setPrefs({ ...prefs, glassmorphism: e.target.checked })}
          />
          <span>Glassmorphism effects</span>
        </label>
        <label className="settings-row settings-toggle">
          <input
            type="checkbox"
            checked={prefs.solid_mode}
            onChange={(e) => setPrefs({ ...prefs, solid_mode: e.target.checked })}
          />
          <span>Accessibility solid mode (high contrast, no transparency)</span>
        </label>
        <label className="settings-row" htmlFor="settings-theme">
          <span>Theme</span>
          <select
            id="settings-theme"
            className="input-field input-compact"
            value={prefs.theme}
            onChange={(e) => setPrefs({ ...prefs, theme: e.target.value })}
          >
            <option value="dark">Bahamut Dark</option>
          </select>
        </label>
      </fieldset>

      <fieldset className="settings-group">
        <legend>Editor tabs</legend>
        <label className="settings-row settings-toggle">
          <input
            type="checkbox"
            checked={prefs.confirm_tab_close}
            onChange={(e) => setPrefs({ ...prefs, confirm_tab_close: e.target.checked })}
          />
          <span>Confirm before closing a tab with unsaved changes</span>
        </label>
      </fieldset>

      {error && <p className="status-error">{error}</p>}
      {feedback && <p className="status-success">{feedback}</p>}

      <div className="flex-row settings-actions">
        <button type="button" className="primary-btn btn-compact" onClick={() => void save()}>
          Save settings
        </button>
        <button type="button" className="secondary-btn btn-compact" onClick={() => void reset()}>
          Reset to defaults
        </button>
      </div>
      <p className="small-text">
        Settings are stored locally in Bahamut's database. Credentials are never stored here.
      </p>
    </div>
  );
}
