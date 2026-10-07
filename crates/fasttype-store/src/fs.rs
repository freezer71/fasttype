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

/// Charge la config. Fichier absent : défauts sans avertissement. Dès que le
/// fichier n'est pas relu tel quel (TOML cassé, pas en UTF-8, clé inconnue ou
/// invalide), l'original est conservé dans `config.toml.bak` : la prochaine
/// sauvegarde réécrira `config.toml`, et rien de ce qu'a écrit l'utilisateur
/// ne doit se perdre. Un fichier illisible en entier est déplacé, un fichier
/// en partie valide est copié.
pub fn load_config(path: &Path) -> (Config, Vec<String>) {
    let bytes = match fs::read(path) {
        Ok(b) => b,
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
    let (config, mut messages, unusable) = match std::str::from_utf8(&bytes) {
        Err(_) => (
            Config::defaults(),
            vec![format!(
                "{} n'est pas en UTF-8 : réglages par défaut",
                path.display()
            )],
            true,
        ),
        Ok(src) => {
            let (config, warnings) = Config::from_toml(src);
            let unusable = matches!(warnings.first(), Some(crate::ConfigWarning::Syntax(_)));
            (
                config,
                warnings.iter().map(ToString::to_string).collect(),
                unusable,
            )
        }
    };
    if messages.is_empty() {
        return (config, messages);
    }
    let bak = path.with_extension("toml.bak");
    let kept = if unusable {
        fs::rename(path, &bak)
    } else {
        write_atomic(&bak, &bytes)
    };
    messages.push(match kept {
        Ok(()) => format!("l'ancien fichier est conservé dans {}", bak.display()),
        Err(e) => format!("impossible de mettre de côté {} ({e})", path.display()),
    });
    (config, messages)
}

pub fn save_config(path: &Path, config: &Config) -> io::Result<()> {
    write_atomic(path, config.to_toml().as_bytes())
}
