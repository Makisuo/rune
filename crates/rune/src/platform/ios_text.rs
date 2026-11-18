//! iOS's text proxy: a focused `text_input` takes the keyboard through a hidden `UITextView`
//! instead of winit's view. winit's view answers only `UIKeyInput` (insert, delete back), so the
//! keyboard offers it no autocorrection, predictions, autocapitalization or text replacement;
//! the text view does all of it, with dictation and marked text, against a copy of the field.
//!
//! The copy runs both ways. Each change the text view makes (typed, corrected, dictated, its
//! selection moved) queues its whole text and selection (`Runtime::platform_text`), which the
//! field takes as typed. After each frame [`sync`] gives the text view the field's text and
//! selection when the app changed them (cleared after a send, a mention inserted), unless an
//! edit of the text view's own is still on its way to the field. Return is Enter, as winit
//! sends it, so a field's submit and key handlers hear it.
//!
//! The text view lies over the field's caret (`Runtime::ime_area`), so the system draws its
//! correction bubbles there, and is clear and takes no touches: Rune draws the field and hears
//! the presses on it. An `El::ime_cursor` surface (an app's own editor) keeps winit's view.
use std::cell::{Cell, RefCell};

use objc2_foundation_video::{NSObject, NSObjectProtocol, NSRange, NSString};
use objc2_ui_kit::{NSObjectUIAccessibility, UIColor, UIScrollViewDelegate, UITextInput, UITextView, UITextViewDelegate, UIView};
use objc2_video::rc::Retained;
use objc2_video::runtime::{Bool, ProtocolObject};
use objc2_video::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::utf16::{byte_of, utf16_of};
use crate::runtime::Runtime;

/// What the text view did, waiting for the next frame to reach the runtime.
enum Report {
    /// Its whole text and selection (byte anchor, focus) after a change.
    Text(String, usize, usize),
    /// The return key.
    Enter,
}

struct Proxy {
    view: Retained<UITextView>,
    /// The view's delegate; the view holds it weakly.
    _delegate: Retained<Delegate>,
    /// winit's view, which the text view lies in: a report asks it to draw a frame.
    host: Retained<UIView>,
}

thread_local! {
    static PROXY: RefCell<Option<Proxy>> = const { RefCell::new(None) };
    static REPORTS: RefCell<Vec<Report>> = const { RefCell::new(Vec::new()) };
    /// Text reports queued in the runtime that it has not applied yet: the field is behind the
    /// text view until then, and `sync` leaves the view alone.
    static PENDING: Cell<u32> = const { Cell::new(0) };
    /// What the text view holds as last reported or set: text and byte selection.
    static KNOWN: RefCell<(String, usize, usize)> = const { RefCell::new((String::new(), 0, 0)) };
    /// Set while `sync` changes the view, whose delegate then reports nothing.
    static SETTING: Cell<bool> = const { Cell::new(false) };
    /// Whether the window's input method is on (`set_allowed`).
    static WANTED: Cell<bool> = const { Cell::new(false) };
    /// Whether winit's own view has the keyboard (an app editor's `ime_cursor`).
    static WINIT_ON: Cell<bool> = const { Cell::new(false) };
    /// A composition to end at the next `sync` (`Runtime::ime_reset`).
    static RESET: Cell<bool> = const { Cell::new(false) };
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RuneTextProxyDelegate"]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl UIScrollViewDelegate for Delegate {}

    unsafe impl UITextViewDelegate for Delegate {
        #[unsafe(method(textViewDidChange:))]
        fn did_change(&self, view: &UITextView) { report(view); }

        #[unsafe(method(textViewDidChangeSelection:))]
        fn did_change_selection(&self, view: &UITextView) { report(view); }

        #[unsafe(method(textView:shouldChangeTextInRange:replacementText:))]
        fn should_change(&self, view: &UITextView, _range: NSRange, text: &NSString) -> Bool {
            // Return is Enter, unless it ends a composition (a CJK keyboard's confirm).
            if SETTING.get() || text.to_string() != "\n" || view.markedTextRange().is_some() { return Bool::YES; }
            REPORTS.with_borrow_mut(|r| r.push(Report::Enter));
            redraw();
            Bool::NO
        }
    }
);

impl Delegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: NSObject's designated initializer, on a fresh allocation.
        unsafe { msg_send![super(this), init] }
    }
}

/// Queues the text view's text and selection for the field.
fn report(view: &UITextView) {
    if SETTING.get() { return; }
    let text = view.text().to_string();
    // `selectedRanges` is iOS 26's; one range is all a field has.
    #[allow(deprecated)]
    let range = view.selectedRange();
    let start = byte_of(&text, range.location);
    let end = byte_of(&text, range.location + range.length);
    let same = KNOWN.with_borrow(|k| k.0 == text && k.1 == start && k.2 == end);
    if same { return; }
    KNOWN.set((text.clone(), start, end));
    REPORTS.with_borrow_mut(|r| r.push(Report::Text(text, start, end)));
    redraw();
}

fn redraw() {
    PROXY.with_borrow(|p| if let Some(p) = p { p.host.setNeedsDisplay(); });
}

/// Turns the window's input method on or off. Off takes the keyboard away at once; on waits
/// for the next [`sync`], which knows whether a text field or an app editor has the focus.
pub(crate) fn set_allowed(window: &Window, allowed: bool) {
    WANTED.set(allowed);
    if allowed { return; }
    resign();
    if WINIT_ON.replace(false) { window.set_ime_allowed(false); }
}

/// Ends the text view's composition at the next [`sync`], as winit's off-and-on does.
pub(crate) fn reset() { RESET.set(true); }

/// A text report the runtime queued has run (`Runtime::run_platform_text`).
pub(crate) fn applied() { PENDING.set(PENDING.get().saturating_sub(1)); }

/// Hands what the text view did since the last frame to the runtime, before it builds one.
pub(crate) fn drain<S: 'static>(rt: &mut Runtime<S>) {
    for report in REPORTS.take() {
        match report {
            Report::Text(text, anchor, focus) => {
                PENDING.set(PENDING.get() + 1);
                rt.platform_text(text, anchor, focus);
            }
            Report::Enter => rt.key(crate::KeyEvent { key: crate::Key::Enter, mods: crate::Mods::default() }),
        }
    }
}

/// After a frame: gives the keyboard to the text view (a focused field) or winit's view (an
/// app editor), and the text view the field's text and selection when the app changed them.
pub(crate) fn sync<S: 'static>(window: &Window, rt: &mut Runtime<S>) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let field = if WANTED.get() { rt.focused_text() } else { None };
    let Some((text, anchor, focus)) = field else {
        resign();
        // winit's view only for an app's own editor: a field's caret whose field lost the
        // runtime's focus (a press elsewhere) wants no keyboard.
        let winit = WANTED.get() && rt.ime_app;
        if WINIT_ON.replace(winit) != winit { window.set_ime_allowed(winit); }
        return;
    };
    if WINIT_ON.replace(false) { window.set_ime_allowed(false); }
    let Some(view) = proxy(mtm, window) else { return };
    SETTING.set(true);
    if let Some(a) = rt.ime_area {
        view.setFrame(CGRect::new(CGPoint::new(a[0] as f64, a[1] as f64), CGSize::new(a[2].max(1.0) as f64, a[3].max(1.0) as f64)));
    }
    if RESET.replace(false) { view.unmarkText(); }
    if PENDING.get() == 0 && REPORTS.with_borrow(Vec::is_empty) {
        let (same_text, same_selection) = KNOWN.with_borrow(|k| (k.0 == text, k.1 == anchor && k.2 == focus));
        if !same_text {
            if view.markedTextRange().is_some() { view.unmarkText(); }
            view.setText(Some(&NSString::from_str(&text)));
        }
        if !same_text || !same_selection {
            let (start, end) = (anchor.min(focus), anchor.max(focus));
            let location = utf16_of(&text, start);
            #[allow(deprecated)]
            view.setSelectedRange(NSRange::new(location, utf16_of(&text, end) - location));
            KNOWN.set((text, anchor, focus));
        }
    }
    SETTING.set(false);
    if !view.isFirstResponder() { view.becomeFirstResponder(); }
}

/// The text view, made the first time a field takes the keyboard.
fn proxy(mtm: MainThreadMarker, window: &Window) -> Option<Retained<UITextView>> {
    if let Some(view) = PROXY.with_borrow(|p| p.as_ref().map(|p| p.view.clone())) { return Some(view); }
    let RawWindowHandle::UiKit(handle) = window.window_handle().ok()?.as_raw() else { return None };
    // SAFETY: winit's handle names its live `UIView`, on the main thread.
    let host: Retained<UIView> = unsafe { Retained::retain(handle.ui_view.as_ptr().cast())? };
    let view = UITextView::new(mtm);
    let delegate = Delegate::new(mtm);
    // SAFETY: `Proxy` keeps the delegate alive as long as the view.
    unsafe { view.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
    let clear = UIColor::clearColor();
    view.setBackgroundColor(Some(&clear));
    view.setTextColor(Some(&clear));
    // SAFETY: a colour, set on the main thread. Clear hides the view's own caret.
    unsafe { view.setTintColor(Some(&clear)) };
    // Rune draws the field and hears the presses on it.
    view.setUserInteractionEnabled(false);
    view.setAccessibilityElementsHidden(true, mtm);
    host.addSubview(&view);
    PROXY.set(Some(Proxy { view: view.clone(), _delegate: delegate, host }));
    KNOWN.set((String::new(), 0, 0));
    Some(view)
}

fn resign() {
    PROXY.with_borrow(|p| {
        if let Some(p) = p && p.view.isFirstResponder() {
            SETTING.set(true);
            p.view.resignFirstResponder();
            // UIKit hands the first responder to the text view's superview, winit's view, which
            // would keep a keyboard up that types nowhere.
            if !WINIT_ON.get() && p.host.isFirstResponder() { p.host.resignFirstResponder(); }
            SETTING.set(false);
        }
    });
}
