use crate::{
    error::{self, NativeResult},
    media::{Dialog, WindowCommand},
    session::Platform,
};
use shinki_emu_abi::*;
use std::ops::ControlFlow;
/// User answer to a native `[dialog]`, submitted back to the core.
#[derive(Debug, PartialEq)]
pub(crate) struct DialogAnswer {
    pub accepted: bool,
    pub text: Option<String>,
}
const DIALOG_OK: u32 = 1;
const DIALOG_CANCEL: u32 = 2;
/// Services one window command; only dialogs produce an answer.
pub(crate) fn request(
    platform: Platform,
    command: WindowCommand,
    epoch: u64,
    context: u64,
    id: u64,
) -> NativeResult<ControlFlow<(), Option<DialogAnswer>>> {
    let mut request = window_request(epoch, context, id, 0);
    let operation = match &command {
        WindowCommand::Dialog(dialog) => {
            return Ok(modal(platform, dialog, epoch, context, id)?.map_continue(Some));
        }
        WindowCommand::Title(title) => {
            request.kind = WINDOW_TITLE;
            request.text = Bytes::new(title.as_bytes());
            "window.title"
        }
        WindowCommand::Pointer {
            x,
            y,
            visible,
            autohide_ms,
        } => {
            request.kind = WINDOW_POINTER;
            if let Some(x) = x {
                request.flags |= POINTER_SET_X;
                request.x = *x;
            }
            if let Some(y) = y {
                request.flags |= POINTER_SET_Y;
                request.y = *y;
            }
            if let Some(visible) = visible {
                request.flags |= POINTER_SET_VISIBILITY;
                if *visible {
                    request.flags |= POINTER_VISIBLE;
                }
            }
            if let Some(ms) = autohide_ms {
                request.flags |= POINTER_SET_AUTOHIDE;
                request.autohide_ms = *ms;
            }
            "window.pointer"
        }
    };
    invoke(platform, &request, operation, &mut []).map(|result| result.map_continue(|_| None))
}

/// Presents `[dialog]` through the Host modal. A Host-side dismissal while the
/// session is live is a Cancel answer; session cancellation stops the owner.
fn modal(
    platform: Platform,
    dialog: &Dialog,
    epoch: u64,
    context: u64,
    id: u64,
) -> NativeResult<ControlFlow<(), DialogAnswer>> {
    let buttons = [
        WindowButton {
            id: DIALOG_OK,
            reserved: 0,
            label: Bytes::new(b"OK"),
        },
        WindowButton {
            id: DIALOG_CANCEL,
            reserved: 0,
            label: Bytes::new(b"Cancel"),
        },
    ];
    let mut request = window_request(epoch, context, id, WINDOW_MODAL);
    request.caption = Bytes::new(dialog.title.as_bytes());
    request.text = Bytes::new(dialog.message.as_bytes());
    request.buttons = buttons.as_ptr();
    request.button_count = if dialog.cancel { 2 } else { 1 };
    let mut input = Vec::new();
    if let Some(initial) = &dialog.input {
        request.flags = MODAL_TEXT_INPUT;
        request.initial_text = Bytes::new(initial.as_bytes());
        input.resize(MAX_MODAL_TEXT, 0);
    }
    let result = match invoke(platform, &request, "window.modal", &mut input)? {
        ControlFlow::Continue(result) => result,
        ControlFlow::Break(()) if platform.cancelled(epoch) => return Ok(ControlFlow::Break(())),
        ControlFlow::Break(()) => {
            return Ok(ControlFlow::Continue(DialogAnswer {
                accepted: false,
                text: None,
            }))
        }
    };
    if result.valid_fields & WINDOW_RESULT_MODAL == 0
        || !buttons[..request.button_count]
            .iter()
            .any(|button| button.id == result.selected_button)
    {
        return Err(error::invalid(
            "SHINKI_EMU_ARTEMIS_PLATFORM",
            "window.modal returned no valid dialog selection",
        ));
    }
    let accepted = result.selected_button == DIALOG_OK;
    let text = if accepted && dialog.input.is_some() {
        if result.input.len > input.len() {
            return Err(error::invalid(
                "SHINKI_EMU_ARTEMIS_PLATFORM",
                "window.modal returned text beyond its buffer",
            ));
        }
        input.truncate(result.input.len);
        Some(String::from_utf8(input).map_err(|_| {
            error::invalid(
                "SHINKI_EMU_ARTEMIS_PLATFORM",
                "window.modal returned invalid UTF-8 text",
            )
        })?)
    } else {
        None
    };
    Ok(ControlFlow::Continue(DialogAnswer { accepted, text }))
}

pub(crate) fn query_state(
    platform: Platform,
    epoch: u64,
    context: u64,
    id: u64,
) -> NativeResult<ControlFlow<(), i32>> {
    let request = window_request(epoch, context, id, WINDOW_QUERY_STATE);
    let result = match invoke(platform, &request, "window.query_state", &mut [])? {
        ControlFlow::Continue(result) => result,
        ControlFlow::Break(()) => return Ok(ControlFlow::Break(())),
    };
    state_flags(result.fullscreen, result.minimized, result.valid_fields).map(ControlFlow::Continue)
}

fn state_flags(mode: u32, minimized: u32, valid_fields: u32) -> NativeResult<i32> {
    let required = WINDOW_RESULT_MODE | WINDOW_RESULT_MINIMIZED;
    if valid_fields & required != required
        || !matches!(
            mode,
            FULLSCREEN_WINDOWED | FULLSCREEN_BORDERLESS | FULLSCREEN_EXCLUSIVE
        )
        || minimized > 1
    {
        return Err(error::invalid(
            "SHINKI_EMU_ARTEMIS_WINDOW_STATE",
            "window.query_state did not return valid mode and minimized fields",
        ));
    }
    Ok(i32::from(mode != FULLSCREEN_WINDOWED) | ((minimized as i32) << 1))
}

fn window_request(epoch: u64, context: u64, id: u64, kind: u32) -> WindowRequest {
    WindowRequest {
        session_epoch: epoch,
        context_epoch: context,
        request_id: id,
        kind,
        flags: 0,
        text: Bytes::EMPTY,
        pixels: Bytes::EMPTY,
        width: 0,
        height: 0,
        x: 0,
        y: 0,
        autohide_ms: 0,
        buttons: std::ptr::null(),
        button_count: 0,
        caption: Bytes::EMPTY,
        initial_text: Bytes::EMPTY,
    }
}

fn invoke(
    platform: Platform,
    request: &WindowRequest,
    operation: &str,
    input: &mut [u8],
) -> NativeResult<ControlFlow<(), WindowResult>> {
    let callback = platform.0.request.ok_or_else(|| {
        error::invalid(
            "SHINKI_EMU_ARTEMIS_PLATFORM_UNAVAILABLE",
            format!("native window service is unavailable for {operation}"),
        )
    })?;
    let mut result = WindowResult {
        selected_button: 0,
        reserved: 0,
        valid_fields: 0,
        fullscreen: 0,
        width: 0,
        height: 0,
        x: 0,
        y: 0,
        input: TextBuffer {
            data: if input.is_empty() {
                std::ptr::null_mut()
            } else {
                input.as_mut_ptr()
            },
            capacity: input.len(),
            len: 0,
        },
        visible: 0,
        minimized: 0,
    };
    let mut error = Error::default();
    let status = unsafe { callback(platform.0.context, request, &mut result, &mut error) };
    if status == CANCELLED {
        return Ok(ControlFlow::Break(()));
    }
    if status != OK {
        return Err(crate::error::invalid(
            "SHINKI_EMU_ARTEMIS_PLATFORM",
            format!(
                "native platform request {operation} failed (status {status}): {}",
                String::from_utf8_lossy(
                    &error.message[..(error.message_len as usize).min(error.message.len())]
                )
            ),
        ));
    }
    Ok(ControlFlow::Continue(result))
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn native_window_flags_use_only_actual_mode_and_minimized() {
        let valid = WINDOW_RESULT_MODE | WINDOW_RESULT_MINIMIZED;
        for mode in [
            FULLSCREEN_WINDOWED,
            FULLSCREEN_BORDERLESS,
            FULLSCREEN_EXCLUSIVE,
        ] {
            for minimized in 0..=1 {
                let expected = i32::from(mode != FULLSCREEN_WINDOWED) | (minimized << 1) as i32;
                assert_eq!(state_flags(mode, minimized, valid).unwrap(), expected);
            }
        }
    }

    #[test]
    fn missing_or_invalid_window_state_is_not_invented() {
        let valid = WINDOW_RESULT_MODE | WINDOW_RESULT_MINIMIZED;
        for (mode, minimized, fields) in [
            (FULLSCREEN_WINDOWED, 0, WINDOW_RESULT_MODE),
            (FULLSCREEN_WINDOWED, 0, WINDOW_RESULT_MINIMIZED),
            (FULLSCREEN_WINDOWED, 0, 0),
            (3, 0, valid),
            (FULLSCREEN_WINDOWED, 2, valid),
        ] {
            let error = state_flags(mode, minimized, fields).unwrap_err();
            assert_eq!(error.code, "SHINKI_EMU_ARTEMIS_WINDOW_STATE");
            assert!(error.message.contains("window.query_state"));
        }
    }

    #[test]
    fn window_query_without_host_fails_explicitly() {
        let platform = Platform(
            WindowService {
                context: std::ptr::null_mut(),
                request: None,
                invalidate: None,
            },
            CancellationService {
                context: std::ptr::null_mut(),
                is_cancelled: None,
            },
        );
        let error = query_state(platform, 1, 1, 1).unwrap_err();
        assert_eq!(error.code, "SHINKI_EMU_ARTEMIS_PLATFORM_UNAVAILABLE");
        assert!(error.message.contains("window.query_state"));
    }

    #[test]
    fn cancelled_window_requests_remain_control_flow() {
        unsafe extern "C" fn cancelled(
            _: *mut std::ffi::c_void,
            _: *const WindowRequest,
            _: *mut WindowResult,
            _: *mut Error,
        ) -> Status {
            CANCELLED
        }
        let platform = Platform(
            WindowService {
                context: std::ptr::null_mut(),
                request: Some(cancelled),
                invalidate: None,
            },
            CancellationService {
                context: std::ptr::null_mut(),
                is_cancelled: None,
            },
        );
        assert!(query_state(platform, 1, 1, 1).unwrap().is_break());
        assert!(
            request(platform, WindowCommand::Title("title".into()), 1, 1, 2)
                .unwrap()
                .is_break()
        );
    }

    #[test]
    fn native_dialog_answers_come_from_the_host_modal() {
        struct Host {
            status: Status,
            button: u32,
            text: &'static [u8],
            cancelled: u32,
            seen: Option<(u32, Vec<u32>, String, String, String, usize)>,
        }
        unsafe extern "C" fn modal(
            context: *mut std::ffi::c_void,
            request: *const WindowRequest,
            result: *mut WindowResult,
            _: *mut Error,
        ) -> Status {
            let host = unsafe { &mut *context.cast::<Host>() };
            let (request, result) = unsafe { (&*request, &mut *result) };
            assert_eq!(request.kind, WINDOW_MODAL);
            let buttons =
                unsafe { std::slice::from_raw_parts(request.buttons, request.button_count) };
            let text = |bytes: Bytes| unsafe { bytes.text(MAX_MODAL_TEXT) }.unwrap().to_owned();
            host.seen = Some((
                request.flags,
                buttons.iter().map(|button| button.id).collect(),
                text(request.caption),
                text(request.text),
                text(request.initial_text),
                result.input.capacity,
            ));
            if host.status == OK {
                if !host.text.is_empty() {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            host.text.as_ptr(),
                            result.input.data,
                            host.text.len(),
                        )
                    };
                }
                result.input.len = host.text.len();
                result.selected_button = host.button;
                result.valid_fields = WINDOW_RESULT_MODAL;
            }
            host.status
        }
        unsafe extern "C" fn cancelled(context: *mut std::ffi::c_void, _: u64) -> u32 {
            unsafe { (*context.cast::<Host>()).cancelled }
        }
        let ask = |host: &mut Host, cancel: bool, input: Option<&str>| {
            let context = (host as *mut Host).cast();
            let platform = Platform(
                WindowService {
                    context,
                    request: Some(modal),
                    invalidate: None,
                },
                CancellationService {
                    context,
                    is_cancelled: Some(cancelled),
                },
            );
            let dialog = Dialog {
                title: "Quit".into(),
                message: "Leave?".into(),
                cancel,
                input: input.map(String::from),
            };
            request(platform, WindowCommand::Dialog(dialog), 1, 1, 1)
        };
        let host = |status, button, text| Host {
            status,
            button,
            text,
            cancelled: 0,
            seen: None,
        };
        let mut accept = host(OK, DIALOG_OK, b"");
        assert_eq!(
            ask(&mut accept, true, None).unwrap(),
            ControlFlow::Continue(Some(DialogAnswer {
                accepted: true,
                text: None,
            }))
        );
        assert_eq!(
            accept.seen.unwrap(),
            (
                0,
                vec![1, 2],
                "Quit".into(),
                "Leave?".into(),
                String::new(),
                0
            )
        );
        let mut reject = host(OK, DIALOG_CANCEL, b"");
        assert_eq!(
            ask(&mut reject, true, None).unwrap(),
            ControlFlow::Continue(Some(DialogAnswer {
                accepted: false,
                text: None,
            }))
        );
        let mut notice = host(OK, DIALOG_OK, b"");
        assert!(ask(&mut notice, false, None).unwrap().is_continue());
        assert_eq!(notice.seen.unwrap().1, vec![1]);
        let mut name = host(OK, DIALOG_OK, "名前".as_bytes());
        assert_eq!(
            ask(&mut name, true, Some("Alice")).unwrap(),
            ControlFlow::Continue(Some(DialogAnswer {
                accepted: true,
                text: Some("名前".into()),
            }))
        );
        let seen = name.seen.unwrap();
        assert_eq!(
            (seen.0, seen.4, seen.5),
            (MODAL_TEXT_INPUT, "Alice".into(), MAX_MODAL_TEXT)
        );
        let mut dismissed = host(CANCELLED, 0, b"");
        assert_eq!(
            ask(&mut dismissed, true, None).unwrap(),
            ControlFlow::Continue(Some(DialogAnswer {
                accepted: false,
                text: None,
            }))
        );
        let mut closing = host(CANCELLED, 0, b"");
        closing.cancelled = 1;
        assert!(ask(&mut closing, true, None).unwrap().is_break());
        let mut invented = host(OK, 7, b"");
        assert_eq!(
            ask(&mut invented, true, None).unwrap_err().code,
            "SHINKI_EMU_ARTEMIS_PLATFORM"
        );
    }
}
