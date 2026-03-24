//! `corp form` — entity formation workflow.

use serde_json::json;

use super::Context;
use crate::output;
use crate::refs::RefKind;

#[derive(clap::Subcommand)]
#[command(long_about = "Entity formation workflow

Lifecycle: pending → documents_generated → documents_signed → filing_submitted → filed → ein_applied → active

Typical workflow:
  corp form create --name \"Anthropic PBC\" --entity-type c_corp --jurisdiction DE
  corp form advance <ID>          # generates documents
  corp form sign <DOC_ID> ...     # sign each document
  corp form advance <ID>          # → documents_signed, then → filing_submitted
  corp form confirm-filing <ID>   # record state acceptance
  corp form advance <ID>          # → filed
  corp form confirm-ein <ID>      # record IRS EIN
  corp form advance <ID>          # → active")]
pub enum FormCommand {
    /// Start a new entity formation. Creates the entity and begins the formation workflow.
    #[command(
        after_help = "Examples:\n  corp form create --name \"Acme Corp\" --entity-type c_corp --jurisdiction DE\n  corp form create --name \"My LLC\" --entity-type llc --jurisdiction WY"
    )]
    Create {
        /// Legal name of the entity
        #[arg(long, help = "Legal name of the entity")]
        name: String,
        /// Entity type: c_corp or llc
        #[arg(long, default_value = "c_corp", value_parser = ["c_corp", "llc"], help = "Entity type: c_corp or llc")]
        entity_type: String,
        /// US state code for jurisdiction of formation (e.g. DE, CA, WY)
        #[arg(
            long,
            default_value = "DE",
            help = "US state code for jurisdiction of formation (e.g. DE, CA, WY)"
        )]
        jurisdiction: String,
    },
    /// Advance the formation state machine to the next stage. Preconditions vary by state.
    Advance {
        /// Entity ID (UUID). Use `corp entities list` to find IDs.
        #[arg(help = "Entity ID (UUID). Use `corp entities list` to find IDs.")]
        entity_id: String,
    },
    /// Show the current formation status and stage for an entity
    Status {
        /// Entity ID (UUID). Use `corp entities list` to find IDs.
        #[arg(help = "Entity ID (UUID). Use `corp entities list` to find IDs.")]
        entity_id: String,
    },
    /// List formation documents
    Documents {
        /// Entity ID (UUID). Use `corp entities list` to find IDs.
        #[arg(help = "Entity ID (UUID). Use `corp entities list` to find IDs.")]
        entity_id: String,
    },
    /// Show a specific document
    Document {
        /// Entity that owns the document
        #[arg(help = "Entity that owns the document")]
        entity_id: String,
        /// Document ID (from `corp form documents`)
        #[arg(help = "Document ID (from `corp form documents`)")]
        document_id: String,
    },
    /// Sign a formation document. All required documents must be signed before advancing.
    Sign {
        /// Document ID to sign (from `corp form documents`)
        #[arg(help = "Document ID to sign (from `corp form documents`)")]
        document_ref: String,
        /// Full legal name of the signer
        #[arg(long, help = "Full legal name of the signer")]
        signer_name: String,
        /// Organizational title (e.g. CEO, Director, Incorporator)
        #[arg(long, help = "Organizational title (e.g. CEO, Director, Incorporator)")]
        signer_role: String,
        /// Email address of the signer (used for duplicate detection)
        #[arg(
            long,
            help = "Email address of the signer (used for duplicate detection)"
        )]
        signer_email: String,
        /// Typed signature (e.g. the signer's name)
        #[arg(long, help = "Typed signature (e.g. the signer's name)")]
        signature_text: String,
        /// Consent statement acknowledged by the signer
        #[arg(long, help = "Consent statement acknowledged by the signer")]
        consent_text: String,
    },
    /// Show the filing record
    Filing {
        /// Entity ID (UUID). Use `corp entities list` to find IDs.
        #[arg(help = "Entity ID (UUID). Use `corp entities list` to find IDs.")]
        entity_id: String,
    },
    /// Record that the state has accepted the filing
    ConfirmFiling {
        /// Entity ID (UUID). Use `corp entities list` to find IDs.
        #[arg(help = "Entity ID (UUID). Use `corp entities list` to find IDs.")]
        entity_id: String,
        /// State-issued confirmation or filing number
        #[arg(long, help = "State-issued confirmation or filing number")]
        confirmation_number: Option<String>,
    },
    /// Show tax profile
    Tax {
        /// Entity ID (UUID). Use `corp entities list` to find IDs.
        #[arg(help = "Entity ID (UUID). Use `corp entities list` to find IDs.")]
        entity_id: String,
    },
    /// Record the Employer Identification Number assigned by the IRS
    ConfirmEin {
        /// Entity ID (UUID). Use `corp entities list` to find IDs.
        #[arg(help = "Entity ID (UUID). Use `corp entities list` to find IDs.")]
        entity_id: String,
        /// IRS Employer Identification Number (format: XX-XXXXXXX)
        #[arg(long, help = "IRS Employer Identification Number (format: XX-XXXXXXX)")]
        ein: String,
    },
}

pub async fn run(cmd: FormCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    match cmd {
        FormCommand::Create {
            name,
            entity_type,
            jurisdiction,
        } => {
            let body = json!({ "legal_name": name, "entity_type": entity_type, "jurisdiction": jurisdiction });
            let value = ctx.client.post("/v1/entities", &body).await?;
            ctx.remember(RefKind::Entity, &value);
            output::print_value(&value, mode);
            output::print_success("Formation started.", mode);
        }
        FormCommand::Advance { entity_id } => {
            let entity_id = ctx.resolve_ref(&entity_id, RefKind::Entity, None)?;
            let value = ctx
                .client
                .post(&format!("/v1/formations/{entity_id}/advance"), &json!({}))
                .await?;
            ctx.remember(RefKind::Entity, &value);
            output::print_value(&value, mode);
            output::print_success("Formation advanced.", mode);
        }
        FormCommand::Status { entity_id } => {
            let entity_id = ctx.resolve_ref(&entity_id, RefKind::Entity, None)?;
            let value = ctx.client.get(&format!("/v1/entities/{entity_id}")).await?;
            if ctx.json {
                output::print_value(&value, mode);
            } else {
                let status = value
                    .get("formation_status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let name = value
                    .get("legal_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&entity_id);
                output::kv("Entity", name, mode);
                output::kv("Formation status", status, mode);
            }
        }
        FormCommand::Documents { entity_id } => {
            let entity_id = ctx.resolve_ref(&entity_id, RefKind::Entity, None)?;
            let value = ctx
                .client
                .get(&format!("/v1/formations/{entity_id}/documents"))
                .await?;
            output::print_value(&value, mode);
        }
        FormCommand::Document {
            entity_id,
            document_id,
        } => {
            let entity_id = ctx.resolve_ref(&entity_id, RefKind::Entity, None)?;
            let document_id = ctx.resolve_ref(&document_id, RefKind::Document, None)?;
            let value = ctx
                .client
                .get(&format!(
                    "/v1/formations/{entity_id}/documents/{document_id}"
                ))
                .await?;
            output::print_value(&value, mode);
        }
        FormCommand::Sign {
            document_ref,
            signer_name,
            signer_role,
            signer_email,
            signature_text,
            consent_text,
        } => {
            let document_ref = ctx.resolve_ref(&document_ref, RefKind::Document, None)?;
            let body = json!({
                "signer_name": signer_name, "signer_role": signer_role,
                "signer_email": signer_email, "signature_text": signature_text,
                "consent_text": consent_text,
            });
            let value = ctx
                .client
                .post(&format!("/v1/documents/{document_ref}/sign"), &body)
                .await?;
            ctx.remember(RefKind::Document, &value);
            output::print_value(&value, mode);
            output::print_success("Document signed.", mode);
        }
        FormCommand::Filing { entity_id } => {
            let entity_id = ctx.resolve_ref(&entity_id, RefKind::Entity, None)?;
            let value = ctx
                .client
                .get(&format!("/v1/formations/{entity_id}/filing"))
                .await?;
            output::print_value(&value, mode);
        }
        FormCommand::ConfirmFiling {
            entity_id,
            confirmation_number,
        } => {
            let entity_id = ctx.resolve_ref(&entity_id, RefKind::Entity, None)?;
            let body = json!({ "confirmation_number": confirmation_number });
            let value = ctx
                .client
                .post(&format!("/v1/formations/{entity_id}/filing/confirm"), &body)
                .await?;
            output::print_value(&value, mode);
            output::print_success("Filing confirmed.", mode);
        }
        FormCommand::Tax { entity_id } => {
            let entity_id = ctx.resolve_ref(&entity_id, RefKind::Entity, None)?;
            let value = ctx
                .client
                .get(&format!("/v1/formations/{entity_id}/tax"))
                .await?;
            output::print_value(&value, mode);
        }
        FormCommand::ConfirmEin { entity_id, ein } => {
            let entity_id = ctx.resolve_ref(&entity_id, RefKind::Entity, None)?;
            let body = json!({ "ein": ein });
            let value = ctx
                .client
                .post(
                    &format!("/v1/formations/{entity_id}/tax/confirm-ein"),
                    &body,
                )
                .await?;
            output::print_value(&value, mode);
            output::print_success("EIN recorded.", mode);
        }
    }
    Ok(())
}
