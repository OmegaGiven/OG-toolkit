use iced::{
    futures::SinkExt,
    widget::{button, column, container, image, mouse_area, row, text, text_input, Space},
    Background, Border, Color, Element, Length, Point, Size, Subscription, Task,
};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use crate::desktop::{load_app_registry, AppEntry};
use crate::filesystem::{self, FileEntry, home_dir, read_dir_entries, DriveInfo, FileDetails};
use crate::panes::{filelist, sidebar, toolbar};

/// Caps how much of a tree a background search will walk, so an accidental
/// search from e.g. `/` or a huge cache dir can't run forever.
const SEARCH_RESULT_CAP: usize = 500;


// ── Color palette (loaded from ~/.config/sway-power/config.json via og-config/og-theme) ──
const APP_TINT_SEED: &str = "og-files";

pub fn load_theme_colors() -> (Color, Color, Color, Color, Color, Color, Color) {
    let cfg = og_config::Config::load();
    let c = og_theme::AppColors::from_config(&cfg, APP_TINT_SEED);
    let selected = Color {
        r: c.accent.r * 0.25 + c.bar_bg.r * 0.75,
        g: c.accent.g * 0.25 + c.bar_bg.g * 0.75,
        b: c.accent.b * 0.25 + c.bar_bg.b * 0.75,
        a: 1.0,
    };
    (c.bar_bg, c.sec_bg, c.text, c.accent, c.dim_text, selected, c.surface)
}

static THEME: std::sync::LazyLock<std::sync::RwLock<(Color,Color,Color,Color,Color,Color,Color)>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(load_theme_colors()));

pub const fn init_colors() {}  // no-op; LazyLock inits on first access

/// Re-reads the shared theme config and swaps it in — lets og-settings'
/// Apply & Save take effect here without closing/reopening this window.
/// Every `BG()`/`TEXT()`/etc. call site is unaffected since they already go
/// through a function call, not a direct field read.
pub fn reload_theme_colors() {
    if let Ok(mut guard) = THEME.write() {
        *guard = load_theme_colors();
    }
}

pub fn BG()          -> Color { THEME.read().unwrap().0 }
pub fn SEC_BG()      -> Color { THEME.read().unwrap().1 }
pub fn TEXT()        -> Color { THEME.read().unwrap().2 }
pub fn ACCENT()      -> Color { THEME.read().unwrap().3 }
pub fn MUTED()       -> Color { THEME.read().unwrap().4 }
pub fn SELECTED_BG() -> Color { THEME.read().unwrap().5 }
pub fn SURFACE()     -> Color { THEME.read().unwrap().6 }

// ── Types ──────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode { List, Grid }

#[derive(Debug, Clone, PartialEq)]
pub enum SortBy { Name, Size, Modified, Kind }

#[derive(Debug, Clone)]
pub enum ClipboardOp {
    Copy(Vec<PathBuf>),
    Cut(Vec<PathBuf>),
}

#[derive(Debug, Clone)]
pub enum ContextMenuKind {
    Entry { path: PathBuf, is_dir: bool },
    Background,
}

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub kind: ContextMenuKind,
    pub renaming: bool,
    pub rename_text: String,
}

#[derive(Debug, Clone)]
pub struct OpenWithDialog {
    pub path: PathBuf,
    pub mime: String,
    pub search: String,
}

#[derive(Debug, Clone)]
pub enum PendingAction {
    DeletePermanently(Vec<PathBuf>),
    EmptyTrash,
}

#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub message: String,
    pub action: PendingAction,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaneDrag {
    Sidebar,
    Preview,
}

const SIDEBAR_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 140.0..=400.0;
const PREVIEW_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 180.0..=500.0;
const DIVIDER_WIDTH: f32 = 5.0;
/// Pixels the cursor must move (from where the mouse went down) before a
/// press counts as a file drag instead of a click.
const FILE_DRAG_THRESHOLD: f32 = 6.0;

// ── Messages ───────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum Message {
    CheckThemeReload,
    Navigate(PathBuf),
    NavigateBack,
    NavigateForward,
    NavigateUp,
    NavigateHome,
    EntryDoubleClicked(PathBuf),
    EntryPressed(PathBuf),
    EntryReleased(PathBuf),
    EntryHoverEnter(PathBuf),
    EntryHoverExit(PathBuf),
    SelectAll,
    SearchChanged(String),
    SearchSubmit,
    SearchBatch(u64, Vec<FileEntry>),
    SearchDone(u64),
    PreviewThumbnailReady(u64, Result<PathBuf, String>),
    ViewModeToggle,
    SortChanged(SortBy),
    ToggleGroupFolders,
    ShowHiddenToggle,
    PathBarEdit(String),
    PathBarSubmit,
    NewFolder,
    NewFile,
    ContextMenuClose,
    ContextMenuOpenEntry(PathBuf),
    ContextMenuOpenBackground,
    OpenDefault(PathBuf),
    OpenWith(PathBuf, String),
    OpenWithBrowse(PathBuf),
    OpenWithSearch(String),
    OpenWithSetDefault(String, String),
    OpenWithClose,
    OpenTerminalHere(PathBuf),
    OpenInTerminalWithPath(PathBuf),
    CopyPath(PathBuf),
    CopyName(PathBuf),
    Compress,
    CompressZip,
    Extract(PathBuf),
    Copy,
    Cut,
    Paste,
    Delete,
    DeletePermanently,
    RestoreFromTrash(PathBuf),
    EmptyTrash,
    ConfirmYes,
    ConfirmNo,
    Duplicate,
    ToggleBookmark(PathBuf),
    RenameStart,
    RenameText(String),
    RenameSubmit,
    SidebarBookmark(PathBuf),
    Refresh,
    CursorMoved(Point),
    PaneDragStart(PaneDrag),
    PaneDragEnd,
    WindowResized(Size),
    Noop,
    WindowIdReady(Option<iced::window::Id>),
    WaylandDisplayReady(Option<usize>),
}

// ── App ────────────────────────────────────────────────────────────────────────
pub struct App {
    pub current_path: PathBuf,
    pub history: Vec<PathBuf>,
    pub forward_stack: Vec<PathBuf>,
    pub entries: Vec<FileEntry>,
    pub selected: HashSet<PathBuf>,
    pub search_query: String,
    pub search_results: Vec<FileEntry>,
    pub search_handle: Option<iced::task::Handle>,
    pub search_generation: u64,
    pub searching: bool,
    pub view_mode: ViewMode,
    pub clipboard: Option<ClipboardOp>,
    pub sort_by: SortBy,
    pub sort_asc: bool,
    pub group_folders: bool,
    pub path_bar_editing: bool,
    pub path_bar_text: String,
    pub context_menu: Option<ContextMenu>,
    pub open_with: Option<OpenWithDialog>,
    pub confirm: Option<ConfirmDialog>,
    pub preview: Option<FileDetails>,
    pub preview_thumbnail: Option<PathBuf>,
    pub preview_thumbnail_loading: bool,
    pub preview_thumbnail_generation: u64,
    pub show_hidden: bool,
    pub app_registry: Vec<AppEntry>,
    pub status_message: String,
    pub devices: Vec<DriveInfo>,
    pub network_drives: Vec<DriveInfo>,
    pub recent: Vec<PathBuf>,
    pub bookmarks: Vec<PathBuf>,
    pub cursor_pos: Point,
    pub context_menu_pos: Point,
    pub sidebar_width: f32,
    pub preview_width: f32,
    pub pane_drag: Option<PaneDrag>,
    pub drag_preview_x: f32,
    pub window_size: Size,
    /// Set on press, cleared on the matching release — a click that never
    /// crosses `FILE_DRAG_THRESHOLD` resolves as a normal
    /// select/navigate; one that does gets promoted to `file_drag`.
    pub press_origin: Option<(PathBuf, Point)>,
    /// Files/folders currently being dragged to another folder, once the
    /// press has moved far enough to count as a drag rather than a click.
    pub file_drag: Option<Vec<PathBuf>>,
    /// Folder row currently under the cursor while `file_drag` is active —
    /// purely for the drop-target highlight.
    pub drop_hover: Option<PathBuf>,
    /// `None` until the real Wayland display handle round-trip finishes;
    /// `Some` once the cross-app drag-out worker thread is up and ready.
    pub drag_tx: Option<calloop::channel::Sender<og_wayland::DragRequest>>,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let start = home_dir();
        let app_registry = load_app_registry();
        let entries = read_dir_entries(&start, false);
        let status = format!("{} items", entries.len());
        let (devices, network_drives) = filesystem::list_drives();
        let recent = filesystem::load_recent();
        let bookmarks = filesystem::load_bookmarks();
        (
            App {
                path_bar_text: start.to_string_lossy().to_string(),
                current_path: start,
                history: Vec::new(),
                forward_stack: Vec::new(),
                entries,
                selected: HashSet::new(),
                search_query: String::new(),
                search_results: Vec::new(),
                search_handle: None,
                search_generation: 0,
                searching: false,
                view_mode: ViewMode::Grid,
                clipboard: None,
                sort_by: SortBy::Name,
                sort_asc: true,
                group_folders: true,
                path_bar_editing: false,
                context_menu: None,
                open_with: None,
                confirm: None,
                preview: None,
                preview_thumbnail: None,
                preview_thumbnail_loading: false,
                preview_thumbnail_generation: 0,
                show_hidden: false,
                app_registry,
                status_message: status,
                devices,
                network_drives,
                recent,
                bookmarks,
                cursor_pos: Point::ORIGIN,
                context_menu_pos: Point::ORIGIN,
                sidebar_width: 200.0,
                preview_width: 260.0,
                pane_drag: None,
                drag_preview_x: 0.0,
                window_size: Size::new(1200.0, 800.0),
                press_origin: None,
                file_drag: None,
                drop_hover: None,
                drag_tx: None,
            },
            iced::window::get_latest().map(Message::WindowIdReady),
        )
    }

    fn navigate_to(&mut self, path: PathBuf) {
        let old = self.current_path.clone();
        self.history.push(old);
        self.forward_stack.clear();
        self.do_load(path);
    }

    fn do_load(&mut self, path: PathBuf) {
        self.current_path = path.clone();
        self.path_bar_text = path.to_string_lossy().to_string();
        self.path_bar_editing = false;
        self.selected.clear();
        self.search_query.clear();
        self.search_results.clear();
        self.searching = false;
        if let Some(handle) = self.search_handle.take() {
            handle.abort();
        }
        self.context_menu = None;
        self.preview = None;
        self.entries = read_dir_entries(&path, self.show_hidden);
        self.sort_entries();
        self.status_message = format!("{} items", self.entries.len());
        self.push_recent(path);
    }

    fn push_recent(&mut self, path: PathBuf) {
        self.recent.retain(|p| p != &path);
        self.recent.insert(0, path);
        self.recent.truncate(10);
        filesystem::save_recent(&self.recent);
    }

    fn sort_entries(&mut self) {
        let asc = self.sort_asc;
        let group_folders = self.group_folders;
        self.entries.sort_by(|a, b| {
            let ord = match self.sort_by {
                SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortBy::Size => a.size.cmp(&b.size),
                SortBy::Modified => a.modified.cmp(&b.modified),
                SortBy::Kind => a.mime_type.cmp(&b.mime_type),
            };
            let ord = if asc { ord } else { ord.reverse() };
            if group_folders {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => ord,
                }
            } else {
                ord
            }
        });
    }

    fn refresh(&mut self) {
        let path = self.current_path.clone();
        self.entries = read_dir_entries(&path, self.show_hidden);
        self.sort_entries();
        self.status_message = format!("{} items  |  {} selected", self.entries.len(), self.selected.len());
    }

    fn refresh_drives(&mut self) {
        let (devices, network_drives) = filesystem::list_drives();
        self.devices = devices;
        self.network_drives = network_drives;
    }

    fn displayed_entries(&self) -> &Vec<FileEntry> {
        if self.search_query.is_empty() {
            &self.entries
        } else {
            &self.search_results
        }
    }

    fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected.iter().cloned().collect()
    }

    fn is_trash_dir(&self) -> bool {
        self.current_path == filesystem::trash_dir().join("files")
    }

    fn update_preview(&mut self) -> Task<Message> {
        self.preview = if self.selected.len() == 1 {
            self.selected.iter().next().and_then(|p| filesystem::get_file_details(p))
        } else {
            None
        };

        self.preview_thumbnail = None;
        self.preview_thumbnail_generation += 1;
        let generation = self.preview_thumbnail_generation;

        let Some(details) = &self.preview else { return Task::none() };
        if details.is_dir {
            return Task::none();
        }
        let mime = details.mime_type.clone();
        let is_image = filesystem::is_previewable_image(&mime);
        let is_video = filesystem::is_video(&mime);
        if !is_image && !is_video {
            return Task::none();
        }

        self.preview_thumbnail_loading = true;
        let source = details.path.clone();

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || filesystem::generate_preview_thumbnail(&source, &mime))
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
            },
            move |result| Message::PreviewThumbnailReady(generation, result),
        )
    }

    /// A press+release that never crossed the drag threshold — same
    /// select/navigate behavior this app always had on a single click.
    fn handle_entry_click(&mut self, path: PathBuf) -> Task<Message> {
        let entry = self.displayed_entries().iter().find(|e| e.path == path).cloned();
        if let Some(e) = entry {
            if e.is_dir {
                self.navigate_to(path);
            } else {
                if self.selected.contains(&path) {
                    self.selected.remove(&path);
                } else {
                    self.selected.clear();
                    self.selected.insert(path.clone());
                }
                self.status_message = format!(
                    "{} items  |  {} selected",
                    self.displayed_entries().len(),
                    self.selected.len()
                );
                return self.update_preview();
            }
        }
        Task::none()
    }

    /// Drag-and-drop of `paths` onto `target_dir` — a plain move, same as
    /// cut+paste into that folder.
    fn move_entries_into(&mut self, paths: &[PathBuf], target_dir: &std::path::Path) {
        let mut moved = 0;
        for src in paths {
            // Dropping a folder onto itself or one of its own descendants
            // would either no-op or corrupt it — skip silently rather than
            // erroring on every ordinary drag that starts inside the
            // target (e.g. dragging within the same open folder).
            if target_dir.starts_with(src) {
                continue;
            }
            let Some(name) = src.file_name() else { continue };
            let dst = target_dir.join(name);
            if dst == *src {
                continue;
            }
            match filesystem::move_file(src, &dst) {
                Ok(_) => moved += 1,
                Err(e) => self.status_message = format!("Error: {}", e),
            }
        }
        if moved > 0 {
            self.status_message = format!("Moved {moved} item(s)");
            self.selected.clear();
            self.refresh();
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::CheckThemeReload => {
                reload_theme_colors();
            }
            Message::Navigate(path) => {
                self.navigate_to(path);
            }
            Message::NavigateBack => {
                if let Some(prev) = self.history.pop() {
                    let cur = self.current_path.clone();
                    self.forward_stack.push(cur);
                    self.do_load(prev);
                }
            }
            Message::NavigateForward => {
                if let Some(next) = self.forward_stack.pop() {
                    let cur = self.current_path.clone();
                    self.history.push(cur);
                    self.do_load(next);
                }
            }
            Message::NavigateUp => {
                if let Some(parent) = self.current_path.parent().map(|p| p.to_path_buf()) {
                    self.navigate_to(parent);
                }
            }
            Message::NavigateHome => {
                self.navigate_to(home_dir());
            }
            Message::EntryPressed(path) => {
                self.press_origin = Some((path, self.cursor_pos));
            }
            Message::EntryReleased(released_path) => {
                let Some((pressed_path, _)) = self.press_origin.take() else { return Task::none() };
                if let Some(dragged) = self.file_drag.take() {
                    self.drop_hover = None;
                    let target_is_dir = self.displayed_entries().iter().any(|e| e.path == released_path && e.is_dir);
                    if target_is_dir && !dragged.contains(&released_path) {
                        self.move_entries_into(&dragged, &released_path);
                    }
                } else {
                    return self.handle_entry_click(pressed_path);
                }
            }
            Message::EntryHoverEnter(path) => {
                self.drop_hover = Some(path);
            }
            Message::EntryHoverExit(path) => {
                if self.drop_hover.as_ref() == Some(&path) {
                    self.drop_hover = None;
                }
            }
            Message::EntryDoubleClicked(path) => {
                let is_dir = self.displayed_entries().iter().any(|e| e.path == path && e.is_dir);
                if is_dir {
                    self.navigate_to(path);
                } else {
                    filesystem::open_default(&path);
                }
            }
            Message::SelectAll => {
                self.selected = self.displayed_entries().iter().map(|e| e.path.clone()).collect();
                self.status_message = format!("{} items  |  {} selected", self.displayed_entries().len(), self.selected.len());
                return self.update_preview();
            }
            Message::SearchChanged(q) => {
                self.search_query = q;
                if let Some(handle) = self.search_handle.take() {
                    handle.abort();
                }
                self.searching = false;

                if self.search_query.is_empty() {
                    self.search_results.clear();
                    self.status_message = format!("{} items", self.entries.len());
                    return Task::none();
                }

                // Instant pass: entries already loaded for the current folder.
                self.search_results = self.entries
                    .iter()
                    .filter(|e| filesystem::matches_query(&e.name, &self.search_query))
                    .cloned()
                    .collect();
                self.status_message = format!("{} results (searching subfolders...)", self.search_results.len());
                self.searching = true;

                let initial_dirs: VecDeque<PathBuf> = self.entries
                    .iter()
                    .filter(|e| e.is_dir)
                    .map(|e| e.path.clone())
                    .collect();

                self.search_generation += 1;
                let generation = self.search_generation;
                let query = self.search_query.clone();
                let show_hidden = self.show_hidden;

                let stream = iced::stream::channel(16, move |mut sender| async move {
                    let mut queue = initial_dirs;
                    let mut found = 0usize;

                    while let Some(dir) = queue.pop_front() {
                        let query = query.clone();
                        let (matches, subdirs) = tokio::task::spawn_blocking(move || {
                            filesystem::scan_dir_search(&dir, &query, show_hidden)
                        })
                        .await
                        .unwrap_or_default();

                        queue.extend(subdirs);

                        if !matches.is_empty() {
                            found += matches.len();
                            let _ = sender.send(Message::SearchBatch(generation, matches)).await;
                        }
                        if found >= SEARCH_RESULT_CAP {
                            break;
                        }
                    }

                    let _ = sender.send(Message::SearchDone(generation)).await;
                });

                let (task, handle) = Task::stream(stream).abortable();
                self.search_handle = Some(handle);
                return task;
            }
            Message::SearchSubmit => {}
            Message::SearchBatch(generation, mut batch) => {
                if generation == self.search_generation {
                    self.search_results.append(&mut batch);
                    self.status_message = format!("{} results (searching subfolders...)", self.search_results.len());
                }
            }
            Message::SearchDone(generation) => {
                if generation == self.search_generation {
                    self.searching = false;
                    self.status_message = format!("{} results", self.search_results.len());
                }
            }
            Message::PreviewThumbnailReady(generation, result) => {
                if generation == self.preview_thumbnail_generation {
                    self.preview_thumbnail_loading = false;
                    self.preview_thumbnail = result.ok();
                }
            }
            Message::ViewModeToggle => {
                self.view_mode = match self.view_mode {
                    ViewMode::Grid => ViewMode::List,
                    ViewMode::List => ViewMode::Grid,
                };
            }
            Message::SortChanged(by) => {
                if self.sort_by == by {
                    self.sort_asc = !self.sort_asc;
                } else {
                    self.sort_by = by;
                    self.sort_asc = true;
                }
                self.sort_entries();
                self.context_menu = None;
            }
            Message::ToggleGroupFolders => {
                self.group_folders = !self.group_folders;
                self.sort_entries();
                self.context_menu = None;
            }
            Message::ShowHiddenToggle => {
                self.show_hidden = !self.show_hidden;
                self.refresh();
                self.context_menu = None;
            }
            Message::PathBarEdit(s) => {
                self.path_bar_text = s;
                self.path_bar_editing = true;
            }
            Message::PathBarSubmit => {
                let path = PathBuf::from(&self.path_bar_text);
                if path.is_dir() {
                    self.navigate_to(path);
                } else {
                    self.path_bar_text = self.current_path.to_string_lossy().to_string();
                    self.path_bar_editing = false;
                    self.status_message = "Path not found".to_string();
                }
            }
            Message::NewFolder => {
                let dest = filesystem::unique_name(&self.current_path, "New Folder");
                match filesystem::create_dir(&dest) {
                    Ok(_) => {
                        self.refresh();
                        self.selected = HashSet::from([dest.clone()]);
                        let task = self.update_preview();
                        self.context_menu = Some(ContextMenu {
                            kind: ContextMenuKind::Entry { path: dest, is_dir: true },
                            renaming: true,
                            rename_text: "New Folder".to_string(),
                        });
                        return task;
                    }
                    Err(e) => self.status_message = format!("Error: {}", e),
                }
            }
            Message::NewFile => {
                let dest = filesystem::unique_name(&self.current_path, "New File.txt");
                match filesystem::create_file(&dest) {
                    Ok(_) => {
                        self.refresh();
                        self.selected = HashSet::from([dest.clone()]);
                        let task = self.update_preview();
                        self.context_menu = Some(ContextMenu {
                            kind: ContextMenuKind::Entry { path: dest, is_dir: false },
                            renaming: true,
                            rename_text: "New File.txt".to_string(),
                        });
                        return task;
                    }
                    Err(e) => self.status_message = format!("Error: {}", e),
                }
            }
            Message::ContextMenuClose => {
                self.context_menu = None;
            }
            Message::ContextMenuOpenEntry(path) => {
                let is_dir = self.displayed_entries().iter().any(|e| e.path == path && e.is_dir);
                let mut task = Task::none();
                if !self.selected.contains(&path) {
                    self.selected.clear();
                    self.selected.insert(path.clone());
                    task = self.update_preview();
                }
                self.context_menu_pos = self.cursor_pos;
                self.context_menu = Some(ContextMenu {
                    kind: ContextMenuKind::Entry { path, is_dir },
                    renaming: false,
                    rename_text: String::new(),
                });
                return task;
            }
            Message::ContextMenuOpenBackground => {
                self.context_menu_pos = self.cursor_pos;
                self.context_menu = Some(ContextMenu {
                    kind: ContextMenuKind::Background,
                    renaming: false,
                    rename_text: String::new(),
                });
            }
            Message::OpenDefault(path) => {
                filesystem::open_default(&path);
                self.context_menu = None;
            }
            Message::OpenWith(path, exec) => {
                filesystem::open_with(&path, &exec);
                self.context_menu = None;
                self.open_with = None;
            }
            Message::OpenWithBrowse(path) => {
                let mime = crate::desktop::mime_for_file(&path);
                self.context_menu = None;
                self.open_with = Some(OpenWithDialog { path, mime, search: String::new() });
            }
            Message::OpenWithSearch(q) => {
                if let Some(dlg) = &mut self.open_with {
                    dlg.search = q;
                }
            }
            Message::OpenWithSetDefault(mime, desktop_id) => {
                crate::desktop::set_default_app(&mime, &desktop_id);
                self.status_message = "Default app updated".to_string();
            }
            Message::OpenWithClose => {
                self.open_with = None;
            }
            Message::OpenTerminalHere(path) => {
                filesystem::open_terminal_here(&path);
                self.context_menu = None;
            }
            Message::OpenInTerminalWithPath(path) => {
                let dir = path.parent().unwrap_or(&path).to_path_buf();
                filesystem::open_terminal_with_prefill(&dir, &path.to_string_lossy());
                self.context_menu = None;
            }
            Message::CopyPath(path) => {
                self.context_menu = None;
                self.status_message = "Path copied to clipboard".to_string();
                return iced::clipboard::write::<Message>(path.to_string_lossy().to_string());
            }
            Message::CopyName(path) => {
                self.context_menu = None;
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                self.status_message = "Name copied to clipboard".to_string();
                return iced::clipboard::write::<Message>(name);
            }
            Message::Compress => {
                let paths = self.selected_paths();
                if !paths.is_empty() {
                    let dest = self.current_path.clone();
                    if let Err(e) = filesystem::compress_paths(&paths, &dest) {
                        self.status_message = format!("Error: {}", e);
                    } else {
                        self.status_message = "Compressed".to_string();
                    }
                }
                self.context_menu = None;
                self.refresh();
            }
            Message::CompressZip => {
                let paths = self.selected_paths();
                if !paths.is_empty() {
                    let dest = self.current_path.clone();
                    if let Err(e) = filesystem::compress_paths_zip(&paths, &dest) {
                        self.status_message = format!("Error: {}", e);
                    } else {
                        self.status_message = "Compressed to .zip".to_string();
                    }
                }
                self.context_menu = None;
                self.refresh();
            }
            Message::Duplicate => {
                let paths = self.selected_paths();
                let mut count = 0;
                for src in &paths {
                    let dst = filesystem::duplicate_name(src);
                    if filesystem::copy_file(src, &dst).is_ok() {
                        count += 1;
                    }
                }
                self.status_message = format!("Duplicated {} item(s)", count);
                self.context_menu = None;
                self.refresh();
            }
            Message::Extract(path) => {
                let dest = self.current_path.clone();
                match filesystem::extract_archive(&path, &dest) {
                    Ok(_) => self.status_message = "Extracted".to_string(),
                    Err(e) => self.status_message = format!("Error: {}", e),
                }
                self.context_menu = None;
                self.refresh();
            }
            Message::Copy => {
                let paths = self.selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(ClipboardOp::Copy(paths));
                    self.status_message = "Copied to clipboard".to_string();
                }
                self.context_menu = None;
            }
            Message::Cut => {
                let paths = self.selected_paths();
                if !paths.is_empty() {
                    self.clipboard = Some(ClipboardOp::Cut(paths));
                    self.status_message = "Cut to clipboard".to_string();
                }
                self.context_menu = None;
            }
            Message::Paste => {
                if let Some(op) = self.clipboard.clone() {
                    match op {
                        ClipboardOp::Copy(paths) => {
                            for src in &paths {
                                let dst = self.current_path.join(src.file_name().unwrap_or_default());
                                if let Err(e) = filesystem::copy_file(src, &dst) {
                                    self.status_message = format!("Error: {}", e);
                                }
                            }
                        }
                        ClipboardOp::Cut(paths) => {
                            for src in &paths {
                                let dst = self.current_path.join(src.file_name().unwrap_or_default());
                                if let Err(e) = filesystem::move_file(src, &dst) {
                                    self.status_message = format!("Error: {}", e);
                                }
                            }
                            self.clipboard = None;
                        }
                    }
                    self.refresh();
                }
                self.context_menu = None;
            }
            Message::Delete => {
                let paths = self.selected_paths();
                if !paths.is_empty() {
                    match filesystem::move_to_trash(&paths) {
                        Ok(_) => self.status_message = format!("Moved {} item(s) to trash", paths.len()),
                        Err(e) => self.status_message = format!("Error: {}", e),
                    }
                    self.selected.clear();
                    self.context_menu = None;
                    self.preview = None;
                    self.refresh();
                }
            }
            Message::DeletePermanently => {
                let paths = self.selected_paths();
                self.context_menu = None;
                if !paths.is_empty() {
                    let noun = if paths.len() == 1 { "item".to_string() } else { format!("{} items", paths.len()) };
                    self.confirm = Some(ConfirmDialog {
                        message: format!("Permanently delete {}? This cannot be undone.", noun),
                        action: PendingAction::DeletePermanently(paths),
                    });
                }
            }
            Message::RestoreFromTrash(path) => {
                self.context_menu = None;
                match filesystem::restore_from_trash(&path) {
                    Ok(dest) => self.status_message = format!("Restored to {}", dest.display()),
                    Err(e) => self.status_message = format!("Error: {}", e),
                }
                self.selected.clear();
                self.preview = None;
                self.refresh();
            }
            Message::EmptyTrash => {
                self.context_menu = None;
                self.confirm = Some(ConfirmDialog {
                    message: "Permanently delete everything in Trash? This cannot be undone.".to_string(),
                    action: PendingAction::EmptyTrash,
                });
            }
            Message::ConfirmYes => {
                if let Some(dlg) = self.confirm.take() {
                    match dlg.action {
                        PendingAction::DeletePermanently(paths) => {
                            match filesystem::delete_permanently(&paths) {
                                Ok(_) => self.status_message = format!("Permanently deleted {} item(s)", paths.len()),
                                Err(e) => self.status_message = format!("Error: {}", e),
                            }
                            self.selected.clear();
                            self.preview = None;
                            self.refresh();
                        }
                        PendingAction::EmptyTrash => {
                            match filesystem::empty_trash() {
                                Ok(_) => self.status_message = "Trash emptied".to_string(),
                                Err(e) => self.status_message = format!("Error: {}", e),
                            }
                            self.refresh();
                        }
                    }
                }
            }
            Message::ConfirmNo => {
                self.confirm = None;
            }
            Message::ToggleBookmark(path) => {
                if let Some(pos) = self.bookmarks.iter().position(|p| p == &path) {
                    self.bookmarks.remove(pos);
                } else {
                    self.bookmarks.push(path);
                }
                filesystem::save_bookmarks(&self.bookmarks);
                self.context_menu = None;
            }
            Message::RenameStart => {
                if let Some(cm) = &mut self.context_menu {
                    if let ContextMenuKind::Entry { path, .. } = &cm.kind {
                        cm.rename_text = path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        cm.renaming = true;
                    }
                }
            }
            Message::RenameText(s) => {
                if let Some(cm) = &mut self.context_menu {
                    cm.rename_text = s;
                }
            }
            Message::RenameSubmit => {
                if let Some(cm) = self.context_menu.take() {
                    if let ContextMenuKind::Entry { path, .. } = cm.kind {
                        let new_path = path.parent()
                            .map(|p| p.join(&cm.rename_text))
                            .unwrap_or_default();
                        match filesystem::rename_entry(&path, &new_path) {
                            Ok(_) => {
                                self.status_message = format!("Renamed to {}", cm.rename_text);
                                self.selected = HashSet::from([new_path]);
                            }
                            Err(e) => self.status_message = format!("Error: {}", e),
                        }
                        self.refresh();
                        return self.update_preview();
                    }
                }
            }
            Message::SidebarBookmark(path) => {
                self.navigate_to(path);
            }
            Message::Refresh => {
                self.refresh();
                self.refresh_drives();
            }
            Message::CursorMoved(pos) => {
                self.cursor_pos = pos;
                // Dragging only moves a cheap ghost line (see `pane_divider`
                // rendering below) — it deliberately does NOT touch
                // sidebar_width/preview_width, which would force a full
                // rebuild + repaint of the grid (images and all) on every
                // single mouse-move tick. The actual resize is committed
                // once, in `PaneDragEnd`.
                if self.pane_drag.is_some() {
                    self.drag_preview_x = pos.x;
                }
                // A held-down press only becomes a file drag once the
                // cursor has actually moved a few pixels — otherwise every
                // ordinary click (press+release near-instantly, same spot)
                // would count as a drag of the pressed item onto itself.
                if self.file_drag.is_none() {
                    if let Some((pressed_path, press_pos)) = &self.press_origin {
                        let dx = pos.x - press_pos.x;
                        let dy = pos.y - press_pos.y;
                        if dx * dx + dy * dy > FILE_DRAG_THRESHOLD * FILE_DRAG_THRESHOLD {
                            let dragged: Vec<PathBuf> = if self.selected.contains(pressed_path) {
                                self.selected.iter().cloned().collect()
                            } else {
                                vec![pressed_path.clone()]
                            };
                            // Also kick off a real OS-level drag, so
                            // dropping outside this window (e.g. onto a
                            // browser's upload zone) works too, not just
                            // dropping onto a folder row in here.
                            if let Some(tx) = &self.drag_tx {
                                let _ = tx.send(og_wayland::DragRequest { paths: dragged.clone() });
                            }
                            self.file_drag = Some(dragged);
                        }
                    }
                }
            }
            Message::PaneDragStart(target) => {
                self.pane_drag = Some(target);
                self.drag_preview_x = match target {
                    PaneDrag::Sidebar => self.sidebar_width,
                    PaneDrag::Preview => self.window_size.width - self.preview_width,
                };
            }
            Message::PaneDragEnd => {
                if let Some(target) = self.pane_drag.take() {
                    match target {
                        PaneDrag::Sidebar => {
                            self.sidebar_width = self.drag_preview_x
                                .clamp(*SIDEBAR_WIDTH_RANGE.start(), *SIDEBAR_WIDTH_RANGE.end());
                        }
                        PaneDrag::Preview => {
                            let width = self.window_size.width - self.drag_preview_x;
                            self.preview_width = width.clamp(*PREVIEW_WIDTH_RANGE.start(), *PREVIEW_WIDTH_RANGE.end());
                        }
                    }
                }
                // Fallback cleanup for a release over empty space (no
                // row's `on_release` fired to consume it there).
                self.press_origin = None;
                self.file_drag = None;
                self.drop_hover = None;
            }
            Message::WindowResized(size) => {
                self.window_size = size;
            }
            Message::Noop => {}
            Message::WindowIdReady(Some(id)) => {
                return iced::window::run_with_handle(id, |handle| {
                    og_wayland::display_ptr_from_window_handle(handle)
                })
                .map(Message::WaylandDisplayReady);
            }
            Message::WindowIdReady(None) => {}
            Message::WaylandDisplayReady(Some(display_ptr)) => {
                self.drag_tx = Some(og_wayland::spawn(display_ptr));
            }
            Message::WaylandDisplayReady(None) => {
                self.status_message = "Cross-app drag-out unavailable (non-Wayland session?)".to_string();
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let toolbar = toolbar::view(
            !self.history.is_empty(),
            !self.forward_stack.is_empty(),
            &self.path_bar_text,
            self.path_bar_editing,
            &self.search_query,
            &self.view_mode,
            self.show_hidden,
        );

        let sidebar = sidebar::view(&self.current_path, &self.devices, &self.network_drives, &self.recent, &self.bookmarks, self.sidebar_width);

        // Preview pane's column is always reserved (even with nothing
        // selected) — toggling it in and out used to shift/resize every
        // other pane each time you clicked a file, which read as the whole
        // window "rearranging itself".
        let file_area_width = self.window_size.width - self.sidebar_width - DIVIDER_WIDTH - self.preview_width - DIVIDER_WIDTH;
        let overlay_open = self.context_menu.is_some() || self.open_with.is_some() || self.confirm.is_some();
        let file_area = filelist::view(
            self.displayed_entries(),
            &self.selected,
            &self.view_mode,
            file_area_width,
            &self.sort_by,
            self.sort_asc,
            overlay_open,
            if self.file_drag.is_some() { self.drop_hover.as_ref() } else { None },
        );

        let path_str = self.current_path.to_string_lossy().to_string();
        let status_str = self.status_message.clone();

        let status_bar = container(
            row![
                text(status_str).size(12).style(move |_| text::Style { color: Some(MUTED()) }),
                iced::widget::Space::with_width(Length::Fill),
                text(path_str).size(12).style(move |_| text::Style { color: Some(MUTED()) }),
            ]
            .spacing(8)
            .padding([4, 12])
        )
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(SEC_BG())),
            text_color: None,
            ..Default::default()
        });

        let mut main_children: Vec<Element<Message>> = vec![
            sidebar,
            pane_divider(PaneDrag::Sidebar),
            file_area,
        ];
        main_children.push(pane_divider(PaneDrag::Preview));
        main_children.push(self.preview_panel(self.preview.as_ref()));
        let main_content = row(main_children).width(Length::Fill).height(Length::Fill);

        let base = column![toolbar, main_content, status_bar]
            .width(Length::Fill)
            .height(Length::Fill);

        let root: Element<Message> = container(base)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(BG())),
                text_color: None,
                ..Default::default()
            })
            .into();

        let root = if let Some(cm) = &self.context_menu {
            self.context_menu_overlay(root, cm)
        } else {
            root
        };

        let root = if let Some(dlg) = &self.open_with {
            self.open_with_overlay(root, dlg)
        } else {
            root
        };

        let root = if let Some(dlg) = &self.confirm {
            self.confirm_overlay(root, dlg)
        } else {
            root
        };

        let root = if let Some(paths) = &self.file_drag {
            let ghost = drag_ghost_files(paths);
            og_drag::with_drag_ghost(root, Some(ghost), self.cursor_pos, iced::Vector::new(14.0, 14.0))
        } else {
            root
        };

        if self.pane_drag.is_some() {
            drag_ghost_line(root, self.drag_preview_x)
        } else {
            root
        }
    }

    fn confirm_overlay<'a>(&'a self, base: Element<'a, Message>, dlg: &'a ConfirmDialog) -> Element<'a, Message> {
        use iced::widget::stack;

        let yes_btn = button(text("Delete").size(13))
            .on_press(Message::ConfirmYes)
            .padding([6, 14])
            .style(|_, status| button::Style {
                background: Some(Background::Color(match status {
                    button::Status::Hovered => Color { r: 0.8, g: 0.2, b: 0.2, a: 1.0 },
                    _ => Color { r: 0.65, g: 0.15, b: 0.15, a: 1.0 },
                })),
                text_color: Color::WHITE,
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            });

        let no_btn = button(text("Cancel").size(13))
            .on_press(Message::ConfirmNo)
            .padding([6, 14])
            .style(|_, status| button::Style {
                background: Some(Background::Color(match status {
                    button::Status::Hovered => SURFACE(),
                    _ => SEC_BG(),
                })),
                text_color: TEXT(),
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            });

        let dialog = container(
            column![
                text(&dlg.message).size(14).style(move |_| text::Style { color: Some(TEXT()) }),
                row![Space::with_width(Length::Fill), no_btn, yes_btn].spacing(8),
            ]
            .spacing(16)
            .padding(16)
            .width(320)
        )
        .style(|_| container::Style {
            background: Some(Background::Color(SEC_BG())),
            text_color: None,
            border: Border { color: Color { r: 0.3, g: 0.27, b: 0.4, a: 1.0 }, width: 1.0, radius: 6.0.into() },
            shadow: Default::default(),
        });

        let overlay = container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        stack![base, overlay].into()
    }

    fn preview_panel<'a>(&'a self, details: Option<&'a FileDetails>) -> Element<'a, Message> {
        const ICON_FONT: iced::Font = iced::Font::with_name("Symbols Nerd Font");

        let Some(d) = details else {
            return container(
                container(text("No selection").size(13).style(move |_| text::Style { color: Some(MUTED()) }))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
            )
            .width(self.preview_width)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(SEC_BG())),
                ..Default::default()
            })
            .into();
        };

        let icon = filesystem::icon_for(&d.mime_type, d.is_dir);
        let is_media = filesystem::is_previewable_image(&d.mime_type) || filesystem::is_video(&d.mime_type);

        let mut col = column![].spacing(10).padding(12).width(Length::Fill);

        if let Some(thumb_path) = &self.preview_thumbnail {
            col = col.push(
                container(
                    image(image::Handle::from_path(thumb_path))
                        .width(Length::Fill)
                        .content_fit(iced::ContentFit::Contain)
                )
                .center_x(Length::Fill)
                .max_height(220)
            );
        } else {
            col = col.push(
                container(text(icon).font(ICON_FONT).size(40).style(move |_| text::Style { color: Some(TEXT()) }))
                    .center_x(Length::Fill)
            );
            if is_media && self.preview_thumbnail_loading {
                col = col.push(
                    container(text("Loading preview...").size(11).style(move |_| text::Style { color: Some(MUTED()) }))
                        .center_x(Length::Fill)
                );
            }
        }
        col = col.push(selectable_text(&d.name, 14));
        col = col.push(context_sep());

        col = col.push(detail_row("Type", if d.is_dir { "Folder".to_string() } else { d.mime_type.clone() }));
        if d.is_dir {
            if let Some(n) = d.item_count {
                col = col.push(detail_row("Items", n.to_string()));
            }
        } else {
            col = col.push(detail_row("Size", format!("{} ({} bytes)", filesystem::format_size(d.size), d.size)));
        }
        if let Some((w, h)) = d.dimensions {
            col = col.push(detail_row("Dimensions", format!("{} x {}", w, h)));
        }

        let fmt_dt = |dt: chrono::DateTime<chrono::Local>| dt.format("%Y-%m-%d %H:%M:%S").to_string();
        col = col.push(detail_row("Modified", d.modified.map(fmt_dt).unwrap_or_else(|| "-".to_string())));
        col = col.push(detail_row("Created", d.created.map(fmt_dt).unwrap_or_else(|| "-".to_string())));
        col = col.push(detail_row("Accessed", d.accessed.map(fmt_dt).unwrap_or_else(|| "-".to_string())));
        col = col.push(detail_row("Permissions", format!("{} ({})", d.permissions, d.mode_octal)));
        col = col.push(detail_row("Owner", format!("{} : {}", d.owner, d.group)));
        if let Some(target) = &d.symlink_target {
            col = col.push(detail_row("Links To", target.to_string_lossy().to_string()));
        }
        col = col.push(detail_row("Path", d.path.to_string_lossy().to_string()));

        if !d.exif.is_empty() {
            col = col.push(context_sep());
            col = col.push(text("Metadata").size(11).style(move |_| text::Style { color: Some(MUTED()) }));
            for tag in &d.exif {
                col = col.push(detail_row(&tag.label, tag.value.clone()));
            }
        }

        container(iced::widget::scrollable(col))
            .width(self.preview_width)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(SEC_BG())),
                ..Default::default()
            })
            .into()
    }

    fn open_with_overlay<'a>(&'a self, base: Element<'a, Message>, dlg: &'a OpenWithDialog) -> Element<'a, Message> {
        use iced::widget::stack;

        let search_lower = dlg.search.to_lowercase();
        let matches = |app: &AppEntry| {
            search_lower.is_empty() || app.name.to_lowercase().contains(&search_lower)
        };

        let mut matched: Vec<&AppEntry> = crate::desktop::apps_for_file(&dlg.path, &self.app_registry)
            .into_iter()
            .filter(|a| matches(a))
            .collect();
        matched.sort_by(|a, b| a.name.cmp(&b.name));

        let matched_ids: HashSet<&str> = matched.iter().map(|a| a.desktop_id.as_str()).collect();
        let mut others: Vec<&AppEntry> = self.app_registry
            .iter()
            .filter(|a| !matched_ids.contains(a.desktop_id.as_str()) && matches(a))
            .collect();
        others.sort_by(|a, b| a.name.cmp(&b.name));

        let default_id = crate::desktop::default_app_for_file(&dlg.path);

        let search = text_input("Search apps...", &dlg.search)
            .on_input(Message::OpenWithSearch)
            .size(13)
            .style(|_, _| text_input::Style {
                background: Background::Color(SURFACE()),
                border: Border { color: ACCENT(), width: 1.0, radius: 3.0.into() },
                icon: TEXT(),
                placeholder: MUTED(),
                value: TEXT(),
                selection: ACCENT(),
            });

        let mut list = column![].spacing(2);
        if matched.is_empty() && others.is_empty() {
            list = list.push(
                text("No matching apps").size(13).style(move |_| text::Style { color: Some(MUTED()) })
            );
        }
        if !matched.is_empty() {
            list = list.push(text("Recommended").size(11).style(move |_| text::Style { color: Some(MUTED()) }));
            for app in &matched {
                let is_default = default_id.as_deref() == Some(app.desktop_id.as_str());
                list = list.push(open_with_row(&dlg.path, &dlg.mime, app, is_default));
            }
        }
        if !others.is_empty() {
            list = list.push(text("Other Applications").size(11).style(move |_| text::Style { color: Some(MUTED()) }));
            for app in &others {
                let is_default = default_id.as_deref() == Some(app.desktop_id.as_str());
                list = list.push(open_with_row(&dlg.path, &dlg.mime, app, is_default));
            }
        }

        let dialog = container(
            column![
                text("Open With").size(15).style(move |_| text::Style { color: Some(TEXT()) }),
                search,
                iced::widget::scrollable(list).height(Length::Fixed(320.0)),
                context_btn("Close", Message::OpenWithClose),
            ]
            .spacing(10)
            .padding(14)
            .width(360)
        )
        .style(|_| container::Style {
            background: Some(Background::Color(SEC_BG())),
            text_color: None,
            border: Border { color: Color { r: 0.3, g: 0.27, b: 0.4, a: 1.0 }, width: 1.0, radius: 6.0.into() },
            shadow: Default::default(),
        });

        let overlay = container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        stack![base, overlay].into()
    }

    fn context_menu_overlay<'a>(&'a self, base: Element<'a, Message>, cm: &'a ContextMenu) -> Element<'a, Message> {
        use iced::widget::stack;

        let has_clipboard = self.clipboard.is_some();
        let multi = self.selected.len() > 1;

        let mut menu_col = column![].spacing(2).padding(6).width(220);
        let mut item_count = 0usize;

        if cm.renaming {
            let rename_input = text_input("New name...", &cm.rename_text)
                .on_input(Message::RenameText)
                .on_submit(Message::RenameSubmit)
                .size(13)
                .style(|_, _| text_input::Style {
                    background: Background::Color(SURFACE()),
                    border: Border { color: ACCENT(), width: 1.0, radius: 3.0.into() },
                    icon: TEXT(),
                    placeholder: MUTED(),
                    value: TEXT(),
                    selection: ACCENT(),
                });
            menu_col = menu_col.push(rename_input);
            menu_col = menu_col.push(context_btn("Confirm Rename", Message::RenameSubmit));
            item_count += 2;
        } else {
            match &cm.kind {
                ContextMenuKind::Entry { path, is_dir } => {
                    if self.is_trash_dir() {
                        menu_col = menu_col.push(context_btn("Restore", Message::RestoreFromTrash(path.clone())));
                        menu_col = menu_col.push(context_btn("Delete Permanently", Message::DeletePermanently));
                        item_count += 2;
                    } else {
                    menu_col = menu_col.push(context_btn("Open", Message::OpenDefault(path.clone())));
                    item_count += 1;

                    if !*is_dir {
                        menu_col = menu_col.push(context_btn("Open With...", Message::OpenWithBrowse(path.clone())));
                        menu_col = menu_col.push(context_btn("Open in Terminal", Message::OpenInTerminalWithPath(path.clone())));
                        item_count += 2;
                    }

                    menu_col = menu_col.push(context_sep());
                    menu_col = menu_col.push(context_btn(if multi { "Cut (selected)" } else { "Cut" }, Message::Cut));
                    menu_col = menu_col.push(context_btn(if multi { "Copy (selected)" } else { "Copy" }, Message::Copy));
                    menu_col = menu_col.push(context_btn(if multi { "Duplicate (selected)" } else { "Duplicate" }, Message::Duplicate));
                    item_count += 3;
                    if has_clipboard {
                        menu_col = menu_col.push(context_btn("Paste Into Folder", Message::Paste));
                        item_count += 1;
                    }
                    if !multi {
                        menu_col = menu_col.push(context_btn("Rename", Message::RenameStart));
                        item_count += 1;
                    }
                    menu_col = menu_col.push(context_btn("Copy Path", Message::CopyPath(path.clone())));
                    menu_col = menu_col.push(context_btn("Copy Name", Message::CopyName(path.clone())));
                    item_count += 2;

                    if *is_dir {
                        menu_col = menu_col.push(context_btn("Open Terminal Here", Message::OpenTerminalHere(path.clone())));
                        item_count += 1;
                        let bookmarked = self.bookmarks.contains(path);
                        let label = if bookmarked { "Remove from Sidebar" } else { "Add to Sidebar" };
                        menu_col = menu_col.push(context_btn(label, Message::ToggleBookmark(path.clone())));
                        item_count += 1;
                    } else if filesystem::is_archive(path) {
                        menu_col = menu_col.push(context_btn("Extract Here", Message::Extract(path.clone())));
                        item_count += 1;
                    }
                    menu_col = menu_col.push(context_btn("Compress to .zip", Message::CompressZip));
                    menu_col = menu_col.push(context_btn("Compress to .tar.gz", Message::Compress));
                    item_count += 2;

                    menu_col = menu_col.push(context_sep());
                    menu_col = menu_col.push(context_btn("Delete (Trash)", Message::Delete));
                    menu_col = menu_col.push(context_btn("Delete Permanently", Message::DeletePermanently));
                    item_count += 2;
                    }
                }
                ContextMenuKind::Background => {
                    if self.is_trash_dir() {
                        menu_col = menu_col.push(context_btn("Empty Trash", Message::EmptyTrash));
                        item_count += 1;
                    } else {
                    menu_col = menu_col.push(context_btn("New Folder", Message::NewFolder));
                    menu_col = menu_col.push(context_btn("New File", Message::NewFile));
                    item_count += 2;
                    if has_clipboard {
                        menu_col = menu_col.push(context_btn("Paste", Message::Paste));
                        item_count += 1;
                    }
                    menu_col = menu_col.push(context_btn("Open Terminal Here", Message::OpenTerminalHere(self.current_path.clone())));
                    menu_col = menu_col.push(context_btn("Copy Path", Message::CopyPath(self.current_path.clone())));
                    let bookmarked = self.bookmarks.contains(&self.current_path);
                    let label = if bookmarked { "Remove from Sidebar" } else { "Add to Sidebar" };
                    menu_col = menu_col.push(context_btn(label, Message::ToggleBookmark(self.current_path.clone())));
                    item_count += 3;
                    }
                    menu_col = menu_col.push(context_sep());
                    menu_col = menu_col.push(context_btn("Select All", Message::SelectAll));
                    item_count += 2;

                    let sort_label = |label: &str, by: SortBy| {
                        let text = if self.sort_by == by {
                            format!("{}  {}", label, if self.sort_asc { "\u{25b2}" } else { "\u{25bc}" })
                        } else {
                            label.to_string()
                        };
                        context_btn(&text, Message::SortChanged(by))
                    };

                    menu_col = menu_col.push(context_sep());
                    menu_col = menu_col.push(sort_label("Sort by Name", SortBy::Name));
                    menu_col = menu_col.push(sort_label("Sort by Size", SortBy::Size));
                    menu_col = menu_col.push(sort_label("Sort by Modified", SortBy::Modified));
                    menu_col = menu_col.push(sort_label("Sort by Kind", SortBy::Kind));
                    let group_label = if self.group_folders {
                        "Folders Grouped First \u{2713}"
                    } else {
                        "Folders Mixed With Files"
                    };
                    menu_col = menu_col.push(context_btn(group_label, Message::ToggleGroupFolders));
                    item_count += 5;

                    menu_col = menu_col.push(context_sep());
                    let hidden_label = if self.show_hidden { "Hide Hidden Files" } else { "Show Hidden Files" };
                    menu_col = menu_col.push(context_btn(hidden_label, Message::ShowHiddenToggle));
                    menu_col = menu_col.push(context_btn("Refresh", Message::Refresh));
                    item_count += 3;
                }
            }
            menu_col = menu_col.push(context_sep());
            menu_col = menu_col.push(context_btn("Close Menu", Message::ContextMenuClose));
            item_count += 2;
        }

        let menu = container(menu_col)
            .style(|_| container::Style {
                background: Some(Background::Color(SEC_BG())),
                text_color: None,
                border: Border { color: Color { r: 0.3, g: 0.27, b: 0.4, a: 1.0 }, width: 1.0, radius: 6.0.into() },
            shadow: Default::default(),
            });

        let menu_width = 232.0;
        let menu_height = (item_count as f32) * 30.0 + 16.0;
        let max_x = (self.window_size.width - menu_width - 10.0).max(0.0);
        let max_y = (self.window_size.height - menu_height - 10.0).max(0.0);
        let x = self.context_menu_pos.x.clamp(0.0, max_x);
        let y = self.context_menu_pos.y.clamp(0.0, max_y);

        let overlay = row![
            Space::with_width(Length::Fixed(x)),
            column![
                Space::with_height(Length::Fixed(y)),
                menu,
                Space::with_height(Length::Fill),
            ]
            .width(Length::Shrink),
            Space::with_width(Length::Fill),
        ]
        .height(Length::Fill);

        // Sits between `base` and `menu`: catches any click that the menu
        // itself doesn't (stack dispatches events top-down, so menu buttons
        // still get first refusal) and dismisses the menu, matching normal
        // "click away to close" popup behavior instead of requiring the
        // explicit "Close Menu" button.
        let dismiss = mouse_area(container(Space::new(Length::Fill, Length::Fill)))
            .on_press(Message::ContextMenuClose);

        stack![base, dismiss, overlay].into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::event::listen_with(track_events),
            // Cheap poll so og-settings' Apply & Save takes effect
            // here live, without needing to close/reopen this window.
            iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::CheckThemeReload),
        ])
    }
}

static CURSOR_EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
static LAST_CURSOR_EVENT_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// This app renders with tiny-skia (CPU, no GPU), so every `CursorMoved` was
/// forcing a full relayout + repaint of the whole grid — including
/// thumbnails — at raw mouse-move rate, which is way more redraws than a
/// human can perceive and was the actual cause of the pane-resize lag.
/// Throttled app-wide to cut CPU use; 25fps is plenty for a resize drag.
fn track_events(event: iced::Event, _status: iced::event::Status, _window: iced::window::Id) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            let epoch = CURSOR_EPOCH.get_or_init(std::time::Instant::now);
            let now_ms = epoch.elapsed().as_millis() as u64;
            let last_ms = LAST_CURSOR_EVENT_MS.load(std::sync::atomic::Ordering::Relaxed);
            if now_ms.saturating_sub(last_ms) < 40 {
                return None;
            }
            LAST_CURSOR_EVENT_MS.store(now_ms, std::sync::atomic::Ordering::Relaxed);
            Some(Message::CursorMoved(position))
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => Some(Message::WindowResized(size)),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(Message::PaneDragEnd)
        }
        _ => None,
    }
}

fn context_btn(label: &str, msg: Message) -> Element<'static, Message> {
    let label = label.to_string();
    button(text(label).size(13))
        .on_press(msg)
        .width(Length::Fill)
        .style(|_, status| button::Style {
            background: Some(Background::Color(match status {
                button::Status::Hovered => SURFACE(),
                _ => SEC_BG(),
            })),
            text_color: TEXT(),
            border: Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

fn open_with_row<'a>(path: &PathBuf, mime: &str, app: &'a AppEntry, is_default: bool) -> Element<'a, Message> {
    let name_line = if is_default {
        format!("{}  (default)", app.name)
    } else {
        app.name.clone()
    };

    let open_btn = button(
        column![
            text(name_line).size(13),
            text(&app.comment).size(10).style(move |_| text::Style { color: Some(MUTED()) }),
        ]
        .spacing(1)
    )
    .on_press(Message::OpenWith(path.clone(), app.exec.clone()))
    .width(Length::Fill)
    .style(|_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => SURFACE(),
            _ => SEC_BG(),
        })),
        text_color: TEXT(),
        border: Border { radius: 3.0.into(), ..Default::default() },
        ..Default::default()
    });

    let content: Element<Message> = if is_default {
        open_btn.into()
    } else {
        let default_btn = button(text("Set Default").size(11))
            .on_press(Message::OpenWithSetDefault(mime.to_string(), app.desktop_id.clone()))
            .style(|_, status| button::Style {
                background: Some(Background::Color(match status {
                    button::Status::Hovered => ACCENT(),
                    _ => SEC_BG(),
                })),
                text_color: MUTED(),
                border: Border { color: MUTED(), width: 1.0, radius: 3.0.into() },
                ..Default::default()
            })
            .padding([2, 6]);

        row![open_btn, default_btn].spacing(6).align_y(iced::Alignment::Center).into()
    };

    content
}

fn detail_row<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    column![
        text(label).size(10).style(move |_| text::Style { color: Some(MUTED()) }),
        selectable_text(&value, 12),
    ]
    .spacing(1)
    .into()
}

/// A `text_input` standing in for read-only text: iced's plain `text`
/// widget has no selection/copy support, but `text_input` does. `on_input`
/// is wired to a no-op so it can't actually be edited — the displayed value
/// always comes back from the fixed `value` argument on the next frame.
fn selectable_text<'a>(value: &str, size: u16) -> Element<'a, Message> {
    text_input("", value)
        .on_input(|_| Message::Noop)
        .size(size)
        .padding(0)
        .style(|_, _| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
            icon: TEXT(),
            placeholder: MUTED(),
            value: TEXT(),
            selection: ACCENT(),
        })
        .into()
}

/// While a pane divider is being dragged, this is the only thing that
/// updates per mouse-move — a bare vertical line laid over the *unchanged*
/// base layout via `stack!`. Actually resizing the sidebar/preview panel
/// (and the file grid's expensive reflow with it) only happens once, in
/// `PaneDragEnd`, instead of on every tick of the drag.
/// Visual confirmation for an in-window file drag (drop onto a folder row
/// to move) — a small chip riding the cursor showing what's being
/// dragged, via og-drag's shared ghost-follows-cursor helper. Separate
/// from the real cross-app Wayland drag `og_wayland::DragRequest` also
/// starts alongside this (see the `CursorMoved` handler) — that one gives
/// the compositor no icon surface today (`start_drag(..., None, ...)`),
/// so this in-window ghost is the only visual feedback that exists while
/// the cursor is still over this window; dragging out into another app
/// still won't show anything under the cursor once it leaves.
fn drag_ghost_files<'a>(paths: &[PathBuf]) -> Element<'a, Message> {
    const ICON_FONT: iced::Font = iced::Font::with_name("Symbols Nerd Font");

    let label = if let [single] = paths {
        single.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    } else {
        format!("{} items", paths.len())
    };
    let icon = if paths.first().is_some_and(|p| p.is_dir()) { "\u{f07b}" } else { "\u{f0f6}" };

    container(
        row![
            text(icon).font(ICON_FONT).size(15).style(move |_| text::Style { color: Some(TEXT()) }),
            text(label).size(13).style(move |_| text::Style { color: Some(TEXT()) }),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 10])
    .style(|_| container::Style {
        background: Some(Background::Color(Color { a: 0.9, ..ACCENT() })),
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    })
    .into()
}

fn drag_ghost_line<'a>(base: Element<'a, Message>, x: f32) -> Element<'a, Message> {
    use iced::widget::stack;

    let line = container(Space::with_width(Length::Fixed(2.0)))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(ACCENT())),
            ..Default::default()
        });

    let overlay = row![
        Space::with_width(Length::Fixed(x)),
        line,
        Space::with_width(Length::Fill),
    ]
    .height(Length::Fill);

    stack![base, overlay].into()
}

/// A thin, drag-to-resize strip between panes. `on_press` starts the drag;
/// the actual resizing happens continuously in `Message::CursorMoved` (see
/// `update`) since iced doesn't give a widget a native drag gesture, and
/// `Message::PaneDragEnd` (bound globally to left-button release) stops it
/// even if the cursor leaves this strip before the button comes up.
fn pane_divider(target: PaneDrag) -> Element<'static, Message> {
    mouse_area(
        container(Space::with_width(Length::Fixed(DIVIDER_WIDTH)))
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(Color { r: 0.25, g: 0.22, b: 0.32, a: 1.0 })),
                ..Default::default()
            })
    )
    .interaction(iced::mouse::Interaction::ResizingHorizontally)
    .on_press(Message::PaneDragStart(target))
    .into()
}

fn context_sep() -> Element<'static, Message> {
    container(iced::widget::Space::with_height(1))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(Color { r: 0.25, g: 0.22, b: 0.32, a: 1.0 })),
            ..Default::default()
        })
        .into()
}
