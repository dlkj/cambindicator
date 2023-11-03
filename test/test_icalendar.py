import unittest
from icalendar import parse


class TestIcalendar(unittest.TestCase):
    def test_read(self):
        (input, events) = parse(test_data.splitlines())

        self.assertEqual(len(events), 16)

        self.assertRaises(StopIteration, input.__next__)


test_data = """BEGIN:VCALENDAR
PRODID:-//192.124.249.105//Waste Calendar Generator//
VERSION:2.0
X-WR-CALNAME:Bins Schedule
X-WR-CALDESC:Bins Schedule
X-WR-TIMEZONE:Europe/London
BEGIN:VEVENT
UID:615ed164-6cf0-4848-a301-e0191c463f18@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221111
SUMMARY:Black Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:bd1671dc-550b-4428-996b-8bb50cbfa135@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221118
SUMMARY:Green Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:8ecbd6fa-0367-44bf-be42-9e9d0788c83e@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221118
SUMMARY:Blue Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:d7038517-7760-4942-be30-8121a9343a40@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221125
SUMMARY:Black Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:e23952bd-6ad7-40fc-a231-0c1ad5f3229b@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221202
SUMMARY:Green Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:33b6dcc8-7bc1-427d-b388-517d4d52f4a9@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221202
SUMMARY:Blue Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:30867dc2-3160-4e04-ad79-a666d70f2bb8@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221209
SUMMARY:Black Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:4cb635a1-4902-4049-9dfb-41b53acfa03c@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221216
SUMMARY:Green Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:19f9d824-884a-4447-b67c-12335a1d60cc@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221216
SUMMARY:Blue Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:294ba28f-f77e-4590-b043-8a13ba4b7fc6@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20221223
SUMMARY:Black Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:a654b6a3-1d49-4968-99ba-b7dc45d1d544@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20230103
SUMMARY:Blue Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:534bdf8e-0bc8-477d-acae-ce7a8a44b248@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20230109
SUMMARY:Black Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:d9b63934-c1ae-438e-9d40-e1608234ea73@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20230114
SUMMARY:Green Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:f4159ef0-3b54-421c-9c2a-c5f159a01b0d@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20230114
SUMMARY:Blue Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:37996fd0-f0b4-4170-9291-1182f962477d@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20230120
SUMMARY:Black Bin Collection
END:VEVENT
BEGIN:VEVENT
UID:e754a01f-704a-4abd-952b-acb8a7f1ee4a@192.124.249.105
DTSTAMP:20221111T192249Z
DTSTART;VALUE=DATE:20230127
SUMMARY:Blue Bin Collection
END:VEVENT
END:VCALENDAR
"""