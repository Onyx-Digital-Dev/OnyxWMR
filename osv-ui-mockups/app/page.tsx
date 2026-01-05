import type React from "react"
import {
  Wifi,
  Bluetooth,
  Battery,
  Volume2,
  Search,
  X,
  Terminal,
  Globe,
  FileText,
  Settings,
  FolderOpen,
  Code,
  Monitor,
  Palette,
  Info,
  Gauge,
  Cpu,
  HardDrive,
  ArrowUp,
  ArrowDown,
  Activity,
  Route,
} from "lucide-react"
import { Suspense } from "react"

// Telemetry metric card component
function TelemetryCard({
  icon: Icon,
  label,
  value,
  unit,
  barValue,
  barColor,
}: {
  icon: React.ComponentType<{ size?: number; strokeWidth?: number; style?: React.CSSProperties }>
  label: string
  value: string
  unit: string
  barValue: number
  barColor: string
}) {
  return (
    <div
      className="rounded-md p-2"
      style={{
        backgroundColor: "#1a3356",
        border: "1px solid #2a4a6e",
      }}
    >
      <div className="flex items-start gap-2">
        <div
          className="w-8 h-8 rounded flex items-center justify-center flex-shrink-0"
          style={{ backgroundColor: "#12294a" }}
        >
          <Icon size={16} strokeWidth={1.5} style={{ color: barColor }} />
        </div>
        <div className="flex-1 min-w-0">
          <div className="text-[11px]" style={{ color: "#6b8ab0" }}>
            {label}
          </div>
          <div className="flex items-baseline">
            <span className="text-lg font-semibold" style={{ color: "#f8f8fc" }}>
              {value}
            </span>
            <span className="text-[13px] ml-0.5" style={{ color: "#6b8ab0" }}>
              {unit}
            </span>
          </div>
          <div className="h-1 rounded-full mt-1" style={{ backgroundColor: "#12294a" }}>
            <div className="h-full rounded-full" style={{ width: `${barValue * 100}%`, backgroundColor: barColor }} />
          </div>
        </div>
      </div>
    </div>
  )
}

function OsvBar() {
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
        {focusWorkspaces.map((ws) => (
          <div
            key={ws.id}
            className="h-1.5 w-3 rounded-sm"
            style={{
              backgroundColor: ws.active ? "#58a6ff" : ws.occupied ? "#2a4a6e" : "#12294a",
            }}
          />
        ))}
        <div className="w-px h-2 mx-0.5" style={{ backgroundColor: "#2a4a6e" }} />
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

      {/* Center: Onyx OSV branding */}
      <div className="text-[11px] font-medium tracking-wide" style={{ color: "#dcdce4" }}>
        Onyx OSV
      </div>

      {/* Right: Time only */}
      <span className="text-[11px]" style={{ color: "#6b8ab0" }}>
        14:32
      </span>
    </div>
  )
}

function OsvIntake() {
  const applications = [
    { name: "Terminal", icon: Terminal, selected: true },
    { name: "Firefox", icon: Globe, selected: false },
    { name: "Files", icon: FolderOpen, selected: false },
    { name: "Text Editor", icon: FileText, selected: false },
    { name: "VS Code", icon: Code, selected: false },
    { name: "Settings", icon: Settings, selected: false },
  ]

  return (
    <div
      className="w-[400px] rounded-lg overflow-hidden"
      style={{
        backgroundColor: "rgba(14, 33, 61, 0.95)",
        border: "1px solid #2a4a6e",
        boxShadow: "0 4px 8px rgba(0, 0, 0, 0.5)",
      }}
    >
      <div className="p-2">
        <div className="h-8 flex items-center gap-2 px-3 rounded-md" style={{ backgroundColor: "#1a3356" }}>
          <Search size={16} strokeWidth={1.5} style={{ color: "#6b8ab0" }} />
          <span className="flex-1 text-[13px]" style={{ color: "#dcdce4" }}>
            term
          </span>
          <X size={16} strokeWidth={1.5} style={{ color: "#6b8ab0" }} className="cursor-pointer hover:opacity-80" />
        </div>
      </div>
      <div className="pb-2">
        {applications.map((app) => {
          const Icon = app.icon
          return (
            <div
              key={app.name}
              className="h-7 flex items-center gap-3 px-3"
              style={{
                backgroundColor: app.selected ? "#1a3356" : "transparent",
                borderLeft: app.selected ? "2px solid #58a6ff" : "2px solid transparent",
              }}
            >
              <Icon size={16} strokeWidth={1.5} style={{ color: app.selected ? "#dcdce4" : "#6b8ab0" }} />
              <span className="text-[13px]" style={{ color: app.selected ? "#dcdce4" : "#6b8ab0" }}>
                {app.name}
              </span>
            </div>
          )
        })}
      </div>
    </div>
  )
}

function OsvControl() {
  const navItems = [
    { name: "Telemetry", icon: Gauge, active: true },
    { name: "Network", icon: Wifi, active: false },
    { name: "Display", icon: Monitor, active: false },
    { name: "Sound", icon: Volume2, active: false },
    { name: "Power", icon: Battery, active: false },
    { name: "Bluetooth", icon: Bluetooth, active: false },
    { name: "Appearance", icon: Palette, active: false },
    { name: "About", icon: Info, active: false },
  ]

  const processes = [
    { name: "osvwm", status: "running", cpu: "2.4%", memory: "128 MB" },
    { name: "osv-bar", status: "running", cpu: "0.8%", memory: "24 MB" },
    { name: "firefox", status: "running", cpu: "12.3%", memory: "1.2 GB" },
    { name: "alacritty", status: "running", cpu: "0.2%", memory: "48 MB" },
    { name: "pipewire", status: "running", cpu: "1.1%", memory: "32 MB" },
    { name: "dbus-daemon", status: "idle", cpu: "0.0%", memory: "8 MB" },
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
      {/* Left sidebar navigation */}
      <div className="w-[200px] py-4 flex-shrink-0" style={{ backgroundColor: "#12294a" }}>
        <div className="text-[11px] uppercase tracking-widest px-3 mb-2" style={{ color: "#6b8ab0" }}>
          System
        </div>
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

      {/* Right content area - Telemetry Dashboard */}
      <div className="flex-1 p-4 overflow-auto">
        <div className="text-[11px] uppercase tracking-widest mb-4" style={{ color: "#6b8ab0" }}>
          System Telemetry
        </div>

        {/* Top row: CPU, Memory, Disk */}
        <div className="grid grid-cols-3 gap-2 mb-3">
          <TelemetryCard icon={Cpu} label="CPU Usage" value="42" unit="%" barValue={0.42} barColor="#58a6ff" />
          <TelemetryCard icon={HardDrive} label="Memory" value="67" unit="%" barValue={0.67} barColor="#a78bfa" />
          <TelemetryCard icon={Activity} label="Disk" value="35" unit="%" barValue={0.35} barColor="#4ade80" />
        </div>

        {/* Network throughput */}
        <div className="grid grid-cols-2 gap-2 mb-4">
          <TelemetryCard
            icon={ArrowUp}
            label="Network Upload"
            value="1.2"
            unit="MB/s"
            barValue={0.3}
            barColor="#22d3ee"
          />
          <TelemetryCard
            icon={ArrowDown}
            label="Network Download"
            value="15.8"
            unit="MB/s"
            barValue={0.65}
            barColor="#22d3ee"
          />
        </div>

        {/* Active Routes / Processes */}
        <div className="flex items-center justify-between mb-2">
          <span className="text-[11px] uppercase tracking-widest" style={{ color: "#6b8ab0" }}>
            Active Routes
          </span>
          <Route size={14} strokeWidth={1.5} style={{ color: "#6b8ab0" }} />
        </div>

        <div
          className="rounded-md overflow-hidden"
          style={{
            backgroundColor: "#1a3356",
            border: "1px solid #2a4a6e",
          }}
        >
          {/* Table header */}
          <div className="flex items-center h-6 px-2 text-[10px] uppercase tracking-wider" style={{ color: "#6b8ab0" }}>
            <div className="w-3" />
            <div className="flex-1 pl-2">Process</div>
            <div className="w-12 text-right">CPU</div>
            <div className="w-16 text-right">Mem</div>
          </div>

          {/* Process rows */}
          {processes.map((proc) => (
            <div key={proc.name} className="flex items-center h-8 px-2" style={{ borderTop: "1px solid #2a4a6e" }}>
              <div
                className="w-2 h-2 rounded-full"
                style={{
                  backgroundColor:
                    proc.status === "running" ? "#4ade80" : proc.status === "idle" ? "#6b8ab0" : "#f87171",
                }}
              />
              <div className="flex-1 pl-2 text-[13px]" style={{ color: "#dcdce4" }}>
                {proc.name}
              </div>
              <div className="w-12 text-right text-[11px]" style={{ color: "#6b8ab0" }}>
                {proc.cpu}
              </div>
              <div className="w-16 text-right text-[11px]" style={{ color: "#6b8ab0" }}>
                {proc.memory}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

function PageContent() {
  return (
    <div className="min-h-screen p-8" style={{ backgroundColor: "#0a1628" }}>
      {/* Mockup 1: osv-bar */}
      <section className="mb-12">
        <h2 className="text-[15px] mb-4" style={{ color: "#f8f8fc" }}>
          osv-bar
        </h2>
        <div className="max-w-4xl">
          <OsvBar />
        </div>
        <div className="mt-3 text-[11px]" style={{ color: "#6b8ab0" }}>
          Left: 6 Focus capsules + 2 Immersion circles | Center: "Onyx OSV" | Right: Time
        </div>
      </section>

      {/* Mockup 2: osv-intake */}
      <section className="mb-12">
        <h2 className="text-[15px] mb-4" style={{ color: "#f8f8fc" }}>
          osv-intake
        </h2>
        <OsvIntake />
        <div className="mt-3 text-[11px]" style={{ color: "#6b8ab0" }}>
          400px launcher with search input and filtered application results
        </div>
      </section>

      {/* Mockup 3: osv-control */}
      <section>
        <h2 className="text-[15px] mb-4" style={{ color: "#f8f8fc" }}>
          osv-control
        </h2>
        <OsvControl />
        <div className="mt-3 text-[11px]" style={{ color: "#6b8ab0" }}>
          800x600 settings panel | Default: Telemetry dashboard with CPU/Memory/Disk, Network throughput, Active Routes
        </div>
      </section>
    </div>
  )
}

export default function Page() {
  return (
    <Suspense fallback={null}>
      <PageContent />
    </Suspense>
  )
}
