import { createAuthClient } from "better-auth/react";
import { magicLinkClient, deviceAuthorizationClient } from "better-auth/client/plugins";
import { passkeyClient } from "@better-auth/passkey/client";

export const authClient = createAuthClient({
  baseURL: process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3000",
  plugins: [
    magicLinkClient(),
    deviceAuthorizationClient(),
    passkeyClient(),
  ],
});

export const {
  signIn,
  signUp,
  signOut,
  useSession,
  getSession,
} = authClient;
