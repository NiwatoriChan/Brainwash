//! Emoji picker modal overlay plugin for Brainwash.

use egui::{vec2, Color32, CornerRadius, CursorIcon, Frame, Id, Margin, RichText, ScrollArea, Stroke};
use mybrain_core::events_bus::AppEvent;
use mybrain_core::frontmatter::Frontmatter;
use mybrain_core::index::LinkIndex;
use mybrain_core::note::NoteId;

use crate::plugins::{MyBrainPlugin, PluginCommand, PluginContext, PluginViewType};
use crate::theme::{self, Tokens};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmojiPickerTarget {
    InsertText,
    Note(NoteId),
    Folder(String),
    NewFolderDialog,
    NewNoteDialog,
}

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

pub struct EmojiPlugin {
    pub search_query: String,
    pub target: Option<EmojiPickerTarget>,
    pub selected_category: Option<&'static str>,
}

impl EmojiPlugin {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            target: None,
            selected_category: None,
        }
    }

    pub fn open_for(&mut self, target: EmojiPickerTarget) {
        self.target = Some(target);
        self.search_query.clear();
        self.selected_category = None;
    }
}

impl Default for EmojiPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MyBrainPlugin for EmojiPlugin {
    fn id(&self) -> &'static str {
        "core.emoji"
    }

    fn name(&self) -> &'static str {
        "Emoji Picker"
    }

    fn description(&self) -> &'static str {
        "Search and insert emojis into notes or customize folder and note icons"
    }

    fn view_type(&self) -> PluginViewType {
        PluginViewType::ModalOverlay
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![PluginCommand {
            id: "insert_emoji".into(),
            title: "Insert Emoji".into(),
            shortcut: None,
            plugin_id: "core.emoji",
        }]
    }

    fn execute_command(&mut self, command_id: &str, _ctx: &mut PluginContext) {
        if command_id == "insert_emoji" {
            self.open_for(EmojiPickerTarget::InsertText);
        }
    }

    fn on_event(&mut self, event: &AppEvent, _ctx: &mut PluginContext) {
        if let AppEvent::OpenModal(ref modal_id) = event {
            if modal_id == "emoji_picker" {
                self.open_for(EmojiPickerTarget::InsertText);
            } else if let Some(note_str) = modal_id.strip_prefix("emoji_note:") {
                if let Ok(id) = NoteId::new(note_str) {
                    self.open_for(EmojiPickerTarget::Note(id));
                }
            } else if let Some(folder_str) = modal_id.strip_prefix("emoji_folder:") {
                self.open_for(EmojiPickerTarget::Folder(folder_str.to_string()));
            }
        }
    }

    fn render_modal(&mut self, ctx_egui: &egui::Context, ctx: &mut PluginContext) {
        let Some(target) = self.target.clone() else {
            return;
        };

        let t = Tokens::get(ctx_egui);
        let mut should_close = false;
        let mut selected_emoji: Option<String> = None;
        let mut should_clear = false;

        let target_label = match &target {
            EmojiPickerTarget::InsertText => "Insert into editor".to_string(),
            EmojiPickerTarget::Note(id) => format!("For note: {}", id.name()),
            EmojiPickerTarget::Folder(path) => format!("For folder: {}", path),
            EmojiPickerTarget::NewFolderDialog => "For new folder".to_string(),
            EmojiPickerTarget::NewNoteDialog => "For new note".to_string(),
        };

        let modal = egui::Modal::new(Id::unique("searchable_emoji_picker_modal"))
            .backdrop_color(Color32::from_black_alpha(150))
            .frame(theme::modal_frame(&t));

        let modal_resp = modal.show(ctx_egui, |ui| {
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
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("Search emojis… (e.g. star, book, check, code, folder)")
                                .frame(Frame::NONE)
                                .desired_width(ui.available_width() - 24.0),
                        );
                        edit.request_focus();

                        if !self.search_query.is_empty()
                            && theme::icon_close_button(ui, &t).clicked()
                        {
                            self.search_query.clear();
                        }
                    });
                });

            ui.add_space(8.0);

            // Category Filter Quick Chips
            let query = self.search_query.trim().to_lowercase();
            let categories = ["All", "Folders", "Notes", "Projects", "Status", "Life"];

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                for cat in categories {
                    let is_sel = if cat == "All" {
                        self.selected_category.is_none() && query.is_empty()
                    } else {
                        self.selected_category == Some(cat)
                    };

                    let bg = if is_sel { t.accent.gamma_multiply(0.2) } else { t.field };
                    let border = if is_sel { t.accent } else { t.field_border };
                    let fg = if is_sel { t.accent_text } else { t.text_dim };

                    let resp = ui.add(
                        egui::Button::new(RichText::new(cat).size(11.5).color(fg))
                            .fill(bg)
                            .stroke(Stroke::new(1.0, border))
                            .corner_radius(CornerRadius::same(t.radius_sm as u8))
                            .min_size(vec2(0.0, 22.0)),
                    );

                    if resp.clicked() {
                        if cat == "All" {
                            self.selected_category = None;
                            self.search_query.clear();
                        } else {
                            self.selected_category = Some(cat);
                        }
                    }
                }
            });

            ui.add_space(8.0);
            theme::bottom_hairline(ui, ui.max_rect(), &t);
            ui.add_space(8.0);

            // Filter emojis
            let filtered: Vec<&'static EmojiDef> = EMOJI_DATABASE
                .iter()
                .filter(|def| {
                    if let Some(cat) = self.selected_category {
                        if !def.category.eq_ignore_ascii_case(cat) {
                            return false;
                        }
                    }
                    if !query.is_empty() {
                        let name_m = def.name.to_lowercase().contains(&query);
                        let kw_m = def.keywords.to_lowercase().contains(&query);
                        let cat_m = def.category.to_lowercase().contains(&query);
                        name_m || kw_m || cat_m
                    } else {
                        true
                    }
                })
                .collect();

            // Emoji Grid
            ScrollArea::vertical()
                .max_height(260.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if filtered.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(30.0);
                            ui.label(RichText::new("No emojis match your search").color(t.text_faint).size(13.0));
                        });
                    } else {
                        let cols = 8;
                        let item_size = vec2(42.0, 42.0);

                        egui::Grid::new("emoji_picker_grid")
                            .spacing(vec2(4.0, 4.0))
                            .show(ui, |ui| {
                                for (idx, def) in filtered.iter().enumerate() {
                                    let resp = ui.add(
                                        egui::Button::new(
                                            RichText::new(def.emoji).size(20.0),
                                        )
                                        .fill(t.field)
                                        .stroke(Stroke::new(1.0, t.field_border))
                                        .corner_radius(CornerRadius::same(t.radius_sm as u8))
                                        .min_size(item_size),
                                    )
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_ui(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(def.emoji).size(18.0));
                                            ui.vertical(|ui| {
                                                ui.label(RichText::new(def.name).strong().color(t.text).size(12.0));
                                                ui.label(RichText::new(def.category).color(t.text_faint).size(10.5));
                                            });
                                        });
                                    });

                                    if resp.clicked() {
                                        selected_emoji = Some(def.emoji.to_string());
                                        should_close = true;
                                    }

                                    if (idx + 1) % cols == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    }
                });

            ui.add_space(10.0);
            theme::bottom_hairline(ui, ui.max_rect(), &t);
            ui.add_space(8.0);

            // Footer actions: Reset / Remove icon
            ui.horizontal(|ui| {
                if matches!(target, EmojiPickerTarget::Folder(_) | EmojiPickerTarget::Note(_)) {
                    if theme::ghost_button(ui, &t, RichText::new("Remove custom icon").color(t.danger).size(12.0)).clicked() {
                        should_clear = true;
                        should_close = true;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::ghost_button(ui, &t, "Cancel").clicked() {
                        should_close = true;
                    }
                });
            });
        });

        if let Some(emoji) = selected_emoji {
            match target {
                EmojiPickerTarget::InsertText => {
                    ctx.emit(AppEvent::InsertText(emoji));
                }
                EmojiPickerTarget::Note(ref id) => {
                    if let Some(note) = ctx.vault.get(id) {
                        let (mut fm, body) = Frontmatter::parse(&note.content);
                        fm.set("icon", emoji.trim());
                        let new_content = if fm.is_empty() {
                            body.trim_start_matches('\n').to_string()
                        } else {
                            format!("{}{}", fm.render(), body.trim_start_matches('\n'))
                        };
                        let _ = ctx.vault.save(id, &new_content);
                        *ctx.index = LinkIndex::build(ctx.vault);
                        ctx.emit(AppEvent::NoteModified(id.clone()));
                    }
                    ctx.toast(format!("Set note icon to {emoji}"));
                }
                EmojiPickerTarget::Folder(ref path) => {
                    let clean = path.trim_end_matches('/').to_string();
                    ctx.settings.folder_emojis.insert(clean, emoji.clone());
                    let _ = ctx.settings.save_to_file(&std::path::PathBuf::from("settings.json"));
                    ctx.toast(format!("Set folder icon to {emoji}"));
                }
                EmojiPickerTarget::NewFolderDialog => {
                    // Signal to shell dialogs
                    ctx.emit(AppEvent::StatusMessage(format!("EMOJI_NEW_FOLDER:{emoji}")));
                }
                EmojiPickerTarget::NewNoteDialog => {
                    // Signal to shell dialogs
                    ctx.emit(AppEvent::StatusMessage(format!("EMOJI_NEW_NOTE:{emoji}")));
                }
            }
        }

        if should_clear {
            match target {
                EmojiPickerTarget::Note(ref id) => {
                    if let Some(note) = ctx.vault.get(id) {
                        let (mut fm, body) = Frontmatter::parse(&note.content);
                        fm.remove("icon");
                        let new_content = if fm.is_empty() {
                            body.trim_start_matches('\n').to_string()
                        } else {
                            format!("{}{}", fm.render(), body.trim_start_matches('\n'))
                        };
                        let _ = ctx.vault.save(id, &new_content);
                        *ctx.index = LinkIndex::build(ctx.vault);
                        ctx.emit(AppEvent::NoteModified(id.clone()));
                    }
                    ctx.toast("Reset note icon to default");
                }
                EmojiPickerTarget::Folder(ref path) => {
                    let clean = path.trim_end_matches('/').to_string();
                    ctx.settings.folder_emojis.remove(&clean);
                    let _ = ctx.settings.save_to_file(&std::path::PathBuf::from("settings.json"));
                    ctx.toast("Reset folder icon to default");
                }
                _ => {}
            }
        }

        if modal_resp.should_close() || should_close {
            self.target = None;
            self.search_query.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_database_completeness() {
        assert!(EMOJI_DATABASE.len() > 100);
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

        let folder_matches: Vec<_> = EMOJI_DATABASE
            .iter()
            .filter(|e| e.category.eq_ignore_ascii_case("Folders"))
            .collect();
        assert!(folder_matches.len() >= 10);
    }
}
