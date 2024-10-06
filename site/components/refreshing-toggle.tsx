"use client";

import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from "@tanstack/react-query";
import { RootToggle } from "fumadocs-ui/components/layout/root-toggle";
import { Home, PlusIcon, Unlink } from "lucide-react";
import { usePathname } from "next/navigation";

const client = new QueryClient();

export const RefreshingToggle = ({ children, initialData }) => (
  <QueryClientProvider client={client}>
    <RefreshingToggleInner children={children} initialData={initialData} />
  </QueryClientProvider>
);

export const RefreshingToggleInner = ({ children, initialData }) => {
  const currentPath = usePathname();

  const query = useQuery({
    queryKey: ["knownServers"],
    queryFn: async () => {
      const data = await fetch("http://localhost:3001/client", {
        headers: {
          Authorization: "Bearer 1234",
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
      title: s.identifier,
      description: "A nice server!",
      url: `/cockpit/${s.identifier}`,
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
