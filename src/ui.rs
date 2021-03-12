use crate::{
    config::Config,
    steamworks_util::{
        OneShotRecvError,
        UgcQueryBuilder,
    },
};
use conrod_core::{
    widget,
    widget_ids,
    Borderable,
    Colorable,
    Labelable,
    Positionable,
    Sizeable,
    Widget,
};
use parking_lot::Mutex;
use std::{
    borrow::Cow,
    error::Error as StdError,
    path::PathBuf,
    sync::Arc,
};
use tokio::runtime::Runtime as TokioRuntime;

widget_ids! {
    pub struct Ids {
        title,

        game_button,
        levelbuilder_button,

        cover_image,

        syncing_label,
    }
}

pub fn gui(ui: &mut conrod_core::UiCell, ids: &Ids, app: &mut App) {
    let button_width = 200.0;
    let button_height = 50.0;
    let cover_image_side = 200.0;

    widget::Text::new("Skeleton Sprint Launcher")
        .color(conrod_core::color::WHITE)
        .font_size(42)
        .mid_top_of(ui.window)
        .set(ids.title, ui);

    widget::Image::new(app.cover_image)
        .w_h(cover_image_side, cover_image_side)
        .down_from(ids.title, 10.0)
        // .up_from(ids.game_button, 0.0)
        .align_middle_x_of(ui.window)
        .set(ids.cover_image, ui);

    for () in widget::Button::new()
        .label("Launch Game")
        .middle_of(ui.window)
        .w_h(button_width, button_height)
        .set(ids.game_button, ui)
    {
        if !app.steam_workshop_sync_state.lock().is_syncing() {
            let game_path = app.config.get_game_path();
            crate::util::open_program(&*game_path.to_string_lossy());
        }
    }

    for () in widget::Button::new()
        .label("Launch Levelbuilder")
        .down_from(ids.game_button, 10.0)
        .w_h(button_width, button_height)
        .set(ids.levelbuilder_button, ui)
    {
        let levelbuilder_path = app.config.get_levelbuilder_path().clone();
        crate::util::open_program(&*levelbuilder_path.to_string_lossy());
    }

    {
        let steam_workshop_sync_state = app.steam_workshop_sync_state.lock();
        let sync_label: Cow<'_, str> = match &*steam_workshop_sync_state {
            SteamWorkshopSyncState::Starting => "Syncing...".into(),
            SteamWorkshopSyncState::InProgress(current, total) => {
                format!("Syncing({}/{})...", current, total).into()
            }
            SteamWorkshopSyncState::Done => "Sync Complete".into(),
            SteamWorkshopSyncState::Failed(_) => "Sync Failed!".into(),
        };

        widget::TitleBar::new(&sync_label, ui.window)
            .color(conrod_core::Color::Rgba(1.0, 1.0, 1.0, 1.0))
            .bottom_left_with_margin_on(ui.window, 0.0)
            .w_h(200.0, 30.0)
            .border(0.0)
            .set(ids.syncing_label, ui);
    }
}

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Steam(steamworks::SteamError),
    SteamWorkshopQuery(crate::steamworks_util::WorkshopQueryError),

    InvalidSyncDir,
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> AppError {
        AppError::Io(e)
    }
}

impl From<steamworks::SteamError> for AppError {
    fn from(e: steamworks::SteamError) -> AppError {
        AppError::Steam(e)
    }
}

impl From<crate::steamworks_util::WorkshopQueryError> for AppError {
    fn from(e: crate::steamworks_util::WorkshopQueryError) -> AppError {
        AppError::SteamWorkshopQuery(e)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Steam(e) => e.fmt(f),
            Self::SteamWorkshopQuery(e) => e.fmt(f),
            Self::InvalidSyncDir => write!(f, "The sync dir is invalid"),
        }
    }
}

#[derive(Debug)]
pub enum SteamWorkshopSyncError {
    Recieve(OneShotRecvError),
    Steam(steamworks::SteamError),
    Io(std::io::Error),

    MissingItemInfo,
}

impl From<OneShotRecvError> for SteamWorkshopSyncError {
    fn from(e: OneShotRecvError) -> Self {
        Self::Recieve(e)
    }
}

impl From<steamworks::SteamError> for SteamWorkshopSyncError {
    fn from(e: steamworks::SteamError) -> Self {
        Self::Steam(e)
    }
}

impl From<std::io::Error> for SteamWorkshopSyncError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for SteamWorkshopSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Recieve(e) => e.fmt(f),
            Self::Steam(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),

            Self::MissingItemInfo => write!(f, "Missing workshop item info"),
        }
    }
}

impl StdError for SteamWorkshopSyncError {}

#[derive(Debug)]
pub enum SteamWorkshopSyncState {
    Starting,

    InProgress(usize, usize),

    Done,
    Failed(SteamWorkshopSyncError),
}

impl SteamWorkshopSyncState {
    pub fn begin_sync(&mut self, len: usize) {
        *self = Self::InProgress(0, len);
    }

    pub fn add_synced(&mut self, synced: usize) {
        if let Self::InProgress(old_synced, total) = self {
            *old_synced += synced;
            if *old_synced >= *total {
                *self = Self::Done;
            }
        }
    }

    pub fn set_fail(&mut self, e: SteamWorkshopSyncError) {
        *self = Self::Failed(e);
    }

    pub fn is_syncing(&self) -> bool {
        matches!(self, Self::Starting | Self::InProgress(_, _))
    }
}

pub struct App {
    pub config: Config,

    cover_image: conrod_core::image::Id,

    pub tokio_rt: TokioRuntime,
    pub steam_client: steamworks::Client,
    steam_single_client: steamworks::SingleClient,
    steam_workshop_sync_state: Arc<Mutex<SteamWorkshopSyncState>>,
}

impl App {
    pub fn new(config: Config, cover_image: conrod_core::image::Id) -> Result<Self, AppError> {
        let tokio_rt = TokioRuntime::new()?;

        // For now, lets make steamworks necessary.
        let (steam_client, steam_single_client) = steamworks::Client::init()?;

        let ugc_query_future = UgcQueryBuilder::new(&steam_client)
            .user_list(steamworks::UserList::Subscribed)
            .send(|res| res.map(|res| res.iter().collect::<Vec<_>>()))?;

        let sync_dir = config.get_workshop_sync_path().clone();
        if !sync_dir.exists() {
            std::fs::create_dir_all(&sync_dir)?;
        }

        if !sync_dir.is_dir() {
            return Err(AppError::InvalidSyncDir);
        }

        let steam_workshop_sync_state = Arc::new(Mutex::new(SteamWorkshopSyncState::Starting));
        let steam_workshop_sync_state_clone = steam_workshop_sync_state.clone();

        let steam_client_clone = steam_client.clone();

        tokio_rt.spawn(async move {
            if let Err(e) = sync_steam_workshop(
                steam_client_clone,
                ugc_query_future.await,
                steam_workshop_sync_state_clone.clone(),
                sync_dir,
            )
            .await
            {
                eprintln!("Sync Failed: {}", e);
                steam_workshop_sync_state_clone.lock().set_fail(e);
            }
        });

        Ok(App {
            config,

            cover_image,

            tokio_rt,
            steam_client,
            steam_single_client,
            steam_workshop_sync_state,
        })
    }

    pub fn update(&mut self) {
        self.steam_single_client.run_callbacks();
    }
}

async fn sync_steam_workshop(
    steam_client: steamworks::Client,
    workshop_data: Result<
        Result<Vec<steamworks::QueryResult>, steamworks::SteamError>,
        OneShotRecvError,
    >,
    steam_workshop_sync_state: Arc<Mutex<SteamWorkshopSyncState>>,
    mut sync_dir: PathBuf,
) -> Result<(), SteamWorkshopSyncError> {
    let workshop_data = workshop_data??;

    steam_workshop_sync_state
        .lock()
        .begin_sync(workshop_data.len());

    for workshop_item in workshop_data.iter() {
        let item_info = steam_client
            .ugc()
            .item_install_info(workshop_item.published_file_id)
            .ok_or(SteamWorkshopSyncError::MissingItemInfo)?;

        sync_dir.push(&workshop_item.title);
        sync_dir.set_extension("txt");
        tokio::fs::copy(&item_info.folder, &sync_dir).await?;
        sync_dir.pop();

        steam_workshop_sync_state.lock().add_synced(1);
    }

    Ok(())
}
