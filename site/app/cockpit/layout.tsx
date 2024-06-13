import { ClerkProvider, RedirectToSignIn, SignedOut } from "@clerk/nextjs";
import { Header } from "../registry/layout";
import { RootToggle } from "fumadocs-ui/components/layout/root-toggle";
import { Home, PlusIcon } from "lucide-react";

const servers = [
  {
    title: "salient-sasquatch",
    description: "Default server",
    url: "/cockpit/salient-sasquatch",
    icon: <Home />,
  },
  {
    title: "furious-bone",
    description: "Default server",
    url: "/cockpit/furious-bone",
    icon: <Home />,
  },
  {
    title: "Add New",
    description: "Connect to a server",
    url: "/cockpit",
    icon: <PlusIcon />,
  },
];

export default function CockpitPage({ children }) {
  return (
    <ClerkProvider>
      <Header title="Cockpit" suspend={false} />
      <SignedOut>
        <RedirectToSignIn />
      </SignedOut>
      <div className="relative">
        {children}
        <div className="absolute top-4 sm:left-4 px-4 py-2 rounded-xl bg-primary-foreground border">
          <RootToggle options={servers} />
        </div>
      </div>
    </ClerkProvider>
  );
}
