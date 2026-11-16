use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::os::windows::ffi::OsStringExt;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};

use acdesktop_core::AdapterError;
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_READ, GetFileInformationByHandle,
};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};

use super::image_identity;

pub(super) struct HelperImage {
    pub(super) path: PathBuf,
    _file: File,
}

impl HelperImage {
    pub(super) fn locate() -> Result<Self, AdapterError> {
        let image = module_image()?;
        let directory = image
            .parent()
            .ok_or_else(|| AdapterError::internal("The hosted library has no directory"))?;
        let helper = Self::open(&directory.join("acdesktop.exe"))?;
        let host_path =
            std::env::current_exe().map_err(|error| AdapterError::internal(error.to_string()))?;
        let host =
            File::open(host_path).map_err(|error| AdapterError::internal(error.to_string()))?;
        ensure_distinct(&helper._file, &host)?;
        Ok(helper)
    }

    fn open(path: &Path) -> Result<Self, AdapterError> {
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .map_err(|error| {
                AdapterError::internal("The hosted control indicator helper is unavailable")
                    .with_platform_detail(error.to_string())
            })?;
        let metadata = file
            .metadata()
            .map_err(|error| AdapterError::internal(error.to_string()))?;
        if !metadata.is_file()
            || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || !image_identity::is_acdesktop_image(path)
        {
            return Err(AdapterError::internal(
                "The hosted control indicator helper is not a regular acdesktop executable",
            ));
        }
        Ok(Self {
            path: path.to_path_buf(),
            _file: file,
        })
    }
}

pub(super) fn is_cli_image() -> Result<bool, AdapterError> {
    let host =
        std::env::current_exe().map_err(|error| AdapterError::internal(error.to_string()))?;
    Ok(image_identity::is_acdesktop_image(&host) && module_image()? == host)
}

fn file_identity(file: &File) -> Result<(u32, u32, u32), AdapterError> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) };
    if ok == 0 {
        return Err(AdapterError::internal(
            "The hosted indicator executable identity could not be established",
        ));
    }
    Ok((
        info.dwVolumeSerialNumber,
        info.nFileIndexHigh,
        info.nFileIndexLow,
    ))
}

fn ensure_distinct(helper: &File, host: &File) -> Result<(), AdapterError> {
    if file_identity(helper)? == file_identity(host)? {
        return Err(AdapterError::internal(
            "The hosted control indicator helper is the host process image",
        ));
    }
    Ok(())
}

fn module_image() -> Result<PathBuf, AdapterError> {
    let mut module = std::ptr::null_mut();
    let flags =
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    let ok =
        unsafe { GetModuleHandleExW(flags, module_image as *const () as *const u16, &mut module) };
    if ok == 0 {
        return Err(AdapterError::internal(
            "The hosted library image could not be resolved",
        ));
    }
    let mut path = vec![0u16; 32_768];
    let len = unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) } as usize;
    if len == 0 || len >= path.len() {
        return Err(AdapterError::internal(
            "The hosted library image path could not be read",
        ));
    }
    Ok(PathBuf::from(OsString::from_wide(&path[..len])))
}

#[cfg(test)]
#[path = "helper_image_tests.rs"]
mod tests;
