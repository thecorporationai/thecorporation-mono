/**
 * @thecorporation/corp — TypeScript wrapper for the TheCorporation platform.
 *
 * ## Subpath exports
 *
 * | Import | Contents |
 * |--------|----------|
 * | `@thecorporation/corp` | All types + CorpClient + server helpers + CLI wrapper |
 * | `@thecorporation/corp/client` | CorpClient only (browser-safe, fetch-based) |
 * | `@thecorporation/corp/server` | Binary resolution, startServer(), runCli() |
 * | `@thecorporation/corp/cli` | Typed Corp CLI wrapper (Corp class) |
 *
 * ## Quick start
 *
 * ```ts
 * import { CorpClient } from "@thecorporation/corp";
 *
 * const client = new CorpClient("http://localhost:8000", "corp_...");
 *
 * // Create a Delaware C-Corp
 * const entity = await client.entities.create({
 *   legal_name: "Acme Corp",
 *   entity_type: "c_corp",
 *   jurisdiction: "DE",
 * });
 *
 * // Advance formation
 * await client.formation.advance(entity.entity_id);
 *
 * // Sign documents
 * const docs = await client.formation.listDocuments(entity.entity_id);
 * for (const doc of docs) {
 *   await client.formation.signDocument(doc.document_id, {
 *     signer_name: "Jane Doe",
 *     signer_role: "Incorporator",
 *     signer_email: "jane@acme.com",
 *     signature_text: "/s/ Jane Doe",
 *     consent_text: "I consent",
 *   });
 * }
 * ```
 */

// Types
export type * from "./types.js";

// HTTP client (browser-safe)
export { CorpClient, CorpApiError } from "./client.js";

// Server binary management (Node.js only)
export {
  getServerBinaryPath,
  getCliBinaryPath,
  resetCache,
  startServer,
  startServerAndWait,
  runCli,
  runCliJson,
  type StartServerOptions,
  type CliRunOptions,
  type CliResult,
} from "./server.js";

// Typed CLI wrapper (Node.js only)
export { Corp } from "./cli.js";
