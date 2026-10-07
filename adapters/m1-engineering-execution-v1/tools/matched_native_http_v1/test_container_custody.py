import importlib.util
import json
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('container_custody_test', Path(__file__).with_name('container_custody.py'))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def fixture():
    name = 'ferric-matched128-vllm-' + 'a' * 16
    intent = {'name': name, 'image': 'sha256:' + 'b' * 64,
              'label_key': 'ferric.matched128', 'label_value': name}
    value = [{'Id': 'c' * 64, 'Name': '/' + name, 'Image': intent['image'], 'Created': 'fixed',
              'Config': {'Labels': {intent['label_key']: name}}, 'State': {'Running': True}}]
    return intent, value


class ContainerTests(unittest.TestCase):
    def test_foreign_identity_never_authorizes_cleanup(self):
        for key in ('Id', 'Name', 'Image', 'Created'):
            intent, value = fixture()
            retained = m.observe(value, intent)
            value[0][key] = 'changed'
            with self.assertRaises(ValueError):
                m.observe(value, intent, retained)

    def test_ownership_label_required(self):
        intent, value = fixture()
        value[0]['Config']['Labels'] = {}
        with self.assertRaises(ValueError):
            m.observe(value, intent)

    def test_never_created_or_already_absent_has_no_mutation(self):
        intent, _ = fixture()
        calls = []
        def command(argv, label, **_):
            calls.append(argv)
            return 0, b'', b''
        result = m.retire(command, intent)
        self.assertTrue(result['absent'])
        self.assertFalse(result['stop_sent'])
        self.assertEqual(len(calls), 1)

    def test_outer_fallback_reclaims_exact_create_before_receipt(self):
        intent, value = fixture()
        calls = []
        def command(argv, label, **_):
            calls.append(argv)
            if label == 'container-retirement-before':
                return 0, json.dumps({'Names': intent['name'], 'ID': value[0]['Id']}).encode(), b''
            if 'inspect' in argv:
                return 0, json.dumps(value).encode(), b''
            if 'stop' in argv:
                value[0]['State']['Running'] = False
            return 0, b'', b''
        result = m.retire(command, intent)
        self.assertTrue(result['remove_sent'])
        mutations = [argv for argv in calls if argv[1] in ('stop', 'rm')]
        self.assertTrue(all(argv[-1] == 'c' * 64 for argv in mutations))
        self.assertTrue(all(intent['name'] not in argv for argv in mutations))

    def test_replaced_container_after_stop_is_not_removed(self):
        intent, value = fixture()
        calls = []
        def command(argv, label, **_):
            calls.append(argv)
            if label == 'container-retirement-before':
                return 0, json.dumps({'Names': intent['name'], 'ID': value[0]['Id']}).encode(), b''
            if label == 'container-retirement-stopped':
                value[0]['Created'] = 'different'
            if 'inspect' in argv:
                return 0, json.dumps(value).encode(), b''
            return 0, b'', b''
        with self.assertRaises(ValueError):
            m.retire(command, intent)
        self.assertFalse(any(argv[1] == 'rm' for argv in calls))


if __name__ == '__main__':
    unittest.main()
