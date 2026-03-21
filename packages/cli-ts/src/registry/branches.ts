import type { CommandDef } from "./types.js";

export const branchCommands: CommandDef[] = [
  {
    name: "branches",
    description: "List branches for an entity",
    route: { method: "GET", path: "/v1/branches" },
    entity: "query",
    display: {
      title: "Branches",
      cols: ["name>Branch", "head_oid>HEAD"],
    },
  },
  {
    name: "branches create",
    description: "Create a new branch",
    route: { method: "POST", path: "/v1/branches" },
    entity: "query",
    options: [
      { flags: "--name <name>", description: "Branch name", required: true },
      { flags: "--from <branch>", description: "Base branch (default: main)", default: "main" },
    ],
    successTemplate: "Branch {branch} created at {base_commit}",
  },
  {
    name: "branches merge",
    description: "Merge a branch into another",
    route: { method: "POST", path: "/v1/branches/{pos}/merge" },
    entity: "query",
    args: [{ name: "branch", required: true, description: "Branch to merge" }],
    options: [
      { flags: "--into <branch>", description: "Target branch (default: main)", default: "main" },
      { flags: "--squash", description: "Squash merge (default: true)" },
    ],
    successTemplate: "Merge {strategy}: {commit}",
  },
  {
    name: "branches delete",
    description: "Delete a branch",
    route: { method: "DELETE", path: "/v1/branches/{pos}" },
    entity: "query",
    args: [{ name: "branch", required: true, description: "Branch to delete" }],
    successTemplate: "Branch deleted",
  },
  {
    name: "branches prune",
    description: "Prune merged branches",
    route: { method: "POST", path: "/v1/branches/{pos}/prune" },
    entity: "query",
    args: [{ name: "branch", required: true, description: "Branch to prune" }],
    successTemplate: "Branch pruned",
  },
];