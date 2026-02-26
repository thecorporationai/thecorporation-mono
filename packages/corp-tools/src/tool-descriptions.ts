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

  const fmts: Record<string, string> = {
    form_entity: 'Form a new {entity_type} named "{entity_name}" in {jurisdiction}',
    convert_entity: "Convert entity to {new_entity_type}",
    dissolve_entity: "Dissolve entity — {dissolution_reason}",
    issue_equity: "Issue {shares} {grant_type} shares to {recipient_name}",
    transfer_shares: "Transfer {shares} shares to {to_recipient_name}",
    issue_safe: "Issue SAFE note to {investor_name} for {_amount}",
    calculate_distribution: "Calculate {distribution_type} distribution of {_amount}",
    create_invoice: "Create invoice for {customer_name} — {_amount}",
    run_payroll: "Run payroll for {pay_period_start} to {pay_period_end}",
    submit_payment: "Submit {_amount} payment to {recipient} via {payment_method}",
    open_bank_account: "Open bank account at {institution_name}",
    reconcile_ledger: "Reconcile ledger from {start_date} to {end_date}",
    generate_contract: "Generate {template_type} contract for {counterparty_name}",
    file_tax_document: "File {document_type} for tax year {tax_year}",
    track_deadline: "Track {deadline_type} deadline — {description}",
    classify_contractor: "Classify contractor {contractor_name} in {state}",
    convene_meeting: "Convene {meeting_type} meeting",
    cast_vote: "Cast {vote} vote",
    schedule_meeting: "Schedule {meeting_type} meeting: {title}",
    update_checklist: "Update workspace checklist",
    create_agent: 'Create agent "{name}"',
    send_agent_message: "Send message to agent",
    update_agent: "Update agent configuration",
    add_agent_skill: 'Add skill "{skill_name}" to agent',
  };

  const fmt = fmts[name];
  if (fmt) {
    try {
      return fmt.replace(/\{(\w+)\}/g, (_, k: string) => String(a[k] ?? "?"));
    } catch { /* fall through */ }
  }
  return name.replace(/_/g, " ");
}
