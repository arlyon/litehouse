import Link from "next/link"
import { Badge } from "@/components/ui/badge"
import { Download, Clock, Box } from "lucide-react"

interface PluginCardProps {
  name: string
  description: string
  version: string
  category: string
  downloads: string
  updatedAt: string
}

export function PluginCard({ name, description, version, category, downloads, updatedAt }: PluginCardProps) {
  return (
    <Link
      href={`/registry/${name}/${version}`}
      className="group block p-5 border border-border rounded-lg bg-card hover:bg-secondary/50 hover:border-accent/50 transition-all duration-200"
    >
      <div className="flex items-start justify-between gap-3 mb-3">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-secondary flex items-center justify-center border border-border group-hover:border-accent/30 transition-colors">
            <Box className="w-5 h-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </div>
          <div>
            <h3 className="font-medium text-foreground group-hover:text-accent transition-colors">{name}</h3>
            <span className="text-xs font-mono text-muted-foreground">v{version}</span>
          </div>
        </div>
        <Badge variant="secondary" className="text-xs font-normal shrink-0">
          {category}
        </Badge>
      </div>
      <p className="text-sm text-muted-foreground leading-relaxed mb-4 line-clamp-2">{description}</p>
      <div className="flex items-center gap-4 text-xs text-muted-foreground">
        <span className="flex items-center gap-1.5">
          <Download className="w-3.5 h-3.5" />
          {downloads}
        </span>
        <span className="flex items-center gap-1.5">
          <Clock className="w-3.5 h-3.5" />
          {updatedAt}
        </span>
      </div>
    </Link>
  )
}
