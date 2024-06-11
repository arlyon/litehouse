import { SignInButton, SignedIn, SignedOut, UserButton } from "@clerk/nextjs";
import { Button } from "./ui/button";
import { CircleUserRound } from "lucide-react";

export const LoginButton = () => {
  return (
    <>
      <SignedIn>
        <UserButton
          userProfileMode="navigation"
          userProfileUrl="/profile"
          afterSignOutUrl="/registry"
        />
      </SignedIn>
      <SignedOut>
        <SignInButton
          fallbackRedirectUrl="/registry"
          signUpFallbackRedirectUrl="/registry"
        >
          <Button
            variant="ghost"
            className="p-0 rounded-full size-[28px] text-muted-foreground bg-muted"
          >
            <CircleUserRound />
          </Button>
        </SignInButton>
      </SignedOut>
    </>
  );
};
