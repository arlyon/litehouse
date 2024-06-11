import { type Plugin, PluginPage } from "@/components/plugin-page";

const PLUGINS = {
  tasmota: {
    versions: [
      { version: "0.1.2", date: new Date("2024-05-13") },
      { version: "0.1.1", date: new Date("2024-05-09") },
      { version: "0.1.0", date: new Date("2024-05-04") },
    ],
    configSchema:
      '{"properties":{"config":{"properties":{"ip":{"description":"The ip address of the device to connect to.","items":[{"format":"uint8","minimum":0,"type":"integer"},{"format":"uint8","minimum":0,"type":"integer"},{"format":"uint8","minimum":0,"type":"integer"},{"format":"uint8","minimum":0,"type":"integer"}],"maxItems":4,"minItems":4,"type":"array"}},"required":["ip"],"type":"object"},"plugin":{"const":"tasmota@0.1.1"}},"required":["plugin","config"],"type":"object"}',
    source: "https://github.com/arlyon/litehouse",
    capabilities: ["http-client"],
    homepage: "https://github.com/arlyon/litehouse",
    description: "Control tasmota-based smart devices.",
    size: 60345,
  },
  weather: {
    versions: [
      { version: "0.1.1", date: new Date("2024-05-09") },
      { version: "0.1.0", date: new Date("2024-05-04") },
    ],
    configSchema:
      '{"properties":{"config":{"properties":{"lat":{"description":"The latitude to fetch the weather for.","format":"double","type":"number"},"lon":{"description":"The longitude to fetch the weather for.","format":"double","type":"number"}},"required":["lat","lon"],"type":"object"},"plugin":{"const":"weather@0.1.1"}},"required":["plugin","config"],"type":"object"}',
    source: "https://github.com/arlyon/litehouse",
    capabilities: ["http-client"],
    homepage: "https://github.com/arlyon/litehouse",
    description:
      "Fetch weather data from the internet using api.open-meteo.com.",
    size: 60345,
  },
};

/**
 * Returns versions in descending order
 *
 * @param title The title of the plugin
 * @param version The version of the plugin. If not provided, the latest version is returned
 */
export const getPluginData = async (
  title: string,
  version?: string,
): Promise<Plugin> => {
  const plugin = PLUGINS[title];
  return {
    version: version || plugin?.versions?.[0]?.version,
    title,
    ...plugin,
  };
};

export const getPlugins = async (): Promise<
  {
    title: string;
    versions: { version: string; date: Date }[];
    description?: string;
    version: { version: string; date: Date };
  }[]
> => {
  return Object.entries(PLUGINS).map(([title, data]) => ({
    title,
    versions: data.versions,
    description: data.description,
    version: data.versions[0],
  }));
};
