"use client";

import { Timer } from "lucide-react";
import { FlowEditor } from "./flow-editor";
import { useEffect, useRef, useState } from "react";
import { client } from "@/lib/cockpit-client";
import { cn } from "@/lib/utils";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTrigger,
} from "./ui/dialog";

export const Cockpit = ({ nodeId }) => {
  const [dcOpen, setDcOpen] = useState(false);
  const [messages, setMessages] = useState([]);
  const [iceConnectionState, setIceConnectionState] =
    useState<RTCIceConnectionState>("new");

  useEffect(() => {
    if (!("window" in globalThis)) {
      return;
    }

    const pc = new RTCPeerConnection({
      iceServers: [
        {
          urls: "stun:stun.l.google.com:19302",
        },
      ],
    });
    const dc = pc.createDataChannel("data");

    dc.onopen = () => {
      setDcOpen(true);
    };

    dc.onmessage = (event) => {
      const message = JSON.parse(event.data);
      setMessages((messages) => [...messages, message]);
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

  return (
    <>
      <Dialog>
        <DialogTrigger>
          <button
            type="button"
            className={cn(
              "size-4 absolute top-4 right-4 z-50 rounded-full border animate-pulse",
              iceConnectionState === "connected"
                ? "bg-green-500 border-green-800"
                : "bg-red-500 border-red-800",
            )}
          />
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <h2 className="text-lg font-semibold">Server Logs</h2>
          </DialogHeader>
          <DialogDescription>
            <div className="h-96 overflow-y-scroll overflow-x-scroll w-full">
              <table className="min-w-full">
                <tbody className="divide-y font-mono">
                  {messages.map((msg, index) => (
                    <tr key={index}>
                      <td className="px-2 py-1 whitespace-nowrap text-sm text-gray-500">
                        {msg.source}
                      </td>
                      <td className="px-2 py-1 whitespace-nowrap text-sm text-purple-500">
                        {msg.level}
                      </td>
                      <td className="px-2 py-1 whitespace-nowrap text-sm text-gray-500">
                        {msg.message}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </DialogDescription>
        </DialogContent>
      </Dialog>
      <FlowEditor
        initialNodes={server.data.nodes}
        initialEdges={server.data.edges}
      />
    </>
  );
};
