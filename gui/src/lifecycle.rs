//! Window lifetime policy, independent of desktop or hardware access.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Windows {
    pub sensors: bool,
    pub details: bool,
    pub settings: bool,
}
impl Windows {
    fn any(self) -> bool {
        self.sensors || self.details || self.settings
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Event {
    CloseSensors,
    SecondaryClosed,
    Host(bool),
    OpenSensors,
    Settings,
    Reconnect,
    Quit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    HideSensors,
    OpenSensors,
    Settings,
    Reconnect,
    Quit,
}
#[derive(Default)]
pub struct Lifecycle {
    available: bool,
    quitting: bool,
}
impl Lifecycle {
    pub fn decide(&mut self, event: Event, minimize: bool, windows: Windows) -> Action {
        if self.quitting {
            return Action::None;
        }
        let action = match event {
            Event::Host(available) => {
                self.available = available;
                if !available && !windows.any() {
                    Action::OpenSensors
                } else {
                    Action::None
                }
            }
            Event::CloseSensors if !minimize => Action::Quit,
            Event::CloseSensors => {
                if self.available || windows.details || windows.settings {
                    Action::HideSensors
                } else {
                    Action::Quit
                }
            }
            Event::SecondaryClosed => {
                if !(windows.any() || minimize && self.available) {
                    Action::Quit
                } else {
                    Action::None
                }
            }
            Event::OpenSensors => Action::OpenSensors,
            Event::Settings => Action::Settings,
            Event::Reconnect => Action::Reconnect,
            Event::Quit => Action::Quit,
        };
        if action == Action::Quit {
            self.quitting = true;
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_close_exits_even_with_host_or_other_windows() {
        for host in [false, true] {
            for details in [false, true] {
                let mut life = Lifecycle::default();
                let windows = Windows {
                    sensors: true,
                    details,
                    settings: true,
                };
                life.decide(Event::Host(host), false, windows);
                assert_eq!(
                    life.decide(Event::CloseSensors, false, windows),
                    Action::Quit
                );
                for event in [
                    Event::OpenSensors,
                    Event::Reconnect,
                    Event::Settings,
                    Event::Quit,
                ] {
                    assert_eq!(life.decide(event, false, windows), Action::None);
                }
            }
        }
    }

    #[test]
    fn no_host_last_window_exits_and_host_loss_restores() {
        let mut life = Lifecycle::default();
        let visible = Windows {
            sensors: true,
            ..Windows::default()
        };
        assert_eq!(
            life.decide(Event::CloseSensors, true, visible),
            Action::Quit
        );
        let mut life = Lifecycle::default();
        life.decide(Event::Host(true), true, visible);
        assert_eq!(
            life.decide(Event::CloseSensors, true, visible),
            Action::HideSensors
        );
        assert_eq!(
            life.decide(Event::Host(false), true, Windows::default()),
            Action::OpenSensors
        );
        assert_eq!(
            life.decide(Event::CloseSensors, true, visible),
            Action::Quit
        );
    }

    #[test]
    fn secondary_window_and_preference_change_cannot_strand_process() {
        let mut life = Lifecycle::default();
        let windows = Windows {
            sensors: true,
            settings: true,
            details: false,
        };
        assert_eq!(
            life.decide(Event::CloseSensors, true, windows),
            Action::HideSensors
        );
        assert_eq!(
            life.decide(Event::SecondaryClosed, true, Windows::default()),
            Action::Quit
        );
        let mut life = Lifecycle::default();
        life.decide(Event::Host(true), true, windows);
        assert_eq!(
            life.decide(Event::SecondaryClosed, false, Windows::default()),
            Action::Quit
        );
    }

    #[test]
    fn restore_and_reconnect_only_request_existing_resources() {
        let mut life = Lifecycle::default();
        let visible = Windows {
            sensors: true,
            ..Windows::default()
        };
        life.decide(Event::Host(true), true, visible);
        assert_eq!(
            life.decide(Event::CloseSensors, true, visible),
            Action::HideSensors
        );
        for windows in [Windows::default(), visible, visible] {
            assert_eq!(
                life.decide(Event::OpenSensors, true, windows),
                Action::OpenSensors
            );
            assert_eq!(
                life.decide(Event::Reconnect, true, windows),
                Action::Reconnect
            );
        }
        assert_eq!(life.decide(Event::Quit, true, visible), Action::Quit);
        assert_eq!(
            life.decide(Event::Host(false), true, Windows::default()),
            Action::None
        );
    }

    #[test]
    fn every_close_combination_has_a_visible_window_tray_or_exit() {
        for minimize in [false, true] {
            for available in [false, true] {
                for details in [false, true] {
                    for settings in [false, true] {
                        let mut life = Lifecycle::default();
                        let windows = Windows {
                            sensors: true,
                            details,
                            settings,
                        };
                        life.decide(Event::Host(available), minimize, windows);
                        let action = life.decide(Event::CloseSensors, minimize, windows);
                        assert!(
                            action == Action::Quit
                                || details
                                || settings
                                || (minimize && available)
                        );
                    }
                }
            }
        }
    }
}
