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
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import { useQuery, useQueryClient } from "@tanstack/react-query";

const messages = [
  "Querying Relay...",
  "Brokering WebRTC...",
  "Validating the IP...",
];

export const FindServer = ({ className }: { className?: string }) => {
  const [message, setMessage] = useState(0);

  const client = useQueryClient();
  const { data } = useQuery({
    queryKey: ["unknownServers"],
    queryFn: async () => {
      const data = await fetch("http://localhost:3001/client", {
        headers: {
          Authorization: "Bearer 1234",
        },
      });
      const servers = await data.json();
      return servers;
    },
    select: (data) => data.filter((s) => s.type === "unknown"),
    refetchInterval: 5000,
  });

  useEffect(() => {
    // every 3 seconds, switch the message
    const interval = setInterval(() => {
      setMessage((m) => (m + 1) % messages.length);
    }, 3000);

    return () => clearInterval(interval);
  }, []);

  return (
    <Dialog open={data?.length > 0}>
      <DialogTrigger>
        <div
          className={cn(
            "border text-green-400 bg-green-950 w-[225px] rounded-full px-3 py-2 border-green-500 flex flex-row gap-2 relative overflow-hidden",
            className,
          )}
        >
          <Loader className="animate-spin" />
          <AnimatePresence mode="popLayout">
            <motion.div
              className="flex-1 text-center"
              key={"text-" + message}
              initial={{ y: -30, opacity: 0 }}
              animate={{ y: 0, opacity: 1 }}
              exit={{ y: 30, opacity: 0 }}
              transition={{ duration: 0.5 }}
            >
              {messages[message]}
            </motion.div>
          </AnimatePresence>
        </div>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Server Found</DialogTitle>
          <DialogDescription>
            We have found a server near you with ip{" "}
            <pre className="inline">{data?.[0]?.ip}</pre>. To prove you are the
            owner, please enter the 6 digit code you set up below.
            <div className="flex w-full items-center justify-center py-8">
              <InputOTP maxLength={6}>
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
            <Button variant="primary">Submit</Button>
          </div>
        </DialogHeader>
      </DialogContent>
    </Dialog>
  );
};
