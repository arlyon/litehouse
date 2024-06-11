import { ClerkProvider } from "@clerk/nextjs";
import { Header } from "../registry/layout";

export default function Page({ children }) {
  return (
    <ClerkProvider>
      <Header title="Registry" />
      <div className="flex flex-col w-full my-20 justify-center items-center gap-10">
        {children}
      </div>
    </ClerkProvider>
  );
}
