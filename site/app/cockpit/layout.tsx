import { ClerkProvider, RedirectToSignIn, SignedOut } from "@clerk/nextjs";
import { Header } from "../registry/layout";
import { RefreshingToggle } from "@/components/refreshing-toggle";
import { auth } from "@clerk/nextjs/server";
import { client } from "@/lib/cockpit-client";

export default async function CockpitPage({ children }) {
  const userData = await auth.protect();

  let servers = [];
  try {
    const data = client["/client"].get({
      headers: {
        authorization: `Bearer ${userData.userId}`,
      },
    });
    servers = await data.json();
  } catch (e) {
    console.error("failed to get servers", e);
    // no-op
  }

  return (
    <ClerkProvider>
      <Header title="Cockpit" link="/cockpit" suspend={false} />
      <div className="relative flex-1 flex">
        {children}
        <div className="absolute top-4 sm:left-4 px-4 py-2 rounded-xl bg-primary-foreground border">
          <RefreshingToggle initialData={servers} userId={userData.userId} />
        </div>
      </div>
    </ClerkProvider>
  );
}
