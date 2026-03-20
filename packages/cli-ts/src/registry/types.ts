import type { CorpAPIClient } from "../api-client.js";
import type { ReferenceResolver } from "../references.js";

export interface CommandDef {
  name: string;
  description: string;
  aliases?: string[];
  route?: { method: "GET" | "POST" | "PUT" | "DELETE"; path: string };
  entity?: boolean | "query";
  args?: ArgDef[];
  options?: OptionDef[];
  optQP?: string[];
  display?: { title: string; cols?: string[]; listKey?: string };
  handler?: (ctx: CommandContext) => Promise<void>;
  local?: boolean;
  hidden?: boolean;
  dryRun?: boolean;
  passThroughOptions?: boolean;
  examples?: string[];
}

export interface ArgDef {
  name: string;
  required?: boolean;
  description?: string;
  variadic?: boolean;
  choices?: string[];
}

export interface OptionDef {
  flags: string;
  description: string;
  required?: boolean;
  choices?: string[];
  default?: unknown;
  type?: "string" | "int" | "float" | "array";
}

export interface CommandContext {
  client: CorpAPIClient;
  positional: string[];
  opts: Record<string, unknown>;
  entityId?: string;
  resolver: ReferenceResolver;
  writer: OutputWriter;
  quiet: boolean;
  dryRun: boolean;
}

export interface OutputWriter {
  writeln(text?: string): void;
  json(data: unknown): void;
  table(title: string, columns: string[], rows: unknown[][]): void;
  panel(title: string, color: string, lines: string[]): void;
  error(msg: string): void;
  success(msg: string): void;
  warning(msg: string): void;
  writeResult(result: Record<string, unknown>, message: string, options?: Record<string, unknown>): void;
  quietId(id: string): void;
  dryRun(operation: string, payload: unknown): void;
}

// Shape emitted in web-routes.json
export interface WebRouteEntry {
  method?: string;
  path?: string;
  entity?: boolean | "query";
  title?: string;
  cols?: string[];
  listKey?: string;
  optQP?: string[];
  write?: boolean;
  local?: boolean;
  custom?: boolean;
}
