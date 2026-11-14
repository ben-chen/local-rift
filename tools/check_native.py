#!/usr/bin/env python3
"""Run selected native UI cases from a release plan against an existing binary."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time

from check_release import digest, require_executable, verify_ui_output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--case', action='append', help='Select a named case; repeat as needed.')
    parser.add_argument('--xvfb', default='xvfb-run')
    args = parser.parse_args()
    cases = json.loads(args.plan.read_text())['ui']
    names = [case['name'] for case in cases]
    if len(names) != len(set(names)):
        parser.error('Case names must be unique.')
    if args.case and not set(args.case).issubset(names):
        parser.error('A selected case does not exist in the plan.')
    cases = [case for case in cases if not args.case or case['name'] in args.case]
    if not cases:
        parser.error('Select at least one UI case.')
    for case in cases:
        if not re.fullmatch(r'[a-z0-9_-]+', case['name']):
            parser.error('Case names must be simple filenames.')
        argv = case['args']
        if not argv or not re.fullmatch(r'--ui-smoke(?:-[a-z0-9-]+)?', argv[0]) or argv[1:] not in ([], ['--small']):
            parser.error('Use a native smoke mode and optional --small.')
        captures = case.get('captures', [])
        if len(set(captures)) != len(captures) or any(not re.fullmatch(r'RIFT_[A-Z0-9_]+', key) for key in captures):
            parser.error('Invalid capture variable.')
        if any(not re.fullmatch(r'RIFT_[A-Z0-9_]+', key) or not isinstance(value, str)
               for key, value in case.get('env', {}).items()):
            parser.error('Only RIFT_ string settings may override the isolated environment.')
    xvfb = require_executable(args.xvfb)
    binary = args.binary.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'status': 'running', 'binary': str(binary), 'binary_sha256': digest(binary),
              'plan_sha256': digest(args.plan), 'runner_sha256': digest(Path(__file__)), 'cases': []}
    (output / 'plan.json').write_text(json.dumps({'ui': cases}, indent=2) + '\n')

    def save():
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')

    save()
    try:
        for case in cases:
            folder = output / case['name']
            folder.mkdir()
            env = {key: value for key, value in os.environ.items() if not key.startswith('RIFT_')}
            env.update(LIBGL_ALWAYS_SOFTWARE='1', LD_LIBRARY_PATH='/usr/lib/x86_64-linux-gnu',
                       XDG_DATA_HOME=str(folder / 'data'), XDG_CONFIG_HOME=str(folder / 'config'),
                       XDG_CACHE_HOME=str(folder / 'cache'))
            env.update(case.get('env', {}))
            env.update({key: str(folder / (key.lower() + '.png')) for key in case.get('captures', [])})
            started = time.monotonic()
            with (folder / 'run.log').open('w') as log:
                subprocess.run([xvfb, '-a', '-s', '-screen 0 1600x1000x24', str(binary), *case['args']],
                               cwd=Path(__file__).resolve().parents[1], env=env,
                               stdout=log, stderr=subprocess.STDOUT, check=True, timeout=300)
            if digest(binary) != report['binary_sha256']:
                raise AssertionError('Binary changed during verification.')
            report['cases'].append({'name': case['name'], 'seconds': round(time.monotonic()-started, 3),
                                    **verify_ui_output(folder, case)})
            save()
            print(case['name'], 'passed', flush=True)
        report['status'] = 'passed'
    except BaseException as error:
        report.update(status='failed', error=str(error))
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
