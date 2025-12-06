import Link from "next/link"
import { Button } from "@/components/ui/button"

export function Header() {
    return (
        <header className="border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
            <div className="container flex h-14 items-center">
                <div className="mr-4 hidden md:flex">
                    <Link href="/" className="mr-6 flex items-center space-x-2">
                        <span className="hidden font-bold sm:inline-block">
                            Litehouse
                        </span>
                    </Link>
                    <nav className="flex items-center space-x-6 text-sm font-medium">
                        <Link
                            href="/registry"
                            className="transition-colors hover:text-foreground/80 text-foreground"
                        >
                            Registry
                        </Link>
                        <Link
                            href="/docs"
                            className="transition-colors hover:text-foreground/80 text-foreground/60"
                        >
                            Documentation
                        </Link>
                    </nav>
                </div>
                <div className="flex flex-1 items-center justify-between space-x-2 md:justify-end">
                    <div className="w-full flex-1 md:w-auto md:flex-none">
                    </div>
                    <nav className="flex items-center">
                        <Button variant="ghost" asChild>
                            <Link href="/login">Login</Link>
                        </Button>
                    </nav>
                </div>
            </div>
        </header>
    )
}
