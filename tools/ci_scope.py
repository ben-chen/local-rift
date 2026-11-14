"""Select full verification unless a complete diff contains only documentation.

Keep this allowlist narrow. CARD_COVERAGE.md is generated catalog output, so it
always requires the full checks. Unknown paths and unavailable history do too.
"""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess


def documentation_only(paths):
    if not paths:
        return False
    for name in paths:
        path = PurePosixPath(name)
        if name in ('README.md', 'LICENSE'):
            continue
        if name == 'docs/CARD_COVERAGE.md' or path.parts[0] != 'docs':
            return False
        if path.suffix not in ('.md', '.png', '.jpg', '.svg') and not (
            path.parent == PurePosixPath('docs/validation') and path.suffix == '.json'
        ):
            return False
    return True


def changed_paths(root, base, head):
    # GitHub supplies hashes; never interpret an event value as a Git option/ref.
    if not all(re.fullmatch(r'[0-9a-f]{40}|[0-9a-f]{64}', sha or '') for sha in (base, head)):
        return None
    if not base.strip('0'):
        return None
    result = subprocess.run(
        ['git', 'diff', '--no-renames', '--name-only', '-z', base, head, '--'],
        cwd=root, capture_output=True,
    )
    if result.returncode:
        return None
    return [os.fsdecode(name) for name in result.stdout.split(b'\0') if name]


def validate_documents(root, paths):
    for name in paths:
        path = root / name
        if name.startswith('docs/validation/') and path.suffix == '.json' and path.is_file():
            json.loads(path.read_text())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', help='Compare this commit with --head; missing history runs full checks')
    parser.add_argument('--head', default='HEAD')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    base = args.base
    if base is None and os.environ.get('GITHUB_EVENT_PATH'):
        event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
        if os.environ.get('GITHUB_EVENT_NAME') == 'pull_request':
            base = event.get('pull_request', {}).get('base', {}).get('sha')
        elif os.environ.get('GITHUB_EVENT_NAME') == 'push':
            base = event.get('before')
    head = subprocess.check_output(['git', 'rev-parse', '--verify', args.head + '^{commit}'], cwd=root, text=True).strip()
    paths = changed_paths(root, base, head)
    full = paths is None or not documentation_only(paths)
    if paths is not None:
        subprocess.run(['git', 'diff', '--check', base, head, '--'], cwd=root, check=True)
        validate_documents(root, paths)
    result = {'full_checks': full, 'head': head, 'base': base, 'changed_paths': paths,
              'reason': 'Code/data/build changes or unavailable history' if full else 'Documentation only; JSON and whitespace checked'}
    print(json.dumps(result, indent=2))
    if os.environ.get('GITHUB_OUTPUT'):
        with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
            output.write(f'full_checks={str(full).lower()}\n')


if __name__ == '__main__':
    main()
