//! The one list of what never stands on the screen however tracked it is (`docs/code-review.md`, 「What is
//! excluded」): files whose contents are a credential, and folders named as their home.

// The endings of a key or a certificate.
const KEY_ENDINGS: [&str; 6] = [".pem", ".key", ".p12", ".pfx", ".jks", ".keystore"];
// The names of a credentials file, before its ending.
const SECRET_STEMS: [&str; 3] = ["credentials", "secret", "secrets"];
// The endings a credentials file is written with: none, or a configuration's or a data file's.
const DATA_ENDINGS: [&str; 7] = ["", ".json", ".yaml", ".yml", ".toml", ".ini", ".txt"];
// The endings of an environment file's sample, which holds no value.
const SAMPLE_ENDINGS: [&str; 3] = [".example", ".sample", ".template"];
// The names an SSH key's file begins with.
const SSH_KEYS: [&str; 4] = ["id_rsa", "id_dsa", "id_ecdsa", "id_ed25519"];
// The folder name a secret's home goes by.
const SECRET_FOLDER: &str = "env";

/// A name's ending from its last dot, lowered; none for a name with no dot or one that only begins with it.
fn ending_of(name: &str) -> String {
  match name.rfind('.') {
    Some(dot) if dot > 0 => name[dot..].to_lowercase(),
    _ => String::new(),
  }
}

/// Whether a file of a checkout is a credential: under a folder named as a secret's home, an environment file
/// that is not a sample, a key, an SSH key, a login file, or a credentials or secrets file.
pub fn is_secret_path(path: &str) -> bool {
  let mut parts: Vec<&str> = path.split('/').collect();
  let name = parts.pop().unwrap_or_default().to_lowercase();

  if parts.iter().any(|part| part.eq_ignore_ascii_case(SECRET_FOLDER)) {
    return true;
  }

  if (name == ".env" || name.starts_with(".env."))
    && !SAMPLE_ENDINGS.iter().any(|ending| name.ends_with(ending))
  {
    return true;
  }

  let ending = ending_of(&name);

  if KEY_ENDINGS.contains(&ending.as_str()) {
    return true;
  }

  if SSH_KEYS.iter().any(|key| name.starts_with(key)) || name == ".netrc" || name == ".pgpass" {
    return true;
  }

  let stem = name.strip_suffix(ending.as_str()).unwrap_or(&name);

  SECRET_STEMS.contains(&stem) && DATA_ENDINGS.contains(&ending.as_str())
}

/// Whether a folder of a checkout hides itself and everything under it: a part of its path named as a secret's
/// home.
pub fn is_secret_folder(path: &str) -> bool {
  path.split('/').any(|part| part.eq_ignore_ascii_case(SECRET_FOLDER))
}
