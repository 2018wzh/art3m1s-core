use super::*;

#[cfg(all(
    feature = "vulkan-backend",
    any(target_os = "windows", target_os = "linux", target_os = "android")
))]
#[test]
#[ignore = "requires a hardware native GPU adapter"]
fn timeline_notifications_follow_complete_load_and_boot_success() {
    struct Root(std::path::PathBuf);
    impl Drop for Root {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        Root(std::env::temp_dir().join(format!("art3m1s-clock-{}-{unique}", std::process::id())));
    std::fs::create_dir_all(&root.0).unwrap();
    std::fs::write(root.0.join("boot.iet"), b"[stop]\n").unwrap();
    let resources = crate::host_files::HostResources::new();
    resources.mount_directory(&root.0).unwrap();
    resources.set_save_dir(Some(&root.0.join("saves"))).unwrap();
    let mut runtime = CoreRuntime::create(8, 8, BackendSelection::NativeVulkanHardware).unwrap();
    runtime.set_resources(resources.clone());
    runtime
        .load_project_bytes(
            b"[WINDOWS]\nWIDTH=8\nHEIGHT=8\nBOOT=boot.iet\nSAVEPATH=save\nCHARSET=UTF-8\n",
            "WINDOWS",
        )
        .unwrap();
    assert!(!runtime.take_timeline_reset());
    let saved = crate::save::SaveData::from_interpreter(&runtime.interpreter);
    resources
        .write("save/slot.dat", &serde_json::to_vec(&saved).unwrap())
        .unwrap();
    resources.write("save/bad.dat", b"bad json").unwrap();
    assert!(runtime.handle_load_game("missing.dat", None).is_err());
    assert!(runtime.handle_load_game("bad.dat", None).is_err());
    assert!(!runtime.take_timeline_reset());
    runtime.handle_load_game("slot.dat", None).unwrap();
    runtime.handle_load_game("slot.dat", None).unwrap();
    assert!(runtime.take_timeline_reset());
    assert!(!runtime.take_timeline_reset());
    runtime.handle_load_game("slot.dat", None).unwrap();
    assert!(runtime.handle_load_game("missing.dat", None).is_err());
    assert!(runtime.take_timeline_reset());
    assert!(!runtime.take_timeline_reset());
    runtime.handle_engine_reset().unwrap();
    runtime.boot_script = Some("missing.iet".into());
    assert!(runtime.handle_engine_reset().is_err());
    assert!(runtime.take_timeline_reset());
    assert!(!runtime.take_timeline_reset());
    assert!(runtime.handle_engine_reset().is_err());
    assert!(!runtime.take_timeline_reset());
}
