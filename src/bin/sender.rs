use eframe::{
    egui::{self, Color32, Context, Frame},
    emath::Align2,
    epaint::{pos2, FontId, Pos2, Rect},
};
use rand::prelude::*;
use std::{
    error::Error,
    net::UdpSocket,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use patchjuggler::{Object, NUM_OBJS, SCALE};

const MOTION: f64 = 0.1;

struct Shared {
    objs: Mutex<Vec<Object>>,
    total_amt: AtomicUsize,
    exit_signal: AtomicBool,
}

fn main() -> Result<(), String> {
    let mut rng = rand::thread_rng();
    let shared = Arc::new(Shared {
        objs: Mutex::new(
            (0..NUM_OBJS)
                .map(|_| Object {
                    pos: [rng.gen::<f64>() * 10., rng.gen::<f64>() * 10.],
                    color: [rng.gen::<u8>(), rng.gen(), rng.gen()],
                })
                .collect(),
        ),
        total_amt: AtomicUsize::new(0),
        exit_signal: AtomicBool::new(false),
    });

    let shared_copy = shared.clone();
    let thread = std::thread::spawn(move || sender_thread(shared_copy).map_err(|e| format!("{e}")));
    // let thread = std::thread::spawn(move || gui_thread(objs_copy).map_err(|e| format!("{e}")));

    // sender_thread(objs).map_err(|e| format!("{e}"))?;

    println!("thread departed!");

    gui_thread(shared.clone()).map_err(|e| format!("{e}"))?;

    shared.exit_signal.store(true, Ordering::Relaxed);

    thread.join().unwrap()
}

fn gui_thread(shared: Arc<Shared>) -> Result<(), Box<dyn Error>> {
    let mut native_options = eframe::NativeOptions::default();

    // We insist to use light theme, because the canvas color is designed to work with light background.
    native_options.follow_system_theme = false;
    native_options.default_theme = eframe::Theme::Light;

    Ok(eframe::run_native(
        "sender GUI",
        native_options,
        Box::new(|_cc| Box::new(SenderApp { shared })),
    )?)
}

fn sender_thread(shared: Arc<Shared>) -> Result<(), Box<dyn Error>> {
    std::thread::sleep(std::time::Duration::from_millis(1000));
    let socket = UdpSocket::bind("127.0.0.1:34255")?;
    let mut i = 0;
    let mut rng = rand::thread_rng();
    loop {
        let addr = "127.0.0.1:34254";

        std::thread::sleep(std::time::Duration::from_millis(10));

        if shared.exit_signal.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut objs = shared.objs.lock().unwrap();
        let mut amt = 0;
        for (i, obj) in objs.iter().enumerate() {
            amt += socket.send_to(&i.to_le_bytes(), addr)?;
            let buf: [u8; std::mem::size_of::<Object>()] = unsafe { std::mem::transmute(*obj) };
            amt += socket.send_to(&buf, addr)?;
        }

        println!("[{i}] Sent {amt} bytes!");
        shared.total_amt.fetch_add(amt, Ordering::Relaxed);

        for obj in objs.iter_mut() {
            obj.pos[0] += (rng.gen::<f64>() - 0.5) * MOTION;
            obj.pos[1] += (rng.gen::<f64>() - 0.5) * MOTION;
        }
        drop(objs);

        // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        // let buf = &mut buf[..amt];
        // buf.reverse();
        // let (amt, addr) = socket.recv_from(&mut buf)?;

        // println!(
        //     "[{i}] Received response of {amt} bytes from {addr:?}! {:?}",
        //     &buf[..amt]
        // );
        i += 1;
    } // the socket is closed here
      // Ok(())
}

pub struct SenderApp {
    shared: Arc<Shared>,
}

impl eframe::App for SenderApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            Frame::canvas(ui.style()).show(ui, |ui| {
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), egui::Sense::click());

                let to_screen = egui::emath::RectTransform::from_to(
                    Rect::from_min_size(Pos2::ZERO, response.rect.size()),
                    response.rect,
                );

                for obj in self.shared.objs.lock().unwrap().iter() {
                    painter.circle(
                        to_screen.transform_pos(pos2(
                            obj.pos[0] as f32 * SCALE,
                            obj.pos[1] as f32 * SCALE,
                        )),
                        3.,
                        Color32::from_rgb(obj.color[0], obj.color[1], obj.color[2]),
                        (1., Color32::BLACK),
                    );
                }

                painter.text(
                    response.rect.left_top(),
                    Align2::LEFT_TOP,
                    format!(
                        "Sent {} bytes",
                        self.shared.total_amt.load(Ordering::Relaxed)
                    ),
                    FontId::proportional(16.),
                    Color32::BLACK,
                );
            });
        });
    }
}
