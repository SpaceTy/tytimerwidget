# Control Widget Style Guide

Guidelines for recreating this widget’s look-and-feel in new `tywidgets` components.

## Overall Look
- **Theme:** Dark glassy panel: `rgba(0, 0, 0, 0.85)` background with white foreground.
- **Chrome:** Borderless, undecorated GTK window, but resizable. Default size `350x500` (from `Config`).
- **Typography:** System font, white text; secondary text uses `.dim-label` with `opacity: 0.6`. Titles use `.title-label` with bold weight.
- **Icons:** Use symbolic icon names (e.g., `window-close`, `display-brightness`, `audio-speakers`) from the standard icon theme; favor minimal line icons.

## Layout & Spacing
- **Root container:** Vertical `Box`, `spacing: 4`, margins `8px` on all sides.
- **Section separators:** Horizontal `Separator` widgets between logical groups (brightness vs. audio).
- **Header bar:** Horizontal `Box`, `spacing: 6`, vertical margins `4px`. Contains a bold title on the left, flexible spacer, and a flat close button on the right.
- **Rows:** Each control row uses a horizontal `Box` with `spacing: 12` and margins `12px` left/right, `6px` top/bottom.
- **Content column:** Inside each row, a vertical `Box` with `spacing: 4` and `hexpand: true` to keep sliders aligned and labels left-aligned.
- **Gesture area:** The whole window surface responds to drag-to-close (upward swipe) and closes on blur unless `--keep-open` is passed.

## Components
- **Slider Row Pattern:**
  - Left icon at `24px`.
  - Title label using `.title-label` for contrast and hierarchy.
  - Horizontal `Scale` without a value display (`draw_value: false`), spanning remaining width.
  - Optional mute toggle on the right; flat circular `ToggleButton` with context-driven icon (`audio-volume-high`/`audio-volume-muted`, `audio-input-microphone`/`microphone-sensitivity-muted`).
- **Buttons:** Use `button.flat` class for icon-only actions to keep chrome minimal; toggles can also add `circular` for a pill/circle outline.
- **Separators:** Use to break categories (e.g., between brightness and audio groups).

## Styling Implementation
- Attach a `CssProvider` at application priority:
  ```rust
  provider.load_from_data("
      window { background: rgba(0, 0, 0, 0.85); color: white; }
      .dim-label { opacity: 0.6; color: white; }
      .title-label { font-weight: bold; color: white; }
      label { color: white; }
      button.flat { color: white; }
  ");
  ```
  Apply with `style_context_add_provider_for_display`.
- Keep text/icon color white; rely on opacity for hierarchy instead of mixing colors.

## Behavior
- **Close affordances:** Close button (top-right), blur-to-close after 100ms, and drag-up quick gesture (>50px in <0.5s). All close the window without extra prompts.
- **Persistence:** Window size is user-configurable via `config.json`, but defaults to 350x500. Keep margins/spacing intact when size changes.
- **Startup:** Load current system state into sliders (brightness, volumes) before wiring change handlers to avoid feedback loops.

## Implementation Structure (How It’s Written)
- **Language & Toolkit:** Rust + `gtk4`; favor builder pattern and `prelude::*` imports for concise widget construction.
- **Entry & Args:** `main.rs` owns the GTK `Application`, parses `Args` (e.g., `--keep-open`), and constructs a `Window`.
- **UI Modules:** `ui/window.rs` builds layout, applies CSS provider, wires gestures/close logic, and composes `SliderRow` components from `ui/slider_row.rs`.
- **Services:** `audio.rs` wraps `pactl`; `brightness.rs` wraps `brightnessctl`. Parsing and system calls stay inside services; UI only calls typed methods like `get_*`/`set_*`.
- **Config:** `config.rs` stores window size defaults and persistence via `ProjectDirs`; load first, then save defaults if missing.
- **State Flow:** Read current system values, set UI controls, then connect handlers (prevents initial change events). Toggle buttons update icons based on mute state.
- **Styling Hook:** A single `CssProvider` at application priority defines global dark theme and class-based tweaks (`title-label`, `dim-label`, `flat`, `circular`).
- **Extensibility:** Add new rows by extending `SliderRow` or creating similar components; keep services isolated for new device domains; retain gesture/close behaviors for family consistency.

## Extending to New Widgets
- Reuse the root container spacing/margins and header pattern for consistency across widgets.
- Keep the dark glassy background and white foreground; introduce accent color only through icons or hover states if needed.
- Use symbolic icons plus concise, left-aligned titles; avoid extra labels when the icon is self-explanatory, but prefer clarity over minimalism.
- Maintain flat, borderless controls; prefer separators and spacing to delineate groups instead of boxes.
- Ensure gestures and close-on-blur remain consistent so widgets feel part of the same family; gate them behind flags (like `--keep-open`) when needed for debugging.
