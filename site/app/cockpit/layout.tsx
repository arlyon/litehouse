import { ClerkProvider, RedirectToSignIn, SignedOut } from "@clerk/nextjs";
import { Header } from "../registry/layout";
import { RefreshingToggle } from "@/components/refreshing-toggle";
import { auth } from "@clerk/nextjs/server";
import { client } from "@/lib/cockpit-client";
import { SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";

export default async function CockpitPage({ children }) {
  const userData = await auth.protect();

  let servers = [];
  try {
    const data = client["/client"].get({
      headers: {
        authorization: `Bearer ${userData.userId}`,
      },
      signal: AbortSignal.timeout(1000),
    });
    servers = await data.json();
  } catch (e) {
    console.error("failed to get servers", e);
    // no-op
  }

  return (
    <ClerkProvider>
      <SidebarProvider className="flex flex-col">
        <Header title="Cockpit" link="/cockpit" suspend={false} />
        <div className="relative flex-1 flex">
          {/* <AppSidebar /> */}
          {children}
          <div className="absolute top-4 left-4 p-1 rounded-xl bg-background border">
            <RefreshingToggle initialData={servers} userId={userData.userId} />
          </div>
        </div>
      </SidebarProvider>
    </ClerkProvider>
  );
}

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
} from "@/components/ui/sidebar";

export function AppSidebar() {
  return (
    <Sidebar>
      <SidebarHeader />
      <SidebarContent>
        <SidebarGroup />
        <SidebarGroup />
      </SidebarContent>
      <SidebarFooter />
    </Sidebar>
  );
}
