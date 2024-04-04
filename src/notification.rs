use gdk_pixbuf::gio::FileIcon;
use relm4::gtk::gio::{prelude::ApplicationExt, Notification};

use relm4::gtk::{IconLookupFlags, IconTheme, TextDirection};

pub fn show_notification(msg: &str) {
    // construct
    let notification = Notification::new("Satty");
    notification.set_body(Some(msg));

    // lookup sattys icon
    let theme = IconTheme::default();
    if theme.has_icon("satty") {
        if let Some(icon_file) = theme
            .lookup_icon(
                "satty",
                &[],
                96,
                1,
                TextDirection::Ltr,
                IconLookupFlags::empty(),
            )
            .file()
        {
            notification.set_icon(&FileIcon::new(&icon_file));
        }
    }

    // send notification
    relm4::main_application().send_notification(None, &notification);
}
