import unittest
from parcel.pricing import quote
from parcel.receipt import receipt


class ParcelTests(unittest.TestCase):
    def test_existing_quotes(self):
        self.assertEqual(quote(1000, "local"), 200)
        self.assertEqual(quote(2000, "national"), 1000)

    def test_invalid_input(self):
        for value in [0, -1, True, 1.5]:
            with self.assertRaises(ValueError):
                quote(value, "local")
        with self.assertRaises(ValueError):
            quote(1000, "moon")

    def test_receipt(self):
        self.assertEqual(receipt(1000, "local"),
                         {"weight_grams": 1000, "zone": "local", "price_cents": 200})
