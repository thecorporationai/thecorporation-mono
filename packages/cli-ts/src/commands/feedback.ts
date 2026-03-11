import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson } from "../output.js";
import chalk from "chalk";

interface FeedbackOptions {
  category?: string;
  email?: string;
  json?: boolean;
}

const MAX_FEEDBACK_LENGTH = 10_000;

export async function feedbackCommand(message: string, opts: FeedbackOptions): Promise<void> {
  if (!message || message.trim().length === 0) {
    printError("Feedback message cannot be empty");
    process.exit(1);
  }
  if (message.length > MAX_FEEDBACK_LENGTH) {
    printError(`Feedback message must be at most ${MAX_FEEDBACK_LENGTH} characters (got ${message.length})`);
    process.exit(1);
  }
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  try {
    const result = await client.submitFeedback(message, opts.category, opts.email);
    if (opts.json) {
      printJson(result);
      return;
    }
    console.log(`\n${chalk.green("✓")} Feedback submitted (${chalk.dim(result.feedback_id)})`);
  } catch (err: any) {
    printError("Failed to submit feedback", err);
    process.exit(1);
  }
}
