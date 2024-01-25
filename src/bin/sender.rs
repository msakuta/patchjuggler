use clap::Parser;
use eframe::{
    egui::{self, Color32, Context, Frame},
    emath::Align2,
    epaint::{pos2, FontId, Pos2, Rect},
};
use rand::prelude::*;
use std::{
    error::Error,
    mem::size_of,
    net::{Ipv4Addr, UdpSocket},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use zerocopy::AsBytes;

use patchjuggler::{Object, NUM_OBJS, SCALE};

const MOTION: f64 = 0.1;

struct Shared {
    args: Args,
    objs: Mutex<Vec<Object>>,
    total_amt: AtomicUsize,
    exit_signal: AtomicBool,
}

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(
        short = 'p',
        long,
        default_value = "34254",
        help = "The port number to send packets to."
    )]
    dest_port: u16,
    #[clap(
        short = 'h',
        long,
        default_value = "127.0.0.1",
        help = "The host address to send packets to."
    )]
    dest_host: Ipv4Addr,
    #[clap(
        short = 'P',
        long,
        default_value = "34255",
        help = "The port number of the sender's socket."
    )]
    src_port: u16,
    #[clap(
        short = 'H',
        long,
        default_value = "127.0.0.1",
        help = "Interval to auto-cleanup cache memory, in seconds."
    )]
    src_host: Ipv4Addr,
    #[clap(
        short = 'r',
        long,
        default_value = "10",
        help = "The rate at which packets are sent in milliseconds"
    )]
    rate: u64,
    #[clap(
        short = 'n',
        long,
        default_value = "1000",
        help = "The number of objects to synchronize"
    )]
    num_objects: usize,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let mut rng = rand::thread_rng();
    let num_objects = args.num_objects;
    let shared = Arc::new(Shared {
        args,
        objs: Mutex::new(
            (0..num_objects)
                .map(|_| {
                    Object::new(
                        [rng.gen::<f64>() * 10., rng.gen::<f64>() * 10.],
                        [rng.gen::<u8>(), rng.gen(), rng.gen()],
                    )
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
    let socket = UdpSocket::bind((shared.args.src_host, shared.args.src_port))?;
    let mut t = 0;
    let mut rng = rand::thread_rng();
    loop {
        let addr = (shared.args.dest_host, shared.args.dest_port);

        std::thread::sleep(std::time::Duration::from_millis(shared.args.rate));

        if shared.exit_signal.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut objs = shared.objs.lock().unwrap();
        for obj in objs.iter_mut() {
            obj.pos[0] += (rng.gen::<f64>() - 0.5) * MOTION;
            obj.pos[1] += (rng.gen::<f64>() - 0.5) * MOTION;
        }

        let mut amt = 0;

        // First, send the number of objects to allocate
        let mut buf = [0u8; size_of::<usize>() + size_of::<usize>()];
        0usize.write_to(&mut buf[..size_of::<usize>()]);
        objs.len().write_to(&mut buf[size_of::<usize>()..]);
        amt += socket.send_to(&buf, &addr)?;

        for (i, obj) in objs.iter().enumerate() {
            let mut buf = [0u8; size_of::<usize>() + size_of::<Object>()];
            (i + 1).write_to(&mut buf[..size_of::<usize>()]);
            obj.write_to(&mut buf[size_of::<usize>()..]);
            amt += socket.send_to(&buf, &addr)?;
        }

        // Don't print to terminal too often
        if t % 100 == 0 {
            println!("[{t}] Sent {amt} bytes!");
        }
        shared.total_amt.fetch_add(amt, Ordering::Relaxed);

        drop(objs);

        t += 1;
    }
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
