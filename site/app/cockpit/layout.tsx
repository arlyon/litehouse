import { Header } from "../registry/layout";
import { RefreshingToggle } from "@/components/refreshing-toggle";
import { auth } from "@/lib/auth";
import { client } from "@/lib/cockpit-client";
import { SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { headers } from "next/headers";
import { redirect } from "next/navigation";

export default async function CockpitPage({ children }) {
  const session = await auth.api.getSession({
    headers: await headers(),
  });

  if (!session) {
    redirect("/sign-in");
  }

  let servers = [];
  try {
    const data = client["/client"].get({
      headers: {
        authorization: `Bearer ${session.user.id}`,
      },
      signal: AbortSignal.timeout(1000),
    });
    servers = await data.json();
  } catch (e) {
    console.error("failed to get servers", e);
    // no-op
  }

  return (
    <SidebarProvider className="flex flex-col">
      <Header title="Cockpit" link="/cockpit" suspend={false} />
      <div className="relative flex-1 flex">
        {/* <AppSidebar /> */}
        {children}
        <div className="absolute top-4 left-4 p-1 rounded-xl bg-background border">
          <RefreshingToggle initialData={servers} userId={session.user.id} />
        </div>
      </div>
    </SidebarProvider>
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
