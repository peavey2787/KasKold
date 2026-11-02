#!/usr/bin/env python3
"""Regression contracts for the direct-Companion mobile shells and web quality scope."""
from __future__ import annotations
import importlib.util
from pathlib import Path
import sys, unittest
ROOT = Path(__file__).resolve().parents[3]

def load(path: Path, name: str):
    spec=importlib.util.spec_from_file_location(name,path); assert spec and spec.loader
    module=importlib.util.module_from_spec(spec); sys.modules[name]=module; spec.loader.exec_module(module); return module

class MobileWebCoverageContracts(unittest.TestCase):
    def test_android_mutation_scope_covers_live_domain_and_infrastructure(self):
        path=ROOT/'qa/checks/android/run_mutation_tests.py'; source=path.read_text(); self.assertNotIn('MUTANTS =',source)
        module=load(path,'android_full_mutation'); mutants,files=module.discover()
        relative=[f.relative_to(module.SOURCE).as_posix() for f in files]
        self.assertTrue(any(x.startswith('domain/') for x in relative)); self.assertTrue(any(x.startswith('infrastructure/') for x in relative))
        self.assertEqual(set(relative),{p.relative_to(module.SOURCE).as_posix() for r in module.SCOPE_ROOTS for p in r.rglob('*.kt')})
        self.assertGreater(len(mutants),0); self.assertEqual(module.MINIMUM_SCORE_PERCENT,100.0)


    def test_web_runtime_collector_maps_the_real_main_module_graph(self):
        path=ROOT/'qa/checks/web/run_web_runtime_coverage.py'; module=load(path,'web_runtime_coverage')
        reachable=module.reachable_modules(); all_js={p.relative_to(ROOT).as_posix() for p in (ROOT/'apps/kaskold-companion-web/web/js').rglob('*.js')}
        self.assertTrue(all_js); self.assertEqual(reachable,all_js)

if __name__=='__main__': unittest.main()
