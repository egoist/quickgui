//! Shared Sparkle appcast and raw Ed25519 verification. No unsigned payload is ever executed.
use super::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};

pub const MAX_FEED_BYTES: usize = 1024 * 1024;
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const SPARKLE: &str = "http://www.andymatuschak.org/xml-namespaces/sparkle";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Item {
    pub version: String,
    pub url: String,
    pub signature: String,
    pub length: u64,
    pub notes: String,
}

pub fn https_url(raw: &str) -> Result<url::Url> {
    let url = url::Url::parse(raw).map_err(|_| "invalid update URL")?;
    if raw.len() > 4096
        || url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("update URLs must use HTTPS without credentials or fragments".into());
    }
    Ok(url)
}

pub fn verifying_key(raw: &str) -> Result<VerifyingKey> {
    let bytes = STANDARD
        .decode(raw.trim())
        .map_err(|_| "publicKey must be a base64 Ed25519 key")?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "publicKey must decode to 32 bytes")?;
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| "invalid Ed25519 public key")?;
    if key.is_weak() {
        return Err("weak Ed25519 public key".into());
    }
    Ok(key)
}

pub fn verify(bytes: &[u8], item: &Item, key: &str) -> Result<()> {
    if bytes.len() as u64 != item.length || item.length == 0 || item.length > MAX_DOWNLOAD_BYTES {
        return Err("update length does not match the signed appcast".into());
    }
    let signature = STANDARD
        .decode(&item.signature)
        .map_err(|_| "invalid update signature encoding")?;
    let signature =
        Signature::from_slice(&signature).map_err(|_| "invalid update signature length")?;
    verifying_key(key)?
        .verify_strict(bytes, &signature)
        .map_err(|_| "update signature verification failed".into())
}

pub fn newest(xml: &str, current: &str, os: &str) -> Result<Option<Item>> {
    if xml.len() > MAX_FEED_BYTES {
        return Err("appcast exceeds 1 MiB".into());
    }
    // DTDs/entities are deliberately disabled (roxmltree's default). Keep the namespace exact,
    // decode XML escapes, and examine only channel/item enclosures, never delta payloads.
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("invalid appcast: {e}"))?;
    if doc.root_element().tag_name().name() != "rss" {
        return Err("expected a Sparkle RSS appcast".into());
    }
    let current = Version::parse(current).map_err(|_| "invalid current version")?;
    let mut newest: Option<(Version, Item)> = None;
    for item in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("channel"))
        .flat_map(|n| n.children())
        .filter(|n| n.has_tag_name("item"))
    {
        let text = |name: &str| {
            item.children()
                .find(|n| n.has_tag_name((SPARKLE, name)))
                .and_then(|n| n.text())
        };
        for enclosure in item.children().filter(|n| n.has_tag_name("enclosure")) {
            let attr = |name: &str| enclosure.attribute((SPARKLE, name));
            if attr("os").is_some_and(|v| v != os) || attr("deltaFrom").is_some() {
                continue;
            }
            // Deliberately scope portable feeds to SemVer releases. Unknown channels and
            // Sparkle rollout/minimum-system constraints must not silently become eligible.
            if text("channel").is_some()
                || text("phasedRolloutInterval").is_some()
                || text("minimumAutoupdateVersion").is_some()
                || text("maximumSystemVersion").is_some()
                || text("minimumSystemVersion").is_some_and(|minimum| !supports_system(minimum))
            {
                continue;
            }
            let Some(raw) = text("shortVersionString")
                .or_else(|| attr("shortVersionString"))
                .or_else(|| text("version"))
                .or_else(|| attr("version"))
            else {
                continue;
            };
            let Ok(version) = Version::parse(raw) else {
                continue;
            };
            if !version.cmp_precedence(&current).is_gt()
                || (!version.pre.is_empty() && current.pre.is_empty())
            {
                continue;
            }
            let Some(url) = enclosure.attribute("url").filter(|s| https_url(s).is_ok()) else {
                continue;
            };
            let Some(signature) =
                attr("edSignature").filter(|s| STANDARD.decode(s).is_ok_and(|v| v.len() == 64))
            else {
                continue;
            };
            let Some(length) = enclosure
                .attribute("length")
                .and_then(|s| s.parse::<u64>().ok())
                .filter(|n| *n > 0 && *n <= MAX_DOWNLOAD_BYTES)
            else {
                continue;
            };
            let notes = item
                .children()
                .find(|n| n.has_tag_name("description"))
                .and_then(|n| n.text())
                .unwrap_or("");
            if notes.len() > 16 * 1024 {
                return Err("update notes exceed 16 KiB".into());
            }
            if newest
                .as_ref()
                .is_none_or(|(v, _)| version.cmp_precedence(v).is_gt())
            {
                newest = Some((
                    version,
                    Item {
                        version: raw.into(),
                        url: url.into(),
                        signature: signature.into(),
                        length,
                        notes: notes.into(),
                    },
                ));
            }
        }
    }
    Ok(newest.map(|(_, item)| item))
}

// Windows appcasts commonly declare 10.0.17763 (as Waku does). Read the real OS version rather
// than an application-manifest compatibility version. Unknown constraints fail closed.
fn supports_system(minimum: &str) -> bool {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct VersionInfo {
            size: u32,
            major: u32,
            minor: u32,
            build: u32,
            platform: u32,
            service_pack: [u16; 128],
        }
        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn RtlGetVersion(info: *mut VersionInfo) -> i32;
        }
        let mut info = VersionInfo {
            size: std::mem::size_of::<VersionInfo>() as u32,
            major: 0,
            minor: 0,
            build: 0,
            platform: 0,
            service_pack: [0; 128],
        };
        let Some(required) = system_version(minimum) else {
            return false;
        };
        (unsafe { RtlGetVersion(&mut info) }) == 0
            && (info.major, info.minor, info.build) >= required
    }
    #[cfg(not(windows))]
    {
        let _ = minimum;
        false
    }
}
fn system_version(raw: &str) -> Option<(u32, u32, u32)> {
    let parts = raw
        .split('.')
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    Some((
        parts[0],
        *parts.get(1).unwrap_or(&0),
        *parts.get(2).unwrap_or(&0),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    fn feed(version: &str, attributes: &str) -> String {
        format!(
            r#"<rss xmlns:s="{SPARKLE}"><channel><item><s:shortVersionString>{version}</s:shortVersionString><enclosure url="https://example.com/app?x=1&amp;y=2" length="3" s:edSignature="{}" {attributes}/></item></channel></rss>"#,
            STANDARD.encode([0; 64])
        )
    }
    #[test]
    fn parses_namespace_and_escapes_filters_os_and_prereleases() {
        let f = feed("1.10.0", "s:os='linux'");
        assert!(
            newest(&f, "1.9.0", "linux")
                .unwrap()
                .unwrap()
                .url
                .ends_with("x=1&y=2")
        );
        assert!(newest(&f, "1.9.0", "windows").unwrap().is_none());
        assert!(newest(&f, "1.10.0", "linux").unwrap().is_none());
        assert!(
            newest(&feed("2.0.0-beta.1", ""), "1.9.0", "linux")
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn rejects_unsigned_oversize_invalid_and_downgraded_feeds() {
        let f = feed("2.0.0", "");
        assert!(
            newest(&f.replace("s:edSignature=", "unsigned="), "1.0.0", "linux")
                .unwrap()
                .is_none()
        );
        assert!(
            newest(&f.replace("https:", "http:"), "1.0.0", "linux")
                .unwrap()
                .is_none()
        );
        assert!(newest("not xml", "1.0.0", "linux").is_err());
        assert!(newest(&" ".repeat(MAX_FEED_BYTES + 1), "1.0.0", "linux").is_err());
        assert!(
            newest(&feed("2.0.0", "s:deltaFrom='1.0.0'"), "1.0.0", "linux")
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn verifies_sparkle_raw_ed25519_and_rejects_tampering() {
        let key = SigningKey::from_bytes(&[42; 32]);
        let mut item = Item {
            version: "2.0.0".into(),
            url: "https://example.com/app".into(),
            signature: STANDARD.encode(key.sign(b"abc").to_bytes()),
            length: 3,
            notes: String::new(),
        };
        let public = STANDARD.encode(key.verifying_key().to_bytes());
        verify(b"abc", &item, &public).unwrap();
        assert!(verify(b"abd", &item, &public).is_err());
        item.length = 4;
        assert!(verify(b"abc", &item, &public).is_err());
        assert!(verifying_key(&STANDARD.encode([0; 32])).is_err());
    }
}
