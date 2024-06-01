#![cfg_attr(not(test), no_std)]
use nom::{
    bytes::complete::{tag, take, take_until},
    character::complete::{not_line_ending, u16, u8},
    combinator::map_parser,
    error::ParseError,
    sequence::{pair, preceded, tuple},
    IResult, Parser,
};

pub fn parse_event<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], ((u16, u8, u8), &'a [u8]), E>
where
    E: ParseError<&'a [u8]>,
{
    tuple((
        preceded(
            pair(
                take_until("DTSTART;VALUE=DATE:"),
                tag("DTSTART;VALUE=DATE:"),
            ),
            parse_date,
        ),
        preceded(
            pair(take_until("SUMMARY:"), tag("SUMMARY:")),
            not_line_ending,
        ),
    ))
    .parse(input)
}

pub fn parse_date<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], (u16, u8, u8), E>
where
    E: ParseError<&'a [u8]>,
{
    tuple((parse_dec4, parse_dec2, parse_dec2)).parse(input)
}

pub fn parse_dec4<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], u16, E>
where
    E: ParseError<&'a [u8]>,
{
    map_parser(take(4usize), u16)(input)
}

pub fn parse_dec2<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], u8, E>
where
    E: ParseError<&'a [u8]>,
{
    map_parser(take(2usize), u8)(input)
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
    fn parse_event_test() {
        let dates: Vec<_> = iterator(
            RESPONSE_BYTES.as_bytes(),
            crate::parse_event::<nom::error::Error<&'_ [u8]>>,
        )
        .collect();

        assert!(!dates.is_empty());
        assert_eq!(
            dates[0],
            ((2023, 08, 18), "Black Bin Collection".as_bytes())
        );
        assert_eq!(
            dates[1],
            ((2023, 08, 25), "Green Bin Collection".as_bytes())
        );
    }

    #[test]
    fn parse_date_test() {
        assert_eq!(
            crate::parse_date::<nom::error::Error<&'_ [u8]>>("2023081855some_more_text".as_bytes())
                .unwrap(),
            ("55some_more_text".as_bytes(), (2023, 08, 18))
        );
    }

    #[test]
    fn parse_dec4() {
        assert_eq!(
            crate::parse_dec4::<nom::error::Error<&'_ [u8]>>("202324".as_bytes()).unwrap(),
            ("24".as_bytes(), 2023)
        );
    }
}
