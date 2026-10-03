"""Selection, inventory and evidence negative controls."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

from selection import expression, identity, select, validate_inventory
from suites import junit_results, source_digest


class SelectionTests(unittest.TestCase):
    def test_source_freeze_covers_authored_game_and_harness_edits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ('Cargo.toml', 'Cargo.lock', 'godot/project.godot',
                         'godot/game/session.gd', 'godot/tests/menu.gd'):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('before')
            before = source_digest(root)
            (root / 'godot/game/session.gd').write_text('after')
            game_changed = source_digest(root)
            self.assertNotEqual(before, game_changed)
            (root / 'godot/tests/menu.gd').write_text('after')
            self.assertNotEqual(game_changed, source_digest(root))

    def setUp(self):
        self.manifest = json.loads(Path(__file__).with_name('test-manifest.json').read_text())
        suites = {}
        for row in self.manifest['tests']:
            suites.setdefault(row['binary'], {'status': 'listed', 'testcases': {}})['testcases'][row['test']] = {'ignored': row['ignored']}
        self.inventory = {'rust-suites': suites, 'test-count': len(self.manifest['tests'])}

    def test_every_area_keeps_required_safeguards_and_accounts_for_every_case(self):
        required = {identity(r) for r in self.manifest['tests'] if r.get('safeguard') == 'always'}
        self.assertGreaterEqual(len(required), 8)
        self.assertTrue(any(r['test'] == 'synthetic_godot_contract' and r.get('safeguard') == 'always'
                            for r in self.manifest['tests']))
        for area in self.manifest['areas']:
            selected, excluded = select(self.manifest, [area])
            keys = {identity(r) for r in selected}
            self.assertTrue(required <= keys)
            self.assertFalse(keys.intersection(identity(r) for r in excluded))
            self.assertEqual(len(selected)+len(excluded), len(self.manifest['tests']))
        selected, _ = select(self.manifest, ['movement'])
        self.assertIn('native_motion_survives_rete_and_drives_character_contact', [r['test'] for r in selected])

    def test_release_covers_every_declared_test_including_local_cases(self):
        selected, excluded = select(self.manifest, release=True)
        self.assertEqual(len(selected), len(self.manifest['tests']))
        self.assertFalse(excluded)
        self.assertTrue(any(r['ignored'] for r in selected))
        validate_inventory(self.manifest, self.inventory)

    def test_unclassified_missing_duplicate_and_ignored_changes_fail(self):
        broken = copy.deepcopy(self.inventory)
        broken['rust-suites']['astrolabe']['testcases']['new_test'] = {'ignored': False}
        with self.assertRaisesRegex(ValueError, 'unclassified'):
            validate_inventory(self.manifest, broken)
        broken = copy.deepcopy(self.inventory)
        broken['rust-suites'].pop('astrolabe')
        with self.assertRaisesRegex(ValueError, 'missing declared'):
            validate_inventory(self.manifest, broken)
        broken = copy.deepcopy(self.manifest)
        broken['tests'].append(broken['tests'][0])
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            validate_inventory(broken, self.inventory)
        broken = copy.deepcopy(self.inventory)
        first = self.manifest['tests'][0]
        broken['rust-suites'][first['binary']]['testcases'][first['test']]['ignored'] ^= True
        with self.assertRaisesRegex(ValueError, 'ignored status'):
            validate_inventory(self.manifest, broken)

    def test_invalid_area_deadline_and_empty_inventory_cannot_pass(self):
        with self.assertRaisesRegex(ValueError, 'unknown areas'):
            select(self.manifest, ['typo'])
        with self.assertRaisesRegex(ValueError, 'empty'):
            expression([])
        broken = copy.deepcopy(self.manifest)
        broken['tests'][0]['timeout_seconds'] = float('nan')
        with self.assertRaisesRegex(ValueError, 'deadline'):
            validate_inventory(broken, self.inventory)
        with self.assertRaises(ValueError):
            validate_inventory(self.manifest, {'rust-suites': {}, 'test-count': 0})

    def test_missing_skipped_duplicate_and_failure_results_do_not_pass(self):
        selected = [{'binary': 'fixture', 'test': 'one'}, {'binary': 'fixture', 'test': 'two'}]
        with tempfile.TemporaryDirectory() as directory:
            p = Path(directory)/'junit.xml'
            p.write_text('<testsuites><testcase classname="fixture" name="one" time="0.1"><failure/></testcase></testsuites>')
            result = junit_results(p, selected)
            self.assertEqual(result['fixture/one']['status'], 'failed')
            self.assertEqual(result['fixture/two']['status'], 'not_run')
            p.write_text('<testsuites><testcase classname="fixture" name="one"><skipped/></testcase></testsuites>')
            self.assertEqual(junit_results(p, selected)['fixture/one']['status'], 'not_run')
            p.write_text('<testsuites>'+2*'<testcase classname="fixture" name="one"/>'+'</testsuites>')
            with self.assertRaisesRegex(ValueError, 'duplicate'):
                junit_results(p, selected)


if __name__ == '__main__':
    unittest.main()
