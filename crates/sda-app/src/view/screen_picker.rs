//! Connected displays drawn at their real positions. Click one to pick it.

use crate::displays::Display;
use crate::ui_theme;
use iced::alignment::Vertical;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Event, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme};

pub const WIDTH: f32 = 360.0;
pub const HEIGHT: f32 = 200.0;
const PAD: f32 = 8.0;

struct Picker<'a, M> {
    displays: &'a [Display],
    selected: Option<usize>,
    on_pick: Box<dyn Fn(usize) -> M + 'a>,
}

impl<M> Picker<'_, M> {
    /// Every display scaled so the union of all rects fits the box.
    fn boxes(&self, bounds: Size) -> Vec<(usize, Rectangle)> {
        let mut union: Option<(f32, f32, f32, f32)> = None;
        for d in self.displays {
            let (x, y, w, h) = d.rect;
            let (x0, y0, x1, y1) = (
                x as f32,
                y as f32,
                (x + w as i32) as f32,
                (y + h as i32) as f32,
            );
            union = Some(match union {
                None => (x0, y0, x1, y1),
                Some((a, b, c, e)) => (a.min(x0), b.min(y0), c.max(x1), e.max(y1)),
            });
        }
        let Some((ux, uy, ux1, uy1)) = union else {
            return Vec::new();
        };
        let (uw, uh) = ((ux1 - ux).max(1.0), (uy1 - uy).max(1.0));
        let k = ((bounds.width - 2.0 * PAD) / uw).min((bounds.height - 2.0 * PAD) / uh);
        let ox = (bounds.width - uw * k) / 2.0;
        let oy = (bounds.height - uh * k) / 2.0;
        self.displays
            .iter()
            .map(|d| {
                let (x, y, w, h) = d.rect;
                let r = Rectangle {
                    x: ox + (x as f32 - ux) * k + 1.0,
                    y: oy + (y as f32 - uy) * k + 1.0,
                    width: w as f32 * k - 2.0,
                    height: h as f32 * k - 2.0,
                };
                (d.index, r)
            })
            .collect()
    }

    fn hit(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let p = cursor.position_in(bounds)?;
        self.boxes(bounds.size())
            .into_iter()
            .find(|(_, r)| r.contains(p))
            .map(|(i, _)| i)
    }
}

impl<M> canvas::Program<M> for Picker<'_, M> {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<M>> {
        if !matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) {
            return None;
        }
        let i = self.hit(bounds, cursor)?;
        Some(canvas::Action::publish((self.on_pick)(i)).and_capture())
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let hover = self.hit(bounds, cursor);
        for (i, r) in self.boxes(bounds.size()) {
            let label = self
                .displays
                .iter()
                .find(|d| d.index == i)
                .map(|d| d.label.as_str())
                .unwrap_or_default();
            let picked = self.selected == Some(i);
            let fill = if picked {
                Color {
                    a: 0.35,
                    ..ui_theme::palette().accent
                }
            } else if hover == Some(i) {
                Color {
                    a: 0.9,
                    ..ui_theme::palette().surface_hi
                }
            } else {
                ui_theme::palette().surface_hi
            };
            let path = Path::rounded_rectangle(r.position(), r.size(), 4.0.into());
            frame.fill(&path, fill);
            let edge = if picked {
                ui_theme::palette().accent
            } else {
                ui_theme::palette().muted
            };
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(edge)
                    .with_width(if picked { 2.0 } else { 1.0 }),
            );
            frame.fill_text(Text {
                content: (i + 1).to_string(),
                position: Point::new(r.center_x(), r.center_y()),
                color: if picked {
                    Color::WHITE
                } else {
                    ui_theme::palette().text
                },
                size: (r.height * 0.4).clamp(12.0, 28.0).into(),
                font: ui_theme::BOLD,
                align_x: iced::advanced::text::Alignment::Center,
                align_y: Vertical::Center,
                ..Text::default()
            });
            if r.height > 60.0 {
                frame.fill_text(Text {
                    content: label.to_string(),
                    position: Point::new(r.center_x(), r.y + r.height - 14.0),
                    color: ui_theme::palette().muted,
                    size: 11.0.into(),
                    align_x: iced::advanced::text::Alignment::Center,
                    align_y: Vertical::Center,
                    ..Text::default()
                });
            }
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.hit(bounds, cursor).is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

/// The picker box. `on_pick` gets the index of the clicked display.
pub fn show<'a, M: 'a>(
    displays: &'a [Display],
    selected: Option<usize>,
    on_pick: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    Canvas::new(Picker {
        displays,
        selected,
        on_pick: Box::new(on_pick),
    })
    .width(WIDTH)
    .height(HEIGHT)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(i: usize, rect: (i32, i32, u32, u32)) -> Display {
        Display {
            index: i,
            label: String::new(),
            rect,
            scale: 1.0,
            is_primary: i == 0,
        }
    }

    #[test]
    fn boxes_scale_the_union_to_fit_and_keep_order() {
        let displays = [d(0, (0, 0, 1440, 900)), d(1, (1440, -100, 1920, 1080))];
        let picker = Picker::<()> {
            displays: &displays,
            selected: None,
            on_pick: Box::new(|_| ()),
        };
        let boxes = picker.boxes(Size::new(WIDTH, HEIGHT));
        assert_eq!(boxes.len(), 2);
        let (_, a) = boxes[0];
        let (_, b) = boxes[1];
        assert!(a.x < b.x, "second display sits to the right");
        assert!(b.y < a.y, "second display sits higher");
        for (_, r) in &boxes {
            assert!(r.x >= 0.0 && r.y >= 0.0);
            assert!(r.x + r.width <= WIDTH && r.y + r.height <= HEIGHT);
        }
        let hit = picker.hit(
            Rectangle::new(Point::ORIGIN, Size::new(WIDTH, HEIGHT)),
            mouse::Cursor::Available(Point::new(b.center_x(), b.center_y())),
        );
        assert_eq!(hit, Some(1));
    }
}
