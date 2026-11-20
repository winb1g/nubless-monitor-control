//! Apple Silicon: send a Set VCP Feature with a non-standard sender address.
//!
//! LG monitors switch inputs on `0xF4` only when the request's sender address is
//! `0x50` instead of the standard `0x51` (LG's side channel; see
//! `docs/research/11-macos-and-input-switching.md`). The `ddc` crates build every
//! packet, checksum included, around `0x51`, so this module finds the display's
//! `IOAVService` itself, the way `ddc-macos` does, and writes the packet.

use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation_sys::base::{kCFAllocatorDefault, CFAllocatorRef, CFRelease, CFTypeRef};
use core_foundation_sys::dictionary::CFDictionaryRef;
use io_kit_sys::keys::kIOServicePlane;
use io_kit_sys::types::{io_iterator_t, io_object_t, io_registry_entry_t};
use io_kit_sys::{
    kIOMasterPortDefault, kIORegistryIterateRecursively, IOIteratorNext, IOObjectRelease,
    IORegistryEntryCreateCFProperty, IORegistryEntryCreateIterator, IORegistryEntryGetName,
    IORegistryEntryGetParentEntry, IORegistryEntryGetPath, IORegistryGetRootEntry,
};
use std::ffi::CStr;
use std::os::raw::{c_uint, c_void};
use std::time::Duration;

/// The monitor's DDC/CI address (7-bit). Behind the HDMI converter chip in some
/// Macs (MCDP29xx) it is `0xB7` instead, as in `ddc-macos`.
const DDC_CI: u32 = 0x37;
const DDC_CI_MCDP29XX: u32 = 0xB7;
/// Same as [`crate::twice`]: LG panels ignore the first write often enough that
/// every request goes out twice.
const BETWEEN_WRITES: Duration = Duration::from_millis(10);

#[link(name = "CoreDisplay", kind = "framework")]
extern "C" {
    fn CoreDisplay_DisplayCreateInfoDictionary(display_id: u32) -> CFDictionaryRef;
    fn IOAVServiceCreateWithService(allocator: CFAllocatorRef, service: io_object_t) -> CFTypeRef;
    fn IOAVServiceWriteI2C(
        service: CFTypeRef,
        chip_address: c_uint,
        data_address: c_uint,
        input_buffer: *const c_void,
        input_buffer_size: c_uint,
    ) -> i32;
}

/// Set VCP Feature `vcp` = `value` on the display `display_id`
/// (`CGDirectDisplayID`), sent with sender address `source`, twice.
pub fn set_vcp(display_id: u32, source: u8, vcp: u8, value: u16) -> Result<(), String> {
    let (service, chip) = av_service(display_id)?;
    let packet = packet(source, vcp, value);
    let write = || unsafe {
        IOAVServiceWriteI2C(
            service.0,
            chip,
            u32::from(source),
            packet.as_ptr().cast(),
            packet.len() as c_uint,
        )
    };
    let _ = write();
    std::thread::sleep(BETWEEN_WRITES);
    match write() {
        0 => Ok(()),
        err => Err(format!(
            "the Mac couldn't send to the monitor (I/O error 0x{:08X})",
            err as u32
        )),
    }
}

/// Set VCP Feature after the sender address (which the Mac sends as the I2C
/// data address): length, opcode `0x03`, code, value high and low, checksum.
/// The checksum is the XOR of the destination (`0x6E`), the sender and every
/// byte, as in standard DDC/CI.
fn packet(source: u8, vcp: u8, value: u16) -> [u8; 6] {
    let [hi, lo] = value.to_be_bytes();
    let mut packet = [0x84, 0x03, vcp, hi, lo, 0];
    packet[5] = packet[..5]
        .iter()
        .fold(((DDC_CI as u8) << 1) ^ source, |sum, b| sum ^ b);
    packet
}

/// A Core Foundation object released on drop.
struct Owned(CFTypeRef);

impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

/// An IOKit object released on drop.
struct Io(io_object_t);

impl Drop for Io {
    fn drop(&mut self) {
        unsafe {
            IOObjectRelease(self.0);
        }
    }
}

/// The display's DDC service and the chip address to write to, found the way
/// `ddc-macos` finds it: the first external `DCPAVServiceProxy` after the
/// display's own entry in the IORegistry.
fn av_service(display_id: u32) -> Result<(Owned, u32), String> {
    let not_found = || "couldn't find the monitor's DDC/CI service".to_string();
    let info = unsafe { CoreDisplay_DisplayCreateInfoDictionary(display_id) };
    if info.is_null() {
        return Err(not_found());
    }
    let info: CFDictionary<CFString, CFType> =
        unsafe { CFDictionary::wrap_under_create_rule(info) };
    let location = info
        .find(CFString::from_static_string("IODisplayLocation"))
        .and_then(|v| v.downcast::<CFString>())
        .ok_or_else(not_found)?
        .to_string();

    let mut iter: io_iterator_t = 0;
    let rc = unsafe {
        IORegistryEntryCreateIterator(
            IORegistryGetRootEntry(kIOMasterPortDefault),
            kIOServicePlane as *const _,
            kIORegistryIterateRecursively,
            &mut iter,
        )
    };
    if rc != 0 {
        return Err(not_found());
    }
    let iter = Io(iter);
    let next = || match unsafe { IOIteratorNext(iter.0) } {
        0 => None,
        entry => Some(Io(entry)),
    };
    while let Some(entry) = next() {
        if path(entry.0).as_deref() != Some(location.as_str()) {
            continue;
        }
        while let Some(entry) = next() {
            if name(entry.0).as_deref() != Some("DCPAVServiceProxy")
                || string_property(entry.0, "Location").as_deref() != Some("External")
            {
                continue;
            }
            let service = unsafe { IOAVServiceCreateWithService(kCFAllocatorDefault, entry.0) };
            if service.is_null() {
                continue;
            }
            return Ok((Owned(service), chip_address(entry.0)));
        }
    }
    Err(not_found())
}

fn chip_address(service: io_registry_entry_t) -> u32 {
    let mut parent: io_registry_entry_t = 0;
    if unsafe { IORegistryEntryGetParentEntry(service, kIOServicePlane as *const _, &mut parent) }
        != 0
    {
        return DDC_CI;
    }
    let parent = Io(parent);
    match string_property(parent.0, "EPICProviderClass").as_deref() {
        Some("AppleDCPMCDP29XX") => DDC_CI_MCDP29XX,
        _ => DDC_CI,
    }
}

fn path(entry: io_registry_entry_t) -> Option<String> {
    let mut buf = [0 as std::os::raw::c_char; 1024];
    (unsafe { IORegistryEntryGetPath(entry, kIOServicePlane as *const _, buf.as_mut_ptr()) } == 0)
        .then(|| {
            unsafe { CStr::from_ptr(buf.as_ptr()) }
                .to_string_lossy()
                .into_owned()
        })
}

fn name(entry: io_registry_entry_t) -> Option<String> {
    let mut buf = [0 as std::os::raw::c_char; 128];
    (unsafe { IORegistryEntryGetName(entry, buf.as_mut_ptr()) } == 0).then(|| {
        unsafe { CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    })
}

fn string_property(entry: io_registry_entry_t, key: &'static str) -> Option<String> {
    let key = CFString::from_static_string(key);
    let value = unsafe {
        IORegistryEntryCreateCFProperty(
            entry,
            key.as_concrete_TypeRef(),
            kCFAllocatorDefault,
            kIORegistryIterateRecursively,
        )
    };
    if value.is_null() {
        return None;
    }
    let value = unsafe { CFType::wrap_under_create_rule(value) };
    value.downcast::<CFString>().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::packet;

    /// The bytes the hand-written test tool sent when HDMI 1 and DisplayPort
    /// switched on a real 27GM950B (2026-11-19), worked out by hand.
    #[test]
    fn packets_match_the_ones_that_switched_inputs_on_hardware() {
        assert_eq!(
            packet(0x50, 0xF4, 0x90),
            [0x84, 0x03, 0xF4, 0x00, 0x90, 0xDD]
        );
        assert_eq!(
            packet(0x50, 0xF4, 0xD0),
            [0x84, 0x03, 0xF4, 0x00, 0xD0, 0x9D]
        );
    }

    /// With the standard sender it's an ordinary DDC/CI Set VCP Feature:
    /// brightness 50 is `51 84 03 10 00 32 9A` on the wire.
    #[test]
    fn with_the_standard_sender_it_is_a_normal_set_vcp() {
        assert_eq!(packet(0x51, 0x10, 50), [0x84, 0x03, 0x10, 0x00, 0x32, 0x9A]);
    }
}
