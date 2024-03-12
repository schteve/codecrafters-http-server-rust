use std::{collections::HashMap, fmt};

use nom::{
    self,
    branch::alt,
    bytes::complete::{tag, take_till, take_until1, take_while1},
    character::complete::{digit1, space1},
    combinator::{map, map_res, value},
    multi::many0,
    sequence::{pair, terminated, tuple},
    IResult,
};

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
    fn parser(input: &str) -> IResult<&str, Self> {
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
    fn parser(input: &str) -> IResult<&str, Self> {
        let (remain, (_, major, _, minor)) = tuple((
            tag("HTTP/"),
            map_res(digit1, |s: &str| s.parse::<u8>()),
            tag("."),
            map_res(digit1, |s: &str| s.parse::<u8>()),
        ))(input)?;

        Ok((remain, Self { major, minor }))
    }
}

impl Default for Version {
    fn default() -> Self {
        Self { major: 1, minor: 1 }
    }
}

impl std::fmt::Display for Version {
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
    fn parser(input: &str) -> IResult<&str, Self> {
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
                path,
                version,
            },
        ))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Request {
    pub req_line: RequestLine,
    pub headers: HashMap<String, String>,
}

impl Request {
    pub fn parser(input: &str) -> IResult<&str, Self> {
        let (remain, (req_line, headers)) = tuple((
            RequestLine::parser,
            many0(pair(
                terminated(take_while1(is_header_key), tag(": ")),
                terminated(take_until1("\r\n"), tag("\r\n")),
            )),
        ))(input)?;

        let headers_owned = headers
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v.to_owned()))
            .collect();

        Ok((
            remain,
            Self {
                req_line,
                headers: headers_owned,
            },
        ))
    }
}

fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\r' || c == '\n'
}

fn is_header_key(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '-'
}

#[derive(Debug, Default, Eq, PartialEq)]
pub enum Status {
    Ok,
    NotFound,
    #[default]
    Internal,
}

impl Status {
    pub fn code(&self) -> u32 {
        match self {
            Self::Ok => 200,
            Self::NotFound => 404,
            Self::Internal => 500,
        }
    }

    pub fn text(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::NotFound => "NOT FOUND",
            Self::Internal => "Internal Server Error",
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code(), self.text())
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct StatusLine {
    pub version: Version,
    pub status: Status,
}

impl std::fmt::Display for StatusLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}\r\n", self.version, self.status)
    }
}

#[derive(Default)]
pub struct Response {
    pub status_line: StatusLine,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl Response {
    pub fn new() -> Self {
        Self {
            status_line: StatusLine {
                version: Version { major: 1, minor: 1 },
                status: Status::NotFound,
            },
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn with_status(mut self, status: Status) -> Self {
        self.status_line.status = status;
        self
    }

    pub fn with_header<K: ToString, V: ToString>(mut self, k: K, v: V) -> Self {
        self.headers
            .insert(k.to_string().to_lowercase(), v.to_string().to_lowercase());
        self
    }

    pub fn with_body<S: ToString>(mut self, body: S) -> Self {
        let body = body.to_string();
        let body_len = body.len();
        self.body = Some(body);
        self.with_header("Content-Type", "text/plain")
            .with_header("Content-Length", body_len.to_string())
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status_line)?;

        // Sort so tests are easier to write
        let mut sorted_headers: Vec<_> = self.headers.iter().collect();
        sorted_headers.sort();
        for (k, v) in sorted_headers {
            write!(f, "{}: {}\r\n", k, v)?;
        }
        write!(f, "\r\n")?;

        if let Some(b) = self.body.as_ref() {
            write!(f, "{}", b)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parser() {
        let input = "HTTP/1.1";

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
        let input = "GET /index.html HTTP/1.1\r\n";

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
        let input = "\
            GET /index.html HTTP/1.1\r\n\
            Host: localhost:4221\r\n\
            User-Agent: curl/7.64.1\r\n\
        ";
        let (remain, req) = Request::parser(input).unwrap();
        println!("remain: {remain}");
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
    fn test_response_to_string() {
        let resp = Response::new().with_status(Status::Ok);
        assert_eq!(resp.to_string(), "HTTP/1.1 200 OK\r\n\r\n");

        let resp = Response::new().with_status(Status::Ok).with_body("abc");
        assert_eq!(
            resp.to_string(),
            "HTTP/1.1 200 OK\r\ncontent-length: 3\r\ncontent-type: text/plain\r\n\r\nabc"
        )
    }
}
