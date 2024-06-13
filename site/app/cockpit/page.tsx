import { FindServer } from "@/components/find-server";
import { Globe } from "@/components/globe";
import { RootToggle } from "fumadocs-ui/components/layout/root-toggle";

import { Home, Loader, PlusIcon } from "lucide-react";

export default function CockpitPage() {
  return (
    <div className="flex flex-col flex-1 relative items-center pt-32 justify-center bg-gradient-to-t from-green-950 to-black">
      <h1 className="text-4xl md:text-6xl font-bold text-center mx-8">
        Searching for Litehouses...
      </h1>
      <div className="w-[600px]">
        <Globe />
      </div>
      <FindServer className="relative -top-60" />
    </div>
  );
}
