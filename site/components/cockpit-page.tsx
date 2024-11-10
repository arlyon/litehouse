"use client";

import { CogIcon, FullscreenIcon, Timer } from "lucide-react";
import { FlowEditor } from "./flow-editor";
import { useEffect, useMemo, useRef, useState } from "react";
import { client } from "@/lib/cockpit-client";
import { cn } from "@/lib/utils";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./ui/dialog";
import {
  AnimatePresence,
  AnimateSharedLayout,
  LayoutGroup,
  motion,
} from "framer-motion";
import { Sidebar } from "./ui/sidebar";
import { Button } from "./ui/button";

export const Cockpit = ({ nodeId }) => {
  const [dcOpen, setDcOpen] = useState(false);
  const [messages, setMessages] = useState([]);
  const [iceConnectionState, setIceConnectionState] =
    useState<RTCIceConnectionState>("new");
  const elementRef = useRef();

  useEffect(() => {
    if (!("window" in globalThis)) {
      return;
    }

    const pc = new RTCPeerConnection({
      iceServers: [
        {
          urls: "stun:stun.l.google.com:19302",
        },
        { urls: "stun:stun1.l.google.com:19302" },
        { urls: "stun:stun2.l.google.com:19302" },
        { urls: "stun:stun3.l.google.com:19302" },
        { urls: "stun:stun4.l.google.com:19302" },
      ],
    });
    const dc = pc.createDataChannel("data");

    dc.onopen = () => {
      setDcOpen(true);
    };

    dc.onmessage = (event) => {
      const message = JSON.parse(event.data);
      setMessages((messages) => [message, ...messages]);
    };

    pc.oniceconnectionstatechange = () => {
      const iceConnectionState = pc.iceConnectionState;
      setIceConnectionState(iceConnectionState);
    };

    (async () => {
      let res;
      try {
        const offer = await pc.createOffer();
        pc.setLocalDescription(offer);
        res = await client["/client/{id}"].post({
          json: offer,
          params: {
            id: nodeId,
          },
          headers: {
            authorization: `Bearer ${nodeId}`,
          },
        });
      } catch (e) {
        console.error("COULD NOT CONNECT", e);
        return;
      }
      const answer = await res.json();
      pc.setRemoteDescription(answer);
    })();
  }, [nodeId]);

  useEffect(() => {
    console.log(messages);
  }, [messages]);

  const server = {
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
          className:
            iceConnectionState === "connected"
              ? "stroke-green-500"
              : "stroke-red-500",
        },
      ],
    },
  };

  const TRACE_COLOR = {
    TRACE: "text-blue-500",
    DEBUG: "text-blue-500",
    INFO: "text-green-500",
    WARN: "text-orange-500",
    ERROR: "text-red-500",
  };

  const messageTypes = useMemo(() => {
    const types = new Set();
    messages.forEach((m) => types.add(m.source));
    const array = Array.from(types);
    array.sort();
    return array;
  }, [messages]);

  const [filtered, setFiltered] = useState([]);

  return (
    <div ref={elementRef} className="relative flex-1 flex">
      <Button
        className="absolute bottom-4 right-4 z-10"
        variant="ghost"
        onClick={() => elementRef.current?.requestFullscreen()}
      >
        <FullscreenIcon className="size-12" />
      </Button>
      <Dialog>
        <DialogTrigger
          className={cn(
            "size-4 absolute top-4 right-4 z-50 rounded-full border animate-pulse",
            iceConnectionState === "connected"
              ? "bg-green-500 border-green-800"
              : "bg-red-500 border-red-800",
          )}
        ></DialogTrigger>
        <DialogContent className="max-w-[calc(100%-4em)] h-full max-h-[calc(100%-4em)] flex flex-col">
          <DialogHeader>
            <div className="flex flex-row items-center gap-3">
              <DialogTitle className="mr-2">Server Logs</DialogTitle>
              {messageTypes.map((type) => (
                <button
                  key={type}
                  type="button"
                  className={cn(
                    filtered.includes(type) ? "text-muted" : null,
                    "text-xs border rounded-full px-3 py-1.5 transition-colors",
                  )}
                  onClick={() =>
                    setFiltered((data) =>
                      data.includes(type)
                        ? data.filter((t) => t !== type)
                        : [...data, type],
                    )
                  }
                >
                  {type}
                </button>
              ))}
            </div>
          </DialogHeader>
          <div className="overflow-x-scroll overflow-y-scroll flex-1">
            <table className="block w-full min-width-full max-w-full">
              <LayoutGroup>
                <motion.tbody className="font-mono w-full table overflow-y-scroll overflow-x-scroll h-full">
                  <AnimatePresence>
                    {messages
                      .filter((m) => !filtered.includes(m.source))
                      .map((msg, index) => (
                        <motion.tr
                          layout
                          key={msg.timestamp}
                          variants={{
                            hidden: { y: -28, opacity: 0 },
                            show: {
                              y: 0,
                              opacity: 1,
                            },
                          }}
                          initial="hidden"
                          animate="show"
                          className="overflow-y-hidden border-t"
                        >
                          <td className="px-1 py-1 whitespace-nowrap sticky text-sm text-gray-500">
                            {msg.timestamp}
                          </td>
                          <td
                            className={cn(
                              "px-1 py-1 whitespace-nowrap text-sm sticky left-0 bg-background",
                              TRACE_COLOR[msg.level] ?? "text-gray-500",
                            )}
                          >
                            {msg.level}
                          </td>
                          <td className="px-1 py-1 whitespace-nowrap text-sm text-gray-500">
                            {msg.source}
                          </td>
                          <td className="px-1 py-1 w-full whitespace-nowrap text-sm">
                            {msg.message}
                          </td>
                        </motion.tr>
                      ))}
                  </AnimatePresence>
                </motion.tbody>
              </LayoutGroup>
            </table>
          </div>
        </DialogContent>
      </Dialog>
      <FlowEditor
        initialNodes={server.data.nodes}
        initialEdges={server.data.edges}
      />
    </div>
  );
};
