import { FindServer } from "@/components/find-server";
import { Globe } from "@/components/globe";
import { RootToggle } from "fumadocs-ui/components/layout/root-toggle";

import { Home, Loader, PlusIcon } from "lucide-react";

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

export default function CockpitPage() {
  return (
    <div className="flex flex-col flex-1 relative items-center justify-center bg-gradient-to-t from-green-950 to-black">
      <div className="absolute top-4 left-4 px-4 py-2 rounded-xl bg-primary-foreground border">
        <RootToggle options={servers} />
      </div>
      <h1 className="text-6xl font-bold text-center">
        Searching for Litehouses...
      </h1>
      <Globe />
      <FindServer />
    </div>
  );
}
