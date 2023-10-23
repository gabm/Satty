use std::io::Read;
use std::{io, time::Duration};

use gdk_pixbuf::{Pixbuf, PixbufLoader};
use gtk::prelude::*;
use relm4::gtk::gdk::Rectangle;
use relm4::RelmWidgetExt;
use relm4::{
    actions::{ActionablePlus, RelmAction, RelmActionGroup},
    gtk::{self, gdk::DisplayManager, Align, CssProvider, Inhibit, Window},
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
};

use anyhow::{anyhow, Context, Result};
use style::Color;

mod math;
mod sketch_board;
mod style;
mod tools;

use crate::sketch_board::SketchBoardConfig;
use crate::style::Size;
use crate::{
    sketch_board::{KeyEventMsg, SketchBoard, SketchBoardMessage},
    tools::Tools,
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(
        short,
        long,
        help = "Filename to read from, use '-' to read from stdin"
    )]
    filename: String,

    #[arg(long, help = "whether to use fullscreen")]
    fullscreen: bool,

    #[arg(long, help = "Which filename to use for saving action")]
    output_filename: Option<String>,
}

struct AppConfig {
    image: Pixbuf,
    args: Args,
}

struct App {
    original_image_width: i32,
    original_image_height: i32,
    sketch_board: Controller<SketchBoard>,
    initially_fullscreen: bool,
}

#[derive(Debug)]
enum AppInput {
    Realized,
}

#[derive(Debug)]
struct ResetResizable;

impl App {
    fn get_monitor_size(root: &Window) -> Option<Rectangle> {
        let surface = root.surface();
        DisplayManager::get()
            .default_display()
            .and_then(|display| display.monitor_at_surface(&surface))
            .and_then(|monitor| Some(monitor.geometry()))
    }

    fn resize_window_initial(&self, root: &Window, sender: ComponentSender<Self>) {
        let monitor_size = match Self::get_monitor_size(root) {
            Some(s) => s,
            None => {
                root.set_default_size(self.original_image_width, self.original_image_height);
                return;
            }
        };

        let reduced_monitor_width = monitor_size.width() as f64 * 0.8;
        let reduced_monitor_height = monitor_size.height() as f64 * 0.8;

        let image_width = self.original_image_width as f64;
        let image_height = self.original_image_height as f64;

        // create a window that uses 80% of the available space max
        // if necessary, scale down image
        if reduced_monitor_width > image_width && reduced_monitor_height > image_height {
            // set window to exact size
            root.set_default_size(self.original_image_width, self.original_image_height);
        } else {
            // scale down and use windowed mode
            let aspect_ratio = image_width / image_height;

            // resize
            let mut new_width = reduced_monitor_width;
            let mut new_height = new_width / aspect_ratio;

            // if new_heigth is still bigger than monitor height, then scale on monitor height
            if new_height > reduced_monitor_height {
                new_height = reduced_monitor_height;
                new_width = new_height * aspect_ratio;
            }

            root.set_default_size(new_width as i32, new_height as i32);
        }

        root.set_resizable(false);

        if self.initially_fullscreen {
            root.fullscreen();
        }

        // this is a horrible hack to let sway recognize the window as "not resizable" and
        // place it floating mode. We then re-enable resizing to let if fit fullscreen (if requested)
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    out.send(ResetResizable).unwrap();
                })
                .drop_on_shutdown()
        });
    }

    fn create_icon(color: Color) -> gtk::Image {
        let pixbuf =
            gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, false, 8, 40, 40).unwrap();
        pixbuf.fill(color.to_rgba_u32());

        gtk::Image::from_pixbuf(Some(&pixbuf))
    }

    fn apply_style() {
        let css_provider = CssProvider::new();
        css_provider.load_from_data(
            "
            .toolbar {color: #f9f9f9 ; background: #00000099;}
            .toolbar-bottom {border-radius: 6px 6px 0px 0px;}
            .toolbar-top {border-radius: 0px 0px 6px 6px;}
            ",
        );
        match DisplayManager::get().default_display() {
            Some(display) => {
                gtk::style_context_add_provider_for_display(&display, &css_provider, 1)
            }
            None => println!("Cannot apply style"),
        }
    }
}

#[relm4::component]
impl Component for App {
    type Init = AppConfig;
    type Input = AppInput;
    type Output = ();
    type CommandOutput = ResetResizable;

    view! {
          main_window = gtk::Window {
            set_default_size: (500, 500),

            connect_show[sender] => move |_| {
                sender.input(AppInput::Realized);
            },

            // this should be inside Sketchboard, but doesn't seem so work there. We hook it here
            // and send the messages there
            add_controller = gtk::EventControllerKey {
                connect_key_pressed[sketch_board_sender] => move | _, key, code, modifier | {
                    sketch_board_sender.emit(SketchBoardMessage::new_key_event(KeyEventMsg::new(key, code, modifier)));
                    Inhibit(false)
                }
            },

            gtk::Overlay {
                add_overlay = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 2,
                    set_valign: Align::Start,
                    set_halign: Align::Center,
                    add_css_class: "toolbar",
                    add_css_class: "toolbar-top",

                    gtk::Button {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "arrow-undo-filled",
                        set_tooltip: "Undo",
                        connect_clicked[sketch_board_sender] => move |_| {sketch_board_sender.emit(SketchBoardMessage::Undo);},
                    },
                    gtk::Button {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "arrow-redo-filled",
                        set_tooltip: "Redo",
                        connect_clicked[sketch_board_sender] => move |_| {sketch_board_sender.emit(SketchBoardMessage::Redo);},
                    },
                    gtk::Separator {},
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "crop-filled",
                        set_tooltip: "Crop",
                        ActionablePlus::set_action::<ToolsAction>: Tools::Crop,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "minus-large",
                        set_tooltip: "Line tool",
                        ActionablePlus::set_action::<ToolsAction>: Tools::Line,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "arrow-up-right-filled",
                        set_tooltip: "Arrow tool",
                        ActionablePlus::set_action::<ToolsAction>: Tools::Arrow,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "checkbox-unchecked-regular",
                        set_tooltip: "Rectangle tool",
                        ActionablePlus::set_action::<ToolsAction>: Tools::Rectangle,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "text-case-title-regular",
                        set_tooltip: "Text tool",
                        ActionablePlus::set_action::<ToolsAction>: Tools::Text,

                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "drop-regular",
                        set_tooltip: "Blur",
                        ActionablePlus::set_action::<ToolsAction>: Tools::Blur,

                    },
                    gtk::Separator {},
                    gtk::Button {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "copy-regular",
                        set_tooltip: "Copy to clipboard",
                        connect_clicked[sketch_board_sender] => move |_| {sketch_board_sender.emit(SketchBoardMessage::CopyClipboard);},
                    },
                    #[name(save_button)]
                    gtk::Button {
                        set_focusable: false,
                        set_hexpand: false,

                        set_icon_name: "save-regular",
                        set_tooltip: "Save",
                        connect_clicked[sketch_board_sender] => move |_| {sketch_board_sender.emit(SketchBoardMessage::SaveFile);},
                    },

                },

                add_overlay = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 2,
                    set_valign: Align::End,
                    set_halign: Align::Center,
                    add_css_class: "toolbar",
                    add_css_class: "toolbar-bottom",

                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        #[local_ref]
                        orange_icon -> gtk::Image {},

                        ActionablePlus::set_action::<ColorAction>: Color::Orange,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        #[local_ref]
                        red_icon -> gtk::Image {},

                        ActionablePlus::set_action::<ColorAction>: Color::Red,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        #[local_ref]
                        green_icon -> gtk::Image {},

                        ActionablePlus::set_action::<ColorAction>: Color::Green,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        #[local_ref]
                        blue_icon -> gtk::Image {},

                        ActionablePlus::set_action::<ColorAction>: Color::Blue,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        #[local_ref]
                        cove_icon -> gtk::Image {},

                        ActionablePlus::set_action::<ColorAction>: Color::Cove,
                    },
                    gtk::Separator {},
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_label: "S",
                        set_tooltip: "Small font size",
                        ActionablePlus::set_action::<SizeAction>: Size::Small,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_label: "M",
                        set_tooltip: "Medium font size",
                        ActionablePlus::set_action::<SizeAction>: Size::Medium,
                    },
                    gtk::ToggleButton {
                        set_focusable: false,
                        set_hexpand: false,

                        set_label: "L",
                        set_tooltip: "Large font size",
                        ActionablePlus::set_action::<SizeAction>: Size::Large,
                    },
                },

                #[local_ref]
                sketch_board -> gtk::Box {}

            }
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            AppInput::Realized => self.resize_window_initial(root, sender),
        }
    }

    fn update_cmd(&mut self, _: ResetResizable, _: ComponentSender<Self>, root: &Self::Root) {
        root.set_resizable(true);
    }

    fn init(
        config: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        Self::apply_style();

        let original_image_width = config.image.width();
        let original_image_height = config.image.height();

        let sketch_board_config = SketchBoardConfig {
            original_image: config.image,
            output_filename: config.args.output_filename.clone(),
        };

        let sketch_board = SketchBoard::builder().launch(sketch_board_config).detach();
        let sketch_board_sender = sketch_board.sender().clone();

        let model = App {
            original_image_width,
            original_image_height,
            sketch_board,
            initially_fullscreen: config.args.fullscreen,
        };

        let sketch_board = model.sketch_board.widget();

        // color icons
        let orange_icon = Self::create_icon(Color::Orange);
        let red_icon = Self::create_icon(Color::Red);
        let green_icon = Self::create_icon(Color::Green);
        let blue_icon = Self::create_icon(Color::Blue);
        let cove_icon = Self::create_icon(Color::Cove);

        let widgets = view_output!();

        widgets
            .save_button
            .set_visible(config.args.output_filename.is_some());

        // Tools Action for selecting tools
        let sketch_board_sender_tmp = sketch_board_sender.clone();
        let tool_action: RelmAction<ToolsAction> =
            RelmAction::new_stateful_with_target_value(&Tools::Crop, move |_, state, value| {
                *state = value;
                sketch_board_sender_tmp.emit(SketchBoardMessage::ToolSelected(*state));
            });

        // Color Action for selecting colors
        let sketch_board_sender_tmp = sketch_board_sender.clone();
        let color_action: RelmAction<ColorAction> =
            RelmAction::new_stateful_with_target_value(&Color::Orange, move |_, state, value| {
                *state = value;
                sketch_board_sender_tmp.emit(SketchBoardMessage::ColorSelected(*state));
            });

        // Size Action for selecting sizes
        let sketch_board_sender_tmp = sketch_board_sender.clone();
        let size_action: RelmAction<SizeAction> =
            RelmAction::new_stateful_with_target_value(&Size::Medium, move |_, state, value| {
                *state = value;
                sketch_board_sender_tmp.emit(SketchBoardMessage::SizeSelected(*state));
            });

        let mut group = RelmActionGroup::<WindowActionGroup>::new();
        group.add_action(tool_action);
        group.add_action(color_action);
        group.add_action(size_action);
        group.register_for_widget(&widgets.main_window);

        ComponentParts { model, widgets }
    }
}

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateful_action!(ToolsAction, WindowActionGroup, "tools", Tools, Tools);
relm4::new_stateful_action!(ColorAction, WindowActionGroup, "colors", Color, Color);
relm4::new_stateful_action!(SizeAction, WindowActionGroup, "sizes", Size, Size);

fn load_image(filename: &str) -> Result<Pixbuf> {
    Ok(Pixbuf::from_file(filename).context("couldn't load image")?)
}

fn run_satty(args: Args) -> Result<()> {
    let image = if args.filename == "-" {
        let mut buf = Vec::<u8>::new();
        io::stdin().lock().read_to_end(&mut buf)?;
        let pb_loader = PixbufLoader::new();
        pb_loader.write(&buf)?;
        pb_loader.close()?;
        pb_loader
            .pixbuf()
            .ok_or(anyhow!("Conversion to Pixbuf failed"))?
    } else {
        load_image(&args.filename)?
    };

    let app = RelmApp::new("com.gabm.satty");
    relm4_icons::initialize_icons();
    app.run_with_args::<App, &str>(AppConfig { args, image }, &[]);
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    match run_satty(args) {
        Err(e) => {
            println!("Error: {e}");
            Err(e)
        }
        Ok(v) => Ok(v),
    }
}
