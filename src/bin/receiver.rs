use eframe::{
    egui::{self, Color32, Context, Frame},
    emath::Align2,
    epaint::{pos2, FontId, Pos2, Rect},
};
use std::{
    error::Error,
    net::UdpSocket,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use juggler::{Object, NUM_OBJS, SCALE};

struct Shared {
    objs: Mutex<Vec<Object>>,
    total_amt: AtomicUsize,
    exit_signal: AtomicBool,
}

fn main() -> Result<(), String> {
    let shared = Arc::new(Shared {
        objs: Mutex::new((0..NUM_OBJS).map(|_| Object { pos: [0., 0.] }).collect()),
        total_amt: AtomicUsize::new(0),
        exit_signal: AtomicBool::new(false),
    });

    let shared_copy = shared.clone();
    let thread =
        std::thread::spawn(move || receiver_thread(shared_copy).map_err(|e| format!("{e}")));

    println!("receiver_thread departed!");

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
        "receiver GUI",
        native_options,
        Box::new(|_cc| Box::new(ReceiverApp { shared })),
    )?)
}

fn receiver_thread(shared: Arc<Shared>) -> Result<(), Box<dyn Error>> {
    let socket = UdpSocket::bind("127.0.0.1:34254")?;
    let mut t = 0;
    loop {
        if shared.exit_signal.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut buf = [0; std::mem::size_of::<usize>()];
        let (amt1, _src) = socket.recv_from(&mut buf)?;
        let i = usize::from_le_bytes(buf);
        let mut buf = [0; std::mem::size_of::<Object>()];
        let (amt2, src) = socket.recv_from(&mut buf)?;

        let total_amt = amt1 + amt2;
        shared.total_amt.fetch_add(total_amt, Ordering::Relaxed);

        let mut objs = shared.objs.lock().unwrap();
        if i < objs.len() {
            unsafe {
                objs[i] = std::mem::transmute(buf);
            }
            // println!(
            //     "[{t}]: Received {total_amt} bytes from {src:?}: {i} = {:?}!",
            //     objs[i]
            // );
        }
        drop(objs);

        // std::thread::sleep(std::time::Duration::from_millis(1000));

        // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        // let buf = &mut buf[..amt];
        // buf.reverse();
        // let amt = socket.send_to(buf, &src)?;
        // println!("[{i}]: Sent {amt} bytes!");
        t += 1;
    }
    // Ok(())
}

pub struct ReceiverApp {
    shared: Arc<Shared>,
}

impl eframe::App for ReceiverApp {
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
                        Color32::from_rgb(191, 191, 191),
                        (1., Color32::BLACK),
                    );
                }

                painter.text(
                    response.rect.left_top(),
                    Align2::LEFT_TOP,
                    format!(
                        "Received {} bytes",
                        self.shared.total_amt.load(Ordering::Relaxed)
                    ),
                    FontId::proportional(16.),
                    Color32::BLACK,
                );
            });
        });
    }
}
