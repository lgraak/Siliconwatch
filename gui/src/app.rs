//! Native window coordination. Socket and configuration I/O run on bounded workers.
use crate::client::{ClientEvent, ClientHandle, ConnectionState};
use crate::lifecycle::{Action, Event, Lifecycle, Windows};
use crate::model::{FreshnessState, GroupKey, SensorModel, SourceStamp, VisibleRow, match_group};
use crate::preferences::*;
use crate::presentation::{reading, supported_unit, visible};
use crate::tray::{TrayAction, TrayHandle, TrayIcon};
use crate::ui::{ColumnSpec, DetailsWindow, MainWindow, SensorRow, SettingsWindow};
use slint::{ComponentHandle, Model, ModelRc, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

enum Preview {
    Preferences(Preferences),
    Export(ExportPlan),
    Statistics,
    Groups(Option<SourceStamp>),
}
#[derive(Clone, Copy, PartialEq)]
enum Saving {
    None,
    Automatic,
    Replace,
}

pub struct Application {
    pub main: MainWindow,
    pub details: DetailsWindow,
    pub settings: SettingsWindow,
    pub model: SensorModel,
    pub preferences: Preferences,
    draft: Preferences,
    rows: Rc<VecModel<SensorRow>>,
    details_rows: Rc<VecModel<SensorRow>>,
    details_indices: Vec<Option<usize>>,
    client: Option<ClientHandle>,
    tray: Option<TrayHandle>,
    lifecycle: Lifecycle,
    connection_status: &'static str,
    tray_notice: String,
    config: Option<ConfigWorker>,
    config_timer: Timer,
    expiry_timer: Timer,
    clock_timer: Timer,
    geometry_timer: Timer,
    effective_main: Option<(u32, u32)>,
    effective_details: Option<(u32, u32)>,
    weak: Weak<RefCell<Self>>,
    busy: bool,
    saving: Saving,
    pending_save: Option<Preferences>,
    preview: Option<Preview>,
    recovery: bool,
    reload_review_required: bool,
    closing: bool,
    known_groups: BTreeSet<GroupKey>,
    fonts: BTreeSet<String>,
    font_notice: String,
}
impl Application {
    pub fn new(refresh_ms: u64) -> Result<Rc<RefCell<Self>>, Box<dyn std::error::Error>> {
        let main = MainWindow::new()?;
        let details = DetailsWindow::new()?;
        let settings = SettingsWindow::new()?;
        let mut preferences = Preferences::default();
        let mut notice: String;
        let mut recovery = false;
        let mut config = None;
        match default_config_path() {
            Ok(choice) => {
                notice = choice.warnings.join(" · ");
                match Store::load(choice.path) {
                    Ok((store, report)) => {
                        preferences = report.preferences;
                        recovery = report.recovery.is_some();
                        if let Some(info) = report.recovery {
                            notice = format!(
                                "Configuration requires recovery: {}. Defaults are temporary; use Settings to review recovery.",
                                info.reason
                            );
                        }
                        if !report.warnings.is_empty() {
                            notice.push_str(&report.warnings.join(" · "));
                        }
                        match ConfigWorker::spawn(store) {
                            Ok(worker) => config = Some(worker),
                            Err(e) => notice = format!("Settings are unsaved: {e}"),
                        }
                    }
                    Err(e) => notice = format!("Settings unavailable; temporary defaults: {e}"),
                }
            }
            Err(e) => notice = format!("Settings unavailable; temporary defaults: {e}"),
        }
        if !preferences.group_overrides.is_empty() {
            notice.push_str(" · Saved device groups await review in Settings");
        }
        let rows = Rc::new(VecModel::default());
        main.set_rows(ModelRc::from(rows.clone()));
        let details_rows = Rc::new(VecModel::default());
        details.set_rows(ModelRc::from(details_rows.clone()));
        let fonts = installed_fonts();
        let mut model = SensorModel::default();
        model.notice = notice;
        let app = Rc::new(RefCell::new(Self {
            main,
            details,
            settings,
            model,
            draft: preferences.clone(),
            preferences,
            rows,
            details_rows,
            details_indices: Vec::new(),
            client: None,
            tray: None,
            lifecycle: Lifecycle::default(),
            connection_status: "Connecting",
            tray_notice: String::new(),
            config,
            config_timer: Timer::default(),
            expiry_timer: Timer::default(),
            clock_timer: Timer::default(),
            geometry_timer: Timer::default(),
            effective_main: None,
            effective_details: None,
            weak: Weak::new(),
            busy: false,
            saving: Saving::None,
            pending_save: None,
            preview: None,
            recovery,
            reload_review_required: false,
            closing: false,
            known_groups: BTreeSet::new(),
            fonts,
            font_notice: String::new(),
        }));
        app.borrow_mut().weak = Rc::downgrade(&app);
        Self::bind(&app);
        let weak = Rc::downgrade(&app);
        app.borrow()
            .clock_timer
            .start(TimerMode::Repeated, Duration::from_secs(1), move || {
                if let Some(app) = weak.upgrade() {
                    let mut a = app.borrow_mut();
                    if a.model.check_clock() {
                        a.render();
                    }
                }
            });
        let ui = app.borrow().main.as_weak();
        let client = ClientHandle::spawn(refresh_ms, move || {
            let _ = ui.upgrade_in_event_loop(|ui| ui.invoke_worker_wakeup());
        })?;
        {
            let mut a = app.borrow_mut();
            a.client = Some(client);
            let icon = a
                .main
                .get_branding_image()
                .to_rgba8()
                .ok_or("Embedded tray icon cannot be decoded".to_owned())
                .and_then(|pixels| {
                    TrayIcon::from_rgba(pixels.width(), pixels.height(), pixels.as_bytes())
                });
            match icon {
                Ok(icon) => {
                    let ui = a.main.as_weak();
                    match TrayHandle::spawn(icon, move || {
                        let _ = ui.upgrade_in_event_loop(|ui| ui.invoke_tray_wakeup());
                    }) {
                        Ok(tray) => a.tray = Some(tray),
                        Err(e) => a.tray_notice = format!("System tray unavailable: {e}"),
                    }
                }
                Err(e) => a.tray_notice = e,
            }
            a.apply_geometry();
            a.render();
        }
        Ok(app)
    }
    fn bind(app: &Rc<RefCell<Self>>) {
        macro_rules! callback {
            ($window:ident, $method:ident, $($arg:ident),* => $body:expr) => {{
                let weak=Rc::downgrade(app);
                app.borrow().$window.$method(move |$($arg),*| { if let Some(app)=weak.upgrade() { let mut a=app.borrow_mut(); ($body)(&mut a,$($arg),*); } });
            }};
        }
        callback!(main,on_worker_wakeup, => |a:&mut Self|a.receive_api());
        callback!(main,on_tray_wakeup, => |a:&mut Self|a.receive_tray());
        callback!(main,on_select_row,index => |a:&mut Self,i:i32|{a.model.select(i.max(0) as usize);a.render();});
        callback!(main,on_activate_row,index => |a:&mut Self,i:i32|a.open_details(i.max(0) as usize));
        callback!(main,on_toggle_group,index => |a:&mut Self,i:i32|{
            if let Some(VisibleRow::Group(group))=a.model.visible_rows().get(i.max(0) as usize).cloned() { a.model.toggle(&group);a.save_group(&group);a.render(); }
        });
        callback!(main,on_navigate,direction => |a:&mut Self,d:slint::SharedString|{
            if d=="activate" { if let Some(i)=a.model.selected_index(){a.open_details(i);} }
            else if d=="toggle" { if let Some(VisibleRow::Group(g))=a.model.selected.clone(){a.model.toggle(&g);a.save_group(&g);} }
            else if let Some(g)=a.model.navigate(&d){a.save_group(&g);}
            a.render(); if let Some(i)=a.model.selected_index(){a.main.invoke_ensure_row_visible(i as i32);}
        });
        callback!(main,on_reconnect, => |a:&mut Self|a.reconnect());
        callback!(main,on_settings, => |a:&mut Self|a.open_settings());
        callback!(main,on_unit_changed,fahrenheit => |a:&mut Self,f:bool|{
            if !a.busy && a.preview.is_none() {
                a.preferences.appearance.temperature_unit=if f{TemperatureUnit::Fahrenheit}else{TemperatureUnit::Celsius};a.autosave();
            }a.render();
        });
        callback!(main,on_reset_statistics, => |a:&mut Self|{a.open_settings();a.set_preview(Preview::Statistics,"Reset this GUI session’s statistics? Cached observations will not be counted again. Preferences and the daemon remain unchanged.".into());});
        callback!(details,on_select_row,index => |a:&mut Self,i:i32|{
            if let Some(Some(source))=a.details_indices.get(i.max(0) as usize){a.model.details_sensor=Some(*source);}a.render_details(Instant::now());
        });
        callback!(details,on_close, => |a:&mut Self|a.close_details());
        callback!(settings,on_close, => |a:&mut Self|a.close_settings());
        callback!(settings,on_apply, => |a:&mut Self|a.apply_draft());
        callback!(settings,on_import_settings, => |a:&mut Self|{let p=PathBuf::from(a.settings.get_path().as_str());a.submit(ConfigJob::Import(p),Saving::None);});
        callback!(settings,on_export_settings, => |a:&mut Self|{let p=PathBuf::from(a.settings.get_path().as_str());a.submit(ConfigJob::PrepareExport{path:p,preferences:a.preferences.clone(),include_groups:a.settings.get_include_groups()},Saving::None);});
        callback!(settings,on_restore_backup, => |a:&mut Self|{a.submit(ConfigJob::ReadBackup,Saving::None);});
        callback!(settings,on_reset_defaults, => |a:&mut Self|a.preview_preferences(Preferences::default()));
        callback!(settings,on_review_groups, => |a:&mut Self|a.review_groups());
        callback!(settings,on_confirm, => |a:&mut Self|a.confirm());
        callback!(settings,on_cancel_preview, => |a:&mut Self|{if !a.busy{a.preview=None;a.settings.set_confirming(false);if a.reload_review_required { a.model.notice="Unsaved: reload review cancelled; automatic saving remains paused until an explicit settings replacement is confirmed.".into();a.settings.set_status(a.model.notice.clone().into());a.render(); }}});
        callback!(settings,on_column_selected,index => |a:&mut Self,i:i32|{if let Some(c)=a.draft.table.column_order.get(i.max(0) as usize){a.settings.set_column_width_px(a.draft.table.column_widths_px.get(*c) as i32);}});
        callback!(settings,on_move_column,delta => |a:&mut Self,d:i32|{let i=a.settings.get_selected_column();let j=i+d;if (0..5).contains(&i)&&(0..5).contains(&j){a.draft.table.column_order.swap(i as usize,j as usize);a.settings.set_selected_column(j);a.set_draft_columns();}});
        callback!(settings,on_column_width_changed,width => |a:&mut Self,w:i32|{let i=a.settings.get_selected_column().max(0) as usize;if let Some(c)=a.draft.table.column_order.get(i){a.draft.table.column_widths_px.set(*c,w.max(0) as u32);a.set_draft_columns();}});
        callback!(settings,on_reload_config, => |a: &mut Self| { a.submit(ConfigJob::Reload, Saving::None); });
        let weak = Rc::downgrade(app);
        app.borrow().main.window().on_close_requested(move || {
            if let Some(a) = weak.upgrade() {
                a.borrow_mut().lifecycle_event(Event::CloseSensors);
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
        let weak = Rc::downgrade(app);
        app.borrow().details.window().on_close_requested(move || {
            if let Some(a) = weak.upgrade() {
                a.borrow_mut().close_details();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
        let weak = Rc::downgrade(app);
        app.borrow().settings.window().on_close_requested(move || {
            if let Some(a) = weak.upgrade() {
                a.borrow_mut().close_settings();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
    fn receive_api(&mut self) {
        if self.closing {
            return;
        }
        if let Some(event) = self.client.as_ref().and_then(ClientHandle::take_update) {
            match event {
                ClientEvent::State { kind, message } => {
                    self.connection_status = if matches!(kind, ConnectionState::Connecting) {
                        "Reconnecting"
                    } else {
                        "Disconnected"
                    };
                    self.model.disconnect(message);
                }
                ClientEvent::Snapshot(snapshot) => {
                    match self.model.accept(*snapshot, Instant::now()) {
                        Ok(changed) => {
                            self.connection_status = "Connected";
                            let groups: BTreeSet<_> = self.model.groups().into_keys().collect();
                            if changed {
                                // Source changes invalidate reviewed associations; saved records remain pending.
                                self.model.collapsed.clear();
                                if !self.preferences.group_overrides.is_empty() {
                                    self.model.notice="Source inventory changed · device preferences await review".into();
                                }
                            }
                            if !self.preferences.table.default_expanded
                                && self.preferences.group_overrides.is_empty()
                            {
                                for g in groups.difference(&self.known_groups) {
                                    self.model.collapsed.insert(g.clone());
                                }
                            }
                            self.known_groups = groups;
                        }
                        Err(message) => {
                            self.connection_status = "Disconnected";
                            self.model.disconnect(message);
                        }
                    }
                }
            }
            self.render();
        }
    }
    pub fn render(&mut self) {
        let now = Instant::now();
        if let Some(tray) = &self.tray {
            tray.update_status(self.connection_status, self.model.freshness(now).label());
        }
        self.appearance();
        let groups = self.model.groups();
        let mut rows = Vec::new();
        for (g, indices) in &groups {
            let title = group_title(
                g,
                indices.first().and_then(|i| self.model.sensors().get(*i)),
            );
            let expanded = !self.model.collapsed.contains(g);
            rows.push(SensorRow {
                label: title.clone().into(),
                quality: format!("{} readings", indices.len()).into(),
                group: true,
                expanded,
                spoken: format!(
                    "{title}, device group, {}, {} readings",
                    if expanded { "expanded" } else { "collapsed" },
                    indices.len()
                )
                .into(),
                ..Default::default()
            });
            if expanded {
                rows.extend(indices.iter().map(|&i| self.sensor_row(i, now)));
            }
        }
        update_rows(&self.rows, rows);
        self.main
            .set_selected_row(self.model.selected_index().map(|i| i as i32).unwrap_or(-1));
        self.main.set_status(self.model.status(now).into());
        let note = self.stats_note();
        self.main.set_stats_note(note.clone().into());
        self.details.set_stats_note(note.into());
        let notice = [&self.model.notice, &self.font_notice, &self.tray_notice]
            .into_iter()
            .filter(|text| !text.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" · ");
        self.main.set_notice(notice.into());
        let mut columns = columns(&self.preferences);
        // Effective readability limits do not overwrite portable requested widths.
        let font_scale = self.preferences.appearance.font_size_px as f32 / 14.0;
        for column in &mut columns {
            let base_minimum = if column.index == 0 { 240.0 } else { 112.0 };
            column.width = column.width.max(base_minimum * font_scale);
        }
        self.main
            .set_table_width(columns.iter().map(|c| c.width).sum());
        self.main.set_columns(ModelRc::new(VecModel::from(columns)));
        self.render_details(now);
        self.expiry_timer.stop();
        if let Some(duration) = self.model.next_expiry(now) {
            let weak = self.weak.clone();
            self.expiry_timer
                .start(TimerMode::SingleShot, duration, move || {
                    if let Some(a) = weak.upgrade() {
                        a.borrow_mut().render();
                    }
                });
        }
    }
    fn stats_note(&self) -> String {
        let count = match self.model.selected {
            Some(VisibleRow::Sensor(i)) => self
                .model
                .sensors()
                .get(i)
                .and_then(|s| self.model.statistics.aggregate(s))
                .map(|a| a.count)
                .unwrap_or(0),
            _ => 0,
        };
        format!("Session · selected n={count} valid observations · no history saved")
    }
    fn sensor_row(&self, index: usize, now: Instant) -> SensorRow {
        let s = &self.model.sensors()[index];
        let unit = supported_unit(s);
        let t = self.preferences.appearance.temperature_unit;
        let aggregate = self.model.statistics.aggregate(s);
        let current = if s.status == "ok" {
            reading(s.value, unit, t)
        } else {
            "—".into()
        };
        let minimum = reading(aggregate.map(|a| a.minimum), unit, t);
        let maximum = reading(aggregate.map(|a| a.maximum), unit, t);
        let average = reading(aggregate.map(|a| a.average), unit, t);
        let count = aggregate.map(|a| a.count).unwrap_or(0).to_string();
        let mut quality = if s.status != "ok" {
            s.status.clone()
        } else if unit.is_none() {
            "unsupported unit/semantics".into()
        } else {
            String::new()
        };
        if self.model.freshness(now) != FreshnessState::Recent {
            if !quality.is_empty() {
                quality.push_str(" · ");
            }
            quality.push_str(match self.model.freshness(now) {
                FreshnessState::Stale => "stale",
                FreshnessState::Historical => "historical",
                FreshnessState::Disconnected => "disconnected",
                _ => "noncurrent",
            });
        }
        if self.model.statistics.unavailable(s) {
            quality.push_str(" · statistics unavailable");
        }
        let label = visible(s.label.as_deref().unwrap_or(&s.attribute));
        let spoken = format!(
            "{label}; {quality}; Current {current}; Minimum {minimum}; Maximum {maximum}; Average {average}; {count} valid observations received"
        );
        SensorRow {
            label: label.into(),
            quality: quality.into(),
            current: current.into(),
            minimum: minimum.into(),
            maximum: maximum.into(),
            average: average.into(),
            count: count.into(),
            spoken: spoken.into(),
            group: false,
            expanded: false,
        }
    }
    pub fn open_details(&mut self, row_index: usize) {
        match self.model.visible_rows().get(row_index).cloned() {
            Some(VisibleRow::Group(group)) => {
                self.model.toggle(&group);
                self.save_group(&group);
                self.render();
            }
            Some(VisibleRow::Sensor(index)) => {
                self.model.select(row_index);
                self.model.details_sensor = Some(index);
                self.render_details(Instant::now());
                let was_visible = self.details.window().is_visible();
                if let Err(e) = self.details.show() {
                    self.model.notice = format!("Cannot open Device Details: {e}");
                }
                if !was_visible {
                    let width = self.preferences.windows.details.width_px;
                    let height = self.preferences.windows.details.height_px;
                    self.details.window().set_size(
                        slint::LogicalSize::new(width as f32, height as f32)
                            .to_physical(self.details.window().scale_factor()),
                    );
                    self.effective_details = None;
                    self.remember_effective_geometry();
                }
                self.details.invoke_focus_readings();
            }
            None => {}
        }
    }
    fn render_details(&mut self, now: Instant) {
        let Some(index) = self
            .model
            .details_sensor
            .filter(|i| *i < self.model.sensors().len())
        else {
            update_rows(&self.details_rows, Vec::new());
            self.details_indices.clear();
            self.details.set_evidence(
                "Source is no longer confidently associated. Select a reading in Sensors.".into(),
            );
            self.details.set_status(self.model.status(now).into());
            return;
        };
        let group = GroupKey::of(&self.model.sensors()[index]);
        let groups = self.model.groups();
        let indices = groups.get(&group).cloned().unwrap_or_default();
        self.details
            .set_device_title(group_title(&group, self.model.sensors().get(index)).into());
        let mut rows = Vec::new();
        let mut mapping = Vec::new();
        for (category, unit) in [
            ("Temperatures", Some("°C")),
            ("Voltages", Some("V")),
            ("Fans", Some("RPM")),
            ("Other / unsupported", None),
        ] {
            let members: Vec<_> = indices
                .iter()
                .copied()
                .filter(|i| supported_unit(&self.model.sensors()[*i]) == unit)
                .collect();
            if members.is_empty() {
                continue;
            }
            rows.push(SensorRow {
                label: category.into(),
                spoken: format!("{category}, category").into(),
                group: true,
                ..Default::default()
            });
            mapping.push(None);
            for i in members {
                rows.push(self.sensor_row(i, now));
                mapping.push(Some(i));
            }
        }
        self.details.set_selected_row(
            mapping
                .iter()
                .position(|i| *i == Some(index))
                .map(|i| i as i32)
                .unwrap_or(-1),
        );
        self.details_indices = mapping;
        update_rows(&self.details_rows, rows);
        self.details.set_status(self.model.status(now).into());
        self.details
            .set_evidence(self.model.evidence(index, &self.preferences, now).into());
    }
    fn appearance(&mut self) {
        let p = &self.preferences;
        let theme = theme_index(p.appearance.theme);
        let requested = &p.appearance.font_family;
        let available = requested == "system-ui" || self.fonts.contains(&requested.to_lowercase());
        let family = if requested == "system-ui" || !available {
            ""
        } else {
            requested
        };
        self.font_notice = if available {
            String::new()
        } else {
            format!(
                "Font ‘{}’ unavailable; using system fallback. Requested family is preserved.",
                visible(requested)
            )
        };
        let size = p.appearance.font_size_px as f32;
        let row = (if p.table.density == Density::Compact {
            28.0_f32
        } else {
            36.0_f32
        })
        .max(size + 14.0);
        self.main.set_theme_index(theme);
        self.details.set_theme_index(theme);
        self.main.set_ui_font_family(family.into());
        self.details.set_ui_font_family(family.into());
        self.settings.set_ui_font_family(family.into());
        self.main.set_ui_font_size(size);
        self.details.set_ui_font_size(size);
        self.settings.set_ui_font_size(size);
        self.main.set_row_height(row);
        self.details.set_row_height(row);
        self.main.set_temperature_fahrenheit(
            p.appearance.temperature_unit == TemperatureUnit::Fahrenheit,
        );
        self.main.set_row_separators(p.table.row_separators);
        self.main.set_alternate_rows(p.table.alternate_rows);
    }
    fn apply_geometry(&mut self) {
        self.effective_main = None;
        self.effective_details = None;
        let p = &self.preferences;
        // Winit converts LogicalSize with its native factor; Slint may have a
        // separate effective factor (for example SLINT_SCALE_FACTOR). Convert
        // explicitly so portable logical dimensions match the rendered UI.
        self.main.window().set_size(
            slint::LogicalSize::new(
                p.windows.main.width_px as f32,
                p.windows.main.height_px as f32,
            )
            .to_physical(self.main.window().scale_factor()),
        );
        self.main.window().set_maximized(p.windows.main.maximized);
        self.details.window().set_size(
            slint::LogicalSize::new(
                p.windows.details.width_px as f32,
                p.windows.details.height_px as f32,
            )
            .to_physical(self.details.window().scale_factor()),
        );
        self.remember_effective_geometry();
    }
    fn remember_effective_geometry(&self) {
        let weak = self.weak.clone();
        self.geometry_timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(350),
            move || {
                if let Some(app) = weak.upgrade() {
                    let mut a = app.borrow_mut();
                    if a.effective_main.is_none() {
                        a.effective_main = Some(logical_dimensions(a.main.window()));
                    }
                    if a.details.window().is_visible() {
                        a.effective_details = Some(logical_dimensions(a.details.window()));
                    }
                }
            },
        );
    }
    fn capture_geometry(&mut self) {
        if self.saving == Saving::Replace || self.reload_review_required {
            return;
        }
        let w = self.main.window();
        let size = w.size().to_logical(w.scale_factor());
        self.preferences.windows.main.maximized = w.is_maximized();
        if !w.is_maximized()
            && self
                .effective_main
                .is_some_and(|initial| initial != logical_dimensions(w))
        {
            self.preferences.windows.main.width_px = (size.width.round() as u32).clamp(760, 4096);
            self.preferences.windows.main.height_px = (size.height.round() as u32).clamp(520, 2160);
        }
        let w = self.details.window();
        let size = w.size().to_logical(w.scale_factor());
        if w.is_visible()
            && size.width > 0.0
            && self
                .effective_details
                .is_some_and(|initial| initial != logical_dimensions(w))
        {
            self.preferences.windows.details.width_px =
                (size.width.round() as u32).clamp(640, 4096);
            self.preferences.windows.details.height_px =
                (size.height.round() as u32).clamp(480, 2160);
        }
    }
    fn close_details(&mut self) {
        self.capture_geometry();
        self.autosave();
        let _ = self.details.hide();
        self.main.invoke_focus_table();
        self.lifecycle_event(Event::SecondaryClosed);
    }
    fn close_settings(&mut self) {
        if self.busy {
            if !(self.main.window().is_visible()
                || self.details.window().is_visible()
                || self.preferences.behavior.minimize_to_tray && self.tray_available())
            {
                self.lifecycle_event(Event::Quit);
            }
            return;
        }
        self.preview = None;
        self.settings.set_confirming(false);
        let _ = self.settings.hide();
        self.main.invoke_focus_table();
        self.lifecycle_event(Event::SecondaryClosed);
    }
    pub fn open_settings(&mut self) {
        if self.closing {
            return;
        }
        let was_visible = self.settings.window().is_visible();
        if !was_visible && self.saving != Saving::Replace && self.preview.is_none() {
            self.draft = self.preferences.clone();
            self.populate_settings();
        }
        if let Err(e) = self.settings.show() {
            self.model.notice = format!("Cannot open Settings: {e}");
        }
        if !was_visible {
            self.settings.window().set_size(
                slint::LogicalSize::new(780.0, 600.0)
                    .to_physical(self.settings.window().scale_factor()),
            );
        }
        self.settings.window().set_minimized(false);
        self.settings.invoke_focus_settings();
    }
    fn populate_settings(&self) {
        let p = &self.draft;
        let s = &self.settings;
        s.set_minimize_to_tray(p.behavior.minimize_to_tray);
        s.set_theme_index(theme_index(p.appearance.theme));
        s.set_font_family(p.appearance.font_family.clone().into());
        s.set_font_size_px(p.appearance.font_size_px as i32);
        s.set_temperature_fahrenheit(p.appearance.temperature_unit == TemperatureUnit::Fahrenheit);
        s.set_main_width_px(p.windows.main.width_px as i32);
        s.set_main_height_px(p.windows.main.height_px as i32);
        s.set_main_maximized(p.windows.main.maximized);
        s.set_details_width_px(p.windows.details.width_px as i32);
        s.set_details_height_px(p.windows.details.height_px as i32);
        s.set_density_index(if p.table.density == Density::Compact {
            0
        } else {
            1
        });
        s.set_row_separators(p.table.row_separators);
        s.set_alternate_rows(p.table.alternate_rows);
        s.set_default_expanded(p.table.default_expanded);
        self.set_draft_columns();
    }
    fn set_draft_columns(&self) {
        self.settings
            .set_columns(ModelRc::new(VecModel::from(columns(&self.draft))));
        let index = self.settings.get_selected_column().clamp(0, 4) as usize;
        self.settings.set_column_width_px(
            self.draft
                .table
                .column_widths_px
                .get(self.draft.table.column_order[index]) as i32,
        );
    }
    fn apply_draft(&mut self) {
        if self.busy {
            return;
        }
        let s = &self.settings;
        let mut p = self.draft.clone();
        p.behavior.minimize_to_tray = s.get_minimize_to_tray();
        p.appearance.theme = match s.get_theme_index() {
            1 => Theme::Dark,
            2 => Theme::Light,
            _ => Theme::OledBlack,
        };
        p.appearance.font_family = s.get_font_family().to_string();
        p.appearance.font_size_px = s.get_font_size_px().max(0) as u32;
        p.appearance.temperature_unit = if s.get_temperature_fahrenheit() {
            TemperatureUnit::Fahrenheit
        } else {
            TemperatureUnit::Celsius
        };
        p.windows.main.width_px = s.get_main_width_px().max(0) as u32;
        p.windows.main.height_px = s.get_main_height_px().max(0) as u32;
        p.windows.main.maximized = s.get_main_maximized();
        p.windows.details.width_px = s.get_details_width_px().max(0) as u32;
        p.windows.details.height_px = s.get_details_height_px().max(0) as u32;
        p.table.density = if s.get_density_index() == 0 {
            Density::Compact
        } else {
            Density::Comfortable
        };
        p.table.row_separators = s.get_row_separators();
        p.table.alternate_rows = s.get_alternate_rows();
        p.table.default_expanded = s.get_default_expanded();
        if let Err(e) = p.validate() {
            self.settings.set_status(e.to_string().into());
            return;
        }
        if self.recovery || self.reload_review_required {
            self.preview_preferences(p);
        } else {
            self.submit(
                ConfigJob::Save {
                    preferences: p,
                    recover_original: false,
                },
                Saving::Replace,
            );
        }
    }
    fn set_preview(&mut self, preview: Preview, text: String) {
        if self.busy {
            return;
        }
        self.preview = Some(preview);
        self.settings
            .set_status("Review ready · nothing has been applied".into());
        self.settings.set_preview(text.into());
        self.settings.set_confirming(true);
    }
    fn preview_preferences(&mut self, p: Preferences) {
        let groups = p.group_overrides.len();
        let text = format!(
            "Replace all GUI preferences with this validated profile?\n\nTheme: {:?} → {:?}\nFont: {} {}px → {} {}px\nTemperature: {:?} → {:?}\nMinimize to Tray: {} → {}\nMain: {}×{}; Details: {}×{}\nColumns: {:?}\nDevice group records: {} (pending explicit review; uncertain matches stay expanded)\n\n{}\nSession statistics are preserved.",
            self.preferences.appearance.theme,
            p.appearance.theme,
            self.preferences.appearance.font_family,
            self.preferences.appearance.font_size_px,
            p.appearance.font_family,
            p.appearance.font_size_px,
            self.preferences.appearance.temperature_unit,
            p.appearance.temperature_unit,
            self.preferences.behavior.minimize_to_tray,
            p.behavior.minimize_to_tray,
            p.windows.main.width_px,
            p.windows.main.height_px,
            p.windows.details.width_px,
            p.windows.details.height_px,
            p.table.column_order,
            groups,
            if self.recovery {
                "The unreadable original will be preserved before recovery."
            } else {
                "The previous valid configuration will be backed up."
            }
        );
        let text = format!(
            "{text}\n\nComplete incoming appearance and layout:\n{:#?}\n{:#?}\n{:#?}\nMissing fonts use the system fallback; requested font and window sizes remain portable.",
            p.appearance, p.windows, p.table
        );
        self.set_preview(Preview::Preferences(p), text);
    }
    fn review_groups(&mut self) {
        if self.model.freshness(Instant::now()) != FreshnessState::Recent {
            self.settings.set_status(
                "Device matches remain pending until a recent live source inventory is available."
                    .into(),
            );
            return;
        }
        if self.busy {
            return;
        }
        let mut text = String::from(
            "Apply the following source matches to this session? Best-effort identities cannot prove physical-device continuity.\n\n",
        );
        if self.preferences.group_overrides.is_empty() {
            text.push_str("No saved device group preferences.");
        }
        for (i, record) in self.preferences.group_overrides.iter().enumerate() {
            text.push_str(&format!(
                "{}: {}\n",
                i + 1,
                match match_group(record, self.model.sensors()) {
                    Ok(g) => format!(
                        "{} → {}",
                        bounded_evidence(&g.path, 512),
                        if record.expanded {
                            "expanded"
                        } else {
                            "collapsed"
                        }
                    ),
                    Err(e) => format!("{e}; remains inactive / expanded"),
                }
            ));
            for (anchor_index, member) in record.members.iter().enumerate() {
                // Validated candidates are bounded canonical tuples. Display evidence,
                // not a new identity interpretation or a promise of hardware continuity.
                let tuple: Vec<serde_json::Value> =
                    serde_json::from_str(&member.candidate_id).unwrap_or_default();
                let part = |index| {
                    tuple
                        .get(index)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unavailable")
                };
                text.push_str(&format!(
                    "  Anchor {} · provider: {} · channel: {}\n    measurement: {} · unit: {}\n    candidate: {}\n",
                    anchor_index + 1,
                    bounded_evidence(part(2), 96),
                    bounded_evidence(part(4), 96),
                    bounded_evidence(&member.measurement, 96),
                    bounded_evidence(&member.unit, 16),
                    bounded_evidence(&member.candidate_id, 256),
                ));
            }
        }
        self.set_preview(Preview::Groups(self.model.source_stamp()), text);
    }
    fn confirm(&mut self) {
        if self.busy {
            return;
        }
        let Some(preview) = self.preview.take() else {
            return;
        };
        self.settings.set_confirming(false);
        match preview {
            Preview::Preferences(p) => {
                self.submit(
                    ConfigJob::Save {
                        preferences: p,
                        recover_original: self.recovery,
                    },
                    Saving::Replace,
                );
            }
            Preview::Export(plan) => {
                self.submit(ConfigJob::CommitExport(plan), Saving::None);
            }
            Preview::Statistics => {
                self.model.statistics.clear();
                self.model.notice =
                    "Session statistics reset · waiting for a new distinct eligible observation"
                        .into();
                self.render();
            }
            Preview::Groups(stamp) => {
                if self.model.freshness(Instant::now()) != FreshnessState::Recent
                    || stamp.is_none()
                    || stamp != self.model.source_stamp()
                {
                    self.settings.set_status("Source inventory changed or unavailable. Review again before applying matches.".into());
                    return;
                }
                self.model.collapsed.clear();
                for record in &self.preferences.group_overrides {
                    if let Ok(g) = match_group(record, self.model.sensors()) {
                        if !record.expanded {
                            self.model.collapsed.insert(g);
                        }
                    }
                }
                self.settings.set_status("Reviewed unique matches applied for this source inventory. Uncertain matches remain expanded.".into());
                self.render();
            }
        }
    }
    fn save_group(&mut self, g: &GroupKey) {
        if self.busy || self.preview.is_some() {
            self.model.notice =
                "Group change is session-only while a settings operation is pending".into();
            return;
        }
        let mut members = Vec::new();
        let sensors = self.model.sensors();
        if let Some(indices) = self.model.groups().get(g) {
            for &i in indices {
                let s = &sensors[i];
                if s.identity.status != "best_effort" {
                    continue;
                }
                if let (Some(candidate), Some(measurement), Some(unit)) = (
                    &s.identity.candidate,
                    &s.identity.evidence.measurement,
                    &s.unit,
                ) {
                    let member = MemberSelector {
                        identity_schema_version: 1,
                        candidate_id: candidate.clone(),
                        measurement: measurement.clone(),
                        unit: unit.clone(),
                    };
                    let test = GroupOverride {
                        expanded: true,
                        members: vec![member.clone()],
                    };
                    if match_group(&test, sensors).is_ok() {
                        members.push(member);
                        if members.len() == 16 {
                            break;
                        }
                    }
                }
            }
        }
        if members.is_empty() {
            self.model.notice =
                "This device has no unique portable anchor; group state is session-only".into();
            return;
        }
        self.preferences
            .group_overrides
            .retain(|r| match_group(r, sensors).as_ref() != Ok(g));
        if self.preferences.group_overrides.len() >= 256 {
            self.model.notice =
                "Device preference limit reached; group state is session-only".into();
            return;
        }
        self.preferences.group_overrides.push(GroupOverride {
            expanded: !self.model.collapsed.contains(g),
            members,
        });
        self.autosave();
    }
    fn autosave(&mut self) {
        if self.saving != Saving::Replace && self.recovery {
            self.model.notice = "Unsaved: review configuration recovery in Settings first".into();
            return;
        }
        if self.reload_review_required {
            self.model.notice =
                "Unsaved: review reloaded preferences in Settings before automatic saving resumes"
                    .into();
        }
        let Some(preferences) = automatic_save_candidate(
            self.reload_review_required,
            self.saving,
            self.busy,
            &self.preferences,
            &mut self.pending_save,
        ) else {
            return;
        };
        self.submit(
            ConfigJob::Save {
                preferences,
                recover_original: false,
            },
            Saving::Automatic,
        );
    }
    fn submit(&mut self, job: ConfigJob, saving: Saving) -> bool {
        if self.busy {
            return false;
        }
        let reload = matches!(&job, ConfigJob::Reload);
        let result = self.config.as_ref().map(|c| c.submit(job));
        match result {
            Some(Ok(())) => {
                if reload {
                    // Reload adopts a new disk fingerprint, so no older in-memory
                    // snapshot may be automatically persisted under that authority.
                    self.reload_review_required = true;
                    self.pending_save = None;
                }
                self.busy = true;
                self.saving = saving;
                self.settings.set_busy(true);
                self.settings.set_status("Working…".into());
                let weak = self.weak.clone();
                self.config_timer.start(
                    TimerMode::Repeated,
                    Duration::from_millis(30),
                    move || {
                        if let Some(a) = weak.upgrade() {
                            a.borrow_mut().receive_config();
                        }
                    },
                );
                true
            }
            _ => {
                self.model.notice =
                    "Settings are unsaved: configuration storage is unavailable".into();
                self.settings.set_status(self.model.notice.clone().into());
                false
            }
        }
    }
    fn receive_config(&mut self) {
        let Some(result) = self.config.as_ref().and_then(ConfigWorker::try_recv) else {
            return;
        };
        self.config_timer.stop();
        self.busy = false;
        self.settings.set_busy(false);
        let saving = std::mem::replace(&mut self.saving, Saving::None);
        match result {
            ConfigResponse::Imported(r) | ConfigResponse::Backup(r) => match r {
                Ok(p) => self.preview_preferences(p),
                Err(e) => self.operation_error(e),
            },
            ConfigResponse::Saved(r) => {
                match apply_completed_save(
                    saving,
                    r,
                    &mut self.preferences,
                    &mut self.pending_save,
                    &mut self.reload_review_required,
                ) {
                    Ok(outcome) => {
                        self.recovery = false;
                        if outcome.replaced {
                            self.draft = self.preferences.clone();
                            self.model.collapsed.clear();
                            self.known_groups.clear();
                            self.apply_geometry();
                            self.populate_settings();
                        }
                        let message = outcome
                            .durability_warning
                            .unwrap_or_else(|| "Preferences saved".into());
                        self.settings.set_status(message.clone().into());
                        self.model.notice = message;
                        if saving == Saving::Replace && !self.preferences.group_overrides.is_empty()
                        {
                            self.model
                                .notice
                                .push_str(" · device group records await review");
                        }
                    }
                    Err(e) => self.operation_error(e),
                }
            }
            ConfigResponse::ExportPrepared(r) => match r {
                Ok(plan) => {
                    let text = format!(
                        "Export the current validated GUI preferences to:\n{}\n\n{}\nThe destination will be checked again when you confirm.",
                        plan.path.display(),
                        if plan.replaces_existing {
                            "This replaces the existing file."
                        } else {
                            "This creates a new file."
                        }
                    );
                    self.set_preview(Preview::Export(plan), text);
                }
                Err(e) => self.operation_error(e),
            },
            ConfigResponse::Exported(r) => match r {
                Ok(outcome) => self.settings.set_status(
                    outcome
                        .durability_warning
                        .unwrap_or_else(|| format!("Exported {}", outcome.path.display()))
                        .into(),
                ),
                Err(e) => self.operation_error(e),
            },
            ConfigResponse::Reloaded(r) => match r {
                Ok(report) => {
                    self.recovery = report.recovery.is_some();
                    self.preview_preferences(report.preferences);
                    self.model.notice =
                        "Reloaded preferences await confirmation; automatic saving is paused"
                            .into();
                    self.settings.set_status(self.model.notice.clone().into());
                }
                Err(e) => self.operation_error(e),
            },
        }
        self.render();
        if let Some(preferences) = self.pending_save.take() {
            self.submit(
                ConfigJob::Save {
                    preferences,
                    recover_original: false,
                },
                Saving::Automatic,
            );
        }
        if self.closing && !self.busy {
            let _ = slint::quit_event_loop();
        }
    }
    fn operation_error(&mut self, e: PreferenceError) {
        self.model.notice = format!("Unsaved / operation failed: {e}");
        self.settings.set_status(self.model.notice.clone().into());
    }
    pub fn tray_available(&self) -> bool {
        self.tray.as_ref().is_some_and(TrayHandle::is_available)
    }
    fn windows(&self) -> Windows {
        Windows {
            sensors: self.main.window().is_visible(),
            details: self.details.window().is_visible(),
            settings: self.settings.window().is_visible(),
        }
    }
    fn receive_tray(&mut self) {
        let Some(events) = self.tray.as_ref().map(TrayHandle::take_events) else {
            return;
        };
        // Quit wins over a concurrent host loss or queued activation.
        if events.actions.contains(&TrayAction::Quit) {
            self.lifecycle_event(Event::Quit);
            return;
        }
        if events.availability_changed {
            self.lifecycle_event(Event::Host(events.available));
        }
        self.tray_notice = events.diagnostic.unwrap_or_default();
        self.render();
        for action in events.actions {
            self.lifecycle_event(match action {
                TrayAction::OpenSensors => Event::OpenSensors,
                TrayAction::Settings => Event::Settings,
                TrayAction::Reconnect => Event::Reconnect,
                TrayAction::Quit => Event::Quit,
            });
        }
    }
    /// Routes tray and window actions through the same lifetime policy.
    pub fn lifecycle_event(&mut self, event: Event) {
        if self.closing {
            return;
        }
        let windows = self.windows();
        let minimize = self.preferences.behavior.minimize_to_tray;
        // Consult the worker's latest capability before a close, even if its UI wake is pending.
        if matches!(event, Event::CloseSensors | Event::SecondaryClosed) {
            let available = self.tray.as_ref().is_some_and(TrayHandle::is_available);
            let action = self
                .lifecycle
                .decide(Event::Host(available), minimize, windows);
            // Closing the last secondary window must exit when no usable tray remains.
            // Host loss received independently instead restores the hidden Sensors window.
            if action != Action::OpenSensors {
                self.execute_lifecycle(action);
            }
        }
        let action = self.lifecycle.decide(event, minimize, windows);
        self.execute_lifecycle(action);
    }
    fn execute_lifecycle(&mut self, action: Action) {
        match action {
            Action::None => {}
            Action::HideSensors => {
                self.capture_geometry();
                self.autosave();
                if let Err(e) = self.main.hide() {
                    self.model.notice = format!("Cannot hide Sensors: {e}");
                    self.request_close();
                }
            }
            Action::OpenSensors => {
                if let Err(e) = self.main.show() {
                    self.model.notice = format!("Cannot restore Sensors: {e}");
                    self.request_close();
                    return;
                }
                self.main.window().set_minimized(false);
                self.main.invoke_focus_table();
            }
            Action::Settings => self.open_settings(),
            Action::Reconnect => self.reconnect(),
            Action::Quit => self.request_close(),
        }
    }
    fn reconnect(&mut self) {
        if self.closing {
            return;
        }
        if let Some(c) = &self.client {
            c.reconnect();
        }
        self.connection_status = "Reconnecting";
        self.model.disconnect("Reconnecting…".into());
        self.render();
    }
    fn request_close(&mut self) {
        if self.closing {
            return;
        }
        self.closing = true;
        if let Some(c) = &self.client {
            c.stop();
        }
        self.expiry_timer.stop();
        self.capture_geometry();
        self.autosave();
        if !self.busy {
            let _ = slint::quit_event_loop();
        } else {
            self.model.notice = "Finishing the configuration transaction before closing…".into();
            self.main.set_notice(self.model.notice.clone().into());
        }
    }
    pub fn shutdown(&mut self) {
        self.geometry_timer.stop();
        self.clock_timer.stop();
        self.expiry_timer.stop();
        self.config_timer.stop();
        if let Some(mut tray) = self.tray.take() {
            if let Err(e) = tray.shutdown() {
                eprintln!("Tray shutdown: {e}");
            }
        }
        if let Some(c) = self.client.take() {
            c.stop();
            let _ = c.join();
        }
        if let Some(c) = self.config.take() {
            c.request_stop();
            let _ = c.join();
        }
    }
}

/// A full-profile replacement owns the active state until its transaction completes.
/// Ordinary saves coalesce later edits, but must never restore an earlier snapshot.
fn automatic_save_candidate(
    reload_review_required: bool,
    saving: Saving,
    busy: bool,
    preferences: &Preferences,
    pending: &mut Option<Preferences>,
) -> Option<Preferences> {
    if saving == Saving::Replace || reload_review_required {
        return None;
    }
    if busy {
        *pending = Some(preferences.clone());
        None
    } else {
        Some(preferences.clone())
    }
}

struct SaveCompletion {
    replaced: bool,
    durability_warning: Option<String>,
}

fn apply_completed_save(
    saving: Saving,
    outcome: Result<SaveOutcome, PreferenceError>,
    preferences: &mut Preferences,
    pending: &mut Option<Preferences>,
    reload_review_required: &mut bool,
) -> Result<SaveCompletion, PreferenceError> {
    let outcome = outcome?;
    let replaced = saving == Saving::Replace;
    if replaced {
        *preferences = outcome.preferences;
        // A pending snapshot belongs to the old profile and cannot follow the import.
        *pending = None;
        *reload_review_required = false;
    }
    Ok(SaveCompletion {
        replaced,
        durability_warning: outcome.durability_warning,
    })
}

fn bounded_evidence(text: &str, limit: usize) -> String {
    let visible = visible(text);
    let mut chars = visible.chars();
    let mut result: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}

fn columns(p: &Preferences) -> Vec<ColumnSpec> {
    p.table
        .column_order
        .iter()
        .map(|c| ColumnSpec {
            title: c.label().into(),
            index: ColumnId::ALL.iter().position(|x| x == c).unwrap_or(0) as i32,
            width: p.table.column_widths_px.get(*c) as f32,
        })
        .collect()
}
fn theme_index(t: Theme) -> i32 {
    match t {
        Theme::OledBlack => 0,
        Theme::Dark => 1,
        Theme::Light => 2,
    }
}
fn group_title(group: &GroupKey, sensor: Option<&crate::protocol::Sensor>) -> String {
    visible(&format!(
        "{} · {}",
        group
            .provider
            .as_deref()
            .or(sensor.map(|s| s.device.as_str()))
            .unwrap_or("Unknown source"),
        group.path.rsplit('/').next().unwrap_or("source")
    ))
}
fn update_rows(model: &VecModel<SensorRow>, rows: Vec<SensorRow>) {
    if model.row_count() != rows.len() {
        model.set_vec(rows);
    } else {
        for (index, row) in rows.into_iter().enumerate() {
            if model.row_data(index).as_ref() != Some(&row) {
                model.set_row_data(index, row);
            }
        }
    }
}
fn installed_fonts() -> BTreeSet<String> {
    // Fixed command and format: imported family names are never commands or file paths.
    std::process::Command::new("fc-list")
        .arg("--format=%{family}\n")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .flat_map(|s| s.split(','))
                .map(|s| s.trim().to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

fn logical_dimensions(window: &slint::Window) -> (u32, u32) {
    let size = window.size().to_logical(window.scale_factor());
    (size.width.round() as u32, size.height.round() as u32)
}

#[cfg(test)]
mod save_ordering_tests {
    use super::*;

    fn with_theme(theme: Theme) -> Preferences {
        let mut p = Preferences::default();
        p.appearance.theme = theme;
        p
    }

    fn committed(preferences: Preferences) -> Result<SaveOutcome, PreferenceError> {
        Ok(SaveOutcome {
            preferences,
            durability_warning: None,
        })
    }

    #[test]
    fn automatic_completion_preserves_newest_live_and_queued_edits() {
        let first = Preferences::default();
        let mut live = with_theme(Theme::Dark);
        let mut pending = None;
        assert!(
            automatic_save_candidate(false, Saving::Automatic, true, &live, &mut pending).is_none()
        );
        live.appearance.theme = Theme::Light;
        assert!(
            automatic_save_candidate(false, Saving::Automatic, true, &live, &mut pending).is_none()
        );
        let completion = apply_completed_save(
            Saving::Automatic,
            committed(first),
            &mut live,
            &mut pending,
            &mut false,
        )
        .unwrap();
        assert!(!completion.replaced);
        assert_eq!(live.appearance.theme, Theme::Light);
        assert_eq!(pending, Some(live));
    }

    #[test]
    fn replacement_ignores_old_close_autosave_and_discards_old_pending_snapshot() {
        let mut live = with_theme(Theme::Dark);
        let old = live.clone();
        let incoming = with_theme(Theme::Light);
        let mut pending = Some(old.clone());
        assert!(
            automatic_save_candidate(false, Saving::Replace, true, &live, &mut pending).is_none()
        );
        assert_eq!(pending, Some(old));
        let completion = apply_completed_save(
            Saving::Replace,
            committed(incoming.clone()),
            &mut live,
            &mut pending,
            &mut false,
        )
        .unwrap();
        assert!(completion.replaced);
        assert_eq!(live, incoming);
        assert!(pending.is_none());
        assert_eq!(
            automatic_save_candidate(false, Saving::None, false, &live, &mut pending),
            Some(incoming)
        );
    }

    #[test]
    fn failed_replacement_does_not_partially_apply_or_lose_prior_edits() {
        let mut live = with_theme(Theme::Dark);
        let before = live.clone();
        let mut pending = Some(before.clone());
        let failure = Err(PreferenceError {
            message: "configuration changed on disk".into(),
        });
        assert!(
            apply_completed_save(
                Saving::Replace,
                failure,
                &mut live,
                &mut pending,
                &mut false
            )
            .is_err()
        );
        assert_eq!(live, before);
        assert_eq!(pending, Some(before));
    }

    #[test]
    fn post_commit_durability_warning_still_applies_replacement() {
        let mut live = Preferences::default();
        let incoming = with_theme(Theme::Light);
        let mut pending = Some(live.clone());
        let outcome = Ok(SaveOutcome {
            preferences: incoming.clone(),
            durability_warning: Some("directory sync failed".into()),
        });
        let completion = apply_completed_save(
            Saving::Replace,
            outcome,
            &mut live,
            &mut pending,
            &mut false,
        )
        .unwrap();
        assert!(completion.replaced);
        assert!(completion.durability_warning.is_some());
        assert_eq!(live, incoming);
        assert!(pending.is_none());
    }

    #[test]
    fn reload_fence_blocks_old_geometry_autosave_during_io_and_after_preview() {
        let mut live = with_theme(Theme::Dark);
        let mut pending = None;
        let mut reload_review_required = true;
        for busy in [true, false] {
            assert!(
                automatic_save_candidate(
                    reload_review_required,
                    Saving::None,
                    busy,
                    &live,
                    &mut pending
                )
                .is_none()
            );
            assert!(pending.is_none());
        }
        // A cancelled preview leaves the fence in place; an explicit replacement
        // which fails must also retain it, even though Store adopted the fingerprint.
        let failed = Err(PreferenceError {
            message: "replacement failed".into(),
        });
        assert!(
            apply_completed_save(
                Saving::Replace,
                failed,
                &mut live,
                &mut pending,
                &mut reload_review_required
            )
            .is_err()
        );
        assert!(reload_review_required);
        assert_eq!(live.appearance.theme, Theme::Dark);
        let incoming = with_theme(Theme::Light);
        apply_completed_save(
            Saving::Replace,
            committed(incoming.clone()),
            &mut live,
            &mut pending,
            &mut reload_review_required,
        )
        .unwrap();
        assert!(!reload_review_required);
        assert_eq!(live, incoming);
        assert_eq!(
            automatic_save_candidate(
                reload_review_required,
                Saving::None,
                false,
                &live,
                &mut pending
            ),
            Some(incoming)
        );
    }

    #[test]
    fn evidence_is_bounded_plain_text_with_visible_truncation() {
        assert_eq!(
            bounded_evidence("device\n\u{202e}", 128),
            "device\\u{a}\\u{202e}"
        );
        assert_eq!(bounded_evidence("αβγδ", 3), "αβγ…");
    }
}
