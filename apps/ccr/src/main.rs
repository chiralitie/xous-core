//! CCR - Claude Code Remote Benchmark App
//!
//! Runs benchmarks when launched from app menu

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

mod bench;

use core::fmt::Write;
use blitstr2::GlyphStyle;
use num_traits::*;
use ux_api::minigfx::*;
use ux_api::service::api::Gid;

pub(crate) const SERVER_NAME_CCR: &str = "_CCR Benchmark_";

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub(crate) enum CcrOp {
    Redraw = 0,
    Quit,
}

struct Ccr {
    content: Gid,
    gam: gam::Gam,
    _gam_token: [u32; 4],
    screensize: Point,
    results: Option<alloc::string::String>,
}

extern crate alloc;
use alloc::string::String;

impl Ccr {
    fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        let gam = gam::Gam::new(&xns).expect("Can't connect to GAM");
        let gam_token = gam
            .register_ux(gam::UxRegistration {
                app_name: String::from(gam::APP_NAME_CCR),
                ux_type: gam::UxType::Chat,
                predictor: None,
                listener: sid.to_array(),
                redraw_id: CcrOp::Redraw.to_u32().unwrap(),
                gotinput_id: None,
                audioframe_id: None,
                rawkeys_id: None,
                focuschange_id: None,
            })
            .expect("Could not register GAM UX")
            .unwrap();

        let content = gam.request_content_canvas(gam_token).expect("Could not get content canvas");
        let screensize = gam.get_canvas_bounds(content).expect("Could not get canvas dimensions");

        Self {
            gam,
            _gam_token: gam_token,
            content,
            screensize,
            results: None,
        }
    }

    fn clear_area(&self) {
        self.gam
            .draw_rectangle(
                self.content,
                Rectangle::new_with_style(
                    Point::new(0, 0),
                    self.screensize,
                    DrawStyle { fill_color: Some(PixelColor::Light), stroke_color: None, stroke_width: 0 },
                ),
            )
            .expect("can't clear content area");
    }

    fn run_benchmarks(&mut self) {
        log::info!("CCR: Running benchmarks...");
        let results = bench::run_all_benchmarks();
        self.results = Some(results);
        log::info!("CCR: Benchmarks complete");
    }

    fn redraw(&mut self) {
        self.clear_area();

        // Run benchmarks on first redraw if not done yet
        if self.results.is_none() {
            self.run_benchmarks();
        }

        // Center the text view on screen
        let mut text_view = TextView::new(
            self.content,
            TextBounds::GrowableFromBr(
                Point::new(
                    self.screensize.x - 20,  // Right edge with margin
                    self.screensize.y - 50,  // Bottom with margin
                ),
                (self.screensize.x - 40) as u16,  // Max width
            ),
        );

        text_view.border_width = 1;
        text_view.draw_border = true;
        text_view.clear_area = true;
        text_view.rounded_border = Some(3);
        text_view.style = GlyphStyle::Small;

        if let Some(ref results) = self.results {
            write!(text_view.text, "CCR Benchmarks\n\n{}", results)
                .expect("Could not write to text view");
        } else {
            write!(text_view.text, "CCR Benchmarks\n\nRunning...")
                .expect("Could not write to text view");
        }

        self.gam.post_textview(&mut text_view).expect("Could not render text view");
        self.gam.redraw().expect("Could not redraw screen");
    }
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("CCR PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME_CCR, None).expect("can't register server");

    let mut ccr = Ccr::new(&xns, sid);

    loop {
        let msg = xous::receive_message(sid).unwrap();
        log::debug!("CCR got message: {:?}", msg);

        match FromPrimitive::from_usize(msg.body.id()) {
            Some(CcrOp::Redraw) => {
                log::debug!("CCR redraw");
                ccr.redraw();
            }
            Some(CcrOp::Quit) => {
                log::info!("CCR quitting");
                break;
            }
            _ => {
                log::debug!("CCR unknown message");
            }
        }
    }

    xous::terminate_process(0)
}
