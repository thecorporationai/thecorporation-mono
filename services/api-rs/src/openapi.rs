//! OpenAPI specification generation via utoipa.
//!
//! Composes per-module `OpenApi` structs into a single spec.
//! Served at `GET /v1/openapi.json`.

use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::OpenApi;

use crate::routes;

/// Build the complete OpenAPI spec by merging per-module specs.
pub fn openapi_spec() -> utoipa::openapi::OpenApi {
    let mut doc = ApiDoc::openapi();

    // Merge each route module's OpenApi spec.
    let modules: Vec<utoipa::openapi::OpenApi> = vec![
        routes::formation::FormationApi::openapi(),
        routes::equity::EquityApi::openapi(),
        routes::governance::GovernanceApi::openapi(),
        routes::treasury::TreasuryApi::openapi(),
        routes::contacts::ContactsApi::openapi(),
        routes::execution::ExecutionApi::openapi(),
        routes::branches::BranchesApi::openapi(),
        routes::auth::AuthApi::openapi(),
        routes::agents::AgentsApi::openapi(),
        routes::compliance::ComplianceApi::openapi(),
        routes::admin::AdminApi::openapi(),
        routes::secrets_proxy::SecretsProxyApi::openapi(),
        routes::llm_proxy::LlmProxyApi::openapi(),
        routes::secret_proxies::SecretProxiesApi::openapi(),
        routes::agent_executions::AgentExecutionsApi::openapi(),
        routes::work_items::WorkItemsApi::openapi(),
    ];

    for module_doc in modules {
        doc.merge(module_doc);
    }

    // Add Bearer auth security scheme.
    let components = doc.components.get_or_insert_with(Default::default);
    components.add_security_scheme(
        "bearer_auth",
        SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
    );

    doc
}

/// Root document with metadata and tags only (paths/schemas come from module merges).
#[derive(OpenApi)]
#[openapi(
    info(
        title = "The Corporation API",
        version = "1.0.0",
        description = "Git-backed corporate operations platform API"
    ),
    security(("bearer_auth" = [])),
    tags(
        (name = "formation", description = "Entity formation and document management"),
        (name = "equity", description = "Canonical cap table, instruments, rounds, and conversions"),
        (name = "governance", description = "Bodies, seats, meetings, and votes"),
        (name = "treasury", description = "Accounts, journal entries, invoices, and banking"),
        (name = "contacts", description = "Contact management"),
        (name = "execution", description = "Intents, obligations, and receipts"),
        (name = "branches", description = "Git branch management"),
        (name = "auth", description = "Authentication and API keys"),
        (name = "agents", description = "Agent management"),
        (name = "compliance", description = "Tax filings and deadlines"),
        (name = "admin", description = "Administration and system health"),
        (name = "secrets_proxy", description = "Secret token resolution"),
        (name = "llm_proxy", description = "LLM proxy"),
        (name = "secret_proxies", description = "Secret proxy configurations"),
        (name = "agent_executions", description = "Agent execution queue"),
        (name = "work_items", description = "Long-term work item coordination"),
    ),
)]
struct ApiDoc;
