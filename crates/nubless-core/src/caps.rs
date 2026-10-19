//! Parser for the MCCS capabilities string a monitor returns over DDC/CI,
//! e.g. `(prot(monitor)type(lcd)model(WK95U)cmds(01 02)vcp(02 10 14(05 08) F7(00 01 02 03))mccs_ver(2.1))`.

use std::collections::BTreeMap;

/// What the monitor says it supports.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// The raw string, kept for bug reports.
    pub raw: String,
    /// `model(...)`. **Not trustworthy**: the 27GM950B reports `WK95U`.
    pub model: Option<String>,
    pub mccs_version: Option<String>,
    /// Declared VCP codes and, if listed, their declared values.
    pub vcp: BTreeMap<u8, Vec<u8>>,
}

impl Capabilities {
    pub fn declares(&self, code: u8) -> bool {
        self.vcp.contains_key(&code)
    }

    pub fn parse(raw: &str) -> Result<Self, CapsError> {
        let mut caps = Capabilities {
            raw: raw.to_string(),
            ..Default::default()
        };
        caps.model = top_level_field(raw, "model").map(|s| s.trim().to_string());
        caps.mccs_version = top_level_field(raw, "mccs_ver").map(|s| s.trim().to_string());
        let vcp = top_level_field(raw, "vcp").ok_or(CapsError::NoVcpSection)?;
        caps.vcp = parse_vcp_list(vcp)?;
        Ok(caps)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CapsError {
    #[error("capabilities string has no vcp(...) section")]
    NoVcpSection,
    #[error("unbalanced parentheses in capabilities string")]
    Unbalanced,
    #[error("bad hex token {0:?} in capabilities string")]
    BadHex(String),
}

/// Returns the contents of `name(...)`, matching nested parentheses.
fn top_level_field<'a>(raw: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}(");
    let start = raw.find(&needle)? + needle.len();
    let mut depth = 1usize;
    for (i, ch) in raw[start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&raw[start..start + i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_vcp_list(s: &str) -> Result<BTreeMap<u8, Vec<u8>>, CapsError> {
    let mut out = BTreeMap::new();
    let mut chars = s.chars().peekable();
    let mut last: Option<u8> = None;
    let mut token = String::new();

    let flush = |token: &mut String, out: &mut BTreeMap<u8, Vec<u8>>, last: &mut Option<u8>| {
        if token.is_empty() {
            return Ok(());
        }
        let code = u8::from_str_radix(token, 16).map_err(|_| CapsError::BadHex(token.clone()))?;
        out.entry(code).or_default();
        *last = Some(code);
        token.clear();
        Ok(())
    };

    while let Some(ch) = chars.next() {
        match ch {
            '(' => {
                flush(&mut token, &mut out, &mut last)?;
                let code = last.ok_or(CapsError::Unbalanced)?;
                let mut inner = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == ')' {
                        closed = true;
                        break;
                    }
                    if c == '(' {
                        return Err(CapsError::Unbalanced);
                    }
                    inner.push(c);
                }
                if !closed {
                    return Err(CapsError::Unbalanced);
                }
                let values = inner
                    .split_whitespace()
                    .map(|t| u8::from_str_radix(t, 16).map_err(|_| CapsError::BadHex(t.into())))
                    .collect::<Result<Vec<_>, _>>()?;
                out.entry(code).or_default().extend(values);
            }
            ')' => return Err(CapsError::Unbalanced),
            c if c.is_ascii_hexdigit() => {
                token.push(c);
                if token.len() == 2 {
                    flush(&mut token, &mut out, &mut last)?;
                }
            }
            c if c.is_whitespace() => flush(&mut token, &mut out, &mut last)?,
            c => return Err(CapsError::BadHex(c.to_string())),
        }
    }
    flush(&mut token, &mut out, &mut last)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::gm950b::CAPABILITIES_SAMPLE;

    #[test]
    fn parses_real_27gm950b_string() {
        let caps = Capabilities::parse(CAPABILITIES_SAMPLE).unwrap();
        assert_eq!(caps.model.as_deref(), Some("WK95U"));
        assert_eq!(caps.mccs_version.as_deref(), Some("2.1"));
        assert!(caps.declares(0x10));
        assert!(caps.declares(0xF9));
        assert_eq!(caps.vcp[&0xF7], vec![0, 1, 2, 3]);
        assert_eq!(caps.vcp[&0x60], vec![0x11, 0x12, 0x0F, 0x10]);
        assert!(caps.vcp[&0x10].is_empty());
        assert!(
            !caps.declares(0x87),
            "sharpness is not declared on this unit"
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(
            Capabilities::parse("(prot(monitor))"),
            Err(CapsError::NoVcpSection)
        );
        assert!(Capabilities::parse("vcp(10 12(01)").is_err());
        assert!(Capabilities::parse("vcp(ZZ)").is_err());
    }
}
