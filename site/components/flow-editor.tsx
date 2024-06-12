"use client";

import {
  Fragment,
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react";
import {
  Background,
  Handle,
  ReactFlow,
  addEdge,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import { RootToggle } from "fumadocs-ui/components/layout/root-toggle";

import Link from "next/link";

import "@xyflow/react/dist/style.css";
import { Home, InfoIcon } from "lucide-react";
import { ActionSidePanel } from "./action-side-panel";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "./ui/sheet";
import { Drawer, DrawerContent, DrawerTrigger } from "./ui/drawer";
import { Button } from "./ui/button";
import { useMediaQuery } from "@/hooks/use-media-query";
import { useNotNull } from "@/hooks/use-not-null";

const initialNodes = [
  { id: "1", position: { x: 0, y: 0 }, data: { label: "1" } },
  { id: "2", position: { x: 0, y: 100 }, data: { label: "2" } },
];
const initialEdges = [{ id: "e1-2", source: "1", target: "2" }];

const ctx = createContext({});

export function FlowEditor({ initialNodes, initialEdges }) {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [selected, setSelected] = useState<string | null>(null);
  const lastSelected = useNotNull(selected);

  const isDesktop = useMediaQuery("(min-width: 768px)");

  const onConnect = useCallback(
    (params) => setEdges((eds) => addEdge(params, eds)),
    [setEdges],
  );

  const types = useMemo(() => ({
    input: Node,
    output: Node,
  }));

  return (
    <ctx.Provider value={{ onClick: (id: string) => setSelected(id) }}>
      <div className="relative flex-1 flex flex-col">
        <ReactFlow
          nodeTypes={types}
          colorMode="dark"
          style={{ height: undefined }}
          className="flex-1"
          panOnScroll={true}
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
        >
          <Background variant="dots" gap={10} size={1} />
        </ReactFlow>
        <div className="absolute top-4 left-4 px-4 py-2 rounded-xl bg-primary-foreground border">
          <RootToggle
            options={[
              {
                title: "salient-sasquach",
                icon: <Home />,
                description: "Default server",
                url: "/cockpit/salient-sasquach",
              },
            ]}
          />
        </div>
        {isDesktop ? (
          selected !== null ? (
            <div className="hidden md:block absolute right-0 w-96 h-full bg-background border-l">
              <ActionSidePanel onClose={() => setSelected(null)} />
            </div>
          ) : null
        ) : (
          <Drawer open={selected !== null} onClose={() => setSelected(null)}>
            <DrawerContent>
              <ActionSidePanel className="max-h-[400px]" />
            </DrawerContent>
          </Drawer>
        )}
      </div>
    </ctx.Provider>
  );
}

const Node = (props) => {
  const { onClick } = useContext(ctx);
  return (
    <div className="flex flex-col gap-2">
      {props.type === "input" ? (
        <Handle id="a" type="source" position="bottom" />
      ) : null}
      <Link
        href="#timer"
        onClick={() => onClick(props.data.label)}
        className="absolute right-2 top-2 text-muted-foreground"
      >
        <InfoIcon className=" size-3" />
      </Link>
      <header className="text-sm flex flex-row items-center gap-1 w-full">
        <div className="size-4">{props.data.icon}</div>
        <span className=" truncate">{props.data.label}</span>
      </header>
      <div className="flex flex-row">
        <input
          type="text"
          className="w-full rounded-l-full py-1 text-center border bg-muted"
        />
        <select className="text-xs rounded-r-full text-center px-2 py-1 border bg-muted border-l-0">
          <option>Secs</option>
          <option>Mins</option>
          <option>Hrs</option>
          <option>Days</option>
          <option>Mnths</option>
        </select>
      </div>
      {props.type === "output" ? (
        <Handle id="b" type="target" position="top" />
      ) : null}
    </div>
  );
};
