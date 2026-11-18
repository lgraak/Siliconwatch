//! Linux StatusNotifierItem and DBusMenu transport, independent of acquisition.
//!
//! One owned worker drives zbus's executor (its detached internal executor is disabled).
//! Calls have deadlines, cancellation drops pending futures, and shutdown joins the worker.
//! Capability is rechecked every two seconds with a 1500 ms aggregate deadline; signals
//! invalidate it earlier. The UI must restore a hidden window when availability is lost.
use futures_lite::{StreamExt, future};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str, Value};
use zbus::{Connection, MessageStream, Proxy, fdo};

const ITEM_PATH: &str = "/StatusNotifierItem";
const MENU_PATH: &str = "/MenuBar";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";
const WATCHERS: [&str; 2] = [
    "org.kde.StatusNotifierWatcher",
    "org.freedesktop.StatusNotifierWatcher",
];
const CHECK_INTERVAL: Duration = Duration::from_secs(2);
const CHECK_DEADLINE: Duration = Duration::from_millis(1500);
const CAPABILITY_LEASE: Duration = Duration::from_secs(5);
const MAX_BATCH: usize = 32;
type Pixmaps = Vec<(i32, i32, Vec<u8>)>;
type Properties = HashMap<String, OwnedValue>;
type Layout = (i32, Properties, Vec<OwnedValue>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrayAction {
    OpenSensors,
    Settings,
    Reconnect,
    Quit,
}
impl TrayAction {
    fn bit(self) -> u8 {
        1 << self as u8
    }
    fn for_id(id: i32) -> Option<Self> {
        match id {
            1 => Some(Self::OpenSensors),
            2 => Some(Self::Settings),
            3 => Some(Self::Reconnect),
            4 => Some(Self::Quit),
            _ => None,
        }
    }
}
#[derive(Debug)]
pub struct TrayEvents {
    pub available: bool,
    pub availability_changed: bool,
    pub actions: Vec<TrayAction>,
    pub diagnostic: Option<String>,
}

/// Rasterized approved branding. The source asset is never modified.
#[derive(Clone)]
pub struct TrayIcon(Pixmaps);
impl TrayIcon {
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Self, String> {
        if width == 0
            || height == 0
            || width > 2048
            || height > 2048
            || rgba.len() != width as usize * height as usize * 4
        {
            return Err("Tray logo must be bounded, complete RGBA pixels".into());
        }
        let mut images = Vec::new();
        // Box sampling keeps the same approved image, with normal display-size scaling.
        for size in [32_u32, 64, 128] {
            let mut argb = Vec::with_capacity((size * size * 4) as usize);
            for y in 0..size {
                for x in 0..size {
                    let x0 = x * width / size;
                    let x1 = ((x + 1) * width / size).max(x0 + 1).min(width);
                    let y0 = y * height / size;
                    let y1 = ((y + 1) * height / size).max(y0 + 1).min(height);
                    let mut total = [0_u64; 4];
                    let count = u64::from((x1 - x0) * (y1 - y0));
                    for sy in y0..y1 {
                        for sx in x0..x1 {
                            let p = ((sy * width + sx) * 4) as usize;
                            let a = u64::from(rgba[p + 3]);
                            total[3] += a;
                            for c in 0..3 {
                                total[c] += u64::from(rgba[p + c]) * a;
                            }
                        }
                    }
                    let alpha = (total[3] / count) as u8;
                    let color = |c: usize| total[c].checked_div(total[3]).unwrap_or(0) as u8;
                    // SNI ARGB32 is network byte order, independent of host endianness.
                    argb.extend_from_slice(&[alpha, color(0), color(1), color(2)]);
                }
            }
            images.push((size as i32, size as i32, argb));
        }
        Ok(Self(images))
    }
}

struct Mailbox {
    available: bool,
    changed: bool,
    verified: Option<Instant>,
    diagnostic: Option<String>,
    actions: u8,
    notified: bool,
    tooltip: String,
    status_revision: u64,
}
struct Shared {
    mailbox: Mutex<Mailbox>,
    wake_ui: Box<dyn Fn() + Send + Sync>,
    icon: TrayIcon,
}
impl Shared {
    fn lock(&self) -> std::sync::MutexGuard<'_, Mailbox> {
        self.mailbox
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn notify(&self) {
        let should_wake = {
            let mut s = self.lock();
            if s.notified {
                false
            } else {
                s.notified = true;
                true
            }
        };
        if should_wake {
            (self.wake_ui)();
        }
    }
    fn availability(&self, available: bool, diagnostic: Option<&str>) {
        let changed = {
            let mut s = self.lock();
            let diagnostic = diagnostic.map(|v| bounded(v, 240));
            let changed = s.available != available || s.diagnostic != diagnostic;
            if s.available != available {
                s.changed = true;
            }
            s.available = available;
            s.verified = available.then(Instant::now);
            s.diagnostic = diagnostic;
            changed
        };
        if changed {
            self.notify();
        }
    }
    fn action(&self, action: TrayAction) {
        self.lock().actions |= action.bit();
        self.notify();
    }
    fn take(&self) -> TrayEvents {
        let mut s = self.lock();
        if s.available && s.verified.is_none_or(|t| t.elapsed() > CAPABILITY_LEASE) {
            s.available = false;
            s.changed = true;
            s.diagnostic = Some("Tray host verification expired".into());
        }
        let mut actions = Vec::with_capacity(4);
        for action in [
            TrayAction::Quit,
            TrayAction::OpenSensors,
            TrayAction::Settings,
            TrayAction::Reconnect,
        ] {
            if s.actions & action.bit() != 0 {
                actions.push(action);
            }
        }
        s.actions = 0;
        s.notified = false;
        let changed = std::mem::take(&mut s.changed);
        TrayEvents {
            available: s.available,
            availability_changed: changed,
            actions,
            diagnostic: s.diagnostic.clone(),
        }
    }
}
fn bounded(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
fn markup_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub struct TrayHandle {
    shared: Arc<Shared>,
    refresh: async_channel::Sender<()>,
    stop: async_channel::Sender<()>,
    worker: Option<JoinHandle<()>>,
}
impl TrayHandle {
    pub fn spawn(icon: TrayIcon, wake: impl Fn() + Send + Sync + 'static) -> std::io::Result<Self> {
        Self::spawn_at(icon, wake, None)
    }
    fn spawn_at(
        icon: TrayIcon,
        wake: impl Fn() + Send + Sync + 'static,
        address: Option<String>,
    ) -> std::io::Result<Self> {
        let shared = Arc::new(Shared {
            mailbox: Mutex::new(Mailbox {
                available: false,
                changed: false,
                verified: None,
                diagnostic: Some("Checking tray host".into()),
                actions: 0,
                notified: false,
                tooltip: "Connection: Connecting; data currentness unknown".into(),
                status_revision: 0,
            }),
            wake_ui: Box::new(wake),
            icon,
        });
        let (stop, stopped) = async_channel::bounded(1);
        let (refresh, refreshed) = async_channel::bounded(1);
        let state = shared.clone();
        let worker = std::thread::Builder::new()
            .name("siliconwatch-tray".into())
            .spawn(move || {
                struct Exit(Arc<Shared>);
                impl Drop for Exit {
                    fn drop(&mut self) {
                        self.0.availability(false, Some("Tray worker stopped"));
                    }
                }
                let _exit = Exit(state.clone());
                future::block_on(run(state, refreshed, address, stopped));
            })?;
        Ok(Self {
            shared,
            refresh,
            stop,
            worker: Some(worker),
        })
    }
    pub fn take_events(&self) -> TrayEvents {
        self.shared.take()
    }
    pub fn is_available(&self) -> bool {
        let s = self.shared.lock();
        s.available && s.verified.is_some_and(|t| t.elapsed() <= CAPABILITY_LEASE)
    }
    pub fn update_status(&self, connection: &str, currentness: &str) {
        let tooltip = format!(
            "Connection: {}\nData: {}",
            markup_escape(&bounded(connection, 80)),
            markup_escape(&bounded(currentness, 240))
        );
        let mut s = self.shared.lock();
        if s.tooltip != tooltip {
            s.tooltip = tooltip;
            s.status_revision = s.status_revision.wrapping_add(1);
            drop(s);
            let _ = self.refresh.try_send(());
        }
    }
    pub fn shutdown(&mut self) -> Result<(), String> {
        let _ = self.stop.try_send(());
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| "Tray worker panicked".to_string())?;
        }
        Ok(())
    }
}
impl Drop for TrayHandle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[derive(Clone)]
struct Item(Arc<Shared>);
#[zbus::interface(name = "org.kde.StatusNotifierItem", spawn = false)]
impl Item {
    #[zbus(property)]
    fn category(&self) -> &str {
        "Hardware"
    }
    #[zbus(property)]
    fn id(&self) -> &str {
        "siliconwatch"
    }
    #[zbus(property)]
    fn title(&self) -> &str {
        "SiliconWatch"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "Active"
    }
    #[zbus(property)]
    fn window_id(&self) -> i32 {
        0
    }
    #[zbus(property)]
    fn icon_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn icon_pixmap(&self) -> Pixmaps {
        self.0.icon.0.clone()
    }
    #[zbus(property)]
    fn overlay_icon_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> Pixmaps {
        Vec::new()
    }
    #[zbus(property)]
    fn attention_icon_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> Pixmaps {
        Vec::new()
    }
    #[zbus(property)]
    fn attention_movie_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(MENU_PATH).expect("constant path")
    }
    #[zbus(property)]
    fn tool_tip(&self) -> (String, Pixmaps, String, String) {
        (
            String::new(),
            Vec::new(),
            "SiliconWatch".into(),
            self.0.lock().tooltip.clone(),
        )
    }
    fn activate(&self, x: i32, y: i32) {
        let _ = (x, y);
        self.0.action(TrayAction::OpenSensors);
    }
    fn secondary_activate(&self, x: i32, y: i32) {
        let _ = (x, y);
        self.0.action(TrayAction::OpenSensors);
    }
    // Hosts with DBusMenu support render MENU_PATH themselves. A host requiring an
    // application-local popup can at least restore the accessible application controls.
    fn context_menu(&self, x: i32, y: i32) {
        let _ = (x, y);
        self.0.action(TrayAction::OpenSensors);
    }
    fn scroll(&self, delta: i32, orientation: &str) {
        let _ = (delta, orientation);
    }
    fn provide_xdg_activation_token(&self, token: &str) {
        let _ = token;
    }
    #[zbus(signal)]
    async fn new_tool_tip(emitter: &zbus::object_server::SignalEmitter<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_status(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        status: &str,
    ) -> zbus::Result<()>;
}

#[derive(Clone)]
struct Menu(Arc<Shared>);
fn invalid(message: &str) -> fdo::Error {
    fdo::Error::InvalidArgs(message.into())
}
fn valid_id(id: i32) -> bool {
    (0..=4).contains(&id)
}
fn validate_property_names(names: &[String]) -> fdo::Result<()> {
    if names.len() > MAX_BATCH || names.iter().any(|v| v.len() > 128) {
        Err(invalid("Too many or oversized property names"))
    } else {
        Ok(())
    }
}
fn properties(id: i32, names: &[String]) -> fdo::Result<Properties> {
    if !valid_id(id) {
        return Err(invalid("Unknown menu item"));
    }
    validate_property_names(names)?;
    let mut values = HashMap::new();
    if id == 0 {
        values.insert(
            "children-display".into(),
            OwnedValue::from(Str::from("submenu")),
        );
    } else {
        let label = [
            "",
            "Open Sensors",
            "Settings",
            "Reconnect",
            "Quit SiliconWatch",
        ][id as usize];
        values.insert("label".into(), OwnedValue::from(Str::from(label)));
        values.insert("enabled".into(), OwnedValue::from(true));
        values.insert("visible".into(), OwnedValue::from(true));
    }
    if !names.is_empty() {
        values.retain(|key, _| names.contains(key));
    }
    Ok(values)
}
fn layout(id: i32, depth: i32, names: &[String]) -> fdo::Result<Layout> {
    let mut children = Vec::new();
    if id == 0 && depth != 0 {
        for child in 1..=4 {
            let node: Layout = (child, properties(child, names)?, Vec::new());
            children.push(
                Value::from(node)
                    .try_to_owned()
                    .map_err(|e| fdo::Error::Failed(e.to_string()))?,
            );
        }
    }
    Ok((id, properties(id, names)?, children))
}
#[zbus::interface(name = "com.canonical.dbusmenu", spawn = false)]
impl Menu {
    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }
    #[zbus(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> fdo::Result<(u32, Layout)> {
        Ok((1, layout(parent_id, recursion_depth, &property_names)?))
    }
    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        property_names: Vec<String>,
    ) -> fdo::Result<Vec<(i32, Properties)>> {
        if ids.len() > MAX_BATCH {
            return Err(invalid("Too many menu IDs"));
        }
        validate_property_names(&property_names)?;
        let ids = if ids.is_empty() {
            (0..=4).collect()
        } else {
            ids
        };
        ids.into_iter()
            .filter(|id| valid_id(*id))
            .map(|id| Ok((id, properties(id, &property_names)?)))
            .collect()
    }
    fn get_property(&self, id: i32, name: &str) -> fdo::Result<OwnedValue> {
        properties(id, &[])?
            .remove(name)
            .ok_or_else(|| fdo::Error::UnknownProperty(name.into()))
    }
    fn event(&self, id: i32, event_id: &str, data: OwnedValue, timestamp: u32) -> fdo::Result<()> {
        let _ = (data, timestamp);
        if !valid_id(id) {
            return Err(invalid("Unknown menu item"));
        }
        if event_id == "clicked" {
            if let Some(action) = TrayAction::for_id(id) {
                self.0.action(action);
            }
        }
        Ok(())
    }
    fn event_group(&self, events: Vec<(i32, String, OwnedValue, u32)>) -> fdo::Result<Vec<i32>> {
        if events.len() > MAX_BATCH {
            return Err(invalid("Too many menu events"));
        }
        Ok(events
            .into_iter()
            .filter_map(|(id, kind, data, time)| {
                self.event(id, &kind, data, time).err().map(|_| id)
            })
            .collect())
    }
    fn about_to_show(&self, id: i32) -> fdo::Result<bool> {
        if valid_id(id) {
            Ok(false)
        } else {
            Err(invalid("Unknown menu item"))
        }
    }
    fn about_to_show_group(&self, ids: Vec<i32>) -> fdo::Result<(Vec<i32>, Vec<i32>)> {
        if ids.len() > MAX_BATCH {
            return Err(invalid("Too many menu IDs"));
        }
        Ok((
            Vec::new(),
            ids.into_iter().filter(|id| !valid_id(*id)).collect(),
        ))
    }
    #[zbus(signal)]
    async fn layout_updated(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn items_properties_updated(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        updated: Vec<(i32, Properties)>,
        removed: Vec<(i32, Vec<String>)>,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn item_activation_requested(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        id: i32,
        timestamp: u32,
    ) -> zbus::Result<()>;
}

async fn deadline<T>(
    duration: Duration,
    task: impl std::future::Future<Output = zbus::Result<T>>,
) -> zbus::Result<T> {
    future::or(task, async move {
        async_io::Timer::after(duration).await;
        Err(zbus::Error::Failure("Tray D-Bus deadline exceeded".into()))
    })
    .await
}
async fn run(
    shared: Arc<Shared>,
    refresh: async_channel::Receiver<()>,
    address: Option<String>,
    stopped: async_channel::Receiver<()>,
) {
    loop {
        let connection = future::or(
            async {
                let _ = stopped.recv().await;
                None
            },
            async {
                Some(
                    deadline(Duration::from_secs(3), async {
                        let builder = match &address {
                            Some(value) => zbus::connection::Builder::address(value.as_str())?,
                            None => zbus::connection::Builder::session()?,
                        };
                        builder
                            .internal_executor(false)
                            .max_queued(32)
                            .method_timeout(Duration::from_millis(750))
                            .serve_at(ITEM_PATH, Item(shared.clone()))?
                            .serve_at(MENU_PATH, Menu(shared.clone()))?
                            .build()
                            .await
                    })
                    .await,
                )
            },
        )
        .await;
        let Some(connection) = connection else {
            return;
        };
        match connection {
            Ok(conn) => {
                let result = future::or(
                    async {
                        loop {
                            conn.executor().tick().await;
                        }
                    },
                    future::or(
                        async {
                            let _ = stopped.recv().await;
                            None
                        },
                        async { Some(watch(&conn, &shared, &refresh).await) },
                    ),
                )
                .await;
                // Dropping zbus futures alone can retain connection references in its
                // executor. Explicitly close the socket before the owned worker exits.
                shared.availability(false, Some("Tray connection closing"));
                let _ = deadline(Duration::from_millis(500), conn.close()).await;
                match result {
                    None => return,
                    Some(result) => shared.availability(
                        false,
                        Some(&format!(
                            "Tray unavailable: {}",
                            result
                                .err()
                                .map(|e| e.to_string())
                                .unwrap_or_else(|| "session ended".into())
                        )),
                    ),
                }
            }
            Err(e) => shared.availability(false, Some(&format!("Tray unavailable: {e}"))),
        }
        if future::or(
            async {
                let _ = stopped.recv().await;
                true
            },
            async {
                async_io::Timer::after(CHECK_INTERVAL).await;
                false
            },
        )
        .await
        {
            return;
        }
    }
}

async fn owner(conn: &Connection, name: &str) -> zbus::Result<String> {
    Proxy::new(
        conn,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .await?
    .call("GetNameOwner", &(name,))
    .await
}
async fn check_host(
    conn: &Connection,
    active: &mut Option<(String, String)>,
) -> zbus::Result<bool> {
    for name in WATCHERS {
        let Ok(watcher_owner) = owner(conn, name).await else {
            continue;
        };
        let watcher = Proxy::new(conn, watcher_owner.as_str(), WATCHER_PATH, name).await?;
        let props = Proxy::new(
            conn,
            watcher_owner.as_str(),
            WATCHER_PATH,
            "org.freedesktop.DBus.Properties",
        )
        .await?;
        let unique = conn
            .unique_name()
            .ok_or_else(|| zbus::Error::Failure("Missing bus name".into()))?
            .as_str();
        let value: OwnedValue = props
            .call("Get", &(name, "IsStatusNotifierHostRegistered"))
            .await?;
        if !bool::try_from(value)? {
            continue;
        }
        let changed = active
            .as_ref()
            .is_none_or(|(n, o)| n != name || o != &watcher_owner);
        let value: OwnedValue = props
            .call("Get", &(name, "RegisteredStatusNotifierItems"))
            .await?;
        let items: Vec<String> = value.try_into()?;
        if changed || !items.iter().any(|s| s == &format!("{unique}{ITEM_PATH}")) {
            let _: () = watcher
                .call("RegisterStatusNotifierItem", &(unique,))
                .await?;
        }
        let value: OwnedValue = props
            .call("Get", &(name, "RegisteredStatusNotifierItems"))
            .await?;
        let items: Vec<String> = value.try_into()?;
        if !items.iter().any(|s| s == &format!("{unique}{ITEM_PATH}")) {
            *active = None;
            return Ok(false);
        }
        if owner(conn, name).await? != watcher_owner {
            *active = None;
            return Ok(false);
        }
        *active = Some((name.into(), watcher_owner));
        return Ok(true);
    }
    *active = None;
    Ok(false)
}
async fn watch(
    conn: &Connection,
    shared: &Arc<Shared>,
    refresh: &async_channel::Receiver<()>,
) -> zbus::Result<()> {
    let mut names = deadline(
        CHECK_DEADLINE,
        MessageStream::for_match_rule(
            zbus::MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .sender("org.freedesktop.DBus")?
                .interface("org.freedesktop.DBus")?
                .member("NameOwnerChanged")?
                .build(),
            conn,
            Some(16),
        ),
    )
    .await?;
    let mut signals = deadline(
        CHECK_DEADLINE,
        MessageStream::for_match_rule(
            zbus::MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .path(WATCHER_PATH)?
                .build(),
            conn,
            Some(16),
        ),
    )
    .await?;
    let mut active = None;
    let mut next_check = Instant::now();
    let mut revision = 0;
    loop {
        if Instant::now() >= next_check {
            match deadline(CHECK_DEADLINE, check_host(conn, &mut active)).await {
                Ok(true) => shared.availability(true, None),
                Ok(false) => shared.availability(false, Some("No usable StatusNotifier tray host")),
                Err(e) => {
                    active = None;
                    shared.availability(false, Some(&format!("Tray host check failed: {e}")));
                }
            }
            next_check = Instant::now() + CHECK_INTERVAL;
        }
        let current_revision = shared.lock().status_revision;
        if revision != current_revision {
            deadline(
                Duration::from_millis(750),
                conn.emit_signal(None::<&str>, ITEM_PATH, ITEM_INTERFACE, "NewToolTip", &()),
            )
            .await?;
            revision = current_revision;
        }
        enum Wake {
            Timer,
            Refresh,
            Name(Option<zbus::Result<zbus::Message>>),
            Host(Option<zbus::Result<zbus::Message>>),
        }
        let wake = future::or(
            async {
                async_io::Timer::at(next_check).await;
                Wake::Timer
            },
            future::or(
                async {
                    let _ = refresh.recv().await;
                    Wake::Refresh
                },
                future::or(async { Wake::Name(names.next().await) }, async {
                    Wake::Host(signals.next().await)
                }),
            ),
        )
        .await;
        match wake {
            Wake::Timer | Wake::Refresh => {}
            Wake::Name(Some(Ok(message))) => {
                let (name, _, _): (String, String, String) = message.body().deserialize()?;
                if WATCHERS.contains(&name.as_str()) {
                    active = None;
                    shared.availability(false, Some("Tray watcher changed"));
                    next_check = Instant::now();
                }
            }
            Wake::Host(Some(Ok(message))) => {
                if active.as_ref().is_some_and(|(_, owner)| {
                    message
                        .header()
                        .sender()
                        .is_some_and(|s| s.as_str() == owner)
                }) {
                    if message
                        .header()
                        .member()
                        .is_some_and(|m| m.as_str() == "StatusNotifierHostUnregistered")
                    {
                        shared.availability(false, Some("Tray host disappeared"));
                    }
                    next_check = Instant::now();
                }
            }
            Wake::Name(Some(Err(e))) | Wake::Host(Some(Err(e))) => return Err(e),
            Wake::Name(None) | Wake::Host(None) => {
                return Err(zbus::Error::Failure("Session bus closed".into()));
            }
        }
    }
}

#[cfg(test)]
#[path = "tray/tests.rs"]
mod tests;
