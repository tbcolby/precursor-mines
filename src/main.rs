#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

//! Minesweeper for Precursor. Board rules, placement, the game clock, cell
//! geometry and best times live in `mines-core` (host-tested). This file routes
//! messages; `app.rs` maps keys, saves and draws.

mod app;

use num_traits::{FromPrimitive, ToPrimitive};
use precursor_appkit::{app as kit, keys, pump, rng, store};

/// xous-names server name. Any unique string; NOT the GAM app name.
const SERVER_NAME: &str = "_Mines_";

/// The on-screen clock only needs whole seconds.
const PUMP_MS: usize = 1000;

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub(crate) enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
    /// Blocking scalar from the pump; answered with `pump::ack`.
    Tick,
    BasisChanged,
    Quit,
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("mines PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME, None).expect("can't register server");

    // GAM app name comes from apps/manifest.json key "mines".
    let reg = kit::register(
        &xns,
        sid,
        gam::APP_NAME_MINES,
        gam::UxType::Framebuffer,
        kit::Opcodes {
            redraw: AppOp::Redraw.to_u32().unwrap(),
            rawkeys: Some(AppOp::Rawkeys.to_u32().unwrap()),
            focus_change: Some(AppOp::FocusChange.to_u32().unwrap()),
            got_input: None,
        },
    );

    let trng = rng::Trng::new(&xns).expect("can't connect to TRNG");
    let store = store::Store::open(app::DICT);
    store::watch_bases(xous::connect(sid).unwrap(), AppOp::BasisChanged.to_usize().unwrap());
    let pump = pump::Pump::spawn(xous::connect(sid).unwrap(), AppOp::Tick.to_usize().unwrap(), PUMP_MS);

    let mut app = app::App::new(reg.screen, store, trng);

    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => app.redraw(),
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                for key in keys::decode_rawkeys([k1, k2, k3, k4]) {
                    if app.on_key(key) == app::Action::Leave {
                        app.save();
                        kit::leave(app.screen());
                    }
                }
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, code, _, _, _, {
                match kit::focus_state(code) {
                    gam::FocusState::Foreground => {
                        app.on_foreground();
                        pump.run();
                    }
                    gam::FocusState::Background => {
                        pump.stop();
                        app.on_background();
                    }
                }
            }),
            Some(AppOp::Tick) => {
                app.on_tick();
                pump::ack(&msg);
            }
            Some(AppOp::BasisChanged) => app.reload(),
            Some(AppOp::Quit) => break,
            None => log::warn!("unknown opcode {:?}", msg.body.id()),
        }
    }
    app.on_background();
    pump.quit();
    kit::shutdown(&xns, sid)
}
