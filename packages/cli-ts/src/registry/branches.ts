import type { CommandDef } from "./types.js";

export const branchCommands: CommandDef[] = [
  {
    name: "branches",
    description: "List branches for an entity repo",
    route: { method: "GET", path: "/v1/branches" },
    entity: "query",
    display: {
      title: "Branches",
      cols: ["name>Branch", "head_oid>HEAD"],
    },
    examples: ["corp branches", "corp branches --json"],
  },
  {
    name: "branches create",
    description: "Create a new branch from an existing one",
    route: { method: "POST", path: "/v1/branches" },
    entity: "query",
    options: [
      { flags: "--name <name>", description: "Branch name", required: true },
      { flags: "--from <branch>", description: "Base branch (default: main)", default: "main" },
    ],
    successTemplate: "Branch {branch} created at {base_commit}",
    examples: ["corp branches create --name 'name'", "corp branches create --json"],
  },
  {
    name: "branches merge",
    description: "Merge a branch into another (default: main)",
    route: { method: "POST", path: "/v1/branches/{pos}/merge" },
    entity: "query",
    args: [{ name: "branch", required: true, description: "Branch to merge" }],
    options: [
      { flags: "--into <branch>", description: "Target branch (default: main)", default: "main" },
      { flags: "--squash", description: "Squash merge (default: true)" },
    ],
    successTemplate: "Merge {strategy}: {commit}",
    examples: ["corp branches merge <branch>", "corp branches merge --json"],
  },
  {
    name: "branches delete",
    description: "Delete a branch",
    route: { method: "DELETE", path: "/v1/branches/{pos}" },
    entity: "query",
    args: [{ name: "branch", required: true, description: "Branch to delete" }],
    successTemplate: "Branch deleted",
    examples: ["corp branches delete <branch>"],
  },
  {
    name: "branches prune",
    description: "Prune a merged branch",
    route: { method: "POST", path: "/v1/branches/{pos}/prune" },
    entity: "query",
    args: [{ name: "branch", required: true, description: "Branch to prune" }],
    successTemplate: "Branch pruned",
    examples: ["corp branches prune <branch>"],
  },
];