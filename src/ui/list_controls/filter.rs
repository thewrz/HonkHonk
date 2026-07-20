/// Whether a view may claim otherwise-unhandled printable keypresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    TypeToFilter,
    ClickOnly,
}

/// The active view's type-to-filter policy and current blocking-layer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivationContext {
    activation: Activation,
    blocking_layer_visible: bool,
}

impl ActivationContext {
    pub const fn new(activation: Activation, blocking_layer_visible: bool) -> Self {
        Self {
            activation,
            blocking_layer_visible,
        }
    }

    pub const fn allows_typing(self) -> bool {
        matches!(self.activation, Activation::TypeToFilter) && !self.blocking_layer_visible
    }
}

/// Transient query state shared by filterable views.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterState {
    query: String,
    had_focus: bool,
}

impl FilterState {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub const fn had_focus(&self) -> bool {
        self.had_focus
    }

    pub fn replace(&mut self, query: String) {
        self.query = query;
        self.had_focus = true;
    }

    pub fn insert(&mut self, text: &str) {
        self.query.push_str(text);
        self.had_focus = true;
    }

    pub fn escape(&mut self) {
        if self.had_focus {
            self.had_focus = false;
        } else {
            self.query.clear();
        }
    }
}

/// Returns items with at least one supplied field containing `query`.
///
/// Matching is case-insensitive and stable: input order is always preserved.
pub fn filter_items<'a, T, F, I, S>(items: &'a [T], query: &str, haystacks: F) -> Vec<&'a T>
where
    F: Fn(&'a T) -> I,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if query.is_empty() {
        return items.iter().collect();
    }

    let needle = query.to_lowercase();
    items
        .iter()
        .filter(|item| {
            haystacks(item)
                .into_iter()
                .any(|field| field.as_ref().to_lowercase().contains(&needle))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Item {
        name: &'static str,
        filename: &'static str,
        category: &'static str,
    }

    const ITEMS: [Item; 3] = [
        Item {
            name: "Angry Goose",
            filename: "goose_honk.WAV",
            category: "Animals",
        },
        Item {
            name: "Vine Boom",
            filename: "vine_boom.mp3",
            category: "Memes",
        },
        Item {
            name: "Applause",
            filename: "crowd.ogg",
            category: "Reactions",
        },
    ];

    fn fields(item: &Item) -> [&str; 3] {
        [item.name, item.filename, item.category]
    }

    #[test]
    fn empty_query_preserves_every_item_in_order() {
        let filtered = filter_items(&ITEMS, "", fields);
        assert_eq!(filtered, ITEMS.iter().collect::<Vec<_>>());
    }

    #[test]
    fn matching_is_case_insensitive_across_all_haystacks() {
        assert_eq!(filter_items(&ITEMS, "GOOSE", fields), vec![&ITEMS[0]]);
        assert_eq!(filter_items(&ITEMS, ".MP3", fields), vec![&ITEMS[1]]);
        assert_eq!(filter_items(&ITEMS, "reactions", fields), vec![&ITEMS[2]]);
    }

    #[test]
    fn filter_state_inserts_produced_text_and_stages_escape() {
        let mut state = FilterState::default();
        state.insert("Hö");
        assert_eq!(state.query(), "Hö");
        assert!(state.had_focus());

        state.escape();
        assert_eq!(state.query(), "Hö");
        assert!(!state.had_focus());

        state.escape();
        assert_eq!(state.query(), "");
    }

    #[test]
    fn replacing_query_restores_focus_stage() {
        let mut state = FilterState::default();
        state.replace("honk".to_owned());
        state.escape();
        state.replace("goose".to_owned());
        assert_eq!(state.query(), "goose");
        assert!(state.had_focus());
    }

    #[test]
    fn only_type_to_filter_without_a_blocker_can_activate() {
        assert!(ActivationContext::new(Activation::TypeToFilter, false).allows_typing());
        assert!(!ActivationContext::new(Activation::ClickOnly, false).allows_typing());
        assert!(!ActivationContext::new(Activation::TypeToFilter, true).allows_typing());
    }
}
