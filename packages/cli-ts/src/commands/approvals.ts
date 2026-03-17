import chalk from "chalk";
import { printError } from "../output.js";

export async function approvalsListCommand(_opts: Record<string, unknown>): Promise<void> {
  console.log(chalk.bold("Approvals in TheCorporation"));
  console.log();
  console.log("Approvals are handled through governance meetings and execution intents.");
  console.log("Use these commands to manage approvals:");
  console.log();
  console.log(chalk.dim("  Board approval via meeting vote:"));
  console.log(`    corp governance convene --body <body> --type board_meeting --title "Approve X"`);
  console.log(`    corp governance vote <meeting> <item> --voter <contact> --vote for`);
  console.log();
  console.log(chalk.dim("  Written consent (no meeting needed):"));
  console.log(`    corp governance written-consent --body <body> --title "Approve X" --description "..."`);
  console.log();
  console.log(chalk.dim("  View pending items:"));
  console.log(`    corp governance meetings <body>        # see scheduled meetings`);
  console.log(`    corp governance agenda-items <meeting>  # see items awaiting votes`);
  console.log(`    corp cap-table valuations               # see pending valuations`);
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
