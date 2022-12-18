class ParseError(Exception):
    pass


class peakable:
    def __init__(self, iterable):
        self._it = iter(iterable)
        self._cache = None

    def __iter__(self):
        return self

    def peek(self):
        if not self._cache:
            self._cache = next(self._it)
        return self._cache

    def __next__(self):
        if self._cache:
            c = self._cache
            self._cache = None
            return c
        return next(self._it)


def parse(input):
    input = peakable(input)

    (input, _) = parse_line(tag("BEGIN:VCALENDAR"))(input)
    (input, _) = header_lines(input)
    (input, events) = many_lines(map(event_lines, tuples_to_dict))(input)
    (input, _) = parse_line(tag("END:VCALENDAR"))(input)
    return (input, events)


def parse_line(tag):
    def line_inner(input):
        (buf, val) = tag(input.peek())
        if len(buf) > 0:
            raise ParseError("incomplete line parse, '{buf}' remaining")
        next(input)
        return (input, val)
    return line_inner


def event_lines(input):
    (input, _) = parse_line(tag("BEGIN:VEVENT"))(input)
    (input, items) = many_lines(alt_lines([
        parse_line(value(None, separated_pair(tag("UID"), tag(
            ":"), take_until_oel_discard))),
        parse_line(value(None, separated_pair(tag("DTSTAMP"), tag(
            ":"), take_until_oel_discard))),
        parse_line(separated_pair(value("START", tag("DTSTART;VALUE=DATE")),
                                  tag(":"), map(take_until_oel, to_date_tuple))),
        parse_line(separated_pair(tag("SUMMARY"), tag(
            ":"), map(take_until_oel, lambda s: s.split(" ")[0].upper()))),
    ]))(input)
    (input, _) = parse_line(tag("END:VEVENT"))(input)
    return (input, items)


def tuples_to_dict(input):
    dict = {}
    for (k, v) in input:
        dict[k] = v
    return dict


def to_date_tuple(input):
    return (
        int(input[0:4]),
        int(input[4:6]),
        int(input[6:8])
    )


def header_lines(input):
    return many_lines(alt_lines([
        parse_line(separated_pair(tag("PRODID"), tag(
            ":"), take_until_oel_discard)),
        parse_line(separated_pair(tag("VERSION"), tag(
            ":"), take_until_oel_discard)),
        parse_line(pair(tag("X-"), take_until_oel_discard))
    ]))(input)


def many_lines(c):
    def many_lines_inner(input):
        results = []

        while True:
            try:
                (input, r) = c(input)
                if r is not None:
                    results.append(r)
            except ParseError:
                return (input, results)

    return many_lines_inner


def alt_lines(combs):
    def alt_lines_inner(input):
        for c in combs:
            try:
                return c(input)
            except ParseError:
                pass
        raise ParseError("No parsers matched alt")

    return alt_lines_inner


def take_until_oel(input):
    return ("", input)


def take_until_oel_discard(_):
    return ("", None)


def pair(first, second):
    def pair_inner(input):
        (input, f) = first(input)
        (input, s) = second(input)
        return (input, (f, s))
    return pair_inner


def separated_pair(first, sep, second):
    def separated_pair_inner(input):
        (input, f) = first(input)
        (input, _) = sep(input)
        (input, s) = second(input)
        return (input, (f, s))
    return separated_pair_inner


def terminated(first, second):
    def terminated_inner(input):
        (input, f) = first(input)
        (input, __) = second(input)
        return (input, f)
    return terminated_inner


def tag(value):
    def tag_inner(input):
        if input.startswith(value):
            return (input[len(value):], value)
        else:
            raise ParseError(f"tag not matched: {value}")
    return tag_inner


def map(tag, f):
    def map_inner(input):
        (input, val) = tag(input)
        return (input, f(val))
    return map_inner


def value(value, tag):
    def map_inner(input):
        (input, _) = tag(input)
        return (input, value)
    return map_inner
