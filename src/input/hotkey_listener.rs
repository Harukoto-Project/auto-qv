use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, WM_HOTKEY, WM_QUIT, WM_USER,
};
use windows_sys::core::BOOL;

use super::hotkey::Hotkey;
use crate::error::{AutoQvError, Result};

const START_ID: i32 = 1;
const ABORT_ID: i32 = 2;
const POLL_INTERVAL: Duration = Duration::from_millis(50);

static CTRL_C_RECEIVED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Start,
    Abort,
}

pub struct HotkeyListener {
    events: Receiver<HotkeyEvent>,
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl HotkeyListener {
    pub fn start(start: Hotkey, abort: Hotkey) -> Result<Self> {
        install_ctrl_handler()?;

        let (event_tx, events) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = thread::spawn(move || run_message_loop(start, abort, event_tx, ready_tx));

        let thread_id = match ready_rx.recv() {
            Ok(Ok(thread_id)) => thread_id,
            Ok(Err(error)) => {
                let _ = thread.join();
                return Err(error);
            }
            Err(_) => {
                return Err(AutoQvError::Input(
                    "hotkey thread terminated unexpectedly".to_string(),
                ));
            }
        };

        Ok(Self {
            events,
            thread_id,
            thread: Some(thread),
        })
    }

    pub fn recv(&self) -> HotkeyEvent {
        loop {
            if CTRL_C_RECEIVED.load(Ordering::SeqCst) {
                return HotkeyEvent::Abort;
            }
            match self.events.recv_timeout(POLL_INTERVAL) {
                Ok(event) => return event,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return HotkeyEvent::Abort,
            }
        }
    }

    pub fn abort_requested(&self) -> bool {
        if CTRL_C_RECEIVED.load(Ordering::SeqCst) {
            return true;
        }
        loop {
            match self.events.try_recv() {
                Ok(HotkeyEvent::Abort) => return true,
                Ok(HotkeyEvent::Start) => continue,
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => return true,
            }
        }
    }
}

impl Drop for HotkeyListener {
    fn drop(&mut self) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> BOOL {
    if ctrl_type == CTRL_C_EVENT || ctrl_type == CTRL_BREAK_EVENT {
        CTRL_C_RECEIVED.store(true, Ordering::SeqCst);
        1
    } else {
        0
    }
}

fn install_ctrl_handler() -> Result<()> {
    let installed = unsafe { SetConsoleCtrlHandler(Some(ctrl_handler), 1) };
    if installed == 0 {
        let code = unsafe { GetLastError() };
        return Err(AutoQvError::Input(format!(
            "failed to install the Ctrl+C handler (error {code})"
        )));
    }
    Ok(())
}

fn register(id: i32, hotkey: &Hotkey) -> Result<()> {
    let registered = unsafe {
        RegisterHotKey(
            ptr::null_mut(),
            id,
            hotkey.modifiers | MOD_NOREPEAT,
            hotkey.key,
        )
    };
    if registered == 0 {
        let code = unsafe { GetLastError() };
        return Err(AutoQvError::Input(format!(
            "failed to register hotkey {hotkey} (error {code}); it may be in use by another application"
        )));
    }
    Ok(())
}

fn unregister_all() {
    unsafe {
        UnregisterHotKey(ptr::null_mut(), START_ID);
        UnregisterHotKey(ptr::null_mut(), ABORT_ID);
    }
}

fn run_message_loop(
    start: Hotkey,
    abort: Hotkey,
    events: Sender<HotkeyEvent>,
    ready: Sender<Result<u32>>,
) {
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    unsafe {
        PeekMessageW(&mut msg, ptr::null_mut(), WM_USER, WM_USER, PM_NOREMOVE);
    }

    if let Err(error) = register(START_ID, &start).and_then(|()| register(ABORT_ID, &abort)) {
        unregister_all();
        let _ = ready.send(Err(error));
        return;
    }

    let thread_id = unsafe { GetCurrentThreadId() };
    if ready.send(Ok(thread_id)).is_err() {
        unregister_all();
        return;
    }

    while unsafe { GetMessageW(&mut msg, ptr::null_mut(), 0, 0) } > 0 {
        if msg.message != WM_HOTKEY {
            continue;
        }
        let event = match msg.wParam as i32 {
            START_ID => HotkeyEvent::Start,
            ABORT_ID => HotkeyEvent::Abort,
            _ => continue,
        };
        if events.send(event).is_err() {
            break;
        }
    }

    unregister_all();
}
