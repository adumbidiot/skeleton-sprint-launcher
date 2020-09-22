//#![windows_subsystem = "windows"] //TODO: Consider feature specific for package, as release builds are helpful sometimes if perf on debug is crap

mod button;
mod config;
mod steamworks_extra;

use crate::{
    button::{
        Button,
        ButtonBuilder,
    },
    config::Config,
};
use ggez::{
    event::{
        self,
        EventHandler,
        MouseButton,
    },
    graphics,
    graphics::{
        Image,
        Text,
    },
    timer,
    Context,
    ContextBuilder,
    GameResult,
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
    rc::Rc,
};
use steamworks::{
    AppIDs,
    AppId,
    UGCType,
    UserList,
    UserListOrder,
};

const DESIRED_FPS: u32 = 60;
const APP_ID: u32 = 690950;

enum Action {
    OpenApp(PathBuf),
}

struct ActionQueue {
    inner: RefCell<VecDeque<Action>>,
}

impl ActionQueue {
    pub fn new() -> Self {
        ActionQueue {
            inner: RefCell::new(VecDeque::new()),
        }
    }

    pub fn push(&self, action: Action) {
        self.inner.borrow_mut().push_back(action);
    }

    pub fn get_next(&self) -> Option<Action> {
        self.inner.borrow_mut().pop_front()
    }
}

pub struct LauncherState {
    config: Config,
    action_queue: ActionQueue,
}

impl LauncherState {
    pub fn new(config: Config) -> Self {
        LauncherState {
            action_queue: ActionQueue::new(),
            config,
        }
    }
}

struct SteamApi {
    single_client: steamworks::SingleClient,
    client: steamworks::Client,
}

impl SteamApi {
    fn init() -> Option<Self> {
        let (client, single_client) = steamworks::Client::init().ok()?;

        Some(SteamApi {
            client,
            single_client,
        })
    }

    fn init_workshop(&self, sync_dir: &Path) -> Rc<RefCell<WorkshopSyncState>> {
        std::fs::create_dir_all(sync_dir).expect("Dir");

        let steam_id = self.client.user().steam_id();
        let query = self
            .client
            .ugc()
            .query_user(
                steam_id.account_id(),
                UserList::Subscribed,
                UGCType::All,
                UserListOrder::LastUpdatedDesc,
                AppIDs::ConsumerAppId(AppId(APP_ID)),
                1,
            )
            .expect("Valid query");

        let mut sync_dir = PathBuf::from(sync_dir);
        query.fetch(move |res| {
            res.expect("Response").iter().for_each(|el| {
                if let Some(info) = steamworks_extra::get_item_install_info(el.published_file_id) {
                    sync_dir.push(&el.title);
                    sync_dir.set_extension("txt");
                    std::fs::copy(&info.path, &sync_dir).expect("Valid copy");
                    sync_dir.pop();
                } else {
                    // Failed to sync blah blah blah
                }
            });
        });

        Rc::new(RefCell::new(WorkshopSyncState::new()))
    }

    fn update(&self) {
        self.single_client.run_callbacks();
    }
}

#[derive(Debug)]
struct WorkshopSyncState {
    total: Option<usize>,
    finished: Option<usize>,
}

impl WorkshopSyncState {
    pub fn new() -> WorkshopSyncState {
        WorkshopSyncState {
            total: None,
            finished: None,
        }
    }

    #[allow(dead_code)]
    fn is_finished(&self) -> bool {
        self.total == self.finished
    }
}

fn main() {
    let config = match config::load_from_file("./config.toml") {
        Some(c) => c,
        None => {
            println!("Could not load config from ./config.toml");
            return;
        }
    };
    println!("Loaded Config: {:#?}", config);

    let steam_api = SteamApi::init();

    let (mut ctx, mut event_loop) = ContextBuilder::new("skeleton-sprint-launcher", "adumbidiot")
        .window_setup(
            ggez::conf::WindowSetup::default()
                .title("Skeleton Sprint Launcher")
                .icon("/icon.ico")
                .vsync(true),
        )
        .window_mode(ggez::conf::WindowMode::default().maximized(false))
        .add_resource_path("./resources")
        .build()
        .expect("Context");

    let mut state = State::new(&mut ctx, steam_api, config);

    match event::run(&mut ctx, &mut event_loop, &mut state) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e),
    }
}

struct State {
    steam_api: Option<SteamApi>,

    bg: Image,
    title: Text,
    #[allow(dead_code)]
    workshop_state: Option<Rc<RefCell<WorkshopSyncState>>>,
    icon: Image,

    buttons: Vec<Button>,
    launcher_state: LauncherState,
}

impl State {
    pub fn new(ctx: &mut Context, steam_api: Option<SteamApi>, config: Config) -> Self {
        let workshop_state = steam_api
            .as_ref()
            .map(|steam_api| steam_api.init_workshop(config.get_workshop_sync_path()));

        let bg = Image::new(ctx, "/bg.png").unwrap();
        let icon = Image::new(ctx, "/cover_art.png").unwrap();

        let game_button = ButtonBuilder::new()
            .text("Launch Game")
            .dimensions([100.0, 100.0, 150.0, 50.0])
            .key_up_handler(|state| {
                state.action_queue.push(Action::OpenApp(
                    state.config.get_game_config().get_path().clone(),
                ));
            })
            .build(ctx)
            .expect("button");

        let builder_button = ButtonBuilder::new()
            .text("Launch Builder")
            .dimensions([100.0, 200.0, 150.0, 50.0])
            .key_up_handler(|state| {
                state.action_queue.push(Action::OpenApp(
                    state.config.get_levelbuilder_config().get_path().clone(),
                ));
            })
            .build(ctx)
            .expect("button");

        let buttons = vec![game_button, builder_button];

        State {
            steam_api,
            bg,
            icon,
            title: Text::new("Skeleton Sprint Launcher"),
            workshop_state,
            buttons,
            launcher_state: LauncherState::new(config),
        }
    }
}

impl EventHandler for State {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        if let Some(steam_api) = self.steam_api.as_ref() {
            steam_api.update();
            for b in self.buttons.iter_mut() {
                b.update(&self.launcher_state);
            }

            while let Some(a) = self.launcher_state.action_queue.get_next() {
                match a {
                    Action::OpenApp(p) => {
                        println!("Opening App '{}'...", p.display());
                        let _child = Command::new(p).spawn().expect("Launch");
                        ctx.continuing = false;
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        while timer::check_update_time(ctx, DESIRED_FPS) {
            graphics::clear(ctx, graphics::WHITE);
            graphics::draw(ctx, &self.bg, graphics::DrawParam::default())?;
            graphics::draw(
                ctx,
                &self.icon,
                graphics::DrawParam::default()
                    .dest([300.0, 100.0])
                    .scale([0.5, 0.5]),
            )?;
            graphics::draw(ctx, &self.title, graphics::DrawParam::default())?;

            for b in self.buttons.iter() {
                b.draw(ctx)?;
            }

            graphics::present(ctx)?;
        }
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            for b in self.buttons.iter_mut() {
                if b.dimensions().contains([x, y]) {
                    b.on_keydown(&self.launcher_state);
                }
            }
        }
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            for b in self.buttons.iter_mut() {
                if b.dimensions().contains([x, y]) {
                    b.on_keyup(&self.launcher_state);
                }
            }
        }
    }
}
