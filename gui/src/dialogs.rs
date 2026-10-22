use std::num::NonZeroIsize;
use std::path::PathBuf;
use std::sync::Arc;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

/// Selettore di cartelle: (titolo, cartella iniziale) → cartella scelta, `None` se annullato.
pub type FolderPicker = Arc<dyn Fn(&str, Option<PathBuf>) -> Option<PathBuf> + Send + Sync>;

/// Selettore di cartelle di Windows, aperto dal server (REQ-030).
pub fn system_folder_picker() -> FolderPicker {
    Arc::new(|title, initial| {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        if let Some(dir) = initial.filter(|d| d.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        // La finestra in primo piano è il browser da cui è partito il clic: il dialogo le si
        // apre davanti invece di finire dietro.
        if let Some(owner) = ForegroundWindow::current() {
            dialog = dialog.set_parent(&owner);
        }
        dialog.pick_folder()
    })
}

struct ForegroundWindow(NonZeroIsize);

impl ForegroundWindow {
    fn current() -> Option<Self> {
        let hwnd = unsafe { GetForegroundWindow() };
        NonZeroIsize::new(hwnd as isize).map(ForegroundWindow)
    }
}

impl HasWindowHandle for ForegroundWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = RawWindowHandle::Win32(Win32WindowHandle::new(self.0));
        // L'handle resta valido per la durata del dialogo: è una finestra di primo livello.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for ForegroundWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}
