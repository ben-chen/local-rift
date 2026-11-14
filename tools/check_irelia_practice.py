#!/usr/bin/env python3
"""Focused Irelia rules, opponent and companion checks; optionally API matches.

This is an iteration check, not full regression or installed UI acceptance.
Every run keeps a new output directory, the actual test list, decision misses,
logs and source identity. Cargo outputs stay inside this worktree.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
TARGETS = ['--lib', '--bin', 'local-rift', '--test', 'irelia_decisions',
           '--test', 'c4_play', '--test', 'irelia_arito', '--test', 'registered_matches']
FILTER = ('test(/irelia|strategic|counter/) or test(/^companion::/) or '
          'test(/^practice_benchmark::/) or '
          'binary(/^(irelia_decisions|c4_play|irelia_arito|registered_matches)$/)')
SUITES = {'local-rift', 'local-rift::bin/local-rift', 'local-rift::irelia_decisions',
          'local-rift::c4_play', 'local-rift::irelia_arito', 'local-rift::registered_matches'}


def source_identity():
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=ROOT)
    diff = git('diff', 'HEAD', '--binary')
    untracked = {}
    for name in git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0'):
        if name:
            path = ROOT / os.fsdecode(name)
            untracked[os.fsdecode(name)] = hashlib.sha256(path.read_bytes()).hexdigest()
    return {'revision': git('rev-parse', 'HEAD').decode().strip(),
            'tracked_diff_sha256': hashlib.sha256(diff).hexdigest(),
            'untracked_sha256': untracked, 'dirty': bool(diff or untracked)}


def test_summary(path):
    tree = ET.parse(path).getroot()
    cases = tree.findall('.//testcase')
    suites = {case.get('classname') for case in cases}
    names = {case.get('name') for case in cases}
    required = {'practice_benchmark::measured_irelia_decisions_and_companion',
                'practice_benchmark::waiting_for_irelia_reset_does_not_ignore_an_enemy_response'}
    if suites != SUITES or not required <= names or not cases:
        raise ValueError('Focused test selection lost a required suite or benchmark.')
    if len(cases) != int(tree.get('tests', '0')) or any(
        case.find(tag) is not None for case in cases for tag in ['failure', 'error', 'skipped']
    ) or any(int(tree.get(key, '0')) for key in ['failures', 'errors', 'skipped']):
        raise ValueError('Focused tests did not all pass.')
    return {'passed': len(cases), 'seconds': float(tree.get('time', '0')),
            'suites': sorted(suites)}


def decision_summary(report):
    rows = report['records']
    positions = {row['position'] for row in rows}
    expected = {(position, seat, level) for position in positions for seat in range(2)
                for level in ['Easy', 'Tactical', 'Strategic']}
    actual = {(row['position'], row['seat'], row['difficulty']) for row in rows}
    if not rows or len(positions) != report['positions'] or actual != expected or len(rows) != len(expected):
        raise ValueError('Decision report is incomplete or contains duplicate cases.')
    if any(type(row[key]) is not bool for row in rows for key in ['must_pass', 'success']):
        raise ValueError('Decision outcomes must be explicit booleans.')
    required = [row for row in rows if row['must_pass']]
    if not required or any(not row['success'] for row in required):
        raise ValueError('A required decision outcome failed.')
    return {'positions': len(positions), 'cases': len(rows), 'required_passed': len(required),
            'levels': [{'difficulty': level,
                        'successes': sum(row['success'] for row in rows if row['difficulty'] == level),
                        'cases': sum(row['difficulty'] == level for row in rows),
                        'misses': [{'position': row['position'], 'seat': row['seat']}
                                   for row in rows if row['difficulty'] == level and not row['success']]}
                       for level in ['Easy', 'Tactical', 'Strategic']]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='A new directory outside the checkout or under target/')
    parser.add_argument('--api', action='store_true', help='Also build the release API and run dated complete matches')
    args = parser.parse_args()
    output = args.output.resolve()
    if output.is_relative_to(ROOT) and not output.is_relative_to(ROOT / 'target'):
        parser.error('Keep reports outside the source tree or under target/.')
    output.mkdir(parents=True, exist_ok=False)
    report = {'status': 'running', 'scope': 'focused_irelia_practice', 'source': source_identity(),
              'steps': [], 'full_regression': 'not run', 'installed_ui': 'not run',
              'api': 'pending' if args.api else 'not requested',
              'limits': 'Authored decision positions; no expert-strength or human-learning claim.'}
    env = {**os.environ, 'CARGO_TARGET_DIR': str(ROOT / 'target'),
           'RIFT_IRELIA_BENCHMARK_REPORT': str(output / 'decisions.json')}

    def save():
        (output / 'status.json').write_text(json.dumps(report, indent=2) + '\n')

    def run(name, command):
        report['stage'] = name
        save()
        print(f'Running {name} (log: {output / (name + ".log")})', flush=True)
        start = time.monotonic()
        with (output / (name + '.log')).open('w') as log:
            result = subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
        report['steps'].append({'name': name, 'command': command, 'exit_code': result.returncode,
                                'seconds': round(time.monotonic() - start, 3)})
        result.check_returncode()

    save()
    try:
        config = (ROOT / '.config/nextest.toml').read_text()
        if config.count('path = "junit.xml"') != 1:
            raise ValueError('Review the nextest JUnit configuration before using this runner.')
        config = config.replace('path = "junit.xml"', 'path = ' + json.dumps(str(output / 'junit.xml')))
        (output / 'nextest.toml').write_text(config)
        run('rules-and-decisions', ['cargo', 'nextest', 'run', '--locked', *TARGETS,
                                   '--config-file', str(output / 'nextest.toml'), '--profile', 'ci',
                                   '-E', FILTER, '--status-level', 'fail', '--final-status-level', 'fail'])
        report['tests'] = test_summary(output / 'junit.xml')
        report['decisions'] = decision_summary(json.loads((output / 'decisions.json').read_text()))
        if args.api:
            run('api-build', ['cargo', 'build', '--locked', '--release', '--bin', 'rift-api'])
            run('api-matches', [sys.executable, str(ROOT / 'tools/check_format_api.py'),
                                str(ROOT / 'target/release/rift-api'), str(output / 'api.json')])
            api = json.loads((output / 'api.json').read_text())
            if api['status'] != 'passed' or not api['records']:
                raise ValueError('Complete API matches did not pass.')
            report['api'] = {'matches': len(api['records']),
                             'games': sum(len(row['result']['history']) for row in api['records']),
                             'scope': 'Paired Irelia snapshot/date matches and future-date Rengar registrations; stability, not strength.'}
        if source_identity() != report['source']:
            raise ValueError('Sources changed during the check; results are retained but not accepted.')
        report.update(status='passed', stage='complete')
    except (Exception, KeyboardInterrupt) as error:
        report.update(status='failed', error=str(error) or type(error).__name__)
        raise
    finally:
        save()
    print(json.dumps({key: report[key] for key in ['status', 'tests', 'decisions', 'api']}, indent=2))


if __name__ == '__main__':
    main()
