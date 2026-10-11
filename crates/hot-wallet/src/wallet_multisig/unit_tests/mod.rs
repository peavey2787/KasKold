use super::*;

const MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const KPUB_A: &str = "kpub1:038f332e03405ab68380000000f0453f0894cc8c84ebf6e6208e0c7916e9ddbd14919f9bbb92b0690b4e353392020327c7136972883eab5a7722ec3d4302f888804ecce61658ae962a2c56bb7571";
const KPUB_B: &str = "kpub1:038f332e03a7457270800000002908be01d75735944f29befbdbcd173ab00df2d44c6d5ab51a839413fda90cbf035b986b584de244f5d6a1939192f676a9f2992a63b0f43cdc452dcb40d9dd7081";
const KPUB_C: &str = "kpub1:038f332e037a262d628000000037234957045cdffdb77c3fdcb25649de3326bd8eb6459276a96ba0b14b99cf05034cf53938d64f4a3d4554e18e9ec0d113b251be5b974386807e95a58530e837e1";

fn wallet() -> HotWallet {
    HotWallet::restore(MNEMONIC, "").unwrap()
}

#[test]
fn static_descriptor_address_is_independent_of_key_order_and_matches_watchers() {
    let first = "11".repeat(32);
    let second = "22".repeat(32);
    let mut wallet = wallet();
    let sorted = wallet
        .import_multisig_descriptor(
            &format!("multi(1,{first},{second})"),
            KaspaNetwork::Mainnet,
            0,
            0,
        )
        .unwrap();
    let reversed = wallet
        .import_multisig_descriptor(
            &format!("multi(1,{second},{first})"),
            KaspaNetwork::Mainnet,
            0,
            0,
        )
        .unwrap();
    assert_eq!(sorted.address, reversed.address);

    // The same descriptor as Kaspa Portal's watch-only multisig model reads it.
    use offline_signer::derivation::multisig::{build_redeem_script, MultisigDescriptor};
    let watcher = MultisigDescriptor::parse(&format!("multi(1,{second},{first})")).unwrap();
    let script = build_redeem_script(1, &watcher.public_keys_at(0, 0, 0).unwrap()).unwrap();
    assert_eq!(
        reversed.address,
        offline_signer::address::script_to_p2sh_address(&script, "kaspa")
    );
}

#[test]
fn multisig_locations_need_a_wallet_chain_non_hardened_index_and_known_network() {
    assert!(validate_multisig_location(KaspaNetwork::Mainnet, 1, 0x7fff_ffff).is_ok());
    for (network, chain, index) in [
        (KaspaNetwork::Mainnet, 2, 0),
        (KaspaNetwork::Mainnet, 0, 0x8000_0000),
        (KaspaNetwork::Unknown, 0, 0),
    ] {
        assert!(matches!(
            validate_multisig_location(network, chain, index),
            Err(HotWalletError::InvalidToolInput)
        ));
    }
}

#[test]
fn multisig_requests_bound_participants_and_threshold() {
    let ok = |threshold, count| {
        validate_multisig_request(threshold, count, KaspaNetwork::Mainnet, 0, 0).is_ok()
    };
    assert!(ok(1, 2));
    assert!(ok(2, 2));
    assert!(ok(1, MAX_MULTISIG_KEYS));
    assert!(!ok(1, 1));
    assert!(!ok(1, MAX_MULTISIG_KEYS + 1));
    assert!(!ok(0, 2));
    assert!(!ok(3, 2));
    assert!(validate_multisig_request(1, 2, KaspaNetwork::Unknown, 0, 0).is_err());
}

#[test]
fn imported_hd_descriptors_are_remembered_once_per_wallet_up_to_capacity() {
    let mut wallet = wallet();
    assert!(wallet
        .multisig_configs()
        .iter()
        .all(|config| !config.active));
    let descriptor = format!("multi_hd45(1,{KPUB_A},{KPUB_B})");
    let first = wallet
        .import_multisig_descriptor(&descriptor, KaspaNetwork::Mainnet, 0, 0)
        .unwrap();
    let active = |wallet: &HotWallet| {
        wallet
            .multisig_configs()
            .iter()
            .filter(|config| config.active)
            .count()
    };
    assert_eq!(active(&wallet), 1);
    // Re-importing the same multisig updates its slot instead of adding one.
    let again = wallet
        .import_multisig_descriptor(&descriptor, KaspaNetwork::Mainnet, 0, 1)
        .unwrap();
    assert_ne!(first.address, again.address);
    assert_eq!(active(&wallet), 1);

    let mut imported = 1;
    for other in [
        format!("multi_hd45(2,{KPUB_A},{KPUB_B})"),
        format!("multi_hd45(1,{KPUB_A},{KPUB_C})"),
        format!("multi_hd45(1,{KPUB_B},{KPUB_C})"),
    ] {
        match wallet.import_multisig_descriptor(&other, KaspaNetwork::Mainnet, 0, 0) {
            Ok(_) => imported += 1,
            Err(error) => {
                assert!(matches!(error, HotWalletError::MultisigInvalid));
                break;
            }
        }
    }
    assert_eq!(active(&wallet), imported);
    assert!(imported < 4, "the multisig store is bounded");
}
