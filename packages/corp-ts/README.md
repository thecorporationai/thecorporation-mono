# @thecorporation/corp

TypeScript client, CLI wrapper, and binary manager for the [TheCorporation](https://www.thecorporation.ai) corporate governance platform.

## Install

```bash
npm install @thecorporation/corp
```

Or run directly without installing:

```bash
npx @thecorporation/corp entities list --local
```

Platform-specific binaries (`corp-server`, `corp-cli`) are installed automatically via optional dependencies.

## Quick start

```ts
import { CorpClient } from "@thecorporation/corp";

const client = new CorpClient("http://localhost:8000", "corp_...");

// Create a Delaware C-Corp
const entity = await client.entities.create({
  legal_name: "Acme Corp",
  entity_type: "c_corp",
  jurisdiction: "DE",
});

// Advance formation
await client.formation.advance(entity.entity_id);

// Sign documents
const docs = await client.formation.listDocuments(entity.entity_id);
for (const doc of docs) {
  await client.formation.signDocument(doc.document_id, {
    signer_name: "Jane Doe",
    signer_role: "Incorporator",
    signer_email: "jane@acme.com",
    signature_text: "/s/ Jane Doe",
    consent_text: "I consent",
  });
}
```

## Subpath exports

| Import | Contents |
|--------|----------|
| `@thecorporation/corp` | Everything: types, client, server helpers, CLI wrapper |
| `@thecorporation/corp/client` | `CorpClient` only (browser-safe, uses `fetch`) |
| `@thecorporation/corp/server` | Binary resolution, `startServer()`, `runCli()` |
| `@thecorporation/corp/cli` | Typed `Corp` CLI wrapper class |

## API domains

`CorpClient` exposes typed sub-clients for every domain:

| Sub-client | Operations |
|------------|------------|
| `client.entities` | `list()`, `create()`, `get()`, `dissolve()` |
| `client.formation` | `advance()`, `listDocuments()`, `signDocument()`, `getFiling()`, `confirmFiling()`, `getTaxProfile()`, `confirmEin()` |
| `client.equity` | `getCapTable()`, `createCapTable()`, `createInstrument()`, `createGrant()`, `issueSafe()`, `convertSafe()`, `createValuation()` |
| `client.governance` | `createBody()`, `createSeat()`, `createMeeting()`, `castVote()`, `quickApprove()`, `writtenConsent()` |
| `client.treasury` | `createInvoice()`, `sendInvoice()`, `createPayment()`, `createPayrollRun()`, `createBankAccount()` |
| `client.contacts` | `list()`, `create()`, `get()`, `update()`, `deactivate()` |
| `client.agents` | `list()`, `create()`, `get()`, `pause()`, `resume()`, `delete()` |
| `client.workItems` | `list()`, `create()`, `get()`, `claim()`, `complete()`, `cancel()` |
| `client.admin` | `listApiKeys()`, `createApiKey()`, `revokeApiKey()` |

## Server management

Start a local `corp-server` instance programmatically:

```ts
import { startServerAndWait } from "@thecorporation/corp/server";

const server = await startServerAndWait({
  dataDir: "./corp-data",
  port: 8000,
});

// ... use the server ...

server.kill();
```

## CLI wrapper

The `Corp` class wraps the `corp` binary with typed methods:

```ts
import { Corp } from "@thecorporation/corp/cli";

const corp = new Corp({ dataDir: "./corp-data" });

const entity = await corp.form.create("Acme Corp", "c_corp", "DE");
```

## Requirements

- Node.js >= 20
- Platform binaries installed automatically via `optionalDependencies`

## Links

- [Website](https://www.thecorporation.ai)
- [Documentation](https://docs.thecorporation.ai)
- [GitHub](https://github.com/thecorporationai/thecorporation-mono)
- [API Reference](https://docs.thecorporation.ai/api/overview/)

## License

Apache-2.0
