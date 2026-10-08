//! Time line form (specs/ide.md §14).

use eframe::egui::{self, Ui, Vec2};
use instance_format::{Action, Moment, Project, ResourceKind, Timeline};

use super::{commit_slot, slot_dirty, Cx, Editor, Request, ResKey};
use crate::widgets::actions::{action_list, arg_dialog, palette, ActionEditState};
use crate::widgets::{show_prompt, Prompt, PromptResult};

/// Add `actions` at `step`, merging with an existing moment. Returns its index.
pub fn put(moments: &mut Vec<Moment>, step: i32, mut actions: Vec<Action>) -> usize {
    match moments.binary_search_by_key(&step, |m| m.step) {
        Ok(i) => {
            moments[i].actions.append(&mut actions);
            i
        }
        Err(i) => {
            moments.insert(i, Moment { step, actions });
            i
        }
    }
}

/// Remove and return the moments with step in `a..=b`.
fn take_range(moments: &mut Vec<Moment>, a: i32, b: i32) -> Vec<Moment> {
    let (taken, kept): (Vec<_>, Vec<_>) = std::mem::take(moments)
        .into_iter()
        .partition(|m| m.step >= a && m.step <= b);
    *moments = kept;
    taken
}

/// Shift (or duplicate) moments in `a..=b` so that `a` moves to `new_start`.
pub fn shift(moments: &mut Vec<Moment>, a: i32, b: i32, new_start: i32, copy: bool) {
    let taken = if copy {
        moments
            .iter()
            .filter(|m| m.step >= a && m.step <= b)
            .cloned()
            .collect()
    } else {
        take_range(moments, a, b)
    };
    for m in taken {
        put(moments, m.step - a + new_start, m.actions);
    }
}

/// Scale distances from `a` of moments in `a..=b` by `pct` percent.
pub fn spread(moments: &mut Vec<Moment>, a: i32, b: i32, pct: f64) {
    for m in take_range(moments, a, b) {
        let s = a + ((m.step - a) as f64 * pct / 100.0).round() as i32;
        put(moments, s, m.actions);
    }
}

/// Merge all moments in `a..=b` into the first of them.
pub fn merge(moments: &mut Vec<Moment>, a: i32, b: i32) {
    let taken = take_range(moments, a, b);
    if let Some(first) = taken.first().map(|m| m.step) {
        for m in taken {
            put(moments, first, m.actions);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Change,
    Delete,
    Shift,
    Duplicate,
    Spread,
    Merge,
}

pub struct TimelineEditor {
    idx: usize,
    work: Timeline,
    moment: Option<usize>,
    actions: ActionEditState,
    prompt: Option<(Op, Prompt)>,
}

impl TimelineEditor {
    pub fn new(idx: usize, work: Timeline) -> Self {
        let moment = if work.moments.is_empty() {
            None
        } else {
            Some(0)
        };
        TimelineEditor {
            idx,
            work,
            moment,
            actions: ActionEditState::default(),
            prompt: None,
        }
    }

    fn ask(&mut self, op: Op, title: &str, fields: &[(&str, String)]) {
        self.prompt = Some((
            op,
            Prompt {
                title: title.into(),
                fields: fields
                    .iter()
                    .map(|(l, v)| (l.to_string(), v.clone()))
                    .collect(),
            },
        ));
    }

    fn apply(&mut self, op: Op, v: &[String], cx: &mut Cx) {
        let n = |i: usize| v.get(i).and_then(|s| s.trim().parse::<i32>().ok());
        let ms = &mut self.work.moments;
        let bad = || Request::Message("Please enter valid numbers.".into());
        match op {
            Op::Add => match n(0) {
                Some(s) => {
                    if s < 0 {
                        cx.requests.push(Request::Message(
                            "Negative moments are never executed.".into(),
                        ));
                    }
                    self.moment = Some(put(ms, s, Vec::new()));
                }
                None => cx.requests.push(bad()),
            },
            Op::Change => match (n(0), self.moment) {
                (Some(s), Some(i)) if i < ms.len() => {
                    let m = ms.remove(i);
                    self.moment = Some(put(ms, s, m.actions));
                }
                _ => cx.requests.push(bad()),
            },
            Op::Delete => match (n(0), n(1)) {
                (Some(a), Some(b)) => {
                    take_range(ms, a, b);
                    self.moment = None;
                }
                _ => cx.requests.push(bad()),
            },
            Op::Shift | Op::Duplicate => match (n(0), n(1), n(2)) {
                (Some(a), Some(b), Some(c)) => {
                    shift(ms, a, b, c, op == Op::Duplicate);
                    self.moment = None;
                }
                _ => cx.requests.push(bad()),
            },
            Op::Spread => match (
                n(0),
                n(1),
                v.get(2).and_then(|s| s.trim().parse::<f64>().ok()),
            ) {
                (Some(a), Some(b), Some(p)) => {
                    spread(ms, a, b, p);
                    self.moment = None;
                }
                _ => cx.requests.push(bad()),
            },
            Op::Merge => match (n(0), n(1)) {
                (Some(a), Some(b)) => {
                    merge(ms, a, b);
                    self.moment = None;
                }
                _ => cx.requests.push(bad()),
            },
        }
        self.actions.selected = None;
    }
}

impl Editor for TimelineEditor {
    fn key(&self) -> ResKey {
        ResKey::Res(ResourceKind::Timeline, self.idx)
    }
    fn title(&self) -> String {
        format!("Time Line Properties: {}", self.work.name)
    }
    fn default_size(&self) -> Vec2 {
        Vec2::new(960.0, 520.0)
    }
    fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        let ctx = cx.ctx.clone();
        let base_id = ui.id().with(("timeline", self.idx));
        if let Some((op, mut pr)) = self.prompt.take() {
            match show_prompt(&ctx, base_id.with("prompt"), &mut pr) {
                PromptResult::Ok(v) => self.apply(op, &v, cx),
                PromptResult::Cancel => {}
                PromptResult::None => self.prompt = Some((op, pr)),
            }
        }
        if let Some(m) = self.moment.and_then(|i| self.work.moments.get_mut(i)) {
            arg_dialog(
                &ctx,
                base_id.with("args"),
                &mut m.actions,
                &mut self.actions,
                cx,
            );
        }
        let cur = self
            .moment
            .and_then(|i| self.work.moments.get(i))
            .map_or(0, |m| m.step)
            .to_string();
        let h = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(130.0);
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.work.name);
                ui.separator();
                if ui.button("Add").clicked() {
                    self.ask(Op::Add, "Add moment", &[("Step:", cur.clone())]);
                }
                if ui
                    .add_enabled(self.moment.is_some(), egui::Button::new("Change"))
                    .clicked()
                {
                    self.ask(Op::Change, "Change moment", &[("New step:", cur.clone())]);
                }
                if ui.button("Delete").clicked() {
                    self.ask(
                        Op::Delete,
                        "Delete moments",
                        &[("From:", cur.clone()), ("Till:", cur.clone())],
                    );
                }
                if ui.button("Clear").clicked() {
                    self.work.moments.clear();
                    self.moment = None;
                }
                ui.separator();
                if ui.button("Shift").clicked() {
                    self.ask(
                        Op::Shift,
                        "Shift moments",
                        &[
                            ("From:", cur.clone()),
                            ("Till:", cur.clone()),
                            ("Shift to:", cur.clone()),
                        ],
                    );
                }
                if ui.button("Duplicate").clicked() {
                    self.ask(
                        Op::Duplicate,
                        "Duplicate moments",
                        &[
                            ("From:", cur.clone()),
                            ("Till:", cur.clone()),
                            ("Copy to:", cur.clone()),
                        ],
                    );
                }
                if ui.button("Spread").clicked() {
                    self.ask(
                        Op::Spread,
                        "Spread moments",
                        &[
                            ("From:", cur.clone()),
                            ("Till:", cur.clone()),
                            ("Percentage:", "100".into()),
                        ],
                    );
                }
                if ui.button("Merge").clicked() {
                    self.ask(
                        Op::Merge,
                        "Merge moments",
                        &[("From:", cur.clone()), ("Till:", cur.clone())],
                    );
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(110.0);
                ui.label("Moments:");
                egui::ScrollArea::vertical()
                    .id_salt("moments")
                    .max_height(h - 30.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (i, m) in self.work.moments.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    self.moment == Some(i),
                                    format!("Step {}", m.step),
                                )
                                .clicked()
                                && self.moment != Some(i)
                            {
                                self.moment = Some(i);
                                self.actions.selected = None;
                            }
                        }
                    });
            });
            ui.separator();
            let act_w = (ui.available_width() - 240.0).max(200.0);
            ui.vertical(|ui| {
                ui.set_width(act_w);
                ui.set_height(h);
                ui.label("Actions:");
                match self.moment.and_then(|i| self.work.moments.get_mut(i)) {
                    Some(m) => action_list(ui, &mut m.actions, &mut self.actions, cx),
                    None => {
                        ui.label(egui::RichText::new("Add or select a moment first.").weak());
                    }
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(220.0);
                if let Some(m) = self.moment.and_then(|i| self.work.moments.get_mut(i)) {
                    palette(ui, &mut m.actions, &mut self.actions);
                }
            });
        });
    }
    fn on_renamed(&mut self, name: &str) {
        self.work.name = name.to_string();
    }
    fn is_dirty(&self, p: &Project) -> bool {
        slot_dirty(p.timelines.get(self.idx), &self.work)
    }
    fn commit(&mut self, p: &mut Project) -> bool {
        commit_slot(p.timelines.get_mut(self.idx), &self.work)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(steps: &[i32]) -> Vec<Moment> {
        steps
            .iter()
            .map(|&s| Moment {
                step: s,
                actions: vec![Action {
                    action_id: s,
                    ..Default::default()
                }],
            })
            .collect()
    }
    fn steps(m: &[Moment]) -> Vec<i32> {
        m.iter().map(|m| m.step).collect()
    }

    #[test]
    fn spread_example() {
        let mut m = ms(&[5, 8, 12]);
        spread(&mut m, 5, 12, 200.0);
        assert_eq!(steps(&m), vec![5, 11, 19]);
    }

    #[test]
    fn shift_merges_collisions() {
        let mut m = ms(&[0, 10, 20]);
        shift(&mut m, 0, 0, 10, false);
        assert_eq!(steps(&m), vec![10, 20]);
        assert_eq!(m[0].actions.len(), 2);
        shift(&mut m, 10, 20, 100, true);
        assert_eq!(steps(&m), vec![10, 20, 100, 110]);
        merge(&mut m, 100, 200);
        assert_eq!(steps(&m), vec![10, 20, 100]);
        assert_eq!(m[2].actions.len(), 3);
    }
}
