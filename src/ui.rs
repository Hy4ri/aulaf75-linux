use crate::{text_input::TextInput, theme};
use aulaf75_linux::{
    hardware::Hardware,
    model::{Binding, Catalogue, Macro, MacroEvent, Workspace},
};
use gpui::{prelude::*, *};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Lighting,
    Keyboard,
    Macros,
    Profiles,
    Settings,
}
#[derive(Clone)]
pub enum Action {
    Page(Page),
    Effect(usize),
    Brightness(i8),
    Speed(i8),
    Color(u8),
    SelectKey(usize),
    Layer(u8),
    Usage(i16),
    Modifier(u8),
    BindKey,
    BindMedia(u16),
    DisableKey,
    ResetKey,
    BindMacro(usize),
    Profile(usize),
    NewProfile,
    DuplicateProfile,
    DeleteProfile,
    RenameProfile,
    Save,
    Export,
    Import,
    MacroSelect(usize),
    MacroNew,
    MacroDelete,
    MacroRename,
    MacroEvent(bool),
    MacroRemove(usize),
    MacroRecord,
    MacroRepeat(i16),
    Sleep(u16),
    Debounce(i8),
    Refresh,
    ConnectAdmin,
    ApplyLighting,
}

pub struct AppView {
    pub catalogue: Catalogue,
    pub workspace: Workspace,
    pub page: Page,
    pub status: String,
    pub device_status: String,
    pub busy: bool,
    pub selected_key: usize,
    pub layer: u8,
    pub pending_usage: u8,
    pub pending_modifiers: u8,
    pub selected_macro: Option<usize>,
    pub recording: bool,
    pub name_input: Entity<TextInput>,
    pub path_input: Entity<TextInput>,
    pub macro_name_input: Entity<TextInput>,
    pub macro_usage_input: Entity<TextInput>,
    pub macro_delay_input: Entity<TextInput>,
    pub storage_path: PathBuf,
    pub hardware: Option<Arc<Mutex<Hardware>>>,
    focus: FocusHandle,
    last_macro_event: Option<Instant>,
    generation: u64,
}
impl AppView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let catalogue = Catalogue::bundled().expect("Bundled F75 catalogue must parse");
        let storage_path = std::env::var_os("AULA_PROFILE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let root = std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        PathBuf::from(std::env::var_os("HOME").unwrap()).join(".config")
                    });
                root.join("aulaf75-linux/profiles.json")
            });
        let (workspace, status) = if storage_path.exists() {
            match Workspace::load(&storage_path, &catalogue) {
                Ok(w) => (w, "Loaded local profiles. Device state is separate.".into()),
                Err(e) => (
                    Workspace::new(&catalogue),
                    format!("Could not load local profiles: {e}. Existing file unchanged."),
                ),
            }
        } else {
            (
                Workspace::new(&catalogue),
                "Local draft. No keyboard setting has been applied.".into(),
            )
        };
        let name = workspace.current().name.clone();
        let export_path = storage_path
            .with_file_name("export.json")
            .display()
            .to_string();
        Self {
            catalogue,
            workspace,
            page: Page::Lighting,
            status,
            device_status: "Not inspected — click Inspect device".into(),
            busy: false,
            selected_key: 1,
            layer: 0,
            pending_usage: 4,
            pending_modifiers: 0,
            selected_macro: None,
            recording: false,
            name_input: cx.new(|cx| TextInput::new(cx, "Profile name", name)),
            path_input: cx.new(|cx| TextInput::new(cx, "Import / export JSON path", export_path)),
            macro_name_input: cx.new(|cx| TextInput::new(cx, "Macro name", "Macro 1")),
            macro_usage_input: cx.new(|cx| TextInput::new(cx, "HID usage (hex)", "04")),
            macro_delay_input: cx.new(|cx| TextInput::new(cx, "Delay in milliseconds", "100")),
            storage_path,
            hardware: None,
            focus: cx.focus_handle(),
            last_macro_event: None,
            generation: 0,
        }
    }
    pub fn dispatch(&mut self, action: Action, _window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = false;
        let key = format!("{}:{}", self.layer, self.selected_key);
        match action {
            Action::Page(p) => {
                self.page = p;
                self.recording = false;
            }
            Action::Effect(i) => {
                self.workspace.current_mut().effect_slot = i;
                changed = true;
            }
            Action::Brightness(d) => {
                let p = self.workspace.current_mut();
                p.brightness = (p.brightness as i16 + d as i16).clamp(0, 9) as u8;
                changed = true;
            }
            Action::Speed(d) => {
                let p = self.workspace.current_mut();
                p.speed = (p.speed as i16 + d as i16).clamp(0, 4) as u8;
                changed = true;
            }
            Action::Color(v) => {
                self.workspace.current_mut().color = v;
                changed = true;
            }
            Action::SelectKey(id) => self.selected_key = id,
            Action::Layer(layer) => self.layer = layer,
            Action::Usage(d) => {
                self.pending_usage = (self.pending_usage as i16 + d).clamp(1, 255) as u8
            }
            Action::Modifier(bit) => self.pending_modifiers ^= bit,
            Action::BindKey => {
                self.workspace.current_mut().remaps.insert(
                    key,
                    Binding::Key {
                        usage: self.pending_usage,
                        modifiers: self.pending_modifiers,
                    },
                );
                changed = true;
            }
            Action::BindMedia(usage) => {
                self.workspace
                    .current_mut()
                    .remaps
                    .insert(key, Binding::Media { usage });
                changed = true;
            }
            Action::DisableKey => {
                self.workspace
                    .current_mut()
                    .remaps
                    .insert(key, Binding::Disabled);
                changed = true;
            }
            Action::ResetKey => {
                self.workspace.current_mut().remaps.remove(&key);
                changed = true;
            }
            Action::BindMacro(index) => {
                if index < self.workspace.current().macros.len() {
                    self.workspace
                        .current_mut()
                        .remaps
                        .insert(key, Binding::Macro { index });
                    changed = true;
                }
            }
            Action::Profile(i) => {
                if i < self.workspace.profiles.len() {
                    self.workspace.selected = i;
                    self.sync_name(cx);
                    self.selected_macro = None;
                    self.recording = false;
                }
            }
            Action::NewProfile => {
                let mut p = Workspace::new(&self.catalogue).profiles.remove(0);
                p.name = format!("Profile {}", self.workspace.profiles.len() + 1);
                self.workspace.profiles.push(p);
                self.workspace.selected = self.workspace.profiles.len() - 1;
                self.sync_name(cx);
                self.selected_macro = None;
                self.recording = false;
                changed = true;
            }
            Action::DuplicateProfile => {
                let mut p = self.workspace.current().clone();
                p.name = format!("{} copy", p.name);
                self.workspace.profiles.push(p);
                self.workspace.selected = self.workspace.profiles.len() - 1;
                self.sync_name(cx);
                self.selected_macro = None;
                self.recording = false;
                changed = true;
            }
            Action::DeleteProfile => {
                if self.workspace.profiles.len() > 1 {
                    self.workspace.profiles.remove(self.workspace.selected);
                    self.workspace.selected = self
                        .workspace
                        .selected
                        .min(self.workspace.profiles.len() - 1);
                    self.sync_name(cx);
                    self.selected_macro = None;
                    changed = true;
                }
            }
            Action::RenameProfile => {
                let name = self.name_input.read(cx).text();
                if !name.trim().is_empty() {
                    self.workspace.current_mut().name = name;
                    changed = true;
                } else {
                    self.status = "Profile name must not be empty".into();
                }
            }
            Action::Save => {
                self.status = match self.workspace.save(&self.storage_path) {
                    Ok(()) => format!(
                        "Saved local profiles to {}. No hardware write.",
                        self.storage_path.display()
                    ),
                    Err(e) => format!("Save failed: {e:#}"),
                };
            }
            Action::Export => {
                let path = PathBuf::from(self.path_input.read(cx).text());
                self.status = match self.workspace.save(&path) {
                    Ok(()) => format!("Exported native workspace JSON to {}", path.display()),
                    Err(e) => format!("Export failed: {e:#}"),
                };
            }
            Action::Import => {
                let path = PathBuf::from(self.path_input.read(cx).text());
                match Workspace::load(&path, &self.catalogue) {
                    Ok(w) => {
                        self.workspace = w;
                        self.sync_name(cx);
                        self.selected_macro = None;
                        self.recording = false;
                        self.status =
                            "Imported local profiles. Nothing applied to hardware.".into();
                    }
                    Err(e) => {
                        self.status = format!("Import rejected; current draft unchanged: {e:#}")
                    }
                }
            }
            Action::MacroSelect(i) => {
                self.selected_macro = Some(i);
                if let Some(m) = self.workspace.current().macros.get(i) {
                    let name = m.name.clone();
                    self.macro_name_input
                        .update(cx, |input, cx| input.set_text(name, cx));
                }
                self.recording = false;
            }
            Action::MacroNew => {
                let name = self.macro_name_input.read(cx).text();
                if name.trim().is_empty() {
                    self.status = "Macro name must not be empty".into();
                } else {
                    let p = self.workspace.current_mut();
                    p.macros.push(Macro {
                        name,
                        events: vec![],
                        repeat: 1,
                    });
                    self.selected_macro = Some(p.macros.len() - 1);
                    changed = true;
                }
            }
            Action::MacroRename => {
                if let Some(i) = self.selected_macro {
                    let name = self.macro_name_input.read(cx).text();
                    if !name.trim().is_empty() {
                        if let Some(m) = self.workspace.current_mut().macros.get_mut(i) {
                            m.name = name;
                            changed = true;
                        }
                    }
                }
            }
            Action::MacroDelete => {
                if let Some(i) = self.selected_macro.take() {
                    let p = self.workspace.current_mut();
                    if i < p.macros.len() {
                        p.macros.remove(i);
                        p.remaps
                            .retain(|_, b| !matches!(b,Binding::Macro {index} if *index==i));
                        for b in p.remaps.values_mut() {
                            if let Binding::Macro { index } = b {
                                if *index > i {
                                    *index -= 1;
                                }
                            }
                        }
                        changed = true;
                    }
                    self.recording = false;
                }
            }
            Action::MacroEvent(pressed) => {
                if let Some(i) = self.selected_macro {
                    let usage = u8::from_str_radix(
                        self.macro_usage_input
                            .read(cx)
                            .text()
                            .trim()
                            .trim_start_matches("0x"),
                        16,
                    );
                    let delay = self.macro_delay_input.read(cx).text().trim().parse::<u32>();
                    match (usage, delay) {
                        (Ok(usage), Ok(delay_ms)) if usage > 0 && delay_ms <= 0xfffff => {
                            if let Some(m) = self.workspace.current_mut().macros.get_mut(i) {
                                m.events.push(MacroEvent {
                                    usage,
                                    pressed,
                                    delay_ms,
                                });
                                changed = true;
                            }
                        }
                        _ => {
                            self.status =
                                "Enter a nonzero hex HID usage and delay 0..1048575 ms".into()
                        }
                    }
                }
            }
            Action::MacroRemove(idx) => {
                if let Some(i) = self.selected_macro {
                    if let Some(m) = self.workspace.current_mut().macros.get_mut(i) {
                        if idx < m.events.len() {
                            m.events.remove(idx);
                            changed = true;
                        }
                    }
                }
            }
            Action::MacroRepeat(d) => {
                if let Some(i) = self.selected_macro {
                    if let Some(m) = self.workspace.current_mut().macros.get_mut(i) {
                        m.repeat = (m.repeat as i32 + d as i32).clamp(1, 65535) as u16;
                        changed = true;
                    }
                }
            }
            Action::MacroRecord => {
                if self.selected_macro.is_some() {
                    self.recording = !self.recording;
                    self.last_macro_event = None;
                    self.focus.focus(_window);
                    self.status=if self.recording {"Recording keys only while this app window is focused. Click Stop to finish."} else {"Recording stopped; macro is a local draft."}.into();
                }
            }
            Action::Sleep(s) => {
                self.workspace.current_mut().sleep_seconds = s;
                changed = true;
            }
            Action::Debounce(d) => {
                let p = self.workspace.current_mut();
                p.debounce_ms = (p.debounce_ms as i16 + d as i16).clamp(0, 255) as u8;
                changed = true;
            }
            Action::Refresh => {
                self.inspect(false, cx);
            }
            Action::ConnectAdmin => {
                self.inspect(true, cx);
            }
            Action::ApplyLighting => {
                self.apply_lighting(cx);
            }
        }
        if changed {
            self.status="Draft changed — Save locally or apply Lighting separately. Remap/macro device encodings remain unvalidated.".into();
        }
        cx.notify();
    }
    fn sync_name(&mut self, cx: &mut Context<Self>) {
        let name = self.workspace.current().name.clone();
        self.name_input.update(cx, |i, cx| i.set_text(name, cx));
    }
    fn inspect(&mut self, admin_prompt: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.hardware = None;
        self.generation += 1;
        let generation = self.generation;
        self.device_status = if admin_prompt {
            self.status = "Approve the system password dialog to grant only your user temporary F75 control access. The app stays non-root.".into();
            "Requesting device access, then validating the keyboard…"
        } else {
            "Checking device identity and reading its profile…"
        }.into();
        let cat = self.catalogue.clone();
        cx.spawn(async move |this,cx|{
            let result=cx.background_spawn(async move {
                if admin_prompt { Hardware::connect_with_admin_prompt(&cat) } else { Hardware::connect(&cat) }
            }).await;
            let _=this.update(cx,|this,cx|{if this.generation!=generation{return;}this.busy=false;
                match result {Ok(hw)=>{this.device_status=hw.description();this.hardware=Some(Arc::new(Mutex::new(hw)));this.status="Device identity and lighting profile validated. Local draft not applied.".into();},Err(e)=>{this.device_status="Device unavailable".into();this.status=format!("Inspect failed: {e:#}");}}
                cx.notify();
            });
        }).detach();
    }
    fn apply_lighting(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(hw) = self.hardware.clone() else {
            self.status = "Inspect and validate the device before applying lighting.".into();
            return;
        };
        let profile = self.workspace.current().clone();
        let applied_name = profile.name.clone();
        let cat = self.catalogue.clone();
        self.busy = true;
        self.status = "Applying lighting and checking hardware readback…".into();
        cx.spawn(async move |this,cx|{
            let result=cx.background_spawn(async move {hw.lock().map_err(|_|anyhow::anyhow!("Device lock poisoned"))?.apply_lighting(&profile,&cat)}).await;
            let _=this.update(cx,|this,cx|{this.busy=false;this.status=match result {Ok(_)=>format!("Lighting readback verified for captured draft '{applied_name}'. Confirm the physical effect; other settings and later edits were not sent."),Err(e)=>format!("Lighting apply failed (not claimed applied): {e:#}")};cx.notify();});
        }).detach();
    }
    fn record_key(&mut self, key: &str, pressed: bool, is_held: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.recording || self.page != Page::Macros || is_held {
            return;
        }
        if [&self.macro_name_input, &self.macro_usage_input, &self.macro_delay_input]
            .iter().any(|input| input.read(cx).focus_handle(cx).is_focused(window)) {
            self.recording = false;
            self.last_macro_event = None;
            self.status = "Recording stopped while editing a text field; those keys were not recorded.".into();
            cx.notify();
            return;
        }
        let Some(i) = self.selected_macro else {
            return;
        };
        let Some(usage) = key_usage(key) else {
            return;
        };
        let now = Instant::now();
        let delay_ms = self
            .last_macro_event
            .map(|t| now.duration_since(t).as_millis().min(0xfffff) as u32)
            .unwrap_or(0);
        self.last_macro_event = Some(now);
        if let Some(m) = self.workspace.current_mut().macros.get_mut(i) {
            m.events.push(MacroEvent {
                usage,
                pressed,
                delay_ms,
            });
            cx.notify();
        }
    }
}

pub fn button(
    cx: &mut Context<AppView>,
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    action: Action,
    active: bool,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let label: SharedString = label.into();
    let selector = id.clone();
    div()
        .debug_selector(move || selector.to_string())
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .text_sm()
        .border_1()
        .border_color(if active {
            theme::accent()
        } else {
            theme::border()
        })
        .bg(if active {
            theme::input_bg()
        } else {
            theme::panel()
        })
        .cursor_pointer()
        .hover(|s| s.bg(theme::input_bg()))
        .on_click(cx.listener(move |this, _, window, cx| this.dispatch(action.clone(), window, cx)))
        .child(label)
}
pub fn heading(label: impl Into<SharedString>) -> Div {
    let label: SharedString = label.into();
    div()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
        .mb_3()
        .child(label)
}
pub fn hint(label: impl Into<SharedString>) -> Div {
    let label: SharedString = label.into();
    div()
        .text_sm()
        .text_color(theme::muted())
        .mb_3()
        .child(label)
}
impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.page {
            Page::Lighting => crate::lighting_view::render(self, window, cx),
            Page::Keyboard => crate::keyboard_view::render(self, window, cx),
            Page::Macros => crate::workspace_views::macros(self, window, cx),
            Page::Profiles => crate::workspace_views::profiles(self, window, cx),
            Page::Settings => crate::workspace_views::settings(self, window, cx),
        };
        let sidebar = div()
            .w(px(170.))
            .flex_shrink_0()
            .p_4()
            .bg(theme::panel())
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .mb_4()
                    .child("AULA F75"),
            )
            .children(
                [
                    (Page::Lighting, "Lighting"),
                    (Page::Keyboard, "Key mapping"),
                    (Page::Macros, "Macros"),
                    (Page::Profiles, "Profiles"),
                    (Page::Settings, "Settings"),
                ]
                .into_iter()
                .map(|(p, label)| {
                    button(
                        cx,
                        format!("nav-{p:?}"),
                        label,
                        Action::Page(p),
                        self.page == p,
                    )
                }),
            );
        div()
            .id("aula-root")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::background())
            .text_color(theme::foreground())
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                this.record_key(&e.keystroke.key, true, e.is_held, window, cx)
            }))
            .on_key_up(cx.listener(|this, e: &KeyUpEvent, window, cx| {
                this.record_key(&e.keystroke.key, false, false, window, cx)
            }))
            .child(
                div().flex().flex_1().min_h_0().child(sidebar).child(
                    div()
                        .id(SharedString::from(format!("page-scroll-{:?}",self.page)))
                        .flex_1()
                        .min_w_0()
                        .p_5()
                        .overflow_y_scroll()
                        .child(body),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .border_t_1()
                    .border_color(theme::border())
                    .p_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .child(self.device_status.clone()),
                    )
                    .child(button(
                        cx,
                        "inspect",
                        "Inspect device",
                        Action::Refresh,
                        false,
                    ))
                    .child(button(
                        cx,
                        "connect-admin",
                        if self.busy {
                            "Device busy…"
                        } else {
                            "Connect (admin prompt)"
                        },
                        Action::ConnectAdmin,
                        false,
                    ))
                    .child(button(
                        cx,
                        "save-local",
                        "Save locally",
                        Action::Save,
                        false,
                    )),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme::muted())
                    .p_3()
                    .w_full()
                    .child(self.status.clone()),
            )
    }
}
fn key_usage(key: &str) -> Option<u8> {
    let s = key.to_ascii_lowercase();
    if s.len() == 1 {
        let b = s.as_bytes()[0];
        if b.is_ascii_lowercase() {
            return Some(4 + b - b'a');
        }
        if (b'1'..=b'9').contains(&b) {
            return Some(30 + b - b'1');
        }
        if b == b'0' {
            return Some(39);
        }
    }
    match s.as_str() {
        "enter" => Some(40),
        "escape" => Some(41),
        "backspace" => Some(42),
        "tab" => Some(43),
        "space" | " " => Some(44),
        "left" => Some(80),
        "right" => Some(79),
        "up" => Some(82),
        "down" => Some(81),
        _ => None,
    }
}
