use env_logger::{Builder, Env};
use merkle_tree::merkle_tree::MerkleTree;
use rayon::prelude::*;
use std::fs::{self, read_dir, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

use super::errors::CommonError;

/// Creates a ZIP archive from a collection of file paths, efficiently handling large files by streaming.
///
/// This function zips a list of files specified in `file_paths` into a single ZIP archive
/// located at `output_path`. It uses streaming to handle large files efficiently and applies DEFLATE
/// compression to each file for effective space saving.
///
/// # Arguments
///
/// * `file_paths` - A slice of paths (`&[P]`) to the files to be included in
///                  the ZIP archive. `P` must implement `AsRef<Path>`, allowing various
///                  path-like types (such as `&str`, `String`, `Path`, `PathBuf`).
/// * `output_path` - The destination path (`P`) for the resulting ZIP archive.
///                   Like `file_paths`, `P` must implement `AsRef<Path>`.
///
/// # Returns
///
/// Returns an `io::Result<()>`. On successful execution, it returns `Ok(())`.
/// If an error occurs (such as a file not being found, read/write errors, or invalid file names),
///  it returns `io::Error`.
///
/// # Error Handling
///
/// The function includes robust error handling. It checks for the existence of files,
/// validates file names for UTF-8 encoding,
/// and handles any I/O errors encountered during ZIP file creation, file reading, or writing.
///
/// # Efficiency with Large Files
///
/// The function reads each file in chunks (buffer size: 4096 bytes) rather than
/// loading the entire content into memory. This streaming approach is particularly
/// beneficial for handling large files, as it significantly reduces memory usage.
///
/// # Compression Method
///
/// Each file in the ZIP archive is compressed using the DEFLATE compression
///  algorithm, which provides a good balance between compression ratio and
///  speed, making it suitable for a wide range of file types.
pub fn zip_files<P: AsRef<Path>>(file_paths: &[P], output_path: &P) -> io::Result<()> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    for path in file_paths {
        let path = path.as_ref();
        if path.is_file() {
            let file_name = path
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid file name"))?;

            zip.start_file(file_name, options)?;
            let mut file = BufReader::new(File::open(path)?);
            let mut buffer = [0; 4096];

            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                zip.write_all(&buffer[..bytes_read])?;
            }
        }
    }

    zip.finish()?;

    Ok(())
}

/// Extracts the contents of a ZIP file to a specified directory.
///
/// This function opens a ZIP archive and extracts each entry (file or directory)
/// to a given destination directory. It handles the creation of directories as needed
/// and writes the contents of each file to the filesystem.
///
/// # Arguments
///
/// * `zip_path` - A path to the ZIP file to be extracted. The type `P` must implement `AsRef<Path>`.
/// * `extract_to` - The destination directory where the ZIP contents will be extracted.
///                  Also requires `P: AsRef<Path>`.
///
/// # Returns
///
/// Returns `io::Result<()>`.
/// # Notes
///
/// - The function automatically handles both files and directories within the ZIP archive.
/// - It ensures that all necessary parent directories are created.
/// - Files are written to disk using buffered I/O for efficiency.
pub fn unzip_file<P: AsRef<Path>>(zip_path: P, extract_to: P) -> io::Result<()> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = extract_to.as_ref().join(file.mangled_name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = BufWriter::new(File::create(&outpath)?);

            // Decompression happens implicitly here as data is read from the ZIP archive
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

/// List the names of files in a given directory.
/// This function only lists files, not directories. It iterates over
/// each entry in the specified directory. If an entry is a file (not
/// a directory), its name is extracted and added to the resulting vector. Subdirectories are ignored.
pub fn list_files_in_dir(dir_path: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if dir_path.is_dir() {
        for entry in read_dir(dir_path)? {
            let entry = entry?;
            if entry.path().is_file() {
                files.push(entry.path());
            }
        }
    }

    Ok(files)
}

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
    let file = File::open(path)?;
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
/// Returns a `Result<MerkleTree, CommonError>`. On success, it contains the
/// Merkle tree constructed from the files' contents. On failure, it returns a
/// `CommonError` indicating the type of error encountered, such as an issue
/// reading the files.
///
/// # Errors
///
/// Returns an error if any file cannot be read or converted to bytes.
pub fn generate_merkle_tree<P: AsRef<Path>>(paths: &Vec<P>) -> Result<MerkleTree, CommonError>
where
    P: AsRef<Path> + Send + Sync,
{
    let leaf_bytes_results: Vec<Result<Vec<u8>, CommonError>> = paths
        .par_iter()
        .map(|path| {
            file_to_bytes(path.as_ref())
                .map_err(|err| CommonError::FileToBytesConversionError(err.to_string()))
        })
        .collect();

    let leaf_bytes: Vec<Vec<u8>> = leaf_bytes_results.into_iter().collect::<Result<_, _>>()?;

    Ok(MerkleTree::new(&leaf_bytes))
}

pub fn delete_files_in_directory(dir: &Path) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
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
        let file_names: Vec<&str> = vec!["file1.txt", "file2.txt"];
        for file_name in &file_names {
            let mut file = File::create(temp_dir_path.join(file_name)).unwrap();
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
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content).unwrap();

        // Use file_to_bytes to read the file into bytes
        let file_bytes = file_to_bytes(&file_path).unwrap();
        assert_eq!(file_bytes, content);
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

        leaf_hashes.sort(); // The `merkle-tree` lib sorts the leaf nodes in an ascending order

        // Verify the leaf nodes used in generating the merkle tree
        assert!(leaf_hashes == MerkleTree::build_leaf_nodes(&leaf_bytes));

        // Verify the leaf nodes in the merkle tree
        assert!(merkle_tree.leaf_nodes() == &leaf_hashes);
    }

    #[test]
    fn should_zip_and_unzip() -> io::Result<()> {
        // Create a temporary directory with test files
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        let mut file = File::create(&file_path)?;
        writeln!(file, "Hello, world!")?;

        // Zip the file
        let zip_path = temp_dir.path().join("test.zip");
        zip_files(&[&file_path], &&zip_path)?;

        // Create another temporary directory for extraction
        let extract_dir = tempdir().unwrap();
        unzip_file(&zip_path, &extract_dir.path().to_path_buf())?;

        // Read the original and extracted files
        let mut original_contents = String::new();
        File::open(&file_path)?.read_to_string(&mut original_contents)?;

        let extracted_file_path = extract_dir.path().join("test_file.txt");
        let mut extracted_contents = String::new();
        File::open(&extracted_file_path)?.read_to_string(&mut extracted_contents)?;

        // Compare the contents
        assert_eq!(original_contents, extracted_contents);
        Ok(())
    }
}

pub fn logger_init(default: Option<&str>) {
    Builder::from_env(Env::default().default_filter_or(default.unwrap_or("warn"))).init();
}
