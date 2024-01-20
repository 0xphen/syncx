use merkle_tree::merkle_tree::MerkleTree;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use super::errors::SynxClientError;

/// List the names of files in a given directory.
/// This function only lists files, not directories. It iterates over
/// each entry in the specified directory. If an entry is a file (not
/// a directory), its name is extracted and added to the resulting vector. Subdirectories are ignored.
pub fn list_dir_files(path: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = entry.path().file_name() {
                    files.push(PathBuf::from(file_name));
                }
            }
        }
    }

    Ok(files)
}

pub fn generate_merkle_root(path: &PathBuf) -> Result<String, SynxClientError> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn should_get_files_in_dir() {
        // Create a temporary directory
        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.path();

        // Create files in the temporary directory
        let file_names = vec!["file1.txt", "file2.txt"];
        for file_name in &file_names {
            let mut file = File::create(temp_dir_path.join(file_name)).unwrap();
        }

        let files = list_dir_files(&temp_dir_path.to_path_buf()).unwrap();
        let retrieved_file_names: Vec<_> = files
            .into_iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();

        assert_eq!(retrieved_file_names.len(), file_names.len());
        for file_name in &file_names {
            assert!(retrieved_file_names.contains(&file_name.to_string()));
        }
    }
}
