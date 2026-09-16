import unittest
from parcelnote import label


class LabelTests(unittest.TestCase):
    def test_recipient(self):
        self.assertEqual(label(" Ada "), "Recipient: Ada")

    def test_empty_recipient(self):
        with self.assertRaises(ValueError):
            label("  ")
