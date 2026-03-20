import type { CorpAPIClient } from "../api-client.js";
import type { ReferenceResolver } from "../references.js";

export type ResourceKind =
  | "entity" | "contact" | "body" | "seat" | "meeting" | "agenda_item"
  | "resolution" | "document" | "instrument" | "share_class" | "round"
  | "safe_note" | "valuation" | "share_transfer" | "invoice" | "payment"
  | "bank_account" | "payroll_run" | "distribution" | "reconciliation"
  | "classification" | "tax_filing" | "deadline" | "agent" | "work_item"
  | "service_request" | "api_key" | "digest";

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

  /** What resource this command produces (for @last tracking and result display) */
  produces?: {
    kind: ResourceKind;
    /** Response field containing the ID (default: `${kind}_id`) */
    idField?: string;
    /** Also set the active entity from the response */
    trackEntity?: boolean;
  };

  /** Success message template with field interpolation: "Created {name} ({entity_id})" */
  successTemplate?: string;
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
  produces?: { kind: string; idField?: string; trackEntity?: boolean };
  successTemplate?: string;
}
