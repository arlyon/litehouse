import { Cockpit } from "@/components/cockpit-page";
import { FlowEditor } from "@/components/flow-editor";
import { auth } from "@clerk/nextjs/server";
import { Edge, type Node } from "@xyflow/react";
import { Home, Mail, Phone, PlusIcon, Timer } from "lucide-react";

export default async function CockpitPage({ params }) {
  // const user = await auth();
  const paramsResolved = await params;
  return <Cockpit nodeId={paramsResolved.slug} />;
}
