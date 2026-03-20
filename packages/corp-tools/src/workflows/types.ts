// ---------------------------------------------------------------------------
// Shared workflow result types
// ---------------------------------------------------------------------------

/**
 * A single step executed inside a multi-step workflow.
 */
export interface WorkflowStep {
  name: string;
  status: "ok" | "skipped" | "failed";
  data?: Record<string, unknown>;
  detail?: string;
}

/**
 * Structured result returned by every workflow function.
 * The caller (CLI, web, tests) decides how to render it.
 */
export interface WorkflowResult {
  success: boolean;
  data?: Record<string, unknown>;
  error?: string;
  steps: WorkflowStep[];
}
