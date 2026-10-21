use std::mem;

use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
    SendInput,
};

use super::pen_button::PenButton;
use super::pen_driver::PenDriver;
use crate::error::{AutoQvError, Result};

pub struct WindowsPenDriver {
    button: PenButton,
    pressed: bool,
}

impl WindowsPenDriver {
    pub fn new(button: PenButton) -> Self {
        Self {
            button,
            pressed: false,
        }
    }
}

impl PenDriver for WindowsPenDriver {
    fn move_rel(&mut self, dx: i32, dy: i32) -> Result<()> {
        send_mouse(dx, dy, MOUSEEVENTF_MOVE)
    }

    fn press(&mut self) -> Result<()> {
        if self.pressed {
            return Ok(());
        }
        send_mouse(0, 0, button_flags(self.button).0)?;
        self.pressed = true;
        Ok(())
    }

    fn release(&mut self) -> Result<()> {
        if !self.pressed {
            return Ok(());
        }
        send_mouse(0, 0, button_flags(self.button).1)?;
        self.pressed = false;
        Ok(())
    }
}

impl Drop for WindowsPenDriver {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

fn button_flags(button: PenButton) -> (u32, u32) {
    match button {
        PenButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        PenButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        PenButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
    }
}

fn send_mouse(dx: i32, dy: i32, flags: u32) -> Result<()> {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let sent = unsafe { SendInput(1, &input, mem::size_of::<INPUT>() as i32) };
    if sent == 1 {
        Ok(())
    } else {
        let code = unsafe { GetLastError() };
        Err(AutoQvError::Input(format!(
            "SendInput failed (error {code})"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_button_to_its_own_flags() {
        assert_eq!(
            button_flags(PenButton::Left),
            (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP)
        );
        assert_eq!(
            button_flags(PenButton::Right),
            (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP)
        );
        assert_eq!(
            button_flags(PenButton::Middle),
            (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP)
        );
    }
}
