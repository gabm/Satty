use crate::{
    style::{Color, Size},
    tools::Tools,
};

use relm4::{
    actions::{ActionablePlus, RelmAction, RelmActionGroup},
    gtk::{prelude::*, Align},
    prelude::*,
};

pub struct ToolsToolbar {
    config: ToolsToolbarConfig,
}

pub struct ToolsToolbarConfig {
    pub show_save_button: bool,
}

pub struct StyleToolbar {}

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

fn create_icon(color: Color) -> gtk::Image {
    let pixbuf = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, false, 8, 40, 40).unwrap();
    pixbuf.fill(color.to_rgba_u32());

    gtk::Image::from_pixbuf(Some(&pixbuf))
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
                set_tooltip: "Undo",
                connect_clicked[sender] => move |_| {sender.output(ToolbarEvent::Undo).unwrap();},
            },
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: "arrow-redo-filled",
                set_tooltip: "Redo",
                connect_clicked[sender] => move |_| {sender.output(ToolbarEvent::Redo).unwrap();},
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
                set_tooltip: "Copy to clipboard",
                connect_clicked[sender] => move |_| {sender.output(ToolbarEvent::CopyClipboard).unwrap();},
            },
            gtk::Button {
                set_focusable: false,
                set_hexpand: false,

                set_icon_name: "save-regular",
                set_tooltip: "Save",
                connect_clicked[sender] => move |_| {sender.output(ToolbarEvent::SaveFile).unwrap();},

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
            RelmAction::new_stateful_with_target_value(&Tools::Crop, move |_, state, value| {
                *state = value;
                sender_tmp
                    .output(ToolbarEvent::ToolSelected(*state))
                    .unwrap();
            });

        let mut group = RelmActionGroup::<ToolsToolbarActionGroup>::new();
        group.add_action(tool_action);
        group.register_for_widget(&widgets.root);

        ComponentParts { model, widgets }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for StyleToolbar {
    type Init = ();
    type Input = ();
    type Output = ToolbarEvent;

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

                create_icon(Color::Orange),

                ActionablePlus::set_action::<ColorAction>: Color::Orange,
            },
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::Red),

                ActionablePlus::set_action::<ColorAction>: Color::Red,
            },
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::Green),

                ActionablePlus::set_action::<ColorAction>: Color::Green,
            },
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::Blue),

                ActionablePlus::set_action::<ColorAction>: Color::Blue,
            },
            gtk::ToggleButton {
                set_focusable: false,
                set_hexpand: false,

                create_icon(Color::Cove),

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
    }

    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = StyleToolbar {};
        let widgets = view_output!();

        // Color Action for selecting colors
        let sender_tmp = sender.clone();
        let color_action: RelmAction<ColorAction> =
            RelmAction::new_stateful_with_target_value(&Color::Orange, move |_, state, value| {
                *state = value;
                sender_tmp
                    .output(ToolbarEvent::ColorSelected(*state))
                    .unwrap();
            });

        // Size Action for selecting sizes
        let sender_tmp = sender.clone();
        let size_action: RelmAction<SizeAction> =
            RelmAction::new_stateful_with_target_value(&Size::Medium, move |_, state, value| {
                *state = value;
                sender_tmp
                    .output(ToolbarEvent::SizeSelected(*state))
                    .unwrap();
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
relm4::new_stateful_action!(ColorAction, StyleToolbarActionGroup, "colors", Color, Color);
relm4::new_stateful_action!(SizeAction, StyleToolbarActionGroup, "sizes", Size, Size);
