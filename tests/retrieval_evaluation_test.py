"""Reject unsafe or vacuous evaluation fixtures and incorrect protocol observations.

These unit fixtures test the checker only. The actual MCP evaluation is a separate
run against a trusted built executable; canned responses are never reported as it.
"""
import importlib.util
import json
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / 'scripts/evaluate-retrieval.py'
FIXTURE = ROOT / 'evals/retrieval/v1.json'


class RetrievalEvaluationTest(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Real-MCP retrieval evaluator must exist')
        spec = importlib.util.spec_from_file_location('retrieval_evaluation', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def fixture(self):
        return json.loads(FIXTURE.read_text(encoding='utf-8'))

    def response(self):
        project = {'id': '11111111-1111-4111-8111-111111111111', 'name': 'Synthetic', 'purpose': 'Checker fixture',
                   'directory': None, 'created_at': '2026-01-01T00:00:00Z'}
        original = {'id': '22222222-2222-4222-8222-222222222222', 'project_id': project['id'], 'kind': 'note', 'title': 'Alpha',
                    'content': 'Full original evidence café', 'sources': ['fixture://original'],
                    'author_type': 'user', 'created_at': '2026-01-01T00:00:00Z', 'superseded_by': None}
        selection = {'mode': 'query', 'candidate_count': 1, 'candidate_window_truncated': False,
                     'omitted_by_budget': 0, 'omitted_by_record_limit': 0, 'selection_complete': True}
        value = {'project': project, 'records': [dict(original, selection_reason='literal_query_match')],
                 'selection': selection,
                 'budget': {'max_bytes': 4096, 'max_records': 8, 'used_bytes': 1930,
                            'estimated_tokens': 483,
                            'estimate_method': 'ceil(serialized_mcp_result_bytes / 4); heuristic only, not a model token count'},
                 'notice': 'Synthetic unit fixture'}
        raw = {'content': [{'type': 'text', 'text': json.dumps(value, ensure_ascii=False, separators=(',', ':'), sort_keys=True)}],
               'structuredContent': value, 'isError': False}
        expected = {'selected_aliases': ['kept'], 'candidate_count': 1,
                    'candidate_window_truncated': False, 'omitted_by_budget': 0,
                    'omitted_by_record_limit': 0, 'selection_complete': True}
        case = {'name': 'unit-only', 'max_bytes': 4096, 'max_records': 8,
                'relevant_aliases': ['kept'], 'forbidden_aliases': [],
                'expected': {'literal': expected}}
        return raw, case, project, {original['id']: original}, {original['id']: 'kept'}

    def check(self, module, raw, case, project, originals, aliases):
        return module.check_result(raw, case, 'literal', project, originals, aliases)

    def sync_text(self, raw):
        raw['content'][0]['text'] = json.dumps(raw['structuredContent'], ensure_ascii=False,
                                             separators=(',', ':'), sort_keys=True)

    def rebalance_mutated_response(self, raw):
        # Build a self-consistent adversarial wire fixture independently of the
        # production checker: otherwise a stale byte counter would hide a gap.
        for _ in range(8):
            self.sync_text(raw)
            complete = dict(raw, resultType='complete')
            measured = len(json.dumps(complete, ensure_ascii=False, separators=(',', ':')).encode('utf-8'))
            budget = raw['structuredContent']['budget']
            if budget['used_bytes'] == measured and budget['estimated_tokens'] == (measured + 3) // 4:
                return
            budget['used_bytes'] = measured
            budget['estimated_tokens'] = (measured + 3) // 4
        self.fail('Controlled response accounting did not converge')

    def test_fixture_cannot_omit_cases_or_mode_expectations(self):
        m = self.module()
        for mutate in [lambda f: f.update(cases=[]),
                       lambda f: f['cases'].pop(),
                       lambda f: f['cases'][0].pop('expected'),
                       lambda f: f['cases'][0]['expected'].pop('terms'),
                       lambda f: f['cases'][0]['expected']['terms'].pop('matched_terms'),
                       lambda f: f['cases'][0]['expected']['terms'].update(matched_terms={})]:
            fixture = self.fixture()
            mutate(fixture)
            with self.assertRaises(ValueError):
                m.validate_fixture(fixture)

    def test_fixture_cannot_expect_foreign_or_unknown_aliases(self):
        m = self.module()
        for alias in ['foreign-record', 'undefined-record']:
            fixture = self.fixture()
            fixture['cases'][0]['expected']['terms']['selected_aliases'] = [alias]
            with self.assertRaises(ValueError):
                m.validate_fixture(fixture)

    def test_fixture_rejects_unbounded_expansion_and_undefined_supersession(self):
        m = self.module()
        fixture = self.fixture()
        fixture['records'][0]['repeat'] = 1001
        with self.assertRaises(ValueError):
            m.validate_fixture(fixture)
        fixture = self.fixture()
        record = next(r for r in fixture['records'] if r['kind'] == 'decision')
        record['supersedes'] = 'undefined-record'
        with self.assertRaises(ValueError):
            m.validate_fixture(fixture)

    def test_canonical_bytes_measure_utf8_and_restore_legacy_completion_marker(self):
        m = self.module()
        raw = {'content': [{'type': 'text', 'text': 'é'}], 'structuredContent': {'text': 'é'}, 'isError': False}
        # Hand-counted compact UTF-8 protocol fixture, including resultType.
        self.assertEqual(m.canonical_result_bytes(raw), 115)
        raw['resultType'] = 'complete'
        self.assertEqual(m.canonical_result_bytes(raw), 115)

    def test_checker_accepts_frozen_complete_observation_without_claiming_real_mcp(self):
        m = self.module()
        observation = self.check(m, *self.response())
        self.assertEqual(observation['selected_aliases'], ['kept'])
        self.assertEqual(observation['canonical_result_bytes'], 1930)
        self.assertEqual(observation['relevant_returned'], 1)
        self.assertEqual(observation['relevant_defined'], 1)

    def test_checker_rejects_reported_size_lies_and_actual_overshoot(self):
        m = self.module()
        raw, *rest = self.response()
        raw['structuredContent']['budget']['used_bytes'] = 1929
        self.sync_text(raw)
        with self.assertRaisesRegex(ValueError, 'byte accounting'):
            self.check(m, raw, *rest)
        raw, *rest = self.response()
        raw['_meta'] = {'padding': 'x' * 5000}
        with self.assertRaisesRegex(ValueError, 'byte cap'):
            self.check(m, raw, *rest)

    def test_checker_rejects_truncation_and_changed_original_sources(self):
        m = self.module()
        for field, changed in [('content', 'Shortened'), ('sources', ['fixture://invented'])]:
            raw, *rest = self.response()
            raw['structuredContent']['records'][0][field] = changed
            self.sync_text(raw)
            with self.assertRaisesRegex(ValueError, 'complete original record'):
                self.check(m, raw, *rest)

    def test_checker_rejects_project_leakage_and_wrong_omission_counters(self):
        m = self.module()
        raw, *rest = self.response()
        raw['structuredContent']['records'][0]['project_id'] = 'foreign-project'
        self.sync_text(raw)
        with self.assertRaisesRegex(ValueError, 'project isolation'):
            self.check(m, raw, *rest)
        raw, *rest = self.response()
        raw['structuredContent']['selection']['omitted_by_budget'] = 1
        self.sync_text(raw)
        with self.assertRaisesRegex(ValueError, 'omission'):
            self.check(m, raw, *rest)

    def test_checker_rejects_disagreeing_compatibility_text(self):
        m = self.module()
        raw, *rest = self.response()
        raw['content'][0]['text'] = '{}'
        with self.assertRaisesRegex(ValueError, 'compatibility text'):
            self.check(m, raw, *rest)

    def test_checker_rejects_shared_serializer_drop_of_required_provenance_fields(self):
        m = self.module()
        for field in ['created_at', 'superseded_by', 'kind', 'author_type', 'sources', 'title']:
            raw, case, project, originals, aliases = self.response()
            originals[next(iter(originals))].pop(field)
            raw['structuredContent']['records'][0].pop(field)
            self.rebalance_mutated_response(raw)
            with self.assertRaisesRegex(ValueError, 'record provenance', msg=field):
                self.check(m, raw, case, project, originals, aliases)

    def test_checker_rejects_correlated_invalid_provenance_values(self):
        m = self.module()
        for field, invalid in [('created_at', 'not-a-timestamp'),
                               ('created_at', '2026-02-30T00:00:00Z'),
                               ('author_type', 'system'), ('sources', [17]),
                               ('superseded_by', 'not-a-uuid')]:
            raw, case, project, originals, aliases = self.response()
            originals[next(iter(originals))][field] = invalid
            raw['structuredContent']['records'][0][field] = invalid
            self.rebalance_mutated_response(raw)
            with self.assertRaisesRegex(ValueError, 'record provenance', msg=field):
                self.check(m, raw, case, project, originals, aliases)

    def test_checker_rejects_shared_project_metadata_drop(self):
        m = self.module()
        raw, case, project, originals, aliases = self.response()
        project.pop('created_at')
        self.rebalance_mutated_response(raw)
        with self.assertRaisesRegex(ValueError, 'project provenance'):
            self.check(m, raw, case, project, originals, aliases)

    def test_checker_rejects_truncated_matched_terms_with_correct_accounting(self):
        m = self.module()
        raw, case, project, originals, aliases = self.response()
        case['expected']['terms'] = dict(case['expected']['literal'],
            query_terms=['alpha', 'evidence'], matched_terms={'kept': ['alpha', 'evidence']})
        selection = raw['structuredContent']['selection']
        selection.update(mode='terms', query_terms=['alpha', 'evidence'],
                         ranking='term_coverage_then_headline_then_recency')
        record = raw['structuredContent']['records'][0]
        record['selection_reason'] = 'ranked_term_match'
        record['matched_terms'] = ['alpha']
        self.rebalance_mutated_response(raw)
        with self.assertRaisesRegex(ValueError, 'matched terms'):
            m.check_result(raw, case, 'terms', project, originals, aliases)

    def test_command_requires_explicit_trusted_binary(self):
        self.module()
        completed = subprocess.run([sys.executable, str(SCRIPT)], capture_output=True, text=True)
        self.assertEqual(completed.returncode, 2)
        self.assertEqual(completed.stdout, '')

    def test_command_returns_failure_before_starting_binary_for_missing_expectations(self):
        self.module()
        with tempfile.TemporaryDirectory(prefix='retrieval-checker-test-') as directory:
            root = Path(directory)
            script = root / 'scripts/evaluate-retrieval.py'
            script.parent.mkdir()
            shutil.copyfile(SCRIPT, script)
            fixture_path = root / 'evals/retrieval/v1.json'
            fixture_path.parent.mkdir(parents=True)
            fixture = self.fixture()
            fixture['cases'][0]['expected'].pop('terms')
            fixture_path.write_text(json.dumps(fixture), encoding='utf-8')
            # Python is an executable, but must never be started as an MCP
            # process: fixture validation must reject this before Client loads.
            completed = subprocess.run([sys.executable, str(script), sys.executable],
                                       capture_output=True, text=True, timeout=3)
            self.assertEqual(completed.returncode, 1)
            self.assertEqual(completed.stdout, '')
            self.assertIn('Both literal and terms expectations are required', completed.stderr)


if __name__ == '__main__':
    unittest.main()
