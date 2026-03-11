import { printError } from "../output.js";

export async function approvalsListCommand(_opts: { json?: boolean }): Promise<void> {
  printError(
    "Approvals are managed through governance meetings.\n" +
    "  Use: corp governance convene ... to schedule a board meeting\n" +
    "  Use: corp governance vote <meeting-ref> <item-ref> ... to cast votes"
  );
  process.exit(1);
}

export async function approvalsRespondCommand(
  _approvalId: string,
  _decision: string,
  _opts: { message?: string }
): Promise<void> {
  printError(
    "Approvals are managed through governance meetings.\n" +
    "  Use: corp governance vote <meeting-ref> <item-ref> --voter <contact-ref> --vote for"
  );
  process.exit(1);
}
