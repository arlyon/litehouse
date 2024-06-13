import { FlowEditor } from "@/components/flow-editor";
import { auth } from "@clerk/nextjs/server";
import { Edge, type Node } from "@xyflow/react";
import { Home, Mail, Phone, PlusIcon, Timer } from "lucide-react";
import { redirect } from "next/navigation";

const servers = [
  {
    title: "salient-sasquatch",
    description: "Default server",
    url: "/cockpit/salient-sasquatch",
    icon: <Home />,
    data: {
      nodes: [
        {
          id: "1",
          position: { x: 0, y: 0 },
          data: {
            label: "Every...",
            icon: <Timer className="w-full h-full" />,
          },
          type: "input",
        },
        {
          id: "2",
          position: { x: 0, y: 100 },
          data: { label: "Send A Text" },
          type: "output",
        },
      ],
      edges: [
        {
          id: "e1-2",
          source: "1",
          target: "2",
          animated: true,
          style: { stroke: "red" },
          // className: "stroke-green-500",
        },
      ],
    },
  },
  {
    title: "furious-bone",
    description: "Default server",
    url: "/cockpit/furious-bone",
    icon: <Home />,
    data: {
      nodes: [
        {
          id: "1",
          position: { x: 0, y: 0 },
          data: {
            label: "Every...",
            icon: <Timer className="w-full h-full" />,
          },
          type: "input",
        },
        {
          id: "2",
          position: { x: -90, y: 150 },
          data: {
            label: "Send A Text",
            icon: <Phone className="w-full h-full" />,
          },
          type: "output",
        },
        {
          id: "3",
          position: { x: 90, y: 150 },
          data: {
            label: "Send an Email",
            icon: <Mail className="w-full h-full" />,
          },
          type: "output",
        },
      ],
      edges: [
        {
          id: "e1-2",
          source: "1",
          target: "2",
          animated: true,
          style: { stroke: "red" },
          // className: "stroke-green-500",
        },
        {
          id: "e1-3",
          source: "1",
          target: "3",
          animated: true,
          style: { stroke: "red" },
          // className: "stroke-green-500",
        },
      ],
    },
  },
  {
    title: "Add New",
    description: "Connect to a server",
    url: "/cockpit",
    icon: <PlusIcon />,
  },
];

export default async function CockpitPage({ params }) {
  const server = servers.find((s) => s.title === params.slug);
  if (server?.data === undefined) {
    // nextjs redirect
    redirect("/cockpit");
  }

  const user = await auth();
  return (
    <FlowEditor
      initialNodes={server.data.nodes}
      initialEdges={server.data.edges}
      servers={servers}
      selectedServer={server}
    />
  );
}
