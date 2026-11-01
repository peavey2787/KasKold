//! User journeys through the real firmware. Each runs in its own process because the
//! firmware owns process-wide singletons, exactly as it owns the chip.
#[cfg(not(feature = "workflow-test-auto"))]
mod explore;
#[cfg(not(feature = "workflow-test-auto"))]
mod journeys;
#[cfg(feature = "workflow-test-auto")]
mod workflow_gate;
