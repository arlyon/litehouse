"use client";

import { Callout } from "fumadocs-ui/components/callout";
import { useSession, signOut } from "@/lib/auth-client";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Plug } from "lucide-react";

export default function Page() {
  const { data: session, isPending } = useSession();
  const router = useRouter();

  useEffect(() => {
    if (!isPending && !session) {
      router.push("/sign-in");
    }
  }, [session, isPending, router]);

  if (isPending) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="animate-pulse">Loading...</div>
      </div>
    );
  }

  if (!session) {
    return null;
  }

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <div className="space-y-4">
        <h1 className="text-3xl font-bold">Profile</h1>

        <div className="border rounded-lg p-6 space-y-4">
          <div>
            <h2 className="text-sm font-medium text-muted-foreground">Email</h2>
            <p className="text-lg">{session.user.email}</p>
          </div>

          {session.user.name && (
            <div>
              <h2 className="text-sm font-medium text-muted-foreground">Name</h2>
              <p className="text-lg">{session.user.name}</p>
            </div>
          )}

          <div className="pt-4">
            <Button
              variant="destructive"
              onClick={async () => {
                await signOut({
                  fetchOptions: {
                    onSuccess: () => {
                      router.push("/sign-in");
                    },
                  },
                });
              }}
            >
              Sign Out
            </Button>
          </div>
        </div>

        <div className="border rounded-lg p-6">
          <h1 className="font-bold flex items-center gap-2">
            <Plug className="h-4 w-4" />
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
        </div>
      </div>
    </div>
  );
}
