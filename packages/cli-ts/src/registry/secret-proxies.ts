import type { CommandDef } from "./types.js";

export const secretProxyCommands: CommandDef[] = [
  {
    name: "secret-proxies",
    description: "List secret proxies for the workspace",
    route: { method: "GET", path: "/v1/workspaces/{wid}/secret-proxies" },
    display: {
      title: "Secret Proxies",
      cols: ["name>Name", "url>URL", "description>Description", "secret_count>Secrets", "@created_at>Created"],
    },
  },
  {
    name: "secret-proxies create",
    description: "Create a secret proxy",
    route: { method: "POST", path: "/v1/workspaces/{wid}/secret-proxies" },
    options: [
      { flags: "--name <name>", description: "Proxy name", required: true },
      { flags: "--url <url>", description: "Proxy URL (or 'self' for local encrypted secrets)", required: true },
      { flags: "--description <desc>", description: "Description" },
    ],
    successTemplate: "Secret proxy {name} created",
  },
  {
    name: "secret-proxies show",
    description: "Show a secret proxy",
    route: { method: "GET", path: "/v1/workspaces/{wid}/secret-proxies/{pos}" },
    args: [{ name: "proxy-name", required: true, description: "Proxy name" }],
    display: {
      title: "Secret Proxy",
      cols: ["name>Name", "url>URL", "description>Description", "secret_count>Secrets", "@created_at>Created"],
    },
  },
  {
    name: "secret-proxies secrets",
    description: "List secret names in a proxy",
    route: { method: "GET", path: "/v1/workspaces/{wid}/secret-proxies/{pos}/secrets" },
    args: [{ name: "proxy-name", required: true, description: "Proxy name" }],
    display: {
      title: "Secrets",
      cols: ["names>Secret Names", "proxy_name>Proxy"],
    },
  },
  {
    name: "secret-proxies set-secrets",
    description: "Set secrets in a proxy (key-value pairs, server encrypts)",
    route: { method: "PUT", path: "/v1/workspaces/{wid}/secret-proxies/{pos}/secrets" },
    args: [{ name: "proxy-name", required: true, description: "Proxy name" }],
    options: [
      { flags: "--secrets <json>", description: "JSON object of key-value secret pairs", required: true },
    ],
    successTemplate: "Secrets updated for {proxy_name}",
  },
];
