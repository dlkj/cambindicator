#![cfg_attr(not(test), no_std)]
use nom::{
    bytes::complete::tag,
    bytes::complete::take_until,
    character::complete::not_line_ending,
    character::complete::u32,
    error::ParseError,
    sequence::{pair, preceded, tuple},
    IResult, Parser,
};

pub fn parse_event<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], (u32, &'a [u8]), E>
where
    E: ParseError<&'a [u8]>,
{
    tuple((
        preceded(
            pair(
                take_until("DTSTART;VALUE=DATE:"),
                tag("DTSTART;VALUE=DATE:"),
            ),
            u32,
        ),
        preceded(
            pair(take_until("SUMMARY:"), tag("SUMMARY:")),
            not_line_ending,
        ),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use nom::combinator::iterator;

    const RESPONSE_BYTES: &str = "HTTP/1.1 200 OK\n
content-length: 356\n
content-type: text/plain; charset=utf-8\n
date: Fri, 11 Aug 2023 14:08:18 GMT\n
\n
BEGIN:VCALENDAR\n
PRODID:-//192.124.249.105//Waste Calendar Generator//\n
VERSION:2.0\n
X-WR-CALNAME:Bins Schedule\n
X-WR-CALDESC:Bins Schedule\n
X-WR-TIMEZONE:Europe/London\n
BEGIN:VEVENT\n
UID:3b6c61f6-b227-455c-9256-0c3ba5297c65@192.124.249.105\n
DTSTAMP:20230812T153351Z\n
DTSTART;VALUE=DATE:20230818\n
SUMMARY:Black Bin Collection\n
END:VEVENT\n
BEGIN:VEVENT\n
UID:510b6b60-91bb-4903-8077-aca56aeeea6e@192.124.249.105\n
DTSTAMP:20230812T153351Z\n
DTSTART;VALUE=DATE:20230825\n
SUMMARY:Green Bin Collection\n
END:VEVENT\n
END:VCALENDAR";

    #[test]
    fn test() {
        for (a, b) in &mut iterator(RESPONSE_BYTES.as_bytes(), crate::parse_event::<()>) {
            println!("{} {}", a, core::str::from_utf8(b).unwrap());
        }
    }
}
