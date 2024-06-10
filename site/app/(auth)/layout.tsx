import { Button } from "@/components/ui/button";
import { SignOutButton, UserProfile } from "@clerk/nextjs";
import { Header } from "../registry/layout";

export default function Page({ children }) {
  return (
    <>
      <Header title="Litehouse" />
      <div className="flex flex-col w-full my-20 justify-center items-center gap-10">
        {children}
      </div>
    </>
  );
}
