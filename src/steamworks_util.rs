use std::{
    convert::TryInto,
    error::Error as StdError,
    future::Future,
    path::PathBuf,
    time::Duration,
};
use steamworks::{
    AppIDs,
    PublishedFileId,
    SteamError,
    UGCType,
    UserList,
    UserListOrder,
};
pub use tokio::sync::oneshot::error::RecvError as OneShotRecvError;

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
    // Linux has a max path length of 4096, which is the largest out of "the big 3", so we use that. Add 1 for NUL.
    let max_path_size = 4096 + 1;
    let mut buffer = vec![0; max_path_size];
    let buffer_len: u32 = buffer.len().try_into().expect("Max Path can fit in a u32");
    let mut size: u64 = 0;
    let mut timestamp: u32 = 0;

    if !unsafe {
        SteamAPI_ISteamUGC_GetItemInstallInfo(
            published_file_id.0,
            &mut size,
            buffer.as_mut_ptr(),
            buffer_len,
            &mut timestamp,
        )
    } {
        return None;
    }

    let end = buffer
        .iter()
        .position(|&el| el == 0)
        .unwrap_or(buffer.len());

    buffer.truncate(end);

    // TODO: Support non-utf8 paths somehow?
    let info = ItemInstallInfo {
        path: PathBuf::from(String::from_utf8_lossy(&buffer).into_owned()),
        size,
        timestamp: Duration::from_millis(u64::from(timestamp)),
    };

    Some(info)
}

#[derive(Debug)]
pub enum WorkshopQueryError {
    CreateQueryError(steamworks::CreateQueryError),
}

impl From<steamworks::CreateQueryError> for WorkshopQueryError {
    fn from(e: steamworks::CreateQueryError) -> Self {
        Self::CreateQueryError(e)
    }
}

impl std::fmt::Display for WorkshopQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateQueryError(e) => e.fmt(f),
        }
    }
}

impl StdError for WorkshopQueryError {}

pub struct UGCQueryBuilder<'a> {
    client: &'a steamworks::Client,
    account_id: steamworks::AccountId,
    user_list: UserList,
    ugc_type: UGCType,
    user_list_order: UserListOrder,
    app_ids: AppIDs,
    page_number: u32,
}

impl<'a> UGCQueryBuilder<'a> {
    /// Creates a new UGCQueryBuilder.
    pub fn new(client: &'a steamworks::Client) -> Self {
        Self {
            client,
            account_id: client.user().steam_id().account_id(),
            user_list: UserList::Published,
            ugc_type: UGCType::All,
            user_list_order: UserListOrder::LastUpdatedDesc,
            app_ids: AppIDs::ConsumerAppId(client.utils().app_id()),
            page_number: 1,
        }
    }

    /// Set account_id. Defaults to the current steam user.
    pub fn account_id(mut self, account_id: steamworks::AccountId) -> Self {
        self.account_id = account_id;
        self
    }

    /// Sets user listing type. Defaults to Published.
    pub fn user_list(mut self, user_list: UserList) -> Self {
        self.user_list = user_list;
        self
    }

    /// Sets UGCType. Defaults to all.
    pub fn ugc_type(mut self, ugc_type: UGCType) -> Self {
        self.ugc_type = ugc_type;
        self
    }

    /// Sets User List Order. Defaults to Last Updated Descending.
    pub fn user_list_order(mut self, user_list_order: UserListOrder) -> Self {
        self.user_list_order = user_list_order;
        self
    }

    /// Sets the AppIds. Defaults to the app_id of the current process as a consumer.
    pub fn app_ids(mut self, app_ids: AppIDs) -> Self {
        self.app_ids = app_ids;
        self
    }

    /// Sets the page_number. It starts ast 1 and defaults to 1.
    pub fn page_number(mut self, page_number: u32) -> Self {
        self.page_number = page_number;
        self
    }

    /// Sends a UGC Query
    pub fn send<
        O: Send + 'static,
        M: FnOnce(Result<steamworks::QueryResults<'_>, SteamError>) -> O + Send + 'static,
    >(
        self,
        mutate: M,
    ) -> Result<impl Future<Output = Result<O, OneShotRecvError>>, WorkshopQueryError> {
        let query = self.client.ugc().query_user(
            self.account_id,
            self.user_list,
            self.ugc_type,
            self.user_list_order,
            self.app_ids,
            self.page_number,
        )?;

        let (tx, rx) = tokio::sync::oneshot::channel();

        query.fetch(move |res| {
            // Don't really care if reciever is dropped...
            let _ = tx.send(mutate(res)).is_ok();
        });

        Ok(async { rx.await })
    }
}
