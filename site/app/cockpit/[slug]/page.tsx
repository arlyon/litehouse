import { Cockpit } from "@/components/cockpit-page";
import { FlowEditor } from "@/components/flow-editor";
import { Edge, type Node } from "@xyflow/react";
import { Home, Mail, Phone, PlusIcon, Timer } from "lucide-react";

export default async function CockpitPage({ params }) {
  const paramsResolved = await params;
  return <Cockpit nodeId={paramsResolved.slug} />;
}
