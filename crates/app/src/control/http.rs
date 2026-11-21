//! The control API's HTTP/1.1, as little of it as its clients need: one
//! request per connection (`Connection: close`), a `Content-Length` body
//! (no chunked bodies), and every part limited ([`super::LIMITS`]). A
//! request names its `Host` once, as HTTP/1.1 asks, and its
//! `Content-Length` at most once: two of either are malformed, so no
//! reader of the request can take another one than RVP did.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Instant;

use super::Limits;

/// One request, its header names in lower case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Request {
    pub method: String,
    /// The request line's target as sent: the path and its query, not
    /// decoded (signatures cover it).
    pub target: String,
    /// The path without its query, percent-decoded.
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// The first header of this name (in lower case).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn query(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Why a request was not read; each has its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReadError {
    /// The client closed the connection without a request: no answer.
    Closed,
    /// 400.
    Malformed,
    /// 408: the request took longer than [`Limits::request`].
    Timeout,
    /// 413.
    BodyTooLarge,
    /// 431.
    HeadersTooLarge,
    /// 501: a chunked body.
    Unsupported,
}

impl ReadError {
    pub fn status(self) -> Option<u16> {
        match self {
            ReadError::Closed => None,
            ReadError::Malformed => Some(400),
            ReadError::Timeout => Some(408),
            ReadError::BodyTooLarge => Some(413),
            ReadError::HeadersTooLarge => Some(431),
            ReadError::Unsupported => Some(501),
        }
    }
}

/// Reads one request within the limits: the whole request, headers and
/// body, must arrive within [`Limits::request`] (a slow client cannot
/// hold a connection longer).
pub(super) fn read_request(stream: &mut TcpStream, limits: &Limits) -> Result<Request, ReadError> {
    let deadline = Instant::now() + limits.request;
    let mut buffer = Vec::with_capacity(1024);
    let mut chunk = [0u8; 2048];
    let head_end = loop {
        if let Some(end) = find(&buffer, b"\r\n\r\n") {
            break end;
        }
        if buffer.len() > limits.header_bytes {
            return Err(ReadError::HeadersTooLarge);
        }
        let read = read_some(stream, &mut chunk, deadline)?;
        if read == 0 {
            return Err(if buffer.is_empty() {
                ReadError::Closed
            } else {
                ReadError::Malformed
            });
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    if head_end > limits.header_bytes {
        return Err(ReadError::HeadersTooLarge);
    }
    let head = std::str::from_utf8(&buffer[..head_end]).map_err(|_| ReadError::Malformed)?;
    let Head {
        method,
        target,
        headers,
        length,
    } = parse_head(head)?;
    if length > limits.body_bytes {
        return Err(ReadError::BodyTooLarge);
    }
    let mut body = buffer[head_end + 4..].to_vec();
    if body.len() > length {
        // Bytes past the body: a second request on the same connection,
        // which this server does not take.
        return Err(ReadError::Malformed);
    }
    while body.len() < length {
        let read = read_some(stream, &mut chunk, deadline)?;
        if read == 0 {
            return Err(ReadError::Malformed);
        }
        body.extend_from_slice(&chunk[..read]);
        if body.len() > length {
            return Err(ReadError::Malformed);
        }
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    Ok(Request {
        method: method.to_owned(),
        target: target.to_owned(),
        path: decode(path).ok_or(ReadError::Malformed)?,
        query: query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                Some((decode(key)?, decode(&value.replace('+', " "))?))
            })
            .collect::<Option<_>>()
            .ok_or(ReadError::Malformed)?,
        headers,
        body,
    })
}

/// A request's line and headers.
#[derive(Debug, PartialEq, Eq)]
struct Head<'a> {
    method: &'a str,
    target: &'a str,
    headers: Vec<(String, String)>,
    /// The body's `Content-Length` (0 without one).
    length: usize,
}

/// The request line and the headers (`head` without its final blank
/// line).
fn parse_head(head: &str) -> Result<Head<'_>, ReadError> {
    let mut lines = head.split("\r\n");
    let mut parts = lines.next().unwrap_or_default().split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(ReadError::Malformed);
    };
    if !version.starts_with("HTTP/1.") || method.is_empty() || !target.starts_with('/') {
        return Err(ReadError::Malformed);
    }
    let mut headers = Vec::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(ReadError::Malformed)?;
        if name.is_empty() || name.contains(char::is_whitespace) {
            return Err(ReadError::Malformed);
        }
        headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
    }
    let count = |name: &str| headers.iter().filter(|(key, _)| key == name).count();
    if count("transfer-encoding") > 0 {
        return Err(ReadError::Unsupported);
    }
    if count("host") != 1 || count("content-length") > 1 {
        return Err(ReadError::Malformed);
    }
    let length = match headers.iter().find(|(key, _)| key == "content-length") {
        Some((_, value)) => value.parse::<usize>().map_err(|_| ReadError::Malformed)?,
        None => 0,
    };
    Ok(Head {
        method,
        target,
        headers,
        length,
    })
}

/// Reads what arrives before the deadline.
fn read_some(
    stream: &mut TcpStream,
    chunk: &mut [u8],
    deadline: Instant,
) -> Result<usize, ReadError> {
    loop {
        let left = deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or(ReadError::Timeout)?;
        stream
            .set_read_timeout(Some(left))
            .map_err(|_| ReadError::Closed)?;
        match stream.read(chunk) {
            Ok(read) => return Ok(read),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(ReadError::Timeout);
            }
            Err(_) => return Err(ReadError::Closed),
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Percent-decoding into UTF-8 text; `None` for a broken escape or text.
pub(super) fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Writes an answer with a JSON body and closes the exchange.
pub(super) fn respond(stream: &mut TcpStream, status: u16, body: &str) -> io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        reason(status),
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_escapes_decode_to_text() {
        assert_eq!(
            decode("/api/action/play%2Dpause").as_deref(),
            Some("/api/action/play-pause")
        );
        assert_eq!(decode("%D0%AF").as_deref(), Some("Я"));
        assert_eq!(decode("%zz"), None);
        assert_eq!(decode("%4"), None);
        assert_eq!(decode("%FF"), None);
    }

    #[test]
    fn a_request_names_its_host_once_and_its_length_at_most_once() {
        let head =
            parse_head("POST /api/seek?x=1 HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 9")
                .expect("a request");
        assert_eq!(head.method, "POST");
        assert_eq!(head.target, "/api/seek?x=1");
        assert_eq!(head.length, 9);
        assert!(parse_head("GET / HTTP/1.1\r\nHost: localhost").is_ok());
        for broken in [
            "GET / HTTP/1.1",
            "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nHost: evil.example",
            "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nhost: 127.0.0.1",
            "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\nContent-Length: 2",
            "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: two",
        ] {
            assert_eq!(parse_head(broken), Err(ReadError::Malformed), "{broken:?}");
        }
        assert_eq!(
            parse_head("POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nTransfer-Encoding: chunked"),
            Err(ReadError::Unsupported)
        );
    }
}
