use clap::Parser;
use eframe::{
    egui::{self, Color32, Context, Frame, Ui},
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

use patchjuggler::{
    object::{BoidScanner, FindScanner, ALIGNMENT_DIST, GROUP_SEPARATION_DIST, SEPARATION_DIST},
    render_objects, Object, SortMap, UpdateScanner, SCALE,
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
        Box::new(|_cc| {
            Box::new(ReceiverApp {
                shared,
                show_grid: true,
                show_neighbors: true,
                show_distances: true,
            })
        }),
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
    show_grid: bool,
    show_neighbors: bool,
    show_distances: bool,
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

    fn render(&mut self, ui: &mut Ui) {
        let objs = self.shared.objs.lock().unwrap();
        let (response, painter) = render_objects(&objs, None, ui);
        drop(objs); // Release the mutex ASAP

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
                "Received {} bytes",
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
                self.render(ui);
            });
        });
    }
}
