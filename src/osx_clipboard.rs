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

use common::*;
use objc::runtime::{Class, Object};
use objc_foundation::{INSArray, INSObject, INSString};
use objc_foundation::{NSArray, NSDictionary, NSObject, NSString};
use objc_id::{Id, Owned, Shared};
use std::mem::transmute;
use std::{error::Error, thread, time::Duration};

#[derive(thiserror::Error, Debug)]
pub enum OSXError {
    #[error(r#"Class::get("NSPasteboard")"#)]
    NSPasteboard,
    #[error(r#"NSPasteboard#generalPasteboard returned null"#)]
    NSPasteboardNull,
    #[error("pasteboard#readObjectsForClasses:options: returned null")]
    ReturnedNull,
    #[error("pasteboard#readObjectsForClasses:options: returned empty")]
    ReturnedEmpty,
    #[error("NSPasteboard#writeObjects: returned false")]
    ReturnedFalse,
}

pub struct OSXClipboardContext {
    pasteboard: Id<Object, Shared>,
}

pub struct ClipboardListener {
    pasteboard: Id<Object, Shared>,
    change_count: i64,
}

impl Iterator for ClipboardListener {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            thread::sleep(Duration::from_millis(100));
            let change_count: i64 = unsafe { msg_send![self.pasteboard, changeCount] };
            if change_count != self.change_count {
                self.change_count = change_count;
                return Some(0);
            }
        }
    }
}

// required to bring NSPasteboard into the path of the class-resolver
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

impl ClipboardProvider<ClipboardListener> for OSXClipboardContext {
    fn new() -> Result<OSXClipboardContext, Box<Error>> {
        let cls = Class::get("NSPasteboard").ok_or(OSXError::NSPasteboard)?;
        let pasteboard: *mut Object = unsafe { msg_send![cls, generalPasteboard] };
        if pasteboard.is_null() {
            return Err(OSXError::NSPasteboardNull)?;
        }
        let pasteboard: Id<Object> = unsafe { Id::from_ptr(pasteboard) };
        let pasteboard = pasteboard.share();
        Ok(OSXClipboardContext { pasteboard })
    }
    fn get_contents(&mut self) -> Result<String, Box<Error>> {
        let string_class: Id<NSObject> = {
            let cls: Id<Class> = unsafe { Id::from_ptr(class("NSString")) };
            unsafe { transmute(cls) }
        };
        let classes: Id<NSArray<NSObject, Owned>> = NSArray::from_vec(vec![string_class]);
        let options: Id<NSDictionary<NSObject, NSObject>> = NSDictionary::new();
        let string_array: Id<NSArray<NSString>> = unsafe {
            let obj: *mut NSArray<NSString> =
                msg_send![self.pasteboard, readObjectsForClasses:&*classes options:&*options];
            if obj.is_null() {
                return Err(OSXError::ReturnedNull)?;
            }
            Id::from_ptr(obj)
        };
        if string_array.count() == 0 {
            Err(OSXError::ReturnedEmpty)?
        } else {
            Ok(string_array[0].as_str().to_owned())
        }
    }

    fn set_contents(&mut self, data: String) -> Result<(), Box<Error>> {
        let string_array = NSArray::from_vec(vec![NSString::from_str(&data)]);
        let _: usize = unsafe { msg_send![self.pasteboard, clearContents] };
        let success: bool = unsafe { msg_send![self.pasteboard, writeObjects: string_array] };
        return if success {
            Ok(())
        } else {
            Err(OSXError::ReturnedFalse)?
        };
    }

    fn iter(&mut self) -> ClipboardListener {
        let change_count: i64 = unsafe { msg_send![self.pasteboard, changeCount] };
        ClipboardListener {
            pasteboard: self.pasteboard.clone(),
            change_count,
        }
    }
}

// this is a convenience function that both cocoa-rs and
//  glutin define, which seems to depend on the fact that
//  Option::None has the same representation as a null pointer
#[inline]
pub fn class(name: &str) -> *mut Class {
    unsafe { transmute(Class::get(name)) }
}
