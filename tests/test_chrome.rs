use tempfile::tempdir;
use web_assistant::browser::chrome::Chrome;

#[test]
fn test_find_executable() {
    let result = Chrome::find_executable();
    assert!(
        result.is_ok(),
        "Expected to find Chrome or Chromium executable"
    );
    let path = result.unwrap();
    assert!(path.exists());
}

#[test]
fn test_user_data_dir_resolution() {
    let temp = tempdir().unwrap();
    let explicit_dir = temp.path().join("custom_profile");

    let resolved = Chrome::resolve_user_data_dir(None, Some(&explicit_dir)).unwrap();
    assert_eq!(resolved, explicit_dir);

    // Resolve without explicit directory
    let default_resolved = Chrome::resolve_user_data_dir(None, None);
    assert!(default_resolved.is_ok());
}

#[test]
fn test_default_args() {
    let temp = tempdir().unwrap();
    let args = Chrome::default_args(9222, temp.path(), true);

    assert!(
        args.iter()
            .any(|a| a.contains("--remote-debugging-port=9222"))
    );
    assert!(args.iter().any(|a| a.contains("--headless=new")));
    assert!(args.iter().any(|a| a.contains("--no-sandbox")));
    assert!(args.iter().any(|a| a.contains("--user-data-dir=")));
}

#[test]
fn test_list_profiles_empty_and_created() {
    let temp = tempdir().unwrap();
    let path = temp.path();

    let initial = Chrome::list_profiles(path);
    assert!(initial.is_empty());

    std::fs::create_dir(path.join("Default")).unwrap();
    std::fs::create_dir(path.join("Profile 1")).unwrap();

    let profiles = Chrome::list_profiles(path);
    assert_eq!(profiles.len(), 2);
    assert!(profiles.iter().any(|p| p.directory == "Default"));
    assert!(profiles.iter().any(|p| p.directory == "Profile 1"));
}
