import { RegistryPage } from "@/components/registry-page";

export default function HomePage() {
  return (
    <main>
      <RegistryPage
        packages={[
          {
            title: "tasmota",
            description: "A plugin for managing your tasmota-powered devices.",
            downloads: 0,
            version: "0.1.2",
          },
        ]}
        users={0}
        pluginCount={4}
        totalDownloads={0}
      />
    </main>
  );
}
