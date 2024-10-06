"use client";

import { Timer } from "lucide-react";
import { FlowEditor } from "./flow-editor";
import { useEffect, useRef, useState } from "react";

export const Cockpit = ({ children, nodeId }) => {
  const [dcOpen, setDcOpen] = useState(false);
  const [messages, setMessages] = useState([]);
  const [iceConnectionState, setIceConnectionState] =
    useState<RTCIceConnectionState>("new");

  useEffect(() => {
    console.log("OMFG");

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
      setMessages((messages) => [...messages, event.data]);
    };

    pc.oniceconnectionstatechange = () => {
      const iceConnectionState = pc.iceConnectionState;
      setIceConnectionState(iceConnectionState);
    };

    (async () => {
      console.log("creating offer");

      let res;
      try {
        const offer = await pc.createOffer();
        pc.setLocalDescription(offer);
        res = await fetch(`http://localhost:3001/client/${nodeId}`, {
          method: "post",
          headers: {
            Accept: "application/json, text/plain, */*",
            "Content-Type": "application/json",
            Authorization: "Bearer 1234",
          },
          body: JSON.stringify(offer),
        });
      } catch (e) {
        console.log("ERROR");
        console.log(e);
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
          // className: "stroke-green-500",
        },
      ],
    },
  };

  return (
    <FlowEditor
      initialNodes={server.data.nodes}
      initialEdges={server.data.edges}
    />
  );
};
