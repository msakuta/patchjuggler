use clap::Parser;
use eframe::{
    egui::{self, Color32, Context, Frame, Ui},
    emath::Align2,
    epaint::FontId,
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

use patchjuggler::{
    object::{BoidScanner, FindScanner},
    render_objects, Object, SortMap, UpdateScanner,
};

struct Shared {
    args: Args,
    objs: Mutex<Vec<Object>>,
    total_amt: AtomicUsize,
    exit_signal: AtomicBool,
    sort_map: Mutex<SortMap>,
    selected_obj: Mutex<Option<usize>>,
    find_result: Mutex<Vec<usize>>,
    use_sort_map: AtomicBool,
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
        objs: Mutex::new(vec![]),
        total_amt: AtomicUsize::new(0),
        exit_signal: AtomicBool::new(false),
        sort_map: Mutex::new(SortMap::new(0)),
        selected_obj: Mutex::new(None),
        find_result: Mutex::new(vec![]),
        use_sort_map: AtomicBool::new(true),
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
        if i == 0 {
            let num_objects =
                usize::read_from(&buf[size_of::<usize>()..size_of::<usize>() * 2]).unwrap();
            shared
                .objs
                .lock()
                .unwrap()
                .resize(num_objects, Object::default());
            shared.sort_map.lock().unwrap().resize(num_objects);
            continue;
        }
        let buf = Object::ref_from(&buf[size_of::<usize>()..]).unwrap();

        shared.total_amt.fetch_add(amt1, Ordering::Relaxed);

        let mut objs = shared.objs.lock().unwrap();
        if i - 1 < objs.len() {
            objs[i - 1] = *buf;
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

impl ReceiverApp {
    fn update_objs(&mut self) {
        let mut objs = self.shared.objs.lock().unwrap();
        // for obj in objs.iter_mut() {
        //     obj.time_step();
        // }
        let mut sort_map = self.shared.sort_map.lock().unwrap();
        let mut scanner = BoidScanner::new(None);
        //*self.shared.selected_obj.lock().unwrap()
        if self.shared.use_sort_map.load(Ordering::Relaxed) {
            let mut find_scanner = FindScanner::new(*self.shared.selected_obj.lock().unwrap());
            sort_map.update(&objs);
            sort_map.scan(&mut objs, &mut scanner);
            sort_map.scan(&mut objs, &mut find_scanner);
            let mut find_result = self.shared.find_result.lock().unwrap();
            *find_result = find_scanner.into_find_result();
        } else {
            for i in 0..objs.len() {
                scanner.start(i, &objs[i]);
                for (j, obj2) in objs.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    scanner.next(j, obj2);
                }
                scanner.end(i, &mut objs[i]);
            }
        }
        // self.shared.find_result.lock().unwrap().clear();
    }

    fn ui_panel(&mut self, ui: &mut Ui) {
        // ui.checkbox(&mut self.show_grid, "Show grid");
        // ui.checkbox(&mut self.show_neighbors, "Show neighbors");
        // ui.checkbox(&mut self.show_distances, "Show distances");
        let mut use_sort_map = self.shared.use_sort_map.load(Ordering::Acquire);
        ui.checkbox(&mut use_sort_map, "Use sort map");
        self.shared
            .use_sort_map
            .store(use_sort_map, Ordering::Release);
    }
}

impl eframe::App for ReceiverApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        self.update_objs();

        egui::SidePanel::right("side_panel")
            .min_width(200.)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.ui_panel(ui))
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            Frame::canvas(ui.style()).show(ui, |ui| {
                let objs = self.shared.objs.lock().unwrap();
                let (response, painter) = render_objects(&objs, None, ui);
                drop(objs); // Release the mutex ASAP

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
