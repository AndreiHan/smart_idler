use chrono::{Local, NaiveTime, TimeDelta};
use idler_utils::idler_win_utils;
use std::{
    process,
    sync::{
        atomic::AtomicBool,
        mpsc::{self, Receiver, Sender, TryRecvError},
        Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, Manager, UserAttentionType, Window, WindowUrl};

pub(crate) struct ControllerChannel {
    pub(crate) tx: Mutex<Sender<String>>,
    pub(crate) active: AtomicBool,
}

fn focus_window(window: &Window) {
    let status = window.set_focus();
    trace!("Focus status: {status:?}");
    let status = window.request_user_attention(Some(UserAttentionType::Informational));
    trace!("User attention status: {status:?}");
}

pub(crate) fn build_controller(app: &AppHandle) {
    if let Some(win) = app.get_window("main") {
        info!("Found 'main' window setting focus");
        focus_window(&win);
        return;
    }
    info!("Could not find 'main' window, launching it");

    let current_app = app.clone();
    thread::spawn(move || {
        match tauri::WindowBuilder::new(&current_app, "main", WindowUrl::App("ui".into()))
            .fullscreen(false)
            .resizable(false)
            .title("Controller")
            .center()
            .inner_size(900.into(), 425.into())
            .build()
        {
            Ok(handle) => {
                focus_window(&handle);
            }
            Err(e) => error!("Failed to create controller app, err: {e}"),
        }
    });
}

pub(crate) fn close_app_remote(rx: Receiver<String>) {
    thread::spawn(move || {
        let mut _sender: Option<mpsc::Sender<()>> = None;
        loop {
            let hour = match rx.recv() {
                Ok(val) => val,
                Err(err) => {
                    info!("Received err: {err}");
                    return;
                }
            };
            debug!("Received time: {hour:?}");
            let Ok(received_time) = NaiveTime::parse_from_str(&hour, "%H:%M") else {
                info!("Received non time value, {hour}. Ignorring");
                _sender = None;
                continue;
            };
            let (sen, receiver) = mpsc::channel::<()>();
            _sender = Some(sen);

            thread::spawn(move || loop {
                let now = Local::now().time();
                let diff = if let Ok(dur) = received_time.signed_duration_since(now).to_std() {
                    dur
                } else {
                    let Some(current_diff) = now
                        .signed_duration_since(received_time)
                        .checked_add(&TimeDelta::days(1))
                    else {
                        error!("Failed to add 1 day to time");
                        break;
                    };
                    match current_diff.to_std() {
                        Ok(d) => d,
                        Err(err) => {
                            error!("Err converting {current_diff} to std, err: {err}");
                            break;
                        }
                    }
                };

                if diff.as_secs() == 0 {
                    info!("Shutdown");
                    idler_win_utils::ExecState::stop();
                    process::exit(0);
                }
                thread::sleep(Duration::from_millis(500));
                match receiver.try_recv() {
                    Ok(()) | Err(TryRecvError::Disconnected) => {
                        info!("Cancelling task for: {received_time}");
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            });
            thread::sleep(Duration::from_millis(500));
        }
    });
}
