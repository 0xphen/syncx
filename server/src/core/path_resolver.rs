const GCS_BACKUP_DIR: &str = "backup";
const ZIP_DIR: &str = "zips";
const TEMP_DIR: &str = "temp";
const WIP_UPLOADS_DIR: &str = "wip_uploads";
const WIP_DOWNLOADS_DIR: &str = "wip_downloads";

///Returns the object name for a file (`file_name`) in google cloud storage
/// `backup` folder.
pub fn gcs_backup_object_name(id: &str, file_name: &str) -> String {
    format!("{}/{}/{}", GCS_BACKUP_DIR, id, file_name)
}

///Returns the object name for a zip file in google cloud storage
/// `zips` folder
pub fn gcs_zip_file_object_name(id: &str) -> String {
    format!("{}/{}.zip", ZIP_DIR, id)
}

pub fn local_merkle_tree_path() -> String {
    format!("{}/merkle_trees", TEMP_DIR)
}

pub fn local_merkle_tree_file(id: &str) -> String {
    format!("{}_mtree.txt", id)
}

pub fn local_zip_dir() -> String {
    format!("{}/{}", TEMP_DIR, ZIP_DIR)
}

pub fn wip_uploads_dir(id: &str) -> String {
    format!("{}/{}/{}", TEMP_DIR, WIP_UPLOADS_DIR, id)
}

pub fn wip_downloads_dir(id: &str) -> String {
    format!("{}/{}/{}", TEMP_DIR, WIP_DOWNLOADS_DIR, id)
}
