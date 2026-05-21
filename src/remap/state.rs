//! Pure tap-vs-hold state machine for the caps-lock dual-function remap.
//!
//! The hook layer feeds events in (`HookEvent`) and the FSM returns a list of
//! `Effect`s describing what should happen to the raw event and what synthetic
//! events to inject. No Win32 imports — this file is exercised entirely by the
//! unit tests below.

use crate::config::{CapsLockRemap, RemapMode};

/// Time source, in milliseconds. Wall-clock origin doesn't matter; only
/// monotonic differences are used by the FSM.
pub type Millis = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    /// A non-caps key was pressed.
    KeyDown { vk: u32, timestamp_ms: Millis },
    /// A non-caps key was released.
    KeyUp { vk: u32, timestamp_ms: Millis },
    /// Caps Lock was pressed.
    CapsDown { timestamp_ms: Millis },
    /// Caps Lock was released.
    CapsUp { timestamp_ms: Millis },
    /// The pending-tap timer elapsed.
    Timeout { timestamp_ms: Millis },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Drop the originating event from the hook chain.
    Suppress,
    /// Let the originating event continue down the hook chain unchanged.
    PassThrough,
    /// Synthesise a key event.
    Inject { vk: u32, down: bool },
    /// Ask the runtime to schedule a `HookEvent::Timeout` after `ms`.
    ScheduleTimeout { ms: u64 },
    /// Cancel any pending timeout.
    CancelTimeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum State {
    #[default]
    Idle,
    /// Caps is held; we're waiting to decide tap vs hold.
    Pending { down_at_ms: Millis },
    /// We've committed to the "hold" key being active.
    HoldActive,
}

/// Coarse-grained variant of [`State`] without the inner data, exposed so the
/// hook callback can fast-path the common Idle case without taking the lock
/// long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Idle,
    Pending,
    HoldActive,
}

#[derive(Debug, Default)]
pub struct Machine {
    state: State,
}

impl Machine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state_kind(&self) -> StateKind {
        match self.state {
            State::Idle => StateKind::Idle,
            State::Pending { .. } => StateKind::Pending,
            State::HoldActive => StateKind::HoldActive,
        }
    }

    /// Returns the list of effects produced by an event under the given config.
    pub fn handle(&mut self, ev: HookEvent, cfg: &CapsLockRemap) -> Vec<Effect> {
        match cfg.mode {
            RemapMode::Off => vec![Effect::PassThrough],
            RemapMode::Single => self.handle_single(ev, cfg),
            RemapMode::Dual => self.handle_dual(ev, cfg),
        }
    }

    fn handle_single(&mut self, ev: HookEvent, cfg: &CapsLockRemap) -> Vec<Effect> {
        let Some(target_vk) = vk_for(&cfg.single_to.0) else {
            return vec![Effect::PassThrough];
        };
        match ev {
            HookEvent::CapsDown { .. } => vec![
                Effect::Suppress,
                Effect::Inject {
                    vk: target_vk,
                    down: true,
                },
            ],
            HookEvent::CapsUp { .. } => vec![
                Effect::Suppress,
                Effect::Inject {
                    vk: target_vk,
                    down: false,
                },
            ],
            _ => vec![Effect::PassThrough],
        }
    }

    fn handle_dual(&mut self, ev: HookEvent, cfg: &CapsLockRemap) -> Vec<Effect> {
        let tap_vk = vk_for(&cfg.tap.0);
        let hold_vk = vk_for(&cfg.hold.0);

        match (&self.state.clone(), ev) {
            // Caps pressed while idle — start the pending window.
            (State::Idle, HookEvent::CapsDown { timestamp_ms }) => {
                self.state = State::Pending {
                    down_at_ms: timestamp_ms,
                };
                vec![
                    Effect::Suppress,
                    Effect::ScheduleTimeout { ms: 0 }, // runtime fills with cfg tap_timeout
                ]
            }
            // Pending → another key arrives: commit to hold.
            (State::Pending { .. }, HookEvent::KeyDown { vk, .. }) => {
                let mut out = vec![Effect::CancelTimeout];
                if let Some(hold) = hold_vk {
                    out.push(Effect::Inject {
                        vk: hold,
                        down: true,
                    });
                }
                out.push(Effect::Inject { vk, down: true });
                out.push(Effect::Suppress);
                self.state = State::HoldActive;
                out
            }
            // Pending → caps released before timeout: it was a tap.
            (State::Pending { .. }, HookEvent::CapsUp { .. }) => {
                self.state = State::Idle;
                let mut out = vec![Effect::CancelTimeout, Effect::Suppress];
                if let Some(tap) = tap_vk {
                    out.push(Effect::Inject {
                        vk: tap,
                        down: true,
                    });
                    out.push(Effect::Inject {
                        vk: tap,
                        down: false,
                    });
                }
                out
            }
            // Pending → timeout fired: caps is now the hold modifier.
            (State::Pending { .. }, HookEvent::Timeout { .. }) => {
                self.state = State::HoldActive;
                if let Some(hold) = hold_vk {
                    vec![Effect::Inject {
                        vk: hold,
                        down: true,
                    }]
                } else {
                    vec![]
                }
            }
            // Hold active → caps released: drop the modifier.
            (State::HoldActive, HookEvent::CapsUp { .. }) => {
                self.state = State::Idle;
                let mut out = vec![Effect::Suppress];
                if let Some(hold) = hold_vk {
                    out.push(Effect::Inject {
                        vk: hold,
                        down: false,
                    });
                }
                out
            }
            // Idle and not caps: pass through.
            (State::Idle, _) => vec![Effect::PassThrough],
            // Defensive: hold-active + other events pass through.
            (State::HoldActive, _) => vec![Effect::PassThrough],
            // Defensive: pending state on KeyUp / other Timeout — pass through.
            (State::Pending { .. }, _) => vec![Effect::PassThrough],
        }
    }
}

fn vk_for(name: &str) -> Option<u32> {
    super::vk::parse(name).map(|v| v.0 as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KeyName, RemapMode};

    fn cfg_dual() -> CapsLockRemap {
        CapsLockRemap {
            mode: RemapMode::Dual,
            single_to: KeyName::new("left_ctrl"),
            tap: KeyName::new("escape"),
            hold: KeyName::new("left_ctrl"),
        }
    }

    fn cfg_single() -> CapsLockRemap {
        CapsLockRemap {
            mode: RemapMode::Single,
            single_to: KeyName::new("left_ctrl"),
            tap: KeyName::new("escape"),
            hold: KeyName::new("left_ctrl"),
        }
    }

    fn cfg_off() -> CapsLockRemap {
        CapsLockRemap {
            mode: RemapMode::Off,
            ..cfg_dual()
        }
    }

    fn vk_escape() -> u32 {
        super::vk_for("escape").unwrap()
    }

    fn vk_ctrl() -> u32 {
        super::vk_for("left_ctrl").unwrap()
    }

    #[test]
    fn off_is_passthrough() {
        let mut m = Machine::new();
        let out = m.handle(HookEvent::CapsDown { timestamp_ms: 0 }, &cfg_off());
        assert_eq!(out, vec![Effect::PassThrough]);
    }

    #[test]
    fn single_remaps_down_and_up() {
        let mut m = Machine::new();
        let down = m.handle(HookEvent::CapsDown { timestamp_ms: 0 }, &cfg_single());
        assert_eq!(
            down,
            vec![
                Effect::Suppress,
                Effect::Inject {
                    vk: vk_ctrl(),
                    down: true
                }
            ]
        );
        let up = m.handle(HookEvent::CapsUp { timestamp_ms: 5 }, &cfg_single());
        assert_eq!(
            up,
            vec![
                Effect::Suppress,
                Effect::Inject {
                    vk: vk_ctrl(),
                    down: false
                }
            ]
        );
    }

    #[test]
    fn dual_tap_sends_tap_key() {
        let mut m = Machine::new();
        let _ = m.handle(HookEvent::CapsDown { timestamp_ms: 0 }, &cfg_dual());
        let out = m.handle(HookEvent::CapsUp { timestamp_ms: 50 }, &cfg_dual());
        assert_eq!(
            out,
            vec![
                Effect::CancelTimeout,
                Effect::Suppress,
                Effect::Inject {
                    vk: vk_escape(),
                    down: true
                },
                Effect::Inject {
                    vk: vk_escape(),
                    down: false
                },
            ]
        );
    }

    #[test]
    fn dual_other_key_commits_to_hold() {
        let mut m = Machine::new();
        let _ = m.handle(HookEvent::CapsDown { timestamp_ms: 0 }, &cfg_dual());
        let out = m.handle(
            HookEvent::KeyDown {
                vk: 0x43, /* 'C' */
                timestamp_ms: 10,
            },
            &cfg_dual(),
        );
        assert_eq!(
            out,
            vec![
                Effect::CancelTimeout,
                Effect::Inject {
                    vk: vk_ctrl(),
                    down: true
                },
                Effect::Inject { vk: 0x43, down: true },
                Effect::Suppress,
            ]
        );
    }

    #[test]
    fn dual_timeout_commits_to_hold() {
        let mut m = Machine::new();
        let _ = m.handle(HookEvent::CapsDown { timestamp_ms: 0 }, &cfg_dual());
        let out = m.handle(HookEvent::Timeout { timestamp_ms: 180 }, &cfg_dual());
        assert_eq!(
            out,
            vec![Effect::Inject {
                vk: vk_ctrl(),
                down: true
            }]
        );
    }

    #[test]
    fn dual_caps_release_after_hold_drops_modifier() {
        let mut m = Machine::new();
        let _ = m.handle(HookEvent::CapsDown { timestamp_ms: 0 }, &cfg_dual());
        let _ = m.handle(HookEvent::Timeout { timestamp_ms: 180 }, &cfg_dual());
        let out = m.handle(HookEvent::CapsUp { timestamp_ms: 250 }, &cfg_dual());
        assert_eq!(
            out,
            vec![
                Effect::Suppress,
                Effect::Inject {
                    vk: vk_ctrl(),
                    down: false
                },
            ]
        );
    }
}
