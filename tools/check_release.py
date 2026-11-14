#!/usr/bin/env python3
"""Build an immutable Git revision, then verify its installed UI and API.

A JSON plan supplies revision, UI argument/capture cases, and API scripts with
accepted native reports. One output directory holds live status, logs, images,
and comparisons. Existing output directories are refused to preserve evidence.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require_executable(command):
    resolved = shutil.which(command)
    if resolved is None:
        raise FileNotFoundError(f'Required release-check executable is unavailable: {command}')
    return resolved


def validate_plan(plan):
    if not re.fullmatch(r'[0-9a-f]{40}', plan.get('revision', '')):
        raise ValueError('The plan must pin a complete Git commit SHA.')
    names = set()
    for group in ('ui', 'api'):
        if not isinstance(plan.get(group), list) or not plan[group]:
            raise ValueError(f'The plan must include at least one {group} check.')
        for case in plan[group]:
            name = case.get('name', '')
            if not re.fullmatch(r'[a-z0-9_-]+', name) or name in names:
                raise ValueError('Check names must be unique simple filenames.')
            names.add(name)
            if group == 'ui':
                argv = case.get('args', [])
                if not argv or not re.fullmatch(r'--ui-smoke(?:-[a-z0-9-]+)?', argv[0]) or argv[1:] not in ([], ['--small']):
                    raise ValueError(f'{name}: expected a native UI smoke mode and optional --small.')
                captures = case.get('captures', [])
                if len(set(captures)) != len(captures) or any(not re.fullmatch(r'RIFT_[A-Z0-9_]+', key) for key in captures):
                    raise ValueError(f'{name}: capture names must be distinct RIFT_ variables.')
                env = case.get('env', {})
                if any(not re.fullmatch(r'RIFT_[A-Z0-9_]+', key) or not isinstance(value, str) for key, value in env.items()):
                    raise ValueError(f'{name}: only string RIFT_ settings may override the isolated environment.')
                minimum = case.get('minimum_games', 0)
                if type(minimum) is not int or minimum < 0 or (argv[0] == '--ui-smoke-series' and minimum < 2):
                    raise ValueError(f'{name}: series checks must require at least two actual games.')
            else:
                if not re.fullmatch(r'check_[a-z0-9_]+\.py', case.get('script', '')):
                    raise ValueError(f'{name}: expected a check script in tools/.')
                if not re.fullmatch(r'[a-z_]+', case.get('records_key', '')):
                    raise ValueError(f'{name}: missing result records key.')
                baseline = json.loads(Path(case['baseline']).read_text())
                if baseline.get('status') != 'passed' or not baseline.get(case['records_key']):
                    raise ValueError(f'{name}: baseline must contain passing nonempty records.')


def compare_records(native, installed, key):
    if native.get('status') != 'passed' or installed.get('status') != 'passed':
        raise AssertionError('Baseline or installed report did not pass.')
    if not native.get(key) or native[key] != installed.get(key):
        raise AssertionError('Full native and packaged records differ.')


def verify_ui_output(folder, case):
    log = (folder / 'run.log').read_text()
    if 'UI smoke: all native click checks passed.' not in log:
        raise AssertionError('Client exited without completing the native UI checks.')
    images = {path.name: path for path in folder.glob('*.png')}
    expected = {key.lower() + '.png' for key in case.get('captures', [])}
    if set(images) != expected:
        raise AssertionError('Native UI capture names differ from the planned captures.')
    for path in images.values():
        if not path.read_bytes().startswith(b'\x89PNG\r\n\x1a\n'):
            raise AssertionError(f'{path.name}: not a PNG capture.')
    counts = re.findall(r'Native UI completed (\d+) actual games', log)
    games = int(counts[-1]) if counts else 0
    if games < case.get('minimum_games', 0):
        raise AssertionError('Native UI did not complete the required actual games.')
    return {'captures': len(images), 'games': games,
            'capture_sha256': {name: digest(path) for name, path in images.items()},
            'log_sha256': digest(folder / 'run.log')}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--xvfb', default='xvfb-run')
    args = parser.parse_args()
    plan = json.loads(args.plan.read_text())
    validate_plan(plan)
    # Detect a missing display/build tool before spending time on the package.
    args.xvfb = require_executable(args.xvfb)
    nix = require_executable('nix')
    nix_store = require_executable('nix-store')
    require_executable('git')
    revision = plan['revision']
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    (output / 'plan.json').write_text(json.dumps(plan, indent=2) + '\n')
    for case in plan['api']:
        (output / (case['name'] + '-baseline.json')).write_bytes(Path(case['baseline']).read_bytes())
    repository = Path(subprocess.check_output(
        ['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'], text=True,
    ).strip()).parent
    uri = f'git+{repository.as_uri()}?rev={revision}'
    report = {'status': 'running', 'stage': 'nix', 'pid': os.getpid(),
              'revision': revision, 'plan_sha256': digest(args.plan),
              'runner_sha256': digest(Path(__file__)),
              'started_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'ui': [], 'api': []}

    def save():
        temporary = output / 'status.tmp'
        temporary.write_text(json.dumps(report, indent=2) + '\n')
        temporary.replace(output / 'status.json')

    def run(command, log, *, cwd=None, env=None, timeout=None, capture=False):
        started = time.monotonic()
        with log.open('w') as stream:
            result = subprocess.run(command, cwd=cwd, env=env,
                                    stdin=subprocess.DEVNULL,
                                    stdout=subprocess.PIPE if capture else stream,
                                    stderr=stream if capture else subprocess.STDOUT,
                                    text=True, timeout=timeout)
        if result.returncode:
            raise RuntimeError(f'{log.name}: command exited {result.returncode}')
        return result.stdout, round(time.monotonic() - started, 3)

    save()
    try:
        display = Path(args.xvfb).resolve()
        if display.is_relative_to('/nix/store'):
            display_store_path = Path(*display.parts[:4])
            run([nix_store, '--add-root', str(output / 'display-tool'), '--indirect',
                 '--realise', str(display_store_path)], output / 'display-root.log')
        # Metadata may return a cached path that is no longer in the store.
        # Archive explicitly materializes the source before registering its root.
        metadata = json.loads(subprocess.check_output(
            [nix, 'flake', 'archive', '--json', uri], text=True,
        ))
        source = Path(metadata['path'])
        # The filtered package inputs do not retain the complete flake source.
        # Keep it alive for the later API scripts and UI working directory.
        source_root = output / 'source'
        run([nix_store, '--add-root', str(source_root), '--indirect', '--realise', str(source)],
            output / 'source-root.log')
        if source_root.resolve(strict=True) != source.resolve(strict=True):
            raise RuntimeError('The source GC root does not retain the tested revision.')
        report['source_gc_root'] = str(source_root)
        save()
        checks = ['package', 'catalog', 'simulation']
        result, seconds = run(
            [nix, 'build', '--out-link', str(output / 'nix-result'), '--json', '--print-build-logs',
             '--cores', '2', '--max-jobs', '2']
            + [f'{uri}#checks.x86_64-linux.{check}' for check in checks],
            output / 'nix.log', capture=True,
        )
        builds = json.loads(result)
        (output / 'nix-builds.json').write_text(json.dumps(builds, indent=2) + '\n')
        paths = [item['outputs']['out'] for item in builds]
        catalog, = [path for path in paths if path.endswith('-local-rift-catalog-check')]
        simulation, = [path for path in paths if path.endswith('-local-rift-simulation-check')]
        package, = [path for path in paths if path not in (catalog, simulation)]
        package = Path(package)
        report['nix'] = {'package': str(package), 'catalog': catalog,
                         'simulation': simulation, 'seconds': seconds,
                         'gc_root_prefix': str(output / 'nix-result')}
        report['source'] = str(source)
        report['stage'] = 'installed_ui'
        save()
        for case in plan['ui']:
            name = case['name']
            report['stage'] = 'installed_ui:' + name
            save()
            folder = output / name
            folder.mkdir()
            env = {key: value for key, value in os.environ.items()
                   if key != 'LD_LIBRARY_PATH' and not key.startswith('RIFT_')}
            env.update(LIBGL_ALWAYS_SOFTWARE='1', XDG_DATA_HOME=str(folder / 'data'),
                       XDG_CONFIG_HOME=str(folder / 'config'), XDG_CACHE_HOME=str(folder / 'cache'))
            env.update(case.get('env', {}))
            captures = case.get('captures', [])
            env.update({key: str(folder / (key.lower() + '.png')) for key in captures})
            _, seconds = run(
                [args.xvfb, '-a', '-s', '-screen 0 1600x1000x24',
                 str(package / 'bin/local-rift'), *case['args']],
                folder / 'run.log', cwd=source, env=env, timeout=300,
            )
            report['ui'].append({'name': name, 'seconds': seconds, **verify_ui_output(folder, case),
                                 'host_library_path_removed': True})
            save()
        report['stage'] = 'installed_api'
        save()
        for case in plan['api']:
            name = case['name']
            report['stage'] = 'installed_api:' + name
            save()
            script = (source / 'tools' / case['script']).resolve()
            if not script.is_relative_to(source / 'tools') or not script.is_file():
                raise ValueError('API runner must be a tracked tool in the tested revision')
            baseline = output / (name + '-baseline.json')
            native = json.loads(baseline.read_text())
            result = output / (name + '.json')
            _, seconds = run([sys.executable, str(script), str(package / 'bin/rift-api'), str(result)],
                             output / (name + '.log'), cwd=source, timeout=1200)
            installed = json.loads(result.read_text())
            key = case['records_key']
            compare_records(native, installed, key)
            report['api'].append({'name': name, 'seconds': seconds, 'records': len(installed[key]),
                                  'records_key': key, 'full_records_equal': True,
                                  'baseline_sha256': digest(baseline), 'report_sha256': digest(result)})
            save()
        report['status'] = 'passed'
        report['stage'] = 'complete'
    except BaseException as error:
        report['status'] = 'failed'
        report['error'] = repr(error)
    finally:
        report['finished_at'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        save()
    print(json.dumps(report, indent=2), flush=True)
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
