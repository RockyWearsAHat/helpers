//! `lint_sign` — the cryptographic seam of the module-sharing channel (native/architecture.dx, "The
//! sharing channel is SIGNED, MONITORED, and never direct").
//!
//! Every machine holds an Ed25519 keypair (generated on first submission); every submission
//! manifest and every promoted registry index is signed; every consumer verifies against the
//! trusted registry keys before anything shared can reach the loaded engine. Client-side keys
//! give attribution and tamper-evidence — the honesty control is the monitored promotion step
//! this signing makes meaningful.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::path::PathBuf;

/// Where this machine's signing key lives. Private key material — never shared, never in any
/// repository; deleting it simply mints a new identity on the next submission.
fn key_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/helpers/signing.key")
}

/// This machine's signing key, generated on first use (0600 on unix).
pub fn machine_key() -> Result<SigningKey, String> {
    let path = key_path();
    if let Ok(raw) = std::fs::read(&path) {
        let bytes: [u8; 32] =
            raw.try_into().map_err(|_| "lint_sign: signing key is corrupt — delete it to mint a new identity".to_string())?;
        return Ok(SigningKey::from_bytes(&bytes));
    }
    let key = SigningKey::generate(&mut rand_core::OsRng);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("lint_sign: {e}"))?;
    }
    std::fs::write(&path, key.to_bytes()).map_err(|e| format!("lint_sign: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

/// Hex of this machine's public key — the submitter fingerprint carried in manifests.
pub fn machine_fingerprint() -> Result<String, String> {
    Ok(hex(machine_key()?.verifying_key().as_bytes()))
}

/// Sign `payload` with the machine key; returns the signature as hex.
pub fn sign(payload: &[u8]) -> Result<String, String> {
    Ok(hex(&machine_key()?.sign(payload).to_bytes()))
}

/// Verify hex `signature` over `payload` against a hex-encoded Ed25519 public key.
pub fn verify(payload: &[u8], signature_hex: &str, pubkey_hex: &str) -> bool {
    let Some(sig_bytes) = unhex(signature_hex) else { return false };
    let Some(key_bytes) = unhex(pubkey_hex) else { return false };
    let Ok(sig_arr): Result<[u8; 64], _> = sig_bytes.try_into() else { return false };
    let Ok(key_arr): Result<[u8; 32], _> = key_bytes.try_into() else { return false };
    let Ok(key) = VerifyingKey::from_bytes(&key_arr) else { return false };
    key.verify(payload, &Signature::from_bytes(&sig_arr)).is_ok()
}

/// Generate a fresh keypair as `(private_hex, public_hex)` — promotion tooling and the
/// contract tests mint registry identities through this, never by hand.
pub fn generate_keypair() -> (String, String) {
    let key = SigningKey::generate(&mut rand_core::OsRng);
    (hex(&key.to_bytes()), hex(key.verifying_key().as_bytes()))
}

/// Sign `payload` with an explicit hex-encoded private key (the registry key at promotion).
pub fn sign_with(payload: &[u8], private_hex: &str) -> Option<String> {
    let bytes: [u8; 32] = unhex(private_hex)?.try_into().ok()?;
    Some(hex(&SigningKey::from_bytes(&bytes).sign(payload).to_bytes()))
}

/// SHA-256 of `bytes`, hex — the content hash the signed manifests and indexes carry.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    hex(&h.finalize())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_verify_and_tampering_is_detected() {
        let dir = std::env::temp_dir().join(format!("lint-sign-test-{}", std::process::id()));
        let _env = crate::test_env_lock();
        std::env::set_var("HOME", &dir);
        let payload = b"module manifest bytes";
        let sig = sign(payload).expect("signs");
        let fp = machine_fingerprint().expect("fingerprint");
        assert!(verify(payload, &sig, &fp), "a genuine signature verifies");
        assert!(!verify(b"tampered bytes", &sig, &fp), "any byte change breaks the signature");
        assert!(!verify(payload, &sig, &fp.replace('a', "b")), "a different key never verifies");
        let sig2 = sign(payload).expect("stable key");
        assert!(verify(payload, &sig2, &fp), "the machine identity persists across calls");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn content_hashes_pin_exact_bytes() {
        assert_eq!(sha256_hex(b"abc").len(), 64);
        assert_ne!(sha256_hex(b"abc"), sha256_hex(b"abd"));
    }
}
