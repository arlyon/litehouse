"use client"

import { useState, useEffect } from "react"
import { PluginCard } from "@/components/plugin-card"
import { CategorySidebar } from "@/components/category-sidebar"
import { Input } from "@/components/ui/input"
import { Search, SlidersHorizontal } from "lucide-react"
import { Button } from "@/components/ui/button"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"
import { getPlugins } from "@/lib/registry"

export default function RegistryPage() {
  const [selectedCategory, setSelectedCategory] = useState("All Categories")
  const [searchQuery, setSearchQuery] = useState("")
  const [plugins, setPlugins] = useState<any[]>([])

  useEffect(() => {
    getPlugins().then(packages => {
      // Transform the data to match the expected format
      const transformedPlugins = packages.map(pkg => ({
        name: pkg.title,
        description: pkg.description || "No description available",
        version: pkg.version.version,
        category: "Integrations", // Default category, can be enhanced later
        downloads: "N/A",
        updatedAt: pkg.version.date.toLocaleDateString(),
      }))
      setPlugins(transformedPlugins)
    })
  }, [])

  const filteredPlugins = plugins.filter((plugin) => {
    const matchesCategory = selectedCategory === "All Categories" || plugin.category === selectedCategory
    const matchesSearch =
      plugin.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      plugin.description.toLowerCase().includes(searchQuery.toLowerCase())
    return matchesCategory && matchesSearch
  })

  return (
    <main className="flex-1">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Page Header */}
        <div className="mb-8">
          <h1 className="text-2xl md:text-3xl font-bold text-foreground mb-2">Registry</h1>
          <p className="text-muted-foreground">Browse and discover WASM plugins for Litehouse</p>
        </div>

        {/* Search and Filters */}
        <div className="flex flex-col sm:flex-row gap-3 mb-8">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <Input
              placeholder="Search plugins..."
              className="pl-9 bg-secondary border-border"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" className="gap-2 shrink-0 bg-transparent">
                <SlidersHorizontal className="w-4 h-4" />
                Sort
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem>Most Downloads</DropdownMenuItem>
              <DropdownMenuItem>Recently Updated</DropdownMenuItem>
              <DropdownMenuItem>Alphabetical</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        {/* Main Content */}
        <div className="flex gap-8">
          <CategorySidebar selected={selectedCategory} onSelect={setSelectedCategory} />

          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between mb-4">
              <span className="text-sm text-muted-foreground">
                {filteredPlugins.length} plugin{filteredPlugins.length !== 1 ? "s" : ""} found
              </span>
            </div>

            <div className="grid gap-4">
              {filteredPlugins.map((plugin) => (
                <PluginCard key={plugin.name} {...plugin} />
              ))}
            </div>

            {filteredPlugins.length === 0 && (
              <div className="text-center py-16 border border-border rounded-lg bg-card">
                <p className="text-muted-foreground">No plugins found matching your criteria.</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </main>
  )
}
