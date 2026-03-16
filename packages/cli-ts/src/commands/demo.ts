import { requireConfig } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson, printReferenceSummary, printSuccess } from "../output.js";
import { withSpinner } from "../spinner.js";
import { ReferenceResolver } from "../references.js";
import { activateFormationEntity } from "../formation-automation.js";
import type { ApiRecord } from "../types.js";

type DemoOptions = {
  name: string;
  scenario?: string;
  minimal?: boolean;
  json?: boolean;
};

function scenarioConfig(name: string, scenario: string): ApiRecord {
  switch (scenario) {
    case "llc":
      return {
        entity_type: "llc",
        legal_name: name,
        jurisdiction: "US-WY",
        members: [
          {
            name: "Alice Chen",
            email: "alice@example.com",
            role: "member",
            investor_type: "natural_person",
            ownership_pct: 60,
            address: {
              street: "251 Little Falls Dr",
              city: "Wilmington",
              state: "DE",
              zip: "19808",
            },
          },
          {
            name: "Bob Martinez",
            email: "bob@example.com",
            role: "member",
            investor_type: "natural_person",
            ownership_pct: 40,
            address: {
              street: "251 Little Falls Dr",
              city: "Wilmington",
              state: "DE",
              zip: "19808",
            },
          },
        ],
        fiscal_year_end: "12-31",
        company_address: {
          street: "251 Little Falls Dr",
          city: "Wilmington",
          state: "DE",
          zip: "19808",
        },
      };
    case "restaurant":
      return {
        entity_type: "llc",
        legal_name: name,
        jurisdiction: "US-DE",
        members: [
          {
            name: "Rosa Alvarez",
            email: "rosa@example.com",
            role: "manager",
            investor_type: "natural_person",
            ownership_pct: 55,
            address: {
              street: "18 Market St",
              city: "Wilmington",
              state: "DE",
              zip: "19801",
            },
          },
          {
            name: "Noah Patel",
            email: "noah@example.com",
            role: "member",
            investor_type: "natural_person",
            ownership_pct: 45,
            address: {
              street: "18 Market St",
              city: "Wilmington",
              state: "DE",
              zip: "19801",
            },
          },
        ],
        fiscal_year_end: "12-31",
        company_address: {
          street: "18 Market St",
          city: "Wilmington",
          state: "DE",
          zip: "19801",
        },
      };
    case "startup":
    default:
      return {
        entity_type: "c_corp",
        legal_name: name,
        jurisdiction: "US-DE",
        authorized_shares: 10_000_000,
        par_value: "0.0001",
        transfer_restrictions: true,
        right_of_first_refusal: true,
        members: [
          {
            name: "Alice Chen",
            email: "alice@example.com",
            role: "chair",
            investor_type: "natural_person",
            shares_purchased: 6_000_000,
            officer_title: "ceo",
            is_incorporator: true,
            address: {
              street: "251 Little Falls Dr",
              city: "Wilmington",
              state: "DE",
              zip: "19808",
            },
          },
          {
            name: "Bob Martinez",
            email: "bob@example.com",
            role: "director",
            investor_type: "natural_person",
            shares_purchased: 4_000_000,
            officer_title: "cto",
            address: {
              street: "251 Little Falls Dr",
              city: "Wilmington",
              state: "DE",
              zip: "19808",
            },
          },
        ],
        fiscal_year_end: "12-31",
        company_address: {
          street: "251 Little Falls Dr",
          city: "Wilmington",
          state: "DE",
          zip: "19808",
        },
      };
  }
}

function meetingTypeForBody(body: ApiRecord): string {
  return String(body.body_type) === "llc_member_vote" ? "member_meeting" : "board_meeting";
}

export async function demoCommand(opts: DemoOptions): Promise<void> {
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);
  const resolver = new ReferenceResolver(client, cfg);
  const scenario = opts.scenario ?? "startup";

  try {
    if (opts.minimal) {
      const result = await withSpinner("Loading", () => client.seedDemo({
        name: opts.name,
        scenario,
      }));
      if (opts.json) {
        printJson(result);
        return;
      }
      printSuccess("Minimal demo seeded.");
      printJson(result);
      return;
    }

    const formationPayload = scenarioConfig(opts.name, scenario);
    const created = await withSpinner("Creating demo entity", () =>
      client.createFormationWithCapTable(formationPayload),
    );
    const entityId = String(created.entity_id ?? created.formation_id ?? "");
    if (!entityId) {
      throw new Error("demo formation did not return an entity_id");
    }

    await resolver.stabilizeRecord("entity", created as ApiRecord);
    resolver.rememberFromRecord("entity", created as ApiRecord);

    const activation = await withSpinner("Activating formation", () =>
      activateFormationEntity(client, resolver, entityId),
    );

    const agent = await withSpinner("Creating demo agent", () =>
      client.createAgent({
        name: `${opts.name} Operator`,
        entity_id: entityId,
        system_prompt: `Operate ${opts.name} and keep the workspace data clean.`,
        scopes: [],
      }),
    );
    await resolver.stabilizeRecord("agent", agent);
    resolver.rememberFromRecord("agent", agent);

    const bodies = await client.listGovernanceBodies(entityId);
    await resolver.stabilizeRecords("body", bodies as ApiRecord[], entityId);

    let meeting: ApiRecord | undefined;
    if (bodies.length > 0) {
      meeting = await client.scheduleMeeting({
        entity_id: entityId,
        body_id: (bodies[0] as ApiRecord).body_id,
        meeting_type: meetingTypeForBody(bodies[0] as ApiRecord),
        title: "Demo kickoff meeting",
        agenda_item_titles: [
          "Approve demo operating budget",
          "Review founder vesting and cap table",
        ],
      });
      await resolver.stabilizeRecord("meeting", meeting, entityId);
      resolver.rememberFromRecord("meeting", meeting, entityId);
    }

    const workItem = await client.createWorkItem(entityId, {
      title: "Review demo workspace",
      description: "Check documents, governance, cap table, and treasury setup.",
      category: "ops",
      created_by_actor: {
        actor_type: "agent",
        actor_id: String(agent.agent_id),
      },
    });
    await resolver.stabilizeRecord("work_item", workItem, entityId);
    resolver.rememberFromRecord("work_item", workItem, entityId);

    const claimedWorkItem = await client.claimWorkItem(entityId, String(workItem.work_item_id), {
      claimed_by_actor: {
        actor_type: "agent",
        actor_id: String(agent.agent_id),
      },
    });

    const bankAccount = await client.openBankAccount({
      entity_id: entityId,
      bank_name: "Mercury",
    });
    await resolver.stabilizeRecord("bank_account", bankAccount, entityId);
    resolver.rememberFromRecord("bank_account", bankAccount, entityId);

    const invoice = await client.createInvoice({
      entity_id: entityId,
      customer_name: "Acme Customer",
      amount_cents: 250000,
      due_date: "2026-04-01",
      description: "Demo advisory services",
    });
    await resolver.stabilizeRecord("invoice", invoice, entityId);
    resolver.rememberFromRecord("invoice", invoice, entityId);

    const contract = await client.generateContract({
      entity_id: entityId,
      template_type: "nda",
      counterparty_name: "Example Counterparty",
      effective_date: "2026-03-12",
      parameters: {},
    });
    await resolver.stabilizeRecord("document", contract, entityId);
    resolver.rememberFromRecord("document", contract, entityId);

    const result: ApiRecord = {
      scenario,
      entity: created,
      activation,
      agent,
      meeting,
      work_item: claimedWorkItem,
      bank_account: bankAccount,
      invoice,
      contract,
    };

    if (opts.json) {
      printJson(result);
      return;
    }

    printSuccess(`Demo environment created for ${opts.name}.`);
    printReferenceSummary("entity", created as ApiRecord, { showReuseHint: true });
    printReferenceSummary("agent", agent, { showReuseHint: true });
    if (meeting) {
      printReferenceSummary("meeting", meeting, { showReuseHint: true });
    }
    printReferenceSummary("work_item", claimedWorkItem, { showReuseHint: true });
    printReferenceSummary("invoice", invoice, { showReuseHint: true });
    printReferenceSummary("bank_account", bankAccount, { showReuseHint: true });
    printJson(result);
  } catch (err) {
    printError(`Failed to seed demo: ${err}`);
    process.exit(1);
  }
}
