import { readFileSync, writeFileSync, mkdirSync, chmodSync, existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import type { CorpConfig } from "./types.js";

const CONFIG_DIR = process.env.CORP_CONFIG_DIR || join(homedir(), ".corp");
const CONFIG_FILE = join(CONFIG_DIR, "config.json");

const DEFAULTS: CorpConfig = {
  api_url: process.env.CORP_API_URL || "https://api.thecorporation.ai",
  api_key: "",
  workspace_id: "",
  hosting_mode: "",
  llm: {
    provider: "anthropic",
    api_key: "",
    model: "claude-sonnet-4-6",
  },
  user: { name: "", email: "" },
  active_entity_id: "",
};

function deepMerge(base: Record<string, unknown>, override: Record<string, unknown>): void {
  for (const [key, value] of Object.entries(override)) {
    if (
      key in base &&
      typeof base[key] === "object" &&
      base[key] !== null &&
      !Array.isArray(base[key]) &&
      typeof value === "object" &&
      value !== null &&
      !Array.isArray(value)
    ) {
      deepMerge(base[key] as Record<string, unknown>, value as Record<string, unknown>);
    } else {
      base[key] = value;
    }
  }
}

export function loadConfig(): CorpConfig {
  const cfg = structuredClone(DEFAULTS);
  if (existsSync(CONFIG_FILE)) {
    const saved = JSON.parse(readFileSync(CONFIG_FILE, "utf-8"));
    deepMerge(cfg as unknown as Record<string, unknown>, saved);
  }
  return cfg;
}

export function saveConfig(cfg: CorpConfig): void {
  mkdirSync(CONFIG_DIR, { recursive: true, mode: 0o700 });
  writeFileSync(CONFIG_FILE, JSON.stringify(cfg, null, 2) + "\n", { mode: 0o600 });
}

export function getValue(cfg: Record<string, unknown>, dotPath: string): unknown {
  const keys = dotPath.split(".");
  let current: unknown = cfg;
  for (const key of keys) {
    if (typeof current === "object" && current !== null && key in current) {
      current = (current as Record<string, unknown>)[key];
    } else {
      return undefined;
    }
  }
  return current;
}

export function setValue(cfg: Record<string, unknown>, dotPath: string, value: string): void {
  const keys = dotPath.split(".");
  let current = cfg;
  for (const key of keys.slice(0, -1)) {
    if (!(key in current) || typeof current[key] !== "object" || current[key] === null) {
      current[key] = {};
    }
    current = current[key] as Record<string, unknown>;
  }
  current[keys[keys.length - 1]] = value;
}

export function requireConfig(...fields: string[]): CorpConfig {
  const cfg = loadConfig();
  const missing = fields.filter((f) => !getValue(cfg as unknown as Record<string, unknown>, f));
  if (missing.length > 0) {
    console.error(`Missing config: ${missing.join(", ")}`);
    console.error("Run 'corp setup' to configure.");
    process.exit(1);
  }
  return cfg;
}

export function maskKey(value: string): string {
  if (!value || value.length < 8) return "***";
  return "***" + value.slice(-4);
}

export function configForDisplay(cfg: CorpConfig): Record<string, unknown> {
  const display = { ...cfg } as Record<string, unknown>;
  if (display.api_key) display.api_key = maskKey(display.api_key as string);
  if (typeof display.llm === "object" && display.llm !== null) {
    const llm = { ...(display.llm as Record<string, unknown>) };
    if (llm.api_key) llm.api_key = maskKey(llm.api_key as string);
    display.llm = llm;
  }
  return display;
}

export function resolveEntityId(cfg: CorpConfig, explicitId?: string): string {
  const eid = explicitId || cfg.active_entity_id;
  if (!eid) {
    console.error(
      "No entity specified. Use --entity-id or set active_entity_id via 'corp config set active_entity_id <id>'."
    );
    process.exit(1);
  }
  return eid;
}
