use clap::Parser;
use eframe::{
    egui::{self, Color32, Context, Frame},
    emath::Align2,
    epaint::{pos2, FontId, Pos2, Rect},
};
use std::{
    error::Error,
    mem::size_of,
    net::{Ipv4Addr, UdpSocket},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use zerocopy::FromBytes;

use patchjuggler::{Object, NUM_OBJS, SCALE};

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
        help = "The port number of the receiver's socket."
    )]
    port: u16,
    #[clap(
        short = 'h',
        long,
        default_value = "127.0.0.1",
        help = "The address of the receiver's socket."
    )]
    host: Ipv4Addr,
}

fn main() -> Result<(), String> {
    let shared = Arc::new(Shared {
        args: Args::parse(),
        objs: Mutex::new((0..NUM_OBJS).map(|_| Object::default()).collect()),
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
    let socket = UdpSocket::bind((shared.args.host, shared.args.port))?;
    let mut _t = 0;
    loop {
        if shared.exit_signal.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut buf = [0; size_of::<usize>() + size_of::<Object>()];
        let (amt1, _src) = socket.recv_from(&mut buf)?;
        let i = usize::read_from(&buf[..size_of::<usize>()]).unwrap();
        let buf = Object::ref_from(&buf[size_of::<usize>()..]).unwrap();

        shared.total_amt.fetch_add(amt1, Ordering::Relaxed);

        let mut objs = shared.objs.lock().unwrap();
        if i < objs.len() {
            objs[i] = *buf;
            // if i == 0 {
            //     println!(
            //         "[{t}]: Received {amt1} bytes from {_src:?}: {i} = {:?}!",
            //         objs[i]
            //     );
            // }
        }
        drop(objs);

        _t += 1;
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
                        Color32::from_rgb(obj.color[0], obj.color[1], obj.color[2]),
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
