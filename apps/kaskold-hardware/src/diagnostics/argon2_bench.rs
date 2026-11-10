//! Developer-only Argon2id calibration using the production PSRAM workspace path.
//!
//! Inputs are fixed, public test material. Never use wallet credentials or seed data here.

use offline_signer::crypto::kdf_calibration::{Candidate, CANDIDATES, PASSWORD, PURPOSE, P_COST, SALT, T_COST};
use offline_signer::crypto::password_kdf::PasswordKdfError;
use shared_signer::bytes::zeroize_bytes;

const CPU_CYCLES_PER_MS: u64 = 240_000;
const WATCHDOG_BUDGET_MS: u64 = 25_000;
const PROBE_GRANULARITY: usize = 64 * 1024;
const PROBE_CAP: usize = 8 * 1024 * 1024;

#[inline(never)]
pub(crate) fn run(watchdog_feed: &mut impl FnMut()) -> bool {
    crate::log!("[argon2-bench] fixed non-secret calibration inputs only");
    crate::log!("[argon2-bench] variant=Argon2id version=19 t={} p={}", T_COST, P_COST);
    match crate::services::memory::psram::region() {
        Ok(region) => crate::log!(
            "[argon2-bench] runtime_psram=0x{:08x}..0x{:08x} bytes={}",
            region.start,
            region.end().unwrap_or(region.start),
            region.len,
        ),
        Err(_) => {
            crate::log!("[argon2-bench] runtime_psram=UNAVAILABLE FAIL");
            return false;
        }
    }
    for candidate in &CANDIDATES {
        watchdog_feed();
        if !run_candidate(candidate) {
            crate::log!("[argon2-bench] stopping: candidate was not safe/successful");
            return false;
        }
        watchdog_feed();
    }
    crate::log!("[argon2-bench] complete; all fixed Argon2/PSRAM candidates passed");
    true
}

#[inline(never)]
fn run_candidate(candidate: &Candidate) -> bool {
    let free_before = crate::services::memory::psram::free_bytes();
    let largest_before = crate::services::memory::psram::probe_largest_allocatable(PROBE_CAP, PROBE_GRANULARITY);
    let started = esp_hal::xtensa_lx::timer::get_cycle_count();
    let result = crate::services::memory::password_kdf::derive_benchmark_key_32(
        PURPOSE,
        PASSWORD,
        &SALT,
        candidate.parameters(),
    );
    let cycles = esp_hal::xtensa_lx::timer::get_cycle_count().wrapping_sub(started);
    let elapsed_ms = u64::from(cycles) / CPU_CYCLES_PER_MS;
    let free_after = crate::services::memory::psram::free_bytes();
    let largest_after = crate::services::memory::psram::probe_largest_allocatable(PROBE_CAP, PROBE_GRANULARITY);
    let liveness_ok = elapsed_ms <= WATCHDOG_BUDGET_MS;
    let status = classify_result(result, &candidate.expected);
    crate::log!(
        "[argon2-bench] m={}KiB t={} p={} alloc={} provenance={} integrity={} vector={} cycles={} ms={} psram=0x{:08x}..0x{:08x} workspace=0x{:08x}..0x{:08x} workspace_bytes={} free_before={} free_after={} largest_before={} largest_after={} watchdog_ok={} result={}",
        candidate.memory_kib, T_COST, P_COST, status.allocation_ok, status.checks.provenance_ok,
        status.checks.integrity_ok, status.checks.vector_ok, cycles, elapsed_ms, status.psram_start,
        status.psram_end, status.workspace_start, status.workspace_end, status.workspace_len,
        free_before, free_after, largest_before, largest_after, liveness_ok,
        status.is_safe(liveness_ok),
    );
    status.is_safe(liveness_ok)
}

#[derive(Clone, Copy)]
struct CandidateChecks {
    provenance_ok: bool,
    integrity_ok: bool,
    vector_ok: bool,
}

impl CandidateChecks {
    fn all_ok(self) -> bool {
        self.provenance_ok && self.integrity_ok && self.vector_ok
    }
}

struct CandidateStatus {
    allocation_ok: bool,
    checks: CandidateChecks,
    psram_start: usize,
    psram_end: usize,
    workspace_start: usize,
    workspace_end: usize,
    workspace_len: usize,
}

impl CandidateStatus {
    fn is_safe(&self, liveness_ok: bool) -> bool {
        self.allocation_ok && self.checks.all_ok() && liveness_ok
    }
}

fn classify_result(
    result: Result<crate::services::memory::password_kdf::BenchmarkResult, PasswordKdfError>,
    expected: &[u8; 32],
) -> CandidateStatus {
    match result {
        Ok(mut result) => {
            let vector_ok = result.key.as_ref().is_ok_and(|key| key == expected);
            if let Ok(key) = result.key.as_mut() {
                zeroize_bytes(key);
            }
            let psram_end = result.info.psram.end().unwrap_or(result.info.psram.start);
            let workspace_end = result.info.workspace_start.checked_add(result.info.workspace_len).unwrap_or(0);
            let provenance_ok = result.info.psram.contains(result.info.workspace_start, result.info.workspace_len);
            CandidateStatus {
                allocation_ok: true,
                checks: CandidateChecks { provenance_ok, integrity_ok: result.integrity_ok, vector_ok },
                psram_start: result.info.psram.start,
                psram_end,
                workspace_start: result.info.workspace_start,
                workspace_end,
                workspace_len: result.info.workspace_len,
            }
        }
        Err(_) => CandidateStatus {
            allocation_ok: false,
            checks: CandidateChecks { provenance_ok: false, integrity_ok: false, vector_ok: false },
            psram_start: 0,
            psram_end: 0,
            workspace_start: 0,
            workspace_end: 0,
            workspace_len: 0,
        },
    }
}
