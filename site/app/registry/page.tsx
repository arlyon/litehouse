import { RegistryPage } from "@/components/registry-page";
import { getPlugins } from "@/lib/registry";

export default async function HomePage() {
  const packages = await getPlugins();

  return (
    <main>
      <RegistryPage
        packages={packages}
        users={0}
        pluginCount={packages.length}
        totalDownloads={0}
      />
    </main>
  );
}
