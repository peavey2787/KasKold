#!/usr/bin/env python3
"""Native Windows CRAP/coverage pipeline equivalent to scripts/linux/quality/crap.sh."""
from __future__ import annotations
import argparse, os, shutil, subprocess, sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import NamedTuple
ROOT=Path(__file__).resolve().parents[3]
PY=sys.executable
NO_COLOR_ENV={'NO_COLOR':'1'}
COMMON_SCORE_ARGS=['--threshold','30','--missing','pessimistic','--sort','crap']

def utc(): return datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
def truthy(v): return str(v).lower() in {'1','true','yes'}
def run(cmd,cwd=ROOT,env=None,log=None,append=False,check=True):
    merged=os.environ.copy(); merged.update(env or {})
    if log:
        mode='a' if append else 'w'
        with open(log,mode,encoding='utf-8',errors='replace') as out:
            p=subprocess.Popen(cmd,cwd=cwd,env=merged,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,errors='replace')
            assert p.stdout
            for line in p.stdout: print(line,end=''); out.write(line)
            code=p.wait()
    else: code=subprocess.run(cmd,cwd=cwd,env=merged,check=False).returncode
    if check and code: raise subprocess.CalledProcessError(code,cmd)
    return code

def capture(cmd,cwd=ROOT,env=None): return subprocess.run(cmd,cwd=cwd,env={**os.environ,**(env or {})},text=True,capture_output=True,check=False)
def nonempty(p,label):
    if not p.is_file() or p.stat().st_size==0: raise RuntimeError(f'{label} was not generated or is empty: {p}')
def source_label():
    r=capture(['git','-C',str(ROOT),'rev-parse','--short','HEAD']) if shutil.which('git') else None
    return f"git {r.stdout.strip()} generated {utc()}" if r and r.returncode==0 else f"working tree generated {utc()}"

def _run_output(cmd,path,label,env=None,log=None,append=True):
    run(cmd,env=env,log=log,append=append)
    nonempty(path,label)

def _read_optional(path):
    return path.read_bytes() if path.is_file() else None

def _has_generated_run(output):
    return (output/'lcov.info').is_file() and (output/'run.json').is_file()

@dataclass(frozen=True)
class _Artifacts:
    lcov: Path
    companion_lcov: Path
    vault_lcov: Path
    cargo_json: Path
    human: Path
    host_json: Path
    firmware_json: Path
    companion_json: Path
    vault_json: Path
    host_human: Path
    firmware_human: Path
    companion_human: Path
    vault_human: Path
    coverage_log: Path
    crap_log: Path
    run_json: Path

def _artifacts(output):
    return _Artifacts(
        lcov=output/'lcov.info',
        companion_lcov=output/'companion_web_lcov.info',
        vault_lcov=output/'vault_web_lcov.info',
        cargo_json=output/'cargo_crap.json',
        human=output/'crap_report_full.txt',
        host_json=output/'cargo_crap_host.json',
        firmware_json=output/'cargo_crap_firmware.json',
        companion_json=output/'cargo_crap_companion_web.json',
        vault_json=output/'cargo_crap_vault_web.json',
        host_human=output/'crap_report_host.txt',
        firmware_human=output/'crap_report_firmware.txt',
        companion_human=output/'crap_report_companion_web.txt',
        vault_human=output/'crap_report_vault_web.txt',
        coverage_log=output/'coverage_run.txt',
        crap_log=output/'crap_run.txt',
        run_json=output/'run.json',
    )

def _binary_missing(toolchain,branch):
    missing=[]
    if branch and not toolchain: missing.append('CRAP_BRANCH_TOOLCHAIN (a pinned nightly toolchain)')
    missing.extend(c for c in ('cargo','rustup','node') if not shutil.which(c))
    return missing

def _probe_toolchain(toolchain):
    missing=[]
    rustc=capture(['rustup','run',toolchain,'rustc','--version'])
    versions={'rustc':rustc.stdout.strip(),'llvm':'','crap':''}
    if rustc.returncode:
        missing.append(f'Rust toolchain {toolchain}')
        return missing,versions
    llvm=capture(['cargo',f'+{toolchain}','llvm-cov','--version'])
    versions['llvm']=llvm.stdout.strip()
    if llvm.returncode: missing.append(f'cargo llvm-cov for {toolchain}')
    crap=capture(['cargo',f'+{toolchain}','crap','--version'])
    versions['crap']=crap.stdout.strip()
    if crap.returncode: missing.append(f'cargo crap for {toolchain}')
    components=capture(['rustup','component','list','--toolchain',toolchain,'--installed'])
    if not any(x.startswith('llvm-tools') for x in components.stdout.splitlines()):
        missing.append(f'llvm-tools-preview for {toolchain}')
    return missing,versions

def _detect_tools(toolchain,branch):
    missing=_binary_missing(toolchain,branch)
    versions={'rustc':'','llvm':'','crap':''}
    if not missing:
        probe_missing,versions=_probe_toolchain(toolchain)
        missing.extend(probe_missing)
    if branch and toolchain and not toolchain.startswith('nightly'):
        missing.append('a pinned nightly toolchain for branch coverage')
    return missing,versions

def _print_detected(toolchain,branch,versions):
    print(f'\nCRAP analysis tools detected:\n  Toolchain:      {toolchain}\n  Rust:           {versions["rustc"]}\n  cargo llvm-cov: {versions["llvm"]}\n  cargo crap:     {versions["crap"]}\n  Branch data:    {"requested" if branch else "not requested"}\n')

def _report_branch_failure(missing):
    print('\nERROR: requested branch coverage could not run.\nThe pinned branch-coverage prerequisites are incomplete:',file=sys.stderr)
    for x in missing: print(f'  - {x}',file=sys.stderr)
    print('Run `scripts/windows/quality/branch-coverage-setup.ps1`, then retry the strict QA pipeline.',file=sys.stderr)
    return 2

def _skip_generation(missing,ratchet):
    print('\nCRAP report generation skipped.\nThe optional local coverage tools are not fully available:')
    for x in missing: print(f'  - {x}')
    return subprocess.run([PY,'qa/checks/quality/crap/check.py','--ignore-generated-report','--ratchet-contract',str(ratchet)],cwd=ROOT).returncode

def _coverage_settings():
    dev=os.getenv('CRAP_DEV_OPT_LEVEL','0')
    test=os.getenv('CRAP_TEST_OPT_LEVEL','0')
    toolchain=os.getenv('CRAP_COVERAGE_TOOLCHAIN','stable')
    branch=truthy(os.getenv('CRAP_ENABLE_BRANCH','0'))
    if branch: toolchain=os.getenv('CRAP_BRANCH_TOOLCHAIN','')
    return dev,test,toolchain,branch

def _prepare_output(output):
    snapshots={'current.json':'previous.json','run.json':'previous_run.json','health_summary.json':'previous_health_summary.json'}
    contents={dst:_read_optional(output/src) for src,dst in snapshots.items()}
    shutil.rmtree(output,ignore_errors=True)
    output.mkdir(parents=True)
    for name,data in contents.items():
        if data is not None: (output/name).write_bytes(data)

def _coverage_commands(toolchain,branch,art):
    cov=['cargo',f'+{toolchain}','llvm-cov','--workspace','--lcov','--ignore-filename-regex',r'unit_tests|online-watcher[\\/]src[\\/]wasm_api[\\/]mod\.rs$','--output-path',str(art.lcov)]
    companion_cov=['cargo',f'+{toolchain}','llvm-cov','--manifest-path',str(ROOT/'apps/kaskold-companion-web/Cargo.toml'),'--workspace','--lib','--lcov','--ignore-filename-regex','unit_tests','--output-path',str(art.companion_lcov)]
    vault_cov=['cargo',f'+{toolchain}','llvm-cov','--manifest-path',str(ROOT/'apps/kaskold-vault-web/Cargo.toml'),'--workspace','--lib','--lcov','--ignore-filename-regex','unit_tests','--output-path',str(art.vault_lcov)]
    if branch:
        for cmd in (cov,companion_cov,vault_cov): cmd.append('--branch')
    return cov,companion_cov,vault_cov

def _run_coverage(toolchain,dev,test,branch,art):
    cov,companion_cov,vault_cov=_coverage_commands(toolchain,branch,art)
    coverage_env={'CARGO_PROFILE_DEV_OPT_LEVEL':dev,'CARGO_PROFILE_TEST_OPT_LEVEL':test}
    print('[CRAP 1/4] Running coverage for each host-testable Rust workspace...')
    print('Coverage profile: dev/test opt-level=0 for source-faithful LLVM function/branch mapping.')
    print('  - Root Cargo workspace')
    _run_output(cov,art.lcov,'root workspace LCOV coverage data',env=coverage_env,log=art.coverage_log,append=False)
    print('  - KasKold Companion Web Rust shell')
    _run_output(companion_cov,art.companion_lcov,'KasKold Companion Web LCOV coverage data',env=coverage_env,log=art.coverage_log)
    print('  - KasKold Vault Web Rust shell')
    _run_output(vault_cov,art.vault_lcov,'KasKold Vault Web LCOV coverage data',env=coverage_env,log=art.coverage_log)
    print(f'[CRAP 1/4] PASS: scope-aligned coverage completed (root {art.lcov.stat().st_size} bytes; KasKold Companion Web {art.companion_lcov.stat().st_size} bytes; KasKold Vault Web {art.vault_lcov.stat().st_size} bytes).\n')

class _ScopeCommands(NamedTuple):
    host: list
    companion: list
    vault: list
    firmware: list

def _scope_commands(toolchain,art):
    return _ScopeCommands(
        host=['cargo',f'+{toolchain}','crap','--workspace','--lcov',str(art.lcov),'--exclude','**/unit_tests/**','--exclude','src/wasm/**','--exclude','src/wasm_api/mod.rs',*COMMON_SCORE_ARGS],
        companion=['cargo',f'+{toolchain}','crap','--path','apps/kaskold-companion-web','--lcov',str(art.companion_lcov),'--exclude','**/unit_tests/**',*COMMON_SCORE_ARGS],
        vault=['cargo',f'+{toolchain}','crap','--path','apps/kaskold-vault-web','--lcov',str(art.vault_lcov),'--exclude','**/unit_tests/**',*COMMON_SCORE_ARGS],
        firmware=['cargo',f'+{toolchain}','crap','--path','apps/kaskold-hardware','--no-default-excludes',*COMMON_SCORE_ARGS],
    )

def _merge_json_command(art):
    return [PY,'qa/checks/quality/crap/merge_reports.py','--host-json',str(art.host_json),'--firmware-json',str(art.firmware_json),'--kaskold-companion-web-json',str(art.companion_json),'--kaskold-vault-web-json',str(art.vault_json),'--output-json',str(art.cargo_json)]

def _run_json_scores(scopes,art):
    print('[CRAP 2/4] Calculating machine-readable CRAP scores by matching scope...')
    print('  - Root Cargo workspace: coverage-backed CRAP')
    _run_output(scopes.host+['--format','json','--output',str(art.host_json)],art.host_json,'root workspace cargo-crap JSON report',env=NO_COLOR_ENV,log=art.crap_log)
    print('  - KasKold Companion Web Rust shell: coverage-backed CRAP')
    _run_output(scopes.companion+['--format','json','--output',str(art.companion_json)],art.companion_json,'KasKold Companion Web cargo-crap JSON report',env=NO_COLOR_ENV,log=art.crap_log)
    print('  - KasKold Vault Web Rust shell: coverage-backed CRAP')
    _run_output(scopes.vault+['--format','json','--output',str(art.vault_json)],art.vault_json,'KasKold Vault Web cargo-crap JSON report',env=NO_COLOR_ENV,log=art.crap_log)
    print('  - Signer firmware: complexity-only CRAP (host LCOV is not valid for Xtensa firmware)')
    _run_output(scopes.firmware+['--format','json','--output',str(art.firmware_json)],art.firmware_json,'signer firmware cargo-crap JSON report',env=NO_COLOR_ENV,log=art.crap_log)
    _run_output(_merge_json_command(art),art.cargo_json,'merged cargo-crap JSON report')
    print(f'[CRAP 2/4] PASS: machine-readable CRAP scoring completed ({art.cargo_json.stat().st_size} bytes).\n')

def _run_human_reports(scopes,art):
    print('[CRAP 3/4] Rendering the human-readable CRAP report by matching scope...')
    _run_output(scopes.host+['--format','human','--output',str(art.host_human)],art.host_human,'root workspace human cargo-crap report',env=NO_COLOR_ENV,log=art.crap_log)
    _run_output(scopes.companion+['--format','human','--output',str(art.companion_human)],art.companion_human,'KasKold Companion Web human cargo-crap report',env=NO_COLOR_ENV,log=art.crap_log)
    _run_output(scopes.vault+['--format','human','--output',str(art.vault_human)],art.vault_human,'KasKold Vault Web human cargo-crap report',env=NO_COLOR_ENV,log=art.crap_log)
    _run_output(scopes.firmware+['--format','human','--output',str(art.firmware_human)],art.firmware_human,'signer firmware human cargo-crap report',env=NO_COLOR_ENV,log=art.crap_log)
    merge=_merge_json_command(art)+['--host-human',str(art.host_human),'--firmware-human',str(art.firmware_human),'--kaskold-companion-web-human',str(art.companion_human),'--kaskold-vault-web-human',str(art.vault_human),'--output-human',str(art.human)]
    _run_output(merge,art.human,'human-readable cargo-crap report')
    print(f'[CRAP 3/4] PASS: human-readable report completed ({art.human.stat().st_size} bytes).\n')

def _write_run_manifest(art,started,toolchain,versions,dev,test,branch):
    args=[PY,'qa/checks/quality/crap/coverage_manifest.py','--output',str(art.run_json),'--started-at',started,'--finished-at',utc(),'--toolchain',toolchain,'--rustc-version',versions['rustc'],'--llvm-cov-version',versions['llvm'],'--cargo-crap-version',versions['crap'],'--dev-opt-level',dev,'--test-opt-level',test,'--root',str(ROOT),'--lcov',str(art.lcov),'--cargo-crap-json',str(art.cargo_json)]
    if branch: args.append('--branch-requested')
    run(args)

def _generate_reports(output,toolchain,dev,test,branch,versions):
    started=utc()
    art=_artifacts(output)
    art.crap_log.write_text(f'KasKold CRAP analysis log\nToolchain: {toolchain}\nScopes: root workspace (LCOV-backed), KasKold Companion Web (LCOV-backed), KasKold Vault Web (LCOV-backed), signer firmware (complexity-only)\n',encoding='utf-8')
    print(f'CRAP analysis is the first QA workload.\nGenerated artifacts will be available before the remaining catalog starts.\nWorking artifact directory: {output}\n')
    _run_coverage(toolchain,dev,test,branch,art)
    scopes=_scope_commands(toolchain,art)
    _run_json_scores(scopes,art)
    _run_human_reports(scopes,art)
    _write_run_manifest(art,started,toolchain,versions,dev,test,branch)
    return art.cargo_json

def _run_web_coverage(output):
    if not _has_generated_run(output): return
    print('[Browser recovery] Running Companion recovery tests with V8 coverage...')
    run([PY,'qa/checks/web/run_web_recovery_coverage.py','--output-dir',str(output/'browser_recovery')])
    nonempty(output/'browser_recovery/summary.json','browser recovery coverage summary')
    nonempty(output/'browser_recovery/v8-coverage.json','browser recovery V8 coverage')
    print('[Web runtime] Mapping all reachable Companion JS modules with merged V8 integration coverage...')
    run([PY,'qa/checks/web/run_web_runtime_coverage.py','--output-dir',str(output/'web_runtime')])
    nonempty(output/'web_runtime/summary.json','web runtime coverage summary')
    nonempty(output/'web_runtime/v8-coverage.json','web runtime V8 coverage')

def _classify_report(output,input_report):
    print('[CRAP 4/4] Classifying production, tests, external, and tools...')
    cls=[PY,'qa/checks/quality/crap/classify_report.py','--input',str(input_report),'--output-dir',str(output),'--source-label',source_label()]
    if (output/'crap_report_full.txt').is_file(): cls += ['--display-report',str(output/'crap_report_full.txt')]
    run(cls)

def _optional_check_args(output):
    args=[]
    if _has_generated_run(output):
        args += ['--lcov',str(output/'lcov.info'),'--run-manifest',str(output/'run.json'),'--browser-recovery-coverage',str(output/'browser_recovery/summary.json'),'--web-runtime-coverage',str(output/'web_runtime/summary.json'),'--health-output',str(output/'health_summary.json')]
    if (output/'previous.json').is_file(): args += ['--previous-report',str(output/'previous.json')]
    if (output/'run.json').is_file() and (output/'previous_run.json').is_file(): args += ['--previous-run-manifest',str(output/'previous_run.json')]
    if _has_generated_run(output) and (output/'previous_health_summary.json').is_file(): args += ['--previous-health-summary',str(output/'previous_health_summary.json')]
    return args

def _run_check(output,ratchet,strict):
    check=[PY,'qa/checks/quality/crap/check.py','--report',str(output/'current.json'),'--ratchet-contract',str(ratchet)]
    if strict: check.append('--strict-report')
    check += _optional_check_args(output)
    run(check)

def _print_summary(output,ratchet):
    print(f'[CRAP 4/4] PASS: reports classified and checked.\n\nFresh CRAP artifacts are ready while the remaining QA tests run:\n  Full report:       {output}/crap_report_full.txt\n  Production report: {output}/crap_report_prod.txt\n  Summary:           {output}/crap_summary.json\n  Health audit:      {output}/health_summary.json\n  LCOV data:         {output}/lcov.info\n\nCommitted quality ratchet:\n  Contract:          {ratchet}\n')

def _parse_args(argv):
    ap=argparse.ArgumentParser()
    ap.add_argument('--input-report',default='')
    ap.add_argument('--strict',action='store_true')
    return ap.parse_args(argv)

def main(argv):
    ns=_parse_args(argv)
    output=Path(os.getenv('CRAP_OUTPUT_DIR',ROOT/'target/qa/crap'))
    ratchet=Path(os.getenv('CRAP_RATCHET_PATH',ROOT/'qa/contracts/quality/crap_ratchets.json'))
    dev,test,toolchain,branch=_coverage_settings()
    input_report=Path(ns.input_report) if ns.input_report else None
    missing,versions=_detect_tools(toolchain,branch)
    if not missing:
        _print_detected(toolchain,branch,versions)
    elif branch and not input_report:
        return _report_branch_failure(missing)
    elif not input_report:
        return _skip_generation(missing,ratchet)
    if input_report and not input_report.is_file():
        print(f'ERROR: CRAP input report does not exist: {input_report}',file=sys.stderr)
        return 2
    _prepare_output(output)
    if not input_report:
        input_report=_generate_reports(output,toolchain,dev,test,branch,versions)
    _run_web_coverage(output)
    _classify_report(output,input_report)
    _run_check(output,ratchet,ns.strict)
    _print_summary(output,ratchet)
    return 0

if __name__=='__main__':raise SystemExit(main(sys.argv[1:]))
