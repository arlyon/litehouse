"use client";

import {
  InputOTP,
  InputOTPGroup,
  InputOTPSeparator,
  InputOTPSlot,
} from "@/components/ui/input-otp";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
  DialogTrigger,
  DialogHeader,
  DialogClose,
} from "@/components/ui/dialog";

import { AnimatePresence, motion } from "framer-motion";

import { Button } from "./ui/button";
import { Loader } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { cn } from "@/lib/utils";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

const messages = [
  "Querying Relay...",
  "Brokering WebRTC...",
  "Validating the IP...",
];

export const FindServer = ({ className }: { className?: string }) => {
  const [message, setMessage] = useState(0);
  const [dcOpen, setDcOpen] = useState(false);
  const [messages, setMessages] = useState([]);
  const [iceConnectionState, setIceConnectionState] =
    useState<RTCIceConnectionState>("new");

  const { data } = useQuery({
    queryKey: ["unknownServers"],
    queryFn: async () => {
      const data = await client["/client"].get({
        headers: {
          authorization: "Bearer 1234",
        }
      });
      const servers = await data.json();
      return servers;
    },
    select: (data) => data.filter((s) => s.type === "unknown"),
    refetchInterval: 5000,
  });

  const { mutate } = useMutation({
    mutationFn: async (data) => {
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

      const offer = await pc.createOffer();
      pc.setLocalDescription(offer);

      const res = await client["/client"].post({
        json: offer as {[key: string]: any},
        headers: {
          authorization: `Basic ${btoa(`${data.seed}:${data.password}`)}`,
        },
      });

      if (res.status === 200) {
        const offerResponse = await res.json();
        pc.setRemoteDescription(offerResponse);
      } else  {
        throw new Error(await res.text());
      }
    },
    onSuccess: (data) => {
      console.log("success", data);
    },
    onError: (err) => {
      console.log("error", err);
    },
  });

  useEffect(() => {
    // every 3 seconds, switch the message
    const interval = setInterval(() => {
      setMessage((m) => (m + 1) % messages.length);
    }, 3000);

    return () => clearInterval(interval);
  }, []);

  const [seed, setSeed] = useState(Number(new Date()));
  const [password, setPassword] = useState("");

  return (
    <Dialog open={data?.length > 0 ? undefined : false}>
      <DialogTrigger disabled={!(data?.length > 0)}>
        <button
          type="button"
          key="server-button"
          className={cn(
            "transition-all border text-green-400 bg-green-950 w-[225px] rounded-full justify-center  px-3 py-2 border-green-500 flex flex-row gap-2 relative overflow-hidden",
            className,
            data?.length > 0
              ? "border-green-300 bg-green-800 text-green-300 hover:-translate-y-1"
              : "",
          )}
        >
          {data?.length > 0 ? null : <Loader className="animate-spin" />}
          <AnimatePresence mode="popLayout">
            <motion.div
              className="flex-1 text-center"
              key={data?.length > 0 ? "text-done" : `text-${message}`}
              initial={{ y: -30, opacity: 0 }}
              animate={{ y: 0, opacity: 1 }}
              exit={{ y: 30, opacity: 0 }}
              transition={{ duration: 0.5 }}
            >
              {data?.length > 0 ? "Begin Pairing" : messages[message]}
            </motion.div>
          </AnimatePresence>
        </button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Server Found</DialogTitle>
          <DialogDescription>
            <div className="flex flex-col gap-2 w-full items-center justify-center py-8">
              Give it a name!
              <NamePicker seed={seed} onRefresh={() => setSeed(Number(new Date()))} />
            </div>
            We have found a server near you with ip{" "}
            <pre className="inline">{data?.[0]?.ip}</pre>. To prove you are the
            owner, please enter the 6 digit code you set up below.
            <div className="flex w-full items-center justify-center py-8">
              <InputOTP maxLength={6} value={password} onChange={(value) => setPassword(value)}>
                <InputOTPGroup>
                  <InputOTPSlot index={0} />
                  <InputOTPSlot index={1} />
                  <InputOTPSlot index={2} />
                </InputOTPGroup>
                <InputOTPSeparator />
                <InputOTPGroup>
                  <InputOTPSlot index={3} />
                  <InputOTPSlot index={4} />
                  <InputOTPSlot index={5} />
                </InputOTPGroup>
              </InputOTP>
            </div>
          </DialogDescription>
          <div className="flex w-full justify-end gap-4">
            <DialogClose>
              <Button variant="secondary" type="reset">
                Cancel
              </Button>
            </DialogClose>
            <Button variant="primary" disabled={password.length !== 6} onClick={() => mutate({
              seed,
              password
            })}>
              Submit
            </Button>
          </div>
        </DialogHeader>
      </DialogContent>
    </Dialog>
  );
};

import { ReloadIcon } from "@radix-ui/react-icons";

class SplitMix32 {
  private state: number;

  constructor(seed: number) {
    this.state = seed;
    // ffwd 5 iterations
    for (let i = 0; i < 5; i++) {
      this.next();
    }
  }

  next() {
    this.state += 0x9e3779b5;
    let z = this.state;
    z = (z ^ (z >>> 16)) >>> 0;
    z = Math.imul(z, 0x85ebca6b) >>> 0;
    z = (z ^ (z >>> 13)) >>> 0;
    z = Math.imul(z, 0xc2b2ae35) >>> 0;
    return (z ^ (z >>> 16)) >>> 0;
  }
}

import { adjectives, nouns } from "human-id";
import { client } from "@/lib/cockpit-client";

const NamePicker = ({seed, onRefresh}) => {
  const id = useMemo(() => {
    const split = new SplitMix32(seed);
    const idx1 = split.next() % adjectives.length;
    const idx2 = split.next() % nouns.length;

    return `${adjectives[idx1]}-${nouns[idx2]}`;
  }, [seed]);

  return (
    <div className="w-80 font-mono text-lg border text-center flex gap-2 flex-row items-center justify-center relative h-[40px]">
      <span className="text-green-500 flex-1">{id}</span>
      <Button
        variant="ghost"
        className="text-sm rounded-none absolute right-0"
        onClick={() => onRefresh()}
      >
        <ReloadIcon />
      </Button>
    </div>
  );
};
