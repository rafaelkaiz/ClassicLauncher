//! A combo box where typing narrows the list: press `m` and only the entries starting with M are shown; Backspace undoes a letter,
//! Enter picks the first match. If nothing starts with the typed text, entries that contain it are shown instead.

use crate::i18n::tr;
use eframe::egui::{self, Event, Key, RichText, Ui};

/// Returns true when the selection changed.
pub fn pick(ui: &mut Ui, id: &str, width: f32, items: &[String], selected: &mut usize) -> bool {
    let fid = ui.scope_id().with(("pick-filter", id));
    let current = items.get(*selected).cloned().unwrap_or_default();
    let mut changed = false;
    let r = egui::ComboBox::from_id_salt(id).selected_text(current).width(width).height(520.0).show_ui(ui, |ui| {
        let mut typed: String = ui.data(|d| d.get_temp(fid)).unwrap_or_default();
        let mut submit = false;
        for e in ui.input(|i| i.events.clone()) {
            match e {
                Event::Text(t) => typed.push_str(&t),
                Event::Key { key: Key::Backspace, pressed: true, .. } => {
                    typed.pop();
                }
                Event::Key { key: Key::Enter, pressed: true, .. } => submit = true,
                _ => {}
            }
        }
        ui.data_mut(|d| d.insert_temp(fid, typed.clone()));

        let low = typed.to_lowercase();
        let mut shown: Vec<usize> = (0..items.len()).filter(|&i| items[i].to_lowercase().starts_with(&low)).collect();
        if shown.is_empty() {
            shown = (0..items.len()).filter(|&i| items[i].to_lowercase().contains(&low)).collect();
        }
        if typed.is_empty() {
            ui.label(RichText::new(tr("Type to search")).italics().weak());
        } else {
            ui.label(RichText::new(format!("{} {}", tr("Search:"), typed)).strong());
        }
        if shown.is_empty() {
            ui.label(tr("No results"));
        }
        for &i in &shown {
            if ui.selectable_label(i == *selected, &items[i]).clicked() {
                *selected = i;
                changed = true;
            }
        }
        if submit {
            if let Some(&i) = shown.first() {
                *selected = i;
                changed = true;
                ui.close();
            }
        }
    });
    if r.inner.is_none() {
        ui.data_mut(|d| d.remove::<String>(fid));
    }
    changed
}
