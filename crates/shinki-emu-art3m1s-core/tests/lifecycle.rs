use shinki_emu_abi::*;
use shinki_emu_art3m1s_core::artemis_query;
use std::{
    ptr,
    sync::Mutex,
    time::{Duration, Instant},
};
/// The adapter admits one native session per process.
static SESSIONS: Mutex<()> = Mutex::new(());
unsafe extern "C" fn emit(_: *mut std::ffi::c_void, record: *const LogRecord) {
    let record = unsafe { &*record };
    eprintln!(
        "{}",
        String::from_utf8_lossy(unsafe { record.message.as_slice(16384).unwrap() })
    );
}
unsafe extern "C" fn flush(_: *mut std::ffi::c_void) {}
unsafe extern "C" fn cancelled(_: *mut std::ffi::c_void, _: u64) -> u32 {
    0
}
unsafe extern "C" fn unavailable(
    _: *mut std::ffi::c_void,
    _: *const WindowRequest,
    _: *mut WindowResult,
    _: *mut Error,
) -> Status {
    UNSUPPORTED
}
unsafe extern "C" fn invalidate(_: *mut std::ffi::c_void, _: u64, _: u64) {}
fn api() -> &'static CoreApi {
    let mut raw = ptr::null();
    assert_eq!(unsafe { artemis_query(ABI_VERSION, &mut raw) }, OK);
    let api = unsafe { &*raw };
    api.validate().unwrap();
    let diagnostics = Diagnostics {
        header: Header::of::<Diagnostics>(),
        context: ptr::null_mut(),
        emit: Some(emit),
        flush: Some(flush),
    };
    assert_eq!(
        unsafe { api.initialize.unwrap()(&diagnostics, &mut Error::default()) },
        OK
    );
    api
}
fn open<'a>(
    api: &CoreApi,
    root: &std::path::Path,
    epoch: u64,
    error: &'a mut Error,
    window_host: WindowService,
    render_extent: Extent,
) -> Result<OpenResult, &'a Error> {
    let path = root.to_str().unwrap();
    let request = OpenRequest {
        header: Header::of::<OpenRequest>(),
        game_path: Bytes::new(path.as_bytes()),
        session_epoch: epoch,
        configuration: ptr::null(),
        configuration_count: 0,
        window: WindowState {
            drawable_width: 64,
            drawable_height: 48,
            scale_factor: 1.0,
            focused: 1,
            suspended: 0,
        },
        host: HostServices {
            header: Header::of::<HostServices>(),
            text: TextService {
                context: ptr::null_mut(),
                request: None,
                acknowledge: None,
                invalidate: None,
            },
            window: window_host,
            cancellation: CancellationService {
                context: ptr::null_mut(),
                is_cancelled: Some(cancelled),
            },
        },
        module_owner: ptr::null_mut(),
        render_extent,
    };
    let mut output = std::mem::MaybeUninit::<OpenResult>::uninit();
    let status = unsafe { api.open.unwrap()(&request, output.as_mut_ptr(), error) };
    let output = unsafe { output.assume_init() };
    if status == OK {
        Ok(output)
    } else {
        assert!(output.session.is_null());
        Err(error)
    }
}
#[test]
fn malformed_archive_releases_native_session_lease() {
    let api = api();
    let _session = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("data.pfs"), b"bad").unwrap();
    for epoch in 1..=2 {
        let mut error = Error::default();
        let failure = open(
            api,
            root.path(),
            epoch,
            &mut error,
            WindowService {
                context: ptr::null_mut(),
                request: Some(unavailable),
                invalidate: Some(invalidate),
            },
            Extent::default(),
        )
        .unwrap_err();
        assert_eq!(
            &failure.code[..failure.code_len as usize],
            b"SHINKI_EMU_ARTEMIS_PROJECT_INI"
        );
    }
}
#[test]
fn non_default_render_extent_is_rejected() {
    let api = api();
    let root = tempfile::tempdir().unwrap();
    let mut error = Error::default();
    let failure = open(
        api,
        root.path(),
        1,
        &mut error,
        WindowService {
            context: ptr::null_mut(),
            request: Some(unavailable),
            invalidate: Some(invalidate),
        },
        Extent {
            width: 1920,
            height: 1080,
        },
    )
    .unwrap_err();
    assert_eq!(
        &failure.code[..failure.code_len as usize],
        b"SHINKI_EMU_ARTEMIS_RENDER_EXTENT"
    );
}

/// Uncompressed PF6 archive with the index layout written by `pf8::Pf8Writer`.
fn pfs(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let names: usize = entries.iter().map(|(name, _)| name.len() + 16).sum();
    let index_size = 4 + names + 4 + (entries.len() + 1) * 8 + 4;
    let mut bytes = b"pf6".to_vec();
    bytes.extend_from_slice(&(index_size as u32).to_le_bytes());
    bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    let mut offset = 7 + index_size;
    let mut size_fields = Vec::new();
    for (name, data) in entries {
        bytes.extend_from_slice(&(name.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&(offset as u32).to_le_bytes());
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        size_fields.push((bytes.len() - 4 - 0xf) as u64);
        offset += data.len();
    }
    bytes.extend_from_slice(&(entries.len() as u32 + 1).to_le_bytes());
    let count_offset = (bytes.len() - 4 - 7) as u32;
    for field in size_fields {
        bytes.extend_from_slice(&field.to_le_bytes());
    }
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&count_offset.to_le_bytes());
    assert_eq!(bytes.len(), 7 + index_size);
    for (_, data) in entries {
        bytes.extend_from_slice(data);
    }
    bytes
}

/// Host window service answering native dialogs by caption.
struct Host {
    answer: fn(&str) -> u32,
    modals: Mutex<Vec<(String, String, usize)>>,
}
unsafe extern "C" fn host_window(
    context: *mut std::ffi::c_void,
    request: *const WindowRequest,
    result: *mut WindowResult,
    _: *mut Error,
) -> Status {
    let host = unsafe { &*context.cast::<Host>() };
    let (request, result) = unsafe { (&*request, &mut *result) };
    match request.kind {
        WINDOW_QUERY_STATE => {
            result.fullscreen = FULLSCREEN_WINDOWED;
            result.minimized = 0;
            result.valid_fields = WINDOW_RESULT_MODE | WINDOW_RESULT_MINIMIZED;
        }
        WINDOW_MODAL => {
            let caption = unsafe { request.caption.text(MAX_MODAL_TEXT) }
                .unwrap()
                .to_owned();
            let message = unsafe { request.text.text(MAX_MODAL_TEXT) }
                .unwrap()
                .to_owned();
            result.selected_button = (host.answer)(&caption);
            result.valid_fields = WINDOW_RESULT_MODAL;
            host.modals
                .lock()
                .unwrap()
                .push((caption, message, request.button_count));
        }
        _ => {}
    }
    OK
}

/// Opens `boot.iet` on the real native owner, waits for `until` and closes.
fn run_script(
    boot: &str,
    answer: fn(&str) -> u32,
    until: impl Fn(&SessionState, &Host) -> bool,
) -> (SessionState, Vec<(String, String, usize)>) {
    let api = api();
    let _session = SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("data.pfs"),
        pfs(&[
            (
                "system.ini",
                b"[WINDOWS]\nWIDTH=64\nHEIGHT=48\nBOOT=boot.iet\nSAVEPATH=save\nCHARSET=UTF-8\n",
            ),
            ("boot.iet", boot.as_bytes()),
        ]),
    )
    .unwrap();
    let host = Box::new(Host {
        answer,
        modals: Mutex::new(Vec::new()),
    });
    let mut error = Error::default();
    let opened = open(
        api,
        root.path(),
        1,
        &mut error,
        WindowService {
            context: (&*host as *const Host).cast_mut().cast(),
            request: Some(host_window),
            invalidate: Some(invalidate),
        },
        Extent::default(),
    )
    .unwrap_or_else(|error| {
        panic!(
            "open failed: {}",
            String::from_utf8_lossy(&error.message[..error.message_len as usize])
        )
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let state = loop {
        let mut state = std::mem::MaybeUninit::<SessionState>::uninit();
        assert_eq!(
            unsafe {
                api.state.unwrap()(opened.session, state.as_mut_ptr(), &mut Error::default())
            },
            OK
        );
        let state = unsafe { state.assume_init() };
        if until(&state, &host) || state.state == SESSION_FAILED || Instant::now() > deadline {
            break state;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut close_error = Error::default();
    let closed = unsafe { api.close.unwrap()(opened.session, &mut close_error) };
    assert_eq!(
        (closed, state.error.code_len),
        (OK, 0),
        "close: {}; state: {}",
        String::from_utf8_lossy(&close_error.message[..close_error.message_len as usize]),
        String::from_utf8_lossy(&state.error.message[..state.error.message_len as usize]),
    );
    (state, host.modals.into_inner().unwrap())
}
fn finished(state: &SessionState, _: &Host) -> bool {
    state.state == SESSION_FINISHED
}
fn assert_finished(state: &SessionState) {
    assert_eq!((state.state, state.flags), (SESSION_FINISHED, 0));
}
const CONFIRM_EXIT: &str = "[dialog title=\"Quit\" message=\"Leave the game?\" varname=\"answer\"]\n[if estimate=\"$answer == 1\"]\n[exit]\n[/if]\n[dialog title=\"Again\" message=\"Still running\"]\n[stop]\n";

#[test]
#[ignore = "requires a hardware native GPU adapter and audio output"]
fn exit_tag_finishes_the_session() {
    let (state, modals) = run_script("[exit]\n[stop]\n", |_| 0, finished);
    assert_finished(&state);
    assert!(modals.is_empty());
}

#[test]
#[ignore = "requires a hardware native GPU adapter and audio output"]
fn lua_exit_finishes_the_session() {
    let (state, _) = run_script(
        "[lua]\nfunction quit(e)\n    e:exit()\nend\n[/lua]\n[calllua function=\"quit\"]\n[stop]\n",
        |_| 0,
        finished,
    );
    assert_finished(&state);
}

#[test]
#[ignore = "requires a hardware native GPU adapter and audio output"]
fn accepted_exit_dialog_finishes_the_session() {
    let (state, modals) = run_script(CONFIRM_EXIT, |_| 1, finished);
    assert_finished(&state);
    assert_eq!(
        modals,
        [("Quit".into(), "Leave the game?".into(), 2)],
        "only the confirmation is shown before exit"
    );
}

#[test]
#[ignore = "requires a hardware native GPU adapter and audio output"]
fn cancelled_exit_dialog_keeps_the_game_running() {
    let (state, modals) = run_script(
        CONFIRM_EXIT,
        |caption| if caption == "Quit" { 2 } else { 1 },
        |state, host| state.state == SESSION_FINISHED || host.modals.lock().unwrap().len() == 2,
    );
    assert_eq!(state.state, SESSION_RUNNING);
    assert_eq!(
        modals,
        [
            ("Quit".into(), "Leave the game?".into(), 2),
            ("Again".into(), "Still running".into(), 1),
        ]
    );
}
