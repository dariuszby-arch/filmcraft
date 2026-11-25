//! Temporary same-frame A/B state restored from the interrupted DW2 work.
use crate::{
    FilmcraftApp,
    frames::{FrameKey, Target},
    theme::Tokens,
};
use egui::{Align2, Color32, Id, Rect, Stroke, pos2};
use filmcraft_project::ClipId;
use filmcraft_time::{FrameRate, Tick};
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Mode {
    #[default]
    Wipe,
    Before,
    After,
}
#[derive(Clone, Copy)]
struct Comparison {
    clip: ClipId,
    index: usize,
    mode: Mode,
    split: f32,
}
fn id() -> Id {
    Id::unique("dw-denoise-before-after")
}
pub(super) fn controls(app: &mut FilmcraftApp, ui: &mut egui::Ui, clip: ClipId, index: usize) {
    let mut state = ui.ctx().data_mut(|d| d.get_temp::<Comparison>(id()));
    let selected = state.is_some_and(|c| c.clip == clip && c.index == index);
    let mut on = selected;
    let response = ui.checkbox(&mut on, tl!("Before / After"));
    app.auto.add_response("denoise.compare", &response, tl!("Before / After"));
    if response.changed() {
        app.stop();
        if on {
            app.ui.program.display = crate::state::DisplayMode::Composite;
            app.ui.program.multicam = false;
            app.ui.program.zoom = Some(2.0);
            app.ui.program.pan = [0.0, 0.0];
        }
        state = on.then_some(Comparison { clip, index, mode: Mode::Wipe, split: 0.5 });
    }
    if let Some(c) = state.as_mut().filter(|c| c.clip == clip && c.index == index) {
        ui.horizontal_wrapped(|ui| {
            for (mode, text, key) in [(Mode::Wipe, tl!("Wipe"), "wipe"), (Mode::Before, tl!("Before"), "before"), (Mode::After, tl!("After"), "after")] {
                let response = ui.selectable_value(&mut c.mode, mode, text);
                app.auto.add_response(&format!("denoise.compare.{key}"), &response, text);
            }
        });
        if c.mode == Mode::Wipe {
            let response = ui.add(egui::Slider::new(&mut c.split, 0.0..=1.0).text(tl!("Wipe Position")));
            app.auto.add_response("denoise.compare.position", &response, tl!("Wipe Position"));
        }
        ui.small(tl!("Same frame; the comparison does not change export."));
    }
    ui.ctx().data_mut(|d| {
        if let Some(c) = state {
            d.insert_temp(id(), c);
        } else {
            d.remove::<Comparison>(id());
        }
    });
}
pub(super) struct Picture {
    pub target: Target,
    pub time: Tick,
    pub rate: FrameRate,
    pub rect: Rect,
    pub scale: f32,
}
pub(super) fn draw(app: &mut FilmcraftApp, ui: &mut egui::Ui, picture: Picture) -> bool {
    let Some(c) = ui.ctx().data_mut(|d| d.get_temp::<Comparison>(id())) else { return false };
    let Picture { target, time, rate, rect, scale } = picture;
    let Target::Sequence(sequence) = target else { return false };
    let valid = app
        .session
        .project
        .sequence(sequence)
        .and_then(|s| s.find_item(c.clip))
        .and_then(|(_, item)| item.effects.get(c.index))
        .is_some_and(|e| e.effect == "dw_video_denoise");
    if !valid {
        ui.ctx().data_mut(|d| d.remove::<Comparison>(id()));
        return false;
    }
    let frame = rate.frame_at(time);
    let after = FrameKey { target: Target::SequenceLive(sequence), frame, size: (scale * 1000.0) as u32, revision: app.session.revision, draft: false };
    let before = FrameKey { target: Target::SequenceBefore(sequence, c.clip, c.index), ..after };
    let project = app.session.project.clone();
    app.frames.request_in_view(before, rate.tick_of(frame), scale, &project, 103);
    app.frames.request_in_view(after, rate.tick_of(frame), scale, &project, 104);
    let (a, b) = (app.frames.get(&before), app.frames.get(&after));
    let (Some(a), Some(b)) = (a, b) else {
        ui.painter().rect_filled(rect, 0.0, app.tokens.monitor_bg);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, tl!("Processing denoising comparison…"), Tokens::ui(13.0), app.tokens.text);
        app.monitor_inexact = true;
        return true;
    };
    let ctx = ui.ctx().clone();
    let (a, b) = (app.texture_for(&ctx, "denoise-before", before, &a), app.texture_for(&ctx, "denoise-after", after, &b));
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    match c.mode {
        Mode::Before => {
            ui.painter().image(a, rect, uv, Color32::WHITE);
        }
        Mode::After => {
            ui.painter().image(b, rect, uv, Color32::WHITE);
        }
        Mode::Wipe => {
            let split = rect.min.x + rect.width() * c.split;
            let (left, right) = (Rect::from_min_max(rect.min, pos2(split, rect.max.y)), Rect::from_min_max(pos2(split, rect.min.y), rect.max));
            ui.painter().with_clip_rect(ui.clip_rect().intersect(left)).image(a, rect, uv, Color32::WHITE);
            ui.painter().with_clip_rect(ui.clip_rect().intersect(right)).image(b, rect, uv, Color32::WHITE);
            ui.painter().line_segment([pos2(split, rect.min.y), pos2(split, rect.max.y)], Stroke::new(2.0, app.tokens.accent));
        }
    }
    let label = match c.mode {
        Mode::Before => tl!("Before"),
        Mode::After => tl!("After"),
        Mode::Wipe => tl!("Before / After"),
    };
    let badge = Rect::from_min_size(rect.min, egui::vec2(160.0, 25.0));
    ui.painter().rect_filled(badge, 0.0, app.tokens.monitor_bg);
    ui.painter().text(badge.center(), Align2::CENTER_CENTER, label, Tokens::ui(12.0), app.tokens.text);
    app.program_shown = Some(after);
    app.auto.add("denoise.compare.picture", rect, label);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparison_is_keyboard_accessible_and_does_not_edit_the_project() {
        let mut app = FilmcraftApp::new(filmcraft_engine::Session::default());
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let revision = app.session.revision;
        let mut now = 0.0;
        let mut frame = |app: &mut FilmcraftApp, events| {
            now += 0.1;
            app.auto.begin_frame();
            ctx.run_ui(egui::RawInput { time: Some(now), events, ..Default::default() }, |ui| {
                controls(app, ui, ClipId(1), 3);
            })
        };
        let out = frame(&mut app, vec![]);
        let tree = out.platform_output.accesskit_update.expect("accessible controls");
        assert!(tree.nodes.iter().any(|(_, n)| n.role() == egui::accesskit::Role::CheckBox && n.label() == Some("Before / After")));
        let focus = app.auto.find("denoise.compare").unwrap().response_id.unwrap();
        ctx.memory_mut(|m| m.request_focus(focus));
        let press = |pressed| egui::Event::Key { key: egui::Key::Space, physical_key: None, pressed, repeat: false, modifiers: Default::default() };
        frame(&mut app, vec![press(true)]);
        frame(&mut app, vec![press(false)]);
        assert!(ctx.data_mut(|d| d.get_temp::<Comparison>(id())).is_some());
        let before = app.auto.find("denoise.compare.before").unwrap().response_id.unwrap();
        ctx.memory_mut(|m| m.request_focus(before));
        frame(&mut app, vec![press(true)]);
        frame(&mut app, vec![press(false)]);
        assert!(ctx.data_mut(|d| d.get_temp::<Comparison>(id())).is_some_and(|c| c.mode == Mode::Before));
        assert_eq!(ctx.memory(|m| m.focused()), Some(before));
        assert_eq!(app.session.revision, revision);
    }
}
