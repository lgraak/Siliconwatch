//! Opt-in native integration probe. Run against an externally supplied local API fixture.
//! This uses the production Application and Winit renderer; it never substitutes a headless UI.
use siliconwatch_gui::app::Application;
use siliconwatch_gui::lifecycle::Event as LifecycleEvent;
use siliconwatch_gui::preferences::{
    Density, Preferences, TemperatureUnit, Theme, parse_profile, read_import, serialize_profile,
};
use siliconwatch_gui::statistics::{Aggregate, RowKey};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, LogicalSize, Model, SharedString, Timer, TimerMode, Window};
use std::{
    cell::{Cell, RefCell},
    error::Error,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn resize(window: &Window, width: f32, height: f32) {
    // Slint's override may differ from Winit's native scale. Supply physical pixels explicitly.
    let scale = window.scale_factor();
    let physical = LogicalSize::new(width, height).to_physical(scale);
    eprintln!(
        "RESIZE logical {width}x{height}, Slint scale {scale}, requested physical {}x{}",
        physical.width, physical.height
    );
    window.set_size(physical);
}

fn key(window: &Window, key: Key) -> Result<()> {
    let text: SharedString = key.into();
    window.try_dispatch_event(WindowEvent::KeyPressed { text: text.clone() })?;
    window.try_dispatch_event(WindowEvent::KeyReleased { text })?;
    Ok(())
}
fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
#[derive(Default)]
struct Capture {
    minimum_logical: (f32, f32),
    rendered_frames: u64,
    pending: bool,
    pixels: Option<std::result::Result<slint::SharedPixelBuffer<slint::Rgba8Pixel>, String>>,
    completed: std::collections::BTreeSet<String>,
}
fn capture_after_render<C: ComponentHandle + 'static>(
    component: &C,
    minimum_logical: (f32, f32),
) -> Result<Rc<RefCell<Capture>>> {
    let capture = Rc::new(RefCell::new(Capture {
        minimum_logical,
        ..Default::default()
    }));
    let state = capture.clone();
    let weak = component.as_weak();
    component
        .window()
        .set_rendering_notifier(move |event, _| {
            if !matches!(event, slint::RenderingState::AfterRendering) {
                return;
            }
            {
                let mut state = state.borrow_mut();
                state.rendered_frames = state.rendered_frames.saturating_add(1);
                if !state.pending {
                    return;
                }
            }
            // FemtoVG exposes the just-rendered back buffer here, before native presentation.
            // No application/model callbacks or PNG encoding run inside the rendering callback.
            let result = weak
                .upgrade()
                .ok_or_else(|| "window disappeared before capture".to_string())
                .and_then(|window| {
                    window
                        .window()
                        .take_snapshot()
                        .map_err(|error| error.to_string())
                });
            let mut state = state.borrow_mut();
            state.pending = false;
            state.pixels = Some(result);
        })
        .map_err(|error| {
            format!("native renderer does not support capture notification: {error:?}")
        })?;
    Ok(capture)
}
fn snapshot(
    window: &Window,
    capture: &Rc<RefCell<Capture>>,
    directory: &Path,
    name: &str,
    log: &mut File,
) -> Result<bool> {
    require(
        window.is_visible(),
        "snapshot requires a visible native window",
    )?;
    let mut state = capture.borrow_mut();
    if state.completed.contains(name) {
        return Ok(true);
    }
    let Some(result) = state.pixels.take() else {
        if !state.pending {
            state.pending = true;
            window.request_redraw();
        }
        return Ok(false);
    };
    let pixels = result.map_err(|error| format!("native frame capture failed: {error}"))?;
    require(
        pixels.width() > 100 && pixels.height() > 100,
        "snapshot is unexpectedly small",
    )?;
    let bytes = pixels.as_bytes();
    let first = &bytes[..4];
    require(
        bytes.chunks_exact(4).any(|pixel| pixel != first),
        "AfterRendering snapshot is a flat frame",
    )?;
    image::save_buffer_with_format(
        directory.join(format!("{name}.png")),
        bytes,
        pixels.width(),
        pixels.height(),
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )?;
    writeln!(
        log,
        "SNAPSHOT {name}: {}x{} physical, scale {}, logical {:.1}x{:.1}, Window::size={}x{}, AfterRendering before presentation",
        pixels.width(),
        pixels.height(),
        window.scale_factor(),
        pixels.width() as f32 / window.scale_factor(),
        pixels.height() as f32 / window.scale_factor(),
        window.size().width,
        window.size().height
    )?;
    log.flush()?;
    let actual = (
        pixels.width() as f32 / window.scale_factor(),
        pixels.height() as f32 / window.scale_factor(),
    );
    let minimum = state.minimum_logical;
    require(
        actual.0 + 1. >= minimum.0 && actual.1 + 1. >= minimum.1,
        &format!(
            "{name}: native frame logical {:.1}x{:.1} is below declared minimum {:.0}x{:.0}; the failed frame was preserved",
            actual.0, actual.1, minimum.0, minimum.1
        ),
    )?;
    state.completed.insert(name.to_owned());
    Ok(true)
}

fn appearance(app: &Rc<RefCell<Application>>, theme: Theme, size: u32, density: Density) {
    let mut app = app.borrow_mut();
    app.preferences.appearance.theme = theme;
    app.preferences.appearance.font_size_px = size;
    app.preferences.table.density = density;
    app.render();
}
fn statistics(app: &Rc<RefCell<Application>>) -> Vec<(RowKey, Option<Aggregate>)> {
    let app = app.borrow();
    app.model
        .sensors()
        .iter()
        .map(|sensor| {
            (
                RowKey::of(sensor),
                app.model.statistics.aggregate(sensor).cloned(),
            )
        })
        .collect()
}

fn statistics_not_reset(
    app: &Rc<RefCell<Application>>,
    before: &[(RowKey, Option<Aggregate>)],
    instance: &Option<String>,
) -> Result<()> {
    let app = app.borrow();
    require(
        app.model.statistics.instance().map(str::to_owned) == *instance,
        "statistics source epoch changed during the settings operation",
    )?;
    for (key, previous) in before {
        if let Some(previous) = previous {
            let sensor = app
                .model
                .sensors()
                .iter()
                .find(|sensor| RowKey::of(sensor) == *key)
                .ok_or("fixture source row disappeared during settings verification")?;
            let after = app
                .model
                .statistics
                .aggregate(sensor)
                .ok_or("settings operation lost a session aggregate")?;
            require(
                after.count >= previous.count
                    && after.minimum <= previous.minimum
                    && after.maximum >= previous.maximum,
                "settings operation reset or reduced a session aggregate",
            )?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let output = PathBuf::from(
        std::env::var("SW_NATIVE_OUTPUT")
            .map_err(|_| "SW_NATIVE_OUTPUT must name an external evidence directory")?,
    );
    fs::create_dir_all(&output)?;
    let mut log = File::create(output.join("native-check.log"))?;
    writeln!(
        log,
        "Production Application; native Winit/FemtoVG; API fixture supplied by the runner."
    )?;
    writeln!(
        log,
        "SLINT_BACKEND={:?}; SLINT_SCALE_FACTOR={:?}; WINIT_X11_SCALE_FACTOR={:?}",
        std::env::var("SLINT_BACKEND").ok(),
        std::env::var("SLINT_SCALE_FACTOR").ok(),
        std::env::var("WINIT_X11_SCALE_FACTOR").ok()
    )?;
    let app = Application::new(1000)?;
    let restart_only = std::env::var_os("SW_NATIVE_RESTART_ONLY").is_some();
    if let Some(path) = std::env::var_os("SW_NATIVE_EXPECT_PROFILE") {
        let path = PathBuf::from(path);
        require(
            path.is_absolute(),
            "SW_NATIVE_EXPECT_PROFILE must be absolute",
        )?;
        let expected = read_import(&path)?;
        let matched = app.borrow().preferences == expected;
        if !matched {
            app.borrow_mut().shutdown();
            return Err("restart did not load the expected persisted preferences".into());
        }
        writeln!(
            log,
            "PASS restart loaded the exact expected profile before any appearance override: {}",
            path.display()
        )?;
        log.flush()?;
    } else {
        require(
            !restart_only,
            "SW_NATIVE_RESTART_ONLY requires SW_NATIVE_EXPECT_PROFILE",
        )?;
    }
    if let Ok(scenario) = std::env::var("SW_NATIVE_TRAY_SCENARIO") {
        require(
            !restart_only,
            "tray scenarios and restart-only are separate runs",
        )?;
        return tray_scenario(app, output, log, &scenario);
    }
    let (main, details, settings) = {
        let app = app.borrow();
        (
            app.main.clone_strong(),
            app.details.clone_strong(),
            app.settings.clone_strong(),
        )
    };
    let main_capture = capture_after_render(&main, (760., 520.))?;
    let details_capture = capture_after_render(&details, (640., 480.))?;
    let settings_capture = capture_after_render(&settings, (640., 480.))?;
    if !restart_only {
        appearance(&app, Theme::OledBlack, 14, Density::Compact);
        main.window().set_maximized(false);
    }
    main.show()?;
    if !restart_only {
        resize(main.window(), 1120., 680.);
    }
    main.invoke_focus_table();
    let started = Instant::now();
    let outcome = Rc::new(RefCell::new(None::<String>));
    let result_cell = outcome.clone();
    let task_app = app.clone();
    let timer = Timer::default();
    let import_path = output.join("native-import.toml");
    let export_path = output.join("native-export.toml");
    let mut candidate = Preferences::default();
    candidate.appearance.theme = Theme::Dark;
    candidate.appearance.font_size_px = 16;
    candidate.table.column_order.reverse();
    fs::write(&import_path, serialize_profile(&candidate)?)?;
    let mut before_import = None::<Preferences>;
    let mut before_statistics = Vec::new();
    let mut statistics_instance = None::<String>;
    let mut settings_open_frame = 0;
    let mut settings_minimum_requested = false;
    let mut phase = 0;
    timer.start(TimerMode::Repeated, Duration::from_millis(450), move || {
        let result = (|| -> Result<bool> {
            require(started.elapsed() < Duration::from_secs(60), "native probe exceeded its 60 second transaction budget")?;
            // Each following phase sees the completed production ConfigWorker result.
            if settings.get_busy() { return Ok(false); }
            match phase {
                0 => {
                    let rows = main.get_rows();
                    if !rows.iter().any(|row| !row.group) {
                        require(started.elapsed() < Duration::from_secs(20), "no sensor rows arrived from the external API fixture within 20 seconds")?;
                        return Ok(false);
                    }
                    writeln!(log, "API STATUS {}", main.get_status())?;
                    for row in rows.iter() {
                        writeln!(log, "ROW group={} label={:?} quality={:?} current={:?} min={:?} max={:?} average={:?} count={:?}", row.group, row.label, row.quality, row.current, row.minimum, row.maximum, row.average, row.count)?;
                    }
                    require(rows.row_count() >= 3, "native keyboard probe requires at least a group and two sensor rows")?;
                    main.invoke_focus_table();
                }
                1 => {
                    if restart_only {
                        if !snapshot(main.window(), &main_capture, &output, "restart-main", &mut log)? { return Ok(false); }
                        writeln!(log, "PASS persisted preferences restart and native rendering")?;
                        log.flush()?;
                        return Ok(true);
                    }
                    if !snapshot(main.window(), &main_capture, &output, "01-main-oled-default", &mut log)? { return Ok(false); }
                    key(main.window(), Key::Home)?;
                    require(main.get_selected_row() == 0, "Home did not select the first row")?;
                    key(main.window(), Key::DownArrow)?;
                    require(main.get_selected_row() == 1, "Down did not select the next row")?;
                    key(main.window(), Key::UpArrow)?;
                    require(main.get_selected_row() == 0, "Up did not restore the previous row")?;
                    key(main.window(), Key::End)?;
                    require(main.get_selected_row() as usize == main.get_rows().row_count() - 1, "End did not select the last row")?;
                    key(main.window(), Key::PageUp)?;
                    require((main.get_selected_row() as usize) < main.get_rows().row_count() - 1, "PageUp did not move toward the first row")?;
                    let before_page_down = main.get_selected_row();
                    key(main.window(), Key::PageDown)?;
                    require(main.get_selected_row() > before_page_down, "PageDown did not move toward the last row")?;
                    key(main.window(), Key::Home)?;
                    let first = main.get_rows().row_data(0).ok_or("missing first group row")?;
                    require(first.group, "fixture must expose a leading device group")?;
                    key(main.window(), Key::RightArrow)?;
                    let expanded = main.get_rows().row_count();
                    key(main.window(), Key::LeftArrow)?;
                    require(main.get_rows().row_count() < expanded, "Left did not collapse the first device group")?;
                    key(main.window(), Key::RightArrow)?;
                    require(main.get_rows().row_count() == expanded, "Right did not restore the expanded group")?;
                    key(main.window(), Key::DownArrow)?;
                    require(!main.get_rows().row_data(main.get_selected_row() as usize).ok_or("selected row absent")?.group, "selected row is not a sensor")?;
                    key(main.window(), Key::Return)?;
                    require(details.window().is_visible(), "Enter did not open the production Details window")?;
                    writeln!(log, "PASS Home/End/Up-Down/Page navigation, Left/Right group expansion, Enter Details")?;
                    resize(details.window(), 960., 720.);
                }
                2 => {
                    if !snapshot(details.window(), &details_capture, &output, "02-details-oled-default", &mut log)? { return Ok(false); }
                    require(!details.get_evidence().is_empty(), "Details did not expose identity/raw diagnostic evidence")?;
                    resize(details.window(), 640., 480.);
                    resize(main.window(), 760., 520.);
                }
                3 => {
                    if !snapshot(main.window(), &main_capture, &output, "03-main-oled-minimum", &mut log)? { return Ok(false); }
                    if !snapshot(details.window(), &details_capture, &output, "04-details-oled-minimum", &mut log)? { return Ok(false); }
                    key(details.window(), Key::Escape)?;
                    require(!details.window().is_visible(), "Escape did not hide Details")?;
                    let before = main.get_selected_row();
                    key(main.window(), Key::DownArrow)?;
                    require(main.get_selected_row() == before + 1, "Details close did not restore main table focus")?;
                    key(main.window(), Key::Home)?;
                    key(main.window(), Key::RightArrow)?;
                    key(main.window(), Key::DownArrow)?;
                    key(main.window(), Key::Return)?;
                    require(details.window().is_visible(), "Details did not reopen through Enter")?;
                    appearance(&task_app, Theme::Dark, 14, Density::Compact);
                    resize(main.window(), 1120., 680.);
                    resize(details.window(), 960., 720.);
                    writeln!(log, "PASS Escape Details and main table focus restoration")?;
                }
                4 => {
                    if !snapshot(main.window(), &main_capture, &output, "05-main-dark", &mut log)? { return Ok(false); }
                    if !snapshot(details.window(), &details_capture, &output, "06-details-dark", &mut log)? { return Ok(false); }
                    key(details.window(), Key::Escape)?;
                }
                5 => {
                    main.invoke_settings();
                    require(settings.window().is_visible(), "Settings callback did not show the production window")?;
                    require(settings.get_theme_index() == 1, "Settings did not load Dark theme before capture")?;
                    if !snapshot(settings.window(), &settings_capture, &output, "07-settings-dark-appearance", &mut log)? { return Ok(false); }
                    settings.invoke_focus_settings();
                    key(settings.window(), Key::Tab)?;
                    key(settings.window(), Key::Return)?;
                }
                6 => {
                    if !snapshot(settings.window(), &settings_capture, &output, "08-settings-dark-files", &mut log)? { return Ok(false); }
                    key(settings.window(), Key::Escape)?;
                    require(!settings.window().is_visible(), "Escape did not close Settings")?;
                    appearance(&task_app, Theme::Light, 14, Density::Compact);
                    main.invoke_focus_table();
                    key(main.window(), Key::Return)?;
                    require(details.window().is_visible(), "Details failed to open in Light theme")?;
                }
                7 => {
                    if !snapshot(main.window(), &main_capture, &output, "09-main-light", &mut log)? { return Ok(false); }
                    if !snapshot(details.window(), &details_capture, &output, "10-details-light", &mut log)? { return Ok(false); }
                    resize(main.window(), 760., 520.);
                    resize(details.window(), 640., 480.);
                    appearance(&task_app, Theme::Light, 24, Density::Comfortable);
                }
                8 => {
                    if !snapshot(main.window(), &main_capture, &output, "11-main-light-font24-minimum", &mut log)? { return Ok(false); }
                    if !snapshot(details.window(), &details_capture, &output, "12-details-light-font24-minimum", &mut log)? { return Ok(false); }
                    appearance(&task_app, Theme::Light, 10, Density::Compact);
                }
                9 => {
                    if !snapshot(main.window(), &main_capture, &output, "13-main-light-font10-minimum", &mut log)? { return Ok(false); }
                    if !snapshot(details.window(), &details_capture, &output, "14-details-light-font10-minimum", &mut log)? { return Ok(false); }
                    key(details.window(), Key::Escape)?;
                    appearance(&task_app, Theme::OledBlack, 14, Density::Compact);
                    settings_open_frame = settings_capture.borrow().rendered_frames;
                    main.invoke_settings();
                }
                10 => {
                    if settings_capture.borrow().rendered_frames == settings_open_frame {
                        settings.window().request_redraw();
                        return Ok(false);
                    }
                    if !settings_minimum_requested {
                        // Model a user resize after the recreated native window has completed a frame.
                        resize(settings.window(), 640., 480.);
                        settings_minimum_requested = true;
                        return Ok(false);
                    }
                    if !snapshot(settings.window(), &settings_capture, &output, "15-settings-oled-minimum", &mut log)? { return Ok(false); }
                    key(settings.window(), Key::Escape)?;
                    require(!settings.window().is_visible(), "Settings did not close at minimum size")?;
                    let before = statistics(&task_app);
                    main.invoke_unit_changed(true);
                    require(task_app.borrow().preferences.appearance.temperature_unit == TemperatureUnit::Fahrenheit, "unit callback did not apply Fahrenheit")?;
                    require(statistics(&task_app) == before, "unit conversion changed underlying session aggregates")?;
                    writeln!(log, "PASS Fahrenheit unit callback preserves canonical aggregates")?;
                }
                11 => {
                    let before = statistics(&task_app);
                    main.invoke_unit_changed(false);
                    require(task_app.borrow().preferences.appearance.temperature_unit == TemperatureUnit::Celsius, "unit callback did not restore Celsius")?;
                    require(statistics(&task_app) == before, "Celsius display conversion changed session aggregates")?;
                }
                12 => {
                    main.invoke_settings();
                    before_import = Some(task_app.borrow().preferences.clone());
                    before_statistics = statistics(&task_app);
                    require(before_statistics.iter().any(|(_, value)| value.is_some()), "fixture supplied no eligible session aggregate to verify preservation")?;
                    statistics_instance = task_app.borrow().model.statistics.instance().map(str::to_owned);
                    settings.set_path(import_path.to_string_lossy().into_owned().into());
                    settings.invoke_import_settings();
                }
                13 => {
                    require(settings.get_confirming(), "validated import did not reach preview")?;
                    if !snapshot(settings.window(), &settings_capture, &output, "16-import-preview", &mut log)? { return Ok(false); }
                    require(task_app.borrow().preferences == *before_import.as_ref().ok_or("missing preferences baseline")?, "import preview changed active preferences")?;
                    settings.invoke_cancel_preview();
                    require(!settings.get_confirming(), "cancel did not dismiss import preview")?;
                    require(task_app.borrow().preferences == *before_import.as_ref().ok_or("missing preferences baseline")?, "cancel changed active preferences")?;
                    statistics_not_reset(&task_app, &before_statistics, &statistics_instance)?;
                    settings.invoke_import_settings();
                }
                14 => {
                    require(settings.get_confirming(), "second import did not reach preview")?;
                    key(settings.window(), Key::Return)?;
                }
                15 => {
                    require(task_app.borrow().preferences == candidate, "confirmed import did not atomically replace active preferences")?;
                    statistics_not_reset(&task_app, &before_statistics, &statistics_instance)?;
                    if !snapshot(main.window(), &main_capture, &output, "17-imported-column-order", &mut log)? { return Ok(false); }
                    writeln!(log, "PASS production ConfigWorker import preview/cancel/confirm; statistics preserved")?;
                    settings.set_path(export_path.to_string_lossy().into_owned().into());
                    settings.set_include_groups(false);
                    settings.invoke_export_settings();
                }
                16 => {
                    require(settings.get_confirming(), "export did not show destination preview")?;
                    if !snapshot(settings.window(), &settings_capture, &output, "18-export-preview", &mut log)? { return Ok(false); }
                    key(settings.window(), Key::Return)?;
                }
                17 => {
                    let exported = parse_profile(&fs::read(&export_path)?)?;
                    require(exported == candidate, "export file did not contain the validated current preferences")?;
                    statistics_not_reset(&task_app, &before_statistics, &statistics_instance)?;
                    let before_cancelled_reset = statistics(&task_app);
                    main.invoke_reset_statistics();
                    require(settings.get_confirming(), "Reset Statistics did not request confirmation")?;
                    settings.invoke_cancel_preview();
                    require(statistics(&task_app) == before_cancelled_reset, "cancelled Reset Statistics cleared aggregates")?;
                    writeln!(log, "PASS production export confirmation/file round-trip and cancelled statistics reset")?;
                    key(settings.window(), Key::Escape)?;
                    require(!settings.window().is_visible(), "final Escape did not hide Settings")?;
                    writeln!(log, "PASS native snapshot scenarios completed. Pixel inspection and assistive-technology qualification are separate evidence; this probe does not certify either.")?;
                    log.flush()?;
                    return Ok(true);
                }
                _ => return Err("invalid native probe phase".into()),
            }
            phase += 1;
            main.window().request_redraw();
            if details.window().is_visible() { details.window().request_redraw(); }
            if settings.window().is_visible() { settings.window().request_redraw(); }
            Ok(false)
        })();
        match result {
            Ok(false) => {}
            Ok(true) => { *result_cell.borrow_mut() = Some(String::new()); let _ = slint::quit_event_loop(); }
            Err(error) => {
                let message = format!("FAIL phase {phase}: {error}");
                let _ = writeln!(log, "{message}");
                let _ = log.flush();
                *result_cell.borrow_mut() = Some(message);
                let _ = slint::quit_event_loop();
            }
        }
    });
    let event_result = slint::run_event_loop_until_quit();
    timer.stop();
    app.borrow_mut().shutdown();
    event_result?;
    match outcome.borrow_mut().take() {
        Some(message) if message.is_empty() => Ok(()),
        Some(message) => Err(message.into()),
        None => Err("native event loop ended before verification completed".into()),
    }
}

/// Real native-window/configuration integration, separate from pure lifecycle tests.
/// The runner supplies an actual host for lifecycle and a private hostless session
/// bus for no-host, or a controlled watcher for host-loss. The probe does not
/// fabricate Host events or alter desktop state; host-loss uses a real D-Bus signal.
fn tray_scenario(
    app: Rc<RefCell<Application>>,
    output: PathBuf,
    mut log: File,
    scenario: &str,
) -> Result<()> {
    require(
        matches!(
            scenario,
            "default-close" | "no-host" | "lifecycle" | "host-loss"
        ),
        "SW_NATIVE_TRAY_SCENARIO must be default-close, no-host, lifecycle or host-loss",
    )?;
    let scenario = scenario.to_owned();
    let (main, settings) = {
        let app = app.borrow();
        (app.main.clone_strong(), app.settings.clone_strong())
    };
    let main_capture = capture_after_render(&main, (760., 520.))?;
    let settings_capture = capture_after_render(&settings, (640., 480.))?;
    main.show()?;
    main.invoke_focus_table();
    writeln!(
        log,
        "TRAY SCENARIO {scenario}: production lifecycle and injected native window/key events; no simulated host capability"
    )?;
    log.flush()?;
    let started = Instant::now();
    let failure = Rc::new(RefCell::new(None::<String>));
    let failure_cell = failure.clone();
    let exit_requested = Rc::new(Cell::new(false));
    let expected_exit = exit_requested.clone();
    let task_app = app.clone();
    let evidence_output = output.clone();
    let mut phase = 0;
    let mut before_hide = Vec::new();
    let mut source_instance = None::<String>;
    let mut selection = None;
    let mut collapsed = std::collections::BTreeSet::new();
    let mut committed_font_size = 0;
    let mut host_loss_requested = false;
    let timer = Timer::default();
    // Poll bounded completion predicates; no delay is used as proof of readiness.
    timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        let result = (|| -> Result<()> {
            require(started.elapsed() < Duration::from_secs(45), "tray native scenario exceeded its 45 second budget")?;
            if expected_exit.get() || settings.get_busy() { return Ok(()); }
            match phase {
                0 => {
                    if !statistics(&task_app).iter().any(|(_, aggregate)| aggregate.is_some()) {
                        require(started.elapsed() < Duration::from_secs(20), "tray fixture produced no eligible sensor observation")?;
                        return Ok(());
                    }
                    require(!task_app.borrow().preferences.behavior.minimize_to_tray, "tray scenario requires a new default-off configuration")?;
                    if scenario == "default-close" {
                        writeln!(log, "PASS default-off loaded; dispatching Sensors CloseRequested, expecting full GUI termination")?;
                        log.flush()?;
                        expected_exit.set(true);
                        main.window().try_dispatch_event(WindowEvent::CloseRequested)?;
                        return Ok(());
                    }
                    if matches!(scenario.as_str(), "lifecycle" | "host-loss") && !task_app.borrow().tray_available() {
                        require(started.elapsed() < Duration::from_secs(20), "real tray host did not become usable")?;
                        return Ok(());
                    }
                    if scenario == "no-host" {
                        require(!task_app.borrow().tray_available(), "no-host scenario unexpectedly has a usable host; runner must supply a private hostless bus")?;
                    }
                    task_app.borrow_mut().lifecycle_event(LifecycleEvent::Settings);
                    require(settings.window().is_visible(), "tray Settings action did not show the existing Settings window")?;
                }
                1 => {
                    if !snapshot(settings.window(), &settings_capture, &evidence_output, "tray-01-default-off", &mut log)? { return Ok(()); }
                    require(!settings.get_minimize_to_tray(), "Settings did not load default-off behavior")?;
                    settings.invoke_focus_settings();
                    key(settings.window(), Key::Tab)?; // Import, Export & Recovery page button.
                    key(settings.window(), Key::Tab)?; // First control on the current Appearance page.
                    let text: SharedString = " ".into();
                    settings.window().try_dispatch_event(WindowEvent::KeyPressed { text: text.clone() })?;
                    settings.window().try_dispatch_event(WindowEvent::KeyReleased { text })?;
                    require(settings.get_minimize_to_tray(), "Tab/Space did not toggle the accessible Minimize to Tray checkbox")?;
                    require(!task_app.borrow().preferences.behavior.minimize_to_tray, "editing the checkbox prematurely changed committed preferences")?;
                }
                2 => {
                    if !snapshot(settings.window(), &settings_capture, &evidence_output, "tray-02-enabled-before-apply", &mut log)? { return Ok(()); }
                    settings.invoke_apply();
                }
                3 => {
                    require(task_app.borrow().preferences.behavior.minimize_to_tray, "Apply did not commit Minimize to Tray")?;
                    let config = siliconwatch_gui::preferences::default_config_path()?.path;
                    let saved = read_import(&config)?;
                    require(saved.behavior.minimize_to_tray, "Minimize to Tray was not persisted")?;
                    require(saved == task_app.borrow().preferences, "persisted behavior/profile differs from the applied preferences")?;
                    fs::write(evidence_output.join("tray-expected-profile.toml"), serialize_profile(&saved)?)?;
                    writeln!(log, "PASS actual Settings Tab/Space checkbox and atomic Apply persisted [behavior].minimize_to_tray=true")?;
                    key(settings.window(), Key::Escape)?;
                    require(!settings.window().is_visible(), "Escape did not close Settings before Sensors close")?;
                    if scenario == "no-host" {
                        require(!task_app.borrow().tray_available(), "host appeared before the no-host close")?;
                        writeln!(log, "PASS enabled preference with unavailable host; dispatching final visible window CloseRequested, expecting exit")?;
                        log.flush()?;
                        expected_exit.set(true);
                        main.window().try_dispatch_event(WindowEvent::CloseRequested)?;
                        return Ok(());
                    }
                    require(task_app.borrow().tray_available(), "host unavailable before hide")?;
                    // Preserve actual user state, not the trivial None/empty defaults.
                    main.invoke_focus_table();
                    key(main.window(), Key::End)?;
                    key(main.window(), Key::LeftArrow)?;
                    key(main.window(), Key::LeftArrow)?;
                    key(main.window(), Key::Home)?;
                    key(main.window(), Key::DownArrow)?;
                    selection = task_app.borrow().model.selected.clone();
                    collapsed = task_app.borrow().model.collapsed.clone();
                    require(matches!(selection, Some(siliconwatch_gui::model::VisibleRow::Sensor(_))), "tray fixture did not establish a selected sensor")?;
                    require(!collapsed.is_empty(), "tray fixture did not establish a collapsed group")?;
                    before_hide = statistics(&task_app);
                    source_instance = task_app.borrow().model.statistics.instance().map(str::to_owned);
                    main.window().try_dispatch_event(WindowEvent::CloseRequested)?;
                    require(!main.window().is_visible(), "enabled close did not hide Sensors")?;
                    require(!task_app.borrow().details.window().is_visible() && !settings.window().is_visible(), "fixture did not reach an all-windows-hidden tray state")?;
                    statistics_not_reset(&task_app, &before_hide, &source_instance)?;
                }
                4 => {
                    if scenario == "host-loss" {
                        if !host_loss_requested {
                            require(!main.window().is_visible(), "Sensors was not hidden before controlled host loss")?;
                            require(task_app.borrow().tray_available(), "controlled host was unavailable before trigger")?;
                            writeln!(log, "READY hidden for host loss")?;
                            log.flush()?;
                            host_loss_requested = true;
                            return Ok(());
                        }
                        if task_app.borrow().tray_available() || !main.window().is_visible() { return Ok(()); }
                        require(std::ptr::eq(main.window(), task_app.borrow().main.window()), "host-loss restore replaced the main component")?;
                        require(task_app.borrow().model.selected == selection, "host-loss restore changed selection")?;
                        require(task_app.borrow().model.collapsed == collapsed, "host-loss restore changed collapsed groups")?;
                        statistics_not_reset(&task_app, &before_hide, &source_instance)?;
                        writeln!(log, "PASS real watcher host-loss signal restored Sensors automatically with selected sensor, collapsed groups and aggregates preserved")?;
                        phase += 1;
                        return Ok(());
                    }
                    require(!main.window().is_visible(), "Sensors unexpectedly became visible while host remained available")?;
                    require(task_app.borrow().tray_available(), "host disappeared during the native hide/restore fixture")?;
                    statistics_not_reset(&task_app, &before_hide, &source_instance)?;
                    let current = statistics(&task_app);
                    let progressed = before_hide.iter().any(|(key, before)| {
                        before.as_ref().is_some_and(|before| current.iter().any(|(current_key, after)| {
                            current_key == key && after.as_ref().is_some_and(|after| after.count > before.count)
                        }))
                    });
                    if !progressed { return Ok(()); }
                    // A distinct publication was admitted while all windows were hidden.
                    writeln!(log, "PASS API client and same-epoch session statistics progressed while Sensors was hidden")?;
                    for _ in 0..3 {
                        task_app.borrow_mut().lifecycle_event(LifecycleEvent::OpenSensors);
                        require(main.window().is_visible(), "Open Sensors did not restore the existing window")?;
                        require(std::ptr::eq(main.window(), task_app.borrow().main.window()), "restore replaced the main component/window handle")?;
                    }
                    require(task_app.borrow().model.selected == selection, "restore changed the existing selection")?;
                    require(task_app.borrow().model.collapsed == collapsed, "restore changed existing group expansion")?;
                    statistics_not_reset(&task_app, &before_hide, &source_instance)?;
                }
                5 => {
                    if !snapshot(main.window(), &main_capture, &evidence_output, "tray-03-restored-session", &mut log)? { return Ok(()); }
                    task_app.borrow_mut().lifecycle_event(LifecycleEvent::Settings);
                    require(settings.window().is_visible(), "tray Settings did not reopen")?;
                    require(std::ptr::eq(settings.window(), task_app.borrow().settings.window()), "tray Settings replaced its existing component")?;
                    require(settings.get_minimize_to_tray(), "restored Settings lost the persisted tray preference")?;
                    committed_font_size = task_app.borrow().preferences.appearance.font_size_px;
                    let edited_font_size = if committed_font_size == 24 { 23 } else { 24 };
                    settings.set_font_size_px(edited_font_size);
                    task_app.borrow_mut().lifecycle_event(LifecycleEvent::Settings);
                    require(settings.get_font_size_px() == edited_font_size, "repeated Settings focus discarded an uncommitted edit")?;
                    require(task_app.borrow().preferences.appearance.font_size_px == committed_font_size, "unapplied Settings edit changed committed preferences")?;
                    let before = statistics(&task_app);
                    task_app.borrow_mut().lifecycle_event(LifecycleEvent::Reconnect);
                    require(statistics(&task_app) == before, "tray Reconnect reset statistics synchronously")?;
                    writeln!(log, "PASS repeated restore reused both window components and preserved selection, expansion and aggregates; Reconnect used the production callback")?;
                }
                6 => {
                    key(settings.window(), Key::Escape)?;
                    require(!settings.window().is_visible(), "Settings remained visible before Quit")?;
                    require(task_app.borrow().preferences.appearance.font_size_px == committed_font_size, "closing Settings applied a discarded draft")?;
                    fs::write(evidence_output.join("tray-expected-profile.toml"), serialize_profile(&task_app.borrow().preferences)?)?;
                    writeln!(log, "PASS repeated Settings activation preserved its uncommitted edit; closing discarded it")?;
                    writeln!(log, "PASS requesting explicit Quit through production lifecycle; runner separately checks tray removal and external daemon survival")?;
                    log.flush()?;
                    expected_exit.set(true);
                    task_app.borrow_mut().lifecycle_event(LifecycleEvent::Quit);
                    return Ok(());
                }
                _ => return Err("invalid native tray scenario phase".into()),
            }
            phase += 1;
            Ok(())
        })();
        if let Err(error) = result {
            let message = format!("FAIL tray phase {phase}: {error}");
            let _ = writeln!(log, "{message}");
            let _ = log.flush();
            *failure_cell.borrow_mut() = Some(message);
            let _ = slint::quit_event_loop();
        }
    });
    let event_result = slint::run_event_loop_until_quit();
    timer.stop();
    app.borrow_mut().shutdown();
    event_result?;
    if let Some(message) = failure.borrow_mut().take() {
        return Err(message.into());
    }
    require(
        exit_requested.get(),
        "event loop exited before the expected production Quit/close",
    )?;
    let mut log = fs::OpenOptions::new()
        .append(true)
        .open(output.join("native-check.log"))?;
    writeln!(
        log,
        "PASS expected close/Quit ended the native event loop and Application::shutdown returned after owned-worker joins; external daemon survival is checked by the runner"
    )?;
    log.flush()?;
    Ok(())
}
