# OSV UI Components

Slint-based UI components for the Onyx OSV Desktop Environment.

## Components

### osv-bar
Status bar with:
- 8 workspace indicators (6 Focus capsules + 2 Immersion circles)
- Centered title showing "Onyx OSV" or focused window name
- System tray: WiFi, Bluetooth, Battery (with %), Volume, Clock

### osv-intake
Application launcher with:
- Search input with icon and clear button
- Filtered results list with keyboard navigation
- App icons and names with selection highlight

### osv-control
System settings panel starting with Telemetry dashboard:
- **Telemetry**: CPU, Memory, Disk usage bars + Network throughput + Active routes/processes
- **Network**: WiFi, Ethernet, VPN settings
- **Display**: Brightness slider, Night mode toggle, Resolution dropdown
- **Sound**: Volume controls, Output device selection
- **Power**: Battery status, Performance modes
- **Bluetooth**: Device pairing and management
- **Appearance**: Wallpaper, Theme settings
- **About**: System information

## Building

```bash
cargo build --release
```

## Running

```bash
# Full preview
cargo run --bin osv-preview

# Individual components
cargo run --bin osv-bar
cargo run --bin osv-intake
cargo run --bin osv-control
```

## Color Palette

| Token   | Hex     | Usage                  |
|---------|---------|------------------------|
| osv-bg  | #0E213D | Primary background     |
| black   | #0a1628 | Deepest/recessed areas |
| dark    | #12294a | Secondary backgrounds  |
| surface | #1a3356 | Interactive surfaces   |
| border  | #2a4a6e | Borders, dividers      |
| muted   | #6b8ab0 | Secondary/disabled text|
| text    | #dcdce4 | Primary text           |
| bright  | #f8f8fc | Emphasized text        |
| accent  | #58a6ff | Interactive highlights |

## Icon Set (Lucide-style)

All icons use 1.5px stroke weight:
- **Bar**: Wifi, Bluetooth, Battery, Volume2
- **Intake**: Search, X, Terminal, Folder, Settings, Globe
- **Control**: Gauge, Monitor, Volume2, Power, Bluetooth, Palette, Info, Cpu, HardDrive, Activity, ArrowUp, ArrowDown, Route, Wifi

## License

GPL-3.0-or-later
