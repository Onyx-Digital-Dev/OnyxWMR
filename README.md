# osvwm - OSV Workspace Management Routine

A disciplined, opinionated Wayland compositor for Onyx OSV. Forked from [Niri](https://github.com/YaLTeR/niri).

**This is NOT a feature-rich tiling WM. It's a routine: predictable, minimal, muscle-memory driven.**

## Core Philosophy

- **8 workspaces total, fixed**: 1-6 regular work, 7-8 immersion (fullscreen apps like games/video)
- **1D horizontal strip per workspace**: Windows arranged left-to-right only
- **Fixed window sizes**: 1u, 2u, 3u, or Frame (fullscreen within workspace)
- **No auto-tiling, no smart layouts**: Explicit control only
- **Immersion spaces (7-8) never lose render**: Seamless switching, no minimize/reload

## Window Sizing

| Size | Width |
|------|-------|
| 1u | 1/3 screen width |
| 2u | 2/3 screen width |
| 3u | Full width (with title bar/borders) |
| Frame | True fullscreen (no decorations) |

## Keybindings (Hardcoded)

| Key | Action |
|-----|--------|
| `Super+K` / `Up` | Workspace up |
| `Super+J` / `Down` | Workspace down |
| `Super+H` / `Left` | Focus left in strip |
| `Super+L` / `Right` | Focus right in strip |
| `Super+Shift+H` | Move window left in strip |
| `Super+Shift+L` | Move window right in strip |
| `Super+Shift+J` | Move window to workspace down |
| `Super+Shift+K` | Move window to workspace up |
| `Super+R` | Cycle window size (1u→2u→3u→1u) |
| `Super+F` | Toggle Frame (fullscreen) |
| `Super+V` | Toggle floating (escape strip) |
| `Super+O` | Overview |
| `Super+Space` | Launch osv-intake |
| `Super+Return` | Terminal (alacritty) |
| `Super+Q` | Close focused window |
| `Super+Shift+Escape` | Exit compositor |

## Project Structure

```
osvwm/
├── crates/
│   ├── osvwm/          # Main compositor (Phase 1)
│   ├── osvwm-config/   # Configuration handling
│   ├── osv-ipc/        # IPC library (shared)
│   ├── osv-bar/        # Status bar (Phase 2)
│   ├── osv-intake/     # Launcher (Phase 3)
│   └── osv-msg/        # IPC CLI tool (Phase 3)
├── resources/
├── mods/               # NixOS modules (future)
└── Cargo.toml          # Workspace
```

## Building

```bash
cargo build --release
# Binary output: target/release/osvwm
```

## Configuration

Wallpaper config at `~/.config/osvwm/wallpaper.conf`:
```
path=/path/to/image.png
mode=fill
```

Modes: `fill`, `fit`, `stretch`, `center`, `tile`

## Development Status

**Phase 1 (Compositor)**: In Progress
- [ ] 8 fixed workspaces (1-6 regular, 7-8 immersion)
- [ ] 1D strip layout, fixed sizes (1u/2u/3u/Frame)
- [ ] Immersion spaces stay rendered (seamless switching)
- [ ] OSV keybindings hardcoded
- [ ] No auto-tiling, no smart behaviors
- [ ] Wallpaper support (direct render)
- [ ] IPC foundation for bar/tools

**Phase 2 (Bar)**: Planned
- [ ] Workspace capsules with window icons
- [ ] Auto-sizing capsules based on window count
- [ ] Immersion workspace distinction
- [ ] "Onyx OSV" centered, time/date right
- [ ] Thin, floating, 3D gradient + shadow
- [ ] Inter font, clean/professional
- [ ] IPC-driven updates from compositor

## License

GPL-3.0-or-later (inherited from Niri)

## Credits

- Based on [Niri](https://github.com/YaLTeR/niri) by Ivan Molodetskikh
- Built with [Smithay](https://github.com/Smithay/smithay)
