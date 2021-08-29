use hv_core::{
    engine::{Engine, LuaExt, LuaResource},
    plugins::Plugin,
    prelude::*,
};
use rustyline::{Config, EditMode, Editor};
use std::{
    error::Error,
    fmt::Write,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

struct StartData {
    call_tx: Sender<String>,
    response_rx: Receiver<String>,
}

impl StartData {
    pub fn go(self) {
        std::thread::spawn(move || {
            let mut rl =
                Editor::<()>::with_config(Config::builder().edit_mode(EditMode::Vi).build());

            loop {
                let s = rl.readline(">>> ").unwrap();
                let trimmed = s.trim();

                rl.add_history_entry(trimmed);
                self.call_tx.send(trimmed.to_owned()).unwrap();

                println!("{}", self.response_rx.recv().unwrap());
            }
        });
    }
}

pub struct Console {
    start_data: Option<Mutex<StartData>>,
    call_rx: Mutex<Receiver<String>>,
    response_tx: Mutex<Sender<String>>,
}

impl Console {
    pub fn new(engine: &Engine) -> Shared<Self> {
        let (call_tx, call_rx) = std::sync::mpsc::channel();
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        engine.insert(Self {
            start_data: Some(Mutex::new(StartData {
                call_tx,
                response_rx,
            })),
            call_rx: Mutex::new(call_rx),
            response_tx: Mutex::new(response_tx),
        })
    }

    pub fn poll(&mut self, lua: &Lua) -> Result<()> {
        if let Some(start_data) = self.start_data.take() {
            start_data.into_inner().unwrap().go();
        }

        for s in self.call_rx.lock().unwrap().try_iter() {
            let mut buf = String::new();
            match lua.load(&s).eval::<LuaMultiValue>() {
                Ok(out) => {
                    for (i, v) in out.into_iter().enumerate() {
                        if let Ok(json) = serde_json::to_string_pretty(&v) {
                            writeln!(&mut buf, "[{}]prt: {}", i, json).unwrap();
                        } else {
                            writeln!(&mut buf, "[{}]dbg: {:?}", i, v).unwrap();
                        }
                    }
                }
                Err(e) => {
                    writeln!(&mut buf, "err: {}", e)?;

                    if let Some(source) = e.source() {
                        writeln!(&mut buf, "caused by: {}", source)?;
                    }
                }
            }

            self.response_tx.lock().unwrap().send(buf).unwrap();
        }

        Ok(())
    }
}

impl LuaUserData for Console {}

impl LuaResource for Console {
    const REGISTRY_KEY: &'static str = "HV_CONSOLE";
}

struct HvConsolePlugin;

impl Plugin for HvConsolePlugin {
    fn name(&self) -> &'static str {
        "console"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
        let console = Console::new(engine);
        lua.register(console.clone())?;
        let poll = lua.create_function(move |lua, ()| {
            console.borrow_mut().poll(lua).to_lua_err()?;
            Ok(())
        })?;

        Ok(lua
            .load(mlua::chunk! {
                {
                    poll = $poll,
                }
            })
            .eval()?)
    }
}

hv_core::plugin!(HvConsolePlugin);
