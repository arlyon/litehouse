import { ClerkProvider, RedirectToSignIn, SignedOut } from "@clerk/nextjs";
import { Header } from "../registry/layout";

export default function CockpitPage({ children }) {
  return (
    <ClerkProvider>
      <Header title="Cockpit" suspend={false} />
      <SignedOut>
        <RedirectToSignIn />
      </SignedOut>
      {children}
    </ClerkProvider>
  );
}
