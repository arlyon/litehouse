"use client"

import { cn } from "@/lib/utils"

const categories = [
  { name: "All Categories", count: 24 },
  { name: "Sensors", count: 8 },
  { name: "Actuators", count: 5 },
  { name: "Integrations", count: 6 },
  { name: "Utilities", count: 3 },
  { name: "Security", count: 2 },
]

interface CategorySidebarProps {
  selected: string
  onSelect: (category: string) => void
}

export function CategorySidebar({ selected, onSelect }: CategorySidebarProps) {
  return (
    <aside className="w-56 shrink-0 hidden lg:block">
      <h3 className="text-xs font-medium text-foreground uppercase tracking-wider mb-3">Categories</h3>
      <nav className="space-y-0.5">
        {categories.map((category) => (
          <button
            key={category.name}
            onClick={() => onSelect(category.name)}
            className={cn(
              "w-full flex items-center justify-between px-3 py-2 text-sm rounded-md transition-colors text-left",
              selected === category.name
                ? "bg-secondary text-foreground"
                : "text-muted-foreground hover:text-foreground hover:bg-secondary/50",
            )}
          >
            <span>{category.name}</span>
            <span className="text-xs text-muted-foreground">{category.count}</span>
          </button>
        ))}
      </nav>
    </aside>
  )
}
