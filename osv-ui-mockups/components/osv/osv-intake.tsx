import { Search, X, Terminal, Globe, FileText, Settings, FolderOpen, Code } from "lucide-react"

export function OsvIntake() {
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
      {/* Search Box */}
      <div className="p-2">
        <div className="h-8 flex items-center gap-2 px-3 rounded-md" style={{ backgroundColor: "#1a3356" }}>
          <Search size={16} strokeWidth={1.5} style={{ color: "#6b8ab0" }} />
          <span className="flex-1 text-[13px]" style={{ color: "#dcdce4" }}>
            term
          </span>
          <X size={16} strokeWidth={1.5} style={{ color: "#6b8ab0" }} className="cursor-pointer hover:opacity-80" />
        </div>
      </div>

      {/* Results List */}
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
