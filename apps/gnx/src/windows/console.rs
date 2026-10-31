use crate::protocol::invalid;
use std::io::{self, Write};
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE},
    System::Console::{
        ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
        ENABLE_VIRTUAL_TERMINAL_INPUT, GetConsoleMode, GetStdHandle, INPUT_RECORD, KEY_EVENT,
        ReadConsoleInputW, STD_INPUT_HANDLE, SetConsoleMode,
    },
};
use zeroize::Zeroizing;

struct InputMode {
    handle: HANDLE,
    original: u32,
    active: bool,
}

impl InputMode {
    fn restore(&mut self) -> io::Result<()> {
        if self.active {
            if unsafe { SetConsoleMode(self.handle, self.original) } == 0 {
                return Err(io::Error::last_os_error());
            }
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for InputMode {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

// Only the interactive password path calls this. Redirected stdin keeps its
// existing line protocol; neither the input value nor its characters are echoed.
pub(super) fn read_masked(prompt: &str) -> io::Result<Zeroizing<String>> {
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if input.is_null() || input == INVALID_HANDLE_VALUE {
        return Err(io::Error::other("consola de entrada no disponible"));
    }
    let mut original = 0;
    if unsafe { GetConsoleMode(input, &mut original) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let masked = original
        & !(ENABLE_ECHO_INPUT
            | ENABLE_LINE_INPUT
            | ENABLE_PROCESSED_INPUT
            | ENABLE_VIRTUAL_TERMINAL_INPUT);
    if unsafe { SetConsoleMode(input, masked) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut mode = InputMode {
        handle: input,
        original,
        active: true,
    };
    let mut output = io::stderr().lock();
    let result = (|| {
        write!(output, "{prompt}")?;
        output.flush()?;
        let mut value = Zeroizing::new(String::new());
        let mut high = None;
        loop {
            let mut record: INPUT_RECORD = unsafe { std::mem::zeroed() };
            let mut count = 0;
            if unsafe { ReadConsoleInputW(input, &mut record, 1, &mut count) } == 0 {
                return Err(io::Error::last_os_error());
            }
            if count == 0 || record.EventType != KEY_EVENT as u16 {
                continue;
            }
            // EventType selects the initialized member of the input union.
            let key = unsafe { record.Event.KeyEvent };
            if key.bKeyDown == 0 {
                continue;
            }
            let unit = unsafe { key.uChar.UnicodeChar };
            for _ in 0..key.wRepeatCount {
                match unit {
                    3 | 27 => {
                        return Err(io::Error::new(
                            io::ErrorKind::Interrupted,
                            "entrada cancelada; no se envió una contraseña",
                        ));
                    }
                    13 => {
                        if high.is_some() {
                            return Err(invalid(
                                "entrada Unicode incompleta; no se envió una contraseña",
                            ));
                        }
                        return Ok(value);
                    }
                    8 => {
                        if high.take().is_none() && value.pop().is_some() {
                            write!(output, "\x08 \x08")?;
                            output.flush()?;
                        }
                        continue;
                    }
                    0 => continue, // Modifiers and navigation keys are not input.
                    0xd800..=0xdbff if high.is_none() => {
                        high = Some(unit);
                        continue;
                    }
                    _ => {}
                }
                let scalar = match (high.take(), unit) {
                    (Some(first), 0xdc00..=0xdfff) => {
                        0x10000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(unit) - 0xdc00)
                    }
                    (None, 0..=0xd7ff | 0xe000..=0xffff) => u32::from(unit),
                    _ => {
                        return Err(invalid(
                            "entrada Unicode inválida; no se envió una contraseña",
                        ));
                    }
                };
                let character = char::from_u32(scalar)
                    .ok_or_else(|| invalid("carácter inválido en la entrada"))?;
                if character.is_control() {
                    continue;
                }
                if value.len() + character.len_utf8() > 1024 {
                    return Err(invalid("contraseña demasiado larga; máximo 1024 bytes"));
                }
                value.push(character);
                write!(output, "*")?;
                output.flush()?;
            }
        }
    })();
    let restored = mode.restore();
    let newline = writeln!(output);
    restored?;
    newline?;
    result
}
