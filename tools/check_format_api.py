#!/usr/bin/env python3
"""Exercise dated complete matches through the shared JSON match coordinator.

The Irelia pairs compare snapshot and pre-ban behavior. Future-date Rengar
fixtures retain the preset main but use explicitly authored legal registrations;
they are not new tournament presets or evidence of AI playing strength.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess

from series_bot import run_match


def checked_match(binary, registrations, seed, level, chooser, date):
    digest = hashlib.sha256()
    phases = set()
    observations = 0

    def trace(request, response):
        nonlocal observations
        result = response.get('result')
        if request['command'] == 'observe':
            observations += 1
            phases.add(result['series']['phase'])
            if date is None:
                assert 'format' not in result
            else:
                assert result['format'] == {'scope': 'global_standard_1v1', 'as_of': date}
        payload, reply = copy.deepcopy([request, response])
        payload.pop('format_date', None)
        if isinstance(reply.get('result'), dict):
            reply['result'].pop('format', None)
        digest.update(json.dumps([payload, reply], sort_keys=True, separators=(',', ':')).encode())

    result = run_match(binary, registrations, seed, level, chooser, trace=trace,
                       setup_policies=['adaptive', 'adaptive'], format_date=date)
    assert phases == {'battlefields', 'play_order', 'game', 'sideboarding', 'complete'}
    assert max(result['wins']) == 2 and len(result['history']) >= 2
    return {'format_date': date, 'result': result, 'observations': observations,
            'phases': sorted(phases), 'trace_sha256': digest.hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--baseline', type=Path, help='Optional earlier Irelia trace report')
    args = parser.parse_args()
    binary = args.binary.resolve()
    baseline = json.loads(args.baseline.read_text()) if args.baseline else None
    report = {'status': 'running', 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'pairs': [], 'future': []}

    def save():
        # One complete record set lets the installed-release runner compare all
        # nine matches without executing this same matrix separately per group.
        report['records'] = [run for pair in report['pairs'] for run in pair['runs']] + report['future']
        args.output.write_text(json.dumps(report, indent=2) + '\n')

    save()
    for index, level in enumerate(['easy', 'tactical', 'strategic']):
        runs = [checked_match(binary, [{'preset': 0}, {'preset': 5}], 112200 + index,
                              level, index % 2, date) for date in [None, '2026-11-22']]
        assert runs[0]['trace_sha256'] == runs[1]['trace_sha256']
        if baseline:
            old = baseline['pairs'][index]
            assert old['difficulty'] == level
            for current, previous in zip(runs, old['runs'], strict=True):
                assert all(current[key] == previous[key] for key in current), level
        report['pairs'].append({'difficulty': level,
                               'equal_full_traces_after_exact_format_projection': True, 'runs': runs})
        save()
        print('Passed paired Irelia matches:', level, flush=True)

    response = subprocess.run([str(binary)], input='{"command":"catalog"}\n',
                              text=True, capture_output=True, check=True, timeout=30)
    catalog = json.loads(response.stdout)['result']
    preset = catalog['presets'][4]
    runes = [0] * 6
    for domain, count in zip(preset['domains'], preset['rune_counts'], strict=True):
        runes[catalog['domains'].index(domain)] = count
    registration = {'deck': {'name': 'Rengar / dated format verification',
                             'legend': preset['legend_id'], 'chosen_champion': preset['chosen_champion'],
                             'cards': preset['cards'], 'runes': runes, 'battlefield': 0},
                    'sideboard': [row for row in preset['registration']['sideboard'] if row[0] != 51],
                    'battlefields': [0, 13, 14]}
    for index, level in enumerate(['easy', 'tactical', 'strategic']):
        result = checked_match(binary, [registration, registration], 112300 + index,
                               level, index % 2, '2026-11-23')
        report['future'].append(result)
        save()
        print('Passed future-date authored Rengar match:', level, flush=True)
    report['status'] = 'passed'
    report['baseline_traces_preserved'] = baseline is not None
    save()


if __name__ == '__main__':
    main()
