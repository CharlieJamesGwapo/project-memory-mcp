#!/usr/bin/env python3
"""Evaluate versioned synthetic retrieval fixtures against a trusted local MCP binary.

Always uses a new temporary archive. Prints JSON observations only after every
gate passes. This is a protocol/selection regression suite, not a model benchmark
or a measurement of billed tokens, cost savings or universal retrieval quality.
"""
import argparse
import copy
from datetime import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import tempfile
import uuid

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'evals/retrieval/v1.json'
REQUIRED_CASES = {'distributed-fields', 'rank-before-window', 'literal-phrase',
                  'history-source-isolation', 'byte-cap-omission'}
EXPECTED_FIELDS = {'selected_aliases', 'candidate_count', 'candidate_window_truncated',
                   'omitted_by_budget', 'omitted_by_record_limit', 'selection_complete'}
LIST_FIELDS = ('completed', 'unfinished', 'next_actions', 'blockers', 'verification')
PROJECT_FIELDS = {'id', 'name', 'purpose', 'directory', 'created_at'}
RECORD_FIELDS = {'id', 'project_id', 'kind', 'author_type', 'sources', 'created_at', 'superseded_by'}
PAYLOAD_FIELDS = {'note': {'title', 'content'}, 'decision': {'decision', 'rationale'},
                  'handoff': {'summary', *LIST_FIELDS}}
TEXT_LIMITS = {'title': 200, 'content': 16000, 'decision': 8000, 'rationale': 8000, 'summary': 4000}


class EvaluationError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise EvaluationError(message)


def text(value, label, maximum):
    require(isinstance(value, str) and value.strip() and len(value) <= maximum,
            f'Invalid {label}')
    value.encode('utf-8')


def alias(value):
    require(isinstance(value, str) and re.fullmatch(r'[a-z][a-z0-9-]{0,49}', value),
            'Invalid fixture alias')


def provenance_uuid(value, label):
    require(isinstance(value, str), f'Invalid {label} UUID')
    try:
        canonical = str(uuid.UUID(value))
    except ValueError as error:
        raise EvaluationError(f'Invalid {label} UUID') from error
    require(value == canonical, f'Invalid {label} UUID')


def provenance_timestamp(value, label):
    require(isinstance(value, str) and re.fullmatch(
        r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})', value),
        f'Invalid {label} timestamp')
    try:
        parsed = datetime.fromisoformat(value.replace('Z', '+00:00'))
    except ValueError as error:
        raise EvaluationError(f'Invalid {label} timestamp') from error
    require(parsed.tzinfo is not None, f'Invalid {label} timestamp')
    if not value.endswith('Z'):
        require(int(value[-5:-3]) <= 23 and int(value[-2:]) <= 59, f'Invalid {label} timestamp')


def validate_project_provenance(project):
    require(isinstance(project, dict) and set(project) == PROJECT_FIELDS, 'Invalid project provenance schema')
    provenance_uuid(project['id'], 'project provenance id')
    provenance_timestamp(project['created_at'], 'project provenance created_at')
    text(project['name'], 'project provenance name', 200)
    text(project['purpose'], 'project provenance purpose', 4000)
    if project['directory'] is not None:
        text(project['directory'], 'project provenance directory', 4096)


def validate_record_provenance(record):
    require(isinstance(record, dict) and record.get('kind') in PAYLOAD_FIELDS, 'Invalid record provenance kind')
    kind = record['kind']
    fields = RECORD_FIELDS | PAYLOAD_FIELDS[kind] | ({'supersedes'} if kind == 'decision' else set())
    require(set(record) == fields, 'Invalid record provenance schema')
    provenance_uuid(record['id'], 'record provenance id')
    provenance_uuid(record['project_id'], 'record provenance project_id')
    provenance_timestamp(record['created_at'], 'record provenance created_at')
    require(record['author_type'] in ('user', 'assistant'), 'Invalid record provenance attribution')
    require(isinstance(record['sources'], list) and len(record['sources']) <= 10, 'Invalid record provenance sources')
    for source in record['sources']:
        text(source, 'record provenance source', 2048)
    for field in ('superseded_by', *(['supersedes'] if kind == 'decision' else [])):
        if record[field] is not None:
            provenance_uuid(record[field], f'record provenance {field}')
    for field in PAYLOAD_FIELDS[kind]:
        if field in LIST_FIELDS:
            require(isinstance(record[field], list) and len(record[field]) <= 20, f'Invalid record provenance {field}')
            for entry in record[field]:
                text(entry, f'record provenance {field}', 1000)
        else:
            text(record[field], f'record provenance {field}', TEXT_LIMITS[field])


def validate_fixture(fixture):
    require(isinstance(fixture, dict) and set(fixture) == {'fixture_version', 'name', 'projects', 'records', 'cases'},
            'Invalid fixture root or missing required sections')
    require(type(fixture['fixture_version']) is int and fixture['fixture_version'] == 1,
            'Unsupported fixture version')
    text(fixture['name'], 'fixture name', 200)
    projects = fixture['projects']
    require(isinstance(projects, list) and 2 <= len(projects) <= 20, 'Fixture needs 2–20 projects')
    project_aliases = set()
    for project in projects:
        require(isinstance(project, dict) and set(project) == {'alias', 'name', 'purpose'}, 'Invalid fixture project')
        alias(project['alias'])
        require(project['alias'] not in project_aliases, 'Duplicate project alias')
        project_aliases.add(project['alias'])
        text(project['name'], 'project name', 200)
        text(project['purpose'], 'project purpose', 4000)
    require(isinstance(fixture['records'], list) and fixture['records'], 'Fixture records are required')
    expanded, known = [], {}
    for record in fixture['records']:
        require(isinstance(record, dict) and {'alias', 'project', 'kind', 'data'} <= set(record)
                and set(record) <= {'alias', 'project', 'kind', 'data', 'repeat', 'supersedes'},
                'Invalid fixture record')
        alias(record['alias'])
        require(record['project'] in project_aliases, 'Unknown record project')
        require(record['kind'] in ('note', 'decision', 'handoff'), 'Invalid record kind')
        repeat = record.get('repeat', 1)
        require(type(repeat) is int and 1 <= repeat <= 250, 'Record repeat exceeds its bound')
        require(len(expanded) + repeat <= 1000, 'Fixture expands beyond 1000 records')
        data = record['data']
        fields = PAYLOAD_FIELDS[record['kind']]
        require(isinstance(data, dict) and set(data) == fields | {'author_type', 'sources'},
                'Missing or unknown fixture record data')
        require(data['author_type'] in ('user', 'assistant'), 'Invalid fixture attribution')
        require(isinstance(data['sources'], list) and len(data['sources']) <= 10, 'Invalid fixture sources')
        for source in data['sources']:
            text(source, 'source', 2048)
        for field in fields:
            if field in LIST_FIELDS:
                require(isinstance(data[field], list) and len(data[field]) <= 20, 'Invalid handoff list')
                for entry in data[field]:
                    text(entry, field, 1000)
            else:
                text(data[field], field, TEXT_LIMITS[field])
        if 'supersedes' in record:
            old = known.get(record['supersedes'])
            require(record['kind'] == 'decision' and repeat == 1 and old is not None
                    and old['kind'] == 'decision' and old['project'] == record['project'],
                    'Undefined, repeated or foreign supersession')
        for index in range(repeat):
            name = record['alias'] if repeat == 1 else f"{record['alias']}-{index:03d}"
            require(name not in known, 'Duplicate expanded record alias')
            item = {key: copy.deepcopy(value) for key, value in record.items() if key != 'repeat'}
            item['alias'] = name
            known[name] = item
            expanded.append(item)
    cases = fixture['cases']
    require(isinstance(cases, list) and 1 <= len(cases) <= 50, 'Evaluation cases are required')
    case_names = set()
    for case in cases:
        require(isinstance(case, dict) and set(case) == {'name', 'project', 'query', 'max_bytes',
                'max_records', 'relevant_aliases', 'forbidden_aliases', 'expected'}, 'Invalid case or missing expectations')
        alias(case['name'])
        require(case['name'] not in case_names, 'Duplicate evaluation case')
        case_names.add(case['name'])
        require(case['project'] in project_aliases, 'Unknown case project')
        text(case['query'], 'case query', 2000)
        require(type(case['max_bytes']) is int and 4096 <= case['max_bytes'] <= 65536, 'Invalid case byte cap')
        require(type(case['max_records']) is int and 1 <= case['max_records'] <= 20, 'Invalid case record cap')
        for field in ('relevant_aliases', 'forbidden_aliases'):
            values = case[field]
            require(isinstance(values, list) and all(isinstance(v, str) for v in values)
                    and len(values) == len(set(values)) and all(v in known for v in values), 'Unknown or duplicate case alias')
        require(case['relevant_aliases'] or case['forbidden_aliases'], 'A case needs relevant or forbidden records')
        require(not set(case['relevant_aliases']) & set(case['forbidden_aliases']), 'Relevant and forbidden records overlap')
        require(all(known[a]['project'] == case['project'] for a in case['relevant_aliases']), 'Foreign relevant record')
        require(isinstance(case['expected'], dict) and set(case['expected']) == {'literal', 'terms'},
                'Both literal and terms expectations are required')
        for mode, expected in case['expected'].items():
            require(isinstance(expected, dict) and set(expected) == EXPECTED_FIELDS | ({'query_terms', 'matched_terms'} if mode == 'terms' else set()),
                    'Missing or unknown mode expectations')
            selected = expected['selected_aliases']
            require(isinstance(selected, list) and all(isinstance(a, str) for a in selected)
                    and len(selected) == len(set(selected)) and len(selected) <= case['max_records']
                    and all(a in known and known[a]['project'] == case['project'] for a in selected),
                    'Unknown, foreign, duplicate or excessive expected selection')
            require(not set(selected) & set(case['forbidden_aliases']), 'Expected selection contains forbidden record')
            for field in ('candidate_count', 'omitted_by_budget', 'omitted_by_record_limit'):
                require(type(expected[field]) is int and 0 <= expected[field] <= 200, 'Invalid expected count')
            for field in ('candidate_window_truncated', 'selection_complete'):
                require(type(expected[field]) is bool, 'Invalid expected completeness flag')
            require(expected['candidate_count'] == len(selected) + expected['omitted_by_budget'] + expected['omitted_by_record_limit'],
                    'Expected omission counts do not conserve candidates')
            require(expected['selection_complete'] == (not expected['candidate_window_truncated']
                    and expected['omitted_by_budget'] == 0 and expected['omitted_by_record_limit'] == 0),
                    'Inconsistent expected completeness')
            if mode == 'terms':
                terms = expected['query_terms']
                require(isinstance(terms, list) and 1 <= len(terms) <= 8
                        and all(isinstance(term, str) and term for term in terms) and len(terms) == len(set(terms)),
                        'Expected query terms are required')
                matches = expected['matched_terms']
                require(isinstance(matches, dict) and set(matches) == set(selected),
                        'Exact matched-term expectations are required for every selected record')
                for matched in matches.values():
                    require(isinstance(matched, list) and matched
                            and all(isinstance(term, str) for term in matched)
                            and len(matched) == len(set(matched))
                            and matched == [term for term in terms if term in matched],
                            'Expected matched terms must be ordered query terms')
    require(REQUIRED_CASES <= case_names, 'Required versioned evaluation cases are missing')
    return {**fixture, 'records': expanded}


def canonical_result_bytes(raw):
    require(isinstance(raw, dict), 'Tool result must be an object')
    canonical = dict(raw)
    canonical.setdefault('resultType', 'complete')
    require(canonical['resultType'] == 'complete', 'Only complete tool results can be measured')
    return len(json.dumps(canonical, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode('utf-8'))


def check_result(raw, case, mode, project, originals, aliases):
    label = f"{case['name']} / {mode}"
    validate_project_provenance(project)
    require(isinstance(raw, dict) and raw.get('isError') is False, f'{label}: tool failed')
    value = raw.get('structuredContent')
    require(isinstance(value, dict) and value.get('project') == project, f'{label}: project isolation failed')
    records = value.get('records')
    require(isinstance(records, list) and len(records) <= case['max_records'], f'{label}: record cap failed')
    selected = []
    for record in records:
        require(isinstance(record, dict) and record.get('project_id') == project['id']
                and record.get('id') in originals, f'{label}: project isolation failed')
        original = {key: item for key, item in record.items() if key not in ('selection_reason', 'matched_terms')}
        validate_record_provenance(original)
        require(original == originals[record['id']], f'{label}: complete original record or provenance changed')
        selected.append(aliases[record['id']])
    expected = case['expected'][mode]
    require(selected == expected['selected_aliases'] and not set(selected) & set(case['forbidden_aliases']),
            f'{label}: selected aliases or order differ from the fixture')
    selection = value.get('selection')
    require(isinstance(selection, dict) and selection.get('mode') == ('terms' if mode == 'terms' else 'query'),
            f'{label}: query mode differs')
    for field in EXPECTED_FIELDS - {'selected_aliases'}:
        require(selection.get(field) == expected[field] and type(selection.get(field)) is type(expected[field]),
                f'{label}: omission/completeness field {field} differs')
    require(selection['candidate_count'] == len(records) + selection['omitted_by_budget'] + selection['omitted_by_record_limit'],
            f'{label}: omission accounting does not conserve candidates')
    if mode == 'terms':
        require(selection.get('query_terms') == expected['query_terms']
                and selection.get('ranking') == 'term_coverage_then_headline_then_recency', f'{label}: ranked query metadata differs')
        for record in records:
            matched = record.get('matched_terms')
            require(record.get('selection_reason') == 'ranked_term_match' and isinstance(matched, list)
                    and matched == expected['matched_terms'].get(aliases[record['id']]),
                    f'{label}: matched terms differ from the exact fixture expectation')
    else:
        require('query_terms' not in selection and 'ranking' not in selection
                and all(r.get('selection_reason') == 'literal_query_match' and 'matched_terms' not in r for r in records),
                f'{label}: literal compatibility shape changed')
    content = raw.get('content')
    require(isinstance(content, list) and len(content) == 1 and isinstance(content[0], dict)
            and content[0].get('type') == 'text' and isinstance(content[0].get('text'), str),
            f'{label}: compatibility text is missing')
    try:
        fallback = json.loads(content[0]['text'])
    except (ValueError, TypeError) as error:
        raise EvaluationError(f'{label}: compatibility text is invalid JSON') from error
    require(fallback == value, f'{label}: compatibility text disagrees with structured content')
    measured = canonical_result_bytes(raw)
    require(measured <= case['max_bytes'], f'{label}: canonical result exceeds byte cap')
    budget = value.get('budget')
    require(isinstance(budget, dict) and budget.get('max_bytes') == case['max_bytes']
            and budget.get('max_records') == case['max_records'], f'{label}: returned caps differ')
    require(type(budget.get('used_bytes')) is int and budget['used_bytes'] == measured
            and type(budget.get('estimated_tokens')) is int and budget['estimated_tokens'] == (measured + 3) // 4,
            f'{label}: byte accounting or heuristic estimate differs')
    return {'selected_aliases': selected, 'canonical_result_bytes': measured,
            'relevant_returned': len(set(selected) & set(case['relevant_aliases'])),
            'relevant_defined': len(case['relevant_aliases']),
            'candidate_count': selection['candidate_count'],
            'candidate_window_truncated': selection['candidate_window_truncated'],
            'omitted_by_budget': selection['omitted_by_budget'],
            'omitted_by_record_limit': selection['omitted_by_record_limit'],
            'selection_complete': selection['selection_complete']}


def evaluate(binary):
    require(FIXTURE.is_file() and not FIXTURE.is_symlink(), 'Fixture must be a regular source file')
    with FIXTURE.open('rb') as stream:
        raw_fixture = stream.read(1024 * 1024 + 1)
    require(len(raw_fixture) <= 1024 * 1024, 'Fixture file exceeds its size bound')
    fixture = validate_fixture(json.loads(raw_fixture))
    spec = importlib.util.spec_from_file_location('retrieval_smoke_client', ROOT / 'scripts/smoke-test.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    with tempfile.TemporaryDirectory(prefix='project-memory-retrieval-eval-') as temporary:
        folder = Path(temporary)
        client = module.Client(binary, folder / 'synthetic.sqlite', folder)
        try:
            initialized = client.request('initialize', {'protocolVersion': '2025-11-25', 'capabilities': {},
                'clientInfo': {'name': 'project-memory-retrieval-evaluation', 'version': '1'}})
            client.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
            server = initialized.get('serverInfo')
            require(isinstance(server, dict), 'Initialize did not identify the server')
            text(server.get('name'), 'server identity name', 200)
            text(server.get('version'), 'server identity version', 200)
            tools = client.request('tools/list', {}).get('tools', [])
            bundle = next((tool for tool in tools if tool.get('name') == 'get_context_bundle'), {})
            require('query_mode' in bundle.get('inputSchema', {}).get('properties', {}),
                    'The trusted binary does not expose terms retrieval; build the matching source version')
            def tool(operation, arguments):
                raw = client.request('tools/call', {'name': operation, 'arguments': arguments})
                require(raw.get('isError') is False and isinstance(raw.get('structuredContent'), dict),
                        f'Synthetic fixture setup failed at {operation}')
                return raw['structuredContent']
            projects, originals, ids, aliases = {}, {}, {}, {}
            for item in fixture['projects']:
                project = tool('create_project', {'name': item['name'], 'purpose': item['purpose']})
                validate_project_provenance(project)
                require(project['name'] == item['name'] and project['purpose'] == item['purpose']
                        and project['directory'] is None, 'Project setup changed supplied fields')
                require(all(project['id'] != previous['id'] for previous in projects.values()),
                        'Project setup returned a duplicate identity')
                projects[item['alias']] = project
            operations = {'note': 'save_note', 'decision': 'record_decision', 'handoff': 'save_handoff'}
            for item in fixture['records']:
                arguments = {**item['data'], 'project_id': projects[item['project']]['id']}
                if 'supersedes' in item:
                    arguments['supersedes'] = ids[item['supersedes']]
                record = tool(operations[item['kind']], arguments)
                validate_record_provenance(record)
                require(all(record.get(key) == value for key, value in arguments.items()),
                        'Fixture setup did not preserve complete original fields')
                require(record.get('kind') == item['kind'] and record.get('superseded_by') is None,
                        'Fixture setup returned an incorrect record kind or history state')
                require(isinstance(record.get('id'), str) and record['id'] not in originals, 'Fixture setup returned duplicate or missing record ID')
                ids[item['alias']] = record['id']
                aliases[record['id']] = item['alias']
                originals[record['id']] = record
                if 'supersedes' in item:
                    originals[ids[item['supersedes']]]['superseded_by'] = record['id']
            observations = []
            for case in fixture['cases']:
                modes = {}
                for mode in ('literal', 'terms'):
                    arguments = {'project_id': projects[case['project']]['id'], 'query': case['query'],
                                 'max_bytes': case['max_bytes'], 'max_records': case['max_records']}
                    if mode == 'terms':
                        arguments['query_mode'] = mode
                    raw = client.request('tools/call', {'name': 'get_context_bundle', 'arguments': arguments})
                    modes[mode] = check_result(raw, case, mode, projects[case['project']], originals, aliases)
                observations.append({'name': case['name'], 'query': case['query'],
                    'max_bytes': case['max_bytes'], 'max_records': case['max_records'], 'modes': modes})
            report = {'status': 'passed', 'fixture_version': fixture['fixture_version'],
                'fixture_name': fixture['name'], 'fixture_sha256': hashlib.sha256(raw_fixture).hexdigest(),
                'server': server, 'synthetic_projects': len(projects), 'synthetic_records': len(originals),
                'protocol_requests': client.sequence, 'cases': observations,
                'measurement': 'Compact UTF-8 canonical MCP tool result, including compatibility text; missing legacy resultType restored as complete. Excludes JSON-RPC envelope.',
                'scope': 'Versioned synthetic protocol and deterministic-selection regression. Byte counts may vary with server timestamps. No model usage, billed-token savings, cost savings or universal relevance claim.'}
        finally:
            try:
                client.close()
            finally:
                if client.process.poll() is None:
                    client.process.kill()
                    client.process.wait()
        require(client.process.returncode == 0, 'Synthetic MCP server did not exit successfully')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path, help='Trusted built MCP executable; evaluation always uses its own temporary archive')
    args = parser.parse_args()
    try:
        binary = args.binary.resolve(strict=True)
        require(binary.is_file() and os.access(binary, os.X_OK), 'Binary must be an executable regular file')
        report = evaluate(binary)
    except (ValueError, OSError, AssertionError, KeyError, TypeError) as error:
        parser.exit(1, f'Retrieval evaluation failed: {error}\n')
    print(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
