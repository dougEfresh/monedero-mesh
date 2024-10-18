use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::app::ExternalMsg;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum HelpMenuLine {
    KeyMap(String, Vec<String>, String),
    Paragraph(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    #[serde(default)]
    pub help: Option<String>,

    #[serde(default)]
    pub messages: Vec<ExternalMsg>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyBindings {
    #[serde(default)]
    pub on_key: BTreeMap<String, Action>,

    #[serde(default)]
    pub on_alphabet: Option<Action>,

    #[serde(default)]
    pub on_number: Option<Action>,

    #[serde(default)]
    pub on_character: Option<Action>,

    #[serde(default)]
    pub on_navigation: Option<Action>,

    #[serde(default)]
    pub on_function: Option<Action>,

    #[serde(default)]
    pub on_alphanumeric: Option<Action>,

    #[serde(default)]
    pub on_special_character: Option<Action>,

    #[serde(default)]
    pub default: Option<Action>,
    // Checklist for adding new field:
    // - [ ] Update App::handle_key
    // - [ ] Update KeyBindings::sanitized
    // - [ ] Update Mode::help_menu
    // - [ ] Update configure-key-bindings.md
    // - [ ] Update debug-key-bindings.md
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut on_keys: BTreeMap<String, Action> = BTreeMap::new();
        on_keys.insert(
            "q".to_string(),
            Action {
                help: Some("quit".to_string()),
                messages: vec![ExternalMsg::Quit],
            },
        );
        Self {
            on_key: on_keys,
            on_alphabet: None,
            on_number: None,
            on_character: None,
            on_navigation: None,
            on_alphanumeric: None,
            default: None,
            on_special_character: None,
            on_function: None,
        }
    }
}

impl Action {
    pub fn sanitized(self, read_only: bool) -> Option<Self> {
        if self.messages.is_empty() {
            None
        } else if read_only {
            if self.messages.iter().all(ExternalMsg::is_read_only) {
                Some(self)
            } else {
                None
            }
        } else {
            Some(self)
        }
    }
}

impl KeyBindings {
    pub fn sanitized(mut self, read_only: bool) -> Self {
        if read_only {
            self.on_key = self
                .on_key
                .into_iter()
                .filter_map(|(k, a)| a.sanitized(read_only).map(|a| (k, a)))
                .collect();

            self.on_alphabet = self.on_alphabet.and_then(|a| a.sanitized(read_only));
            self.on_number = self.on_number.and_then(|a| a.sanitized(read_only));
            self.on_alphanumeric = self.on_alphanumeric.and_then(|a| a.sanitized(read_only));
            self.on_character = self.on_character.and_then(|a| a.sanitized(read_only));
            self.on_navigation = self.on_navigation.and_then(|a| a.sanitized(read_only));
            self.default = self.default.and_then(|a| a.sanitized(read_only));
        };
        self
    }

    pub fn extend(mut self, other: Self) -> Self {
        self.on_key.extend(other.on_key);
        self.on_alphabet = other.on_alphabet.or(self.on_alphabet);
        self.on_number = other.on_number.or(self.on_number);
        self.on_alphanumeric = other.on_alphanumeric.or(self.on_alphanumeric);
        self.on_character = other.on_character.or(self.on_character);
        self.on_navigation = other.on_navigation.or(self.on_navigation);
        self.default = other.default.or(self.default);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub help: Option<String>,

    #[serde(default)]
    pub extra_help: Option<String>,

    #[serde(default)]
    pub key_bindings: KeyBindings,

    #[serde(default)]
    pub layout: crate::ui::AppLayout,

    #[serde(default)]
    pub prompt: Option<String>,
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            help: None,
            extra_help: None,
            key_bindings: Default::default(),
            layout: Default::default(),
            prompt: None,
        }
    }
}

impl Mode {
    pub fn sanitized(mut self, read_only: bool, global_key_bindings: KeyBindings) -> Self {
        self.key_bindings = global_key_bindings
            .sanitized(read_only)
            .extend(self.key_bindings.sanitized(read_only));
        self
    }

    pub fn help_menu(&self) -> Vec<HelpMenuLine> {
        let extra_help_lines = self.extra_help.clone().map(|e| {
            e.lines()
                .map(|l| HelpMenuLine::Paragraph(l.into()))
                .collect::<Vec<HelpMenuLine>>()
        });

        self.help
            .clone()
            .map(|h| {
                h.lines()
                    .map(|l| HelpMenuLine::Paragraph(l.into()))
                    .collect()
            })
            .unwrap_or_else(|| {
                let lines = extra_help_lines
                    .unwrap_or_default()
                    .into_iter()
                    .chain(self.key_bindings.on_key.iter().filter_map(|(k, a)| {
                        let remaps = self
                            .key_bindings
                            .on_key
                            .iter()
                            .filter_map(|(rk, ra)| {
                                if rk == k {
                                    None
                                } else if a == ra {
                                    Some(rk.clone())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<String>>();
                        a.help
                            .clone()
                            .map(|h| HelpMenuLine::KeyMap(k.into(), remaps, h))
                    }))
                    .chain(
                        self.key_bindings
                            .on_alphabet
                            .iter()
                            .map(|a| ("[a-Z]", a.help.clone()))
                            .filter_map(|(k, mh)| {
                                mh.map(|h| HelpMenuLine::KeyMap(k.into(), vec![], h))
                            }),
                    )
                    .chain(
                        self.key_bindings
                            .on_number
                            .iter()
                            .map(|a| ("[0-9]", a.help.clone()))
                            .filter_map(|(k, mh)| {
                                mh.map(|h| HelpMenuLine::KeyMap(k.into(), vec![], h))
                            }),
                    )
                    .chain(
                        self.key_bindings
                            .on_alphanumeric
                            .iter()
                            .map(|a| ("[0-Z]", a.help.clone()))
                            .filter_map(|(k, mh)| {
                                mh.map(|h| HelpMenuLine::KeyMap(k.into(), vec![], h))
                            }),
                    )
                    .chain(
                        self.key_bindings
                            .on_character
                            .iter()
                            .map(|a| ("[*]", a.help.clone()))
                            .filter_map(|(k, mh)| {
                                mh.map(|h| HelpMenuLine::KeyMap(k.into(), vec![], h))
                            }),
                    )
                    .chain(
                        self.key_bindings
                            .on_navigation
                            .iter()
                            .map(|a| ("[nav]", a.help.clone()))
                            .filter_map(|(k, mh)| {
                                mh.map(|h| HelpMenuLine::KeyMap(k.into(), vec![], h))
                            }),
                    )
                    .chain(
                        self.key_bindings
                            .default
                            .iter()
                            .map(|a| ("[default]", a.help.clone()))
                            .filter_map(|(k, mh)| {
                                mh.map(|h| HelpMenuLine::KeyMap(k.into(), vec![], h))
                            }),
                    );

                let mut remapped = HashSet::new();
                let mut result = vec![];

                for line in lines {
                    match line {
                        HelpMenuLine::Paragraph(p) => result.push(HelpMenuLine::Paragraph(p)),
                        HelpMenuLine::KeyMap(k, r, d) => {
                            if !remapped.contains(&k) {
                                for k in r.iter() {
                                    remapped.insert(k.clone());
                                }
                                result.push(HelpMenuLine::KeyMap(k, r, d));
                            }
                        }
                    }
                }

                result
            })
    }
}
