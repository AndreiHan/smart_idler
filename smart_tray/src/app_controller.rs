use chrono::{Local, NaiveTime};
use idler_utils::idler_win_utils;
use std::{
    process,
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{Manager, UserAttentionType};

pub(crate) struct ControllerChannel {
    pub(crate) tx: Mutex<Sender<String>>,
    pub(crate) active: Mutex<bool>,
}

pub(crate) fn build_controller(app: &tauri::AppHandle) {
    if let Some(win) = app.get_window("main") {
        info!("Found 'main' window setting focus");
        let _ = win.set_focus();
        let _ = win.request_user_attention(Some(UserAttentionType::Informational));
        return;
    } else {
        info!("Could not find 'main' window, launching it");
    };

    let current_app = app.clone();
    thread::spawn(move || {
        match tauri::WindowBuilder::new(&current_app, "main", tauri::WindowUrl::App("ui".into()))
            .fullscreen(false)
            .resizable(false)
            .title("Controller")
            .center()
            .inner_size(900.into(), 425.into())
            .build()
        {
            Ok(handle) => {
                let _ = handle.set_focus();
                let _ = handle.request_user_attention(Some(UserAttentionType::Informational));
            }
            Err(e) => error!("Failed to create controller app, err: {}", e),
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
                    info!("Received err: {}", err);
                    return;
                }
            };
            debug!("Received time: {:?}", hour);
            let Ok(received_time) = NaiveTime::parse_from_str(&hour, "%H:%M") else {
                info!("Received non time value, {}. Ignorring", hour);
                _sender = None;
                continue;
            };
            let (sen, receiver) = mpsc::channel::<()>();
            _sender = Some(sen);

            thread::spawn(move || loop {
                let now = Local::now().time();
                let diff = match received_time.signed_duration_since(now).to_std() {
                    Ok(d) => d,
                    Err(err) => {
                        error!("Received negative time, err: {}", err);
                        break;
                    }
                };

                if diff.as_secs() == 0 {
                    warn!("Shutdown");
                    idler_win_utils::ExecState::stop();
                    process::exit(0);
                }
                thread::sleep(Duration::from_millis(500));
                match receiver.try_recv() {
                    Ok(()) | Err(TryRecvError::Disconnected) => {
                        info!("Cancelling task for: {}", received_time);
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            });
            thread::sleep(Duration::from_millis(500));
        }
    });
}
