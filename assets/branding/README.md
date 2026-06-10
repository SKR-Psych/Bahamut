# Bahamut Branding Assets Inventory

This directory contains the original logo assets and variants for the Bahamut desktop environment.

## Application & Installer Icons (generated)

The application icon set under `apps/bahamut-desktop/src-tauri/icons/`
(Windows `.ico`, macOS `.icns`, PNG sizes, and Windows Store logos) is
**generated** from the master asset
`assets/branding/source/Bahamut Logo no bg no title.png` (1536×1536,
transparent). Do not edit the generated files by hand. To regenerate after
changing the master asset, run from `apps/bahamut-desktop/`:

```bash
npm run icons
```

This wraps `tauri icon` (preserving transparency, aspect ratio, and colours)
and removes the Android/iOS variants, which the desktop-only app does not
use. The icons are wired up in `src-tauri/tauri.conf.json` and apply to the
executable, window/taskbar, Start menu shortcut, and the NSIS and MSI
installers.

## Source Assets (`/assets/branding/source/`)

### 1. Transparent Background, Icon Only
- **`Bahamut Logo no bg no title.png`**
  - *Treatment*: Transparent background, icon only.
  - *Recommended Purpose*: Preferred source for desktop applications, Electron window/taskbar icons, installer graphics, and favicons.
- **`Bahamut no bg no title.png`**
  - *Treatment*: Transparent background, alternative icon rendering.
  - *Recommended Purpose*: Supporting secondary icon placements.

### 2. Transparent Background, Logo with Title
- **`Bahamut Logo no bg with title.png`**
  - *Treatment*: Transparent background, logo icon paired with the wordmark "Bahamut".
  - *Recommended Purpose*: Primary asset for documentation headers (e.g. main repository README), application onboarding screens, "About Bahamut" dialogs, and splash screens.
- **`Bahamut no bg with title.png`**
  - *Treatment*: Transparent background, alternative wordmark layout.
  - *Recommended Purpose*: General marketing or promotional headers.

### 3. Dark Background Variants
- **`Bahamut Logo no title.png`**
  - *Treatment*: Dark background, icon only.
  - *Recommended Purpose*: High-contrast dark contexts, wallpapers, or social headers.
- **`Bahamut Logo with title.png`**
  - *Treatment*: Dark background, logo with wordmark.
  - *Recommended Purpose*: App store banners, presentation slides, or marketing media.

### 4. Light Background Variants
- **`Bahamut white bg no title.png`**
  - *Treatment*: White background, icon only.
  - *Recommended Purpose*: High-contrast light backgrounds, print media, or light documentation themes.
- **`Bahamut white bg with title.png`**
  - *Treatment*: White background, logo with wordmark.
  - *Recommended Purpose*: Traditional white paper headers, print templates, and light web page banners.
