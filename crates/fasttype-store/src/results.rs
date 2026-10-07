//! Historique des résultats : un `TestResult` JSON par ligne, ajouté en une
//! seule écriture à la fin du test.

use fasttype_core::result::TestResult;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedResults {
    pub results: Vec<TestResult>,
    /// Lignes illisibles ignorées (écriture interrompue, édition à la main).
    pub skipped_lines: usize,
}

pub struct ResultLog {
    path: PathBuf,
}

/// Vrai si le fichier est vide ou finit par un saut de ligne.
fn ends_with_newline(file: &mut File) -> io::Result<bool> {
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok(true);
    }
    file.seek(SeekFrom::Start(len - 1))?;
    let mut last = [0u8; 1];
    file.read_exact(&mut last)?;
    Ok(last[0] == b'\n')
}

impl ResultLog {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Ajoute une ligne. Si la dernière ligne a été tronquée par un plantage,
    /// on repart sur une nouvelle ligne pour ne pas coller les deux.
    pub fn append(&self, r: &TestResult) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)?;
        let mut line = if ends_with_newline(&mut file)? {
            String::new()
        } else {
            String::from("\n")
        };
        line.push_str(&serde_json::to_string(r).map_err(io::Error::other)?);
        line.push('\n');
        file.write_all(line.as_bytes())
    }

    pub fn load(&self) -> io::Result<LoadedResults> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        };
        let mut results = Vec::new();
        let mut skipped_lines = 0;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str(line) {
                Ok(r) => results.push(r),
                Err(_) => skipped_lines += 1,
            }
        }
        Ok(LoadedResults {
            results,
            skipped_lines,
        })
    }
}
