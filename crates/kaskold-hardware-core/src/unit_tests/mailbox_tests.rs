use crate::runtime::worker::{CancelAction, CrossCoreMailbox, ReserveError, WorkerLifecycle};

#[test]
fn mailbox_round_trip_progress_result_and_default_paths_are_exact() {
    let mailbox = CrossCoreMailbox::<u32, u64>::default();
    assert_eq!(mailbox.reserve(), Err(ReserveError::Unavailable));
    assert!(mailbox.is_idle());
    mailbox.mark_ready();
    let generation = mailbox.reserve().expect("reserve");
    assert_eq!(mailbox.reserve(), Err(ReserveError::Busy));
    assert_eq!(mailbox.progress(generation), 0);
    assert!(mailbox.publish_job(generation, 0x1122_3344, 7));
    assert_eq!(mailbox.progress(generation), 7);
    assert_eq!(mailbox.take_job(), Some(0x1122_3344));
    mailbox.set_progress(63);
    assert_eq!(mailbox.progress(generation), 63);
    assert!(mailbox.publish_result(generation, 0x5566_7788_99aa_bbcc));
    assert_eq!(mailbox.take_result(generation), Some(0x5566_7788_99aa_bbcc));
    assert_eq!(mailbox.take_result(generation), None);
    assert!(mailbox.is_idle());
    assert_eq!(mailbox.progress(generation), 0);
}

#[test]
fn mailbox_rejects_stale_publication_and_scrubs_cancelled_ownership_states() {
    let mailbox = CrossCoreMailbox::<std::vec::Vec<u8>, std::vec::Vec<u8>>::new();
    mailbox.mark_ready();

    let stale = mailbox.reserve().expect("stale queued");
    mailbox.cancel(stale);
    assert!(mailbox.is_idle());
    assert!(!mailbox.publish_job(stale, std::vec![1, 2, 3], 1));
    assert!(mailbox.is_idle());

    let queued = mailbox.reserve().expect("queued retry");
    assert!(mailbox.publish_job(queued, std::vec![4, 5, 6], 2));
    mailbox.cancel(queued);
    assert!(mailbox.is_idle());
    assert_eq!(mailbox.take_job(), None);

    let busy = mailbox.reserve().expect("busy");
    assert!(mailbox.publish_job(busy, std::vec![7], 3));
    assert_eq!(mailbox.take_job(), Some(std::vec![7]));
    mailbox.cancel(busy);
    assert!(!mailbox.publish_result(busy, std::vec![8]));
    assert!(mailbox.is_idle());

    let completed = mailbox.reserve().expect("completed");
    assert!(mailbox.publish_job(completed, std::vec![9], 4));
    assert_eq!(mailbox.take_job(), Some(std::vec![9]));
    assert!(mailbox.publish_result(completed, std::vec![10]));
    mailbox.cancel(completed);
    assert!(mailbox.is_idle());
    assert_eq!(mailbox.take_result(completed), None);
}

#[test]
fn mailbox_discard_cancel_active_and_wrong_generation_result_paths_are_covered() {
    let mailbox = CrossCoreMailbox::<u8, u8>::new();
    mailbox.cancel_active();
    mailbox.mark_ready();

    let generation = mailbox.reserve().expect("reserve");
    assert!(mailbox.publish_job(generation, 1, 5));
    assert_eq!(mailbox.take_job(), Some(1));
    mailbox.cancel(generation);
    assert!(!mailbox.publish_result(generation, 2));
    assert!(mailbox.is_idle());

    let generation = mailbox.reserve().expect("reserve completed");
    assert!(mailbox.publish_job(generation, 3, 6));
    assert_eq!(mailbox.take_job(), Some(3));
    assert!(mailbox.publish_result(generation, 4));
    mailbox.discard_completed();
    assert!(mailbox.is_idle());
    mailbox.discard_completed();

    let generation = mailbox.reserve().expect("reserve cancel active");
    assert!(mailbox.publish_job(generation, 5, 7));
    mailbox.cancel_active();
    assert!(mailbox.is_idle());
    assert_eq!(mailbox.progress(generation.wrapping_add(1)), 0);
}

#[test]
fn a_second_publish_destroys_the_job_and_the_worker_does_not_wedge() {
    let mailbox = CrossCoreMailbox::<std::vec::Vec<u8>, u8>::new();
    mailbox.mark_ready();
    let generation = mailbox.reserve().expect("reserve");
    assert!(mailbox.publish_job(generation, std::vec![1], 1));
    assert!(!mailbox.publish_job(generation, std::vec![2], 2));
    assert!(!mailbox.is_idle());

    assert_eq!(mailbox.take_job(), None);
    assert!(mailbox.is_idle());
    assert_eq!(mailbox.progress(generation), 0);
    assert!(mailbox.reserve().is_ok());
}

#[test]
fn lifecycle_reports_progress_only_to_the_generation_that_owns_it() {
    let lifecycle = WorkerLifecycle::default();
    lifecycle.mark_ready();
    let generation = lifecycle.reserve().expect("reserve");
    assert!(lifecycle.publish_ready(generation, 9));
    assert_eq!(lifecycle.progress(generation), 9);
    assert_eq!(lifecycle.progress(generation.wrapping_add(1)), 0);
}

#[test]
fn cancelling_an_idle_or_finished_generation_changes_nothing() {
    let lifecycle = WorkerLifecycle::new();
    lifecycle.mark_ready();
    let generation = lifecycle.reserve().expect("reserve");
    assert_eq!(lifecycle.cancel(generation), CancelAction::DropQueuedJob);
    lifecycle.finish_cancelled();

    let generation = lifecycle.reserve().expect("reserve again");
    assert!(lifecycle.publish_ready(generation, 0));
    assert!(lifecycle.claim_ready());
    assert!(lifecycle.begin_publish(generation));
    assert!(lifecycle.finish_publish());
    assert!(lifecycle.claim_result(generation));
    assert_eq!(lifecycle.cancel(generation), CancelAction::None);
    lifecycle.finish_result_take();
    assert_eq!(lifecycle.cancel(generation), CancelAction::None);
    assert!(lifecycle.is_idle());
}

#[test]
fn a_claimed_job_with_an_empty_slot_returns_the_lifecycle_to_idle() {
    let lifecycle = WorkerLifecycle::new();
    lifecycle.mark_ready();
    let generation = lifecycle.reserve().expect("reserve");
    assert!(lifecycle.publish_ready(generation, 4));
    lifecycle.abort_missing_job();
    assert!(!lifecycle.is_idle());
    assert!(lifecycle.claim_ready());
    lifecycle.abort_missing_job();
    assert!(lifecycle.is_idle());
    assert_eq!(lifecycle.progress(generation), 0);
}
