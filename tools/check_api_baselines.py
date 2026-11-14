#!/usr/bin/env python3
"""Collect native API baselines using the API cases from a release plan.

Existing baselines are compared in full for review, not silently overwritten.
Changed records do not imply a regression or an improvement; inspect the report
before accepting these results for an immutable packaged release.
"""
import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import time

from check_release import digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    cases = json.loads(args.plan.read_text())['api']
    names = [case['name'] for case in cases]
    if not cases or len(set(names)) != len(names):
        parser.error('API cases must be nonempty and have unique names.')
    for case in cases:
        if not re.fullmatch(r'[a-z0-9_-]+', case['name']):
            parser.error('Case names must be simple filenames.')
        if not re.fullmatch(r'check_[a-z0-9_]+\.py', case['script']):
            parser.error('Cases must name a check script in tools/.')
        script = (root / 'tools' / case['script']).resolve(strict=True)
        if not script.is_relative_to(root / 'tools'):
            parser.error('Check scripts must remain in tools/.')
        if not re.fullmatch(r'[a-z_]+', case['records_key']):
            parser.error('Cases must name their result records key.')
    binary = args.binary.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    (output / 'plan.json').write_text(json.dumps({'api': cases}, indent=2) + '\n')
    report = {'status': 'running', 'binary': str(binary), 'binary_sha256': digest(binary),
              'plan_sha256': digest(args.plan), 'runner_sha256': digest(Path(__file__)), 'cases': []}

    def save():
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for case in cases:
            name, key = case['name'], case['records_key']
            script = root / 'tools' / case['script']
            script_hash = digest(script)
            prior = None
            if case.get('baseline'):
                prior_path = Path(case['baseline'])
                prior_bytes = prior_path.read_bytes()
                prior = json.loads(prior_bytes)
                if prior.get('status') != 'passed' or not prior.get(key):
                    raise AssertionError(f'{name}: prior baseline did not pass or has no records.')
                (output / (name + '-prior.json')).write_bytes(prior_bytes)
            result = output / (name + '.json')
            started = time.monotonic()
            with (output / (name + '.log')).open('w') as log:
                subprocess.run([sys.executable, str(script), str(binary), str(result)],
                               cwd=root, stdout=log, stderr=subprocess.STDOUT,
                               check=True, timeout=1200)
            if digest(binary) != report['binary_sha256'] or digest(script) != script_hash:
                raise AssertionError('Binary or check script changed during verification.')
            actual = json.loads(result.read_text())
            if actual.get('status') != 'passed' or not isinstance(actual.get(key), list) or not actual[key]:
                raise AssertionError(f'{name}: check did not produce passing nonempty records.')
            equal = actual[key] == prior[key] if prior is not None else None
            report['cases'].append({'name': name, 'script': case['script'], 'script_sha256': script_hash,
                                    'records_key': key, 'records': len(actual[key]), 'baseline': str(result),
                                    'sha256': digest(result), 'full_records_equal_to_prior': equal,
                                    'seconds': round(time.monotonic() - started, 3)})
            save()
            print(name, 'passed', len(actual[key]), 'records; equals prior:', equal, flush=True)
        report['status'] = 'passed'
    except BaseException as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
