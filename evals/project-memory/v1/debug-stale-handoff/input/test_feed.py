import unittest

from feed import next_page


class FeedTests(unittest.TestCase):
    def test_first_page(self):
        self.assertEqual(next_page([4, 1, 3, 2]), [1, 2])

    def test_following_page_uses_last_seen_cursor(self):
        first = next_page([1, 2, 3, 4])
        self.assertEqual(next_page([1, 2, 3, 4], after_id=first[-1]), [3, 4])

    def test_cursor_zero_is_not_the_first_page(self):
        self.assertEqual(next_page([0, 1, 2], after_id=0), [1, 2])

    def test_after_final_item_is_empty(self):
        self.assertEqual(next_page([1, 2], after_id=2), [])


if __name__ == "__main__":
    unittest.main()
