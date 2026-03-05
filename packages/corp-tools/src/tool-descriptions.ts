function centsToUsd(cents: number): string {
  return "$" + (cents / 100).toLocaleString("en-US", { minimumFractionDigits: 0, maximumFractionDigits: 0 });
}

export function describeToolCall(name: string, args: Record<string, unknown>): string {
  const a = { ...args } as Record<string, unknown>;
  for (const k of ["amount_cents", "principal_amount_cents", "total_amount_cents", "valuation_cap_cents"]) {
    if (k in a) {
      try { a._amount = centsToUsd(Number(a[k])); } catch { a._amount = String(a[k]); }
    }
  }
  a._amount ??= "?";
  a.institution_name ??= "Mercury";
  a.payment_method ??= "ach";

  const action = a.action as string | undefined;
  const key = action ? `${name}:${action}` : name;

  const fmts: Record<string, string> = {
    // workspace
    "workspace:status": "Get workspace status",
    "workspace:list_entities": "List all entities",
    "workspace:obligations": "List obligations",
    "workspace:billing": "Get billing status",

    // entity
    "entity:get_cap_table": "Get cap table",
    "entity:list_documents": "List documents",
    "entity:list_safe_notes": "List SAFE notes",
    "entity:create": 'Create pending {entity_type} named "{entity_name}"',
    "entity:add_founder": 'Add founder "{name}" ({role}, {ownership_pct}%)',
    "entity:finalize": "Finalize formation and generate documents + cap table",
    "entity:form": 'Form a new {entity_type} named "{entity_name}" in {jurisdiction}',
    "entity:convert": "Convert entity to {new_entity_type}",
    "entity:dissolve": "Dissolve entity — {reason}",

    // equity
    "equity:start_round": 'Start equity round "{name}"',
    "equity:add_security": "Add {quantity} shares to {recipient_name} in round",
    "equity:issue_round": "Issue all securities and close the round",
    "equity:issue": "Issue {shares} {grant_type} shares to {recipient_name}",
    "equity:issue_safe": "Issue SAFE note to {investor_name} for {_amount}",
    "equity:transfer": "Direct transfer {shares} shares (bypass governance)",
    "equity:distribution": "Calculate {distribution_type} distribution of {_amount}",

    // valuation
    "valuation:create": "Create {valuation_type} valuation effective {effective_date}",
    "valuation:submit": "Submit valuation for board approval",
    "valuation:approve": "Approve valuation",

    // meeting
    "meeting:schedule": 'Schedule {meeting_type} meeting: "{title}"',
    "meeting:notice": "Send notice for meeting",
    "meeting:convene": "Convene meeting",
    "meeting:vote": "Cast {vote_value} vote",
    "meeting:resolve": "Compute resolution for agenda item",
    "meeting:finalize_item": "Finalize agenda item to {status}",
    "meeting:adjourn": "Adjourn meeting",
    "meeting:cancel": "Cancel meeting",
    "meeting:consent": 'Create written consent: "{title}"',
    "meeting:attach_document": "Attach document to resolution",
    "meeting:list_items": "List agenda items for meeting",
    "meeting:list_votes": "List votes on agenda item",

    // finance
    "finance:create_invoice": "Create invoice for {customer_name} — {_amount}",
    "finance:run_payroll": "Run payroll for {pay_period_start} to {pay_period_end}",
    "finance:submit_payment": "Submit {_amount} payment to {recipient} via {payment_method}",
    "finance:open_bank_account": "Open bank account at {institution_name}",
    "finance:reconcile": "Reconcile ledger from {start_date} to {end_date}",

    // compliance
    "compliance:file_tax": "File {document_type} for tax year {tax_year}",
    "compliance:track_deadline": "Track {deadline_type} deadline — {description}",
    "compliance:classify_contractor": "Classify contractor {contractor_name} in {state}",
    "compliance:generate_contract": "Generate {template_type} contract for {counterparty_name}",

    // document
    "document:signing_link": "Get signing link for document",
    "document:signer_link": "Generate signer link for obligation",
    "document:download_link": "Get download link for document",
    "document:preview_pdf": "Preview document PDF for {document_id}",

    // checklist
    "checklist:get": "Get workspace checklist",
    "checklist:update": "Update workspace checklist",

    // agent
    "agent:list": "List agents",
    "agent:create": 'Create agent "{name}"',
    "agent:message": "Send message to agent",
    "agent:update": "Update agent configuration",
    "agent:add_skill": 'Add skill "{skill_name}" to agent',
  };

  const fmt = fmts[key];
  if (fmt) {
    try {
      return fmt.replace(/\{(\w+)\}/g, (_, k: string) => String(a[k] ?? "?"));
    } catch { /* fall through */ }
  }
  return action ? `${name} → ${action}` : name.replace(/_/g, " ");
}
