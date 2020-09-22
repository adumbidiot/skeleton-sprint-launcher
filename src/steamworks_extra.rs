use std::{
    path::PathBuf,
    time::Duration,
};
use steamworks::PublishedFileId;

extern "C" {
    pub fn SteamAPI_ISteamUGC_GetItemInstallInfo(
        id: u64,
        size: *mut u64,
        pchFolder: *mut u8,
        cchFolderSize: u32,
        punTimeStamp: *mut u32,
    ) -> bool;
}

#[derive(Debug)]
pub struct ItemInstallInfo {
    pub path: PathBuf,
    pub size: u64,
    pub timestamp: Duration,
}

/// # Returns
/// Some(ItemInstallInfo) if the workshop item is already installed.
/// None if:
/// * cchFolderSize is 0.
/// * The workshop item has no content.
/// * The workshop item is not installed.
pub fn get_item_install_info(published_file_id: PublishedFileId) -> Option<ItemInstallInfo> {
    let max_path_size = 1024; // IDK
    let mut buffer = vec![0; max_path_size];
    let mut size: u64 = 0;
    let mut timestamp: u32 = 0;

    if !unsafe {
        SteamAPI_ISteamUGC_GetItemInstallInfo(
            published_file_id.0,
            &mut size as *mut u64,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            &mut timestamp as *mut u32,
        )
    } {
        return None;
    }

    let end = buffer
        .iter()
        .position(|&el| el == 0)
        .unwrap_or(buffer.len());

    buffer.resize(end, 0);

    let info = ItemInstallInfo {
        path: PathBuf::from(String::from_utf8(buffer).ok()?),
        size,
        timestamp: Duration::from_millis(u64::from(timestamp)),
    };

    Some(info)
}
