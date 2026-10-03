//! Detection and dispatch of specialized covenant witness routes.

use super::{
    bind_commit_reveal, bind_merkle, bind_oracle_v1, bind_private_swap, exactly_one_signature,
    Input, Map, RouteKind, Signature, SpecializedRoute, SpecializedWitness, Value,
};

pub(crate) fn bind_specialized_witnesses(
    global: &Map<String, Value>,
    source_inputs: &[Value],
    transaction: &mut super::super::compact::Transaction,
) -> Result<(), String> {
    if global
        .get("covenantBranch")
        .is_some_and(|value| !value.is_null())
    {
        return Err(
            "global.covenantBranch is forbidden at the verified boundary; covenantExecution is authoritative"
                .to_string(),
        );
    }
    reject_all_specialized(global.get("proprietaries"), "global.proprietaries")?;
    if source_inputs.len() != transaction.inputs.len() {
        return Err("specialized witness input count mismatch".to_string());
    }

    for (index, (source, input)) in source_inputs
        .iter()
        .zip(transaction.inputs.iter_mut())
        .enumerate()
    {
        let object = source
            .as_object()
            .ok_or_else(|| format!("input[{index}] not object"))?;
        let proprietary = optional_object(
            object.get("proprietaries"),
            &format!("input[{index}].proprietaries"),
        )?;
        let Some(proprietary) = proprietary else {
            continue;
        };
        let route = detect_route(proprietary, index)?;
        let Some(route) = route else {
            continue;
        };
        let specialized = bind_route(index, route, proprietary, input)?;
        input.specialized_witness = Some(specialized);
    }
    Ok(())
}

pub(crate) fn optional_object<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<Option<&'a Map<String, Value>>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(value)) => Ok(Some(value)),
        Some(_) => Err(format!("{label} must be an object or null")),
    }
}

pub(crate) fn detect_route(
    fields: &Map<String, Value>,
    index: usize,
) -> Result<Option<RouteKind>, String> {
    let routes = route_presence(fields);
    reject_mixed_routes(routes, index)?;
    reject_unsupported_route_fields(fields, index)?;
    Ok(route_from_presence(routes))
}

pub(crate) fn route_presence(fields: &Map<String, Value>) -> [bool; 4] {
    [
        fields.contains_key("privateSwapClaim"),
        fields.contains_key("oracleV1Claim") || fields.contains_key("oracleV1Signature"),
        fields.contains_key("commitPartA") || fields.contains_key("commitPartB"),
        fields.contains_key("merkleProof") || fields.contains_key("merkleDestSpk"),
    ]
}

pub(crate) fn reject_mixed_routes(routes: [bool; 4], index: usize) -> Result<(), String> {
    if routes.into_iter().filter(|value| *value).count() > 1 {
        return Err(format!(
            "input[{index}] mixes multiple specialized covenant routes"
        ));
    }
    Ok(())
}

pub(crate) fn reject_unsupported_route_fields(
    fields: &Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    for key in fields.keys() {
        if crate::wire::pskt_schema::is_specialized_covenant_routing_field(key)
            && !crate::wire::pskt_schema::is_supported_specialized_covenant_routing_field(key)
        {
            return Err(format!(
                "input[{index}].proprietaries.{key} has no typed verified witness plan"
            ));
        }
    }
    Ok(())
}

pub(crate) fn route_from_presence(routes: [bool; 4]) -> Option<RouteKind> {
    let [private, oracle, commit, merkle] = routes;
    if private {
        Some(RouteKind::PrivateSwap)
    } else if oracle {
        Some(RouteKind::OracleV1)
    } else if commit {
        Some(RouteKind::CommitReveal)
    } else if merkle {
        Some(RouteKind::Merkle)
    } else {
        None
    }
}

pub(crate) fn reject_all_specialized(value: Option<&Value>, label: &str) -> Result<(), String> {
    let Some(object) = optional_object(value, label)? else {
        return Ok(());
    };
    if let Some(field) = object
        .keys()
        .find(|key| crate::wire::pskt_schema::is_specialized_covenant_routing_field(key))
    {
        return Err(format!(
            "{label}.{field} is input-scoped specialized covenant metadata"
        ));
    }
    Ok(())
}

pub(crate) fn bind_route(
    index: usize,
    route: RouteKind,
    fields: &Map<String, Value>,
    input: &Input,
) -> Result<SpecializedWitness, String> {
    let (mask, truth, signature) = validate_route_binding(index, input)?;
    let (route_id, script) = dispatch_route(index, route, fields, input, mask, truth, signature)?;
    Ok(SpecializedWitness {
        route: route_id,
        signature_script: script,
        supplied_mask: mask,
        supplied_true_mask: truth,
    })
}

pub(crate) fn validate_route_binding(
    index: usize,
    input: &Input,
) -> Result<(u16, u16, &Signature), String> {
    if input.redeem.is_empty() {
        return Err(format!(
            "input[{index}] specialized covenant is missing redeemScript"
        ));
    }
    let (mask, truth) = input.covenant_execution.ok_or_else(|| {
        format!("input[{index}] specialized covenant is missing covenantExecution")
    })?;
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| format!("input[{index}] invalid covenant branch structure: {error:?}"))?;
    validate_selector_assignment(index, mask, truth, branches.selector_mask())?;
    let signature = exactly_one_signature(index, &input.signatures)?;
    let binding = branches
        .key_at(signature.position)
        .map_err(|_| format!("input[{index}] specialized signature is not branch-bound"))?;
    if !binding.matches_selectors(mask, truth) {
        return Err(format!(
            "input[{index}] specialized signature does not belong to covenantExecution"
        ));
    }
    Ok((mask, truth, signature))
}

pub(crate) fn validate_selector_assignment(
    index: usize,
    mask: u16,
    truth: u16,
    expected_mask: u16,
) -> Result<(), String> {
    if truth & !mask != 0 || mask != expected_mask {
        return Err(format!(
            "input[{index}] covenantExecution is not a complete selector assignment"
        ));
    }
    Ok(())
}

pub(crate) fn dispatch_route(
    index: usize,
    route: RouteKind,
    fields: &Map<String, Value>,
    input: &Input,
    mask: u16,
    truth: u16,
    signature: &Signature,
) -> Result<(SpecializedRoute, Vec<u8>), String> {
    match route {
        RouteKind::PrivateSwap => bind_private_swap(index, fields, input, mask, truth, signature),
        RouteKind::OracleV1 => bind_oracle_v1(index, fields, input, mask, truth, signature),
        RouteKind::CommitReveal => bind_commit_reveal(index, fields, input, mask, truth, signature),
        RouteKind::Merkle => bind_merkle(index, fields, input, mask, truth, signature),
    }
}
