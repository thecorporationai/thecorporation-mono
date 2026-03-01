use api_rs::domain::execution::types::AuthorityTier;
use api_rs::domain::governance::policy_engine::{
    AuthoritySource, PolicyConflict, PolicyDecision, apply_conflict_fail_closed, evaluate_intent,
};
use api_rs::domain::governance::policy_lattice::is_monotone_restriction_step;
use api_rs::domain::governance::proof_obligations::{evaluate_proof_obligations, verify_decision};
use proptest::prelude::*;
use serde_json::json;

fn metadata_strategy() -> impl Strategy<Value = serde_json::Value> {
    let lane_id = prop::option::of(prop_oneof![
        Just("lane-3.1-nda".to_owned()),
        Just("lane-3.3-renewal".to_owned()),
        Just("lane-3.3-insurance".to_owned()),
    ]);
    let template = prop::option::of(any::<bool>());
    let reversible = prop::option::of(any::<bool>());
    let mods = prop::collection::vec(
        prop_oneof![
            Just("indemnification".to_owned()),
            Just("governing_law".to_owned()),
            Just("ip_assignment".to_owned()),
            Just("exclusivity".to_owned()),
            Just("economics".to_owned()),
        ],
        0..4,
    );
    let price = prop::option::of(0_f64..30_f64);
    let premium = prop::option::of(0_f64..30_f64);
    (lane_id, template, reversible, mods, price, premium).prop_map(
        |(lane_id, template, reversible, modifications, price, premium)| {
            let mut metadata = json!({});
            if let Some(v) = lane_id {
                metadata["laneId"] = json!(v);
            }
            if let Some(v) = template {
                metadata["templateApproved"] = json!(v);
            }
            if let Some(v) = reversible {
                metadata["isReversible"] = json!(v);
            }
            if !modifications.is_empty() {
                metadata["modifications"] = json!(modifications);
            }
            if price.is_some() || premium.is_some() {
                metadata["context"] = json!({});
                if let Some(v) = price {
                    metadata["context"]["priceIncreasePercent"] = json!(v);
                }
                if let Some(v) = premium {
                    metadata["context"]["premiumIncreasePercent"] = json!(v);
                }
            }
            metadata
        },
    )
}

proptest! {
    #[test]
    fn governance_law_props_deterministic(metadata in metadata_strategy()) {
        let d1 = evaluate_intent("execute_standard_form_agreement", &metadata);
        let d2 = evaluate_intent("execute_standard_form_agreement", &metadata);

        prop_assert_eq!(d1.tier(), d2.tier());
        prop_assert_eq!(d1.allowed(), d2.allowed());
        prop_assert_eq!(d1.requires_approval(), d2.requires_approval());
        prop_assert_eq!(d1.blockers(), d2.blockers());
        prop_assert_eq!(d1.escalation_reasons(), d2.escalation_reasons());
        prop_assert_eq!(d1.clause_refs(), d2.clause_refs());
    }

    #[test]
    fn governance_law_props_monotone_template_disapproval(
        modifications in prop::collection::vec(
            prop_oneof![Just("exclusivity".to_owned()), Just("indemnification".to_owned())],
            0..3
        )
    ) {
        let base = json!({
            "laneId": "lane-3.1-nda",
            "templateApproved": true,
            "modifications": modifications,
        });
        let stricter = json!({
            "laneId": "lane-3.1-nda",
            "templateApproved": false,
            "modifications": modifications,
        });

        let d_base = evaluate_intent("execute_standard_form_agreement", &base);
        let d_stricter = evaluate_intent("execute_standard_form_agreement", &stricter);
        prop_assert!(
            is_monotone_restriction_step(&d_base, &d_stricter),
            "base={:?} stricter={:?}",
            d_base,
            d_stricter
        );
    }
}

#[test]
fn governance_law_props_conflicts_fail_closed() {
    let decision = PolicyDecision::new(
        AuthorityTier::Tier2,
        true,
        Vec::new(),
        Vec::new(),
        vec!["delegation.authority_tiers".to_owned()],
        Vec::new(),
        vec![PolicyConflict {
            higher_source: AuthoritySource::Law,
            lower_source: AuthoritySource::Heuristic,
            reason: "contradiction".to_owned(),
        }],
        Some(AuthoritySource::Heuristic),
    );

    let decision = apply_conflict_fail_closed(decision);
    assert!(!decision.allowed());
}

proptest! {
    #[test]
    fn governance_law_props_invariant_allowed_eq_no_blockers(metadata in metadata_strategy()) {
        let capabilities = [
            "execute_standard_form_agreement",
            "pay_recurring_obligation",
            "engage_contractor",
            "routine_correspondence",
            "issue_equity",
            "totally.unknown.intent",
        ];
        for cap in capabilities {
            let d = evaluate_intent(cap, &metadata);
            prop_assert_eq!(
                d.allowed(),
                d.blockers().is_empty(),
                "allowed/blockers invariant violated for {}: allowed={}, blockers={:?}",
                cap, d.allowed(), d.blockers()
            );
            let expected_approval = d.tier() > AuthorityTier::Tier1;
            prop_assert_eq!(
                d.requires_approval(),
                expected_approval,
                "tier/approval invariant violated for {}: tier={:?}, requires_approval={}",
                cap, d.tier(), d.requires_approval()
            );
        }
    }

    #[test]
    fn governance_law_props_verified_decision_satisfies_all_proofs(metadata in metadata_strategy()) {
        let capabilities = [
            "execute_standard_form_agreement",
            "pay_recurring_obligation",
            "engage_contractor",
            "routine_correspondence",
            "issue_equity",
            "totally.unknown.intent",
        ];
        for cap in capabilities {
            let d = evaluate_intent(cap, &metadata);
            let verified = verify_decision(d);
            let report = evaluate_proof_obligations(verified.decision());
            prop_assert!(
                report.passed(),
                "VerifiedDecision should satisfy all proofs for {}: violations={:?}",
                cap,
                report.violations.iter().map(|v| &v.code).collect::<Vec<_>>()
            );
        }
    }
}
