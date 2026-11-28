//! Inline, virtualized parameter selection shared by every binding context.
use crate::{app::Keyforge, binding_controls::BindingChoice};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme, IndexPath,
    list::{ListDelegate, ListItem, ListState},
    select::SelectItem,
};
use keyforge_core::{
    Command, bindings,
    editing::{BindingEdit, BindingTarget},
};

pub(crate) fn search(items: &[BindingChoice], query: &str) -> Vec<BindingChoice> {
    let query = query.trim().to_lowercase();
    let mut matches = items
        .iter()
        .filter(|item| item.matches(&query))
        .cloned()
        .collect::<Vec<_>>();
    if !query.is_empty() {
        matches.sort_by_key(|item| {
            let code = item.value.to_lowercase();
            if code == query {
                0
            } else if code.starts_with(&query) {
                1
            } else if code.contains(&query) {
                2
            } else {
                3
            }
        });
    }
    matches
}
pub(crate) struct ParameterPicker {
    pub items: Vec<BindingChoice>,
    matches: Vec<BindingChoice>,
    query: String,
    focused: Option<IndexPath>,
    target: BindingTarget,
    parameter: usize,
    owner: WeakEntity<Keyforge>,
}
impl ParameterPicker {
    pub fn new(
        items: Vec<BindingChoice>,
        target: BindingTarget,
        parameter: usize,
        owner: WeakEntity<Keyforge>,
    ) -> Self {
        Self {
            matches: items.clone(),
            items,
            query: String::new(),
            focused: None,
            target,
            parameter,
            owner,
        }
    }
    pub fn set_items(&mut self, items: Vec<BindingChoice>) {
        self.items = items;
        self.matches = search(&self.items, &self.query);
        if self
            .focused
            .is_some_and(|index| index.row >= self.matches.len())
        {
            self.focused = None;
        }
    }
}
impl ListDelegate for ParameterPicker {
    type Item = ListItem;
    fn items_count(&self, _: usize, _: &App) -> usize {
        self.matches.len()
    }
    fn render_item(
        &mut self,
        index: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let item = self.matches.get(index.row)?;
        let selected = self
            .owner
            .read_with(cx, |owner, _| {
                owner
                    .core
                    .assignment_form(&self.target)
                    .and_then(|form| form.arguments.get(self.parameter).cloned())
                    .is_some_and(|value| bindings::modifiers(&value).0 == item.value)
            })
            .unwrap_or(false);
        Some(
            ListItem::new(index.row)
                .selected(selected || self.focused == Some(index))
                .h(px(36.))
                .px_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_size(px(12.))
                                .child(item.value.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(cx.theme().muted_foreground)
                                .child(item.description.clone()),
                        )
                        .when(selected, |row| row.child("✓")),
                ),
        )
    }
    fn set_selected_index(
        &mut self,
        index: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.focused = index;
    }
    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.query = query.into();
        self.matches = search(&self.items, query);
        Task::ready(())
    }
    fn confirm(&mut self, _: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let Some(value) = self
            .focused
            .and_then(|index| self.matches.get(index.row))
            .map(|item| item.value.clone())
        else {
            return;
        };
        let owner = self.owner.clone();
        let target = self.target.clone();
        let index = self.parameter;
        cx.defer_in(window, move |_, window, cx| {
            let _ = owner.update(cx, |_, cx| {
                cx.defer_in(window, move |owner, window, cx| {
                    if owner.binding_target != target {
                        return;
                    }
                    let current = owner
                        .core
                        .assignment_form(&target)
                        .and_then(|form| form.arguments.get(index).cloned())
                        .unwrap_or_default();
                    if bindings::modifiers(&current).0 != value {
                        owner.command(
                            Command::EditBinding {
                                target,
                                edit: BindingEdit::Argument {
                                    index,
                                    value,
                                    selected: true,
                                },
                            },
                            window,
                            cx,
                        );
                    }
                });
            });
        });
    }
    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let _ = self.owner.update(cx, |owner, cx| {
            owner.expanded_action = None;
            owner.binding_advanced = false;
            owner.focus.focus(window);
            cx.notify();
        });
    }
}
