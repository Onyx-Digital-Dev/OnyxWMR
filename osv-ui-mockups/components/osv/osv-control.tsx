import { Wifi, Monitor, Volume2, Battery, Bluetooth, Palette, Info, ChevronDown, Sun, Moon } from "lucide-react"

export function OsvControl() {
  const navItems = [
    { name: "Network", icon: Wifi, active: false },
    { name: "Display", icon: Monitor, active: true },
    { name: "Sound", icon: Volume2, active: false },
    { name: "Power", icon: Battery, active: false },
    { name: "Bluetooth", icon: Bluetooth, active: false },
    { name: "Appearance", icon: Palette, active: false },
    { name: "About", icon: Info, active: false },
  ]

  return (
    <div
      className="w-[800px] h-[600px] rounded-lg overflow-hidden flex"
      style={{
        backgroundColor: "#0E213D",
        border: "1px solid #2a4a6e",
        boxShadow: "0 4px 8px rgba(0, 0, 0, 0.5)",
      }}
    >
      {/* Left Sidebar */}
      <div className="w-[200px] py-2 flex-shrink-0" style={{ backgroundColor: "#12294a" }}>
        {navItems.map((item) => {
          const Icon = item.icon
          return (
            <div
              key={item.name}
              className="h-9 flex items-center gap-3 px-4 cursor-pointer"
              style={{
                backgroundColor: item.active ? "#1a3356" : "transparent",
                borderLeft: item.active ? "2px solid #58a6ff" : "2px solid transparent",
              }}
            >
              <Icon size={16} strokeWidth={1.5} style={{ color: item.active ? "#dcdce4" : "#6b8ab0" }} />
              <span className="text-[13px]" style={{ color: item.active ? "#dcdce4" : "#6b8ab0" }}>
                {item.name}
              </span>
            </div>
          )
        })}
      </div>

      {/* Right Content Area */}
      <div className="flex-1 p-4 overflow-auto">
        {/* Section Header */}
        <div className="text-[11px] uppercase tracking-wider mb-4" style={{ color: "#6b8ab0" }}>
          Display Settings
        </div>

        {/* Brightness Card */}
        <div className="rounded-lg p-4 mb-4" style={{ backgroundColor: "#1a3356" }}>
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <Sun size={16} strokeWidth={1.5} style={{ color: "#dcdce4" }} />
              <span className="text-[13px]" style={{ color: "#dcdce4" }}>
                Brightness
              </span>
            </div>
            <span className="text-[13px]" style={{ color: "#6b8ab0" }}>
              75%
            </span>
          </div>

          {/* Slider */}
          <div className="h-1.5 rounded-full overflow-hidden" style={{ backgroundColor: "#12294a" }}>
            <div className="h-full w-3/4 rounded-full" style={{ backgroundColor: "#58a6ff" }} />
          </div>
        </div>

        {/* Night Mode Card */}
        <div className="rounded-lg p-4 mb-4" style={{ backgroundColor: "#1a3356" }}>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Moon size={16} strokeWidth={1.5} style={{ color: "#dcdce4" }} />
              <span className="text-[13px]" style={{ color: "#dcdce4" }}>
                Night Mode
              </span>
            </div>

            {/* Toggle Switch - Enabled */}
            <div className="w-9 h-5 rounded-full p-0.5 cursor-pointer" style={{ backgroundColor: "#58a6ff" }}>
              <div className="w-4 h-4 rounded-full ml-auto" style={{ backgroundColor: "#f8f8fc" }} />
            </div>
          </div>
        </div>

        {/* Resolution Card */}
        <div className="rounded-lg p-4 mb-4" style={{ backgroundColor: "#1a3356" }}>
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <Monitor size={16} strokeWidth={1.5} style={{ color: "#dcdce4" }} />
              <span className="text-[13px]" style={{ color: "#dcdce4" }}>
                Resolution
              </span>
            </div>
          </div>

          {/* Dropdown Select */}
          <div
            className="h-8 flex items-center justify-between px-3 rounded-md cursor-pointer"
            style={{ backgroundColor: "#12294a", border: "1px solid #2a4a6e" }}
          >
            <span className="text-[13px]" style={{ color: "#dcdce4" }}>
              2560 x 1440 @ 144Hz
            </span>
            <ChevronDown size={16} strokeWidth={1.5} style={{ color: "#6b8ab0" }} />
          </div>
        </div>

        {/* Refresh Rate Card */}
        <div className="rounded-lg p-4" style={{ backgroundColor: "#1a3356" }}>
          <div className="text-[11px] uppercase tracking-wider mb-3" style={{ color: "#6b8ab0" }}>
            Display Info
          </div>

          <div className="space-y-2">
            <div className="flex justify-between">
              <span className="text-[13px]" style={{ color: "#6b8ab0" }}>
                Monitor
              </span>
              <span className="text-[13px]" style={{ color: "#dcdce4" }}>
                Dell U2723QE
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-[13px]" style={{ color: "#6b8ab0" }}>
                Connection
              </span>
              <span className="text-[13px]" style={{ color: "#dcdce4" }}>
                DisplayPort
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-[13px]" style={{ color: "#6b8ab0" }}>
                Color Depth
              </span>
              <span className="text-[13px]" style={{ color: "#dcdce4" }}>
                10-bit
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
