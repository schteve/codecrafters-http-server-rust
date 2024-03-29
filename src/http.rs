use std::{collections::HashMap, fmt, io, str};

use nom::{
    self,
    branch::alt,
    bytes::complete::{tag, take_till, take_until1, take_while1},
    character::complete::{digit1, space1},
    combinator::{map, map_res, opt, rest, value},
    multi::many0,
    sequence::{pair, preceded, terminated, tuple},
    IResult,
};

use crate::ser::Serialize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

impl Method {
    fn parser(input: &[u8]) -> IResult<&[u8], Self> {
        alt((
            value(Self::Get, tag("GET")),
            value(Self::Head, tag("HEAD")),
            value(Self::Post, tag("POST")),
            value(Self::Put, tag("PUT")),
            value(Self::Delete, tag("DELETE")),
            value(Self::Connect, tag("CONNECT")),
            value(Self::Options, tag("OPTIONS")),
            value(Self::Trace, tag("TRACE")),
            value(Self::Patch, tag("PATCH")),
        ))(input)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

impl Version {
    fn parser(input: &[u8]) -> IResult<&[u8], Self> {
        let (remain, (_, major, _, minor)) = tuple((
            tag("HTTP/"),
            map_res(digit1, |s: &[u8]| str::from_utf8(s).unwrap().parse::<u8>()),
            tag("."),
            map_res(digit1, |s: &[u8]| str::from_utf8(s).unwrap().parse::<u8>()),
        ))(input)?;

        Ok((remain, Self { major, minor }))
    }
}

impl Default for Version {
    fn default() -> Self {
        Self { major: 1, minor: 1 }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP/{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RequestLine {
    pub method: Method,
    pub path: String,
    pub version: Version,
}

impl RequestLine {
    fn parser(input: &[u8]) -> IResult<&[u8], Self> {
        let (remain, (method, _, path, _, version, _)) = tuple((
            Method::parser,
            space1,
            map(take_till(is_whitespace), ToOwned::to_owned),
            space1,
            Version::parser,
            tag("\r\n"),
        ))(input)?;

        Ok((
            remain,
            Self {
                method,
                path: String::from_utf8(path).unwrap(),
                version,
            },
        ))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Request {
    pub req_line: RequestLine,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl Request {
    pub fn parser(input: &[u8]) -> IResult<&[u8], Self> {
        let (remain, (req_line, headers, body)) = tuple((
            RequestLine::parser,
            many0(pair(
                terminated(take_while1(is_header_key), tag(": ")),
                terminated(take_until1("\r\n"), tag("\r\n")),
            )),
            opt(preceded(tag("\r\n"), rest)),
        ))(input)?;

        let headers_owned = headers
            .into_iter()
            .map(|(k, v)| {
                (
                    str::from_utf8(k).unwrap().to_lowercase(),
                    str::from_utf8(v).unwrap().to_owned(),
                )
            })
            .collect();

        Ok((
            remain,
            Self {
                req_line,
                headers: headers_owned,
                body: body.map(|b| b.to_vec()),
            },
        ))
    }

    pub fn get_content_length(&self) -> Option<usize> {
        self.headers
            .get("content-length")
            .and_then(|s| s.parse().ok())
    }
}

fn is_whitespace(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\r' || c == b'\n'
}

fn is_header_key(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'-'
}

#[derive(Debug, Default, Eq, PartialEq)]
pub enum Status {
    Ok,
    Created,
    BadRequest,
    NotFound,
    #[default]
    Internal,
}

impl Status {
    pub fn code(&self) -> u32 {
        match self {
            Self::Ok => 200,
            Self::Created => 201,
            Self::BadRequest => 400,
            Self::NotFound => 404,
            Self::Internal => 500,
        }
    }

    pub fn text(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Created => "Created",
            Self::BadRequest => "Bad Request",
            Self::NotFound => "NOT FOUND",
            Self::Internal => "Internal Server Error",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code(), self.text())
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct StatusLine {
    pub version: Version,
    pub status: Status,
}

impl fmt::Display for StatusLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}\r\n", self.version, self.status)
    }
}

#[derive(Default)]
pub struct Response {
    pub status_line: StatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl Response {
    pub fn new(status: Status) -> Self {
        Self {
            status_line: StatusLine {
                version: Version { major: 1, minor: 1 },
                status,
            },
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn with_header<K: ToString, V: ToString>(mut self, k: K, v: V) -> Self {
        self.headers
            .insert(k.to_string().to_lowercase(), v.to_string().to_lowercase());
        self
    }

    pub fn with_body<S: ToString>(mut self, body: &[u8], content_type: S) -> Self {
        let body_len = body.len();
        self.body = Some(body.to_owned());
        self.with_header("Content-Type", content_type.to_string())
            .with_header("Content-Length", body_len.to_string())
    }
}

impl Serialize for Response {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        write!(writer, "{}", self.status_line)?;

        // Sort so tests are easier to write
        let mut sorted_headers: Vec<_> = self.headers.iter().collect();
        sorted_headers.sort();
        for (k, v) in sorted_headers {
            write!(writer, "{}: {}\r\n", k, v)?;
        }
        write!(writer, "\r\n")?;

        if let Some(b) = self.body.as_ref() {
            writer.write_all(b)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parser() {
        let input = b"HTTP/1.1";

        let (remain, ver) = Version::parser(input).unwrap();
        assert!(remain.is_empty());
        assert_eq!(ver, Version { major: 1, minor: 1 });
    }

    #[test]
    fn test_version_to_string() {
        let ver = Version { major: 1, minor: 1 };
        assert_eq!(ver.to_string(), "HTTP/1.1");
    }

    #[test]
    fn test_request_line_parser() {
        let input = b"GET /index.html HTTP/1.1\r\n";

        let (remain, req_line) = RequestLine::parser(input).unwrap();
        assert!(remain.is_empty());
        assert_eq!(
            req_line,
            RequestLine {
                method: Method::Get,
                path: String::from("/index.html"),
                version: Version { major: 1, minor: 1 }
            }
        );
    }

    #[test]
    fn test_request_parser() {
        let input = b"\
            GET /index.html HTTP/1.1\r\n\
            Host: localhost:4221\r\n\
            User-Agent: curl/7.64.1\r\n\
        ";
        let (remain, req) = Request::parser(input).unwrap();
        assert!(remain.is_empty());
        assert_eq!(
            req,
            Request {
                req_line: RequestLine {
                    method: Method::Get,
                    path: String::from("/index.html"),
                    version: Version { major: 1, minor: 1 },
                },
                headers: [
                    (String::from("host"), String::from("localhost:4221")),
                    (String::from("user-agent"), String::from("curl/7.64.1")),
                ]
                .into_iter()
                .collect(),
                body: None,
            }
        );
    }

    #[test]
    fn test_status_line_to_string() {
        let status_line = StatusLine {
            version: Version { major: 1, minor: 1 },
            status: Status::Ok,
        };
        assert_eq!(status_line.to_string(), "HTTP/1.1 200 OK\r\n");

        let status_line = StatusLine::default();
        assert_eq!(
            status_line.to_string(),
            "HTTP/1.1 500 Internal Server Error\r\n"
        );
    }

    #[test]
    fn test_response_to_bytes() {
        let resp = Response::new(Status::Ok);
        assert_eq!(resp.to_bytes(), b"HTTP/1.1 200 OK\r\n\r\n");

        let resp = Response::new(Status::Ok).with_body(b"abc", "text/plain");
        assert_eq!(
            resp.to_bytes(),
            b"HTTP/1.1 200 OK\r\ncontent-length: 3\r\ncontent-type: text/plain\r\n\r\nabc"
        )
    }
}
