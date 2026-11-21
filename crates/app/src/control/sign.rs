//! The control API's signed requests (the protocol is in [`super`]'s
//! module comment): HMAC-SHA-256 (RFC 2104) over sha2's SHA-256, the text
//! a signature covers, and the nonces RVP hands out.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

/// The `Authorization` scheme of a signed request, and the first line of
/// the text it signs.
pub const SCHEME: &str = "RVP-HMAC-SHA256";

/// SHA-256's block in bytes: HMAC's key length.
const BLOCK: usize = 64;

/// HMAC-SHA-256 of `message` keyed with `key` (RFC 2104).
#[must_use]
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let pad = |with: u8| block.map(|byte| byte ^ with);
    let inner = Sha256::new()
        .chain_update(pad(0x36))
        .chain_update(message)
        .finalize();
    let outer = Sha256::new()
        .chain_update(pad(0x5c))
        .chain_update(inner)
        .finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer);
    out
}

/// Lower-case hex digits of `bytes`.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            // Writing to a String does not fail.
            let _ = write!(text, "{byte:02x}");
            text
        })
}

/// The text a request's signature covers: the lines `RVP-HMAC-SHA256`,
/// the port, the nonce, the method and the target, then the body.
fn text(port: u16, nonce: &str, method: &str, target: &str, body: &[u8]) -> Vec<u8> {
    let mut text = format!("{SCHEME}\n{port}\n{nonce}\n{method}\n{target}\n").into_bytes();
    text.extend_from_slice(body);
    text
}

/// A request's signature in lower-case hex: `target` is the request
/// line's target as sent (the path and its query, not decoded).
#[must_use]
pub fn signature(
    token: &str,
    port: u16,
    nonce: &str,
    method: &str,
    target: &str,
    body: &[u8],
) -> String {
    hex(&hmac_sha256(
        token.as_bytes(),
        &text(port, nonce, method, target, body),
    ))
}

/// The `Authorization` value of a signed request (tests and other
/// clients written in Rust).
#[must_use]
pub fn authorization(
    token: &str,
    port: u16,
    nonce: &str,
    method: &str,
    target: &str,
    body: &[u8],
) -> String {
    let signature = signature(token, port, nonce, method, target, body);
    format!("{SCHEME} nonce={nonce}, signature={signature}")
}

/// The nonce and the signature of `nonce=N, signature=S` (what follows
/// the scheme): each once, nothing else.
pub(super) fn credentials(text: &str) -> Option<(&str, &str)> {
    let (mut nonce, mut signature) = (None, None);
    for part in text.split(',') {
        let (key, value) = part.trim().split_once('=')?;
        let slot = match key {
            "nonce" => &mut nonce,
            "signature" => &mut signature,
            _ => return None,
        };
        if slot.replace(value).is_some() {
            return None;
        }
    }
    Some((nonce?, signature?))
}

/// The nonces handed out and not spent yet, oldest first.
pub(super) struct Nonces {
    live: Mutex<VecDeque<(String, Instant)>>,
    life: Duration,
    most: usize,
}

impl Nonces {
    /// Nonces that live `life` from being handed out, `most` at once.
    pub fn new(life: Duration, most: usize) -> Self {
        Self {
            live: Mutex::new(VecDeque::with_capacity(most)),
            life,
            most,
        }
    }

    /// A new nonce (the oldest goes when `most` are kept); `None` when the
    /// system gives no random bytes.
    pub fn issue(&self, now: Instant) -> Option<String> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).ok()?;
        let nonce = hex(&bytes);
        let mut live = super::lock(&self.live);
        self.forget_old(&mut live, now);
        while !live.is_empty() && live.len() >= self.most {
            live.pop_front();
        }
        live.push_back((nonce.clone(), now));
        Some(nonce)
    }

    /// Spends `nonce`: true when RVP handed it out less than its life ago
    /// and no request named it before.
    pub fn spend(&self, nonce: &str, now: Instant) -> bool {
        let mut live = super::lock(&self.live);
        self.forget_old(&mut live, now);
        match live.iter().position(|(known, _)| known == nonce) {
            Some(at) => {
                live.remove(at);
                true
            }
            None => false,
        }
    }

    fn forget_old(&self, live: &mut VecDeque<(String, Instant)>, now: Instant) {
        while live
            .front()
            .is_some_and(|(_, issued)| now.saturating_duration_since(*issued) >= self.life)
        {
            live.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hex"))
            .collect()
    }

    /// RFC 4231's test cases 1 to 4, 6 and 7 (5 truncates the output).
    #[test]
    fn hmac_matches_rfc_4231() {
        let long_key = [0xaa; 131];
        let cases: [(&[u8], &[u8], &str); 6] = [
            (
                &[0x0b; 20],
                b"Hi There",
                "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
            ),
            (
                b"Jefe",
                b"what do ya want for nothing?",
                "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            ),
            (
                &[0xaa; 20],
                &[0xdd; 50],
                "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
            ),
            (
                &unhex("0102030405060708090a0b0c0d0e0f10111213141516171819"),
                &[0xcd; 50],
                "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b",
            ),
            (
                &long_key,
                b"Test Using Larger Than Block-Size Key - Hash Key First",
                "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
            ),
            (
                &long_key,
                b"This is a test using a larger than block-size key and a larger than \
                  block-size data. The key needs to be hashed before being used by the \
                  HMAC algorithm.",
                "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
            ),
        ];
        for (key, message, expected) in cases {
            assert_eq!(hex(&hmac_sha256(key, message)), expected);
        }
    }

    #[test]
    fn a_signature_covers_every_part() {
        let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let nonce = "00112233445566778899aabbccddeeff";
        let base = signature(token, 48931, nonce, "POST", "/api/seek", br#"{"to":12}"#);
        assert_eq!(base.len(), 64);
        for other in [
            signature(token, 48932, nonce, "POST", "/api/seek", br#"{"to":12}"#),
            signature(
                token,
                48931,
                "00112233445566778899aabbccddeefe",
                "POST",
                "/api/seek",
                br#"{"to":12}"#,
            ),
            signature(token, 48931, nonce, "GET", "/api/seek", br#"{"to":12}"#),
            signature(token, 48931, nonce, "POST", "/api/seek?", br#"{"to":12}"#),
            signature(token, 48931, nonce, "POST", "/api/seek", br#"{"to":13}"#),
            signature(
                &token.replace('0', "1"),
                48931,
                nonce,
                "POST",
                "/api/seek",
                br#"{"to":12}"#,
            ),
        ] {
            assert_ne!(other, base);
        }
        assert_eq!(
            authorization(token, 48931, nonce, "POST", "/api/seek", br#"{"to":12}"#),
            format!("RVP-HMAC-SHA256 nonce={nonce}, signature={base}")
        );
    }

    #[test]
    fn credentials_name_a_nonce_and_a_signature_once() {
        assert_eq!(credentials("nonce=ab, signature=cd"), Some(("ab", "cd")));
        assert_eq!(credentials("signature=cd,nonce=ab"), Some(("ab", "cd")));
        assert_eq!(credentials("nonce=ab"), None);
        assert_eq!(credentials("nonce=ab, nonce=ab, signature=cd"), None);
        assert_eq!(credentials("nonce=ab, signature=cd, realm=x"), None);
        assert_eq!(credentials("nonce ab, signature=cd"), None);
    }

    #[test]
    fn nonces_serve_once_within_their_life_and_are_bounded() {
        let start = Instant::now();
        let nonces = Nonces::new(Duration::from_secs(10), 3);
        let first = nonces.issue(start).expect("nonce");
        assert!(first.len() == 32 && first.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(nonces.spend(&first, start));
        assert!(!nonces.spend(&first, start), "spent");
        assert!(!nonces.spend("00112233445566778899aabbccddeeff", start));
        // Too old.
        let old = nonces.issue(start).expect("nonce");
        assert!(!nonces.spend(&old, start + Duration::from_secs(10)));
        // At most three: a fourth drops the oldest.
        let kept: Vec<String> = (0..4)
            .map(|_| nonces.issue(start).expect("nonce"))
            .collect();
        assert!(!nonces.spend(&kept[0], start));
        for nonce in &kept[1..] {
            assert!(nonces.spend(nonce, start + Duration::from_secs(9)));
        }
        assert_eq!(super::super::lock(&nonces.live).len(), 0);
    }
}
