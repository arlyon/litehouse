"use client";

import { Callout } from "fumadocs-ui/components/callout";

import {
  RedirectToSignIn,
  SignedIn,
  SignedOut,
  UserProfile,
} from "@clerk/nextjs";
import { Plug } from "lucide-react";

export default function Page() {
  return (
    <>
      <SignedOut>
        <RedirectToSignIn />
      </SignedOut>
      <SignedIn>
        <UserProfile>
          <UserProfile.Page
            label="My Plugins"
            labelIcon={<Plug className="h-4 w-4" />}
            url="/plugins"
          >
            <h1 className="font-bold flex">
              My Plugins
              <span className="font-mono text-muted-foreground font-normal text-sm ml-auto">
                0 uploaded
              </span>
            </h1>
            <hr className="my-4" />
            <Callout title="Coming soon...">
              We are working on the final touches before opening up the registry
              to the world. Hold on tight!
            </Callout>
          </UserProfile.Page>
        </UserProfile>
      </SignedIn>
    </>
  );
}
