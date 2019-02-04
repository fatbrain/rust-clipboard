/*
Copyright 2016 Avraham Weinstock

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use clipboard_win::{Clipboard, formats, get_clipboard_string, set_clipboard_string};

use common::ClipboardProvider;
use std::error::Error;
use std::io;

use std::os::windows::prelude::*;

pub struct WindowsClipboardContext;

pub struct Handler;

use winapi::shared::windef::HWND;

fn create_window() -> io::Result<HWND> {
    use winapi::um::winuser::{CreateWindowExW, HWND_MESSAGE};
    use ::std::ptr;
    use std::ffi::OsStr;
    use std::iter::once;
    let class_name: Vec<u16> = OsStr::new("STATIC").encode_wide().chain(once(0)).collect();

    let result = unsafe { CreateWindowExW(
        0, class_name.as_ptr(), ptr::null(),
        0, 0, 0, 0, 0,
        HWND_MESSAGE, ptr::null_mut(),
        ptr::null_mut(), ptr::null_mut())
    };

    if result.is_null() {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

pub struct ClipboardListener(HWND);

impl Iterator for ClipboardListener {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match get_msg(self.0) {
            Ok(_) => Some(0),
            Err(_) => Some(1),
        }
    }
}

impl ClipboardListener {
    pub fn new() -> io::Result<Self> {
        let hwnd = create_window()?;
        if unsafe { winapi::um::winuser::AddClipboardFormatListener(hwnd) } == 1 {
            Ok(ClipboardListener(hwnd))
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

impl Drop for ClipboardListener {
    fn drop(&mut self) {
        unsafe { winapi::um::winuser::RemoveClipboardFormatListener(self.0) };
    }
}

use winapi::um::winuser::{GetMessageW, LPMSG, MSG, WM_CLIPBOARDUPDATE};

pub fn get_msg(hwnd: HWND) -> io::Result<MSG> {
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    let result = unsafe { GetMessageW(&mut msg as LPMSG, hwnd, WM_CLIPBOARDUPDATE, WM_CLIPBOARDUPDATE) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(msg)
    }
}

impl ClipboardProvider<ClipboardListener> for WindowsClipboardContext {
    fn new() -> Result<Self, Box<Error>> {
        Ok(WindowsClipboardContext)
    }
    fn get_contents(&mut self) -> Result<String, Box<Error>> {
        Ok(get_clipboard_string()?)
    }
    fn set_contents(&mut self, data: String) -> Result<(), Box<Error>> {
        Ok(set_clipboard_string(&data)?)
    }
    fn iter(&mut self) -> ClipboardListener {
        ClipboardListener::new().unwrap()
    }
}
