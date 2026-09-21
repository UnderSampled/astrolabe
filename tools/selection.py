"""Reviewed inventory and fail-closed selection; no wildcard classifications."""
import math


def identity(row):
    return row['binary'] + '/' + row['test']


def validate_inventory(manifest, inventory):
    if manifest.get('schema') != 1:
        raise ValueError('unsupported test manifest schema')
    declared = {}
    for row in manifest['tests']:
        key = identity(row)
        if key in declared:
            raise ValueError(f'duplicate classification: {key}')
        if not row['areas'] or set(row['areas']) - set(manifest['areas']):
            raise ValueError(f'invalid areas: {key}')
        if not row['release'] or not row['fixture'] or not row['kind']:
            raise ValueError(f'missing release/fixture/kind classification: {key}')
        if not math.isfinite(row['timeout_seconds']) or row['timeout_seconds'] <= 0:
            raise ValueError(f'invalid deadline: {key}')
        if set(row['prerequisites']) - {'godot', 'disc', 'liblzo2', 'ffmpeg', 'original'}:
            raise ValueError(f'unknown prerequisite: {key}')
        if row.get('safeguard') not in (None, 'always', 'gameplay'):
            raise ValueError(f'unknown safeguard policy: {key}')
        declared[key] = row
    found = {}
    for binary, suite in inventory['rust-suites'].items():
        if suite['status'] != 'listed':
            raise ValueError(f'test binary was not discovered: {binary}')
        for name, test in suite['testcases'].items():
            found[binary + '/' + name] = test
    unknown = sorted(found.keys() - declared.keys())
    missing = sorted(declared.keys() - found.keys())
    if unknown or missing:
        raise ValueError(f'unclassified tests: {unknown}; missing declared tests: {missing}')
    if len(found) != inventory['test-count']:
        raise ValueError('inventory test count does not agree with discovered cases')
    for key, test in found.items():
        if test['ignored'] != declared[key]['ignored']:
            raise ValueError(f'ignored status changed without classification review: {key}')
    return declared


def select(manifest, areas=(), local=False, release=False):
    wanted = set(areas)
    if wanted - set(manifest['areas']):
        raise ValueError(f'unknown areas: {sorted(wanted - set(manifest["areas"]))}')
    while True:
        expanded = wanted | {dep for area in wanted
                             for dep in manifest['dependencies'].get(area, [])}
        if expanded == wanted:
            break
        wanted = expanded
    if areas and not any(wanted.intersection(r['areas'])
                         for r in manifest['tests'] + manifest['checks']):
        raise ValueError('requested area contains no classified tests')
    gameplay = bool(wanted.intersection({'movement', 'ai', 'animation'})) or not areas
    selected, excluded = [], []
    for row in manifest['tests']:
        reason = None
        if release:
            reason = 'complete release inventory'
        elif row.get('safeguard') == 'always':
            reason = 'mandatory safeguard'
        elif row.get('safeguard') == 'gameplay' and gameplay:
            reason = 'mandatory actual-input gameplay safeguard'
        elif areas and not wanted.intersection(row['areas']):
            excluded.append(dict(row, reason='unaffected area'))
            continue
        elif row['ignored'] and not local:
            excluded.append(dict(row, reason='local capability not selected'))
            continue
        else:
            reason = 'selected area or dependency' if areas else 'routine suite'
        selected.append(dict(row, reason=reason))
    if not selected:
        raise ValueError('empty test selection')
    return selected, excluded


def expression(rows):
    if not rows:
        raise ValueError('empty executable selection')
    # Names originate in compiled discovery and exact manifest entries. Quoting
    # follows nextest filter DSL, not a shell or fuzzy test-name substring.
    def quote(s):
        import re
        if not re.fullmatch(r'[A-Za-z0-9_:./-]+', s):
            raise ValueError(f'unsupported test identifier in filter: {s!r}')
        return '=' + s
    return ' | '.join(f'(binary_id({quote(r["binary"])}) & test({quote(r["test"])}))'
                      for r in rows)
