# Bahamut Branding Assets Inventory

This directory contains the original logo assets and variants for the Bahamut desktop environment.

## Asset Tiers: source → derived master → generated icons

There are three tiers of icon-related assets. Only the first is hand-made.

1. **Source artwork** — `assets/branding/source/Bahamut Logo no bg no title.png`
   (1536×1536, transparent). The original design export, kept untouched. The
   visible emblem occupies only ~60% of this canvas (bbox 920×948 at
   (308,213)), which is correct for artwork but makes app icons look
   undersized.
2. **Derived app-icon master** — `assets/branding/derived/Bahamut App Icon
   Master.png` (1116×1116, transparent). A *crop + pad* of the source — no
   scaling, recolouring, or redrawing: the emblem's alpha bounding box
   (threshold ≥16) is cut out pixel-for-pixel and centred on a square canvas
   sized so the emblem fills **~85%** (even margins: 98px left/right,
   84px top/bottom — the safety margin against Windows icon masks). This is
   the input to icon generation. If the source artwork changes, recreate it
   the same way (measure alpha bbox, pad to ~85% fill).
3. **Generated Tauri icons** — `apps/bahamut-desktop/src-tauri/icons/`
   (Windows `.ico` with 16/24/32/48/64/256 frames, macOS `.icns`, PNG sizes,
   Windows Store logos). Generated from the derived master; never edit by
   hand. Measured emblem fill in the shipped icons: 85–92% across all `.ico`
   frame sizes — standard Windows app-icon scale.

To regenerate after changing the master, run from `apps/bahamut-desktop/`:

```bash
npm run icons
```

This wraps `tauri icon` (preserving transparency, aspect ratio, and colours),
removes the Android/iOS variants, which the desktop-only app does not use,
and refreshes the in-app UI asset `src/assets/bahamut-logo.png` (a 256px
derivative shown in the application header and welcome/empty states). The
icons are wired up in `src-tauri/tauri.conf.json` and apply to the
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
