/**
 * @module cli
 *
 * Programmatic interface to the `corp` CLI binary.
 * Provides typed wrappers around CLI commands for use in scripts and tests.
 *
 * ```ts
 * import { Corp } from "@thecorporation/corp/cli";
 *
 * const corp = new Corp({ apiUrl: "http://localhost:8000", apiKey: "corp_..." });
 * const entity = await corp.form.create("Acme Corp", "c_corp", "DE");
 * await corp.form.advance(entity.entity_id);
 * ```
 */

import { runCliJson, type CliRunOptions } from "./server.js";
import type {
  Entity,
  EntityType,
  Document,
  Filing,
  TaxProfile,
  Contact,
  GovernanceBody,
  Meeting,
  Agent,
  WorkItem,
} from "./types.js";

// ── Corp CLI wrapper ─────────────────────────────────────────────────────────

export class Corp {
  private opts: CliRunOptions;

  readonly form: FormCli;
  readonly entities: EntitiesCli;
  readonly governance: GovernanceCli;
  readonly contacts: ContactsCli;
  readonly agents: AgentsCli;
  readonly workItems: WorkItemsCli;

  constructor(opts: CliRunOptions = {}) {
    this.opts = opts;
    this.form = new FormCli(this.opts);
    this.entities = new EntitiesCli(this.opts);
    this.governance = new GovernanceCli(this.opts);
    this.contacts = new ContactsCli(this.opts);
    this.agents = new AgentsCli(this.opts);
    this.workItems = new WorkItemsCli(this.opts);
  }
}

// ── Formation ────────────────────────────────────────────────────────────────

class FormCli {
  constructor(private opts: CliRunOptions) {}

  create(name: string, entityType: EntityType = "c_corp", jurisdiction = "DE"): Promise<Entity> {
    return runCliJson(["form", "create", "--name", name, "--entity-type", entityType, "--jurisdiction", jurisdiction], this.opts);
  }

  advance(entityId: string): Promise<Entity> {
    return runCliJson(["form", "advance", entityId], this.opts);
  }

  status(entityId: string): Promise<Entity> {
    return runCliJson(["form", "status", entityId], this.opts);
  }

  documents(entityId: string): Promise<Document[]> {
    return runCliJson(["form", "documents", entityId], this.opts);
  }

  sign(documentId: string, opts: {
    signerName: string;
    signerRole: string;
    signerEmail: string;
    signatureText: string;
    consentText: string;
  }): Promise<Document> {
    return runCliJson([
      "form", "sign", documentId,
      "--signer-name", opts.signerName,
      "--signer-role", opts.signerRole,
      "--signer-email", opts.signerEmail,
      "--signature-text", opts.signatureText,
      "--consent-text", opts.consentText,
    ], this.opts);
  }

  filing(entityId: string): Promise<Filing> {
    return runCliJson(["form", "filing", entityId], this.opts);
  }

  confirmFiling(entityId: string, confirmationNumber?: string): Promise<Filing> {
    const args = ["form", "confirm-filing", entityId];
    if (confirmationNumber) args.push("--confirmation-number", confirmationNumber);
    return runCliJson(args, this.opts);
  }

  tax(entityId: string): Promise<TaxProfile> {
    return runCliJson(["form", "tax", entityId], this.opts);
  }

  confirmEin(entityId: string, ein: string): Promise<TaxProfile> {
    return runCliJson(["form", "confirm-ein", entityId, "--ein", ein], this.opts);
  }

  /**
   * Run the full formation workflow from Pending to Active.
   * Returns the finalized entity.
   */
  async fullLifecycle(
    name: string,
    entityType: EntityType = "c_corp",
    jurisdiction = "DE",
    signer: { name: string; role: string; email: string } = {
      name: "Authorized Signer",
      role: "Incorporator",
      email: "signer@example.com",
    },
  ): Promise<Entity> {
    // 1. Create entity
    const entity = await this.create(name, entityType, jurisdiction);
    const eid = entity.entity_id;

    // 2. Advance to DocumentsGenerated
    await this.advance(eid);

    // 3. Sign all documents
    const docs = await this.documents(eid);
    for (const doc of docs) {
      await this.sign(doc.document_id, {
        signerName: signer.name,
        signerRole: signer.role,
        signerEmail: signer.email,
        signatureText: `/s/ ${signer.name}`,
        consentText: "I consent to signing this document electronically",
      });
    }

    // 4. Advance through remaining states
    await this.advance(eid); // DocumentsSigned
    await this.advance(eid); // FilingSubmitted
    await this.advance(eid); // Filed

    // 5. Confirm filing
    await this.confirmFiling(eid, `${jurisdiction}-${Date.now()}`);

    // 6. Advance to EinApplied
    await this.advance(eid);

    // 7. Confirm EIN
    await this.confirmEin(eid, "12-3456789");

    // 8. Final advance to Active
    return this.advance(eid);
  }
}

// ── Entities ─────────────────────────────────────────────────────────────────

class EntitiesCli {
  constructor(private opts: CliRunOptions) {}

  list(): Promise<Entity[]> {
    return runCliJson(["entities", "list"], this.opts);
  }

  show(entityRef: string): Promise<Entity> {
    return runCliJson(["entities", "show", entityRef], this.opts);
  }

  create(name: string, entityType: EntityType = "c_corp", jurisdiction = "DE"): Promise<Entity> {
    return runCliJson(["entities", "create", "--name", name, "--entity-type", entityType, "--jurisdiction", jurisdiction], this.opts);
  }

  dissolve(entityRef: string): Promise<Entity> {
    return runCliJson(["entities", "dissolve", entityRef], this.opts);
  }
}

// ── Governance ───────────────────────────────────────────────────────────────

class GovernanceCli {
  constructor(private opts: CliRunOptions) {}

  quickApprove(bodyRef: string, title: string, description: string): Promise<unknown> {
    return runCliJson(["governance", "quick-approve", "--body-id", bodyRef, "--title", title, "--description", description], this.opts);
  }
}

// ── Contacts ─────────────────────────────────────────────────────────────────

class ContactsCli {
  constructor(private opts: CliRunOptions) {}

  list(): Promise<Contact[]> {
    return runCliJson(["contacts", "list"], this.opts);
  }

  add(name: string, email?: string, category?: string): Promise<Contact> {
    const args = ["contacts", "add", "--name", name];
    if (email) args.push("--email", email);
    if (category) args.push("--category", category);
    return runCliJson(args, this.opts);
  }
}

// ── Agents ───────────────────────────────────────────────────────────────────

class AgentsCli {
  constructor(private opts: CliRunOptions) {}

  list(): Promise<Agent[]> {
    return runCliJson(["agents", "list"], this.opts);
  }

  create(name: string, prompt?: string, model?: string): Promise<Agent> {
    const args = ["agents", "create", "--name", name];
    if (prompt) args.push("--prompt", prompt);
    if (model) args.push("--model", model);
    return runCliJson(args, this.opts);
  }
}

// ── Work Items ───────────────────────────────────────────────────────────────

class WorkItemsCli {
  constructor(private opts: CliRunOptions) {}

  list(): Promise<WorkItem[]> {
    return runCliJson(["work-items", "list"], this.opts);
  }

  create(title: string, category: string, description?: string): Promise<WorkItem> {
    const args = ["work-items", "create", "--title", title, "--category", category];
    if (description) args.push("--description", description);
    return runCliJson(args, this.opts);
  }
}
