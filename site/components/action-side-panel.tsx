import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { SheetDescription, SheetHeader, SheetTitle } from "./ui/sheet";

export function ActionSidePanel(props) {
  const history = [
    {
      id: "1",
      timestamp: new Date(),
      event: "Successful sign up",
      icon: "check",
    },
    {
      id: "2",
      timestamp: new Date(),
      event: "Failed login attempt",
      icon: "x",
    },
    {
      id: "3",
      timestamp: new Date(),
      event: "Suspicious activity detected",
      icon: "alert",
    },
    {
      id: "4",
      timestamp: new Date(),
      event: "Successful sign up",
      icon: "check",
    },
    {
      id: "5",
      timestamp: new Date(),
      event: "Failed login attempt",
      icon: "x",
    },
    {
      id: "6",
      timestamp: new Date(),
      event: "Suspicious activity detected",
      icon: "alert",
    },
    {
      id: "7",
      timestamp: new Date(),
      event: "Successful sign up",
      icon: "check",
    },
    {
      id: "8",
      timestamp: new Date(),
      event: "Failed login attempt",
      icon: "x",
    },
    {
      id: "9",
      timestamp: new Date(),
      event: "Suspicious activity detected",
      icon: "alert",
    },
  ];

  return (
    <div className={cn("flex h-full w-full flex-col", props.className)}>
      <div className="flex items-center border-b p-4">
        <h2 className="flex-1 text-lg font-medium">Details</h2>
        <Button size="icon" variant="ghost" onClick={props.onClose}>
          <XIcon className="h-5 w-5" />
          <span className="sr-only">Close</span>
        </Button>
      </div>
      <div className="flex-1 overflow-auto p-4">
        <div className="space-y-6">
          <InfoSection
            title="Instance"
            content="kingswood-home"
            pStyle="font-mono"
          />
          <InfoSection title="Plugin" content="native" pStyle="font-mono" />
          <InfoSection title="Version" content="v2.3.1" pStyle="font-mono" />
          <InfoSection
            title="Description"
            content="This action is triggered at a user-defined interval."
          />
          <InfoSection
            title="Description"
            content={
              <div className="space-y-2">
                {history.map((e) => (
                  <div key={e.id} className="flex items-center gap-2">
                    <div className="h-2 w-2 rounded-full bg-green-500" />
                    <p className="text-muted-foreground text-sm">{e.event}</p>
                    <span className="ml-auto text-xs text-muted-foreground">
                      {e.timestamp.toLocaleString()}
                    </span>
                  </div>
                ))}
              </div>
            }
          />
        </div>
      </div>
    </div>
  );
}

function InfoSection({ title, content, pStyle }) {
  return (
    <div className="flex flex-col gap-1">
      <h3 className="text-sm font-medium">{title}</h3>
      {typeof content === "string" ? (
        <p className={cn("text-muted-foreground", pStyle)}>{content}</p>
      ) : (
        content
      )}
    </div>
  );
}

function XIcon(props) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M18 6 6 18" />
      <path d="m6 6 12 12" />
    </svg>
  );
}
