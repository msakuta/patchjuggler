use clap::Parser;
use eframe::{
    egui::{self, Color32, Context, Frame, Ui},
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

use patchjuggler::{
    object::{BoidScanner, ALIGNMENT_DIST, GROUP_SEPARATION_DIST, RANDOM_MOTION, SEPARATION_DIST},
    render_objects, Object, SortMap, UpdateScanner, SCALE, SPACE_WIDTH,
};

pub const SELECT_RADIUS: f32 = 0.5;

struct Shared {
    args: Args,
    objs: Mutex<Vec<Object>>,
    total_amt: AtomicUsize,
    exit_signal: AtomicBool,
    sort_map: Mutex<SortMap>,
    selected_obj: Mutex<Option<usize>>,
    find_result: Mutex<Vec<usize>>,
    use_sort_map: AtomicBool,
    randomness: Mutex<f64>,
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
    #[clap(
        short = 'b',
        long,
        default_value = "10",
        help = "The number of objects to send in one burst. Having a low value helps GUI to run smoothly but will have overhead sending patches"
    )]
    burst_objs: usize,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let mut rng = rand::thread_rng();
    let num_objects = args.num_objects;
    let objs = (0..num_objects)
        .map(|_| {
            Object::new(
                [
                    rng.gen::<f64>() * SPACE_WIDTH,
                    rng.gen::<f64>() * SPACE_WIDTH,
                ],
                [rng.gen::<u8>(), rng.gen(), rng.gen()],
            )
        })
        .collect();
    let sort_map = SortMap::new(num_objects);
    let shared = Arc::new(Shared {
        args,
        objs: Mutex::new(objs),
        total_amt: AtomicUsize::new(0),
        exit_signal: AtomicBool::new(false),
        sort_map: Mutex::new(sort_map),
        selected_obj: Mutex::new(None),
        find_result: Mutex::new(vec![]),
        use_sort_map: AtomicBool::new(true),
        randomness: Mutex::new(RANDOM_MOTION),
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
        Box::new(|_cc| {
            Box::new(SenderApp {
                shared,
                show_grid: true,
                show_neighbors: true,
                show_distances: true,
            })
        }),
    )?)
}

/// A local scanner that finds neighboring objects info
struct FindScanner {
    find_index: Option<usize>,
    find_result: Vec<usize>,
    i: Option<usize>,
}

impl FindScanner {
    fn new(find_index: Option<usize>) -> Self {
        Self {
            find_index,
            find_result: vec![],
            i: None,
        }
    }
}

impl UpdateScanner for FindScanner {
    fn start(&mut self, i: usize, _obj1: &Object) {
        self.i = Some(i);
    }

    fn next(&mut self, j: usize, _obj2: &Object) {
        if self.i.is_some() && self.i == self.find_index {
            self.find_result.push(j);
        }
    }

    fn end(&mut self, _i: usize, _obj: &mut Object) {}
}

fn sender_thread(shared: Arc<Shared>) -> Result<(), Box<dyn Error>> {
    std::thread::sleep(std::time::Duration::from_millis(1000));
    let socket = UdpSocket::bind((shared.args.src_host, shared.args.src_port))?;
    let mut t = 0;
    let mut n = 0;
    let mut rng = rand::thread_rng();
    loop {
        let addr = (shared.args.dest_host, shared.args.dest_port);

        std::thread::sleep(std::time::Duration::from_millis(shared.args.rate));

        if shared.exit_signal.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut objs = shared.objs.lock().unwrap();
        let mut sort_map = shared.sort_map.lock().unwrap();
        // let hash_table = vec![HashEntry::default(); objs.len()];
        // let start_offsets = vec![usize::MAX; objs.len()];
        let mut scanner = BoidScanner::new(Some(&mut rng), *shared.randomness.lock().unwrap());
        if shared.use_sort_map.load(Ordering::Relaxed) {
            let mut find_scanner = FindScanner::new(*shared.selected_obj.lock().unwrap());
            sort_map.update(&objs);
            sort_map.scan(&mut objs, &mut scanner);
            sort_map.scan(&mut objs, &mut find_scanner);
            let mut find_result = shared.find_result.lock().unwrap();
            *find_result = find_scanner.find_result;
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
            shared.find_result.lock().unwrap().clear();
        }

        let mut amt = 0;

        // First, send the number of objects to allocate
        let mut buf = [0u8; size_of::<usize>() + size_of::<usize>()];
        0usize.write_to(&mut buf[..size_of::<usize>()]);
        objs.len().write_to(&mut buf[size_of::<usize>()..]);
        amt += socket.send_to(&buf, &addr)?;

        for (i, obj) in objs.iter().enumerate().skip(n).take(shared.args.burst_objs) {
            let mut buf = [0u8; size_of::<usize>() + size_of::<Object>()];
            (i + 1).write_to(&mut buf[..size_of::<usize>()]);
            obj.write_to(&mut buf[size_of::<usize>()..]);
            amt += socket.send_to(&buf, &addr)?;
        }
        n += shared.args.burst_objs;
        if objs.len() <= n {
            n = 0;
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
    show_grid: bool,
    show_neighbors: bool,
    show_distances: bool,
}

impl SenderApp {
    fn render(&mut self, ui: &mut Ui) {
        let (response, painter) = render_objects(
            &self.shared.objs.lock().unwrap(),
            *self.shared.selected_obj.lock().unwrap(),
            ui,
            false,
        );

        let to_screen = egui::emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
            response.rect,
        );

        let from_screen = egui::emath::RectTransform::from_to(
            response.rect,
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
        );

        if response.clicked() {
            if let Some(scr_pos) = response.interact_pointer_pos() {
                let pos = from_screen.transform_pos(scr_pos) / SCALE;
                let closest_obj = self.shared.objs.lock().unwrap().iter().enumerate().fold(
                    None,
                    |acc: Option<(f32, usize)>, cur| {
                        let dist = (pos - pos2(cur.1.pos[0] as f32, cur.1.pos[1] as f32)).length();
                        if let Some(acc) = acc {
                            if dist < SELECT_RADIUS && dist < acc.0 {
                                Some((dist, cur.0))
                            } else {
                                Some(acc)
                            }
                        } else if dist < SELECT_RADIUS {
                            Some((dist, cur.0))
                        } else {
                            None
                        }
                    },
                );
                *self.shared.selected_obj.lock().unwrap() = closest_obj.map(|o| o.1);
            }
        }

        if self.show_neighbors {
            if let Some(selected_obj) = *self.shared.selected_obj.lock().unwrap() {
                let objs = self.shared.objs.lock().unwrap();
                if let Ok(first_result) = self.shared.find_result.lock() {
                    let pos0 = objs[selected_obj].pos;
                    for j in first_result.iter() {
                        let pos_j = objs[*j].pos;
                        painter.line_segment(
                            [
                                to_screen.transform_pos(pos2(
                                    pos0[0] as f32 * SCALE,
                                    pos0[1] as f32 * SCALE,
                                )),
                                to_screen.transform_pos(pos2(
                                    pos_j[0] as f32 * SCALE,
                                    pos_j[1] as f32 * SCALE,
                                )),
                            ],
                            (1., Color32::from_rgb(0, 127, 255)),
                        );
                    }
                }
            }
        }

        if self.show_distances {
            if let Some(&selected_obj) = self.shared.selected_obj.lock().unwrap().as_ref() {
                let pos = self.shared.objs.lock().unwrap()[selected_obj].pos;
                for (dist, color) in [
                    (SEPARATION_DIST, Color32::from_rgb(255, 0, 255)),
                    (ALIGNMENT_DIST, Color32::from_rgb(0, 127, 127)),
                    (GROUP_SEPARATION_DIST, Color32::from_rgb(127, 127, 0)),
                ] {
                    painter.circle_stroke(
                        to_screen.transform_pos(pos2(pos[0] as f32 * SCALE, pos[1] as f32 * SCALE)),
                        dist as f32 * SCALE,
                        (1., color),
                    );
                }
            }
        }

        if self.show_grid {
            let objs = self.shared.objs.lock().unwrap();
            SortMap::render_grid(
                objs.iter().map(|o| [o.pos[0] as f32, o.pos[1] as f32]),
                &response,
                &painter,
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
    }

    fn ui_panel(&mut self, ui: &mut Ui) {
        ui.checkbox(&mut self.show_grid, "Show grid");
        ui.checkbox(&mut self.show_neighbors, "Show neighbors");
        ui.checkbox(&mut self.show_distances, "Show distances");
        let mut use_sort_map = self.shared.use_sort_map.load(Ordering::Acquire);
        ui.checkbox(&mut use_sort_map, "Use sort map");
        self.shared
            .use_sort_map
            .store(use_sort_map, Ordering::Release);
        ui.label("Randomness:");
        let mut randomness = self.shared.randomness.lock().unwrap();
        ui.add(egui::widgets::Slider::new(&mut *randomness, (0.)..=0.1));
        drop(randomness);
    }
}

impl eframe::App for SenderApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::SidePanel::right("side_panel")
            .min_width(200.)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.ui_panel(ui))
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            Frame::canvas(ui.style()).show(ui, |ui| {
                self.render(ui);
            });
        });
    }
}
