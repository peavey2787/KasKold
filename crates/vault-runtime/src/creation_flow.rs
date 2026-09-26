//! Presentation-safe projection of the shared wallet-creation flow contract.

pub fn creation_flow_json() -> String {
    let stages = shared_signer::creation_flow::CREATION_STAGE_ORDER
        .iter()
        .map(|stage| stage.name())
        .collect::<Vec<_>>();
    serde_json::json!({
        "stages": stages,
        "diceRollTargets": shared_signer::creation_flow::DICE_ROLL_TARGETS,
        "touchEntropyTarget": shared_signer::creation_flow::TOUCH_ENTROPY_TARGET,
    })
    .to_string()
}
