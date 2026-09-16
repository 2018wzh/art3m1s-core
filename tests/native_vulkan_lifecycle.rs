#![cfg(all(feature = "vulkan-backend", target_os = "windows"))]

use art3m1s_core::backend::{BackendKind, BackendSelection};
use art3m1s_core::runtime::CoreRuntime;

#[test]
#[ignore = "requires a hardware Vulkan adapter"]
fn native_vulkan_sessions_create_read_back_and_reopen() {
    for _ in 0..3 {
        let mut runtime = CoreRuntime::create(64, 48, BackendSelection::NativeVulkan).unwrap();
        assert_eq!(runtime.backend_info().kind, BackendKind::Vulkan);
        assert!(runtime.backend_info().capabilities.readback);
        let pixels = runtime.advance_and_render(16);
        assert_eq!(pixels.len(), 64 * 48 * 4);
        drop(runtime);
    }
}
