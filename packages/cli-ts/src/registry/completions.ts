import type { CommandDef, CommandContext } from "./types.js";

// ── Completion script generators ─────────────────────────────────────────────

/**
 * Build a hierarchical map from a flat registry:
 *   { "governance": { subcmds: ["seats", "bodies"], options: [...] }, ... }
 */
interface CmdNode {
  name: string;
  description: string;
  options: string[]; // flag strings, e.g. "--json", "--entity-id"
  subcmds: CmdNode[];
}

function buildTree(commands: CommandDef[]): CmdNode[] {
  const topLevelMap = new Map<string, CmdNode>();
  const topLevel: CmdNode[] = [];

  for (const cmd of commands) {
    if (cmd.hidden) continue;

    const parts = cmd.name.split(" ");

    // Collect all flags for this command (mirrors what buildCLI does)
    const opts: string[] = ["--json"];
    if (cmd.entity) opts.push("--entity-id");
    if (cmd.dryRun) opts.push("--dry-run");
    for (const o of cmd.options ?? []) {
      if (o.hidden) continue;
      // Extract the long flag name (first token starting with --)
      const longFlag = o.flags.split(/\s+/).find((f) => f.startsWith("--"));
      if (longFlag) {
        // Strip trailing angle/square brackets: "--shell <shell>" -> "--shell"
        opts.push(longFlag.replace(/<[^>]*>|\[[^\]]*\]/, "").trim());
      }
    }

    const node: CmdNode = {
      name: parts[parts.length - 1],
      description: cmd.description,
      options: opts,
      subcmds: [],
    };

    if (parts.length === 1) {
      topLevelMap.set(parts[0], node);
      topLevel.push(node);
    } else {
      const parentName = parts[0];
      let parent = topLevelMap.get(parentName);
      if (!parent) {
        parent = { name: parentName, description: "", options: [], subcmds: [] };
        topLevelMap.set(parentName, parent);
        topLevel.push(parent);
      }
      parent.subcmds.push(node);
    }
  }

  return topLevel;
}

// ── Bash ─────────────────────────────────────────────────────────────────────

function generateBash(commands: CommandDef[]): string {
  const tree = buildTree(commands);

  const lines: string[] = [
    "# corp bash completion",
    "# Source this file or add to ~/.bash_completion.d/",
    "#   source <(corp completions --shell bash)",
    "",
    "_corp_completions() {",
    '  local cur prev words cword',
    '  _init_completion 2>/dev/null || {',
    '    COMPREPLY=()',
    '    cur="${COMP_WORDS[COMP_CWORD]}"',
    '    prev="${COMP_WORDS[COMP_CWORD-1]}"',
    '    words=("${COMP_WORDS[@]}")',
    '    cword=$COMP_CWORD',
    "  }",
    "",
    '  local subcommand=""',
    '  local subsubcommand=""',
    '  if [[ ${#words[@]} -ge 2 ]]; then subcommand="${words[1]}"; fi',
    '  if [[ ${#words[@]} -ge 3 ]]; then subsubcommand="${words[2]}"; fi',
    "",
  ];

  // Top-level command names
  const topNames = tree.map((n) => n.name).join(" ");
  lines.push(`  local top_commands="${topNames}"`);
  lines.push("");

  // Switch on subcommand
  lines.push('  case "$subcommand" in');

  for (const parent of tree) {
    if (parent.subcmds.length > 0) {
      // Parent with subcommands: on depth=2 complete subcmd names, on depth=3 complete flags
      const subNames = parent.subcmds.map((s) => s.name).join(" ");
      lines.push(`    ${parent.name})`);
      lines.push(`      local ${parent.name}_subcmds="${subNames}"`);
      lines.push(`      case "$subsubcommand" in`);
      for (const child of parent.subcmds) {
        const childFlags = child.options.join(" ");
        lines.push(`        ${child.name})`);
        lines.push(`          COMPREPLY=( $(compgen -W "${childFlags}" -- "$cur") )`);
        lines.push("          return 0 ;;");
      }
      // No subsubcommand matched: complete subcmd names (or flags of the parent itself)
      const parentFlags = parent.options.join(" ");
      lines.push("        *)");
      if (parent.options.length > 0) {
        lines.push(`          COMPREPLY=( $(compgen -W "${parent.subcmds.map((s) => s.name).join(" ")} ${parentFlags}" -- "$cur") )`);
      } else {
        lines.push(`          COMPREPLY=( $(compgen -W "$${parent.name}_subcmds" -- "$cur") )`);
      }
      lines.push("          return 0 ;;");
      lines.push("      esac ;;");
    } else {
      // Leaf command at top level: complete flags
      const flags = parent.options.join(" ");
      lines.push(`    ${parent.name})`);
      lines.push(`      COMPREPLY=( $(compgen -W "${flags}" -- "$cur") )`);
      lines.push("      return 0 ;;");
    }
  }

  lines.push("    *)");
  lines.push('      COMPREPLY=( $(compgen -W "$top_commands" -- "$cur") )');
  lines.push("      return 0 ;;");
  lines.push("  esac");
  lines.push("}");
  lines.push("");
  lines.push("complete -F _corp_completions corp");
  lines.push("complete -F _corp_completions npx corp");

  return lines.join("\n");
}

// ── Zsh ──────────────────────────────────────────────────────────────────────

function escapeZshDesc(s: string): string {
  return s.replace(/'/g, "'\\''").replace(/:/g, "\\:");
}

function generateZsh(commands: CommandDef[]): string {
  const tree = buildTree(commands);

  const lines: string[] = [
    "#compdef corp",
    "# corp zsh completion",
    "# Add to your fpath and run: compdef _corp corp",
    "#   eval \"$(corp completions --shell zsh)\"",
    "",
    "_corp() {",
    "  local state",
    '  local -a top_commands',
    "",
  ];

  // Build top-level command list
  lines.push("  top_commands=(");
  for (const node of tree) {
    lines.push(`    '${node.name}:${escapeZshDesc(node.description)}'`);
  }
  lines.push("  )");
  lines.push("");

  lines.push("  _arguments -C \\");
  lines.push("    '(-h --help)'{-h,--help}'[Show help]' \\");
  lines.push("    '(-V --version)'{-V,--version}'[Show version]' \\");
  lines.push("    '(-q --quiet)'{-q,--quiet}'[Quiet output]' \\");
  lines.push("    '1: :->cmd' \\");
  lines.push("    '*: :->args'");
  lines.push("");
  lines.push("  case $state in");
  lines.push("    cmd)");
  lines.push("      _describe 'command' top_commands ;;");
  lines.push("    args)");
  lines.push("      case $words[2] in");

  for (const parent of tree) {
    lines.push(`        ${parent.name})`);
    if (parent.subcmds.length > 0) {
      // Has subcommands
      lines.push("          local -a subcmds");
      lines.push("          subcmds=(");
      for (const child of parent.subcmds) {
        lines.push(`            '${child.name}:${escapeZshDesc(child.description)}'`);
      }
      lines.push("          )");
      lines.push("          if (( CURRENT == 3 )); then");
      lines.push("            _describe 'subcommand' subcmds");
      lines.push("          else");
      lines.push("            case $words[3] in");
      for (const child of parent.subcmds) {
        const argSpec = child.options.map((f) => `'${f}[option]'`).join(" \\\n                ");
        lines.push(`              ${child.name})`);
        lines.push(`                _arguments ${argSpec} ;;`);
      }
      lines.push("            esac");
      lines.push("          fi ;;");
    } else {
      // Leaf: complete flags
      const argSpec = parent.options.map((f) => `'${f}[option]'`).join(" \\\n            ");
      if (argSpec) {
        lines.push(`          _arguments ${argSpec} ;;`);
      } else {
        lines.push("          ;; # no options");
      }
    }
  }

  lines.push("      esac ;;");
  lines.push("  esac");
  lines.push("}");
  lines.push("");
  lines.push("_corp");

  return lines.join("\n");
}

// ── Fish ─────────────────────────────────────────────────────────────────────

function generateFish(commands: CommandDef[]): string {
  const tree = buildTree(commands);

  const lines: string[] = [
    "# corp fish completion",
    "# Save to ~/.config/fish/completions/corp.fish",
    "#   corp completions --shell fish > ~/.config/fish/completions/corp.fish",
    "",
  ];

  // Disable file completions for corp
  lines.push("complete -c corp -f");
  lines.push("");

  // Top-level commands
  for (const node of tree) {
    const desc = node.description.replace(/'/g, "\\'");
    lines.push(`complete -c corp -n '__fish_use_subcommand' -a '${node.name}' -d '${desc}'`);
  }
  lines.push("");

  // Subcommands and flags
  for (const parent of tree) {
    const parentName = parent.name;

    if (parent.subcmds.length > 0) {
      // Subcommands: complete child names when the parent is selected
      for (const child of parent.subcmds) {
        const desc = child.description.replace(/'/g, "\\'");
        lines.push(
          `complete -c corp -n "__fish_seen_subcommand_from ${parentName}; and not __fish_seen_subcommand_from ${parent.subcmds.map((s) => s.name).join(" ")}" -a '${child.name}' -d '${desc}'`,
        );
      }
      lines.push("");

      // Flags for each subcommand
      for (const child of parent.subcmds) {
        for (const flag of child.options) {
          // Strip leading dashes for fish -l
          const long = flag.replace(/^--/, "");
          lines.push(
            `complete -c corp -n "__fish_seen_subcommand_from ${parentName}; and __fish_seen_subcommand_from ${child.name}" -l '${long}'`,
          );
        }
        if (child.options.length > 0) lines.push("");
      }
    } else {
      // Leaf: flags under this top-level command
      for (const flag of parent.options) {
        const long = flag.replace(/^--/, "");
        lines.push(
          `complete -c corp -n "__fish_seen_subcommand_from ${parentName}" -l '${long}'`,
        );
      }
      if (parent.options.length > 0) lines.push("");
    }
  }

  return lines.join("\n");
}

// ── Registry factory ─────────────────────────────────────────────────────────

/**
 * Returns the `completions` CommandDef.
 *
 * Accepts the full registry so the handler can generate shell scripts
 * from real command data without creating a circular import.
 */
export function makeCompletionsCommand(allCommands: CommandDef[]): CommandDef {
  return {
    name: "completions",
    description: "Generate shell completion scripts",
    local: true,
    options: [
      {
        flags: "--shell <shell>",
        description: "Shell type (bash, zsh, fish)",
        choices: ["bash", "zsh", "fish"],
        required: true,
      },
    ],
    examples: [
      "corp completions --shell bash",
      "corp completions --shell zsh",
      "corp completions --shell fish",
      "source <(corp completions --shell bash)",
      'eval "$(corp completions --shell zsh)"',
      "corp completions --shell fish > ~/.config/fish/completions/corp.fish",
    ],
    handler: async (ctx: CommandContext): Promise<void> => {
      const shell = ctx.opts.shell as string;

      // Exclude the completions command itself to avoid self-referential noise
      const cmds = allCommands.filter((c) => c.name !== "completions");

      let script: string;
      switch (shell) {
        case "bash":
          script = generateBash(cmds);
          break;
        case "zsh":
          script = generateZsh(cmds);
          break;
        case "fish":
          script = generateFish(cmds);
          break;
        default:
          ctx.writer.error(`Unknown shell: ${shell}. Choose bash, zsh, or fish.`);
          process.exit(1);
      }

      process.stdout.write(script + "\n");
    },
  };
}
