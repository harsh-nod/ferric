"""Regression checks for the data-only four-forward report analyzer."""
import copy
import unittest

from analyze import validate_trajectory


class TrajectoryTests(unittest.TestCase):
    def setUp(self):
        self.inputs = [9112, 67, 25, 576]
        outputs = [67, 25, 576, 2701]
        self.events = [dict(position=i, generation=i + 1, input_token=token,
                            output_token=outputs[i], status='completed')
                       for i, token in enumerate(self.inputs)]

    def test_fixed_seed_and_actual_own_output_history(self):
        validate_trajectory(self.events, self.inputs)

    def test_wrong_seed(self):
        self.events[0]['input_token'] = 220
        with self.assertRaises(ValueError):
            validate_trajectory(self.events, self.inputs)

    def test_empty_or_short_history(self):
        for length in range(4):
            with self.subTest(length=length), self.assertRaises(ValueError):
                validate_trajectory(self.events[:length], self.inputs[:length])

    def test_wrong_position_or_generation(self):
        for field in ('position', 'generation'):
            events = copy.deepcopy(self.events)
            events[2][field] += 1
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_trajectory(events, self.inputs)

    def test_broken_recurrence(self):
        self.events[1]['input_token'] = 68
        with self.assertRaises(ValueError):
            validate_trajectory(self.events, self.inputs)

    def test_failed_event(self):
        self.events[3]['status'] = 'failed'
        with self.assertRaises(ValueError):
            validate_trajectory(self.events, self.inputs)

    def test_captured_input_join(self):
        self.inputs[3] += 1
        with self.assertRaises(ValueError):
            validate_trajectory(self.events, self.inputs)

    def test_noninteger_and_out_of_vocabulary_output(self):
        for value in (True, 1.0, -1, 151936):
            events = copy.deepcopy(self.events)
            events[3]['output_token'] = value
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_trajectory(events, self.inputs)


if __name__ == '__main__':
    unittest.main(verbosity=2)
