"use client";

import { useSession, signOut } from "@/lib/auth-client";
import { Button } from "./ui/button";
import { CircleUserRound } from "lucide-react";
import Link from "next/link";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

export const LoginButton = () => {
  const { data: session, isPending } = useSession();

  if (isPending) {
    return (
      <div className="animate-pulse bg-muted rounded-full size-[28px]" />
    );
  }

  if (session?.user) {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            className="p-0 rounded-full size-[28px] text-muted-foreground bg-muted"
          >
            <CircleUserRound />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuLabel>
            {session.user.email}
          </DropdownMenuLabel>
          <DropdownMenuSeparator />
          <DropdownMenuItem asChild>
            <Link href="/profile">Profile</Link>
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            onClick={async () => {
              await signOut({
                fetchOptions: {
                  onSuccess: () => {
                    window.location.href = "/registry";
                  },
                },
              });
            }}
          >
            Sign Out
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    );
  }

  return (
    <Link href="/sign-in">
      <Button
        variant="ghost"
        className="p-0 rounded-full size-[28px] text-muted-foreground bg-muted"
      >
        <CircleUserRound />
      </Button>
    </Link>
  );
};
