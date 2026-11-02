use super::*;

#[test]
fn owner_paths_choose_selectors_and_only_the_time_path_stamps_a_lock_time() {
    let timed = OwnerSpendPlan {
        redeem_script: {
            let mut cltv = Vec::new();
            crate::protocol::script::push_int(&mut cltv, 500);
            cltv.push(0xb0);
            cltv
        },
        path: SpendPath::Key(0),
        after_timelock: true,
    };
    let (timed_spec, lock_time) = timed
        .spec("covenant", "destination", 1, "empty", "low")
        .expect("timed spec");
    assert_eq!(lock_time, 500);
    assert_eq!(timed_spec.config.lock_time, 500);
    assert_eq!(timed_spec.config.path, SpendPath::Key(0));

    for branch in ["owner", ""] {
        let plain = OwnerSpendPlan::new("51", branch).expect("owner plan");
        let (spec, lock_time) = plain
            .spec("covenant", "destination", 1, "empty", "low")
            .expect("plain spec");
        assert_eq!(lock_time, 0);
        assert_eq!(spec.config.path, SpendPath::Owner);
    }
    assert!(OwnerSpendPlan::new("51", "heir").is_err());
}
