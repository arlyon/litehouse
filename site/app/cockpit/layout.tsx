import { ClerkProvider, RedirectToSignIn, SignedOut } from "@clerk/nextjs";
import { Header } from "../registry/layout";
import { RefreshingToggle } from "@/components/refreshing-toggle";

export default async function CockpitPage({ children }) {
  console.log("im here");
  // const data = await fetch("http://localhost:3001/client", {
  //   headers: {
  //     Authorization: "Bearer 1234",
  //   },
  // });
  // const servers = await data.json();

  // console.log(servers);
  //
  const servers = []

  return (
    <ClerkProvider>
      <Header title="Cockpit" suspend={false} />
      <SignedOut>
        <RedirectToSignIn />
      </SignedOut>
      <div className="relative flex-1 flex">
        {children}
        <div className="absolute top-4 sm:left-4 px-4 py-2 rounded-xl bg-primary-foreground border">
          <RefreshingToggle initialData={servers} />
        </div>
      </div>
    </ClerkProvider>
  );
}
