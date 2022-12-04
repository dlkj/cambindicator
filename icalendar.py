class ParseError(Exception):
    pass


def parse(input):
    (input, _) = terminated(tag("BEGIN:VCALENDAR"), newline)(input)
    (input, _) = header(input)
    (input, events) = many(event)(input)
    (input, _) = tag("END:VCALENDAR")(input)
    print(input.strip())
    for e in events:
        print(e)
    return True


def event(input):
    (input, _) = terminated(tag("BEGIN:VEVENT"), newline)(input)
    (input, items) = many(alt([
        terminated(separated_pair(tag("UID"), tag(
            ":"), take_until(newline)), newline),
        terminated(separated_pair(tag("DTSTAMP"), tag(
            ":"), take_until(newline)), newline),
        terminated(separated_pair(tag("DTSTART;VALUE=DATE"),
                   tag(":"), take_until(newline)), newline),
        terminated(separated_pair(tag("SUMMARY"), tag(
            ":"), take_until(newline)), newline),
    ]))(input)
    (input, _) = terminated(tag("END:VEVENT"), newline)(input)
    return (input, items)


def header(input):
    (input, _) = many(alt([
        terminated(separated_pair(tag("PRODID"), tag(
            ":"), take_until(newline)), newline),
        terminated(separated_pair(tag("VERSION"), tag(
            ":"), take_until(newline)), newline),
        terminated(pair(tag("X-"), take_until(newline)), newline)
    ]))(input)
    return (input, ())


def many(c):
    def many_inner(input):
        results = []

        while True:
            try:
                (input, r) = c(input)
                results.append(r)
            except ParseError:
                return (input, results)

    return many_inner


def alt(combs):
    def alt_inner(input):
        for c in combs:
            try:
                return c(input)
            except ParseError:
                pass
        raise ParseError("No parsers matched alt")

    return alt_inner


def take_until(tag):
    def take_until_inner(input):

        for i in range(len(input)):
            try:
                tag(input[i:])
                return (input[i:], input[:i])
            except ParseError:
                pass
        return ("", input)
    return take_until_inner


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


def newline(input):
    try:
        return tag("\n")(input)
    except ParseError:
        try:
            return tag("\n\r")(input)
        except ParseError:
            raise ParseError("new line not matched")
