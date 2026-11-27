import unittest

from formats import parse_format


class FormatTests(unittest.TestCase):
    def test_letter_case(self):
        self.assertEqual(parse_format("JSON"), "json")
        self.assertEqual(parse_format("cSv"), "csv")

    def test_surrounding_spaces(self):
        self.assertEqual(parse_format(" JSON "), "json")

    def test_unsupported_format(self):
        with self.assertRaises(ValueError):
            parse_format("xml")


if __name__ == "__main__":
    unittest.main()
