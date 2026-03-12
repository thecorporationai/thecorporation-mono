import { CorpAPIClient } from "./api-client.js";
import { ReferenceResolver } from "./references.js";
import type { ApiRecord } from "./types.js";

const DEFAULT_SIGNATURE_CONSENT = "I agree to sign this document electronically.";
const DEFAULT_ATTESTATION_CONSENT = "I attest the filing information is accurate and authorized.";

type SignatureRequirement = {
  role: string;
  signer_name: string;
  signer_email?: string;
};

export type AutoSignSummary = {
  documents_seen: number;
  documents_signed: number;
  signatures_added: number;
  documents: ApiRecord[];
};

export type ActivationSummary = {
  entity_id: string;
  initial_status: string;
  final_status: string;
  signatures_added: number;
  documents_signed: number;
  steps: string[];
  formation: ApiRecord;
};

function normalizeRole(value: unknown): string {
  return String(value ?? "").trim().toLowerCase();
}

function fallbackSignerEmail(requirement: SignatureRequirement): string {
  const slug = String(requirement.signer_name ?? "signer")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, ".")
    .replace(/^\.+|\.+$/g, "");
  return `${slug || "signer"}@example.com`;
}

function getSignatureRequirements(document: ApiRecord): SignatureRequirement[] {
  const content = document.content;
  if (!content || typeof content !== "object" || Array.isArray(content)) {
    return [];
  }
  const requirements = (content as ApiRecord).signature_requirements;
  if (!Array.isArray(requirements)) {
    return [];
  }
  return requirements
    .filter((item): item is ApiRecord => typeof item === "object" && item !== null && !Array.isArray(item))
    .map((item) => ({
      role: String(item.role ?? "").trim(),
      signer_name: String(item.signer_name ?? "").trim(),
      signer_email:
        typeof item.signer_email === "string" && item.signer_email.trim()
          ? item.signer_email.trim()
          : undefined,
    }))
    .filter((item) => item.role.length > 0 && item.signer_name.length > 0);
}

function getSignedRoles(document: ApiRecord): Set<string> {
  const signatures = Array.isArray(document.signatures) ? document.signatures : [];
  return new Set(
    signatures
      .filter((item): item is ApiRecord => typeof item === "object" && item !== null && !Array.isArray(item))
      .map((item) => normalizeRole(item.signer_role))
      .filter(Boolean),
  );
}

function deterministicEin(entityId: string): string {
  const digits = entityId.replace(/\D/g, "");
  const nineDigits = `${digits}123456789`.slice(0, 9).padEnd(9, "0");
  return `${nineDigits.slice(0, 2)}-${nineDigits.slice(2)}`;
}

function filingIdentifier(prefix: string, entityId: string): string {
  const token = entityId.replace(/[^a-zA-Z0-9]/g, "").slice(0, 12).toLowerCase() || "formation";
  return `${prefix}-${token}`;
}

export async function autoSignFormationDocument(
  client: CorpAPIClient,
  entityId: string,
  documentId: string,
): Promise<{ document: ApiRecord; signatures_added: number }> {
  const document = await client.getDocument(documentId, entityId) as ApiRecord;
  const requiredSignatures = getSignatureRequirements(document);
  if (requiredSignatures.length === 0) {
    return { document, signatures_added: 0 };
  }

  const signedRoles = getSignedRoles(document);
  let signaturesAdded = 0;
  for (const requirement of requiredSignatures) {
    if (signedRoles.has(normalizeRole(requirement.role))) {
      continue;
    }
    await client.signDocument(documentId, entityId, {
      signer_name: requirement.signer_name,
      signer_role: requirement.role,
      signer_email: requirement.signer_email ?? fallbackSignerEmail(requirement),
      signature_text: requirement.signer_name,
      consent_text: DEFAULT_SIGNATURE_CONSENT,
    });
    signedRoles.add(normalizeRole(requirement.role));
    signaturesAdded += 1;
  }

  const refreshed = await client.getDocument(documentId, entityId) as ApiRecord;
  return { document: refreshed, signatures_added: signaturesAdded };
}

export async function autoSignFormationDocuments(
  client: CorpAPIClient,
  resolver: ReferenceResolver,
  entityId: string,
): Promise<AutoSignSummary> {
  const summaries = await client.getEntityDocuments(entityId) as ApiRecord[];
  await resolver.stabilizeRecords("document", summaries, entityId);

  let signaturesAdded = 0;
  let documentsSigned = 0;
  const documents: ApiRecord[] = [];

  for (const summary of summaries) {
    const documentId = String(summary.document_id ?? "");
    if (!documentId) {
      continue;
    }
    const { document, signatures_added } = await autoSignFormationDocument(
      client,
      entityId,
      documentId,
    );
    signaturesAdded += signatures_added;
    if (signatures_added > 0) {
      documentsSigned += 1;
    }
    documents.push(document);
  }

  return {
    documents_seen: summaries.length,
    documents_signed: documentsSigned,
    signatures_added: signaturesAdded,
    documents,
  };
}

export async function activateFormationEntity(
  client: CorpAPIClient,
  resolver: ReferenceResolver,
  entityId: string,
  options: {
    evidenceUri?: string;
    evidenceType?: string;
    filingId?: string;
    receiptReference?: string;
    ein?: string;
  } = {},
): Promise<ActivationSummary> {
  let formation = await client.getFormation(entityId) as ApiRecord;
  const initialStatus = String(formation.formation_status ?? "");
  const steps: string[] = [];
  let signaturesAdded = 0;
  let documentsSigned = 0;

  for (let i = 0; i < 10; i += 1) {
    const status = String(formation.formation_status ?? "");
    if (status === "active") {
      return {
        entity_id: entityId,
        initial_status: initialStatus,
        final_status: status,
        signatures_added: signaturesAdded,
        documents_signed: documentsSigned,
        steps,
        formation,
      };
    }

    if (status === "pending") {
      throw new Error("Formation is still pending. Finalize it before activation.");
    }

    if (status === "documents_generated") {
      const signed = await autoSignFormationDocuments(client, resolver, entityId);
      signaturesAdded += signed.signatures_added;
      documentsSigned += signed.documents_signed;
      await client.markFormationDocumentsSigned(entityId);
      steps.push(`signed ${signed.signatures_added} signatures across ${signed.documents_signed} documents`);
      formation = await client.getFormation(entityId) as ApiRecord;
      continue;
    }

    if (status === "documents_signed") {
      const gates = await client.getFormationGates(entityId);
      if (gates.requires_natural_person_attestation && !gates.attestation_recorded) {
        await client.recordFilingAttestation(entityId, {
          signer_name: gates.designated_attestor_name,
          signer_role: gates.designated_attestor_role,
          signer_email: gates.designated_attestor_email ?? fallbackSignerEmail({
            role: String(gates.designated_attestor_role ?? "attestor"),
            signer_name: String(gates.designated_attestor_name ?? "attestor"),
          }),
          consent_text: DEFAULT_ATTESTATION_CONSENT,
          notes: "Recorded automatically by corp form activate",
        });
        steps.push("recorded filing attestation");
      }
      const gatesAfterAttestation = await client.getFormationGates(entityId);
      if (
        gatesAfterAttestation.requires_registered_agent_consent_evidence
        && Number(gatesAfterAttestation.registered_agent_consent_evidence_count ?? 0) === 0
      ) {
        await client.addRegisteredAgentConsentEvidence(entityId, {
          evidence_uri: options.evidenceUri ?? `generated://registered-agent-consent/${entityId}`,
          evidence_type: options.evidenceType ?? "generated",
          notes: "Recorded automatically by corp form activate",
        });
        steps.push("recorded registered-agent consent evidence");
      }
      const finalGates = await client.getFormationGates(entityId);
      if (Array.isArray(finalGates.filing_submission_blockers) && finalGates.filing_submission_blockers.length > 0) {
        throw new Error(
          `Formation filing is still blocked: ${finalGates.filing_submission_blockers.join("; ")}`,
        );
      }
      await client.submitFiling(entityId);
      steps.push("submitted filing");
      formation = await client.getFormation(entityId) as ApiRecord;
      continue;
    }

    if (status === "filing_submitted") {
      await client.confirmFiling(entityId, {
        external_filing_id: options.filingId ?? filingIdentifier("sim", entityId),
        receipt_reference: options.receiptReference ?? filingIdentifier("receipt", entityId),
      });
      steps.push("confirmed filing");
      formation = await client.getFormation(entityId) as ApiRecord;
      continue;
    }

    if (status === "filed") {
      await client.applyEin(entityId);
      steps.push("submitted EIN application");
      formation = await client.getFormation(entityId) as ApiRecord;
      continue;
    }

    if (status === "ein_applied") {
      await client.confirmEin(entityId, { ein: options.ein ?? deterministicEin(entityId) });
      steps.push("confirmed EIN");
      formation = await client.getFormation(entityId) as ApiRecord;
      continue;
    }

    throw new Error(`Unsupported formation status for activation: ${status || "unknown"}`);
  }

  throw new Error("Formation activation did not converge within 10 steps.");
}
