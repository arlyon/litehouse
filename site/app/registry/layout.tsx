import { LoginButton } from "@/components/login-button";
import { ManifestEditor } from "@/components/manifest-editor";
import { ThemeToggle } from "@/components/theme-toggle";
import { ClerkProvider } from "@clerk/nextjs";
import { PackageIcon, SearchIcon } from "lucide-react";
import Link from "next/link";
import { Suspense, type PropsWithChildren, Error, Fragment } from "react";
import { ErrorBoundary } from "react-error-boundary";
import { Logo } from "../page";
import { Toolbar } from "@/components/toolbar";

const Suspend = ({ children }) => (
  <Suspense
    fallback={
      <div className="animate-pulse bg-muted rounded-full size-[28px]" />
    }
  >
    {children}
  </Suspense>
);

export const Header = ({
  title,
  provider: Provider = Fragment,
  suspend = true,
}) => {
  const Suspense = suspend ? Suspend : Fragment;
  return (
    <div className="sticky top-0 z-50 h-16 border-b transition-colors border-foreground/10 bg-background/50 backdrop-blur-md">
      <div className="mx-auto flex size-full max-w-container flex-row items-center justify-between gap-4 px-4">
        <h1 className="font-semibold flex items-center">
          <Link href="/">
            <Logo className="mr-2 h-6 w-6" />
          </Link>
          <Link href="/registry" className="w-max">
            Litehouse <span className="text-green-600">{title}</span>
          </Link>
        </h1>
        <div className="relative w-full max-w-md flex items-center gap-2">
          <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
            <SearchIcon className="h-5 w-5 text-primary" />
          </div>
          <input
            className="block w-full rounded-full border-accent bg-secondary border pl-10 pr-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent dark:focus:ring-neutral-50"
            placeholder="Search packages..."
            type="text"
          />
          <ThemeToggle className="hidden md:inline-flex" />
          <ManifestEditor className="hidden md:block" />
          <div>
            <div className="size-[28px]">
              <ErrorBoundary
                fallback={
                  <div>
                    <div className="animate-pulse size-[28px] rounded-full bg-muted" />
                  </div>
                }
              >
                <Suspense>
                  <Provider>
                    <Toolbar />
                    <LoginButton />
                  </Provider>
                </Suspense>
              </ErrorBoundary>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

const Layout = ({ children }: PropsWithChildren<unknown>) => {
  return (
    <div>
      <Header title="Registry" provider={ClerkProvider} />
      {/* <div className="flex justify-between mb-6">
        <div>
          <p className="text-muted-foreground">Total Downloads</p>
          <p className="font-bold">{props.totalDownloads}</p>
        </div>
        <div>
          <p className="text-muted-foreground">Plugin Count</p>
          <p className="font-bold">{props.pluginCount}</p>
        </div>
        <div>
          <p className="text-muted-foreground">Users</p>
          <p className="font-bold">{props.users}</p>
        </div>
      </div> */}
      <div className="container">{children}</div>
    </div>
  );
};

export default Layout;
