#![cfg_attr(not(test), no_std)]

use nom::{
    bytes::{complete::tag, complete::take_until},
    character::complete::u16,
    character::complete::u8,
    error::ParseError,
    sequence::{preceded, terminated, tuple},
    IResult, Parser,
};

type DateTimeTuple = (u16, u8, u8, u8, u8, u8);

pub fn parse<'a, E>(input: &'a [u8]) -> IResult<&'a [u8], DateTimeTuple, E>
where
    E: ParseError<&'a [u8]>,
{
    //2023-08-11T14:08:18.945857+00:00

    let datetime = tuple((
        terminated(u16, tag("-")), // year
        terminated(u8, tag("-")),  // month
        terminated(u8, tag("T")),  // day
        terminated(u8, tag(":")),  // hour
        terminated(u8, tag(":")),  // minute
        terminated(u8, tag(".")),  // second
    ));

    preceded(
        take_until("utc_datetime: "),
        preceded(tag("utc_datetime: "), datetime),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {

    const RESPONSE_BYTES: &str = "HTTP/1.1 200 OK\n
access-control-allow-credentials: true\n
access-control-allow-origin: *\n
access-control-expose-headers:\n
cache-control: max-age=0, private, must-revalidate\n
content-length: 356\n
content-type: text/plain; charset=utf-8\n
cross-origin-window-policy: deny\n
date: Fri, 11 Aug 2023 14:08:18 GMT\n
server: Fly/49bc237b (2023-08-04)\n
x-content-type-options: nosniff\n
x-download-options: noopen\n
x-frame-options: SAMEORIGIN\n
x-permitted-cross-domain-policies: none\n
x-ratelimit-limit: 1800\n
x-ratelimit-remaining: 1798\n
x-ratelimit-reset: 1691766000\n
x-request-from: 82.13.76.38\n
x-request-id: F3pZZKvTMTyimC17wPxh\n
x-request-regions: a/lhr;s/cdg\n
x-response-origin: 3e93778a-1f50-8c78-0d58-9f54f5991491\n
x-runtime: 285us\n
x-xss-protection: 1; mode=block\n
via: 1.1 fly.io\n
fly-request-id: 01H7JETDZX5GAXS62EQGJW553J-lhr\n
\n
abbreviation: BST\n
client_ip: 82.13.76.38\n
datetime: 2023-08-11T15:08:18.945857+01:00\n
day_of_week: 5\n
day_of_year: 223\n
dst: true\n
dst_from: 2023-03-26T01:00:00+00:00\n
dst_offset: 3600\n
dst_until: 2023-10-29T01:00:00+00:00\n
raw_offset: 0\n
timezone: Europe/London\n
unixtime: 1691762898\n
utc_datetime: 2023-08-11T14:08:18.945857+00:00\n
utc_offset: +01:00\n
week_number: 32";

    #[test]
    fn test() {
        match crate::parse::<()>(RESPONSE_BYTES.as_bytes()) {
            Ok((_, r)) => assert!(r == (2023, 8, 11, 14, 8, 18)),
            Err(_) => assert!(false, "failed to parse"),
        }
    }
}
