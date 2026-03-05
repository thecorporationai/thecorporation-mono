//! Transaction packet model (stored as `execution/packets/{packet_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::ids::{EntityId, IntentId, PacketId, PacketSignatureId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    Transfer,
    Fundraising,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionPacketStatus {
    Drafted,
    ReadyForSignature,
    FullySigned,
    Executable,
    Executed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PacketItem {
    pub item_id: String,
    pub title: String,
    pub document_path: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSignature {
    signature_id: PacketSignatureId,
    signer_identity: String,
    channel: String,
    signed_at: DateTime<Utc>,
}

impl PacketSignature {
    pub fn new(signature_id: PacketSignatureId, signer_identity: String, channel: String) -> Self {
        Self {
            signature_id,
            signer_identity,
            channel,
            signed_at: Utc::now(),
        }
    }

    pub fn signature_id(&self) -> PacketSignatureId {
        self.signature_id
    }
    pub fn signer_identity(&self) -> &str {
        &self.signer_identity
    }
    pub fn channel(&self) -> &str {
        &self.channel
    }
    pub fn signed_at(&self) -> DateTime<Utc> {
        self.signed_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPacket {
    packet_id: PacketId,
    entity_id: EntityId,
    intent_id: IntentId,
    workflow_type: WorkflowType,
    workflow_id: String,
    status: TransactionPacketStatus,
    manifest_hash: String,
    items: Vec<PacketItem>,
    required_signers: Vec<String>,
    signatures: Vec<PacketSignature>,
    evidence_refs: Vec<String>,
    created_at: DateTime<Utc>,
    finalized_at: Option<DateTime<Utc>>,
}

impl TransactionPacket {
    pub fn new(
        packet_id: PacketId,
        entity_id: EntityId,
        intent_id: IntentId,
        workflow_type: WorkflowType,
        workflow_id: String,
        mut items: Vec<PacketItem>,
        mut required_signers: Vec<String>,
    ) -> Self {
        items.sort_by(|a, b| a.item_id.cmp(&b.item_id));
        required_signers.sort();
        required_signers.dedup();

        let manifest_hash = compute_manifest_hash(&items, &required_signers);

        Self {
            packet_id,
            entity_id,
            intent_id,
            workflow_type,
            workflow_id,
            status: TransactionPacketStatus::Drafted,
            manifest_hash,
            items,
            required_signers,
            signatures: Vec::new(),
            evidence_refs: Vec::new(),
            created_at: Utc::now(),
            finalized_at: None,
        }
    }

    pub fn mark_ready_for_signature(&mut self) {
        self.status = TransactionPacketStatus::ReadyForSignature;
    }

    pub fn record_signature(
        &mut self,
        signature_id: PacketSignatureId,
        signer_identity: String,
        channel: String,
    ) {
        if self
            .signatures
            .iter()
            .any(|s| s.signer_identity() == signer_identity)
        {
            return;
        }
        self.signatures
            .push(PacketSignature::new(signature_id, signer_identity, channel));
        if self.required_signers.iter().all(|required| {
            self.signatures
                .iter()
                .any(|s| s.signer_identity() == required)
        }) {
            self.status = TransactionPacketStatus::FullySigned;
        }
    }

    pub fn mark_executable(&mut self) {
        self.status = TransactionPacketStatus::Executable;
    }

    pub fn mark_executed(&mut self) {
        self.status = TransactionPacketStatus::Executed;
        self.finalized_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self) {
        self.status = TransactionPacketStatus::Failed;
    }

    pub fn add_evidence_ref(&mut self, reference: String) {
        if reference.trim().is_empty() {
            return;
        }
        if !self.evidence_refs.iter().any(|r| r == &reference) {
            self.evidence_refs.push(reference);
        }
    }

    pub fn packet_id(&self) -> PacketId {
        self.packet_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn intent_id(&self) -> IntentId {
        self.intent_id
    }
    pub fn workflow_type(&self) -> WorkflowType {
        self.workflow_type
    }
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
    pub fn status(&self) -> TransactionPacketStatus {
        self.status
    }
    pub fn manifest_hash(&self) -> &str {
        &self.manifest_hash
    }
    pub fn items(&self) -> &[PacketItem] {
        &self.items
    }
    pub fn required_signers(&self) -> &[String] {
        &self.required_signers
    }
    pub fn signatures(&self) -> &[PacketSignature] {
        &self.signatures
    }
    pub fn evidence_refs(&self) -> &[String] {
        &self.evidence_refs
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn finalized_at(&self) -> Option<DateTime<Utc>> {
        self.finalized_at
    }
}

fn compute_manifest_hash(items: &[PacketItem], required_signers: &[String]) -> String {
    let payload = serde_json::json!({
        "items": items,
        "required_signers": required_signers,
    });
    let bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_packet() -> TransactionPacket {
        TransactionPacket::new(
            PacketId::new(),
            EntityId::new(),
            IntentId::new(),
            WorkflowType::Transfer,
            "wf-1".to_owned(),
            vec![
                PacketItem {
                    item_id: "b".to_owned(),
                    title: "B".to_owned(),
                    document_path: "docs/b.md".to_owned(),
                    required: true,
                },
                PacketItem {
                    item_id: "a".to_owned(),
                    title: "A".to_owned(),
                    document_path: "docs/a.md".to_owned(),
                    required: true,
                },
            ],
            vec!["ceo".to_owned(), "board".to_owned()],
        )
    }

    #[test]
    fn manifest_hash_is_deterministic() {
        let p1 = make_packet();
        let p2 = make_packet();
        assert_eq!(p1.manifest_hash(), p2.manifest_hash());
    }

    #[test]
    fn fully_signed_after_all_required() {
        let mut packet = make_packet();
        packet.mark_ready_for_signature();
        packet.record_signature(
            PacketSignatureId::new(),
            "ceo".to_owned(),
            "portal".to_owned(),
        );
        assert_eq!(packet.status(), TransactionPacketStatus::ReadyForSignature);
        packet.record_signature(
            PacketSignatureId::new(),
            "board".to_owned(),
            "portal".to_owned(),
        );
        assert_eq!(packet.status(), TransactionPacketStatus::FullySigned);
    }
}
