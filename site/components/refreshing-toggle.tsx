"use client";

import { client } from "@/lib/cockpit-client";
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from "@tanstack/react-query";
import { RootToggle } from "fumadocs-ui/components/layout/root-toggle";
import { Home, PlusIcon, Unlink } from "lucide-react";
import { usePathname } from "next/navigation";

const queryClient = new QueryClient();

export const RefreshingToggle = ({ children, initialData, userId }) => (
  <QueryClientProvider client={queryClient}>
    <RefreshingToggleInner
      children={children}
      initialData={initialData}
      userId={userId}
    />
  </QueryClientProvider>
);

export const RefreshingToggleInner = ({ children, initialData, userId }) => {
  const currentPath = usePathname();

  const query = useQuery({
    queryKey: ["knownServers"],
    queryFn: async () => {
      const data = await client["/client"].get({
        headers: {
          authorization: `Bearer ${userId}`,
        },
      });
      const servers = await data.json();
      return servers;
    },
    select: (data) => data.filter((s) => s.type === "known"),
    refetchInterval: 5000,
    initialData: initialData,
  });

  const options = [
    ...(query.data?.map((s) => ({
      title: s.node_id,
      description: "A nice server!",
      url: `/cockpit/${s.node_id}`,
      icon: <Home />,
    })) ?? []),
    {
      title: "Add New",
      description: "Connect to a server",
      url: "/cockpit",
      icon: <PlusIcon />,
    },
  ];

  if (options.find((o) => o.url === currentPath) === undefined) {
    options.unshift({
      title: "Reconnecting...",
      icon: <Unlink />,
      url: currentPath,
      description: "We have lost connection to the server",
    });
  }

  return <RootToggle options={options} />;
};
