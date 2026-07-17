//! Disposable guest proving typed, dynamically-instantiated facet resources.

wit_bindgen::generate!({
    path: "../wit",
    world: "provider-world",
    generate_all,
});

use exports::litehouse::esphome_resource_spike::{color_provider, level_provider, on_off_provider};
use litehouse::esphome_resource_spike::model;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

static NODE: OnceLock<Arc<NodeState>> = OnceLock::new();

#[derive(Debug)]
struct NodeState {
    switch_a: AtomicBool,
    switch_b: AtomicBool,
    led_on: AtomicBool,
    led_level: AtomicU8,
    led_rgb: Mutex<model::Rgb>,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            switch_a: AtomicBool::new(false),
            switch_b: AtomicBool::new(true),
            led_on: AtomicBool::new(false),
            led_level: AtomicU8::new(64),
            led_rgb: Mutex::new(model::Rgb {
                red: 16,
                green: 32,
                blue: 64,
            }),
        }
    }
}

fn node() -> Arc<NodeState> {
    Arc::clone(NODE.get_or_init(|| Arc::new(NodeState::default())))
}

#[derive(Debug, Clone, Copy)]
enum OnOffSlot {
    SwitchA,
    SwitchB,
    Led,
}

struct OnOffEndpoint {
    key: &'static str,
    slot: OnOffSlot,
    node: Arc<NodeState>,
}

impl on_off_provider::GuestEndpoint for OnOffEndpoint {
    fn key(&self) -> String {
        self.key.into()
    }

    fn query(&self) -> model::OnOffState {
        let current = match self.slot {
            OnOffSlot::SwitchA => self.node.switch_a.load(Ordering::Relaxed),
            OnOffSlot::SwitchB => self.node.switch_b.load(Ordering::Relaxed),
            OnOffSlot::Led => self.node.led_on.load(Ordering::Relaxed),
        };
        model::OnOffState { current }
    }

    fn set(&self, value: bool) {
        match self.slot {
            OnOffSlot::SwitchA => self.node.switch_a.store(value, Ordering::Relaxed),
            OnOffSlot::SwitchB => self.node.switch_b.store(value, Ordering::Relaxed),
            OnOffSlot::Led => self.node.led_on.store(value, Ordering::Relaxed),
        }
    }
}

struct LevelEndpoint {
    node: Arc<NodeState>,
}

impl level_provider::GuestEndpoint for LevelEndpoint {
    fn key(&self) -> String {
        "led-brightness".into()
    }

    fn query(&self) -> model::LevelState {
        model::LevelState {
            current: self.node.led_level.load(Ordering::Relaxed),
        }
    }

    fn set(&self, value: u8) {
        self.node.led_level.store(value, Ordering::Relaxed);
        self.node.led_on.store(value != 0, Ordering::Relaxed);
    }
}

struct ColorEndpoint {
    node: Arc<NodeState>,
}

impl color_provider::GuestEndpoint for ColorEndpoint {
    fn key(&self) -> String {
        "led-rgb".into()
    }

    fn query(&self) -> model::Rgb {
        *self
            .node
            .led_rgb
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn set(&self, value: model::Rgb) {
        let illuminated = value.red != 0 || value.green != 0 || value.blue != 0;
        *self
            .node
            .led_rgb
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = value;
        self.node.led_on.store(illuminated, Ordering::Relaxed);
    }
}

struct Provider;

impl on_off_provider::Guest for Provider {
    type Endpoint = OnOffEndpoint;

    fn discover() -> Vec<on_off_provider::Endpoint> {
        let node = node();
        [
            ("switch-a", OnOffSlot::SwitchA),
            ("switch-b", OnOffSlot::SwitchB),
            ("led-power", OnOffSlot::Led),
        ]
        .into_iter()
        .map(|(key, slot)| {
            on_off_provider::Endpoint::new(OnOffEndpoint {
                key,
                slot,
                node: Arc::clone(&node),
            })
        })
        .collect()
    }
}

impl level_provider::Guest for Provider {
    type Endpoint = LevelEndpoint;

    fn discover() -> Vec<level_provider::Endpoint> {
        vec![level_provider::Endpoint::new(LevelEndpoint {
            node: node(),
        })]
    }
}

impl color_provider::Guest for Provider {
    type Endpoint = ColorEndpoint;

    fn discover() -> Vec<color_provider::Endpoint> {
        vec![color_provider::Endpoint::new(ColorEndpoint {
            node: node(),
        })]
    }
}

export!(Provider);
