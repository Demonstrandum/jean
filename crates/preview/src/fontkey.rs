//! Licensed fonts ship age-encrypted; this decrypts them at startup with the key in `secrets/font.key`, into memory only
//! (the plaintext never touches the disk). No key, or a wrong one: the font is simply not served and the UI uses the system monospace.

use std::io::Read;
use std::path::Path;
use std::str::FromStr;

/// Decrypt age `cipher` with the X25519 identity in `key_file` (an `age-keygen` file).
pub fn decrypt(cipher: &[u8], key_file: &Path) -> Result<Vec<u8>, String> {
    let text = std::fs::read_to_string(key_file).map_err(|e| format!("no key at {}: {e}", key_file.display()))?;
    let secret = text.lines().map(str::trim).find(|l| l.starts_with("AGE-SECRET-KEY-")).ok_or("no AGE-SECRET-KEY line in the key file")?;
    let id = age::x25519::Identity::from_str(secret).map_err(|e| e.to_string())?;
    let decryptor = age::Decryptor::new(cipher).map_err(|e| format!("not an age file (a Git LFS pointer? run `git lfs pull`): {e}"))?;
    let mut reader = decryptor.decrypt(std::iter::once(&id as &dyn age::Identity)).map_err(|e| format!("cannot decrypt (wrong key?): {e}"))?;
    let mut out = Vec::new();
    reader.read_to_end(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_file(id: &age::x25519::Identity, name: &str) -> std::path::PathBuf {
        use age::secrecy::ExposeSecret;
        let p = std::env::temp_dir().join(format!("fontkey-{}-{name}", std::process::id()));
        std::fs::write(&p, format!("# created: test\n# public key: {}\n{}\n", id.to_public(), id.to_string().expose_secret())).unwrap();
        p
    }

    #[test]
    fn decrypts_with_the_right_key_and_not_the_wrong_one() {
        let (right, wrong) = (age::x25519::Identity::generate(), age::x25519::Identity::generate());
        let cipher = age::encrypt(&right.to_public(), b"wOF2 pretend font").unwrap();
        assert_eq!(decrypt(&cipher, &key_file(&right, "a")).unwrap(), b"wOF2 pretend font");
        assert!(decrypt(&cipher, &key_file(&wrong, "b")).unwrap_err().contains("wrong key"));
        assert!(decrypt(&cipher, Path::new("/nonexistent/font.key")).unwrap_err().contains("no key"));
    }

    #[test]
    fn an_lfs_pointer_is_reported_as_such() {
        let id = age::x25519::Identity::generate();
        let e = decrypt(b"version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 1\n", &key_file(&id, "c")).unwrap_err();
        assert!(e.contains("git lfs pull"), "{e}");
    }
}
