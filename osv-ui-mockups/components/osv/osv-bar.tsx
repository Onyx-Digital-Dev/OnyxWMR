import { Wifi, Bluetooth, Battery, Volume2 } from "lucide-react"

export function OsvBar() {
  const focusWorkspaces = [
    { id: 1, active: true, occupied: true },
    { id: 2, active: false, occupied: true },
    { id: 3, active: false, occupied: false },
    { id: 4, active: false, occupied: false },
    { id: 5, active: false, occupied: true },
    { id: 6, active: false, occupied: false },
  ]

  const immersionWorkspaces = [
    { id: 7, active: false, occupied: false },
    { id: 8, active: false, occupied: true },
  ]

  return (
    <div
      className="w-full h-4 flex items-center justify-between px-2 rounded-md"
      style={{
        background: "linear-gradient(to bottom, #1a3356, #0E213D)",
        border: "1px solid rgba(42, 74, 110, 0.3)",
        boxShadow: "0 1px 3px rgba(0, 0, 0, 0.35)",
      }}
    >
      {/* Left: Workspace Indicators */}
      <div className="flex items-center gap-1">
        {/* Focus Workspaces */}
        {focusWorkspaces.map((ws) => (
          <div
            key={ws.id}
            className="h-1.5 w-3 rounded-sm"
            style={{
              backgroundColor: ws.active ? "#58a6ff" : ws.occupied ? "#2a4a6e" : "#12294a",
            }}
          />
        ))}

        {/* Separator */}
        <div className="w-px h-2 mx-0.5" style={{ backgroundColor: "#2a4a6e" }} />

        {/* Immersion Workspaces */}
        {immersionWorkspaces.map((ws) => (
          <div
            key={ws.id}
            className="h-1.5 w-1.5 rounded-full"
            style={{
              backgroundColor: ws.active ? "#58a6ff" : ws.occupied ? "#2a4a6e" : "#12294a",
              border: "1px solid #2a4a6e",
            }}
          />
        ))}
      </div>

      {/* Center: Window Title */}
      <div className="text-[11px] truncate max-w-48" style={{ color: "#6b8ab0" }}>
        Firefox — OSV Documentation
      </div>

      {/* Right: System Tray + Clock */}
      <div className="flex items-center gap-2">
        <Wifi size={12} strokeWidth={1.5} style={{ color: "#6b8ab0" }} />
        <Bluetooth size={12} strokeWidth={1.5} style={{ color: "#6b8ab0" }} />
        <div className="flex items-center gap-0.5">
          <Battery size={12} strokeWidth={1.5} style={{ color: "#6b8ab0" }} />
          <span className="text-[10px]" style={{ color: "#6b8ab0" }}>
            87%
          </span>
        </div>
        <Volume2 size={12} strokeWidth={1.5} style={{ color: "#6b8ab0" }} />
        <span className="text-[11px] ml-1" style={{ color: "#6b8ab0" }}>
          14:32
        </span>
      </div>
    </div>
  )
}
