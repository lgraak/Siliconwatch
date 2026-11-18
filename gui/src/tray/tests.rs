use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
fn shared() -> (Arc<Shared>, Arc<AtomicUsize>) {
    let wakes = Arc::new(AtomicUsize::new(0));
    let count = wakes.clone();
    let shared = Arc::new(Shared {
        mailbox: Mutex::new(Mailbox {
            available: false,
            changed: false,
            verified: None,
            diagnostic: None,
            actions: 0,
            notified: false,
            tooltip: String::new(),
            status_revision: 0,
        }),
        wake_ui: Box::new(move || {
            count.fetch_add(1, Ordering::SeqCst);
        }),
        icon: TrayIcon::from_rgba(1, 1, &[1, 2, 3, 255]).unwrap(),
    });
    (shared, wakes)
}
#[test]
fn exact_icon_pixels_use_network_argb_and_bounded_display_sizes() {
    let icon = TrayIcon::from_rgba(1, 1, &[1, 2, 3, 255]).unwrap();
    assert_eq!(icon.0.len(), 3);
    for (w, h, data) in icon.0 {
        assert_eq!(w, h);
        assert_eq!(data.len(), (w * h * 4) as usize);
        assert!(data.chunks_exact(4).all(|v| v == [255, 1, 2, 3]));
    }
    assert!(TrayIcon::from_rgba(0, 1, &[]).is_err());
    assert!(TrayIcon::from_rgba(2, 1, &[0; 4]).is_err());
    assert!(TrayIcon::from_rgba(4096, 4096, &[]).is_err());
}
#[test]
fn transparent_pixels_do_not_tint_scaled_branding() {
    let icon = TrayIcon::from_rgba(128, 1, &[255, 0, 0, 0, 0, 0, 255, 255].repeat(64)).unwrap();
    assert_eq!(&icon.0[0].2[..4], &[127, 0, 0, 255]);
}
#[test]
fn actions_coalesce_and_quit_has_priority_without_losing_restore() {
    let (state, wakes) = shared();
    for _ in 0..1000 {
        state.action(TrayAction::OpenSensors);
        state.action(TrayAction::Reconnect);
    }
    state.action(TrayAction::Quit);
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    assert_eq!(
        state.take().actions,
        vec![
            TrayAction::Quit,
            TrayAction::OpenSensors,
            TrayAction::Reconnect
        ]
    );
    assert!(state.take().actions.is_empty());
    state.action(TrayAction::Settings);
    assert_eq!(wakes.load(Ordering::SeqCst), 2);
}
#[test]
fn availability_loss_and_reappearance_are_observable_and_not_spammed() {
    let (state, wakes) = shared();
    state.availability(true, None);
    assert!(state.take().availability_changed);
    for _ in 0..10 {
        state.availability(true, None);
    }
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    state.availability(false, Some("gone"));
    let events = state.take();
    assert!(!events.available);
    assert!(events.availability_changed);
    state.availability(false, Some("gone"));
    assert_eq!(wakes.load(Ordering::SeqCst), 2);
    state.availability(true, None);
    assert!(state.take().available);
}
#[test]
fn expired_capability_fails_closed() {
    let (state, _) = shared();
    state.availability(true, None);
    state.lock().verified = Some(Instant::now() - CAPABILITY_LEASE - Duration::from_millis(1));
    let events = state.take();
    assert!(!events.available);
    assert!(events.availability_changed);
}
#[test]
fn menu_is_exactly_the_four_authorized_actions() {
    let root = layout(0, -1, &[]).unwrap();
    assert_eq!(root.2.len(), 4);
    let labels = ["Open Sensors", "Settings", "Reconnect", "Quit SiliconWatch"];
    for (id, label) in (1..=4).zip(labels) {
        let props = properties(id, &[]).unwrap();
        assert_eq!(
            <&str>::try_from(props.get("label").unwrap()).unwrap(),
            label
        );
    }
    assert!(layout(0, 0, &[]).unwrap().2.is_empty());
    assert!(layout(2, -1, &[]).unwrap().2.is_empty());
    assert!(layout(99, -1, &[]).is_err());
}
#[test]
fn menu_filters_properties_and_rejects_unbounded_requests() {
    let p = properties(1, &["label".into()]).unwrap();
    assert_eq!(p.len(), 1);
    assert!(p.contains_key("label"));
    assert!(properties(1, &vec!["x".into(); 33]).is_err());
    let (state, _) = shared();
    let menu = Menu(state);
    assert!(menu.get_group_properties(vec![1; 33], vec![]).is_err());
    assert!(menu.about_to_show_group(vec![0; 33]).is_err());
}
#[test]
fn only_known_clicked_events_execute_actions() {
    let (state, _) = shared();
    let menu = Menu(state.clone());
    menu.event(1, "opened", OwnedValue::from(0_i32), 0).unwrap();
    assert!(state.take().actions.is_empty());
    assert!(
        menu.event(20, "clicked", OwnedValue::from(0_i32), 0)
            .is_err()
    );
    menu.event(2, "clicked", OwnedValue::from(0_i32), 0)
        .unwrap();
    assert_eq!(state.take().actions, vec![TrayAction::Settings]);
    assert_eq!(
        menu.event_group(vec![(999, "clicked".into(), OwnedValue::from(0_i32), 0)])
            .unwrap(),
        vec![999]
    );
}
#[test]
fn status_is_bounded_plain_text_and_distinguishes_connection_from_data() {
    let (state, _) = shared();
    let (refresh, _) = async_channel::bounded(1);
    let (stop, _) = async_channel::bounded(1);
    let handle = TrayHandle {
        shared: state.clone(),
        refresh,
        stop,
        worker: None,
    };
    handle.update_status("Connected", "Historical <old> & stale");
    assert_eq!(
        state.lock().tooltip,
        "Connection: Connected\nData: Historical &lt;old&gt; &amp; stale"
    );
    let revision = state.lock().status_revision;
    handle.update_status("Connected", "Historical <old> & stale");
    assert_eq!(revision, state.lock().status_revision);
    handle.update_status(&"x".repeat(1000), &"x".repeat(1000));
    assert!(state.lock().tooltip.len() < 350);
}
#[test]
fn cancellation_wakes_and_joins_worker_without_a_bus() {
    let (stop, receiver) = async_channel::bounded::<()>(1);
    let start = Instant::now();
    let thread = std::thread::spawn(move || {
        future::block_on(future::or(
            async {
                receiver.recv().await.unwrap();
            },
            async {
                std::future::pending::<()>().await;
            },
        ))
    });
    stop.try_send(()).unwrap();
    thread.join().unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
}
#[test]
fn menu_wire_signatures_match_dbusmenu() {
    use zbus::zvariant::Type;
    assert_eq!(<Layout>::SIGNATURE.to_string(), "(ia{sv}av)");
    assert_eq!(<Pixmaps>::SIGNATURE.to_string(), "a(iiay)");
}

// Explicit integration qualification, separate from deterministic unit tests.
struct PrivateBus {
    child: std::process::Child,
    address: String,
}
impl PrivateBus {
    fn start() -> Self {
        use futures_lite::io::AsyncBufReadExt;
        let child = std::process::Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("private dbus-daemon required");
        let mut bus = Self {
            child,
            address: String::new(),
        };
        let stdout = async_io::Async::new(bus.child.stdout.take().unwrap()).unwrap();
        let mut reader = futures_lite::io::BufReader::new(stdout);
        bus.address = future::block_on(deadline(Duration::from_secs(2), async {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            Ok(line.trim().to_string())
        }))
        .unwrap();
        assert!(bus.address.starts_with("unix:"));
        bus
    }
}
impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
struct TestWatcher {
    host: Arc<std::sync::atomic::AtomicBool>,
    blocked: Arc<std::sync::atomic::AtomicBool>,
    entered: async_channel::Sender<()>,
    items: Arc<Mutex<Vec<String>>>,
}
#[zbus::interface(name = "org.kde.StatusNotifierWatcher", spawn = false)]
impl TestWatcher {
    fn register_status_notifier_item(&self, service: &str) {
        self.items
            .lock()
            .unwrap()
            .push(format!("{service}{ITEM_PATH}"));
    }
    #[zbus(property)]
    async fn is_status_notifier_host_registered(&self) -> bool {
        if self.blocked.load(Ordering::SeqCst) {
            let _ = self.entered.try_send(());
            std::future::pending::<()>().await;
        }
        self.host.load(Ordering::SeqCst)
    }
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.lock().unwrap().clone()
    }
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}
async fn await_capability(
    handle: &TrayHandle,
    wakes: &async_channel::Receiver<()>,
    expected: bool,
) -> TrayEvents {
    deadline(Duration::from_secs(6), async {
        loop {
            let event = handle.take_events();
            if event.available == expected {
                return Ok(event);
            }
            wakes
                .recv()
                .await
                .map_err(|e| zbus::Error::Failure(e.to_string()))?;
        }
    })
    .await
    .unwrap()
}
#[test]
#[ignore = "requires spawning an isolated session dbus-daemon; run explicitly for desktop protocol qualification"]
fn private_bus_registration_menu_loss_reappearance_deadline_and_shutdown() {
    let mut bus = PrivateBus::start();
    let (wake, wakes) = async_channel::bounded(1);
    let mut handle = TrayHandle::spawn_at(
        TrayIcon::from_rgba(1, 1, &[4, 8, 12, 255]).unwrap(),
        move || {
            let _ = wake.try_send(());
        },
        Some(bus.address.clone()),
    )
    .unwrap();
    future::block_on(async {
        deadline(Duration::from_secs(6), async {
            loop {
                let event = handle.take_events();
                if event.diagnostic.as_deref() == Some("No usable StatusNotifier tray host") {
                    assert!(!event.available);
                    break;
                }
                wakes
                    .recv()
                    .await
                    .map_err(|e| zbus::Error::Failure(e.to_string()))?;
            }
            Ok(())
        })
        .await
        .unwrap();
        let host = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let blocked = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let items = Arc::new(Mutex::new(Vec::<String>::new()));
        let (entered, entry) = async_channel::bounded(1);
        let watcher = TestWatcher {
            host: host.clone(),
            blocked: blocked.clone(),
            entered,
            items: items.clone(),
        };
        let conn = zbus::connection::Builder::address(bus.address.as_str())
            .unwrap()
            .internal_executor(false)
            .name(WATCHERS[0])
            .unwrap()
            .serve_at(WATCHER_PATH, watcher)
            .unwrap()
            .method_timeout(Duration::from_secs(1))
            .build()
            .await
            .unwrap();
        future::or(
            async {
                loop {
                    conn.executor().tick().await;
                }
            },
            async {
                assert!(await_capability(&handle, &wakes, true).await.available);
                let registered = items.lock().unwrap().last().unwrap().clone();
                let unique = registered.strip_suffix(ITEM_PATH).unwrap();
                let item = Proxy::new(&conn, unique, ITEM_PATH, ITEM_INTERFACE)
                    .await
                    .unwrap();
                let props = Proxy::new(&conn, unique, ITEM_PATH, "org.freedesktop.DBus.Properties")
                    .await
                    .unwrap();
                let all: HashMap<String, OwnedValue> =
                    props.call("GetAll", &(ITEM_INTERFACE,)).await.unwrap();
                assert_eq!(<&str>::try_from(&all["Title"]).unwrap(), "SiliconWatch");
                let pixmaps: Pixmaps = all["IconPixmap"].try_clone().unwrap().try_into().unwrap();
                assert_eq!(&pixmaps[0].2[..4], &[255, 4, 8, 12]);
                let menu = Proxy::new(&conn, unique, MENU_PATH, "com.canonical.dbusmenu")
                    .await
                    .unwrap();
                let (revision, tree): (u32, Layout) = menu
                    .call("GetLayout", &(0_i32, -1_i32, Vec::<String>::new()))
                    .await
                    .unwrap();
                assert_eq!(revision, 1);
                assert_eq!(tree.2.len(), 4);
                for _ in 0..8 {
                    let _: () = item.call("Activate", &(0_i32, 0_i32)).await.unwrap();
                }
                assert_eq!(handle.take_events().actions, vec![TrayAction::OpenSensors]);
                let _: () = menu
                    .call("Event", &(2_i32, "clicked", Value::from(0_i32), 0_u32))
                    .await
                    .unwrap();
                assert_eq!(handle.take_events().actions, vec![TrayAction::Settings]);
                let _: () = menu
                    .call("Event", &(3_i32, "clicked", Value::from(0_i32), 0_u32))
                    .await
                    .unwrap();
                assert_eq!(handle.take_events().actions, vec![TrayAction::Reconnect]);
                let _: () = menu
                    .call("Event", &(4_i32, "clicked", Value::from(0_i32), 0_u32))
                    .await
                    .unwrap();
                assert_eq!(handle.take_events().actions, vec![TrayAction::Quit]);
                let invalid: zbus::Result<()> = menu
                    .call("Event", &(99_i32, "clicked", Value::from(0_i32), 0_u32))
                    .await;
                assert!(invalid.is_err());
                // Same watcher owner, no host-change signal: periodic verification must notice.
                host.store(false, Ordering::SeqCst);
                let start = Instant::now();
                assert!(!await_capability(&handle, &wakes, false).await.available);
                assert!(start.elapsed() < Duration::from_secs(5));
                host.store(true, Ordering::SeqCst);
                assert!(await_capability(&handle, &wakes, true).await.available);
                // Actual bus name ownership loss and reacquisition must re-register the same item.
                conn.release_name(WATCHERS[0]).await.unwrap();
                assert!(!await_capability(&handle, &wakes, false).await.available);
                conn.request_name(WATCHERS[0]).await.unwrap();
                assert!(await_capability(&handle, &wakes, true).await.available);
                assert!(items.lock().unwrap().len() >= 2);
                let dbus = Proxy::new(
                    &conn,
                    "org.freedesktop.DBus",
                    "/org/freedesktop/DBus",
                    "org.freedesktop.DBus",
                )
                .await
                .unwrap();
                let mut ownership = dbus
                    .receive_signal_with_args("NameOwnerChanged", &[(0, unique)])
                    .await
                    .unwrap();
                // Cancellation while an actual watcher property call is withheld.
                blocked.store(true, Ordering::SeqCst);
                deadline(Duration::from_secs(5), async {
                    entry
                        .recv()
                        .await
                        .map_err(|e| zbus::Error::Failure(e.to_string()))
                })
                .await
                .unwrap();
                let start = Instant::now();
                handle.shutdown().unwrap();
                assert!(start.elapsed() < Duration::from_secs(1));
                let owned: bool = dbus.call("NameHasOwner", &(unique,)).await.unwrap();
                if owned {
                    deadline(Duration::from_secs(1), async {
                        loop {
                            let message = ownership.next().await.ok_or_else(|| {
                                zbus::Error::Failure("ownership stream ended".into())
                            })?;
                            let (_, _, new): (String, String, String) =
                                message.body().deserialize()?;
                            if new.is_empty() {
                                return Ok(());
                            }
                        }
                    })
                    .await
                    .expect("shutdown must remove tray bus name within one second");
                }
                // A fresh actual handle also handles complete session bus disappearance.
                let (wake2, wakes2) = async_channel::bounded(1);
                let mut other = TrayHandle::spawn_at(
                    TrayIcon::from_rgba(1, 1, &[4, 8, 12, 255]).unwrap(),
                    move || {
                        let _ = wake2.try_send(());
                    },
                    Some(bus.address.clone()),
                )
                .unwrap();
                // Withheld host keeps this unavailable; worker still must not block cancellation.
                deadline(Duration::from_secs(6), async {
                    loop {
                        let e = other.take_events();
                        if e.diagnostic
                            .as_deref()
                            .is_some_and(|x| x.contains("check failed"))
                        {
                            break;
                        }
                        wakes2
                            .recv()
                            .await
                            .map_err(|e| zbus::Error::Failure(e.to_string()))?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
                bus.child.kill().unwrap();
                bus.child.wait().unwrap();
                let start = Instant::now();
                other.shutdown().unwrap();
                assert!(start.elapsed() < Duration::from_secs(1));
            },
        )
        .await;
    });
}

#[test]
#[ignore = "requires spawning an isolated session dbus-daemon; run explicitly for desktop protocol qualification"]
fn private_bus_disconnect_invalidates_an_available_tray() {
    let mut bus = PrivateBus::start();
    let (wake, wakes) = async_channel::bounded(1);
    let mut handle = TrayHandle::spawn_at(
        TrayIcon::from_rgba(1, 1, &[4, 8, 12, 255]).unwrap(),
        move || {
            let _ = wake.try_send(());
        },
        Some(bus.address.clone()),
    )
    .unwrap();
    future::block_on(async {
        let (entered, _) = async_channel::bounded(1);
        let watcher = TestWatcher {
            host: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            blocked: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            entered,
            items: Arc::new(Mutex::new(Vec::new())),
        };
        let conn = zbus::connection::Builder::address(bus.address.as_str())
            .unwrap()
            .internal_executor(false)
            .name(WATCHERS[0])
            .unwrap()
            .serve_at(WATCHER_PATH, watcher)
            .unwrap()
            .build()
            .await
            .unwrap();
        future::or(
            async {
                loop {
                    conn.executor().tick().await;
                }
            },
            async {
                assert!(await_capability(&handle, &wakes, true).await.available);
                let start = Instant::now();
                bus.child.kill().unwrap();
                bus.child.wait().unwrap();
                assert!(!await_capability(&handle, &wakes, false).await.available);
                assert!(start.elapsed() < Duration::from_secs(5));
                handle.shutdown().unwrap();
            },
        )
        .await;
    });
}
