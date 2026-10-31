use egui::{vec2, Color32, CornerRadius, CursorIcon, Frame, Id, Margin, RichText, ScrollArea, Stroke};

use crate::state::{AppState, EmojiPickerTarget};
use crate::theme::{self, Tokens};

pub struct EmojiDef {
    pub emoji: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub keywords: &'static str,
}

pub const EMOJI_DATABASE: &[EmojiDef] = &[
    // Folders & Organization
    EmojiDef { emoji: "📁", name: "Folder", category: "Folders", keywords: "folder directory file documents" },
    EmojiDef { emoji: "📂", name: "Open Folder", category: "Folders", keywords: "open folder directory files" },
    EmojiDef { emoji: "🗂️", name: "Card Index", category: "Folders", keywords: "index dividers tabs organize" },
    EmojiDef { emoji: "🗃️", name: "File Box", category: "Folders", keywords: "box archive storage documents" },
    EmojiDef { emoji: "📦", name: "Package", category: "Folders", keywords: "package box shipping bundle" },
    EmojiDef { emoji: "🗄️", name: "File Cabinet", category: "Folders", keywords: "cabinet files storage office" },
    EmojiDef { emoji: "📋", name: "Clipboard", category: "Folders", keywords: "clipboard checklist board tasks" },
    EmojiDef { emoji: "📌", name: "Pushpin", category: "Folders", keywords: "pin pushpin note remember" },
    EmojiDef { emoji: "📍", name: "Round Pin", category: "Folders", keywords: "location pin place marker" },
    EmojiDef { emoji: "🏷️", name: "Label", category: "Folders", keywords: "tag label price category" },
    EmojiDef { emoji: "🔖", name: "Bookmark", category: "Folders", keywords: "bookmark mark save favorite" },
    EmojiDef { emoji: "📎", name: "Paperclip", category: "Folders", keywords: "clip paperclip attachment" },
    EmojiDef { emoji: "💼", name: "Briefcase", category: "Folders", keywords: "briefcase work business portfolio" },
    EmojiDef { emoji: "🎒", name: "Backpack", category: "Folders", keywords: "backpack school study travel" },

    // Notes & Documents
    EmojiDef { emoji: "📝", name: "Memo", category: "Notes", keywords: "memo note draft paper writing" },
    EmojiDef { emoji: "📄", name: "Document", category: "Notes", keywords: "document page file text sheet" },
    EmojiDef { emoji: "📑", name: "Page Curl", category: "Notes", keywords: "page tabs bookmark document" },
    EmojiDef { emoji: "📜", name: "Scroll", category: "Notes", keywords: "scroll ancient script paper" },
    EmojiDef { emoji: "📖", name: "Open Book", category: "Notes", keywords: "book reading guide manual" },
    EmojiDef { emoji: "📚", name: "Books", category: "Notes", keywords: "books library study learn" },
    EmojiDef { emoji: "📓", name: "Notebook", category: "Notes", keywords: "notebook journal diary" },
    EmojiDef { emoji: "📕", name: "Red Book", category: "Notes", keywords: "red book textbook novel" },
    EmojiDef { emoji: "📗", name: "Green Book", category: "Notes", keywords: "green book textbook novel" },
    EmojiDef { emoji: "📘", name: "Blue Book", category: "Notes", keywords: "blue book manual novel" },
    EmojiDef { emoji: "📙", name: "Orange Book", category: "Notes", keywords: "orange book diary novel" },
    EmojiDef { emoji: "✍️", name: "Writing Hand", category: "Notes", keywords: "write author draft sign" },
    EmojiDef { emoji: "🖊️", name: "Pen", category: "Notes", keywords: "pen ballpoint ink writing" },
    EmojiDef { emoji: "🖋️", name: "Fountain Pen", category: "Notes", keywords: "fountain pen luxury writing" },
    EmojiDef { emoji: "✏️", name: "Pencil", category: "Notes", keywords: "pencil draw sketch edit" },

    // Projects & Ideas
    EmojiDef { emoji: "💡", name: "Idea", category: "Projects", keywords: "idea lightbulb creative inspiration" },
    EmojiDef { emoji: "🚀", name: "Rocket", category: "Projects", keywords: "rocket launch startup project fast" },
    EmojiDef { emoji: "⭐", name: "Star", category: "Projects", keywords: "star favorite top feature bookmark" },
    EmojiDef { emoji: "🌟", name: "Glowing Star", category: "Projects", keywords: "star highlight sparkle shine" },
    EmojiDef { emoji: "🎯", name: "Target", category: "Projects", keywords: "target goal aim focus bullseye" },
    EmojiDef { emoji: "⚡", name: "Lightning", category: "Projects", keywords: "lightning bolt fast energy quick" },
    EmojiDef { emoji: "🔥", name: "Fire", category: "Projects", keywords: "fire hot trend urgent priority" },
    EmojiDef { emoji: "🎨", name: "Art Palette", category: "Projects", keywords: "palette canvas art design visual" },
    EmojiDef { emoji: "🔬", name: "Microscope", category: "Projects", keywords: "science research experiment lab" },
    EmojiDef { emoji: "🧪", name: "Test Tube", category: "Projects", keywords: "test chemistry experiment lab" },
    EmojiDef { emoji: "🔮", name: "Crystal Ball", category: "Projects", keywords: "crystal ball magic future predict" },
    EmojiDef { emoji: "🧩", name: "Puzzle", category: "Projects", keywords: "puzzle game component piece solve" },
    EmojiDef { emoji: "🛠️", name: "Tools", category: "Projects", keywords: "tools hammer wrench repair dev" },
    EmojiDef { emoji: "⚙️", name: "Gear", category: "Projects", keywords: "gear settings config machinery" },
    EmojiDef { emoji: "💻", name: "Laptop", category: "Projects", keywords: "laptop computer dev code software" },
    EmojiDef { emoji: "🖥️", name: "Desktop", category: "Projects", keywords: "computer screen workstation" },
    EmojiDef { emoji: "⌨️", name: "Keyboard", category: "Projects", keywords: "keyboard typing input dev" },
    EmojiDef { emoji: "📱", name: "Mobile", category: "Projects", keywords: "mobile phone app smartphone" },
    EmojiDef { emoji: "🌐", name: "Globe", category: "Projects", keywords: "web world internet network" },
    EmojiDef { emoji: "🤖", name: "Robot", category: "Projects", keywords: "robot ai bot machine agent" },
    EmojiDef { emoji: "🧠", name: "Brain", category: "Projects", keywords: "brain mind knowledge think" },
    EmojiDef { emoji: "🔍", name: "Magnifier Left", category: "Projects", keywords: "search find inspect discover" },
    EmojiDef { emoji: "🔎", name: "Magnifier Right", category: "Projects", keywords: "search zoom examine details" },

    // Status & Priorities
    EmojiDef { emoji: "✅", name: "Check Mark", category: "Status", keywords: "check done complete pass verify" },
    EmojiDef { emoji: "❌", name: "Cross Mark", category: "Status", keywords: "cross cancel delete fail no" },
    EmojiDef { emoji: "⚠️", name: "Warning", category: "Status", keywords: "warning alert caution danger" },
    EmojiDef { emoji: "❗", name: "Exclamation", category: "Status", keywords: "exclamation important notice attention" },
    EmojiDef { emoji: "❓", name: "Question", category: "Status", keywords: "question help query unknown" },
    EmojiDef { emoji: "🔴", name: "Red Circle", category: "Status", keywords: "red urgent critical high p0" },
    EmojiDef { emoji: "🟠", name: "Orange Circle", category: "Status", keywords: "orange high priority p1" },
    EmojiDef { emoji: "🟡", name: "Yellow Circle", category: "Status", keywords: "yellow medium in-progress p2" },
    EmojiDef { emoji: "🟢", name: "Green Circle", category: "Status", keywords: "green low done normal p3" },
    EmojiDef { emoji: "🔵", name: "Blue Circle", category: "Status", keywords: "blue info notice" },
    EmojiDef { emoji: "🟣", name: "Purple Circle", category: "Status", keywords: "purple category" },
    EmojiDef { emoji: "⚪", name: "White Circle", category: "Status", keywords: "white empty draft" },
    EmojiDef { emoji: "⚫", name: "Black Circle", category: "Status", keywords: "black dark archive" },
    EmojiDef { emoji: "🚩", name: "Red Flag", category: "Status", keywords: "flag milestone marker goal" },
    EmojiDef { emoji: "🏁", name: "Checkered Flag", category: "Status", keywords: "flag finish race done" },
    EmojiDef { emoji: "⏳", name: "Hourglass Flow", category: "Status", keywords: "hourglass wait pending time" },
    EmojiDef { emoji: "⌛", name: "Hourglass Done", category: "Status", keywords: "hourglass timer finish expired" },
    EmojiDef { emoji: "⏰", name: "Alarm Clock", category: "Status", keywords: "alarm clock reminder schedule" },
    EmojiDef { emoji: "⏱️", name: "Stopwatch", category: "Status", keywords: "stopwatch timer speed track" },
    EmojiDef { emoji: "📅", name: "Calendar", category: "Status", keywords: "calendar date daily day plan" },
    EmojiDef { emoji: "📆", name: "Tearoff Calendar", category: "Status", keywords: "calendar day date daily" },
    EmojiDef { emoji: "🔔", name: "Bell", category: "Status", keywords: "bell notification alert event chime" },
    EmojiDef { emoji: "🔕", name: "Muted Bell", category: "Status", keywords: "bell mute silent quiet" },
    EmojiDef { emoji: "🔒", name: "Lock", category: "Status", keywords: "lock private secure encrypted" },
    EmojiDef { emoji: "🔓", name: "Unlock", category: "Status", keywords: "unlock open public accessible" },
    EmojiDef { emoji: "🔑", name: "Key", category: "Status", keywords: "key secret token auth" },
    EmojiDef { emoji: "🛡️", name: "Shield", category: "Status", keywords: "shield security protect defense" },

    // Life & Miscellaneous
    EmojiDef { emoji: "🏠", name: "House", category: "Life", keywords: "home house personal domestic" },
    EmojiDef { emoji: "🏢", name: "Office", category: "Life", keywords: "office building company enterprise" },
    EmojiDef { emoji: "🎓", name: "Graduation", category: "Life", keywords: "graduation university degree study" },
    EmojiDef { emoji: "💰", name: "Money Bag", category: "Life", keywords: "money finance salary budget" },
    EmojiDef { emoji: "💳", name: "Credit Card", category: "Life", keywords: "credit card banking payments" },
    EmojiDef { emoji: "🛒", name: "Shopping Cart", category: "Life", keywords: "cart shopping buy store items" },
    EmojiDef { emoji: "👥", name: "People", category: "Life", keywords: "team group members users meeting" },
    EmojiDef { emoji: "👤", name: "Person", category: "Life", keywords: "user profile contact account" },
    EmojiDef { emoji: "💬", name: "Speech Bubble", category: "Life", keywords: "chat comment talk discussion" },
    EmojiDef { emoji: "💭", name: "Thought Bubble", category: "Life", keywords: "thought idea dream consider" },
    EmojiDef { emoji: "☕", name: "Coffee", category: "Life", keywords: "coffee tea break morning espresso" },
    EmojiDef { emoji: "🍵", name: "Tea", category: "Life", keywords: "tea cup drink relax green tea" },
    EmojiDef { emoji: "🍕", name: "Pizza", category: "Life", keywords: "pizza food lunch meal restaurant" },
    EmojiDef { emoji: "🍔", name: "Burger", category: "Life", keywords: "burger food fastfood lunch" },
    EmojiDef { emoji: "🍎", name: "Red Apple", category: "Life", keywords: "apple fruit healthy nutrition" },
    EmojiDef { emoji: "🌿", name: "Herb", category: "Life", keywords: "herb plant leaf nature green bio" },
    EmojiDef { emoji: "🌲", name: "Pine Tree", category: "Life", keywords: "tree pine forest nature wood" },
    EmojiDef { emoji: "🌸", name: "Cherry Blossom", category: "Life", keywords: "flower blossom spring sakura" },
    EmojiDef { emoji: "☀️", name: "Sun", category: "Life", keywords: "sun sunny weather bright light" },
    EmojiDef { emoji: "🌙", name: "Crescent Moon", category: "Life", keywords: "moon night dark evening sleep" },
    EmojiDef { emoji: "🌧️", name: "Rain", category: "Life", keywords: "rain weather storm cloud" },
    EmojiDef { emoji: "🎵", name: "Musical Note", category: "Life", keywords: "music song sound audio tune" },
    EmojiDef { emoji: "🎧", name: "Headphones", category: "Life", keywords: "headphones audio listen podcast" },
    EmojiDef { emoji: "🎮", name: "Video Game", category: "Life", keywords: "game controller gaming play fun" },
    EmojiDef { emoji: "✈️", name: "Airplane", category: "Life", keywords: "airplane flight travel trip vacation" },
    EmojiDef { emoji: "🚗", name: "Car", category: "Life", keywords: "car vehicle travel transport drive" },
    EmojiDef { emoji: "🚲", name: "Bicycle", category: "Life", keywords: "bicycle bike cycle sport ride" },
    EmojiDef { emoji: "🩺", name: "Stethoscope", category: "Life", keywords: "health medical doctor clinic hospital" },
    EmojiDef { emoji: "💊", name: "Pill", category: "Life", keywords: "pill medicine health pharmacy tablet" },
    EmojiDef { emoji: "❤️", name: "Red Heart", category: "Life", keywords: "heart love favorite like emotion" },
    EmojiDef { emoji: "💙", name: "Blue Heart", category: "Life", keywords: "blue heart like favorite" },
    EmojiDef { emoji: "💚", name: "Green Heart", category: "Life", keywords: "green heart like nature" },
    EmojiDef { emoji: "💜", name: "Purple Heart", category: "Life", keywords: "purple heart favorite royal" },
    EmojiDef { emoji: "👍", name: "Thumbs Up", category: "Life", keywords: "thumbs up approve agree good like" },
    EmojiDef { emoji: "👎", name: "Thumbs Down", category: "Life", keywords: "thumbs down disagree bad dislike" },
    EmojiDef { emoji: "🎉", name: "Party Popper", category: "Life", keywords: "party celebration celebrate event win" },
    EmojiDef { emoji: "🏆", name: "Trophy", category: "Life", keywords: "trophy winner champion award prize" },
    EmojiDef { emoji: "🥇", name: "1st Medal", category: "Life", keywords: "medal first gold achievement" },
];

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    let Some(target) = state.emoji_picker_target.clone() else {
        return;
    };

    let t = Tokens::get(ctx);
    let mut should_close = false;
    let mut selected_emoji: Option<String> = None;
    let mut should_clear = false;

    let target_label = match &target {
        EmojiPickerTarget::Note(id) => format!("For note: {}", id.name()),
        EmojiPickerTarget::Folder(path) => format!("For folder: {}", path),
        EmojiPickerTarget::NewFolderDialog => "For new folder".to_string(),
        EmojiPickerTarget::NewNoteDialog => "For new note".to_string(),
    };

    let target_color = match &target {
        EmojiPickerTarget::Note(id) => state.get_note_color(id),
        EmojiPickerTarget::Folder(path) => state.get_folder_color(path),
        EmojiPickerTarget::NewNoteDialog => state.new_note_color.as_deref().and_then(crate::state::parse_color_str),
        EmojiPickerTarget::NewFolderDialog => state.new_folder_color.as_deref().and_then(crate::state::parse_color_str),
    };
    let glyph_color = target_color.unwrap_or(Color32::WHITE);

    let modal = egui::Modal::new(Id::unique("searchable_emoji_picker_modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(theme::modal_frame(&t));

    let modal_resp = modal.show(ctx, |ui| {
        ui.set_width(420.0);
        ui.set_max_height(480.0);

        // Header
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(RichText::new("Choose Emoji").color(t.text).strong().size(16.0));
                ui.label(RichText::new(target_label).color(t.accent_text).size(11.5));
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::icon_close_button(ui, &t).clicked() {
                    should_close = true;
                }
            });
        });

        ui.add_space(10.0);

        // Search Bar
        Frame::NONE
            .fill(t.field)
            .stroke(Stroke::new(1.0, t.field_border))
            .corner_radius(CornerRadius::same(t.radius_sm as u8))
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon_search(ui, 13.0, t.text_faint);
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut state.emoji_picker_search)
                            .hint_text("Search emojis… (e.g. star, book, check, code, folder)")
                            .frame(Frame::NONE)
                            .desired_width(ui.available_width() - 24.0),
                    );
                    edit.request_focus();

                    if !state.emoji_picker_search.is_empty()
                        && theme::icon_close_button(ui, &t).clicked()
                    {
                        state.emoji_picker_search.clear();
                    }
                });
            });

        ui.add_space(8.0);

        // Category Filter Quick Chips
        let query = state.emoji_picker_search.trim().to_lowercase();
        let categories = ["All", "Folders", "Notes", "Projects", "Status", "Life"];
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for cat in categories {
                let is_cat_active = if cat == "All" {
                    query.is_empty()
                } else {
                    query == cat.to_lowercase()
                };

                let (bg, stroke, text_color) = if is_cat_active {
                    (t.accent_soft, Stroke::new(1.0, t.accent_border), t.accent_text)
                } else {
                    (t.field, Stroke::new(1.0, t.field_border), t.text_dim)
                };

                let mut rt = RichText::new(cat).size(11.0).color(text_color);
                if is_cat_active {
                    rt = rt.strong();
                }

                if ui
                    .add(
                        egui::Button::new(rt)
                            .fill(bg)
                            .stroke(stroke)
                            .corner_radius(CornerRadius::same(t.radius_sm as u8)),
                    )
                    .clicked()
                {
                    if cat == "All" {
                        state.emoji_picker_search.clear();
                    } else {
                        state.emoji_picker_search = cat.to_lowercase();
                    }
                }
            }
        });

        ui.add_space(8.0);

        // Color selector row (same as quick capture)
        let curr_color_name: Option<String> = match &target {
            EmojiPickerTarget::Note(id) => state.get_note_color_name(id),
            EmojiPickerTarget::Folder(path) => state.get_folder_color_name(path),
            EmojiPickerTarget::NewNoteDialog => state.new_note_color.clone(),
            EmojiPickerTarget::NewFolderDialog => state.new_folder_color.clone(),
        };

        ui.horizontal(|ui| {
            ui.label(RichText::new("Color:").size(12.0).color(t.text_dim));
            ui.add_space(4.0);
            if let Some(action) = theme::color_picker_dots(ui, &t, state.settings.effective_color_palette(), curr_color_name.as_deref()) {
                match action {
                    theme::ColorPickerAction::SetColor(new_color) => {
                        match &target {
                            EmojiPickerTarget::Note(id) => {
                                state.set_note_color(id, new_color.as_deref());
                            }
                            EmojiPickerTarget::Folder(path) => {
                                state.set_folder_color(path, new_color.as_deref());
                            }
                            EmojiPickerTarget::NewNoteDialog => {
                                state.new_note_color = new_color;
                            }
                            EmojiPickerTarget::NewFolderDialog => {
                                state.new_folder_color = new_color;
                            }
                        }
                    }
                    theme::ColorPickerAction::OpenCustomColor => {
                        let custom_target = match &target {
                            EmojiPickerTarget::Note(id) => crate::state::CustomColorTarget::Note(id.clone()),
                            EmojiPickerTarget::Folder(path) => crate::state::CustomColorTarget::Folder(path.clone()),
                            EmojiPickerTarget::NewNoteDialog => crate::state::CustomColorTarget::NewNoteDialog,
                            EmojiPickerTarget::NewFolderDialog => crate::state::CustomColorTarget::NewFolderDialog,
                        };
                        state.open_custom_color_picker(custom_target, curr_color_name.as_deref());
                    }
                }
            }
        });

        ui.add_space(6.0);
        ui.label(RichText::new("Emoji:").size(12.0).color(t.text_dim));
        ui.add_space(2.0);

        // Filter database
        let filtered: Vec<&EmojiDef> = EMOJI_DATABASE
            .iter()
            .filter(|item| {
                if query.is_empty() {
                    true
                } else {
                    item.name.to_lowercase().contains(&query)
                        || item.category.to_lowercase().contains(&query)
                        || item.keywords.to_lowercase().contains(&query)
                }
            })
            .collect();

        // Emojis Grid
        ScrollArea::vertical()
            .id_salt("emoji_picker_scroll")
            .max_height(260.0)
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("No matching emojis found").color(t.text_faint).size(12.5));
                    });
                    ui.add_space(20.0);
                } else {
                    let col_count = 8;
                    let btn_size = vec2(38.0, 36.0);

                    for chunk in filtered.chunks(col_count) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                            for def in chunk {
                                let (rect, resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
                                let hovered = resp.hovered();
                                if hovered {
                                    ui.painter().rect_filled(
                                        rect,
                                        CornerRadius::same(t.radius_sm as u8),
                                        t.hover,
                                    );
                                    ui.painter().rect_stroke(
                                        rect,
                                        CornerRadius::same(t.radius_sm as u8),
                                        Stroke::new(1.0, t.card_border),
                                        egui::StrokeKind::Inside,
                                    );
                                }

                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    def.emoji,
                                    egui::FontId::proportional(18.0),
                                    glyph_color,
                                );

                                if resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text(format!("{} ({})", def.name, def.category))
                                    .clicked()
                                {
                                    selected_emoji = Some(def.emoji.to_string());
                                    should_close = true;
                                }
                            }
                        });
                        ui.add_space(2.0);
                    }
                }
            });

        ui.add_space(10.0);
        theme::bottom_hairline(ui, ui.max_rect(), &t);
        ui.add_space(8.0);

        // Footer Actions
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("Clear / Default").color(t.text_dim).size(12.0))
                .on_hover_text("Remove custom emoji")
                .clicked()
            {
                should_clear = true;
                should_close = true;
            }

            let count_label = format!("{} emojis", filtered.len());
            ui.label(RichText::new(count_label).color(t.text_faint).size(11.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::ghost_button(ui, &t, RichText::new("Cancel").color(t.text_dim)).clicked() {
                    should_close = true;
                }
            });
        });
    });

    if let Some(emoji) = selected_emoji {
        match &target {
            EmojiPickerTarget::Note(id) => {
                state.set_note_emoji(id, Some(&emoji));
                state.set_status(format!("Set note icon to {emoji}"));
            }
            EmojiPickerTarget::Folder(path) => {
                state.set_folder_emoji(path, Some(&emoji));
                state.set_status(format!("Set folder icon to {emoji}"));
            }
            EmojiPickerTarget::NewFolderDialog => {
                state.new_folder_emoji = Some(emoji);
            }
            EmojiPickerTarget::NewNoteDialog => {
                state.new_note_emoji = Some(emoji);
            }
        }
    } else if should_clear {
        match &target {
            EmojiPickerTarget::Note(id) => {
                state.set_note_emoji(id, None);
                state.set_status("Cleared note icon");
            }
            EmojiPickerTarget::Folder(path) => {
                state.set_folder_emoji(path, None);
                state.set_status("Reset folder icon to default");
            }
            EmojiPickerTarget::NewFolderDialog => {
                state.new_folder_emoji = None;
            }
            EmojiPickerTarget::NewNoteDialog => {
                state.new_note_emoji = None;
            }
        }
    }

    if modal_resp.should_close() || should_close {
        state.emoji_picker_target = None;
        state.emoji_picker_search.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_database_completeness() {
        assert!(EMOJI_DATABASE.len() > 100);
        // Verify common emojis exist
        assert!(EMOJI_DATABASE.iter().any(|e| e.emoji == "📁"));
        assert!(EMOJI_DATABASE.iter().any(|e| e.emoji == "📝"));
        assert!(EMOJI_DATABASE.iter().any(|e| e.emoji == "🚀"));
        assert!(EMOJI_DATABASE.iter().any(|e| e.emoji == "📅"));
    }

    #[test]
    fn test_emoji_search_filtering() {
        let query = "rocket";
        let matches: Vec<_> = EMOJI_DATABASE
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(query)
                    || e.keywords.to_lowercase().contains(query)
                    || e.category.to_lowercase().contains(query)
            })
            .collect();

        assert!(!matches.is_empty());
        assert!(matches.iter().any(|e| e.emoji == "🚀"));

        // Search category "Folders"
        let folder_matches: Vec<_> = EMOJI_DATABASE
            .iter()
            .filter(|e| e.category.eq_ignore_ascii_case("Folders"))
            .collect();
        assert!(folder_matches.len() >= 10);
    }
}

