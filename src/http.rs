use nom::{
    self,
    branch::alt,
    bytes::complete::{tag, take_till},
    character::complete::{digit1, space1},
    combinator::{map, map_res, recognize, value},
    sequence::tuple,
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

    pub fn serialize(&self) -> String {
        format!("HTTP/{}.{}", self.major, self.minor)
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
    pub host: String,
    pub user_agent: String,
}

impl Request {
    pub fn parser(input: &str) -> IResult<&str, Self> {
        let (remain, (req_line, _, host, _, _, user_agent, _)) = tuple((
            RequestLine::parser,
            tag("Host: "),
            map(recognize(take_till(is_whitespace)), ToOwned::to_owned),
            tag("\r\n"),
            tag("User-Agent: "),
            map(recognize(take_till(is_whitespace)), ToOwned::to_owned),
            tag("\r\n"),
        ))(input)?;

        Ok((
            remain,
            Self {
                req_line,
                host,
                user_agent,
            },
        ))
    }
}

fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\r' || c == '\n'
}

#[derive(Debug, Eq, PartialEq)]
pub enum Status {
    Ok,
    NotFound,
}

impl Status {
    pub fn code(&self) -> u32 {
        match self {
            Self::Ok => 200,
            Self::NotFound => 404,
        }
    }

    pub fn text(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::NotFound => "NOT FOUND",
        }
    }

    pub fn serialize(&self) -> String {
        format!("{} {}", self.code(), self.text())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct StatusLine {
    pub version: Version,
    pub status: Status,
}

impl StatusLine {
    pub fn serialize(&self) -> String {
        format!(
            "{} {}\r\n",
            self.version.serialize(),
            self.status.serialize()
        )
    }
}

pub struct Response {
    pub status_line: StatusLine,
}

impl Response {
    pub fn with_status(status: Status) -> Self {
        Self {
            status_line: StatusLine {
                version: Version { major: 1, minor: 1 },
                status,
            },
        }
    }
    pub fn serialize(&self) -> String {
        format!("{}\r\n", self.status_line.serialize())
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
    fn test_version_serialize() {
        let ver = Version { major: 1, minor: 1 };
        assert_eq!(ver.serialize(), "HTTP/1.1");
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
        assert!(remain.is_empty());
        assert_eq!(
            req,
            Request {
                req_line: RequestLine {
                    method: Method::Get,
                    path: String::from("/index.html"),
                    version: Version { major: 1, minor: 1 },
                },
                host: String::from("localhost:4221"),
                user_agent: String::from("curl/7.64.1"),
            }
        );
    }

    #[test]
    fn test_status_line_serialize() {
        let status_line = StatusLine {
            version: Version { major: 1, minor: 1 },
            status: Status::Ok,
        };
        assert_eq!(status_line.serialize(), "HTTP/1.1 200 OK\r\n");
    }

    #[test]
    fn test_response_serialize() {
        let resp = Response {
            status_line: StatusLine {
                version: Version { major: 1, minor: 1 },
                status: Status::Ok,
            },
        };

        assert_eq!(resp.serialize(), "HTTP/1.1 200 OK\r\n\r\n");
    }
}
