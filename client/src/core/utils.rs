use merkle_tree::{merkle_tree::MerkleTree, utils::hash_bytes};
use rayon::prelude::*;
use rayon::prelude::*;
use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::errors::SynxClientError;

/// List the names of files in a given directory.
/// This function only lists files, not directories. It iterates over
/// each entry in the specified directory. If an entry is a file (not
/// a directory), its name is extracted and added to the resulting vector. Subdirectories are ignored.
pub fn list_files_in_dir(dir_path: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if dir_path.is_dir() {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            if entry.path().is_file() {
                files.push(entry.path());
            }
        }
    }

    Ok(files)
}

// pub fn files_to_bytes<P: AsRef<Path>>(files: &Vec<P>) -> Result<Vec<Vec<u8>>, SynxClientError> {
//   let files_as_bytes = files.par_iter().map(|file| {

//   })
// }

/// Reads a file and accumulates its contents into a `Vec<u8>`.
///
/// This function reads the file in chunks to efficiently handle large files while
/// minimizing memory usage. It's suitable for a wide range of file sizes, from small
/// to large, as it does not load the entire file into memory at once.
///
/// Note: While this method is more memory-efficient than reading the entire file at once,
/// the resulting `Vec<u8>` will still contain the entire file content in memory.
/// Hence, very large files may still pose memory constraints.
///
/// # Arguments
///
/// * `path` - The path to the file to be read.
///
/// # Returns
///
/// An `io::Result<Vec<u8>>` which is a vector containing all the bytes of the file,
/// or an `io::Error` in case of any error during reading.
pub fn file_to_bytes<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    let mut reader = io::BufReader::new(file);
    let mut buffer = Vec::new();
    let mut chunk = vec![0; 8192];

    while let Ok(bytes_read) = reader.read(&mut chunk) {
        if bytes_read == 0 {
            break; // End of file reached
        }

        buffer.extend_from_slice(&chunk[..bytes_read]);
    }

    Ok(buffer)
}

/// Generates a Merkle tree from the contents of multiple files.
///
/// This function utilizes `rayon` for parallel processing, significantly improving
/// performance when handling multiple files, particularly beneficial for large files
/// or a large number of files. Each file is read in parallel and converted into a
/// byte array, which forms the leaves of the Merkle tree.
///
/// # Arguments
///
/// * `paths` - A vector of paths (`Vec<P>`), where each path points to a file.
///     `P` must implement `AsRef<Path>` and must be safe to send across threads
///     (`Send`) and access from multiple threads (`Sync`).
///
/// # Returns
///
/// Returns a `Result<MerkleTree, SynxClientError>`. On success, it contains the
/// Merkle tree constructed from the files' contents. On failure, it returns a
/// `SynxClientError` indicating the type of error encountered, such as an issue
/// reading the files.
///
/// # Errors
///
/// Returns an error if any file cannot be read or converted to bytes.
pub fn generate_merkle_tree<P: AsRef<Path>>(paths: Vec<P>) -> Result<MerkleTree, SynxClientError>
where
    P: AsRef<Path> + Send + Sync,
{
    let leaf_bytes_results: Vec<Result<Vec<u8>, SynxClientError>> = paths
        .par_iter()
        .map(|path| {
            file_to_bytes(path.as_ref())
                .map_err(|err| SynxClientError::FileToBytesConversionError(err.to_string()))
        })
        .collect();

    let leaf_bytes: Vec<Vec<u8>> = leaf_bytes_results.into_iter().collect::<Result<_, _>>()?;

    Ok(MerkleTree::new(&leaf_bytes))
}

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
            let mut file = fs::File::create(temp_dir_path.join(file_name)).unwrap();
        }

        let files = list_files_in_dir(&temp_dir_path.to_path_buf()).unwrap();
        let retrieved_file_names: Vec<_> = files
            .into_iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();

        assert_eq!(retrieved_file_names.len(), file_names.len());
        for file_name in &file_names {
            assert!(retrieved_file_names.contains(&file_name.to_string()));
        }
    }

    #[test]
    fn should_convert_file_to_bytes() {
        // Create a temporary directory
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("mock_file.txt");

        // Precomputed content
        let content = b"Hello, world!";

        // Create and write to mock file
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(content).unwrap();

        // Use file_to_bytes to read the file into bytes
        let file_bytes = file_to_bytes(&file_path).unwrap();
        assert_eq!(file_bytes, content);

        println!("SEE: {:?}", generate_merkle_tree(vec![file_path]));
    }

    #[test]
    fn should_generate_merkle_tree() {
        let (leaf_a, leaf_b, leaf_c, leaf_d) = (b"leaf_a", b"leaf_b", b"leaf_c", b"leaf_d");

        let mut leaf_bytes = vec![
            leaf_a.to_vec(),
            leaf_b.to_vec(),
            leaf_c.to_vec(),
            leaf_d.to_vec(),
        ];

        let merkle_tree = MerkleTree::new(&leaf_bytes);

        let mut leaf_hashes = leaf_bytes
            .iter()
            .map(|bytes| hash_bytes(bytes))
            .collect::<Vec<String>>();

        leaf_hashes.sort();
        // Verify the leaf nodes used in generating the merkle tree
        assert!(leaf_hashes == MerkleTree::build_leaf_nodes(&leaf_bytes));

        // Verify the leaf nodes in the merkle tree
        assert!(merkle_tree.nodes[0] == leaf_hashes);
    }
}
