import {
  chmodSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import type { CorpConfig } from "./types.js";

const CONFIG_DIR = process.env.CORP_CONFIG_DIR || join(homedir(), ".corp");
const CONFIG_FILE = join(CONFIG_DIR, "config.json");
const AUTH_FILE = join(CONFIG_DIR, "auth.json");
const CONFIG_LOCK_DIR = join(CONFIG_DIR, "config.lock");
const CONFIG_LOCK_TIMEOUT_MS = 5000;
const CONFIG_LOCK_RETRY_MS = 25;
const CONFIG_STALE_LOCK_MS = 60_000;
const MAX_LAST_REFERENCES = 128;
const TRUSTED_API_HOST_SUFFIXES = ["thecorporation.ai"];

const CONFIG_WAIT_BUFFER = new SharedArrayBuffer(4);
const CONFIG_WAIT_SIGNAL = new Int32Array(CONFIG_WAIT_BUFFER);

const ALLOWED_CONFIG_KEYS = new Set([
  "api_url",
  "api_key",
  "workspace_id",
  "hosting_mode",
  "llm.provider",
  "llm.api_key",
  "llm.model",
  "llm.base_url",
  "user.name",
  "user.email",
  "active_entity_id",
]);

const SENSITIVE_CONFIG_KEYS = new Set(["api_url", "api_key", "workspace_id"]);

type CorpAuthConfig = {
  api_url?: string;
  api_key?: string;
  workspace_id?: string;
  llm?: {
    api_key?: string;
  };
};

const DEFAULTS: CorpConfig = {
  api_url: process.env.CORP_API_URL || "https://api.thecorporation.ai",
  api_key: process.env.CORP_API_KEY || "",
  workspace_id: process.env.CORP_WORKSPACE_ID || "",
  hosting_mode: "",
  llm: {
    provider: "anthropic",
    api_key: process.env.CORP_LLM_API_KEY || "",
    model: "claude-sonnet-4-6",
    base_url: process.env.CORP_LLM_BASE_URL || undefined,
  },
  user: { name: "", email: "" },
  active_entity_id: "",
};

function sleepSync(ms: number): void {
  Atomics.wait(CONFIG_WAIT_SIGNAL, 0, 0, ms);
}

function withConfigLock<T>(fn: () => T): T {
  mkdirSync(CONFIG_DIR, { recursive: true, mode: 0o700 });
  const startedAt = Date.now();
  while (true) {
    try {
      mkdirSync(CONFIG_LOCK_DIR, { mode: 0o700 });
      break;
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== "EEXIST") {
        throw err;
      }
      try {
        const ageMs = Date.now() - statSync(CONFIG_LOCK_DIR).mtimeMs;
        if (ageMs >= CONFIG_STALE_LOCK_MS) {
          rmSync(CONFIG_LOCK_DIR, { recursive: true, force: true });
          continue;
        }
      } catch {
        // Ignore lock-stat failures and continue waiting.
      }
      if (Date.now() - startedAt >= CONFIG_LOCK_TIMEOUT_MS) {
        throw new Error("timed out waiting for the corp config lock");
      }
      sleepSync(CONFIG_LOCK_RETRY_MS);
    }
  }

  try {
    return fn();
  } finally {
    rmSync(CONFIG_LOCK_DIR, { recursive: true, force: true });
  }
}

function ensureSecurePermissions(): void {
  mkdirSync(CONFIG_DIR, { recursive: true, mode: 0o700 });
  try {
    chmodSync(CONFIG_DIR, 0o700);
  } catch {
    // Ignore chmod failures on filesystems without POSIX permission support.
  }
  if (existsSync(CONFIG_FILE)) {
    try {
      chmodSync(CONFIG_FILE, 0o600);
    } catch {
      // Ignore chmod failures on filesystems without POSIX permission support.
    }
  }
  if (existsSync(AUTH_FILE)) {
    try {
      chmodSync(AUTH_FILE, 0o600);
    } catch {
      // Ignore chmod failures on filesystems without POSIX permission support.
    }
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isLoopbackHost(hostname: string): boolean {
  return hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1";
}

function isTrustedCorpHost(hostname: string): boolean {
  return TRUSTED_API_HOST_SUFFIXES.some(
    (suffix) => hostname === suffix || hostname.endsWith(`.${suffix}`),
  );
}

function allowUnsafeApiUrl(): boolean {
  return process.env.CORP_UNSAFE_API_URL === "1";
}

export function validateApiUrl(value: string): string {
  let parsed: URL;
  try {
    parsed = new URL(value.trim());
  } catch {
    throw new Error("api_url must be a valid absolute URL");
  }

  if (parsed.username || parsed.password) {
    throw new Error("api_url must not include embedded credentials");
  }

  const protocol = parsed.protocol.toLowerCase();
  const hostname = parsed.hostname.toLowerCase();
  if (protocol !== "https:" && !(protocol === "http:" && isLoopbackHost(hostname))) {
    throw new Error("api_url must use https, or http only for localhost/loopback development");
  }
  if (protocol === "https:" && !isLoopbackHost(hostname) && !isTrustedCorpHost(hostname) && !allowUnsafeApiUrl()) {
    throw new Error(
      "api_url must point to a trusted TheCorporation host or localhost; set CORP_UNSAFE_API_URL=1 to allow a custom self-hosted URL",
    );
  }

  parsed.hash = "";
  return parsed.toString().replace(/\/+$/, "");
}

export function validateLlmBaseUrl(value: string): string {
  let parsed: URL;
  try {
    parsed = new URL(value.trim());
  } catch {
    throw new Error("llm.base_url must be a valid absolute URL");
  }

  if (parsed.username || parsed.password) {
    throw new Error("llm.base_url must not include embedded credentials");
  }

  const protocol = parsed.protocol.toLowerCase();
  const hostname = parsed.hostname.toLowerCase();
  if (protocol !== "https:" && !(protocol === "http:" && isLoopbackHost(hostname))) {
    throw new Error("llm.base_url must use https, or http only for localhost/loopback development");
  }

  parsed.hash = "";
  return parsed.toString().replace(/\/+$/, "");
}

function normalizeString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function normalizeActiveEntityMap(value: unknown): Record<string, string> | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const entries = Object.entries(value).filter(
    ([workspaceId, entityId]) =>
      typeof workspaceId === "string" && typeof entityId === "string" && entityId.length > 0,
  );
  if (entries.length === 0) {
    return undefined;
  }
  return Object.fromEntries(entries);
}

function trimReferenceEntries(
  entries: Array<[string, string]>,
): Array<[string, string]> {
  if (entries.length <= MAX_LAST_REFERENCES) {
    return entries;
  }
  return entries.slice(entries.length - MAX_LAST_REFERENCES);
}

function normalizeReferenceMap(value: unknown): Record<string, string> | undefined {
  if (!isObject(value)) {
    return undefined;
  }
  const entries = Object.entries(value).filter(
    ([key, ref]) => typeof key === "string" && typeof ref === "string" && ref.trim().length > 0,
  );
  if (entries.length === 0) {
    return undefined;
  }
  return Object.fromEntries(
    trimReferenceEntries(entries.map(([key, ref]) => [key, ref.trim()])),
  );
}

function mergeConfigAndAuth(
  configRaw: unknown,
  authRaw: unknown,
): Record<string, unknown> {
  const merged: Record<string, unknown> = isObject(configRaw) ? { ...configRaw } : {};
  if (!isObject(authRaw)) {
    return merged;
  }
  for (const key of ["api_url", "api_key", "workspace_id"]) {
    const value = authRaw[key];
    if (typeof value === "string") {
      merged[key] = value;
    }
  }
  if (isObject(authRaw.llm)) {
    const llm = isObject(merged.llm) ? { ...merged.llm } : {};
    if (typeof authRaw.llm.api_key === "string") {
      llm.api_key = authRaw.llm.api_key;
    }
    merged.llm = llm;
  }
  return merged;
}

function readJsonFile(path: string): unknown {
  if (!existsSync(path)) {
    return {};
  }
  return JSON.parse(readFileSync(path, "utf-8")) as unknown;
}

function hasLegacySensitiveConfig(raw: unknown): boolean {
  if (!isObject(raw)) {
    return false;
  }
  if (typeof raw.api_url === "string" || typeof raw.api_key === "string" || typeof raw.workspace_id === "string") {
    return true;
  }
  return isObject(raw.llm) && typeof raw.llm.api_key === "string";
}

function normalizeConfig(raw: unknown): CorpConfig {
  const cfg = structuredClone(DEFAULTS) as CorpConfig;
  if (!isObject(raw)) {
    return cfg;
  }

  const savedApiUrl = normalizeString(raw.api_url);
  if (savedApiUrl) {
    try {
      cfg.api_url = validateApiUrl(savedApiUrl);
    } catch {
      cfg.api_url = DEFAULTS.api_url;
    }
  }
  cfg.api_key = normalizeString(raw.api_key) ?? cfg.api_key;
  cfg.workspace_id = normalizeString(raw.workspace_id) ?? cfg.workspace_id;
  cfg.hosting_mode = normalizeString(raw.hosting_mode) ?? cfg.hosting_mode;
  cfg.active_entity_id = normalizeString(raw.active_entity_id) ?? cfg.active_entity_id;

  if (isObject(raw.llm)) {
    cfg.llm.provider = normalizeString(raw.llm.provider) ?? cfg.llm.provider;
    cfg.llm.api_key = normalizeString(raw.llm.api_key) ?? cfg.llm.api_key;
    cfg.llm.model = normalizeString(raw.llm.model) ?? cfg.llm.model;
    const baseUrl = normalizeString(raw.llm.base_url);
    if (baseUrl && baseUrl.trim()) {
      try {
        cfg.llm.base_url = validateLlmBaseUrl(baseUrl);
      } catch {
        cfg.llm.base_url = undefined;
      }
    }
  }

  if (isObject(raw.user)) {
    cfg.user.name = normalizeString(raw.user.name) ?? cfg.user.name;
    cfg.user.email = normalizeString(raw.user.email) ?? cfg.user.email;
  }

  const activeEntityIds = normalizeActiveEntityMap(raw.active_entity_ids);
  if (activeEntityIds) {
    cfg.active_entity_ids = activeEntityIds;
  }
  const lastReferences = normalizeReferenceMap(raw.last_references);
  if (lastReferences) {
    cfg.last_references = lastReferences;
  }
  if (cfg.workspace_id && cfg.active_entity_id) {
    cfg.active_entity_ids = {
      ...(cfg.active_entity_ids ?? {}),
      [cfg.workspace_id]: cfg.active_entity_id,
    };
  }

  return cfg;
}

function serializeConfig(cfg: CorpConfig): string {
  const normalized = normalizeConfig(cfg);
  const serialized: Record<string, unknown> = {
    hosting_mode: normalized.hosting_mode,
    llm: {
      provider: normalized.llm.provider,
      model: normalized.llm.model,
      ...(normalized.llm.base_url ? { base_url: normalized.llm.base_url } : {}),
    },
    user: {
      name: normalized.user.name,
      email: normalized.user.email,
    },
    active_entity_id: normalized.active_entity_id,
  };
  if (normalized.active_entity_ids && Object.keys(normalized.active_entity_ids).length > 0) {
    serialized.active_entity_ids = normalized.active_entity_ids;
  }
  if (normalized.last_references && Object.keys(normalized.last_references).length > 0) {
    serialized.last_references = normalized.last_references;
  }
  return JSON.stringify(serialized, null, 2) + "\n";
}

function serializeAuth(cfg: CorpConfig): string {
  const normalized = normalizeConfig(cfg);
  const serialized: CorpAuthConfig = {
    api_url: normalized.api_url,
    api_key: normalized.api_key,
    workspace_id: normalized.workspace_id,
  };
  if (normalized.llm.api_key) {
    serialized.llm = { api_key: normalized.llm.api_key };
  }
  return JSON.stringify(serialized, null, 2) + "\n";
}

function requireSupportedConfigKey(dotPath: string): void {
  if (!ALLOWED_CONFIG_KEYS.has(dotPath)) {
    throw new Error(`unsupported config key: ${dotPath}`);
  }
}

function validateSensitiveConfigUpdate(dotPath: string, forceSensitive = false): void {
  if (SENSITIVE_CONFIG_KEYS.has(dotPath) && !forceSensitive) {
    throw new Error(`refusing to update security-sensitive key '${dotPath}' without --force`);
  }
}

function setKnownConfigValue(cfg: CorpConfig, dotPath: string, value: string): void {
  switch (dotPath) {
    case "api_url":
      cfg.api_url = validateApiUrl(value);
      return;
    case "api_key":
      cfg.api_key = value.trim();
      return;
    case "workspace_id":
      cfg.workspace_id = value.trim();
      return;
    case "hosting_mode":
      cfg.hosting_mode = value.trim();
      return;
    case "llm.provider":
      cfg.llm.provider = value.trim();
      return;
    case "llm.api_key":
      cfg.llm.api_key = value.trim();
      return;
    case "llm.model":
      cfg.llm.model = value.trim();
      return;
    case "llm.base_url":
      cfg.llm.base_url = value.trim() ? validateLlmBaseUrl(value) : undefined;
      return;
    case "user.name":
      cfg.user.name = value.trim();
      return;
    case "user.email":
      cfg.user.email = value.trim();
      return;
    case "active_entity_id":
      setActiveEntityId(cfg, value.trim());
      return;
    default:
      throw new Error(`unsupported config key: ${dotPath}`);
  }
}

function readConfigUnlocked(): CorpConfig {
  ensureSecurePermissions();
  const configRaw = readJsonFile(CONFIG_FILE);
  const authRaw = readJsonFile(AUTH_FILE);
  return normalizeConfig(mergeConfigAndAuth(configRaw, authRaw));
}

function writeConfigUnlocked(cfg: CorpConfig): void {
  ensureSecurePermissions();
  const configTempFile = `${CONFIG_FILE}.${process.pid}.tmp`;
  const authTempFile = `${AUTH_FILE}.${process.pid}.tmp`;
  writeFileSync(configTempFile, serializeConfig(cfg), { mode: 0o600 });
  writeFileSync(authTempFile, serializeAuth(cfg), { mode: 0o600 });
  renameSync(configTempFile, CONFIG_FILE);
  renameSync(authTempFile, AUTH_FILE);
  ensureSecurePermissions();
}

function migrateLegacySensitiveConfigIfNeeded(): void {
  withConfigLock(() => {
    ensureSecurePermissions();
    const configRaw = readJsonFile(CONFIG_FILE);
    if (!hasLegacySensitiveConfig(configRaw)) {
      return;
    }
    const authRaw = readJsonFile(AUTH_FILE);
    const migrated = normalizeConfig(mergeConfigAndAuth(configRaw, authRaw));
    writeConfigUnlocked(migrated);
  });
}

export function loadConfig(): CorpConfig {
  migrateLegacySensitiveConfigIfNeeded();
  return readConfigUnlocked();
}

export function saveConfig(cfg: CorpConfig): void {
  withConfigLock(() => {
    writeConfigUnlocked(cfg);
  });
}

export function updateConfig(mutator: (cfg: CorpConfig) => void): CorpConfig {
  return withConfigLock(() => {
    const cfg = readConfigUnlocked();
    mutator(cfg);
    writeConfigUnlocked(cfg);
    return cfg;
  });
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

export function setValue(
  cfg: Record<string, unknown>,
  dotPath: string,
  value: string,
  options: { forceSensitive?: boolean } = {},
): void {
  requireSupportedConfigKey(dotPath);
  validateSensitiveConfigUpdate(dotPath, options.forceSensitive);
  setKnownConfigValue(cfg as CorpConfig, dotPath, value);
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
  delete display.last_references;
  if (typeof display.llm === "object" && display.llm !== null) {
    const llm = { ...(display.llm as Record<string, unknown>) };
    if (llm.api_key) llm.api_key = maskKey(llm.api_key as string);
    display.llm = llm;
  }
  return display;
}

export function getActiveEntityId(cfg: CorpConfig): string {
  if (cfg.workspace_id && cfg.active_entity_ids?.[cfg.workspace_id]) {
    return cfg.active_entity_ids[cfg.workspace_id];
  }
  return cfg.active_entity_id;
}

export function setActiveEntityId(cfg: CorpConfig, entityId: string): void {
  cfg.active_entity_id = entityId;
  if (!cfg.workspace_id) {
    return;
  }
  cfg.active_entity_ids = {
    ...(cfg.active_entity_ids ?? {}),
    [cfg.workspace_id]: entityId,
  };
}

function referenceScopeKey(workspaceId: string, entityId?: string): string {
  if (workspaceId && entityId) {
    return `workspace:${workspaceId}:entity:${entityId}`;
  }
  if (workspaceId) {
    return `workspace:${workspaceId}`;
  }
  return "global";
}

export function getLastReference(
  cfg: CorpConfig,
  kind: string,
  entityId?: string,
): string | undefined {
  const normalizedKind = kind.trim().toLowerCase();
  const entityScopedKey = `${referenceScopeKey(cfg.workspace_id, entityId)}:${normalizedKind}`;
  if (entityId) {
    return cfg.last_references?.[entityScopedKey];
  }
  const workspaceScopedKey = `${referenceScopeKey(cfg.workspace_id)}:${normalizedKind}`;
  return cfg.last_references?.[workspaceScopedKey];
}

export function setLastReference(
  cfg: CorpConfig,
  kind: string,
  referenceId: string,
  entityId?: string,
): void {
  const normalizedKind = kind.trim().toLowerCase();
  const scopedKey = `${referenceScopeKey(cfg.workspace_id, entityId)}:${normalizedKind}`;
  const nextEntries = Object.entries({
    ...(cfg.last_references ?? {}),
    [scopedKey]: referenceId.trim(),
  });
  cfg.last_references = Object.fromEntries(trimReferenceEntries(nextEntries));
}

export function resolveEntityId(cfg: CorpConfig, explicitId?: string): string {
  const eid = explicitId || getActiveEntityId(cfg);
  if (!eid) {
    console.error(
      "No entity specified. Use --entity-id or set active_entity_id via 'corp config set active_entity_id <id>'."
    );
    process.exit(1);
  }
  return eid;
}
