use std::collections::HashMap;

use anyhow::{Context, Result};
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;

use crate::config::Binding;

pub struct Registered {
    pub hotkey: HotKey,
    pub binding_index: usize,
}

#[derive(Debug, Clone)]
pub struct BindingError {
    pub index: usize,
    pub message: String,
}

pub struct Manager {
    manager: GlobalHotKeyManager,
    bindings: HashMap<u32, Registered>,
}

impl Manager {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("create hotkey manager")?;
        Ok(Self {
            manager,
            bindings: HashMap::new(),
        })
    }

    pub fn set_bindings(&mut self, list: &[Binding]) -> Vec<BindingError> {
        for (_, reg) in self.bindings.drain() {
            let _ = self.manager.unregister(reg.hotkey);
        }

        let mut errors = Vec::new();
        let mut seen: HashMap<u32, usize> = HashMap::new();

        for (index, binding) in list.iter().enumerate() {
            let trimmed = binding.key.trim();
            if trimmed.is_empty() {
                continue;
            }
            let hotkey: HotKey = match trimmed.parse() {
                Ok(h) => h,
                Err(e) => {
                    errors.push(BindingError {
                        index,
                        message: format!("invalid hotkey '{trimmed}': {e}"),
                    });
                    continue;
                }
            };
            if let Some(prev) = seen.get(&hotkey.id()) {
                errors.push(BindingError {
                    index,
                    message: format!("duplicate of binding #{prev}"),
                });
                continue;
            }
            if let Err(e) = self.manager.register(hotkey) {
                errors.push(BindingError {
                    index,
                    message: format!("register failed: {e}"),
                });
                continue;
            }
            seen.insert(hotkey.id(), index);
            self.bindings.insert(
                hotkey.id(),
                Registered {
                    hotkey,
                    binding_index: index,
                },
            );
            tracing::info!("registered binding #{} '{}' ({})", index, binding.label, trimmed);
        }

        if !errors.is_empty() {
            tracing::warn!(
                "{} of {} binding(s) failed to register",
                errors.len(),
                list.len()
            );
        }
        errors
    }

    pub fn binding_index_for(&self, id: u32) -> Option<usize> {
        self.bindings.get(&id).map(|b| b.binding_index)
    }
}
