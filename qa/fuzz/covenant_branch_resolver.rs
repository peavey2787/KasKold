#![no_main]

use libfuzzer_sys::fuzz_target;
use shared_signer::covenant_branch::resolve_covenant_branches;

fuzz_target!(|data: &[u8]| {
    if let Ok(resolved) = resolve_covenant_branches(data) {
        let mut bindings = Vec::with_capacity(resolved.len());
        for position in 0..resolved.len() {
            let position = u8::try_from(position).expect("resolver result is bounded to u8 positions");
            let binding = resolved
                .key_at(position)
                .expect("every reported resolver position must be addressable");
            // The same public key may intentionally occur on distinct branches.
            // What must never be duplicated is the complete key+branch binding.
            assert!(
                !bindings.contains(&binding),
                "resolver returned a duplicate key/branch binding"
            );
            assert!(binding.if_mask & !binding.decision_mask == 0);
            assert!(binding.matches_selectors(binding.decision_mask, binding.if_mask));
            bindings.push(binding);
        }
        assert!(resolved
            .key_at(u8::try_from(resolved.len()).unwrap_or(u8::MAX))
            .is_err());
    }
});
