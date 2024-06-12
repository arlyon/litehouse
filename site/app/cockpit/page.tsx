import { FlowEditor } from "@/components/flow-editor";
import { auth } from "@clerk/nextjs/server";
import { Edge, type Node } from "@xyflow/react";
import { Timer } from "lucide-react";

const initialNodes = [
  {
    id: "1",
    position: { x: 0, y: 0 },
    data: { label: "Every...", icon: <Timer className="w-full h-full" /> },
    type: "input",
  },
  {
    id: "2",
    position: { x: 0, y: 100 },
    data: { label: "Send A Text" },
    type: "output",
  },
] as Node[];
const initialEdges = [
  {
    id: "e1-2",
    source: "1",
    target: "2",
    animated: true,
    style: { stroke: "red" },
    // className: "stroke-green-500",
  },
] as Edge[];

export default async function CockpitPage() {
  const user = await auth();
  return <FlowEditor initialNodes={initialNodes} initialEdges={initialEdges} />;
}
