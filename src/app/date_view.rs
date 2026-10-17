//! "Date calculation" mode: the difference between two dates, or adding and
//! subtracting years, months and days. The layout and sizes follow
//! `DateCalculator.xaml` from the Windows Calculator source.

use chrono::{Datelike, Days, Local, Months, NaiveDate};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Event, FontId, Id, Key, MouseWheelUnit, Painter, Pos2,
    Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, WidgetType, epaint::Shadow, pos2,
    text::LayoutJob, vec2,
};

use super::icons::{ICON_CALENDAR, ICON_CARET_DOWN, ICON_CARET_UP, ICON_CHEVRON_DOWN, icon_label};
use super::{
    CalculatorApp, Flyout, RADIUS, access,
    palette::Palette,
    radio_button,
    region::{self, formats},
    semibold,
    strings::{tr, tr_format},
};
use crate::dates::{self, MAX_DATE, MAX_OFFSET, MIN_DATE};

const PAGE_MARGIN: f32 = 16.0;
const CONTROL_HEIGHT: f32 = 32.0;
/// From this window width on, Windows shows results in a larger font.
const WIDE_LAYOUT_WIDTH: f32 = 480.0;
const COMBO_WIDTH: f32 = 84.0;
const COMBO_GAP: f32 = 12.0;
const LIST_ITEM_HEIGHT: f32 = 32.0;
const LIST_VISIBLE_ITEMS: usize = 9;
const CALENDAR_WIDTH: f32 = 336.0;
const CALENDAR_HEADER_HEIGHT: f32 = 48.0;
const WEEKDAY_ROW_HEIGHT: f32 = 38.0;
const CALENDAR_ROW_HEIGHT: f32 = 41.0;
/// How far the mouse wheel has to move to flip one calendar page.
const WHEEL_STEP: f32 = 40.0;

/// The two pages of the mode, picked in the drop-down at the top.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Difference,
    AddSubtract,
}

impl Page {
    const ALL: [Page; 2] = [Page::Difference, Page::AddSubtract];

    fn title(self) -> &'static str {
        match self {
            Page::Difference => tr("Date_DifferenceOption.Content"),
            Page::AddSubtract => tr("Date_AddSubtractOption.Content"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum DateField {
    From,
    To,
    /// The "From" date of "Add or subtract days".
    Start,
}

impl DateField {
    /// The caption above the date picker, which also names it.
    fn header(self) -> &'static str {
        tr(match self {
            DateField::From => "DateDiff_FromHeader.Header",
            DateField::To => "DateDiff_ToHeader.Header",
            DateField::Start => "AddSubtract_Date_FromHeader.Header",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Offset {
    Years,
    Months,
    Days,
}

impl Offset {
    const ALL: [Offset; 3] = [Offset::Years, Offset::Months, Offset::Days];

    fn label(self) -> &'static str {
        match self {
            Offset::Years => tr("YearsLabel.Text"),
            Offset::Months => tr("MonthsLabel.Text"),
            Offset::Days => tr("DaysLabel.Text"),
        }
    }
}

/// Zoom level of the calendar: days of a month, months of a year, years of a decade.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CalendarView {
    Month,
    Year,
    Decade,
}

#[derive(Clone, Copy, PartialEq)]
enum Popup {
    /// The list of pages ("Difference between dates", "Add or subtract days").
    Pages,
    /// `shown` is the first day of the shown month (in any view).
    Calendar {
        field: DateField,
        anchor: Rect,
        view: CalendarView,
        shown: NaiveDate,
    },
    /// The 0–999 list of a years, months or days box. `scroll_pending` scrolls
    /// the selected value into place on the first frame.
    Offset {
        field: Offset,
        anchor: Rect,
        scroll_pending: bool,
    },
}

/// What a click in the calendar asks for.
enum CalendarAction {
    Pick(NaiveDate),
    Show(CalendarView, NaiveDate),
}

/// State of the date calculation mode.
pub(super) struct DateState {
    page: Page,
    from: NaiveDate,
    to: NaiveDate,
    start: NaiveDate,
    subtract: bool,
    years: u32,
    months: u32,
    days: u32,
    popup: Option<Popup>,
    /// Mouse wheel movement over the calendar that hasn't flipped a page yet.
    wheel: f32,
}

impl DateState {
    pub(super) fn new() -> Self {
        let today = today();
        Self {
            page: Page::Difference,
            from: today,
            to: today,
            start: today,
            subtract: false,
            years: 0,
            months: 0,
            days: 0,
            popup: None,
            wheel: 0.0,
        }
    }

    /// Closes an open drop-down or calendar. Returns `false` if none was open.
    pub(super) fn close_popup(&mut self) -> bool {
        self.popup.take().is_some()
    }

    pub(super) fn popup_open(&self) -> bool {
        self.popup.is_some()
    }

    /// Switches pages. Like Windows' `DateCalcOption_Changed` (`DateCalculator.xaml.cs`),
    /// the From date goes along to the page switched to; the To date stays. Picking the page
    /// that is already shown does nothing, as Windows' drop-down reports no change then.
    fn set_page(&mut self, page: Page) {
        if page == self.page {
            return;
        }
        self.page = page;
        match page {
            Page::Difference => self.from = self.start,
            Page::AddSubtract => self.start = self.from,
        }
    }

    /// The main result line; Ctrl+C copies it.
    pub(super) fn result_text(&self) -> String {
        match self.page {
            Page::Difference => {
                dates::describe_difference(self.from, self.to, &region::date_words()).0
            }
            Page::AddSubtract => self.resulting_date().map_or_else(
                || tr("Date_OutOfBoundMessage").to_string(),
                |date| formats().long_date(date),
            ),
        }
    }

    fn resulting_date(&self) -> Option<NaiveDate> {
        let (years, months, days) = (self.years, self.months, self.days);
        if self.subtract {
            dates::subtract(self.start, years, months, days)
        } else {
            dates::add(self.start, years, months, days)
        }
    }

    fn date(&self, field: DateField) -> NaiveDate {
        match field {
            DateField::From => self.from,
            DateField::To => self.to,
            DateField::Start => self.start,
        }
    }

    fn set_date(&mut self, field: DateField, date: NaiveDate) {
        let date = dates::clamp(date);
        match field {
            DateField::From => self.from = date,
            DateField::To => self.to = date,
            DateField::Start => self.start = date,
        }
    }

    fn offset(&self, field: Offset) -> u32 {
        match field {
            Offset::Years => self.years,
            Offset::Months => self.months,
            Offset::Days => self.days,
        }
    }

    fn set_offset(&mut self, field: Offset, value: u32) {
        let value = value.min(MAX_OFFSET);
        match field {
            Offset::Years => self.years = value,
            Offset::Months => self.months = value,
            Offset::Days => self.days = value,
        }
    }
}

impl CalculatorApp {
    /// Keyboard in date mode: Esc closes popups, Ctrl+C copies the result.
    pub(super) fn handle_date_keyboard(&mut self, ctx: &egui::Context) {
        for event in ctx.input(|i| i.events.clone()) {
            match event {
                Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => {
                    if self.flyout != Flyout::None {
                        self.flyout = Flyout::None;
                    } else {
                        self.date.close_popup();
                    }
                }
                Event::Copy => ctx.copy_text(self.date.result_text()),
                _ => {}
            }
        }
    }

    /// Draws the mode below the header. `full` is the whole window.
    pub(super) fn draw_date_mode(&mut self, ui: &mut Ui, full: Rect, top: f32) {
        let p = self.palette;
        let left = full.left() + PAGE_MARGIN;
        let painter = ui.painter().clone();
        let selector = self.draw_page_selector(ui, pos2(left, top + 4.0));
        let content = top + 36.0;
        let result_size = if full.width() >= WIDE_LAYOUT_WIDTH {
            20.0
        } else {
            14.0
        };
        let wrap_width = full.width() - 2.0 * PAGE_MARGIN;

        match self.date.page {
            Page::Difference => {
                caption(
                    p,
                    &painter,
                    pos2(left, content + 17.0),
                    tr("DateDiff_FromHeader.Header"),
                );
                self.draw_date_picker(ui, DateField::From, pos2(left, content + 41.0));
                caption(
                    p,
                    &painter,
                    pos2(left, content + 97.0),
                    tr("DateDiff_ToHeader.Header"),
                );
                self.draw_date_picker(ui, DateField::To, pos2(left, content + 121.0));
                caption(
                    p,
                    &painter,
                    pos2(left, content + 177.0),
                    tr("Date_DifferenceLabel.Text"),
                );
                let (result, in_days) =
                    dates::describe_difference(self.date.from, self.date.to, &region::date_words());
                let pos = pos2(left, content + 197.0);
                let name = tr_format("Date_DifferenceResultAutomationName", &[&result]);
                let bottom = paragraph(p, &painter, pos, result, result_size, wrap_width);
                let rect = Rect::from_min_max(pos, pos2(pos.x + wrap_width, bottom));
                access::text(ui, rect, Id::new("date-result"), &name, true);
                if let Some(in_days) = in_days {
                    caption(p, &painter, pos2(left, bottom + 8.0), &in_days);
                }
            }
            Page::AddSubtract => {
                caption(
                    p,
                    &painter,
                    pos2(left, content + 17.0),
                    tr("AddSubtract_Date_FromHeader.Header"),
                );
                self.draw_date_picker(ui, DateField::Start, pos2(left, content + 41.0));

                // Radio buttons are at least 80 wide, with 20 between them (DateCalculator.xaml).
                let radios = content + 96.0;
                let center_y = radios + CONTROL_HEIGHT / 2.0;
                let subtract = self.date.subtract;
                let add_label = painter.layout_no_wrap(
                    tr("AddOption.Content").into(),
                    FontId::proportional(14.0),
                    p.text,
                );
                let subtract_x = left + (28.0 + add_label.size().x).max(80.0) + 20.0;
                let add_id = Id::new("date-add");
                let add_clicked = radio_button(
                    p,
                    ui,
                    pos2(left, center_y),
                    tr("AddOption.Content"),
                    !subtract,
                    add_id,
                );
                let subtract_id = Id::new("date-subtract");
                let subtract_pos = pos2(subtract_x, center_y);
                let subtract_clicked = radio_button(
                    p,
                    ui,
                    subtract_pos,
                    tr("SubtractOption.Content"),
                    subtract,
                    subtract_id,
                );
                if add_clicked {
                    self.date.subtract = false;
                }
                if subtract_clicked {
                    self.date.subtract = true;
                }

                let labels = radios + CONTROL_HEIGHT + 29.0;
                for (i, field) in Offset::ALL.into_iter().enumerate() {
                    let x = left + i as f32 * (COMBO_WIDTH + COMBO_GAP);
                    caption(p, &painter, pos2(x, labels), field.label());
                    let combo = Rect::from_min_size(
                        pos2(x, labels + 22.0),
                        vec2(COMBO_WIDTH, CONTROL_HEIGHT),
                    );
                    self.draw_offset_combo(ui, field, combo);
                }

                let date_label = labels + 22.0 + CONTROL_HEIGHT + 35.0;
                caption(p, &painter, pos2(left, date_label), tr("DateLabel.Text"));
                let pos = pos2(left, date_label + 20.0);
                let result = self.date.result_text();
                let name = tr_format("Date_ResultingDateAutomationName", &[&result]);
                let bottom = paragraph(p, &painter, pos, result, result_size, wrap_width);
                let rect = Rect::from_min_max(pos, pos2(pos.x + wrap_width, bottom));
                access::text(ui, rect, Id::new("date-result"), &name, true);
            }
        }

        self.draw_date_popup(ui, full, selector);
    }

    // -------------------------------------------------------------- controls

    /// The borderless drop-down at the top that switches pages.
    fn draw_page_selector(&mut self, ui: &Ui, text_pos: Pos2) -> Rect {
        let p = self.palette;
        let painter = ui.painter();
        let label = painter.layout_no_wrap(
            self.date.page.title().to_string(),
            FontId::proportional(14.0),
            p.text,
        );
        let rect = Rect::from_min_size(
            pos2(text_pos.x - 12.0, text_pos.y),
            vec2(12.0 + label.size().x + 38.0, CONTROL_HEIGHT),
        );
        let name = tr("DateCalculationOption.AutomationName");
        let info = access::combo_box(name, self.date.page.title());
        let id = Id::new("date-page");
        let response = access::control(ui, p, rect, id, true, RADIUS, info);
        painter.rect_filled(rect, RADIUS, subtle_fill(p, &response));
        let label_pos = pos2(text_pos.x, rect.center().y - label.size().y / 2.0);
        painter.galley(label_pos, label, p.text);
        let chevron = icon_label(ICON_CHEVRON_DOWN, 12.0);
        paint_centered(
            painter,
            pos2(rect.right() - 20.0, rect.center().y),
            chevron,
            p.text,
        );
        if response.clicked() {
            self.date.popup = Some(Popup::Pages);
            self.opened_from(ui.ctx(), id);
        }
        rect
    }

    /// A date with a calendar glyph; a click opens the calendar.
    fn draw_date_picker(&mut self, ui: &Ui, field: DateField, text_pos: Pos2) {
        let p = self.palette;
        let date = self.date.date(field);
        let painter = ui.painter();
        let text = formats().date(date);
        let label = painter.layout_no_wrap(text.clone(), FontId::proportional(14.0), p.text);
        let rect = Rect::from_min_size(
            pos2(text_pos.x - 12.0, text_pos.y),
            vec2(12.0 + label.size().x + 36.0, CONTROL_HEIGHT),
        );
        // Windows' date pickers are named by their headers.
        let info = access::combo_box(field.header(), &text);
        let id = Id::new("date-picker").with(field);
        let response = access::control(ui, p, rect, id, true, RADIUS, info);
        painter.rect_filled(rect, RADIUS, subtle_fill(p, &response));
        let glyph_center = pos2(text_pos.x + label.size().x + 16.0, rect.center().y);
        painter.galley(
            pos2(text_pos.x, rect.center().y - label.size().y / 2.0),
            label,
            p.text,
        );
        let glyph = icon_label(ICON_CALENDAR, 16.0);
        paint_centered(painter, glyph_center, glyph, p.text);
        if response.clicked() {
            self.date.popup = Some(Popup::Calendar {
                field,
                anchor: rect,
                view: CalendarView::Month,
                shown: first_of_month(date),
            });
            self.opened_from(ui.ctx(), id);
        }
    }

    /// A years, months or days box; a click opens the 0–999 list.
    fn draw_offset_combo(&mut self, ui: &Ui, field: Offset, rect: Rect) {
        let p = self.palette;
        let value = self.date.offset(field).to_string();
        let id = Id::new("date-offset").with(field);
        let info = access::combo_box(field.label(), &value);
        let response = access::control(ui, p, rect, id, true, RADIUS, info);
        let fill = if response.is_pointer_button_down_on() {
            p.control_pressed
        } else if response.hovered() {
            p.control_hover
        } else {
            p.control
        };
        let painter = ui.painter();
        painter.rect_filled(rect, RADIUS, fill);
        painter.rect_stroke(
            rect,
            RADIUS,
            Stroke::new(1.0, p.control_border),
            StrokeKind::Inside,
        );
        painter.text(
            pos2(rect.left() + 12.0, rect.center().y),
            Align2::LEFT_CENTER,
            value,
            FontId::proportional(14.0),
            p.text,
        );
        let chevron = icon_label(ICON_CHEVRON_DOWN, 12.0);
        let chevron_pos = pos2(rect.right() - 18.0, rect.center().y);
        paint_centered(painter, chevron_pos, chevron, p.text_secondary);
        if response.clicked() {
            self.date.popup = Some(Popup::Offset {
                field,
                anchor: rect,
                scroll_pending: true,
            });
            self.opened_from(ui.ctx(), id);
        }
    }

    // ---------------------------------------------------------------- popups

    fn draw_date_popup(&mut self, ui: &mut Ui, full: Rect, selector: Rect) {
        let p = self.palette;
        let Some(popup) = self.date.popup else {
            self.date.wheel = 0.0;
            return;
        };
        if matches!(popup, Popup::Calendar { .. }) {
            ui.painter().rect_filled(full, 0.0, p.backdrop);
        }
        // A click anywhere outside the popup closes it ("light dismiss").
        let backdrop = ui.interact(full, Id::new("date-popup-backdrop"), Sense::CLICK);
        access::set_covered(ui.ctx(), false);
        match popup {
            Popup::Pages => self.draw_page_list(ui, full, selector),
            Popup::Calendar {
                field,
                anchor,
                view,
                shown,
            } => self.draw_calendar(ui, full, field, anchor, view, shown),
            Popup::Offset {
                field,
                anchor,
                scroll_pending,
            } => self.draw_offset_list(ui, full, field, anchor, scroll_pending),
        }
        if backdrop.clicked() {
            self.date.popup = None;
        }
    }

    fn draw_page_list(&mut self, ui: &Ui, full: Rect, selector: Rect) {
        let p = self.palette;
        const ITEM_HEIGHT: f32 = 36.0;
        const STRIDE: f32 = ITEM_HEIGHT + 2.0;
        let selected = Page::ALL.iter().position(|&page| page == self.date.page);
        let selected = selected.unwrap_or(0) as f32;
        // The selected item sits right over the drop-down, as in WinUI.
        let item_top = selector.center().y - ITEM_HEIGHT / 2.0;
        let top = (item_top - 4.0 - selected * STRIDE).max(full.top() + 4.0);
        let size = vec2(
            276.0_f32.min(full.width() - 10.0),
            8.0 + Page::ALL.len() as f32 * STRIDE - 2.0,
        );
        let rect = Rect::from_min_size(pos2(full.left() + 5.0, top), size);
        popup_frame(p, ui.painter(), rect, p.popup);
        ui.interact(rect, Id::new("date-page-list"), Sense::CLICK);

        for (i, page) in Page::ALL.into_iter().enumerate() {
            let item = Rect::from_min_size(
                rect.min + vec2(4.0, 4.0 + i as f32 * STRIDE),
                vec2(rect.width() - 8.0, ITEM_HEIGHT),
            );
            let id = Id::new("date-page-item").with(i);
            let selected = page == self.date.page;
            let focus = self.take_popup_focus(selected);
            if list_item(p, ui, id, item, page.title(), selected, focus) {
                self.date.set_page(page);
                self.date.popup = None;
            }
        }
    }

    fn draw_offset_list(
        &mut self,
        ui: &mut Ui,
        full: Rect,
        field: Offset,
        anchor: Rect,
        scroll_pending: bool,
    ) {
        let p = self.palette;
        let selected = self.date.offset(field) as usize;
        let fits = ((full.height() - 16.0) / LIST_ITEM_HEIGHT).floor() as usize;
        let visible = LIST_VISIBLE_ITEMS.min(fits).max(1);
        let height = visible as f32 * LIST_ITEM_HEIGHT + 8.0;
        // Like WinUI, put the selected value over the box when possible.
        let above = selected.min(visible / 2);
        let lowest_top = (full.bottom() - 4.0 - height).max(full.top() + 4.0);
        let top = (anchor.top() - 4.0 - above as f32 * LIST_ITEM_HEIGHT)
            .clamp(full.top() + 4.0, lowest_top);
        let rect = Rect::from_min_size(pos2(anchor.left(), top), vec2(anchor.width(), height));
        popup_frame(p, ui.painter(), rect, p.popup);
        ui.interact(rect, Id::new("date-offset-list"), Sense::CLICK);

        let mut chosen = None;
        ui.scope_builder(UiBuilder::new().max_rect(rect.shrink(4.0)), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut area = egui::ScrollArea::vertical()
                .id_salt(("date-offset-list", field))
                .auto_shrink(false);
            if scroll_pending {
                area = area.vertical_scroll_offset((selected - above) as f32 * LIST_ITEM_HEIGHT);
            }
            let rows = MAX_OFFSET as usize + 1;
            area.show_rows(ui, LIST_ITEM_HEIGHT, rows, |ui, range| {
                for value in range {
                    let size = vec2(ui.available_width(), LIST_ITEM_HEIGHT);
                    let (row, _) = ui.allocate_exact_size(size, Sense::hover());
                    let item = row.shrink2(vec2(0.0, 1.0));
                    let id = Id::new("date-offset-item").with(value);
                    let focus = self.take_popup_focus(value == selected);
                    let label = value.to_string();
                    if list_item(p, ui, id, item, &label, value == selected, focus) {
                        chosen = Some(value as u32);
                    }
                }
            });
        });

        if let Some(Popup::Offset { scroll_pending, .. }) = &mut self.date.popup {
            *scroll_pending = false;
        }
        if let Some(value) = chosen {
            self.date.set_offset(field, value);
            self.date.popup = None;
        }
    }

    // -------------------------------------------------------------- calendar

    fn draw_calendar(
        &mut self,
        ui: &Ui,
        full: Rect,
        field: DateField,
        anchor: Rect,
        view: CalendarView,
        shown: NaiveDate,
    ) {
        let p = self.palette;
        let width = CALENDAR_WIDTH.min(full.width() - 24.0);
        let height =
            CALENDAR_HEADER_HEIGHT + WEEKDAY_ROW_HEIGHT + 2.0 + 6.0 * CALENDAR_ROW_HEIGHT + 8.0;
        // Windows opens the calendar slightly above the date it edits.
        let top = (anchor.top() - 29.0)
            .min(full.bottom() - 8.0 - height)
            .max(full.top() + 8.0);
        let rect = Rect::from_min_size(pos2(full.left() + 12.0, top), vec2(width, height));

        let painter = ui.painter().clone();
        popup_frame(p, &painter, rect, p.calendar);
        let header = rect.with_max_y(rect.top() + CALENDAR_HEADER_HEIGHT);
        let top_corners = CornerRadius {
            nw: 8,
            ne: 8,
            sw: 0,
            se: 0,
        };
        painter.rect_filled(header.shrink(1.0), top_corners, p.calendar_header);
        painter.hline(
            rect.x_range(),
            header.bottom(),
            Stroke::new(1.0, p.calendar_line),
        );
        ui.interact(rect, Id::new("calendar"), Sense::CLICK);

        let mut action = None;

        // Header: the title zooms out, the arrows flip pages.
        let arrow_width = (width / 7.0).max(48.0);
        let title_button = Rect::from_min_max(
            pos2(rect.left() + 8.0, rect.top() + 4.0),
            pos2(rect.right() - 2.0 * arrow_width - 4.0, rect.top() + 44.0),
        );
        let zoom_out = match view {
            CalendarView::Month => Some(CalendarView::Year),
            CalendarView::Year => Some(CalendarView::Decade),
            CalendarView::Decade => None,
        };
        let title = match view {
            CalendarView::Month => formats().year_month(shown),
            CalendarView::Year => shown.year().to_string(),
            CalendarView::Decade => {
                let start = decade_start(shown);
                format!("{start} - {}", start + 9)
            }
        };
        let id = Id::new("calendar-title");
        if flat_button(p, ui, id, title_button, zoom_out.is_some(), &title) {
            action = zoom_out.map(|view| CalendarAction::Show(view, shown));
        }
        painter.text(
            pos2(title_button.left() + 8.0, title_button.center().y),
            Align2::LEFT_CENTER,
            title,
            FontId::new(14.0, semibold()),
            p.text,
        );

        // WinUI's own names for the arrows, which aren't in the Calculator's resources.
        let arrows = [
            (
                ICON_CARET_UP,
                false,
                rect.right() - 1.5 * arrow_width,
                "Previous",
            ),
            (
                ICON_CARET_DOWN,
                true,
                rect.right() - 0.5 * arrow_width,
                "Next",
            ),
        ];
        for (glyph, forward, x, name) in arrows {
            let target = step_page(view, shown, forward);
            let button =
                Rect::from_center_size(pos2(x, rect.top() + 24.0), vec2(arrow_width - 4.0, 40.0));
            let id = Id::new("calendar-arrow").with(forward);
            if flat_button(p, ui, id, button, target.is_some(), name) {
                action = target.map(|shown| CalendarAction::Show(view, shown));
            }
            let color = if target.is_some() {
                p.calendar_arrow
            } else {
                p.text_disabled
            };
            paint_centered(&painter, button.center(), icon_label(glyph, 8.0), color);
        }

        // The mouse wheel flips pages too, and so do Page Up and Page Down.
        if ui.rect_contains_pointer(rect) {
            self.date.wheel += ui.input(|i| wheel_delta(&i.events));
            if self.date.wheel.abs() >= WHEEL_STEP {
                let forward = self.date.wheel < 0.0;
                self.date.wheel = 0.0;
                if let Some(shown) = step_page(view, shown, forward) {
                    action = Some(CalendarAction::Show(view, shown));
                }
            }
        }
        for (key, forward) in [(Key::PageUp, false), (Key::PageDown, true)] {
            if ui.input(|i| i.key_pressed(key))
                && let Some(shown) = step_page(view, shown, forward)
            {
                action = Some(CalendarAction::Show(view, shown));
            }
        }

        let body = Rect::from_min_max(
            pos2(rect.left() + 8.0, header.bottom()),
            pos2(rect.right() - 8.0, rect.bottom() - 8.0),
        );
        let today = today();
        let selected = self.date.date(field);
        // Opened with the keyboard, the calendar focuses the selected date.
        let cells = Cells {
            today,
            selected,
            focus: self.take_popup_focus(true),
        };
        let picked = match view {
            CalendarView::Month => month_view(p, ui, &painter, body, shown, cells),
            CalendarView::Year => year_view(p, ui, body, shown, cells),
            CalendarView::Decade => decade_view(p, ui, body, shown, cells),
        };
        action = picked.or(action);

        match action {
            Some(CalendarAction::Pick(date)) => {
                self.date.set_date(field, date);
                self.date.popup = None;
            }
            Some(CalendarAction::Show(view, shown)) => {
                self.date.popup = Some(Popup::Calendar {
                    field,
                    anchor,
                    view,
                    shown,
                });
            }
            None => {}
        }
    }
}

/// What the calendar's cells need to know: today, the selected date, and whether the
/// selected cell takes the keyboard focus (a calendar opened with the keyboard).
#[derive(Clone, Copy)]
struct Cells {
    today: NaiveDate,
    selected: NaiveDate,
    focus: bool,
}

/// Days of a month: 6 weeks starting on Sunday, with neighbor months dimmed.
fn month_view(
    p: &Palette,
    ui: &Ui,
    painter: &Painter,
    body: Rect,
    shown: NaiveDate,
    cells: Cells,
) -> Option<CalendarAction> {
    let column = body.width() / 7.0;
    // The weekday names start at the region.s first day of the week.
    let week_start = formats().first_day;
    for i in 0..7 {
        let name = &formats().weekdays[(week_start.num_days_from_sunday() as usize + i) % 7];
        let center = pos2(
            body.left() + (i as f32 + 0.5) * column,
            body.top() + 2.0 + WEEKDAY_ROW_HEIGHT / 2.0,
        );
        painter.text(
            center,
            Align2::CENTER_CENTER,
            name,
            FontId::new(13.0, semibold()),
            p.text,
        );
    }

    let grid_top = body.top() + 2.0 + WEEKDAY_ROW_HEIGHT;
    let page_start = dates::month_page_start(shown.year(), shown.month(), week_start);
    let mut action = None;
    for i in 0..42 {
        let day = page_start + Days::new(i);
        if !(MIN_DATE..=MAX_DATE).contains(&day) {
            continue;
        }
        let (row, col) = ((i / 7) as f32, (i % 7) as f32);
        let cell = Rect::from_min_size(
            pos2(
                body.left() + col * column,
                grid_top + row * CALENDAR_ROW_HEIGHT,
            ),
            vec2(column, CALENDAR_ROW_HEIGHT),
        );
        let state = CellState {
            today: day == cells.today,
            selected: day == cells.selected,
            dimmed: day.month() != shown.month(),
            focus: cells.focus && day == cells.selected,
        };
        let id = Id::new("calendar-day").with(i);
        // Screen readers get the whole date, not just the day of the month.
        let label = day.day().to_string();
        if calendar_cell(p, ui, id, cell, &label, &formats().date(day), state) {
            action = Some(CalendarAction::Pick(day));
        }
    }
    action
}

/// Months of a year in a 4×4 grid; the first months of the next year are dimmed.
fn year_view(
    p: &Palette,
    ui: &Ui,
    body: Rect,
    shown: NaiveDate,
    cells: Cells,
) -> Option<CalendarAction> {
    let january = NaiveDate::from_ymd_opt(shown.year(), 1, 1)?;
    let mut action = None;
    for i in 0..16 {
        let month = january.checked_add_months(Months::new(i))?;
        if !(first_of_month(MIN_DATE)..=first_of_month(MAX_DATE)).contains(&month) {
            continue;
        }
        let same_month = |date: NaiveDate| first_of_month(date) == month;
        let state = CellState {
            today: same_month(cells.today),
            selected: same_month(cells.selected),
            dimmed: month.year() != shown.year(),
            focus: cells.focus && same_month(cells.selected),
        };
        let label = &formats().months[month.month0() as usize];
        let name = formats().year_month(month);
        let id = Id::new("calendar-month").with(i);
        let cell = grid_cell(body, i as usize);
        if calendar_cell(p, ui, id, cell, label, &name, state) {
            action = Some(CalendarAction::Show(CalendarView::Month, month));
        }
    }
    action
}

/// Years of a decade in a 4×4 grid; the years after the decade are dimmed.
fn decade_view(
    p: &Palette,
    ui: &Ui,
    body: Rect,
    shown: NaiveDate,
    cells: Cells,
) -> Option<CalendarAction> {
    let start = decade_start(shown);
    let mut action = None;
    for i in 0..16 {
        let year = start + i;
        if !(MIN_DATE.year()..=MAX_DATE.year()).contains(&year) {
            continue;
        }
        let state = CellState {
            today: cells.today.year() == year,
            selected: cells.selected.year() == year,
            dimmed: year > start + 9,
            focus: cells.focus && cells.selected.year() == year,
        };
        let id = Id::new("calendar-year").with(i);
        let cell = grid_cell(body, i as usize);
        let label = year.to_string();
        if calendar_cell(p, ui, id, cell, &label, &label, state) {
            let january = NaiveDate::from_ymd_opt(year, 1, 1)?;
            action = Some(CalendarAction::Show(CalendarView::Year, january));
        }
    }
    action
}

/// Cell `index` of the 4×4 grid used by the year and decade views.
fn grid_cell(body: Rect, index: usize) -> Rect {
    let size = vec2(body.width() / 4.0, (body.height() - 2.0) / 4.0);
    let (row, col) = ((index / 4) as f32, (index % 4) as f32);
    Rect::from_min_size(
        pos2(body.left() + col * size.x, body.top() + 2.0 + row * size.y),
        size,
    )
}

#[derive(Clone, Copy)]
struct CellState {
    today: bool,
    selected: bool,
    dimmed: bool,
    /// Take the keyboard focus.
    focus: bool,
}

/// A round calendar cell: today is filled with the accent color, the selected date
/// gets an accent ring. `name` is what a screen reader says. Returns `true` when clicked.
fn calendar_cell(
    p: &Palette,
    ui: &Ui,
    id: Id,
    cell: Rect,
    label: &str,
    name: &str,
    state: CellState,
) -> bool {
    let radius = (cell.width().min(cell.height()) / 2.0 - 1.0).min(28.0);
    let circle = Rect::from_center_size(cell.center(), vec2(2.0 * radius, 2.0 * radius));
    let info = access::item(WidgetType::Button, name, state.selected);
    let response = access::control(ui, p, circle, id, true, radius, info);
    if state.focus {
        response.request_focus();
    }
    let pressed = response.is_pointer_button_down_on();
    let hovered = response.hovered();
    let (fill, text) = if state.today {
        let fill = if pressed {
            p.accent_pressed
        } else if hovered {
            p.accent_hover
        } else {
            p.accent
        };
        (fill, p.text_on_accent)
    } else {
        let fill = if pressed {
            p.calendar_pressed
        } else if hovered {
            p.calendar_hover
        } else {
            Color32::TRANSPARENT
        };
        (fill, if state.dimmed { p.other_month } else { p.text })
    };
    let painter = ui.painter();
    painter.circle_filled(cell.center(), radius, fill);
    if state.selected && !state.today {
        painter.circle_stroke(cell.center(), radius - 1.0, Stroke::new(2.0, p.accent));
    }
    painter.text(
        cell.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        text,
    );
    response.clicked()
}

// -------------------------------------------------------------- helpers

/// A small gray label above a control.
fn caption(p: &Palette, painter: &Painter, pos: Pos2, text: &str) {
    painter.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::proportional(12.0),
        p.text_secondary,
    );
}

/// Wrapped result text; returns its bottom edge.
fn paragraph(
    p: &Palette,
    painter: &Painter,
    pos: Pos2,
    text: String,
    size: f32,
    wrap_width: f32,
) -> f32 {
    let galley = painter.layout(text, FontId::proportional(size), p.text, wrap_width);
    let bottom = pos.y + galley.size().y;
    painter.galley(pos, galley, p.text);
    bottom
}

/// An item of a drop-down list: highlighted when hovered or selected, with the accent
/// pill on the selected one. `focus` gives it the keyboard focus (the selected item of
/// a list opened with the keyboard). Returns `true` when clicked.
pub(super) fn list_item(
    p: &Palette,
    ui: &Ui,
    id: Id,
    rect: Rect,
    label: &str,
    selected: bool,
    focus: bool,
) -> bool {
    let info = access::item(WidgetType::SelectableLabel, label, selected);
    let response = access::control(ui, p, rect, id, true, RADIUS, info);
    if focus {
        response.request_focus();
    }
    let fill = if response.is_pointer_button_down_on() {
        p.popup_item_pressed
    } else if response.hovered() || selected {
        p.popup_item
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, RADIUS, fill);
    if selected {
        let pill =
            Rect::from_center_size(pos2(rect.left() + 5.0, rect.center().y), vec2(3.0, 16.0));
        painter.rect_filled(pill, 2.0, p.accent);
    }
    painter.text(
        pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        p.text,
    );
    response.clicked()
}

/// A transparent button that only shows a background on hover.
/// `name` is what a screen reader says.
fn flat_button(p: &Palette, ui: &Ui, id: Id, rect: Rect, enabled: bool, name: &str) -> bool {
    let info = access::button(enabled, name);
    let response = access::control(ui, p, rect, id, enabled, RADIUS, info);
    if enabled {
        let fill = if response.is_pointer_button_down_on() {
            p.calendar_pressed
        } else if response.hovered() {
            p.calendar_hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, RADIUS, fill);
    }
    enabled && response.clicked()
}

fn subtle_fill(p: &Palette, response: &egui::Response) -> Color32 {
    if response.is_pointer_button_down_on() {
        p.subtle_pressed
    } else if response.hovered() {
        p.subtle_hover
    } else {
        Color32::TRANSPARENT
    }
}

/// Shadow, background and border of a popup.
pub(super) fn popup_frame(p: &Palette, painter: &Painter, rect: Rect, fill: Color32) {
    let shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: p.popup_shadow,
    };
    painter.add(shadow.as_shape(rect, 8));
    painter.rect_filled(rect, 8.0, fill);
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, p.popup_border),
        StrokeKind::Inside,
    );
}

fn paint_centered(painter: &Painter, center: Pos2, label: LayoutJob, color: Color32) {
    let galley = painter.layout_job(label);
    painter.galley(center - galley.size() / 2.0, galley, color);
}

/// Sum of vertical mouse wheel movement this frame, in points
/// (positive when scrolling up).
fn wheel_delta(events: &[Event]) -> f32 {
    events
        .iter()
        .map(|event| match event {
            Event::MouseWheel { unit, delta, .. } => match unit {
                MouseWheelUnit::Point => delta.y,
                MouseWheelUnit::Line => delta.y * WHEEL_STEP,
                MouseWheelUnit::Page => delta.y * WHEEL_STEP * 10.0,
            },
            _ => 0.0,
        })
        .sum()
}

/// The previous or next page of a calendar view, if it is in the supported range.
fn step_page(view: CalendarView, shown: NaiveDate, forward: bool) -> Option<NaiveDate> {
    let months = Months::new(match view {
        CalendarView::Month => 1,
        CalendarView::Year => 12,
        CalendarView::Decade => 120,
    });
    let next = if forward {
        shown.checked_add_months(months)
    } else {
        shown.checked_sub_months(months)
    }?;
    let in_range = match view {
        CalendarView::Month => {
            (first_of_month(MIN_DATE)..=first_of_month(MAX_DATE)).contains(&next)
        }
        CalendarView::Year => (MIN_DATE.year()..=MAX_DATE.year()).contains(&next.year()),
        CalendarView::Decade => {
            decade_start(next) + 9 >= MIN_DATE.year() && decade_start(next) <= MAX_DATE.year()
        }
    };
    in_range.then_some(next)
}

fn first_of_month(date: NaiveDate) -> NaiveDate {
    date.with_day(1).unwrap_or(date)
}

fn decade_start(date: NaiveDate) -> i32 {
    date.year() - date.year().rem_euclid(10)
}

fn today() -> NaiveDate {
    dates::clamp(Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn from_date_carries_over_between_pages() {
        // Windows copies the From date to the page switched to (`DateCalcOption_Changed`).
        let mut state = DateState::new();
        state.set_date(DateField::From, date(2020, 1, 15));
        state.set_date(DateField::To, date(2021, 6, 1));
        state.set_page(Page::AddSubtract);
        assert_eq!(state.date(DateField::Start), date(2020, 1, 15));
        assert_eq!(state.resulting_date(), Some(date(2020, 1, 15)));

        state.set_date(DateField::Start, date(2024, 2, 29));
        state.set_page(Page::Difference);
        assert_eq!(state.date(DateField::From), date(2024, 2, 29));
        // Only the From date is shared.
        assert_eq!(state.date(DateField::To), date(2021, 6, 1));

        // Picking the page that is already shown keeps its date.
        state.set_date(DateField::From, date(2025, 5, 5));
        state.set_page(Page::Difference);
        assert_eq!(state.date(DateField::From), date(2025, 5, 5));
    }
}
