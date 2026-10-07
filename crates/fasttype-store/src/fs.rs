//! Écritures sûres et fichier de config.

use crate::config::Config;
use crate::paths::parent_of;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Écrit dans un fichier temporaire voisin, le synchronise, puis le renomme :
/// le fichier visé est soit l'ancien, soit le nouveau, jamais un mélange.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = parent_of(path);
    fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "chemin sans nom de fichier"))?;
    let tmp = dir.join(format!(".{}.tmp", name.to_string_lossy()));
    let result = (|| {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Charge la config. Fichier absent : défauts sans avertissement. TOML cassé :
/// défauts, et le fichier est déplacé vers `config.toml.bak` pour ne rien perdre.
pub fn load_config(path: &Path) -> (Config, Vec<String>) {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Config::defaults(), Vec::new()),
        Err(e) => {
            return (
                Config::defaults(),
                vec![format!(
                    "{} illisible ({e}) : réglages par défaut",
                    path.display()
                )],
            );
        }
    };
    let (config, warnings) = Config::from_toml(&src);
    let mut messages: Vec<String> = warnings.iter().map(ToString::to_string).collect();
    if matches!(warnings.first(), Some(crate::ConfigWarning::Syntax(_))) {
        let bak = path.with_extension("toml.bak");
        match fs::rename(path, &bak) {
            Ok(()) => messages.push(format!(
                "l'ancien fichier est conservé dans {}",
                bak.display()
            )),
            Err(e) => messages.push(format!(
                "impossible de mettre de côté {} ({e})",
                path.display()
            )),
        }
    }
    (config, messages)
}

pub fn save_config(path: &Path, config: &Config) -> io::Result<()> {
    write_atomic(path, config.to_toml().as_bytes())
}
