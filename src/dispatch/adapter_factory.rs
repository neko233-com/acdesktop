pub(crate) fn build_adapter() -> impl acdesktop_core::PlatformAdapter {
    #[cfg(target_os = "macos")]
    {
        acdesktop_macos::MacOSAdapter::new()
    }

    #[cfg(target_os = "windows")]
    {
        acdesktop_windows::WindowsAdapter::new()
    }

    #[cfg(target_os = "linux")]
    {
        acdesktop_linux::release_input_on_signals();
        acdesktop_linux::LinuxAdapter::new()
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    compile_error!("Unsupported platform")
}
