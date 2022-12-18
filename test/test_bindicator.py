import unittest
from bindicator import get_bins_for_date


class TestIcalendar(unittest.TestCase):
    def test_get_bins_for_date(self):

        events = [{'START': (2022, 11, 11), 'SUMMARY': 'BLACK'},
                  {'START': (2022, 11, 18), 'SUMMARY': 'GREEN'},
                  {'START': (2022, 11, 18), 'SUMMARY': 'BLUE'},
                  {'START': (2022, 11, 25), 'SUMMARY': 'BLACK'},
                  {'START': (2022, 12, 2), 'SUMMARY': 'GREEN'},
                  {'START': (2022, 12, 2), 'SUMMARY': 'BLUE'},
                  {'START': (2022, 12, 9), 'SUMMARY': 'BLACK'},
                  {'START': (2022, 12, 16), 'SUMMARY': 'GREEN'},
                  {'START': (2022, 12, 16), 'SUMMARY': 'BLUE'},
                  {'START': (2022, 12, 23), 'SUMMARY': 'BLACK'},
                  {'START': (2023, 1, 3), 'SUMMARY': 'BLUE'},
                  {'START': (2023, 1, 9), 'SUMMARY': 'BLACK'},
                  {'START': (2023, 1, 14), 'SUMMARY': 'GREEN'},
                  {'START': (2023, 1, 14), 'SUMMARY': 'BLUE'},
                  {'START': (2023, 1, 20), 'SUMMARY': 'BLACK'},
                  {'START': (2023, 1, 27), 'SUMMARY': 'BLUE'}]

        self.assertSetEqual(
            get_bins_for_date(events, (2022, 12, 10, 5, 17, 7, 7, 0)), set())
        self.assertSetEqual(
            get_bins_for_date(events, (2022, 12, 16, 5, 17, 7, 7, 0)), {"GREEN", "BLUE"})
        self.assertSetEqual(
            get_bins_for_date(events, (2023, 1, 20, 5, 17, 7, 7, 0)), {"BLACK"})
        self.assertSetEqual(
            get_bins_for_date(events, (2023, 1, 3, 5, 17, 7, 7, 0)), {"BLUE"})
