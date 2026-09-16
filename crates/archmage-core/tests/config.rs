use archmage_core::config::Config;
use std::fs;

#[test]
fn default_is_platform_application_directory() {
    let config = Config::resolve(None, None).unwrap();
    assert!(config.data_root.is_absolute());
    assert!(
        config
            .data_root
            .to_string_lossy()
            .to_lowercase()
            .contains("archmage")
    );
}
#[test]
fn explicit_root_is_resolved_without_creating_it() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let c = Config::resolve(Some(&root), None).unwrap();
    assert!(!root.exists());
    c.prepare().unwrap();
    assert!(c.data_root.is_dir());
}
#[cfg(unix)]
#[test]
fn new_data_directory_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let c = Config::resolve(Some(&temp.path().join("private")), None).unwrap();
    c.prepare().unwrap();
    assert_eq!(
        fs::metadata(c.data_root).unwrap().permissions().mode() & 0o777,
        0o700
    );
}
#[cfg(unix)]
#[test]
fn symlink_into_source_is_rejected_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir(&repo).unwrap();
    let alias = temp.path().join("alias");
    std::os::unix::fs::symlink(&repo, &alias).unwrap();
    assert!(Config::resolve(Some(&alias.join("data")), Some(&repo)).is_err());
    assert!(!repo.join("data").exists());
}
#[test]
fn file_as_root_and_filesystem_root_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("file");
    fs::write(&file, "x").unwrap();
    assert!(Config::resolve(Some(&file), None).is_err());
    assert!(Config::resolve(Some(std::path::Path::new("/")), None).is_err());
}
#[cfg(unix)]
#[test]
fn nonwritable_parent_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o500)).unwrap();
    let result = Config::resolve(Some(&temp.path().join("data")), None).and_then(|c| c.prepare());
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
}
