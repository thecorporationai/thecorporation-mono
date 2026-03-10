import type { Command } from "commander";
import { printJson } from "../output.js";

type SchemaArgument = {
  name: string;
  required: boolean;
  variadic: boolean;
  defaultValue?: unknown;
  choices?: string[];
};

type SchemaOption = {
  flags: string;
  name: string;
  description: string;
  required: boolean;
  mandatory: boolean;
  variadic: boolean;
  defaultValue?: unknown;
  choices?: string[];
};

type SchemaCommand = {
  path: string;
  name: string;
  description: string;
  aliases: string[];
  arguments: SchemaArgument[];
  options: SchemaOption[];
  subcommands: SchemaCommand[];
};

function readChoices(value: unknown): string[] | undefined {
  if (Array.isArray(value) && value.every((entry) => typeof entry === "string")) {
    return value;
  }
  return undefined;
}

function commandToSchema(command: Command, parentPath = ""): SchemaCommand {
  const cmd = command as Command & {
    registeredArguments?: Array<{
      name: () => string;
      required?: boolean;
      variadic?: boolean;
      defaultValue?: unknown;
      argChoices?: unknown;
    }>;
  };

  const path = parentPath ? `${parentPath} ${command.name()}` : command.name();
  return {
    path,
    name: command.name(),
    description: command.description(),
    aliases: command.aliases(),
    arguments: (cmd.registeredArguments ?? []).map((arg) => ({
      name: arg.name(),
      required: Boolean(arg.required),
      variadic: Boolean(arg.variadic),
      defaultValue: arg.defaultValue,
      choices: readChoices(arg.argChoices),
    })),
    options: command.options.map((option) => ({
      flags: option.flags,
      name: option.attributeName(),
      description: option.description ?? "",
      required: option.required,
      mandatory: option.mandatory,
      variadic: option.variadic,
      defaultValue: option.defaultValue,
      choices: readChoices(option.argChoices),
    })),
    subcommands: command.commands.map((child) => commandToSchema(child, path)),
  };
}

export function schemaCommand(program: Command, opts: { compact?: boolean }): void {
  const manifest = {
    name: program.name(),
    version: program.version(),
    description: program.description(),
    generated_at: new Date().toISOString(),
    commands: program.commands.map((command) => commandToSchema(command, program.name())),
  };
  if (opts.compact) {
    console.log(JSON.stringify(manifest));
    return;
  }
  printJson(manifest);
}
