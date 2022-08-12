#![windows_subsystem = "windows"]

use std::collections::hash_map::Entry;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tide::{Response, StatusCode};
use windows::Win32::System::Power::{ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState};

fn set_sleep_state(allow: bool) -> bool {
    let mut state = ES_CONTINUOUS;
    if !allow {
        state |= ES_SYSTEM_REQUIRED;
    }
    let res = unsafe {
        SetThreadExecutionState(state)
    };
    return res.0 != 0;
}

#[derive(Clone)]
struct State {
    wakers: HashMap<String, String>,
    password: Option<String>,
}

impl State {
    fn check_password(&self, p: Option<String>) -> tide::Result<()> {
        if self.password.is_none() || self.password.as_ref().unwrap().is_empty() {
            return Ok(())
        }
        if p.is_none() {
            return Err(tide::Error::from_str(StatusCode::Unauthorized, "Missing password"));
        }
        if &p.unwrap() == self.password.as_ref().unwrap() {
            return Ok(())
        }
        Err(tide::Error::from_str(StatusCode::Unauthorized, "Invalid password"))
    }

    fn list_wakers(&self) -> String {
        let mut wakers = self.wakers.values().map(|s| &**s).collect::<Vec<_>>();
        wakers.sort_by(|a, b| a.cmp(b));
        wakers.join(", ")
    }

    fn keep_awake(&mut self, name: String) -> String {
        let is_first = self.wakers.is_empty();
        loop {
            let id: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(30)
                .map(char::from)
                .collect();
            let entry = self.wakers.entry(id.clone());
            if let Entry::Occupied(_) = &entry {
                continue;
            }
            if is_first {
                log::info!("Disabling sleep");
                set_sleep_state(false);
            }
            entry.or_insert(name);
            return id;
        }
    }

    fn task_done(&mut self, id: String) -> Option<String> {
        let v = self.wakers.remove(&id);
        if v.is_none() {
            return v;
        }
        if self.wakers.is_empty() {
            log::info!("No more wakers left, turning sleep back on");
            set_sleep_state(true);
        }
        return v;
    }
}

#[derive(serde::Deserialize)]
struct Config {
    ip: std::net::IpAddr,
    port: Option<u16>,
    password: Option<String>,
}

impl Config {
    fn load() -> Self {
        toml::from_str(&Self::read()).expect("Unable to parse config")
    }

    fn read() -> String {
        if let Ok(s) = std::fs::read_to_string("config.toml") {
            return s;
        }
        if let Ok(mut path) = std::env::current_exe() {
            path.pop();
            path.push("config.toml");
            if let Ok(s) = std::fs::read_to_string(path) {
                return s;
            }
        }
        r#"
            ip = '127.0.0.1'
        "#.to_owned()
    }
}


#[async_std::main]
async fn main() -> tide::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let cfg = Config::load();

    let mut app = tide::with_state(Arc::new(Mutex::new(State {
        wakers:   HashMap::new(),
        password: cfg.password,
    })));

    app.at("/wakers").get(|req: tide::Request<Arc<Mutex<State>>>| async move {
        #[derive(serde::Deserialize)]
        struct WakersParams {
            password: Option<String>,
        }
        let params: WakersParams = req.query()?;

        let state = req.state().lock().unwrap();
        state.check_password(params.password)?;

        Ok(state.list_wakers())
    });

    app.at("/keep-awake").post(|req: tide::Request<Arc<Mutex<State>>>| async move {
        #[derive(serde::Deserialize)]
        struct KeepAwakeParams {
            name: String,
            password: Option<String>,
        }
        let params: KeepAwakeParams = req.query()?;

        let mut state = req.state().lock().unwrap();
        state.check_password(params.password)?;

        log::info!("Registering sleep disable request for {}", params.name);
        let id = state.keep_awake(params.name);
        Ok(id)
    });

    app.at("/task-done").post(|req: tide::Request<Arc<Mutex<State>>>| async move {
        #[derive(serde::Deserialize)]
        struct TaskDoneParams {
            id: String,
            password: Option<String>,
        }
        let params: TaskDoneParams = req.query()?;

        let mut state = req.state().lock().unwrap();
        state.check_password(params.password)?;

        log::info!("Ending sleep request {}", params.id);
        let task_name = state.task_done(params.id);
        Ok(if task_name.is_some() {
            log::info!("Ended sleep request from {}", task_name.unwrap());
            Response::new(StatusCode::NoContent)
        } else {
            log::info!("Unrecognized wake ID");
            let mut res = Response::new(StatusCode::Unauthorized);
            res.set_body("Unrecognized wake ID");
            res
        })
    });

    app.listen((format!("{}", cfg.ip), cfg.port.unwrap_or(5678))).await?;
    Ok(())
}
