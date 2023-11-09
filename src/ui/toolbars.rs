use std::borrow::Cow;

use crate::{
    style::{Color, Size},
    tools::Tools,
};

use gdk_pixbuf::{
    glib::{FromVariant, Variant, VariantTy},
    Pixbuf,
};
use relm4::{
    actions::{ActionablePlus, RelmAction, RelmActionGroup},
    gtk::{prelude::*, Align, ColorDialog, Window},
    prelude::*,
};

pub struct ToolsToolbar {
    config: ToolsToolbarConfig,
}

pub struct ToolsToolbarConfig {
    pub show_save_button: bool,
}

pub struct StyleToolbar {
    custom_color: Color,
    custom_color_pixbuf: Pixbuf,
}

#[derive(Debug, Copy, Clone)]
pub enum ToolbarEvent {
    ToolSelected(Tools),
    ColorSelected(Color),
    SizeSelected(Size),
    Redo,
    Undo,
    SaveFile,
    CopyClipboard,
}

#[derive(Debug, Copy, Clone)]
pub enum StyleToolbarInput {
    ShowColorDialog,
    ColorDialogFinished(Option<Color>),
}

fn create_icon_pixbuf(color: Color) -> Pixbuf {
    let pixbuf = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, false, 8, 40, 40).unwrap();
    pixbuf.fill(color.to_rgba_u32());
    pixbuf
}
fn create_icon(color: Color) -> gtk::Image {
    gtk::Image::from_pixbuf(Some(&create_icon_pixbuf(color)))
}

#[relm4::component(pub)]
impl SimpleComponent for ToolsToolbar {
    type Init = ToolsToolbarConfig;
    type Input = ();
    type Output = ToolbarEvent;

    view! {
        root = gtk::Box {
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
                set_tooltip: "Undo (Ctrl-Z)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::Undo);},
            },
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: "arrow-redo-filled",
                set_tooltip: "Redo (Ctrl-Y)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::Redo);},
            },
            gtk::Separator {},
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: "cursor-regular",
                set_tooltip: "Crop",
                ActionablePlus::set_action::<ToolsAction>: Tools::Pointer,
            },
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

                set_icon_name: "number-circle-1-regular",
                set_tooltip: "Numbered Marker",
                ActionablePlus::set_action::<ToolsAction>: Tools::Marker,

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
                set_tooltip: "Copy to clipboard (Ctrl+C)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::CopyClipboard);},
            },
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: "save-regular",
                set_tooltip: "Save (Ctrl+S)",
                connect_clicked[sender] => move |_| {sender.output_sender().emit(ToolbarEvent::SaveFile);},

                set_visible: model.config.show_save_button
            },

        },
    }

    fn init(
        config: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ToolsToolbar { config };
        let widgets = view_output!();

        // Tools Action for selecting tools
        let sender_tmp: ComponentSender<ToolsToolbar> = sender.clone();
        let tool_action: RelmAction<ToolsAction> =
            RelmAction::new_stateful_with_target_value(&Tools::Pointer, move |_, state, value| {
                *state = value;
                sender_tmp
                    .output_sender()
                    .emit(ToolbarEvent::ToolSelected(*state));
            });

        let mut group = RelmActionGroup::<ToolsToolbarActionGroup>::new();
        group.add_action(tool_action);
        group.register_for_widget(&widgets.root);

        ComponentParts { model, widgets }
    }
}

#[derive(Debug, Copy, Clone)]
enum ColorButtons {
    Orange = 0,
    Red = 1,
    Green = 2,
    Blue = 3,
    Cove = 4,
    Custom = 5,
}

impl StyleToolbar {
    fn show_color_dialog(&self, sender: ComponentSender<StyleToolbar>, root: Option<Window>) {
        let current_color = Some(self.custom_color.into());
        relm4::spawn_local(async move {
            let dialog = ColorDialog::builder()
                .modal(true)
                .title("Choose Colour")
                .with_alpha(true)
                .build();

            let color = dialog
                .choose_rgba_future(root.as_ref(), current_color.as_ref())
                .await
                .ok()
                .and_then(|c| Some(Color::from_gdk(c)));

            sender.input(StyleToolbarInput::ColorDialogFinished(color));
        });
    }

    fn map_button_to_color(button: ColorButtons) -> Option<Color> {
        match button {
            ColorButtons::Orange => Some(Color::orange()),
            ColorButtons::Red => Some(Color::red()),
            ColorButtons::Green => Some(Color::green()),
            ColorButtons::Blue => Some(Color::blue()),
            ColorButtons::Cove => Some(Color::cove()),
            ColorButtons::Custom => None,
        }
    }
}

#[relm4::component(pub)]
impl Component for StyleToolbar {
    type Init = ();
    type Input = StyleToolbarInput;
    type Output = ToolbarEvent;
    type CommandOutput = ();

    view! {
        root = gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 2,
            set_valign: Align::End,
            set_halign: Align::Center,
            add_css_class: "toolbar",
            add_css_class: "toolbar-bottom",

            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::orange()),

                ActionablePlus::set_action::<ColorAction>: ColorButtons::Orange,
            },
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::red()),

                ActionablePlus::set_action::<ColorAction>: ColorButtons::Red,
            },
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::green()),

                ActionablePlus::set_action::<ColorAction>: ColorButtons::Green,
            },
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::blue()),

                ActionablePlus::set_action::<ColorAction>: ColorButtons::Blue
            },
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::cove()),

                ActionablePlus::set_action::<ColorAction>: ColorButtons::Cove,
            },
            gtk::Separator {},

            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                gtk::Image::from_pixbuf(Some(&model.custom_color_pixbuf)) {
                    #[watch]
                    set_from_pixbuf: Some(&model.custom_color_pixbuf)
                },

                ActionablePlus::set_action::<ColorAction>: ColorButtons::Custom,

                connect_clicked => StyleToolbarInput::ShowColorDialog,
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
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            StyleToolbarInput::ShowColorDialog => {
                self.show_color_dialog(sender, root.toplevel_window());
            }
            StyleToolbarInput::ColorDialogFinished(color) => {
                let color = color.unwrap_or(self.custom_color);

                self.custom_color = color;
                self.custom_color_pixbuf = create_icon_pixbuf(color);
                sender
                    .output_sender()
                    .emit(ToolbarEvent::ColorSelected(color));
            }
        }
    }
    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let custom_color = Color::pink();
        let custom_color_pixbuf = create_icon_pixbuf(custom_color);

        // create model
        let model = StyleToolbar {
            custom_color,
            custom_color_pixbuf,
        };

        // create widgets
        let widgets = view_output!();

        // Color Action for selecting colors
        let sender_tmp: ComponentSender<StyleToolbar> = sender.clone();
        let color_action: RelmAction<ColorAction> = RelmAction::new_stateful_with_target_value(
            &ColorButtons::Orange,
            move |_, state, value| {
                *state = value;

                // custom color will be handled by presenting a dialog
                if let Some(color) = Self::map_button_to_color(value) {
                    sender_tmp
                        .output_sender()
                        .emit(ToolbarEvent::ColorSelected(color));
                }
            },
        );

        // Size Action for selecting sizes
        let sender_tmp = sender.clone();
        let size_action: RelmAction<SizeAction> =
            RelmAction::new_stateful_with_target_value(&Size::Medium, move |_, state, value| {
                *state = value;
                sender_tmp
                    .output_sender()
                    .emit(ToolbarEvent::SizeSelected(*state));
            });

        let mut group = RelmActionGroup::<StyleToolbarActionGroup>::new();
        group.add_action(color_action);
        group.add_action(size_action);

        group.register_for_widget(&widgets.root);

        ComponentParts { model, widgets }
    }
}
relm4::new_action_group!(ToolsToolbarActionGroup, "tools-toolbars");
relm4::new_stateful_action!(ToolsAction, ToolsToolbarActionGroup, "tools", Tools, Tools);

relm4::new_action_group!(StyleToolbarActionGroup, "style-toolbars");
relm4::new_stateful_action!(
    ColorAction,
    StyleToolbarActionGroup,
    "colors",
    ColorButtons,
    ColorButtons
);
relm4::new_stateful_action!(SizeAction, StyleToolbarActionGroup, "sizes", Size, Size);

impl StaticVariantType for ColorButtons {
    fn static_variant_type() -> Cow<'static, VariantTy> {
        Cow::Borrowed(VariantTy::UINT16)
    }
}

impl ToVariant for ColorButtons {
    fn to_variant(&self) -> Variant {
        Variant::from(*self as u16)
    }
}

impl FromVariant for ColorButtons {
    fn from_variant(variant: &Variant) -> Option<Self> {
        <u16>::from_variant(variant).and_then(|v| match v {
            0 => Some(ColorButtons::Orange),
            1 => Some(ColorButtons::Red),
            2 => Some(ColorButtons::Green),
            3 => Some(ColorButtons::Blue),
            4 => Some(ColorButtons::Cove),
            5 => Some(ColorButtons::Custom),
            _ => None,
        })
    }
}
