//! Open and close the projector window, the Identify windows, and the frame
//! clock the live scene is drawn with.

use crate::app::App;
use crate::displays::{self, Display};
use crate::msg::Msg;
use iced::{Point, Size, Task, window};
use std::time::{Duration, Instant};

/// How long the Identify windows stay up.
const IDENTIFY_FOR: Duration = Duration::from_secs(3);

fn settings(d: &Display, size: Size, offset: f32) -> window::Settings {
    let (x, y, _, _) = d.rect;
    window::Settings {
        size,
        position: window::Position::Specific(Point::new(x as f32 + offset, y as f32 + offset)),
        decorations: false,
        resizable: false,
        level: window::Level::AlwaysOnTop,
        ..window::Settings::default()
    }
}

/// Open the projector fullscreen on `display` and remember the choice.
pub fn open(app: &mut App, display: &Display) -> Task<Msg> {
    let (_, _, w, h) = display.rect;
    let (id, task) = window::open(settings(display, Size::new(w as f32, h as f32), 1.0));
    app.projector = Some(id);
    app.projector_display = Some(display.index);
    if app.core.cfg.output_display != Some(display.index as i32) {
        app.core.cfg.output_display = Some(display.index as i32);
        app.core.save_cfg();
    }
    task.then(move |_| window::set_mode::<Msg>(id, window::Mode::Fullscreen))
        .discard()
}

pub fn close(app: &mut App) -> Task<Msg> {
    match app.projector.take() {
        Some(id) => window::close(id),
        None => Task::none(),
    }
}

/// Close when open, else open on the chosen display. Without a GPU the
/// projector would show plain text, so the toggle does nothing. With fewer
/// than two displays known the list is refreshed, a projector may have
/// been plugged in since boot.
pub fn toggle(app: &mut App) -> Task<Msg> {
    if app.projector.is_some() {
        return close(app);
    }
    if !app.gpu_ok {
        return Task::none();
    }
    let Some(d) = app
        .projector_display
        .and_then(|i| app.displays.get(i))
        .cloned()
    else {
        return refresh_displays();
    };
    if app.displays.len() < 2 {
        return Task::batch([refresh_displays(), open(app, &d)]);
    }
    open(app, &d)
}

/// Ask the OS for the display list again.
pub fn refresh_displays() -> Task<Msg> {
    Task::perform(async { displays::list() }, Msg::DisplaysListed)
}

/// Pick the display and move an open projector there.
pub fn choose_display(app: &mut App, index: usize) -> Task<Msg> {
    app.projector_display = Some(index);
    if app.core.cfg.output_display != Some(index as i32) {
        app.core.cfg.output_display = Some(index as i32);
        app.core.save_cfg();
    }
    if app.projector.is_none() {
        return Task::none();
    }
    let Some(d) = app.displays.get(index).cloned() else {
        return Task::none();
    };
    Task::batch([close(app), open(app, &d)])
}

/// One small numbered window per display for a few seconds.
pub fn identify(app: &mut App) -> Task<Msg> {
    let mut tasks = vec![identify_done(app)];
    for d in app.displays.clone() {
        let (id, task) = window::open(settings(&d, Size::new(240.0, 160.0), 40.0));
        app.identify.push(id);
        tasks.push(task.discard());
    }
    app.identify_seq += 1;
    let seq = app.identify_seq;
    tasks.push(Task::perform(
        async { tokio::time::sleep(IDENTIFY_FOR).await },
        move |_| Msg::IdentifyDone(seq),
    ));
    Task::batch(tasks)
}

pub fn identify_done(app: &mut App) -> Task<Msg> {
    Task::batch(app.identify.drain(..).map(window::close))
}

/// The display number an Identify window shows, or `None` for other ids.
pub fn identify_number(app: &App, id: window::Id) -> Option<usize> {
    app.identify.iter().position(|w| *w == id).map(|i| i + 1)
}

/// A window went away. Forget it if it was ours.
pub fn closed(app: &mut App, id: window::Id) {
    if app.projector == Some(id) {
        app.projector = None;
    }
    app.identify.retain(|w| *w != id);
}

/// A frame arrived. Advance the clock and drop a finished transition so the
/// frames subscription stops.
pub fn tick(app: &mut App, now: Instant) {
    app.now = now;
    app.live.settle(now);
}
