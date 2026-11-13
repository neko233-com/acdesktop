//! Multi-touch injection through `InjectTouchInput`, so a game session can
//! send a whole finger pose (tap, pinch, rotate, three-finger swipe) as one
//! frame instead of a stream of single-point events.
//!
//! The `POINTER_TOUCH_INFO` mapping lives here so the game dispatcher and the
//! standalone `touch` path share one flag table. `rcContactRaw` stays zero:
//! the OS answers a frame that fills it alongside `rcContact` with
//! `ERROR_INVALID_PARAMETER`. Native calls are
//! `#[cfg(not(test))]`; test builds record the contacts into a fake sink, so
//! no test ever injects a real touch.

use acdesktop_core::{AdapterError, Deadline, ErrorCode, TouchContact, TouchPhase};

pub(crate) const MAX_TOUCH_CONTACTS: u32 = 10;
#[cfg(not(test))]
const TOUCH_RADIUS: i32 = 4;
#[cfg(not(test))]
const TOUCH_PRESSURE: u32 = 32_000;

pub(crate) fn send_touch_frame(
    contacts: &[TouchContact],
    deadline: Deadline,
) -> Result<(), AdapterError> {
    deadline.ensure_remaining()?;
    if contacts.is_empty() {
        return Ok(());
    }
    if contacts.len() as u32 > MAX_TOUCH_CONTACTS {
        return Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            format!("At most {MAX_TOUCH_CONTACTS} touch contacts can be injected in one frame"),
        ));
    }
    imp::ensure_initialized()?;
    imp::inject(contacts)
}

#[cfg(target_os = "windows")]
mod imp {
    use super::{AdapterError, TouchContact};
    #[cfg(not(test))]
    use super::{ErrorCode, TOUCH_PRESSURE, TOUCH_RADIUS, TouchPhase};
    #[cfg(not(test))]
    use std::sync::Mutex;
    #[cfg(not(test))]
    use windows_sys::Win32::Foundation::RECT;
    #[cfg(not(test))]
    use windows_sys::Win32::UI::Input::Pointer::{
        InitializeTouchInjection, InjectTouchInput, POINTER_FLAG_DOWN, POINTER_FLAG_INCONTACT,
        POINTER_FLAG_INRANGE, POINTER_FLAG_UP, POINTER_FLAG_UPDATE, POINTER_TOUCH_INFO,
        TOUCH_FEEDBACK_DEFAULT,
    };
    #[cfg(not(test))]
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        PT_TOUCH, TOUCH_MASK_CONTACTAREA, TOUCH_MASK_PRESSURE,
    };

    #[cfg(not(test))]
    static CAPACITY: Mutex<u32> = Mutex::new(0);

    #[cfg(not(test))]
    pub(super) fn ensure_initialized() -> Result<(), AdapterError> {
        let mut capacity = CAPACITY.lock().map_err(|_| {
            AdapterError::new(ErrorCode::Internal, "touch injection state is poisoned")
        })?;
        if *capacity >= super::MAX_TOUCH_CONTACTS {
            return Ok(());
        }
        let ok =
            unsafe { InitializeTouchInjection(super::MAX_TOUCH_CONTACTS, TOUCH_FEEDBACK_DEFAULT) };
        if ok == 0 {
            return Err(last_error("InitializeTouchInjection"));
        }
        *capacity = super::MAX_TOUCH_CONTACTS;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn ensure_initialized() -> Result<(), AdapterError> {
        Ok(())
    }

    pub(super) fn inject(contacts: &[TouchContact]) -> Result<(), AdapterError> {
        #[cfg(test)]
        {
            super::touch_send_fake_sink::record(contacts);
            Ok(())
        }
        #[cfg(not(test))]
        {
            let infos: Vec<POINTER_TOUCH_INFO> = contacts.iter().map(to_pointer_info).collect();
            let ok = unsafe { InjectTouchInput(infos.len() as u32, infos.as_ptr()) };
            if ok == 0 {
                return Err(last_error("InjectTouchInput"));
            }
            Ok(())
        }
    }

    #[cfg(not(test))]
    fn to_pointer_info(contact: &TouchContact) -> POINTER_TOUCH_INFO {
        let flags = match contact.phase {
            TouchPhase::Down => POINTER_FLAG_DOWN | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
            TouchPhase::Move => POINTER_FLAG_UPDATE | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
            TouchPhase::Up => POINTER_FLAG_UP,
        };
        let mut info: POINTER_TOUCH_INFO = unsafe { std::mem::zeroed() };
        info.pointerInfo.pointerType = PT_TOUCH;
        info.pointerInfo.pointerId = contact.id;
        info.pointerInfo.ptPixelLocation.x = contact.x;
        info.pointerInfo.ptPixelLocation.y = contact.y;
        info.pointerInfo.pointerFlags = flags;
        info.touchMask = TOUCH_MASK_CONTACTAREA | TOUCH_MASK_PRESSURE;
        info.rcContact = rect_around(contact.x, contact.y);
        info.pressure = TOUCH_PRESSURE;
        info
    }

    #[cfg(not(test))]
    fn rect_around(x: i32, y: i32) -> RECT {
        RECT {
            left: x - TOUCH_RADIUS,
            top: y - TOUCH_RADIUS,
            right: x + TOUCH_RADIUS,
            bottom: y + TOUCH_RADIUS,
        }
    }

    #[cfg(not(test))]
    fn last_error(context: &str) -> AdapterError {
        AdapterError::new(
            ErrorCode::ActionFailed,
            format!("{context} failed: {}", std::io::Error::last_os_error()),
        )
        .with_suggestion(
            "Run the session on an interactive desktop that accepts touch injection, then retry",
        )
        .with_disposition(acdesktop_core::DeliverySemantics::not_delivered())
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::{AdapterError, TouchContact};

    pub(super) fn ensure_initialized() -> Result<(), AdapterError> {
        Ok(())
    }

    pub(super) fn inject(_contacts: &[TouchContact]) -> Result<(), AdapterError> {
        Err(AdapterError::not_supported("touch_frame"))
    }
}

crate::input::define_input_fake_sink!(touch_send_fake_sink, acdesktop_core::TouchContact);

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{TouchContact, TouchPhase, send_touch_frame};
    use acdesktop_core::Deadline;
    use std::time::Duration;

    #[test]
    fn a_touch_frame_reaches_the_recording_sink() {
        let deadline = Deadline::from_duration(Duration::from_secs(5)).expect("a test deadline");
        super::touch_send_fake_sink::reset();
        let contacts = [
            TouchContact {
                id: 0,
                x: 10,
                y: 20,
                phase: TouchPhase::Down,
            },
            TouchContact {
                id: 1,
                x: 30,
                y: 40,
                phase: TouchPhase::Move,
            },
        ];
        send_touch_frame(&contacts, deadline).expect("the recording sink accepts a touch frame");
        let recorded = super::touch_send_fake_sink::recorded();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[1].id, 1);
        assert_eq!(recorded[0].phase, TouchPhase::Down);
    }

    #[test]
    fn too_many_contacts_are_rejected() {
        let deadline = Deadline::from_duration(Duration::from_secs(5)).expect("a test deadline");
        let contacts: Vec<TouchContact> = (0..11)
            .map(|id| TouchContact {
                id,
                x: 0,
                y: 0,
                phase: TouchPhase::Down,
            })
            .collect();
        assert!(send_touch_frame(&contacts, deadline).is_err());
    }
}
