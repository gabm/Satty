use std::io::Read;
use std::{io, time::Duration};

use command_line::CommandLine;
use gdk_pixbuf::{Pixbuf, PixbufLoader};
use gtk::prelude::*;
use relm4::gtk::gdk::Rectangle;

use relm4::{
    gtk::{self, gdk::DisplayManager, CssProvider, Inhibit, Window},
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
};

use anyhow::{anyhow, Context, Result};

use sketch_board::SketchBoardOutput;
use ui::toast::Toast;
use ui::toolbars::{StyleToolbar, ToolsToolbar};

mod command_line;
mod math;
mod renderer;
mod sketch_board;
mod style;
mod tools;
mod ui;

use crate::sketch_board::SketchBoardConfig;
use crate::sketch_board::{KeyEventMsg, SketchBoard, SketchBoardInput};

use crate::ui::toolbars::ToolsToolbarConfig;

struct AppConfig {
    image: Pixbuf,
    args: CommandLine,
}

struct App {
    original_image_width: i32,
    original_image_height: i32,
    sketch_board: Controller<SketchBoard>,
    initially_fullscreen: bool,
    toast: Controller<Toast>,
    tools_toolbar: Controller<ToolsToolbar>,
    style_toolbar: Controller<StyleToolbar>,
}

#[derive(Debug)]
enum AppInput {
    Realized,
}

#[derive(Debug)]
enum AppCommandOutput {
    ResetResizable,
}

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
                    out.emit(AppCommandOutput::ResetResizable);
                })
                .drop_on_shutdown()
        });
    }

    fn apply_style() {
        let css_provider = CssProvider::new();
        css_provider.load_from_data(
            "
            .toolbar {color: #f9f9f9 ; background: #00000099;}
            .toast {
                color: #f9f9f9;
                background: #00000099;
                border-radius: 6px;
                margin-top: 50px;
            }
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
    type CommandOutput = AppCommandOutput;

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
                    sketch_board_sender.emit(SketchBoardInput::new_key_event(KeyEventMsg::new(key, code, modifier)));
                    Inhibit(false)
                }
            },

            gtk::Overlay {
                add_overlay = model.tools_toolbar.widget(),

                add_overlay = model.style_toolbar.widget(),

                add_overlay = model.toast.widget(),

                model.sketch_board.widget(),
            }
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            AppInput::Realized => self.resize_window_initial(root, sender),
        }
    }

    fn update_cmd(
        &mut self,
        command: AppCommandOutput,
        _: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match command {
            AppCommandOutput::ResetResizable => root.set_resizable(true),
        }
    }

    fn init(
        config: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        Self::apply_style();

        // Toast
        let toast = Toast::builder().launch(2000).detach();

        // SketchBoard
        let sketch_board_config = SketchBoardConfig {
            original_image: config.image.clone(),
            output_filename: config.args.output_filename.clone(),
            early_exit: config.args.early_exit,
        };

        let sketch_board = SketchBoard::builder().launch(sketch_board_config).forward(
            toast.sender(),
            |t| match t {
                SketchBoardOutput::ShowToast(msg) => ui::toast::ToastMessage::Show(msg),
            },
        );

        let sketch_board_sender = sketch_board.sender().clone();

        // Toolbars
        let tools_toolbar = ToolsToolbar::builder()
            .launch(ToolsToolbarConfig {
                show_save_button: config.args.output_filename.is_some(),
            })
            .forward(sketch_board.sender(), |e| SketchBoardInput::ToolbarEvent(e));

        let style_toolbar = StyleToolbar::builder()
            .launch(())
            .forward(sketch_board.sender(), |e| SketchBoardInput::ToolbarEvent(e));

        // Model
        let model = App {
            original_image_width: config.image.width(),
            original_image_height: config.image.height(),
            sketch_board,
            initially_fullscreen: config.args.fullscreen,
            toast,
            tools_toolbar,
            style_toolbar,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

fn load_image(filename: &str) -> Result<Pixbuf> {
    Ok(Pixbuf::from_file(filename).context("couldn't load image")?)
}

fn run_satty(args: CommandLine) -> Result<()> {
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

    let app = RelmApp::new("com.gabm.satty").with_args(vec![]);
    relm4_icons::initialize_icons();
    app.run::<App>(AppConfig { args, image });
    Ok(())
}

fn main() -> Result<()> {
    let args = CommandLine::do_parse();

    match run_satty(args) {
        Err(e) => {
            println!("Error: {e}");
            Err(e)
        }
        Ok(v) => Ok(v),
    }
}
