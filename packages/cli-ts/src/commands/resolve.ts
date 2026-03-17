import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson } from "../output.js";
import { ReferenceResolver, shortId, type ResourceKind } from "../references.js";

const KINDS = new Set<ResourceKind>([
  "entity",
  "contact",
  "share_transfer",
  "invoice",
  "bank_account",
  "payment",
  "payroll_run",
  "distribution",
  "reconciliation",
  "tax_filing",
  "deadline",
  "classification",
  "body",
  "meeting",
  "seat",
  "agenda_item",
  "resolution",
  "document",
  "work_item",
  "agent",
  "valuation",
  "safe_note",
  "instrument",
  "share_class",
  "round",
]);

const ENTITY_SCOPED_KINDS = new Set<ResourceKind>([
  "contact",
  "share_transfer",
  "invoice",
  "bank_account",
  "payment",
  "payroll_run",
  "distribution",
  "reconciliation",
  "tax_filing",
  "deadline",
  "classification",
  "body",
  "meeting",
  "seat",
  "agenda_item",
  "resolution",
  "document",
  "work_item",
  "valuation",
  "safe_note",
  "instrument",
  "share_class",
  "round",
]);

export async function resolveCommand(
  kind: string,
  ref: string,
  opts: { entityId?: string; bodyId?: string; meetingId?: string },
): Promise<void> {
  const normalizedKind = kind.trim().toLowerCase() as ResourceKind;
  if (!KINDS.has(normalizedKind)) {
    printError(`Unsupported resolve kind: ${kind}`);
    process.exit(1);
  }

  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);

  try {
    const entityId = ENTITY_SCOPED_KINDS.has(normalizedKind) || opts.entityId || opts.bodyId || opts.meetingId
      ? await resolver.resolveEntity(opts.entityId)
      : undefined;
    const bodyId = entityId && opts.bodyId ? await resolver.resolveBody(entityId, opts.bodyId) : undefined;
    const meetingId = entityId && opts.meetingId
      ? await resolver.resolveMeeting(entityId, opts.meetingId, bodyId)
      : undefined;

    let resolvedId: string;
    switch (normalizedKind) {
      case "entity":
        resolvedId = await resolver.resolveEntity(ref);
        break;
      case "contact":
        resolvedId = await resolver.resolveContact(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "share_transfer":
        resolvedId = await resolver.resolveShareTransfer(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "invoice":
        resolvedId = await resolver.resolveInvoice(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "bank_account":
        resolvedId = await resolver.resolveBankAccount(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "payment":
        resolvedId = await resolver.resolvePayment(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "payroll_run":
        resolvedId = await resolver.resolvePayrollRun(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "distribution":
        resolvedId = await resolver.resolveDistribution(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "reconciliation":
        resolvedId = await resolver.resolveReconciliation(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "tax_filing":
        resolvedId = await resolver.resolveTaxFiling(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "deadline":
        resolvedId = await resolver.resolveDeadline(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "classification":
        resolvedId = await resolver.resolveClassification(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "body":
        resolvedId = await resolver.resolveBody(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "meeting":
        resolvedId = await resolver.resolveMeeting(requiredEntity(entityId, normalizedKind), ref, bodyId);
        break;
      case "seat":
        resolvedId = await resolver.resolveSeat(requiredEntity(entityId, normalizedKind), ref, bodyId);
        break;
      case "agenda_item":
        resolvedId = await resolver.resolveAgendaItem(
          requiredEntity(entityId, normalizedKind),
          requiredMeeting(meetingId, normalizedKind),
          ref,
        );
        break;
      case "resolution":
        resolvedId = await resolver.resolveResolution(requiredEntity(entityId, normalizedKind), ref, meetingId);
        break;
      case "document":
        resolvedId = await resolver.resolveDocument(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "work_item":
        resolvedId = await resolver.resolveWorkItem(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "agent":
        resolvedId = await resolver.resolveAgent(ref);
        break;
      case "valuation":
        resolvedId = await resolver.resolveValuation(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "safe_note":
        resolvedId = await resolver.resolveSafeNote(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "instrument":
        resolvedId = await resolver.resolveInstrument(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "share_class":
        resolvedId = await resolver.resolveShareClass(requiredEntity(entityId, normalizedKind), ref);
        break;
      case "round":
        resolvedId = await resolver.resolveRound(requiredEntity(entityId, normalizedKind), ref);
        break;
      default:
        throw new Error(`Unhandled resolve kind: ${normalizedKind}`);
    }

    printJson({
      kind: normalizedKind,
      input: ref,
      resolved_id: resolvedId,
      short_id: shortId(resolvedId),
      ...(entityId ? { entity_id: entityId } : {}),
      ...(bodyId ? { body_id: bodyId } : {}),
      ...(meetingId ? { meeting_id: meetingId } : {}),
    });
  } catch (err) {
    printError(`Failed to resolve reference: ${err}`);
    process.exit(1);
  }
}

function requiredEntity(entityId: string | undefined, kind: ResourceKind): string {
  if (!entityId) {
    throw new Error(`--entity-id is required to resolve ${kind}.`);
  }
  return entityId;
}

function requiredMeeting(meetingId: string | undefined, kind: ResourceKind): string {
  if (!meetingId) {
    throw new Error(`--meeting-id is required to resolve ${kind}.`);
  }
  return meetingId;
}
